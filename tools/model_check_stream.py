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
    uflush, ldrain_plain, ldrain_temp, lorder = cfg
    node = Node(temporal=(ce is None))
    enablers = {}   # ts -> alive (for CE shapes); also A-facts for temporal
    IF = "IF"
    if_present = (ce == "not")  # not with no enablers = unblocked
    if ce == "not" and if_present:
        pass  # IF materializes lazily via first toggle staging? Start: stage at first fire walk
    linked_now = False
    out = []
    started = False

    def is_linked():
        if ce is None:
            return bool(node.lmem or node.sl) and bool(node.rmem or node.sr)
        # CE shapes: path linked iff the CE admits the IF (exists: enabler
        # alive; not: none alive) AND... D-031 not-position linking is
        # inverted; model the OBSERVABLE: evaluation reaches the join iff
        # IF is present or staged.
        return if_now() or bool(node.rmem or node.sr) and if_now()

    def if_now():
        alive = any(enablers.values())
        return alive if ce == "exists" else (not alive) if ce == "not" else False

    def drain_held_rights(exclude_ts=None):
        keep, drain = [], []
        for e in node.sr:
            (drain, keep)[e[0] == exclude_ts].append(e)
        for ts, s in sorted(drain, key=lambda x: x[1]):
            node.rmem.append((ts, s))
        node.sr = keep

    def walk_window(sl, sr, dels):
        """One evaluation window over the given staged slices + dels.
        Returns creations."""
        creations = []
        # deletes first (expirations): remove rights, kill pairs implicitly
        for ts in dels:
            node.rmem = [(t, s) for (t, s) in node.rmem if t != ts]
            node.sr = [(t, s) for (t, s) in node.sr if t != ts]
        if node.temporal:
            pre_r = list(node.rmem)
            for e in reversed(sl):
                node.lmem.append(e)
            for ts, s in sr:  # staged list order = newest first
                node.rmem.append((ts, s))
                part = sorted(node.lmem, key=lambda x: x[1])
                for a, _ in part:
                    if eligible(op, lo, hi, a, ts):
                        creations.append((a, ts))
            lefts_iter = sl if lorder == "head" else list(reversed(sl))
            for l, _ in lefts_iter:
                for b, _ in pre_r:
                    if eligible(op, lo, hi, l, b):
                        creations.append((l, b))
        else:
            pre_l = list(node.lmem)
            for ts, s in sr:
                node.rmem.append((ts, s))
                for a, _ in pre_l:
                    creations.append((a, ts))
            lefts_iter = sl if lorder == "head" else list(reversed(sl))
            for e in lefts_iter:
                node.lmem.append(e)
                for b, _ in list(node.rmem):
                    creations.append((e[0], b))
        return creations

    for fire in fires:
        windows = []
        pending_if_del = False
        for step in fire:
            if step[0] == "adv":
                # expirations stage; CE toggle materializes at the walk
                for ts in step[1]:
                    if ts in enablers:
                        enablers[ts] = False
                # link transition from an advance (non-flush): ldrain
                if ce is not None:
                    if if_now() and not linked_now:
                        ld = ldrain_temp if node.temporal else ldrain_plain
                        if ld in ("all", "nonflush"):
                            drain_held_rights()
                        linked_now = True
                    elif not if_now():
                        linked_now = False
                continue
            _, role, ts = step
            was_if = if_now()
            if role.startswith("E"):
                enablers[ts] = True
            flushed = []
            if role in ("A", "AB") or (ce is None and role == "A"):
                node.sl.insert(0, (ts, node.stamp()))
            if role in ("B", "AB"):
                node.sr.insert(0, (ts, node.stamp()))
            # link check after this insert's staging
            now_linked = is_linked() if ce is None else if_now()
            if ce is not None and not now_linked:
                linked_now = False
            trig_link = now_linked and not linked_now
            if trig_link:
                ld = ldrain_temp if node.temporal else ldrain_plain
                if ld == "all":
                    drain_held_rights(exclude_ts=ts if role in ("B", "AB") else None)
                linked_now = True
            # CE toggle staged by THIS insert
            sl_trig = []
            if ce is not None and if_now() and not was_if:
                sl_trig.append((IF_TS, node.stamp()))
            # trigger-scoped flush: this insert's own staging (+ IF toggle)
            sr_trig = [e for e in node.sr[:1] if e[0] == ts and role in ("B", "AB")]
            if role in ("B", "AB") and sr_trig:
                node.sr = node.sr[1:]
            own_sl = [e for e in node.sl[:1] if e[0] == ts and role in ("A", "AB") and ce is None]
            if own_sl:
                node.sl = node.sl[1:]
            if now_linked:
                w = walk_window(sl_trig + own_sl, sr_trig, [])
                if w:
                    windows.append(w)
            else:
                # unlinked flush behavior
                if uflush == "drain" or (uflush == "drain_t" and node.temporal):
                    for e in sr_trig:
                        node.rmem.append(e)
                    for e in own_sl:
                        node.lmem.append(e)
                    for e in sl_trig:
                        node.lmem.append(e)
                else:
                    node.sr = sr_trig + node.sr
                    node.sl = own_sl + sl_trig + node.sl
        # fire evaluation: the final window (advance dels + IF toggles
        # from advances + all remaining staging)
        dels = []
        sl_fire = list(node.sl)
        node.sl = []
        # IF toggle from advances materializes here
        if ce is not None:
            present_now = if_now()
            have_if = any(t == IF_TS for t, _ in node.lmem) or any(t == IF_TS for t, _ in sl_fire)
            if present_now and not have_if:
                sl_fire = [(IF_TS, node.stamp())] + sl_fire
            if not present_now and have_if:
                node.lmem = [(t, s) for (t, s) in node.lmem if t != IF_TS]
                sl_fire = [(t, s) for (t, s) in sl_fire if t != IF_TS]
        # D-091: the fire evaluation runs only for LINKED paths —
        # unlinked staging is HELD (t6/t14's cross-fire holds).
        fire_linked = (
            (bool(node.lmem or node.sl or sl_fire) and bool(node.rmem or node.sr))
            if ce is None
            else if_now()
        )
        if fire_linked:
            sr_fire = list(node.sr)
            node.sr = []
            w = walk_window(sl_fire, sr_fire, dels)
            if w:
                windows.append(w)
        else:
            node.sl = sl_fire + node.sl  # restore holds
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
