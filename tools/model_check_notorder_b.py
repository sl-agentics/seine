#!/usr/bin/env python3
"""CEP item-1b Family B — MODEL for the EVENT-not EXPIRY firing order (P-FIRST
regime). Validates a PREDICT against the oracle population from fuzz_notorder_b.py
(0-div is the port spec). CRACKED 2026-07-09 (D-143): the SEGMENT model.

THE RULE (`not E0() P()`, blocked P's fire at the expiry-unblock advance):
  Process ops chronologically. A SEGMENT counter increments on each E0 INSERT
  (the initial blocker AND every mid-run arrival). Each P records:
    - ins_seg: the segment current at its insert;
    - upd_seg / upd_app: the segment current at its LAST update + a global apply
      sequence (only if updated).
  FINAL PLACEMENT: a P updated into a LATER segment than its insert MOVES to that
  segment's UPDATES sublist; a same-segment update leaves it an INSERT (no move).
  FIRE ORDER = segments in REVERSE index order (newest first); within a segment,
  INSERTS first (insertion order = gidx asc) then UPDATES (apply order DESC =
  last-updated first).

Why segments (not epochs, the D-140 model): D-140's `fuzz_notorder` was BLOCKER-
FIRST, where every epoch flush is blocked ⇒ each epoch is its own segment (epoch
reversal). In the P-FIRST regime (a P inserted before the blocker — the real
witness cf401x362, and this population) epoch boundaries do NOT segment; only E0
inserts do. So D-140's epoch key is the blocker-first special case; this segment
model is the general P-first rule. (blocker-first is NOT reproduced by this model
— it is a separate regime; the port branches on it. Verified 0-div on 2671+
P-first scenarios; the D-140 pins are blocker-first and unaffected.)

Usage:  model_check_notorder_b.py <notpop_b_*.json> [MODEL]
  MODEL=seg (default, the cracked spec) | d140 (the blocker-first special case)
  | flush (D-150: the MECHANICAL per-arrival flush simulator — 0-div on ALL
    event-blocker regimes: pure-bf 694+679-fresh, P-first, MIXED, val; 9041
    scenarios + 55 probes. Subsumes seg/seg2/d140 on this family; the spec
    for the bf-with-arrivals port. Scope: EVENT blockers (@expires) only —
    the plain-fact `not D()` family (fuzz_notorder.py) is different machinery.)
"""
import json, os, sys


def predict_seg(scn):
    seg = [0]
    gidx = {}; ins_seg = {}; upd_seg = {}; upd_app = {}; vof = {}
    idx = [0]; app = [0]

    def do_insert(f):
        if f["type"] == "E0":
            seg[0] += 1
        else:  # a positive-pattern (P) fact
            v = f["fields"]["v"]; vof[idx[0]] = v
            gidx[v] = idx[0]; ins_seg[v] = seg[0]
        idx[0] += 1

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg[0]; upd_app[v] = app[0]; app[0] += 1

    for f in scn["facts"]:
        do_insert(f)
    for ep in scn["epochs"]:
        for a in ep["actions"]:
            if a["op"] == "update":
                do_update(a)
            elif a["op"] == "insert":
                do_insert(a)
            # advance / delete: no segmentation effect
        for f in ep["facts"]:
            do_insert(f)

    ps = list(gidx)
    seg_of = {}; is_upd = {}
    for v in ps:
        if v in upd_seg and upd_seg[v] != ins_seg[v]:
            seg_of[v], is_upd[v] = upd_seg[v], True
        else:
            seg_of[v], is_upd[v] = ins_seg[v], False
    out = []
    for s in sorted({seg_of[v] for v in ps}, reverse=True):
        mem = [v for v in ps if seg_of[v] == s]
        inserts = sorted((v for v in mem if not is_upd[v]), key=lambda v: gidx[v])
        updates = sorted((v for v in mem if is_upd[v]), key=lambda v: -upd_app[v])
        out.extend(inserts + updates)
    return out


