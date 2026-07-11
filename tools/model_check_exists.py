#!/usr/bin/env python3
"""CEP item-1b Family B (exists) — `exists E1() P()` firing-order model.
Validates a PREDICT against the oracle firing SEQUENCE from fuzz_existsorder.py.

EMODEL=flush (D-152) is THE SPEC: the mechanical per-arrival flush simulator,
0-div on 5,507 oracle scenarios — all 14 banked D-144/D-147 populations (3,500,
toggle scaffold) AND the 7 full-axis SEINE_EXPOP_FULL populations (2,007: P
deletes, partial witness deletes, delayed first satisfaction, staggered
multi-witness expiry, due-on-arrival witnesses, the DROOLS-455 leak boundary,
witness updates, pure-P windows) — including the expiry transient-fires the
old key sims could not represent. The engine port is `ExShadow` (engine.rs).

EMODEL=epoch/seg are the RETIRED D-144/D-147 key models (kept for the record):
0-div only on the delete-toggle regimes; structurally blind to expiry
transients and the full-axis regimes (epoch fails ~46% of a mixed banked
population on the firing-sequence check).

THE RULE: P's fire when the witness EXISTS; each satisfy transition (live 0->1)
re-fires the whole held memory.
  - the FIRST satisfaction fires the accumulated P's FIFO (insertion order);
  - every RE-FIRE uses the D-140 EPOCH model: batch by last-touch epoch, REVERSE
    (newest first), the INITIAL epoch LAST; within a batch INSERTS then UPDATES
    (newest apply first). WITHIN-BATCH inserts sub-order by ins_seg DESC then
    insertion order (a P inserted after a mid-epoch witness arrival precedes an
    earlier one — D-147, ex801x145);
  - D-147 (regime 2, was the D-144 fence): a P inserted while SATISFIED — in the
    satisfying epoch AT/AFTER the transition witness (`ins_seg >= satisfy seg`)
    — fires IMMEDIATELY as a fresh stream insert (arrival order), NOT inside the
    re-fire batch; a before-witness insert joins the batch as its newest epoch.
    This closed cf407x121 (NE6).
EMODEL=epoch (default) | seg (the rejected mirror-of-not variant, kept for record)
| flush (D-152: the MECHANICAL per-arrival flush simulator — the D-150 machinery
  with the EXISTS polarity; subsumes epoch/the D-147 regime-2 split on this family).
Usage: model_check_exists.py <existspop_*.json>
"""
import json, os, sys


