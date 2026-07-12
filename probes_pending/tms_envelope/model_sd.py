#!/usr/bin/env python3
"""model_sd — executable spec of the (cloud x self-defeat) TMS regime (D-189).

THE UNIFIED TABLE (post-c3d; supersedes the rung-2 three-clause wording):

  1. QUEUE-HEAD DISCIPLINE: the executor always fires from the queue
     head; the queue orders by (salience desc, DECLARATION position);
     an item fires its tuple list one firing at a time, re-consulting
     the queue after each — it continues iff still head. A LAZY
     justifier's self-defeat belief drop lands when its item next
     reaches the head (its pop); an EAGER (no-loop) justifier's drop
     lands when its firing run ends. Same-salience observers therefore
     glimpse the transient iff their declaration position precedes the
     justifier's (rung 1); strictly-higher always; strictly-lower never.
  2. IN-FIRING SELF-CANCELLATION: the moment a justifier's insert
     breaks its own not, its remaining same-item tuples (fan-out and
     or-twin branches) are suppressed — they never fire (rung 2, B).
  3. t10 LEAK / t15 REVIVE: the drop's un-break of the justifier's OWN
     not revives nothing (suppressed stays suppressed — the dead
     blocker leaks); a left-side P WM change (delete) revives the
     suppressed tuples of every rule with P in its LHS.

  4. MEMBER ORDER (graft-derived, D-189 phases 1-2; zero toggles —
     graft-phase1.md holds the dump evidence; 0-div on 750 fresh
     cases, seeds 6001-6005):
     - PHYSICAL LIST: adds prepend (add-at-head); deletes remove in
       place; a processed break+unbreak fold REVERSES the list.
     - SHARER SPLIT: the first-DECLARED member of a shared beta
       prefix owns the t0 staged-insert list (insertion FIFO); later
       sharers memory-scan the current phys. Same split for obs_join
       twins over the shared [LK x P] join.
     - FOLD STAGING (shared [P x not] group): owner-members holding a
       staging get the PRE-reversal scan; the self-defeated justifier
       stages PRE if it is the t0-owner else POST (ownership, not
       staging-presence — gt8 fold-2); stagingless deleters
       memory-scan the reversed phys.
     - UNSHARED folds (lead-k1 lazy / k0-lazy) stay PENDING until a
       member's eval consumes the scan as its WHOLE continuation
       list, then the phys reverses. A k0-NL same-batch fold NETS OUT
       on the off-path node iff the justifier is declared before the
       deleter; deleter-first-decl churns (x70-class).
     - lead-justifiers and del_join consume insertion order.

  Clause-B fold-site note: fold-at-WM-action vs fold-at-next-eval is
  argued OUTPUT-INVISIBLE in-envelope — every path to a cancelled
  tuple's firing passes an evaluation first; externals land only at
  epoch boundaries. Port-phase code read decides the site.

Scenario dialect (the fuzzer's grammar; mirrors the sd_* cells):
  facts: P(f0=i) list, insertion-ordered.
  rules (decl order = list order), each one of:
    {"kind":"justifier", "sal":int, "eager":bool, "ortwin":bool,
     "k":0|1, "notpos":"lead"|"trail", "breaks":bool}
        k=1: P($x) [not LK]  ->  insertLogical(LK(x, False))
        k=0: [not LK]        ->  insertLogical(LK(7, False))
        ortwin (eager-only in grammar): (not LK) or (not LK), k=0
        breaks=False: inserts LK(x, True) which does NOT break the
        not-guard `not LK(f1 != true)` (no self-defeat; fact persists)
    {"kind":"obs_lk",  "sal":int}    LK($v)            -> {}
    {"kind":"obs_join","sal":int}    LK($v) P($x)      -> {}
    {"kind":"obs_p",   "sal":int}    P($x)             -> {} (inert control)
    {"kind":"del_not", "sal":int}    P($x) not LK      -> delete($p) [no-loop]
    {"kind":"del_join","sal":int}    P($x) LK(f1==false)-> delete($p) [no-loop]
Output: {"firings": [(rule_idx_name, pvalue_or_None), ...],
         "finals": sorted fact list, "runaway": bool}
"""
FIRE_CAP = 300