def predict_d140(scn):
    """The D-140 blocker-first model (last-touch EPOCH batch, reverse, initial
    last; within batch inserts then updates). Diverges on the P-first population;
    kept for the bisect."""
    facts = scn["facts"]
    gidx = {}; vof = {}; batch = {}; kind = {}; useq = {}
    idx = 0; app = [0]
    for f in facts:
        if f["type"] == "P":
            v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = 0; kind[v] = "ins"
        idx += 1
    for e_i, ep in enumerate(scn["epochs"]):
        for a in ep["actions"]:
            if a["op"] == "update":
                v = vof.get(a["target"])
                if v is not None:
                    batch[v] = e_i + 1; kind[v] = "upd"; useq[v] = app[0]; app[0] += 1
        for f in ep["facts"]:
            if f["type"] == "P":
                v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = e_i + 1; kind[v] = "ins"
            idx += 1
    ps = list(gidx)
    ebs = sorted({batch[v] for v in ps if batch[v] >= 1})
    out = []
    for b in list(reversed(ebs)) + [0]:
        members = [v for v in ps if batch[v] == b]
        members.sort(key=lambda v: (0, gidx[v]) if kind[v] == "ins" else (1, -useq[v]))
        out.extend(members)
    return out


def predict_seg2(scn):
    """D-146 UNIFIED rule (mixed initial positions, `xf_cep_not_order_mixed_initial`).
    BLOCKER-FIRST (no initial P before the first E0) -> the D-140 epoch model.
    Else (P-first / MIXED): segments (E0-insert count) NEWEST-first as D-143; WITHIN
    a segment three classes: [epoch>=1 inserts, gidx asc] ++ [updates, apply-seq
    DESC] ++ [EPOCH-0 INITIALS, gidx asc — the last-class tail]. Class moves: a P
    updated into a LATER segment moves there as an update (D-143); an EPOCH-0
    initial updated AT ALL (even same-segment) promotes into the updates slot
    (D-145 m_updP2mid); an epoch>=1 insert updated same-segment stays an insert
    (D-143 nb801x0)."""
    seg = 0
    gidx = {}; ins_seg = {}; ins_epoch = {}; upd_seg = {}; upd_app = {}; is_upd = {}
    vof = {}
    idx = 0; app = [0]
    first_e0 = first_p = None

    def do_insert(f, epoch_no):
        nonlocal seg, idx, first_e0, first_p
        if f["type"] == "E0":
            if first_e0 is None:
                first_e0 = idx
            seg += 1
        else:
            v = f["fields"]["v"]; vof[idx] = v
            if first_p is None:
                first_p = idx
            gidx[v] = idx; ins_seg[v] = seg; ins_epoch[v] = epoch_no; is_upd[v] = False
        idx += 1

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg; upd_app[v] = app[0]; is_upd[v] = True; app[0] += 1

    for f in scn["facts"]:
        do_insert(f, 0)
    for e_i, ep in enumerate(scn["epochs"]):
        for a in ep["actions"]:
            if a["op"] == "update":
                do_update(a)
            elif a["op"] == "insert":
                do_insert(a, e_i + 1)
        for f in ep["facts"]:
            do_insert(f, e_i + 1)

    blocker_first = first_e0 is not None and (first_p is None or first_e0 < first_p)
    if blocker_first:
        return predict_d140(scn)

    ps = list(gidx)
    seg_of = {}; cls = {}
    for v in ps:
        moved = is_upd[v] and (upd_seg[v] > ins_seg[v] or ins_epoch[v] == 0)
        if moved:
            seg_of[v] = upd_seg[v]; cls[v] = 1
        elif ins_epoch[v] == 0:
            seg_of[v] = ins_seg[v]; cls[v] = 2
        else:
            seg_of[v] = ins_seg[v]; cls[v] = 0
    out = []
    for s in sorted({seg_of[v] for v in ps}, reverse=True):
        mem = [v for v in ps if seg_of[v] == s]
        mem.sort(key=lambda v: (cls[v], -upd_app[v] if cls[v] == 1 else gidx[v]))
        out.extend(mem)
    return out


