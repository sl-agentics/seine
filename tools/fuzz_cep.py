#!/usr/bin/env python3
"""CEP E1 fuzz (D-101): draws deterministic point-event scenarios —
explicit @expires everywhere (inference is E2-fenced, a8), pseudo-clock
advances, after/before temporal joins, TMS justification off events
(the a6/a7 composition), not/exists over events — and differentials
full canonical outputs (firings + final WM) through the standard
`seine-harness diff` batches (one oracle JVM per batch).

Covers (D-109): @expires INFERENCE — event types drawn WITHOUT
expires_ms get their reach inferred from the temporal constraints
(earlier=hi, later=-lo→leak, MAX-merge, transitive CHAINS via the STP
closure), mixed explicit+inferred within a scenario.

Covers (D-110/D-112): accumulate over event streams — windowed
(window:time(N), per-subtree eviction at ts+N + the A→B seam) OR plain —
whose source events also arrive in epochs, exercising the ACCUMULATE-EAGER
removal (a window eviction or expiration drops the count/sum at
advance-time, before the epoch's inserts and firing by salience, not
deferred to quiescence). count()/sum(), optional source constraint
(filter-first) and salience (cross-rule ordering).

Covers (CEP E2 item C, this slab): external event UPDATE / DELETE — an
epoch action mutates ({"op":"update","target":k,"fields":{...}}) or
retracts ({"op":"delete","target":k}) an already-inserted fact, composing
with expiration deadlines, windows, accumulate-eager removal, not-CE,
temporal joins and the TMS cascade. Targeting is SOUND-BY-CONSTRUCTION:
  * only the INITIAL-FACTS PREFIX [0,k) is targeted — those handles are
    inserted before any fire, so their visible-insertion index (which the
    runner + oracle both key on, INCLUDING later rule-inserted D/P3 facts)
    is stable and firing-INDEPENDENT; epoch/RHS facts are never targeted.
  * only PROVABLY-LIVE facts are targeted — P never expires; an explicit-
    expiry event is targeted only while clock < ts+expires (strictly
    before its earliest possible deadline); inferred-expiry events are not
    targeted (their reach is not modeled here); a deleted target leaves
    the pool. So neither engine ever mutates a dead/expired handle — that
    edge (delete-of-dead: Drools no-ops, engine errors; update-of-expired:
    engine revives the event into the accumulate; update-of-deleted: both
    error) is pinned by the c_* recon probes + a D-entry, NOT fuzzed.
  * a reset clears the pool (post-reset WM is empty; the index restarts).

Fences honored: window:length + standalone-pattern windows (follow-on
slab); event timestamps drawn at-or-after the current scenario clock (the
past-deadline insert edge is unprobed); distinct-or-tied deadlines both
drawn (a2: ties are pinned stable); reset+window not co-drawn (D-114
reset×WindowNode incoherence); reset+mutate not co-hazardous (pool
cleared on reset).

Usage: .venv/bin/python tools/fuzz_cep.py <n> <seed>
"""
import json
import os
import random
import subprocess
import sys

BATCH = 150
INF = 1 << 60
# D-166: SEINE_TJUPD=1 — the UPDATE-RECENCY axis (tj-tail family): every
# scenario mutates, one update per epoch (biased to temporal event types),
# more initial facts (partner multiplicity) and guaranteed fresh arrivals
# (the enumeration observation point). All overrides are applied AFTER the
# default RNG draws so the flag-off stream is byte-identical.
TJUPD = bool(os.environ.get("SEINE_TJUPD"))
# A batch that exceeds this is presumed to contain a NON-TERMINATING scenario
# (an engine spin the fire limit can't catch — a rare pre-existing latent). We
# bisect it out (per-scenario engine `run`) so the campaign completes rather
# than wedging (the memory HANG protocol).
BATCH_TIMEOUT = 200
SCN_TIMEOUT = 8


