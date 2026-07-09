#!/usr/bin/env python3
"""not×temporal INFERENCE reference model (arc B, D-130 — the coupled arc after
the D-129 firing-deferral). Single `not` off a positive anchor (`not_partner`),
windows `[0ms,hi]` (lo=0, matching the fuzz_not_temporal population), @expires
either ABSENT (inferred reach) or a LARGE explicit value (arc-A isolation).
`simulate()` is the candidate spec; the harness differs it vs the gate oracle
over a shuffled population with a coin-flip single clock advance.

--- the inference semantic (arc B, oracle-measured; see docs/drools-inferred-
    expiry-never.md for the getExpirationOffset mechanism) ---
For `not E1(this OP[lo,hi] $a)` the temporal constraint is `E1 OP[lo,hi] E0`.
Drools infers a per-TYPE expiration offset = max upperBound of the type's row
in the temporal-distance matrix (NEVER when < 0). Measured:
  after[lo,hi]:  offset(E0)=hi          offset(E1)= (lo==0 ? 0 : NEVER)
  before[lo,hi]: offset(E0)=(lo==0?0:NEVER)  offset(E1)=hi
An event of type T with offset o is reaped when clock >= T.ts + o + 1 (present
through ts+o). NEVER = never reaped. Explicit @expires=E overrides to offset=E.

Firing (this lo=0 population; fire-points are clock 0 and the single advance):
  * ft (window close) = a+hi (after) / a-lo=a (before).
  * An anchor with NO in-window blocker arms a window-close TIMER: it fires at
    continuous clock ft during whatever advance spans ft (robust to its own
    later reaping — ft < death always holds when lo=0). Fires at clock 0 when
    ft==0 (before, a=0).
  * An anchor WITH an in-window blocker at insertion does NOT arm the timer.
    Un-blocking via expiry does not re-arm it, and the only post-expiry fire-
    point (the advance end) has every finite-offset event already reaped -> it
    NEVER fires. (Same firing outcome as arc A's immortal blocker: blocked =>
    silent, whether the blocker is inferred-mortal or explicit-immortal.)
  * Order: within one advance, reverse close-time (descending ft, the PREPEND
    discipline); clock-0 firings precede advance firings.

Faithfulness bar: ZERO model-vs-oracle divergences on the shuffled population.
"""
import json
import os
import random
import re
import subprocess
import sys

REPO = "/home/bryan/rust-rules"
EXP = 100000            # the population's "large" explicit @expires (arc-A leg)


def etype(n, expires):
    ev = {"timestamp": "ts"}
    if expires:
        ev["expires_ms"] = EXP
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}], "event": ev}


def gen(rng, name):
    op = rng.choice(["after", "before"])
    hi = rng.choice([50, 100, 150])
    expires = rng.random() < 0.5
    advance = rng.random() < 0.5
    drl = f"rule NT when $a : E0() not E1(this {op}[0ms,{hi}ms] $a) then end\n"
    a_ts = rng.sample(range(0, 6), rng.choice([1, 1, 2]))
    facts = [("E0", t) for t in a_ts]
    base = a_ts[0]
    used = set()
    for _ in range(rng.randint(1, 3)):
        for _try in range(20):
            d = rng.randint(0, hi + 10)              # some window misses too
            ts = base + d if op == "after" else base - d
            if ts not in used:
                used.add(ts)
                facts.append(("E1", ts))
                break
    rng.shuffle(facts)
    epochs = [{"actions": [{"op": "advance", "ms": 1000}], "facts": []}] if advance else []
    return {"name": name, "types": [etype("E0", expires), etype("E1", expires)],
            "drl": drl,
            "facts": [{"type": t, "fields": {"ts": v}} for t, v in facts],
            "epochs": epochs}


def simulate(scn):
    m = re.search(r'not E1\(this (after|before)\[(\d+)ms,(\d+)ms\] \$a\)', scn["drl"])
    op, lo, hi = m.group(1), int(m.group(2)), int(m.group(3))
    anchors = [f["fields"]["ts"] for f in scn["facts"] if f["type"] == "E0"]
    blockers = [f["fields"]["ts"] for f in scn["facts"] if f["type"] == "E1"]

    def in_window(b, a):
        return (a + lo <= b <= a + hi) if op == "after" else (a - hi <= b <= a - lo)

    # single-advance population: fire-points are clock 0 and the advance end
    maxclock = 0
    for ep in scn.get("epochs", []):
        for act in ep.get("actions", []):
            if act.get("op") == "advance":
                maxclock += act["ms"]
    clocks = [0] + ([maxclock] if maxclock > 0 else [])

    # ft (window-close) per unblocked anchor; blocked anchors never fire here
    entries = []                                     # (a_ts, ft)
    for a in anchors:
        if any(in_window(b, a) for b in blockers):
            continue
        ft = a + hi if op == "after" else a - lo
        entries.append((a, ft))

    fired, firings = set(), []
    for clock in clocks:
        batch = [(a, ft) for (a, ft) in entries if a not in fired and ft <= clock]
        batch.sort(key=lambda p: p[1], reverse=True)  # reverse close-time (PREPEND)
        for a, _ in batch:
            fired.add(a)
            firings.append(a)
    return firings


def _oracle(paths):
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
                       cwd=REPO, capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try:
            o = json.loads(line)
        except Exception:
            continue
        res = o.get("result")
        out[o["scenario"]] = None if not res else [
            fr["matches"][0]["fields"]["ts"] for fr in res["firings"]]
    return out


def fuzz(n, seed):
    rng = random.Random(seed)
    OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/model_not_infer"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 150, n)):
            scn = gen(rng, f"nif{seed}x{i}")
            p = os.path.join(OUT, f"nif{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = _oracle(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            pred = simulate(scns[nm])
            if pred != ora.get(nm):
                ndiff += 1
                diffs.append(nm)
                if len(diffs) <= 15:
                    print(f"  DIV {nm}: model={pred} oracle={ora.get(nm)}")
                    print(f"      drl={scns[nm]['drl'].strip()}")
                    print(f"      facts={[(f['type'], f['fields']['ts']) for f in scns[nm]['facts']]}"
                          f" exp={'ts' in json.dumps(scns[nm]['types'][0].get('event',{})) and scns[nm]['types'][0]['event'].get('expires_ms','-')}"
                          f" epochs={scns[nm]['epochs']}")
        done += len(paths)
    print(f"not-infer model-vs-oracle: {n} seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")


if __name__ == "__main__":
    fuzz(int(sys.argv[2]), int(sys.argv[3])) if sys.argv[1] == "fuzz" else None
