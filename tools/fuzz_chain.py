#!/usr/bin/env python3
"""Targeted temporal-CHAIN fuzz — population-measures the join-order fix
on its exact shape: 2- and 3-node after/before chains with random
anchor/partner multiplicities, insertion orders, and timestamps.

Runs engine (`run`) and oracle (`oracle`) over a batch and compares the
full firing order. Reports divergences with the scenario so they can be
bisected against HEAD.

Usage: chain_fuzz.py <n> <seed> <outdir>
"""
import json
import os
import random
import subprocess
import sys

EXP = 100000


def etype(n):
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": EXP}}


def gen(rng, name, nnodes):
    """Build a random chain E0..En. Single anchor (one E0) so we stay in
    the solved single-anchor family, but random E1.. multiplicities."""
    types = [f"E{i}" for i in range(nnodes + 1)]
    ops = []
    # per-join op + window
    for i in range(nnodes):
        op = rng.choice(["after", "before"])
        hi = rng.choice([50, 100, 150, 200])
        ops.append((op, hi))
    # build DRL
    conj = ["$a0 : E0()"]
    for i in range(1, nnodes + 1):
        op, hi = ops[i - 1]
        conj.append(f"$a{i} : E{i}(this {op}[0ms,{hi}ms] $a{i-1})")
    drl = f"rule CH when {' '.join(conj)} then end\n"

    # choose timestamps so every partner joins its predecessor.
    # anchor E0 @ t0; E_i @ t_{i-1} + delta within [0,hi] (after) or minus (before)
    facts = []  # (type, ts)
    t0 = 0
    # E1 partners: pick a count 1..4, each a distinct ts that joins E0
    counts = [rng.randint(1, 4)] + [rng.randint(1, 2) for _ in range(nnodes - 1)]
    # for a clean single-anchor chain, layer by layer we need one representative
    # timestamp per layer to anchor the next layer's window; use the first.
    layer_anchor = t0
    per_layer_ts = []
    for i in range(1, nnodes + 1):
        op, hi = ops[i - 1]
        base = layer_anchor
        tss = []
        used = set()
        for _ in range(counts[i - 1]):
            for _try in range(20):
                d = rng.randint(0, hi)
                ts = base + d if op == "after" else base - d
                if ts not in used:
                    used.add(ts)
                    tss.append(ts)
                    break
        per_layer_ts.append(tss)
        layer_anchor = tss[0]  # chain off the first partner of this layer
    # keep only chains where deeper layers still join the chosen anchor:
    # we anchored each layer off tss[0], so the FIRST of each layer forms a
    # valid chain; other partners of layer i join E_{i-1}=per_layer_ts[i-2][0]
    # via the same window base, so they're valid too.

    # assemble insertion order: E0 plus all partners, shuffled
    facts.append(("E0", t0))
    for i in range(1, nnodes + 1):
        for ts in per_layer_ts[i - 1]:
            facts.append((f"E{i}", ts))
    rng.shuffle(facts)

    return {"name": name, "types": [etype(t) for t in types], "drl": drl,
            "facts": [{"type": t, "fields": {"ts": ts}} for t, ts in facts],
            "epochs": []}, ops, per_layer_ts


REPO = "/home/bryan/rust-rules"


def run(cmd, files, cwd):
    cwd = REPO
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", cmd] + files,
                       cwd=cwd, capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try:
            o = json.loads(line)
        except Exception:
            continue
        res = o.get("result")
        out[o["scenario"]] = None if res is None else res.get("firings", [])
    return out


def seq(firings):
    if firings is None:
        return None
    return ["+".join(sorted(f'{m["type"]}{m["fields"]["ts"]}' for m in fr["matches"]))
            for fr in firings]


def main():
    global REPO
    n, seed, outdir = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
    if len(sys.argv) > 4:
        REPO = sys.argv[4]
    os.makedirs(outdir, exist_ok=True)
    cwd = REPO
    rng = random.Random(seed)
    BATCH = 200
    ndiff = 0
    done = 0
    diffs = []
    while done < n:
        paths = []
        for i in range(done, min(done + BATCH, n)):
            nnodes = rng.choice([2, 2, 3])  # mostly 2-node
            scn, _, _ = gen(rng, f"ch{seed}x{i}", nnodes)
            p = os.path.join(outdir, f"ch{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
        eng = run("run", paths, cwd)
        ora = run("oracle", paths, cwd)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            e, o = seq(eng.get(nm)), seq(ora.get(nm))
            if e != o:
                ndiff += 1
                diffs.append(nm)
                print(f"DIFF {nm}")
            else:
                os.remove(p)
        done += len(paths)
    print(f"--- chain-fuzz: {n} cases seed {seed}: {ndiff} divergences")
    if diffs:
        print("kept:", " ".join(diffs))


if __name__ == "__main__":
    main()
