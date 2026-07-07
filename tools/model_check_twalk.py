#!/usr/bin/env python3
"""D-102 cycle 4: temporal-walk MICRO-ORDER model check.

Models ONE pop evaluation of a temporal join node (the pop path only —
flush-path pins like t14 guard separately) and the terminal consume
per sharing rule. Pins carry measured firing orders for BOTH rules
where shared (sink0 = decl-first rule, peer = the other).

State entering the pop:
  mem_lefts:  [(ts, seq)] already in left memory (lseq order = seq)
  mem_rights: [(ts, seq)] already in right memory (push order)
  st_lefts:   [(ts, seq)] staged lefts, arrival order (seq)
  st_rights:  [(ts, seq)] staged rights, arrival order (seq)

Walk (certified skeleton, D-101): staged lefts FILL first (lseq
stamped); rightIns processes staged rights head-first (newest) x
partner lefts (scan dim); leftIns processes staged lefts (iter dim)
x pre-batch right memory (scan dim).

Dims:
  pscan:  rightIns partner order over ALL lefts (mem lseq + fresh):
          asc | desc | fresh_desc_mem_asc | fresh_asc_mem_desc
  rscan:  leftIns x pre_rights memory order: push | reverse
  liter:  leftIns iteration: head (staged LIFO) | arrival
  c_sink0: fwd | rev   (consume for the decl-first rule)
  c_peer:  fwd | rev   (consume for sharing peers)
  c_single: fwd | rev  (consume when the node is unshared)
"""
import itertools

AFTER, BEFORE = "after", "before"


def eligible(op, lo, hi, a_ts, b_ts):
    d = b_ts - a_ts if op == AFTER else a_ts - b_ts
    return lo <= d <= hi


def walk(cfg, op, lo, hi, mem_lefts, mem_rights, st_lefts, st_rights):
    pscan, rscan, liter, _, _, _ = cfg
    creations = []
    lefts = [(e[0], e[1], e[2] if len(e) > 2 else "mem") for e in mem_lefts]
    # fill staged lefts (arrival), stamped after memory
    fills = [(ts, seq, "fresh") for ts, seq in st_lefts]
    all_lefts = lefts + fills
    # rightIns: staged rights HEAD-first (newest = highest seq first)
    for bts, _bseq in sorted(st_rights, key=lambda x: -x[1]):
        if pscan == "asc":
            part = sorted(all_lefts, key=lambda x: x[1])
        elif pscan == "desc":
            part = sorted(all_lefts, key=lambda x: -x[1])
        elif pscan == "fresh_desc_mem_asc":
            part = sorted([l for l in all_lefts if l[2] != "mem"], key=lambda x: -x[1]) + \
                   sorted([l for l in all_lefts if l[2] == "mem"], key=lambda x: x[1])
        else:  # fresh_asc_mem_desc — fresh = THIS FIRE (incl. flush-filled)
            part = sorted([l for l in all_lefts if l[2] != "mem"], key=lambda x: x[1]) + \
                   sorted([l for l in all_lefts if l[2] == "mem"], key=lambda x: -x[1])
        for ats, _, _ in part:
            if eligible(op, lo, hi, ats, bts):
                creations.append((ats, bts))
    # leftIns x pre-batch right memory
    lefts_iter = sorted(st_lefts, key=lambda x: -x[1]) if liter == "head" \
        else sorted(st_lefts, key=lambda x: x[1])
    pre_r = list(mem_rights) if rscan == "push" else list(reversed(mem_rights))
    for ats, _ in lefts_iter:
        for bts, _ in pre_r:
            if eligible(op, lo, hi, ats, bts):
                creations.append((ats, bts))
    return creations


def consume(creations, mode):
    return list(creations) if mode == "fwd" else list(reversed(creations))