def predict_flush(scn):
    """D-152: the MECHANICAL per-arrival flush simulator for the EVENT-witness
    exists family (`exists E1() P()`, E1 @role(event) @expires, bare patterns) —
    the D-150 machinery frame with the EXISTS-side mechanics graft-observed on
    this family (BfDump on ex501x14 / ex990x20 / ex990x32; flushExpirations +
    WorkingMemoryReteExpireAction sources read for names). Validated 0-div on
    5,274 oracle scenarios (14 banked D-144/147 populations + 7 full-axis
    SEINE_EXPOP_FULL populations) + the xm1-4 transient-fire probes. The
    pieces:
      - hidden state = the join's right-memory LIST ORDER (rtm) + the staged-
        right-ins backlog (a GLOBAL prepend list that survives unfired
        boundaries) + the IF left's own staging. Every eval drains staged-ins
        LIFO into rtm, rights BEFORE lefts; children emit in arrival order
        iff the IF is THROUGH (fresh stream inserts — the D-147 regime-2
        rule);
      - each E1 op force-flushes an eval at its queue position (STREAM per-
        arrival flush) that processes RIGHTS along the path; STAGED LEFTS
        (the IF's initial left-ins, from session creation) wait for the
        FIRE-LOOP eval — so a FIRST satisfaction emits reverse(rtm) at the
        fire-loop (swallowing post-witness drains: ex990x20 [3,1,2]), while
        a RE-satisfy (IF resident in the exists memory) emits at the witness
        exec, after that exec's own drain (the D-144 epoch reversal + the
        D-147 before-witness rule fall out);
      - the fire-loop eval runs iff the RuleAgendaItem got QUEUED this
        window: the satisfy-link COMPLETING the segment (E1 count 0->1 with
        the join populated), P staging while the exists side is populated,
        or a terminal-reaching delete. One-sided windows queue nothing — the
        IF (and the P backlog) can sit staged across epochs (ex990x32
        cycle 0: witnesses with no P's never link);
      - an explicit E1 delete retracts AT ITS QUEUE POSITION; an unsatisfy
        edge (count 1->0) is the IF left-DELETE: children die, QUEUED
        activations cancel (matchCancelled). EXPIRY retracts instead run at
        QUIESCENCE (registered by advance / due-on-arrival same-flush, in
        deadline order; deadline = ts+@expires+1) — AFTER the agenda
        drained, so pre-quiescence emissions FIRE first (transient fires,
        xm1-4) and marked-expired witnesses keep counting/blocking until
        their retract (ex990x32 ep3: the IF blocks on a marked witness);
      - a bare-P update is an IMMEDIATE rtm move-to-tail at its position
        (empty inferred mask => reorder-only), a no-op while staged; it never
        stages, queues, or re-fires. Witness updates are INERT. A P delete
        annihilates its staged insert or leaves rtm and cancels its queued
        activation."""
    E1_EXPIRES = next((t["event"]["expires_ms"] for t in scn["types"]
                       if t["name"] == "E1" and "event" in t), 100)
    vof = {}                      # global fact index -> P value
    rtm = []                      # join right memory (P order) — THE carrier
    staged = []                   # join staged right-ins backlog, arrival order
                                  # (a GLOBAL prepend list: survives unfired
                                  # boundaries — graft ex990x20 [16])
    e1_alive = {}                 # uid -> deadline for live E1s
    e1_del_uid = {}               # global fact index -> E1 uid (delete targets)
    pending_exp = []              # uids REGISTERED for retract this flush
    if_staged = [True]            # the IF left-ins is itself STAGED at the
                                  # exists until the FIRST fire-loop eval — a
                                  # force-flush drains rights only (graft: no
                                  # exec-time emission on the first satisfy,
                                  # ex990x20 [19]-[23] fires [3,1,2])
    if_through = [False]          # the IF is blocked-by-a-witness and its
                                  # child lives in the join's ltm (SATISFIED)
    Q = []                        # executor FIFO (pending activations)
    fired = []
    clock = [0]
    idx = [0]
    exec_queued = [False]         # RuleAgendaItem: the fire-loop eval runs iff
                                  # something QUEUED it this window (segment-
                                  # completing link / terminal-reaching delete /
                                  # staging while linked) — a one-sided window
                                  # does not (ex990x20/ex990x32 cycle 0)
    join_count = [0]              # join right counter (live P's incl. staged)

    def drain_staged(emit_children):
        rtm.extend(reversed(staged))
        if emit_children:
            Q.extend(staged)
        staged.clear()

    def eval_window(e1_op=None, fire_loop=False):
        # 1. the exists consumes its staged E1 op (upstream of the join)
        satisfy = False
        if e1_op is not None:
            kind, uid = e1_op
            if kind == "ins":
                # right-insert took the counter 0->1 with the IF left IN THE
                # EXISTS MEMORY (processed, unblocked): block + child left-ins
                # reach the join IN THIS EVAL — the re-satisfy emission is at
                # the witness's exec. A still-STAGED IF waits (fire-loop).
                if not if_staged[0] and not if_through[0] and len(e1_alive) == 1:
                    satisfy = True
            else:  # retract: explicit delete ("del") or quiescence expiry ("exp")
                del e1_alive[uid]
                if if_through[0] and not e1_alive:
                    if_through[0] = False    # left-delete: children die
                    Q.clear()                # matchCancelled for queued
        # 2. join right-ins drain (before the same eval's left-ins)
        drain_staged(emit_children=if_through[0])
        # 3. left-ins processing (the join's staged lefts, after its rights):
        #    the satisfy child — and, in a FIRE-LOOP eval only, the IF's own
        #    staged left-ins (blocked iff a witness lives => first emission)
        if satisfy:
            if_through[0] = True
            Q.extend(reversed(rtm))
        if fire_loop and if_staged[0]:
            if_staged[0] = False
            if e1_alive:
                if_through[0] = True
                Q.extend(reversed(rtm))

    def flush_entry_insert(f):
        if f["type"] == "E1":
            deadline = f["fields"]["ts"] + E1_EXPIRES + 1
            uid = idx[0]
            was_empty = not e1_alive
            # DROOLS-455 (xq1): a NEGATIVE effectiveEnd maps to Long.MAX_VALUE
            # = never expires (the leak); nonneg-past registers due-on-arrival
            e1_alive[uid] = deadline if deadline >= 0 else float("inf")
            e1_del_uid[idx[0]] = uid
            if was_empty and join_count[0] > 0:
                exec_queued[0] = True        # satisfy-link COMPLETES the segment
                                             # (exists 0->1 with the join
                                             # populated) => linkRule queues; an
                                             # arrival with NO P's never queues
                                             # (ex990x32 cycle 0: the IF stays
                                             # staged for epochs)
            eval_window(("ins", uid))        # STREAM force-flush at its position
            if 0 <= deadline <= clock[0]:
                pending_exp.append(uid)      # due on arrival: same-flush register
        else:
            v = f["fields"]["v"]; vof[idx[0]] = v
            staged.append(v)
            join_count[0] += 1
            if e1_alive:
                exec_queued[0] = True        # staging notifies iff the exists
                                             # side is populated (marked-expired
                                             # witnesses count until retract)
        idx[0] += 1

    def flush_entry_update(a):
        v = vof.get(a["target"])
        if v is None:
            return
        if v in rtm:
            rtm.remove(v)
            rtm.append(v)

    def flush_entry_delete(a):
        tgt = a["target"]
        if tgt in e1_del_uid:                # an E1 handle
            uid = e1_del_uid[tgt]
            if uid not in e1_alive:
                return                       # already expired/deleted
            if uid in pending_exp:
                pending_exp.remove(uid)      # registered expiry superseded
            if if_through[0]:
                exec_queued[0] = True        # terminal-reaching deletes notify
            eval_window(("del", uid))        # retract AT ITS QUEUE POSITION
            return
        v = vof.get(tgt)
        if v is None:
            return
        if if_through[0]:
            exec_queued[0] = True            # child delete reaches the terminal
        if v in staged or v in rtm:
            join_count[0] -= 1
        if v in staged:
            staged.remove(v)                 # addDelete annihilates staged ins
        elif v in rtm:
            rtm.remove(v)
        if v in Q:
            Q.remove(v)                      # matchCancelled

    def flush_entry_advance(ms):
        clock[0] += ms
        due = sorted(((d, u) for u, d in e1_alive.items()
                      if d <= clock[0] and u not in pending_exp))
        pending_exp.extend(u for _, u in due)

    def fire_all():
        if exec_queued[0]:
            eval_window(fire_loop=True)
        fired.extend(Q)                      # the agenda drains BEFORE
        Q.clear()                            # quiescence (transient fires)
        # QUIESCENCE: flushExpirations — per-retract force-evals in
        # registration (deadline) order; an unsatisfy here cancels only
        # quiescence-born emissions (the pre-flush ones already fired).
        for u in list(pending_exp):
            if u not in e1_alive:
                continue                     # explicitly deleted after register
            eval_window(("exp", u))
        pending_exp.clear()
        fired.extend(Q)
        Q.clear()
        exec_queued[0] = False               # consumed by the closing round

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


