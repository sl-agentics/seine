#!/usr/bin/env python3
"""Two-node temporal CHAIN model check.

Reuses the single-node temporal machine (model_check_temporal.run_model)
per node, and adds ONE free dimension: the inter-node HANDOFF order of a
fresh child batch produced by node1 as it propagates to node2.

  handoff = 'creation'  -> node2 receives children in node1 creation order
            'drain'     -> node2 receives them reversed (staged/prepend
                           order = the terminal drain order)

Chain rule: $a:E0() $b:E1(after[0,50] $a) $c:E2(after[0,100] $b)
  node1: E0(left) x E1(right)  -> tuple carries b_ts = E1.ts
  node2: (E0,E1)(left) x E2(right)

We validate the SAME per-node cfg (one of the 6 single-node survivors)
reproduces every two-node oracle firing when handoff='drain'.
"""
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__)))
# import the certified single-node machine
sys.path.insert(0, "/home/bryan/rust-rules/tools")
from model_check_temporal import run_model, AFTER, BEFORE  # noqa

# The 6 single-node survivors (from `python3 tools/model_check_temporal.py`)
SURVIVORS = [
    ("newest", "newest", "fills_first", "rights_first", "arrival", "pre", "arrival_before_fresh", "at_link"),
    ("newest", "newest", "fills_first", "rights_first", "arrival", "pre", "arrival_after_fresh", "at_link"),
    ("newest", "arrival", "fills_first", "rights_first", "arrival", "pre", "arrival_before_fresh", "at_link"),
    ("newest", "arrival", "fills_first", "rights_first", "arrival", "pre", "arrival_after_fresh", "at_link"),
    ("newest", "arrival", "fills_first", "rights_first", "memory", "pre", "arrival_before_fresh", "at_link"),
    ("newest", "arrival", "fills_first", "rights_first", "memory", "pre", "arrival_after_fresh", "at_link"),
]


def node_creations(cfg, fire, op, lo, hi):
    """Return per-fire creation lists (un-reversed) = reverse of the
    model's firing output."""
    fired = run_model(cfg, [fire], op, lo, hi)[0]
    return list(reversed(fired))  # undo the terminal prepend-reverse


def chain(cfg, handoff, n1_e1s, n2_e2s,
          op1=AFTER, lo1=0, hi1=50, op2=AFTER, lo2=0, hi2=100, e0=1):
    """n1_e1s: E1 timestamps in INSERTION order (rights at node1).
       n2_e2s: E2 timestamps in INSERTION order (rights at node2).
    Returns the final firing sequence as list of (e1_ts, e2_ts)."""
    # node1 fire: rights (E1s) then the single left (E0), all one batch
    n1_fire = [("B", t) for t in n1_e1s] + [("A", e0)]
    c1 = node_creations(cfg, n1_fire, op1, lo1, hi1)  # list of (e0, e1)
    child_e1_order = [b for (a, b) in c1]              # creation order
    if handoff == "drain":
        child_e1_order = list(reversed(child_e1_order))
    # node2 fire: the batch of lefts (ts = e1) arrive, then E2 rights
    n2_fire = [("A", e1) for e1 in child_e1_order] + [("B", t) for t in n2_e2s]
    fired2 = run_model(cfg, [n2_fire], op2, lo2, hi2)[0]  # list of (e1, e2)
    return fired2


# Two-node oracle pins: (name, e1_insertion, e2_insertion, expected final
# as list of (e1,e2)). Derived from the oracle battery run.
PINS = [
    # Group A core + perms: final = INSERTION order of E1, one E2@110
    ("A_26_23_25", [26, 23, 25], [110], [(26, 110), (23, 110), (25, 110)]),
    ("A_23_25_26", [23, 25, 26], [110], [(23, 110), (25, 110), (26, 110)]),
    ("A_25_26_23", [25, 26, 23], [110], [(25, 110), (26, 110), (23, 110)]),
    ("A_wide", [10, 30, 50], [110], [(10, 110), (30, 110), (50, 110)]),
    ("A_wide2", [30, 10, 50], [110], [(30, 110), (10, 110), (50, 110)]),
    # Group B: 2 E1
    ("B_20_40", [20, 40], [110], [(20, 110), (40, 110)]),
    ("B_40_20", [40, 20], [110], [(40, 110), (20, 110)]),
    # C_e0last
    ("C_e0last", [26, 23, 25], [110], [(26, 110), (23, 110), (25, 110)]),
    # D_3x2: two E2, each fires the 3 lefts in insertion order; E2@110 first
    ("D_3x2", [26, 23, 25], [110, 120],
     [(26, 110), (23, 110), (25, 110), (26, 120), (23, 120), (25, 120)]),
    ("D_3x2b", [26, 23, 25], [120, 110],
     [(26, 120), (23, 120), (25, 120), (26, 110), (23, 110), (25, 110)]),
    # F: single E1 (no batch) -> trivially insertion
    ("F_1e1", [26], [110], [(26, 110)]),
]


def main():
    print("handoff=DRAIN:")
    any_full = False
    for cfg in SURVIVORS:
        bad = []
        for name, e1s, e2s, want in PINS:
            got = chain(cfg, "drain", e1s, e2s)
            if got != want:
                bad.append((name, got, want))
        tag = "ALL-PASS" if not bad else f"{len(bad)} fail"
        print(f"  cfg lscan={cfg[4]} l={cfg[1]} held={cfg[6]}: {tag}")
        if not bad:
            any_full = True
        else:
            for nm, got, want in bad[:3]:
                print(f"      {nm}: got {[b for a,b in got]} want {[b for a,b in want]}"
                      if all(len(x)==2 for x in got) else f"      {nm}: got {got} want {want}")
    print("\nhandoff=CREATION (should match the BUGGY engine, not oracle):")
    for cfg in SURVIVORS[:1]:
        for name, e1s, e2s, want in PINS:
            got = chain(cfg, "creation", e1s, e2s)
            mark = "ok" if got == want else "DIFF"
            print(f"  {name}: {mark} got_e1={[b for a,b in got]}")
    print("\nDRAIN reproduces oracle for at least one cfg:", any_full)


if __name__ == "__main__":
    main()
