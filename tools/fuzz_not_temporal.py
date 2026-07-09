#!/usr/bin/env python3
"""not×temporal population differ (D-128 recon → staging the `not` slab).

Sibling of `fuzz_exists_temporal.py`, but `not` instead of `exists` and with
an OPTIONAL trailing clock advance — because the `not` divergence is not
admission order (that was exists/D-127); it is (gap-1) a window-CLOSE firing
DEFERRAL and (gap-2) @expires inference THROUGH the not-temporal. So the fuzz
sweeps both axes: a coin-flip trailing `advance` (probes the deferral timing —
Seine fires a satisfied `not` immediately, Drools waits for the window to
close) and explicit-vs-absent @expires (probes the inference).

Shapes: `not` partner off a positive anchor (`not_partner`), off a positive
chain (`chain_not`), or between two positives (`not_mid`); after+before
windows, 1-2 anchors, shuffled insertion. Runs `seine-harness diff` and keeps
divergent scenarios in <outdir>.

NOTE: on the main tree every `not`-temporal case FAILS at compile (the D-120
wall stays up for `not`; D-127 lifted it for `exists` only) — that is the wall
working. Point [repo] at an UNFENCED scratch (`CeKind::Not` allowed too) to
measure real divergences.

Usage: fuzz_not_temporal.py <n> <seed> <outdir> [repo]
"""
import json
import os
import random
import subprocess
import sys

EXP = 100000
REPO = "/home/bryan/rust-rules"


def etype(n, expires):
    ev = {"timestamp": "ts"}
    if expires:
        ev["expires_ms"] = EXP
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}], "event": ev}


def gen(rng, name):
    shape = rng.choice(["not_partner", "not_partner", "chain_not", "not_mid"])
    op1 = rng.choice(["after", "before"])
    h1 = rng.choice([50, 100, 150])
    expires = rng.random() < 0.5           # inference axis
    advance = rng.random() < 0.5           # deferral axis
    if shape == "not_partner":
        drl = f"rule NT when $a : E0() not E1(this {op1}[0ms,{h1}ms] $a) then end\n"
        ntypes = 2
    elif shape == "chain_not":
        op2 = rng.choice(["after", "before"])
        h2 = rng.choice([50, 100])
        drl = (f"rule NT when $a : E0() $b : E1(this {op1}[0ms,{h1}ms] $a) "
               f"not E2(this {op2}[0ms,{h2}ms] $b) then end\n")
        ntypes = 3
    else:  # not_mid: not between two positives, both anchored on $a
        op2 = rng.choice(["after", "before"])
        h2 = rng.choice([50, 100])
        drl = (f"rule NT when $a : E0() not E1(this {op1}[0ms,{h1}ms] $a) "
               f"$c : E2(this {op2}[0ms,{h2}ms] $a) then end\n")
        ntypes = 3
    facts = []
    a_ts = rng.sample(range(0, 6), rng.choice([1, 1, 2]))
    facts += [("E0", t) for t in a_ts]
    base = a_ts[0]
    for lvl in range(1, ntypes):
        op, hi = (op1, h1) if lvl == 1 else (op2, h2)
        tss, used = [], set()
        for _ in range(rng.randint(1, 3)):
            for _try in range(20):
                d = rng.randint(0, hi + 10)   # some window misses too
                ts = base + d if op == "after" else base - d
                if ts not in used:
                    used.add(ts)
                    tss.append(ts)
                    break
        facts += [(f"E{lvl}", t) for t in tss]
        base = tss[0]
    rng.shuffle(facts)
    epochs = [{"actions": [{"op": "advance", "ms": 1000}], "facts": []}] if advance else []
    return {"name": name, "types": [etype(f"E{i}", expires) for i in range(ntypes)],
            "drl": drl,
            "facts": [{"type": t, "fields": {"ts": ts}} for t, ts in facts],
            "epochs": epochs}


def main():
    global REPO
    n, seed, outdir = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    if len(sys.argv) > 4:
        REPO = sys.argv[4]
    os.makedirs(outdir, exist_ok=True)
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    rng = random.Random(seed)
    done = ndiff = 0
    diffs = []
    while done < n:
        paths = []
        for i in range(done, min(done + 150, n)):
            scn = gen(rng, f"nt{seed}x{i}")
            p = os.path.join(outdir, f"nt{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
        r = subprocess.run(
            ["cargo", "run", "-q", "-p", "seine-harness", "--", "diff"] + paths,
            cwd=REPO, capture_output=True, text=True, env=env, timeout=300)
        for line in r.stdout.splitlines():
            if line.startswith("FAIL"):
                ndiff += 1
                diffs.append(line.split()[1])
                print(line)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            if nm not in diffs:
                os.remove(p)
        done += len(paths)
    print(f"--- not-temporal: {n} cases seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")
    if diffs:
        print("kept:", " ".join(diffs[:30]))


if __name__ == "__main__":
    main()
