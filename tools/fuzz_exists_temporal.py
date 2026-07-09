#!/usr/bin/env python3
"""exists×temporal population differ (D-126 → the unwalling slab's gate).

Shapes: an exists-temporal partner off a positive anchor (`ex_partner`),
off a positive chain (`chain_ex`), or between two positives (`ex_mid`);
after+before windows, 1-2 anchors, shuffled insertion, explicit @expires
everywhere (inference stays OUT of scope — probe it separately). Runs
`seine-harness diff` (engine vs oracle, full canonical output) and keeps
divergent scenarios in <outdir>.

NOTE: on the GATED tree every case FAILS at compile (the D-120 wall:
"temporal constraints on not/exists CEs are a follow-on slab") — that is
the wall working. Point [repo] at an UNFENCED scratch worktree (or the
ported engine) to measure real divergences. D-126 baseline on the
unfenced D-125 engine: 10/450 @ seed 11001 — ALL multi-anchor admission
order (engine insertion-order vs oracle most-recently-blocked-first).

Usage: fuzz_exists_temporal.py <n> <seed> <outdir> [repo]
"""
import json
import os
import random
import subprocess
import sys

EXP = 100000
REPO = "/home/bryan/rust-rules"


def etype(n):
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": EXP}}


def gen(rng, name):
    shape = rng.choice(["ex_partner", "ex_partner", "chain_ex", "ex_mid"])
    op1 = rng.choice(["after", "before"])
    h1 = rng.choice([50, 100, 150])
    if shape == "ex_partner":
        drl = (f"rule EX when $a : E0() exists E1(this {op1}[0ms,{h1}ms] $a) then end\n")
        ntypes = 2
    elif shape == "chain_ex":
        op2 = rng.choice(["after", "before"])
        h2 = rng.choice([50, 100])
        drl = (f"rule EX when $a : E0() $b : E1(this {op1}[0ms,{h1}ms] $a) "
               f"exists E2(this {op2}[0ms,{h2}ms] $b) then end\n")
        ntypes = 3
    else:  # ex_mid: exists between two positives, both anchored on $a
        op2 = rng.choice(["after", "before"])
        h2 = rng.choice([50, 100])
        drl = (f"rule EX when $a : E0() exists E1(this {op1}[0ms,{h1}ms] $a) "
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
                d = rng.randint(0, hi + 10)  # some window misses too
                ts = base + d if op == "after" else base - d
                if ts not in used:
                    used.add(ts)
                    tss.append(ts)
                    break
        facts += [(f"E{lvl}", t) for t in tss]
        base = tss[0]
    rng.shuffle(facts)
    return {"name": name, "types": [etype(f"E{i}") for i in range(ntypes)],
            "drl": drl,
            "facts": [{"type": t, "fields": {"ts": ts}} for t, ts in facts],
            "epochs": []}


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
            scn = gen(rng, f"ext{seed}x{i}")
            p = os.path.join(outdir, f"ext{seed}x{i}.json")
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
    print(f"--- exists-temporal: {n} cases seed {seed}: {ndiff} divergences")
    if diffs:
        print("kept:", " ".join(diffs[:30]))


if __name__ == "__main__":
    main()
