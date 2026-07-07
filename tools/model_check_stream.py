#!/usr/bin/env python3
"""D-102 checker cycle: enumerate STREAM-mode composition models against
ALL join-level pins (t-ladder temporal shapes + u/v-ladder CE-relink
plain shapes + min_sj/cf56 self-joins + cf5x18).

Scenario model: fires = list of fires; each fire = list of STEPS in
action order. Steps:
  ("ins", role, ts)   — a fact insert (flushes per the flush dim)
  ("adv", [ts...])    — advance: the listed B/right facts EXPIRE
                        (batch deletes staged, processed at the fire
                        evaluation; never flushes — pin a3)
Roles: "A" left/anchor, "B" right/prober, "AB" self-join both,
  "E+" CE-enabler insert (exists: IF appears / not: IF retracts),
  "E-adv" is expressed via ("adv", ...) killing enabler timestamps.
For CE shapes the join's LEFT side is the IF pseudo-tuple; E-events
toggle it. ce = None (plain temporal/join) | "exists" | "not".

Model dimensions (enumerated):
  uflush:  unlinked-path insert flush behavior:
           "noop" (staging held) | "drain" (fact -> own-side memory,
           no children) | "drain_t" (drain at TEMPORAL nodes only)
  ldrain:  drain-at-link of held RIGHTS (arrival, excl. trigger):
           "off" | "all" | "nonflush" (only advance/non-insert links)
           — applied per node kind via ldrain_plain/ldrain_temp
  lorder:  staged-left iteration at the fire walk: "head" | "arrival"
  wjoin:   what a linked insert's trigger-scoped flush joins against:
           memory only (fixed — staging never cross-joins in a flush)
Walks (fixed, certified): TEMPORAL fire-walk = fills lefts, rightIns
  newest-first x lefts-arrival, leftIns(lorder) x pre-walk right
  memory. PLAIN fire-walk = rightIns newest x pre-walk left MEMORY,
  leftIns(lorder... certified is head-first) x full right memory.
Windows: each flush = a window; the fire evaluation = final window;
  firings = concat(reversed(creations_w)).
IF-tuple: exists -> present iff any enabler alive; not -> present iff
  none. Toggling stages an IF leftIns/leftDel; insert-triggered
  toggles ride the trigger's flush; advance-triggered toggles
  materialize at the fire walk. IF retract kills children.
"""
import itertools

def eligible(op, lo, hi, a, b):
    if op is None:
        return True
    d = b - a if op == "after" else a - b
    return lo <= d <= hi


class Node:
    def __init__(self, temporal):
        self.temporal = temporal
        self.lmem = []   # (ts_or_IF, seq)
        self.rmem = []
        self.sl = []     # staged lefts (prepend)
        self.sr = []     # staged rights (prepend)
        self.seq = 0

    def stamp(self):
        s = self.seq
        self.seq += 1
        return s