def predict_flush(scn):
    """D-150: the MECHANICAL per-arrival flush simulator for the EVENT-blocker
    not-order family (`not E0() P()`, E0 @role(event) @expires, unconstrained
    not, bare P — empty listen masks). A direct replay of the graft-observed
    Drools machinery (oracle/.../BfDump + its PropagationList proxy + linking
    trace; DECISIONS D-150). Validated 0-div on ALL regimes of this family
    (blocker-first-with-arrivals AND P-first AND mixed — it subsumes seg/seg2/
    d140 here, which are per-regime shadows of this machinery). The hidden
    state the D-140/D-143 keys could not express is the join's RIGHT-MEMORY
    LIST ORDER (rtm) plus the staged-right-insert BACKLOG, evolved by:
      - each E0 op (insert or expiry-delete) FORCE-FLUSHES an eval at its
        propagation-queue position (STREAM per-arrival flush); P ops do not;
      - an eval drains the join's staged-ins backlog: rtm-append in staged-
        LIFO order (batch reversed);
      - the fire-loop eval runs iff the RuleAgendaItem got queued: a P's
        FIRST-staged insert queues it ONLY while the segment is fully linked,
        and an E0 right-insert processed while the segment is linked UNLINKS
        the unconstrained NotNode (PhreakNotNode.unlinkNotNodeOnRightInsert);
        the last E0's retract relinks it (NotNode counter 1->0). While
        unlinked, P inserts accumulate staged ACROSS epochs (the backlog);
      - a bare-P UPDATE (empty inferred mask) is an IMMEDIATE
        rtm.removeAdd (move-to-tail) at its queue position when the tuple is
        in rtm, a NO-OP when it is still staged (BetaNode.modifyObject
        reorder-only branch); it never stages, queues, or re-fires;
      - the unblock left-insert emits children over rtm IN ORDER, prepended
        into the target staging => the terminal appends REVERSED: firing
        order = reverse(rtm); a same-eval right-ins drain lands BEFORE the
        left-ins emission (PhreakJoinNode doNode order)."""
    E0_EXPIRES = next((t["event"]["expires_ms"] for t in scn["types"]
                       if t["name"] == "E0" and "event" in t), 100)
    vof = {}                      # global fact index -> P value
    idx = 0
    rtm = []                      # join right memory (P order) — THE carrier
    staged = []                   # join staged right-ins backlog, arrival order
    e0_alive = {}                 # uid -> deadline for live E0s
    e0_del_uid = {}               # global fact index -> E0 uid (delete targets)
    pending_exp = []              # uids REGISTERED for retract this flush
    not_linked = [True]
    join_count = [0]              # join right counter: links >0, unlinks at 0
    join_linked = [False]
    exec_queued = [False]
    if_blocked = [False]
    if_propagated = [False]       # the InitialFact left-ins (first fire-loop eval)
    Q = []                        # executor FIFO (pending activations)
    fired = []
    clock = [0]

    def segment_linked():
        return not_linked[0] and join_linked[0]

    def drain_staged(emit_children):
        # doRightInserts: iterate staged LIFO; rtm.add each; child per left.
        rtm.extend(reversed(staged))
        if emit_children:
            # per staged right a child is PREPENDED into trg; the terminal
            # then processes trg head-first => arrival order (double reversal)
            Q.extend(staged)
        staged.clear()

    def eval_window(e0_op=None, fire_loop=False):
        # 1. the not processes its staged E0 op (upstream of the join)
        unblock = False
        if e0_op is not None:
            kind, uid = e0_op
            if kind == "ins":
                if segment_linked():
                    not_linked[0] = False    # unlinkNotNodeOnRightInsert —
                                             # fires only WHILE LINKED (D-150;
                                             # re-confirmed D-153 nb880x7: a
                                             # blocker arriving before any P
                                             # leaves the bit SET, so the
                                             # first P links the segment)
                if if_propagated[0] and not if_blocked[0]:
                    if_blocked[0] = True     # block the IF: children die
                    Q.clear()                # matchCancelled for queued
            else:  # expiry retract (quiescence)
                del e0_alive[uid]
                if if_propagated[0] and if_blocked[0] and not e0_alive:
                    unblock = True
        # 2. join right-ins drain (before left-ins)
        drain_staged(emit_children=if_propagated[0] and not if_blocked[0])
        # 3. unblock emission (the not's left-ins reaching the join)
        if unblock:
            if_blocked[0] = False
            Q.extend(reversed(rtm))
        # 4. the IF's own staged left-ins — processed only by a FIRE-LOOP
        # eval (the exists-arc discovery, D-152/D-153: an E0 force-flush
        # skips staged lefts; one-sided windows never queue, so the IF can
        # sit staged for epochs and its first emission then swallows the
        # whole rtm at that point — nb884x248/nb886x21)
        if fire_loop and not if_propagated[0]:
            if_propagated[0] = True
            if_blocked[0] = bool(e0_alive)
            if not if_blocked[0]:
                Q.extend(reversed(rtm))

    def flush_entry_insert(f):
        nonlocal idx
        if f["type"] == "E0":
            # Drools schedules the expire job at endTs + @expires + 1 (an
            # advance to EXACTLY ts+expires does not expire — mu4 probe);
            # an arrival already past its NONNEG deadline enqueues the expire
            # action in the SAME flush (still a quiescence retract); a
            # NEGATIVE deadline never schedules at all — the DROOLS-455 leak,
            # immortal (D-152 boundary, oracle xq1-xq3).
            deadline = f["fields"]["ts"] + E0_EXPIRES + 1
            uid = idx
            e0_alive[uid] = deadline if deadline >= 0 else float("inf")
            e0_del_uid[idx] = uid
            # staging notify: first-E0 links the not-bit (already linked);
            # later E0s setNodeDirty — queue iff segment linked (pre-unlink)
            if segment_linked():
                exec_queued[0] = True
            eval_window(("ins", uid))        # STREAM force-flush at its position
            if 0 <= deadline <= clock[0]:
                pending_exp.append(uid)
        else:
            v = f["fields"]["v"]; vof[idx] = v
            staged_was_empty = not staged
            staged.append(v)
            join_count[0] += 1
            if join_count[0] == 1:
                join_linked[0] = True        # counter 0->1: linkNode
                if segment_linked():
                    exec_queued[0] = True
            elif staged_was_empty and segment_linked():
                exec_queued[0] = True        # setNodeDirty notify
        idx += 1

    def flush_entry_update(a):
        # BetaNode.modifyObject, empty inferred mask (bare P): reorder-only —
        # IMMEDIATE rtm move-to-tail at the entry's FIFO position; staged-only
        # tuple (memory==null) => total no-op. Never stages/queues/re-fires.
        v = vof.get(a["target"])
        if v is None:
            return
        if v in rtm:
            rtm.remove(v)
            rtm.append(v)

    def flush_entry_delete(a):
        # An EXPLICIT delete retracts AT ITS QUEUE POSITION (unlike expiry
        # quiescence — D-138 delete-time semantics): E0 delete = the same
        # retract force-eval as an expiry (relink on counter 1->0, unblock if
        # last, backlog drain in its eval); P delete = staged-insert
        # annihilation or rtm removal + activation cancel, notify-iff-linked.
        tgt = a["target"]
        if tgt in e0_del_uid:                # an E0 handle
            uid = e0_del_uid[tgt]
            if uid not in e0_alive:
                return                       # already expired/deleted
            if uid in pending_exp:
                pending_exp.remove(uid)      # registered expiry superseded
            if len(e0_alive) == 1:           # counter 1->0: relink + queue
                not_linked[0] = True
                exec_queued[0] = True
            elif segment_linked():
                exec_queued[0] = True
            eval_window(("del", uid))
            return
        v = vof.get(tgt)
        if v is None:
            return
        if v in staged or v in rtm:
            join_count[0] -= 1
            if join_count[0] == 0:
                join_linked[0] = False       # counter 1->0: join unlinks
            elif segment_linked():
                exec_queued[0] = True        # stagedDeleteWasEmpty notify
        if v in staged:
            staged.remove(v)                 # addDelete annihilates staged ins
        elif v in rtm:
            rtm.remove(v)
        if v in Q:
            Q.remove(v)                      # matchCancelled

    def flush_entry_advance(ms):
        # WorkingMemoryReteExpireAction entries only REGISTER the expiration
        # (+ mark expired) at their queue position; the RETRACTS are deferred
        # to quiescence (ActivationsManagerImpl.flushExpirations) — i.e. after
        # EVERY queued entry of this fireAllRules, post-ADV updates included.
        clock[0] += ms
        due = sorted(((d, u) for u, d in e0_alive.items()
                      if d <= clock[0] and u not in pending_exp))
        pending_exp.extend(u for _, u in due)

    def fire_all():
        if exec_queued[0]:                   # the fire-loop eval — runs iff
            eval_window(fire_loop=True)      # QUEUED; processes the staged
            exec_queued[0] = False           # IF (D-153)
        fired.extend(Q)
        Q.clear()
        # QUIESCENCE: flushExpirations — per-retract force-flush evals in
        # registration (deadline) order; the last retract relinks + unblocks.
        for u in list(pending_exp):
            if len(e0_alive) == 1:           # counter 1->0: relink + queue
                not_linked[0] = True
                exec_queued[0] = True
            elif segment_linked():
                exec_queued[0] = True
            eval_window(("del", u))
        pending_exp.clear()
        if exec_queued[0] and not if_propagated[0]:
            eval_window(fire_loop=True)      # a relink queued the executor
                                             # with the IF still staged: it
                                             # processes in the continuing
                                             # fire loop (D-153)
        fired.extend(Q)
        Q.clear()
        exec_queued[0] = False               # consumed by the closing fire round

    for f in scn["facts"]:
        flush_entry_insert(f)
    fire_all()
    for ep in scn["epochs"]:
        for a in ep["actions"]:
            if a["op"] == "update":
                flush_entry_update(a)
            elif a["op"] == "insert":
                flush_entry_insert(a)
            elif a["op"] == "delete":
                flush_entry_delete(a)
            elif a["op"] == "advance":
                flush_entry_advance(a["ms"])
        for f in ep["facts"]:
            flush_entry_insert(f)
        fire_all()
    return fired