# (name, op, lo, hi, mem_lefts, mem_rights, st_lefts, st_rights,
#  shared?, {role: firings})  role in sink0|peer|single
PINS = [
    # min_sj: AB@6 flush-filled left (same fire) + right-6 consumed to
    # memory at the flush; pop: fills AB@40; staged right 40.
    # pop creations must be [(6,40),(40,40)] (flush window (6,6) is
    # outside this model). single REV -> [(40,40),(6,40)].
    ("min_sj_pop", AFTER, 0, 100, [(6, 1, "fire")], [(6, 2)], [(40, 3)], [(40, 4)],
     False, {"single": [(40, 40), (6, 40)]}),
    # cf56: BEFORE self-join, AB@31 then AB@4 same batch; @31's flush
    # fills left-31 + right-31 memory (link at @31); pop fills 4.
    # pin firings [(31,31),(4,4),(31,4)]; flush window (31,31) outside
    # -> pop must yield [(4,4),(31,4)] after REV => creations [(31,4),(4,4)].
    ("cf56_pop", BEFORE, 0, 100, [(31, 1, "fire")], [(31, 2)], [(4, 3)], [(4, 4)],
     False, {"single": [(4, 4), (31, 4)]}),
    # t1: prologue all-fresh: A0,A50,A100 staged lefts; B100 staged right.
    # single rule; firings (100,100),(50,100),(0,100)
    ("t1", AFTER, 0, 200, [], [], [(0, 1), (50, 2), (100, 3)], [(100, 4)],
     False, {"single": [(100, 100), (50, 100), (0, 100)]}),
    # t15: A60, A20 staged lefts; B100 right. firings (20,100),(60,100)
    ("t15", AFTER, 0, 500, [], [], [(60, 1), (20, 2)], [(100, 3)],
     False, {"single": [(20, 100), (60, 100)]}),
    # 551: mem_rights [31,26] (self-drained, 31 pushed first);
    # staged lefts E1@27(seq3), E1@7(seq4). No staged rights.
    # shared: TJ0 (sink0) [(27,31),(7,26),(7,31)]; TJ1 (peer) [(7,31),(7,26),(27,31)]
    ("cf551", AFTER, 0, 100, [], [(31, 1), (26, 2)], [(27, 3), (7, 4)], [],
     True, {"sink0": [(27, 31), (7, 26), (7, 31)],
            "peer": [(7, 31), (7, 26), (27, 31)]}),
    # 526 fire2: mem_lefts E0@4(1),E0@24(2),E0@38(3) (lseq from fire1);
    # staged left E0@67(seq5); staged right E1@80(seq4).
    # eligible: (38,80),(67,80).
    # TJ0 (sink0) [(38,80),(67,80)]; TJ1 (peer) [(67,80),(38,80)]
    ("cf526", AFTER, 0, 50, [(4, 1), (24, 2), (38, 3)], [], [(67, 5)], [(80, 4)],
     True, {"sink0": [(38, 80), (67, 80)],
            "peer": [(67, 80), (38, 80)]}),
    # 616 fire1: self-join AB: staged lefts 22(1),24(3); staged rights 22(2),24(4)
    # (each event stages both sides; left before right per insert).
    # after[0,150]. TJ0 (sink0, salience9-evaluated... consume role by DECL)
    # TJ0 [(22,22),(24,24),(22,24)]; TJ1 [(22,24),(24,24),(22,22)]
    ("cf616", AFTER, 0, 150, [], [], [(22, 1), (24, 3)], [(22, 2), (24, 4)],
     True, {"sink0": [(22, 22), (24, 24), (22, 24)],
            "peer": [(22, 24), (24, 24), (22, 22)]}),
    # 134 fire3: mem_lefts [1209(1)]; mem_rights [1209(1)];
    # staged left 1257(2); staged right 1257(3). before[0,150].
    # TJ0 [(1257,1209),(1257,1257)]; TJ1 [(1257,1257),(1257,1209)]
    ("cf134", BEFORE, 0, 150, [(1209, 1)], [(1209, 1)], [(1257, 2)], [(1257, 3)],
     True, {"sink0": [(1257, 1209), (1257, 1257)],
            "peer": [(1257, 1257), (1257, 1209)]}),
]


def main():
    dims = [
        ["asc", "desc", "fresh_desc_mem_asc", "fresh_asc_mem_desc"],  # pscan
        ["push", "reverse"],   # rscan
        ["head", "arrival"],   # liter
        ["fwd", "rev"],        # c_sink0
        ["fwd", "rev"],        # c_peer
        ["fwd", "rev"],        # c_single
    ]
    survivors = []
    for cfg in itertools.product(*dims):
        ok = True
        for name, op, lo, hi, ml, mr, sl, sr, shared, want in PINS:
            cre = walk(cfg, op, lo, hi, ml, mr, sl, sr)
            for role, w in want.items():
                mode = {"sink0": cfg[3], "peer": cfg[4], "single": cfg[5]}[role]
                if consume(cre, mode) != w:
                    ok = False
                    break
            if not ok:
                break
        if ok:
            survivors.append(cfg)
    print(f"{len(survivors)} survivor(s)")
    for s in survivors[:12]:
        print("  pscan=%s rscan=%s liter=%s sink0=%s peer=%s single=%s" % s)
    if not survivors:
        best = []
        for cfg in itertools.product(*dims):
            bad = []
            for name, op, lo, hi, ml, mr, sl, sr, shared, want in PINS:
                cre = walk(cfg, op, lo, hi, ml, mr, sl, sr)
                for role, w in want.items():
                    mode = {"sink0": cfg[3], "peer": cfg[4], "single": cfg[5]}[role]
                    if consume(cre, mode) != w:
                        bad.append(f"{name}:{role}")
            best.append((len(bad), cfg, bad))
        best.sort(key=lambda x: x[0])
        for n, cfg, bad in best[:6]:
            print(n, cfg, bad)


if __name__ == "__main__":
    main()
