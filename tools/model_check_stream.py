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


def run(cfg, ce, op, lo, hi, fires):
    (plain_dr, plain_held, plain_rgen, temp_dr, temp_lorder, uflush) = cfg
    temporal = ce is None
    lmem, rmem = [], []   # (ts, seq)
    sl, sr = [], []       # staged (prepend): (ts, seq, linkgen) 'pre'|'post'
    enablers = {}
    seq = [0]
    out = []

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
        for step in fire:
            if step[0] == "adv":
                for ts in step[1]:
                    if ts in enablers:
                        enablers[ts] = False
                # expiration deletes: staged rights annihilate; memory
                # rights get a pending del processed at the fire walk
                continue
            _, role, ts = step
            was_if = if_now()
            if role.startswith("E"):
                enablers[ts] = True
            was_linked = linked()
            if role in ("A", "AB"):
                sl.insert(0, (ts, stamp(), fno))
            if role in ("B", "AB"):
                # linkgen relative to the PRE-INSERT link state: a right
                # staged while unlinked (incl. the link trigger itself)
                # is 'pre'; staged onto an already-linked path is 'post'
                sr.insert(0, (ts, stamp(), "pre" if not was_linked else "post"))
            if_toggled = ce is not None and if_now() and not was_if
            # ---- the FLUSH for this insert ----
            if not linked():
                if uflush == "drain_t" and temporal:
                    # unlinked temporal insert self-drains to memory
                    if role in ("B", "AB"):
                        e = sr.pop(0)
                        rmem.append((e[0], e[1]))
                    if role in ("A", "AB"):
                        e = sl.pop(0)
                        lmem.append((e[0], e[1]))
                continue
            creations = []
            # left deltas (incl. IF toggles) pair x right MEMORY, consume
            if if_toggled:
                lmem.append((IF_TS, stamp()))
                for b in rmem:
                    creations.append((IF_TS, b[0]))
            if role in ("A", "AB") and temporal:
                # temporal left delta
                if temp_dr == "walk" or True:
                    e = sl.pop(0)
                    lmem.append((e[0], e[1]))
                    for b in rmem:
                        if eligible(op, lo, hi, e[0], b[0]):
                            creations.append((e[0], b[0]))
            elif role in ("A", "AB") and not temporal and ce is None:
                e = sl.pop(0)
                lmem.append((e[0], e[1]))
                for b in rmem:
                    creations.append((e[0], b[0]))
            # right deltas
            dr = temp_dr if temporal else plain_dr
            if role in ("B", "AB") and dr == "walk":
                e = sr.pop(0)
                rmem.append((e[0], e[1]))
                for a in sorted(lmem, key=lambda x: x[1]):
                    if temporal:
                        if eligible(op, lo, hi, a[0], e[0]):
                            creations.append((a[0], e[0]))
                    else:
                        creations.append((a[0], e[0]))
            # held staging visibility: plain_held=visible would let the
            # eval consume held items too (the measured-broken mode);
            # model only "hidden" faithfully for held (visible = consume
            # all in staged order, pairing x memory)
            if not temporal and plain_held == "visible":
                while sl:
                    e = sl.pop(0)
                    lmem.append((e[0], e[1]))
                    for b in rmem:
                        creations.append((e[0], b[0]))
                while sr:
                    e = sr.pop(0)
                    rmem.append((e[0], e[1]))
            if creations:
                windows.append(creations)
        # ---- the FIRE evaluation (D-091: linked paths only —
        # unlinked staging HOLDS across the fire) ----
        creations = []
        # IF UNLINK maintenance runs regardless of the gate (a blocked
        # CE retracts its IF even when the fire evaluates nothing)
        if ce is not None and not if_now():
            lmem = [(t, s) for (t, s) in lmem if t != IF_TS]
        fire_linked = (
            (bool(lmem or sl) and bool(rmem or sr)) if temporal else bool(if_now())
        )
        if not fire_linked:
            out.append([c for win in windows for c in reversed(win)])
            continue
        # advance-sourced IF toggle materializes here
        if ce is not None:
            present = if_now()
            have = any(t == IF_TS for t, _ in lmem)
            if present and not have:
                lmem.append((IF_TS, stamp()))
                # pairs with right MEMORY in memory order
                for b in rmem:
                    creations.append((IF_TS, b[0]))
            if not present and have:
                lmem = [(t, s) for (t, s) in lmem if t != IF_TS]
        # expiration dels: remove dead staged rights (annihilate) and
        # memory rights (they never pair again) — model: enabler deaths
        # only affect the IF; B-side expirations arrive via adv lists
        # handled in pins by not reusing dead ts values.
        if temporal:
            # temporal fire walk (cycle-1 survivor): fills lefts,
            # rightIns newest-first x lefts-arrival, leftIns x pre-walk
            # right memory (temp_lorder)
            pre_r = list(rmem)
            fills = list(sl)
            sl = []
            for e in reversed(fills):
                lmem.append((e[0], e[1]))
            rights = list(sr)
            sr = []
            for e in rights:  # staged order = newest first
                rmem.append((e[0], e[1]))
                for a in sorted(lmem, key=lambda x: x[1]):
                    if eligible(op, lo, hi, a[0], e[0]):
                        creations.append((a[0], e[0]))
            lefts_iter = fills if temp_lorder == "head" else list(reversed(fills))
            for e in lefts_iter:
                for b in pre_r:
                    if eligible(op, lo, hi, e[0], b[0]):
                        creations.append((e[0], b[0]))
        else:
            # plain fire walk: rightIns (generation-ordered) x pre-walk
            # left MEMORY; then leftIns head-first x full right memory
            pre_l = list(lmem)
            rights = list(sr)
            sr = []
            pre = [e for e in rights if e[2] == "pre"]
            post = [e for e in rights if e[2] == "post"]
            pre_arr = sorted(pre, key=lambda x: x[1])
            post_arr = sorted(post, key=lambda x: x[1])
            if plain_rgen == "pre_lifo_then_post_lifo":
                seqr = pre + post
            elif plain_rgen == "pre_lifo_then_post_arr":
                seqr = pre + post_arr
            elif plain_rgen == "pre_arr_then_post_lifo":
                seqr = pre_arr + post
            elif plain_rgen == "pre_arr_then_post_arr":
                seqr = pre_arr + post_arr
            elif plain_rgen == "arrival":
                seqr = sorted(rights, key=lambda x: x[1])
            else:
                seqr = rights  # head/LIFO
            for e in seqr:
                rmem.append((e[0], e[1]))
                for a in pre_l:
                    creations.append((a[0], e[0]))
            lefts = list(sl)
            sl = []
            for e in lefts:
                lmem.append((e[0], e[1]))
                for b in rmem:
                    creations.append((e[0], b[0]))
        if creations:
            windows.append(creations)
        out.append([c for win in windows for c in reversed(win)])
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
]


def main():
    dims = [
        ["noop", "drain", "drain_t"],       # uflush
        ["off", "all", "nonflush"],         # ldrain_plain
        ["off", "all", "nonflush"],         # ldrain_temp
        ["head", "arrival"],                # lorder
    ]
    survivors = []
    for cfg in itertools.product(*dims):
        ok = True
        for name, ce, op, lo, hi, fires, want in PINS:
            try:
                got = run(cfg, ce, op, lo, hi, fires)
            except Exception:
                ok = False
                break
            if got != want:
                ok = False
                break
        if ok:
            survivors.append(cfg)
    print(f"{len(survivors)} survivor(s)")
    for s in survivors[:16]:
        print("  uflush=%s ldrain_plain=%s ldrain_temp=%s lorder=%s" % s)
    if not survivors:
        best = {}
        for cfg in itertools.product(*dims):
            bad = []
            for name, ce, op, lo, hi, fires, want in PINS:
                try:
                    if run(cfg, ce, op, lo, hi, fires) != want:
                        bad.append(name)
                except Exception:
                    bad.append(name + "!")
            if len(bad) <= 2:
                print("  near-miss:", cfg, "fails", bad)


if __name__ == "__main__":
    main()