def predict_pflush(scn):
    """D-158: the PLAIN-fact blocker machine (`not D() P()`, D plain, STREAM
    session) — the cf313x4 family. Graft-derived (BfDump on the pnb_* battery,
    6 dumps): the same join-staging skeleton as predict_flush with the plain
    deltas:
      - a plain-D right-INSERT stages until the next eval (no per-arrival
        force-flush — bf_expdel [9]->[10]: the block lands at the fire-loop
        eval, cancelling a not-yet-fired P match);
      - a plain-D right-DELETE (explicit) evals AT its queue position
        (bf_expdel [32]->[33]);
      - a logical D dies with its justifier: E1 expiry (quiescence retract,
        deadline order), explicit E1 delete (at position), or tag-update
        CHURN (J re-fires: new-D insert BEFORE old-D retract — bf_full
        [38]->[40] — so the not never transiently releases; the blocked left
        HANDS OFF to the new blocker);
      - a bare-P UPDATE is an immediate rtm move-to-tail at its exec position
        when flushed, a no-op while staged (bf_full [52] vs bf_no_churn);
      - an eval drains staged P's rtm-append in arrival order; the release
        (blocker count -> 0) drains first, then emits reverse(rtm)
        (bf_expdel MATCH order; bf_triple / bf_multiepoch compose all of it);
      - while BLOCKED, P-side staging alone never evals (bf_no_churn: staged
        P2 survives three epochs of live agenda work); while UNBLOCKED the
        fire-loop eval flushes + emits staged P's in arrival order.
    The agenda is ONE FIFO of items — J fires and NE emissions sequence by
    creation position; a re-block cancels queued unfired NE items."""
    exp = next((t["event"]["expires_ms"] for t in scn.get("types", [])
                if t["name"] == "E1" and "event" in t), 100)
    trace = os.environ.get("SEINE_PFLUSH_TRACE")

    def tr(msg):
        if trace:
            print(f"    | {msg}")
    vof = {}                     # global idx -> P value
    staged, rtm, fired = [], [], []
    agenda = []                  # [("ne", v) | ("j", kind, e1_uid)] FIFO
    e1 = {}                      # uid -> {"dl": deadline, "d": d_uid or None}
    d_alive = set()              # live D uids (explicit + logical)
    d_of_idx = {}                # global idx -> D uid (explicit targets)
    d_staged = []                # pending not-side ops [("ins"|"del", d_uid)]
    pending_exp = []             # E1 uids registered for quiescence retract
    if_propagated = [False]
    if_blocked = [False]
    smem_init = [False]
    next_d = [10**6]
    idx = [0]
    clock = [0]

    exec_queued = [False]

    def cancel_ne():
        agenda[:] = [it for it in agenda if it[0] != "ne"]

    def join_left_ins():
        """The (re-)released left propagates into the join: drain staged
        right-ins (reversed-append into rtm) and emit — staged children in
        arrival order, then the pre-drain rtm reversed. NOTE reversed(pre_rtm
        ++ reversed(staged)) == staged ++ reversed(pre_rtm): flush-then-
        reverse and staged-children-first are the same rule."""
        pre_rtm = list(rtm)
        rtm.extend(reversed(staged))
        emitted = list(staged) + list(reversed(pre_rtm))
        staged.clear()
        return emitted

    def eval_ne(fire_loop=False):
        """A network eval over the not's staged right-ops, processed
        SEQUENTIALLY in ARRIVAL order with TRANSIENT releases: blocker count
        1->0 releases the left INTO the join (the join drains staged P's and
        emits); a later ins in the same batch RE-BLOCKS — the drain persists,
        the unfired emissions cancel (bf_full: sync TMS del-then-queued-ins
        churn flushes [P2]; nb4001x67/x85: a second live blocker absorbs the
        del, so the join never drains). A quiescence expiry retract is the
        same eval with a single del op."""
        pending = []                         # this eval's uncommitted emissions
        left_at_start = if_propagated[0] and not if_blocked[0]
        # lazy smem init: the FIRST-ever eval drains the join's staged rights
        # into rtm even while blocked (nb4001x119 [3,4|1,2] / x144 [2,1])
        if not smem_init[0]:
            smem_init[0] = True
            rtm.extend(reversed(staged))
            staged.clear()
        for kind, d in d_staged:
            if kind == "ins":
                d_alive.add(d)
                if if_propagated[0] and not if_blocked[0]:
                    if_blocked[0] = True     # (re-)block: children die
                    pending.clear()
                    cancel_ne()
            else:
                d_alive.discard(d)
                if not d_alive and if_propagated[0] and if_blocked[0]:
                    if_blocked[0] = False    # release (maybe transient)
                    pending.extend(join_left_ins())
        d_staged.clear()
        # the join is visited by this segment eval iff its LEFT was populated
        # when the eval began (delta-linked) — it drains its right staging
        # even when the not blocked mid-eval (bf_x7 [45]: rtm gains the whole
        # batch child-less; nb4001x145 [12|10,11]); a blocked-at-start eval
        # leaves the backlog staged (nb4001x85/x54)
        if left_at_start and staged:
            rtm.extend(reversed(staged))
            if not if_blocked[0]:
                pending.extend(staged)       # still linked: ordinary children
            staged.clear()
        # the IF's own staged left-ins — first fire-loop eval only
        if fire_loop and not if_propagated[0]:
            if_propagated[0] = True
            if_blocked[0] = bool(d_alive)
            if not if_blocked[0]:
                pending.extend(join_left_ins())
        agenda.extend(("ne", v) for v in pending)
        tr(f"eval(fl={fire_loop}) rtm={rtm} blocked={if_blocked[0]} "
           f"agenda={agenda}")

    def churn(uid):
        """One J fire for E1 uid: insertLogical(new D) — and on a re-fire the
        old D's TMS retract. ARRIVAL ORDER at the not: the TMS WM-DELETE is
        SYNCHRONOUS at the fire while the RHS insertLogical is QUEUED and
        stages at the post-RHS flush — the del arrives FIRST (bf_full [40]
        stages del[D(z)] with ins[] still empty; [43] adds the ins)."""
        nd = next_d[0]; next_d[0] += 1
        old = e1[uid]["d"]
        if old is not None:
            stage_d_del(old)         # a D-DELETE staging queues the executor
        d_staged.append(("ins", nd))
        if if_propagated[0] and not if_blocked[0]:
            exec_queued[0] = True    # a right-ins while LINKED evaluates (and
                                     # blocks) before later arrivals (nb4103x160)
        e1[uid]["d"] = nd
        idx[0] += 1              # the logical D consumes an nth_inserted slot

    def drain_agenda():
        while agenda:
            it = agenda.pop(0)
            if it[0] == "ne":
                fired.append(it[1])
            else:
                _, kind, uid = it
                if uid in e1:
                    churn(uid)   # "ins" first fire and "refire" both churn
                # mid-drain eval: a del-bearing churn (bf_full), or any D
                # staging while NE still has queued items (the executor
                # evaluates the network before firing its next item)
                if d_staged and (exec_queued[0]
                                 or any(x[0] == "ne" for x in agenda)):
                    eval_ne()
                    exec_queued[0] = False

    def entry_insert(f):
        if f["type"] == "P":
            v = f["fields"]["v"]; vof[idx[0]] = v
            staged.append(v)
            if if_propagated[0] and not if_blocked[0]:
                exec_queued[0] = True        # linked: a P-ins queues the eval
        elif f["type"] == "E1":
            uid = idx[0]
            e1[uid] = {"dl": f["fields"]["ts"] + exp + 1, "d": None}
            agenda.append(("j", "ins", uid))
        else:                                # explicit plain D: stages only —
            d = next_d[0]; next_d[0] += 1    # a pure right-INS on a blocked
            d_of_idx[idx[0]] = d             # not notifies nothing (x8/x54);
            d_staged.append(("ins", d))      # while unblocked the drain's
            if if_propagated[0] and not if_blocked[0]:
                exec_queued[0] = True        # queued-items eval blocks it
        idx[0] += 1

    def stage_d_del(d):
        """A D retract reaching the not: if the D's INSERT is still sitting
        unprocessed in the staging, the delete ANNIHILATES it (TupleSets
        addDelete on a staged insert — nb4001x139/x91: an ins+del pair inside
        one staging window never blocks, never releases); else it stages a
        del and queues the executor."""
        if ("ins", d) in d_staged:
            d_staged.remove(("ins", d))
            d_alive.discard(d)
            return False
        d_staged.append(("del", d))
        exec_queued[0] = True
        return True

    def entry_update(a):
        tgt = a["target"]
        if tgt in vof:                       # bare-P: reorder-only, immediate
            v = vof[tgt]
            if v in rtm:
                rtm.remove(v); rtm.append(v)
        elif tgt in e1 and "tag" in a.get("fields", {}):
            agenda.append(("j", "refire", tgt))

    def entry_delete(a):
        tgt = a["target"]
        if tgt in vof:
            v = vof[tgt]
            if v in staged:
                staged.remove(v)
            elif v in rtm:
                rtm.remove(v)
            agenda[:] = [it for it in agenda if it != ("ne", v)]
        elif tgt in d_of_idx:                # explicit D delete
            stage_d_del(d_of_idx.pop(tgt))
        elif tgt in e1:                      # explicit E1: J-match cancel ->
            meta = e1.pop(tgt)               # TMS D retract staging + queue
            if tgt in pending_exp:
                pending_exp.remove(tgt)
            if meta["d"] is not None:
                stage_d_del(meta["d"])

    def entry_advance(ms):
        clock[0] += ms
        due = sorted((m["dl"], u) for u, m in e1.items()
                     if m["dl"] <= clock[0] and u not in pending_exp)
        pending_exp.extend(u for _, u in due)

    def quiescence_retract(u):
        """flushExpirations: the E1 retract cascades to its logical D — one
        single-del eval (an absorbed 2->1 never touches the join). An
        annihilated still-staged ins needs no eval."""
        meta = e1.pop(u)
        if meta["d"] is not None and stage_d_del(meta["d"]):
            eval_ne()
            exec_queued[0] = False

    def fire_all():
        if (exec_queued[0] or not if_propagated[0]
                or any(x[0] == "ne" for x in agenda)):
            eval_ne(fire_loop=True)
            exec_queued[0] = False
        drain_agenda()
        # QUIESCENCE: flushExpirations — narrow per-retract cascades in
        # deadline order, firing between retracts
        for u in list(pending_exp):
            if u not in e1:
                continue
            quiescence_retract(u)
            drain_agenda()
        pending_exp.clear()
        if exec_queued[0]:                   # e.g. a churn-del staged by the
            eval_ne(fire_loop=True)          # last drain with nothing queued
            exec_queued[0] = False           # behind it
            drain_agenda()

    for f in scn["facts"]:
        entry_insert(f)
    fire_all()
    for ep in scn["epochs"]:
        for a in ep["actions"]:
            if a["op"] == "update":
                entry_update(a)
            elif a["op"] == "insert":
                entry_insert(a)
            elif a["op"] == "delete":
                entry_delete(a)
            elif a["op"] == "advance":
                entry_advance(a["ms"])
        for f in ep["facts"]:
            entry_insert(f)
        fire_all()
    return fired


def predict(scn, model="seg"):
    if model == "d140":
        return predict_d140(scn)
    if model == "seg2":
        return predict_seg2(scn)
    if model == "flush":
        return predict_flush(scn)
    if model == "pflush":
        return predict_pflush(scn)
    return predict_seg(scn)


def main():
    pop = json.load(open(sys.argv[1]))
    model = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("MODEL", "seg")
    bad = 0
    for e in pop:
        got = predict(e["scenario"], model)
        want = [v for v in e["order"] if v is not None]
        if got != want:
            bad += 1
            if bad <= 14:
                print(f"  MISMATCH {e['scenario']['name']:12} predict={got} oracle={want}")
    print(f"{len(pop)} scenarios (MODEL={model}): {'ALL MATCH ✓' if bad == 0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
