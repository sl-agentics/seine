#!/usr/bin/env python3
"""Order-cluster witness registry + checker (D-196 work).

Every population case that drove a model_sd law in the order-cluster
rounds, with its regeneration recipe (seed, index) and its oracle
truth captured from the fuzz re-run files at find time. The oracle
truths are INLINED below (seq + finals) so the check needs no /tmp
state and no oracle runs — pure model regression.

Usage: python3 check_witnesses.py   (exit 0 = all witnesses + corners
match; the fresh-seed protocol runs this before any population gate)
"""
import sys
import os
import random

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(
    os.path.dirname(os.path.abspath(__file__)))), "rust-rules", "tools"))
sys.path.insert(0, "/home/bryan/rust-rules/tools")
from model_sd import simulate  # noqa: E402
from fuzz_tms_sd import draw   # noqa: E402

# (seed, idx): (oracle firing seq, oracle finals) — captured 3x-stable.
WITNESSES = {
    (7002, 68):  ([("R0",1),("R1",4),("R0",3),("R1",1),("R1",2),("R1",3)], []),
    (6003, 41):  ([("R0",1),("R3",4),("R0",3),("R3",1),("R3",2),("R3",3)], []),
    (6003, 88):  ([("R1",1),("R0",2),("R0",3),("R1",2),("R0",3),("R1",3)], []),
    (6001, 90):  ([("R3",1),("R3",2),("R3",3),("R2",3),("R2",2),("R2",1)], []),
    (7004, 67):  ([("R3",1),("R3",2),("R3",3),("R3",4),("R1",1),("R2",4),
                   ("R1",3),("R2",1),("R2",2),("R2",3)], []),
    (7004, 108): ([("R1",1),("R0",4),("R1",2),("R0",1),("R0",2),("R0",3)], []),
    (7004, 131): ([("R0",1),("R0",2),("R2",1),("R2",2),("R1",1),("R1",2)], []),
    (7001, 103): ([("R0",1),("R1",1),("R1",2),("R1",3),("R0",2),("R1",2),
                   ("R1",1),("R1",3),("R0",3),("R1",3),("R1",2),("R1",1),
                   ("R2",1),("R2",2),("R2",3)], []),
    (6003, 0):   ([("R3",1),("R0",None),("R3",2),("R0",None),("R3",3),
                   ("R0",None)], []),
    (7004, 51):  ([("R1",1),("R1",2),("R1",3),("R1",4),("R0",1),("R0",2),
                   ("R0",3),("R0",4)], []),
    (6001, 66):  ([("R0",1),("R0",2),("R2",1),("R2",2)], []),
    (7006, 34):  ([("R1",1),("R1",2),("R1",3),("R2",1),("R2",2),("R2",3)], []),
    (7002, 56):  ([("R1",1),("R0",2),("R0",1)], []),
    (7007, 79):  ([("R2",1),("R2",2),("R1",None)], []),
    (7007, 98):  ([("R1",1),("R1",2),("R1",3),("R1",4),("R0",None)], []),
    (7002, 119): ([("R1",1),("R0",1),("R0",2),("R0",3)], []),
    (6003, 30):  ([("R1",1),("R0",1),("R0",2),("R0",3)], []),
    (7005, 51):  ([("R1",1),("R0",1),("R0",2),("R0",3)], []),
    (7007, 10):  ([("R0",1),("R2",1),("R2",2),("R3",1),("R3",2)], []),
    (7008, 11):  ([("R0",1),("R1",1),("R1",2),("R1",3),("R1",4)], []),
    (7008, 32):  ([("R0",1),("R1",1),("R1",2)], []),
    (7009, 147): ([("R0",1),("R0",2),("R2",1),("R2",2)], []),
    (6001, 131): ([("R1",1),("R1",2),("R1",3),("R1",4),("R2",None),
                   ("R3",1),("R3",2),("R3",3),("R3",4)],
                  [("P",1),("P",2),("P",3),("P",4)]),
    (7004, 92):  ([("R0",1),("R0",2),("R0",3),("R1",3),("R1",2),("R1",1)],
                  [("P",1),("P",2),("P",3)]),
}

# The 2x2 decl-axis corners (gt20a/gt20b; oracle 3x 2026-07-12).
def _J(name, sal):
    return {"kind": "justifier", "name": name, "sal": sal, "k": 1,
            "notpos": "trail", "eager": True, "ortwin": False,
            "breaks": True, "amut": "set_break", "mutfirst": False}


def _DN(name, sal): return {"kind": "del_not", "name": name, "sal": sal}
def _OL(name, sal): return {"kind": "obs_lk", "name": name, "sal": sal}
def _OP(name, sal): return {"kind": "obs_p", "name": name, "sal": sal}


CORNERS = {
    "gt20a_declaxis_jfirst": ([1, 2], [_J("R0", 10), _DN("R1", 0)],
                              [("R0", 1), ("R1", 1), ("R1", 2)]),
    "gt20b_declaxis_dfirst": ([1, 2], [_DN("R0", -10), _J("R1", 0),
                                       _OL("R2", 0), _OP("R3", -1)],
                              [("R1", 1), ("R3", 1), ("R3", 2),
                               ("R0", 2), ("R0", 1)]),
}


def main():
    cache = {}
    n_ok = 0
    total = 0
    for (seed, idx), (oseq, ofin) in sorted(WITNESSES.items()):
        total += 1
        if seed not in cache:
            rng = random.Random(seed)
            cache[seed] = [draw(rng) for _ in range(150)]
        facts, rules = cache[seed][idx]
        r = simulate(facts, rules)
        ok = (not r["runaway"] and [tuple(x) for x in r["firings"]] == oseq
              and sorted(r["finals"] or []) == sorted(ofin))
        n_ok += ok
        if not ok:
            print(f"DIVERGES x{seed}x{idx}: model={r['firings']} oracle={oseq}")
    for name, (facts, rules, oseq) in CORNERS.items():
        total += 1
        r = simulate(facts, rules)
        ok = not r["runaway"] and [tuple(x) for x in r["firings"]] == oseq
        n_ok += ok
        if not ok:
            print(f"DIVERGES {name}: model={r['firings']} oracle={oseq}")
    print(f"--- {n_ok}/{total} witnesses")
    return 0 if n_ok == total else 1


if __name__ == "__main__":
    sys.exit(main())