def run(cfg, ce, op, lo, hi, fires, shared=False):
    """Cycle-4 round 2: simulate the LANDED flush semantics (fixed) +
    the temporal-walk micro-order dims (enumerated). Entry states
    DERIVE from the simulation — nothing is hand-encoded.

    Landed-fixed semantics: unlinked temporal deltas self-drain
    (drain_t); linked temporal flushes stash BOTH sides when the path
    was ALREADY linked pre-insert; the LINK-TRANSITION flush fills+
    pairs lefts x right memory UNLESS the node is SHARED (then stash);
    rights never flush-pair; plain-join semantics per cycle 3
    (stay/hidden/pre_lifo_then_post_arr); expiration = eager corpse
    flag + quiescence delete (post-fire removal here).

    cfg = (pscan, rscan, liter, c_sink0, c_peer)
      pscan: pop rightIns partner order over lefts:
             this_fire_arr_mem_desc | this_eval_arr_mem_desc | asc | desc
      rscan: leftIns x pre-batch right memory: push | reverse
      liter: pop leftIns iteration: head | arrival
      c_sink0/c_peer: per-window consume: fwd | rev

    Returns {"sink0": [...per-fire firings...], "peer": [...]}.
    """
    (pscan, rscan, liter, c_sink0, c_peer, if_flush, wstruct) = cfg
    temporal = ce is None
    lmem, rmem = [], []   # (ts, seq, fire_no_filled)
    sl, sr = [], []       # staged (prepend): (ts, seq, linkgen)
    enablers = {}
    expired = set()
    seq = [0]
    out = {"sink0": [], "peer": []}

    def stamp():
        seq[0] += 1
        return seq[0]

    def if_now():
        alive = any(enablers.values())
        return alive if ce == "exists" else (not alive) if ce == "not" else None

    def linked():
        if temporal:
            return bool(lmem or sl) and bool(rmem or sr)
        return bool(if_now())

    for fno, fire in enumerate(fires):
        windows = []
        pending_expire = []
        for step in fire:
            if step[0] == "adv":
                for ts in step[1]:
                    if ts in enablers:
                        enablers[ts] = False
                    expired.add(ts)          # eager corpse flag
                    pending_expire.append(ts)  # lazy quiescence delete
                continue
            _, role, ts = step
            was_if = if_now()
            if role.startswith("E"):
                enablers[ts] = True
            was_linked = linked()
            ab_id = stamp() if role == "AB" else None
            if role in ("A", "AB"):
                sl.insert(0, (ts, stamp() if ab_id is None else ab_id, fno))
            if role in ("B", "AB"):
                sr.insert(0, (ts, stamp() if ab_id is None else ab_id,
                              "pre" if not was_linked else "post"))
            if_toggled = ce is not None and if_now() and not was_if
            # ---- the FLUSH ----
            if not linked():
                if temporal:
                    # drain_t: unlinked temporal deltas self-drain
                    if role in ("B", "AB"):
                        e = sr.pop(0)
                        rmem.append((e[0], e[1], fno))
                    if role in ("A", "AB"):
                        e = sl.pop(0)
                        lmem.append((e[0], e[1], fno))
                continue
            creations = []
            if if_toggled:
                held_pre = any(e[2] == "pre" for e in sr)
                mode = if_flush
                if if_flush == "pair_unless_held":
                    mode = "stage" if held_pre else "pair"
                if mode == "stage":
                    # the toggle rides to the FIRE walk (staged left)
                    sl.insert(0, (IF_TS, stamp(), fno))
                else:
                    lmem.append((IF_TS, stamp(), fno))
                    if mode == "pair":
                        for b in rmem:
                            if b[0] not in expired:
                                creations.append((IF_TS, b[0]))
            if temporal:
                if role in ("A", "AB"):
                    if was_linked or shared:
                        pass  # landed: pre-linked or shared -> stash (stay staged)
                    else:
                        # LINK-TRANSITION flush: lefts fill + pair x right memory
                        e = sl.pop(0)
                        lmem.append((e[0], e[1], fno))
                        for b in rmem:
                            if b[0] in expired:
                                continue
                            if eligible(op, lo, hi, e[0], b[0]):
                                creations.append((e[0], b[0]))
                # rights: never flush-pair (stay staged) — landed
            else:
                if role in ("A", "AB") and ce is None:
                    e = sl.pop(0)
                    lmem.append((e[0], e[1], fno))
                    for b in rmem:
                        creations.append((e[0], b[0]))
                # plain rights: stay (cycle-3 landed)
            if creations:
                windows.append(creations)
        # ---- the FIRE evaluation ----
        creations = []
        if ce is not None and not if_now():
            lmem = [e for e in lmem if e[0] != IF_TS]
        fire_linked = (
            (bool(lmem or sl) and bool(rmem or sr)) if temporal else bool(if_now())
        )
        if not fire_linked:
            for role, mode in (("sink0", c_sink0), ("peer", c_peer)):
                out[role].append([c for w in windows
                                  for c in (w if mode == "fwd" else list(reversed(w)))])
            # quiescence delete
            lmem = [e for e in lmem if e[0] not in expired or e[0] == IF_TS]
            rmem = [e for e in rmem if e[0] not in expired]
            sl = [e for e in sl if e[0] not in expired]
            sr = [e for e in sr if e[0] not in expired]
            continue
        if ce is not None:
            present = if_now()
            have = any(e[0] == IF_TS for e in lmem) or any(e[0] == IF_TS for e in sl)
            if present and not have:
                lmem.append((IF_TS, stamp(), fno))
                for b in rmem:
                    if b[0] not in expired:
                        creations.append((IF_TS, b[0]))
            if not present and have:
                lmem = [e for e in lmem if e[0] != IF_TS]
        if temporal:
            pre_r = list(rmem)
            fills = list(sl)
            rights = list(sr)
            is_ab_batch = (
                bool(fills)
                and {e[1] for e in fills} == {e[1] for e in rights}
            )
            if wstruct == "per_fact_ab" and is_ab_batch:
                # per-FACT newest-first (853/616/134/min_sj decode):
                #   1. cross-RIGHT arm: own-right x {older-staged +
                #      memory} lefts (pscan order, self excluded)
                #   2. SELF-pair
                #   3. cross-LEFT arm: own-left x older-staged rights
                #      (arrival) + memory rights (rscan)
                sl = []
                sr = []
                facts = sorted(fills, key=lambda x: -x[1])  # newest first
                pre_mem_lefts = [a for a in lmem]
                for e in facts:
                    lmem.append((e[0], e[1], fno))
                for e in reversed(facts):
                    rmem.append((e[0], e[1], fno))
                for e in facts:
                    # arm 1: older staged lefts (arrival) then memory
                    # lefts (prior newest-first — the pscan shape)
                    older_l = sorted([a for a in facts if a[1] < e[1]],
                                     key=lambda x: x[1])
                    mem_l = sorted(pre_mem_lefts, key=lambda x: -x[1])
                    for a in older_l + mem_l:
                        if a[0] in expired or a[0] == IF_TS:
                            continue
                        if eligible(op, lo, hi, a[0], e[0]):
                            creations.append((a[0], e[0]))
                    # arm 2: self
                    if e[0] not in expired and eligible(op, lo, hi, e[0], e[0]):
                        creations.append((e[0], e[0]))
                    # arm 3: older staged rights (arrival) then memory
                    older_r = sorted([b for b in facts if b[1] < e[1]],
                                     key=lambda x: x[1])
                    pre_r_iter = pre_r if rscan == "push" else list(reversed(pre_r))
                    for b in older_r + pre_r_iter:
                        if b[0] in expired:
                            continue
                        if eligible(op, lo, hi, e[0], b[0]):
                            creations.append((e[0], b[0]))
                if creations:
                    windows.append(creations)
                for role, mode in (("sink0", c_sink0), ("peer", c_peer)):
                    out[role].append([c for w in windows
                                      for c in (w if mode == "fwd" else list(reversed(w)))])
                lmem = [e for e in lmem if e[0] not in expired or e[0] == IF_TS]
                rmem = [e for e in rmem if e[0] not in expired]
                continue
            sl = []
            fill_seqs = set()
            for e in reversed(fills):
                lmem.append((e[0], e[1], fno))
                fill_seqs.add(e[1])
            sr = []
            for e in rights:  # staged order = newest first
                rmem.append((e[0], e[1], fno))
                # partner scan (THE cycle-4 dim)
                if pscan == "rel_arrival":
                    post = [a for a in lmem if a[1] > e[1]]
                    pre_a = [a for a in lmem if a[1] <= e[1]]
                    part = sorted(post, key=lambda x: x[1]) + \
                           sorted(pre_a, key=lambda x: x[1])
                elif pscan == "this_fire_arr_mem_desc":
                    this_f = [a for a in lmem if a[2] == fno]
                    prior = [a for a in lmem if a[2] != fno]
                    part = sorted(this_f, key=lambda x: x[1]) + \
                           sorted(prior, key=lambda x: -x[1])
                elif pscan == "this_eval_arr_mem_desc":
                    this_e = [a for a in lmem if a[1] in fill_seqs]
                    prior = [a for a in lmem if a[1] not in fill_seqs]
                    part = sorted(this_e, key=lambda x: x[1]) + \
                           sorted(prior, key=lambda x: -x[1])
                elif pscan == "asc":
                    part = sorted(lmem, key=lambda x: x[1])
                else:
                    part = sorted(lmem, key=lambda x: -x[1])
                for a in part:
                    if a[0] in expired or a[0] == IF_TS:
                        continue
                    if eligible(op, lo, hi, a[0], e[0]):
                        creations.append((a[0], e[0]))
            lefts_iter = fills if liter == "head" else list(reversed(fills))
            pre_r_iter = pre_r if rscan == "push" else list(reversed(pre_r))
            for e in lefts_iter:
                for b in pre_r_iter:
                    if b[0] in expired:
                        continue
                    if eligible(op, lo, hi, e[0], b[0]):
                        creations.append((e[0], b[0]))
        else:
            pre_l = list(lmem)
            rights = list(sr)
            sr = []
            pre = [e for e in rights if e[2] == "pre"]
            post_arr = sorted([e for e in rights if e[2] == "post"], key=lambda x: x[1])
            seqr = pre + post_arr  # cycle-3 survivor: pre_lifo_then_post_arr
            for e in seqr:
                rmem.append((e[0], e[1], fno))
                for a in pre_l:
                    if a[0] in expired:
                        continue
                    creations.append((a[0], e[0]))
            lefts = list(sl)
            sl = []
            for e in lefts:
                lmem.append((e[0], e[1], fno))
                for b in rmem:
                    if b[0] not in expired:
                        creations.append((e[0], b[0]))
        if creations:
            windows.append(creations)
        for role, mode in (("sink0", c_sink0), ("peer", c_peer)):
            out[role].append([c for w in windows
                              for c in (w if mode == "fwd" else list(reversed(w)))])
        # quiescence delete (post-fire)
        lmem = [e for e in lmem if e[0] not in expired or e[0] == IF_TS]
        rmem = [e for e in rmem if e[0] not in expired]
        sl = [e for e in sl if e[0] not in expired]
        sr = [e for e in sr if e[0] not in expired]
    return out


