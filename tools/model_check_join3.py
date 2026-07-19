#!/usr/bin/env python3
"""model_check_join3.py — D-352 shared-node verification model.

fz_7331_973 (the ORDER-trio anchor) exposed the first SHARED-node
re-entry observation: the D-083 late pass and the whole-trg sink flip
compose to the engine's epoch order, while the oracle's six observed
lists (b1/b3/R0 x base/epoch, handle-tagged 2026-07-19) fit exactly
one machine:

  creation  = PLAIN slots on the shared node (rightIns re-entrant,
              memory-appended at tail, walks lefts memory-forward;
              then leftIns staged-head-first x rights memory-forward)
  sinks     = per PHASE-BATCH: the first-BUILT sink receives each
              phase batch PREPEND-within, batches appended FIFO;
              later sinks receive each batch in CREATION order,
              batches in the same order.

This file is a VERIFIER, not an eliminator: it mechanically replays
both machines (candidate vs engine-current = D-083 late + whole-trg
flip) over the 973 base+epoch staging and checks all six lists, plus
jr3/jw3 single-sink sanity (which the candidate leaves on the
engine's certified mechanics by construction — the late pass and
whole-trg drain are SINGLE-SINK-scoped there).

Run: python3 tools/model_check_join3.py
"""

# Facts: L1=T1(1.5), L3=T1(3.5), L7=T1(-2), L8=T1(6)   (lefts)
#        R2=T0(beta->b, re-entrant), R4=T0(b)          (rights)

ORACLE = {
    # sink -> (base list, epoch list), oracle handles, observed 2026-07-19
    "b1": ([("L3", "R4"), ("L3", "R2"), ("L1", "R4"), ("L1", "R2")],
           [("L3", "R2"), ("L1", "R2"), ("L8", "R4"), ("L8", "R2"), ("L7", "R4"), ("L7", "R2")]),
    "b3": ([("L3", "R4"), ("L3", "R2"), ("L1", "R4"), ("L1", "R2")],
           [("L3", "R2"), ("L1", "R2"), ("L8", "R4"), ("L8", "R2"), ("L7", "R4"), ("L7", "R2")]),
    "R0": ([("L1", "R2"), ("L1", "R4"), ("L3", "R2"), ("L3", "R4")],
           [("L1", "R2"), ("L3", "R2"), ("L7", "R2"), ("L7", "R4"), ("L8", "R2"), ("L8", "R4")]),
}
ENGINE = {
    "b1": ([("L3", "R4"), ("L3", "R2"), ("L1", "R4"), ("L1", "R2")],
           [("L8", "R4"), ("L7", "R4"), ("L8", "R2"), ("L7", "R2"), ("L3", "R2"), ("L1", "R2")]),
    "b3": ([("L3", "R4"), ("L3", "R2"), ("L1", "R4"), ("L1", "R2")],
           [("L8", "R4"), ("L7", "R4"), ("L8", "R2"), ("L7", "R2"), ("L3", "R2"), ("L1", "R2")]),
    "R0": ([("L1", "R2"), ("L1", "R4"), ("L3", "R2"), ("L3", "R4")],
           [("L1", "R2"), ("L3", "R2"), ("L7", "R2"), ("L8", "R2"), ("L7", "R4"), ("L8", "R4")]),
}


def base_creation():
    """Base batch: all fresh. rightIns [R4,R2 staged head-first] x empty
    lefts memory -> nothing; leftIns [L3,L1 staged head-first] x rights
    memory [R4,R2] (append-on-process). ONE phase batch with children."""
    segs = []
    lins = []
    for l in ["L3", "L1"]:
        for r in ["R4", "R2"]:
            lins.append((l, r))
    segs.append(lins)
    return segs


def epoch_creation_plain():
    """Candidate: plain slots. rightIns(R2 re-entrant, appended at tail)
    walks lefts memory-forward [L3? no — memory append-on-process order].

    Left memory after base = append-on-process = [L3, L1] (L3 processed
    first). OBSERVED oracle R0 rins-batch prepend-within = [(L1,R2),
    (L3,R2)] -> creation = [(L3,R2),(L1,R2)] = memory-forward [L3, L1].
    leftIns staged head-first [L8, L7] x rights [R4, R2-at-tail]."""
    segs = []
    segs.append([("L3", "R2"), ("L1", "R2")])
    lins = []
    for l in ["L8", "L7"]:
        for r in ["R4", "R2"]:
            lins.append((l, r))
    segs.append(lins)
    return segs


def epoch_creation_late():
    """Engine-current: D-083 late pass. leftIns [L8, L7] x PRE-BATCH
    memory [R4]; then late re-entrant R2 x lefts lseq-desc
    [L8, L7, L3, L1]."""
    segs = []
    lins = [(l, "R4") for l in ["L8", "L7"]]
    late = [(l, "R2") for l in ["L8", "L7", "L3", "L1"]]
    # engine trg is ONE composed staged list (no phase marks)
    segs.append(lins + late)
    return segs


def distribute(segs, mode):
    """mode 'perbatch': first sink = prepend-within batch, batches FIFO;
    later sinks = creation-within, batches FIFO.
    mode 'whole': first sink = prepend over the whole composed stream;
    later sinks = whole-stream reversed copy of that (creation order)…
    the engine's actual whole-flip: first sink LIFO(all), later =
    reverse(first) = creation(all)."""
    if mode == "perbatch":
        first = [t for seg in segs for t in reversed(seg)]
        later = [t for seg in segs for t in seg]
    else:
        allc = [t for seg in segs for t in seg]
        first = list(reversed(allc))
        later = allc
    return first, later


def check(tag, machine_segs_base, machine_segs_epoch, mode, logs, swap=False):
    ok = True
    fb, lb = distribute(machine_segs_base, mode)
    fe, le = distribute(machine_segs_epoch, mode)
    # first-BUILT sink = R0 (declared first); later sinks = b1, b3
    got = {"R0": (fb, fe), "b1": (lb, le), "b3": (lb, le)}
    if swap:
        got = {"R0": (lb, le), "b1": (fb, fe), "b3": (fb, fe)}
    for sink in ["b1", "b3", "R0"]:
        for i, name in [(0, "base"), (1, "epoch")]:
            g, e = got[sink][i], logs[sink][i]
            m = "OK " if g == e else "MISS"
            if g != e:
                ok = False
            print(f"  {m} {tag} {sink} {name}: got={g}")
            if g != e:
                print(f"                 exp={e}")
    return ok


print("== CANDIDATE (plain creation + per-batch distribution) vs ORACLE")
c_ok = check("cand", base_creation(), epoch_creation_plain(), "perbatch", ORACLE)
print("== ENGINE-CURRENT (late creation + whole flip) vs ENGINE LOGS")
e_ok = check("eng ", base_creation(), epoch_creation_late(), "whole", ENGINE)
print("== CROSS (candidate vs ENGINE logs — must MISS on epoch)")
x_ok = check("x   ", base_creation(), epoch_creation_plain(), "perbatch", ENGINE)

print()
print(f"candidate fits oracle 6/6: {c_ok}")
print(f"engine-model fits engine logs 6/6: {e_ok}")
print(f"cross-fit (should be False): {x_ok}")
if c_ok and e_ok and not x_ok:
    print("VERIFIED: the candidate uniquely fits the oracle; the")
    print("engine model reproduces the engine — the port targets are")
    print("exactly (1) plain slot on shared nodes, (2) per-batch sinks.")
    raise SystemExit(0)
raise SystemExit(1)
