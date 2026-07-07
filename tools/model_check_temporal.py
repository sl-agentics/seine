#!/usr/bin/env python3
"""D-101 t-ladder model check: enumerate temporal-join composition
models against ALL oracle pins; report survivors.

A scenario = list of fires; each fire = ordered arrivals
[(role, ts)] where role in {A (anchor/left), B (prober/right),
AB (self-join both)}. Constraint: delta(b, a) in [lo, hi] with
delta = b-a (after) or a-b (before).

Model dimensions:
  proc_order: staged facts within a fire process 'newest' | 'arrival'
  role_split: 'rights_then_lefts' | 'lefts_then_rights' | 'global'
              (global = one pass in proc_order; each fact fills+joins)
  self_fill:  for global/AB — fill own left before right-join? T/F
  lscan:      anchor-partner scan 'arrival' | 'ts_asc' | 'ts_desc' | 'memory'
  rscan:      prober-partner scan (left-phase) same options
  left_joins: fresh lefts join pre-batch right memory 'pre' | 'full' | 'none'
  held:       held staged rights process 'arrival_before_fresh' |
              'arrival_after_fresh' | 'newest_with_fresh'
Firing order = reverse of creation order per fire (certified prepend).
"""
import itertools
import sys

AFTER, BEFORE = "after", "before"


def eligible(op, lo, hi, a_ts, b_ts):
    d = b_ts - a_ts if op == AFTER else a_ts - b_ts
    return lo <= d <= hi


def run_model(cfg, fires, op, lo, hi):
    r_order, l_order, fill_t, first, lscan, left_joins, held_mode, drain = cfg
    lefts = []   # (ts, arrival_seq)
    rights = []
    seq = 0
    held_rights = []  # staged, not yet processed (never-linked holds)
    out_all = []

    def scan(items, mode):
        if mode == "arrival":
            return sorted(items, key=lambda x: x[1])
        if mode == "ts_asc":
            return sorted(items, key=lambda x: (x[0], x[1]))
        if mode == "ts_desc":
            return sorted(items, key=lambda x: (-x[0], x[1]))
        return list(items)  # memory order

    for fire in fires:
        creations = []
        batch = []
        for role, ts in fire:
            batch.append((role, ts, seq))
            seq += 1
        linked = bool(lefts) or any(r in ("A", "AB") for r, _, _ in batch)
        if not linked:
            held_rights.extend((ts, s) for r, ts, s in batch if r in ("B", "AB"))
            out_all.append([])
            continue
        arr = batch
        # sequential arrival staging with drain-at-link (dim: drain)
        b_lefts = []
        b_rights = []
        for r, ts, s in arr:
            if r in ("A", "AB"):
                if drain == "at_link" and not b_lefts and not lefts:
                    # the LINK moment: pre-link staged rights (held
                    # from earlier fires AND same-batch pre-anchor)
                    # drain to memory in arrival order, no children
                    pre = sorted(held_rights + b_rights, key=lambda x: x[1])
                    rights.extend(pre)
                    held_rights = []
                    b_rights = []
                b_lefts.append((ts, s))
            if r in ("B", "AB"):
                b_rights.append((ts, s))
        if l_order == "newest":
            b_lefts = list(reversed(b_lefts))
        if r_order == "newest":
            b_rights = list(reversed(b_rights))
        hr = sorted(held_rights, key=lambda x: x[1])
        held_rights = []
        seqr = (hr + b_rights) if held_mode == "arrival_before_fresh" else (b_rights + hr)

        pre_rights = list(rights)

        def left_join(l):
            if left_joins == "none":
                return
            mem = rights if left_joins == "full" else pre_rights
            for b in scan(mem, "memory"):
                if eligible(op, lo, hi, l[0], b[0]):
                    creations.append((l[0], b[0]))

        def right_join(b):
            for a in scan(lefts, lscan):
                if eligible(op, lo, hi, a[0], b[0]):
                    creations.append((a[0], b[0]))

        if fill_t == "fills_first":
            for l in b_lefts:
                lefts.append(l)
            passes = []
            if first == "lefts_first":
                passes = [("L", b_lefts), ("R", seqr)]
            else:
                passes = [("R", seqr), ("L", b_lefts)]
            for kind, items in passes:
                for x in items:
                    if kind == "L":
                        left_join(x)
                    else:
                        rights.append(x)
                        right_join(x)
        else:  # with_pass: fill as each side's pass processes it
            if first == "lefts_first":
                for l in b_lefts:
                    lefts.append(l)
                    left_join(l)
                for b in seqr:
                    rights.append(b)
                    right_join(b)
            else:
                for b in seqr:
                    rights.append(b)
                    right_join(b)
                for l in b_lefts:
                    lefts.append(l)
                    left_join(l)
        out_all.append(list(reversed(creations)))
    return out_all