IF_TS = -999999  # IF pseudo-left sentinel (pairs render as (IF, right))


def pin(name, ce, op, lo, hi, fires, want):
    return (name, ce, op, lo, hi, fires, want)


PINS = [
    # --- temporal (ce=None): the t-ladder essentials ---
    pin("min_sj", None, "after", 0, 100,
        [[("ins", "AB", 6), ("ins", "AB", 40)]],
        [[(6, 6), (40, 40), (6, 40)]]),
    pin("t1", None, "after", 0, 200,
        [[("ins", "A", 0), ("ins", "A", 50), ("ins", "A", 100), ("ins", "B", 100)]],
        [[(100, 100), (50, 100), (0, 100)]]),
    pin("t5", None, "before", 0, 200,
        [[("ins", "A", 0), ("ins", "A", 60), ("ins", "A", 120), ("ins", "A", 250), ("ins", "B", 20)]],
        [[(120, 20), (60, 20)]]),
    pin("t6", None, "after", 0, 200,
        [[("ins", "B", 100), ("ins", "B", 150)], [("ins", "A", 50)]],
        [[], [(50, 150), (50, 100)]]),
    pin("t7", None, "after", 0, 200,
        [[("ins", "B", 100)], [("ins", "A", 50), ("ins", "B", 150)]],
        [[], [(50, 100), (50, 150)]]),
    pin("t10", None, "after", 0, 200,
        [[("ins", "B", 100)], [("ins", "B", 150)], [("ins", "A", 50)]],
        [[], [], [(50, 150), (50, 100)]]),
    pin("t13", None, "after", 0, 200,
        [[("ins", "B", 100)], [("ins", "A", 50)], [("ins", "B", 150)]],
        [[], [(50, 100)], [(50, 150)]]),
    pin("t14", None, "after", 0, 500,
        [[("ins", "B", 150), ("ins", "B", 100), ("ins", "A", 10)], [("ins", "A", 50)]],
        [[(10, 100), (10, 150)], [(50, 100), (50, 150)]]),
    pin("t15", None, "after", 0, 500,
        [[("ins", "A", 60), ("ins", "A", 20), ("ins", "B", 100)]],
        [[(20, 100), (60, 100)]]),
    pin("cf56", None, "before", 0, 100,
        [[("ins", "AB", 31), ("ins", "AB", 4)]],
        [[(31, 31), (4, 4), (31, 4)]]),
    # --- CE relink shapes (plain join under exists/not; rights = P@v) ---
    # u1/cf5x18: exists; fire1 E+, P1; fire2 advance kills enablers;
    # fire3 E+ then P2. IF pairs render (IF_TS, v).
    pin("u1", "exists", None, 0, 0,
        [[("ins", "E+", 1000), ("ins", "B", 1)],
         [("adv", [1000])],
         [("ins", "E+", 2000), ("ins", "B", 2)]],
        [[(IF_TS, 1)], [], [(IF_TS, 1), (IF_TS, 2)]]),
    # u3: not; fire1 E+ blocks, P1 (held); fire2 advance unblocks + P2
    pin("u3", "not", None, 0, 0,
        [[("ins", "B", 1), ("ins", "E+", 1000)],
         [("adv", [1000]), ("ins", "B", 2)]],
        [[], [(IF_TS, 2), (IF_TS, 1)]]),
    # v2: exists; fire1 P1 (held, no enabler); fire2 E+ then P2
    pin("v2", "exists", None, 0, 0,
        [[("ins", "B", 1)],
         [("ins", "E+", 1000), ("ins", "B", 2)]],
        [[], [(IF_TS, 2), (IF_TS, 1)]]),
    # v3: not; fire1 P1 (unblocked -> fires); fire2 E+ blocks; fire3 advance + P2
    pin("v3", "not", None, 0, 0,
        [[("ins", "B", 1)],
         [("ins", "E+", 1000)],
         [("adv", [1000]), ("ins", "B", 2)]],
        [[(IF_TS, 1)], [], [(IF_TS, 2), (IF_TS, 1)]]),
    # v4: exists; two held P generations, then E+
    pin("v4", "exists", None, 0, 0,
        [[("ins", "B", 1)], [("ins", "B", 2)], [("ins", "E+", 1000)]],
        [[], [], [(IF_TS, 1), (IF_TS, 2)]]),
    # v5: not; P1 fires (memory); E+ blocks + P2 (held); advance + P3
    pin("v5", "not", None, 0, 0,
        [[("ins", "B", 1)],
         [("ins", "E+", 1000), ("ins", "B", 2)],
         [("adv", [1000]), ("ins", "B", 3)]],
        [[(IF_TS, 1)], [], [(IF_TS, 3), (IF_TS, 2), (IF_TS, 1)]]),
    # u1c: exists; P2 arrives WITH the advance fire (held pre-link);
    # E+ alone at fire3 -> [(2),(1)]
    pin("u1c", "exists", None, 0, 0,
        [[("ins", "E+", 1000), ("ins", "B", 1)],
         [("adv", [1000]), ("ins", "B", 2)],
         [("ins", "E+", 2000)]],
        [[(IF_TS, 1)], [], [(IF_TS, 2), (IF_TS, 1)]]),
]

