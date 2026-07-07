#!/usr/bin/env python3
"""CEP E1 fuzz (D-101): draws deterministic point-event scenarios —
explicit @expires everywhere (inference is E2-fenced, a8), pseudo-clock
advances, after/before temporal joins, TMS justification off events
(the a6/a7 composition), not/exists over events — and differentials
full canonical outputs (firings + final WM) through the standard
`seine-harness diff` batches (one oracle JVM per batch).

Fences honored (D-101): no windows (E2); no inference (explicit
expires_ms); events never updated/deleted externally; event timestamps
drawn at-or-after the current scenario clock (the past-deadline insert
edge is unprobed); distinct-or-tied deadlines both drawn (a2: ties are
pinned stable).

Usage: .venv/bin/python tools/fuzz_cep.py <n> <seed>
"""
import json
import os
import random
import subprocess
import sys

BATCH = 150


class Gen:
    def __init__(self, rng):
        self.r = rng

    def scenario(self, name):
        r = self.r
        # types: 2 event types + 1 plain + logical target
        self.etypes = []
        types = []
        for i in range(2):
            exp = r.choice([50, 100, 100, 200, 400])
            types.append({
                "name": f"E{i}",
                "fields": [{"name": "ts", "type": "i64"}, {"name": "tag", "type": "String"}],
                "event": {"timestamp": "ts", "expires_ms": exp},
            })
            self.etypes.append((f"E{i}", exp))
        types.append({"name": "P", "fields": [{"name": "v", "type": "i64"}]})
        types.append({"name": "D", "fields": [{"name": "tag", "type": "String"}]})
        types.append({"name": "P3", "fields": [{"name": "v", "type": "i64"}]})

        rules = []
        ri = 0
        # temporal-join rule(s)
        for _ in range(r.randint(1, 2)):
            a, b = r.choice(self.etypes)[0], r.choice(self.etypes)[0]
            op = r.choice(["after", "before"])
            lo = r.choice([0, 0, 50])
            hi = lo + r.choice([50, 100, 150])
            cons = f'tag == "{r.choice("xyz")}"' if r.random() < 0.3 else ""
            sal = f" salience {r.randint(-5, 15)}" if r.random() < 0.4 else ""
            rules.append(
                f'rule TJ{ri}{sal} when $a : {a}({cons}) '
                f'$b : {b}(this {op}[{lo}ms,{hi}ms] $a) then end'
            )
            ri += 1
        # TMS justification off an event + observers (a6/a7 shape)
        if r.random() < 0.75:
            e = r.choice(self.etypes)[0]
            rules.append(f'rule J{ri} when $e : {e}($t : tag) then insertLogical(new D($t)); end')
            ri += 1
            rules.append(f'rule RD{ri} salience {r.randint(0, 12)} when D() then end')
            ri += 1
            rules.append(f'rule ND{ri} salience {r.randint(-8, 12)} when not D() P() then end')
            ri += 1
            if r.random() < 0.5:
                # the a7c shape: a same-epoch chain around the cascade
                rules.append(f'rule G{ri} salience {r.randint(8, 20)} when P(v == 2) then insert(new P3(3)); end')
                ri += 1
                rules.append(f'rule C{ri} salience {r.randint(-5, 5)} when P3() then end')
                ri += 1
        # not/exists over events
        if r.random() < 0.6:
            e = r.choice(self.etypes)[0]
            neg = r.choice(["not", "exists"])
            sal = f" salience {r.randint(-8, 8)}" if r.random() < 0.5 else ""
            rules.append(f'rule NE{ri}{sal} when {neg} {e}() P() then end')
            ri += 1

        # facts at clock 0
        facts = [{"type": "P", "fields": {"v": 1}}]
        for _ in range(r.randint(1, 4)):
            t = r.choice(self.etypes)[0]
            facts.append({"type": t, "fields": {"ts": r.randint(0, 40), "tag": r.choice("xyz")}})

        # epochs: advances + fresh events at/after the running clock
        clock = 0
        epochs = []
        for _ in range(r.randint(1, 3)):
            actions = []
            if r.random() < 0.15:
                # D-104: in-place session reset — the paged-batch axis
                actions.append({"op": "reset"})
                clock = 0
            if r.random() < 0.9:
                ms = r.choice([30, 50, 100, 100, 150, 300, 600])
                actions.append({"op": "advance", "ms": ms})
                clock += ms
            efacts = []
            for _ in range(r.randint(0, 2)):
                t = r.choice(self.etypes)[0]
                efacts.append({"type": t, "fields": {"ts": clock + r.randint(0, 30), "tag": r.choice("xyz")}})
            if r.random() < 0.3:
                efacts.append({"type": "P", "fields": {"v": 2}})
            epochs.append({"actions": actions, "facts": efacts})

        return {"name": name, "types": types, "drl": "\n".join(rules) + "\n",
                "facts": facts, "epochs": epochs}


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    tmp = os.environ.get("FUZZ_TMP", "/tmp") + f"/cepfuzz_{seed}"
    os.makedirs(tmp, exist_ok=True)
    fails = 0
    done = 0
    while done < n:
        batch = []
        for i in range(done, min(done + BATCH, n)):
            scn = Gen(random.Random(seed * 7_654_321 + i)).scenario(f"cf{seed}x{i}")
            path = f"{tmp}/cf{seed}x{i}.json"
            json.dump(scn, open(path, "w"), indent=1)
            batch.append(path)
        r = subprocess.run(
            ["cargo", "run", "-q", "-p", "seine-harness", "--", "diff", *batch],
            capture_output=True, text=True,
        )
        for line in r.stdout.splitlines():
            if line.startswith("FAIL"):
                fails += 1
                print(line)
        for line in r.stdout.splitlines()[-1:]:
            print(f"  batch@{done}: {line}")
        done += len(batch)
        for p in batch:
            base = os.path.basename(p).split(".")[0]
            keep = any(l.startswith("FAIL") and l.split()[1] == base
                       for l in r.stdout.splitlines())
            if not keep:
                os.remove(p)
    print(f"--- cep-fuzz complete: {n} cases, seed {seed}, {fails} divergences")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