def predict_pexists(scn):
    """D-162: the PLAIN-witness exists machine (`exists D() P()` / `exists
    D(tag=="x") P()` / the logical J-drive, D PLAIN, STREAM session) — the
    fuzz_existsorder.py SEINE_EXPOP_PLAIN family. The pflush (D-158) plain-
    staging join skeleton with the EXISTS polarity, the D-161 NET witness
    semantics, and a graft-consistent LINK-COUNTER queue economy. Derived
    from the banked populations (existspop_5001-5003; discriminators
    ex5001x{75,79,88,103,106,125,129,130,170,248}, ex5003x280):

    JOIN MACHINERY (pflush verbatim — the downstream join is the same node):
      - a P-INSERT stages in arrival order; a bare-P UPDATE is an immediate
        rtm move-to-tail when in rtm, a no-op while staged; a P-DELETE
        annihilates its staged insert or leaves rtm at its entry position
        and cancels its queued activation;
      - every eval drains the staged rights (reversed-append into rtm) and
        emits them (arrival order) iff the rule is THROUGH after the net
        witness step; the satisfy emission is join_left_ins = staged-arrival
        ++ reversed(pre-rtm) — algebraically reversed(post-drain rtm)
        (ex5001x75 [6,4,5,1,3]; ex5001x106 [8,9,10,5,6,7,3,4,1,2]); NO
        refraction: a re-satisfy re-fires the whole right memory (x75 8-vs-6).

    EXISTS SIDE (net + counter):
      - witness (D) ops STAGE until an eval; the eval applies them as ONE
        NET batch (D-161 pins: churn coalesces in every provenance); only
        the NET 0->1 / 1->0 transition satisfies/unsatisfies (ex5001x79/
        x170: a cascade-del + same-window logical re-ins nets to NO
        transition => no re-fire, the through-drain still emits fresh P's);
      - DELETES are SYNCHRONOUS at entry: an explicit D-delete and a TMS
        cascade retract (E1 delete / J-churn old-D) move the link COUNTER
        immediately (annihilating a still-staged ins outright); UPDATES are
        DEFERRED: an alpha-exit (x->z) of a PROCESSED D stages a del with
        NO counter move (the D-155 update-deferral principle; ex5001x125:
        a P staged after the exit-update still queues), while an alpha-exit
        of a STILL-STAGED ins is a staging-level annihilation (counter
        moves; ex5002x88 stays pure-FIFO); an alpha-admit (z->x) stages a
        fresh ins (counter++);
      - QUEUE SIGNALS, two classes. LINK signals: a D-ins taking the
        counter 0->1 with the join populated (satisfy-link), or a P-ins
        staging while the counter is >0. A sync counter 1->0 DEQUEUES all
        pending link signals (segment delink — x88, x79). WM signals:
        EXPLICIT deletes only — a D-delete (even one that annihilates:
        x280/x248 drain on the boundary eval) or a P-delete of an
        rtm-RESIDENT tuple (x130) — never dequeued. TMS cascade deletes
        carry NO signal (x170: an unobserved unsatisfy coalesces forever);
      - the eval runs iff queued (fire-loop and mid-drain alike; the
        executor re-evaluates before each next item when witness ops are
        staged); the IF left is itself STAGED until the first fire-loop
        eval (the first satisfaction emits there — a pure-P backlog with no
        queued eval accumulates staged across epochs and a later satisfy
        emits it in pure arrival order: ex5001x103 [2..7] over 3 epochs);
      - logical drive: J fires interleave in the ONE agenda FIFO; a churn
        stages the old-D TMS del (sync) then the new-D ins; an explicit E1
        delete cascades its D's TMS del at entry; no advances => no expiry.
    Validated 0-div against existspop_5001-5003 (banked) + fresh seeds;
    the spec for the D-162 engine port."""
    trace = os.environ.get("SEINE_PEXISTS_TRACE")

    def tr(msg):
        if trace:
            print(f"    | {msg}")
    vof = {}                     # global idx -> P value
    staged, rtm, fired = [], [], []
    agenda = []                  # [("ne", v) | ("j", kind, e1_uid)] FIFO
    e1 = {}                      # uid -> {"d": d_uid or None}
    d_tag = {}                   # d_uid -> current tag (explicit D's)
    d_of_idx = {}                # global idx -> d_uid (explicit D targets)
    d_alive = set()              # PROCESSED live witnesses (exists memory)
    d_staged = []                # pending exists-side ops [("ins"|"del", d_uid)]
    counter = [0]                # the exists link counter: |d_alive| +
                                 # staged ins - SYNC dels (deferred-update
                                 # dels do NOT move it)
    if_propagated = [False]
    if_blocked = [True]          # exists polarity: blocked while NO witness
    next_d = [10**6]
    idx = [0]
    exec_link = [False]          # link-class signal: dequeued on sync 1->0
    exec_wm = [False]            # WM-class signal: explicit deletes, sticky
    cons = 'tag == "x"' in scn["drl"]

    def alpha(t):
        return (t == "x") if cons else True

    def queued():
        return exec_link[0] or exec_wm[0]

    def consume():
        exec_link[0] = False
        exec_wm[0] = False

    def cancel_ne():
        agenda[:] = [it for it in agenda if it[0] != "ne"]

    def join_left_ins():
        pre_rtm = list(rtm)
        rtm.extend(reversed(staged))
        emitted = list(staged) + list(reversed(pre_rtm))
        staged.clear()
        return emitted

    def counter_dec():
        counter[0] -= 1
        if counter[0] == 0:
            exec_link[0] = False             # segment delink: dequeue

    def stage_d_ins(d):
        d_staged.append(("ins", d))
        counter[0] += 1
        if counter[0] == 1 and (staged or rtm):
            exec_link[0] = True              # satisfy-link completes the segment

    def stage_d_del_sync(d, wm):
        """A SYNCHRONOUS witness retract (explicit D delete / TMS cascade):
        annihilates a still-staged ins outright, moves the counter at entry
        (1->0 dequeues link signals); an EXPLICIT delete additionally
        carries its WM eval signal."""
        if ("ins", d) in d_staged:
            d_staged.remove(("ins", d))
        else:
            d_staged.append(("del", d))
        counter_dec()
        if wm:
            exec_wm[0] = True

    def stage_d_del_deferred(d):
        """An alpha-EXIT UPDATE of a processed D: the retract is deferred
        to the eval (D-155 update-deferral) — no counter move, no signal."""
        d_staged.append(("del", d))

    def eval_ne(fire_loop=False):
        pending = []
        # NET witness step: apply the staged D ops as one batch (D-161)
        was = bool(d_alive)
        net = set(d_alive)
        for kind, d in d_staged:
            (net.add if kind == "ins" else net.discard)(d)
        d_alive.clear(); d_alive.update(net)
        d_staged.clear()
        counter[0] = len(d_alive)
        now = bool(d_alive)
        if if_propagated[0]:
            if was and not now and not if_blocked[0]:
                if_blocked[0] = True         # unsatisfy: children die
                cancel_ne()
            elif not was and now and if_blocked[0]:
                if_blocked[0] = False        # satisfy: left-ins into the join
                pending.extend(join_left_ins())
        # join right-drain: every eval moves the staged P's into rtm
        # (reversed-append); children emit iff THROUGH after the net step
        if staged:
            rtm.extend(reversed(staged))
            if if_propagated[0] and not if_blocked[0]:
                pending.extend(staged)
            staged.clear()
        # the IF's own staged left-ins — first fire-loop eval only
        if fire_loop and not if_propagated[0]:
            if_propagated[0] = True
            if_blocked[0] = not now
            if not if_blocked[0]:
                pending.extend(join_left_ins())
        agenda.extend(("ne", v) for v in pending)
        tr(f"eval(fl={fire_loop}) was={was} now={now} rtm={rtm} "
           f"staged={staged} blocked={if_blocked[0]} agenda={agenda}")

    def churn(uid):
        """One J fire for E1 uid: the TMS WM-DELETE of the old D is
        synchronous at the fire, the RHS insertLogical stages after it."""
        nd = next_d[0]; next_d[0] += 1
        old = e1[uid]["d"]
        if old is not None:
            stage_d_del_sync(old, wm=False)
        stage_d_ins(nd)
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
                    churn(uid)
                # mid-drain eval: the executor re-evaluates the network
                # before its next item when witness ops are staged
                if d_staged and (queued()
                                 or any(x[0] == "ne" for x in agenda)):
                    eval_ne(fire_loop=True)
                    consume()

    def entry_insert(f):
        if f["type"] == "P":
            v = f["fields"]["v"]; vof[idx[0]] = v
            staged.append(v)
            if counter[0] > 0:
                exec_link[0] = True          # staging notifies while linked
        elif f["type"] == "E1":
            e1[idx[0]] = {"d": None}
            agenda.append(("j", "ins", idx[0]))
        else:                                # explicit plain D
            d = next_d[0]; next_d[0] += 1
            d_of_idx[idx[0]] = d
            d_tag[d] = f["fields"]["tag"]
            if alpha(d_tag[d]):
                stage_d_ins(d)
        idx[0] += 1

    def entry_update(a):
        tgt = a["target"]
        if tgt in vof:                       # bare-P: reorder-only, immediate
            v = vof[tgt]
            if v in rtm:
                rtm.remove(v); rtm.append(v)
        elif tgt in d_of_idx:                # explicit D tag update (cons)
            d = d_of_idx[tgt]
            old, new = d_tag[d], a["fields"]["tag"]
            d_tag[d] = new
            if alpha(old) and not alpha(new):
                if ("ins", d) in d_staged:   # alpha-exit of a STAGED ins:
                    d_staged.remove(("ins", d))  # staging-level annihilation
                    counter_dec()            # (counter moves, no signal)
                else:
                    stage_d_del_deferred(d)  # processed D: deferred retract
            elif not alpha(old) and alpha(new):
                stage_d_ins(d)               # alpha-admit: a fresh ins
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
            d = d_of_idx.pop(tgt)
            if alpha(d_tag[d]) and (("ins", d) in d_staged or d in d_alive):
                stage_d_del_sync(d, wm=True)
            d_tag.pop(d, None)
        elif tgt in e1:                      # explicit E1: J-match cancel ->
            meta = e1.pop(tgt)               # TMS cascade retract (no signal)
            if meta["d"] is not None:
                stage_d_del_sync(meta["d"], wm=False)

    def fire_all():
        if queued():
            eval_ne(fire_loop=True)
            consume()
        drain_agenda()
        # QUIESCENCE: staged witness ops left unprocessed at the window's
        # end evaluate now even with nothing queued — an unsatisfy crossing
        # a fire boundary is OBSERVED here (ex5003x276/ex5002x56: the next
        # window's re-ins then re-fires the memory), while a same-window
        # del+ins pair has already coalesced at the mid-drain eval
        # (ex5001x170/x79)
        if d_staged:
            eval_ne(fire_loop=True)
            consume()
            drain_agenda()

    for f in scn["facts"]:
        entry_insert(f)
    tr("== fire 0 ==")
    fire_all()
    for i, ep in enumerate(scn["epochs"]):
        for a in ep["actions"]:
            if a["op"] == "update":
                entry_update(a)
            elif a["op"] == "insert":
                entry_insert(a)
            elif a["op"] == "delete":
                entry_delete(a)
        for f in ep["facts"]:
            entry_insert(f)
        tr(f"== fire {i+1} == staged={staged} d_staged={d_staged} "
           f"q={queued()} c={counter[0]} fired={fired}")
        fire_all()
    return fired