class Gen:
    def __init__(self, rng):
        self.r = rng

    def ep_suf(self, t):
        """CEP E2 item D: ` from entry-point "Sk"` suffix for an event pattern
        of type t (empty for DEFAULT / non-EP scenarios). Goes at the END of a
        pattern (after constraints / temporal / window)."""
        ep = self.type_ep.get(t) if self.use_ep else None
        return f' from entry-point "{ep}"' if ep else ''

    def efact(self, t, fields):
        """An event fact routed into its type's entry point (item D) — ONLY
        when a rule actually references that EP (else it's an insert into an
        unreferenced entry point, which both engines correctly reject as out
        of subset)."""
        # CEP E2 item E: interval events carry a @duration value, fixed at
        # insert (dur mutation is the item-C fence — updates never set it).
        # dur=0 is drawn too (the point-equivalence path).
        if t in self.dur_types and "dur" not in fields:
            fields["dur"] = self.r.choice([0, 10, 20, 30, 50, 80])
        fd = {"type": t, "fields": fields}
        ep = self.type_ep.get(t) if self.use_ep else None
        if ep and ep in self.ep_referenced:
            fd["entry_point"] = ep
        return fd

    # CEP E2 item E (D-119): the 11 Allen ops → their valid param arities.
    ALLEN = {
        "coincides": (0, 1, 2), "meets": (0, 1), "metby": (0, 1),
        "overlaps": (0, 1, 2), "overlappedby": (0, 1, 2),
        "during": (0, 1, 2, 4), "includes": (0, 1, 2, 4),
        "starts": (0, 1), "startedby": (0, 1),
        "finishes": (0, 1), "finishedby": (0, 1),
    }

    def allen_pred(self):
        """A random Allen op + a valid param list (D-119). Pairs are ordered
        lo<=hi (min/max, the two during/includes windows); coincides' two
        tolerances are order-independent. Returns the `this <op>[..]` text."""
        r = self.r
        op = r.choice(list(self.ALLEN))
        k = r.choice(self.ALLEN[op])
        pool = [0, 5, 10, 20, 30, 50, 80]
        if k == 0:
            return f"this {op}"
        if k == 1:
            ps = [r.choice(pool)]
        elif k == 2:
            ps = sorted(r.choice(pool) for _ in range(2))
        else:  # k == 4: two ordered [lo,hi] pairs
            ps = sorted(r.choice(pool) for _ in range(2)) + sorted(r.choice(pool) for _ in range(2))
        return f"this {op}[" + ",".join(f"{p}ms" for p in ps) + "]"

    def scenario(self, name):
        r = self.r
        # CEP E2 item C: half the scenarios draw external update/delete
        # actions over the initial-facts prefix (see module docstring for
        # the soundness invariant).
        self.mutate = r.random() < 0.5
        if TJUPD:
            self.mutate = True
        # types: 2-3 event types + 1 plain + logical target. D-109: in an
        # inference scenario, most event types OMIT expires_ms (engine
        # infers the reach); a minority stay explicit (mixed path).
        infer = r.random() < 0.5
        n_ev = r.randint(2, 3)
        # D-114: a RESET of a windowed accumulate that held a value fires a
        # spurious extra [0] in Drools (plain accumulates don't) — a fenced
        # Drools reset×WindowNode INCOHERENCE (scenarios/xfail/
        # xf_win_reset_incoherence). Don't draw both in one scenario.
        self.has_window = False
        self.etypes = []
        self.type_expiry = {}  # type name -> explicit expires_ms, or None
        self.dur_types = set()  # CEP E2 item E: types declared @duration(dur)
        types = []
        for i in range(n_ev):
            ev = {"timestamp": "ts"}
            if not infer or r.random() < 0.3:
                ev["expires_ms"] = r.choice([50, 100, 100, 200, 400])
            fields = [{"name": "ts", "type": "i64"}, {"name": "tag", "type": "String"}]
            # CEP E2 item E (D-118): ~45% of event types are INTERVALS
            # occupying [ts, ts+dur] (the endTS drives after/before distance,
            # Allen predicates, and the +dur expiration shift); the rest are
            # points (no @duration ⇒ dur 0 ⇒ byte-identical to pre-item-E).
            if r.random() < 0.45:
                ev["duration"] = "dur"
                fields.insert(1, {"name": "dur", "type": "i64"})
                self.dur_types.add(f"E{i}")
            types.append({"name": f"E{i}", "fields": fields, "event": ev})
            self.etypes.append(f"E{i}")
            self.type_expiry[f"E{i}"] = ev.get("expires_ms")
        types.append({"name": "P", "fields": [{"name": "v", "type": "i64"}]})
        types.append({"name": "D", "fields": [{"name": "tag", "type": "String"}]})
        types.append({"name": "P3", "fields": [{"name": "v", "type": "i64"}]})
        # D-317: the windowed/plain acc-JUSTIFIER axis (D-312/D-316 lifts) —
        # a purely-logical type fed only by W rules' insertLogical; the
        # eviction/expiration swap is the patrolled surface.
        types.append({"name": "DW", "fields": [{"name": "v", "type": "i64"}]})

        # CEP E2 item C (D-115) FENCE sets: event types whose external
        # UPDATE/DELETE composition is xfail'd (classes 1/2/3) and must NOT be
        # fuzzed. Populated during rule generation below.
        self.temporal_types = set()      # class 1: after/before join re-fire
        self.windowed_acc_types = set()  # class 2: evicted/expired revival
        self.exists_types = set()        # class 3: exists witness churn
        self.allen_types = set()         # item E: types under a NEW Allen op
        # CEP E2 item D: partition SOME scenarios' event types across named
        # entry points (DEFAULT + S1/S2) — a routing dimension that composes
        # with every rule + mutation. A pattern of type T and a fact of type T
        # both carry T's entry point (self.ep_suf / the fact `entry_point`).
        self.use_ep = r.random() < 0.5
        self.type_ep = {t: (r.choice([None, None, "S1", "S2"]) if self.use_ep else None)
                        for t in self.etypes}
        if self.use_ep and not any(self.type_ep.values()):
            self.type_ep[self.etypes[0]] = "S1"
        self.ep_referenced = set()  # EP names a rule actually uses (filled below)
        rules = []
        ri = 0
        # DIAGNOSTIC (CEP_NO_TEMPORAL=1): suppress after/before temporal-join
        # + chain rules, to isolate whether the update axis diverges through
        # ANY node other than temporal Behavior nodes (item-C triage). Not
        # for normal runs — temporal joins are core coverage.
        no_temporal = bool(os.environ.get("CEP_NO_TEMPORAL"))
        # temporal-join rule(s): after/before (D-101) OR a NEW Allen op
        # (item E). The D-120 explicit-@expires fence is LIFTED (D-164):
        # Allen ops now emit their constant interval edges into the STP
        # matrix (the 124-cell reach ladder), so un-annotated types under
        # Allen ops infer exactly Drools' offsets and draw freely.
        for _ in range(0 if no_temporal else r.randint(1, 2)):
            if r.random() < 0.35:
                a, b = r.choice(self.etypes), r.choice(self.etypes)
                self.temporal_types.update((a, b))
                self.allen_types.update((a, b))
                pred = self.allen_pred()
            else:
                a, b = r.choice(self.etypes), r.choice(self.etypes)
                self.temporal_types.update((a, b))
                op = r.choice(["after", "before"])
                lo = r.choice([0, 0, 50])
                hi = lo + r.choice([50, 100, 150])
                pred = f"this {op}[{lo}ms,{hi}ms]"
            cons = f'tag == "{r.choice("xyz")}"' if r.random() < 0.3 else ""
            sal = f" salience {r.randint(-5, 15)}" if r.random() < 0.4 else ""
            rules.append(
                f'rule TJ{ri}{sal} when $a : {a}({cons}){self.ep_suf(a)} '
                f'$b : {b}({pred} $a){self.ep_suf(b)} then end'
            )
            ri += 1
        # D-109: transitive CHAIN E0 -> E1 -> E2 (distinct bindings) —
        # exercises the STP closure (earliest event inherits the SUMMED
        # reach). Per-hop op so after/before/mixed chains all appear.
        if n_ev >= 3 and not no_temporal and r.random() < 0.5:
            self.temporal_types.update(("E0", "E1", "E2"))
            op1, op2 = r.choice(["after", "before"]), r.choice(["after", "before"])
            lo1, lo2 = r.choice([0, 50]), r.choice([0, 50])
            hi1, hi2 = lo1 + r.choice([50, 100]), lo2 + r.choice([50, 100])
            rules.append(
                f'rule CH{ri} when $a : E0(){self.ep_suf("E0")} '
                f'$b : E1(this {op1}[{lo1}ms,{hi1}ms] $a){self.ep_suf("E1")} '
                f'$c : E2(this {op2}[{lo2}ms,{hi2}ms] $b){self.ep_suf("E2")} then end'
            )
            ri += 1
        # D-110/D-112: accumulate over an EVENT stream — windowed (over
        # window:time(N)) OR plain — whose source events ALSO arrive in
        # epochs, so it exercises the ACCUMULATE-EAGER removal: a clock job
        # (window eviction OR expiration) drops the count/sum at
        # advance-time — before the epoch's inserts into the same accumulate
        # (no transient) and firing by SALIENCE, not deferred to quiescence
        # (df_* pins; model_check_accdefer). count()/sum(), optional source
        # constraint (filter-first) and salience (cross-rule ordering).
        for _ in range(r.randint(0, 2)):
            if r.random() < 0.55:
                e = r.choice(self.etypes)
                wsal = f" salience {r.randint(-5, 12)}" if r.random() < 0.4 else ""
                cons = f'tag == "{r.choice("xyz")}"' if r.random() < 0.35 else ""
                win = (f" over window:time({r.choice([50, 100, 100, 150, 200])}ms)"
                       if r.random() < 0.6 else "")
                if win:
                    self.has_window = True
                    self.windowed_acc_types.add(e)
                fdraw = r.random()
                if fdraw < 0.5:
                    src, fn = f"{e}({cons})", "$c : count()"
                elif fdraw < 0.75:
                    inner = (cons + ", " if cons else "") + "$t : ts"
                    src, fn = f"{e}({inner})", "$c : sum($t)"
                else:
                    # D-324: the collectList ORDER axis — tags draw from a
                    # 3-symbol pool, so duplicate values are the norm and
                    # every eviction/expiration exercises the D-323
                    # first-value-equal reverse (pr_co_w* pins); the
                    # Collection rides the firing tuple, order-diffable.
                    inner = (cons + ", " if cons else "") + "$t : tag"
                    src, fn = f"{e}({inner})", "$c : collectList($t)"
                # D-317: ~45% of acc rules JUSTIFY a logical DW with the
                # result (in-subset since D-312; windowed since D-316) — a
                # window eviction / expiration / epoch insert re-fires the
                # SAME act and the refire-supersede swaps the DW; observers
                # make the swap agenda-visible, not/exists composes the
                # teardown with the certified not-CE lanes. Collection
                # results can't feed DW(v i64) — collect draws skip it.
                wj = fdraw < 0.75 and r.random() < 0.45
                rhs = "insertLogical(new DW($c));" if wj else ""
                rules.append(
                    f'rule W{ri}{wsal} when accumulate( {src}{win}{self.ep_suf(e)}; {fn} ) then {rhs} end'
                )
                ri += 1
                if wj:
                    rules.append(f'rule RW{ri} salience {r.randint(-4, 10)} when DW() then end')
                    ri += 1
                    if r.random() < 0.5:
                        rules.append(f'rule NW{ri} salience {r.randint(-8, 8)} when not DW(v >= 1) P() then end')
                        ri += 1
        # TMS justification off an event + observers (a6/a7 shape)
        drew_j = False
        if r.random() < 0.75:
            drew_j = True
            e = r.choice(self.etypes)
            rules.append(f'rule J{ri} when $e : {e}($t : tag){self.ep_suf(e)} then insertLogical(new D($t)); end')
            ri += 1
            rules.append(f'rule RD{ri} salience {r.randint(0, 12)} when D() then end')
            ri += 1
            rules.append(f'rule ND{ri} salience {r.randint(-8, 12)} when not D() P() then end')
            ri += 1
            if r.random() < 0.5:
                # the a7c shape: a same-epoch chain around the cascade
                rules.append(f'rule G{ri} salience {r.randint(8, 20)} when P(v == 2) then insert(new P3(3)); end')
                ri += 1
                rules.append(f'rule C{ri} salience {r.randint(-5, 5)} when P3() then end')
                ri += 1
        # not/exists over events. The D-317 exists-only J-fence is
        # LIFTED (D-321): the probe round refuted the "ND/NE landing
        # split" read — the TMS/J lane is CLEAN (n9_tms_ride), and the
        # real class behind cf317901x11/cf317902x0/x205/cf318903x111
        # is the STALE-LEFT RELEASE: a `not <EVENT>()` observer whose
        # P left is DELETED while a re-block cycle spans the deletion
        # fires the dead left at a later expiration release
        # (xf_ndne_n8_ride canonical; probes_pending/ndne_landing/
        # PINS.md). Finds of that named class re-report until its
        # port lands — bisect against the minimal witnesses.
        if r.random() < 0.6:
            e = r.choice(self.etypes)
            neg = r.choice(["not", "exists"])
            if neg == "exists":
                self.exists_types.add(e)
            sal = f" salience {r.randint(-8, 8)}" if r.random() < 0.5 else ""
            rules.append(f'rule NE{ri}{sal} when {neg} {e}(){self.ep_suf(e)} P() then end')
            ri += 1

        # facts at clock 0. The mutation axis (item C) targets ONLY these
        # initial facts — their visible-insertion index is stable and
        # firing-independent (module docstring). Record each target's
        # earliest deadline for the liveness gate.
        # CEP E2 item D: only route facts to entry points a rule actually
        # references (an unreferenced-EP insert is out of subset — both
        # engines reject it).
        drl_str = "\n".join(rules)
        self.ep_referenced = {ep for ep in ("S1", "S2")
                              if f'entry-point "{ep}"' in drl_str}
        self.targets = []  # {idx, type, deadline, targetable, deleted}
        facts = [{"type": "P", "fields": {"v": 1}}]
        self.targets.append({"idx": 0, "type": "P", "deadline": INF,
                             "targetable": True, "deleted": False})
        nfacts = r.randint(1, 4)
        if TJUPD:
            nfacts += 2
        for _ in range(nfacts):
            t = r.choice(self.etypes)
            ts = r.randint(0, 40)
            facts.append(self.efact(t, {"ts": ts, "tag": r.choice("xyz")}))
            ex = self.type_expiry.get(t)
            self.targets.append({
                "idx": len(facts) - 1, "type": t,
                # earliest possible deadline = ts+ex (a rule-referenced
                # event's real deadline is ts+ex+1, so this is conservative)
                "deadline": (ts + ex) if ex is not None else 0,
                "targetable": ex is not None,  # inferred reach not modeled
                "deleted": False,
            })

        # epochs: advances + fresh events at/after the running clock. The
        # advance pool straddles common inferred boundaries (hi/sum ±1).
        clock = 0
        epochs = []
        for _ in range(r.randint(1, 3)):
            actions = []
            if r.random() < 0.15 and not self.has_window:
                # D-104: in-place session reset — the paged-batch axis.
                # D-114: skipped when a windowed accumulate is present (the
                # reset×WindowNode Drools-incoherence is fenced, not tested).
                actions.append({"op": "reset"})
                clock = 0
                # reset clears WM — every prior handle is gone, so the
                # target pool is emptied (post-reset facts are untargeted).
                for tg in self.targets:
                    tg["deleted"] = True
            if r.random() < 0.9:
                ms = r.choice([30, 49, 50, 51, 99, 100, 101, 149, 150, 151,
                               199, 200, 201, 250, 300, 600])
                actions.append({"op": "advance", "ms": ms})
                clock += ms
            # this epoch's fresh arrivals — drawn BEFORE the mutation so the
            # class-3 exists-churn fence can see which types arrive here.
            efacts = []
            n_arr = r.randint(0, 2)
            if TJUPD:
                n_arr = max(n_arr, 1)
            for _ in range(n_arr):
                t = r.choice(self.etypes)
                efacts.append(self.efact(t, {"ts": clock + r.randint(0, 30), "tag": r.choice("xyz")}))
            if r.random() < 0.3:
                efacts.append({"type": "P", "fields": {"v": 2}})
            epoch_ins = {f["type"] for f in efacts}
            # CEP E2 item C (D-115): external update/delete over a provably-
            # live initial-fact target (liveness reflects the post-advance
            # clock). Classes 1/2/3 are PORTED; only the item-1b temporal-join
            # ORDER latents remain fenced on UPDATE:
            #   UPDATE excludes temporal-join event types — NOT class-1 (that
            #     re-fire is ported, D-137), but the pre-existing temporal-join
            #     ORDER latents (item 1b: cf313 not-order, @duration interval
            #     join-order) surfaced by lifting the fence; all bisect-to-HEAD
            #     byte-identical, un-caused by the item-C ports. Windowed
            #     accumulate is NOW allowed: class-2 evicted/expired revival
            #     (D-137, clock_removed guard) and §1a live-modify property-
            #     reactivity (D-139, windowed = bindings-only watch mask) both
            #     ported.
            #   DELETE excludes an exists-witness churn (deleting an
            #     exists_type event while a same-type event arrives here).
            # (class-2 EXPIRATION, update-of-deleted and double-delete are
            # already unreachable — the liveness gate never targets a past-
            # deadline or deleted handle; delete-of-dead now no-ops anyway.)
            if self.mutate and ((r.random() < 0.6) or TJUPD):
                live = [tg for tg in self.targets
                        if tg["targetable"] and not tg["deleted"]
                        and clock < tg["deadline"]]
                # D-157: the temporal-type UPDATE fence is LIFTED by default
                # — every family it guarded is closed (D-141 tj-ts, D-143..153
                # existential order, D-154/155 A2 winacc, D-156 tj pair-order).
                upd_ok = list(live)
                del_ok = list(live)  # D-138: class-3 external exists-witness churn PORTED

                def pick(pool):  # bias toward EVENT targets (the item-C heart)
                    if TJUPD:  # D-166: bias to temporal-join participant types
                        tj = [t for t in pool if t["type"] in self.temporal_types]
                        if tj and r.random() < 0.85:
                            return r.choice(tj)
                    ev = [t for t in pool if t["type"] != "P"]
                    return r.choice(ev) if ev and r.random() < 0.75 else r.choice(pool)

                if upd_ok and del_ok:
                    op = "delete" if r.random() < 0.4 else "update"
                    if TJUPD and r.random() < 0.85:
                        op = "update"
                elif del_ok:
                    op = "delete"
                elif upd_ok:
                    op = "update"
                else:
                    op = None
                if op == "delete":
                    tg = pick(del_ok)
                    actions.append({"op": "delete", "target": tg["idx"]})
                    tg["deleted"] = True
                elif op == "update":
                    tg = pick(upd_ok)
                    if tg["type"] == "P":
                        fields = {"v": r.choice([1, 2, 3])}
                    else:
                        fields = {}
                        if r.random() < 0.7:
                            fields["tag"] = r.choice("xyz")
                        if r.random() < 0.5:
                            # ts-field update: deadline FIXED at insert (c1)
                            fields["ts"] = clock + r.randint(0, 30)
                        if not fields:
                            fields["tag"] = r.choice("xyz")
                    actions.append({"op": "update", "target": tg["idx"], "fields": fields})
            epochs.append({"actions": actions, "facts": efacts})

        return {"name": name, "types": types, "drl": "\n".join(rules) + "\n",
                "facts": facts, "epochs": epochs}


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    tmp = os.environ.get("FUZZ_TMP", "/tmp") + f"/cepfuzz_{seed}"
    os.makedirs(tmp, exist_ok=True)
    fails = 0
    hangs = 0
    done = 0
    while done < n:
        batch = []
        for i in range(done, min(done + BATCH, n)):
            scn = Gen(random.Random(seed * 7_654_321 + i)).scenario(f"cf{seed}x{i}")
            path = f"{tmp}/cf{seed}x{i}.json"
            json.dump(scn, open(path, "w"), indent=1)
            batch.append(path)
        try:
            r = subprocess.run(
                ["cargo", "run", "-q", "-p", "seine-harness", "--", "diff", *batch],
                capture_output=True, text=True, timeout=BATCH_TIMEOUT,
            )
        except subprocess.TimeoutExpired:
            # a scenario in this batch NON-TERMINATES — bisect (engine-only
            # `run`, fast) to record + keep it, drop the rest, and continue.
            hung = []
            for p in batch:
                try:
                    subprocess.run(["cargo", "run", "-q", "-p", "seine-harness",
                                    "--", "run", p], capture_output=True,
                                   text=True, timeout=SCN_TIMEOUT)
                except subprocess.TimeoutExpired:
                    hung.append(os.path.basename(p).split(".")[0])
            hangs += len(hung)
            print(f"  HANG batch@{done}: {hung}")
            done += len(batch)
            for p in batch:
                if os.path.basename(p).split(".")[0] not in hung:
                    os.remove(p)
            continue
        for line in r.stdout.splitlines():
            if line.startswith("FAIL"):
                fails += 1
                print(line)
        for line in r.stdout.splitlines()[-1:]:
            print(f"  batch@{done}: {line}")
        done += len(batch)
        for p in batch:
            base = os.path.basename(p).split(".")[0]
            keep = any(l.startswith("FAIL") and l.split()[1] == base
                       for l in r.stdout.splitlines())
            if not keep:
                os.remove(p)
    tail = f", {hangs} hangs" if hangs else ""
    print(f"--- cep-fuzz complete: {n} cases, seed {seed}, {fails} divergences{tail}")
    sys.exit(1 if (fails or hangs) else 0)


if __name__ == "__main__":
    main()
