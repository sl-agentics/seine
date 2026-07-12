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

  Member order (empirical, population-refinable): trailing-not and
  join items enumerate P-tuples in INSERTION order at first
  derivation; REVIVED tuples enumerate LIFO (latest P first);
  leading-not items enumerate LIFO always (3060).

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
    derive_seq = [0 for _ in rules]       # OTHERS-fires snapshot at last (re)derivation
    others = [0 for _ in rules]           # firings by OTHER rules
    churned = [False for _ in rules]      # guard reopened by a landed drop
    yielded = [False for _ in rules]      # fired a tuple, then another rule fired
    has_fired_any = [False for _ in rules]
    fire_count = 0
    has_lead_just = any(r["kind"] == "justifier" and r.get("k", 1) == 1
                        and r.get("notpos", "trail") == "lead" for r in rules)

    def lk_breaking_alive():
        return any(k[1] is False for k in LK)

    def stale(ri):
        return others[ri] > derive_seq[ri]

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
            avail = [v for v in P if v not in sup[ri] and v not in fired[ri]]
            rev = sorted([v for v in avail if v in revived[ri]], key=lambda v: -P_seq[v])
            ini = [v for v in avail if v not in revived[ri]]
            avail = rev + ini if rev else ini
            return [(v, v) for v in avail]
        if k == "obs_lk":
            return [((lk, LK[lk]["gen"]), None) for lk in list(LK)
                    if (lk, LK[lk]["gen"]) not in fired[ri]]
        if k == "obs_join":
            return [((lk, LK[lk]["gen"], v), v) for lk in list(LK) for v in P
                    if (lk, LK[lk]["gen"], v) not in fired[ri]]
        if k == "obs_p":
            return [(v, v) for v in P if v not in fired[ri]]
        if k in ("del_not", "del_join"):
            if k == "del_not" and lk_breaking_alive():
                return []
            if k == "del_join" and not any(key[1] is False for key in LK):
                return []                 # zombies ARE visible (c3d): the
                                          # flag gates cascade immunity only
            avail = [v for v in P if v not in fired[ri]]
            # member order (empirical): LIFO once the item has YIELDED
            # mid-list (c1/c3a rounds 2+), or when its tuple set is STALE
            # with no churn-re-derivation since t0 (x130-class: other
            # rules fired, guard never churned). Fresh/churned first
            # rounds are FIFO (c3b/c3d/c1-round-1). OPEN 1-cell corner:
            # d4's round-1 LIFO after a LEAD-lazy churn — documented,
            # not encoded (order-only).
            if yielded[ri] or stale(ri) \
               or (k == "del_not" and churned[ri] and has_lead_just):
                avail = sorted(avail, key=lambda v: -P_seq[v])
            return [(v, v) for v in avail]
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
        derive_seq[ri] = fire_count

    def retract_lk(key):
        LK.pop(key, None)

    def land_lazy(ri):
        landed = bool(drops[ri])
        for key in drops[ri]:
            retract_lk(key)               # t10: no self-revival for lazy
            if rules[ri].get("ortwin") and not rules[ri].get("eager"):
                sup[ri].discard("IF")     # or-twin lazy: twin re-derives -> runaway
                twin_left[ri] = 2
        drops[ri].clear()
        if not landed:
            return
        # a drop actually landed: guard reopen re-derives del_not memories
        for rj, r in enumerate(rules):
            if r["kind"] in ("del_not",):
                derive_seq[rj] = others[rj]
                churned[rj] = True

    def land_eager(ri):
        r = rules[ri]
        landed = bool(eager_pend[ri])
        for key in eager_pend[ri]:
            retract_lk(key)
            if r.get("notpos", "trail") == "lead" and r.get("k", 1) == 1:
                # flush-time unbreak of an UPSTREAM not re-propagates:
                # tuples re-derive as new objects (fired clears) -> the
                # d3/d1/d5 self-contained runaway when Ps remain
                rederive(ri, clear_fired=True)
        eager_pend[ri].clear()
        if not landed:
            return
        for rj, r2 in enumerate(rules):
            if r2["kind"] in ("del_not",):
                derive_seq[rj] = others[rj]
                churned[rj] = True

    def cascade_p_death(pv):
        for key in list(LK):
            e = LK[key]
            if e["dep"] == pv and not e["zombie"]:
                retract_lk(key)           # D-076 eager cascade (x130)

    def t15_revive(deleted_p):
        for rj, r in enumerate(rules):
            if r["kind"] == "justifier" and r.get("k", 1) == 1 \
               and not r.get("ortwin") and not r.get("eager") \
               and r.get("breaks", True):
                # only tuples that DIED in the defeat churn re-derive as new
                # objects (d4); a non-breaking justifier's fired tuples never
                # died, so nothing refires (x52/x68/x130)
                rederive(rj, clear_fired=True)

    steps = 0
    while True:
        steps += 1
        if steps > FIRE_CAP:
            return {"firings": firings, "finals": None, "runaway": True}
        ri = head()
        for rj in range(len(rules)):      # eager drops land on losing the head
            if rj != ri and eager_pend[rj]:
                land_eager(rj)
        if ri is None:
            ri = head()                   # eager landing may re-derive work
            if ri is None:
                break
        land_lazy(ri)
        ts = tuples(ri)
        if not ts:
            continue
        key, pval = ts[0]
        for rj in range(len(rules)):
            if rj != ri:
                if has_fired_any[rj]:
                    yielded[rj] = True
                others[rj] += 1
        has_fired_any[ri] = True
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
                for rj, r2 in enumerate(rules):
                    if r2["kind"] == "del_join":
                        derive_seq[rj] = others[rj] + 1  # this firing counts
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
        elif r["kind"] in ("obs_lk", "obs_join", "obs_p"):
            firings.append((name, pval))
        elif r["kind"] in ("del_not", "del_join"):
            firings.append((name, pval))
            P.remove(pval)
            cascade_p_death(pval)
            t15_revive(pval)
    finals = sorted([("P", v) for v in P] + [("LK", k[0]) for k in LK])
    return {"firings": firings, "finals": finals, "runaway": False}


if __name__ == "__main__":
    import sys
    print("importable module; validation drives it via validate_cells.py", file=sys.stderr)
