#!/usr/bin/env python3
"""SHARED temporal-join node ORDER reference model (D-136 → the port). N rules
(TJ0..) sharing ONE temporal-join LHS `$a:E0() $b:E1(op[0,hi] $a)`; the join node
is SHARED, so the engine bails the D-125 per-arrival flush to legacy pop-time and
orders wrong. `simulate()` is the candidate spec; the harness differs it vs the
oracle over a shuffled population with an optional 2nd epoch.

--- the composition (D-136, VALIDATED 0-div / 6 seeds / 1800 cases) ---
The single node's tuple batch per fire cycle = the D-125 flush order
(`model_join_flush.Node`, 100% proven). Composition:
  * RULE-GROUPED — the RuleExecutor drains one rule's agenda queue fully before
    the next (equal salience ⇒ declaration order), so each rule's batch fires
    contiguously; a fire cycle = one fireAllRules = one epoch.
  * PEER REVERSAL — the FIRST sink (TJ0) fires the batch FORWARD (D-125 order);
    every PEER sink (TJ1, TJ2, …) fires it REVERSED. This is the D-071/D-102
    peer-copy discipline (SegmentPropagator prepends ⇒ peers are LIFO). It is
    the whole "interleaved" residual the D-136 recon flagged — NOT a deeper
    agenda-pop composition.
So per epoch: TJ0 forward, peers reversed, over that epoch's NEW D-125 tuples.

Faithfulness bar: ZERO model-vs-oracle divergences on the shuffled population.
"""
import json
import os
import random
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from model_join_flush import Node  # validated D-125 single-rule join order

REPO = "/home/bryan/rust-rules"


def etype(n):
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": 100000}}


def gen(rng, name):
    op = rng.choice(["after", "before"])
    hi = rng.choice([50, 100, 150])
    nrules = rng.choice([2, 2, 3])
    pat = f"$a : E0() $b : E1(this {op}[0ms,{hi}ms] $a)"
    drl = "".join(f"rule TJ{i} when {pat} then end\n" for i in range(nrules))
    facts = []
    a_ts = rng.sample(range(0, 6), rng.choice([1, 2, 2]))
    facts += [("E0", t) for t in a_ts]
    base = a_ts[0]
    tss, used = [], set()
    for _ in range(rng.randint(1, 3)):
        for _try in range(20):
            d = rng.randint(0, hi + 10)
            ts = base + d if op == "after" else base - d
            if ts not in used:
                used.add(ts)
                tss.append(ts)
                break
    facts += [("E1", t) for t in tss]
    rng.shuffle(facts)
    ep = []
    if rng.random() < 0.5 and len(facts) > 2:
        k = rng.randint(1, len(facts) - 1)
        head, tail = facts[:k], facts[k:]
        facts = head
        ep = [{"actions": [], "facts": [{"type": t, "fields": {"ts": v}} for t, v in tail]}]
    return {"name": name, "types": [etype("E0"), etype("E1")], "drl": drl,
            "facts": [{"type": t, "fields": {"ts": v}} for t, v in facts], "epochs": ep}


def _tuple_render(t):
    # firings are (E0_ts, E1_ts); render as the sorted (type,ts) tuple the diff uses
    return tuple(sorted((("E0", t[0]), ("E1", t[1]))))


def simulate(scn):
    m = re.search(r'this (after|before)\[(\d+)ms,(\d+)ms\] \$a', scn["drl"])
    op, hi = m.group(1), int(m.group(3))
    nrules = scn["drl"].count("rule TJ")
    node = Node(op, 0, hi)

    # per epoch: feed the epoch's facts, snapshot the NEW D-125 firings
    epoch_fact_lists = [scn["facts"]] + [ep.get("facts", []) for ep in scn.get("epochs", [])]
    seen = 0
    firings = []
    for efacts in epoch_fact_lists:
        for f in efacts:
            if f["type"] == "E0":
                node.left_insert([f["fields"]["ts"]])
            elif f["type"] == "E1":
                node.right_insert(f["fields"]["ts"])
        new = [list(t) for t in node.firings[seen:]]
        seen = len(node.firings)
        # RULE-GROUPED within this fire cycle (RuleExecutor drains each rule's
        # queue fully, decl order). The FIRST sink (TJ0) gets the batch FORWARD
        # (D-125 order); every PEER sink (TJ1..) gets it REVERSED — the D-071/
        # D-102 peer-copy discipline (SegmentPropagator prepends ⇒ LIFO).
        for i in range(nrules):
            seq = new if i == 0 else list(reversed(new))
            for t in seq:
                firings.append((f"TJ{i}", _tuple_render(t)))
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
            (fr["rule"], tuple(sorted((m["type"], m["fields"]["ts"]) for m in fr["matches"])))
            for fr in res["firings"]]
    return out


def fuzz(n, seed):
    rng = random.Random(seed)
    OUT = f"/tmp/model_shared_tjo{seed}"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = order = setm = 0
    diffs = []
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 150, n)):
            scn = gen(rng, f"ms{seed}x{i}")
            p = os.path.join(OUT, f"ms{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = _oracle(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            pred = simulate(scns[nm])
            o = ora.get(nm)
            if pred != o:
                ndiff += 1
                import collections
                kind = "ORDER" if o is not None and collections.Counter(pred) == collections.Counter(o) else "SET"
                if kind == "ORDER":
                    order += 1
                else:
                    setm += 1
                if len(diffs) < 8:
                    diffs.append(nm)
                    print(f"  {kind} {nm}: model={[(r,list(t)) for r,t in pred]}")
                    print(f"        oracle={[(r,list(t)) for r,t in o] if o else o}")
                    print(f"        drl_rules={scns[nm]['drl'].count('rule')} epochs={bool(scns[nm]['epochs'])} "
                          f"facts={[(f['type'],f['fields']['ts']) for f in scns[nm]['facts']]}")
        done += len(paths)
    print(f"shared-tjo model-vs-oracle: {n} seed {seed}: {ndiff} div ({100*ndiff//max(1,n)}%) "
          f"[ORDER {order}, SET {setm}]")


if __name__ == "__main__":
    fuzz(int(sys.argv[2]), int(sys.argv[3])) if sys.argv[1] == "fuzz" else None