PINS = [
    # (name, op, lo, hi, fires, expected firings per fire)
    ("min_sj", AFTER, 0, 100,
     [[("AB", 6), ("AB", 40)]],
     [[(6, 6), (40, 40), (6, 40)]]),
    ("t1", AFTER, 0, 200,
     [[("A", 0), ("A", 50), ("A", 100), ("B", 100)]],
     [[(100, 100), (50, 100), (0, 100)]]),
    ("t4", AFTER, 0, 100,
     [[("A", 0), ("A", 40), ("A", 80), ("A", 120), ("B", 80)]],
     [[(80, 80), (40, 80), (0, 80)]]),
    ("t5", BEFORE, 0, 200,
     [[("A", 0), ("A", 60), ("A", 120), ("A", 250), ("B", 20)]],
     [[(120, 20), (60, 20)]]),
    ("t6", AFTER, 0, 200,
     [[("B", 100), ("B", 150)], [("A", 50)]],
     [[], [(50, 150), (50, 100)]]),
    ("t7", AFTER, 0, 200,
     [[("B", 100)], [("A", 50), ("B", 150)]],
     [[], [(50, 100), (50, 150)]]),
    ("t8", AFTER, 0, 200,
     [[("A", 10), ("A", 60)], [("B", 100)]],
     [[], [(60, 100), (10, 100)]]),
    ("t10", AFTER, 0, 200,
     [[("B", 100)], [("B", 150)], [("A", 50)]],
     [[], [], [(50, 150), (50, 100)]]),
    ("t13", AFTER, 0, 200,
     [[("B", 100)], [("A", 50)], [("B", 150)]],
     [[], [(50, 100)], [(50, 150)]]),
    ("t14", AFTER, 0, 500,
     [[("B", 150), ("B", 100), ("A", 10)], [("A", 50)]],
     [[(10, 100), (10, 150)], [(50, 100), (50, 150)]]),
    ("t15", AFTER, 0, 500,
     [[("A", 60), ("A", 20), ("B", 100)]],
     [[(20, 100), (60, 100)]]),
    ("cf56", BEFORE, 0, 100,
     [[("AB", 31), ("AB", 4)]],
     [[(31, 31), (4, 4), (31, 4)]]),
]


def main():
    dims = [
        ["newest", "arrival"],          # right-pass order over fresh
        ["newest", "arrival"],          # left-pass order (fills+joins)
        ["fills_first", "with_pass"],   # left fills before the right pass?
        ["lefts_first", "rights_first"],# which JOIN pass runs first
        ["arrival", "memory"],          # anchor scan in the right pass
        ["pre", "full", "none"],        # left-join memory visibility
        ["arrival_before_fresh", "arrival_after_fresh"],  # held rights
        ["none", "at_link"],            # pre-link right drain (D-094 lineage)
    ]
    survivors = []
    for cfg in itertools.product(*dims):
        ok = True
        for name, op, lo, hi, fires, want in PINS:
            got = run_model(cfg, fires, op, lo, hi)
            if got != want:
                ok = False
                break
        if ok:
            survivors.append(cfg)
    print(f"{len(survivors)} survivor(s)")
    for s in survivors[:12]:
        print("  r=%s l=%s fill=%s first=%s lscan=%s ljoin=%s held=%s drain=%s" % s)
    if not survivors:
        # near-misses: fail on exactly one pin
        for cfg in itertools.product(*dims):
            bad = [n for n, op, lo, hi, f, w in PINS if run_model(cfg, f, op, lo, hi) != w]
            if len(bad) == 1:
                print("  near-miss:", cfg, "fails", bad[0])


if __name__ == "__main__":
    main()