def predict(scn, model=None):
    import os as _os
    model = model or _os.environ.get("EMODEL", "epoch")
    if model == "flush":
        return predict_flush(scn)
    if model == "pexists":
        return predict_pexists(scn)
    seg = [0]
    epoch = [0]       # fire-boundary (scenario epoch) index
    gidx = {}; ins_seg = {}; upd_seg = {}; upd_app = {}; is_upd = {}
    ins_epoch = {}; upd_epoch = {}
    vof = {}; idx = [0]; app = [0]
    ts_of = {}        # E1 fact idx -> deadline (ts + 100)
    live = set()      # live E1 fact indices
    clock = [0]
    firings = []
    fired = [False]  # has the exists been satisfied (fired) before?

    def order_now():
        ps = list(gidx)
        if not fired[0]:
            # FIRST satisfaction: fire the accumulated P's FIFO (insertion order).
            # Only after the witness TOGGLES do the epoch batches reverse.
            return sorted(ps, key=lambda v: gidx[v])
        if model == "epoch":
            # D-140-style EPOCH batches: batch = last-touch epoch; reverse
            # (newest first), the INITIAL epoch (0) LAST; within a batch INSERTS
            # (gidx) then UPDATES (newest apply first).
            batch = {v: (upd_epoch[v] if is_upd.get(v) else ins_epoch[v]) for v in ps}
            ebs = sorted({batch[v] for v in ps if batch[v] >= 1}, reverse=True)
            out = []
            for b in ebs + [0]:
                mem = [v for v in ps if batch[v] == b]
                # within-batch inserts: ins_seg DESC then gidx — a P inserted
                # after a mid-epoch witness arrival precedes an earlier one
                # (expop_ins residuals ex801x145/x150/x42)
                inss = sorted((v for v in mem if not is_upd.get(v)), key=lambda v: (-ins_seg[v], gidx[v]))
                upds = sorted((v for v in mem if is_upd.get(v)), key=lambda v: -upd_app[v])
                out.extend(inss + upds)
            return out
        # seg model (mirror of not, within-segment flipped)
        seg_of = {}; role = {}
        for v in ps:
            if is_upd.get(v):
                seg_of[v] = upd_seg[v]; role[v] = 1
            else:
                seg_of[v] = ins_seg[v]; role[v] = 0
        out = []
        for s in sorted({seg_of[v] for v in ps}, reverse=True):
            mem = [v for v in ps if seg_of[v] == s]
            upds = sorted((v for v in mem if role[v] == 1), key=lambda v: -upd_app[v])
            inss = sorted((v for v in mem if role[v] == 0), key=lambda v: gidx[v])
            out.extend(upds + inss)
        return out

    def do_insert(f, fi):
        if f["type"] == "E1":
            # expiry check happens at advance; here just track liveness
            was = len(live)
            live.add(fi)
            ts_of[fi] = f["fields"]["ts"] + 100
            if was == 0:            # unsatisfied -> satisfied: RE-FIRE (or cycle 1)
                firings.extend(order_now())
                fired[0] = True
            seg[0] += 1             # segment boundary (like not's E0 insert)
        else:  # P
            v = f["fields"]["v"]; vof[fi] = v
            gidx[v] = idx[0]; ins_seg[v] = seg[0]; upd_seg[v] = seg[0]; is_upd[v] = False
            ins_epoch[v] = epoch[0]; upd_epoch[v] = epoch[0]
            if live:                 # inserted while SATISFIED: fires immediately
                firings.append(v)    # (fresh stream insert, arrival order)

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg[0]; upd_app[v] = app[0]; is_upd[v] = True; app[0] += 1
            upd_epoch[v] = epoch[0]

    def do_delete(a):
        live.discard(a["target"])

    def do_advance(ms):
        clock[0] += ms
        for fi in list(live):
            if clock[0] >= ts_of[fi]:
                live.discard(fi)

    fi = 0
    for f in scn["facts"]:
        do_insert(f, fi); fi += 1
    for ep in scn["epochs"]:
        epoch[0] += 1
        for a in ep["actions"]:
            if a["op"] == "update": do_update(a)
            elif a["op"] == "delete": do_delete(a)
            elif a["op"] == "advance": do_advance(a["ms"])
            elif a["op"] == "insert": do_insert(a, fi); fi += 1
        for f in ep["facts"]:
            do_insert(f, fi); fi += 1
    return firings


def main():
    pop = json.load(open(sys.argv[1]))
    bad = 0
    for e in pop:
        got = predict(e["scenario"])
        want = [v for v in e["firings"] if v is not None]
        if got != want:
            bad += 1
            if bad <= 14:
                print(f"  MISMATCH {e['scenario']['name']:12} predict={got} oracle={want}")
    print(f"{len(pop)} scenarios: {'ALL MATCH ✓' if bad == 0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
