#!/usr/bin/env python3
"""not×temporal INFERENCE reference model (arc B, D-130 → chains, D-131). Covers
the three fuzz_not_temporal shapes — `not_partner`, `chain_not`, `not_mid` —
windows `[0ms,hi]` (lo=0), @expires either ABSENT (inferred reach) or a LARGE
explicit value (arc-A isolation). `simulate()` is the candidate spec; the
harness differs it vs the gate oracle over a shuffled population with a
coin-flip single clock advance.

--- the inference semantic (arc B, oracle-measured; docs/drools-inferred-expiry-
    never.md for the getExpirationOffset mechanism) ---
For `not E1(this OP[lo,hi] $a)` (constraint `E1 OP[lo,hi] E0`) Drools infers a
per-TYPE expiration offset = max upperBound of the type's row in the temporal-
distance matrix (NEVER when < 0). after ⇒ off(E0)=hi, off(E1)=(lo?NEVER:0);
before ⇒ mirror. Reap at ts+off+1; explicit @expires=E ⇒ off=E. KEY (D-130):
for this lo=0 population the inference is INVISIBLE to firings — a blocked
anchor stays silent whether the blocker is inferred-mortal or explicit-immortal,
so arc A ≡ arc B on the firing SET (they differ only in reaped facts). Verified
to hold across all three shapes.

--- firing (D-130 not_partner, D-131 chains) ---
Fire-points are clock 0 and the single advance. Each shape reduces to:
  1. POSITIVE matches (tuples), enumerated in D-125 temporal-join creation order
     (`model_join_flush.Node`): not_partner ⇒ the anchor a itself; chain_not ⇒
     (a,b) over E0-E1(op1); not_mid ⇒ (a,c) over E0-E2(op2).
  2. The `not` FILTERS (blocked ⇒ silent) and DEFERS: a tuple is blocked when
     the not's pattern has an in-window event on its anchor (not_partner/not_mid
     anchor = a; chain_not anchor = b). ft (window close) = anchor+hn (after) /
     anchor (before), hn = the not's hi.
  3. Fire clock-0 tuples (ft<=0) in FIFO creation order; the advance batch fires
     descending close-time (ft). ft < death always holds (lo=0), so an unblocked
     tuple always outlives its close.

Faithfulness bar / scope (D-131):
  * FIRING SET — ZERO divergences, all 3 shapes, ~4500 cases (the semantic
    content: which tuples match/fire is fully modelled).
  * ORDER — 0-div for not_partner and for the CROSS-close-time ordering; the
    residual (~0.6%, chain_not/not_mid only, all order-only, never a set/count
    miss) is the WITHIN-same-close-time multi-tuple order. Source read (drools-
    core 9.44, D-131): temporal not defers via a scheduled window-close; the
    un-block re-propagates through PhreakNotNode.doRightDeletes (addInsert =
    prepend), and ALL time-scheduled firings drain a PriorityQueue ordered SOLELY
    by fire-time (DefaultTimerJobInstance.compareTo, no secondary key). So same-
    close-time order is a Java binary-heap ARTIFACT of the add/poll sequence, not
    a clean semantic (why it flip-flops black-box; same class as fz_42_84 hash-
    order). FENCED here; the engine port matches it only if Seine's scheduler
    reproduces Drools' PQ tie-order, else these graduate to xfail/ (heap-order
    expected-divergences). Repro: nif7001x146, nif7002x120, nif7003x321.
"""
import json
import os
import random
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from model_join_flush import Node, win_match          # validated D-125 join order

REPO = "/home/bryan/rust-rules"
EXP = 100000            # the population's "large" explicit @expires (arc-A leg)


def etype(n, expires):
    ev = {"timestamp": "ts"}
    if expires:
        ev["expires_ms"] = EXP
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}], "event": ev}


def gen(rng, name):
    shape = rng.choice(["not_partner", "not_partner", "chain_not", "not_mid"])
    op1 = rng.choice(["after", "before"])
    h1 = rng.choice([50, 100, 150])
    expires = rng.random() < 0.5
    advance = rng.random() < 0.5
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


def _join_tuples(scn, op, hi, left, right):
    """positive-join tuples [a, partner] in D-125 creation order."""
    node = Node(op, 0, hi)
    for f in scn["facts"]:
        if f["type"] == left:
            node.left_insert([f["fields"]["ts"]])
        elif f["type"] == right:
            node.right_insert(f["fields"]["ts"])
    return [list(t) for t in node.firings]