# --- cycle-4 two-rule pins: want = {"sink0": [...], "peer": [...]} ---
PINS2 = [
    # 551 fire1: A=E1, B=E0, after[0,100]; arrivals B31,B26,A27,A7
    ("cf551", None, "after", 0, 100,
     [[("ins", "B", 31), ("ins", "B", 26), ("ins", "A", 27), ("ins", "A", 7)]],
     {"sink0": [[(27, 31), (7, 26), (7, 31)]],
      "peer": [[(7, 31), (7, 26), (27, 31)]]}),
    # 526: A=E0, B=E1, after[0,50]; fire1 B9,A38,A24,A4 -> (4,9);
    # fire2 adv(nothing dies) + B80, A67
    ("cf526", None, "after", 0, 50,
     [[("ins", "B", 9), ("ins", "A", 38), ("ins", "A", 24), ("ins", "A", 4)],
      [("adv", []), ("ins", "B", 80), ("ins", "A", 67)]],
     {"sink0": [[(4, 9)], [(38, 80), (67, 80)]],
      "peer": [[(4, 9)], [(67, 80), (38, 80)]]}),
    # 616 fire1: self-join after[0,150]; AB22, AB24
    ("cf616", None, "after", 0, 150,
     [[("ins", "AB", 22), ("ins", "AB", 24)]],
     {"sink0": [[(22, 22), (24, 24), (22, 24)]],
      "peer": [[(22, 24), (24, 24), (22, 22)]]}),
    # 721: shared E0xE1 after[0,150]; fire1 A40,B15 (no pair);
    # fire2 A47 then B47 -> creations [(40,47),(47,47)]
    ("cf721", None, "after", 0, 150,
     [[("ins", "A", 40), ("ins", "B", 15)],
      [("adv", []), ("ins", "A", 47), ("ins", "B", 47)]],
     {"sink0": [[], [(47, 47), (40, 47)]],
      "peer": [[], [(40, 47), (47, 47)]]}),
    # 853 fire1: three-left shared self-join before[0,100];
    # AB1, AB21, AB23 one prologue batch
    ("cf853", None, "before", 0, 100,
     [[("ins", "AB", 1), ("ins", "AB", 21), ("ins", "AB", 23)],
      [("adv", []), ("ins", "AB", 71)]],
     {"sink0": [[(1, 1), (21, 1), (21, 21), (23, 21), (23, 1), (23, 23)],
                [(71, 23), (71, 21), (71, 1), (71, 71)]],
      "peer": [[(23, 23), (23, 1), (23, 21), (21, 21), (21, 1), (1, 1)],
               [(71, 71), (71, 1), (71, 21), (71, 23)]]}),
    # 134: self-join before[0,150]; fire1 AB14,AB6; fire3 adv kills
    # 14,6 + AB1209; fire4 adv(nothing) + AB1257
    ("cf134", None, "before", 0, 150,
     [[("ins", "AB", 14), ("ins", "AB", 6)],
      [("adv", [14, 6]), ("ins", "AB", 1209)],
      [("adv", []), ("ins", "AB", 1257)]],
     {"sink0": [[(14, 14), (6, 6), (14, 6)], [(1209, 1209)],
                [(1257, 1209), (1257, 1257)]],
      "peer": [[(14, 6), (6, 6), (14, 14)], [(1209, 1209)],
               [(1257, 1257), (1257, 1209)]]}),
]