def simulate(facts, rules):
    P = list(facts)
    P_seq = {v: i for i, v in enumerate(facts)}
    LK = {}      # key -> {"owner": ri, "dep": ..., "zombie": bool, "gen": int}
    lk_gen = [0]                          # bumps on every LK creation
    firings = []
    sup = [set() for _ in rules]
    fired = [set() for _ in rules]
    revived = [set() for _ in rules]
    drops = [[] for _ in rules]           # lazy drops: land at the item's pop
    eager_pend = [[] for _ in rules]      # eager drops: land when the item loses the head
    twin_left = [2 if r.get("ortwin") else None for r in rules]
    # GRAFT-DERIVED ORDER LAYER (D-189 phase 2; graft-phase1/2 docs):
    # one physical list for the shared [P x not-LK] prefix group
    # (trail-justifiers + del_not) and one per private del-group; adds
    # PREPEND; a processed break+unbreak fold REVERSES the list; the
    # first-DECLARED group member owns the t0 staged-insert list
    # (insertion FIFO); a self-defeated justifier's re-adds stage to
    # its own path in PRE-reversal scan order (consumed FIFO on t15
    # revive); an UNSHARED group's fold stays pending until a member's
    # eval consumes the scan inline; everyone else memory-scans the
    # current phys head->tail. lead-justifiers and del_join consume
    # insertion order (their observed datum); obs_p insertion order.
    pmut = {v: 0 for v in facts}          # P.f1 values (A-shape setters)
    prtm = list(reversed(facts))          # join right-memory: head-add;
                                          # UPDATE relocates to TAIL (gt9)
    phys = list(reversed(facts))          # shared group phys, add-at-head
    grp = [i for i, r in enumerate(rules) if
           (r['kind'] == 'justifier' and r.get('k', 1) == 1
            and r.get('notpos', 'trail') == 'trail'
            and r.get('amut') != 'set_break')   # the f1 alpha breaks node sharing
           or r['kind'] == 'del_not']
    t0_owner = grp[0] if grp else None
    jstaged = [None for _ in rules]       # per-rule staged member list
    if t0_owner is not None:
        jstaged[t0_owner] = list(facts)
    pending_fold = [None]                 # unshared-group fold scan, or None
    pf_reversed = [False]                 # True = the fold already reversed
                                          # phys (set_break lane); the consume
                                          # block then must NOT re-reverse
    shared_grp = len(grp) >= 2
    fire_count = 0
    # ⚖ private phys for alpha'd trail set_break justifiers (gt16): the
    # f1 alpha gives them their OWN node list — folds there reverse IT,
    # not the group phys; the touch-revive continuation scans it.
    jphys = [list(reversed(facts))
             if (r["kind"] == "justifier" and r.get("k", 1) == 1
                 and r.get("amut") == "set_break"
                 and r.get("notpos", "trail") == "trail") else None
             for r in rules]

    def lk_breaking_alive():
        return any(k[1] is False for k in LK)

    def group_order(ri, eligible):
        # PURE (tuples() is called on every queue peek): the pending-fold
        # consume happens in the fire path, not here
        if pending_fold[0] is not None:
            return [v for v in pending_fold[0] if v in eligible]
        if jstaged[ri]:
            return [v for v in jstaged[ri] if v in eligible]
        return [v for v in phys if v in eligible]

    def tuples(ri):
        r = rules[ri]
        k = r["kind"]
        if k == "justifier":
            if r.get("ortwin"):
                n = twin_left[ri] if not lk_breaking_alive() else 0
                return [("IF", None)] * (n or 0) if "IF" not in sup[ri] else []
            guard_ok = not lk_breaking_alive()
            if r.get("k", 1) == 0:
                return [("IF", None)] if guard_ok and "IF" not in sup[ri] and "IF" not in fired[ri] else []
            if not guard_ok:
                return []
            eligible = set(v for v in P if v not in sup[ri] and v not in fired[ri]
                           and not (r.get("amut") == "set_break" and pmut.get(v)))
            if r.get("notpos", "trail") == "lead":
                order = [v for v in P if v in eligible]          # insertion (banked)
            elif r.get("amut") == "set_break":
                # private node: t0 insertion order until a touch-revive
                # installs the phys-scan continuation (gt16: fires 1,
                # then 3 — jstaged set by t15_revive)
                order = ([v for v in jstaged[ri] if v in eligible]
                         if jstaged[ri] else [v for v in P if v in eligible])
            else:
                order = group_order(ri, eligible)
            return [(v, v) for v in order]
        if k == "obs_lk":
            return [((lk, LK[lk]["gen"]), None) for lk in list(LK)
                    if (lk, LK[lk]["gen"]) not in fired[ri]]
        if k == "obs_join":
            # sharers consume each generation in MIRROR orders (gt9):
            # owner = reversed rtm-scan, later sharer = rtm-scan
            firstoj = min(rj for rj in range(len(rules))
                          if rules[rj]["kind"] == "obs_join")
            scan = [v for v in prtm if v in P]
            pv = list(reversed(scan)) if ri == firstoj else scan
            return [((lk, LK[lk]["gen"], v), v) for lk in list(LK) for v in pv
                    if (lk, LK[lk]["gen"], v) not in fired[ri]]
        if k == "obs_p":
            return [(v, v) for v in P if v not in fired[ri]]
        if k in ("del_not", "del_join"):
            if k == "del_not" and lk_breaking_alive():
                return []
            if k == "del_join" and not any(key[1] is False for key in LK):
                return []                 # zombies ARE visible (c3d): the
                                          # flag gates cascade immunity only
            eligible = set(v for v in P if v not in fired[ri])
            if k == "del_join":
                order = [v for v in P if v in eligible]          # insertion order
            else:
                order = group_order(ri, eligible)
            return [(v, v) for v in order]
        raise ValueError(k)

    def queued(ri):
        return bool(tuples(ri)) or bool(drops[ri])

    def head():
        q = [ri for ri in range(len(rules)) if queued(ri)]
        return min(q, key=lambda ri: (-rules[ri]["sal"], ri)) if q else None

    def rederive(ri, clear_fired):
        r = rules[ri]
        for v in list(sup[ri]):
            if v != "IF":
                sup[ri].discard(v)
                revived[ri].add(v)
        if clear_fired:
            for v in list(fired[ri]):
                if v in P:
                    fired[ri].discard(v)
                    revived[ri].add(v)

    def retract_lk(key):
        LK.pop(key, None)

    def fold_on_drop(ri):
        r = rules[ri]
        scan = list(phys)
        if r["kind"] == "justifier" and r.get("k", 1) == 1:
            if ri not in grp:
                # UNSHARED PLAIN justifier (lead): eager = same-batch
                # fold nets out on other nodes (gt10); LAZY = the
                # drop's later batch churns them (pending fold, the
                # gt3/d4 machinery — the 7001x114-class regression
                # fix). set_break rules never reach here — their folds
                # go per-landing via fold_setbreak (gt16/gt17).
                churns = not r.get("eager")
                if churns and any(
                        rules[rj]["kind"] == "del_not" for rj in range(len(rules))):
                    pending_fold[0] = scan
            elif r.get("notpos", "trail") == "trail":
                # shared fold (gt5/gt7 dumps): members HOLDING a stale
                # staging get the PRE-reversal scan; the self-defeated
                # justifier WITHOUT one gets the POST-reversal order
                # staged; stagingless deleters memory-scan the reversed
                # phys. Then the phys reverses.
                pre = [v for v in scan]
                post = list(reversed(pre))
                for rj in grp:
                    if rj != ri and jstaged[rj] is not None:
                        jstaged[rj] = list(pre)      # owner-deleters: PRE
                # the self-defeated justifier stages by OWNERSHIP, not by
                # staging-presence (gt8 fold-2: non-owner with leftover
                # staging still gets POST)
                jstaged[ri] = list(pre) if ri == t0_owner else list(post)
                phys.reverse()
            else:
                # lead justifier: its own private nodes re-derive in
                # insertion order (observed); the del-group's [P x not]
                # node fold stays PENDING for the deleter's own eval
                if any(rules[rj]["kind"] == "del_not" for rj in range(len(rules))):
                    pending_fold[0] = scan
        elif r["kind"] == "justifier" and r.get("k", 1) == 0:
            dels = [rj for rj in range(len(rules)) if rules[rj]["kind"] == "del_not"]
            if not dels:
                pass
            elif r.get("eager"):
                # k0-NL same-batch fold NETS OUT on the off-path node iff
                # the justifier is declared BEFORE the deleter (gt6/x11);
                # a deleter declared FIRST churns: pre-scan staging
                # replace + reversal (the x70-class five)
                if any(rj < ri for rj in dels):
                    for rj in dels:
                        if jstaged[rj] is not None:
                            jstaged[rj] = [v for v in scan]
                    phys.reverse()
            else:
                pending_fold[0] = scan

    def fold_setbreak(ri):
        # ⚖ deleter-side fold, set_break lane (gt16/gt17 dumps + x90/
        # x131/x108): each BREAKING LK that LANDED in the network folds
        # the del-group at its death — the scan REPLACES any prior
        # pending fold, the group phys reverses. nb keys never fold
        # (gt17: the deleter's t0 staging survives FIFO). Scoped to
        # shapes with a del_not consumer (all witnesses); no-consumer
        # compositions are the fuzzer's to flag.
        if any(rules[rj]["kind"] == "del_not" for rj in range(len(rules))):
            pending_fold[0] = list(phys)
            pf_reversed[0] = True         # reversal happens NOW (the dump:
            phys.reverse()                # D's ltm is reversed pre-firing)

    def land_lazy(ri):
        landed_break_plain = False
        for key in drops[ri]:
            retract_lk(key)               # t10: no self-revival for lazy
            if rules[ri].get("ortwin") and not rules[ri].get("eager"):
                sup[ri].discard("IF")     # or-twin lazy: twin re-derives -> runaway
                twin_left[ri] = 2
            if key[1] is False:           # a BREAKING key landed
                if rules[ri].get("amut") == "set_break":
                    fold_setbreak(ri)
                else:
                    landed_break_plain = True
        drops[ri].clear()
        if landed_break_plain:
            fold_on_drop(ri)

    def land_eager(ri):
        r = rules[ri]
        landed_break_plain = False
        for key in eager_pend[ri]:
            retract_lk(key)
            if r.get("notpos", "trail") == "lead" and r.get("k", 1) == 1:
                # flush-time unbreak of an UPSTREAM not re-propagates:
                # tuples re-derive as new objects (fired clears) -> the
                # d3/d1/d5 self-contained runaway when Ps remain
                rederive(ri, clear_fired=True)
            if key[1] is False:
                if r.get("amut") == "set_break":
                    # ⚖ eager set_break cycles at foreign nodes: a
                    # MUTFIRST key never propagated (the D-195 race —
                    # nothing to fold; x119/x30: D consumes t0 order
                    # even D-first); an ILFIRST key propagated — LEAD
                    # still nets out (gt18 dump: D's ltm pristine, zero
                    # folds; x131's 2-fold match was parity coincidence)
                    # and TRAIL folds IFF the del_not is DECLARED BEFORE
                    # the justifier (the c2x2 corners + x56/gt19 vs
                    # x10/x11: sink order — a D-node ahead of J's node
                    # processes the ins before J's cascade nets it; a
                    # D-node behind receives ins+del together). The
                    # mutfirst pop-landed LAST key folds regardless of
                    # decl — it rides drops[] to land_lazy, where its
                    # long-processed insert meets the delete
                    # sequentially (x90/x51/gt13-LK2).
                    if (not r.get("mutfirst")
                            and r.get("notpos", "trail") == "trail"
                            and any(rules[rj]["kind"] == "del_not"
                                    and rj < ri for rj in range(len(rules)))):
                        fold_setbreak(ri)
                else:
                    landed_break_plain = True
        eager_pend[ri].clear()
        if landed_break_plain:
            fold_on_drop(ri)

    def cascade_p_death(pv):
        for key in list(LK):
            e = LK[key]
            if e["dep"] == pv and not e["zombie"]:
                retract_lk(key)           # D-076 eager cascade (x130)

    def t15_revive(deleted_p, actor=None):
        for rj, r in enumerate(rules):
            if rj == actor:
                continue                  # a self-inflicted delete never
                                          # revives the actor's own tuples
                                          # (7001 census; kin of fz_42_2442)
            if r["kind"] == "justifier" and r.get("k", 1) == 1 \
               and not r.get("ortwin") and not r.get("eager") \
               and r.get("breaks", True):
                # only tuples that DIED in the defeat churn re-derive as new
                # objects (d4); a non-breaking justifier's fired tuples never
                # died, so nothing refires (x52/x68/x130)
                if r.get("amut") == "set_break" and pmut.get(deleted_p):
                    # ⚖ the alpha gate (gt16 F3 / x108): a pmut'd P is
                    # alpha'd out of the set_break rule's node — its
                    # delete never TOUCHES the node, the stale breaker
                    # right-tuple stays staged, no revive (the
                    # starvation law; lead and trail alike)
                    continue
                rederive(rj, clear_fired=True)
                if r.get("amut") == "set_break" \
                   and r.get("notpos", "trail") == "trail":
                    # ⚖ private-node continuation (gt16): the touch-
                    # eval re-adds in the rule's OWN pre-fold phys scan
                    # order, then that phys reverses; lead re-derives
                    # in insertion order (banked, x108)
                    jstaged[rj] = [v for v in jphys[rj]
                                   if v in P and not pmut.get(v)
                                   and v != deleted_p]
                    if not jstaged[rj]:
                        jstaged[rj] = None
                    jphys[rj].reverse()

    steps = 0
    while True:
        steps += 1
        if steps > FIRE_CAP:
            return {"firings": firings, "finals": None, "runaway": True}
        ri = head()
        while True:                       # run-end drops land BEFORE the next
            changed = False               # selection commits (sdp6003x67)
            for rj in range(len(rules)):
                if rj != ri and eager_pend[rj]:
                    land_eager(rj)
                    changed = True
            if not changed:
                break
            ri = head()
        if ri is None:
            break
        land_lazy(ri)
        ts = tuples(ri)
        if not ts:
            continue
        key, pval = ts[0]
        in_group = (rules[ri]["kind"] == "del_not"
                    or (rules[ri]["kind"] == "justifier" and rules[ri].get("k", 1) == 1
                        and rules[ri].get("notpos", "trail") == "trail"))
        if in_group and pending_fold[0] is not None:
            # this member's eval consumed the fold inline: the re-add scan
            # is its WHOLE continuation list (minus the fired head), the
            # node processes the re-adds, the phys reverses (gt3/d4 + the
            # population's RD-continuation signature)
            jstaged[ri] = [v for v in pending_fold[0] if v != key and v in P]
            if not jstaged[ri]:
                jstaged[ri] = None
            pending_fold[0] = None
            if not pf_reversed[0]:        # plain-lane folds reverse at
                phys.reverse()            # consume; set_break folds already did
            pf_reversed[0] = False
        elif jstaged[ri] and key in jstaged[ri]:
            jstaged[ri].remove(key)
            if not jstaged[ri]:
                jstaged[ri] = None
        fired[ri].add(key)
        fire_count += 1
        name = rules[ri].get("name", f"R{ri}")
        r = rules[ri]
        if r["kind"] == "justifier":
            firings.append((name, pval))
            if r.get("ortwin"):
                twin_left[ri] -= 1
            ins_val = pval if r.get("k", 1) == 1 else 7
            breaks = r.get("breaks", True)
            lk_key = (ins_val, False if breaks else True)
            if lk_key not in LK:
                lk_gen[0] += 1
                LK[lk_key] = {"owner": ri, "dep": key, "zombie": False,
                              "gen": lk_gen[0]}
            if breaks:
                if r.get("ortwin"):
                    sup[ri].add("IF")
                else:
                    for k2, _ in ts[1:]:
                        sup[ri].add(k2)
                LK[lk_key]["zombie"] = True      # dep cancelled at the break
                if r.get("eager"):
                    eager_pend[ri].append(lk_key)
                else:
                    drops[ri].append(lk_key)
            amut = r.get("amut")
            if amut and pval is not None:
                # A-shape RHS mutation: a self-inflicted DELETE carries the
                # dep EAGERLY (plain cascade — gt12: the obs never glimpses);
                # a self-inflicted UPDATE-break defers (fz_42_2442's actual
                # shape) — zombie + lazy/run-end drop.
                if amut == "set_break":
                    was_zombie = LK[lk_key]["zombie"]
                    pmut[pval] = 1
                    if pval in prtm:              # join-rtm relocation (gt9);
                        prtm.remove(pval)         # not-ltm stays in place (gt10)
                        prtm.append(pval)
                    # ⚖ eval-consumption landing (gt13/gt14 dumps + x51,
                    # tmslens-results.md): the update-break's dep-teardown
                    # lands at the justifier's next NETWORK EVAL. Mid-run
                    # (revivable Ps remain) that eval precedes the next
                    # firing — the loses-head landing nets the LK out
                    # unseen; a LAST-firing break waits for the item's
                    # next POP: strictly-higher observers fire on the
                    # zombie, sub-salience observers never do (gt14).
                    # tuples(ri) can't be the mid-run test — a live
                    # breaking LK gates the guard — so: any pmut-unset P.
                    # Lazy routing untouched (no dump evidence yet).
                    survivors = any(not pmut.get(v) for v in P)
                    if not was_zombie:            # pure lane (breaks=False)
                        LK[lk_key]["zombie"] = True
                        # the last-key pop window: closed ONLY for the
                        # ilfirst TRAIL corner (x147: obs@7 never fires);
                        # ilfirst LEAD keeps it (x6001x131 obs_lk once,
                        # x7004x92 obs_join across the full P-scan), as
                        # does mutfirst (gt13's RO2 window)
                        if r.get("eager") and (survivors or (
                                not r.get("mutfirst")
                                and r.get("notpos", "trail") == "trail")):
                            eager_pend[ri].append(lk_key)
                        else:
                            drops[ri].append(lk_key)
                    elif (r.get("eager") and r.get("mutfirst")
                          and not survivors and lk_key in eager_pend[ri]):
                        # composite (breaks=True + set_break) — MUTFIRST
                        # ONLY (x51 vs the x26/x58/x71/x95 quartet): the
                        # staged pair drains in RHS order, so ilfirst
                        # lets the insert's not-break reach the tuple
                        # FIRST (D-076 eager cascade at propagation — no
                        # window), while mutfirst stages the update's
                        # join-break first (lazy eval-consumption — the
                        # last cycle's drop rides to the pop). Mid-cycle
                        # drops stay at loses-head either way (the
                        # suppress/revive machinery needs the LK dead
                        # before the refire — sd_d* certified).
                        eager_pend[ri].remove(lk_key)
                        drops[ri].append(lk_key)
                elif amut == "del":
                    route_needed = not LK[lk_key]["zombie"]
                    if route_needed:
                        LK[lk_key]["zombie"] = True
                    P.remove(pval)
                    if pval in phys:
                        phys.remove(pval)
                    if pval in prtm:
                        prtm.remove(pval)
                    for rj in range(len(rules)):
                        if jstaged[rj] and pval in jstaged[rj]:
                            jstaged[rj].remove(pval)
                        if jphys[rj] and pval in jphys[rj]:
                            jphys[rj].remove(pval)
                    if route_needed:
                        # ⚖ del-lane eval-consumption (x88/x0 windows;
                        # x66 the eager mid-run net-out; gt12's "eager
                        # cascade" was salience-confounded — its observer
                        # sat BELOW the justifier): same split as
                        # set_break — an EAGER justifier's mid-run del
                        # nets out at the between-firings eval, the last
                        # firing (and every LAZY firing) rides to the
                        # POP; higher-salience observers glimpse. The
                        # foreign cascade below stays D-076-eager (x130).
                        if r.get("eager") and tuples(ri):
                            eager_pend[ri].append(lk_key)
                        else:
                            drops[ri].append(lk_key)
                    elif (r.get("eager") and r.get("mutfirst")
                          and not any(v not in fired[ri] for v in P)
                          and lk_key in eager_pend[ri]):
                        # del+breaks composite (x79/x98): the D-195
                        # last-key pop re-route, mirrored — the final
                        # generation's LK rides to the pop; strictly-
                        # higher observers glimpse it once. ilfirst
                        # composites stay loses-head (no witness).
                        # (tuples(ri) can't be the survivors test — the
                        # live breaking LK gates the guard; unfired-P is.)
                        eager_pend[ri].remove(lk_key)
                        drops[ri].append(lk_key)
                    cascade_p_death(pval)
                    t15_revive(pval, actor=ri)
        elif r["kind"] in ("obs_lk", "obs_join", "obs_p"):
            firings.append((name, pval))
        elif r["kind"] in ("del_not", "del_join"):
            firings.append((name, pval))
            P.remove(pval)
            if pval in prtm:
                prtm.remove(pval)
            if pval in phys:
                phys.remove(pval)
            for rj in range(len(rules)):
                if jstaged[rj] and pval in jstaged[rj]:
                    jstaged[rj].remove(pval)
                if jphys[rj] and pval in jphys[rj]:
                    jphys[rj].remove(pval)
            cascade_p_death(pval)
            t15_revive(pval)
    finals = sorted([("P", v) for v in P] + [("LK", k[0]) for k in LK])
    return {"firings": firings, "finals": finals, "runaway": False}


if __name__ == "__main__":
    import sys
    print("importable module; validation drives it via validate_cells.py", file=sys.stderr)