def _wins(drl):
    return [(m.group(1), int(m.group(2)), int(m.group(3)))
            for m in re.finditer(r'(after|before)\[(\d+)ms,(\d+)ms\]', drl)]


def simulate(scn):
    drl = scn["drl"]
    wins = _wins(drl)
    tsof = lambda T: [f["fields"]["ts"] for f in scn["facts"] if f["type"] == T]

    # entries: (render, ft, crt) — one per surviving positive tuple.  The not's
    # anchor is where its window closes; ft = anchor+hn (after) / anchor (before).
    entries = []
    if "$b : E1" in drl:                                   # chain_not
        (op1, _, h1), (opn, _, hn) = wins                  # join E0-E1, not E2
        e2 = tsof("E2")
        for crt, (a, b) in enumerate(_join_tuples(scn, op1, h1, "E0", "E1")):
            if any(win_match(opn, 0, hn, b, x) for x in e2):
                continue                                    # blocked -> silent
            ft = b + hn if opn == "after" else b
            entries.append((f"{a}-{b}", ft, crt))
    elif "$c : E2" in drl:                                  # not_mid
        (opn, _, hn), (op2, _, h2) = wins                  # not E1, join E0-E2
        e1 = tsof("E1")
        for crt, (a, c) in enumerate(_join_tuples(scn, op2, h2, "E0", "E2")):
            if any(win_match(opn, 0, hn, a, x) for x in e1):
                continue
            ft = a + hn if opn == "after" else a
            entries.append((f"{a}-{c}", ft, crt))
    else:                                                   # not_partner
        (opn, _, hn), = wins
        e1 = tsof("E1")
        for crt, a in enumerate(tsof("E0")):
            if any(win_match(opn, 0, hn, a, x) for x in e1):
                continue
            ft = a + hn if opn == "after" else a
            entries.append((str(a), ft, crt))

    maxclock = sum(act["ms"] for ep in scn.get("epochs", [])
                   for act in ep.get("actions", []) if act.get("op") == "advance")
    clocks = [0] + ([maxclock] if maxclock > 0 else [])

    fired, firings = set(), []
    for ci, clock in enumerate(clocks):
        batch = [e for e in entries if e[2] not in fired and e[1] <= clock]
        if ci == 0:
            batch.sort(key=lambda e: e[2])                     # clock-0: FIFO/creation
        else:
            batch.sort(key=lambda e: (-e[1], e[2]))            # advance: desc close-time, then creation
        for r, _, crt in batch:
            fired.add(crt)
            firings.append(r)
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
        if not res:
            out[o["scenario"]] = []
            continue
        seq = []
        for fr in res["firings"]:
            d = {m["type"]: m["fields"]["ts"] for m in fr["matches"]}
            seq.append("-".join(str(d[f"E{i}"]) for i in sorted(int(k[1:]) for k in d)))
        out[o["scenario"]] = seq
    return out


def fuzz(n, seed):
    rng = random.Random(seed)
    OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/model_not_infer"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    by_shape = {}
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
            shape = ("chain_not" if "$b : E1" in scns[nm]["drl"]
                     else "not_mid" if "$c : E2" in scns[nm]["drl"] else "not_partner")
            pred = simulate(scns[nm])
            og = ora.get(nm)
            if pred != og:
                ndiff += 1
                diffs.append(nm)
                # classify: order-only (same multiset) vs a genuine SET/count miss
                kind = "ORDER" if sorted(pred) == sorted(og or []) else "SET"
                by_shape[f"{shape}/{kind}"] = by_shape.get(f"{shape}/{kind}", 0) + 1
                if kind == "SET" or len(diffs) <= 8:
                    print(f"  DIV {nm} [{shape}]: model={pred} oracle={ora.get(nm)}")
                    print(f"      drl={scns[nm]['drl'].strip()}")
                    print(f"      facts={[(f['type'], f['fields']['ts']) for f in scns[nm]['facts']]}"
                          f" exp={scns[nm]['types'][0]['event'].get('expires_ms','-')}"
                          f" adv={bool(scns[nm]['epochs'])}")
        done += len(paths)
    tag = f" by-shape {by_shape}" if by_shape else ""
    print(f"not-infer model-vs-oracle: {n} seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%){tag}")


if __name__ == "__main__":
    fuzz(int(sys.argv[2]), int(sys.argv[3])) if sys.argv[1] == "fuzz" else None