def main():
    dims = [
        ["rel_arrival", "this_fire_arr_mem_desc", "this_eval_arr_mem_desc", "asc", "desc"],  # pscan
        ["push", "reverse"],   # rscan
        ["head", "arrival"],   # liter
        ["fwd", "rev"],        # c_sink0
        ["fwd", "rev"],        # c_peer
        ["pair", "fill", "stage", "pair_unless_held"],  # if_flush
        ["phased", "per_fact_ab"],  # walk structure for AB batches
    ]
    survivors = []
    for cfg in itertools.product(*dims):
        ok = True
        # single-rule pins: firings compare against the sink0 role
        for name, ce, op, lo, hi, fires, want in PINS:
            try:
                got = run(cfg, ce, op, lo, hi, fires)["sink0"]
            except Exception:
                ok = False
                break
            if got != want:
                ok = False
                break
        if ok:
            for name, ce, op, lo, hi, fires, want in PINS2:
                try:
                    got = run(cfg, ce, op, lo, hi, fires, shared=True)
                except Exception:
                    ok = False
                    break
                if got["sink0"] != want["sink0"] or got["peer"] != want["peer"]:
                    ok = False
                    break
        if ok:
            survivors.append(cfg)
    print(f"{len(survivors)} survivor(s)")
    for s in survivors[:16]:
        print("  pscan=%s rscan=%s liter=%s sink0=%s peer=%s iff=%s ws=%s" % s)
    if not survivors:
        best = []
        for cfg in itertools.product(*dims):
            bad = []
            for name, ce, op, lo, hi, fires, want in PINS:
                try:
                    if run(cfg, ce, op, lo, hi, fires)["sink0"] != want:
                        bad.append(name)
                except Exception:
                    bad.append(name + "!")
            for name, ce, op, lo, hi, fires, want in PINS2:
                try:
                    got = run(cfg, ce, op, lo, hi, fires, shared=True)
                    if got["sink0"] != want["sink0"]:
                        bad.append(name + ":s0")
                    if got["peer"] != want["peer"]:
                        bad.append(name + ":pr")
                except Exception:
                    bad.append(name + "!")
            best.append((len(bad), cfg, bad))
        best.sort(key=lambda x: x[0])
        for n, cfg, bad in best[:8]:
            print(n, cfg, bad)


if __name__ == "__main__":
    main()
