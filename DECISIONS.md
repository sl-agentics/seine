# DECISIONS.md — running log

The body below is an **append-only** log of semantics probes, tie-break
discoveries, design decisions, and known limitations (each checkpoint ends
with a handoff note). The block just below — **CURRENT STATE** — is the ONE
mutable part: a living summary, **overwrite it each checkpoint** so a fresh
session orients here instead of reading 4900 lines. Keep it short; put the
detail in a D-entry below and the active-slab detail in the plan file.

---

## CURRENT STATE  (living summary — overwrite each checkpoint)

_Last updated: 2026-07-12, post-D-201 (TMS-envelope arc: P1-P5 +
P6-part-1 ALL LANDED — census 599→84 cumulative −86.0%; the residue
= 64 permanent runaway-mismatches + 20 fixable tails; SIXTEEN
graduates arc-total; pushed through 1f76175 [D-200], D-201
UNPUSHED). Earlier closed-arc records below kept verbatim. **THE D-177 FIX IS
LANDED (Bryan-gated): the HALT ARC IS CLOSED END-TO-END.** ⚖ THE
LANDING LAW (third standing law, docs/tjupd-ledger-mechanisms.md
top): delete teardowns land by MODE×CAUSE — stream ⇒ at the delete's
propagation (external: action / RHS: firing); cloud ⇒ victim's
item-reach (D-076); expiration ⇒ post-fire/quiescence; **the
executor NEVER reopens a committed pick** (cf5x17 confirmed; the
static return ~7234 untouched; agenda_open ×19 byte-identical across
3 measurements). ⚖ METHOD LAW (beside it): a pin is an
interpretation of a probe; an underdetermined output is not a
finding (the D-176 residual's "Drools reopens the pick pre-fire" was
this defect — corrected at D-177). THE FIX (D-177 §5, landed
verbatim at D-178): `tms_eager_break(f, from_delete)` — the k≥2
exclusion lifts for stream explicit deletes; one predicate, both
delete paths via on_delete_ex; the pick machinery untouched.
GRADUATED at D-178: tju_spin_deps_{extdel,delpartner} +
hm1/hm1b/hm2b → regressions/ (**corpus 11/1088/344**
byte-identical); probes_pending/halt/ EMPTY. Gates this slab: cargo
9, lint **1622/0/0**, fuzz_cep ×4 = 0, **TJUPD ×5×400 = 0**, tjt
25/25, tj_upd 64 + exactly tu81x60, mju 0/200, notpop 84 + expop 104
fresh ALL MATCH (8031/8037), population 2,199/2,200 (the 1 =
tu81x60, pre-existing, A/B'd, filed), bindings 72 (landed-tree
release .so). **THE FLAG-EAGER PORT IS LANDED (D-180,
Bryan-gated): the plain-arm insert walks now corpse-check the
WALKING fact too (two guards, phreak.rs do_join_node; memory push
stays — flag-eager, retraction-lazy); all seven fe cells == oracle
and GRADUATED (fe1/2/3/4/6 → regressions/, fe5/fe7 →
pr_fe_{below_boundary,temporal_arm}); corpus **11/1090/349**
byte-identical, lint **1628/0/0**, full battery green (fuzz ×9 = 0,
population 2,199/2,200, bindings 72, the ⚠ D-112 cf1x flip-flop zone
HELD).** **THE D-181 LINGERING-DEL FIX IS LANDED (D-182,
Bryan-gated) — ⚖ identity-model law instance #3 closed (one
predicate at the D-171 exemption site: temporal nodes scan ALL
pending s0-ins so a lingering exit-del stays visible with its
re-entry ins and processes del-then-ins in stage order). The tu81x60
family GRADUATED (5 → regressions/, 2 → pr_tux60_* control pins);
corpus **11/1092/354** byte-identical, lint **1634/0/0**, tj_upd
**64/64**, population **2,200/2,200 all 11 seeds**, fuzz ×9 = 0,
bindings 72 (landed-tree .so). ⇒ **THE BATTERY'S OPEN LEDGER IS
EMPTY** — every filed divergence fixed-and-graduated or
fenced-by-nature (D-134 §6 ties, fz_42_84). Known-open surfaces are
noted SUB-CELLS only (born-expired/D-133, update-refold ph=1,
stream-AB partner-only sites, the D-175 hypothetical all-alive
marked act — D-117 guard stays as its backstop).** window:length arc
CLOSED at D-185 (port landed; corpus **11/1124/354**, lint
**1662/0/0**; follow-ups not ledgered: CepWindowTest length
baselines, SEINE_WINLEN axis merge, standalone windows P3; body
D-183..D-185). **ACTIVE ARC: D-080 TMS ENVELOPE (opened D-186 2026-07-11; plan
`~/.claude/plans/tms-envelope-arc.md`, recon home
`probes_pending/tms_envelope/`).** Scoping D-186 (68 witnesses
re-baselined byte-stable; buckets L-SD ×13 / L-MB ×18 / I-RD ×12 /
I-ST ×1 / compound ×1; runaways+nondet fenced-by-nature; residue R1 =
cloud×belief-loss landing rows ⚠ D-106-adjacent, R2 = static
stated/justified model). D-187: xfail ENGINE-DRIFT GATE landed (make
diff 4th tier; movement needs re-triage + xfail-rebank + D-entry).
L-SD: rung 1+2 (D-187/188) = the three-clause (cloud × self-defeat)
row, all 13 accounted; CONSOLIDATION 0-DIV (D-189/190): ⚖ QUEUE-HEAD
DISCIPLINE + the member-order physics (add-at-head, fold⇒reversal,
sharer/ownership staging — graft-derived, zero toggles); model_sd.py
+ fuzz_tms_sd.py = the executable spec + population instrument,
750/750 v1. L-MB (D-191–D-193): v2 A-shape grammar; D-191 headline
retracted at D-192 (⚖ method law template); pinned physics: update
relocates join-rtm to TAIL / not-ltm in place (gt9/gt10), obs_join
mirror orders, fold batching, del = EAGER dep-cascade (gt12), the
INTERVENING-ACTION fold law (gt11: ilfirst churns / mutfirst folds).
D-194 (record = probe-dir files + commits, no D-entry): gt1-gt13 +
39-cell truths banked in-repo; HANDOFF-instrument.md = the TmsDump
recipe + 3 residue clusters + H1-H3. **D-195: THE TMSDUMP LENS IS
LIVE IN-TREE (SdDump, one dump both lenses, non-flushing verified)
AND THE LEAD-NL BELIEF-STAGING CLUSTER IS CLOSED — ⚖-candidate
EVAL-CONSUMPTION LANDING: the amut update-break's dep-teardown lands
at the justifier's next NETWORK EVAL (mid-run ⇒ between-firings eval,
LK nets out unseen; last-firing ⇒ the item's NEXT POP — gt14
splitter: strictly-higher observers fire on the zombie-justified LK,
sub-salience never; quiescence-landing dead); composite lane
(breaks+set_break) is MUTFIRST-gated — the RHS pair drains in RHS
order, ilfirst's not-break wins the race (D-076 eager, no window),
mutfirst's join-break wins (lazy, window) = the fold law's
belief-layer sibling (x51 vs x26/x58/x71/x95; round-1 overreach
falsified by fuzz, honest record in tmslens-results.md). H2
falsified (deps attach at insertLogical exec), H1-WM falsified (the
queue drains BETWEEN same-rule firings; non-preemption is
agenda-layer = the interposer arc's lane). v2 populations
**741/750** (7001/7002/6001 149; 6003 147; fresh 7004 147 — its 4
finds bisected: 3 pre-existing order-class + x51 fixed
out-of-sample); ZERO belief-staging divergences remain; survivors =
2 order clusters: mf-lazy-trail continuation (x68/x41/x88 + kin
x90/x67/x108/x131) + nb-trail tails (x103/x0). gt14+gt15 banked;
lint 1716/0/0; validator 39/39; corpus + engine untouched, walls
up.** **D-196 (Bryan-sequenced: clusters → 0-div → interposer):
BOTH ORDER CLUSTERS CLOSED (gt16/gt17 dumps: ⚖ two-phase unbreak /
stale-rtm starvation with the ALPHA-GATE touch + private-phys
continuation; the nb deleter's t0-FIFO — the churn proxy removed;
⚖ del-lane eval-consumption — gt12's "eager cascade" was
salience-confounded; the eager-composite matrix via gt18/gt19 dumps
+ the gt20 2×2: mutfirst never propagates mid-run, ilfirst-lead
nets, ilfirst-trail folds iff del_not decl-first). **v2 = TRUE
0-DIV: 1800/1800, twelve seeds incl. never-used 7011 clean on first
contact**; 26-witness registry (check_witnesses.py); engine census =
the port A/B baseline. **THE R1 INTERPOSER LADDER IS OPEN AND ITS
FIRST RUNG IS 6/6 GREEN 3×-stable** (mechanical model predictions
exact on both spines: pop window = salience threshold; eager k0 no
glimpse; clause-B fan-out starvation; the D-195 between-row) ⇒ the
(cloud × belief-loss) landing rows are a CONFIRMED TABLE. lint
1722/0/0.** **D-197 (THE PORT SLAB, round 1 — Bryan: "the port is
translation"): the deferral CAUSE MODEL landed (three lanes + the
late-dep race flag on tms.deferred; engine.rs TMS machinery only,
the executor pick/halt untouched) — the interposer ladder is
ENGINE-GREEN 6/6, census 599→505 (−15.7%, all 12 seeds, model
0-div held), fz_123_941 GRADUATED out of xfail (10/10; regressions
355), fz_123_9175 rebanked toward-oracle (drift bank 74),
agenda_open ×19 byte-identical ×2, corpus byte-identical, lint
1723/0/0. Round 2 = P4 clause-B, P5 clause-C, the lazy L-MB fine
structure, THEN P3 (the equal-salience drain split, ⚠
D-106-adjacent, deliberately last) — port-target-list.md.**
**D-198 (round 2): P5 clause-C + P4-lead LANDED — sd_c1/sd_b2
exact; THREE GRADUATES (fz_42_5213 the 20-firing alternation,
fz_123_3060 the clause-B over-cell, fz_7_9550 the L-SD×L-MB
COMPOUND); census 505→483 net, HONEST MIX (5 seeds improved / 5
regressed slightly on non-pinned shapes = round-3 targets);
corpus 11/1124/358, drift 71, agenda_open ×19 identical, ladder
6/6, model 0-div held; ⚠ a population-only PANIC caught by the
12-seed net (prefix-park revive, fixed) — populations are the
panic net.** **D-199 (round 3): P4 CLOSED + the t15 lanes
completed — the or-twin was NODE SHARING, not PnShadow (the
leak's env-lookup missed the shared node's first-owner env; the
DEPTH-match fixes it); TWO GRADUATES (fz_7_9375, fz_123_9175 —
corpus 11/1124/360, drift 69); the round-2 census regressions
CURED (65 broken slots → order-only residue; per-case
attribution: all 65 were D-198 machinery, the depth-match broke
zero) via three more model-translated lanes: the ⚖ land_eager
lead-k1 unpark (self-killed premises ONLY — ⚠ the ungated cut
made the engine follow Drools into the d3/d5 no-amut RUNAWAY,
sdp7002x40 spun; the census net caught it, second panic-class
catch in two rounds; census loops now timeout 900/seed), the ⚖
revive ACTOR EXCLUSION (self-inflicted left-death never revives
the actor's own), and the lead park-RECORD + ⚖ t15 foreign-death
SWEEP (WM-level trigger in on_delete_ex, stale-value alpha admit
= the starvation law; lead revives insertion-order, trail keeps
sd_c1's reversed chain). CENSUS **483 → 242 (−49.9%; cumulative
599 → 242, −59.6%)**, model 0-div 12×150/150, agenda_open ×19
identical ×4, ladder 6/6 ×4, receipts green after EVERY change.
The classified residue: ~64 = the d3/d5 no-amut runaway family
(oracle runs away / engine terminates — PERMANENTLY OPEN by the
terminates-invariant, the census floor); ~63 ORDER-ONLY (P6 —
the model's order layer is the spec); the SET rest = P3's
equal-salience window (sdp7002x3: the decl-preceding
same-salience observer's glimpse) + pick-order physics (P6) +
the undiagnosed x73 class.** **D-201 (P6 part 1, four
model-translated lanes): ⚖ the k0 FOLD/CHURN law — del-group rules
force-evaluate before the drain's retract (the cross-batch
staging annihilation had blinded their nots to the break; the
un-break now re-adds REVERSED per the blocked-list prepend order;
lazy = all del-group rules churn, eager = sink order rj < l) —
THE ARC'S SINGLE BIGGEST MOVER (197→101 alone: the whole k0 ORDER
family + the lazy pick-order/set_break SET classes were one
mechanism); the composite last-key RIDE (bit16 at defer-push via
the survivors read; flush + post-fire exclude it, drain[pop] only
= drops[]/land_lazy; ⚠ a run_live-gated first cut was the wrong
direction — mid-run keys land at selections even queue-empty);
the del-lane race widening (a DELETED premise at insertLogical is
the D-195 race too ⇒ bit2; the last generation's zombie window,
x98); the trail sweep (the annihilation starves trail parked-del
too; re-add order notpos-split: lead=insertion, trail=reversed
sd_c1 chain). CENSUS **197→84 (cumulative 599→84, −86.0%)**,
model 0-div 12×150/150, corpus 11/1124/370 byte-identical ×4,
drift 59 untouched ×4, agenda_open ×19 identical, lint 1738/0/0.
COMPOSITION: 64 permanent (d3/d5 no-amut runaways) + 20 FIXABLE
TAILS (12 ORD + 8 SET, no cluster > 4 — fine-structure corners
of landed mechanisms).** **D-200: P3 LANDED — ROUND 3
COMPLETE, P1-P5 ALL LANDED. The pop-precedence drain split at
post-fire-continue: the halt keeps certified strictly-higher
(D-091 untouched, the pick untouched); the DRAIN defers under
equal-salience decl-preceding preemption — LANE-SCOPED to bit1
(NOT-side self-defeat) entries ONLY: ⚠ the whole-drain first cut
broke 14 certified t20-lane cells (bit0 justifiers, no not —
their continue-drain is pr_tms_t20*-certified); the D-197 cause
flags made the scoping expressible; all 14 recovered. TEN
GRADUATES in one change — the ENTIRE P3 witness list: min812
(the anchor glimpse), fz_123_{2135,3370,4318,7637,9133},
fz_777_9637, fz_7_812, fz_7_9864 (= the oracle's 19), and
fz_7_1353 THE FINALS-DIFF COMPOUND (4→12, its 8 lost firings
returned exactly as the target list predicted). Corpus
11/1124/370, drift 59, agenda_open ×19 byte-identical ×6 this
sitting, ladder 6/6, cells 39/39, witnesses 26/26, lint
1738/0/0. CENSUS 242→197; ROUND-3 CUMULATIVE 483→197 (−59.2%);
FROM THE PRE-PORT BASELINE **599→197 (−67.1%)**. Composition:
64 RUNAWAY-MISMATCH (the d3/d5 no-amut family — permanent, the
census floor), 63 ORDER-ONLY (P6), 70 SET (pick-order physics +
the x73 class + tails).** NEXT: the 20 tails (each a
1-4-case fine-structure corner) or call the population slab
COMPLETE — Bryan's call; I-RD after the slab (Bryan's order). Any engine change stays
gated (⚠ D-106 tripwire + D-177 landing-not-pick). Other
candidates: class-3 re-entrant churn, Allen-beyond-Drools.
Prior: D-168→D-185 all landed. Fenced-by-nature: D-134 §6 ties,
fz_42_84, the d3/d5 no-amut runaways. `git log --oneline -20`
for HEAD._

**Repo:** Seine — differential-tested Rust port of a bounded Drools 9.44.0.Final
subset. **Prime directive: PROBE-FIRST** — the oracle settles every semantic;
NEVER hand-derive PHREAK/temporal staging (it flip-flops — re-proven twice).
Workflow / env quirks / doctrine: memory `seine-workflow.md`.

**Git:** on `main`, fully PUSHED through the D-185 stack
(Bryan-directed 2026-07-11, branch-only, NO tag: D-181 `9cd64dd` →
D-182 `bafbabc` → D-183 `b8dc624` → D-184 `cf41076` → D-185
`98e8d87` + pointer/state commits). Resume point: NEXT is Bryan's
call (the D-185 §5 follow-ups; the battery's open ledger is EMPTY).
**RELEASED: v0.4.2 → PyPI, 2026-07-11** (Bryan-directed; tag
`v0.4.2` on `cf14cb5`, pipeline landed green — Bryan-confirmed;
follows v0.4.1 on `2a482e8` earlier the same day). The
v*-tag⇒publish hazard note stands for FUTURE tags. ⚠ **NO `v*` TAGS
until a PyPI release is intended** — `ci.yml`'s
`release`/`publish-pypi` fire on tag push and the `pypi` environment
has NO protection rules (gh-verified): a new tag publishes
`seine-rs` with no manual gate. Arc history: the D-entries
below (D-136 shared-tjo → D-137/138/139 item-C → D-140 not-order →
D-141 tj-ts → D-143..D-153 the mechanical shadows → D-154/155 A2 winacc
→ D-156 tj pair-order → D-158/159 plain-not → D-160 acc drain → D-161
plain-exists wedge).

**Gates (green @ the D-161 commit):** baseline 11 / probes **1070** /
regressions **299** byte-identical / lint **1468 live·0 ghost·0 inert** /
9 Rust suites / bindings pytest 72. Verify: `make diff` ·
`make lint-probes` · `cargo test`. Specs stay executable: plain-not
`SEINE_NOTPOP_PLAIN=1 tools/fuzz_notorder_b.py <n> <seed>` +
`tools/model_check_notorder_b.py <pop> pflush`; event-not MODEL=flush;
winacc/acc-drain `tools/model_check_winacc.py <popdir>`; plain-exists
populations `SEINE_EXPOP_PLAIN=1 tools/fuzz_existsorder.py <n> <seed>`
+ `EMODEL=pexists tools/model_check_exists.py <pop>` (seeds 5001-5007
banked in job 577ad61a tmp; the D-162 spec, PORTED).
**Red on resume ⇒ drift — investigate before building.**

**Landed (background — log has detail):** v0.4.0 CEP E1 + reset + agenda groups
+ queries×mutation + aggregation; data-types (D-096–098); TMS; P1c group CEs;
CEP E2 A–E (D-109..D-120); **temporal-join-order fix (D-121→D-125)**:
per-arrival temporal flush — eager partner with a present anchor joins
INDIVIDUALLY at its flush, a held batch reverses ONCE via the staged
`addInsert`-prepend, unshared temporal fills stamp lseq in staged order. Sites:
`engine.rs stream_flush_ex` cascade dispatch + `phreak.rs Node::flush_ins_delta`
+ the `do_join_node` fill stamp; bails (shared/AB/upd-del/ph=1/RIA) keep legacy.
Spec stays executable: `tools/model_join_flush.py`. **exists×temporal
(D-127):** the existential analog — a temporal EXISTS node is admitted
per-arrival by `phreak.rs exists_flush_admit` (a pure-insert batch replayed in
arrival order — max-FactId groups one upstream join emission, admitted/blocked
REVERSED once), and `engine.rs stream_flush_ex` no longer `self_drain_delta`s
an exists node (its full staging flows to the eval). Gated to temporal
`Kind::Exists`; `not`/non-temporal exists byte-identical. Exists deletes are
unobservable (retractions never fire) so the port is insert-only. Spec:
`tools/model_exists_flush.py`.

**✅ not×temporal ENGINE PORT — DONE (D-132/133 §3A reaping, D-134 §3B firing
deferral).** The LAST CEP-E2 fence is down; joins (D-125), exists (D-127), and
`not` (D-134) all unwalled. Design (D-134 authoritative; report
`docs/not-temporal-port-mechanism.md` is the pre-port plan):
- **§3A REAPING** — a temporal `not` gets a PHANTOM `temporal_pos` recording its
  after/before @expires-inference edges (`engine.rs` ~2320; bare-NEVER check
  uses `temporal_pos`), + the D-133 `schedule_expiration` boundary
  (deadline<clock ⇒ leak, ==clock ⇒ lazy delete, >clock ⇒ schedule).
- **§3B FIRING DEFERRAL** — `not_fire_time` (after ⇒ anchor+hi; before[0,hi] ⇒
  anchor) → `not_emit_or_defer` holds the left in `node.lefts` and pushes
  `node.new_deferrals` (no child) → engine → `fire_deadlines` (mirror
  `deadlines`, +creation seq) → `drain_pending_fires` at fire-quiescence
  (BEFORE the reap) releases due lefts to `node.pending_release`, ordered
  creation@clock0 / (−ft,creation)@advance, ONE prepend reversal → agenda. Fires
  only still-UNBLOCKED lefts (blocked⇒silent forever ⇒ un-block re-fire SUPPRESSED
  for temporal nots). `not_releasing` flag makes the not_mid released-left's
  downstream E2 join ignore `is_expired` (partners alive at fire_time). Specs:
  `tools/model_not_defer.py` (arc A), `tools/model_not_infer.py` (arc B/chains).
- **Verified:** `fuzz_not_temporal` 0 firing-SET div / ~4600 cases,
  engine==validated-model; diff 11/956/284; the ~0.6% within-close-time ORDER
  residual (chain_not/not_mid) fenced to
  `scenarios/xfail/xf_cep_not_{chain_heaptie,mid_release_join_order}`; 5 recon
  witnesses graduated to `scenarios/probes/pr_cep_not_*`.

**✅ @expires INFERENCE through an exists — DONE (D-135, one line).** A temporal
exists now gets a phantom `temporal_pos` (`engine.rs` ~2281, the not §3A device
extended to `CeKind::Exists`) so its after/before edges record and the bare-NEVER
override stops forcing NEVER. FACTS-only (reaping) gap (~25% → 0-div); FIRING
0-div throughout (invisible to exists firings, like the not); NO §3B. Gates:
`fuzz_exists_infer.py` 0 firing+facts, `make diff` 11/958/284 (+witnesses
`pr_cep_e_exists_infer_{reap,before}`), `fuzz_exists_temporal` (D-127) 0-div,
cargo test / lint 1336·0·0 / bindings 72. The whole CEP-E2 inference arc (joins,
exists, not; explicit + inferred @expires) is now unwalled.

**✅ SHARED temporal-join node ORDER — DONE (D-136).** The compose = D-125 base +
WHOLE-epoch peer reversal (the engine did neither at pop-time = 14% nor at flush =
61%). Landed in 3 scoped pieces: DIVERT clean-shape shared temporal joins (insert-
only delta, all-Term sinks) out of the eval walk on EVERY arrival (linked or not —
an unlinked left in staging self-drains REVERSED, flipping the base) →
ACCUMULATE the per-arrival `flush_ins_delta` emission into `phreak.rs Node.tj_epoch`
FORWARD → DRAIN once at `fire_all` (first sink `append_into_pending`, peers
`peer_merge_term`). `fuzz_shared_tjo.py` 0-div (5 seeds/700); diff 11/958/288;
`xf_cep_tjorder_dual_tms` GRADUATED + 3 `fz_tjo_shared*` witnesses. See D-136.

**➡ CURRENT ISSUES — remaining CEP backlog (readiness-ordered; recon/specs/battery
ALREADY BUILT — start from here, don't re-derive).** The whole temporal-join arc
is landed (joins D-125, exists D-127, not D-134, @expires-inference D-135, shared
order D-136); the CEP surface is faithful except:

1. **E2 item-C classes 1/2/3 — event UPDATE/DELETE re-propagation (D-115 fenced →
   D-137 PARTIAL PORT, committed locally, NOT pushed).** Classes 1 & 2 FIXED (corpus
   byte-identical); class 3 CHARACTERIZED + DEFERRED (**ACTIVE next slab**); the two
   update fuzz fences STAY (each guards a SEPARATE out-of-C gap — 1a/1b).
   - **class 1 — PORTED** (`pr_cep_c_upd_temporal` graduated): a POSITIVE temporal
     (after/before/Allen) join node is NOT property-reactive; updating the event on
     the TEMPORAL side re-fires (the ANCHOR/left input does NOT —
     `pr_cep_c_upd_anchor`). Fix = `on_update` `(true,true)` forces `add_upd` when
     `pat.ce==Positive && node.temporal`. Validated after/shared(D-136)/chain.
   - **class 2 — PORTED** (`pr_cep_c_upd_{evict_revive,after_exp}` graduated): a
     per-node `TrieNode.clock_removed` set (filled in `stage_acc_removal`) makes the
     `on_update` `(false,true)` re-entry NOT revive a clock-removed (evicted/
     expired, still `is_alive`) event into an accumulate.
   - **class 3 — EXTERNAL churn PORTED (D-138); rule-RHS re-entrant variant fenced.**
     The exists delete+reinsert churn is EVENT-SPECIFIC (a PLAIN-fact churn
     coalesces on both engines, matching Drools' `PhreakExistsNode`). ROOT: STREAM
     defers external deletes to `fire_all` while inserts stream-flush per-arrival ⇒
     a del-first event-exists churn evaluates ins-before-del and coalesces. FIX
     (`engine.rs delete_fact`): an EXPLICIT (`!in_expiration_drain`) delete of an
     EVENT witness at an exists/not node FORCE-EVALUATES the affected rules at
     DELETE-TIME (arrival order) so the same-epoch reinsert's stream-flush re-blocks
     + re-fires; scoped to exists/not-over-event-type rules ⇒ corpus byte-identical.
     `xf_cep_c_del_churn_exists` GRADUATED → `pr_cep_c_del_churn_exists` (+3 pins
     `pr_cep_c_exists_{churn_bare,churn_plain,delonly}`). Gates: diff 11/974/288,
     lint 1361, class-3 fuzz 0-div 3×800 (fence `del_ok` lifted), blast-radius
     42/123/7 == HEAD. FENCED (`xf_cep_c_del_churn_exists_rule`): the RULE-RHS
     variant (delete during its own fire) — the delete-time eval doesn't fire
     re-entrantly; NOT fuzz-reachable. Recon:
     `~/.claude/plans/cep-e2-item-c-class3-findings.md`,
     `tools/model_check_exists_churn.py`, `oracle/.../ExistsDump.java`. See D-138.
   Gate MET for 1&2: `make diff` 11/**970**/288, lint 1352, cargo test, bindings 72,
   blast-radius seeds 42/123/7 == pristine HEAD (CEP-gated). D-115's "lift fences ⇒
   0-div" premise was OPTIMISTIC (fences do double-duty — 1a/1b). See D-137.

1a. **windowed-accumulate LIVE-modify property-reactivity — PORTED (D-139).** The
    rule was NOT "windowed is reactive on FUNCTION fields, plain re-folds on any
    modify" (the D-137 sketch — WRONG): probing showed watch(windowed) = source
    BINDINGS only, watch(plain) = source CONSTRAINTS∪BINDINGS (plain IS
    property-reactive too). One `on_update` block gates a windowed accumulate's
    source re-fold on `bind_fields` (= listen_mask minus constraints); plain/every
    other node keeps `listen_mask` (byte-identical). `xf_cep_c_upd_win_{live,noop}`
    GRADUATED + 4 discriminators; `windowed_acc_types` UPDATE fence LIFTED; spec
    `tools/model_check_react.py`. See D-139.

1b. **pre-existing temporal-join-ORDER latents (E1-hardening)** — surfaced by
    lifting `temporal_types` (8/800 fuzz div, ALL bisect-to-HEAD byte-identical:
    cf313 not-order, @duration interval join-order). NOT class 1, NOT caused by the
    `add_upd` port. Same family as item #2 below. Fence KEPT.

2. **non-temporal `not X() P()` firing ORDER — ✅ DONE (D-140).** The banked model
   (`9c6735c`) is now ENFORCED by a post-hoc AGENDA reorder (the per-fact
   last-touch/insert-epoch/update-seq stamp reorder the findings doc sketched — far
   cheaper than a flush rework), event-gated + corpus byte-identical. On unblock the
   blocked P's fire by BATCH = last-touch epoch (initial=0 LAST; epoch batches
   REVERSE for EVENT / already-correct FORWARD for PLAIN, left untouched), within-
   batch inserts-then-updates(reverse-apply). THE CRUX that made it corpus-safe: the
   reorder is GATED to the CLEAN regime (no fired P inserted in the current cycle) —
   an in-cycle insert (immediate delete-unblock / fresh post-unblock insert) keeps
   the engine's already-correct FIFO (HEAD-identical), which is why `pr_cep_c_del_not`
   / `_u3` / `_v3` / `_v5` stayed byte-identical while the reversal only fires in the
   `fuzz_notorder`-validated regime. Witnesses `cf313x13`/`cf401x344` A/B-proven
   fixed; 3 pins `pr_cep_not_order_ev_{expiry,delete,upd}`. The early
   "PriorityQueue-tie" mischaracterization is fully retired (it is DEFINED by code).
   See D-140; recon/model `~/.claude/plans/cep-not-order-findings.md`.

3. **window × interval count-during-window — PARSER wall, not semantics.**
   `accumulate($e:E() ...; count($e))` (bound accumulate SOURCE) doesn't parse (2
   `engine_fenced` probes, `cp_win_int_*`). Backlog: (a) DRL parser extension for a
   bound accumulate source, THEN (b) the B×E membership-during-window compose. The
   parser piece is the gate; not started.

4. **temporal+delete+TMS NON-TERMINATION (D-117; deep, backstopped — SAFE now).**
   `spin_guard` ERRORS (~18s) instead of hanging. Root = a TMS
   `exp_deferred`/`deferred` RE-ADD cycle inside the D-080/D-106 halt-model envelope
   (memory BARS local halt-semantics patches ⇒ needs a halt-model rework, the
   biggest/riskiest). Single repro
   `scenarios/hang-backlog/pre_existing_temporal_delete_hang`. E1-hardening.

5. **Full-Allen inference — ROADMAP, off-oracle (do NOT start before faithful-
   first).** `xf_cep_e_{ic,inf}_*` = Drools-incoherence (keepA/keepB) + the
   documented @expires-inference LEAK. The "fix" is the beyond-Drools SUPERSET
   (spec-driven vs Allen 1983, opt-in/quarantined) — `docs/allen-beyond-drools.md`,
   D-118/119. Bryan-ruled parked until Seine is certifiably faithful on the whole
   CEP surface.

**Fenced by nature — NO backlog (would need JVM emulation; document, don't chase):**
within-close-time not ORDER (`xf_cep_not_*` = equal-fire-time
`java.util.PriorityQueue` tie, D-134 §6 / D-131) · reset×WindowNode incoherence
(`xf_win_reset_incoherence`, D-114) · identity-hash order (`fz_42_84` + the
fz_42/fz_123 latent family).

**Also walled (E1-hardening / scope):** windowed-accumulate removal TIMING (~0.5%,
D-112/114 — needs a `model_check_stream`+WindowNode sub-recon, do NOT hand-tune) ·
D-080 TMS envelope · window×TMS · `window:length` + standalone window · Upstream
#2366 (`docs/drools-inferred-expiry-never.md`).

---

## 2026-07-03 — Session start, Phase 0

### D-001: Environment
- Java: OpenJDK 21.0.11 (Ubuntu). Maven 3.8.7.
- Rust: 1.96.1 stable, installed via rustup (minimal profile) this session —
  `~/.cargo/bin` must be on PATH.
- **Oracle pinned: Drools `9.44.0.Final`** (pre-seeded in local `~/.m2`; Maven
  Central reachable for missing transitives). Pinned in `oracle/pom.xml` via
  `<drools.version>` property. Locale forced to `en_US` in the runner.

### D-002: Project name
"**Seine**" — a seine is a fishing net, echoing Rete ("net") without using the
Drools trademark (brief §8). Crates: `seine-engine`, `seine-harness`. Name is
local-only for now; re-check crates.io availability before any publish.

### D-003: Canonical result JSON schema (locked — both runners target this forever)
```json
{
  "facts":   [ {"type": "T", "fields": {"a": 1, "b": "x"}}, ... ],
  "firings": [ {"rule": "R", "matches": [ <fact rendering>, ... ]}, ... ]
}
```
- `facts` = final working-memory contents as a **multiset**, canonically sorted
  by rendering (type, then field values); fields serialized with sorted keys.
- `firings` = **ordered** log of rule firings (afterMatchFired). Each entry
  carries the matched facts' renderings, sorted lexicographically *within* the
  entry (so we don't depend on either engine's internal tuple ordering), while
  the firing sequence itself is order-significant.
- Comparison is **semantic, not textual**: comparator parses both JSONs;
  f64 equality is IEEE-754 bit equality; i64 is exact. Java must emit doubles
  with a decimal point (Jackson default) so JSON number types round-trip.

### D-004: Scenario format
```json
{
  "name": "...",
  "types": [ {"name": "Person", "fields": [{"name":"age","type":"i64"}, ...]}, ... ],
  "facts": [ {"type": "Person", "fields": {"age": 30, ...}}, ... ],
  "drl":   "rule ... end"
}
```
- Field lists are **ordered** (arrays, not maps): declared-type constructor
  argument order in generated DRL `declare` blocks follows this order.
- Java runner does NOT codegen Java classes; it prepends generated `declare`
  blocks to the scenario DRL and instantiates via the `FactType` API. This
  keeps the oracle fully data-driven.
- Types: `i64` → long, `f64` → double, `String` → String, `bool` → boolean.

### D-005: Oracle runner design
- Batch-capable from day one (fuzz phases need thousands of cases without JVM
  restart): accepts N scenario paths, emits NDJSON `{"scenario": ..., "result": ...}`
  per line to stdout. Errors surface as `{"scenario":..., "error":...}`.
- Deps: drools-compiler/-core/-kiesession/-mvel/-xml-support + kie-api/-internal,
  Jackson for JSON, slf4j-nop to silence logging. All Drools deps pinned 9.44.0.Final.

### D-006: Oracle verified end-to-end (Phase 0 gate 1) ✅
`p0_trivial_adult` runs through real Drools 9.44.0.Final: DRL compiles (declare
blocks + rule), 2 firings captured, canonical JSON emitted, stderr clean.
- **Observed:** `KieSession.getObjects()` order is nondeterministic across runs
  (bob/alice/carol vs bob/carol/alice) → comparator MUST treat `facts` as a
  multiset. It does; canonicalization lives in the comparator, not the runners.
- **Preliminary observation (NOT yet pinned):** same-rule activations for facts
  all inserted before `fireAllRules()` fired in fact *insertion order*
  (alice→carol). Must be pinned with dedicated Phase 1 probes (multi-rule,
  salience, interleaved insert) before relying on it.
- Environment gotcha: the box had JRE-only Java 21; installed
  `openjdk-21-jdk-headless` via sudo apt for javac.

---

### D-007: Phase 0 walking skeleton GREEN ✅ (done-bar met)
- Rust workspace: `engine/` (seine-engine) + `harness/` (seine-harness).
- Store layout per brief: per-type per-field columnar arenas, global
  insertion-ordered `FactId(u32)` handles (never reused, so handle order ==
  Drools fact-handle recency), alive flags for Phase-2 retraction.
- DRL parser covers the Phase 0–1 grammar (single pattern, `==/!=/</<=/>/>=`,
  bindings, `salience`, `no-loop`, RHS `insert(new T(...))` with literals /
  `$fieldBind` / `$factBind.getField()`). Everything else is a parse error —
  the scope wall is mechanical.
- **PROVISIONAL conflict resolution** (must probe in Phase 1): highest
  salience, then lowest fact handle, then rule declaration order. Only the
  single-rule insertion-order part is oracle-backed (D-006).
- `make diff` = single command differential run (builds oracle if stale):
  PASS p0_trivial_adult, 1/1. `make test` = pure-Rust tests (6 passing).

## Phase 1

### D-008: Conflict resolution PINNED via probes pr01–pr08 (oracle-verified)
Drools 9.44.0.Final, all facts via insert, fireAllRules(), java dialect:
- **Order key = (salience DESC, rule declaration index ASC, fact insertion
  order ASC), re-evaluated globally after every firing.**
- pr01: equal salience → rule declaration order (A before B).
- pr02: salience descending across B(20) > A(10) > D(0,default) > C(-5).
- pr03/pr05: equal salience is rule-major: ALL of an earlier rule's
  activations (facts in insertion order) fire before the next rule's.
- pr06: **preemption**: if a firing inserts a fact that activates an
  earlier-declared rule, that rule fires NEXT, before the current rule's
  remaining activations (B(1),A(1),B(2),A(2)).
- pr07: declaration order, NOT rule-name order (Zeta fired before Alpha).
- pr08: fact insertion order, NOT field-value order (9,1,5).
- Engine `next_activation` implements exactly this key. All probes are
  permanent regression scenarios under scenarios/probes/.
- NOT yet pinned (Phase 2): tuple ordering for multi-pattern activations,
  behavior under update/delete, timestamp/recency tie-breaks after mutation.

### D-009: Declared-type boolean getters are isX() ONLY (oracle-pinned)
Probe: `$s.getOk()` on a declared type with `ok : boolean` is a Drools
**compile error** ("The method getOk() is undefined"); `$s.isOk()` works.
- Parser accepts both `getX`/`isX` and resolves to field `x`; the engine is
  therefore *more lenient* than Drools (`getOk` on bool would compile here but
  not in Drools). The generator only emits the Drools-legal form, so the
  differential surface stays in-subset. Known, documented leniency — not a
  divergence risk (divergence requires oracle-legal input).
- Regression: scenarios/probes/pr11_bool_is_getter.json.

### D-010: Phase 1 curated corpus + property generator
- Curated: p1_ops_{i64,f64,str_bool}, p1_multi_constraint, p1_empty_pattern_
  no_match, p1_bindings_rhs, p1_duplicate_facts, p1_salience_preempt,
  p1_chain, plus probes pr09 (string relational ops DO work in DRL and match
  Rust byte-order comparison for ASCII — corpus strings stay ASCII-only) and
  pr10 (numeric cross-type: i64 field vs f64 literal and vice versa promote
  like Java). All green.
- Generator (`seine-harness fuzz <count> [seed]`, default seed 42,
  SplitMix64): 2–4 types × 1–3 typed fields; 1–6 rules; 0–3 constraints +
  0–2 field bindings per pattern; salience −10..10 (35% of rules); no-loop
  (10%); RHS 0–2 inserts with literal/binding/getter args (type-correct,
  i64→f64 widening allowed). **Termination by construction:** a rule matching
  Ti only inserts Tj with j>i (type-index DAG), so chains strictly climb.
  Divergent cases are auto-saved to scenarios/failures/.

### D-012: Phase 1 COMPLETE ✅ (done-bar met)
- Curated corpus: 21/21 PASS (`make diff`).
- Property fuzz: **10,000 cases, seed 42, 0 divergences**, 237s wall
  (`cargo run -q -p seine-harness -- fuzz 10000 42`). Reproducible: case k of
  seed s is deterministic.
- Trial-run stats (first 100 cases): 72% of scenarios produce ≥1 firing,
  414 firings total, max 42 in one scenario — the corpus is not trivially
  empty.

---

**HANDOFF @ FINAL checkpoint (Phases 0–2 COMPLETE)** — Definition of Done
per brief §6, against the D-017 subset:
- Curated corpus: **102/102 PASS** (`make diff`): phase-0/1 seed suites,
  probes pr01–pr11 + u01–u16 + j01–j22, 47 named fuzz regressions. Every
  scenario asserts final-fact-set AND ordered-firing-log equivalence
  against real Drools 9.44.0.Final.
- Fuzz: **30,000 full cases (seeds 42, 7, 123) + 8k spot cases, all at
  zero divergences** over the Phase-1+2 grammar (`make fuzz SEED=n`).
  Runs are deterministic (SplitMix64; case k of seed s always identical).
- ONE out-of-subset xfail (xfail/fz_42_4373, D-016/D-022) with an
  automated delta-minimizer (xfail/minimize.py) and analysis notes; the
  subset wall (D-017: mutation programs ≤2-pattern rules) is enforced by
  the generator and documented in the README.
- `make test` = 6 pure-Rust tests, no JVM needed.
- Environment for a fresh session: PATH needs `~/.cargo/bin`; JVM 21 +
  Maven resolve Drools from `~/.m2` (pinned 9.44.0.Final in oracle/pom.xml).
- If resuming: (1) the open xfail — extend minimize.py to drop constraints
  and RHS actions, shrink values, then hand-trace the ~15-update swap;
  (2) Phase 3 stretch items (not/exists, accumulate, matches/contains/in)
  were NOT started — Phases 1–2 solidity was prioritized per the brief.

**HANDOFF @ checkpoint 3** — Phase 1 COMPLETE (single-pattern rules: all six
operators × 4 field types, bindings, salience, preemption, chains, no-loop
(inert for inserts), 10k fuzz cases zero divergences). Phase 2 goldens
already captured in D-011 (probes_pending/j01–j05, oracle-only). Next:
extend engine to multi-pattern joins (left-major nested-loop activation
order per j01), cross-pattern var constraints (`Expr::Var` rhs), then
update/modify/delete RHS with render-after-RHS switch (j03), no-loop
(j04), activation cancellation on delete (j05); then move j-probes into
scenarios/, add curated Phase 2 corpus, extend fuzzer grammar (joins +
mutation with termination discipline), 10k fuzz. Open divergences: none.

### D-018: Agenda evaluation = outrank model (fz_42_2906 corrected D-015's peek)
An executing rule is interrupted only by rules that OUTRANK it (salience
desc, then decl order). Implementation: eager (no-loop) rules merge staged
batches at every flush; then walk priority order merging each network and
fire the first unfired match — rules below the firing rule accumulate.
This replaced the "peek at first non-executing rule" model (which over-
evaluated: fz_42_2906's single-batch left-major order proved rules below
the executor are NOT evaluated mid-execution). pr06 preemption follows
from outranking; fz_42_4138 per-firing batches follow from no-loop
eagerness; fz_42_4141's one-batch follows from lazy descent.

### D-019: Phase 2 COMPLETE ✅ (done-bar met, subset per D-017)
- Curated corpus: **95/95 PASS** (`make diff`) — probes pr01–pr11,
  u01–u13, j01–j22, p0/p1 suites, 41 named fuzz regressions.
- Property fuzz over the full Phase-2 grammar (joins ≤3 patterns in
  insert/delete programs, ≤2 patterns with update/modify, self-joins,
  guard-monotone mutation): **10,000 cases seed 42 AND 10,000 cases seed 7,
  both 0 divergences** (~255s each; final run after D-020 fixes, corpus at
  100/100).
- Open xfails (xfail/): fz_42_3408, fz_42_4373 — 3-pattern rules × long
  multi-update histories, outside the D-017 subset, kept with analysis
  notes in D-016 for a future session.

### D-020: RHS binding snapshots + indexed-equality coercion (seed-7 wave)
Second-seed fuzz found 3 value-level (not ordering) divergences:
- **LHS bindings used on the RHS are snapshots taken when the consequence
  starts** (Drools extracts declarations once): setters earlier in the same
  RHS must not affect later `$b` references (fz_7_2525: `setF1(-2);
  setF1($b)` restores the match-time value). Getter calls (`$p.getX()`)
  remain live reads. Engine: `Src::SnapField` + per-firing snapshot.
- **Join `==` coerces the bound value to the LEFT field's type** (Java cast:
  double→long truncates toward zero) — `I(n == $x)` with n=0, $x=-0.5
  MATCHES (u14, fz_7_4974). Join `!=` and relationals promote to double
  (u15: `n != $x` with $x=1.5 matches ALL ints), and literal comparisons
  always promote (`I(n == 1.5)` never matches). Engine: `eval_cmp_join`.
- Probes u14/u15 + 3 regressions added; corpus 100/100.

### D-021: Hot-prefix move-to-front (u16) — fz_42_3408 resolved
Post-final-checkpoint: probe u16 (u13's shape + a SECOND update event)
reproduced the xfail class minimally and pinned the missing rule: prefixes
holding a fact that is HOT at one of their positions move to the front of
their level's prefix memory (relative order kept) — gated by hot positions,
unlike the right-memory move which is ungated (D-018/fz_42_3433 vs 4359).
fz_42_3408 now passes and is a regression; corpus 102/102.

### D-022: Cascade-based refire requeue (fz_42_4373 minimization, round 1)
A delta-minimizer (xfail/minimize round) shrank fz_42_4373 to 3 rules /
2 facts and pinned the requeue mechanism exactly: refires propagate like
inserts — the left-update stream walks a hot tuple's existing extensions
in RIGHT-MEMORY order, emissions REVERSE between joins, then the
right-update stream walks the left memory; the terminal requeues in
arrival order with dedup. This replaced the position-ascending
approximation (which coincidentally matched all shallower pins — the new
cascade reproduces every one of them; corpus 102/102, spot fuzz 8k more
cases clean). The minimized round-1 case passes.
**fz_42_4373 (full) remains the single open xfail**: divergence moved from
firing 391 to 665; a second minimization round leaves a 4-fact/3-rule case
diverging at firing 109 of 172 — a positional swap between a requeued
refire and a pending entry after ~15 update events. Next session: extend
the minimizer to also drop individual constraints/actions and shrink fact
fields, then hand-trace. The D-017 generator wall stays until resolved.

### D-023: LAST XFAIL RESOLVED — unified update cascade; D-017 wall LIFTED
Session continuation. Tooling first: `SEINE_HANDLES=1` makes both runners
emit fact-handle tags (`__h`) for unambiguous log comparison (oracle handle
ids are 1-based, engine 0-based — offset by one); tools/minimize.py is a
delta-debugger that shrinks a scenario while the divergence persists
(rules, facts, constraints, setters, statements).
Three minimization rounds against fz_42_4373 pinned, in order:
1. Refires propagate through the join chain exactly like inserts (D-022's
   cascade — round-1 case).
2. A hot-moved prefix block is NOT in prior memory order (round 3).
3. **The unifying rule (round 4): a property-hot update re-enters the
   staged flow as a re-insert.** Its U-chain (left-stream over the right
   memory, reversal between joins, right-stream over the left memory)
   determines at every level: the re-prepended block order of the prefix
   memory (with fresh creation seqs, so subsequent hot-first iterations see
   U order), and the requeue order of previously-fired activations at the
   terminal. Pending activations still keep their positions (u01–u04).
   This subsumes D-021's move-to-front and D-022's requeue ordering — both
   were special cases of the same mechanism.
fz_42_4373 passes. (The wall-lift attempted here was later re-imposed —
see D-025.)

### D-024: Widened-grammar wave (seeds 42/777) — three more pins
Lifting D-017 and fuzzing the full grammar found 3 divergences; each
minimized to ≤3 rules / ≤3 facts with tools/minimize.py + SEINE_HANDLES:
- **fz_42_5243 (2 rules, 2 facts):** the rule that just fired re-evaluates
  its own network even if its own RHS UNLINKED it (the executor is still
  active) — engine: force-merge of the last-fired rule bypassing the
  linking gate. Virgin/bystander unlinked rules still accumulate (fz_7_145
  unchanged).
- **fz_42_9462 (2 rules, 2 facts):** PENDING join activations whose tuple
  is hot at a RIGHT position also requeue (retract+reassert of the join
  child), and the requeue block is PREPENDED ahead of kept entries — every
  earlier requeue case had an empty kept list, masking the placement.
- **fz_777_1853 (1 rule, 2-3 facts, two rounds):**
  (a) HOT-position memory moves happen BEFORE the update cascade
  (fz_42_1057 sees moved order) but UNGATED moves of non-listening right
  memories happen AFTER it (the same-batch requeue sees pre-move order;
  fz_42_3433 only observed the move from a later batch);
  (b) the final requeue matrix: **requeue iff FIRED or RIGHT-hot; a
  PENDING activation hot only at pos0 (pure left-update, or k==1) is
  updated in place** — reconciling u01–u04, fz_42_2804/9462 and both
  rounds of fz_777_1853.
Corpus 106/106 after promoting all three.

### D-025: Widened-grammar campaign paused — wall re-imposed; open class
### = requeue PLACEMENT among pending join activations
After D-024's fixes, a 4-seed × 10k campaign on the unrestricted grammar
still produced ~2 divergences per 10k. Two minimized counterexamples now
DIRECTLY contradict each other under every simple placement rule tried:
- fz_42_9462 wants a requeued pending activation AHEAD of a pending cold
  one; the fz_42_3554 min-case wants requeued pending activations to stay
  IN PLACE (its firing-1 batch), while its firing-0 batch is ambiguous.
- Hand-derivation of PHREAK's agenda (in-place child updates vs
  retract/reassert, activation numbering, queue discipline) no longer
  converges from black-box order observations alone at this depth; the
  next step is modelling the true per-rule activation QUEUE (activation
  numbers, possibly LIFO segments) rather than a list with placement
  heuristics.
- State: engine keeps ALL D-023/D-024 fixes (each independently validated;
  corpus 106/106 includes fz_42_5243/9462, fz_777_1853); the D-017 wall is
  RE-IMPOSED in the generator; ~22 unminimized widened-grammar failures
  are parked in xfail/ as the work queue for the next campaign
  (tools/minimize.py + SEINE_HANDLES=1 are the workflow).
- IMPORTANT correction: the wall does NOT fully exclude the open class —
  a post-fix walled fuzz found ~2/10k divergences (fz_42_3311-class: the
  class reaches 2-pattern mutation programs too; earlier 30k-clean runs
  simply never drew these shapes). The proven-subset claim is therefore
  weakened until the class is closed; all failure cases are parked in
  xfail/ (26 files).
- One of them (fz_42_3311 round 1) pinned cleanly along the way: a BARE
  update() carries Drools' ALL-SET mask, which is CLASS-reactive — it
  refires even empty-listen patterns (unlike property masks, j13); engine
  treats the u64::MAX sentinel mask as intersecting everything.
- DIRECTION DECIDED for the next round: stop black-box order-fitting. The
  drools-core 9.44.0.Final -sources jar (fetched into ~/.m2, extracted for
  READING ONLY under the scratchpad) shows the real structures:
  PhreakJoinNode.doNode phase order (rightDel, leftDel,
  reorderRightMemory(removeAdd→moves tuple to END), reorderLeftMemory
  (remove-all→re-append), rightUpdates, leftUpdates, rightInserts,
  leftInserts), TupleList memories APPEND at tail, TupleSets staged lists
  PREPEND (LIFO), and child-tuple lists per parent. The next engine
  iteration should be a faithful behavioral port of this node algorithm
  (still validated only through oracle probes; no code copied), replacing
  the fitted emission heuristics in merge_staged.

**HANDOFF @ phreak-port MERGE (Session 3 close)** — The behavioral port of
the PHREAK node algorithm is the engine (engine/src/phreak.rs + engine.rs
integration). Proven state at merge:
- Corpus: 156/156 (was 106; +26 graduated ex-xfails, +24 new probes and
  regressions from this session's discriminator ladders).
- Fuzz, UNWALLED grammar (mutation + 3-pattern rules mix freely): seeds
  42, 7, 123, 777, 999 x 10,000 cases = 50k cases, ZERO divergences.
- All 26 parked xfail cases from D-025 resolved and graduated; xfail/ is
  gone; D-016/D-017/D-025 walls retired.
- `make test` green; tree clean at every commit on the branch.
New pinned mechanism classes this session: eager/lazy evaluation windows
(the j01-vs-9462 discriminator), bucket-change vs same-bucket child
sync-walks with cursor threading, object-identity staging folds,
downstream-pending clash-moves, property-miss right-tuple reAdd with
child realignment, side-aware index-key coercion, agenda-item lifecycle,
and build-time alpha literal sharing/hashing (D-027..D-029).
Next session candidates: Phase 3 stretch (not/exists, accumulate,
matches/contains/in) — restrict the generator first, probe before
implementing; or scale campaigns (more seeds, larger CASES) for the
current subset.

### D-029: Alpha-node literal sharing + hash-threshold coercion (seed 777)
fz_777_4504 (first unwalled multi-seed campaign find) exposed BUILD-TIME
alpha-network semantics for `field == literal`, pinned by probe series
w1-w18 + the pr_lit matrix:
- Node identity: (type, preceding-literal-constraint chain, field,
  literal COERCED to the field's type). A later rule whose coerced
  literal collides SHARES the first-built node and inherits its ORIGINAL
  literal: after `n == 1`, `n == 1.5` matches n=1 (w10); built the other
  way around, `n == 1` matches NOTHING (w16 — first-built 1.5 wins).
- Hashed sinks: >= 3 sibling eq-nodes (post-sharing) on one field switch
  membership to the COERCED key — a double literal on a long field
  truncates: `f0 == 2.5` matches f0 == 2 (w5/w8/w12, fz_777_4504's
  {1, -2, 2.5} group). Three IDENTICAL literals share one node and stay
  below the threshold (w7).
- Below the threshold each node evaluates its first-built literal with
  double promotion: standalone `n == 2.5` matches nothing (w4/u15).
`!=` and relationals always promote to double (pr_lit: `f0 != 2.5` and
`f0 == 2.5` BOTH match f0 == 2 when the eq-group is hashed).
Implemented as a compile-time literal rewrite (share_and_hash_alphas).
Multi-seed unwalled campaign: seeds 42/7/123/999 clean at 10k; seed 777
clean after this fix.

## Query CEs in rules — Phase Q2 (2026-07-05)

**HANDOFF @ Phase Q2 close (2026-07-05)** — `?query` pull CEs in rules
are CERTIFIED (D-056..D-058): corpus 533/533 (+50 Q2 probes, +21
graduated fuzz regressions incl. minimized cases), witnessed fuzz 6
seeds x 10k = 60k cases zero divergences with ?query CEs in ~10% of
draws. The 8-puzzle demo (demo/eight_puzzle.py) validates the
Prolog-grade claim end to end: recursion + unification + backtracking
goal-search with in-engine path extraction through the Q2 bridge; its
frozen instance is a corpus scenario. Walls unchanged: push CEs,
query+mutation (D-051/D-057), not/exists/accumulate beside CEs,
salience over CE vars, D-055 recursion fences, >96-key resize.
If resuming: (1) the push/reactive CE form is the natural next phase
(qx2_late_push pinned the basic refire; open-query row lifecycle
unprobed); (2) negation-as-failure inside queries; (3) the D-058
arming/linking model is pinned black-box — a MemDump of
PathMemory.linkedSegmentMask on query paths would confirm the
mechanism if edge cases surface; (4) scale campaigns remain cheap
insurance (D-058's classes needed ~1.5/10k draw rates).

### D-056: `?query` pull CEs in rules PINNED — the rule-site bridge into
### the Q1 stack machine (probes qx0..qx7, 36 scenarios; sources:
### PhreakQueryNode, QueryElementNode/QueryTupleSets, RuleNetworkEvaluator
### .evalQueryNode/evalStackEntry; python replica q2_check replays 36/36)
New DRL surface: `?Name(a1, ..., ak;)` as a rule CE at any LHS position.
Args are positional over the query's params: a literal (exact param
type), a var bound by an earlier pattern/CE (filters inside the callee),
or a FRESH var (binds per result row; usable in later patterns, CE args
and the RHS). The rule fires once per result row.

**Evaluation window** (qx2 series): the pull happens LAZILY at the
rule's agenda evaluation window, against the WM as of that moment
(qx2_lazy_window; rule-derived facts included, qx2_derived_chain,
qx6_rec_derived). `?` CEs are NOT reactive: WM changes to queried types
never refire already-evaluated lefts (qx2_late_pull); each NEW left
pulls at ITS OWN window (qx2_new_left). The push form (no `?`) IS
reactive (qx2_late_push) — walled, D-057.

**Match rendering:** the CE contributes the call's args array to the
match objects — null at BOUND positions, the row's value at UNBOUND
positions (qx0_bound/lit, qx1_params2; internal callee declarations
never appear). Both runners canonicalize it as
`{"type":"QueryArgs","fields":{"value":[...]}}`, ORDER-significant
(raw Object[].toString carries an identity hash). A leading CE matches
on InitialFact (qx0_first). A repeated unbound var gets per-position
row values in the array, and the DOWNSTREAM binding takes the LAST
position's value (qx4_dupvar_out: row (2,3) via ?CPair($v,$v;) fires
QA[2,3] and inserts Out(3)) — unlike nested-call threading, which
stays first-wins (D-054).

**Ordering — the machine** (all replica-verified): the CE is a Q1
nested-call site embedded in the rule path.
1. Rule-side lefts reach the CE in RAW staged order — full LIFO across
   evaluation windows (newest window first, LIFO within: qx6_windows
   fired A1,A2,A3,A4 = reverse of staging [A4,A3,A2,A1]); a preceding
   join's output batch is consumed in its staged order (qx6_join_before).
2. doLeftInserts consumes src head→tail, PREPENDING one dquery env per
   left into every callee-branch pool (pool = reverse of src); branch
   frames push in declaration order onto the LIFO stack (last branch
   evaluates first — same as D-054 nested calls). All Q0/Q1 internals
   (D-050/D-052/D-053 fact levels, D-054 call frames/sweeps) unchanged.
3. Each result row PREPENDS a child tuple [left + args array] into the
   CE's result staging AT ARRIVAL (rowAdded → addInsert). All rows
   arrive while the site's resume frame is still pending, so the D-055
   late-result re-push stays UNREACHABLE through the rule bridge
   (replica asserts staging empty at every site entry, 36/36).
4. At the site's resume pop the staging drains to the next rule level:
   ORDER-PRESERVED for single-sink CEs (TupleSetsImpl.addTo = addAll).
   Net observable: one left's rows fire in REVERSE of the standalone
   getQueryResults order (qx1_order_std); left blocks fire in reverse
   of the staged-left order; downstream joins/CEs consume the drained
   list with standard staging semantics (qx1_next_level/thread/two_ce/
   same_twice; fact-level parity qx5_batch2/batch3; call-level
   qx5_batch_call; recursive interleave qx5_rec_multi).
5. SHARED CEs (multi-sink) stage into a QueryTupleSets whose drain
   RE-REVERSES (addTo re-addInserts head→tail), then D-037 propagation:
   first-BUILT sink gets the drained list as-is, later sinks get
   flipped copies — so the first sink fires rows in standalone/arrival
   order while later sinks and unshared CEs fire reverse-arrival
   (qx3_two_rules, qx5_three_rules; evaluation window owned by whoever
   reaches it first, polarity fixed by build order: qx3_salience;
   leading-CE variant qx6_share_first).

**Sharing identity** (for the trie): two rules share a `?query` CE node
iff the query name AND the args template match — literals by value
(qx5_share_lit: lit vs var ⇒ no share), bound-var args BY NAME
(qx7_share_bound2: $aid vs $bid ⇒ no share, ne_t13-style), unbound
positions as placeholders (var NAMES irrelevant: qx5_share_name shares
$x vs $y). Preceding-prefix identity per D-036/D-037 as usual.

### D-057: Phase Q2 wall
IN: `?`-prefixed pull CEs in rules over D-055-shape queries (recursive
and not); args = literals / bound vars / fresh vars; multiple CEs per
rule incl. the same query twice (qx5_same_twice); CEs at any position
incl. leading (InitialFact) and after joins; CE-bound vars flowing into
later patterns, later CE args, and RHS insert args; shared CE prefixes
across rules; INSERT-ONLY programs — rules may insert queried types
(no reactivity, termination unaffected; qx2_derived_chain) and even
recursive-query source types when the DATA stays acyclic by
construction (qx6_rec_derived; generators never do this).
OUT (compile-rejected in the engine and/or excluded from generators):
- PUSH query CEs (no `?`): reactive (qx2_late_push pinned the basic
  refire) but the open-query row-update/remove lifecycle is unprobed.
- Query+mutation stays walled (D-051): no update/delete epochs in
  query scenarios; generated Q2 programs keep insert-only RHS — the
  PhreakQueryNode doLeftUpdates/doLeftDeletes paths (left churn at CE
  nodes, dquery re-parameterization) are unprobed.
- not/exists/accumulate/collect CEs in the SAME rule as a `?query` CE
  (linking/staging interplay unprobed); `?query` inside not/exists.
- CE-bound vars in salience expressions (typing unprobed, D-043 scope).
- Expression args (`$b.getF()`, arithmetic) and fact-binding args.
- Repeated unbound vars in one CE call: engine implements the last-wins
  pin (qx4_dupvar regression) but generators never emit them.
- Arg/param type mismatches: exact-type match required (engine
  compile error; Drools would coerce per Java assignability, unprobed).

### D-058: Q2 fuzz wave 1 — three pins the hand probes missed
### (23 divergences over seeds 42/7, all minimized/bisected to ≤2-rule
### cases; corpus 533/533 after; supersedes D-056's sharing identity)
1. **Query-network pattern memories are STATEFUL** (fz_42_1016 →
   probes qx8_statemem/qx8_statemem3): staged alpha-passing facts drain
   into a pattern's memory AT EACH EVALUATION of its query network —
   newest-first within the drain batch, batches APPENDED; deletes leave
   at the next drain. A ?query CE evaluating mid-firing therefore
   splits memories into drain windows; a fresh reverse-insertion
   rebuild coincides only when every evaluation happens post-quiescence
   (exactly the pre-Q2 envelope, which is why Q0/Q1 never saw it).
   Engine: persistent QueryMem keyed by (query, branch, node), one
   shared drain in the evaluator.
2. **Queries are agenda items** (min_6527 bisect; sources:
   PathMemory.queueRuleAgendaItem → addQueryAgendaItem,
   ActivationsManagerImpl.evaluateQueriesForRule,
   AbstractGroupEvaluator): once a ?query CE has pulled a query, the
   resident dqueries keep its network paths linked — ARMED — and every
   subsequent WM event queues the query's agenda item at (salience 0,
   its declaration position in the unit's interleaved rule+query
   sequence). The item's evaluation is a DRAIN WINDOW (nothing fires);
   it runs when the agenda walk reaches it, and a CE-bearing rule's
   evaluation first drains its depending queries (transitive call
   closure — evaluateQueriesForRule). Standalone getQueryResults
   retracts its dquery and never arms, so query-only scenarios keep
   their single post-quiescence batch (fz_7_546/fz_777_145 pinned the
   distinction). Also from this wave: an EMPTY-src call level pushes no
   frames and evaluation CONTINUES at the next node (evalQueryNode's
   return-false path) — post-call patterns still drain their windows.
3. **CE node sharing is ALL-UNBOUND-args only** (min_6795 →
   pr_qx9_min_neither/pr_qx9_n_noQ1; pr_qx9_share_bound_late):
   QueryElement.equals compares args templates whose UNBOUND positions
   hold the Variable.v singleton while literal and declaration args are
   per-rule objects — so identical literal args or same-named bound
   args do NOT share; each rule's CE pulls fresh at its own agenda
   window (min_6795's low-salience twin fired on facts inserted after
   the first rule's empty window). All-unbound templates DO share, with
   consume-once semantics: a late sink is STARVED of rows already
   consumed at an earlier sharer's window (pr_qx9_share_late/late2/
   late3). D-056's "bound vars BY NAME" sharing component is RETRACTED.
Generator gates from the same wave: QR rules attach only to fully
insert-only programs (rule DELETES draw independently of
allow_mutation; the engine walls ?query CEs beside any mutation
action); a fresh var minted by the SAME call is repeated-unbound, not
bound (fz_42_4330-class: Drools NPEs or returns null-position rows —
the engine walls repeated-fresh-var positions like any unbound arg).

## Recursive queries — Phase Q1 (2026-07-04)

### D-054: recursive-query semantics PINNED — the stack-machine model
### (probes qa1..qa7, qb1..qb6, qc1..qc7 + sources + MemDump5; the
### python replica machine_q1.py replays 75/75 fenced-subset calls)
New DRL surface: `or` CEs in query bodies (top-level, branches optionally
parenthesized with `and`-joined patterns), POSITIONAL patterns
(`Location($x, $y;)` — args map to declared field order; a bound
var/param = unification, a FRESH var = field binding, a literal =
same-type alpha), and QUERY CALLS as patterns (`contained($x, $z;)` —
positional args only; literals allowed). The doc transitive closure runs
verbatim and returns exact closures.

**Basics** (qa1-qa3, qb3-qb6, qc3-qc5, qc7): positional ≡ named
constraint form, row-for-row. A call's candidates multiply per callee
row (duplicates preserved); callee-internal bindings never leak;
`getIdentifiers` = params + FIRST branch's declarations (later-branch
locals are absent; row.get on them THROWS — oracle runner now encodes
those as JSON null, and rows from a branch render other-branch locals
as null). Call args thread D-052-style: bound positions filter inside
the callee; unbound positions bind FIRST-WINS per returned row
(`SelfPair($x): contained($x,$x;)` = full closure, $x from position 1).
Params may go unused in a branch (qc7) — that branch simply doesn't
filter on them. Non-recursive call DAGs (chains, diamonds, two calls
per branch, or-of-calls, 3-branch non-recursive or) all pin exactly.

**Evaluation machine** (RuleNetworkEvaluator/PhreakQueryNode/
PhreakQueryTerminalNode sources + MemDump5 path-order dump): queries
evaluate as a LIFO stack machine over per-branch node lists —
1. getQueryResults stages the root tuple into EVERY branch's SHARED
   staging pool (peers), then evaluates paths in DECLARATION order
   (pathMemories order == subrule order, MemDump5); rows APPEND to the
   collector. A pool may be swept EARLY: any nested takeAll of that
   branch's pool (see 3) carries pending tuples with it — their rows
   still route correctly by tuple parentage (this produced qb2's
   b1,b3,b2 block order — one mechanism, no nondeterminism).
2. Fact levels batch exactly like Q0: consume src head→tail, children
   PREPEND into the next stage (all D-050/D-052/D-053 rules apply
   inside query branches).
3. A call level pushes a RESUME frame (site, accumulated-results
   splice), stages one nested dquery env per src tuple by PREPENDING
   into every callee-branch pool, then pushes one BranchEval per callee
   branch (declaration order) each taking `takeAll(pool)` — LIFO pop
   means the LAST callee branch evaluates first; result blocks come out
   base-branch-first because terminal routing PREPENDS each nested row
   (child tuple = caller env + threaded bindings) into the call-site's
   shared result staging, double-reversing.
4. A RESUME pop splices the site's pending results after its captured
   trg and continues at the node after the call.
Determinism confirmed (row orders reproduce across JVM runs); the
python machine replays every in-subset probe call byte-exactly,
including 123-row full closures, 12-deep chains, trees, DAGs,
duplicate-edge multiplicity and post-call constraint filtering.

### D-055: Phase Q1 wall — the certified recursion shape
IN: self-recursive queries of EXACTLY 2 or-branches with the BASE
branch first and the recursive branch second; exactly one self-call,
not the first CE of its branch (a fact pattern must precede it);
non-recursive queries: 1..3 or-branches, arbitrary non-recursive call
DAGs (incl. shared callees and repeated calls); positional syntax in
query bodies; acyclic call-reachable DATA only.
OUT (probed, documented, engine compile-rejects or generator avoids):
- CYCLIC data under recursion: Drools HANGS (no tabling — qa8 timeout).
  Engine backstop: evaluation step limit -> clean error. Generators
  build acyclic relations by construction.
- LEFT recursion (self-call first in its branch): Drools silently
  returns 0 rows for derivable facts (qb7 — wrong, terminating);
  compile-rejected.
- 3+ or-branches on RECURSIVE queries and recursive-branch-FIRST
  ordering: real Drools delivers late self-recursive results through a
  resume RE-PUSH (PhreakQueryTerminalNode.checkAndTriggerQuery-
  Reevaluation) whose scheduling we did not fully pin (qb2 [None,None]
  and qc1 diverge only there; that mechanism is UNREACHABLE in the
  fenced shape — verified 0 re-push firings across all 75 in-subset
  calls). Fence, don't hack.
- Mutual recursion (call-graph cycles of length >= 2): compile-rejected
  (untested interleaving).
- `?query(...)` pull CEs in RULES: next phase (query-as-condition
  bridge).
- Query+mutation interplay: still walled at D-051.

## Queries — Phase Q0 (2026-07-04)

### D-049: Query differential harness — scenario/result schema extension
Scenario gains an optional top-level `"queries"` array: ordered calls run
AFTER the initial fire and all epochs, against the final WM.
```json
"queries": [ {"call": "ByAge", "args": [30, null]} ]
```
- `args` are JSON scalars typed like fact fields (integer→long,
  decimal→double, string, bool). JSON `null` = UNBOUND (Java
  `Variable.v`) — safe encoding because the subset has no null field
  values.
- Oracle runs `session.getQueryResults(name, args...)` per call and emits
  a result section:
```json
"queries": [ {"call":..., "args": <echo>, "identifiers": ["$p","$a"],
              "rows": [ {"$p": <fact rendering>, "$a": <scalar rendering>} ]} ]
```
  Scalar bindings render like accumulate results: `{"type":
  "Long|Double|String|Boolean", "fields": {"value": ...}}` (String branch
  added to the oracle renderer; unreachable for pre-query scenarios).
- **Canonical comparison**: `queries` arrays are positional; `call`/`args`
  compared semantically; `identifiers` compared as a SET; `rows` are
  ORDER-SIGNIFICANT, each row a map identifier→rendering. Missing
  section == empty section (back-compat with pre-query scenarios).
- Drools' `getIdentifiers()` ORDER is a `HashMap` iteration artifact
  (verified: bucket order of `String.hashCode & 15` explains q1/q2/q5/q6
  orders) — deliberately NOT modeled; hence set comparison.
- Oracle query output is deterministic: 3 independent JVM runs over the
  full 21-probe set produced byte-identical query sections (facts order
  still varies per D-006 — queries and facts differ here).

### D-050: Query semantics PINNED — probes q1–q9, qr_*, qc_order, qo_*,
### qm_mixed, qn_join, qd_depth + live-memory ground truth (MemDump 1–3)
Everything below is oracle-verified; the full model replays all 50
probe query calls exactly (scratch model_check.py, 50/50).

**Basics** (q1–q9): queries see the final WM including forward-chained
facts; duplicate facts yield duplicate rows (multiset); a query whose
type has no facts yields 0 rows (no error); defining queries perturbs
NOTHING about rule firings or final facts (q8); repeated calls in one
session are stable; unbound args unify (each row carries the matched
value); bound args filter. Row values include ALL identifiers: params
(bound or unified), pattern bindings, field bindings.

**Row ordering — the full evaluation model.** getQueryResults evaluates
the query's join chain PULL-style with PHREAK staged sets; everything
observable reduces to:
1. Each pattern owns a "right memory" holding the type's alpha-passing
   facts in REVERSE WM-insertion order ("arrival order") — inserts stage
   LIFO (`TupleSetsImpl.addInsert` prepends) and drain into the memory at
   the query's first evaluation. Derived facts sit in the same global
   insertion sequence at their actual insertion point.
2. Memory structure per pattern:
   - ≥1 beta equality constraint → hash table (`TupleIndexHashTable`,
     128 slots): index fields = FIRST equality (textual order) plus
     subsequent equalities that are NOT param-unifications, capped at 3
     (`compositeKeyDepth` default 3; a 2nd unification NEVER indexes —
     IndexSpec skips it: qc_order QA/QB group by first key only).
   - no beta equality → plain list (arrival order).
3. Hashing (verified bit-exact against live tables, startResult=993 for
   Person.age etc.):
   - `slot = rehash(h) & 127` where `h` folds `h = 31*h + javaHash(v)`
     over indexed values, seeded by `seed = 31; seed += 31*seed + extIdx`
     per index field; `rehash` = JDK6 supplemental
     (`h ^= h>>>20 ^ h>>>12; h ^= h>>>7 ^ h>>>4`, u32).
   - javaHash: Long `(v ^ v>>>32) as i32`; Double over
     `doubleToLongBits`; Boolean 1231/1237; String = UTF-16
     `31*h + c`.
   - **extractor index `extIdx` = 1 + rank of the field's accessor
     method name** among the generated bean's no-arg public methods
     sorted by name: `getX`/`isX` (bool) per field + `getClass`,
     `hashCode`, `toString` (slot 0 = `this`). Pinned across 18 type
     shapes (MemDump3; the boolean `isMarried`→6 case is what broke
     every simpler rule).
   - Key-lists: new key PREPENDS into its slot's chain; tuples APPEND
     within a list (so within-key order = arrival order).
4. Join iteration per consumed left tuple:
   - `indexedUnificationJoin` (any indexed param-unification, textual
     position irrelevant — qo_first/qo_beta U4==U5): ALWAYS full-table
     iteration, slots ascending → chain order → within-list order,
     filtering ALL beta constraints (bound params filter, unbound bind).
   - indexed without unification (qn_join, qo_beta U6): bucket lookup by
     the left-bound key (hash + value equality), iterate that key-list
     in arrival order, filter remaining constraints.
   - plain: whole list in arrival order, filter.
5. Staging: S1 = [query tuple]; join i consumes S_i head→tail and
   PREPENDS each emitted child into S_{i+1}; the terminal consumes the
   last stage head→tail APPENDING rows. (Net effect: single-pattern
   queries emit rows in slot-DESCENDING/reverse-arrival order; q7's
   3-pattern parity a1-fwd/b-rev/c-fwd falls out of the same mechanics.)

### D-051: Query subset wall (Phase Q0)
IN: non-recursive queries of 1–3 positive patterns over declared types;
typed params; unification `==` on params (any count, any textual
position); regular join equalities/inequalities to prior bindings or
`$b.field`; field bindings; literal alpha constraints; bound/unbound/
mixed invocation from the API; queries coexisting with rules; derived
facts; multiple calls per scenario; empty results; duplicate rows.
OUT (documented, excluded from generator + probes reject):
- query-calling-query, `?query(...)` pull patterns in rules, `query`
  CEs inside rule LHS;
- not/exists/accumulate/collect INSIDE query bodies;
- update/delete epoch actions in scenarios that also declare queries
  (staged-insert cancellation + removeAdd reordering unprobed; D-016's
  alpha move-to-front interplay unknown) — insert-only epochs are fine;
- ≥96 distinct values per indexed key (table resize re-buckets with
  chain reversal — unmodeled);
- f64 query args that are NaN/±0.0 (Double.equals vs numeric == at the
  index boundary unprobed);
- field names that don't fit the lowercase `getX`/`isX` accessor-sort
  rule or collide with getClass/hashCode/toString.

### D-052: multi-site unification is PER-SITE against the pattern-entry
### value; first site binds at pattern EXIT (fz_4242_621/1945, q11_multisite)
First query-fuzz wave (seed 4242, 2000 cases) caught what the hand
probes missed: `P(a == $x, b == $x)` with $x UNBOUND matches EVERY P —
there is NO cross-site consistency inside one pattern. Drools evaluates
each unification site against the tuple state ON ENTRY to the pattern
(unbound arg ⇒ every site passes; bound ⇒ every site filters), and the
FIRST textual site's field value becomes the param's binding when the
pattern exits — `P(a == $x, b == $x)` rows report $x = a, the swapped
form reports $x = b, and a FOLLOWING pattern's `c == $x` filters against
that exit binding (q11 ABC). Bound calls conjoin all sites as expected
(AB[2] = 0 rows). Engine fix: constraint evaluation reads the entry env;
unification writes are collected per candidate (first site wins) and
applied at emission. Index composition is unaffected (2nd site is a
unification ⇒ never indexed, D-050).

### D-053: beta constraints are SORTED regular-equalities-first; the
### index NEVER mixes unifications with regular keys (fz_4242_8775,
### fz_777_145, q12_mixed_index — corrects part of D-050)
10k-per-seed waves caught two more order divergences, both explained by
one build-time fact the hand probes could not see (live createMemory
dumps, MemDump4): the pattern's beta-constraint array is SORTED before
IndexSpec/setUnificationJoin run — regular (non-unification) equalities
first, then unifications, then non-indexables. Consequences:
- If a pattern has ANY regular equality, the index = the regular
  equalities ONLY (textual order among themselves, duplicates included —
  `f0 == $b, f0 == $b` builds DoubleCompositeIndex[f0,f0], seed 31810 —
  cap 3) and `indexedUnificationJoin` is FALSE: bucket lookup on the
  bound key; unification constraints just filter (bound) or bind at
  pattern exit (unbound, D-052).
- Only a pattern with NO regular equality full-iterates, and its index
  is the FIRST unification alone — so hash-slot order is only ever
  observable through SINGLE-FIELD seeds. (This is why qc_order/qm_mixed
  passed under the D-050 formulation: their shapes made both models
  coincide.)
- D-050's "first equality + subsequent non-unification equalities"
  composite is superseded by the above; everything else in D-050 stands.
Wall addition: operands bound in the SAME pattern (`$b : f1, f0 == $b`)
compile to alpha predicates in Drools — rejected by the engine, excluded
from the generator (D-051 extension).

### D-048: row-object ingestion sugar + seine-rs packaging/CI
- Lists of row objects — @seine.fact instances, plain dicts, or any
  attribute-bearing objects (dataclasses, Pydantic models) — are
  accepted anywhere tables are. The sugar reshapes rows into the
  certified dict-of-columns path in schema order (@fact class keys
  win, then the rows' own @fact class, then first-dict key order) and
  adds ZERO semantics: None and type errors still reject at the
  certified boundary. `seine.Session` is now a thin Python wrapper so
  insert()/insert_row() take row objects too.
- EXPLICIT schemas: the native session accepts
  schemas={type: {field: subset-type}}; @fact class keys contribute
  theirs automatically, so `{Flagged: []}` declares an empty type
  (previously required a typed Arrow table).
- Packaging: the PyPI distribution is **seine-rs** (the `seine` name
  is taken); the import remains `import seine`.
- CI (.github/workflows/ci.yml): the FULL differential gate (oracle
  build + cargo test + make diff) on every push/PR, the bindings
  pytest suite, abi3 wheels (linux + macos arm) as artifacts, and
  wheels attached to GitHub releases on v* tags. Unverified until the
  first remote run.

### D-047: EXTERNAL update/delete by handle CERTIFIED
Engine surface: `update_fact(id, fields)` (sets values, propagates with
the CHANGED-FIELDS property mask — oracle mirror is the 3-arg
session.update(fh, obj, modifiedProperties)) and `delete_fact(id)`;
external events carry NO rule origin (no-loop never suppresses them).
Scenario epochs gain ordered `actions` (insert/update/delete) targeting
the Nth VISIBLE inserted fact (synthetics excluded) — the oracle tracks
handles via an objectInserted listener, so rule-derived facts are
targetable (xu6). Pins, all differential:
- Probes xu1..xu6 passed on first contact (queued activations keep
  position and salience, alpha enter/leave on not-blockers across
  epochs, mask-miss no-ops, accumulate reverse on stored contributions,
  delete cancels + unblocks).
- **External actions compose ACTION-ORDERED at k=1 terminals**
  (xv2/xv3: reversing the actions reverses the firings) **but
  PHASE-GROUPED through beta paths** (xv4/xv5: order-insensitive).
  k=1 pattern-0 staging is now a WINDOW QUEUE: one window per external
  action; the initial batch and each RHS flush stay single windows;
  TupleSets folds span windows.
- **Slot memory on LIA-level pattern-0 staging** (fz_7_5801/xa/xb): a
  cancelled staged INSERT re-added later — an external exit + re-enter
  while the rule is unlinked — takes its ORIGINAL arrival slot, not the
  head. Scoped to trie s0_in only (k=1 is action-ordered; trg-level
  recreated children stay prepend, c13).
- **Rights-phase temp staging at accumulate nodes gates on the left
  not being staged** (getStagedType()==NONE in doRight*): a left
  touched on both sides enters the temp set in the LEFT phase, i.e.
  LAST (fz_7_5893; ALSO the real mechanism behind fz_123_449 — a
  newest-first chain reversal fixed 449's symptom, broke 25 round-2
  cases across all seeds, and was reverted; the 25 are graduated as
  arrival-order pins).
- CERTIFIED: zero divergences over 5 seeds (42/7/123/777/999 x 10,000,
  round 3, external actions in ~30% of scenarios' epochs; zero xfail
  draws).
- Bindings: session.update(handle, **fields) / session.delete(handle);
  insert/insert_row return handles (provenance for targeting);
  boundary tests cover semantics, dead-handle errors, certified action
  ordering, and epochs-with-actions parity replayed through Python.

### D-046: multi-fire CERTIFIED — the incremental envelope
Scenario schema gains optional `epochs: [{facts: [...]}]`: each epoch
inserts a batch into the SAME session and calls fireAllRules again;
the firing log continues across epochs (per-call fire limit, both
runners).
- The engine needed exactly ONE change: post-build `Engine::insert`
  now propagates immediately (session.insert semantics — staging and
  link/queue effects at insert time, agenda evaluation at the next
  fire). Everything else — staging accumulation, linking, accumulate
  float state, sticky dynamic item salience, eager re-entry — was
  already incremental-correct: probes mf1..mf6 passed on first
  differential contact after the fix.
- Pins: old tuples do NOT refire on a new fire call; CE flips across
  quiescence behave as live staging; accumulate reverse/add sequences
  CONTINUE across fires (float state carries bit-exactly); update-guard
  rules re-trigger for fresh facts only; the stale-item-salience
  machinery (D-043) spans quiescence.
- Generator emits epochs in ~30% of scenarios (external inserts are
  exempt from the insert-above DAG discipline — per-fact guard work
  stays bounded); the minimizer drops whole epochs and epoch facts.
- CERTIFIED: zero divergences over 5 seeds (42/7/123/777/999 x 10,000
  with epochs in the grammar; zero xfail draws).
- Bindings: the one-shot restriction is LIFTED — fire() is repeatable,
  each call returns ITS OWN delta (derived = live-after minus
  live-before, deleted likewise; Python inserts between fires belong
  to the before-set). Boundary tests cover quiescent refires,
  per-fire deltas, and epochs-scenario parity driven insert/fire/insert
  through the Python API against the native runner.

### D-045: Layer-2 Pythonic authoring — compiles to DRL TEXT
`@seine.fact` annotated classes (int/float/str/bool -> the subset
types; annotation order = constructor order) whose class attributes are
operator-overloaded FieldRefs, and a `Rule` builder (`when` /
`when_not` / `when_exists` / `accumulate` / `collect`,
`then_insert` / `then_modify` / `then_delete`, salience as int, bound
field, or single `term op term` expression). Everything builds a
declarative AST at definition time and renders into the frozen DRL
grammar — `rule.to_drl()` shows exactly what the engine runs, and the
differential guarantees cover Python-authored rules verbatim because
the engine only ever sees generated DRL.
- Bindings are DEMAND-DRIVEN: `p.field` used in a later constraint,
  RHS arg, aggregate arg or salience materializes a `$b : field`
  declaration in its owning pattern (a two-pass render: demands
  collected before patterns print — join constraints reference
  earlier patterns).
- The authoring layer re-encodes the certified walls as guided
  CompileErrors AT DEFINITION TIME: Python callables anywhere in
  conditions/salience ("cannot run in the match loop"); nested salience
  arithmetic (closed grammar is one binary op); collect sources
  referencing other patterns (RIA subnetworks, D-041); min/max-over-
  float results anywhere downstream (opaque Number, D-039); ANY
  accumulate result in salience (unprobed, D-043); bindings inside
  not/exists (Drools scope); non-@fact classes, unsupported
  annotations, incomplete insert field sets, cross-type constraints.
- Tests: golden DRL for every construct, every golden construct parsed
  and fired by the real engine, one fencing test per wall, and a
  parity test proving authored rules and hand-written DRL produce
  identical firing sequences and derived facts.
- Packaging: maturin mixed layout — `seine` is a Python package
  (authoring + wrappers) over `seine._native` (the D-044 boundary).
  Zero engine-code changes.

### D-044: Layer-1 Python bindings — the boundary adds ZERO semantics
PyO3/maturin crate (`bindings/`, workspace non-default member; native
gates never build it). Facts cross as Arrow columnar batches via the
PyCapsule C-stream interface (polars/pyarrow/pandas>=2.2 in,
`seine.Table` out — importable zero-copy on the Python side); Python
holds integer HANDLES into the Rust arenas, never per-fact objects.
Contract, enforced by construction and by bindings/tests/test_boundary.py:
- **Zero semantics in the binding:** exact widening only (i8/16/32,
  u8/16/32 -> i64; f32 -> f64), done in Rust; f64 round-trips are
  bit-exact (tested). NULLS ARE REJECTED loudly — the certified subset
  has no null semantics, and silently zeroing them would void the
  differential guarantees on real data. Unsupported Arrow types
  (dates, decimals, dictionaries, ...) are TypeErrors.
- **One-shot sessions:** build -> insert -> fire() -> read; a second
  fire() raises. The certified envelope is insert-all-then-fire-once;
  incremental refiring is NOT exposed until the harness certifies
  multi-fire scenarios.
- **Callbacks are observers:** `on_fire(rule, [(type, handle)])` runs
  after the GIL-free fire_all completes, in firing order — 
  observationally identical to streaming for an immutable one-shot
  result, and working memory is unreachable from the callback by
  construction (the declarative RHS remains the only mutation path).
- **Results = WM delta first:** result.derived() (facts inserted by
  rules, per type), result.deleted_handles(), result.facts() (final
  view), plus a long-format firing audit (seq, rule, pos, type, handle,
  values_json with POST-RHS renderings, D-013 semantics).
- **Parity tests:** corpus scenarios (salience expressions, accumulate
  reversal, join refire ordering) pushed through the Python API fire
  identically to the native harness — rule sequences and rendered
  values compared row-for-row.
- Rules are DRL strings; every subset wall stays a parse/compile error
  surfaced as a Python ValueError. Layer 2 (Pythonic authoring) will
  COMPILE TO DRL TEXT so the differential harness covers
  Python-authored rules verbatim.

### D-043: salience EXPRESSIONS pinned (se1..se15) — implementation contract
Scope: `salience( <term> [op <term>] )` with op in {+,-,*}, terms = int
literals or numeric LHS bindings (i64/f64). Method calls, full MVEL
bodies, float literals and non-numeric bindings are fenced (parse or
compile error), like custom accumulate functions. Pins:
- **Per-activation salience, GLOBAL interleave:** each activation
  carries its own computed salience; the agenda fires strictly by
  (activation salience DESC, rule decl-index ASC) across rules —
  RA(7), RB-static(5), RA(3) (se1/se2/se5). Mechanism: dynamic-salience
  RuleExecutors keep a per-activation priority queue; the OUTER
  RuleAgendaItem's salience continuously tracks its queue TOP (0 when
  empty or not yet evaluated), re-sorting the item (RuleExecutor.
  updateSalience / getNextTuple / MatchConflictResolver).
- **Evaluated at activation CREATION and at RE-ADD of a fired
  activation; a QUEUED activation keeps its ORIGINAL salience through
  property restages** (se3/se4; PhreakRuleTerminalNode.doLeftUpdates
  only calls update(salienceInt) on the !isQueued path). Late high
  activations jump the line (se10).
- **Within-rule ties (dynamic only): NEWEST activation first**
  (activation-number DESC, se13) — unlike static rules' FIFO tupleList.
  Cross-rule ties: decl order (se6).
- **Numerics:** the expression evaluates in the binding's type and the
  result passes through Java Number.intValue(): i64 results take the
  LOW 32 BITS (se14: 3e9 wraps negative), f64 results truncate toward
  zero with i32 saturation, NaN -> 0 (se8: 6.5 -> 6; se15: -0.5 -> 0).
- Static `salience N` rules keep the FIFO executor (no queue) — all
  existing corpus semantics unchanged.
- Accumulate-result bindings are excluded from generated salience
  expressions (typing unprobed); a salience expression with only
  literals still marks the rule DYNAMIC (Drools isDynamic()).
- CERTIFIED: zero divergences over 5 seeds (42/7/123/777/999 x 10,000
  = 50,000 cases with salience expressions in the grammar; round 2,
  witnessed to completion; not a single xfail drawn).
- Campaign pins (round 1): re-added fired activations KEEP their
  original activation number — dynamic ties order by FIRST creation,
  not re-add time (fz_7_6534); removeRuleAgendaItemWhenEmpty applies
  to EAGER evaluations too — an emptied item stops claiming
  shared-node windows (fz_42_8775; the engine's stale queued flag let
  a dead no-loop sharer consume a later batch). Minimizer variants can
  be degenerate (dropped guards -> fire-limit grinds): tools/minimize.py
  now times out variants at 120s and treats them as non-divergent.

### D-042: OPEN — not-CE unblock REFIRE ORDER in >=3-pattern rules
Round-4 fuzz (the accumulate-era grammar reshuffle) drew two cases the
engine gets wrong ONLY in the relative refire ORDER of tuples unblocked
together at a not node inside a >=3-pattern rule under churn
(fz_7_2364: [T0, T1-join, not]; fz_999_8145: [T0, not-in-list, T2-join],
no-loop). All values, sets and counts agree; the order of exactly two
simultaneously-reactivated activations is swapped.
- Probe matrix (nb1..nb6, promoted where passing): level-1 nots agree
  in all entry styles (nb1 modify-entry, nb2 delete-of-a-left-blocker);
  level-2 nots agree for INSERT-entering blockers (nb5/nb6) but diverge
  for MODIFY-entering blockers whose delete also removes a blocked left
  (nb3 = minimal: 2 rules, 4 facts).
- Mechanism NOT yet pinned: PhreakNotNode doRightInserts/doRightUpdates
  /doRightDeletes all walk memories FORWARD per source, addBlocked
  prepends, TupleList.add appends, and BetaNode.modifyObject turns an
  alpha-entering modify into assertObject — every combination derived
  from those primitives reproduces the ENGINE's order, not the oracle's.
  Reversing our unblock walk fixes nb3/fz_7_2364 but breaks
  nb1/nb2/nb6 and 4 corpus cases (tried, reverted). Suspects for next
  session: the temp-blocked machinery (updateBlockersAndPropagate) and
  segment-level staging interleave for the modify-entry window.
- Quarantine: scenarios/xfail/ holds the four artifacts + nb3; make
  diff excludes the directory; fuzz reports drawn xfail cases as XFAIL
  (name match) without recording them as failures. The certification
  claim is CLEAN MODULO these documented xfails.
- INSTANCE 3 (fz_27182_1227, salience-era grammar shuffle): the class
  also triggers with an INSERT-entered blocker when additional LEFTS
  arrive while the not is blocked (mixed-batch blocked list; minimized:
  static-salience 3-pattern self-join, no salience expressions
  involved). Same order-only signature; added to the quarantine under
  the accepted carve-out.
- RESOLUTION (user decision, 2026-07-04): the carve-out is ACCEPTED as
  documented rather than pursued — the class is rare (2 in 50k draws),
  order-only, and mechanism-ambiguous after deep source reading. The
  quarantine and this record ARE the fix of record; revisit only if
  fuzz surfaces a VALUE-bearing variant or new evidence pins the
  mechanism.

### D-041: addAll is BLIND; clashes resolve at child-touch time (fz_123_8822, fz_7_2843, fz_999_7966, fz_999_4371, mg1..mg8, mn1..mn7)
The accumulate-era fuzz waves exposed four intertwined pins:
- **Cross-window child clashes (fz_123_8822 kernel 1, fz_7_2843,
  fz_999_7966):** TupleSetsImpl.addAll is a BLIND tail concatenation.
  A child touched in a later window is reconciled at TOUCH TIME inside
  doNode against the FIRST sink's pending staging
  (updateChildLeftTuple / deleteChildLeftTuple / normalizeStagedTuples):
  a pending INSERT moves INTO the current batch keeping its insert
  kind (positioned by the new batch's order); a pending UPDATE moves
  as an update; a delete of a pending insert cancels outright.
  Engine: do_node threads the first sink's pending (Out::child_*);
  append_into_pending is now pure concatenation. The accumulate
  result-child staging mirrors propagateResult (normalize + addUpdate
  — the kind is NOT preserved there, unlike updateChildLeftTuple).
- **Materialized peers (fz_123_8822 kernel 2):** processPeerInserts on
  an EXISTING peer runs updateChildLeftTupleDuringInsert; when the
  peer is unstaged and already lives in the peer node's LEFT MEMORY,
  the net effect is a memory removeAdd (move to the END, key kept)
  with NOTHING staged: the re-delivered peer neither re-joins nor
  refires, but subsequent right-inserts see the moved position
  (Node::peer_merge_left). Terminal peers in the same corner would
  arrive as UPDATEs (hasNodeMemory=false) — not yet exercised by any
  case; noted as a watch item.
- **Collect gate correction (mg1..mg8, superseding D-040's first cut):**
  the LIA->collect modify gate = pattern-0's CONSTRAINT fields (its
  listened properties — bare bindings do NOT count) + the collect
  source's beta references into pattern 0. Consequence usage (mg2) and
  later patterns' references (mg8) do NOT inherit through the collect.
- **Subnetwork fence (fz_999_4371, mn1..mn7):** a collect source
  referencing outer bindings builds an RIA SUBNETWORK; there Drools
  false-admits a pattern-0 fact that FAILS its alpha when a mask-missed
  property modify arrives (mn6: `T0($b : f1, f0 == false)` matched a
  fact with f0=true after a setF1 modify; the inline-accumulate
  equivalent mn7 behaves correctly). Subnetworks are unported; the
  parser now rejects variable references inside collect sources and the
  generator no longer emits them.

### D-040: COLLECT swallows unreferenced left MODIFIES (lu_a..lu_h)
fz_42_2091: a rule `T2($b : f1) collect(T0(...)) accumulate(...)` did
not refire in Drools when another rule property-updated the T2, but the
engine refired. Discriminators:
- plain-join control refires (lu_b); inline accumulate FIRST refires
  (lu_c, lu_e); collect at level >=2 refires (lu_d); collect FIRST
  swallows (lu_f, lu_h) even when the update writes a DIFFERENT value
  (lu_a — not value-comparison);
- giving the collect source a beta constraint on the left binding
  restores the refire (lu_g).
Mechanism: `from collect` builds an AccumulateNode around
CollectAccumulator (CollectBuilder), which is structurally known to
read NOTHING from the left, so the node's left declared mask is just
its beta constraints' left references plus inherited downstream
interest; the LIA's per-sink mask check then DROPS pattern-0 modifies
that miss it. Inline accumulates compile opaque lambdas -> ALL-SET
left masks -> always re-propagate. Engine: level-1 collect trie nodes
carry `collect_left_gate` (union over sharing rules of pattern-0
fields referenced by later patterns' constraints and RHS args); the
LIA skips staging a MODIFY into such a child unless the mask
intersects (bare updates = ALL-SET pass). Deeper collects are
unfiltered — inter-beta propagation carries no masks.

### D-039: accumulate-result compile TYPING (27-case matrix, tc_*/rc_*)
Inline-accumulate results carry a compile-time Java type:
sum(double)->Double, sum(long)->Long, count->Long, average->Double,
min/max(long)->Long, but **min/max(double) -> opaque Comparable/Number**.
Usability follows Java assignability exactly:
- Downstream comparisons (`field <op> $r`, MVEL): Double/Long results
  compile against ANY numeric field; opaque results compile against
  NOTHING (fz_4242_490, tc_m1/m5/m6/m7 vs tc_s1..s4/c1/c2/a1/a2/m2/m3/m4).
- RHS constructor args: Long -> long or double (widening), Double ->
  double only (never long: rc_sf_i/rc_a_i errors), opaque -> nothing
  (rc_mf_f/rc_mf_i; fz_4242_99).
- Engine wall: min/max-over-f64 results error in comparisons AND RHS
  args; all other results flow with their natural field type (the
  existing I64->F64 widening matches Long->double). Generator mirror:
  min/max-over-f64 results are not bound outward; other results join
  and feed RHS args freely.
- The 19 COMPILING matrix combinations are corpus probes; erroring ones
  stay out (both-error cases are flagged by the judge as likely out of
  subset, by design).
- collect results are bound but referenced nowhere downstream (not
  registered as field bindings; List constraints fenced at parse).

### D-038: accumulate/collect semantics PINNED (probes acc1..acc16)
Phase 3b scope: inline `accumulate( <src> ; $r : func($a) )` with the
built-ins sum/count/average/min/max, plus `ArrayList()/List() from
collect( <src> )`. Custom accumulate functions, multi-function
accumulates, `from accumulate`, result-pattern constraints, and fact/
extra bindings inside the source are FENCED (parse errors). Pins:
- **Match rendering:** the accumulate CE contributes its RESULT object
  to the match (a Number; collect: a Collection) — a leading accumulate
  is CE-first and matches on InitialFact too (acc1). The oracle
  canonicalizes Numbers as {type: Long|Double, fields:{value}} and any
  Collection as {type: "Collection", fields:{value:[<renderings>]}}
  with ORDER-significant elements; java.util imports are added to the
  oracle prelude.
- **Result types:** sum(i64)->Long, sum(f64)->Double, count->Long,
  average->Double, min/max -> the argument's type (acc1).
- **Empty-source results:** sum->0/0.0 and count->0 still fire;
  average/min/max of an empty set return NULL and the tuple does NOT
  propagate (no firing; a previously-propagated child is retracted) —
  default accumulateNullPropagation=false (acc2/acc10).
- **EXACT float sequencing (the heart of the port):**
  - initial fold consumes staged inserts NEWEST-FIRST: sum{0.1,0.2,0.3}
    printed exactly 0.6 = (0.3+0.2)+0.1, and average's total matched the
    same order (0.6/3 = 0.19999999999999998) (acc1);
  - deletes REVERSE the stored per-match contribution: 0.6 - 0.2 =
    0.39999999999999997, not a 0.4 recompute (acc4);
  - updates are reverse(stored)+accumulate(new): (0.6-0.2)+0.25 =
    0.6499999999999999 (acc5); inserts add to the running total (acc6);
  - min/max do not support reverse: a removal reinits and REFOLDS over
    the remaining match list (order-insensitive result);
  - a value-unchanged mask-overlapping update still runs the
    reverse+accumulate pair AND refires (acc7); a mask-miss update
    (fields outside source constraints + arg binding) does nothing
    (acc13).
- **collect:** ArrayList semantics — initial fold appends newest-first
  ([0.3,0.2,0.1] for insertion order 0.1,0.2,0.3), reverse removes
  IN PLACE preserving order, later inserts APPEND ([0.3,0.1,0.4])
  (acc8). Empty collect propagates an empty list.
- **Per-left contexts** with beta-constrained sources (k == $x), the
  result usable in later patterns and RHS args (acc9); accumulate
  composes with not/exists and multiple accumulates per rule
  (acc14..16). Left updates: bucket-unchanged still-matching matches
  KEEP their stored contributions (our functions have no required
  left declarations); a join-key change reinits and refolds over the
  new bucket (acc12: 0.7); a dying left just discards its context
  (acc11).
- PhreakAccumulateNode phase order (sources): leftDel, rightDel,
  rightUpd (join-style right reorder), leftUpd (left reorder),
  rightIns, leftIns; touched lefts collect into a temp TupleSets and
  results evaluate at the END (temp inserts head-first, then updates),
  each ensuring/updating a REUSED result fact handle and staging the
  single result child as insert/update/delete-on-null.

### D-037: TRUE SHARED-NODE TRIE + name-sensitive constraint identity
### (fz_42_297/580/952, probes ne_t13..t15) — supersedes D-036's
### "per-rule copies suffice" conclusion
The D-036 wall-lift exposed a coverage hole: with the corrected identity,
random constraint draws essentially never collide, so 3000 unwalled cases
contained ZERO true shared prefixes. The generator now REUSES an earlier
rule's pattern prefix (~15% of rules, bindings renamed) — and the very
first reuse-enabled run produced 3 divergences that per-rule networks
cannot reproduce:
- **fz_42_580 (minimized: identical-LHS twins at different saliences,
  facts arriving across two windows):** the shared join evaluates ONCE
  per window at the first-reached sharer's turn; the lagging sharer
  receives PER-BATCH copies. Its terminal accumulates the preserved
  copies FIFO (TupleSetsImpl.addAll walks to the tail) while flipped
  peer copies stack LIFO (per-tuple prepends). A per-rule network copy
  evaluating everything in one merged batch produces a different join
  order (the oracle fired batch 1 before batch 2, each batch internally
  reversed vs the eager sharer's order).
- Engine restructured accordingly: `Lia` + `TrieNode` shared instances
  (one phreak::Node per structurally-equal prefix; level-1 nodes hold
  the eagerly-copied pos0 staging), per-rule state reduced to the
  terminal queue + `term_pending`. evaluate_rule walks the rule's trie
  path; each dirty node consumes its staging once and propagates every
  batch to ALL sinks in build order — first sink via append_into_pending
  (addAll semantics), later sinks via flipped merge_into_pending copies.
  The claim-by-window behavior falls out of the agenda order plus the
  queued/linked gates; the D-033 static flip machinery is deleted
  (subsumed). k=1 rules keep their per-rule pos0 staging (pr04/pr08).
- **fz_42_297 (minimized: twins whose join constraint references
  differently-NAMED bindings)** pinned one more identity component:
  a constraint that REFERENCES a binding compares by its expression
  text, so `f1 != $x` and `f1 != $y` do NOT share even though $x/$y
  bind the same field (ne_t13), while same-named references share
  (ne_t14, not-CE variant ne_t15). Unreferenced declarations remain
  name-irrelevant (ne_t2/t6/t8/t9). pattern_key now includes the
  variable name (plus its source position) for Var-rhs constraints.
  Generated rules name bindings per-rule, so reused prefixes with join
  constraints correctly do NOT share — the fuzzed sharing surface is
  bare/literal prefixes and unreferenced bindings, matching Drools.
- ne_t11's clean result was circumstantial (single batch); D-036's
  claim that per-rule copies suffice is RETRACTED — the trie is the
  faithful model.
- Wave 4 (fz_7_2122, fz_999_3298 — the first trie campaign):
  - **Per-event link effects:** within ONE WM action, Drools propagates
    through the alpha sinks sequentially, so an intermediate node link
    (a not node re-linking on its blocker's delete) transiently links a
    path and QUEUES its item even though a LATER node of the same action
    unlinks the path again (fz_7_2122: the queued sharer then claims the
    unblock window, splitting batches). The engine now runs link/queue
    bookkeeping after EVERY node staging event instead of once per
    action.
  - **Peer-merge clash semantics (fz_999_3298):** a peer-copy UPDATE
    that touches an already-staged tuple is SKIPPED — the entry keeps
    its position AND kind (processPeerUpdates' staged-type check) —
    unlike the intra-chain merge where an update moves a pending insert
    to the head. Peer INSERT clashes do move to the head
    (updateChildLeftTupleDuringInsert). peer_merge_into_pending walks
    the source lists head-first with per-entry prepends, so the
    batch-reversal and LIFO batch stacking emerge rather than being
    applied as a wholesale flip.
- Corpus: **245/245** (5 fuzz regressions + 4 minimized twins + ne_t13..15).
- Final campaign over the FULLY-unwalled, reuse-enabled grammar (shared
  prefixes x mutation x CEs x salience mixing freely, ~1% of cases with
  true shared >=2-pattern prefixes): seeds 42/7/123/777/999 x 10,000 =
  **50k cases, ZERO divergences**.

### D-036: Sharing identity CORRECTED (bound-field set); D-035 wall LIFTED;
### window-claim theory RETRACTED (probes ne_t1..ne_t11)
Session 5. Re-examining the D-035 xfails with fresh probes disproved the
"dynamic window-claim" model and dissolved the whole open class:
- **Node-sharing identity includes the SET of field-bound fields.**
  ne_t1: different bound fields -> NO sharing (both rules fire unshared
  orders). ne_t3: a bare pattern does not share with a binding pattern.
  ne_t10: same LISTEN MASK but different declaration sets (constraint
  `f0 > 0` vs constraint + `$x : f0`) -> NO sharing — it is the
  DECLARATION set, not the property mask. Binding names (ne_t2/ne_s5),
  order (ne_t6), duplicates (ne_t9), constraint/binding interleaving
  (ne_t7) and fact-level `$p :` bindings (ne_t8) are all irrelevant.
- **The static build-order flip model was right all along**
  (SegmentPropagator.processPeers: the ORIGINAL staged list goes to the
  FIRST-built sink segment via addAll; every later peer gets prepended
  copies — one flip). ne_t5: the first sink keeps the preserved list
  even when its path NEVER LINKS (extension pattern unsatisfiable) —
  there is no runtime claim. fz_42_8472, the case that motivated the
  window-claim theory, is explained by identity alone: its "sharers"
  bound different fields (R3 {f1} vs R4 {f0}), so Drools never shared
  them and the engine's binding-blind pattern_key applied a flip that
  should not exist. Same story for fz_7_2081 ({f0} vs {}), fz_7_2859
  ({f1} vs {f0,f1}) and fz_777_7592 — ALL FOUR xfails pass with the
  corrected key and are graduated to regressions. xfail/ is gone.
- **True sharing x mutation behaves correctly under per-rule networks**
  (ne_t11: identical bare twins + a mid-run delete + a third late-
  salience sharer — engine matches oracle), so no shared-segment
  architecture is needed for the current subset; per-rule copies with
  static sink-order flips are behaviorally equivalent on everything
  pinned so far.
- Engine: CompiledPattern.bind_fields (bitmask, set semantics) folded
  into pattern_key. Generator: the D-035 wall is REMOVED (shared
  prefixes fuzz freely again) and the delete distribution is restored
  to its historical independent form; the wall's key-threading
  scaffolding is deleted. Dead code cleanup: the unused FIFO staging
  variants and Node.first are gone.
- Corpus: **233/233** (ne_t1..ne_t11 promoted; 4 ex-xfails graduated).

**HANDOFF @ external-WM close (Session 6, 2026-07-04)** — D-047
certified external update/delete by handle end to end (probe wave,
window-queue and slot-memory semantics, 5x10k round-3 clean) and the
Python boundary exposes it (update/delete by handle, handle-returning
inserts). The full working-memory lifecycle now crosses the boundary:
insert -> fire -> update/delete -> fire, all differentially certified.
Row-object sugar and wheel CI landed (D-048). No remaining planned items.

**HANDOFF @ multi-fire close (Session 6, 2026-07-04)** — D-046
certified the incremental envelope (epochs in harness + generator,
5x10k clean) and the bindings' one-shot restriction is lifted: sessions
insert/fire repeatedly with per-fire deltas. v0.1.0 tags the prior
one-shot state. Remaining ideas (none started): external update/delete
by handle (needs its own probe wave — only inserts cross the boundary
today), row-object ingestion sugar, wheel CI.

**HANDOFF @ bindings Layer 2 (Session 6, 2026-07-04)** — Pythonic
authoring shipped (D-045): @seine.fact classes + Rule builder compile
to DRL text; all certified walls re-surface as definition-time
CompileErrors with pointed messages. 32 Python tests + native gates
green, zero engine diff. The notebook story is complete end to end:
dataclass-style schemas -> Python rules -> certified engine -> Arrow
results. Possible next steps (none started): incremental multi-fire
certification (harness scenarios first, then lift the one-shot
restriction), pandas/pydantic row-object ingestion sugar, wheel CI.

**HANDOFF @ bindings Layer 1 (Session 6, 2026-07-04)** — `seine` is
now importable: `seine.run(drl, {"T": polars_df})` runs the certified
engine over Arrow batches and hands back the WM delta + firing audit as
Arrow (D-044). Gate: 15 boundary tests (fidelity/rejection/lifecycle/
parity) + native corpus 360/360 + zero engine-code diff. Dev loop:
`VIRTUAL_ENV=<venv> maturin develop -m bindings/Cargo.toml && pytest
bindings/tests/`. Next (Layer 2, NOT started): dataclass/Pydantic fact
schemas and Python rule authoring compiling down to DRL text — the
grammar is frozen in engine/src/drl.rs; anything it can't express stays
a compile error (the custom-accumulate fencing pattern).

**HANDOFF @ salience-expressions close (Session 6, 2026-07-04)** —
D-043 landed on `salience-expr` and merged: computed salience over
numeric bindings with the full agenda lifecycle (per-activation values
fixed at creation/re-add, sticky item salience, newest-first dynamic
ties by PERSISTENT activation number, eager dynamic rules, intValue()
numerics). Certified zero divergences over 5 seeds x 10k. The engine
subset is now feature-complete per the original Phase-3 scope: joins,
property reactivity, CEs, operators, accumulate/collect, salience
expressions. Open: the D-042 order-only carve-out (3 quarantined
instances). Fenced by design: custom accumulate functions, `from
accumulate`, subnetwork collects, MVEL salience bodies.

**HANDOFF @ Phase 3b close (Session 5, 2026-07-04)** — accumulate/
collect landed on the `accumulate` branch (D-038..D-041) with the exact
float op-sequence port (stored per-match contributions, reverse/
reaccumulate, result-handle reuse, null retraction), the result-typing
walls, the collect left-modify gate, the subnetwork fence, and three
deep propagation corrections the new grammar exposed in PRE-EXISTING
paths: blind addAll with touch-time clash resolution, the normalized-
delete peer channel, and materialized-peer semantics at nodes and
terminals. Certification: corpus 337/337, `make test` green, 5-seed x
10k campaign = 0 divergences, 2 documented xfails (D-042).
- D-042 is OPEN: not-CE unblock refire ORDER in >=3-pattern rules with
  modify-entering blockers (nb3 is the 2-rule/4-fact minimal). The
  quarantine (scenarios/xfail/ + fuzz XFAIL reporting) keeps the gate
  honest. Next session: pin the mechanism (suspects: temp-blocked /
  updateBlockersAndPropagate machinery, segment staging interleave for
  the modify-entry window), fix, dissolve the quarantine.
- MERGED to main with the D-042 carve-out accepted as documented
  (user decision): clean-modulo-2-documented-xfails is the certified
  state of record.
- Remaining unstarted: salience expressions; custom accumulate
  functions and `from accumulate` stay fenced by design.

**HANDOFF @ D-037 close (Session 5, 2026-07-04)** — The node-sharing
model is now a TRUE shared prefix trie (one node instance per
structurally-equal prefix, evaluated once per agenda window, per-batch
propagation to all sinks). Proven state at close:
- Corpus 245/245 (`make diff`); `make test` green. No xfails, no walls:
  mutation, 3-pattern rules, CEs, and shared prefixes (incl. the
  generator's deliberate ~15% prefix reuse) all mix freely.
- Fuzz: 50k cases (5 seeds x 10k) on the final grammar, zero
  divergences.
- Sharing identity (D-036/D-037): type + CE kind + ordered constraints
  (var references compare BY NAME, ne_t13/t14) + the bound-field SET
  (ne_t1..t10); binding names/order/duplicates and fact-level bindings
  irrelevant unless referenced.
- Propagation (D-037): first-built sink gets addAll-appended batches
  (FIFO for laggards); later sinks get per-entry prepend peer copies
  (reversed per batch, LIFO stacking) with skip-if-staged update clashes;
  link/queue effects run per node event within a WM action.
- If resuming: (1) accumulate/collect remain unstarted (largest PHREAK
  node; oracle needs a Number-rendering canonicalization like
  InitialFact's); (2) salience expressions (dynamic-salience agenda);
  (3) scale campaigns stay cheap insurance — this session's classes
  (D-033/D-035..37) all hid below ~1/10k draw rates until the generator
  was taught to draw them.

**HANDOFF @ Phase 3 close (Session 4, 2026-07-04)** — Stretch items
`matches`/`contains`/`in` and `not`/`exists` are DONE per D-034's bar;
`accumulate`/`collect` and salience expressions were NOT started (scoped
out, independently optional per brief §2). Proven state at close:
- Corpus 218/218 (`make diff`); `make test` green (12 unit tests incl.
  the regex matcher's oracle-pinned cases). FOUR xfail cases parked
  (xfail/: node-sharing window-claim classes, D-035).
- Fuzz over the D-035-walled grammar (operators + CEs + mutation +
  3-pattern rules; no shared prefixes): seeds 42/7/123/777/999 x 10k =
  50k cases, zero divergences; plus the 30k operator-only wave earlier
  and the ~50k unwalled cases that surfaced the D-032/D-033/D-035
  classes along the way.
- New mechanism classes this session: D-030 (operator semantics + the
  in-list prefix-chain rule), D-031 (existential blocker model, CE match
  rendering, InitialFact, not-node linking pulse), D-032 (queue-on-unlink
  agenda transitions; comparison/range indexes on existential nodes),
  D-033 (node-sharing segment-boundary flips — affects pure-join
  programs too; identical-LHS twins fire in opposite orders).
- Environment for a fresh session: PATH needs `~/.cargo/bin`; JVM 21 +
  Maven resolve Drools from `~/.m2` (pinned 9.44.0.Final). drools-core
  and drools-base -sources jars live in `~/.m2` for READING (behavior
  reference only; re-fetch via `mvn dependency:sources`).
- If resuming: (1) the D-035 open class — model true shared segments
  (one node instance per shared prefix, evaluated at the first-reaching
  item's window) and lift the generator wall; xfail/fz_7_2081+2859 are
  the acceptance tests; (2) accumulate/collect — probe first:
  match-object rendering of Number results needs an oracle
  canonicalization like InitialFact's; PhreakAccumulateNode is the
  largest remaining node; (3) salience expressions need the
  dynamic-salience agenda queue; (4) scale campaigns (more seeds /
  larger CASES) are cheap insurance — the D-033 class showed rare
  shapes can hide for 100k+ cases.

## Phase 3 (stretch: operators, not/exists — 2026-07-04)

### D-033: CE fuzz wave 2 — NODE-SHARING SEGMENT FLIPS (fz_123_3881,
### fz_7_6245; probes ne_s1..ne_s10) — a pre-existing latent gap closed
Seeds 7/123 each found one divergence; both minimized to rules SHARING a
beta prefix. Discriminator ladder ne_s1..ne_s10 pinned a mechanism that
affects PURE-JOIN programs too (ne_s3!) and had simply never been drawn
observably in the previous ~130k fuzz cases (needs two rules with
structurally identical pattern prefixes, diverging continuations, and
>=2 facts on a shared non-first pattern):
- **Rules with structurally equal pattern prefixes share beta nodes.**
  Binding names are irrelevant (ne_s5); literals compare by their D-029
  alpha-node identity; each rule's terminal is always its own sink.
- **Where sharers diverge, the shared node is a segment tip: the
  FIRST-declared sink's continuation receives the staged propagation
  as-is; every LATER sink receives a REVERSED copy.** Consequences, all
  oracle-pinned: a 3-pattern extension of a shared 2-pattern prefix
  fires its tuples in the OPPOSITE order of the unshared control
  (ne_s3 vs ne_s4); identical-LHS twin rules fire in opposite orders
  (ne_s7: R1 ascending, R1b descending); swapping declaration order
  swaps who is preserved (ne_s8: both DESCEND — the not-rule, now the
  first sink, keeps the unshared order while the 2-pattern rule flips);
  three sinks each flip once (ne_s9); boundaries stack per depth
  (ne_s10). Trailing not/exists after a shared prefix (the original
  fz_123_3881) is just this flip passing through the CE node.
- Engine: compute_segment_flips derives per-(rule, node) flip flags at
  build time (prefix keys on the pre-D-029-rewrite compiled patterns);
  evaluate_rule reverses a node's staged output lists when its
  continuation is a non-first sink. Per-rule networks otherwise remain
  independent — sharing is modeled ONLY as this boundary flip.
- The D-028-era "proven" claim implicitly excluded shared-prefix
  programs; the corpus (203 scenarios) passes unchanged with the flip in
  place, confirming no prior scenario exercised the shape observably.

### D-035: OPEN class + wall — node sharing beyond the static case
Seed 7's rerun after D-033 produced fz_7_2081/fz_7_2859 (xfail/):
programs where rules SHARE a beta prefix AND mutate (delete) facts that
feed the shared join. Drools evaluates a shared node ONCE, in the window
of whichever sharer's agenda item is reached first, then propagates to
all sinks; our per-rule copies evaluate at each rule's own window, so
batch boundaries diverge under mutation (enumeration and requeue orders
shift). The D-033 flip covers sharing for INSERT-ONLY programs — pinned
by ne_s1..s10 plus ne_s11 (multi-window insert arrivals PASS).
- fz_42_8472 (insert-only, STATIC!) then showed the D-033 flip's owner
  is not declaration order: sharers R3 (salience -1, extension pattern
  EMPTY -> path never links) and R4 (salience -5) fired R4 UNFLIPPED.
  The consistent model over all seven data points (ne_s7/s8/s9/s10/s11,
  fz_123_3881, fz_42_8472): **the sink on the path whose agenda item
  actually EVALUATES the shared segment first receives the staged list
  direct (preserved); the other sinks get flipped copies at that
  moment.** With equal salience and all sharers linked, first-evaluated
  = first-declared — the statically-modeled class that ne_s1..s11 pin
  and the engine reproduces. Salience differences or unlinked sharers
  move the claim at runtime — modeling that faithfully requires true
  shared segments (one node instance, one evaluation window).
- WALL (generator): NO generated program emits two rules with
  structurally identical pattern prefixes >= 2 patterns — canonical
  per-pattern keys (type, CE kind, non-binding constraints with eq
  literals field-type-normalized, var refs by source tuple position)
  are tracked per scenario and colliding rules are regenerated
  (fallback: single-pattern). Deletes are gated on allow_mutation.
  The static equal-salience linked class stays pinned by the curated
  ne_s corpus; everything else is the open class (xfail/).
- Also from this wave (fz_777_6791, insert-only — NOT the walled class):
  **a range-INDEXED constraint is never re-evaluated after the index
  probe, and the probe COERCES to the stored side's type** (TupleIndexRBTree
  coerceType + SingleBetaConstraints' indexed skip). With i64 rights and
  an f64 binding, `exists B(y >= $x)` matches y=2 against $x=2.5 (the
  probe truncates, ne_r3) and the not-mirror never blocks/refires
  (ne_r5 pins the left-tree direction). Engine: allowed_ce skips the
  index_ci constraint for existential nodes; the range scans' stored-type
  coercion is authoritative. Probes pr_ne_r3/r4/r5 + regression
  fz_777_6791. (Un-indexed relational CE constraints — e.g. a second
  var constraint beyond the indexed one — still evaluate promoted.)
- Next session: model true shared segments (one node instance per shared
  prefix, evaluation at the first-reaching item's window, propagation
  into per-rule continuations) and lift the wall; the four xfail cases
  are the acceptance test.

### D-034: Phase 3 DONE-BAR (operators + not/exists; accumulate NOT started)
- Curated corpus: **218/218 PASS** (`make diff`) — D-028's 156 plus
  pr_op_* (14), pr_ne_* (41 incl. the ne_s sharing ladder and ne_r
  range-index probes), and 7 CE fuzz regressions incl. minimized twins.
- Operator grammar fuzz: seeds 42/7/123 x 10,000 = 30k cases, zero
  divergences (before the CE grammar landed).
- CE grammar fuzz (not/exists + operators + mutation + 3-pattern rules
  mixing freely, D-035-walled: no structurally shared >=2-pattern
  prefixes across rules): seeds 42, 7, 123, 777, 999 x 10,000 = 50k
  cases at zero divergences after the D-032/D-033/D-035 fixes.
- Generator termination discipline extended for CEs (D-032): RHS insert
  types must exceed ALL pattern type indices including not/exists CE
  types, so consequence chains can never re-insert a blocker/support at
  or below their own LHS; refire counts stay bounded by the finite event
  pool of lower types (induction over the type order). CE patterns carry
  no bindings; mutation targets and RHS getters reference positive
  patterns only; first-position CEs generated at low probability
  (InitialFact path).
- NOT started (documented out of this run's scope): accumulate/collect
  (largest remaining PHREAK node; needs oracle-side Number rendering in
  match lists), salience expressions (dynamic-salience agenda queue).
  Both remain independently optional per brief §2 Phase 3.

### D-030: matches/contains/in semantics PINNED (probes op_m*/op_c*/op_i*)
Oracle-verified on Drools 9.44.0.Final; probe files promoted to
scenarios/probes/pr_op_*.json:
- **`matches` is java.util.regex full-string matching** (String.matches):
  `s matches "b"` does NOT match "abc" (op_m2); `""` matches only the empty
  string (op_m5). Classes/ranges/negation `[^a]`, alternation, groups,
  `. * + ?` behave standard (op_m4). It even COMPILES on numeric fields
  (op_m3: `n matches "1"` fires — value stringified); SUBSET WALL: the
  engine restricts `matches` to String fields with literal String rhs, so
  the engine is stricter than Drools here (safe: generator never emits it).
- **`contains` on a String field is substring semantics** (op_c1), and
  `contains ""` matches every string (op_c2). Wall: String field + literal
  String needle only (our fact model has no collections).
- **`in`/`not in` are a composite OR of `==`-with-promotion branches**:
  a double literal in the list does NOT truncate against a long field
  (op_i3: `n in (2.5, 9)` skips n=2), int literals promote against double
  fields (op_i3b), string and bool lists work (op_i5).
- **`in` does NOT participate in D-029 alpha eq-node machinery**: its
  branches don't count toward the >=3 hash threshold (op_i4: `n == 2.5`
  beside an in-rule stays sub-threshold/promote) and don't share nodes
  with plain `==` constraints (op_i6: `in (1.5, 9)` does not inherit the
  `n == 1` node's literal). BUT an in-constraint DOES contribute to the
  preceding-constraint prefix chain that scopes downstream eq-node groups
  (op_i7: three `n == lit` nodes under a common `m in (5)` prefix hash and
  truncate, while the identical literal at top level stays promote-only).
  Engine: share_and_hash_alphas pushes a descriptor for every constraint
  kind into the prefix; only Cmp/Eq/Lit constraints form group members.
- **Listen masks include fields referenced by the new operators** (op_m6:
  masked update {s,n,t} refires matches/in/contains rules; op_m7: a
  {t}-only update does not refire a rule matching on s).
- Engine regex: a tiny backtracking matcher over the tame subset
  (literals, `.`, classes with ranges/negation, groups, `|`, `* + ?`),
  full-string acceptance — equivalent to Java for this feature set (no
  backrefs/lookaround; acceptance-only so greediness is irrelevant).
  Corpus strings stay ASCII and newline-free (pr09/D-010), so Java's
  `.`-excludes-newline and negated-class-includes-newline edge cases
  cannot arise. Everything else (`{n,m}`, `\d`, anchors, `$`-vars) is a
  parse error = subset wall.

### D-031: not/exists CE semantics PINNED (probe ladder ne_n*/ne_e*/ne_f*/ne_l*)
Oracle-verified on Drools 9.44.0.Final; drools-core sources re-fetched for
READING (behavior reference only — no code copied). Pins:
- **Match rendering:** not/exists CEs contribute NO element to the firing's
  match list (ne_n1/ne_e1); a rule whose FIRST pattern is a CE matches on
  Drools' InitialFactImpl, which appears in the match objects (ne_f1) but
  never in the final fact set. The oracle canonicalizes it as
  `{"type":"InitialFact","fields":{}}` (raw toString carries an identity
  hash — nondeterministic); the engine mirrors with a synthetic reserved
  InitialFact fact inserted before scenario facts when needed.
- **Blocker model** (from sources, behavior confirmed by probes): each left
  tuple holds <=1 blocker (first matching right in bucket order); blocked
  lefts leave the left memory; a right's blocked-list PREPENDS. not
  propagates unblocked lefts, exists propagates blocked ones.
- **Cancellation/refire:** blocker arrival cancels pending not-activations
  (ne_n3); losing the last exists-support cancels pending exists ones
  (ne_e3). Support/blocker HANDOVER (another matching right remains) keeps
  state without firing or cancelling (ne_n7/ne_n10/ne_e3b/ne_e6). Unblocking
  REFIRES an already-fired not match (ne_n5); a mass unblock fires in
  REVERSE left-arrival order (ne_n4: A3,A2,A1).
- **No refire on in-place updates:** a property-relevant update of the
  blocking/supporting fact that leaves the block state unchanged does NOT
  refire the rule (ne_e5: exists refired neither, contrast join j12; not is
  trivially inert while blocked). Only alpha/bucket TRANSITIONS act (as
  right ins/del: ne_n8 fires R after the blocker leaves its alpha).
- **Chains:** CE children pass through later joins as ordinary tuples with
  the standard D-013 prefix reversal (ne_j1 fired A2C7,A2C8,A1C7,A1C8).
- **Linking:** not nodes start LINKED; only UNCONSTRAINED (no join
  constraint) not nodes can unlink — they unlink while rights exist (with
  a one-evaluation link pulse on the 0->1 right insert so the blocking
  batch processes) and re-link when the right count returns to 0. exists
  links like a join (rights nonempty). ne_l1/ne_l2: lefts staged while
  unlinked accumulate; the re-link batch processes right-delete unblocks
  BEFORE accumulated left inserts (ne_l2 fired A0 then A1).
- **doNode phase order (sources):** leftDel, existential-reorder-left,
  existential-reorder-right (captures tempBlocked + tempNextRightTuple =
  next non-staged neighbor forward else backward; re-added updates with
  empty tempNext become their own resume point), rightIns, rightUpd
  (unblocked-pass then tempBlocked walk; a null tempNext flips a loop-wide
  iterate-from-start flag that persists for later rights), rightDel
  (re-search from bucket start, staged-deleted rights ineligible), leftUpd
  (keep still-allowed blocker iff every beta constraint is
  equality-indexable or there is <=1 — isLeftUpdateOptimizationAllowed),
  leftIns. Staged-UPDATE lefts are skipped by every right-side walk
  ("children cannot be processed twice") and re-attached to the current
  right's blocked list when met in a tempBlocked walk.
- Subset walls: bindings ($x : f or fact binds) inside not/exists patterns
  are rejected (Drools scopes them out anyway); bare `not T(...)` /
  `exists T(...)` forms only (no parenthesized CE groups, no nesting); the
  type name InitialFact is reserved.

### D-032: CE fuzz wave 1 — agenda queue-on-unlink + COMPARISON (range)
### indexes on not/exists (fz_42_3774, fz_42_7768)
The first 10k CE-grammar fuzz run produced 2 divergences; both minimized
to <=3 rules / 3 facts (tools/minimize.py, now also dropping constraints
and RHS statements):
- **Queue-on-unlink (fz_42_3774 + discriminators ne_x1..ne_x5):** an
  exists rule whose last support dies and reappears in a LATER firing
  REFIRES (ne_x2), while a same-RHS delete+insert does NOT (ne_x1:
  blocker handover inside one batch keeps the child). Drools source:
  PathMemory.doUnlinkRule — every rule LINKED->UNLINKED transition
  force-queues the agenda item (dirty forced), so the delete window
  evaluates before later re-inserts. Engine: on_delete/on_update capture
  rule_linked before/after and queue on the transition. The not-side
  mirrors (ne_x3: same-batch delete+insert of a blocker never fires the
  not; ne_x4: a low-salience not whose unblock window is preempted by a
  re-insert never fires; ne_x5: a low-salience exists keeps its queued
  activation through a support handover).
- **Range indexes (fz_42_7768/fz_min_7768):** not/exists nodes with a
  relational join constraint and Number/Number or same-class operands are
  COMPARISON-indexed by default (IndexUtil.canHaveRangeIndexForNodeType:
  NotNode/ExistsNode only — join nodes need the opt-in config, which is
  why 50k join-grammar cases never saw it). TupleIndexRBTree semantics
  (behavioral port, phreak::Index::Cmp):
  - memories sort by the constraint operand (left memory by the binding
    value, right memory by the field), FIFO within equal keys;
  - a probe walk starts at the range boundary NEAREST the probe and moves
    away from it: for `field > $b` / `>=` blocked-left scans run
    DESCENDING $b while blocker scans run ASCENDING field; `<` / `<=`
    mirror (fz_min_7768's unblock burst fires the $b=-1 group before the
    $b=-2 group, each in insertion order);
  - probes coerce to the stored side's type (same convention as the hash
    index, u14/fz_123_3057);
  - equality indexes take precedence (any `==` var constraint); `!=` is
    never indexable; comparison memories never capture resume points
    (resumeFromCurrent=false: tempBlocked walks restart from the range
    head, and the doRightUpdates from-start flag initializes true).
- Corpus at 199 after promoting the pair + minimized twins + ne_x probes.

### D-028: PHREAK port LANDED — corpus 145/145, all xfails closed, wall lifted
The faithful port (branch `phreak-port`) replaced the fitted merge engine.
`engine/src/phreak.rs` implements the node algorithm; `engine.rs` keeps
compile/RHS/agenda. Everything below is oracle-pinned (probes pr_c*, pr_d*,
pr_v*, pr_coerce + 20 graduated fz_123_* regressions):
- Staging: TupleSets prepend (LIFO), consumed head-first everywhere; the
  staged-type folds are by OBJECT identity, so a killed-and-recreated child
  is del+ins, never an in-place update (c13). Same-list re-staging is a
  no-op; a walk touching a tuple staged in the DOWNSTREAM pending set moves
  it to the head (updateChildLeftTuple clash rule; merge_into_pending).
- Memories: TupleList append; removeAdd re-keys and moves to the END.
  Child tuples link at the END of both parents' lists; the sync-walk
  insert case threads a cursor (insert-before-cursor keeps alignment).
  Bucket-change vs same-bucket branches per doRightUpdates/doLeftUpdates,
  including the staged-update-left skip ("children cannot be processed
  twice") — right-insert processing has NO effective skip (flags cleared).
- k=1 rules: WM staging consumed OLDEST-first (pr08/pr04 pin).
- Terminal: updates then inserts, head-first, appending to the executor
  queue; queued activations keep position; unqueued (fired) re-append.
- Eagerness is real but only controls WHEN evaluation happens (per flush
  for no-loop rules); it does NOT change consumption order (c7 vs c10-c13
  probe ladder: the j01-vs-9462 "contradiction" was eager evaluation
  windows, not staging conventions).
- Property-miss reAdd: a modify whose mask MISSES a right input still
  removeAdds the right tuple (re-keyed, to memory END) immediately and
  re-appends its children in their left parents' lists — no staging, no
  child updates (fz_42_4359/3433 vs fz_42_1057/fz_123_1438; probes d4-d7).
- Indexed join keys are stored in each side's NATURAL type; the probing
  side coerces to the stored side's type: left-probes-right truncates
  (u14), right-probes-left widens, so long -1 does not find double -1.5
  (fz_123_3057; pr_coerce matrix).
- Agenda-item lifecycle (fz_42_1464 vs fz_42_124): the item is created on
  first LINK; once queued it EVALUATES whenever reached even if currently
  unlinked (memories advance, nothing fires); it is removed when its
  activation queue empties; new staging re-queues it ONLY while linked;
  never-linked rules accumulate staged input unevaluated (fz_7_145).
  The just-fired rule is still force-evaluated (fz_42_5243).
- A 64-combo grid search over staging/consumption directions confirmed
  the source-literal conventions are uniquely optimal; every remaining
  divergence was a missing MECHANISM, not a direction.
D-016/D-017/D-025 are RETIRED: the generator wall is lifted permanently
(gen.rs allows mutation + 3-pattern rules together). D-021/D-022 cascade
heuristics are superseded by the port. xfail/ is gone — all 26 cases are
regressions now.

### D-026: Faithful node-algorithm port — attempted, reverted, groundwork
### banked for next session
A full behavioral port of PhreakJoinNode/PhreakRuleTerminalNode was built
and exercised against the corpus, then REVERTED (46/106 → the fitted
engine at HEAD stays authoritative at 106/106). What the attempt
established (all verified by hand-simulation against oracle logs):
- The real algorithm reproduces u09's initial batch EXACTLY under: staged
  TupleSets prepend (LIFO) consumed newest-first, right-inserts processed
  before left-inserts, memories append at tail, trg prepends per child.
- The port's terminal semantics are the truth for the requeue class:
  RuleExecutor.tupleList holds only QUEUED activations; fired tuples leave
  the list (getNextTuple = removeFirst + setQueued(false)); a terminal
  UPDATE is a no-op for queued tuples and re-APPENDS unqueued (fired) ones
  ("effectively recreated"); no-loop compares the propagation origin's
  terminal; the salience queue only exists for dynamic salience.
- THE DISCRIMINATING PAIR for the remaining unknown: j01 (2-pattern
  indexed join, fires in left-FIFO x right-ascending order) vs fz_42_9462
  (2-pattern indexed self-join, initial firing order effectively
  left-LIFO). No single FIFO/LIFO staging convention reproduces both under
  the ported doNode; the difference likely lives in the eager-evaluation
  flush boundaries (9462's rule is no-loop/eager, j01's is not) and/or the
  indexed-join child-sync walk (doRightUpdatesProcessChildren).
- Next session: resume the port on a branch; instrument BOTH engines with
  SEINE_HANDLES over j01/u09/9462/pr08/pr04 as the calibration set; read
  PhreakJoinNode.doRightUpdatesProcessChildren + TupleIndexHashTable
  iteration order; only swap the engine when the calibration set is green,
  then run the corpus + full fuzz.
Sources for READING live under the scratchpad (re-fetch:
`mvn dependency:sources -DincludeArtifactIds=drools-core` and unzip; do
NOT copy code into the port — behavior only, validated via oracle).

## Phase 2 (pre-work: goldens captured, engine not yet extended)

### D-011: Join + mutation semantics observed via probes j01–j05 (oracle-only,
files in probes_pending/ — move into scenarios/ once the engine supports them)
- j01/j02: join activation order = leftmost pattern's fact handle asc, then
  right pattern's handle asc (nested-loop order, left-major). Match object
  list is in pattern declaration order [P, A].
- j03: **afterMatchFired renders facts POST-RHS**: `bump`'s own match shows
  `done: true` (the value its RHS just wrote). Engine currently renders
  matches pre-RHS; identical for Phase 1 (no mutation), but Phase 2 MUST
  switch to render-after-RHS. Also: update() re-evaluates and fires
  newly-matching rules ("see" fired after).
- j04: no-loop suppresses self-reactivation from the rule's own update();
  fires exactly once.
- j05: delete() cancels not-yet-fired activations (P(2)'s "see" activation
  never fired). Deleted facts can still be rendered in the firing log entry
  of the deleting rule (Java object outlives retraction; our arena keeps
  values under a dead alive-flag, so same capability).

### D-013: Phase 2 semantics FULLY PINNED via probes j01–j22 (oracle-verified)
**Join activation order (j01, j02, j08, j09, j17):** for patterns p0..pk-1,
enumerate left-major with a twist: prefix list for p1 = p0's facts ascending
(alpha→first-join is NOT reversed); before joining each pattern pi with i≥2,
REVERSE the accumulated prefix list (PHREAK prepends tuples into the next
join's staged list); right-side facts always iterate in ascending handle
order. Firing order within a rule = final list order. Verified exactly on
2-, 3- (j08: P2Q2R2 first), and 4-pattern (j17, all 16 tuples) joins.
Self-joins include same-fact-in-multiple-positions tuples (j09: (P1,P1)).
Match rendering lists facts in pattern declaration order, values POST-RHS.

**Property reactivity (ON by default; j06, j07, j12, j13, j14):**
- Pattern listen-mask = fields referenced in its constraints, INCLUDING
  field bindings (j14). Empty pattern `P()` listens to NOTHING (j13: no
  refire ever).
- update() modification mask = union of fields written by setters on that
  fact in the RHS before the update call; **no setters ⇒ ALL-fields mask**
  (j21: bare update() self-loops infinitely — fire-limit parity required).
- On update: every activation (fired or pending) whose tuple contains the
  fact at a position whose listen-mask overlaps the modification mask is
  cancelled & re-created if still matching — fired ones fire AGAIN (j12),
  non-overlapping ones do NOT refire (j06/j07: mask {t} vs listen {n}).
- Re-created activations occupy their natural (handle-order) position in
  the rule's candidate order, not last (j18: see fired 1, 20, 3).
- Refires preempt by the normal agenda key immediately (j16).
- no-loop: suppresses ONLY the same rule-instance's re-creation caused by
  its own update (j04); other rules and other tuples unaffected.

**Mutation misc:** modify($p){ setX(..), setY(..) } ≡ setters+update with
the block's mask (j10). delete() cancels pending activations (j05, j11);
deleted facts still render in the deleting rule's own firing entry (arena
keeps values under a dead flag). j22: left-side updates re-join and refire
with re-evaluated bindings.

**Termination discipline for the Phase-2 generator:** update rules must be
guard-monotone (pattern requires `g == false`, RHS sets g=true before
update; bool setters only ever write true), inserts keep the type-index
DAG rule (target index > max pattern index). Bare update() (all-fields
mask) is NEVER generated — it non-terminates (j21).

### D-014: Incremental join-network semantics PINNED (probes u01–u10 +
fuzz counterexamples fz_7_58/87/145/159, all now regressions)
The Phase-2 fuzzer found 4 divergences in its first 200 cases; resolving
them pinned the full PHREAK staging model. The engine now maintains a real
per-rule join network:
- **Eager alpha, lazy beta:** alpha tests are evaluated at insert/update
  time (a fact that starts alpha-passing only after a later update takes
  that LATER queue position — fz_7_58). Beta (join) processing is deferred
  per rule until the agenda next considers it, so deltas from several
  firings can merge into ONE batch (fz_7_87: two inserts from one RHS).
- **Segment linking (fz_7_145):** while any pattern position has zero
  alpha-active facts the rule is unlinked — staged events accumulate
  (pruning/cancellation still applies) and are processed as one batch when
  every position has data. This is why "initial facts + later inserts" can
  be one batch for a rule whose first pattern started empty.
- **Batch processing per join** (u05–u10): staged left tuples first, each
  against the FULL right memory; then staged right facts against PRE-batch
  lefts; update-driven new pairs before both, in update-event order (u07).
  Emissions REVERSE when propagated to the next join (linked-list prepend)
  and append unreversed at the terminal. Memory orders: alpha and prefix
  memories BLOCK-PREPEND new batches (FIFO within batch; u09 pinned
  [new..., old...] right iteration, fz_7_159 pinned batch-2-before-batch-1
  prefix iteration); the terminal match list keeps kept entries in place
  and appends emissions (u01–u04: still-matching updates keep position).
- Deactivate→reactivate cycles lose list position (re-derived tuples).
- Curated corpus after this work: 55/55 PASS (`make diff`).
- NOT pinned (documented leniencies): mixed insert+update emission
  interleaving within one batch beyond u07's coverage; multi-update single
  RHS refire ordering (generator emits ≤1 update per RHS); alpha-memory
  iteration order after unlink/relink cycles; setters without a following
  update() (Drools leaves stale matches; generator always pairs them).

### D-015: Second fuzz wave — full PHREAK agenda/staging model (probes u11,
### regressions fz_42_*, 17 resolved + 3 open xfails)
Phase-2 fuzz (seed 42) found 20 divergences by case ~4400; resolving 17
pinned the deepest layer of PHREAK semantics:
- **Eager vs lazy rule evaluation:** no-loop rules evaluate their staged
  batch at EVERY flush window (their activations must be known); plain
  rules evaluate via the agenda peek — walk priority order, merging dirty
  networks, stopping after the first rule other than the one that just
  fired that has an unfired match. Rules beyond keep accumulating batches
  (fz_42_4138 vs fz_42_4141 — same shape, differ only in no-loop).
- **Hot updates move facts to the FRONT of their alpha memories**
  (fz_42_388/1057), while pending activations keep agenda position.
- **Fired activations re-created by an update lose their agenda position**:
  they requeue during the update phase (before insert-derived appends),
  ordered per hot event, hot positions ascending, terminal-join left-memory
  order within, hot-moved rights first (fz_42_2804/2055/1057).
- Left-update child iteration follows tuple CREATION order, not memory
  order (u11, fz_42_1176): creation seqs tracked per prefix/match entry.
- Emission phases per join: LI (staged lefts x full rights), RI (staged
  rights x [hot lefts creation-order, cold lefts memory-order]), LU (hot
  lefts x full rights, missing only), RU (hot rights, missing only).
- Corpus: 72/72 green (`make diff`), including 17 fz_42_* regressions.

### D-016: OPEN xfails (xfail/, excluded from make diff) — updated
- fz_42_3433 RESOLVED: alpha-memory move-to-front on update is NOT gated by
  listen masks (any update repositions the fact in every alpha memory it
  occupies; property reactivity gates only tuple re-evaluation). Now a
  regression + engine behavior.
- fz_42_3408, fz_42_4373 remain OPEN: both need >2-pattern rules with long
  multi-update histories; the residual gap appears to be hot-left iteration
  order divergence between indexed and unindexed joins after accumulated
  moves (u11: hot-first for a join whose key changed; 3408's unconstrained
  join at the same shape iterates cold-first). u12/u13 (clean single-update
  probes of the same shapes) PASS — only deep histories diverge. Next
  session: build a u14 probe = u13 + a SECOND update event, compare hot
  iteration; suspect per-event compounding of alpha/prefix moves.

### D-017: Subset wall — mutation programs are capped at 2-pattern rules
Because of D-016, the PROVEN subset excludes programs that combine
update/modify with rules of 3+ patterns. The generator enforces it
(`allow_mutation` programs cap every rule at <=2 patterns; 3-pattern rules
appear only in insert-only programs). 1-2 pattern mutation semantics and
3-pattern static semantics are each fully pinned; several 3-pattern+update
scenarios pass anyway and remain as extra regressions beyond the promise
(fz_42_1176/2537/4138/3433, u11-u13).

---

**HANDOFF @ checkpoint 2** — Phase 0 COMPLETE. Proven: full pipeline
(scenario JSON → DRL parse → columnar WM → match/fire → canonical JSON →
comparator) matches real Drools 9.44.0.Final byte-for-byte semantically on
p0_trivial_adult; `make diff` green, `make test` green. Next: Phase 1 —
(1) probe conflict resolution: multi-rule same-fact tie-break, salience
order, interleaved insert-during-fire ordering; (2) curated single-pattern
scenarios (all operators × all field types, bindings, no-loop); (3) seeded
property generator ≥10k cases. Open divergences: none. Open risks: agenda
policy beyond single-rule case is provisional (D-007).

**HANDOFF @ checkpoint 1** — Phase 0 in progress. Proven: Java oracle
(oracle/, Drools 9.44.0.Final pinned) runs scenario JSON → canonical NDJSON,
verified on `scenarios/phase0/p0_trivial_adult.json`. Build:
`cd oracle && mvn -q -DskipTests package`; run:
`java -cp "oracle/target/classes:$(cat oracle/target/classpath.txt)" dev.seine.oracle.OracleRunner <scenario>...`.
Next: Rust workspace (engine + harness crates), walking-skeleton engine
(parse this one rule, columnar arena WM), comparator, `make diff` green on
p0_trivial_adult. No open divergences.

## Verification-stack pivot (2026-07-05)

### D-059: Tiered corpus anchored by Drools' own regression suite
Strategy restructure, zero engine changes (tests/docs/harness plumbing
only). Differential testing is now a layered stack; `make diff` reports
per tier, all through the same harness/oracle/comparator:
1. **baseline** (`scenarios/baseline/`) — scenarios ADAPTED from Drools
   9.44.0.Final's own regression tests (drools-test-coverage,
   Apache-2.0, attribution in NOTICE, per-scenario `provenance` keys).
   Third-party spec tests: an in-subset failure here is a faithfulness
   bug nobody on this project authored. 7 members at close, 7/7 green,
   0 divergences found. Failing members would quarantine to
   scenarios/baseline-quarantine/ (excluded like xfail/) pending triage.
2. **probes** — the D-0xx curated pins (probes/, phase0-2, demo).
3. **regressions** — graduated fuzz finds. The fuzzer's charter is now
   explicitly "explore beyond the baseline", not "be comprehensive".
- **FEATURES.md** is the coverage matrix over the full Drools 9.44
  feature surface (docs + module structure + test modules):
  IMPLEMENTED (with D-0xx pins) / ROADMAP (prioritized, with upstream
  acceptance tests) / CANT (specific architectural constraint) / WONT
  (exclusion-as-strength). Ten genuinely-ambiguous features are parked
  in §5 for an explicit ruling, not guessed.
- Deliverable-2 docs: docs/baseline-extraction.md (pipeline + yield),
  docs/roadmap-acceptance.md (ROADMAP tests = definition of done),
  docs/drools-test-skiplist.md (CANT/WONT/not-DRL-behavior tests =
  honest limitations), docs/drools-test-routing.tsv (903 upstream test
  methods routed with reasons).
- Pipeline (tools/): gen_bean_catalog.py (model beans -> catalog, 121
  beans, ctor delegation resolved), extract_baseline.py (Java test ->
  scenario JSON; token-based package/import/global removal; WM-inert
  RHS stripping only; inline scalar `declare` lifting; provenance +
  JUnit-expected fire counts), baseline_gate.py (4 stages: engine
  parse gate = SUBSET ARBITER; oracle run; FIRE-COUNT DRIFT CHECK =
  translation honesty guard; differential).
- Bring-up lessons the gate caught: single-line DRLs were emptied by
  line-based package stripping (2 degenerate "passes" + 3 drift cases,
  all before any scenario was committed); RHS reassembly once produced
  `thenmodify(...)` (then-splice bug) — the drift guard is what made
  these visible. Extraction v1 scanned 903 methods across 88
  inline-DRL classes: 71 candidates, 7 in-subset (the rest are routed
  feature-wall evidence feeding FEATURES.md), 0 faithfulness bugs.
- Yield expansion (extractor/harness only, cataloged in
  docs/baseline-extraction.md): epochs translation for FactHandle
  update/delete tests (~77 methods; needs a `bare-update` all-set-mask
  action op in BOTH runners first — the 2-arg session.update semantics
  per fz_42_3311), per-class helper inlining (~229), counted-loop
  unrolling (21), query-call translation (recursive scenarios need
  timeout-guarded oracle runs, D-055 hang hazard), external-.drl
  resource tests (ExecutionFlowControlTest, FirstOrderLogicTest).
- Drools sources for reading live at ~/drools-9.44-src (shallow clone,
  tag 9.44.0.Final of github.com/apache/incubator-kie-drools; re-fetch:
  `git clone --depth 1 --branch 9.44.0.Final <url>`). Behavior/tests
  only — no code copied into the engine (NOTICE provenance story).
- Gate at close: `make test` green; `make diff` = baseline 7/7,
  probes 332/332, regressions 201/201.

## Feature-matrix rulings (2026-07-05)

User rulings resolving the ten ambiguities parked in FEATURES.md §5
(one D-entry per ruling, D-060..D-069). Docs-only change: no engine,
harness, or scenario changes. Each §5 row moves into its resolved
bucket (§1–§4); acceptance rows added to docs/roadmap-acceptance.md
for the newly-ROADMAP features; skiplist notes updated.

### D-060: CEP pseudo-clock → WONT
Even the deterministic pseudo-clock (`@role(event)` + `advanceTime` +
windows + temporal operators) introduces a **second WM lifecycle**
(event expiration) beside the certified one. The "no temporal" boundary
stays clean: the entire CEP family is WONT, pseudo-clock included.
Revisitable only as its own dedicated phase if real demand appears —
not as an incremental carve-out.

### D-061: Bounded expression grammar → ROADMAP-P3 (constraint arithmetic only); general `eval` stays CANT
Constraint arithmetic (`age + 1 > $x`) lands as ROADMAP-P3 via the
D-043-style **closed grammar**: literals + bindings + `+ - *`, same
single evaluator, no interpreter. General `eval(...)` is confirmed CANT
with no subset-grammar carve-out — the interpreter boundary is the
product edge. `enabled`/`salience` expression forms, if ever extended,
follow the same closed grammar.

### D-062: Globals — sinks stripped (done), read-only scalar globals ROADMAP-P4, Java-object globals WONT
(a) Globals-as-RHS-sinks (`list.add(...)`) are already translated away
by the baseline extractor (D-059) with the firing log as the stronger
assertion — DONE, no engine surface. (b) Read-only **scalar** globals
usable in constraints: ROADMAP-P4 (a per-session constant environment;
deterministic, fits the closed constraint grammar). (c) Full
Java-object globals (mutable services/collections reachable from rules)
are WONT: side-channel state invisible to the differential harness.

### D-063: Null field values → ROADMAP-P2 (raised from P3); `!.` stays CANT
Raised to P2: real-world account/servicing data is null-dense, and the
why-engine over realistic data needs nulls sooner than P3 implies.
Arrow validity bitmaps make the encoding natural. The null-comparison
matrix is a **large probe surface — per-operator** (`==`/`!=`/
relationals/`matches`/`contains`/`in`/accumulate null handling), so it
is scoped as its own phase when it lands, with the D-0xx probe-ladder
treatment. Null-safe dereference `!.` remains CANT (object graphs,
FEATURES.md §3).

### D-064: Date → ROADMAP-P3; BigDecimal/BigInteger → ROADMAP-hard, NOT CANT
Date fields: ROADMAP-P3 via epoch-i64 encoding + date-literal parsing
(the clean columnar story; `DateComparisonTest` as acceptance).
BigDecimal/BigInteger: **reframed from the CANT lean.** Money in the
target domain (lending/servicing) is *bounded-precision decimal*, which
HAS a lossless columnar encoding — scaled fixed-point over i128, the
DECIMAL(p,s) approach databases use. It is not architecturally
forbidden; it is deferred-and-hard (huge Java coercion matrix to pin).
Bucketed ROADMAP-P4 (hard) with the encoding note. We do not stamp CANT
on the one type the financial-services target domain legally requires
for money.

### D-065: Declared-type inheritance (`declare X extends Y`) → CANT
Supertype matching breaks the **one-type-one-arena invariant**
everywhere it is load-bearing: alpha/beta indexes key on (type, field),
property-reactivity masks are per-type bit positions, and node-sharing
identity (D-029/D-033) assumes one arena per pattern type. A
pattern-on-supertype scanning the union of subtype arenas is an arena
redesign, not a feature. Stated as the blocking constraint in §3.

### D-066: Fact equality for TMS → value-equality over declared fields; TMS flagged PRODUCT-CRITICAL
Two rulings. (1) Mechanism: `insertLogical` justification sets use
**value-equality over declared fields** — cheap in columnar (column-wise
compare), no `@key` subsets, no Java equals/hashCode emulation.
Equality-assert *mode* as a session config stays WONT (config-matrix
argument, §4). (2) Priority reframe: TMS is **PRODUCT-CRITICAL, not a
side feature** — `insertLogical` + justification + cascading retract is
the substrate of the why/why-not derivation engine (facts that
auto-retract when support disappears ARE the "why does this still
hold / why did that clear" machinery). The ROADMAP row now carries the
thesis-load-bearing flag; priority stays P2 in sequence but it is the
anchor of that tier.

### D-067: Char fields / char literals → WONT (out of subset)
Niche type, odd DRL stringification of `'x'` literals, near-zero demand
in the target domain. Walled out of the subset; noted in docs. Revisit
only if a real corpus needs it — then decide 1-char-String vs i64
code-point encoding.

### D-068: Virtual date for `date-effective`/`date-expires` → WONT
A ruleset whose behavior depends on the calendar is exactly the
nondeterminism the temporal wall exists for — even with a fixed
"evaluation date" scenario field. The distinction is now explicit in
§4: dates as **fact data compared against** = ROADMAP (D-064); dates as
**engine-evaluated effective/expiry attributes** = WONT. Users model
dates as fact fields.

### D-069: Declarative agenda → WONT
Rules controlling other rules' matches couples agenda internals to user
rules — deterministic but exotic meta-control, small upstream surface
(m.i `DeclarativeAgendaTest`, 16 methods). Agenda-groups (already
ROADMAP-P3) cover the real use cases. `DeclarativeAgendaTest` moves
from "pending ruling" to a firm skiplist entry.

**HANDOFF** — §5 rulings recorded (D-060..D-069), FEATURES.md §5 emptied
into §1–§4. ROADMAP priority changes: nulls P3→P2 (D-063), TMS flagged
product-critical (D-066), BigDecimal added as ROADMAP-P4-hard with the
i128 scaled-fixed-point note (D-064), constraint arithmetic P3 (D-061),
scalar globals P4 (D-062), Date P3 (D-064). New CANT: declared-type
inheritance (D-065). New WONT: pseudo-clock CEP (D-060), Java-object
globals (D-062), char (D-067), virtual-date attributes (D-068),
declarative agenda (D-069). No engine changes; gate unchanged
(baseline 7/7, probes 332/332, regressions 201/201).

## Phase P1a — `or` CE + parenthesized CE groups (2026-07-05)

### D-070: `or` CE = parse-time subrule expansion (probe ladder or_a1..a43, or_b1..b5)
Oracle-verified on Drools 9.44.0.Final; 35 probes promoted to
scenarios/probes/pr_or_*. The whole feature is a PARSER rewrite: an
`or` rule expands to DNF at parse time, one ordinary engine rule
(SUBRULE) per branch, sharing name/attributes/RHS. Zero changes to the
evaluator, trie, agenda or query machinery beyond a no-loop scope fix.
- **Expansion:** nested `or` flattens (a13x); multiple or-groups cross
  left-major — earlier groups vary slowest: `(A or B) (C or D)` →
  AC, AD, BC, BD (a23). Grammar: infix `X or Y` / `X and Y` (and binds
  tighter), prefix `(or …)` / `(and …)` (a7/a14), parenthesized infix
  groups incl. single-pattern `(A())` (a35/a35b/a43). TOP-LEVEL
  juxtaposition is AND across whole or-expressions: `A() or B() C()`
  ≡ `(A or B) and C` (a4); bare juxtaposition INSIDE parens is a parse
  error in Drools and here (a42).
- **Agenda:** each subrule is a separate terminal in build order —
  decl_pos now counts TERMINALS (subrules/queries), so the order key
  (salience DESC, decl ASC, insertion ASC) makes branch-1 activations
  fire before branch-2 even when branch-2's are older (a2/a2b/a17);
  all subrules sit at the parent's slot for cross-rule order and
  preemption (a3/a3b/a16). Static salience applies to every branch
  (a18); dynamic salience evaluates over the FIRING branch's bindings
  (a19; bare `salience $v` form added — Drools-legal). Relative
  rule/query agenda order is preserved by expansion (positions inflate
  monotonically; ties impossible), so D-058 query items need no change.
- **Semantics:** matches render only the branch's own patterns (a1); a
  fact matching k branches fires k times (a5); not/exists/accumulate/
  ?query branches behave as leading-CE rules incl. InitialFact
  rendering (a15/a25/a33/a39x/a40); joins after an or-group evaluate
  per-subrule with the standard D-013 orders (a4bx/a22).
- **no-loop is per PARENT rule** (a20): an update from any branch's RHS
  suppresses re-activation of every sibling subrule (Drools compares
  the shared Rule object). Engine: CompiledRule.def.parent + the four
  origin checks compare parents.
- **Sharing:** subrules share alphas/trie exactly like plain rules —
  the fz_42_580 twin-share shape with either twin turned into an
  or-rule (dead extra branch) reproduces the original firing sequence
  byte-for-byte (a28/a29).
- **Declarations:** a var referenced downstream/RHS/salience must be
  bound in EVERY branch, else compile error (a12/a30b/a37 — engine
  errors likewise). FIELD bindings repeat freely across branches with
  per-branch values (a6/a22/or_b5, incl. different field types when
  unreferenced). FACT bindings: same name across branches legal iff
  same pattern TYPE (or_b1/or_b4 — usable in RHS delete); duplicate
  within a branch (or_b2) or cross-branch type conflict (or_a26/or_b3)
  = "Duplicate declaration" compile error, mirrored in the parser.
- **Fences kept honest:** `not (…)`/`exists (…)` CE groups stay a clean
  parse error until P1c (a41 pinned the Drools behavior: legal,
  InitialFact match). Prefix groups need ≥2 operands.
- Generator: ~18% of acc-free rules gain 1-2 copy-mutated branches
  (same binding names — every-branch-bound by construction; update
  GUARD never mutated, preserving termination), infix and prefix
  renderings; acc/collect rules stay single-branch (identical acc
  twins would fuzz the unprobed acc-sharing surface).
- Baseline: +1 (bl_cop_OrTest_testEmptyIdentifier, 7→8). OrTest
  routing: 4/14 extracted; the or-relevant remainder blocks on `||`
  inline groups (P1b ×3) and extractor yield items (external-WM
  epochs, facttype-api — D-059 catalog). Misc2Test or-scope methods
  (testDeclarationsScopeUsingOR*) are eval/null-walled (CANT/P2
  routing evidence).
- Corpus after P1a: probes 332→367 (35 pr_or_*), baseline 7→8,
  regressions 201→205 (D-071 finds).

### D-071: per-sink child-kind resolution — kept-kind inserts peer-copy
### as UPDATES (fz_42_890, first or-campaign find; pre-existing bug)
The or-grammar campaign's reshuffled draws exposed a LATENT forward-
engine bug (bisect: pre-P1a engine byte-identical on the repro — not
introduced by D-070). Minimized (fz_min_890, 3 rules / 2 facts): R5
(salience 7) and R0 (salience -8) share a 2-level trie prefix; R1's
bare update() re-touches the shared join's child while lazy R0's
terminal still holds the ORIGINAL child INSERT unconsumed.
- Drools: updateChildLeftTuple resolves the touched child against EACH
  SINK's own staged state — at R0's segment the pending INSERT keeps
  its kind (moves into the current batch); R5's already-consumed peer
  stages an UPDATE, whose not-node leftUpd propagates and REFIRES R5's
  fired activation. Oracle: R5, R1, R5, R0.
- Engine (before): child_upd resolved the kind against the FIRST
  sink's pending only and copied the RESOLVED batch to every peer; the
  kept-kind INSERT then hit peer_merge_left's materialized-tuple path
  (removeAdd, nothing staged — fz_123_8822) and R5 never refired.
- Fix: `Staged.peer_upd` side-channel (the norm_del precedent, mirror
  case): child_upd marks kept-kind entries; the first sink appends
  them as inserts unchanged; peer NODE copies stage them as UPDATES
  with the fz_999_3298 staged-clash skip. Terminal peers already
  modeled this via peer_live insert→update conversion — untouched.
- fz_123_8822 (true re-delivered inserts) and fz_999_3298 keep their
  pinned behavior: the marker rides ONLY on updateChildLeftTuple's
  kept-kind resolution.
- Graduated: fz_42_890 + fz_min_890, plus same-campaign finds
  fz_7_3315 (first or-bearing find) and fz_7_3462 — all pass with the
  fix.

### D-072: shared-LIA modify gate decides ONCE at the first-built child
### (fz_999_7082 — second latent find; pre-existing, bisect-verified)
Seed 999 of the or-campaign (1 divergence in 50k) minimized to a
no-or shape: a join rule (T1($b : f1) x T0()) and a collect rule
(T1($b : f1) + collect(T0…)) SHARING a LIA (same alpha, same
bound-set), a third rule updating the T1 via setF1+update. Probe
bisection (m7082_r3nobind/r3last/r3k1/r3cons all PASS; mg1u ruled out
update-vs-modify mask inference):
- **Pin:** for a shared LIA, the stage-vs-drop decision for a
  pattern-0 property MODIFY is made ONCE against the FIRST-BUILT trie
  child's effective left mask — a collect child contributes its D-040
  gate (constraint fields + collect beta refs; bare bindings do NOT
  count), a join child its full listen mask (bindings count) — and the
  decision applies to EVERY trie child of that LIA: join-first STAGES
  the modify for a gated collect sibling (m7082_vis_jf: both refire);
  collect-first DROPS it for the join sibling (m7082_vis_cf2: neither
  refires). k=1 rules on the LIA gate independently on the canonical
  listen mask (m7082_r3k1). ALL-SET (bare update) always stages.
- The engine previously gated per-child (only collect children) —
  wrong in both directions. Fix: compute child_stage once from
  children[0]'s gate-or-listen in on_update; the per-child gate drop
  is deleted. mg1..mg8 unchanged (single-child LIAs degenerate to the
  old rule).
- Probes promoted: pr_lia_gate_jf, pr_lia_gate_cf, pr_mg1u_update
  (setter+update mask-inference control). Regressions: fz_999_7082 +
  fz_min_7082. Corpus: probes 367→370, regressions 205→207.
- P1a fuzz gate (WITNESSED): full 5x10k rerun on the final engine
  (D-070 or-grammar in the generator, D-071 + D-072 fixes in) — seeds
  42/7/123/777/999, **50,000 cases, ZERO divergences**. Gate at close:
  make test green; make diff = baseline 8/8, probes 370/370,
  regressions 207/207.


## Phase P1b — inline &&/||/!() constraint groups (2026-07-05)

### D-073: inline boolean constraint groups (probe ladder ib1..ib31)
Oracle-verified; 28 probes promoted (pr_ib*). Grammar: `a > 5 && a < 10`,
`a == 1 || a == 2`, `!(…)`, nested parens, abbreviated restrictions
(`a > 5 && < 10`, `b > $x || == 1`), bind-with-restriction
(`$v : b > 0`, `$name : name in (…)` — InTest#testInOperator), keyword
leaves (`matches`/`contains`/`in`/`not in` inside groups, ib13).
`&&` binds tighter than `||` (ib5). Two-tier compile model:
- **Top-level `&&` SPLITS into comma-equivalent constraints** at parse
  time — the conjuncts keep full alpha identity: they join D-029
  eq-hash groups (ib24 ≡ ib24b: `a == 2.5 && a > -1000` truncates in a
  hash group exactly like the comma form) and share trie prefixes
  (ib15/ib28 ≡ comma twins on the fz_42_580 shape, abbreviated form
  included). Leaves demote to the existing Constraint variants.
- **`||`/`!()` tops compile to ONE composite Group** with `in`-like
  semantics: leaf `==` promotes to double, never truncates (ib23 —
  `a == 2.5 || a == 99` misses a=2), never joins an eq-hash group
  (ib21) and does NOT count toward the >=3 hash threshold (ib22: two
  plain eq siblings + composite stay unhashed). Groups are alpha-CHAIN
  members for prefix scoping (like InList) with a structural identity
  key (referenced var names identity-significant, D-037).
- Cross-pattern refs inside groups make the pattern beta and evaluate
  at join time (ib14/ib30); same-pattern refs mirror top-level Cmp
  resolution; groups referencing bindings are rejected on pattern 0.
  Groups work inside not/exists patterns (ib26/ib27) and or-branches;
  listen masks include every leaf field (ib16). Bindings INSIDE group
  branches stay out of subset (fence); query bodies keep the plain
  grammar (fence: query-network composite sharing unprobed).
- Non-relational abbreviated forms after && / || (`matches`-without-
  field etc.) stay fenced except the probed bind-with-keyword forms.
- left_update_optimization counts cross-var groups as non-equality
  beta constraints (conservative isLeftUpdateOptimizationAllowed).
- Baseline +3 (8→11): InTest#testInOperator, InTest#testNegatedIn
  (named P1 acceptance), OrTest#testConstraintConnectorOr.
  OrTest#testRestrictionsWithOr / #testOrWithReturnValueRestriction
  stay honestly out (constraint arithmetic / eval — D-061 CANT until
  the P3 closed grammar). Misc2Test#testTypeCheckInOr = dialect wall;
  #testVariableMatchesField = matches-vs-binding, out of subset.
- Generator: ~12% of non-collect patterns gain a group constraint
  (disjunctions, negations, abbreviated ranges); corpus probes
  370→398, baseline 11.

### D-074: `in`/`not in` compile-time normalization — alpha-chain
### sharing identity (fz_42_6342 → probes w6342_*/af_p*/q1..q6)
The P1b campaign's first find minimized to or-branch twins differing
only in `not in ("zz")` vs `!= "zz"` — the oracle fired the second
branch NEWEST-first while the engine fired both oldest-first. Probe
bisection (plain twins reproduce it; twins with genuinely-different
constraints do NOT; single rule normal; k=1 sharers normal):
- **Pin:** Drools compiles `not in (a, b, …)` to an AND of `!=`
  constraints that SPLITS like top-level `&&` — each conjunct is an
  ordinary alpha-chain node sharing with a written `!=` (q2, q4);
  `in (a, b, …)` compiles to an OR composite that shares with the
  equivalent written `||`-of-`==` group (q3, q5b) and — even
  single-element — never joins or counts toward D-029 eq-hash groups
  (q6, refining D-030: the no-hash pin was right, the no-SHARING
  assumption was engine-only). With the identity normalized, the
  observed order flip is nothing new: the twins FULLY share their
  first-pattern LIA and the D-036/D-037 first-sink-preserved /
  later-sink-flipped batch propagation applies.
- Engine: compile_rule now lowers negated InList to a sequence of
  plain Ne cmps and non-negated InList to a Test::Group
  (Or-of-Eq) with the same identity key a written `\|\|` group gets;
  Test::InList is deleted. Query bodies keep their own InList compile
  (no groups in query grammar — D-073 fence).
- Graduated: fz_42_6342 + fz_min_6342.

### D-075: three latent pre-P1a order bugs quarantined (xfail/), found
### by the P1b campaign, bisect-verified independent of P1a/P1b
The widened grammar keeps reaching rare staging shapes. Seeds 42/7/999
produced four more finds; ALL minimize to shapes with NO P1b feature
(or none load-bearing) and reproduce byte-identically on the pre-P1a
engine (522d2cb). Three distinct mechanism families, each needing its
own probe ladder:
1. **Multi-window join activation order** (fz_7_455 → fz_min_455):
   2 rules, modify + epoch; the engine and oracle pick different join
   pairs to fire first when left/right stagings span an external-insert
   window and a rule-firing window.
2. **Collect + delete + setter-without-update** (fz_42_4816 →
   fz_min_4816): collect over a type a lower-salience rule deletes,
   plus a bare setter (no update()) — firing order after the delete
   diverges.
3. **Query row order / dynamic salience** (fz_999_3959 → fz_min_3959:
   ?query pull row order across epochs; fz_42_6812 → fz_min_6812:
   dynamic-salience + no-loop pair order).
All four full scenarios + minimized repros sit in scenarios/xfail/
(documented-open, D-042 mechanism — excluded from the gate, fuzz
re-flag suppressed by name). They are the top of the next
engine-hardening phase's worklist, BEFORE P1c extends the existential
machinery they touch.

**P1b gate (WITNESSED):** 5x10k rerun on the final engine (D-073
groups in grammar, D-074 normalization in) — seeds 42/7/123/777/999,
**50,000 cases, 0 divergences**, 4 xfail hits = exactly the D-075
quarantined names (no new members of those families). Gate at close:
make test green; make diff = baseline 11/11, probes 398/398,
regressions 209/209.

**HANDOFF @ P1b close** — P1a (D-070..D-072, commit 578cbdc) and P1b
(D-073..D-075) landed. Baseline yield so far: 7→11 (OrTest
testEmptyIdentifier + testConstraintConnectorOr, InTest testInOperator
+ testNegatedIn). P1c (nested not/exists CE groups) NOT started: the
D-075 latent order bugs touch the existential machinery P1c would
extend — harden first (fz_min_455 / fz_min_4816 / fz_min_6812 /
fz_min_3959 in xfail/ are the worklist), then lift the D-031
"bare not/exists only" fence.


## Truth maintenance — TMS phase (2026-07-05)

### D-076: insertLogical / justification / cascading retract (probe
### ladder tms_e*/t*/w*/u* + TmsDump reflection; Bryan-supervised)
Oracle = Drools 9.44.0.Final WITH the drools-tms module (required since
Drools 8: insertLogical without it is a build error; classpath addition
proven corpus-inert, all tiers green). All pins oracle-verified; the
stated/justified internals fell to TmsDump reflection (getEqualityKey /
getBeliefSet) after black-box witnesses contradicted every model.

**THE DESIGN CONSTRAINT (product-critical, Bryan's brief): the
justification graph is QUERYABLE, not internal bookkeeping.** The
engine keeps the TMS as first-class state (equality keys -> justified
handle + belief set of (rule, tuple, seq) supports + stated siblings)
and derives retraction FROM the graph. Public surface:
`Engine::justifications() -> Vec<JustificationView>` and
`Engine::why(fact)` — per justified fact: rendering, ordered supports
(rule name + matched tuple + seq), stated siblings. This IS the
why-engine's substrate: "what justifies this fact" is a lookup, "what
would have to change for it to retract" is the support list.
Integration test tms_queryable.rs pins the surface.

- **Equality (D-066 mechanism):** value-equality over ALL declared
  fields. Oracle side: declare blocks now emit `@key` on every field
  (without @key, declared types are identity — tms_e1: no sharing;
  with it, equal logical inserts merge — tms_e2). @key-all proven
  corpus-inert (full tiers green before any TMS scenario existed).
  f64 keys use Java Double.equals bit semantics: NaN==NaN,
  +0.0 != -0.0 (tms_u6; engine keys via f64::to_bits). Partial @key
  stays out of subset (tms_e11 evidence: key-subset equality, first
  object's non-key fields win).
- **Lifecycle:** justified handle per key; deps merge across rules and
  tuples (e2/e3/e10, dump-d beliefs=2); same-activation deps are
  idempotent (dump-c); last-dep removal auto-retracts with cascades
  (e7). Flagship not-CE shape works (e4).
- **Timing — TWO paths (t1/t5/t8 vs t11/t12/t15/min_1310; dump5's
  event sequences settled it):** dep removal rides Drools'
  cancelActivation -> removeLogicalDependencies at TERMINAL-tuple
  deletion. (1) EAGER: a DELETE or alpha-breaking UPDATE of a fact IN
  the justifying tuple cancels within the breaking WM action
  (ModifyPreviousTuples analog) — before any later pop, regardless of
  the justifier's salience (t5: sal-2 justifier's fact retracted
  before a sal-5 witness). EXCEPT self-inflicted breaks: a justifier
  breaking its OWN tuple mid-firing lands lazy (fz_42_2442 — a
  higher-salience rule fires on the fact first). (2) LAZY: network-
  mediated breaks (not/exists blocker transitions — facts NOT in the
  tuple) process at the justifier's agenda-item evaluation,
  salience-ordered: higher-salience rules FIRE on the transient fact
  first (t11: a sal-100 witness fires on a fact that then retracts;
  t12; min_1310's accumulate rule fired on a transient logical fact).
  Drools checks salience preemption BEFORE re-evaluating a fired
  rule's network, so the engine keeps its certified post-firing force
  evaluation (window claiming, D-037) and DEFERS only the TMS
  side-effect. Drain points, all probe-pinned: (a) the post-firing
  continuation drains unless a STRICTLY-higher-salience item waits —
  equal salience/earlier decl does NOT preempt it (min608 vs t11);
  (b) an EAGER (no-loop/dyn-salience) justifier's entry drains at the
  flush IFF the breaking action property-HIT the tuple's LEFT side —
  right-side-only breaks wait (the tms_t20 2x2 event dumps: only
  binding+setter kills the transient before an equal-decl witness);
  (c) otherwise the item's next pop. A bare no-loop own-update with
  NOTHING breaking removes NO deps (pr_tms_noloop_bare_upd: the
  logical fact survives — j04's skip is not a cancellation).
- **Refire-supersede (fz_7777_112/74, dump-c):** when an activation
  REFIRES, deps from its previous firing not re-established by the new
  firing are removed at end-of-firing (Drools
  cancelRemainingPreviousLogicalDependencies): update-keeps-match with
  same value = stable fact, no blip; changed bindings retract the old
  value's fact after the refire. Engine: prologue snapshot +
  epilogue sweep in execute_rhs.
- **Self-defeat parks, left-side events revive (t10/t11/t15):**
  `A() not LK() -> insertLogical(LK)` fires ONCE, fact absent, NO
  refire — the retraction's unblock re-add is suppressed (Drools
  leaks the dead blocker); a property-relevant UPDATE of a tuple fact
  re-propagates and REFIRES (t15: two firings), unrelated events do
  not. Engine: one-shot suppress_once consumed at push_activation.
- **Stated/justified interplay (w1..w5, dumps 1-3; Bryan: model
  faithfully):** stated inserts are plain identity-mode inserts —
  stated equals COEXIST with the justified fact (w1/w5). insertLogical
  onto a stated-only key inserts nothing but records a dep that
  evaporates with the stated fact (dump-b, e6). THE QUIRK: delete() on
  a key with a live justified handle kills the JUSTIFIED fact +
  belief set whichever handle was named (dumps 1/2a); once a key has
  hosted a justified handle, deleting a stated sibling is a SILENT
  NO-OP (dump3 — the fact is effectively undeletable). Modeled
  exactly; bug-shaped but deterministic and pinned.
- **Walls (Bryan: compile-time):** (1) setters/update/modify on a
  logically-inserted TYPE — Drools runtime-errors with murky triggers
  (tms_u1 "cannot modify", tms_u4 "mixed stated and justified" even
  with no live justified handle); subset walls it at compile time,
  external updates at call time. (2) insertLogical from
  accumulate/collect/?query rules (justifying-tuple revalidation
  cannot re-run those conditions). (3) ?query CEs + insertLogical in
  one unit (D-057 extension: TMS retracts are WM deletes the drain
  windows would see). (4) rules-before-facts required once a unit has
  insertLogical.
- Acceptance-test routing (honest): ErrorOnInsertLogicalTest =
  function-blocks/exceptions + external-wm-api; Misc2Test
  testPhreakTMS = arithmetic + wm-introspection; testQueryCorruption =
  declare-annotations; drools-tms module tests = internals
  (skiplist). ZERO baseline yield — certification weight = the 20
  promoted pr_tms_* probes + differential fuzz, as with the Q phases.

### D-077: stated/justified key lifecycle — the full quirk model
### (fz_42_1395/2442/2659, dumps 6-8, NamedEntryPoint/SimpleBeliefSystem
### sources as behavior reference)
The first TMS campaign's finds completed the stated/justified pins:
- **Key death (fz_42_1395):** when the justified handle dies and no
  stated siblings remain, the KEY VANISHES — a later stated insert of
  the same value starts a FRESH key (deletable normally). The dump3
  undeletable-sibling quirk applies only to siblings that COEXISTED
  with a justified handle.
- **Pending logical beliefs UNSTAGE (fz_42_2659, dump7/dump8):** an
  insertLogical onto a stated-only key records a dep + PENDING values
  (no WM insert — dump-b). Deleting the stated handle from a RULE
  consequence UNSTAGES the belief: the justified fact MATERIALIZES
  live (rules fire on it; it dies only when its deps do). An EXTERNAL
  session.delete nets materialize-then-die inside the call (dump8's
  +WM/-WM pair) — nothing observable survives, which the engine
  models as key death (tms_e6 differential-green either way).
- **Collect removal is Collection.remove(Object) (fz_42_2019,
  D-078 fallout of @key-all):** value-equality removes the FIRST
  equal element of a collect list, not the identical instance — the
  engine's collect reverse now picks the list victim by value.
Latent find quarantined per the D-075 pattern: fz_42_3924 (+ min) —
or-twin not-nodes with an update-away-and-back epoch, bisect-proven
pre-existing (pre-TMS engine byte-identical; fails under the PRE-@key
oracle too) — scenarios/xfail/.

### D-078: TMS generator grammar + certification gate
Generator: ~30% of scenarios designate the LAST type as the LOGICAL
type; CE-only matches of the logical type may self-justify (the t10
family); setters/updates never touch it (wall-safe by construction);
external updates reroute to deletes; ?query rules and TMS never mix.
Smoke fuzz immediately caught the refire-supersede gap (fz_7777_112/74,
minimized + graduated). After the first full campaign (57 finds), the
envelope was FENCED per Bryan's ruling (D-080): the logical type is
PURE — only insertLogical produces it (no stated inserts by rules,
initial facts, or epochs; no rule deletes of it; external deletes
remain), and justifiers carry no mutation actions in the same RHS.

### D-080: TMS certified envelope — compound transient-visibility
### micro-timing documented-open (Bryan's fence+quarantine ruling)
Three timing layers were pinned and fixed from the first campaign
(D-076's drain points; the unstage materialization D-077). The
residual 36 finds (~0.12% of draws, 32 order-only) are COMPOUND
stacks of transient-visibility micro-timing — which third-party rules
glimpse a logical fact between its insertion and its lazily-processed
retraction — under (a) justifiers that mutate/delete in the same RHS
as insertLogical (26) and (b) stated/justified key mixing under rule
deletes, where Drools' immediateDelete vs staged cancellation paths
diverge (10, min4048 family). Every single-mechanism minimization
PASSES (promoted as probes); only the compounds diverge, and each
peel exposed another RuleExecutor internal. Per the D-042/D-075
pattern: the 36 sit in scenarios/xfail/ as witnesses, the generator
no longer draws the two shapes (D-078), and the SEMANTICS of mixing
remain certified by the hand-probe matrix (w-series, t20 2x2, dumps).
**Bonus finding: Drools itself is NONDETERMINISTIC on three of the
shapes** (fz_42_84/581/2657 — identity-hash-order-dependent TMS
cascade churn: the same scenario terminates or hits the fire limit
across JVM launches). Those are un-certifiable by any differential
harness and sit in xfail as nondeterminism witnesses — independent
evidence that the fence line is drawn where Drools' own behavior
stops being a function of the program.

**Recursion accounting (Bryan's question):** the cascade is
call-recursive (retract -> on_delete -> eager-break -> retract), but
(a) it TERMINATES structurally — each level kills >=1 live justified
fact, nothing resurrects mid-cascade (no rule fires during
propagation), keys merge idempotently so cycles can't sustain; and
(b) stack depth is SUBSET-BOUNDED — a chain link needs derived values
and the subset has no arithmetic (D-061), so RHS args are copies
(same key, merge) or literals (finite): depth <= #rules x
literal-combos. Locked by the depth-12 chain test in
tms_queryable.rs. P3 constraint arithmetic would lift the bound —
its roadmap row now carries a "cascade goes iterative first" prereq.

(Pre-commit catches: the defer flag leaked out of evaluations into
the drain loops TWICE — first past eager evaluations (seed 42 wedged
3h), then past the UNLINKED-RULE early return (seed 123 wedged 6h;
gdb backtrace off the live process pinned the exact loop). Both are
non-firing infinite spins the fire-limit cannot catch. Fix is now
STRUCTURAL: evaluate_rule is a wrapper that scopes the flag around
evaluate_rule_inner — no per-exit hygiene to forget. Lesson recorded:
slow gate = `ps` + `gdb -p <pid> -batch -ex bt` FIRST, not waiting.)

Second-campaign refinements (three more pins, then the fence closed):
- **Eager unmatch is k=1-scoped** (pr_tms_k2lazy/min3783): the
  tuple-fact-delete teardown reaches the terminal directly only for
  single-positive-pattern justifiers; k>=2 tuples die via staged
  propagation = the LAZY path — a witness fires on the transient
  between a join-justifier's tuple-fact delete and its item's
  evaluation. Every t1/t5/t8 eager pin used k=1.
- **Flush drains are OWN-ORIGIN only** (min3783 vs tms_t20_b_s): the
  eager-flush dep-removal fires for the justifier's own left-side
  action; foreign-origin left hits wait for the pop. TMS terminal-del
  side-effects now defer out of BOTH the post-firing force evaluation
  and eager-flush evaluations.
- **The self-defeat park covers the dead blocker's WHOLE blocked
  list** (pr_tms_t21: sibling tuples blocked by the same fact stay
  parked; the rule fires once, not per-tuple).
- **CE-only self-justifiers are fenced out of the generator**
  (fz_42_946 family): with >=2 deps on one key (or-twin branches,
  multi-rule justification), the self-defeat cycle is a GENUINE
  DROOLS RUNAWAY (fire-limit, 17 of the second campaign's finds) —
  the engine terminates where Drools does not. Single-tuple semantics
  stay certified via pr_tms_t10/t11/t15/t21. Remaining second-campaign
  witnesses quarantined; fz_999_9976 bisect-proven pre-existing
  (collect join-order latent, D-075 family).

### D-079: CEP-as-TMS investigation queued (Bryan's post-TMS note)
Bryan: the D-060 WONT on CEP (incl. the deterministic pseudo-clock)
may soften now that TMS is landed — IF the non-wallclock CEP subset is
a SPECIAL CASE of TMS: event `@expires`/window lifetimes as justified
facts whose support is a logical-clock window fact, expiration =
justification loss = the certified D-076 cascade. Queued as a
ROADMAP-P3 INVESTIGATION row (FEATURES §2): the deliverable is a
mapping memo (probe-first, PseudoClockEventsTest as reference), not an
implementation. D-060's "second WM lifecycle" objection stands unless
the reduction is clean; if it IS clean, the objection dissolves by
construction (one lifecycle: TMS).

**TMS gate (WITNESSED, final binary):** tiers baseline 11/11, probes
431/431, regressions 252/252; fuzz seeds 42/7/123/777/999 x 10,000 =
**50,000 cases, ZERO divergences**, xfail hits = quarantined names
only. Corpus at close: 431 probes (33 pr_tms_* + timing matrix),
252 regressions (incl. fz_999_3020 dyn-salience flush pin), 92 xfail
witnesses (D-080 envelope + Drools-nondeterminism + pre-existing
latents), baseline 11.

**HANDOFF @ TMS close** — insertLogical/justification/cascade landed
(D-076..D-080) with the QUERYABLE justification graph
(Engine::justifications()/why()) as the why-engine substrate. Next
per Bryan: D-075/D-080 hardening worklist (two pins already queued:
ex1a out-and-back right re-entry, hb4 exists multi-right left order;
then collect/dyn-salience/query-row families), THEN P1c nested
existential CE groups on the hardened base.


## Hardening wave 1 — D-075/D-080 latent order-bug backlog (2026-07-06)

### D-081: alpha out-and-back re-entry + slot-memory fire-boundary
### (fz_42_3924 + fz_min_1144 families graduated from xfail)
Bryan's directive: harden the quarantined latents before P1c. Two
mechanisms pinned and fixed, nine probes promoted (pr_hw_*):
1. **Existential right re-entry (pr_hw_reentry_not/ortwin, hw_ex1a):**
   a fact leaving and re-entering a not/exists alpha within one staged
   batch (update-out then update-back) leaves BOTH a del and an ins
   staged (del-then-ins does not fold; ins-then-del does). The blocker
   re-search treated any staged-del right as ineligible, so the engine
   unblocked and fired where Drools re-blocks against the re-added
   (fresh, unstaged) RightTuple and nets ZERO firings. Fix: a
   staged-del right that is ALSO staged-ins is eligible — both-present
   uniquely marks re-entry. Clears fz_42_3924 + fz_min_3924b (the
   or-twin variant needs nothing extra: subrule sharing was innocent).
2. **Slot memory is scoped to the fire boundary (pr_hw_slot_*):**
   D-047's cancelled-slot restore (fz_7_5801) applies to out-and-back
   WITHIN one fire window (cancel + re-add in one epoch's actions).
   Re-entries after fireAllRules returns place at the HEAD like fresh
   adds — the engine's slots persisted forever, reconstructing stale
   orders (fz_min_1144: exists left-batch fired newest-first instead
   of arrival order; the earlier "exists iteration order" theory was
   wrong — the STAGING order was the bug). Fix: cancelled slots clear
   when fire_all returns. fz_7_5801 + min preserved exactly.
Also pinned as probes: exists mass-support = left-ARRIVAL order while
not mass-unblock stays reverse-arrival (pr_hw_exists_support /
pr_hw_not_unblock, refining ne_n4's asymmetry); multi-window join
activation order falls out of the certified phase machinery
(pr_hw_joinwin*, 3 probes — family A's core was never broken).
**Wave-1 gate (WITNESSED):** tiers 11/431/252 green pre-gate; fuzz
5x10k = 50,000 cases, ZERO divergences (xfail hits = quarantined
names only). Graduated: fz_42_3924(+min), fz_42_1144(+min+plain) —
xfail worklist 5 smaller.

OPEN, next in queue: fz_999_5014's residual = the JOIN-edition
re-entry (rightDel kills the re-add's fresh children — same
single-FactId identity gap at join nodes); fz_min_455 (modify-layer
join order); collect pair (fz_min_4816/xf_min_9976); dyn-salience
pair order (fz_min_6812); query rows (fz_min_3959).


### D-082: right-insert PROVENANCE is semantic — model-check survivor,
### partial landing, and the D-083 discriminator plan (WIP CHECKPOINT)

**The finding (tools/model_check_join.py, 1536 candidate machines
eliminated against 13 oracle fire-sequences -> one core survivor):**
right-insert provenance is semantic. FRESH-INSERT rights join
pre-batch lefts (the certified D-013 behavior, unchanged).
UPDATE-ENTRY rights (alpha entry via modify) process in a LATE pass
AFTER left-inserts — they see same-batch lefts in memory — walking
lefts NEWEST-ARRIVAL-first. Forced by pr_hw_jw3 vs pr_hw_jr10:
event-identical timelines, opposite oracle orders; entry provenance
is the only difference. Implemented as: ph=1 provenance tag on
update-entry right staging (engine.rs alpha-transition site) + late
pass B in do_join_node + an arrival-sequence side-table
(Node.lseq — certified memory ORDER untouched; arrival is tracked
separately because staged-iteration fill order is NOT arrival order,
and coupling them corrupts later batches' walks).

**Verified fixed by this:** fz_999_5014 (+min), fz_min_6812 +
fz_42_6812 (the dyn-salience pair-order latent — same root), the
pr_hw_jr1..jr10 re-entry ladder (10/10, promoted to probes), wave-1
pr_hw_* probes hold. (5014/6812 stay in xfail until D-083 closes —
they pass today; graduation happens when the tree is fully green.)

**The open conflict (why this is a checkpoint, not a close):** two
oracle-certified behaviors conflict under the current model. The
fz_min_455 fix (rights-arrival memory fill) breaks 34 D-013-era
probes; BOTH are oracle-certified in different shapes. And 7
scenarios are KNOWN-RED at this commit — u12_selfjoin_multi_hot,
u13_unindexed_hot_mid, u16_two_updates_compound (D-027 update-order
pins) + fz_42_1176, fz_42_3408, fz_777_3846, fz_999_3298 — certified
shapes where SOME update-entries must stay early. They name the
discriminator precisely. A finer discriminator is still hiding.

**The D-083 plan (next session, fresh context):** extend the replica
with the 7 counterexample timelines + oracle expectations. Enumerate
candidate discriminator dimensions — pure-entry vs re-entry,
rule-origin vs external, linked-history — and eliminate against the
counterexamples. Do NOT pre-commit to any one discriminator; let the
counterexamples select the survivor. Both behaviors are
oracle-certified, so the goal is faithfully reproducing the real
provenance-dependent dual behavior, not choosing one. Then implement
the survivor. Same eliminate-against-the-oracle loop that just
cleared four families.


### D-083: update-entry rights split on RE-ENTRY, not provenance —
### pure entries are PLAIN inserts; the D-082 conflict is closed
### (tools/model_check_join2.py: 32 machines x 22 oracle timelines,
### unique survivor; corpus 732/732)

Executed the D-082 plan, two elimination rounds:

**Round 1 — the 7 counterexamples select provenance.** Rebuilt the
replica as a full two-level join-pipeline port (model_check_join2.py):
certified mechanics FIXED (LIFO staging, head-first consumption,
memory append-on-process, reorder re-appends hot lefts at the END in
staged order with child reAdds, Rupd/Lupd cursor sync-walks, plain
right-inserts walking the post-reorder bucket memory-forward, LIFO trg,
terminal dels->upds->ins) and ONLY the update-entry-right treatment
free. Timelines hand-extracted from oracle logs: u12/u13/u16 (flip
batches decompose as: refires via the upd channel, then RU children in
post-reorder MEMORY-REVERSED order), fz_42_1176 (RU block before
Lupd-new children — Drools' rightInserts-after-leftUpdates phase order
made visible; hot-refresh order = child-list order via LIFO staging),
fz_42_3408 (three flush batches, incl. B2's re-appended block firing
between the B3 hot block and the colds), fz_999_3298 (LIA-level:
node "arrival" = staged-processing order, insertion-REVERSED within a
batch), fz_777_3846 (left-side update-entry = plain LINS; its children
fire BEFORE the right-RU block purely from trg LIFO). 64 machines ->
unique survivor: rule-origin = plain / external = late+lseq-desc.
Landed as ph = origin.is_none(); tree went 718/718.

**Round 2 — the fuzz gate falsifies provenance within minutes.**
Seed-42 case 440 (external PURE-entry + same-epoch facts-insert on a
LINKED node) diverged: the oracle fires it PLAIN. Bisect: identical at
D-082 — a pre-existing hole the jr ladder never drew (jr1-jr8 are all
out-and-back RE-entries; jr10's pure entry is masked by never-linked
staging accumulation, fz_7_145 — with held staging it reproduces under
PLAIN treatment, no late pass involved). New probes filled the matrix
(pure/re-entry x action/facts-insert): pr_hw_jr11/jr16/jr18 (pure +
same-batch inserts, both flavors) fire PLAIN orders exactly;
pr_hw_jr17 (re-entry + facts-insert) fires the late order. Replica
round 2 with gate dimension {provenance, reentry, always_late, never}
x late-pass treatment, 32 machines x 22 timelines -> unique survivor:

- **gate = REENTRY: an update-entry right whose fact has a staged DEL
  at the same node in the same batch (left the alpha earlier in the
  batch, out-and-back) takes the late pass (after left-inserts, lefts
  walked newest-lseq-first, LIFO trg — D-082's machinery, unchanged).**
- **ALL pure entries — rule-origin or external — are ordinary right
  inserts: rightInserts slot, post-reorder memory-forward walk.** The
  reorder phase's re-append of hot lefts is what makes their children
  fire hot-block-first (memory-reversed) — no special walk needed.

This is the SAME staged-del+staged-ins signature D-081 pinned for
existential re-entries — one mechanism across node kinds. D-082's
"fresh-vs-update provenance" was a proxy: in its data, every rule
case was pure and every discriminating external case was a re-entry.

Engine: ph=1 iff s_right.del holds the fact at the (false,true)
alpha transition (engine.rs); the D-082 late pass + lseq side-table
stand, now correctly gated. The one-line provenance version is gone.

State: corpus 732/732 (11 baseline + 454 probes + 267 regressions) —
the 7 counterexamples green (u12/u13/u16
were D-027-era pins red since the D-082 checkpoint), pr_hw_jr11/16/
17/18 promoted, fz_42_440 + fz_42_6521 (both provenance-falsifiers
from the round-1 fuzz run) graduated. xfail graduates
(bisect-attributed, 4x stability-checked): fz_999_5014,
fz_42_6812+min (D-082's late pass, documented), fz_27182_1227+min,
fz_999_8145+min, fz_7_9151 (already green at D-082 — cleared by the
D-081/D-082 waves, never re-checked). xfail 87 -> 79, all re-verified
still-red under the final model = D-080 TMS envelope + the D-081
queue (fz_min_455 rights-arrival fill, fz_min_4816/xf_min_9976
collect pair, fz_min_3959 query rows, nb3, xf_tms_min812,
fz_42_84-family Drools-nondeterminism witnesses).
Fuzz gate (WITNESSED): seeds 42/7/123/777/999 x 10,000 = 50,000
cases, ZERO divergences (~315s/seed; seed 999 drew 1 name-suppressed
quarantined xfail, no new failures).


## Hardening wave 2 — the D-081 queue (2026-07-06, post-D-083)

### D-084 (OPEN, fenced): held-staging drain semantics across fire
### boundaries — six-round elimination record; 455/4816 families
### re-parked; ten new oracle pins landed as green probes

fz_min_455's mechanism (SEINE_TRACE): a rule left empty by its own
firing goes unlinked; a later flush's right insert stages at its node
and is never evaluated before the fire call ends. At the next call
the engine drains the held right LIFO-merged AFTER that call's fresh
stagings — Drools pairs the held right FIRST (fill [#1,#2,#4], not
[#1,#4,#2]). The probe ladder pr_rl2..rl10 (all PROMOTED, all GREEN —
they pin drain orders that hold-semantics already reproduces) plus
four fuzz counterexamples drove six elimination rounds over candidate
mechanisms; EVERY round's survivor was falsified by the next 10k-seed
gate (the D-083 fuzz-gate lesson working as designed):

1. Eager re-queue of unlinked-was-linked dirty rules — killed by
   pr_rl3 (two same-fire flushes drain as ONE accumulated batch).
2. Fire-end forced drain of every ever-linked dirty path — killed by
   xu2 + pr_hw_not_unblock (not-gated rules hold).
3. Whole-node fire-boundary windows — killed by fz_42_4035 + pr_rl9's
   inert-RHS full-queue readout (both-sides-live nodes LIFO-merge).
4. One-side-empty node windows — killed by fz_123_2742 (external-
   origin held rights hold even with the left side gone).
5. Per-side windows + other-side-quiet — killed by fz_123_3482
   (a rule-flush left on a shared prefix must stay held).
6. Per-side + rule-flush-origin-only — killed by fz_999_6009 (a
   rule-flush T2 class where the advance re-orders R2's deletes).

RULING (stop-rule: a scope predicate past ~3 conjuncts that fuzz
keeps falsifying is a wrong reification): the boundary-advance is
DISABLED (close_boundary_windows no-ops; the TrieNode.win plumbing
and the walk's window-batch loop stay, inert, for the resumed hunt).
The engine keeps the pre-D-084 hold-everything-LIFO semantics —
oracle-wrong for exactly TWO shapes, both re-parked to xfail:
fz_min_455 + fz_7_455 and fz_42_4816 + fz_min_4816. Every other
casualty of the six rounds PASSES under hold semantics and is
graduated green: fz_42_4035, fz_123_2742, fz_123_3482, fz_999_6009
(regressions — they now guard the resumed hunt from repeating rounds
3-6), pr_rl2..rl10 (probes).

Next step when resumed (decide with Bryan first): port the real
staged-tuple lifecycle from the drools-core sources
(SegmentMemory.getStagedLeftTuples, PathMemory link notifications,
RuleExecutor.evaluateNetworkIfDirty, LazyPhreakBuilder segment
init) — the D-025 precedent — rather than a seventh black-box round.
The 455-class draw rate is ~1-2 per 50k cases; the fence is name-
keyed in xfail and the four scenarios document the exact envelope.

### D-085: accumulate propagateResult drops the peer kept-kind marker
### — xf_min_9976 + fz_999_9976 closed

eval_acc_node's propagateResult path resolves a result UPDATE against
the FIRST sink's pending insert (normalizeStagedTuples) and re-stages
it as an INSERT — but omitted the trg.peer_upd marker that
Out::child_upd sets (D-071 kept-kind). With the first sink NEVER
evaluating (a never-linked sharer holding the pending insert
forever), the second sink's peer_merge_left saw a plain insert for a
tuple LIVE at that peer and dropped the staging entirely
(re-add-to-memory-end, no refire) — eating the oracle's refire of the
existing activation when a collect result grows. One line: push the
marker before add_ins_ph. Shape: two rules sharing a leading
`collect(...)` where the first-built sink's second pattern never
matches (fz_999_9976's R1 f1-matches filter).

### D-086: armed query items queue only while the query path is
### LINKED — fz_min_3959 + fz_999_3959 closed

The blanket pending=armed over-approximation ("a drain that appends
nothing is inert") is unsound across multi-epoch scenarios: an armed
query (D-058) whose every or-branch misses some positive pattern does
NOT queue on WM events in Drools — its staged facts accumulate and
drain as ONE window at the linking event. fz_min_3959: Q1's
`T0(f1 != true)` pattern is empty until epoch-2's insert, so Drools'
memory = [10] + [-1e9,-5,100] (epoch-1's 100 rides the epoch-2
window, newest-first within it) while the engine drained per-epoch
([10][100][-1e9,-5]) and swapped rows. Mechanism confirmed by grafted
runner dumps (RunnerDump: JoinNode(17) key-list [10,-1e9,100] with
ZERO query calls — the fill is eager via the armed item, gated by
linking; a plain KieSession replica without the arming ?query rules
fills lazily in one reverse-insertion batch). Engine:
queries::query_linked (some branch with every positive pattern's
alpha populated) gates mark_queries_pending. Pull evaluations
(?query CE / getQueryResults) drain regardless, as before. In-subset
the link transition is monotonic (queries + mutation stay walled,
D-051), so the gate's surface is exactly the probed shape.

**Wave-2 gate (WITNESSED):** corpus 749/749 (11 baseline + 463 probes
+ 275 regressions); fuzz seeds 42/7/123/777/999 x 10,000 = 50,000
cases, ZERO divergences, zero quarantined-name draws. Configuration:
hold-LIFO boundary semantics (D-084 advance disabled) + D-085 marker
+ D-086 query link gate. xfail count now 75: OUT this wave
3959-pair (D-086), 9976-pair (D-085), 3482 graduated green (4035/
2742/6009 were fuzz finds, never parked); IN (back) 455-pair +
4816-pair (the D-084 fence). The 75 = 68 D-080 TMS envelope +
D-042 order-trio (nb3, fz_7_2364, fz_min_7_2364) + the 4-scenario
D-084 fence.

**HANDOFF @ wave-2 close (2026-07-06) — Bryan's rulings + the wave-3
worklist:**
- D-084 (455/4816 fence): RESUME VIA SOURCES-PORT ONLY — Bryan ruled
  black-box has hit its limit (six falsified rounds); the port of the
  drools-core staged-tuple lifecycle (SegmentMemory.getStagedLeftTuples,
  PathMemory link notifications, RuleExecutor.evaluateNetworkIfDirty)
  is deferred to a LATER session, likely Opus (read-the-source-and-
  port-a-located-mechanism work). Do NOT black-box this class further.
  Validation harness for that port is already in place: pr_rl2..rl10 +
  fz_42_4035/fz_123_2742/fz_123_3482/fz_999_6009 (green, guard rounds
  3-6) + the 4 fenced scenarios (455-pair, 4816-pair).
- NEXT SESSION (fresh context): D-080 TMS envelope TRIAGE — classify
  the 68 TMS xfail witnesses into (a) pinnable → probe + fix, (b)
  Drools-nondeterministic → verify 3x across JVM launches, fence as
  UNCERTIFIABLE with the runs documented (fz_42_84 family expected
  here — quarantine-and-document is the CORRECT outcome for
  nondeterminism, not cracking), (c) genuinely-ambiguous micro-timing
  → fence with a D-entry. Commit triage results; keep DECISIONS
  current. Reminder: oracle TMS probes need 2-3 runs before trusting
  any PASS (D-080 note).
- Fold the D-042 order-trio (nb3, fz_7_2364, fz_min_7_2364 —
  mut+del+not order-only quarantines, pre-TMS) into that triage or
  fence it explicitly with its own entry.
- State at handoff: HEAD 0a614a7, corpus 749/749, 50k fuzz clean,
  xfail 75 = 68 TMS witnesses + D-042 order-trio (3) + the D-084
  fence (455-pair + 4816-pair = 4). Tooling from this wave:
  RunnerDump.java pattern (graft memory dumps into a copy of the
  oracle runner — hand-built session reproductions missed what it
  caught), pr_rl9-style inert-RHS full-queue readouts.


## D-080 TMS envelope triage (2026-07-06, post-wave-2)

### D-087: xfail quarantine triaged — ZERO in-envelope pins; every
### witness classified and fenced on 10-run oracle evidence
### (tools/triage_xfail.py; per-witness table in docs/xfail-triage.md)

Executed the wave-2 handoff mandate: classify the 68 D-080 TMS
witnesses into pin / fence-nondeterministic / fence-ambiguous, folding
in the D-042 order-trio. Method: engine once + oracle x10 INDEPENDENT
JVM LAUNCHES per witness (above the D-080 2-3x bar), canonical D-003
comparison, plus a textual screen of every witness against the
D-078/D-080 fence line (markers: A = justifier same-RHS mutation,
B = stated insert of the logical type, RD = rule delete of it,
SJ = CE-only self-justifier).

**Headline: the pin bucket is EMPTY.** All 45 deterministic divergers
carry fence markers (census A 25 / B 29 / RD 12 / SJ 17; combos led by
A,B x14 and pure SJ x13) — no witness diverges inside the certified
envelope, the fence sits exactly where D-078/D-080 drew it, and no
engine change is warranted. The remaining 23 TMS witnesses have no
stable oracle to certify against at all (22 runaways + 1 order-nondet).

Classification (all 75 xfail files, non-TMS families included):
- (i) COMPOUND TRANSIENT-VISIBILITY, 45 — oracle 10/10 identical;
  small firing-multiset deltas in BOTH directions (differing transient
  windows, not a systematic under/over-fire); 5 also differ in final
  facts. Narrative pair: xf_tms_min812 (engine parks the self-defeat;
  Drools lets a sibling accumulate rule fire ONCE against the transient
  before the lazy retraction — 2 firings vs 1, same facts) and
  fz_7_9902 (firing logs IDENTICAL; the oracle nets one extra stated
  duplicate — stated/justified key bookkeeping, no timing component).
  Fenced per D-080, now itemized per witness.
- (ii) DROOLS RUNAWAY, 22 — oracle fire-limit 10/10 for EVERY witness;
  all SJ shapes (the fz_42_946 family); the engine terminates on all
  of them (2–15 firings — the certified self-defeat park). The
  fz_42_84 family (84/581/2657) did NOT reproduce D-080's pass/limit
  flip in 10 launches; the recorded launch-dependence stands — either
  way there is no stable oracle answer to certify against, and clean
  termination is the strictly better behavior.
- (iii) DROOLS ORDER-NONDET, 1 — fz_123_6887 (B,RD): 6/10 vs 4/10
  firing-order flip across launches (same 14-firing multiset, same
  facts; an R5/R3 refire-interleave swap). A NEW nondeterminism
  witness beyond the 84-family — further independent evidence the
  fence line sits where Drools' own behavior stops being a function
  of the program. (The engine is additionally 3 transient refires
  short of both variants — family-(i) class; facts match.)
- (iv) D-042 ORDER-TRIO, 3 — nb3/fz_7_2364/fz_min_7_2364 (no TMS):
  oracle 10/10 stable, engine order-only (first swap @2–3). The
  accepted carve-out is RE-AFFIRMED on stronger evidence; the
  D-081/D-083 re-entry machinery did not dislodge it (the class
  siblings fz_999_8145/fz_27182_1227 graduated at D-083; this trio is
  the residue). Revisit per D-042's trigger only (value-bearing
  variant or new mechanism evidence), most naturally alongside the
  D-084 sources-port (both are RuleExecutor/staging internals).
- (v) D-084 FENCE, 4 — the 455/4816 pairs re-verified
  oracle-DETERMINISTIC 10/10: the held-staging class is deterministic
  mechanics, not nondeterminism — consistent with Bryan's
  sources-port ruling. fz_42_4816 is ORDER-ONLY (swap @51 of 64);
  the other three carry equal-count firing/fact swaps.

No engine, corpus, or generator changes — documentation artifacts
only. tools/triage_xfail.py is rerunnable (engine + N fresh-JVM
oracle replicates + shape screen; prints a loud PIN-CANDIDATE line if
any diverger ever appears without a fence marker) and reproduced the
identical taxonomy on an independent 3-launch smoke run (13 launches
total). xfail stays 75 name-keyed files; corpus/fuzz gate unchanged
from the wave-2 close (749/749, 50k clean at 0a614a7). With this the
D-075/D-080 hardening worklist is CLOSED — P1c (nested existential CE
groups) is unblocked.

**HANDOFF @ triage close (2026-07-06)** — D-087 landed at 707090a:
xfail fully itemized (zero pins), the D-075/D-080 hardening worklist
is CLOSED. Gate re-verified at that commit: `make test` green,
`make diff` 749/749 (11/463/275). No engine changes this session —
documentation, tooling, memory only. NEXT: **P1c nested existential
CE groups** (FEATURES §2 P1: multi-pattern/nested `not(…and…)`,
`exists(…or…)`; pairs with the D-070 CE-group machinery) on the
hardened base — probe-first per §0. Deferred, trigger-gated: D-084
sources-port (Bryan: later session, likely Opus; validation harness
pre-built), D-042 trio (value-bearing variant or new mechanism
evidence; revisit naturally rides the D-084 port). Reminder for any
TMS-adjacent probing: 2–3 oracle runs before trusting a PASS (D-080),
and tools/triage_xfail.py re-screens the quarantine in one command.


## Phase P1c — nested existential CE groups (2026-07-06)

### D-088: PROBE FINDINGS (pre-implementation — Bryan review gate):
### RIA-subnetwork semantics for not(…and…)/exists(…or…) PINNED
### (probe ladder sn_* — 33 scenarios in probes_pending/p1c/, all
### decoded, zero contradictions; order probes byte-stable across 2
### independent JVM launches; sources: LogicTransformer,
### GroupElementBuilder, PhreakSubnetworkNotExistsNode,
### RuleNetworkEvaluator.doRiaNode/doRiaNode2, RightInputAdapterNode)

NO ENGINE CHANGES in this checkpoint — findings only, per Bryan's
"report before implementing" directive. The engine still walls all
these shapes (D-031); the oracle ran every probe.

**Acceptance envelope (scope per Bryan step 3).** Misc2Test
#testNestedNots1/2/3 exercise: not(A and B); not((A and B) or (C and
B)); repeated identical conjuncts across/within rules (sharing —
DROOLS-444 was the crash); ((not A) or (not B)) or-of-bare-nots (P1a
DNF already covers); all leading-CE on EMPTY WM asserting fire
counts. sn_d2 reproduces testNestedNots2's counts exactly
(1,1,1,1,4 = 8). FirstOrderLogicTest#testRemoveIdentitiesSubNetwork:
`P($l : likes) not(C(t == $l) and C(t == $l))` outer-correlated
self-join group + retract-driven unblock — shape adapted (the test's
RemoveIdentitiesOption.YES is a config = WONT; under default config
self-pairs DO count, sn_a7, j09-consistent). NOTE: neither test is
machine-extractable to baseline (JDK fact classes String()/Integer();
kbase config) — acceptance weight rides the adapted probe mirrors,
as with the Q phases.

**Compile model (LogicTransformer, drools-base — parse-time rewrites
the engine must mirror):**
1. `not(A or B)` → `and(not A, not B)` (De Morgan). Observable:
   sn_f1a — not(A or B) fires ONCE on empty WM while `(not A) or
   (not B)` fires TWICE (DNF subrules).
2. `exists(A or B)` → `not( and( not(A), not(B) ) )` — double
   negation, NOT exists-per-branch. Observable: sn_f2 — fires ONCE
   even when both A and B present ((exists A) or (exists B) fires
   twice); sn_f3 — gains/loses membership without refires while ≥1
   member type is populated. CONSEQUENCE: exists(…or…) REQUIRES bare
   nots nested inside a subnetwork (sn_g5 pins that shape directly).
3. or-inside-and pulls up to top-level DNF = P1a subrule machinery
   (sn_f5: (A or B) + not(C and D) = 2 subrule firings).
4. Single-child groups collapse (pack): only not(AND)/exists(AND)
   reach the network builder.

**Network build (GroupElementBuilder).** The inner AND chains
ORDINARY join nodes off the FORK tuple source (the outer prefix —
inner constraints see outer bindings, sn_a5; inner bindings cross
inner patterns, sn_a6/sn_a9 at 3 patterns). A RightInputAdapterNode
converts the subnetwork tip into the outer CE node's right input.
(NotNode carries TupleStartEqualsConstraint, ExistsNode empty
constraints — both irrelevant to evaluation, which correlates
structurally.)

**Evaluation (PhreakSubnetworkNotExistsNode) — a THIRD CE machine,
counting-based, NOT the bare-CE blocker model:**
- Per-left matches list; each subnetwork tuple maps to its start
  left by PARENTAGE (BetaNode.getStartTuple: parent walk to the fork
  index + peer walk to this node). No blocker search, no right
  memory scans, no index machinery at the outer node.
- Phase order: leftDel, rightIns, leftIns, rightUpd(=NO-OP),
  rightDel (deliberately last), leftUpd (after rightDel).
- Transitions only at count edges: not fires at 0 matches (leftIns)
  and on →0 (rightDel); exists fires on 0→1 (rightIns); children die
  on the inverse edges. Counting subsumes handover: support/blocker
  2→1 = NO refire, NO cancel (sn_b6).
- Subnetwork-tuple UPDATES are literally dropped ("here before, here
  now"): in-place inner updates never refire — even value-CHANGED
  still-alpha-passing ones (sn_b7, sn_e1). Only alpha TRANSITIONS
  act (exit sn_b8, entry sn_b9 — modify-entry reaches subnetworks).
- LEFT updates propagate a child UPDATE → fired activations REFIRE,
  gated by the outer pattern's listen mask (sn_b10: `$v : f0`
  binding listens {f0} and refires; bare `P()` does not). no-loop
  scopes per rule as usual (sn_c8).
- Pending activations cancel on pair-formation (sn_b2) and on
  last-support loss (sn_b2x), exactly like bare CEs.
- Evaluation window: the subnetwork evaluates INLINE at the outer
  node's turn (stack resume in doRiaNode; RIA stages SubnetworkTuples
  into the outer node's staged RIGHTS with same-batch ins+del
  folding to nothing). Lazy rules accumulate; eager (no-loop) rules
  see per-flush windows (sn_c9: eager not/exists fire P1,P2; lazy
  fire P2,P1 off the LIFO-accumulated batch). Same-RHS delete+insert
  of a support: NO refire (sn_c5b — phase order keeps count ≥1);
  cross-firing delete-then-reinsert: REFIRES (sn_c5 — the ne_x2
  queue-on-unlink analog; exists sinks unlink when the subnetwork
  path unlinks, so the transition force-queues).
- Linking asymmetry (staticDoLink/UnlinkRiaNode): subnetwork-path
  LINK links the outer sink; subnetwork-path UNLINK **links a NOT
  sink** (nothing can block — sn_c7: not fires with the inner alpha
  EMPTY, before any subnetwork data ever existed) and **unlinks an
  EXISTS sink** (holds staging until support is possible).

**Order pins (the headline: subnetwork CEs are EXACTLY INVERTED vs
bare CEs within a window):**
- not children ride the LEFT walk → ARRIVAL order: initial batch
  sn_a3 (P1,P2,P3), rule-origin mass-unblock sn_b3/sn_b3x.
  Bare not = reverse-arrival (ne_n4/pr_hw_not_unblock).
- exists children ride the RIGHT walk (subnetwork staging) →
  REVERSE-ARRIVAL: initial batch sn_a3 (P3,P2,P1), mass-support
  sn_b4. Bare exists = arrival (pr_hw_exists_support).
- EXTERNAL-action windows flip the not side: external delete
  unblocks fire REVERSE-arrival (sn_x1, sn_x2) vs rule-origin
  arrival (sn_b3, sn_b3x) — 2×2 filled per the D-083 lesson (origin
  is the discriminator, not left count). External insert support =
  reverse-arrival like rule-origin (sn_x1 epoch 3).
- Pass-through: a not node PRESERVES its incoming batch order (then
  the standard D-013 prefix reversal applies at later joins — sn_c3
  R3: P2Q1,P2Q2,P1Q1,P1Q2); an exists node REVERSES the incoming
  batch (sn_c3 R2 full reversal of the join output).
- Sharing: the certified trie model extends verbatim — first sink
  preserved, later sinks flipped (sn_d1 twins; sn_d3 not+exists
  sharing ONE subnetwork RIA with kind-specific orders), and
  referenced inner-binding NAMES are identity-significant exactly
  like ne_t13/t14 (sn_d4: $y/$z twins do NOT share — no flip; $y/$y
  twin DOES — flipped).

**Quirk check:** the D-041/mn6 subnetwork false-admit does NOT
reproduce for not-groups (sn_e2 — mask-hit modify of an
alpha-FAILING outer fact stays correctly excluded; bare-not control
agrees). The quirk stays collect-specific; no new Drools quirks
surfaced; every probe fits one model.

**Walls verified against the oracle (all recorded, honest fences):**
- Inner bindings referenced DOWNSTREAM of the group = faithful
  Drools COMPILE ERROR (sn_g1) — engine mirrors as parse error.
- Legal-in-Drools but PROPOSED OUT of P1c (recorded behavior for the
  fence notes): `not(exists(A and B))` (sn_g2), `not(not(A))`
  (sn_g3 — fires iff A exists), `exists((A and B) or C)` (sn_g4 —
  composite or-branches build RIA-inside-RIA after the rewrite).
  Fence = clean parse error on composite groups NESTED inside
  groups; bare not/exists inside a group stay IN (sn_g5 — required
  by the exists(or) rewrite and the forall shape not(A and not B),
  both behave compositionally).

**Proposed P1c envelope (for Bryan's review):**
IN: not/exists over AND-groups of 2–3 positive patterns; inner
bindings crossing inner patterns and referencing outer bindings;
literal alphas + the certified operator set inside groups; bare
not/exists nested INSIDE groups; not(or)/exists(or) with
single-pattern branches (compiled via the pinned rewrites); leading
(InitialFact) and any-position groups; multiple groups per rule;
shared groups across rules; group CEs inside or-branches; rule-RHS
and external mutation of inner/outer facts; D-031's parenthesized
single-pattern fence lifts (`not (A())` = bare not after collapse).
OUT (compile-rejected, mirroring the acceptance envelope): composite
groups nested inside groups (RIA-in-RIA: not(exists(and)),
not(not()), exists(or) with composite branches); bindings escaping
groups (faithful Drools error); groups in query bodies (D-073 fence
stands); accumulate/collect/?query inside groups; group CEs in
insertLogical-justifier rules (D-076 wall extension — revalidation
over subnetworks unprobed); >3 inner patterns.

**Implementation sketch (NOT started; post-review):** parse-time
rewrites (De Morgan / double-negation / collapse) → subnetwork = a
trie BRANCH off the fork prefix reusing the certified join nodes,
tipped by an RIA staging into the outer node's rights (peer copies
for later sinks — existing machinery); new SubnetNot/SubnetExists
node implementing the counting machine with the pinned phase order;
start-tuple correlation = the branch's fork-prefix tuple id (native
to the trie); linking gates per the asymmetry; queue-on-unlink
reuses D-032. Replica-first (model_check pattern) against all 33
probes for the list-level order fine structure (not=arrival,
exists=reverse, external-not=reverse) BEFORE Rust. Generator: group
draws with the type-DAG termination discipline extended to inner
patterns; fuzz-gate EVERY discriminator (D-083 lesson) — the
external-vs-rule-origin unblock asymmetry gets targeted weight.

### D-089: P1c LANDED — group CEs as trie-branch subnetworks + the
### counting machine; D-088's origin-keyed unblock claim CORRECTED
### (replica tools/model_check_subnet.py + probes sn_b3e/sn_x5;
### corpus 793/793 at first differential contact; fuzz gate pending)

**Correction to D-088 (the replica's catch):** the "external-vs-
rule-origin unblock asymmetry" was a SECOND-layer confound. The real
axis is the PHASE that creates the not-children: leftIns children
fire ARRIVAL order (the left walk), rightDel unblock children fire
REVERSE-arrival (the right walk). Origin correlated in all seven
D-088 probes because rule-origin deletes always landed before the
not's first evaluation (staged ins+del FOLD at the RIA hop — the
pair never forms, children ride leftIns) while external deletes
landed after (formed matches die via rightDel). Discriminators:
sn_b3e (rule-origin delete + EAGER no-loop not that already
evaluated → fires REVERSE) and sn_x5 (external delete folding with
held staging before any evaluation → fires ARRIVAL). No origin flag
exists anywhere in the implementation — one machine, one mechanism.
The dual behavior DISSOLVES; faithful reproduction needs no
provenance tracking.

**Replica (tools/model_check_subnet.py):** certified mechanics fixed
(LIFO staging, head-first consumption, merge/append_into_pending,
first-sink append + later-sink peer flip, terminal FIFO, agenda
salience/decl, eager-per-flush vs lazy accumulation), free dimensions
= RIA transfer direction, counting-node child staging per phase, tip
delete-walk, external variant, fork sink order. 16/512 survivors =
one parity family; the source-faithful member (RIA hop REVERSES via
per-entry prepend; child creations prepend; walks head-first; NO
external special-casing) was implemented and confirmed by the probe
battery. c3-not additionally pinned fork build order: the subnetwork
attaches FIRST (Drools GroupElementBuilder order), the outer node is
a LATER sink of the fork.

**Implementation:**
- Parser (drl.rs): CeNode gains Not/Exists; `not (`/`exists (`
  intercepted at lhs_unary; normalize_ce = the LogicTransformer
  mirror (NotOr → and-of-nots; ExistOr → not(and(not,not)); AndOr
  left-major pull-up; single-child pack); lower_group fences
  RIA-in-RIA (not(not), not(exists(and)), composite or-branches),
  >3 inner elements, acc/collect/?query inside groups, bindings on
  bare-CE members (D-031 kept), and collapses single-pattern groups
  (the or_a41 fence lift). Group-inner bindings join the
  duplicate-declaration check (no shadowing — subset stricter than
  Drools, generator never emits it).
- Engine (engine.rs/phreak.rs): groups FLATTEN in compile_rule
  ([inner..., Outer] with SubRole markers; inner tuple slots extend
  the main prefix without claiming rule-tuple positions; inner
  bindings scoped out after the group → later references fail with
  the faithful "unknown binding" error, sn_g1). build_network hangs
  the subnet branch off the fork (inner chain = ordinary shared trie
  join nodes — sharing identity for free, incl. ne_t13
  name-sensitivity inside groups, sn_d4), tips carry Sink::Ria into
  the outer node; kinds SubnetNot/SubnetExists evaluate engine-side
  (eval_subnet_node): counting per start-left (truncation to the
  fork prefix), phase order leftDel/rightIns/leftIns/rightUpd-NOOP/
  rightDel/leftUpd, children through the D-041/D-071 Out clash
  machinery. RIA staging = per-entry prepend with TupleSets folds
  (same-batch ins+del cancels — sn_c5b no-refire vs sn_c5
  cross-firing refire). Linking: inner positions never gate; subnet
  NOT never gates (fires with an empty inner alpha before any
  subnetwork data, sn_c7); subnet EXISTS waits for a producible
  branch (all inner alphas populated) or live matches
  (staticDoLink/UnlinkRiaNode asymmetry).
- D-076 wall extension (Bryan's ruling): insertLogical from rules
  with group CEs = compile error (justification revalidation over
  subnetworks unprobed). D-057 ?query-mix wall covers groups via CE
  kind. Groups in query bodies remain fenced (D-073).
- Probes: 44 promoted (pr_sn_*), incl. the full order battery,
  rewrite pins, sharing, masks, external epochs. sn_g1..g4 stay
  UNPROMOTED as fence evidence (g1 = both-sides compile error;
  g2/g3/g4 = engine fence vs Drools-legal RIA-in-RIA, recorded in
  D-088). Gate at this commit: make test green (incl. 2 new parser
  test suites), make diff 793/793 (11 baseline + 507 probes + 275
  regressions). Generator + 5x10k fuzz: NEXT (gate line appended
  below when witnessed).

**P1c gate (WITNESSED):** corpus 795/795 (11 baseline + 509 probes +
275 regressions — 45 pr_sn_* + pr_acc_lu_range promoted); fuzz seeds
42/7/123/777/999 x 10,000 = **50,000 cases** with group CEs in ~19%
of cases (and/or forms, outer-correlation, inner-crossing, bare-not
inners incl. the forall-correlation shape): seeds 42/7/777 zero
divergences; seeds 123 and 999 drew ONE divergence each —
fz_123_8426 and fz_999_2256, BOTH bisected PRE-EXISTING (pre-P1c
engine byte-identical on both minimized repros; the
D-071/D-072/D-075/D-077 widened-grammar-flushes-latents precedent),
both quarantined per D-075 (D-090a/b below), and both seeds RERUN
CLEAN modulo the name-keyed suppression. The first campaign launch
also caught an unlinked-queue-pruning PANIC in new code, fixed at
400852b (inner tpos values share the numeric space of later MAIN
slots by design — every rule-tuple-space consumer now excludes
SubRole::Inner; the pindex source lookup scans backward). Note for
the D-084 port: sn_right staging is NOT in the (inert) TrieNode.win
plumbing — integrate it if the boundary-advance returns.

**forall reducibility (Bryan's Q4, flagged — stays P2):** Drools'
ForallBuilder rewrites `forall(base, remaining…)` to
`not(base and not(remaining…))`. The MULTI-pattern single-remaining
form is a pure parse rewrite onto the D-089 substrate — zero new
machinery; the load-bearing correlation shape
`not(A($y : k) and not(B(m == $y)))` is probe-backed (sn_a10) and in
the fuzz grammar. NOT free: the flagship SINGLE-pattern form injects
a `this == <base>` identity join (no fact-identity operator in the
subset), and multi-remaining builds RIA-in-RIA (fenced). Recorded in
FEATURES.md; forall remains its own phase.

### D-090a (quarantine): fz_123_8426 — accumulate leftUpd churn with
### the source and the left touched in ONE batch; LATENT, own-ladder
Minimized to 2 rules / 3 facts / no epochs (xfail/fz_min_8426): R0 =
`T0($b : f1)` + `accumulate(T0(f1 != -3, f0 >= $b, $s : f0);
min($s))` at salience -7; R2 (sal -8, no-loop, or-twins) rewrites
every T0's f1 := f0. In the churn tail the oracle's min for left
T0(f0=6) returns **-2** — a source fact whose `f0 >= $b` beta
constraint FAILS under the updated binding — while the engine
re-filters and returns 6. The naive theory (left updates never
re-filter range-constrained matches) is FALSIFIED by
pr_acc_lu_range (promoted, green: a clean left update over a range
source re-filters correctly in BOTH runners). Distinguishing
ingredients: the SAME facts are both accumulate LEFT and SOURCE
candidates, one RHS batch updates them in both roles (the fz_7_5893
both-sides temp-staging machinery), min's no-reverse refold path.
Needs its own discriminator ladder (both-roles x constraint-kind x
refold matrix). NOT the D-084 class (single fire call).

### D-090b (quarantine): fz_999_2256 — or-subrule self-emptying RHS
### across MULTI-EPOCH external inserts; LATENT; suspected member of
### the D-091 evaluation-timing class
Minimized to 2 rules / 0 initial facts / 2 insert-epochs
(xfail/fz_min_2256): R5 (or-twins over `T0(f0 == false)` variants)
inserts a T1 and setF0(true)+update — emptying its own alpha
(subrule unlink) — across two external-insert windows; R2 (plain
3-pattern join, inert RHS) pairs a different T1/order than the
oracle in the tail. The P1c group CE in the original draw was NOT
load-bearing (minimizer dropped it). The self-emptying-unlink +
fire-boundary shape matches the D-091 halt/deferred-evaluation
mechanism — LISTED IN THE PORT'S VALIDATION BATTERY: if the port
flips it green, attribution is confirmed and it graduates; if not,
it gets its own ladder. (Both quarantines: full + min pairs in
xfail/, name-keyed fuzz suppression, xfail count 75 -> 79.)

## D-084 sources-port — recon (2026-07-06, post-P1c gate)

### D-091: THE 455 MECHANISM FOUND IN SOURCE (pre-implementation —
### Bryan review gate): the just-fired rule re-evaluates its network
### ONLY on the fire-loop's CONTINUE path; an OUTRANKED (halted) rule
### defers to its next agenda pop, and a DIRTY-but-EMPTY item stays
### queued. The engine's unconditional post-firing force-evaluation
### evaluates too EARLY, shrinking the drain window.
### (Sources: RuleExecutor.fire/evaluateNetworkIfDirty/
### removeRuleAgendaItemWhenEmpty, PathMemory.doLinkRule/doUnlinkRule/
### queueRuleAgendaItem, SegmentMemory.notifyRuleLinkSegment,
### RuleNetworkEvaluator.evaluateNetwork/innerEval,
### RuleAgendaConflictResolver.doCompare; verified against
### SEINE_TRACE + SEINE_HANDLES runs of fz_min_455 on both runners.)

**The lifecycle as it actually is:**
1. Per-rule executor state = QUEUED (item in the agenda group) plus a
   separate DIRTY flag. DIRTY is set by (a) every staging notify on a
   LINKED path — SegmentMemory.notifyRuleLinkSegment fires on each
   staging event, → PathMemory.linkSegment → (isRuleLinked) →
   doLinkRule → queueRuleAgendaItem = setDirty(true) + enqueue if not
   queued — and (b) LINKED→UNLINKED transitions (doUnlinkRule =
   setDirty(true) + enqueue). Staging on an UNLINKED path only marks
   the segment's dirtyNodeMask — the executor is not notified (the
   hold, fz_7_145).
2. Network evaluation happens ONLY at (a) item pop
   (evaluateNetworkAndFire → evaluateNetworkIfDirty: if dirty, walk
   ALL segments draining staged sets regardless of current link
   state, then dirty=false), and (b) INSIDE the fire loop after each
   firing — on the CONTINUE path only.
3. The fire loop (RuleExecutor.fire): fireActivation →
   flushPropagations → dyn-salience requeue → haltRuleFiring
   { fire-limit; evaluateEagerList(); peek next item; HALT iff the
   next item STRICTLY outranks (salience DESC, loadOrder ASC —
   RuleAgendaConflictResolver.doCompare < 0) } → on HALT: break with
   NO self re-evaluation → else evaluateNetworkIfDirty(self), next
   tuple.
4. removeRuleAgendaItemWhenEmpty: remove ONLY when !dirty AND the
   tuple list is empty. A dirty-but-empty item survives; its next pop
   drains everything staged since — including input that arrived
   while the path was UNLINKED.

**fz_min_455 decoded (trace-verified both sides):** R0 (sal -2)
fires, its modify empties its own LIA (unlink → dirty + queued) and
restages T0 for R1 (sal 0). Drools: R0 HALTS (R1 outranks) without
evaluating; R1 refires and inserts T1#3, which stages at R0's join
(no notify — unlinked — but the item is already queued+dirty); R0's
pop then drains the left-del AND T1#3 in ONE window → T1#3 reaches
the right MEMORY in fire 1. Fire 2 stages only the fresh T1#5; the
new left joins the memory [T1#2, T1#3, T1#5] in memory order and the
first-fired R0 activation pairs the FRESH right (value-bearing: its
modify copies f0=3). The ENGINE force-evaluated R0 immediately after
its firing — draining ONLY the left-del — so T1#3 arrived at a
dequeued, unlinked rule and HELD across the fire boundary,
LIFO-merged behind fire-2's stagings → held-paired-first, f0=-4.
D-084's six black-box rounds all failed because the free parameter
was EVALUATION TIMING (a whole-agenda property), not staging-list
placement (a node-local one).

**Coexistence with the certified pins:** fz_42_5243 (just-fired rule
re-evaluates even after self-unlink) lives on the CONTINUE path —
5243's executor was not outranked. The discriminator between 5243
and 455 is exactly haltRuleFiring's strict-outrank peek. fz_42_8775
(emptied item stops claiming windows) = removal with !dirty && empty
— unchanged. D-018's outrank walk (rules below the executor are not
evaluated) is the peek discipline itself — unchanged.

**Port shape (engine, post-approval):** add a per-rule DIRTY flag
beside `queued`; restructure next_activation from
walk-all-queued-rules-per-firing into pop-item/fire-loop semantics:
evaluate once at pop; per firing: flush → eager list → peek →
halt-without-self-eval iff strictly outranked, else self
re-evaluate; item removal only when !dirty && queue empty. Expected
casualties to re-pin: none of the rl-ladder (pr_rl2..rl10 pinned
drain ORDERS the true mechanism must reproduce); the D-084 fence
pairs (455-pair, 4816-pair) must FLIP to green; watch the D-042
trio (nb3/fz_7_2364 — Bryan: the revisit naturally rides this port).
Risk surface: evaluation-window claiming for shared nodes (D-037)
shifts in preempted scenarios; the eager-list placement must keep
fz_42_4138/4141; the full corpus + 5x10k gate arbitrates.

### D-091 LANDED: the RuleExecutor dirty-flag lifecycle port — the
### D-084 fence LIFTED, 455/4816 families graduated green
### (Bryan-approved after the reclassification premise was refuted by
### measurement: oracle deterministic 15+ launches, no HashSet in the
### traced path — the finding of record is the DETERMINISTIC
### mechanism below)

Implementation (surgical, three sites in engine.rs):
1. `RuleNet.dirty` — the executor's network-needs-evaluation flag,
   SEPARATE from `queued`. Set on every staging notify while LINKED
   (refresh_linked ~ queueRuleAgendaItem.setDirty) and on link/unlink
   transitions (note_link_effects ~ doLinkRule/doUnlinkRule); cleared
   when the network evaluates (both completion paths of
   evaluate_rule_inner, and on the no-op fast path). The flag GATES
   every evaluation, force included (evaluateNetworkIfDirty): staging
   that arrives while UNLINKED never sets it, so a queued-but-clean
   item pops without draining — the faithful hold.
2. The post-firing self re-evaluation in next_activation is now
   CONDITIONAL on the fire-loop's continue path: when a
   STRICTLY-higher-salience item waits, the just-fired rule HALTS
   without re-evaluating (RuleExecutor.fire: haltRuleFiring breaks
   BEFORE the in-loop evaluateNetworkIfDirty). The gate is the same
   strictly-higher predicate that governed the D-076 TMS defer drain
   (min608 vs t11) — Drools' halt structure is WHY that pin exists;
   the two are now one mechanism. fz_42_5243 (just-fired re-eval
   after self-unlink) lives on the continue path — preserved.
3. Item removal requires `!dirty && queue-empty`
   (removeRuleAgendaItemWhenEmpty) at all three dequeue sites
   (post-firing, eager loop, pop loop) — a dirty-but-empty item
   survives to its next pop and drains everything staged since.

One fallout, fixed faithfully: the eager-flush TMS drain
(pr_tms_selfbreak_flush / pr_tms_t20d) — the deferred entry for an
eager justifier's own break was previously created by the (now
correctly halted) force-evaluation; the eager block now drains the
flush-eligible entries its own evaluation produces and re-evaluates,
so the dep removal lands at the SAME flush (evaluateEagerList inside
haltRuleFiring — the t20 2x2 pins hold).

Validation:
- The four D-084-fenced scenarios FLIP GREEN and are graduated to
  regressions after 4x stability checks: fz_min_455 + fz_7_455,
  fz_42_4816 + fz_min_4816 — the six-round black-box class closed by
  porting the real mechanism (evaluation TIMING, a whole-agenda
  property black-box staging probes could not reach).
- fz_min_2256/fz_999_2256 do NOT flip — the D-090b same-class
  suspicion is DISPROVEN; the pair stays quarantined as its own
  family (multi-epoch or-subrule churn, own ladder when picked up).
- D-042 trio (nb3, fz_7_2364, fz_min_7_2364): unchanged (still
  order-only red) — the port did not dislodge it, consistent with
  D-087's re-affirmation; its revisit trigger stands.
- rl-ladder pr_rl2..rl10 + the round-3..6 guards
  (fz_42_4035/fz_123_2742/fz_123_3482/fz_999_6009): all green — the
  drain orders they pinned fall out of the true mechanism.
- Corpus: 799/799 (11 baseline + 509 probes + 279 regressions).
- xfail 79 -> 75 (the four graduations; 8426/2256 quarantines stay).
- D-084's inert boundary-window plumbing (TrieNode.win +
  close_boundary_windows) remains disabled and now PERMANENTLY
  obsolete — the hold/drain semantics are carried by the dirty-flag
  lifecycle; the plumbing can be deleted in a cleanup pass.

Provenance: comprehension-only reading of RuleExecutor, PathMemory,
SegmentMemory, RuleNetworkEvaluator, TupleSetsImpl,
RuleAgendaConflictResolver — behavior ported, no code copied or
transliterated; validated against the oracle (same discipline as the
TupleIndexHashTable and query-stack-machine ports). NOTICE's existing
comprehension clause covers it; no NOTICE change required.
**D-091 gate (WITNESSED):** `make test` green; corpus **799/799**
(11 baseline + 509 probes + 279 regressions, incl. the four
graduated D-084 scenarios); fuzz seeds 42/7/123/777/999 x 10,000 =
**50,000 cases, ZERO divergences** (xfail draws = the two documented
quarantines fz_123_8426 / fz_999_2256 only, name-suppressed). The
D-084 fence is LIFTED; the held-staging class is CLOSED via the
sources-port. Remaining xfail: 75 = 68 D-080 TMS envelope + D-042
order-trio (3) + the 8426/2256 quarantine pairs (4).

**HANDOFF @ D-091 close (2026-07-06)** — The D-084 worklist is done:
the fence lifted via the real mechanism (evaluation timing), not a
seventh black-box round. Open quarantines with their own ladders
when picked up: fz_123_8426 (accumulate both-roles churn; naive
theory falsified by pr_acc_lu_range), fz_999_2256 (multi-epoch
or-subrule churn; D-091 attribution DISPROVEN by the port). D-042
trio unchanged (revisit trigger stands). Cleanup candidate: the
inert TrieNode.win / close_boundary_windows plumbing is permanently
obsolete post-port. P1c + D-091 both certified on this tree.

### D-091 cleanup: the obsolete D-084 boundary-window plumbing DELETED
The inert machinery is gone: `close_boundary_windows` (no-op'd since
the D-084 fence), `TrieNode.win` and its constructor/rule_dirty/walk
integration (the walk consumes staging directly — one LIFO-merged
batch, held-drain semantics carried entirely by the D-091 dirty-flag
lifecycle), and the orphaned `Node::lefts_empty`/`rights_empty`
advance-eligibility helpers. Behavior-neutral by construction (the
window vec was permanently empty); verified: `make test` green,
corpus 799/799, spot fuzz seed 42 x 10k clean.

## D-090a discriminator ladder (2026-07-06, post-D-091)

### D-092: THE 8426 MECHANISM PINNED (pre-implementation — Bryan
### review gate): Drools' accumulate LEFT-UPDATE merge skips the
### min/max refold whenever the extremum's removal is not the LAST
### dirtying step of the walk — a stale extremum survives in the
### function context and result fact forever, with a CORRECT match
### set. (AccDump ground truth + 9-probe ladder + two out-of-sample
### confirmations; sources: PhreakAccumulateNode
### .doLeftUpdatesProcessChildren/removeMatch/reaccumulateForLeftTuple,
### MinMaxAccumulateFunction.tryReverse.)

**The mechanism (probe-pinned, all arms):** the same-bucket left-
update path walks the right memory merged against the left's match
list (cursor pairing). Per element, an `isDirty` flag is ASSIGNED
(last-writer-wins): removal of the CURRENT EXTREMUM -> true (min/max
tryReverse fails only for the extremum; non-extremal removals are
no-op-reversible -> false); a KEPT match -> false
(hasRequiredDeclarations() == false for built-ins); a newly-allowed
ADD -> no write. Per-removal refolds are suppressed
(removeMatch(..., reaccumulate=false)); the ONE refold runs at walk
end iff the final isDirty is true. Consequence: the fold goes stale
(fn/result keep the removed extremum) whenever the extremum removal
is followed by any kept match or non-extremal removal — while the
MATCH SET is maintained correctly, so reversible functions
(sum/count/average) are always right and the quirk is INVISIBLE to
them (alu6b/alu7c green). The result NEVER self-heals (quiescent
fn{min=stale}, AccDump).

**Evidence:** ground truth via oracle/…/AccDump.java (RunnerDump
pattern: per-firing dump of acc memories, match chains + stored
contributions, function context, result fact). fz_min_8426 firing 11:
matches {12, 6} correct, fn{min=-2} stale — the fired -2 decoded
exactly. Ladder (probes_pending/alu*): 7a [rm-extremum, keep] ->
STALE; 7b [keep, rm-extremum LAST] -> refold (this is also why
pr_acc_lu_range was green); 7c sum -> correct (reversible); 7d
[rm, keep, rm-nonextremal] -> STALE (the arm that killed the naive
last-writer model: the trailing removal is no-op-reversible ->
writes false); 7f 4-source -> full merge confirmed (no walk
truncation; memory unmoved by reAddRight); 7g [keep, rm-extremum,
keep] -> STALE and 7h [keep, keep, rm-extremum] -> refold (both
predicted BEFORE running); 7i [rm-extremum, ADD] -> refold (add
writes nothing). alu6 ablations: or-twins not load-bearing;
insertion order load-bearing (walk order); both-roles NOT the axis —
the fz_min_8426 both-roles shape merely arranges extremum-removal-
then-kept in one walk. alu3/4/5 (earlier, green) never exercised
the merge (salience layout: the acc's first evaluation happened
post-churn) — retained as fold-from-scratch controls.

**Scope:** leftUpd merge ONLY (rightDel/rightUpd and the indexed
bucket-change path pass reaccumulate=true and refold correctly —
acc4/acc12 pins unaffected). Observable surface = min/max over i64
(D-039 walls f64 min/max results). Deterministic given event
history; faithful reproduction requires the merge walk (memory order
x match-list cursor), tryReverse-fails-only-for-extremum, the
last-writer isDirty, and the end-gate — in the engine's
eval_acc_node left-update path, which today re-derives cleanly
(correct-but-unfaithful).

**Port shape (post-approval):** eval_acc_node's left-update arm
replaces clean re-derivation with the pinned merge machine for
min/max; probes alu7a/7d/7f/7g + the 8426 pair flip green and all
alu* promote; fuzz-gate 5x10k before logging the gate line. NO
ENGINE CHANGES in this commit — probes + AccDump only; gates
unchanged (engine still diverges on 7a/7d/7f/7g + the quarantined
8426 pair, all sitting in probes_pending/ + xfail/ until the port).

### D-093: 8426 RULING — CORRECT, don't reproduce: the stale-extremum
### defect is DURABLE upstream (verified on 10.1.0 + byte-identical on
### main) and Seine deliberately diverges; doctrine refined
Bryan's ruling, executed after the upstream check came back on the
"persists" branch:
- **Upstream verification:** the D-092 mechanism is unchanged on
  current Drools main (doLeftUpdatesProcessChildren's last-writer
  isDirty + removeMatch(reaccumulate=false) + MinAccumulateFunction.
  tryReverse all byte-identical), and EMPIRICALLY reproduced on
  Drools 10.1.0 (throwaway oracle from Maven Central: alu7a fires the
  stale -2; fz_min_8426 firing[11] carries -2 — identical to
  9.44.0.Final). No fix ever landed upstream.
- **The ruling:** Seine keeps its CORRECT re-derivation (no engine
  change; the correct min/max IS the intended semantics — Drools'
  own match bookkeeping agrees with Seine and contradicts its own
  fold). This is an INTENTIONAL, DOCUMENTED divergence on a
  value-bearing upstream defect — the first of its kind in the
  project.
- **DOCTRINE (banked):** Seine faithfully reproduces Drools'
  semantics and stable/intentional behaviors — quirks included (the
  D-076 delete quirk, orderings, coercions) — but CORRECTS
  value-bearing DEFECTS where Drools' own state is self-inconsistent
  (here: match set says {12,6}, fold says -2, forever). Faithfulness
  is to Drools-the-spec, not to defects — even durable ones.
- **Witness reclassification:** xfail/fz_123_8426 + fz_min_8426 +
  alu6a + alu7a/7d/7f/7g = DOCUMENTED-EXPECTED-DIVERGENCE witnesses
  (Seine correct, Drools durably buggy) — excluded from the gate like
  the Drools-nondeterminism families, same honest-quarantine
  machinery, opposite polarity. Eleven green probes promoted
  (pr_alu3/4/5, pr_alu6b/c/d/e, pr_alu7b/c/h/i + the earlier
  pr_acc_lu_range): they pin the CORRECT behaviors both engines agree
  on (reversible-function churn, extremum-removal-last refolds,
  removal-then-add refolds, fold-from-scratch controls).
- **Generator gate (D-093 wall):** min/max accumulates draw only in
  mutation-free scenarios, and external UPDATE actions reroute to
  deletes when a min/max accumulate exists (the defect surface needs
  a left-update merge; sum/count/average are immune and keep full
  churn coverage). Without the gate every fuzz campaign would re-draw
  known-expected divergences.
- **Upstream report FILED:** apache/incubator-kie-issues#2366
  (2026-07-07, open) — title, affected versions (9.44.0.Final,
  10.1.0, main), self-contained KieHelper reproducer, root cause with
  the arm table, suggested isDirty |= fix, discriminating-case
  matrix. Text preserved in docs/drools-bug-stale-minmax.md. If
  upstream fixes it, the divergence becomes convergence — track the
  issue when bumping oracle versions.
- The D-090a "own ladder" work is CLOSED by this entry (mechanism
  D-092, ruling D-093). Remaining from the quarantine backlog:
  fz_999_2256 (D-090b — next).

## D-090b discriminator work (2026-07-06/07)

### D-094: THE 2256 MECHANISM PINNED (pre-implementation — Bryan
### review gate): within ONE fact-update Drools processes alpha
### ENTRIES during the OTN sink walk and defers alpha EXITS to the
### end-of-modify drain (ModifyPreviousTuples) — entry-before-exit
### creates a TRANSIENT all-linked window; the transient-queued item
### (fz_7_2122) drains held staging into MEMORIES mid-fire, so
### cross-boundary arrivals compose FIFO in memory where the engine
### holds them LIFO in staging. (AccDump/RTN-item ground truth;
### three eliminations en route.)

**The decode (fz_min_2256, all dump-verified):** R5's fire-1 RHS =
[insert T1("b"); setF0(true); update(T0#0)]. During the post-firing
flush: T1("b") links R2's T1 node (the LIA still stale-holds T0#0);
T0#0's update then ENTERS pattern-1's alpha BEFORE its pattern-0/LIA
exit processes (entries ride the OTN sink walk; unmatched previous
tuples retract at the END of modifyObject) — for that instant R2's
single segment is ALL-LINKED -> doLinkRule creates+queues the item
and sets the executor dirty (the item is OBSERVABLE at the fire-1
boundary: item[queued=false dirty=false] where pre-flush it was
null — items are only created by doLinkRule). The LIA exit then
unlinks the path, but the queued+dirty item pops later in fire 1
(D-091 lifecycle), drains T1("b") into the right MEMORY (rtm[b],
staging empty at the boundary — dump), fires nothing, empties clean.
Fire 2's fresh T1("zz") appends AFTER b -> the new left pair joins
memory [b, zz] -> fires zz-first. The ENGINE processes the update's
EXIT first (its on_update visits LIAs before trie nodes), never sees
the transient, never creates the item -> T1("b") stays STAGED across
the boundary and LIFO-merges behind zz -> fires b-first (the swap;
value-bearing through downstream field reads).

**Eliminated en route (each by a targeted dump/probe):** (1) an
end-of-fire staged-drain sweep — DISPROVEN by the idle-fire control
(external insert with nothing firing stays STAGED across the
boundary); (2) lazy segment-init pulls — createSegmentMemory/
processBetaNode create memories only, never drain; (3)
flushLeftTupleIfNecessary — stream/event/data-driven only. The
D-091-attribution hypothesis (D-090b) was already disproven by the
port; this mechanism is the true member of the family — note it is
the SAME machinery as fz_7_2122's pin, refined one level: the
per-event link bookkeeping the engine already implements must also
see the WITHIN-UPDATE transient.

**Port shape (post-approval):** reorder Engine::on_update into two
passes over the network — pass A: alpha ENTRIES and in-place
(mask-hit) updates, in node build order; pass B: alpha EXITS — with
note_link_effects after every node event as today. The D-081/D-083
same-node out-and-back signatures are cross-EVENT and unaffected;
the mask-miss reAdd is single-node; fz_7_2122's cross-event pin is
preserved. Validation: fz_min_2256 + fz_999_2256 flip green and
graduate; full corpus (810) + 5x10k fuzz arbitrate the reorder's
blast radius. Tooling banked: AccDump now dumps JoinNode memories,
staged sets, RTN PathMemory masks and item state per WM event and
firing, and replays epochs — the RunnerDump pattern's generic form.

### D-094 LANDED: two-pass on_update (entries before exits) — the
### 2256 family closed; D-090b quarantine dissolved
Implementation: Engine::on_update processes each fact-update in two
passes over the network — pass A: alpha ENTRIES ((false,true), incl.
the D-083 re-entry ph tagging and maybe_pulse) and in-place mask-hit
updates ((true,true), incl. the mask-miss reAdd arm and the D-072
shared-LIA gate), LIAs then trie nodes in build order; pass B: alpha
EXITS ((true,false)). note_link_effects runs after every node event
in both passes, so the WITHIN-UPDATE transient all-linked window now
exists exactly as in Drools (entries ride the OTN sink walk; exits
defer to the ModifyPreviousTuples end-drain) — a transiently-linked
path creates+queues its item (fz_7_2122 refined), and the D-091
dirty-item pop drains held staging into memories mid-fire. Same-node
out-and-back signatures (D-081/D-083) are cross-EVENT and untouched;
per-node staged ins+del from ONE update is impossible (transitions
are exclusive per node), so no new fold interactions.

**Gate (WITNESSED, Bryan's bar: pair flips + ZERO regressions):**
`make test` green; fz_min_2256 + fz_999_2256 FLIP GREEN and graduate
to regressions (4x stability incl. the campaign draw); corpus
**812/812** (11 baseline + 520 probes + 281 regressions) with zero
previously-green perturbations; fuzz seeds 42/7/123/777/999 x 10,000
= **50,000 cases, ZERO divergences**. xfail 80 -> 78 = 68 D-080 TMS
envelope + D-042 trio (3) + the D-093 expected-divergence set (7:
8426 pair + alu6a + alu7a/7d/7f/7g). The D-075/D-090 quarantine
backlog is now FULLY resolved: 455/4816 (D-091 port), 8426 (D-093
ruling + upstream #2366), 2256 (this port), 6812/3959/5014/9976
(earlier waves). Every non-TMS, non-D-042 latent found since P1b has
been mechanism-pinned rather than fenced.
**HANDOFF @ D-094 close (2026-07-07)** — The quarantine-cracking arc
is complete: both D-090 families resolved by mechanism (D-092/D-093
ruling for 8426 with upstream issue #2366; D-094 port for 2256).
State: corpus 812/812, 50k fuzz clean, xfail 78 (68 TMS envelope +
D-042 trio + 7 D-093 expected-divergence witnesses). Tooling asset:
oracle/…/AccDump.java — the generic ground-truth graft (join/acc
memories, staged sets, RTN masks + item state, per-WM-event and
per-firing dumps, epoch replay). Open, trigger-gated: D-042 trio
(value-bearing variant or new mechanism evidence), D-080 TMS
envelope (fence stands), upstream #2366 (revisit the D-093
divergence set if Drools fixes it — convergence would let the alu
witnesses graduate).

## Data-type semantics scoping (2026-07-07)

### D-095: THIRD DOCTRINE AXIS — ecosystem-facing data-type semantics
### conform to the COLUMNAR DATA ECOSYSTEM (Arrow/DuckDB/pandas), not
### Drools/Java; oracle-selection principle recorded (Bryan's ruling;
### ROADMAP scoping only — nothing built now)

The faithfulness doctrine now has three axes:
1. ENGINE/RULE semantics -> Drools is the spec (reproduce, quirks
   included — the original charter).
2. Value-bearing DEFECTS where Drools is self-inconsistent ->
   correct, document, report upstream (D-093).
3. ECOSYSTEM-FACING DATA-TYPE semantics (nulls, exact decimals) ->
   the columnar data ecosystem is authoritative — Arrow / DuckDB /
   pandas — NOT Drools/Java. Seine's facts originate there (Arrow
   ingestion, D-044) and its audience expects those semantics; Java
   accidents (null-as-missing-reference, IEEE-754 floats for money)
   are not the spec.

**Nulls (ROADMAP-P2, re-scoped from D-063):** implement SQL
three-valued logic — null = UNKNOWN, propagating through comparisons
and boolean logic per SQL 3VL: `NULL = NULL -> NULL`,
`NULL > 5 -> NULL`, `NULL AND false -> false`,
`NULL AND true -> NULL`. Ingestion normalizes Arrow-null /
pandas-NA/NaN / DuckDB-NULL to one proper null. This is a DELIBERATE
DEVIATION from Drools (whose null behavior is Java reference
semantics per-operator); the D-063 per-operator probe-matrix plan
stands but its authority target changes.

**Exact decimals (ROADMAP-P2, raised from D-064's P4-hard):** a
native exact-decimal fact type, Arrow Decimal128/256-compatible,
with EXACT arithmetic — no IEEE-754 float path for money, ever.
Load-bearing for the financial-decisioning soundness thesis. The
D-064 storage note stands (scaled fixed-point over i128, the
DECIMAL(p,s) approach); the Java BigDecimal coercion-matrix concern
dissolves — we conform to Arrow/SQL decimal semantics instead.
Deliberate deviation from Drools/Java.

**Oracle-selection principle (banked):** the right oracle by
concern — Drools 9.44.0.Final for engine/rule semantics; DuckDB as
the authoritative implementation of SQL 3VL + DECIMAL for data-type
semantics (since these features deliberately diverge from Drools,
differential-testing them against Drools would be testing against
the wrong spec). The harness grows a second oracle when these land;
scenario schema will need per-feature oracle routing.

Nothing implemented in this entry — FEATURES rows updated; whoever
builds P2 nulls/decimals conforms to the ecosystem, not Drools.

## Data-types arc — Phase 0 (2026-07-07)

### D-096: DuckDB oracle STOOD UP + the 3VL/DECIMAL semantics PINNED;
### design checkpoint OPEN (pre-implementation — Bryan review gate)
- Oracle pinned: **duckdb 1.5.4 + pyarrow 24.0.0** in the repo venv
  (.venv — first project venv; PEP-668 blocks system pip). The pin
  ritual mirrors Drools-9.44: tools/pin_duckdb.py GENERATES the
  ground-truth tables (docs/duckdb-datatype-pins.md); regenerate +
  diff on any version bump.
- Measured pins (headlines; full tables in the doc): comparison ops
  with any NULL operand → NULL, with IS [NOT] DISTINCT FROM as the
  definite forms; full 3VL AND/OR/NOT tables (NULL AND FALSE = FALSE
  — no naive short-circuit); the `not in` null trap reproduces
  (`1 NOT IN (2, NULL)` → NULL → excluded); WHERE admits only TRUE
  and excludes UNKNOWN from test AND negation; string ops with null
  → NULL; **null keys never equi-join**; aggregates SKIP nulls
  (count(x)=0 / sum=avg=min=max=NULL over all-null AND empty);
  GROUP BY/DISTINCT collapse nulls into ONE group (the TMS
  value-equality-key answer); NaN is a VALUE in DuckDB (NaN=NaN
  TRUE, sorts greatest) — the measured rationale for boundary
  NaN→NULL normalization on nullable float fields. DECIMAL: literals
  type by shape (typeof(1.23)=DECIMAL(3,2)); cross-scale equality is
  value-based; + grows precision by 1 at max-scale, * adds scales;
  scale-reduction rounds HALF-UP incl. negatives (1.005→1.01,
  -1.005→-1.01 — NOT banker's); downcast overflow ERRORS loudly;
  SUM(DECIMAL(p,s))→DECIMAL(38,s) exact; **AVG(decimal)→DOUBLE**
  (matches the certified average→f64); MIN/MAX preserve type;
  decimal=double compares value-wise (hazard flagged; proposal:
  WALL decimal-vs-f64 in Seine). Arrow round-trip verified
  (decimal128(p,s) ↔ DECIMAL(p,s); validity-null ↔ SQL NULL; float
  NaN arrives as a value).
- Design checkpoint: docs/design-datatypes.md — per-field OPT-IN
  nullability (certified surface untouched), decimal-as-string JSON,
  i128 scaled fixed-point storage, 3VL evaluation with WHERE-TRUE
  admission, `field == null` ⇒ IS NULL surface mapping, null-skipping
  aggregates with the ONE flagged axis conflict (sum(empty): Drools
  0 vs SQL NULL — ruling requested), DuckDB-oracle scope =
  match-sets + aggregates over insert-only scenarios (chaining stays
  Drools-axis), oracle routing via scenario "oracle" key, phased
  landing plan. Five open questions listed for Bryan. NO ENGINE
  CHANGES in this commit.

### D-097: design-checkpoint rulings (Bryan) — the data-types arc is GO
1. `field == null` / `field != null` parse as IS NULL / IS NOT NULL
   (definite two-valued tests; Drools' surface, SQL's semantics) —
   APPROVED.
2. **sum(empty/all-null) = 0, and it FIRES** — the Drools-certified
   engine-axis behavior WINS over SQL's NULL for the accumulate
   RESULT; null CONTRIBUTIONS are still skipped per the pins. This is
   the arc's ONE deliberate deviation from the DuckDB oracle, and the
   duckdb comparator must special-case it (sum over an empty/all-null
   group: engine 0 vs SQL NULL — mapped as equivalent). avg/min/max
   need no special case: SQL NULL result == Drools no-propagate ==
   engine no-fire. DOCUMENTED here per Bryan's instruction.
3. Per-field OPT-IN nullability (`"nullable": true`; default
   non-nullable keeps D-044 loud rejection) — APPROVED.
4. **Decimal-vs-f64 comparison: WALLED — compile error** (stricter
   than DuckDB's cast-to-double, pin J documents the un-walled
   semantics). Money never meets floats in Seine; the wall IS the
   thesis. decimal-vs-i64 stays (exact).
5. DuckDB-oracle scope = match sets + aggregate results over
   insert-only scenarios; chaining/agenda/mutation stay
   Drools-certified — APPROVED.

### D-097 phase 1 LANDED: nulls in the engine (SQL 3VL, pin-conformant)
Store: Value::Null + per-nullable-column validity bitmaps (Arrow
model); TypeSchema.nullable bitmask (opt-in); store push/set is the
single nullability gate (loud error for non-nullable — the D-044
posture). Parser: `null` literal (cmp rhs ==/!= only, in-list
members, RHS args). Compile: surface `== null`/`!= null` ->
Test::IsNull/GExpr::IsNull (definite); null in-list members ->
constant-UNKNOWN leaves (Test::Unknown/GExpr::Unknown — `not in`
trap exact); null insert/setter literals need nullable targets;
null-through-binding into non-nullable = loud runtime error.
Evaluation: eval_gexpr is TRI-STATE (Option<bool>, admission =
Some(true)) — the load-bearing case is !(...) over UNKNOWN staying
UNKNOWN; top-level conjunctions keep bool leaves (UNKNOWN==reject
coincide); eval_cmp's None-ord arm makes Null-vs-anything false at
every leaf incl. range scans (null probe/stored never match);
keys_match: null key components never equi-join, INCLUDING
null-null (pin F). KeyVal::Null: TMS value-equality keys collapse
nulls (pin H). Accumulate folds skip null contributions (sum/avg/
min/max — avg skips BOTH sum and count; a null can't become the
first extremum; try_reverse of a skipped null does NOT trigger the
min/max refold); count()/collect unaffected; all-null sum = 0 and
fires (ruling 2). Walls: queries over nullable types; salience over
nullable fields; non-eq ops vs null.
**Gate:** engine/tests/d097_nulls.rs — 8 conformance tests generated
from pins A–G (WHERE-TRUE + negation exclusion, IS NULL surface vs
3VL join, connective tables incl. NULL-AND-FALSE=FALSE via negated
groups, in/not-in traps, null string ops, eq-hash null-key join,
aggregate skips + ruling-2 sum) — 8/8. make test 7 suites green.
**make diff 812/812 byte-identical** — the certified Drools corpus
is untouched by the core 3VL changes (the opt-in design holds).

### D-097 phase 2 LANDED: the DuckDB differential runner + first
### null corpus — 8/8
tools/diff_duckdb.py (venv; asserts the 1.5.4 pin) translates
oracle:"duckdb" scenarios: types -> tables (nullable columns), facts
-> rows with idx = visible insertion order, each rule LHS -> SQL
(constraints map to the direct SQL operators — which IS the 3VL
authority; surface null tests -> IS [NOT] NULL; in-lists verbatim
incl. null members; matches/contains -> regexp_full_match/contains
with proper single-quoting; not/exists -> [NOT] EXISTS; accumulate
-> correlated scalar subquery with sum/count COALESCE(...,0) per
ruling 2 and avg/min/max gated IS NOT NULL = the no-propagate
equivalence). Comparator: order-INSENSITIVE per-rule multisets of
(visible-handle tuples + acc values); engine side runs with
SEINE_HANDLES=1 and maps __h through result.facts (synthetics like
InitialFact carry handles and are skipped). `make diff-duckdb`.
scenarios/duckdb/: 8 hand probes (cmp+negation 3VL, connective
tables, in/not-in traps, null-key joins incl. eq-hash, null string
ops, not/exists over null keys, aggregates with null skips, all-null
sum-0/min-no-propagate) — **8/8 PASS on first full contact** after
two comparator fixes (SQL string quoting; handle mapping). Phase 3
(null-rich generator + duckdb fuzz gate) and phase 4 (decimals) next.

### D-097 phase 3 LANDED: null-rich DuckDB fuzz — the 3VL surface is
### differentially witnessed
tools/fuzz_duckdb.py: python-side generator (gen.rs untouched per the
design) drawing insert-only, inert-RHS scenarios over the phase-1
surface — 2-3 types, ~55% nullable fields at 30% null density, cmp
across all ops, surface null tests, in/not-in with null members,
composite groups (incl. negation), matches/contains, same-type-only
eq/relational joins through possibly-null bindings, not/exists,
accumulate sum/count/average/min/max over nullable args. Engine-axis
exclusions documented in the header (epochs/actions/TMS/queries/
or-rules/salience; i64-vs-f64 eq joins per D-020; f64 values are
multiples of 0.25 so float sums are exact under any addition order;
bools ==/!= only; translator paren-depth limits).
**Gate: seeds 11/22/33 x 2000 = 6,000 generated cases, ZERO
divergences, ZERO generator rejects** (+ the 60-case shakedown,
seed 1). Every case is a fresh differential of the engine's 3VL
implementation against DuckDB 1.5.4 match sets. Phases remaining:
4 decimals, 5 bindings/Arrow boundary, 6 FEATURES promotion.

### D-098: authoring surface RATIFIED (typing module) — designed
### BEFORE phase 4 so engine and surface stay consistent
`Optional[X]`/`X | None` -> nullable bitmask;
`Annotated[Decimal, seine.Decimal(p, s)]` -> decimal(p,s) fields
(get_type_hints(include_extras=True) introspection). Six points in
docs/design-datatypes.md §6 — emphatic: bare `Decimal` is a LOUD
CompileError naming the fix (never defaulted precision), and the
Optional/NaN distinction is legible API semantics (the type
declaration IS the NaN-vs-NULL choice, docstringed as designed).
Marker validation (1<=p<=38, 0<=s<=p) must equal the engine's i128
limits. PEP-563 latent bug noted: 0.2.0's raw __annotations__ read
breaks under `from __future__ import annotations` even for int/str
fields — the get_type_hints move lands in phase 5 as a fix
regardless. Phase 4 (engine decimals) proceeds toward this target.

### D-098 phase 4 LANDED: exact decimals in the engine — pin-J
### conformant, DuckDB-differential witnessed
Types: FieldType::Dec{p,s} (1<=p<=38, Arrow Decimal128-compatible);
Value::Dec{u: i128, s} self-carrying; ColData::Dec per-row (u,s)
(user fields pre-normalized to field scale by coerce; acc results
store exact computed scale). Helpers (store.rs): dec_cmp — exact
cross-scale compare with the overflow-decides-by-sign trick (no
256-bit arithmetic: if the scale-aligned side overflows i128 it
strictly exceeds the other, so its sign is the answer); dec_parse
(exact strings only), dec_rescale (exact widening, HALF-UP narrowing
per pin J), dec_fits (declared precision), dec_render, dec_normalize
(trailing-zero strip — KeyVal::D TMS identity, 1.10 == 1.1).
Ingestion: strings/integers only — IEEE floats REJECTED (coerce
wall); half-up to field scale; loud precision-overflow errors.
Literals: written decimals (lexed f64) recover EXACTLY via shortest
round-trip repr (exact for <= 15 significant digits); conversion at
every compile site (cmp, groups, in-lists, RHS insert/setter args).
**The D-097-4 wall**: decimal-vs-f64 comparison is a COMPILE error
naming itself; f64 never converts to decimal anywhere. Eval: dec
arms in eval_cmp (Dec-Dec, Dec-I64 exact), keys_match (cross-scale
value-equal join keys), value_ord (range scans), min/max fold.
Aggregates: sum exact over i128 with scale-aligning folds and LOUD
overflow (DECIMAL(38) posture, pin J), result widens to
DECIMAL(38,s) via the new ACC_DECIMAL ("Decimal") hidden type;
average -> f64 (pin J: AVG is DOUBLE — the one deliberate
decimal-to-float edge); min/max preserve the decimal; ruling-2
composition: empty/all-null decimal sum = 0 AT THE FIELD'S SCALE and
fires. Eq-hash exclusion: decimal Eq literals are chain members
only, never eq-hash group members (cross-scale value equality vs
representation hashing — plain alpha eval is exact; deliberate,
documented). Walls: queries over decimal types (with nullable, one
wall family); salience rejects Dec via the numeric check.
**Gates:** engine/tests/d098_decimals.rs 6/6 first run (exact
comparisons incl. the 0.1+0.2 class, cross-scale join equality,
half-up rounding incl. negatives, precision overflow + float
rejection, the wall's compile error, aggregate matrix, in-lists +
RHS round-trip). make test 8 suites green; corpus 812/812 untouched.
DuckDB corpus 11/11 (3 new decimal probes). Fuzz: generator draws
decimal(p,s) fields p 8-12, s 0-3 (values at field scale;
family-matched joins so decimals cross scales but never meet f64)
EOF
echo prepped— **seeds 44/55/66 x 2000 = 6,000 decimal+null cases, ZERO
divergences, zero rejects** (+ 60-case shakedown). Phase 5 (Arrow/
typing boundary incl. the ratified D-098 surface + the PEP-563 fix)
and phase 6 (FEATURES promotion) remain.

### D-098 phases 5+6 LANDED: the ratified typing surface + the
### Arrow/row boundary; FEATURES promoted — THE DATA-TYPES ARC IS
### COMPLETE
Phase 5 (bindings): @seine.fact now introspects via
get_type_hints(include_extras=True) — fixing the shipped 0.2.0
PEP-563 latent bug (stringized annotations broke even int/str
fields) — and implements the six ratified §6 points: Optional[X]/
X|None -> "t?" nullable; Annotated[Decimal, seine.Decimal(p,s)] ->
"decimal(p,s)" with construction-time validation matching the
engine's i128 limits; bare Decimal = loud CompileError naming the
fix; nesting normalizes (Optional/Annotated at any level); the
NaN-vs-NULL choice IS the type declaration (Optional[float] ingests
NaN as NULL; bare float keeps bit-exact NaN — D-044 preserved),
docstringed as designed. Rust boundary: schema strings carry
nullable/decimal; ingestion is DECLARED-SCHEMA-AWARE (validity ->
Null only for nullable fields, NaN -> Null only for nullable floats,
decimal128 columns rescale to the declared (p,s) with loud
overflow); py rows: None/decimal.Decimal/int accepted per target,
floats walled from decimals; results export nullable Arrow columns
and Decimal128 arrays (polars round-trip: Decimal dtype, exact
strings, null_count). Session/run gain schemas= passthrough;
Engine::fact_type_name added for typed updates.
**Gates:** bindings 60/60 (48 pre-existing untouched + 12 new
boundary tests incl. the PEP-563 regression under `from __future__
import annotations`); engine 8 suites; corpus 812/812; duckdb 11/11.
Phase 6: FEATURES rows promoted to §1 with the D-095 authority
noted. Remaining liftable walls recorded (queries/salience over
nullable+decimal types). Interleaved finding, same session: Bryan's
insertLogical parse error was the PUBLISHED 0.2.0 wheel predating
TMS (b94f11b not an ancestor of v0.2.0; 35 commits behind) — his
exact rule (Person / not Blocker / insertLogical) runs correctly on
main incl. TMS auto-retraction; local maturin builds now carry
everything; v0.3.0 is Bryan's release call.

## CEP investigation (2026-07-08)

### D-099: CEP-as-TMS INVESTIGATION COMPLETE (memo-first per D-079;
### no implementation) — the framing holds semantically, fails
### mechanically, and BOTH halves shape the port
Full memo: docs/memo-cep-as-tms.md. Source-pinned findings
(drools-core 9.44): (1) expiration NEVER touches Drools' TMS — both
@expires (ObjectTypeNode.ExpireJob) and sliding windows
(SlidingTimeWindow.expireFacts) call doRetractObject, i.e. the
ordinary retraction path — so the faithful port is a DEADLINE-ORDERED
RETRACTION SCHEDULER over our certified delete cascade, with the TMS
connection arriving free when events justify insertLogical chains;
(2) WindowNode CLONES the event handle (cloneAndLink) and expires
the CLONE — window expiry is per-window-subtree unmatch while
@expires is WM-wide retract (the fact-survives-other-rules
observable separates them differentially); (3) pseudo-clock
advanceTime pops due jobs in fire-time order and SETS THE CLOCK TO
EACH TRIGGER'S OWN TIME before executing — mid-advance states are
spec; (4) **equal fire-time ties are UNSPECIFIED** (compareTo is
fire-Date-only into a java PriorityQueue heap) — a D-035-class
surface, probe-then-pin-or-fence; (5) temporal operators are a
closed interval-test family that COLLAPSES to delta-range checks for
point events — specialized Test variants, NO general constraint
arithmetic needed, D-032-indexable; (6) @timestamp-from-field kills
all wall-clock dependence — the deterministic subset requires it.
Oracle: STREAM + PSEUDO clock + advance_ms epoch actions — fully
scriptable, deterministic modulo (4). Thesis fit is strong
(delinquency buckets ARE window:time; payment sequencing IS
point-event temporal joins). RECOMMENDATION: promote to a P2 arc,
E0 = supervised probe-ladder recon (tie order, mid-advance agenda
composition, window-clone scope, inferred-expiration rules,
expiration x TMS cascade, expiration x D-076 defer-drain), then
E1 point events + @expires + after/before, E2 windows, E3 the rest.
Fences: no wall clock / fireUntilHalt / entry points / @duration /
rule timers; distinct expiry instants pending the tie probe.

### D-100: CEP E0 RECON COMPLETE — six-rung probe ladder, zero
### contradictions with the D-099 model; the two open risks resolve
### FAVORABLY (Bryan review gate before E1)
Oracle plumbing landed (OracleRunner): type-level event metadata
{"event": {"timestamp": f, "expires_ms": N}} -> @role/@timestamp/
@expires declare annotations; STREAM + pseudo-clock session ONLY
when a scenario declares events (certified path untouched — full
`make diff` re-verified green); epoch action {"op":"advance",
"ms":N}. Probe results (probes_pending/cep/, all first-pass):
- a1 @expires: WM-wide retract at the advance boundary; not-CE
  observes; final WM drops the event. The basic machine works
  end-to-end.
- a2 TIE ORDER: two same-instant expirations retract in a STABLE
  order — **10/10 fresh JVMs byte-identical** (insertion-shaped heap
  order for this class). Posture: PIN the arrival-order behavior,
  fuzz-gate larger tie batches (the D-083 lesson) rather than fence.
- a3 MID-ADVANCE: **expirations batch** — two due retractions
  (t=100, t=200) under one advance(300) propagate as ONE batch at
  the epoch's fire with NO intermediate agenda evaluation (Rmid
  never fired). The timer level rolls the clock per job, but the
  RETE/agenda sees a single composed batch -> our port is
  D-047-SHAPED (deadline-ordered retraction batch + one evaluate),
  simpler than the memo's worst case.
- a4 WINDOW-CLONE SCOPE confirmed: the window's accumulate REFIRED
  when the clone expired (count 1 -> 0) while the FACT stayed in the
  WM (@expires far away) — per-subtree unmatch vs WM-wide retract,
  exactly as read from cloneAndLink.
- a5/a5b INFERRED EXPIRATION confirmed and DIRECTIONAL: with no
  @expires anywhere, the after[0,100ms] anchor side (E1) expired at
  ~its reach and vanished from the final WM; the probing side (E2)
  persisted. Control (in-window arrival) fires. Exact inference
  rules = an E1-phase ladder.
- a6 EXPIRATION x TMS: an expired event's logical dependent
  auto-retracted through the certified D-076 cascade (J -> RD ->
  expiry -> ND; final WM has neither E nor D). The memo's "TMS for
  free at one remove" claim is now witnessed.
E1 (point events + @expires + after/before + the deadline queue +
generator/fuzz) awaits Bryan's go. Defer-drain composition (a7) is
queued as an E1-ladder rung alongside the inference-rule probes.

## CEP E1 (2026-07-08)

### D-101: E1 IN PROGRESS — defer-drain pinned and PORTED (a7 trio +
### quiescence pool); clock/deadline-queue/advance + after/before
### landed; temporal scan order pinned same-batch; ONE OPEN FORK
### (t6: held-staging x temporal composition) — Bryan checkpoint
**Defer-drain (front-loaded per Bryan):** a7 (cascade depth), a7b
(strictly-higher interleave), a7c (cascade vs same-epoch chain), and
the DECISIVE a7d delete-twin: Drools drains EXPIRATION-sourced TMS
cascades at AGENDA QUIESCENCE, but delete-sourced ones through the
certified gate (a7d matches both engines untouched — the certified
TMS machinery is correct; the behavior is expiration-specific).
Port: `tms.expiring` marks (set in advance()) route act
invalidations from BOTH trigger paths (lazy terminal hook + k=1
eager-break scan) into `tms.expire_deferred`, drained at the agenda
quiescence point in next_activation (clear-marks-BEFORE-drain: the
first cut live-locked by re-deferring through its own entry check).
Chained cascades (D->D2) complete at the drain (a7's full-depth
observation). a1/a2/a3/a4/a6/a7/a7b/a7c/a7d = 9 rungs; 8 promoted
(a4 windows = E2); ties stable 10/10 (pinned, arrival order).
**Engine machinery:** clock_ms + BTreeMap deadline queue +
declare_event (explicit expires_ms REQUIRED — a8 pinned explicit
@expires OVERRIDING inferred reach, no max-merge; inference = E2);
advance() = deadline-ordered batch of external deletes (a3
composition); scheduling on external AND RHS inserts; harness event
metadata + advance ops; `[`/`]` lexed (the temporal syntax was
unexercised until fuzz); `this after/before[lo,hi] $a` ->
Test::Temporal (beta; positive-CE only; queries walled).
**Temporal join order (t-ladder):** same-batch semantics PINNED:
temporal nodes flip the insert composition (leftIns FILLS joining
only pre-batch right memory, THEN rightIns joins full left memory)
and scans iterate partners ASCENDING BY TIMESTAMP (creation order;
firing order is the certified prepend-reverse). Implemented via
Node::temporal + ts keys through key_of_left/right (anchor ts /
own ts). min_sj + t1-t5 differentially GREEN (promoted, corpus
826/826).
**THE OPEN FORK (t6):** held probers (never-linked fire-1 staging)
joined by a fresh anchor in fire 2 — engine fires (50,100),(50,150),
oracle (50,150),(50,100). Held-staging x temporal-scan composition
(the D-084/D-091/D-094 lineage recombining with the new scan order);
hand-models contradict across the same-batch and held cases — the
D-083 stop signal. Needs its own focused sub-ladder (held anchors vs
held probers, ties, multi-fire interleaves, STREAM-mode propagation
timing) before the CEP fuzz gate can run. t6 stays in
probes_pending/cep/. Gates at this commit: 8 suites, corpus 826/826,
zero blast radius (temporal branch is flag-gated; plain joins
byte-identical).

### D-101 (continued): the t6 sub-ladder CRACKED — three mechanisms
### pinned and ported, temporal ladder 15/15, corpus 834/834; ONE new
### class open (u-ladder: STREAM-mode plain-node composition)
**Method:** hand-models contradicted across shapes (the D-083
signal), so tools/model_check_temporal.py enumerated the composition
space against ALL twelve pins — zero survivors twice isolated the
missing dimensions; the third run produced ONE six-member survivor
family (degenerate residue only).
**Mechanism 1 — drain-at-link (the D-094 memory-fill lineage):**
rights staged while a temporal node's path is UNLINKED drain into
right MEMORY in ARRIVAL order at the link moment — INCLUDING
same-batch pre-anchor rights (t14's mid-batch link) but EXCLUDING
the link-TRIGGERING fact itself (t1/t15: a prober that completes the
path stays staged). Port: note_link_effects_ex threads the current
WM event's fact; Node::drain_staged_rights_to_memory.
**Mechanism 2 — the temporal walk composition:** staged lefts ALL
fill first (no joins); staged rights process head-first (newest)
joining lefts in ARRIVAL (lseq) order; then staged lefts join the
PRE-BATCH right memory (incl. link drains) in memory order. The
earlier ts-ASC model was a coincidence-fit (every early probe drew
timestamps increasing with arrival; cf56's inverted draw broke it).
**Mechanism 3 — expiration teardown is LAZY on the CERTIFIED path:**
the quiescence-pool model (previous entry) was WRONG mechanism,
right observables: a7c's "quiescence" was just the justifier's
salience-0 item popping LAST, and fuzz case cf5x0's salience TIE
(J2 decl-before-NE5 -> cascade drains first) proved the drain rides
the EXISTING tms.deferred item-pop machinery — expiring-marked acts
now push onto tms.deferred (lazy) while external deletes keep the
certified EAGER k=1 teardown (a7d). The expire_deferred pool is
DELETED. Corollary pin (cf5x17): after a popped item drains
deferred dels, it COMMITS to firing its own head activation — the
post-pop preemption re-check applies ONLY to dyn-salience items
(Drools' executor keeps control through the current fire; the
static re-check let a mid-pop-activated higher rule preempt ✗).
**State:** temporal ladder 15/15 (t1-t15 + min_sj + cf56); a-ladder
9/9 stays green; corpus 834/834 (8 t-rungs promoted); 8 suites; all
certified paths byte-identical. **OPEN (u-ladder):** shakedown case
cf5x18 (saved as probes_pending/cep/cep_u1_stream_exists_relink)
diverges on a rule with NO temporal constraint — `exists E0() P()`
re-linking after total expiration orders P-side pairs
ARRIVAL-first in Drools vs fresh-first certified — STREAM-mode
staging semantics for event-typed facts differ from CLOUD at PLAIN
nodes too. The E1 fuzz gate stays blocked pending the u-ladder
(exists/not/plain-join x event re-link shapes).

### D-101 (u-ladder recon): STREAM-mode composition scope BOUNDED;
### per-RHS-insert windows PINNED; the not/exists relink walk
### asymmetry OPEN (next model-check cycle)
Oracle pins (probes_pending/cep/cep_u*): **u2/u2b** — a plain-plain
join (no event types in the rule) orders IDENTICALLY in event
(STREAM) and no-event (CLOUD) sessions: the stream composition
changes are CONFINED to event-fed/CE-relink shapes; the certified
corpus classes cannot perturb (bounding result). **u4** — RHS
inserts in a STREAM session flush PER-INSERT: a consumer fires two
same-RHS inserts in ARRIVAL order (certified CLOUD = LIFO batch) —
the D-047 window machinery applies per RHS insert in event sessions
(shouldFlush = isStreamMode() in assertObject, the D-084-era source
read). **u3 vs u1/cf5x18 (OPEN)** — the P-side pair order after a
CE relink SPLITS by CE kind: a NOT-relink (expiration-triggered)
orders (IF,P2),(IF,P1) = the certified cloud walk; an EXISTS-relink
(insert-triggered) orders (IF,P1),(IF,P2) = the temporal walk shape.
Two hand-model rounds contradicted (the D-083 signal) — the next
cycle extends tools/model_check_temporal.py with CE-kind and
link-trigger dimensions plus 4-6 discriminating probes (held-side
swaps, insert-vs-advance triggers per CE), then ports, then the E1
fuzz gate. No engine changes in this commit; the E1 gate stays
blocked pending the asymmetry.

## CEP E1 Arc-0 kickoff (2026-07-07, plan pure-pondering-seahorse)

### D-102 (recon): the u-ladder asymmetry DISSOLVED — not/exists was
### never the variable; the mechanism is PER-INSERT FLUSH WINDOWS in
### event sessions (unifying cf5x18 with u4)
The v-probe batch (probes_pending/cep/cep_v2..v5) triangulated the
three-way confound (CE kind x relink-trigger kind x P1 location):
- v2 {exists, insert-relink, P1 HELD} -> fresh-first (certified)
- v3 {not, advance-relink, P1 IN MEMORY} -> fresh-first (certified)
- u3 {not, advance, held} -> fresh-first (certified)
- cf5x18 {exists, insert-relink, P1 IN MEMORY} -> P1-FIRST (deviant)
Only the {insert-relink AND memory-resident partner} cell deviates —
CE kind is INERT. The mechanism (source-anchored:
`shouldFlush = isStreamMode()` in BetaNode.assertObject): in STREAM
sessions every INSERT force-flushes the path — the relink-triggering
E0 insert evaluates the network in its own MINI-WINDOW, pairing the
re-entered IF left with MEMORY rights (P1) immediately; later
same-epoch inserts (P2) flush into their own windows; the rule queue
composes ACTION-ORDERED across windows (the D-047 shape). v2 shows
no deviation because P1 was still STAGED at flush time (a flush
joins MEMORY, not staging); advance-triggered relinks never flush
(a3: expiration retractions queue to the epoch's fire). This is the
SAME mechanism as u4 (per-RHS-insert windows) — one port covers
both. v4 pins two-held-generation arrival order; v5 pins the
mixed-location sequence (fresh, held, memory) for the not side.
**Port shape (next):** in event sessions (!event_specs.is_empty()),
every insert (external, epoch-fact, RHS) closes its window AND
immediately evaluates affected linked rules' networks (activation
queueing only, no agenda pop — forceFlushLeftTuple semantics),
riding the existing D-047 s0_close_window + evaluate_rule
machinery. Extend tools/model_check_temporal.py with the flush
dimension + not/exists node semantics BEFORE porting (the u3
hand-model of our own engine came out wrong — the checker is the
arbiter). Gate: v-probes + full u/t/a ladders + corpus + fuzz_cep.

### D-102 addendum: naive-flush variant results = PIN DATA for the
### checker; the port needs TRIGGER-SCOPED flush propagation
A first-cut stream_flush (whole-network evaluation after EVERY
insert in event sessions — external, RHS, insertLogical) was built
and differentially measured, then REVERTED (working tree back to
0778e80's engine). Results (all valuable pins for the model-check
cycle):
- FIXED: cf5x18/u1 (the seed), u4 (per-RHS windows) — the flush
  family is the right mechanism.
- KEPT GREEN: u3, v3, v4, min_sj, t1, t6, t7, t14, a1, a6.
- STILL WRONG: v2 (the flush drained the HELD P1 into the relink
  window — Drools' forceFlushLeftTuple propagates ONLY the
  triggering insert's own staging, leaving the held backlog for the
  epoch fire; source: flushLeftTupleIfNecessary passes
  createLeftTupleTupleSets(leftTuple=null) = EMPTY sets), and v5
  (mixed locations — order needs the trigger-scoped model plus
  possibly plain-node drain-at-link; hand-models contradicted
  between v4 and v5 — the D-083 stop).
- REGRESSED: a7c — the mid-RHS flush perturbed the lazy TMS
  deferred-drain composition (Rhi fired before Rcons again),
  meaning RHS-insert flushes must NOT re-evaluate the justifier's
  network ahead of its item pop — trigger-scoping likely fixes this
  too (the whole-network flush drained J's deferred state early).
**Next (the checker cycle, fresh context):** extend
tools/model_check_temporal.py with: not/exists relink semantics (IF
left re-entry/retract events at the downstream join), flush variants
{none, whole-network, TRIGGER-SCOPED (head-segment split of the
prepend-staged lists — the trigger's additions are the list heads)},
plain-node drain-at-link on/off, and the window/queue composition.
Enumerate against ALL pins: a-ladder (esp. a7c), t1–t15, u1/u3/u4,
v2–v5, cf5x0/17/18. Implement the survivor with the head-segment
staging split (snapshot staged lengths before on_insert; flush only
the delta; restore the withheld tail). Then: full ladders + corpus +
fuzz_cep shakedown → 3×1000 gate → D-101/D-102 close + FEATURES.

### D-102 (checker cycle close): the survivor family PORTED to 13/14
### on branch d102-flush-wip; ONE regression (a3) open — eval-boundary
### split of an expiration del pair
The model check (74b7bbd) survived as: trigger-scoped LEFT-flushing
stream flush (forceFlushLeftTuple semantics — held RIGHTS stay
staged; the trigger's own right delta + all left staging flush),
touch-scoped to the trigger's paths (a7c: untouched paths must not
process staged deletes early), plus plain-node drain-at-link at
NONFLUSH (advance-triggered) links only, alive-filtered (a3's dead
facts stay staged for del-annihilation). Implementation on branch
**d102-flush-wip** (main stays green at 74b7bbd): stream_flush_ex
with per-node right-tail stash/restore + requeue, prologue
per-insert flushes WITHOUT window closes (the initial batch is ONE
window — a3's batch pin), temporal self-drain replacing the old
drain-at-link, SEINE_FLUSH_DEBUG hooks.
**Green on the branch:** cf5x18/u1 (the seed), u3, u4, v2, v3, v4,
v5, min_sj, t1, t6, t7, t14, a1, a6, a7c, a7d — 13/14 of the
spot-check matrix (all previously-forked rungs now pass).
**OPEN (a3_mid_advance):** the two-expiration batch (E1@100, E2@200,
one advance(300)) regresses: Rmid fires transiently. Trace diff
(baseline vs branch): baseline processes E1's rightDel and E2's
leftDel in ONE Rmid evaluation (leftDel phase kills the parked E2
before the rightDel unblocks — no activation); the branch splits
them across TWO evaluations with a firing between (E1-rightDel
eval unblocks parked E2 -> activation -> fires -> E2-leftDel eval
prunes too late). Four hypotheses eliminated empirically: the
plain drain (gated off — still fails), prologue window closes
(removed — still fails), dead-fact drains (alive filter — still
fails), the requeue (debug shows it never fires on a3). The
remaining delta is HOW fire-2's evaluation windows split under the
branch — needs eval-boundary tracing (add an evaluation counter to
SEINE_TRACE) comparing baseline/branch step structure on a3.
Fresh-context task: instrument, isolate the split, fix, then the
FULL gate sequence (all ladders + corpus 834 + fuzz_cep shakedown
60 -> 3x1000) and the D-101/D-102 close.

### D-102 (a3 resolved; u2-class cycle queued): the a3 eval-split was
### the stash blinding existential BLOCKERS — fixed by kind-scoping;
### the u2-class (plain binding-joins in stream) is the next checker
### cycle with four fresh pins
The trigger-attributed eval trace (SEINE_EVAL_DEBUG, baseline vs
branch) adjudicated a3 in one pass: the branch's fire-1 FLUSH
evaluated Rmid with the held E1-ins STASHED — the not node was
artificially empty, E2 propagated, and Rmid fired at FIRE 1 (not
fire 2 as previously assumed). The advisor-prior (flush window
over-scoping at fire 2) was WRONG in location but right in kind:
another reduction, not new machinery. Fix on d102-flush-wip: the
flush stash takes rights at Kind::Join nodes ONLY — a held right at
a not/exists node is a BLOCKER whose visibility the flush walk must
keep (v2's join rights stay stashed; a3's blocker stays visible).
a3 + the full prior matrix green (30/31).
**Open (u2-class, 4 new pins):** the REWRITTEN u2 (the original had
a getter-syntax bug and never ran engine-side) + u2c (bare join) +
u2d (split epochs) pin: plain-join rights NEVER flush-pair and
stream==cloud left-major composition, including held-lefts shapes.
v2c pins the v2 fresh-right-first pattern surviving a join
constraint. CONFLICT: stashing delta rights at plain joins fixes
u2-class but breaks v2-class fire order; the fire-walk's
held-vs-fresh right generation order at plain nodes must differ
from the temporal t7 rule. Next sitting: model_check_stream cycle 3
— add dims {delta-right stash on/off, plain-fire right order
(head | arrival | held-arrival-first | fresh-head-first), drain
at all-links vs nonflush} against the full pin set (now ~20).
Branch state: d102-flush-wip carries everything incl. the
SEINE_EVAL_DEBUG instrumentation; main stays green at this commit.

### D-102 (audit + state correction): harness liveness lint LANDED
### (888/888); a stash-cycle engine clobber found and fixed; the TRUE
### branch state is 30/31 with ONE open pin (u2) — checker cycle 3
### adjudicates the measured seesaw
**Harness hardening (the u2 lesson, corrected):** the harness never
passed u2 silently — differentials fail loudly on one-side errors.
The REAL latent classes: (1) oracle-recon probes carrying
engine-invalid DRL unnoticed until drafted into a gate; (2)
green-because-inert differentials (both sides run, nothing fires).
tools/lint_probes.py (make lint-probes) guards both: every probe
must run engine-side AND produce firings or query rows;
deliberate-empty pins carry expect_inert (20 annotated — zero-firing
regression pins, blocked-not probes, qx-empty); WALLED recon probes
carry engine_fenced and the lint verifies they STAY REJECTED (the
ghosts inverted into standing fence-regression guards). Audit:
888/888 live/guarded; the CEP checker pin set verified clean.
**Ops lesson (twice bitten):** stash/checkout cycles during
baseline-instrumentation dances clobbered the branch engine (a
git add -A then committed a 202-line regression silently — caught
because the matrix seesawed impossibly). Standing rule: after any
stash dance, `git diff HEAD~1 --stat` before trusting a measurement;
better, take baseline traces via a WORKTREE not stash cycles.
**True branch state (engine restored, re-measured): 30/31.** a3 ✓
(kind-scoped stash — existential blockers stay visible), v2/v2c/
cf5x18 ✓ (delta-left flushes release the IF), u2c/u2d ✓ (bare and
split-epoch joins left-major). OPEN: u2 alone (same-batch
binding-join in an event session) — engine pairs held lefts at the
rights' flush evals; oracle composes left-major at the fire.
Symmetric join-side stashing (held lefts too) fixes u2 but breaks 7
others (measured 23/31) — the uniform rule can't serve both; the
advisor-predicted shape stands: node-kind-scoped (and possibly
side- and delta-scoped) stash/order tables, adjudicated by
model_check_stream cycle 3 against the audited pin set. The
recurring signature (seventh instance) is now a standing heuristic:
when a composition regresses, ask FIRST "what is this operation
treating as uniform that isn't?"

### D-102 (cycle 3 complete): the survivor PORTED — full 31-rung
### matrix GREEN, corpus untouched; one bounded fuzz class remains
### (the k=1 expiration-teardown leak, instrumented)
**Checker cycle 3** (tools/model_check_stream.py, rewritten run()):
after two model-bug rounds (the fire linked-gate lost in the
rewrite; IF-unlink maintenance hoisted above the gate) and one new
dimension (LINK-RELATIVE right generations — staged rights label
'pre' or 'post' relative to the path's link state at staging), the
enumeration produced a 4-member survivor family:
**{plain rights never flush (stay); held staging hidden at flushes;
plain fire order = pre-link-LIFO then post-link-ARRIVAL; temporal
dims degenerate; drain_t}** — the eighth node-kind table row.
**Port** (main): ph=4 stamps pre-link plain-join rights in event
sessions; the phreak plain rightIns walk splits pre-LIFO/post-arr
ONLY when ph=4 entries exist (the first cut's unconditional rev()
flipped the certified cloud walk — caught by the u2b CLOUD control
failing, exactly what controls are for); the flush stash hides ALL
plain-join rights + pre-tail lefts + pre-tail DELS at all node
kinds (expirations batch to the fire — the u3/v3/v5 trio's staged
expiration delete was walking at the next insert's flush and
prematurely unblocking the not; the trigger's OWN del effects, e.g.
a blocking insert's leftDel, are delta and still flush).
**State: the full 31-rung matrix GREEN** (a/t/u/v ladders, both
cloud controls, self-joins, cf-seeds); 8 suites; corpus 834/834
byte-identical; lint-probes clean.
**OPEN (bounded): the cf5x17-class shakedown residue** — a k=1
justifier's expiration teardown still processes EARLY through a
path the k1-window del-stash does not cover: SEINE_TMS_DEBUG shows
TWO tms_on_terminal_del(J1) calls, the first during the advance
(now rerouted to the lazy deferred list via the restored
expiring-check in tms_on_terminal_del — the direct queue-prune
callers bypassed the eager-break routing), the second during a
subsequent insert's flush evaluation with expiring already cleared;
the k1-stash debug never fires, so the staged delete reaches that
walk from a source OTHER than nets[ri].s0 window dels — locating
that source (likely the k=1 queue-prune or a trie-side path for
k=1 rules) is the next bounded step. Instrumentation in place:
SEINE_TMS_DEBUG, SEINE_EVAL_DEBUG, SEINE_FLUSH_DEBUG, k1-stash
prints. After it: shakedown to zero -> 3x1000 campaign ->
D-101/D-102 close + FEATURES promotion.

### D-102 (forensics + the expiration composition): FIVE mechanisms
### landed; three 60-shakedowns CLEAN; campaign launched
Forensic finding first: the cf5x17 k=1 "leak" was MY EDIT — the
k1-stash replace had silently no-oped (unconditional success print,
no assert). Re-applied WITH asserts: cf5x17 green immediately.
Edit-hygiene rule now standing: every scripted patch asserts its
anchors.
Then the fuzz peel (12 -> 9 -> 3 -> 1 -> 0 divergences across two
seeds) pinned FIVE mechanisms, each oracle-probed before porting:
1. **Expiration boundary is STRICTLY-AFTER** (b1/b2 pins): an event
   survives clock == ts+expires inclusive; deadline = ts+expires+1
   (Drools schedules the ExpireJob at offset+1). One-line fix.
2. **TMS expiration teardown timing** (q1/q2/q4 pins): an expiring
   justifier's teardown drains at the J-rule's POST-FIRING block
   (after the RHS — q2: re-justification keeps D continuous, no
   RD re-fire) or at agenda QUIESCENCE if the rule never fires
   (q1/q4: past even salience -5). NEVER at an empty pop, never at
   a flush. Implemented as tms.exp_deferred, SEPARATE from the
   certified D-076 deferred list (fz_7_3783 regressed when the
   quiescence drain touched D-076 entries — the certified cloud
   machinery restored verbatim).
3. **Expiration deletes propagate at QUIESCENCE** (cf5x33: a not-CE
   over an expired event stays BLOCKED through all salience-0 pops
   of the next fire). advance() only marks + queues
   pending_expirations; the quiescence step in next_activation
   processes the batch through the certified delete path, drains
   the freshly-routed teardowns IN THE SAME ROUND (cf11x24: both
   effect kinds materialize before the rescan; salience orders the
   observers), then rescans.
4. **The expired FLAG is EAGER** (cf11x55/8/19/37): a
   pending-expired event makes NO NEW join pairs (fresh walks skip
   flagged partners at plain+temporal joins — store.is_expired via
   a JoinEnv default) while its EXISTING network effects (not/
   exists blocking) persist until the lazy delete. Flag-eager,
   retraction-lazy — Drools' propagation-queue structure exactly.
5. **The plain-node link drain gates on the quiescence-delete
   phase** (cf11x11): with expiring's lifetime now spanning the
   epoch, the old !expiring.is_empty() gate misfired on
   insert-triggered links; in_expiration_drain flag replaces it.
State: 31-rung matrix + q/b probes green; suites 8; corpus 834/834;
lint clean; shakedowns seeds 5/11/23 = 0/0/0 divergences.

### D-102 (campaign): 3x1000 = 12 divergences -> temporal-stay fix
### -> 9 remain (0.3%); the two-rule discriminator found; fill-only
### overshoots — next cycle needs a flush-pairing pin ladder
Campaign seeds 101/202/303 (3000 scenarios): 12 divergences. The
kept cases produced a NEW discriminator class the 39-pin matrix
provably could not see: TWO same-body temporal rules at different
salience (cf101x616/cf101x134). Since single-rule pins cannot
distinguish flush-window pairing from one-batch-newest-first
composition, these shapes expose WHERE pairs are created:
- cf101x616 pinned **temporal delta rights do not flush-pair**
  (temp_dr=stay): the shared node's pairs are created at the FIRST
  reaching evaluation (the higher-salience rule's pop); the second
  rule's terminal receives them and appends CREATION-order at its
  own pop (certified D-027 lazy semantics). PORTED: the flush stash
  takes temporal-join rights on LINKED paths (unlinked deltas stay
  for the t6/t14 self-drain). Cleared 3 of 12; full gates green.
- cf101x134 shows the SAME two-rule reversal driven by temporal
  LEFT deltas pairing against memory rights at the flush. A pure
  fill-only flush (lefts fill, no children) cleared it but BROKE 6
  t/u-family pins (measured, reverted) — some flush pairing is
  real; the boundary between fill-only and pairing needs its own
  pin ladder (vary: left-vs-right delta, linked-ness at the flush,
  window structure, two-rule observers). NEXT CYCLE's shape.
Residual: 9/3000 (0.3%) — the temporal two-rule micro-order family
(kept under tmp/cepfuzz_101/202/303) + one observer-order case
(cf101x987). All counts/WM/composition classes are CLOSED; what
remains is pair-creation-site micro-order visible only through
shared-network two-rule shapes.
Also landed: 20 pins promoted to scenarios/probes (u/v/b/q + the
u2c/u2d/v2c discriminators, recreated after being lost to the
topology churn — never committed); corpus now 851; lint 910/910
incl. fence guards; model_check_stream main() dims fixed to
cycle-3 (the earlier dims edit was ANOTHER silent no-op replace —
assert-your-anchors is now doctrine, twice proven).

### D-102 (residual peel, sitting 2): TWO mechanisms landed (9 -> 5
### remain); the 551-vs-t14 link-flush contradiction is CYCLE 4
Landed, fully gated (matrix 39 + suites 8 + corpus 851):
1. **Linked-left temporal stash** (cf101x134): on an ALREADY-linked
   path (pre-insert linked, per a new snapshot flag), temporal
   left deltas stay staged at the flush — the pop walk fills and
   pairs them in one batch (creations [(1257,1257),(1257,1209)] =
   rightIns-then-leftIns phases). The LINK-TRANSITION flush keeps
   the certified fill+pair vs right memory (t6/t7/t10/t12/t13/t14
   — the fill-only measurement's breakage set, all late-anchor
   rungs, now explained: fill-only DROPPED pairs by splitting fill
   from pair across evaluations).
2. **Referenced-type expiration boundary** (cf202x364, probe b8 +
   b3-b7 ladder): the +1 (strictly-after) boundary belongs to the
   ObjectTypeNode path — an event type NO rule references has no
   OTN and expires at EXACTLY ts+expires. Engine: deadline = ts +
   exp + (referenced ? 1 : 0). The b-ladder (8 probes) promoted.
**OPEN — the cycle-4 contradiction (5 cases: 551/526/173 + 987 +
853/810/998 unclassified):** cf101x551 (TWO shared-body rules,
salience 13/0, same-fire link): the oracle does NOT pair at the
mid-batch link-transition flush — one pop batch, leftIns
HEAD-first, LAZY creation-order firings for both rules (TJ0's
creation order matches engine; TJ1's window split does not). But
t14 (ONE rule, same-fire link) REQUIRES flush-pairing with
reverse-creation consume. Discriminator candidates: rule-sharing
(identical bodies -> shared segment), salience-driven first-eval
site, prologue-vs-external flush. model_check_stream cycle 4:
add two-rule shared-node shapes, a flush-pair-at-link dim
{always, single-rule-only, external-only, never}, lazy-vs-eager
consume per eval site; pins 551/616/134 + t6/t7/t14/t15 + the
u/v regression guard set.

### D-102 (sitting 2 close): sharing suppression LANDED (9 -> 4);
### the temporal-walk micro-order table is CYCLE 4's input — six
### hand-model flips say enumerate, don't derive
**Landed and gated** (matrix 45, corpus 857, suites 8): a temporal
node SHARED by >1 rule never flush-pairs (cf101x551/173/998 —
force-flushing a shared segment would feed multiple rule paths out
of agenda order; the pop composes instead). Engine: node_shared
(path-membership count) forces the linked-left stash on shared
temporal nodes regardless of link transitions.
**OPEN (4 cases: cf101x987, cf202x526, cf202x853, cf303x810) + the
cycle-4 table.** 526's two-rule shape exposed the temporal walk's
MICRO-ORDER as 4 coupled dimensions the pins now constrain from
BOTH ends (measured creation/consume orders):
- 551: creations [(27,31),(7,26),(7,31)] — leftIns iterates
  ARRIVAL (27 before 7 despite prepend-head=7); each left x
  pre_rights NEWEST-first ([26,31] from memory [31,26]); sink0
  (decl-first rule) consumes FORWARD; peer consumes REVERSE.
- 526: creations [(38,80),(67,80)] — rightIns partner scan
  ARRIVAL-ASC (38 memory-gen before 67 fresh); sink0 FORWARD,
  peer REVERSE.
- t1 (single rule): rightIns partners must yield firings
  newest-FIRST — under sink0-FORWARD this needs partner scan
  DESC, contradicting 526's ASC unless generation-split
  (fresh-newest-first vs memory-arrival) or consume differs
  unshared-vs-shared.
DO NOT hand-derive further (six sign-flips this sitting). Cycle 4:
a micro-checker over {partner scan: asc|desc|fresh-first-desc|
fresh-first-asc, pre_rights scan: push|reverse, leftIns iter:
head|arrival, sink0 consume: fwd|rev, peer consume: fwd|rev,
(un)shared split: yes|no} against pins t1/t14/t15/551-both-rules/
526-both-rules/616/134 (all orderings recorded above and in the
probe JSONs; the fuzz keeps under tmp/cepfuzz_*). Then re-gate,
classify 987/853/810, campaign to zero.

### D-102 (cycle 4, round 1): the twalk micro-checker found ONE
### survivor but the PIN ENCODINGS were hand-derived pop-states —
### the port oscillated; cycle-4 round 2 must SIMULATE, not encode
tools/model_check_twalk.py enumerated {partner scan, pre_rights
scan, leftIns iter, sink0/peer/single consume} against 8 pins and
produced exactly one survivor: **partner scan = THIS-FIRE lefts
arrival-first then prior-fire newest-first; pre_rights push-order;
leftIns head; sink0+single consume REVERSE-creation, peer FORWARD**
(the sink0/peer split matches the engine's existing prepend/append
fan-out — only the partner scan needed porting).
BUT the port regressed 616/134/min_sj in three different stampings
(this-eval, fire-entry, fire-end boundaries) — because the model's
per-pin ENTRY STATES (what is in memory vs staged at the pop; which
generation a flush-filled left belongs to) were themselves
hand-derived, reintroducing exactly the hand-model hazard the
checker exists to remove. The passing-but-different pre_fill_len
variant treats flush-filled lefts as MEMORY at the pop; the model's
min_sj encoding treats them as THIS-FIRE; both satisfy their own
frame and contradict on the engine.
**Round 2 (next sitting): integrate the twalk dims into
model_check_stream.py** — it already simulates flushes/self-drains/
stashes per the landed semantics, so pop-entry states are DERIVED,
not encoded. Add: two-rule (sink0/peer) firing pins, the partner-
scan dims, per-consume-role dims; pins 551/526/616/134 both-rule
orders + min_sj/cf56/t1/t15 + t6/t7/t14 as flush-path regression
guards. Engine reverted to 317b178's green state (matrix 45,
corpus 857, campaign residual 4: cf101x987, cf202x526+853,
cf303x810 — 526 re-opens with the partner-scan revert, plus the
853/810/987 unclassified).

### D-102 (cycle 4, round 2): the SIMULATING checker converged — the
### survivor ported faithfully via three port-bug fixes; campaign
### residual = ONE structural pin (853-class), fenced by analysis
Round 2 rebuilt model_check_stream.run() to SIMULATE the landed
semantics (drain_t, linked stashes, sharing suppression, eager
corpse flags, quiescence deletes) so pop-entry states DERIVE —
the round-1 encoding hazard is gone. Two-rule pins compare per
consume role (sink0 = decl-first rule, peer = sharers).
**The survivor** (unique, all pins): partner scan = THIS-FIRE
lefts (filled OR self-drained this fire) in ARRIVAL order, then
prior-fire lefts NEWEST-first; pre_rights push-order; leftIns
head-first; sink0+single consume REVERSE-creation, peer FORWARD;
IF-toggle = pair-at-flush UNLESS the path holds PRE-LINK (ph=4)
rights, in which case the ENTIRE flush evaluation defers to the
pop (u1-vs-u1c: a fresh-with-the-relink P2 takes the flush window;
a held P2 forces one pop batch — measured on u1s/u1c controls,
promoted).
**Port** (three bugs found by trace, each a model-to-engine
mapping): (1) drained lefts must stamp fire_no too (t15-class);
(2) the fire boundary is END-incremented (between-fire inserts
stamp the NEXT fire); (3) the this-fire partition sorts by lseq,
not positional order (fills push in prepend order). Plus the
pair_unless_held gate checks BEFORE the stash empties staging.
**State: 45-rung matrix green; corpus 859 green; suites 8; both
1000-campaigns' residuals cleared except cf202x853** — a
three-left same-batch shared AB-self-join whose creation order
groups by LEFT (leftIns-driven), which the checker PROVES the
current dim space cannot express (every config fails exactly it):
the walk needs a STRUCTURE dimension (per-fact interleaving for
same-batch self-joins) — next cycle's single question, pinned
with both rule orders in the checker.

### D-102 (blast-radius correction): the stay/partner-scan semantics
### are SHARED-NODE-scoped — fresh campaign seeds caught an 18%
### regression the 47-pin matrix could not see
Fresh campaign seeds (7/13/29) measured 188/173/178 divergences per
1000 — vs 4-12 for the pre-temporal-stay engine. Commit bisect
pinned the break at 0dc2a4e (temporal-stay) with round-2's partner
scan compounding. Root cause: BOTH mechanisms were derived from
two-rule pins (616/551/526/134/853 — ALL shared-node shapes) and
ported UNSCOPED to every temporal node; ordinary single-rule
scenarios regressed en masse. The matrix stayed green throughout —
its pins are exactly the shapes the mechanisms were built for.
**Fix: scope both to node.shared** (a phreak-Node flag set from
path-membership): shared temporal nodes get stay-at-flush + the
this-fire-first partner scan; unshared nodes keep the certified
pre-0dc2a4e behavior (delta rights walk at flushes; lseq-ASC
partners). The pair_unless_held eval gate also narrowed to
enabler-type-triggered flushes only (flush_trigger_tid).
Recovery: 534/539 of the fresh kept set; matrix 47/47; corpus 859;
suites 8; old residuals 11/12 (853 open as before).
**Method lesson (for the doctrine file): a survivor family measured
only against its own discriminating pins is UNBOUNDED in blast
radius — every ported mechanism needs a fresh-seed population
measure before commit, not just the matrix.** The 101/202/303 keeps
were all shapes the mechanisms addressed; the fresh seeds were the
first population draw AFTER the ports.

### D-102 (853 closed; the residual is TWO cases, one class): rights
### enter memory in ARRIVAL order after per-fact AB walks; the
### 412-class pins the NEXT discriminator (and exposes a flawed
### control probe)
The per-fact AB walk left rights memory NEWEST-first; 853's fire-2
(unpinned in the checker — the pin only covered fire 1) showed the
next fire's leftIns x memory iterates ARRIVAL. Fixed in the engine
AND the model in lockstep; the 853 pin extended to fire 2; checker
survivor unique and unchanged. State: **5997/6000 campaign evidence
green** (548+551 keeps, 47-rung matrix, corpus 861, suites 8).
**OPEN (cf7x597 + cf29x412 — one class):** at an exists-relink
where the enabler ALSO arrives with held pre-link rights, the
oracle DOES flush-evaluate (creations [(IF,held),(IF,memory)]
window then the fresh pair) — contradicting u1c where the same
held-right shape required the deferred pop batch. Candidate
discriminator: the enabler type's UNCONSTRAINED alpha is shared
with other rules (412: E0 feeds TJ0/$a + J1 + the exists) vs
u1c's private enabler. **The u1s "shared-alpha" control was
FLAWED**: its second rule constrained the alpha (ts > 999999),
which builds a DIFFERENT alpha node — it never tested sharing.
Next sitting: a true shared-alpha probe pair (unconstrained
second rule), then the pair_unless_held gate learns the real
condition, checker-first.

### D-101/D-102 CLOSE: deterministic CEP E1 is CERTIFIED — final
### campaign 3x1000 = 0/0/0 divergences
The gate: fresh seeds 59/61/67 (never used before) at 1000 scenarios
each — ZERO divergences. Cumulative campaign evidence this arc:
~15,000 scenarios across 12 seeds, every divergence peeled to a
pinned mechanism (matrix now 55 CEP rungs in scenarios/probes),
suites 8, corpus 863 byte-identical, lint clean.
**The certified E1 semantics inventory** (each entry oracle-pinned,
model-checked where composition was at stake, population-measured):
- Pseudo-clock; BTreeMap deadlines at ts+expires+1 for
  rule-referenced event types (exactly ts+expires for unreferenced
  — no OTN); advance() marks eagerly (corpse flag: no NEW join
  pairs; existing not/exists blocking persists) and deletes at
  agenda QUIESCENCE in one round with the TMS teardowns; salience
  orders the observers after the round.
- TMS x expiration: expiring justifiers' teardowns ride the
  post-firing drain of their J-rule or the quiescence round
  (exp_deferred, separate from certified D-076 deferred).
- STREAM flush: trigger-scoped, touch-scoped, LEFT-flushing;
  plain-join rights NEVER flush-pair (all staged rights stash,
  pre-tail lefts + all dels stash; delta lefts and the trigger's
  own del effects flush); k=1 window dels/upds stash (never the
  insert's own); pre-link (ph=4) rights fire pre-LIFO then
  post-link arrival; IF-toggle at a link TRANSITION with held
  pre-link rights and prior link history exempt-evaluates (held
  rights visible, certified phase order); FIRST-ever link defers
  (pmem creation).
- SHARED temporal nodes (>1 rule path): stay-at-flush (both
  sides); sink0 consumes reverse-creation, peers forward (the
  fan-out prepend/append asymmetry).
- Temporal walk: rel_arrival partner scan (post-right lefts
  arrival-first, then pre-right arrival — subsumes lseq-ASC);
  same-batch AB self-joins walk PER-FACT newest-first (cross-right
  arm, self-pair, cross-left arm; memory arms in lseq-arrival);
  unlinked temporal deltas self-drain; per-fact fills enter memory
  in arrival order.
E2 fences unchanged: windows, @expires inference, @duration, entry
points, event updates/external deletes.
Checkers: tools/model_check_stream.py (7 dims, 30+ pins, two-rule
roles, simulated states), tools/model_check_temporal.py,
tools/model_check_twalk.py (historical). Doctrine additions this
arc: assert-your-anchors on scripted edits; population-measure
every ported mechanism; controls must share the exact network node
(the u1s constrained-alpha flaw); simulate states, never encode.

### D-103: positioned syntax errors — fail fast and loud (Arc 1)
Every DRL error now carries its source position. Mechanics:
- The lexer returns parallel char-offset spans per token; its own
  errors (unterminated string/comment, bad literals, unexpected
  chars) carry the offset directly.
- DrlError became { msg, span: Option<u32> }; all 72 construction
  sites converted mechanically (assert-anchored scripts): Parser
  method sites -> self.perr (current token's span) or perr_prev
  (the just-consumed token — the "expected X, got {tok}" pattern,
  19 sites; fixes the off-by-one where next() had advanced past the
  offender); post-parse lowering sites -> derr (span-less; the
  semantic wall text stands alone).
- attach_position renders once at the parse_file boundary:
  "... at line L, col C:\n  <source line>\n  <caret>". Example:
    DRL parse error: unexpected character '=' at line 4, col 22:
        not Blocker(name = $n)
                         ^
- EngineError: compile errors already carried "rule {name}:" via
  the D-073 closure; the two UNIT-level walls (D-057 qce x
  mutation, D-076/D-057 qce x insertLogical) now LIST the offending
  rule names on both sides.
- Python surface: messages flow through PySession unchanged
  (verified: line/col + caret reach CompileError).
Gates: engine/tests/d103_errors.rs (8 asserts: line/col, caret,
source echo, later-line, lexer positions, EOF-lands-on-last-token,
wall rule-naming, rule-scoped naming); suites 9; corpus 867
byte-identical (zero behavior change — error paths only); bindings
61/61; lint 926/926 (b8 annotated expect_inert — its rule
deliberately never references the event type).

### D-104: Engine::reset() — in-place session reset for paged
### batches (Arc 2), differential vs
### StatefulKnowledgeSessionImpl.reset()
Oracle-first: the runner gained {"op":"reset"} casting to the impl
class. FIRST MEASUREMENT FINDING: **reset() drops the session's
event listeners** — the initial ladder showed post-reset firings
happening but unlogged (rs_r1/r3/r7) and the insertion-index
listener dead (rs_r2 crashed on target 0). The runner re-registers
its listeners after reset; the pin set then came out clean:
- rs_r1 basic: pre-reset WM/agenda gone; post-reset fires fresh.
- rs_r2 handles: the insertion index RESTARTS (post-reset target 0
  = the first post-reset insert; handleFactory counters cleared).
- rs_r3 TMS: logical facts vanish (no re-justification residue);
  not-CE observers fire fresh.
- rs_r4 clock: pseudo-clock back to 0; an event whose ts would be
  ancient under the old clock lives a full fresh lifetime, and the
  ts+expires+1 boundary works on the NEW clock.
- rs_r5/r11: held staging (unlinked paths, ph4 generations,
  shared-node stashes) cleared — nothing leaks into post-reset
  composition.
- rs_r7 InitialFact: re-created — not-CE rules RE-FIRE post-reset
  (lists_built=false re-runs the prologue).
- rs_r8 queries: fresh; rs_r9 double-reset; rs_r10 reset with
  PENDING expirations mid-flight (corpse flags + pending list
  cleared; same-ts re-inserts unaffected).
**Engine::reset()**: clears every runtime field (store facts/
handles/expired via FactStore::reset keeping schemas; lias/trie/
nets rebuilt via build_network from the compiled rules — pattern
keys are pure, the alpha-sharing rewrites live in the cmps; TMS/
deadlines/clock/pending/ever_linked/query state to defaults;
lists_built=false; InitialFact re-asserted). Rules, queries,
event_specs, rule_order survive.
Gates: 10-probe ladder promoted (pr_rs_*); suites 9; corpus 877;
bindings 62/62 (Session.reset() + paged-batch equivalence test);
lint 936; fuzz_cep now DRAWS {"op":"reset"} at 0.15/epoch (clock
tracking resets with it) — campaign seeds 73/79/83 = 0/0/0 across
3000 scenarios of reset x CEP x TMS x flush composition.

### D-105: python sugar catch-up (Arc 3) — insertLogical, CEP,
### nulls, inline groups
All four compile-to-DRL only: the rendered text rides the certified
grammar and differential; no new evaluation machinery.
1. **TMS**: Rule.then_insert_logical(cls, **fields) renders
   insertLogical(new Cls(...)). The D-076 unit walls surface at
   build with rule names (test: modify-on-logical-type names the
   offender); delete of a logical type stays legal (stated
   retraction — the wall covers setters/update/modify only).
2. **CEP (E1)**: seine.Event(timestamp=, expires_ms=) +
   @fact(event=...) (parameterized decorator; explicit expires_ms
   REQUIRED, D-101/a8 — the error names the fence);
   seine.this_after/this_before(anchor, lo_ms, hi_ms) render
   `this after[lo,hi] $pN` with the anchor's fact var demanded in
   a pre-pass (anchors precede their temporal patterns);
   Session.advance(ms). Events flow class -> __seine_event__ ->
   _collect_events (rules' patterns + RHS classes + facts keys) ->
   the native events dict -> Engine::declare_event BEFORE rule
   compilation (Test::Temporal needs the spec at compile).
3. **Nulls (D-095/D-096)**: field.is_null()/is_not_null() render
   `f == null`/`f != null`; `field == None` is a CompileError
   naming is_null() and the Optional declaration — the 3VL choice
   stays explicit and legible.
4. **Inline boolean groups (D-073)**: |, &, ~ on constraints build
   groups rendering `(a || b)`, `(a && b)`, `!(a)` (pr_ib31's
   certified negation shape); leaves must share ONE pattern class
   (owners()-set check; the error names the foreign class and the
   D-073 no-cross-pattern rule).
Gates: bindings 70/70 (test_arc3: goldens + engine round-trips for
TMS auto-retraction, temporal pairing + advance expiration, null
firing, group firing); suites 9; corpus 877 (untouched — sugar
only). Agenda-group sugar deferred to after Arc 4 per the plan.

### D-106: agenda groups — agenda-group + focus stack + setFocus
### (Arc 4); core CERTIFIED, one fine-structure class OPEN
Grammar: `agenda-group "name"` rule attribute (lexer keyword-join
like no-loop); RHS `drools.setFocus("name");` (the only drools.*
method in the subset — the error names the fence).
**The recon ladder pinned** (13 probes, pr_ag*): unfocused groups
never fire (MAIN default); setFocus pushes; groups partition BEFORE
salience; last-setFocus-on-top; re-focus RELOCATES an already-
stacked group to the top (ag9's dance); focusing an empty group
pops through; an emptied group pops and does not resurrect across
fires (ag10); nested focus; TMS justifiers inside groups; no-loop
in groups; MAIN cannot preempt a focused group (ag13); dynamic
salience within groups (ag15 — found a LIVELOCK: the dyn-salience
preemption re-check had to be group-scoped or an out-of-group
higher rule loops the pop forever).
**Engine**: focus_stack on the agenda; the pop scan filters by the
stack top (queries live in MAIN); empty tops pop through before the
quiescence blocks; the post-firing strictly-higher halt check is
scoped to the top group; SetFocus relocates-or-pushes; reset()
clears the stack.
**Walls (measured)**: setFocus to a group NO rule declares is a
Drools runtime NPE (ConsequenceException) — walled at COMPILE
naming the rule and the fix (the D-076 pattern).
**Fuzz** (generator draws agenda-group at 12%/rule from {ga,gb} +
setFocus at 10% from DECLARED groups only; 5x10k campaign):
exposed the EXECUTOR-BINDING semantics — a non-halted executor
keeps control through its item WITHOUT a rescan iff, after empty
groups pop through, the focus is back on ITS OWN group
(fz_9001_1795: continue across an empty-group push; fz_9004_9:
halt when the pushed group holds activations). 30 campaign
witnesses fixed and kept as regression pins (scenarios/failures).
**OPEN (5 witnesses, probes_pending/agenda_open/)**: the halt
check's fine structure — fz_9003_879 shows the oracle CONTINUING a
salience -8 executor past queued salience-0 MAIN items right after
its setFocus emptied through; the halt comparison's pool (per-group
queues? item-creation timing at insert-staging?) needs its own
probe ladder or checker cycle. Also filed: fz_9001_6127
(probes_pending/fuzz_finds/) — an accumulate x update x eager
composition divergence with ZERO agenda constructs (a pre-existing
bug freshly sampled by the shifted generator; strip-test proven).
Gates: suites 9; corpus 944 (ladder promoted + 30 witness pins);
bindings 72/72 (Rule(agenda_group=) + then_set_focus sugar + the
undeclared-target wall test); lint clean.

### D-106 (halt-class drive, sitting 2): 879 CLOSED via two new
### mechanisms; the class narrowed to 5+1 witnesses and a mapped
### dimension space — checker-shaped for the next cycle
Two mechanisms landed (both measured, both gated):
1. **The peek evaluates-if-dirty** (fz_9003_879): the executor's
   halt-check peeks the focus-stack top; a queued-empty-DIRTY item
   evaluates first (the certified pop-path evaluation) — staging
   the constraints reject must not read as group-nonempty. 879's
   [R2, R2] continue-at-salience(-8) reproduced exactly.
2. **The pre-force drain list** (fz_9005_2842, fz_42_5243's rule
   applied): the executor's continue pool is the PRE-re-evaluation
   queue — activations born of the current firing's own RHS wait
   for the next reachable pop. Implemented as pre_force_qlen
   captured before the post-fire-force.
Plus three oracle discriminator probes (ag_h1/h2/h3, kept as
pending pins): fire-born HIGHER items — static AND dynamic — do
NOT halt a continuing executor ([L, L, X] all three), killing the
static-vs-dyn hypothesis.
**The remaining structure (5 witnesses: 7397/6467/214/873/2842)**:
2842 halts for a live (eager-evaluated, queue-nonempty) dyn item
while 1795/879 continue past dirt-only items — the "live queues
only" visibility model — but its blocker POOL oscillated through
{anywhere: 47/88, stack+MAIN: 47/88, stack-only: untested} vs the
stable peek-eval model's 83/88. SIX hand-flips = the D-083 signal:
the next cycle enumerates {peek pool, dirt visibility, eval-at-
peek, walk-through order, own-item comparison} mechanically
against ALL 88 witnesses (the fixed 30 + the ladder + h-probes +
the open 6). State: 83/88; certified gates all green (suites 9,
corpus 956, bindings 72, lint 959).

### D-106 (halt-class close-out): the pool space is mechanically
### EXHAUSTED — the engine-as-checker matrix (10 configs x 88
### witnesses) pins the stable model; 5 witnesses remain, each
### needing an individual decode
The adjudication rig: SEINE_HALT_TOP {eval-dirty | live-only} x
SEINE_HALT_POOL {none | any | stack+MAIN | MAIN | stack | MAIN-dyn}
run over all 88 agenda witnesses. Results: eval-dirty dominates
(live-only: 47-50/88); EVERY blocker-pool variant scores 77-81 vs
the stable none=83 — the executor's transparent-top continue
consults NO other group's queues, and the 2842-class halt is NOT a
pool-structured salience comparison. The five open witnesses
(7397/6467/214/873/2842 + the non-agenda 6127) each need a
trace-level decode; the halt-check hypothesis space {peek pool,
dirt visibility, dyn-ness} is EXCLUDED for them wholesale.
Stable config hard-coded (83/88); certified gates green (suites 9,
corpus 956x3 tiers, bindings 72, lint). The h1-h3 discriminator
probes ride probes_pending/agenda_open as oracle pins.

### ⚠⚠ D-106 STANDING CAVEAT (Bryan's ruling, 2026-07-07): THE HALT
### MODEL IS WRONG — IT IS A CLOSE APPROXIMATION, NOT THE MECHANISM ⚠⚠
**READ THIS BEFORE TOUCHING THE AGENDA EXECUTOR OR TRUSTING ITS
SEMANTICS AT THE MARGINS.** The shipped halt/continue model
(peek-evaluates-dirty + transparent-top + pre-force drain list,
no blocker pool) satisfies 83/88 witnesses and every certified
gate, but the FIVE open witnesses (probes_pending/agenda_open:
fz_9001_7397, fz_9003_6467, fz_9004_214, fz_9005_873,
fz_9005_2842) PROVE it is not Drools' actual mechanism — it is a
behavioral approximation that happens to coincide on the covered
surface. The matrix run (10 configs x 88 witnesses) excluded the
entire {peek pool, dirt visibility, dyn-ness} hypothesis space, so
the true mechanism is structured along a dimension we have NOT
identified. Consequences:
- Any future agenda-adjacent divergence should be triaged against
  THIS caveat first: do not patch the approximation locally; the
  revisit should re-derive the executor's halt from the five
  witnesses (trace-level decode each) and/or the Drools
  RuleExecutor source, checker-first.
- The five witnesses and the h1-h3 oracle discriminator probes are
  the pinned evidence base for that revisit; keep them current.
- New agenda features (auto-focus, lock-on-active — the ruled
  follow-up) must NOT build on the approximation without closing
  this first.
Banked at Bryan's direction; agenda-group core remains CERTIFIED
for the covered surface (13-probe ladder + 30 campaign pins +
5x10k fuzz draws, all green).

### D-107: queries across mutation epochs — the D-057 walls LIFTED
### (Arc 5)
Schema first: per-epoch query invocation ({"queries": [...]} inside
an epoch) in BOTH runners — queries run against that epoch's
post-quiescence WM; results append to the flat queries log.
**The qmut ladder (9 probes, pr_qm*) pinned the semantics:**
- ?query CEs are PULL-AT-ACTIVATION: churn on the QUERIED side
  never re-evaluates existing or absent matches (qm2: an update
  flipping a fact into the query result does NOT retro-fire the
  resident caller; qm4: RHS updates same; qm3: deletes same).
- CALLER-side churn is a fresh re-pull: update = the old match
  dies + a new activation pulls against the current WM (qm8/qm10);
  delete kills the match (qm9).
- TMS composes (qm5/qm7): logical retraction/re-assertion is
  visible to the NEXT pull, never retroactively.
- Standalone queries see the current WM per call (qm1) — which
  exposed a REAL bug: the D-056 accumulated drain windows kept
  facts an external UPDATE had flipped out of the pattern; the
  window now RE-TESTS alpha at every drain (still-passing facts
  keep their qx8-pinned accumulation).
**Lifted**: the compile wall (qce x update/modify/delete), the
qce x insertLogical wall (D-076/D-057), the runtime
reject_mutation_with_qce, AND the walk-level left-upd/del wall —
the qce node now carries per-site child-row memory
(qce_children): leftDel retracts the left's pulled rows (row facts
killed); leftUpd = retract + fresh re-pull as NEW activations.
reset() clears it. q2_walls flipped to assert composition; the
D-103 wall-naming test repointed at the D-106 setFocus wall.
**Generator**: the qce-vs-mutation exclusion lifted; per-epoch
query draws (30% of drawn calls also run mid-scenario).
**Campaign 5x10k**: 10 divergences — triaged by strip-test +
pre-Arc-5 bisect: 7 = the BANKED D-106 agenda-approximation tail
(filed with the caveat witnesses), 2 pre-existing non-query finds
(fz_9104_1496 accumulate-class, fz_9105_5693 TMS-class — filed in
probes_pending/fuzz_finds with 6127), and ONE ours:
**OPEN_fz_9103_4499** (probes_pending/qmut) — double-?query-CE
rules over-fire QOut (x17) under plain epoch INSERTS (no mutation;
the over-pull is in the fresh-left x armed-query composition).
Gates: ladder 9/9 promoted (corpus 987); suites clean; bindings
72; lint 978.
**OPS INCIDENT (doctrine escalation)**: a `git checkout -` after a
detached-HEAD bisect landed on a STALE previous-location; worse,
the session had been committing on a DETACHED HEAD for 14 commits
(main sat at b375c9a while D-103..D-106 lived detached — the stash
that "resolved" earlier dances had silently detached us). Recovery:
ff-merge main to the detached tip + clean stash pop; nothing lost.
NEW STANDING RULES: (1) NEVER bisect via stash/checkout in-place —
use `git worktree` (now twice-escalated); (2) after ANY checkout,
verify `git branch --show-current` is main before committing;
(3) periodically confirm `git log origin/main..main` counts match
expectations.

### D-107 addendum (Bryan's note, 2026-07-07): the two pre-existing
### fuzz finds may be DROOLS incoherence — revisit with that lens
The two divergences triaged out of the Arc-5 campaign as
pre-existing non-query finds — **fz_9104_1496** (accumulate x
update composition) and **fz_9105_5693** (TMS x update composition),
both in probes_pending/fuzz_finds/ — must be revisited with the
question inverted: is DROOLS ITSELF being incoherent here? Do not
assume the oracle side is right. The revisit should check the
oracle's behavior for internal consistency (e.g., minimize each,
vary fact/rule order for oracle-side instability, compare against
Drools' own documented semantics and its issue tracker) BEFORE
attempting any engine change. If Drools is incoherent, these land
on faithfulness axis 2 (value-bearing defect: correct + report —
the D-039/D-090 pattern), not as engine bugs. The same lens applies
to their earlier sibling fz_9001_6127 (accumulate x update x eager)
in the same directory.

### D-108: structured aggregation — collectList, collectSet, groupby
### (Arc 6); DRL-level, oracle-probed end to end
**Recon overturned the plan's premise**: all three work in the
9.44 DRL TEXT surface (groupby was expected to be model-only). The
16-pin ga-ladder (promoted, pr_ga*) pinned:
- **collectList**: fold=append in NETWORK STAGING ORDER (fire-1
  batches arrive reverse-insertion — the certified D-027 world;
  ga7: incremental thereafter — deletes remove in place, late
  inserts append); duplicates kept, ONE instance leaves per
  reverse (ga16); strings collect fine (ga11).
- **collectSet**: COUNTED-set semantics — a duplicate value
  survives a sibling fact's delete (ga15). Iteration order in
  Drools is raw HashSet internals (ga13: [3,100,-5,-1000000007];
  ga14 strings by hashCode) — the D-052-class unspecified order,
  resolved per the D-090 pattern: BOTH sides canonicalize SORTED
  under a distinct SetCollection type (oracle render patched for
  java.util.Set; engine stores sorted; list order stays
  significant).
- **groupby( SOURCE ; $key ; $res : func($arg) )**: one activation
  per live key; the match element is the [result, key] composite
  (QueryArgs-rendered, ga3 raw); re-keys migrate with both groups
  re-firing (ga8); emptied groups retract SILENTLY (ga9); results
  and keys bind downstream (ga10 joins on $c); empty-string keys
  group fine (ga12); any contributing change re-fires (no
  value-dedup). Engine: per-key AccCtx groups on the acc node,
  per-pattern hidden row types ([res, key]) for downstream binds,
  children [left, rowfact]. **Leading position ONLY** — groupby
  after other patterns is walled loudly (the ga-pins are all
  leading; the joined form is the next slice, with query-side
  aggregation composition).
**Fuzz**: the generator draws collectList/collectSet (results
opaque — no downstream comparisons); 30k campaign: ZERO divergences
involve the new functions (4 witnesses triaged: 3 banked-agenda
tail, 1 new sibling of the OPEN qce class — OPEN_fz_9201_1660 filed
with 4499, which hits the D-055 step-limit backstop loudly).
Lint gains the open_divergence category (filed witnesses are
neither ghosts nor fences). Gates: suites clean, corpus
1003-scenario probes tier + all tiers green, lint 998.
Python sugar for the new functions: deferred with the joined-
groupby slice (one authoring pass for both).

### D-109: @expires INFERENCE (CEP E2 arc, item A) — recon PINNED,
### PRE-implementation (awaiting Bryan's port gate)
**Ordering confirmed** (Bryan, 2026-07-07): CEP E2 = A→B→C→D→E,
**inference-first** — land the temporal-reach offset now, seam the
window term for B (plan `~/.claude/plans/graceful-waddling-stallman.md`).

**Mechanism — fully pinned** (62-probe boundary ladder, 3× fresh-JVM
stable; source-corroborated). In STREAM mode Drools infers a per-event-
type expiration offset from the temporal constraints:
`TemporalDependencyMatrix.getExpirationOffset` (the row-max) assembled in
`PatternBuilder.attachObjectTypeNode` / `getExpirationForType`, matrix built
by `BuildUtils.calculateTemporalDistance` (drools-core 9.44).

- **Per constraint `$b rel[lo,hi] $a`** (after ⇒ $b later; before ⇒ $b
  earlier), each participating type contributes an upperBound to a MAX:
  - **EARLIER event** (forward reach) → `+hi`.
  - **LATER event** (backward reach) → `-lo` (BuildUtils reverse =
    `Interval(-hi,-lo)`; the row-max takes upperBound = `-lo`).
- **offset = MAX of contributions** (matrix row-max, incl. Floyd-Warshall
  transitive closure over multi-event chains). If `max_ub < 0` →
  **NEVER_EXPIRES** (the type LEAKS forever). If ≥0 → offset = `max_ub`
  (Drools adds +1 for same-ts matching; Seine feeds expires_ms = `max_ub`
  to the existing D-102 `ts+expires+1` rule-referenced scheduler).

**THE LOAD-BEARING QUIRK (hand-reasoning gets it WRONG):** the LATER event
in `after[lo,hi]` expires at **0 iff lo==0, else NEVER**. Semantically the
later event's partner is always in the past ⇒ "always 0", but Drools leaks
it whenever lo>0 (backward upperBound = `-lo` < 0 → NEVER). Pinned:
`after[0,100]` probe gone@1; `after[1,100]/[20,80]/[50,100]` probe
present@100000. `before` mirrors exactly (earlier=$b, later=$a):
`before[0,100]` anchor gone@1, `before[50,100]` anchor present@100000.

**Solid pins:**
- **earlier = hi** (lo ignored, it is `hi` not the span): `after[0,100]/
  [50,100]/[20,80]` anchor present@hi, gone@(hi+1).
- **MAX-merge**: E1 anchoring `after[0,100]`+`after[0,300]` → present@300/
  gone@301 (= MAX 300, not min/first/sum).
- **boundary == explicit** (the differential proof): `infctl` @expires=100
  (certified D-102 path) and inferred-earlier=100 are BYTE-IDENTICAL
  (present@100/gone@101) — inferred maps onto the existing explicit scheduler
  with expires_ms = max_ub.
- **explicit wins, NO max-merge** (a8): @expires=50 in `after[0,100]` →
  gone@80 (offset 50; matrix's 100 IGNORED). `PatternBuilder`:
  `if(hard) use explicit; else max(matrix, behaviors/windows, soft)` — an
  explicit TIME_HARD @expires SUPPRESSES inference for that type.

**A→B seam (honest, documented):** real OTN offset =
`max(matrix_term, window_term)` (PatternBuilder:356-376).
`SlidingTimeWindow.getExpirationOffset()=size` (window:time(N) ⇒ N; the +1
convention is B's boundary-probe job), `SlidingLengthWindow=-1` (count-based,
no clock ⇒ no offset). Item A implements matrix_term; window_term stays None
behind an assert/TODO — B closes it (re-pin a4-style inference-with-window).

**Proposed Seine port (surgical, awaiting gate):**
- `harness/src/runner.rs:36` — allow absent expires_ms (→ declare_event None).
- engine: compile-time inference pass AFTER rule-compile — walk all
  `Constraint::Temporal`, per un-annotated event type compute
  `max_ub = max{ +hi if earlier, -lo if later }`; `<0` ⇒ leave NO deadline
  (never), else fill `event_specs` offset = `max_ub`. Explicit expires_ms
  skips inference. Scheduler unchanged.
- FENCE: TIME_SOFT `@expires(policy=TIME_SOFT)` out of subset (harness renders
  hard only); transitive multi-hop temporal chains = a fuzz-watch surface.

**Artifacts:** 62 recon probes `probes_pending/cep/inf{a,ctl,x,y}_*`
(engine_fenced). **Gate to green:** promote the boundary ladder to
`scenarios/probes/`, `make diff` byte-identical, extend `tools/fuzz_cep.py`
(un-annotated event types + advances straddling inferred boundaries), 3×1000
fresh-seed campaign at 0 divergences, `make lint-probes` clean.

### D-109 PORT LANDED (CEP E2 item A, @expires inference) — with
### TWO fuzz-flushed mechanisms the recon ladder could not reach
**Implemented + green.** Reach inference as reconned (compile-time
`infer_event_expiry` at the end of `add_rules_drl`; per un-annotated
event type, expiry = the closed row-max forward reach, fed to the D-102
`ts+expires+1` scheduler; explicit `@expires` skips inference, a8).
`event_specs` value → `Option<i64>` (None = never); `declare_event` takes
`Option`; `runner.rs:36` wall relaxed; `bindings` Some-wrap. The widened
CEP fuzz then flushed TWO mechanisms the boundary ladder never hit —
both pinned checker-first and ported:

- **(1) TRANSITIVE CLOSURE** (trans_e1 pin). Drools Floyd-Warshalls the
  temporal matrix (`TimeUtils.calculateTemporalDistance`), so a chain's
  EARLIEST event inherits the SUMMED reach (E1→E2→E3 = 100+50 = 150, not
  the pairwise 100). Ported as a per-rule STP closure
  (`accumulate_temporal_closure`): directed upperBound edges (reverse
  edges carry lower bounds → one matrix suffices), Floyd-Warshall, row-max
  per position. Verified engine≡oracle on after/before/mixed chains +
  diamond (pr_cep_inf_chain/beforechain/mixed/diamond).

- **(2) THE NEVER-OVERWRITE** (fuzz 42→10→0 over two rounds; the big one).
  `TemporalDependencyMatrix.getExpirationOffset` returns NEVER when a
  pattern's row-max upperBound is < 0, and `PatternBuilder.
  attachObjectTypeNode` uses that to OVERWRITE the type's OTN offset to
  NEVER (order-INDEPENDENT — bare/nb/char/iso probes; NOT max). So an
  inferred event type NEVER expires (leaks) if ANY of its patterns is
  non-forward: (a) BARE — a positive/`not`/`exists` pattern with no
  temporal constraint; (b) purely-BACKWARD — the LATER event of
  `after[lo>0]` (row-max −lo), a self-join's probe side, or a cross-rule
  later reference. This UNIFIES the lo>0 leak (a single backward pattern)
  with the bare rule and the self-join; the lo=0/lo>0 discontinuity is
  EXACTLY the reach ≥0 / <0 boundary. Explicit hard `@expires` is immune
  (set in the `if(hard)` branch, never overwritten). Ported as a
  `never_inferred` set (bare-pattern scan in `compile_rule` + negative-
  reach positions from the closure); `infer_event_expiry` returns None
  for it. Pins: pr_cep_inf_bare_positive/bare_not/selfjoin_never/
  selfjoin_lo0_finite/crossrule_never/bare_explicit_immune.

**Fuzz** (`tools/fuzz_cep.py` extended: inference-mode scenarios —
un-annotated types, mixed explicit/inferred, transitive chains,
boundary-straddling advances). 3×1000 fresh seeds (5001-3): **0
inference-related divergences**. TWO finds total, BOTH bisect-confirmed
PRE-EXISTING E1 temporal-join FIRING-ORDER shapes (worktree at 5b23e7c:
OLD engine == NEW engine, ≠ oracle; explicit-expiry, my inference code
never touches that path) — the D-070 "widened grammar flushes latent
bugs" lesson. Quarantined minimized to `scenarios/xfail/
xf_cep_tjorder_{dual_tms,chain_exists}.json`; both are multi-rule
temporal-join order × TMS-agenda/exists micro-timing (the D-080/D-101
envelope), DEFERRED to an E1-hardening pass. A bonus: the CEP fuzz now
also exercises latent E1 order shapes.

**Gates:** baseline 11 / probes 729 (26 boundary + 8 closure + 6
never-rule inference pins) / regressions 281 — all BYTE-IDENTICAL; lint
1033; 8 suites clean. **Files:** `engine.rs` (event_specs Option,
temporal_ub / never_inferred, accumulate_temporal_closure,
infer_event_expiry, schedule Option), `runner.rs`, `bindings/src/lib.rs`,
`tools/fuzz_cep.py`. **A→B SEAM kept**: window:time(N) size folds into
`temporal_ub` (max) when item B lands — re-pin a4-style inference-with-
window then. **Upstream:** `docs/drools-inferred-expiry-never.md` drafted
(the never-overwrite = a silent event-leak footgun; framed as
intended-or-not for upstream, unlike the #2366 defect). Certified corpus
byte-identical throughout (CEP gated on `!event_specs.is_empty()`).

### D-110: WINDOWS (CEP E2 item B) — recon PINNED (core + the A→B
### seam), PRE-implementation (awaiting Bryan's port gate)
**Mechanism pinned** (36-probe ladder, oracle passes `over window:` DRL
to Drools verbatim; engine walls it at `drl.rs` accumulate `;`-expect).
Readout = accumulate `count()`/`sum()` (window membership) + WM presence.

- **`window:time(N)`** — CLOCK-RELATIVE sliding: an event is in the
  window iff `clock − ts < N`; evicted at exactly `ts+N` (win_t_b: count
  1 at adv 99, 0 at adv 100 — note NO +1, unlike expiration's ts+N+1).
  Per-EVENT (win_t_slide: E@0 out at 100, E@50 out at 150). PER-SUBTREE:
  window eviction unmatches the accumulate (count→0) but the FACT stays
  in WM if something else retains it (a4; win_t_b E_in_WM w/ big
  @expires) — the fact-survives observable is the differential separator
  vs @expires.
- **`window:length(N)`** — keeps the N MOST-RECENTLY-INSERTED events
  (FIFO by insertion, NOT by ts): win2_len_1 sum=20 (E@20 only),
  len_2 sum=30 (E@10+E@20), len_3 sum=30 (all). Per-subtree (facts stay
  in WM, count capped at N). NO clock.
- **THE A→B SEAM — CLOSED** (item A left `max(matrix_ub, window_ub)` with
  window_ub=None). `window:time(N)` FEEDS the inferred `@expires`:
  accumulate-only, no explicit expiry → E EXPIRES from WM at `ts+N`
  (win3_seam_b: present 99, gone 100 — boundary `ts+N`, NO +1; per-event
  win3_seam_multi). MAX with the temporal reach: E in window:time(100) +
  earlier in after[0,200] → gone at 201 = `ts+200+1` (win3_seam_tmax) —
  i.e. OTN offset = `max(window_size N, matrix_ub+1)`; window contributes
  N RAW, the matrix term keeps its +1. **Seine mapping:** fold `(N−1)`
  into `temporal_ub` so the existing D-109 `ts+expires+1` scheduler
  yields `max(ub+1, N)` — window-only → ts+N, temporal-wins → ts+ub+1.
  Also: a windowed pattern is NOT "bare" (the window offset overrides the
  D-109 never-overwrite — it takes the non-NEVER `distance` path in
  attachObjectTypeNode). **`window:length` does NOT feed inference**
  (`SlidingLengthWindow.getExpirationOffset=-1`): win2_seam_len events
  never expire (E_in_WM n=3 at adv 100000, count capped 2) — another
  leak footgun.
- **explicit `@expires` SUPPRESSES the window term** (hard wins, NOT max):
  explicit=50 + window:time(100) → E gone at 51 = ts+50+1
  (win4_expl50), window ignored — PatternBuilder `if(hard) use it`.
- **constraint-in-window**: `E(tag=="x") over window:time(N)` windows the
  ALPHA-FILTERED events (win4_constr: count 1, the y-event excluded).
- **standalone `E() over window:time(N)`** PARSES + fires (win2_stand);
  the sliding-membership effect on a plain (non-accumulate) pattern needs
  a re-evaluating readout — DEFERRED with the deep compositions.

**DEFERRED to a model-check sub-recon** (the E1 close ran on
`model_check_stream` — these compositions flip-flop; extend the checker,
don't hand-reason): window × STREAM per-insert flush; window × TMS
(a windowed justifier's eviction vs the justified fact); window-node
SHARING identity; `window:length` eviction under external update/delete.
Use the AccDump/RunnerDump graft for WindowNode memory ground truth
(memo §4).

**Proposed port (surgical core, awaiting gate):** parser — `drl.rs`
accumulate source accepts `over window:time(N)|window:length(N)` before
`;` (+ the standalone pattern form); a per-node window membership
structure (time = deadline-queue eviction reusing the D-101 BTreeMap;
length = count-based FIFO ring); per-subtree unmatch through the
certified delete/unmatch path (count re-fires on evict, fact untouched);
close the A→B seam (window:time size−1 into `temporal_ub`; window:length
contributes nothing; a windowed pattern is not `never_inferred`).
**Gate:** as D-109 — promote the ladder, `make diff` byte-identical,
extend `tools/fuzz_cep.py` (window draws), 3×1000 campaign, lint. The
deep compositions get their own D-entry + Bryan gate after the
model-check.
**Artifacts:** 36 recon probes `probes_pending/cep/win{,2,3,4}_*`
(engine_fenced).

## 2026-07-07 — Session (cont.), CEP E2 item B RUNTIME

### D-111: window:time runtime (eviction + A→B seam) — CORE landed pin-green; the accumulate+expiration DEFERRAL gap surfaced (pre-existing) blocks the fuzz campaign

**Ported the surgical core (all oracle-pinned this session, PROBE-FIRST —
ran the 36 fenced recon probes + 10 fresh edge probes live before touching
the engine).** Un-walled `window:time` at compile (`engine.rs` CompiledAcc
now carries `window_time`), built the runtime, closed the seam:

- **Per-subtree eviction** — at insert, `schedule_window_evictions` queues
  `(windowed acc node idx, event)` at EXACTLY `ts+N` (no +1) in a new
  `window_deadlines` BTreeMap (precomputed `window_nodes` maps event type →
  its windowed Acc trie nodes). `advance()` drains due entries into
  `pending_window_evictions`; at agenda quiescence `evict_from_window` does
  a SCOPED right-delete at that ONE node (`active.remove` guard +
  `s_right.add_del` + `note_link_effects_ex` → eval_acc_node Phase B→G
  re-fires the count) WITHOUT killing the fact. The `active` guard gives the
  double-removal NO-OP for free (win4_expl50: an explicit expiry < N removes
  the event first; the later eviction finds it gone). The fact survives
  WM-wide (win_t_b/win_x_bare/win_x_back: E persists while count→0).
- **A→B seam (pattern-level, not type-level — the probe corrected the
  hand-intuition).** In `compile_rule` the windowed accumulate source folds
  `N−1` into `temporal_ub` (max-merged) AND is skipped from the bare-pattern
  `never_inferred` insert — but a SEPARATE bare/backward pattern on the same
  type STILL adds it, and that NEVER overwrite DOMINATES the window
  (win_x_bare/win_x_back pinned live: E persists, only the subtree count
  drops — a type-level exemption would have wrongly expired E at ts+N). A
  larger temporal reach wins via the max (win3_seam_tmax=200 → deadline
  201); a smaller one loses to the window (win_x_fwd50: fold 99 beats reach
  50 → deadline 100). Explicit `@expires` suppresses inference (skips the
  fold) but NOT eviction (win2_seam_time_expl / win_t_b: count still drops
  at ts+N).
- Certified-corpus guard holds: `window_nodes`/`event_specs` empty ⇒ every
  new path is inert. Reset recomputes `window_nodes` in `build_network`
  (trie reindexes).

**Gate (core): GREEN.** 38 window probes promoted to `scenarios/probes/pr_cep_win*`
+ `pr_cep_a4_window_scope` (un-fenced), all byte-identical vs the live
oracle. `make diff` baseline 11 / probes 767 / regressions 281
byte-identical; `make lint-probes` 1117 live 0 ghost; 8 Rust suites. The
length/standalone recon probes stay `engine_fenced` in `probes_pending`
(follow-on slab).

**FUZZ CAMPAIGN BLOCKED — a PRE-EXISTING accumulate+expiration DEFERRAL gap
(NOT a window bug).** Extending `fuzz_cep.py` with window draws surfaced
that Seine's D-102 expiration deferral (expiration propagates at agenda
QUIESCENCE, pinned for the not-CE-blocking case) does NOT compose with
`accumulate` — two facets, BOTH reproduced on a PLAIN accumulate with NO
window and CONFIRMED on pristine HEAD (823e97a) via `git worktree`:
- **count transient** (`scenarios/xfail/xf_acc_expire_reinsert`):
  `sum(ts)` over `E(@expires 50)`, E@10, advance 100 (E@10 expires), insert
  E@100 → oracle `[10],[100]`; Seine `[10],[110],[100]` — the deferred
  removal of E@10 transiently coexists with the later insert.
- **firing order** (`scenarios/xfail/xf_acc_expire_order`): `accumulate
  count` salience 5 + a salience-0 rule, E expires + a concurrent insert →
  oracle fires the count-drop `W[0]` by salience (BEFORE the s0 rule); Seine
  defers it to a late quiescence round (fires it AFTER). Drools fires all
  clock jobs at advanceTime, THEN the agenda orders by salience; Seine's
  deferral does not salience-interleave the accumulate count-drop.

The window-evict INHERITS this deferral (it drains at the same quiescence),
so window scenarios with a later insert into the accumulate or a concurrent
agenda diverge identically. Campaign 300@seed1: 47 divergences, **ALL with
the CORRECT final WM** — 39 pure firing-REORDER (identical firing multiset),
8 a missing/extra INTERMEDIATE count-drop re-eval (transient coalescing);
**ZERO wrong window counts, ZERO wrong WM**. The eviction/seam VALUES are
sound; every divergence is the deferral composition. The CEP fuzz never
drew `accumulate` before, so the gap was latent (no promoted probe exercises
concurrent-agenda accumulate+expiration either).

**This is the `window × STREAM-flush` model-check sub-recon the plan already
defers — now with a concrete pre-existing root cause.** NOT hand-reasoned
further (D-083: deferral compositions flip-flop; extend `model_check_stream`
with the accumulate-removal-ordering dimension, don't encode). A speculative
"stage evicts in the expiration round to salience-interleave" was tried and
REVERTED — it made the evict-vs-expire order worse and does not touch the
underlying deferral.

**HANDOFF — Bryan gate (checkpoint).** Window runtime + seam are landed and
pin-green; the certified corpus is untouched. The fuzz campaign is blocked
by the pre-existing accumulate+expiration deferral gap. Decision needed:
(A) land the core now, defer the deferral fix + the window campaign to a
follow-on (model-check `model_check_stream` for the accumulate-removal
ordering; the two xfail repros are the anchors) — matches the plan's
window×flush deferral; or (B) take the deferral fix first (own recon,
touches certified D-102 machinery). Recommend A. Uncommitted; commit awaits
Bryan.
**Artifacts:** 38 promoted `scenarios/probes/pr_cep_win*` +
`pr_cep_a4_window_scope`; 10 un-fenced recon probes in `probes_pending/cep`
(win_x_bare/back/fwd50, win2_seam_time_99); 2 xfail repros; `fuzz_cep.py`
window draws (dedicated initial-only `EW` stream, still surfaces the
deferral gap via any accumulate+removal+concurrency).

## 2026-07-07 — Session (cont.), accumulate-eager deferral fix

### D-112: accumulate removals are EAGER (advance-time, by salience) — the pre-existing accumulate+expiration deferral gap FIXED; the windowed removal-TIMING composition deferred to a WindowNode model-check

**Recon (PROBE-FIRST, `probes_pending/cep/df_*`):** a uniform ladder (E
expires, a concurrent insert) pinned the eager/lazy split PER NODE KIND —
- `df_ord_acc` `[1],[0],Rp`: a PLAIN accumulate count-drop (sal 5) fires
  BEFORE Rp (sal 0) → EAGER; `df_ord_acc_lo` `[1],Rp,[0]`: by its OWN
  salience. `df_reins_{sum,count,max}` `[10],[100]`: the removal lands
  BEFORE a later insert into the accumulate (no transient).
- `df_ord_not` `[Rp,not]`: a not-CE (sal 5) fires AFTER Rp (sal 0) → LAZY
  (the D-102 cf5x33 pin). `df_ord_inter` `[acc1,acc0,Rp,not]`: the SAME
  expiring event drives an EAGER accumulate drop AND a LAZY not-CE unblock
  in one scenario — the split is per-node-type, not per-delete.
- Window EVICTION (fact survives): EAGER (`df_win_evict_ctl`,
  `df_evict_reins` `[10],[100]`).

**Model-check (`tools/model_check_accdefer.py`):** enumerated the removal
timing {eager,lazy} per node kind against the df pins → UNIQUE survivor
**accumulate EAGER, not-CE LAZY** (`acc=lazy` fails df_ord_acc; `not=eager`
fails df_ord_not). model_check_stream models the beta-JOIN flush micro-
order only (no accumulate node, no agenda) so it can't decide this — a
focused checker per doctrine ("compose a novel harness").

**Port:** in `advance()`, expiring events take an EAGER scoped right-delete
(`stage_acc_removal`, was `evict_from_window`) at every PLAIN accumulate
node they feed (`eager_acc_removals`), and window evictions apply their
scoped delete eagerly too — both stage the count-drop BEFORE the fire's
inserts and fire by salience. The fact WM-removal + not-CE/temporal/join
effects stay DEFERRED (pending_expirations → quiescence). `acc_nodes`
(all accumulate nodes, `window_time` optional) replaces `window_nodes`;
`pending_window_evictions` + `drain_pending_window_evictions` removed.

**Gate GREEN + blast radius CLEAN:** 18 accumulate-deferral pins promoted;
`make diff` baseline 11 / probes 785 / regressions 281 byte-identical;
lint 1153; 8 Rust suites. **Blast radius:** main Drools-axis fuzz
(gen.rs, non-CEP) 3×seed = 3900 cases, 0 divergences (non-CEP never
advances the clock ⇒ the new path is inert). CEP fuzz (now drawing
accumulates over event streams with epoch inserts): **PLAIN accumulate 0
divergences and no-accumulate 0 divergences across seeds 1/2/7** — the fix
is clean and the certified E1/temporal/TMS is untouched. The 2 committed
`xf_acc_expire_*` repros now PASS (promoted to pins
`df_plain_expire_{reins,order}`).

**DEFERRED — the WINDOWED-accumulate removal TIMING composition.** The
residual CEP-fuzz divergences are ALL windowed accumulate (≈0.5–5% of
windowed draws; 0 plain, 0 non-acc). The exact eager/lazy of a window
EVICTION vs a coincident/earlier windowed EXPIRATION, under a later insert
into the accumulate + concurrent salience, FLIP-FLOPS: `df_win_expire_reins`
`[10],[110],[100]` (windowed expiration is LAZY — the transient IS the
oracle) vs `df_win_evict_ctl` `[10],[100]` (eviction EAGER); and fuzz
cf1x65 (count-drop should fire by salience) vs cf1x233 (inferred expiry=N
→ lazy) vs cf1x249 (eviction transient) cannot be reconciled by a simple
rule — minimal 1-event repros all PASS, so it needs the multi-event flush
micro-order. Per D-083 this is NOT hand-tuned. **NEXT (own sub-recon):
extend `model_check_stream` with a WindowNode + AccumulateNode and
enumerate the window-eviction/expiration-vs-insert flush micro-order;
anchors `scenarios/xfail/xf_win_acc_defer_{1,2}`.** Current code keeps the
pin-correct eager eviction + lazy windowed-expiration (an improvement over
D-111's fully-lazy eviction; 38 window pins + corpus hold).

**HANDOFF:** plain accumulate+expiration deferral bug is FIXED and clean;
uncommitted; commit awaits Bryan. Windowed removal-timing = the WindowNode
model-check sub-recon.
**Artifacts:** `tools/model_check_accdefer.py`; 18 promoted pins
`scenarios/probes/pr_cep_df_*` + `pr_cep_win_defer_*`; 2 xfail anchors;
`fuzz_cep.py` accumulate-over-event-stream draws.

### D-113: window accumulate NODE-SHARING identity fix — the bulk of the "windowed removal-timing composition" was a concrete node-sharing bug, not a flush micro-order

Starting the WindowNode model-check sub-recon (task#6) PROBE-FIRST, the
first anchor `xf_win_acc_defer_1` bisected to a MINIMAL repro (`share_same`,
now `pr_cep_win_share_wvp`): a WINDOWED accumulate `accumulate(E1($t:ts)
over window:time(100ms); sum($t))` and a PLAIN `accumulate(E1($t:ts);
sum($t))` with the SAME source binding both reported the WINDOWED value
(`W2[217], W3[217]`; oracle `W2[217], W3[226]`). Root cause: `pattern_key`
(the D-037 node-sharing identity) folded the accumulate as
`func:arg_name:arg_field` but OMITTED `window_time` (and `key_field`) — so
D-111's `over window:time(N)` was invisible to sharing and a windowed acc
collided with a plain one over the same binding, sharing the node and
propagating the windowed result to both. (Latent since D-111: `window_time`
was added to the spec but not to this key.) FIX: fold `:w{window_time}:g
{key_field}` into the accumulate key. Uniform string change ⇒ existing
sharing preserved (corpus byte-identical); only window/groupby-differing
accumulates now split. Pins `pr_cep_win_share_{wvp,nn,id}` (windowed≠plain,
diff-N≠, identical=share).

**Impact:** this was the MAJORITY of the "windowed composition" fuzz
divergences — CEP fuzz seed 7 **19→2**, seed 1 **8→3** after the fix; corpus
byte-identical (788/281/11), lint 1159, 8 suites. `xf_win_acc_defer_1` now
PASSES (retired from xfail). So the earlier "flip-flopping" (D-112) was
largely this concrete bug masquerading as a timing composition — a good
reminder to bisect fuzz cases to minimal repros before declaring a flush
micro-order.

**STILL DEFERRED — the genuine windowed removal-TIMING residual (~0.5% of
windowed draws).** After the sharing fix the remaining failures are the
multi-epoch eviction/expiration TRANSIENT (a windowed accumulate keeps an
evicted/expired event transiently when a later insert arrives — cf1x233/
cf1x249/cf1x320; anchor `xf_win_acc_defer_2`). Minimal 1–2-event repros all
PASS (evt1/evt2_a/evt2_b, df_win_evict_ctl), so it genuinely lives in the
MULTI-EVENT flush micro-order — the WindowNode model-check target (extend
`model_check_stream` with a WindowNode + AccumulateNode; do NOT hand-tune,
D-083). (`cf7x121`-class residuals are pre-existing temporal-join-order,
unrelated.) **NEXT:** the WindowNode flush-order model-check.
**Artifacts:** `pr_cep_win_share_{wvp,nn,id}`; `engine.rs pattern_key` acc
key; anchor `xf_win_acc_defer_2`.

### D-114: the reset×WindowNode residual is a Drools INCOHERENCE — FENCED (closes the WindowNode sub-recon, task#6)

After D-113 cleared the bulk, the windowed residual was ~0.5% and ALL
reset-related (seed3 0/300). "Probe deeper first" (Bryan) instead of jumping
to the WindowNode model-check paid off: bisected to a minimal repro
(`scenarios/xfail/xf_win_reset_incoherence`) and a CLEAN discriminator —
- **plain** accumulate + reset (prior value) → `[23],[275],[0]` (both match);
- **windowed** accumulate, identical shape → oracle `[23],[275],[0],[0]`,
  Seine `[23],[275],[0]` — Drools fires ONE EXTRA spurious `[0]` (the second
  is `0→0`, no change), only for the WINDOWED accumulate.
The extra count is always exactly 1 regardless of pre-reset event count
(n1/n2/n3 all +1; n0 +0), and it fires AT the post-reset eviction, not at a
later clock — so it's not a phantom-per-event nor a leftover job. It's a
Drools reset×WindowNode inconsistency: resetting a windowed accumulate that
held a value emits a redundant firing that a plain accumulate does not.
**Seine is arguably MORE correct (consistent batching, no spurious fire); it
is NOT a Seine bug and NOT the flush-order composition it looked like** — so
neither a code fix nor the WindowNode model-check simulator is warranted.

**FENCED** (D-107 Drools-incoherence lens): anchor
`xf_win_reset_incoherence` in `scenarios/xfail/` (excluded from the gate);
`fuzz_cep.py` skips the `reset` draw when a windowed accumulate is present
(`self.has_window`) so the CEP fuzz doesn't re-flag the fenced incoherence.
CEP fuzz now clean on the window axis (seeds 1/3/7 = 0/0/1, the one being
`cf7x121` = pre-existing temporal-join-order, unrelated). Corpus
byte-identical. **Task#6 CLOSED:** the windowed-removal "composition" was, in
order of impact, (1) D-113 node-sharing bug [fixed], (2) D-114 reset×window
Drools-incoherence [fenced], (3) pre-existing temporal-join-order [separate].
No genuine flush-order composition remained — the WindowNode model-check
simulator is NOT needed.
**Artifacts:** `fuzz_cep.py` reset-vs-window fence; anchor
`xf_win_reset_incoherence`.

### D-115: CEP E2 item C (event UPDATE / external DELETE) — the D-047 mutation path has FOUR composition gaps with the CEP machinery; HYBRID resolution (cheap-port delete-of-dead + FENCE classes 1/2/3), Bryan-ruled

**Recon (probe-first + fuzz-driven).** The seed recon (6 probes, prior
handoff) showed the D-047 external update/delete plumbing + deadline + D-112
eager-accumulate already handle the BASICS (event @timestamp fixed at insert;
delete cancels; delete/alpha-update drop an accumulate). The plan guessed C
might be "trivially clean." It is NOT. Extending `tools/fuzz_cep.py` with a
SOUND live-only mutation axis (targets only the initial-facts prefix `[0,k)`
— indices are firing-INDEPENDENT since those handles precede any fire and the
runner+oracle both key the SAME visible-insertion index incl. rule-inserted
D/P3; targets only PROVABLY-LIVE facts — P forever, explicit-expiry events
while `clock < ts+expires`; inferred-expiry not targeted; deletes leave the
pool; reset clears it) flushed **four distinct, bisect-confirmed mutation-
driven composition gaps** (every find's no-mutation variant PASSES — no
pre-existing latent). Common theme = the re-fire / UNIFORM-FOLD signature:
the external mutation path does not reproduce Drools' re-propagation when an
event is entangled with temporal / clock-removal / existential state.

- **CLASS 1 — update × temporal join** (`xf_cep_c_upd_temporal`): Drools
  re-fires an after/before match on ANY external update of a participating
  event (even a no-op value, even an irrelevant field — temporal Behavior
  nodes are NOT property-reactive). Engine treats them as property-reactive
  (like a plain join) → does NOT re-fire → UNDER-fires. `TJ0` vs `TJ0 TJ0`.
  Deterministic (oracle 3×); both after/before; plain beta joins AGREE
  (`pr_cep_c_plainjoin_upd`).
- **CLASS 2 — update revives a clock-removed event into an accumulate**
  (`xf_cep_c_upd_evict_revive` window; `xf_cep_c_upd_after_exp` expiration —
  ONE root): a clock job (window eviction at `ts+N`, or expiration) STAGES
  the removal (count drops); a later external update re-propagates the still-
  `is_alive` event → engine RE-ADDS it; Drools kept it removed. `W[1]W[1]`
  vs `W[1]W[0]`. Mechanism: `advance` marks expiry + stages the eager
  acc-removal but leaves the handle `is_alive` (kill deferred to
  `drain_pending_expirations`, engine.rs:3680); window eviction is a staged
  right-delete; `on_update` re-propagates on `is_alive`+alpha-pass, ignorant
  of the evicted/expired state. Controls PASS: evict/expire-only; update-
  BEFORE-removal (`pr_cep_c_upd_before_evict`); delete-after-removal
  (`pr_cep_c_del_after_evict` — delete kills the handle, doesn't re-propagate).
- **CLASS 3 — external delete+insert witness churn × exists**
  (`xf_cep_c_del_churn_exists`): external delete is IMMEDIATE (`on_delete`) →
  un-fires exists; a same-epoch insert re-fires it in Drools; engine keeps
  exists linked across the churn → UNDER-fires. `NE` vs `NE NE`. C-SPECIFIC:
  the EXPIRATION analog PASSES (`pr_cep_c_exists_churn_expire` — expiration
  retraction to exists defers to quiescence, D-102, and nets out). Controls
  PASS: delete-only-no-reinsert (`pr_cep_c_del_exists_noreins`); not-CE.
- **CLASS 4 — dead-handle edges.** delete-of-already-deleted
  (`pr_cep_c_double_del`): Drools' `session.delete` is LENIENT (no-op); the
  engine hard-errored "delete of dead handle". update-of-deleted
  (`c_upd_after_del`, `engine_fenced` in probes_pending): both error
  (engine "dead handle"; oracle NPE) — OUT OF SUBSET.

**What WORKS (pinned `pr_cep_c_*`):** delete broadly; update × alpha
entry/exit / live (non-windowed) accumulate re-fold / windowed ts-update
before eviction / not / exists (no-op) / TMS-justifier; plain beta-join
update; delete-of-EXPIRED (expired events stay `is_alive` until drain).

**BRYAN'S RULING — HYBRID (cheap-port + fence rest).** This matches the
stop-rule (fuzz keeps finding cells across the shared, corpus-critical D-047
path → fence + plan a port, D-025/D-084 precedent; E2 already walls
@duration).
1. **PORT (D-115):** `delete_fact` NO-OPS on a dead handle (engine.rs:3780,
   `Err`→`Ok`) — Drools-faithful, low risk. `pr_cep_c_double_del` flips green;
   corpus byte-identical (no corpus scenario can rely on delete-of-dead
   erroring — Drools no-ops, so it would already FAIL). update_fact UNCHANGED
   (update-of-deleted stays out-of-subset).
2. **FENCE classes 1/2/3 + update-of-deleted:** `fuzz_cep.py` tracks per-
   event-type hazard sets during rule-gen (`temporal_types`,
   `windowed_acc_types`, `exists_types`) and skips the divergent combos —
   UPDATE excludes temporal + windowed event types; DELETE excludes an
   exists-witness churn (delete an exists_type while a same-type event
   arrives that epoch). class-2-EXPIRATION / update-of-deleted / double-
   delete are already unreachable (liveness gate never targets a past-
   deadline or deleted handle). Fenced CEP fuzz CLEAN: 3×1000 fresh
   (seeds 11/12/13) = 0 divergences. The 4 minimal class repros are
   quarantined to `scenarios/xfail/xf_cep_c_*` (`open_divergence`) and,
   with the working `pr_cep_c_*` boundary pins + the mutation fuzz, form the
   PRE-BUILT validation battery for the deferred re-propagation port (temporal
   Behavior modify re-fire; on_update evicted/expired guard; exists external-
   delete round-trip).

**Gates:** baseline 11 / probes 804 / regressions 281 byte-identical; lint
1176 live/0 ghost/0 inert (incl. `c_upd_after_del` engine_fenced wall). 16
`pr_cep_c_*` promoted, 4 `xf_cep_c_*` quarantined. **Blast-radius** (Bryan's
D-112 mandate — the port touches the shared D-047 path): `make fuzz` main-axis
(gen.rs) seeds 42/123/7 — the only divergences found are DELETE-FREE
pre-existing latents (fz_42/fz_123 temporal-join/accumulate-match family,
the E1-hardening envelope), which the delete-only change provably cannot
touch → ZERO delete-related regressions.
**Artifacts:** `engine.rs` delete_fact no-op; `fuzz_cep.py` mutation axis +
class-1/2/3 fences + `CEP_NO_TEMPORAL` diagnostic flag; `pr_cep_c_*` (16),
`xf_cep_c_*` (4), `probes_pending/cep/c_upd_after_del` (engine_fenced);
findings handoff `~/.claude/plans/cep-e2-item-c-findings.md`.

### D-116: CEP E2 item D (entry points, `from entry-point`) — UNWALLED. Named entry points = an orthogonal ROUTING DIMENSION on the alpha network; full port incl. mutation×EP (Bryan scope ruling)

**Recon (probe-first, oracle-pinned — 14 `probes_pending/entrypoint/`).**
`Type(...) from entry-point "S1"` draws a pattern from a NAMED partitioned
stream instead of the DEFAULT WM. Pinned semantics: (1) **PARTITION** — a
pattern matches ONLY facts inserted into its EP; DEFAULT patterns don't see
EP facts and vice-versa (bidirectional isolation, ep2/ep3/ep4/ep10). (2)
**REGISTRATION** — an EP must be referenced by ≥1 rule to be insertable
(`session.getEntryPoint(unref)` = null → NPE; ep_unref). (3) **COMPOSITION** —
the partition is respected per-EP by accumulate/not/exists/cross-EP join/
expiration(shared clock)/window (ep6–9, ep_evt_expire, ep_window_a); node-
sharing partitions by EP (ep_share). (4) **SYNTAX** — `Pattern(...) [over
window:time(N)] from entry-point "X"` (from LAST). DEFAULT is implicit.

**Port — entry-point = a routing DIMENSION (a fact carries an EP tag, a
pattern carries an EP tag, the fact enters the pattern iff they match).**
- **Parser** (`drl.rs`): `entry-point` hyphenated keyword (lexer, like
  `no-loop`); `from entry-point "name"` accepted in `pattern()` (falls through
  carrying `Pattern.entry_point`) AND after a window in `accumulate_pattern()`
  (the windowed source's EP trails the window).
- **Routing** (`engine.rs`): `CompiledPattern.entry_point: u32` (interned;
  0=DEFAULT); per-fact `fact_eps: Vec<u32>` (sparse, DEFAULT/RHS/synthetic=0);
  the single choke point is `alpha_passes` — one added clause `fact_ep(f) ==
  pat.entry_point`, so ALL routing (insert/update/delete/accumulate/not/join)
  and node-sharing partition by EP for free. `pattern_key` folds `|e{ep}` (a
  uniform `e0` on the all-DEFAULT corpus → grouping unchanged). `insert` split
  into `insert_default` (store) + `after_insert` (schedule+route) so
  `insert_into(type,fields,ep)` sets the fact's EP tag BEFORE routing.
  Registration: `ep_ids` interned at compile from rule references; an insert
  into an unreferenced name errors (faithful — both engines reject). The final
  WM `facts()` dump EXCLUDES named-EP facts (mirrors `session.getObjects()` =
  DEFAULT only). Reset clears `fact_eps` (the compiled EP table survives).
- **Runner**: reads the fact/action `entry_point` field → `insert_into`.
- **Oracle** (`OracleRunner.java`): `insertFact` routes to
  `getEntryPoint(ep).insert`; an `epMap` (handle→EntryPoint) routes
  update/delete through the fact's EP (session.update/delete on a named-EP
  handle throws "Invalid Entry Point"). Corpus-inert (default inserts →
  default EP).

**Mutation×EP (Bryan's scope):** update/delete of an EP-inserted fact composes
— `nth_inserted` spans EP inserts (oracle `objectInserted` fires per-EP, same
index), and `on_update`/`on_delete` route EP-filtered via `alpha_passes`
(ep_del/ep_upd_leave/ep_upd_enter/ep_del_mixed). The item-C class-1/2/3 fences
apply per-EP unchanged (an EP event in a temporal join / evicted / exists-churn
recurs identically; the same fuzz fences cover it).

**Gates:** baseline 11 / probes 822 / regressions 281 byte-identical; lint
1195; 9 suites. 18 `pr_cep_ep_*` promoted (basic partition + compositions +
mutation×EP; 4 isolation pins `expect_inert`), `ep_unref` engine_fenced
(out-of-subset). **Fuzz:** `fuzz_cep.py` extended with a per-type EP dimension
(each scenario partitions its event types across DEFAULT/S1/S2 — EP × every
existing composition + mutation, reusing the class-1/2/3 fences); a fact only
routes to a RULE-REFERENCED EP (else out-of-subset — both engines correctly
reject, validating registration). 3×1000 (seeds 21/22/23): **ZERO EP-caused
issues** — every find (5 value divergences + 1 non-termination) bisected to
HEAD (fails/hangs on the pre-item-D engine; strip-EP still fails), i.e. the
known temporal-join-order / accumulate-match latent family the CEP fuzz
occasionally hits (cf7x121 precedent), NOT item D. The 1 non-termination is a
PRE-EXISTING temporal+delete engine spin in `next_activation`/`fire_all` (the
fire limit can't catch it); repro
`scenarios/hang-backlog/pre_existing_temporal_delete_hang` (un-gated) for the
E1-hardening backlog. `fuzz_cep.py` gained a BATCH_TIMEOUT hang-guard (bisect +
record, don't wedge — the memory HANG protocol).
Blast-radius `make fuzz` (insert-path refactor: seeds 42/7 — only delete-free
pre-existing main-axis latents, zero regressions). Certified corpus byte-
identical throughout (EP gated — all-DEFAULT scenarios take the `e0`/fact_ep=0
paths unchanged). **Artifacts:** `drl.rs` (lexer + Pattern.entry_point +
from-entry-point parse), `engine.rs` (CompiledPattern.entry_point, entry_points/
ep_ids/fact_eps, intern_ep/fact_ep, insert_into/insert_default/after_insert,
alpha_passes EP clause, pattern_key fold, facts() filter, reset), `runner.rs`,
`OracleRunner.java` (insertFact + epMap), `fuzz_cep.py` EP dimension,
`pr_cep_ep_*` (18), `probes_pending/entrypoint/ep_unref`. Plan/recon:
`~/.claude/plans/cep-e2-item-d.md`.

### D-117: E1-hardening — NON-TERMINATION spin-guard (backstop, not a root-cause fix); Bryan-ruled before item E

The item-D EP fuzz flushed a PRE-EXISTING engine non-termination (bisected to
HEAD; `scenarios/hang-backlog/pre_existing_temporal_delete_hang`): a
temporal-join + external-delete + advance + TMS(`insertLogical`) shape spins
forever in `next_activation` — the fire limit can't catch it because no *fire*
completes; the cycle is the TMS `exp_deferred`/`deferred` re-add drain (a rule's
`tms_on_terminal_del` re-adds a deferred entry for itself, so the drain
`while let` never empties). D-080/D-106 envelope — the memory bars patching the
halt-model semantics locally. **Bryan ruled: backstop the hang before item E**
(a hang is a robustness bug qualitatively worse than a value divergence; the
deep temporal-join-order value class stays deferred).

**Backstop:** a per-`next_activation`-CALL step counter `spin_guard` +
`spin_tick()` guarding the two TMS deferred-drain `while let`s and the main
agenda `loop`. Past `AGENDA_SPIN_LIMIT` (50M — a huge margin: one legit call's
work is bounded by agenda size + deferred size, at most a few million) it sets
`pending_err` and returns `None`, which `fire_all` already surfaces as an error.
So the engine now ALWAYS TERMINATES — a genuine re-add cycle ERRORS (~18s)
instead of hanging. NOT a semantic fix: the underlying cycle is unchanged (the
repro stays a divergence, un-gated in `scenarios/hang-backlog/`); the root-cause
fix is E1-hardening. Per-CALL (not per-fire_all) budget so a large legit
multi-fire session never accumulates toward the limit. Corpus byte-identical
(11/822/281 — the guard never trips a legitimate scenario); lint 1195; 9 suites.
**Artifacts:** `engine.rs` (`spin_guard` field, `spin_tick`, per-call reset +
3 loop guards); `scenarios/hang-backlog/README` updated. **NEXT: E2 item E
(`@duration` interval events) — the last E2 fence item (walled, DECISIONS:4529).**

### D-118: @duration INTERVAL events (CEP E2 item E, the LAST E2 fence) — recon PINNED, PRE-implementation (awaiting Bryan's port gate)
**Probe-first recon complete; NO engine change yet** (engine still walls
`@duration`; oracle plumbing added is corpus-inert). 57 oracle probes
(`probes_pending/cep/e_recon/`, 6 generators), 3× key-discriminator stable,
probes tier 822/822 byte-identical with the rebuilt oracle.

**THE UNIFYING MODEL (one conceptual change).** `declare T @role(event)
@duration(f)` makes T an INTERVAL event occupying `[ts, ts+f]` instead of a
point `[ts, ts]`. The ENTIRE feature is: **`endTS = ts + dur`** (was `ts`).
Every existing consumer of an event's "end" already exists and already uses
`ts` — making it `ts+dur` IS the feature. Point events / any type with no
`@duration` ⇒ dur=0 ⇒ `endTS==ts` ⇒ byte-identical (corpus-preservation anchor,
proven: `@duration(0)` result is byte-identical to the point control).

**SOLID PINS (oracle ground truth):**
- **Temporal `after`/`before` measure later.START − earlier.END** where END =
  ts+dur. `$b:B(this after[lo,hi] $a)`: self=B later, anchor=A earlier ⇒
  distance = `B.ts − (A.ts + A.dur)`. Only the EARLIER event's duration enters;
  the later event's duration is IRRELEVANT (2×2: `e_p1_ip` FIRE / `e_p1_pi`
  inert / `e_p1_ii`==`ip`). `before` mirrors: self=B earlier ⇒ `A.ts −
  (B.ts + B.dur)` (`al_before_int_fire`). Bounds INCLUSIVE both ends
  (`[100,100]` fires, `[101,200]`/`[0,99]` inert). Endpoint EXACT, no ±1
  (dur=30 ⇒ `[70,70]` fires, `[69,69]`/`[71,71]` inert).
- **Expiration uses END + the same +1 quirk.** Explicit `@expires(X)`: event
  removed when clock > ts+dur+X, i.e. at `ts+dur+X+1`. Point (dur=0) ⇒
  `ts+X+1` (`ex_pt_exp100`: alive@100, gone@101 — the D-102/D-109 +1). Interval
  dur=50 exp=100 ⇒ alive@150, gone@151 (`ex_int_d50_exp100_at150/151`).
- **A→E SEAM: the D-109 inferred offset is UNCHANGED by duration; only its
  APPLICATION shifts to the end.** Interval dur=50, no `@expires`, earlier in
  `after[0,100]` ⇒ inferred offset 100 (NOT 150) applied from ts+dur ⇒ gone@151
  (`i2_int_d50_off100_at151`); point control gone@101 — difference is EXACTLY
  dur. So `infer_event_expiry` needs NO change; only the scheduler's deadline
  gains `+dur`. (Probe method note: observe expiry via the FACTS multiset, NOT
  a `not E()`/bare `E()` rule — a non-temporal reference LEAKS the inference to
  NEVER and hid the result on the first pass; explicit `@expires` is immune, a8.)
- **`not`/`exists` over an interval anchor use END and compose** (`cp_exists_*`,
  `cp2_not_*_adv`: interval A end=130 ⇒ B@200∈[190,210] matches; point A end=100
  ⇒ B∉[160,180]). (`not`+temporal needs a clock advance to CLOSE the window —
  pre-existing deferral, not an E signal.)
- **`window:time` COMPOSES** with interval events (no error); the precise
  start-vs-end MEMBERSHIP/eviction boundary is an OPEN B×E sub-question deferred
  to the port (needs a count-observation probe; the initial-fire log can't see
  eviction).
- **Mutation × duration:** updating the `@duration` field does NOT change the
  interval — Drools reads it once at insert, like `@timestamp`
  (`cp_mut_dur_0to30` stays inert after dur 0→30). Falls in the item-C
  mutation-re-propagation FENCE; symmetric with `@timestamp` (which the engine
  re-reads from the store today, byte-identical because the corpus never mutates
  timestamps).
- **FULL ALLEN ALGEBRA is available in Drools over `@duration` intervals** and
  all ops are duration-sensitive: `during`/`overlaps`/`coincides`/`meets` each
  FIRE on the relation and inert on a near-miss; `during` is interval-ONLY
  (points ⇒ inert). The Seine parser currently accepts ONLY `after`/`before`
  (drl.rs:1389-90, both mandate `[lo,hi]`). ⇒ **Q1 SCOPE QUESTION.**

**THE THREE GATE QUESTIONS (for Bryan):**
- **Q1 (operator scope):** port only `after`/`before` extended to intervals
  (the subset's current ops — minimal, natural, covers the pinned semantics), OR
  also add the Allen ops (`during`/`overlaps`/`coincides`/`includes`/`meets`/
  `starts`/`finishes` + inverses)? Allen ops are net-new parser + a new
  `Constraint`/`Test` representation (several take no `[lo,hi]`). **Recommend:
  after/before-to-intervals now; Allen ops as an explicit follow-on if wanted.**
- **Q2 (A→E inference seam):** confirmed — inferred offset UNCHANGED, applied
  from `ts+dur`. Port = `schedule_expiration` deadline gains `+dur`;
  `infer_event_expiry` untouched. (Informational; recommend accept.)
- **Q3 (corpus preservation):** `@duration(0) ≡ point` byte-identical — GATE on
  it (no `@duration` ⇒ dur_fi None ⇒ dur=0 everywhere). Non-negotiable.

**⟶ GATE RULING (Bryan, 2026-07-08):** **Q1 = ADD THE FULL ALLEN ALGEBRA**
(not after/before-only) — the port covers `during`/`overlaps`/`coincides`/
`meets`/`includes`/`starts`/`finishes` + inverses. **Q2 accepted** (inferred
offset unchanged, apply from `ts+dur`). **Q3 gated** (dur=0≡point,
non-negotiable). **CONSEQUENCE:** the recon above SAMPLED only 4 Allen ops
(existence + duration-sensitivity); a faithful port needs the FULL per-operator
recon FIRST — exact direction (which side is `this` vs anchor), optional
parameters (`overlaps[maxDist]`, `meets[dist]`, `coincides[sDev,eDev]`,
`during`/`finishes`/`starts` param forms), boundary inclusivity, and which
endpoints each compares (Allen's 13 relations over `[start,end]`). ⇒ **NEXT =
Allen-operator recon ladder (D-119), THEN the port.** No further SCOPE gate
needed (Bryan ruled scope); the Allen recon is pure probe-first detail.

**PROPOSED SEINE PORT (after/before core — surgical; Allen ops layer on top
post-D-119):**
- **Schema:** event object gains optional `"duration": fieldName`.
- **Oracle:** DONE (`declareBlocks` renders `@duration(f)`; corpus-inert).
- **`event_specs`** (`HashMap<TypeId,(usize,Option<i64>)>`, engine.rs:1012):
  add the duration field-idx ⇒ `(ts_fi, Option<expires>, Option<dur_fi>)` (or a
  small `EventSpec` struct). `declare_event` (3234) gains `duration:
  Option<&str>`.
- **`Test::Temporal`** (enum 94; compile 2214-2266; EVAL at **6892-6900 and
  6988**): carry `self_dur_fi`/`anchor_dur_fi` (resolved from `event_specs` at
  compile; None⇒0). Eval subtracts the EARLIER event's duration — `after`:
  `d = own − (a + anchor_dur)`; `before`: `d = a − (own + self_dur)`. Two eval
  sites, same change.
- **`schedule_expiration`** (3393; deadline `ts + exp + plus` at 3410): ⇒
  `ts + dur + exp + plus`. `infer_event_expiry` (3371) UNCHANGED.
- **`runner.rs`** (event-object read) + **`bindings`** (`declare_event`): pass
  the duration field (item D touched both).
- **FENCE:** Allen ops (pending Q1); window:time membership start-vs-end
  (pending B×E sub-probe); mutation×duration (item-C fence).

**Artifacts:** 57 recon probes + 6 generators `probes_pending/cep/e_recon/`
(oracle-only, engine-walled). Oracle `OracleRunner.declareBlocks` +
`@duration` branch (rebuilt, corpus-inert; probes 822/822). **Gate to green
(post-Bryan):** port per above, promote a boundary ladder to
`scenarios/probes/` (2×2 + boundary + expiry ±1 + A→E seam + dur=0 anchor +
not/exists compose), `make diff` byte-identical, extend `tools/fuzz_cep.py`
(a per-type `@duration` dimension + advances straddling `ts+dur+offset`), 3×1000
fresh-seed at 0 divergences, `make lint-probes` clean. **This closes the CEP E2
fence** (A–E all resolved); remaining deferrals = item-C re-propagation port +
E1-hardening backlog.

### D-119: Allen-algebra operators (item E, Bryan Q1 = full Allen) — PREDICATES + PARAMS pinned; the @expires-INFERENCE reach per op is a counterintuitive OPEN surface (scope question)
**Probe-first recon of the 11 Allen ops beyond after/before** (Bryan ruled Q1 =
add the full algebra, D-118). 62 oracle probes `probes_pending/cep/e_allen/`
(3 generators). Convention: `$a:A() $b:B(this <op> $a)` reads **"B `op` A"** —
**`this`=B is the SUBJECT, `$a`=A the OBJECT** (cross-checks `xdir_*`: a
during-config under `includes` and an includes-config under `during` are BOTH
inert ⇒ the ops are directional, not symmetric). Endpoints: `Xs=X.ts`,
`Xe=X.ts+X.dur` (the D-118 `endTS=ts+dur`).

**BARE PREDICATES (all strict `<`/`==`, no tolerance) — full matrix pinned:**
| op (B op A)   | predicate                | op (B op A)   | predicate                |
|---------------|--------------------------|---------------|--------------------------|
| coincides     | Bs==As ∧ Be==Ae          | during        | As<Bs ∧ Be<Ae            |
| meets         | Be==As                   | includes      | Bs<As ∧ Ae<Be            |
| metby         | Bs==Ae                   | starts        | Bs==As ∧ Be<Ae           |
| overlaps      | Bs<As<Be<Ae              | startedby     | Bs==As ∧ Be>Ae           |
| overlappedby  | As<Bs<Ae<Be              | finishes      | Be==Ae ∧ Bs>As           |
| after[l,h]    | l ≤ Bs−Ae ≤ h            | finishedby    | Be==Ae ∧ Bs<As           |
| before[l,h]   | l ≤ As−Be ≤ h            |               |                          |

**PARAMETERIZED forms (each bounds a specific distance; boundary INCLUSIVE):**
- `coincides[dev]` ⇒ |Bs−As|≤dev ∧ |Be−Ae|≤dev; `coincides[sDev,eDev]` ⇒
  |Bs−As|≤sDev ∧ |Be−Ae|≤eDev (`coincides_2dev_*`).
- `meets[dev]` ⇒ |Be−As|≤dev; `metby[dev]` ⇒ |Bs−Ae|≤dev.
- `starts[dev]`/`startedby[dev]` ⇒ |Bs−As|≤dev (+ the Be side); `finishes[dev]`/
  `finishedby[dev]` ⇒ |Be−Ae|≤dev (+ the Bs side).
- `overlaps[max]` ⇒ overlap `Be−As` ≤ max; `overlaps[min,max]` ⇒
  min ≤ Be−As ≤ max (`overlaps_min_*`). (overlappedby symmetric on `Ae−Bs`.)
- `during[max]` ⇒ dS≤max ∧ dE≤max; `during[min,max]` ⇒ both in [min,max];
  `during[lo1,hi1,lo2,hi2]` ⇒ **dS∈[lo1,hi1] ∧ dE∈[lo2,hi2]** where
  **dS=Bs−As (start-dist), dE=Ae−Be (end-dist)** — the asym probe
  (`during_4p_asym_ok` fires, `_swap` inert) fixes which pair is start vs end.
  (includes symmetric with A,B swapped: dS=As−Bs, dE=Be−Ae.)

**⚠ OPEN SURFACE — the @expires INFERENCE reach through Allen ops is
OP-SPECIFIC and COUNTERINTUITIVE** (smoke test `inf_*`, insert-one-event +
advance 100000, observe presence): **coincides → FINITE; overlaps → FINITE
(even BARE); during → NEVER (even PARAMETERIZED); meets → NEVER; finishes →
NEVER.** This does NOT follow "bare⇒never / param⇒finite" (param `during` still
leaks; bare `overlaps` bounds). FULL never/finite classification (`ic_*`,
insert-one + advance-100000, per POSITION — anchor `$a`=A vs subject `this`=B):
after/before/coincides/starts/startedby = FINITE both; during = never both;
meets/overlappedby/finishes = never(A)/FINITE(B); metby/overlaps/includes/
finishedby = FINITE(A)/never(B). Only `during` fully leaks; finite/never depends
on PARAMS too (the D-109 lo>0 leak: `after[0,100]` both finite because lo=0).
This IS the D-109 STP machinery GENERALIZED — each op's endpoint predicate ⇒
directed upperBound edges ⇒ Floyd-Warshall row-max ⇒ finite iff max_ub≥0; the
edges likely FALL OUT of the endpoint representation the port already needs
(derive-from-predicate), but the reach VALUES still need an oracle ladder to
verify. Each op's contribution pinned from the oracle per-op × param × position
— a dedicated ladder (mirrors D-109's after/before `earlier=hi`/lo>0-leak work).
NOT hand-derivable (flip-flops). **⇒ SCOPE
QUESTION for Bryan** (see below).

**PORT REPRESENTATION (design, for the after/before + Allen port):**
- Generalize `Constraint::Temporal`/`Test::Temporal` (engine.rs:94, 2214, eval
  6892/6988): replace `after: bool` with an `AllenOp` enum (13 variants) + a
  small param array (≤4 i64, op-specific defaults) + BOTH events' `(ts_fi,
  dur_fi)` (self already has own_fi; add self_dur_fi, anchor_ts_fi already =
  anchor.1, add anchor_dur_fi). Eval computes `As,Ae,Bs,Be` and applies the
  op's predicate — a single match over `AllenOp`. after/before stay the D-118
  `endTS` distance; all others are pure endpoint comparisons ⇒ the eval is
  branchy but shallow, no new machinery.
- Parser (drl.rs:1385-1404): generalize the `this <op>` match from {after,
  before} to the 13 keywords; parse the optional `[p1,..,pk]` (0-4 durations).
- Inference (temporal_edges, 2234-2255): **per-op STP edges — BLOCKED on the
  open inference-reach recon.** For after/before it stays as D-109.
- Node identity (`pattern_key` 1785): fold the op + params (two different Allen
  ops over the same binding must NOT share).

**Artifacts:** 62 probes `probes_pending/cep/e_allen/` (bare matrix
`*_fire`/near-misses, `*_param`/`during_4p_*`, `inf_*` smoke; 3 generators).
Predicates + params oracle-pinned; inference-reach is the one open recon item.
**NEXT: Bryan scope call on the inference reach (pin-all vs fence), then the
combined after/before+Allen port.**

**⟶ GATE RULING (Bryan, 2026-07-08): FENCE Allen-op @expires inference in
slab 1.** Land the Allen PREDICATE port (matching semantics) certified
byte-identical to Drools FIRST; inference stays D-109 (after/before only), and
Allen-op-referenced un-annotated event types are FENCED — require explicit
`@expires` (else `engine_fenced`/expected-divergence witnesses + a generator
gate excluding un-annotated types under Allen ops). The reach VALUES over Allen
ops are NOT pinned this slab. **PLUS an envisioned ENHANCEMENT (Bryan, noted for
AFTER certification):** once the product is certifiably faithful to Drools,
FULLY implement the Allen algebra *beyond* Drools — coherent interval semantics
where Drools is incomplete/incoherent (e.g. the `during`/`meets` inference leak,
any op×window/acc gaps). This is a DELIBERATE break from probe-first: there is
NO oracle for behavior Drools lacks, so it is SPEC-DRIVEN (Allen's interval
algebra as the spec), tracked in `docs/allen-beyond-drools.md`. NOT current
work — a post-faithfulness roadmap item. ⇒ **NEXT = the slab-1 port (predicates
+ params + `endTS=ts+dur`, Allen inference fenced) per the D-118/D-119 surface.**

### D-120: @duration INTERVAL events + full Allen PREDICATE algebra (CEP E2 item E) — PORTED, byte-identical; Allen @expires INFERENCE fenced. THE LAST E2 FENCE, CLOSED.
**The slab-1 port of D-118 (core) + D-119 (Allen) landed** per the handoff
(`~/.claude/plans/cep-e2-item-e-port.md`). Every event now occupies an interval
`[ts, ts+dur]` (`endTS = ts+dur`, dur=0 for points ⇒ BYTE-IDENTICAL, the Q3
gate). Certified: baseline 11 / probes **944** / regressions 281 byte-identical;
lint **1325 live/0 ghost/0 inert**; 9 Rust suites + a new `eval_allen` unit
module (bare matrix + params + directionality + point reduction).

**PORT SURFACE (as built):**
- `event_specs` value → an `EventSpec { ts_fi, expires, dur_fi }` struct (was a
  2-tuple); `declare_event` gains `duration: Option<&str>` → resolves an i64
  field-idx; `runner.rs` reads `event.duration`, `bindings` passes None (Python
  surface unchanged, like inference).
- `drl::AllenOp` (13 variants) + `AllenOp::from_keyword`/`arity_ok`.
  `Constraint::Temporal`/`Test::Temporal` carry `op` + `params: Vec<i64>` (0-4)
  + both events' `(ts_fi, dur_fi)`; parser generalizes `this <op>[p..]` to the 13
  keywords with per-op arity validation (after/before still mandate `[lo,hi]`).
- EVAL = a pure `eval_allen(op, params, Bs,Be,As,Ae)` (both join sites) applying
  the D-119 predicate table; helpers `overlap_bounds`/`during_bounds` fold the
  Drools default minDev=1 (bare during/overlaps = STRICT inside). after/before
  reduce to the E1 point delta when dur=0 ⇒ byte-identical.
- `pattern_key`: after/before keep the EXACT E1 string (node-sharing identity
  preserved); Allen ops fold `op+params+both dur-fi` (D-113 anti-mis-share).
- `schedule_expiration` deadline → `ts + dur + exp + plus`; `infer_event_expiry`
  UNCHANGED (D-118 Q2: the inferred offset is duration-independent, only its
  application shifts by +dur). The i2_* interval-inference seam is byte-identical.

**THE FENCE (Allen-op @expires inference — D-119 ruling):** the 11 NEW Allen ops
emit NO STP edge and do NOT register in `temporal_pos_type`, so an event type
referenced ONLY via an Allen op reads as a bare pattern and infers NEVER. This is
FAITHFUL for the never-classified ops (during, and each op's never-position) and
a DOCUMENTED divergence for the finite-classified ones. VERIFIED against the
oracle: exactly **17** ic_*/inf_* probes diverge (Seine keeps the event / Drools
expires it), and the set matches the D-119 per-op×position classification
EXACTLY — coincides/starts/startedby (both positions), meets/overlappedby/
finishes (keepB), metby/overlaps/includes/finishedby (keepA). after/before keep
full D-109 inference (not fenced). Mixed after/before+Allen types stay faithful
(the after/before edge already registers the type). Witnesses:
`scenarios/xfail/xf_cep_e_*` (open_divergence). Lift = the beyond-Drools /
full-inference follow-on (`docs/allen-beyond-drools.md`), NOT this slab.

**not/exists × temporal — kept WALLED (follow-on slab).** Recon (cp_*/cp2_*)
showed `exists`+temporal COMPOSES cleanly over intervals (END used, matching
byte-identical), but `not`+temporal has two unresolved gaps needing their own
recon ladder: (1) a window-CLOSE deferral (`not B(this after $a)` fires
immediately in Seine but Drools defers until the clock passes the window —
cp_not_pt_fire), and (2) the anchor A gets an inferred `@expires` THROUGH the
not-temporal in Drools but not in Seine's positive-only inference (cp2_not_*_adv).
A silent wrong firing is worse than a clean wall, so the E1 Positive-only wall
STAYS; the 6 cp_*/cp2_* recon probes are `engine_fenced`. Item E ports
POSITIVE-pattern intervals + the full Allen predicate set.

**window × interval — RESOLVED.** Windowed-interval EXPIRATION is byte-identical
(the endTS+window-offset folds in for free; `pr_cep_e_win_int_d50_at120/160`
fire/expire correctly). The recon's `cp_win_*` probes only failed on an UNRELATED
pre-existing parser wall (`accumulate($e:E() ...; count($e))` — a bound accumulate
source); marked `engine_fenced`. The B×E membership-during-window (count
observation mid-window) needs that accumulate form, a separate follow-on.

**Gate / promotion:** 145 recon probes resolved → **120 → `scenarios/probes/
pr_cep_e_*`** (byte-identical; near-miss + interval-inference probes carry
`expect_inert`) + **2** supported-form window probes; **17 → `scenarios/xfail/
xf_cep_e_*`** (the fenced inference witnesses); **8 `engine_fenced`** in place (6
not/exists + 2 accumulate-bound-source). `tools/fuzz_cep.py` extended: a per-type
`@duration` dimension (~45% intervals, dur=0 drawn) + the 11 Allen ops with valid
param arities, FENCED to explicit-`@expires` types only (an un-annotated type
under an Allen op would re-flag the known xfail divergence). 3×1000 fresh-seed
(101/202/303) surfaced **6 divergences, ALL bisect-to-HEAD PRE-EXISTING** — the
diverging rules are after/before-chain (temporal-join-order), not-CE+mutation
ordering, and accumulate-count (the E1-hardening backlog); each is BYTE-IDENTICAL
HEAD-vs-branch once made HEAD-parseable (strip @duration / convert Allen→after),
so **0 are attributable to the port**. RATE CONFIRMED PRE-EXISTING: HEAD's
UNMODIFIED `fuzz_cep.py` at the same seed 101 finds the SAME 2 divergences/1000
(the port neither introduces nor inflates the latent rate; it only reshapes which
instances the RNG stream lands on).

**⇒ CEP E2 FENCE CLOSED (A–E all resolved):** A @expires inference (D-109), B
windows (D-110–114), C event update/delete (D-115), D entry points (D-116), E
@duration intervals + Allen predicates (this entry). **Remaining deferrals:**
not/exists×temporal (follow-on), the beyond-Drools full-Allen enhancement
(`docs/allen-beyond-drools.md`), item-C re-propagation port, and the E1-hardening
backlog (temporal-join-order / accumulate-match latents + the D-117-guarded
non-termination).

---

## 2026-07-08 — CEP temporal-join-order discriminator HUNT (D-121)

### D-121: the temporal-join firing-ORDER latent is a FAMILY of interdependent batch-reversal facets — NO local fix is faithfulness-clean; Bryan GATE = commit to the drools-core temporal-staging SOURCES-PORT (push held until fixed)

**Ask (Bryan):** "start the hunt for the discriminator" — the D-082/D-083
join-order-provenance lineage, still open for CEP temporal nodes (the ~2/1000
CEP-fuzz flush, the standing push-block). Prior restart plan: model-check to a
UNIFIED discriminator. **Result: there is no single discriminator — it is a
family, and no LOCAL engine edit is faithfulness-clean.**

**Method (probe/model-check-first, per doctrine):** reproduced the golden
`e0last` repro; built an oracle BATTERY (`tools/cep_join_battery.py`, 33 cases:
E1-perm × anchor-position × multiplicity), a shuffled-insertion temporal-chain
FUZZ (`tools/fuzz_chain.py` — the probe that surfaced the finer facets; the
existing `fuzz_cep.py` under-shuffles), and a 2-node model-check
(`tools/model_check_chain.py`). Traced the engine end-to-end (SEINE_TRACE) and
read drools-core sources (PhreakJoinNode, TupleSetsImpl, RuleNetworkEvaluator).

**Root cause (sources-confirmed):** Drools orders temporal-join output by the
ARRIVAL PROVENANCE of each staged tuple (how it arrived — individually vs as a
reversed upstream batch; before vs after its anchor). `TupleSetsImpl.addInsert`
PREPENDS (LIFO staging); `PhreakJoinNode.doNode` runs doRightInserts THEN
doLeftInserts, each appending to memory in LIFO order and scanning the opposite
memory FORWARD; the reversal cascades node-to-node and its PARITY encodes
provenance. The Seine engine BATCHES inserts in `do_node` and COLLAPSES that
provenance — `e0first` and `e0last` reach node2 with IDENTICAL `sl.ins` yet must
fire OPPOSITELY.

**The four facets (each a real oracle divergence, golden repros in the battery):**
1. **Upstream left-batch stamp** (`e0last`): node2 reverses an upstream multi-fact
   left batch that is already in arrival order. (Battery Group A/B/D, 3-node
   chains, `before` mirror.)
2. **Anchor-first eager-vs-batch** (`e0first`, C_e0mid): anchor before some
   partners ⇒ opposite order from `e0last` despite identical node2 input ⇒ NO
   node2-level fix can separate them.
3. **Right-held arrival** (`ch7001x0`): held-right vs fresh-right ordering (the
   D-102 rel_arrival facet).
4. **Multi-anchor per-right grouping** (E_2e0_first, ≥2 anchors): oracle groups
   output per-RIGHT, engine per-anchor.

**NO-LOCAL-FIX proof (empirical; shuffled chain-fuzz vs a pristine HEAD worktree):**
- naive scan→memory swap (prior): regressed t1/t4/t5/t8/t15.
- **node2-only** (forward-stamp upstream multi-fact left batch): `make diff`
  BYTE-IDENTICAL (944) + fixed facet-1 (e0last, Group A/B/D, real CEP-fuzz latents
  cf50003x263/x894) BUT chain-fuzz vs HEAD = **+172 resolved / −121 NEW
  regressions** per 1500 (facet-2). Net fewer, but ~120 NEW Drools-divergences ⇒
  UNFAITHFUL.
- **node1 arrival-split** (emit [after-anchor DESC]++[before ASC] by right_sseq vs
  anchor left_sseq): fixed facets 1+2 (battery 32/33) BUT regressed facet-3;
  chain-fuzz EXPLODED to 542/1500. Reverted.
- Every knob fixes one batch-reversal direction and regresses its opposite. The
  corpus stays green only because it does not pin these shapes.
- **Engine REVERTED to HEAD (byte-identical); nothing landed.**

**Bryan GATE (2026-07-08):** (1) COMMIT to the drools-core temporal-staging
sources-port — reproduce Drools' per-flush staged-tuple ORDERING with arrival
provenance so the engine can distinguish e0first/e0last (the only
faithfulness-clean path); SCOPE + mechanism-report before touching the engine.
(2) HOLD the push (E2 arc D-109..D-120) until the latent is actually FIXED, not
merely characterized.

**Port scope (next, pre-engine):** the mechanism lives ABOVE `do_node` — in the
segment-linking + staged-tuple accumulation order (drools RuleNetworkEvaluator /
SegmentMemory / LeftInputAdapterNode; Seine's flush driver in `engine.rs`). Step
1 = pin the e0first/e0last staged-order provenance (extend the AccDump graft —
`oracle/.../AccDump.java` `dumpJoinNode` already dumps ltm/rtm/staged; add a
pre-`fireAllRules` dump + descend JoinNodes in `walk` — to capture staged
left/right + memory order BEFORE fire for both; D-086 reusable method) so the port
is source-grounded not hand-derived (hand-derivation flip-flopped repeatedly this
run). Harness = `tools/fuzz_chain.py`
vs a HEAD worktree (regression gate) + `tools/cep_join_battery.py` + `make diff`
(944). Faithfulness bar: ZERO new Drools-divergences.

### D-122: temporal-join SOURCES-PORT step 1 — the AccDump graft was BROKEN for temporal (CLOUD mode); FIXED to faithful; mechanism PINNED from ground truth; uniform v1 model DISPROVEN (27% population); Drools staged-tuple disciplines extracted for v2

Executed D-121 step 1 (pin e0first/e0last provenance from oracle ground truth,
not hand-derivation). Result: a faithful introspection graft, a sharp mechanism,
and a **disproven-but-instructive** first model. **Nothing landed in the engine;
push still HELD; gate untouched** (only diagnostic `AccDump.java` changed —
`make diff`/`cargo test` run through `OracleRunner`, not the graft).

**(1) The graft was silently UNFAITHFUL — fixed.** `oracle/.../AccDump.java`
(built for the non-temporal accumulate probes D-090a/092/094) declared event
types as PLAIN facts (no `@role(event)/@timestamp/@duration/@expires`) and built
the KieBase in **CLOUD** mode with a realtime clock. On the temporal battery it
(a) threw `DefaultFactHandle cannot be cast to EventHandle` at `fireAllRules`,
and (b) when forced past that, produced e0last = `25,23,26` — the CLOUD order,
which MATCHES the buggy engine, NOT the STREAM gate. Fix = mirror
`OracleRunner` EXACTLY: event annotations in `declareBlocks`, and `hasEvents ⇒
build(EventProcessingOption.STREAM) + pseudo-clock`. Also render left tuples as
their fact chain `(E0,E1..)` (the opaque `LeftTuple.toString` hid the joined
facts). Now byte-faithful to the gate: **e0first `25,23,26`, e0last `26,23,25`**.
LESSON: a diagnostic graft is only trustworthy once cross-checked against the
gate runner on the SAME input — CLOUD-vs-STREAM silently flips join order.

**(2) Mechanism PINNED (decoded ground truth, all 33 battery cases).**
Invariant: **firing order = reverse(node2 left-tuple-memory order)** — holds for
every case incl. multi-anchor (Group E, facet-4) and multi-right (Group D).
node1's RIGHT memory is **IDENTICAL** for e0first vs e0last (`[26,23,25]`) — so
the distinction is NOT in node1's memory; it is the **emission provenance** into
node2 (fact-handle recency + held-vs-eager). Held partners (arrived before their
anchor) drain as a reversed batch when the anchor left-inserts; eager partners
(after) append in arrival order. node2 ltm across the E0-position sweep:
e0first `[26,23,25]`, e0mid1 `[26,23,25]`, e0mid2 `[23,26,25]`, e0last
`[25,23,26]` — all = `reverse(held)++eager`. This is the D-082/D-083 "arrival
provenance" made exact and machine-checkable.

**(3) v1 model DISPROVEN — the curated-battery TRAP.** Encoded the above as a
uniform rule ("every beta insert scans the opposite memory in REVERSE, appends
emissions") in `tools/model_join_flush.py` (`battery` + `fuzz` modes, model
differed vs the gate oracle). It fits **33/33 curated** cases — but **diverges
on ~27 % of the random shuffled-insertion population** (400 cases seed 7001:
107 divergences; 200@7002 ≈ 26 %). EVERY failure has a DEEPER partner **HELD**
(e.g. E2 arriving before its E0/E1 context — facet-3, "right-held arrival") and
its interaction with the drain. So the 33-case battery MASKS the real surface;
`fuzz_chain`-style shuffled insertion is the honest bar. A memory-scan-direction
knob is provably NOT the faithful algorithm — confirming D-121's "family of
interdependent facets," now with a concrete failure class and % rate.

**(4) Drools staged-tuple disciplines extracted (drools-core 9.44 sources; the
ORACLE remains the arbiter).** `TupleSetsImpl.addInsert` **PREPENDS**
(`insertFirst = new`; staged output is LIFO). `TupleList.add` (node memory)
**APPENDS** (`this.last = new`; FIFO). `PhreakJoinNode.doLeftInserts`:
`ltm.add(lt)`, iterate `rtm` via `getFirstRightTuple`+`it.next`, each match →
`insertChildLeftTuple` → `trgLeftTuples.addInsert`. `doRightInserts`: symmetric
over `ltm`. So node2's memory order is the double-reversal of prepend-staged /
FIFO-memory across a segment flush, keyed by recency — NOT a scan flag. **v2
plan:** model the OTN/LIA staging into node1's src sets + the segment `doNode`
flush order + recency-ordered memory iteration, validate to **0 divergences** on
`model_join_flush.py fuzz` BEFORE porting to `engine.rs`. Only then touch the
engine (still under the D-121 push-hold).

**Artifacts:** `AccDump.java` faithful (committed); `tools/model_join_flush.py`
(v1 dead-end + reusable model↔oracle validator); battery/gate-oracle tables
regen via `tools/cep_join_battery.py gen` + `cargo run -p seine-harness --
oracle`. Prime directive held throughout: every claim here is oracle-probed, the
one hand-derived model was killed by the population differ.

### D-123: temporal-join firing-order — the FAITHFUL flush model (v2) CRACKED and VALIDATED (0 divergences on ~4300 shuffled cases incl. multi-anchor); the exact spec for the engine port

Step 2 done: built v2 in `tools/model_join_flush.py simulate()` from the source
disciplines (D-122) and validated it to **ZERO divergences vs the gate oracle**
on 33 curated + ~4300 random shuffled-insertion cases (single- AND multi-anchor).
This is the faithful algorithm — the port spec. **Still nothing in the engine;
push HELD.**

**The model (per-propagation phreak flush).** Each external fact propagates
depth-first the instant it arrives (Drools defers to `fireAllRules`, but replaying
the insertion queue one-at-a-time is behaviourally identical for firing order —
proven by the population differ). At a beta node:
- memory (`ltm`/`rtm`) is FIFO **append**; a match scans the OPPOSITE memory
  **FORWARD** (`getFirst`+`it.next`);
- each emitted child left-tuple is **PREPENDED** (`addInsert`) into the child's
  staged-left set; the child's `doLeftInserts` then reads that set in
  `getInsertFirst` order (= prepend order) and **appends** to its own memory;
- the terminal "fires" its staged set in `getInsertFirst` order.
**The crux:** a SINGLE emit (an eager individual insert — partner arrives while
its anchor is already in memory) is identity; a BATCH of N emits (an anchor
`doLeftInserts` draining N held partners) is reversed **exactly once** by the
prepend-then-getInsertFirst round-trip. e0first = all-eager (no reversal, node2
ltm `[26,23,25]`, fires `25,23,26`); e0last = one batch drain (reversed once,
ltm `[25,23,26]`, fires `26,23,25`); the held-right family (E2 before its E0/E1)
fires as the batch is processed, NOT as a later memory scan — which is exactly
what v1 got wrong.

**Why v1 died and v2 lives.** v1 baked the reversal into the SCAN DIRECTION
(scan opposite in reverse, append) — correct for a lone batch but wrong whenever
the count of emits per propagation isn't one-batch-at-the-end (held-right, mixed
eager/batch): 27 % population divergence. v2 puts the reversal where Drools does
— the staged `addInsert` prepend + `getInsertFirst` read — so it's automatically
identity for singletons and one-shot for batches, at every depth. Same 33 curated
cases, but 0 % vs 27 % on the population. LESSON (banked): a rule that fits the
curated battery proves nothing; the shuffled-insertion population differ is the
only honest bar — and the reversal is a STAGING artifact, not a scan flag.

**Validation:** `model_join_flush.py battery` (33/33) + `fuzz <n> <seed>`
(single-anchor: seeds 7001/7002/7003/8001/9999 @400 + 12345@1500, all 0) +
`fuzzm <n> <seed>` (multi-anchor facet-4: 5001/5002/5003@400, all 0). The
multi-anchor `fuzzm` matters: facet-4 is where the prior ENGINE node1-arrival-
split blew up 542/1500 (D-121) — v2 is clean there.

**NEXT (step 3 — the engine port, under the push-hold).** Translate the cascade
into the Seine flush driver (`engine.rs`, the `do_node` caller / staging — NOT a
`phreak.rs` scan tweak): reproduce staged `addInsert`-prepend + `getInsertFirst`
processing + FIFO memory + forward opposite-scan, so held-vs-eager provenance is
preserved. Re-cert bar: `make diff` (944) byte-identical AND `fuzz_chain.py` vs a
HEAD worktree = 0 regressions AND the CEP-fuzz latents (cf50003x263/x894) resolve.
`model_join_flush.py` is the executable spec; port until the engine matches it.

### D-124: engine-port RECON — the divergence is FLUSH GRANULARITY (drain + batched single-flush), not a node-local order; the port is an architectural change to temporal-node flush, matching v2's per-propagation cascade

Traced the engine (`SEINE_TRACE=1`) on `C_e0first`/`C_e0last` to pin WHERE it
departs from the validated v2 model, BEFORE editing (3 prior local fixes failed —
D-121). Finding: **the engine collapses e0first and e0last to identical node
inputs, so no node-local edit can separate them — confirmed at the trace level.**

- **node0** (`E0⋈E1`): for BOTH cases `do_node[0]` sees `sl.ins=[E0], sr.ins=[]`
  — the three E1s were already drained to right MEMORY in arrival order
  `[26,23,25]` (D-101/D-102 self-drain), and node0's single deferred flush does
  `doLeftInserts(E0)` over that memory, emitting `[25,23,26]` IDENTICALLY in both.
- **node1** (`(E0,E1)⋈E2`): consequently `sl.ins=[25,23,26]` + `sr.ins=[E2]`,
  BYTE-IDENTICAL for e0first and e0last. doNode runs doRightInserts(E2) vs an
  empty ltm (nothing), then doLeftInserts emits in sl.ins order ⇒ engine fires
  `25,23,26` for BOTH. Oracle: e0first `25,23,26` (matches by structure), e0last
  `26,23,25` (diverges).

**Root cause (matches v2's contrast):** v2 processes each insert as its OWN full
propagation. In e0first the E1s arrive while E0 is present ⇒ EAGER right-inserts
that append to node1.ltm as `[26,23,25]`; the later E2 right-insert then
prepend-reverses ⇒ `25,23,26`. In e0last the E1s are held, drained as ONE batch ⇒
node1.ltm `[25,23,26]` ⇒ E2 prepend-reverse ⇒ `26,23,25`. The engine's DRAIN +
deferred SINGLE flush erases the eager-vs-held provenance at node0, so both look
like e0last's batch — but it emits the batch WITHOUT the E2-right-insert
prepend-reversal (E2 is batched into the same do_node, processed before ltm
fills), so it lands on `25,23,26` (e0first's answer) for both.

**⇒ The port is NOT a scan/stamp tweak; it is flush GRANULARITY.** The engine
must process temporal-node propagations so that (a) an eager partner joins a
present anchor INDIVIDUALLY (not drained to memory then batch-joined), and (b) a
later same-node right-insert reverses via the staged prepend — i.e. reproduce
v2's per-propagation cascade for temporal nodes. This is the drain/self-drain +
segment-flush path in `engine.rs` (D-101/D-102 machinery), NOT `phreak.rs`
`do_join_node` alone. High-risk (the exact area of the 542/1500 facet-3 blowup);
do it in a focused pass with `model_join_flush.py` as the spec and `make diff`
(944) + `fuzz_chain` vs a HEAD worktree as the twin gates. RECON only this
checkpoint — engine still at HEAD.

### D-125: THE ENGINE PORT LANDED — per-arrival temporal flush cascade (v2 model), all gates green: chain-fuzz 204→0/1500, 0 NEW regressions, both cf50003 latents resolved

The D-121→D-124 arc closes. The port is TWO coupled pieces (each alone was a
proven dead end; together they equal v2):

**P1 — the per-arrival cascade** (`engine.rs stream_flush_ex`, replacing the
unconditional temporal self-drain loop; `phreak.rs Node::flush_ins_delta`): at
each per-insert flush, an ELIGIBLE temporal join node consumes its staged
INSERTS per-arrival instead of draining them blind to memory —
- a staged RIGHT (arrival order = `.rev()` of the prepend list) appends to
  memory and, if the left memory is populated, EAGER-JOINS it individually,
  scanning lefts in MEMORY order and prepending emissions (`addInsert`);
- a staged LEFT (getInsertFirst order — `s0_in` folds first, then `s_left`)
  appends to memory with **lseq stamped in that SAME staged order** and joins
  the right memory in memory order;
- emissions route to the node's single sink (`append_into_pending`, the walk's
  first-sink discipline); ascending `ni` = parents before children, so a
  same-flush cascade completes (node0's eager emit fills node1 this flush).
So a lone eager emit is identity and an anchor draining N held partners
reverses EXACTLY once (the staged prepend) — v2's whole game. Eager-vs-held
provenance now lives in the CHILD's memory + lseq order, which the certified
rel_arrival right-insert scan then reads back naturally (post-set empty,
pre-set sorted (lsq=0, lseq)).

**Eligibility (else the certified legacy `self_drain_delta`, unchanged):**
unshared `Kind::Join` temporal node, insert-only staging (no upd/del on
s0_in/s_left/s_right), NOT both sides staged (AB self-join shapes keep the
D-102 per-fact walk), no ph=1 rights, exactly one Node/Term sink (Term only
when no emission is possible — an emitting Term-sinked node is linked, and a
linked rule's flush eval already consumed the staging), never RIA. Shared
temporal nodes keep the whole D-102 cf101x* stash machinery.

**P2 — the fill stamp** (`phreak.rs do_join_node`, temporal non-AB fill): an
UNSHARED temporal fill stamps lseq in STAGED (getInsertFirst) order instead of
`.rev()` — a genuine anchor-drain batch keeps its single reversal for LATER
right-insert partner scans (the linked-eval case P1 never sees: anchor arrival
links the rule, the flush eval consumes the batch, a later E2' joins by lseq).
Shared nodes keep the arrival stamp. This is the disproven "node2-only forward
stamp" — UNFAITHFUL at HEAD granularity (−121 NEW: eager singles arrived as
batches and got flipped), CORRECT under P1 (eager singles are per-arrival
singletons, order-free; only genuine batches remain multi-fact).

**Gates (ALL green):**
- `make diff`: baseline 11 + probes **947** + regressions **284** byte-identical
  (pre-existing 944 untouched; +3 promoted probes, +3 graduated regressions).
- `fuzz_chain` seed 7001: HEAD-worktree **204/1500** divergences → port
  **0/1500** = +204 resolved / **0 NEW**; seed 7002: **0/1500**.
- multi-anchor engine-vs-oracle (facet-4, `_gen_multi` shapes) seed 9001:
  **0/600**.
- CEP-fuzz FRESH seeds 60007/60013: **0/1200**, 0 hangs (windows/TMS/
  not-exists/accumulate/mutation composition clean).
- the two real CEP-fuzz latents **cf50003x263 / cf50003x894 PASS**.
- `model_join_flush.py battery` 33/33 (spec unchanged); engine-vs-oracle
  battery 33/33; `cargo test` 9 suites; lint 1331 live / 0 ghosts / 0 inert.

**Corpus adds:** promoted `pr_cep_tjo_e0first_eager_singles` /
`pr_cep_tjo_e0last_batch_reversal` (the golden pair, facets 2/1) /
`pr_cep_tjo_multi_anchor_per_right` (facet 4); graduated
`xf_cep_tjorder_chain_exists` (RESOLVED → regressions/) + `fz_tjo_7001_2node`
+ `fz_tjo_7001_3node` (resolved chain-fuzz representatives, incl. 3-node).

**Still open (unchanged scope):** `xf_cep_tjorder_dual_tms` stays xfail — a
SHARED temporal node (TJ0/TJ1 same pattern) composed with TMS; the cascade
deliberately bails there (Drools defers shared-segment evaluation to agenda
pops; eager flush-pairing is provably wrong — cf101x551 vs t14). That family
needs the agenda-pop composition arc, not this port. Item-C re-propagation,
not/exists×temporal, and the rest of the E1-hardening backlog are untouched.

**Lesson (uniform-fold signature, again):** the bug was "one drain applied
across distinguishable arrivals". The fix scoped the distinction (eager vs
held) instead of adding machinery — and the TWO dead-end "knobs" (forward
stamp; eager split) were each HALF of v2, unfaithful alone because the other
half's compensation was still in place. Model-first paid for itself: the port
was mechanical once `simulate()` was validated, and every ordering question
("which side scans what, in which order") had an executable answer.

### D-126: post-port FENCE SWEEP (recon) — nothing fenced in E2 lifts from D-125 as-is; exists×temporal is CLOSER but has its own multi-anchor admission-order family (2 new witnesses)

Bryan asked whether any E2 fence lifts now that the per-arrival flush landed.
Swept the whole fenced surface; answer: **no fence lifts outright, and every
still-failing signature is byte-for-byte the one that motivated its fence** —
the port changed exactly what it claimed and nothing else.

- **All 101 xfails still fail** (batch sweep): the 4 `xf_cep_c_*` (item-C
  re-propagation — mutation semantics, not join order), the 17 `xf_cep_e_*`
  Allen-expiry witnesses (inference fence, EXPECTED to diverge),
  `xf_cep_tjorder_dual_tms` (shared-node facet, known), the alu*/win_reset
  deliberate witnesses, and the old fz_* quarantines.
- **hang-backlog** `pre_existing_temporal_delete_hang`: still trips the D-117
  spin-guard (~18s error) — the cycle is the TMS `exp_deferred` re-add drain in
  `next_activation`, untouched by flush granularity.
- **`win2_*` walls**: parse/feature-level (`window:length` follow-on; standalone
  windows; the `cp_win_int_*` bound-source accumulate form) — confirmed error
  text, unreachable by an engine-semantics change.
- **not/exists×temporal (the D-120 fence) — scratch-worktree recon** (wall
  lifted at `engine.rs` ~2279 + STP edges made positional-only, recon-only,
  worktree removed): the D-120 signature reproduces EXACTLY on the ported
  engine — `cp_exists_*` + `cp_not_int_inert` PASS; `cp_not_pt_fire` still
  engine-1-vs-oracle-0 (gap-1 window-close deferral, a NOT-node timer
  semantic); `cp2_not_*_adv` still engine-keeps-A (gap-2 inference through the
  not-temporal). Neither gap is flush-granularity.
- **NEW FINDING — the curated exists probes are another battery trap** (the v1
  lesson again): a 450-case shuffled population fuzz over exists×temporal
  shapes (`ex_partner`/`chain_ex`/`ex_mid`, explicit @expires) on the unfenced
  scratch found **10/450 divergences, ALL multi-anchor admission ORDER** — one
  exists-blocker admits two E0 anchors; the engine fires them in insertion
  order, the oracle most-recently-blocked-first (the `RightTuple.addBlocked`
  PREPEND analog in `do_existential_node`, which D-125 deliberately excludes —
  Kind::Join only). Witnesses saved as `probes_pending/cep/e_recon/
  cp_ex_multi_anchor_before` / `_after` (engine_fenced; lint 1333/0/0).

**⇒ Unwalling exists×temporal is now a CANDIDATE SLAB with a known work item:**
the existential analog of the per-arrival discipline (blocked-list admission
order), model-first like D-123 (a small exists-flush replica validated against
the oracle population BEFORE touching `do_existential_node`), plus the gap-2
inference question scoped out. `not`×temporal stays fenced on gap-1/gap-2
regardless. No engine change this checkpoint (recon only; gated tree
untouched — `make diff` 11+947+284 unaffected).

### D-127: exists×temporal ENGINE PORT landed — per-arrival existential admission (`exists_flush_admit`), all gates green; `not` stays fenced

The D-126 candidate closes. Model-first (D-123 discipline), then port, then the
fence lifted exists-only LAST.

**Model (`tools/model_exists_flush.py`, sibling of `model_join_flush.py`):** a
minimal network replica (positive joins + exists nodes) with the SAME phreak
disciplines as the validated v2 join model — node memory APPENDS, opposite
memory scanned FORWARD, emissions PREPEND into the child staged set (a batch of
N reverses exactly once). Exists specifics: a left is blocked by the FIRST
matching right; a RIGHT-insert blocks every matching unblocked left in memory
order and emits that batch REVERSED once; a LEFT-BATCH (one upstream join
emission) admits/parks then emits the admitted REVERSED once. **0-div vs the
gate oracle on the shuffled population** — 6+ seeds, ~2450 cases, incl. the
D-126 seed 11001 that gave the engine 10/450. Curated cases alone are
disqualifying (v1 lesson) — the population is the bar.

**What the engine did wrong (probe-measured, not derived):** the temporal
EXISTS node is NOT on the join per-arrival flush path (`flush_ins_delta`) — it
is processed POP-time/batched by `do_existential_node`. Two coupled defects:
(1) the cascade's `self_drain_delta` drained the exists node's `s_left` in
`.iter().rev()` order — REVERSING each upstream join-emission batch in memory —
and without checking blockers (so a blocker-before-left admission would be
LOST); (2) even with correct memory, the batched rightIns-then-leftIns phase
order emits admissions in insertion order, not the oracle's most-recently-
blocked-first. Ground-truth traces: `cp_ex_multi_anchor_before` engine
[E0@1,E0@5] vs oracle [E0@5,E0@1]; interleaved `E0@1,E0@2,E1@50,E0@3,E1@51`
engine [1,2,3] vs oracle/model [2,1,3] (a PARTIAL reversal — proves per-arrival
is required, a naive reorder cannot reproduce it).

**Port (two sites, both gated to temporal `Kind::Exists`; `not` and non-
temporal exists byte-identical):**
1. `phreak.rs do_existential_node` — for a pure-insert batch, `exists_flush_
   admit` replays the staged inserts in ARRIVAL order. FactIds are monotonic
   with insertion, so a left tuple "arrives" at its max FactId (completing
   fact); staged lefts sharing a completing fact are ONE upstream join emission
   (kept in staged order = the join's own single reversal) and admit/emit as a
   reversed batch; rights arrive at their own id and block memory lefts (also
   reversed). All keys distinct ⇒ a plain sort, independent of staged list
   order (s0_in prepends, s_left appends). Emissions staged so `trg.ins`
   (getInsertFirst = the static-salience FIFO firing order) equals the replay.
2. `engine.rs stream_flush_ex` cascade — a temporal exists node no longer
   `self_drain_delta`s; its full staged history flows to the eval, where (1)
   reconstructs the order. (Lazy-PHREAK: leaving staging in place is correct.)

**Deletes are out of scope BY PROOF, not omission:** for `exists`, a right-
delete only takes a left blocked→unblocked (a child retraction) or blocked→
still-blocked (nothing) — it can NEVER fire. Retractions don't append to the
firing log, so exists blocker-delete/re-admit ordering is UNOBSERVABLE in the
differential (oracle-confirmed: `ex_del_multi`/`ex_del_readmit` add no firing).
So the port is correctly insert-only; `fuzz_exists_temporal.py` (insert-only)
is the complete gate, and `fuzz_cep` never pairs a temporal constraint with
not/exists so it is unaffected.

**Fence lift (LAST):** `engine.rs` ~2279 now errors only on `CeKind::Not`
(`exists` allowed); the after/before STP-inference edge (~2316) is guarded on
`tpos.is_some()` so a POSITIONLESS exists records no @expires-inference edge —
inference-through-an-exists stays out of scope (explicit @expires only, D-126).

**Witnesses promoted:** `cp_ex_multi_anchor_{before,after}`, `cp_exists_int_
fire`, `cp_exists_pt_inert` → `scenarios/probes/pr_cep_e_ex*` / `pr_cep_e_
exists_*` (engine_fenced dropped; join the differential). The four
`cp_not*`/`cp2_not*` and two `cp_win*` stay engine_fenced (walls up).

**Gates (all green):** `make diff` **11 / 951 / 284** byte-identical (was 947;
+4 promoted) · `fuzz_exists_temporal` 0-div on 9 seeds (~3.4k cases) ·
`fuzz_chain` 0-div (D-125 join population intact) · fresh `fuzz_cep` clean (one
divergence, `cf313x13` firing[12] on non-temporal `not E2() P()`, reproduces
IDENTICALLY with the slab stashed — a pre-existing latent, NOT this slab) ·
`cargo test` 9 suites · `make lint-probes` 1333 live·0·0 · bindings pytest 72.

**Still fenced (unchanged):** `not`×temporal (gap-1 window-close deferral, gap-2
anchor-@expires-through-the-not — neither is admission order); @expires
INFERENCE through an exists; shared temporal nodes (legacy paths). These are
follow-ons.

### D-128: not×temporal RECON (staging the next slab) — it is NOT an admission-order port; the window-close firing DEFERRAL dominates (~30% population), so the timer + inference arcs are prerequisites

Bryan asked to stage `not`×temporal next. Probe-first, like the D-126 recon
that staged exists (which taught: MEASURE the population, don't trust curated
gaps). Built `tools/fuzz_not_temporal.py` (sibling of `fuzz_exists_temporal.py`:
`not` shapes `not_partner`/`chain_not`/`not_mid`, plus a coin-flip trailing
`advance` for the deferral axis and coin-flip explicit `@expires` for the
inference axis). Lifted the `not` fence in a scratch (`if false && p.ce ==
CeKind::Not`; the STP edge is already `tpos.is_some()`-guarded so positionless
`not` records no inference edge) and measured.

**Finding: ~27–31% divergence across seeds 5001/5002/5003** — an order of
magnitude worse than exists (D-127 was a clean 2%, one admission-order family).
`not`×temporal is NOT a clean slab. The divergences are NOT admission order;
they are the two D-120 gaps, and the FIRST one pervades everything:

- **gap-1, window-close firing DEFERRAL (the dominant driver).** Seine
  propagates a `not` the moment it is satisfied SO FAR; Drools DEFERS the
  firing until the pseudo-clock proves the window closed (no later blocker can
  arrive). Without an `advance` the clock never passes the window, so Drools
  fires 0 where Seine fires ≥1. Even PURE cases (no advance, no @expires)
  diverge on firing COUNT and ORDER — e.g. `nt5001x204` (chain `not E2(after
  $b)`) engine 2 vs oracle 1 with `firing[0]` order differing; `nt5001x171`
  (`not E1(before[0,50] $a)`, both E1s outside the window) engine 1 vs oracle 0
  — identical signature to the toy witness `cp_not_pt_fire`. Witness saved:
  `probes_pending/cep/e_recon/cp_not_chain_defer` (engine_fenced).
- **gap-2, @expires INFERENCE through the not.** On `advance`, Drools infers the
  anchor's expiry through the not-temporal and expires it (Seine keeps it) —
  the `cp2_not_*_adv` signature. 64/81 divergent cases (seed 5001) carried an
  advance (deferral+inference), only 10/81 were pure (neither) — but those 10
  still diverge, confirming the deferral is not advance-gated, it is the
  baseline `not` timing.

**⇒ not×temporal is a HARD slab with two PREREQUISITE arcs, neither admission
order:** (A) a window-close firing DEFERRAL scheduler — a `not` satisfied only
"so far" must be held on a timer until the clock retires the window (a genuine
temporal-scheduling machine, unlike D-127's per-arrival reordering); and (B)
@expires inference through a not-temporal (the positive-only inference extended
to the not path — related to the parked exists-inference question). The D-127
`exists_flush_admit` machinery does NOT transfer (it reorders admissions; here
the problem is WHEN a satisfied not fires, not in what order). Staged, not
started. No engine change (recon only; fence reverted, gated tree
`make diff` 11/951/284 unaffected; `cp_not_chain_defer` joins the fenced set,
lint 1334/0/0).

### D-129: not×temporal cold-start — arc A (firing DEFERRAL) semantic PINNED + validated model (`model_not_defer.py`, 0-div on the not_partner population)

Cold-started the not slab (Bryan). Probe-first on the dominant arc — the
window-close firing deferral — then model-first (D-123 discipline). Two things
the recon (D-128) hadn't separated are now nailed.

**The deferral firing clock (oracle-swept, single anchor A, no blocker).** A
`not` does not fire when satisfied-so-far; Drools holds it until the pseudo-
clock (starts at 0, advances ONLY via explicit `advance` — insertion does not
move it) proves no blocker can still arrive:
- `after[lo,hi]`: fires at clock ≥ **A.ts + hi** (blocker window [A+lo,A+hi] is
  future). advance 179→0, 180→1 for A@100/after[60,80].
- `before[lo,hi]` with **lo=0**: fires at clock ≥ **A.ts** (window touches the
  present — a coincident blocker is possible).
- `before[lo,hi]` with **lo>0**: fires **IMMEDIATELY** at the initial fire
  (window [A-hi,A-lo] is strictly in A's past; once A is present no earlier
  event arrives). Formula: `fire_time = A+hi`(after)/`A-lo`(before); immediate
  iff `fire_time < A.ts`, else defer to `fire_time` (fires at the initial fire
  when `fire_time ≤ 0`).

**Firing ORDER.** A blocker with ts in the window suppresses (cancels) the
firing. Deferred firings that come due in ONE `advanceTime` fire in **reverse
close-time order** (descending fire_time — the addInsert-PREPEND discipline
again, the same reversal exists/joins showed); across separate advances, in
advance order. IMMEDIATE firings fire at the initial fire in **insertion/FIFO**
order (a before-lo>0 not behaves like a plain not). `model_not_defer.py
simulate()` encodes exactly this and is **0-div vs the gate oracle on the
shuffled not_partner population, 6 seeds / 1800 cases**.

**Arcs are COUPLED (why the isolation matters).** With NO explicit @expires,
Drools INFERS a reach for the blocker from the temporal constraint: `before
[20,40]` A@4 / B@-30 blocks at clock 0 but UN-blocks after `advance 100` (the
blocker's inferred @expires retires it). So absent @expires does NOT isolate the
deferral — it enables the inference arc. The validated model uses LARGE explicit
@expires (nothing expires) to pin arc A alone.

**⇒ Next (staged in D-128, arc A now done):** (B) model the inference through
the not (blocker/anchor expiry when @expires is absent/finite — the coupled
arc, related to the parked exists-inference); then chains (`not` off a join) +
`not_mid`; then the ENGINE port — a deferral SCHEDULER (hold a satisfied not on
a pseudo-clock timer keyed to fire_time, fire on advance, cancel on a blocker)
NOT the D-127 admission reorder. No engine change this checkpoint (recon+model;
`make diff` 11/951/284, lint 1334/0/0 untouched).

### D-130: not×temporal arc B — @expires INFERENCE through the not PINNED + validated model (`model_not_infer.py`, 0-div on the not_partner population); the inference is invisible to FIRINGS (blocked ⇒ silent either way), it only reaps working memory

Continued the not slab (Bryan, "continue on not×temporal"). Arc B = the coupled
inference arc D-129 isolated out with a large explicit @expires. Probe-first
(oracle-swept, then model-first) on the not_partner shape, @expires ABSENT.

**The inferred expiration offsets (oracle-measured; mechanism =
`docs/drools-inferred-expiry-never.md`, the D-109 reverse-engineering of
`TemporalDependencyMatrix.getExpirationOffset` = max upperBound of the type's
row, NEVER when < 0).** For `not E1(this OP[lo,hi] $a)` (constraint `E1 OP
[lo,hi] E0`):

| | offset(E0 anchor) | offset(E1 blocker) |
|---|---|---|
| after[lo,hi]  | hi                  | lo==0 ? 0 : NEVER |
| before[lo,hi] | lo==0 ? 0 : NEVER   | hi                |

An event of type T is reaped when clock ≥ `T.ts + offset + 1` (present through
ts+offset; measured to the tick via the result `facts` multiset — cleaner than
firing-inference). NEVER (a purely-backward reach, `−lo<0`) ⇒ never reaped.
Explicit @expires=E overrides to offset=E (reaped at ts+E+1, same +1). This
EXPLAINS the D-128 `cp2_not_*_adv` gap AND the D-129 "before[20,40] A@4/B@−30
un-blocks after advance 100": the blocker's offset=hi retires it. It also
predicted (verified) the `after[lo>0,hi]` **blocker is immortal** (offset −lo →
NEVER): `after[20,40] A@4 B@30` never fires because B@30 is never reaped.

**Firing rule (lo=0 population; the only fire-points are clock 0 and the single
advance, and at the advance end every finite-offset event is already reaped):**
- `ft` (window close) = a+hi (after) / a−lo=a (before). ft < death ALWAYS when
  lo=0 (death = a+hi+1 or a+1), so an unblocked anchor always outlives its own
  window close by one tick.
- An anchor with **no in-window blocker** arms a window-close TIMER and fires at
  continuous ft *during* whatever advance spans ft — robustly, even though the
  final clock (1000) is long past its own reaping (verified: `after[0,80] A@4`
  fires though E0 is reaped at 85 ≪ 1000). Fires at clock 0 when ft==0 (before,
  a=0).
- An anchor **with an in-window blocker at insertion does NOT arm the timer**;
  un-blocking via expiry does not re-arm it, and the sole post-expiry fire-point
  (the advance end) has the anchor already reaped ⇒ it **NEVER fires**. Verified
  the discriminator: `after[0,80] A@4 B@10` fires when the advance STOPS at 84
  (fireAllRules finds it satisfied+window-closed+anchor-alive) but does NOT fire
  on a single jump 0→1000 (anchor reaped by fireAllRules time). ⇒ **for this
  population the inference is invisible to firings**: a blocked anchor is silent
  whether the blocker is inferred-mortal (arc B) or explicit-immortal (arc A) —
  the arc collapses to the same firing SET, differing only in the reaped `facts`
  (which the engine port must still get right).
- Order: within one advance, reverse close-time (descending ft, the PREPEND
  discipline); clock-0 firings precede advance firings. (No ties/immediate-regime
  in not_partner: lo=0 kills the before-lo>0 immediate case, and distinct a_ts
  ⇒ distinct ft.)

`model_not_infer.py simulate()` encodes exactly this and is **0-div vs the gate
oracle on the shuffled not_partner population, 6 seeds / 2300 cases** (both
@expires and advance coin-flips).

**Chain recon (staged next).** chain_not / not_mid dumped
under absent @expires: the not-mechanism GENERALIZES unchanged (blocked⇒silent
`cn_blk`/`nm_blk`; unblocked⇒fires `cn_nob`/`nm_nob`; out-of-window E2 doesn't
block `cn_out`), composed with the **D-125 temporal join** for the parent bind
(chain_not needs the $a–$b join — `cn_nojoin` no-fires; not_mid needs the
positive $c — `nm_noc` no-fires) and the firing now renders the join TUPLE
(`[0,20]`,`[0,30]`). So chains = (join-order D-125) × (not deferral+inference) —
same not-mechanism, richer tuple + join-order; not yet modeled.

**⇒ Next:** (1) extend `model_not_infer.py` to chain_not/not_mid = fold the
D-125 join-order model in and render tuples; (2) THEN the ENGINE port — a
deferral SCHEDULER (arc A) whose blocked anchors stay silent under inferred
expiry (arc B needs no extra firing logic, but the port MUST reproduce the
inferred-offset reaping so `facts` match) NOT the D-127 admission reorder. No
engine change this checkpoint (probe+model; new tool `tools/model_not_infer.py`;
`make diff` 11/951/284 unaffected, tree clean but for the untracked tool).

### D-131: not×temporal arc B — CHAINS (chain_not / not_mid) modeled; FIRING SET 0-div all 3 shapes, but the within-close-time chain tuple ORDER is not-node PHREAK staging (D-125 analog) — fenced from the model, not black-box-ground

Extended `model_not_infer.py` from `not_partner` to all three fuzz_not_temporal
shapes. The composition (probe-confirmed) is: **positive temporal join (D-125
order, reuse `model_join_flush.Node`) → the `not` FILTERS (blocked ⇒ silent) and
DEFERS to its window-close → schedule.** The not's anchor is where its window
closes: not_partner/not_mid anchor = `$a`; chain_not anchor = `$b` (the joined
E1). ft = anchor+hn (after) / anchor (before). Firing SET (which tuples match) =
`_join_tuples` (E0-E1 for chain_not, E0-E2 for not_mid) minus the blocked ones;
tuples render the join TUPLE (the not contributes no element, D-031).

**Ordering, probe-measured:** clock-0 (ft≤0) tuples fire FIFO/creation order
(x159 `[3,5]`, cn0_fifo `[5,3]` — both the join's propagation order, verified);
the advance batch fires **descending close-time** across anchors (nm_2a: a=5
ft55 before a=0 ft50). Both modelled 0-div. What resists: the **within-same-
close-time** multi-tuple order (chain shapes, when several tuples share the not-
anchor's ft — same `b` with different `a` in chain_not, same `a` with different
`c` in not_mid). Measured tie-break behaviour is NOT a creation-index sort:
`nm_ins_81_86`/`nm_ins_86_81` (1 anchor) fire REVERSE of the positive partner's
insertion order, but `nm_2a` (2 anchors) fires a=5's partners FORWARD while
a=0's REVERSE in the *same* case. Five hypotheses (crt asc/desc, reverse-insert,
LIFO, whole-batch reverse) each fit some cells and break others — the classic
PHREAK-staging flip-flop. This is the **not-node window-close re-propagation
staging**, the direct analog of the D-121..125 join-flush order (which needed a
drools-core sources-informed per-propagation flush model to pin).

**SOURCE READ (drools-core 9.44, behaviour only — Apache-2.0, nothing copied).**
Peeked to settle whether the tie is a clean rule or an artifact. Mechanism:
`PhreakNotNode.doLeftInserts` propagates an unblocked left-tuple IMMEDIATELY
(`insertChildLeftTuple` → `trgLeftTuples.addInsert` = PREPEND); a temporal not
holds the tuple via a scheduled window-close, and on close/blocker-expiry
`doRightDeletes` re-propagates the un-blocked lefts (again `addInsert` PREPEND,
iterating `rightTuple.getBlocked()`). ALL time-scheduled firings drain through
`PseudoClockScheduler.queue`, a `java.util.PriorityQueue<TimerJobInstance>`, and
`DefaultTimerJobInstance.compareTo` orders **solely by `trigger.hasNextFireTime()`
— NO secondary key** (verified, lines 54-56). So same-close-time jobs are EQUAL
in the queue ⇒ their relative fire order is a **binary-heap artifact** of the
add/poll sequence (Java `PriorityQueue` is NOT stable for equal elements) — the
add order being the schedule/propagation order, itself carrying `addInsert`
prepends. ⇒ the within-close-time order is an **implementation artifact, not a
semantic** (same class as the `fz_42_84` identity-hash-order quarantine — the
doctrine says document, don't chase). This is the airtight reason the black-box
tie flip-flopped, and it re-scopes the fence: the FENCE IS CORRECT to keep in the
MODEL; the ENGINE PORT will match these ~0.6% cases only if Seine's scheduler
reproduces Drools' PriorityQueue tie-order (a port/scheduler concern, testable
vs the oracle) — else they graduate to `xfail/` as heap-order expected-
divergences, NOT a firing-set error.

**⇒ Result / scope.** FIRING SET: **0 divergences, all 3 shapes, ~4500 cases /
9 seeds** — the semantic content is fully modelled (which tuples fire, blocked ⇒
silent under inferred/explicit @expires alike, D-130's "inference invisible to
firings" holds across chains too). ORDER: 0-div for not_partner + the cross-
close-time ordering; residual **~0.6%, chain_not/not_mid ONLY, every one order-
only (never a set/count miss)** = the within-close-time not-node staging. Per
the STOP-RULE (don't grind an Nth black-box round on a flush micro-order),
FENCED from the model and staged. Repro seeds: `nif7001x146` (not_mid),
`nif7002x120` / `nif7003x321` (chain_not).

**⇒ Next.** The not-node flush staging is best cracked WITH the engine port
(it reuses the D-125 flush-cascade machinery — model the not-node as a per-
propagation flush node whose window-close emission obeys the same prepend
discipline; the model's within-close-time order finalises alongside the port,
exactly as `model_join_flush`'s order did with D-125). Until then the model is
the FIRING-SET spec + the cross-close-time order spec. No engine change this
checkpoint (`make diff` 11/951/284 unaffected).

### D-132: not×temporal ENGINE PORT — GATED & begun; §3A (not @expires inference) implemented & STAGED behind the fence; the port BISECTED two PRE-EXISTING positive before-inference latents the not-population flushes (fence held, corpus byte-identical)

Bryan GATED the port (`docs/not-temporal-port-mechanism.md`): §3B removal-driven
`fire_deadlines`, §6 quarantine the heap-tie undefined behavior. Began the port.

**§3A (arc-B reaping — the not's @expires inference) IMPLEMENTED.** Gave a
temporal `not` a PHANTOM temporal-matrix position (`CompiledPattern.temporal_pos`,
a high-base index that records the after/before STP edges without claiming a
tuple slot — `tpos` is too wired into constraint eval to overload). Routed the
edge recording (`engine.rs` ~2320) and the bare-pattern-NEVER check
(`engine.rs` ~3010) through `temporal_pos` (= `tpos` for positives/exists, so
they're byte-identical; = phantom for a temporal `not`). With the fence lifted,
the not's own offsets come out right (after: E0=hi, E1=lo?0:NEVER; before mirror)
and a facts-only differ (`facts_check.py`, engine `run` vs `oracle`, firings
ignored) dropped not-temporal facts from all-fenced to **~1.5% divergence**.

**⚠ THE PORT FLUSHED PRE-EXISTING POSITIVE LATENTS (bisected to the pure-positive
path — NO `not` involved, so NOT caused by §3A).** The ~1.5% residual is entirely
two latents that the not-population (absent @expires + far-past `before` shapes)
is the first to exercise at scale (D-125 isolated join order with LARGE @expires;
these never fired):
- **pos_far** — `$a:E0() $c:E2(this before[0ms,100ms] $a)`, E2@−129, advance:
  the ENGINE reaps E2 (inferred offset = hi = 100 ⇒ deadline −28) but DROOLS
  KEEPS E2@−129 (offset NEVER/huge). The engine's `before` earlier-operand
  inferred offset diverges from Drools. (Drools' actual before-offset is
  unprobed — the doc's `−lo` rule predicts 0, not NEVER either; NEEDS a probe.)
- **pos_ins** — `$c:E2(this before[0ms,50ms] $a)`, E2@−51, NO advance: DROOLS
  drops E2 at INSERTION (clock 0 ≥ its deadline 0), the engine keeps it — the
  reaper runs only in `advance()`, there is no at-insert already-expired sweep.

Both reproduce with `tools/probe_before_latents.py`; both are pure-positive, so
they are latent bugs in the D-109 positive inference / reaper, surfaced (not
caused) by the port. They BLOCK a clean `fuzz_not_temporal` gate.

**⇒ Decision surfaced to Bryan (supervision).** The not port can't gate 0-div
until these positive latents are resolved. Options per each: (a) FIX in-scope —
probe Drools' real `before[0,hi]` earlier-operand offset + add an at-insert
already-expired sweep; or (b) QUARANTINE — `scenarios/xfail/` + a generator gate
excluding the far-past-before surface, and proceed with the not port on the rest.
Recommend probing the before-offset first (it's a correctness bug in a landed
feature, likely worth fixing) then deciding.

**State: §3A code committed but INERT (fence restored).** `make diff` 11/951/284
byte-identical, lint 1334/0/0 — positives/exists unchanged, `not` still fenced.
NEXT after the latent decision: lift the fence, resolve/quarantine the latents,
then §3B (the `fire_deadlines` deferral scheduler).

### D-133: the two pre-existing positive before-inference latents (D-132) FIXED — a born-expired LEAK + an at-insert reap; §3A (not inference) now facts-validated 0-div on the population

Bryan chose PROBE-THEN-FIX. Probed Drools' real reaper boundary (E0 immortal,
E2 inferred, sweep the advance): an event's inferred offset IS `hi` (E2@0,
before[0,100] gone at clock 101 = ts+hi+1 — the engine's offset was right). The
divergence is the SCHEDULING BOUNDARY at the insertion clock (measured to the
tick, before[0,100], deadline=ts+101):

| deadline vs insert-clock 0 | Drools |
|---|---|
| deadline < 0 (born in the past) | **KEPT forever** — can't schedule a past job (leak) |
| deadline == 0 | **matches + FIRES this cycle, then dropped** |
| deadline > 0 | scheduled; reaped when clock ≥ deadline |

The engine's `advance()` reaper (`deadlines.range(..=clock)`) reaped the
born-in-past ones (pos_far) and never reaped the at-insert-due ones without an
advance (pos_ins). **Fix (both in `schedule_expiration`, `engine.rs` ~3545):**
`match deadline.cmp(&clock_ms)` — `Less` ⇒ don't schedule (leak); `Equal` ⇒
`pending_expirations.push` (the LAZY delete — NOT `mark_expired`, so the event
still matches/fires in this cycle and drops at the post-fire quiescence drain,
matching the oracle's fire-then-drop); `Greater` ⇒ schedule as before.

**Verified.** pos_far / pos_ise both match; a MATCHING at-insert-due event fires
`-30--51` AND drops E2 (oracle-identical). `make diff` 11/951/284 BYTE-IDENTICAL,
lint 1334/0/0 (the fix only changes born-expired / at-insert-due events, which
the certified corpus never had). Positive reaper fuzz (`/tmp/seine_posfuzz.py`
shape) **0-div facts+firings, 750 cases / 3 seeds** (far-past + boundary + multi-
advance). **⇒ §3A payoff:** with the latents fixed, `facts_check.py` on the
not-population (fence temp-lifted) is **0 facts divergences, 750 cases / 3 seeds**
— the arc-B REAPING half of the not port is COMPLETE & validated. Fence restored
(firing §3B pending). NEXT: lift the fence for good + §3B (the `fire_deadlines`
firing-deferral scheduler) — the last piece.

### D-134: not×temporal §3B ENGINE PORT — the firing-DEFERRAL scheduler LANDED; the LAST CEP-E2 fence is down. Firing SET 0-div (~4600 cases), engine==validated-model, within-close-time ORDER residual fenced to xfail

The last piece. Lifted the fence (`engine.rs` `Constraint::Temporal` / `p.ce ==
CeKind::Not`), activating §3A, and built §3B so a satisfied temporal `not`
DEFERS to its pseudo-clock window close instead of firing at insert.

**Design — hold-in-lefts + `pending_release` re-fire (NOT the report's
removal-driven `fire_deadlines`).** The report §3B recommended a phantom
"window-blocker" whose scheduled REMOVAL drives the existing right-delete
re-fire path. Set aside: a phantom right pollutes `node.rights` (find_blocker /
allowed_ce would look it up in the store). Landed design instead:
- `not_fire_time(node, l)` (JoinEnv): `Some(anchor.ts+hi)` for `after`,
  `Some(anchor.ts)` for `before[0,hi]` (the DEFERRED regime); `None` for
  `before[lo>0]` (IMMEDIATE regime — fires at insert) / non-after-before /
  non-temporal. Point-event formula (anchor.ts, no @duration end).
- At the not's left-insert (and left-update newly-satisfied), `not_emit_or_defer`:
  a temporal not in the deferred regime does NOT `create_ce_child`; it pushes
  `(left, origin, fire_time)` to `node.new_deferrals` AND keeps the left in
  `node.lefts` (so a later blocker's right-insert still blocks it — free
  cancellation via the existing scan). ALWAYS defers, even when already due.
- The engine drains `new_deferrals` after `do_node` into
  `fire_deadlines: BTreeMap<i64, Vec<(ni, Tup, Origin, seq)>>` (mirror of
  `deadlines`), stamping a monotonic creation `seq`.
- `drain_pending_fires` (fire-quiescence, BEFORE `drain_pending_expirations` so
  a not fires while its anchor is still alive) collects the due
  (`fire_time <= clock`) entries and orders them to the model: **creation order
  at the initial fire (clock 0, agenda FIFO); (−fire_time, creation) at an
  advance (the PseudoClockScheduler PQ)**. Each released left goes to its not
  node's `pending_release`; the rule re-queues + re-evaluates. do_existential_node
  fires a `pending_release` left ONLY if still UNBLOCKED (in `node.lefts`) with no
  child yet — a blocked (or blocked-then-expired) left is silent forever, the
  arc-B `model_not_infer` rule.
- **Un-block re-fire SUPPRESSED for a temporal not** (`if !node.temporal`) at the
  right-delete / right-update / left-update paths: a blocker's REMOVAL (an
  inferred-mortal E1 reaping early) must not resurrect a once-blocked firing.
- **Ordering falls out of one PREPEND reversal** (`child_ins` addInsert): push
  `reverse(target)` so the agenda equals `target` (empirically pinned by
  `w_two_after`, not hand-derived).
- **not_mid downstream-join fix:** a released left in `not_mid`
  (`$a:E0 not E1 $c:E2`) joins E2 at fire_time, but a collapsed `advance(1000)`
  has already expiration-FLAGGED E2 partners that reap AFTER that fire_time
  (still `is_alive`, deleted only at the later drain). A `not_releasing` flag
  (a local in `evaluate_rule_inner`, threaded into `JoinEnvImpl` at the do_node
  site) makes `is_expired` read false for the whole released-left propagation —
  the model ignores partner expiration for this join. This turned the sole
  SET-miss the fuzz found into an order-only residual.

**Verification.** `fuzz_not_temporal` (all 3 shapes): **0 firing-SET divergences
across ~4600 cases** (seeds 11-14/21-26/100-104); a targeted engine-vs-`model_not_infer`
check is **engine==model on every case**. `make diff` 11 / **956** (+5 graduated
witnesses) / 284 byte-identical; `make lint-probes` 1334/0/0; `cargo test` 9
suites; bindings pytest 72. 9 hand witnesses (`after`/`before`, adv/no-adv,
blocked, interval, no-expires, two-anchor order) all pass. The 5 fenced recon
witnesses (`cp*not*`, incl. `cp_not_chain_defer`) GRADUATED to
`scenarios/probes/pr_cep_not_*` (3 `expect_inert`, 2 firing).

**Fenced (§6):** the ~0.6% WITHIN-close-time multi-tuple ORDER (chain_not /
not_mid, order-only, never a set/count miss). Split: most are the D-131 UNDEFINED
tie-order — Drools' `DefaultTimerJobInstance.compareTo` keys SOLELY on fire-time
(no secondary key, verified), so equal-fire-time jobs' relative order is NOT
decided by any Drools logic; it falls out of `java.util.PriorityQueue`'s non-stable
heap add/poll — a JVM / Java-core side-effect, NOT a semantic (same class as the
`fz_42_84` identity-hash-order quarantine). That is why the black-box tie
flip-flopped and why the validated model diverges too (no spec exists to match). A
few are the not_mid released-left downstream-join order (pop-time `do_join_node`
scans descending-ts then PREPENDS, reversing vs the D-125 flush order). Both are
the "chain within-close-time" residual D-131 flagged; per doctrine (do NOT grind
undefined behavior) quarantined to
`scenarios/xfail/xf_cep_not_{chain_heaptie,mid_release_join_order}`. Not
black-box-ground; matching it would need byte-emulating `java.util.PriorityQueue`,
not a better scheduler.

**Not pushed** (Bryan holds the push; the D-127..D-134 stack + docs).
[Pushed 2026-07-09 — Bryan cleared the hold; branch-only, no tags.]

### D-135: @expires INFERENCE through an exists — RECON + PORT LANDED (one-line). FACTS-only (reaping) ~25% → 0-div; FIRING 0-div throughout (inference invisible, like the not); mechanism = the not §3A phantom-`temporal_pos` extended to exists, NO §3B

Scoped the parked "@expires inference through an exists" candidate (D-126/D-127
kept it out — the exists port was insert-only + explicit-@expires only; the STP
edge is guarded so a positionless exists records no inference edge). Probe-first:
`tools/fuzz_exists_infer.py` sweeps exists×temporal with @expires ABSENT half the
time + a coin-flip advance, splitting engine-vs-oracle into FIRING-set vs
FACTS-only.

**Recon result (seeds 5001-5004, ~1000 cases):**
- **FIRING-set divergences: 0.** The inference is INVISIBLE to firings — exactly
  the D-130 key insight, now confirmed for exists: an exists fires when a partner
  is PRESENT (at clock 0 in the population, before any reaping); its retraction
  (when a partner expires) is unobservable (D-127); and the anchor reaps only
  after its own firing. So arc-A-style firing logic is NOT needed.
- **FACTS-only divergences: ~25%** (all in the absent-@expires + advance cases).
  Drools infers a finite expiry and REAPS; the engine forces the exists type to
  NEVER (no phantom `temporal_pos`) so it KEEPS the events. The kept events match
  the §2b/D-130 table exactly: for `exists E1(OP[lo,hi] $a)` (constraint
  `E1 OP[lo,hi] E0`), after ⇒ E0=hi, E1=lo?0:NEVER; before ⇒ mirror; reap at
  ts+off+1. Uniform across `ex_partner`/`chain_ex`/`ex_mid` — no chain
  composition quirk (unlike not_mid, there is no deferral, so no release-time
  downstream-join order to compose).

**⇒ Scope: this is the §3A REAPING analog of the not port, and ONLY that —
there is NO §3B (no firing-deferral scheduler).** A strictly SMALLER slab than
the not. **Mechanism = extend the not's phantom `temporal_pos`** (D-132 §3A):
`engine.rs` ~2281, change `if p.ce == CeKind::Not` to also cover
`CeKind::Exists` so an exists pattern gets a phantom matrix position; then its
after/before STP edges record (~2350) and the bare-NEVER override (~3030) stops
forcing its type to NEVER — Floyd-Warshall + `infer_event_expiry` fold them in
unchanged, and the reaper (incl. the D-133 boundary fix) is untouched. **Gate:**
`fuzz_exists_infer.py` 0 FIRING + 0 FACTS divergence; `make diff` byte-identical
(gate on temporal + `CeKind::Exists`); the D-127 firing gate
(`fuzz_exists_temporal.py`) stays 0-div; verify the phantom does NOT perturb the
D-127 admission order (it drives inference only, not the exists node's tpos/kind).
**Risk:** low — no new firing machinery; watch the same positive-latent reaper
boundary (already landed D-133) and the byte-identical corpus.

**PORT LANDED (same session, one line).** `engine.rs` ~2281:
`if p.ce == CeKind::Not` → `if matches!(p.ce, CeKind::Not | CeKind::Exists)` so a
temporal exists gets a phantom `temporal_pos`; its after/before edges then record
and the bare-NEVER override (which already keys on `temporal_pos`, D-132) stops
forcing NEVER — Floyd-Warshall + `infer_event_expiry` + the reaper (incl. the
D-133 boundary) fold it in UNCHANGED. Byte-identical everywhere except a temporal
exists with an after/before edge (a non-temporal exists records no edge ⇒ still
NEVER; explicit @expires overrides). **Gates:** `fuzz_exists_infer.py` 0 FIRING +
0 FACTS (4 seeds / 1200 cases, was ~25% facts); `make diff` 11 / **958** / 284
(+2 locked witnesses `pr_cep_e_exists_infer_{reap,before}`); `fuzz_exists_temporal`
(D-127 firing) still 0-div; `cargo test` 9 suites; lint 1336/0/0; bindings 72.
The recon prediction held exactly — no §3B, no surprises. [Pushed 2026-07-09 —
branch-only, no tags; `origin/main` at `5f7862f`.]

### D-136: SHARED temporal-join node ORDER — RECON (staging the next slab). ORDER-only (14%, 0 SET-miss); the target order is KNOWN (per-rule == single-rule D-125); a genuine model-first slab, NOT a one-liner

Scoped the parked shared-temporal facet (`xf_cep_tjorder_dual_tms`; D-102 bailed
it to legacy pop-time). Isolated probe `tools/fuzz_shared_tjo.py` — 2-3 rules
with the SAME temporal-join LHS (`$a:E0() $b:E1(op[0,hi] $a)`) ⇒ the join node is
SHARED; positive-only (no TMS/not/salience) to isolate the order.

**Recon (seed 7101, 300 cases):**
- **14% ORDER-only divergence, 0 SET-miss** — a shared temporal join never
  produces a wrong firing SET/count, only a wrong firing SEQUENCE. Shows up
  single-epoch too (16% single / 11% multi), so it is not just a pop-boundary
  effect.
- **The target order is a KNOWN spec, not a heap tie.** A single-rule-variant
  diff proved the oracle's PER-RULE tuple order == the single-rule D-125 flush
  order (`model_join_flush`) for 100% of cases. The engine's legacy bail
  (`stream_flush_ex` ~3760: a `node_linked && node_shared` temporal node
  stash-alls ⇒ pop-time) orders the shared node's tuples WRONG.
- **~76% of divergences (32/42) are GROUPED-by-rule** (each sharing rule fires
  its D-125 batch contiguously, rules in decl order) — a clean tractable spec.
  **~24% (10/42) INTERLEAVE across rules** (mostly multi-epoch) — the harder
  agenda-pop composition (shared-segment re-pop under salience, the D-091/D-106
  envelope).

**⇒ Assessment: a genuine MODEL-FIRST slab, NOT a one-liner** (unlike D-135
exists). Well-defined and tractable (clear target order, never black-box), but it
needs (1) a model of the shared-node cross-rule agenda composition (grouped
single-epoch + interleaved multi-epoch), validated 0-div, then (2) engine
plumbing to un-bail the shared flush — route a shared node's per-arrival D-125
emissions to EACH sharing rule path in agenda order. **Risk MEDIUM:** D-102's
NOTE that the naive unscoped force-flush blast-radiused 18% of single-rule
scenarios is the warning — the plumbing must stay scoped to shared temporal
nodes. **Priority: LOW** — ORDER-only (never a correctness/set bug), currently
xfail'd. Recommend the full model→validate→port with a fresh context + a go-ahead.
Tool: `tools/fuzz_shared_tjo.py` (ORDER/SET split; the future gate).

**MODEL VALIDATED (same session) — `tools/model_shared_tjo.py`, 0-div / 6 seeds /
1800 cases.** The composition is SIMPLER than the recon feared (the "interleaved
24%" was not a deep agenda arc): per fire cycle (epoch) the single node's D-125
tuple batch fires RULE-GROUPED (RuleExecutor drains each rule's queue, decl
order), and **the FIRST sink (TJ0) fires it FORWARD while every PEER sink
(TJ1, TJ2, …) fires it REVERSED** — the D-071/D-102 peer-copy discipline
(SegmentPropagator prepends ⇒ peers LIFO). A v1 "all rules forward" was 77%
wrong; adding the peer-reversal → 0-div.

**PORT — naive un-bail FAILS (de-risked, NOT landed).** Hypothesis: stop stashing
shared temporal nodes so they flush per-arrival and the existing peer-merge gives
the order. Tried it (`stream_flush_ex` push-empty for shared temporal): **61%
shared-div (WORSE than the 14% bail) + 2 corpus regressions**
(`pr_cep_53_perfact_memory_arrival`, `pr_rs_r11_stream_staging`) — reverted, diff
back to 11/958/284. ROOT CAUSE: the flush emits PER-ARRIVAL, so peer sinks get
each small batch reversed; concatenated per-arrival reversals ≠ the WHOLE-epoch
reversal the oracle wants. The correct port needs TJ0's per-arrival D-125 order
BUT peers reversed over the FULL epoch batch — a distinct compose the engine does
neither at pop-time (14% wrong) nor at flush (61% wrong). **Next: engine plumbing
for that compose** (batch the shared node's peer emission and reverse once), gated
on `fuzz_shared_tjo.py` 0-div + `make diff` byte-identical. Still ORDER-only / LOW
priority; the SPEC (model) is locked so the port is well-defined.

**PORT LANDED (fresh context + go-ahead) — `fuzz_shared_tjo.py` 0-div / 5 seeds /
700 cases + the model's 6/1800; `make diff` 11 / 958 / **288** byte-identical.**
The compose the recon called for, wired in three scoped pieces:

1. **Divert (stash loop, `stream_flush_ex`).** A shared temporal join of the
   VALIDATED shape — clean INSERT-only delta (no upd/del, no ph=1 right), ALL
   sinks Term — is taken OUT of the eval walk (so `do_join_node` never re-orders
   it, the 61% naive-un-bail trap) into a side `shared_tj_stash`. Diverted on
   EVERY arrival, **linked or not** — the key correction the recon model hid:
   an UNLINKED left left in staging batch-flushes / self-drains REVERSED, which
   flips the base order the peers then reverse AGAIN (a first cut that kept the
   `node_linked` gate was 12%, all base-order flips). Draining each arrival's
   single-side delta to memory in arrival order (what the unshared D-125 path
   already does) is what makes the base = D-125. Non-clean shared temporal nodes
   keep the legacy pop-time bail (byte-identical).
2. **Accumulate (D-125 flush loop).** The diverted delta runs `flush_ins_delta`
   (the D-125 base order — where pop-time `do_join_node` gives its REVERSE) and
   the emission is appended to the node's new `tj_epoch` buffer in FORWARD order.
   It is NOT routed to the sinks here: per-arrival routing can't form the peer
   reversal (the peer copy reverses each single-tuple batch = a no-op, and
   term_pending drains between arrivals — the exact "concatenated per-arrival
   reversals ≠ whole-epoch reversal" the recon measured at 61%).
3. **Drain ONCE (fire boundary, `fire_all`).** Before the pop loop, each node
   with a non-empty `tj_epoch` emits the WHOLE epoch batch: first sink FORWARD
   (`append_into_pending` = addAll ⇒ the D-125 order), every peer REVERSED
   (`peer_merge_term` prepends the whole batch ⇒ the SegmentPropagator LIFO
   reversal, now over the whole epoch). RULE-GROUPING falls out of the existing
   RuleExecutor (each rule drains its queue in decl order); per-epoch reversal
   falls out of the terminal draining between fire cycles. **This is the compose
   the engine did neither at pop-time (14%) nor at flush (61%): D-125 base +
   whole-epoch peer reversal.**

**Files:** `engine.rs` (the three pieces above); `phreak.rs` (`Node.tj_epoch`,
empty on every non-shared/non-temporal node ⇒ byte-identical path). ~1 field + 2
scoped blocks; no change to `flush_ins_delta`, `do_join_node`, `peer_merge_term`,
or any unshared path.

**Gates:** `fuzz_shared_tjo.py` 0 ORDER / 0 SET (seeds 7101-7104, 8201; 700
cases; was 14%); `make diff` 11 / 958 / **288** byte-identical; `cargo test` 9
suites; `make lint-probes` 1336/0/0; bindings pytest 72.

**BONUS — `xf_cep_tjorder_dual_tms` GRADUATED** (xfail → `scenarios/regressions/`).
The 7-rule TMS scenario (TJ0/TJ1 share the temporal join; J2 insertLogical, ND4
`not D`, salience ladder) was xfail'd on exactly this order bug; the scoped fix
corrects the shared-join order while the TMS deletes (upd/del staging ⇒ clean=
false) stay on the legacy path — now byte-identical, deterministic (3/3). Locked
alongside 3 new positive witnesses `fz_tjo_shared{2_peer,3,_epoch}` (2-rule peer,
3-rule two-peer, 3-rule×epoch — each in the pre-fix divergence set). The whole
CEP-E2 shared-temporal-join ORDER facet is UNWALLED; only the ~0.6% within-close-
time not-ORDER residual (D-134 §6) remains — and that one is UNDEFINED behavior
(equal-fire-time `PriorityQueue` order, a JVM/Java-core side-effect per D-131), not
a fixable order, so it stays fenced by nature.

**Not pushed** (branch-only, Bryan holds the push).

### D-137: CEP E2 item C classes 1/2/3 — DEFERRED re-propagation PORT. Class 1 (temporal-join update re-fire) + Class 2 (clock-removed revival) LANDED, corpus byte-identical; Class 3 (exists explicit-delete churn) CHARACTERIZED + DEFERRED (the existential right-phase order is D-031-pinned ⇒ needs a model_check sub-recon). FENCE DOUBLE-DUTY: the two update fences also guard SEPARATE out-of-C gaps, so they CANNOT be lifted to 0-div by fixing only 1/2/3

Resumed the port D-115 fenced (HYBRID: cheap delete-of-dead + FENCE classes
1/2/3; battery pre-built). Probe-first, one class at a time.

**CLASS 1 — temporal-join update re-fire (engine UNDER-fired) — PORTED.** A
POSITIVE temporal (after/before/Allen) join Behavior node is NOT
property-reactive: Drools re-fires the match on ANY external update of the event
on the TEMPORAL (constraint-bearing) side, even a no-op/irrelevant field.
Distinct-anchor/prober discriminator probes (NOT the xfail's self-join) PINNED
the trigger: updating the PROBER (the pattern carrying the `Test::Temporal`, i.e.
the `node.temporal` node) re-fires; updating the ANCHOR (pattern 0, the plain
left input) does NOT (`pr_cep_c_upd_anchor`) — so the fix is NARROW. Port
(`engine.rs on_update` `(true,true)` branch): `temporal_refire = pat.ce ==
Positive && node.temporal` forces `add_upd` regardless of the listen mask (the
mask-miss `re_add_right_fact` never re-fires). Validated: `xf_cep_c_upd_temporal`
→ green + after / shared (D-136 two-rule, both re-fire in order) / chain probes,
corpus byte-identical. Scoped to Positive — temporal not (D-134) / exists (D-127)
keep their own semantics.

**CLASS 2 — clock-removed revival (engine OVER-fired) — PORTED.** A window
eviction / expiration-eager acc-removal (`stage_acc_removal`) drops the event
from `trie[ni].active` but leaves it `is_alive` until the deferred drain; a later
external UPDATE hits the `on_update` `(false,true)` re-entry and re-adds it to the
accumulate (count springs back). Port: a per-node `TrieNode.clock_removed` set
populated in `stage_acc_removal`; the entry branch suppresses the revival when
the event is in it. Naturally CEP-gated (only events reach `stage_acc_removal` ⇒
empty on the plain corpus ⇒ byte-identical; FactIds are monotonic and `reset`
rebuilds the trie, so no stale-id hazard). Validated: `xf_cep_c_upd_evict_revive`
(window) + `xf_cep_c_upd_after_exp` (expiration) → green, corpus byte-identical,
population fuzz-checked (windowed fence lifted, seed 201/600).

**CLASS 3 — exists explicit-delete churn (engine UNDER-fires) — CHARACTERIZED,
DEFERRED.** Delete the sole `exists` witness + reinsert a fresh one in the SAME
epoch ⇒ Drools un-fires+re-fires (NE NE); the engine coalesces (NE). Full model
pinned by probes (`pr_cep_c_exists_*`): (1) NOT external-specific — a RULE-RHS
`delete($w); insert(new witness)` churn ALSO re-fires (`xf_cep_c_del_churn_exists_
rule`: NE,CH,NE vs NE,CH) ⇒ it is ANY EXPLICIT (non-expiration) delete; (2)
EXPIRATION churn must STAY coalesced (`pr_cep_c_exists_exp_churn`, D-102; engine
distinguishes via `in_expiration_drain`); (3) ORDER-sensitive — delete-first
re-fires (count 1→0→1), insert-first does NOT (`pr_cep_c_exists_ins_first`),
separate-epoch works (`pr_cep_c_exists_sepepoch`), delete-one-of-two no-ops
(`pr_cep_c_exists_2wit`). ROOT: the existential right-phase order is
rightIns-BEFORE-rightDel (`phreak.rs` ~1487, D-031 pinned) — so the same-batch
reinsert W′ is already in right memory when the delete of the blocker W
re-searches, is found as a replacement blocker, and the left never unblocks ⇒ no
child retract, no re-fire. The fix requires INVERTING/scoping that pinned order
for explicit deletes — a flip-flop-prone existential-staging change the doctrine
MANDATES a model_check for (the "rule-vs-external survived 17 hand timelines"
warning is literally this class). DEFERRED to a dedicated model_check sub-recon
(like `model_check_join2`); `xf_cep_c_del_churn_exists`(+`_rule`) stay xfail, the
fence stays. NOT hand-tuned (D-083 discipline).

**FENCE DOUBLE-DUTY (revises D-115's optimistic gate premise).** The item-C gate
assumed "lift the 3 fences ⇒ fresh fuzz 0-div." FALSE — each update fence ALSO
incidentally suppresses a SEPARATE, out-of-item-C gap, so it cannot be lifted to
0-div by fixing only 1/2/3:
- `windowed_acc_types` (was class 2): lifting it flushed a WINDOWED-accumulate
  LIVE-modify PROPERTY-REACTIVITY gap (`xf_cep_c_upd_win_{live,noop}`) — a
  windowed accumulate is property-reactive on the FUNCTION's fields: `count()` +
  an irrelevant/no-op `tag` update does NOT re-fire in Drools but the engine
  re-folds ⇒ OVER-fires; `sum(val)` + a `val` update re-folds and AGREES
  (`pr_cep_c_win_sum_upd`); a PLAIN accumulate re-folds on ANY modify (agrees).
  This is the flagged WindowNode "do-not-hand-tune" wall, NOT class 2 (the
  clock-removed revival, now fixed). Fence KEPT.
- `temporal_types` (was class 1): lifting it (seed 202/800) flushed 8
  divergences — ALL bisect-to-HEAD PRE-EXISTING temporal-join-ORDER / not-order
  E1-hardening latents (byte-identical on the pristine HEAD worktree, my
  `add_upd` port does NOT change them; incl. the cf313-family `not X() P()` order
  and @duration-interval join-order). NOT class 1 (the update re-fire, now
  fixed). Fence KEPT.
⇒ Classes 1 & 2 are FIXED (their divergence witnesses flip green + graduate) but
the two update fences stay, each now guarding an orthogonal deferred gap.
Consistent with D-115's own HYBRID.

**Gates (green @ working tree):** `make diff` 11 / **970** / 288 byte-identical
(+12 `pr_cep_c_*`: 3 graduated xfails + 9 boundary pins); `make lint-probes`
**1352** live·0·0; `cargo test` 9 suites; bindings pytest **72**; blast-radius
`make fuzz` seeds 42/123/7 divergence set IDENTICAL to the pristine HEAD worktree
(the port is CEP-gated — `clock_removed` only via `stage_acc_removal`, `add_upd`
only on `node.temporal`+Positive — so the event-free gen.rs main axis is provably
untouched; the fz_42/123 DELETE-FREE latents are unchanged). **Battery:** 3
xfails→probes (`upd_temporal` / `upd_evict_revive` / `upd_after_exp`), 9 new
`pr_cep_c_*` pins, 3 new xfail witnesses (`del_churn_exists_rule`,
`upd_win_live`, `upd_win_noop`); `xf_cep_c_del_churn_exists` kept.

**Artifacts:** `engine.rs` (`TrieNode.clock_removed` field + `stage_acc_removal`
insert + `on_update` class-1 `temporal_refire` and class-2 revival guard). **Left
for a fresh-context slab (needs a go-ahead):** (a) class 3 — the existential
right-phase-order model_check + un-bail; (b) windowed-accumulate modify
property-reactivity (the WindowNode sub-recon); (c) the pre-existing
temporal-join-ORDER E1-hardening family. **Committed locally, NOT pushed (Bryan
holds the push); class 3 is the ACTIVE next slab (Bryan-directed).**

### D-138: CEP E2 item C class 3 (exists external-delete churn) — EXTERNAL churn PORTED (the primary + fuzz-reachable case). The fix = a DELETE-TIME eval that resolves the STREAM ins-before-del deferral; the rule-RHS re-entrant variant stays fenced

Resumed the class-3 port after the D-137 recon (event-specific; graft
`oracle/.../ExistsDump.java`; model_check `tools/model_check_exists_churn.py` —
`event_explicit_arrival` the unique 0-div spec). The port required navigating the
STREAM architecture, not the within-`do_node` batch reorder the recon first
sketched (that sketch was byte-identical but inert — reverted at `6665f40`).

**Mechanism (the crux — `SEINE_TRACE` + an `in_flush` trace confirmed it).** In
STREAM mode each INSERT stream-flushes PER-ARRIVAL (`after_insert`→`stream_flush`,
`in_flush=true`), but an external DELETE only stages `s_right.add_del` and is
DEFERRED to `fire_all` (`in_flush=false`, the sole `do_node` site). So a
DELETE-FIRST event-exists churn evaluates the reinsert (stream-flush) BEFORE the
delete (fire_all) — ins-before-del — and the delete's re-search finds the
just-flushed reinsert as a replacement blocker ⇒ COALESCE. A PLAIN-fact churn
coalesces on BOTH engines (matches Drools' own `PhreakExistsNode.doNormalNode`,
hand-traced); only an EVENT witness re-fires in Drools, in ARRIVAL order
(del→unblock→child-retract, then ins→reblock→child-assert = re-fire).

**Port (`engine.rs delete_fact`, one scoped block).** After `on_delete`, an
EXPLICIT (`!in_expiration_drain`) delete of an EVENT witness at an exists/not node
FORCE-EVALUATES the affected rules (`evaluate_rule(ri, true, false)` under a
saved/restored `in_stream_flush`) AT DELETE-TIME — so the delete's unblock +
child-retract happens in ARRIVAL order, BEFORE the same-epoch reinsert's
stream-flush re-blocks and re-fires. Scoped to rules carrying an exists/not CE
over the victim's TYPE ⇒ the plain corpus and every non-event / non-existential
delete are untouched (they keep the deferred `fire_all` drain). Expiration deletes
stay deferred (D-102). Reproduces `event_explicit_arrival`: del-first re-fires;
ins-first / plain / expiration / 2-witness / delete-only all coalesce (the
reinsert's flush finds the still-present or re-searched blocker).

**FENCED — the RULE-DRIVEN (RHS) variant** (`xf_cep_c_del_churn_exists_rule` stays
xfail): a rule RHS `delete($w); insert(new witness)` churns the witness DURING its
own fire; the delete-time `evaluate_rule` does NOT fire re-entrantly (the RHS
delete still defers to `fire_all` ⇒ coalesce; `NE,CH` vs oracle `NE,CH,NE`). Needs
a re-entrant-safe delete-time eval; NOT fuzz-reachable (the fuzz generates no
witness-deleting rules) — a narrower residual than the primary external case.

**Gates (green):** `make diff` 11 / **974** / 288 byte-identical (external churn
xfail GRADUATED → `pr_cep_c_del_churn_exists` + 3 discriminator pins
`pr_cep_c_exists_{churn_bare,churn_plain,delonly}`); `make lint-probes` 1361;
`cargo test`; **class-3 fuzz** with the `del_ok` exists-churn fence LIFTED
(`fuzz_cep.py`) — 0-div 3×800 (seeds 301/302/303); blast-radius `make fuzz`
42/123/7 == pristine HEAD (the `delete_fact` change is event-gated ⇒ the
event-free gen.rs axis is untouched). The class-1 `temporal_types` + class-2
`windowed_acc_types` UPDATE fences STAY (D-137 1a/1b — the separate deeper gaps).
⇒ **CEP E2 item C = classes 1 (D-137) + 2 (D-137) + 3-external (D-138) PORTED;**
remaining fenced: the rule-RHS re-entrant churn + windowed live-modify
property-reactivity + the pre-existing temporal-join ORDER latents.

### D-139: CEP E2 item C §1a (windowed-accumulate live-modify property-reactivity) — PORTED; the D-137 "plain re-folds on ANY modify" finding CORRECTED

Resumed the readiness-ordered CEP backlog at item 1a — the windowed-accumulate
LIVE-modify over-fire that the class-2 `windowed_acc_types` fence had been
hiding (`xf_cep_c_upd_win_{live,noop}`: engine re-folds a windowed accumulate on
a no-op/irrelevant field update ⇒ `W2 W2`; oracle `W2`). Flagged in the notes as
the "do-not-hand-tune WindowNode wall", so PROBE-FIRST — a 28-cell oracle matrix
(no advance ⇒ pure update reactivity, isolated from expiry/eviction).

**The rule (probed, deterministic 3×; OVERTURNS the D-137 finding).** An external
in-place UPDATE of an accumulate SOURCE event that keeps it matching (alpha still
passes, still in window) re-fires the rule iff the updated field intersects the
node's WATCH MASK:
  watch(PLAIN accumulate)    = source CONSTRAINT fields ∪ source BINDING fields
  watch(WINDOWED accumulate) = source BINDING fields ONLY  (constraints dropped)
The D-137 finding claimed "plain re-folds on ANY modify" — FALSE: a plain
`count(tag=="y")` does NOT re-fire on an unread-field update (`p_cnt_other`=1,
`pr_cep_c_plain_cnt_other`). Plain IS property-reactive, on the standard source
mask; WINDOWED is reactive on a NARROWER mask that drops the alpha constraints.
Discriminators: `w_bind_notfn`=2 (a bound-but-fn-unused `$w:oth` update RE-fires
⇒ the mask is BINDINGS, not fn-args) and `w_cnt_valconstr`=1 (a constraint-only
`val>5` update does NOT ⇒ constraints excluded). The timestamp follows the same
rule (watched iff BOUND, independent of driving window membership): the fuzz's
`sum($t:ts)` re-fires on a ts update (`fz_wsum_ts`=2) but `count()` never does
(`fz_wcnt_ts`=1). Executable spec: `tools/model_check_react.py` (32 cells,
engine==oracle==predicate).

**Why it fits the engine cleanly.** An accumulate node's SOURCE events are its
RIGHT inputs (engine.rs:1638), so an in-place source update flows through the
`on_update` trie `(true,true)` branch (~engine.rs:5083), gated on
`pat.listen_mask & mask`. `listen_mask` = constraints∪bindings (a Bind sets both
listen_mask and bind_fields at compile; a Cmp sets only listen_mask), and
`bind_fields` = bindings-only — so the watch-mask difference is EXACTLY
listen_mask-minus-constraints = bind_fields. **Port (one scoped block):** for a
WINDOWED accumulate node (`pat.acc.window_time.is_some()`) gate the source re-fold
on `bind_fields`; plain/join/not/exists keep `listen_mask` (byte-identical). A
constraint-field update on a windowed source is now a mask-miss (immediate
right-memory re-add, no re-fold, no re-fire). The `mask==u64::MAX` bare/class
short-circuit is preserved (unreachable from external field-updates, which always
carry a specific mask — engine.rs:3341 — so the fuzz/probes never hit it).

**Fuzz.** Lifted the `windowed_acc_types` UPDATE fence in `fuzz_cep.py` (class 2
was already ported D-137; §1a now too). 3×800 (seeds 401/402/403) = 2400 cases,
0 NEW divergences. The lone flag `cf401x344` is the pre-existing NON-temporal
`not E0() P()` firing-ORDER latent (CURRENT-ISSUES item #2, the cf313 family) —
it bisects byte-identical to HEAD (engine change stashed) and has no windowed
accumulate at all; the fence-lift's RNG shift merely re-rolled the fuzz surface
onto it. NOT a §1a regression. The `temporal_types` UPDATE fence STAYS (item 1b,
the separate pre-existing temporal-join ORDER latents). Blast-radius: the general
`gen.rs` fuzzer emits ZERO windows ⇒ `window_time.is_some()` is never true there
⇒ the change is PROVABLY inert on the main axis (byte-identical, no re-run needed).

**Gates (green @ working tree):** `make diff` 11 / **980** / 288 byte-identical
(`xf_cep_c_upd_win_{live,noop}` GRADUATED → `pr_cep_c_upd_win_{live,noop}` + 4
discriminators `pr_cep_c_{win_sumc_tag,win_bind_notfn,win_cnt_valconstr,plain_cnt_other}`);
`make lint-probes` **1367**·0·0; `cargo test` 9 suites; bindings pytest 72;
class-1a fuzz 0-div 3×800; `model_check_react.py` 32/32. ⇒ **CEP E2 item C §1a
CLOSED.** Remaining item-C fenced: the rule-RHS re-entrant exists churn (D-138,
not fuzz-reachable) + item 1b temporal-order latents + item #2 non-temporal
not-order (both pre-existing, model-first).

### D-140: item #2 non-temporal `not <EVENT>() P()` firing-ORDER — ENGINE PORT LANDED. The banked MODEL (`model_check_notorder.py`, `9c6735c`) is now ENFORCED by a post-hoc AGENDA reorder, gated to the CLEAN unblock regime. Corpus byte-identical; cf313x13/cf401x344 FIXED (A/B-proven); ~4200 event-fuzz + 360 plain engine-vs-oracle 0-div; ZERO blast-radius

**What.** On unblock of a non-temporal `not <EVENT>() P()`, the blocked P's fire
grouped by BATCH = each P's LAST-TOUCH epoch: epoch batches REVERSE (newest first),
the INITIAL batch (epoch 0) LAST; within a batch, inserts (insertion order) precede
updates (newest apply first). This is the D-checkpoint-`9c6735c` model, now enforced.
Engine pre-port: event-EXPIRY full-LIFO, event-DELETE full-FIFO (both ~90% divergent
vs oracle); PLAIN blocker already correct ⇒ LEFT UNTOUCHED (gate excludes it).

**How — approach (b), post-hoc agenda reorder (NOT the staging-flush rework).** The
firing order of a STATIC-salience rule = the order its terminal `queue` drains
(`fire_all` picks `idx=0`, removeFirst FIFO); the not→join fan lays those activations
down in the wrong order. Rather than re-plumb the delicate not→join stream staging
(the true D-125 analog, high corpus-flip risk), the pick is REORDERED in place — the
static branch of `fire_all` now mirrors the dynamic-salience `max_by_key`, choosing
the smallest `not_order_key` instead of FIFO. Five scoped pieces in `engine.rs`:
- **`FactTouch` stamp** (`fact_touch: HashMap<FactId, FactTouch>` + `upd_seq_next`):
  per external fact, `insert_epoch`/`epoch` = `fire_no` at insert / last-touch,
  `is_upd`/`upd_seq`. `fire_no` bumps once per `fire_all` (one per external epoch),
  so `epoch` == the model's batch index (initial-batch inserts = 0). Written in
  `after_insert` + `update_fact` (an update PRESERVES `insert_epoch`); cleared on
  `reset`. Insertion order (`gidx`) is free from the monotonic `FactId`.
- **`not_order_pos: Option<usize>`** on `CompiledRule`, computed at compile: `Some(P's
  tuple pos)` iff compiled patterns are exactly `[InitialFact, non-temporal NOT over
  an @role(event) type, positive P]`; else `None`. @role membership is set at type
  declaration (pre-compile). PLAIN blockers / temporal nots / any other shape ⇒ None
  ⇒ FIFO, byte-identical.
- **`not_order_key`** = the model as a sort key (event variant: reverse batches,
  epoch-0 last, ins-before-upd).
- **CLEAN-REGIME GUARD** (the crux). The model is validated ONLY where every fired P
  was staged while blocked in a PRIOR fire cycle. If ANY fired P was INSERTED in the
  current cycle (`insert_epoch >= fire_no`), the fire mixes an in-cycle stream — an
  immediate delete-unblock and/or a fresh post-unblock insert — whose NATURAL FIFO
  order the engine already emits correctly; the reorder FALLS BACK to `idx=0` so those
  stay byte-identical to HEAD.

**Why the guard (the two mischaracterizations corrected by probing).** (1) A first cut
reordered the whole queue greedily → broke `pr_cep_c_del_not` (a DELETE unblock with a
P inserted AFTER the delete: that P was NEVER blocked, so the oracle streams it FIFO
`[1,2]`, not reversed). (2) The obvious fix "reorder only `epoch < fire_no`" then broke
`cf401x344`: its P1 was UPDATED in the unblock epoch (last-touch = fire_no) yet was
inserted initially and blocked throughout — an update must NOT make a fact "fresh".
The reconciling insight: DELETE unblocks fire the released batch IMMEDIATELY (so a
same-epoch later insert fires separately, FIFO), while EXPIRY DEFERS to `fire_all` (so
a same-epoch insert joins the reversed batch) — `pr_cep_c_del_not` (delete) vs
`pr_cep_{u3,v3,v5}` (expiry) need OPPOSITE treatment of the unblock-epoch P, which a
per-fact stamp can't tell apart. But all four are cases the ENGINE ALREADY EMITS
CORRECTLY (pinned probes, FIFO). So the guard sidesteps the whole ambiguity: reorder
ONLY the clean regime (no in-cycle insert among the fired P's — exactly what
`model_check_notorder`/`fuzz_notorder` covers, unblock epoch always empty), and defer
to HEAD-identical FIFO otherwise. FENCED (untested, model-undefined): a P inserted
WHILE BLOCKED in the unblock epoch then released same-epoch — no corpus/fuzz case
exercises it; the guard classes it FIFO.

**Verified.** `make diff` **11 / 983 / 288** byte-identical (3 graduated pins
`pr_cep_not_order_ev_{expiry,delete,upd}`); `make lint-probes` **1370**·0·0; `cargo
test` 9 suites; `model_check_notorder.py` 0-div (event expiry+delete). Engine-vs-oracle
0-div: 1800 validated-seed + 2100 FRESH-seed event scenarios (expiry+delete) + 360
plain — ALL PASS. Witnesses `cf313x13` (`NE6: not E2() P()`) + `cf401x344` (`NE7: not
E0() P()`) A/B-PROVEN: PASS post-port, FAIL on stashed HEAD (causation). BLAST-RADIUS
ZERO: all 7 `gen.rs` main-fuzzer divergences across seeds 42/123/7 (11k cases) have NO
event type ⇒ `not_order_pos` None ⇒ code inert; `fz_42_258` A/B byte-identical HEAD vs
post-port. `gen.rs` reorder is engaged only for `not <event>() P()` in the clean
regime; every other path takes the `None`/guard FIFO arm. ⇒ **item #2 CLOSED.** Item
1b (temporal-join ORDER latents, the fz_42/123/7 family) remains the last pre-existing
model-first order gap; item #2's `not X() P()` mischaracterization as a PriorityQueue
tie (early recon) is fully retired — it is DEFINED by code and now enforced.

### D-141: item 1b Family A — temporal-join UPDATE re-propagation. RECON corrected the "ORDER latents" label (it is dominantly a SET/COUNT family), NAILED the mechanism (a CEP event's temporal position is INSERT-FIXED; the engine re-read the LIVE ts), and PORTED the fix (a `store.event_ts` snapshot). 8/12 witnesses + both minimal repros FIXED; corpus byte-identical; blast-radius analytically ZERO

**Recon overturned the label.** Lifting the `fuzz_cep.py` `temporal_types` UPDATE fence
(seeds 313/401/402/403 ×400) surfaced 12 divergences, ALL bisect-to-HEAD (stash D-140 →
identical). NOT "ORDER latents" (the DECISIONS guess): **10/12 are firing-COUNT/SET**
divergences from re-propagating a temporal event's UPDATE; only 2/12 are order. Perfect
correlation: the 10 SET all have a ts-update on a temporal event; the 2 order have none.

**Family A (10/12) mechanism — NAILED, both directions.** A CEP event's TEMPORAL
POSITION is fixed at insert; the ts FIELD stays mutable (probe `ts_field_mut`: oracle
fires `R(ts>100)` on the updated value ⇒ non-temporal reads see the update). But the
engine's temporal-join eval + index keys re-read the LIVE field, so a ts-update
over/under-fires the join. Minimal repros: `tj_ts_update` (E2.ts 30→211 ⇒ engine JOINS
on 211, oracle doesn't on fixed 30 — engine 1/oracle 0) and `tj_ts_update_under`
(200→30 ⇒ oracle JOINS on 200, engine doesn't on 30 — engine 0/oracle 1).

**Fix — `FactStore.event_ts` insert snapshot.** `after_insert` snapshots each event's
ts (`set_event_ts`); the three temporal ts-reads — `JoinEnvImpl::allowed` +
`allowed_ce` (`bs`/`as_`) and the index keys `key_of_left`/`key_of_right` — now read
`store.temporal_ts` (the snapshot, else the live field). The deadline was already
insert-fixed (`schedule_expiration` snapshots at insert; not re-scheduled on update), so
it is untouched. NATURALLY byte-identical: snapshot == live unless the ts is UPDATED, so
the whole clean corpus is unaffected; only the fenced ts-update regime changes.

**Verified.** `make diff` **11 / 986 / 288** byte-identical (3 pins
`pr_cep_tj_ts_{update_overfire,update_underfire,field_mutable}`); `make lint-probes`;
`cargo test` 51. Bisect (stash D-141 → D-140-HEAD): 8 witnesses FAIL→PASS, the 4
residuals byte-IDENTICAL (D-141 only fixes, never worsens). Fresh-seed fence-lifted fuzz
(404-409, 2400 cases): **ZERO temporal-JOIN divergences** ⇒ the join-ts family is
complete; the only residuals are windowed-accumulate + not-order (see below).
BLAST-RADIUS analytically ZERO: `gen.rs` emits NO temporal ops and updates only bool
guard fields (never event ts), so `temporal_ts` is never read on the main axis (same
class as D-139's "gen.rs emits no windows"); corpus byte-identity confirms no
temporal-ts-update anywhere in the corpus.

**Residuals — the item-1b tail (deferred, all bisect-to-HEAD, findings
`~/.claude/plans/cep-item-1b-findings.md`):**
- **A2 — windowed-accumulate over an updated ts** (`cf401x25`/`cf401x42`, W3 =
  `accumulate(E2($t:ts) over window:time; sum($t))`; fresh W2 hits). SEPARATE
  sub-mechanism (window membership / re-fold over a ts-update), NOT the join eval —
  D-141 leaves it byte-identical. Related to D-139 windowed-accumulate reactivity.
- **B — not-order in the temporal regime** (`cf401x362` event-`not`, `cf313x4`
  plain-`not`; no ts-update). The clean D-140 model does not hold under surrounding
  temporal activity (e.g. `cf401x362` P1-updated-in-unblock-epoch: D-140 predicts
  [1,2] but oracle is [2,1]). A distinct order coupling; deferred.
The fz_42/123/7 main-fuzzer latents are UNRELATED to item 1b (they carry no event
type — accumulate/identity-hash family; D-140's blast-radius A/B already showed
`fz_42_258` byte-identical HEAD-vs-port).

### D-142: item-1b Family B (existential firing-ORDER in the temporal-EXPIRY regime) — model-first RECON + infrastructure. KEY FINDING: the order turns on BLOCKER-vs-P insert position; D-140's `fuzz_notorder` was blocker-FIRST only, so its model is that special case. Deterministic ⇒ portable; MODEL NOT yet cracked (the ACTIVE slab)

**Scope.** The Family-B tail (D-141 §Family B): `not`/`exists` firing-order when the
blocker leaves by EXPIRY under surrounding temporal activity. Witnesses `cf401x362`
(event-`not E0() P()`), `cf313x4` (plain-`not D() P()`), `cf407x121` (`exists E1() P()`
— an EXISTS; the earlier "NE6 not-order" label was wrong). DETERMINISTIC across 5 fresh
oracle runs ⇒ defined-by-code, PORTABLE — NOT the fence-by-nature `java.util.
PriorityQueue` tie of D-134 §6. Order-only, low-impact; Bryan chose the full model-first
arc for CEP order-faithfulness.

**Infrastructure (committed `dec1c0e`, no engine change).** `tools/fuzz_notorder_b.py`
(population capture: event-`not` expiry unblock with P inserts/updates across epochs
INCLUDING the unblock epoch + mid-run blocker arrivals; P-FIRST initial order) +
`tools/model_check_notorder_b.py` (predict harness; `MODEL=d140` = the item-#2 model).

**THE KEY FINDING — blocker-vs-P insert position.** A `not E0() P()` with P2 inserted
in a prior epoch and P1 UPDATED in the unblock (advance) epoch orders:
- blocker inserted BEFORE P (`[E0, P1]`) ⇒ the update PROMOTES P1 (last-touch batch,
  = D-140) ⇒ `[1,2]`;
- blocker inserted AFTER P (`[P1, E0]` — the REAL-witness order, `cf401x362`) ⇒ NO
  promotion ⇒ `[2,1]`.
This is why D-140 looked complete: its `fuzz_notorder` put the blocker at idx0
(blocker-FIRST) EXCLUSIVELY, so its validated population never left the easy regime;
the general P-first regime IS Family B. `fuzz_notorder_b` initially inherited that
blind spot (blocker-first ⇒ d140 matched 566/566); fixed to P-first ⇒ d140 diverges
~55% (132/244, 146/244), correctly reproducing the divergence. A mid-run blocker
ARRIVAL is just another way to land "blocker after P".

**Multi-dimensional (D-125-class).** Also confirmed to move the order: blocker COUNT
(`notB_2init`), NON-FINAL advances (`notB_2adv`), and epoch structure. Update POSITION
vs the advance does NOT (`notB_base` == `notB_base_before`). CONTRADICTION proving a
model is needed (not a one-liner): `cf401x344` (P-first, multi-blocker, updated TWICE)
PROMOTES → `[1,2]`; `notB_base` (P-first, single blocker, updated once) does NOT →
`[2,1]` — blocker-position is necessary, not sufficient.

**STATUS + next.** Model NOT cracked (d140 ~55% divergent on the P-first population).
NEXT (fresh-context runbook at the TOP of `~/.claude/plans/cep-item-1b-findings.md`):
refine `predict()` to 0-div, then PORT (extend the D-140 `fire_all` reorder +
`FactTouch` with the blocker-position/count/advance axes, event-gated, corpus
byte-identical). This slightly recontextualizes D-140 (its validation was blocker-first
only) but D-140's LANDED state is unaffected — corpus byte-identical, and P-first
not-order is exactly this documented tail.

### D-143: item-1b Family B — the SEGMENT model, CRACKED + PORTED. The event-`not` EXPIRY firing order in the P-FIRST regime is the SEGMENT model (P's grouped by event-insert count, newest-segment-first); enforced by a regime-branched extension of the D-140 reorder. Model 0-div on 2750 scenarios; engine diff 0-fail on all; corpus byte-identical (11/989/288); real witness cf401x362 A/B FAIL→PASS; blast-radius zero

**The model (`tools/model_check_notorder_b.py`, `MODEL=seg`).** `not E0() P()`, blocked
P's fire at the expiry-unblock advance. A SEGMENT counter advances on EACH E0 INSERT
(initial blocker AND every mid-run arrival). Each released P records `ins_seg` (segment
at insert) and, if updated, `upd_seg` + a global apply-seq (segment at last update). A P
updated into a LATER segment than its insert RE-STAGES into that segment; a same-segment
update does NOT move it. FIRE ORDER = segments NEWEST-first; within a segment, INSERTS
(insertion order) then UPDATES (newest apply first). Derived probe-first (controlled
oracle scenarios): pure-initial → forward; 2 insert-epochs → forward (NOT D-140's
reverse — the tell); update = move-to-front, refined to segment re-stage; mid-run
arrival = a segment boundary; update to an already-in-segment fact = no-op.

**Why segments, not D-140 epochs (the blocker-position resolution).** D-140's
`fuzz_notorder` was BLOCKER-FIRST (E0 before any P) — there every epoch flush is blocked
so each epoch is its own segment (⇒ epoch reversal, the D-140 model). In the P-FIRST
regime (a P inserted before the blocker — the real witness cf401x362 + the whole
`fuzz_notorder_b` population) epoch boundaries do NOT segment; only E0 inserts do. So
D-140's epoch key is the blocker-first SPECIAL CASE; the segment model is the general
rule. This retires the D-142 "multi-dimensional (count/advance/position)" framing — all
those axes fall out of the single segmentation.

**The port (`engine/src/engine.rs`, gated to the existing `not <event>() P()` shape).**
`FactTouch` += `ins_seg`/`upd_seg`; a monotonic `event_seg` bumps on every event insert
(`after_insert`), stamped at insert and re-stamped at update. `seg_order_key` = the model
as a `min_by_key` sort key `(-seg, insert<update, tie)`. The `fire_all` reorder BRANCHES:
P-FIRST (a released P has `ins_seg==0` ⇒ inserted before the first blocker) →
`seg_order_key`; BLOCKER-FIRST → the D-140 `not_order_key`, byte-identical (the pins).
The regime is LATCHED per-rule (`RuleNet.seg_p_first`, sticky-true) — the signal P fires
and leaves the queue, so re-deriving it per pick flips the regime mid-drain (the
nb801x110 bug: seg-2 tail mis-picked the epoch key once the `ins_seg==0` P left → caught
in validation, fixed by the latch).

**Verified.** `model_check_notorder_b seg` 0-div on 733 (seeds 801-803) + 1938 fresh
(901-905). ENGINE diff 0-fail on all 2750 individual scenario files. `make diff`
**11 / 989 / 288** byte-identical (+3 graduated pins `pr_cep_not_order_ev_pfirst{,_arr,
_upd}` — P-first counterparts of the D-140 blocker-first pins). `make lint-probes`
1376·0·0; `cargo test`. cf401x362 (the real event-`not` witness) A/B **FAIL→PASS**
(stash the engine port → FAIL, restore → PASS = causation). Fence-lifted `fuzz_cep` A/B
(seeds 401/313/407/410/411 ×120 identical divergence sets; 420-427 ×200 = 1 pre-existing
temporal-join divergence, A/B-unchanged) ⇒ ZERO new divergences across ~3k fresh cases.
BLAST-RADIUS analytically ZERO: `gen.rs` emits no event types ⇒ `not_order_pos` always
None AND `event_seg` never bumps ⇒ the whole Family-B path is dead on the main axis
(same argument as D-140/D-141); fuzz 42/123/7 = 1/2/2 divergences (the pre-existing
accumulate/identity-hash family, unchanged).

**Remaining Family-B tails (SEPARATE mechanisms, gate excludes them — NOT this port):**
`exists E1() P()` (cf407x121 — EXISTS not `not`, own model needed); fence-lifted
plain-`not` order (cf313x4 — plain blocker ⇒ D-140/D-143 leave plain firing order
alone; the FENCED cf313x4 passes); A2 windowed-accumulate-over-updated-ts (cf401x25/42,
cf423x107 — the D-139/D-141 reactivity tail). And the mixed-regime corner `e_p_blk_p`
(an initial P inserted AFTER the blocker — needs epoch+segment composition; outside the
population, absent from the corpus, no fuzz witness). The whole item-1b arc (Family A
D-141 ts-snapshot + Family B D-143 segment order) is now LANDED except these documented
tails; findings `~/.claude/plans/cep-item-1b-findings.md`.

### D-144: item-1b Family B (exists) — `exists E1() P()` witness-toggle RE-FIRE order. The re-fire order is the D-140 EPOCH model (reuses `not_order_key`), gated to RE-FIRES only (the FIRST satisfaction fires FIFO). Ported by extending the D-143 gate to `exists`; corpus byte-identical (11/991/288); engine-vs-oracle 0-fail on 2150+ scenarios; the real witness cf407x121 improves but its satisfying-epoch-insert sub-case (regime 2) is a documented FENCE

**Scope + the divergence.** `exists E1() P()` (E1 an @event) — P's fire when the
witness EXISTS; each satisfy transition (live-witness 0→1) re-fires the whole held P
memory. While the witness is ABSENT the P's stage (delete/expiry drop it); on re-arrival
they re-fire. The RE-FIRE ORDER was FIFO in the engine but epoch-reversed in Drools (all
divergences ORDER-only — 0 count/multiset diffs across the population, so the engine's
batch structure incl. expiry transient-fires + multi-toggle is already correct).

**The model (`tools/model_check_exists.py`, cracked probe-first).** The re-fire order is
the **D-140 EPOCH model** — batch by last-touch epoch, REVERSE (newest first), the INITIAL
epoch LAST; within a batch INSERTS (insertion order) then UPDATES (newest apply first); a
P updated in a later epoch re-stages into that batch. So exists re-fire == the D-140
blocker-first `not` order, NOT the D-143 P-first SEGMENT model (probes rejected the
mirror-of-not segment variant: `a_updP1`/`ex601x16`/`ex601x10` fixed the within-segment
order to epoch batches). The FIRST satisfaction is special: it fires the accumulated P's
FIFO/forward (a P-set spanning epochs first-fires in insertion order — cf. the corpus pin
`pr_cep_v4_exists_two_held_gens`); only after the witness TOGGLES do the batches reverse.

**The port (`engine/src/engine.rs`) — REUSES the D-143 machinery.** The compile gate
extends to `exists <EVENT>() P()` (`is_exists`, `CompiledRule.order_exists`); a gated
exists uses `not_order_key` (the epoch key) unconditionally — never the P-first
`seg_order_key`. Two exists-specific pieces: (1) `RuleNet.last_fire_no` (committed at the
FIRE BOUNDARY, not per pick — else it flips the regime mid-drain, the seg_p_first-latch
lesson, `ex601x10`) makes the reorder engage only on a RE-FIRE (`last_fire_no < fire_no`);
the first satisfaction falls to FIFO. (2) the `in_cycle` guard stays for exists too,
FENCING regime 2.

**Verified.** `make diff` **11 / 991 / 288** byte-identical (+2 graduated pins
`pr_cep_exists_order_refire_{epoch,update}`); `make lint-probes` 1378·0·0; `cargo test`.
Engine-vs-oracle diff **0-fail** on 2150+ exists scenarios (`fuzz_existsorder.py`, seeds
501-503 + 601-603 + 701-703, multi-toggle + expiry + delete). `model_check_exists.py`
0-div on the clean delete-single-toggle regime (its simplified sim doesn't replicate
expiry transient-fires; the engine diff is the full gate). Fence-lifted `fuzz_cep` A/B
(seeds 407/420/421/422): ZERO new divergences. D-143 (`not`) preserved (cf401x362 PASS,
notpop 0-fail). BLAST-RADIUS zero (gen.rs no events ⇒ gate None ⇒ path dead; fuzz 42
unchanged).

**FENCED — regime 2 (the cf407x121 residual):** a P inserted in the SATISFYING epoch
(alongside the re-arrival witness). A P inserted BEFORE the witness joins the newest
batch (epoch-reorder); one inserted AFTER fires last (FIFO) — an insert-vs-witness timing
split the `insert_epoch` stamp can't distinguish (dropping `in_cycle` fixed the before
case but broke the after case: 110→239 divergent). So exists re-fires with a
satisfying-epoch insert fall to FIFO (matches HEAD — not a regression; cf407x121's NE6
`[1,2,3,2]` vs oracle `[1,3,2,2]`). Needs a per-P before/after-witness segment bit; left
as the documented exists tail. Other item-1b tails unchanged (plain-`not` cf313x4,
windowed-accumulate A2, mixed-regime `e_p_blk_p`).

### D-145: mixed-regime `not` order (`e_p_blk_p`) — hand-built witness FILED + candidate rule cracked at hand-probe level. `scenarios/xfail/xf_cep_not_order_mixed_initial.json`; 10/10 probes fit "within a segment: epoch-inserts, then updates, then the POST-BLOCKER EPOCH-0 INITIALS last" — a strict extension of D-143, NOT yet population-validated

**The corner.** Initial P's on BOTH sides of the blocker (`[P1, E0, P2]`) — the shape
neither D-140's fuzz (blocker-first) nor D-143's `fuzz_notorder_b` (all initials before
the blocker) ever generated, absent from the certified corpus and the fence-lifted
sweeps. DETERMINISTIC, order-only (SET 0-div): oracle `[3,2,1]` vs engine `[2,3,1]`
(the D-143 seg model latches P-first via P1 and orders seg1's inserts purely by gidx;
the oracle demotes the post-blocker epoch-0 initial P2 behind the epoch insert P3).

**Candidate rule (hand-probed 10/10, `$JOB/tmp/mprobe.py` — m_base/m_0ep/m_2ep/
m_2in1ep/m_2after/m_2before/m_updP1/m_updP2mid/m_updP2unb/m_upd2/m_upd2r/m_arr).**
Segments (blocker-insert count) newest-first exactly as D-143; WITHIN a segment =
`[epoch>=1 inserts, gidx asc] ++ [updates, apply-seq desc] ++ [epoch-0 initials, gidx
asc]`. Class moves: an epoch-0 initial UPDATED at all — even same-segment — promotes
into the updates slot (`m_updP2mid` `[3,2,1]`); an epoch>=1 insert updated same-segment
stays an insert (D-143's nb801x0 no-op); updates order newest-apply-first (`m_upd2`
`[3,1,2]` vs `m_upd2r` `[3,2,1]`). Epoch batches inside the segment go FORWARD
(`m_2ep` `[3,4,2,1]` — my first reversal hypothesis was WRONG; the probes corrected
it). REDUCES to D-143 exactly when no segment holds post-blocker initials, so the
port would be a pure extension of `seg_order_key` (an initials-last class + the
epoch-0-update promotion), byte-identical on the validated population by construction.

**Status: witness + candidate only — NOT ported.** The method lesson stands: hand
battery ≠ population spec. To close: extend `fuzz_notorder_b` with mixed initial
positions (P's after the blocker, including multi-blocker/arrival interactions with
the untested epoch-0-initial-updated-across-an-arrival case), model_check to 0-div,
then extend the key. xfail is outside the diff tiers ⇒ corpus gates unchanged
(**11/991/288** re-verified). Impact remains LOW (order-only, out-of-distribution).

### D-146: mixed-regime `not` order PORTED — the D-145 candidate rule population-validated to 0-div (`MODEL=seg2`, ~5150 scenarios incl. the D-143 reduction) and enforced by a `seg_order_key` class extension; the D-145 witness GRADUATED. BONUS DISCOVERY: BLOCKER-FIRST **with arrivals** is a NEW uncracked latent family (D-140's population had none) — recon'd + fenced with witnesses

**The population.** `tools/fuzz_notorder_b.py` now places the blocker at a RANDOM
position among the initial P's (`n_before ∈ [1,3]`, `n_after ∈ [0,2]`), spanning
P-first + MIXED (post-blocker epoch-0 initials, incl. epoch-0 initials updated ACROSS
an arrival — the untested D-145 interaction, generated naturally by the epoch
updates + arrival machinery). `SEINE_NOTPOP_BF=1` additionally allows `n_before==0`
(blocker-first-with-arrivals) for the new-family recon only.

**The model (`tools/model_check_notorder_b.py MODEL=seg2`) — 0-div.** Exactly the
D-145 candidate: segments newest-first as D-143; WITHIN a segment `[epoch≥1 inserts,
gidx] ++ [updates, apply-seq desc] ++ [epoch-0 initials, gidx]`; move rules: update
into a LATER segment moves (D-143), an EPOCH-0 initial updated at ALL promotes into
the updates slot, an epoch≥1 insert updated same-segment stays. Validated: 743 (seeds
811-813) + 1736 FRESH (821-825) mixed scenarios ALL MATCH + the 2671 OLD D-143
populations ALL MATCH (the strict-reduction proof — seg2 == seg where no post-blocker
initials exist).

**The port.** `seg_order_key` gains the class split: `moved := is_upd && (upd_seg >
ins_seg || insert_epoch == 0)`; key `(-seg, class{0=ins,1=upd,2=epoch-0-initial},
tie)`. Order-identical to the D-143 key on the validated population by construction
(class 2 only exists in seg0 there). Verified: engine diff 0-fail on all 2500 new +
2750 old population files + 1950 exists files; `make diff` byte-identical; the D-145
witness PASSES → GRADUATED to `scenarios/regressions/` (the D-136 convention) + a new
pin `pr_cep_not_order_mixed_upd_promote` (the epoch-0 update-promotion); mprobe
battery 12/12; fence-lifted fuzz spot = the known residual set exactly.

**NEW FAMILY (fenced): BLOCKER-FIRST with mid-run arrivals.** ~31% divergent (16/51,
seed 831) vs the engine's D-140 key — D-140's population had NO arrivals, so its
epoch model is the no-arrival special case. UNCRACKED: d140 40/127, the P-first class
model 60/127, segments-desc+d140-within 33/127; hand analysis flip-flops
(nb811x8's mid-epoch-update-promotes vs nb811x110's unblock-update-demotes;
`xf_cep_not_bf_arrival`'s [3,5,4,1,2] isn't even segment-descending) ⇒ needs its own
model arc. Witnesses `scenarios/xfail/xf_cep_not_bf_arrival{,2}.json`; deterministic,
order-only, out of every validated distribution. Engine behavior there = HEAD
(unchanged): no P has `ins_seg==0` ⇒ the seg branch never engages.

### D-147: exists regime-2 SOLVED — the D-146 segment lens transferred: a P inserted while SATISFIED is a FRESH stream insert (fires after the re-fire batch), detected by `ins_seg >= RuleNet.satisfy_seg`; plus an `ins_seg`-DESC within-batch sub-key. The D-144 `in_cycle` fence is REPLACED; **cf407x121 PASSES** (left the fence-lifted residual set); all populations 0-fail

**The insight transfer (the point of the exercise).** The D-144 fence ("a P inserted
in the SATISFYING epoch — before/after-witness timing `insert_epoch` can't tell
apart") is exactly a SEGMENT question: the witness insert bumps `event_seg`, so a
before-witness P has `ins_seg < S_w` and an after-witness P has `ins_seg >= S_w`.
THE RULE (cracked via `model_check_exists.py` + a dedicated population): a P inserted
while SATISFIED (at/after the transition witness) fires IMMEDIATELY as a fresh stream
insert, arrival order — NOT inside the re-fire batch; a before-witness insert joins
the batch as its newest epoch (touch epoch == fire_no ⇒ fires first). REFINEMENT
(multi-toggle populations): within an epoch batch, inserts sub-order by `ins_seg`
DESC then insertion (a P inserted after a mid-epoch witness arrival precedes an
earlier one — ex801x145).

**The port.** `RuleNet.satisfy_seg` = `event_seg` stamped at every EMPTY→NON-EMPTY
queue enqueue (`push_activation`) — for a gated exists that is the satisfy transition
(the witness bumped `event_seg` just before its flush enqueued the batch; fresh P's
enqueue onto the non-empty queue without re-stamping). The gated-exists re-fire pick
replaces the D-144 `in_cycle`-FIFO fallback with the split: fresh (`ins_seg >=
satisfy_seg && insert_epoch >= fire_no`) → FIFO tail; else the D-140 epoch key + the
`ins_seg`-DESC insert sub-key. The `not` arm keeps its `in_cycle` guard unchanged.

**Verified.** Engine diff 0-fail: 600 regime-2 (delete-clean, seeds 841-843) + 400
multi-toggle/expiry+regime-2 (expop_ins — was ~110 order-divergent) + 500 FRESH
(851-852) + all 1950 D-144 populations + 100 from the re-banked generator. `make
diff` **11 / 994 / 289** byte-identical (+2 pins `pr_cep_exists_order_{satisfying_ins,
insseg_subkey}`); lint 1382; cargo test. **cf407x121 PASSES** and is GONE from the
fence-lifted residual set — remaining: cf313x4 (plain-not), cf401x25/42 + cf423x107
(windowed-accumulate A2), all non-existential families. Tools re-banked:
`fuzz_existsorder.py` (regime-2 axis) + `model_check_exists.py` (the full rule; sim
0-div on delete-toggle regimes, expiry validated via engine diff). Blast-radius zero
(gen.rs no events; corpus byte-identity). The exists tail is CLOSED; item-1b's
remaining tails: plain-not cf313x4, A2 windowed-accumulate, and the new D-146
blocker-first-with-arrivals family.

### D-148: the D-093 upstream defect is FIXED IN DROOLS — apache/incubator-kie-drools#6796, authored by Mario Fusco, merged 2026-07-08 (ONE DAY after the report), adopting the report's suggested `isDirty |=` repair VERBATIM at both arms + a regression test derived from the filed reproducer. No release carries it yet ⇒ the pinned oracle keeps the defect and the D-093 quarantine/wall REMAIN; convergence graduation is queued for the next oracle bump

**The event.** apache/incubator-kie-issues#2366 (the D-093 report, filed 2026-07-07)
was CLOSED 2026-07-08T13:06Z by PR **apache/incubator-kie-drools#6796** — "Fix
accumulate for non-reversible functions", authored by **Mario Fusco** (Drools lead;
merged by Gabriele Cardosi, merge commit `275baf9c`). The change is EXACTLY the
repair the report suggested: `isDirty = accumulate.hasRequiredDeclarations()` →
`isDirty |= …` (the re-add arm) and `isDirty = !reversed` → `isDirty |= !reversed`
(the removeMatch arm) in `PhreakAccumulateNode.doLeftUpdatesProcessChildren` — the
last-writer-wins clobber D-092 identified as the mechanism. The added upstream test
(`AccumulateTest.testAccumulateMinStaleAfterLeftTupleUpdate`) is the report's
reproducer shape verbatim (P/S/G, `b: -10→5`, sources `{12, -2}`, expected
`[-2, 12]`). External validation of the whole differential-testing arc: D-092's
mechanism analysis, D-093's doctrine call (Seine corrects value-bearing defects),
and the report were all adopted upstream in one day.

**What changes NOW: nothing.** The oracle stays pinned at 9.44.0.Final (defect
present); the latest release 10.2.0 (2026-04-28) PRE-dates the merge and Apache
snapshot publishing is stale (999/10.x.999 lastUpdated Mar/Apr 2026), so no
fetchable artifact carries the fix — the empirical convergence check defers to the
next release, exactly the path D-093 anticipated ("track the issue when bumping
oracle versions"). The D-093 quarantine (documented-expected-divergence witnesses)
and the generator wall (min/max mutation-free) REMAIN IN FORCE.

**Queued for the next oracle bump (the convergence protocol):** (1) verify the bump
target's tag contains `275baf9c`; (2) re-run the seven quarantined witnesses —
`alu6a`, `alu7a/7d/7f/7g`, `fz_123_8426`, `fz_min_8426` — expecting CONVERGENCE
(Seine's correct re-derivation == fixed Drools) and graduate them into the normal
gate; (3) LIFT the D-093 generator wall (min/max accumulates re-enter mutation
scenarios; external updates stop rerouting to deletes) and re-fuzz the restored
surface; (4) update `docs/drools-bug-stale-minmax.md` (resolution banner added now,
D-148) and re-run the full corpus per the pin-bump protocol. Doctrine note: the
D-093 "faithfulness is to Drools-the-spec, not to defects" ruling is now vindicated
by Drools itself — the corrected behavior IS the spec.

### D-149: bf-with-arrivals RECON — the INSERT/ARRIVAL structure is CRACKED (per-epoch batches, newest-first, within-batch ins_seg-DESC — ZERO update-free mismatches), but UPDATE placement defeats every static key/batch frame (3456-combo sweep caps at 72.2%); the family is a per-arrival FLUSH-MACHINERY problem (the pre-D-125 signature) — fenced pending a graft + flush-simulation arc

**Population + probes.** PURE blocker-first population now first-class:
`SEINE_NOTPOP_BF_ONLY=1 tools/fuzz_notorder_b.py` (seeds 861-863, 694 orderable;
engine = d140 key, ~31% divergent). Three controlled probe batteries
(`$JOB/tmp/bfprobe.py` + x-/d-series inline): single-update × {target class:
epoch-0 initial / seg1 insert / seg2 insert} × {timing: mid / final-preADV /
final-postADV}, multi-update apply-order permutations, insert-after-update, and
x110/x47-minimizing shapes.

**CRACKED: the update-free structure.** Per-epoch INS-batches, NEWEST-epoch first,
within-batch inserts (ins_seg DESC, gidx) — i.e., D-140's epoch reversal + the
D-147 seg-DESC sub-key. Zero mismatches on update-free scenarios across the
population. (bf_base [4,3,1,2]: the post-arrival insert leads its epoch batch.)

**RESISTS: update placement.** A parameterized batch/key model
(`$JOB/tmp/bfsim.py`: promotion classes {epoch-0|ep≥1}×{same|cross-seg}×{mid|
preADV|postADV}, mid-join epoch-vs-pool, pool keys created/max, batch max-touch
bumping, final-join front/tail/adjacent — 3456 combos) caps at **72.2%** (best =
promote-always + seg-desc ≈ d140+segdesc). The smoking-gun FLIP-FLOP PAIRS: the
same update class places OPPOSITELY in near-identical shapes — bf_P1_pre
[1,4,3,2] (epoch-0 cross final-pre promotion FRONTS) vs nb861x58 [2,1] (TAILS);
d_ctl [2,3,1,4] (3 promotions front) vs the x110 shape [4,5,6,2,3,1] (3
promotions tail) vs d_P4P1 [1,4,3,2] (touch-then-promotion fronts); apply-ORDER
sensitivity is real (d_addP4first [2,3,1,4] vs d_addP4last [4,2,3,1]) but its
composition with batch position follows no static rule tried. Probing corrected
several D-146-era guesses: within-segment epochs are NOT purely reversed
(the batch frame is), and the promotion table drawn from nb811x8 alone does not
generalize.

**Conclusion + path.** The signature matches pre-D-125 temporal joins: the order
is GENERATED by the per-arrival STREAM flush/staging machinery (per-insert flush
windows, join-emission prepends, terminal drain cycles), not expressible as a
per-fact sort key over insert/update stamps. To close: the D-125/D-138
methodology — a GRAFT (Java staging dump on the not→join path, ExistsDump-style)
to observe the oracle's actual staging events, then a faithful per-arrival flush
SIMULATOR (model_join_flush.py-class) validated to 0-div, then port. A dedicated
arc. ENGINE UNCHANGED (bf stays at HEAD = the d140 key; corpus untouched —
tooling + docs only this entry). Witnesses `xf_cep_not_bf_arrival{,2}` updated
with the full recon. Item-1b tails now: plain-not cf313x4, A2
windowed-accumulate, bf-with-arrivals (recon'd, awaiting the flush-sim arc).

### D-150: bf-with-arrivals MECHANISM CRACKED via the graft arc — the order is generated by FIVE pieces of concrete Drools machinery (rtm list order + staged backlog + queue-position updates + quiescence expirations + not-node unlinking); a MECHANICAL simulator (`MODEL=flush`) is 0-div on 9041 scenarios ACROSS ALL THREE REGIMES (it subsumes seg/seg2/d140 on the event-blocker family) + 55 probes — ENGINE PORT AWAITS THE GATE

**The graft (BfDump.java).** ExistsDump-style runner graft, extended twice
during the arc: (1) listener-driven DYNAMIC dumps — matchCreated/Cancelled/
Fired + WM events each dump the not/join beta memories (right-memory ORDER,
staged R ins/del/upd, blocked lefts) and the RuleExecutor tupleList mid-fire;
(2) the decisive instrument: a reflection PROXY swapped over the session's
`ActivationsManagerImpl.propagationList` that logs every PropagationEntry
ENQUEUE and (by wrapping the takeAll chain) every entry EXECUTION with a state
dump after each. Plus slf4j-simple (jobs-tmp classpath copy; slf4j-nop swapped
out) enabling Drools' own RuleNetworkEvaluator/SegmentMemory/PathMemory TRACE
(eval-pass structure, LinkNode/UnlinkNode/LinkRule/Queue with masks).

**THE MACHINERY (all graft-observed; sources read for names only).**
1. **The hidden state is the JOIN's right-memory LIST ORDER (`rtm`) plus the
   staged-right-insert BACKLOG** — not any per-fact stamp. Firing order at an
   unblock = **reverse(rtm)**: doLeftInserts iterates rtm in order, each child
   is PREPENDED into the target staging, the terminal appends head-first
   (executor FIFO, fires front-first — PhreakRuleTerminalNode/RuleExecutor).
2. **Every external op is a FIFO PropagationEntry** executed inside
   fireAllRules. An EVENT insert (E0 at the not) FORCE-FLUSHES a network eval
   at its queue position (BetaNode.assertObject: `shouldFlush=isStreamMode()`
   → TupleEvaluationUtil.forceFlushLeftTuple → headerless outerEval — the
   D-125 per-arrival flush, now seen on the not-path). Each eval drains the
   join's staged-ins LIFO into rtm (batch-reversed; the emission reversal then
   makes within-batch firing FORWARD/gidx — the D-149 "cracked" structure).
3. **Expire entries only REGISTER** (WorkingMemoryReteExpireAction.execute =
   registerExpiration + mark-expired; NO retract at its position). ALL
   retracts run at QUIESCENCE — `ActivationsManagerImpl.flushExpirations`,
   AFTER every queued entry of that fireAllRules **including post-ADV
   updates** — in deadline order, each retract with its own force-eval
   (NotNode.doDeleteRightTuple ends in flushLeftTupleIfNecessary). Re-block
   scans surviving rtm_not; the LAST retract (counter 1→0) RELINKS the not,
   queues the rule, and its eval UNBLOCKS: emission = reverse(rtm). Expiry
   deadline = **ts + @expires + 1** (advance to the exact boundary does NOT
   expire — mu4 probe); an arrival already past deadline enqueues its expire
   action in the same flush.
4. **A bare-P update (empty inferred mask — `P()` has no constraints) is an
   IMMEDIATE `rtm.removeAdd` (move-to-tail) at its FIFO queue position**
   (BetaNode.modifyObject line-298 reorder-only branch; no staging, no
   executor queueing, no re-fire — updates NEVER re-fire this family, all
   re-fires come from unblock re-emission). An update of a still-STAGED P
   (memory==null) is a TOTAL NO-OP. Sequential updates = sequential
   move-to-tails (the d_addP4first/last apply-order sensitivity, exactly).
5. **P staging queues the fire-loop eval ONLY while the segment is fully
   linked** — and an E0 right-insert batch processed while the segment is
   linked **UNLINKS the unconstrained NotNode**
   (PhreakNotNode.unlinkNotNodeOnRightInsert; segment mask drops its bit;
   re-link at the last E0 retract). While unlinked, P inserts accumulate
   STAGED across epochs (nb861x58's ep2 backlog — the flip-flop's other
   half); a linked-era epoch drains per-epoch (the "per-epoch batches").

**Why every static key failed (D-146/D-149, 3456-combo sweep cap 72.2%):**
placement is the composition of (a) which drain WINDOW each insert lands in
(a function of linking history), (b) queue-position move-to-tails over the
CURRENT rtm (visible only against what has already drained — a move among
staged-invisible peers is silent), and (c) the quiescence rule putting even
post-ADV updates BEFORE the unblock emission. All three are history-valued;
no per-fact stamp carries them. The D-149 flip-flops dissolve: bf_P1_pre's
update moved a drained P1 to the rtm tail (fronts at emission); nb861x58's
identical-class update moved rtm=[P1] (identity) while P2 sat STAGED and
appended after at the retract drain (tails). d_ctl/d_P4P1/d_addP4first/last
are literal move-to-tail sequences. The out-of-sample discriminators
(unlk2ep/unlk3ep: merged multi-epoch backlog windows — [2,3,1]/[2,3,4,1]
where the D-149 per-epoch frame predicts [3,2,1]/[4,3,2,1]) were predicted
by the model BEFORE running and confirmed by the oracle.

**The simulator (`tools/model_check_notorder_b.py MODEL=flush`,
`predict_flush`).** ~120 lines replaying exactly the five pieces. Validated
**0-div on 9041 banked+fresh population scenarios**: pure-bf 694 (seeds
861-863) + 679 FRESH (871-873, SEINE_NOTPOP_BF_ONLY) + P-first (notpopb/2/3/
fresh, 801-803) + MIXED (notpopb_mixed, notpopb_m2 811-813/821-825) + val
(901-905) — i.e. **the mechanical model subsumes seg (D-143), seg2 (D-146),
and the d140 key on the ENTIRE event-blocker family**; they are per-regime
shadows of this machinery. Plus **55/55 probes**: all D-149 flip-flop pairs
and batteries (bfprobe_sc/2/3), the backlog discriminators (bk*/unlk*), the
update-history axes (u_*/ax*), and mid-run unblock/re-block/re-fire
timelines (mu1-mu4, 7-firing multi-unblock sequences). SCOPE: event
blockers (@role(event) @expires); the plain-fact `not D()` family
(fuzz_notorder.py, D-140) is DIFFERENT machinery (no stream force-flush, no
expiration deferral — explicit deletes retract at their queue position) and
stays on the landed D-140 model.

**Status.** ENGINE UNCHANGED — `xf_cep_not_bf_arrival{,2}` stay xfail (the
engine still runs the d140 key on bf shapes); corpus 11/994/289
byte-identical, lint 1382 live, cargo suites green. The graft
(oracle/.../BfDump.java) is committed as the reusable instrument (the
PropagationList proxy is the new RunnerDump-family tool). **PORT DESIGN
QUESTION FOR THE GATE:** the mechanical model suggests replacing the D-140/
D-143/D-146 key-based agenda reorder with a per-rule rtm-ORDER simulation
(a Vec the engine already effectively has in `FactStore` iteration order —
the port = maintain move-to-tail on update + drain-window bookkeeping +
reverse at emission), which would retire three phenomenological models and
their regime branches at once — but it touches the fire path of a landed,
byte-identical corpus, so: mechanism report filed, Bryan gates the port.

### D-151: EPICYCLES RETIRED — the mechanical BfShadow replaces the D-140/D-143/D-146 not-family key models in the engine; corpus byte-identical, 7,800+ engine-vs-oracle scenarios clean, the bf-with-arrivals witnesses GRADUATED, and the delete family improves by 203 cases with zero regressions

**Bryan's call:** "retire the epicycles model for the better-working elliptical
orbits model" — the D-150 mechanical model goes in the engine; the
phenomenological keys go.

**Spec extension first (delete ops).** `predict_flush` gained explicit-delete
semantics: an E0 delete retracts AT ITS QUEUE POSITION (D-138 delete-time —
unlike expiry quiescence), same retract eval (relink on counter 1→0, unblock
if last); a P delete annihilates its staged insert or leaves rtm + cancels its
queued activation. dl881x20 exposed a missing linking rule: **the JOIN itself
unlinks when its right counter hits 0 (last P deleted) and relinks on the next
P insert** — which flips whether a later E0 arrival unlinks the not
(join_count in the model). Validated on a 597-case delete-augmented population
(seeds 881-883; 63 invalid update-after-dead scenarios excluded as
oracle-NPE): 0-div, plus the full 9,041 + 55 regression sweep stays 0-div.
**Spec totals: 9,693 scenarios.**

**The engine port (`BfShadow` in engine.rs).** A per-rule shadow state machine
— rtm order, staged backlog, e0_alive, pending expirations, not/join link
bits, exec_queued — stepped by the EXTERNAL op stream at the exact hook
points the engine already stamps (`after_insert`, `update_fact`,
`delete_fact` (non-expiration only), `advance` in engine deadline order), plus
`pre_fire`/`post_fire` at the fire boundary. `pre_fire` replays the fire-loop
eval + quiescence expirations and ranks the predicted emission
(`emit_rank`); the gated static pick takes rank-min (FIFO tiebreak) instead
of the retired keys. `schedule_expiration` now returns the deadline-vs-clock
ordering (the due-on-arrival Equal case registers in the same flush; Less =
the D-132 leak, alive forever).
RETIRED: `seg_order_key`, `RuleNet.seg_p_first` + its latch,
`FactTouch.upd_seg` + stamps — the whole D-143/D-146 branch structure.
KEPT: `not_order_key` + `ins_seg`/`satisfy_seg`/`last_fire_no` (the gated
EXISTS path, D-144/D-147 — its mechanical treatment is a future arc) and the
D-140 in_cycle guard (a fired P inserted THIS cycle ⇒ FIFO — now the
RHS-regime fence).
STATIC exclusions keep the shadow inside its validated surface (else no
shadow ⇒ plain FIFO): bare patterns only (constraints∪bindings empty ⇒ the
empty inferred update mask), non-event P, distinct blocker/P classification,
no rule RHS inserting/mutating a gated type, no windowed accumulate over a
gated type. gen.rs emits no events ⇒ the main fuzz axis provably never
builds a shadow.

**Gates (all green).** Corpus `make diff` 11/994/**291** byte-identical —
every D-140/143/144/146/147 pin reproduced by the shadow; `xf_cep_not_bf_
arrival{,2}` now PASS engine-vs-oracle and are **GRADUATED to regressions/**
(fuzz suppression lifted). Lint 1384 live / 0 ghost / 0 inert; 9 cargo
suites. ENGINE-vs-oracle sweeps: pure-bf banked 694 + fresh 679 (seeds
871-873) CLEAN; P-first + mixed + val populations **5,100 scenarios CLEAN**
(the retirement reproduces everything the keys did); delete population
511/534 valid pass — the 23 residuals are all in_cycle-guard territory
(same-epoch insert + delete-unblock ⇒ FIFO fallback), where **HEAD failed
226**: the port fixes 203 delete-family divergences with **ZERO regressions**
(A/B against a HEAD worktree, oracle classpath grafted in). fuzz_cep seeds
313/401/407 ×400 = 0 divergences; fresh seed 511 ×400 A/B'd on both trees =
0 both.

**Standing scope notes.** The 23 delete-residuals are the in_cycle guard's
price — closing them means extending the validated spec into the same-cycle
insert regime (the guard exists because delete-immediate vs expiry-deferred
unblock inserts genuinely differ, D-140) — a future slab if wanted. The
gated-EXISTS family still runs the D-144/D-147 keys; the mechanical
treatment of PhreakExistsNode (witness-toggle emission) is the natural next
retirement. The D-134 §6 PriorityQueue tie stays fenced by nature.

### D-152: the EXISTS keys retired for the mechanical ExShadow — graft recon cracked the exists-side machinery (staged-IF deferral + segment-link gating), the spec went 0-div on 5,507 oracle scenarios across every axis the keys never saw, and the port fixes 1,012 full-axis divergences with zero regressions; BONUS: the D-133 expiration boundary was CORRECTED (nonneg-past deadlines register due-on-arrival; only NEGATIVE deadlines leak — DROOLS-455)

**Bryan's directive:** "Please retire exists the same way" — the D-151 arc
(graft → mechanical spec → shadow port) applied to the gated
`exists <EVENT>() P()` family, retiring the D-144 re-fire epoch key and the
D-147 regime-2 segment split.

**Graft recon (BfDump on ex501x14 / ex990x20 / ex990x32).** The D-150 frame
transfers, but THREE exists-side mechanics had to be observed, not derived
(two hand-derivation attempts predicted wrong orders before the dumps
settled it):
1. **The IF (InitialFact) left is itself a STAGED tuple at the exists** until
   the first fire-loop eval. An E1 force-flush processes RIGHTS along the
   path (drains the join backlog staged-LIFO into rtm) but staged LEFTS
   wait — so a FIRST satisfaction emits reverse(rtm) at the FIRE-LOOP eval,
   swallowing P's drained after the witness in the same window (ex990x20
   fires [3,1,2] where the retired key said FIFO [1,2,3]), while a
   RE-satisfy (IF resident, re-blocked by the arriving witness) emits at the
   witness's own exec after that exec's drain. The D-144 "first fires FIFO /
   re-fires reverse" split and the D-147 before/after-witness rule are both
   THIS seam — the banked toggle scaffold (witness always initial, single
   drain window) could not distinguish them.
2. **The fire-loop eval runs iff the RuleAgendaItem got QUEUED this window**:
   the satisfy-link COMPLETING the segment (witness count 0→1 with the join
   populated), P staging while the exists side is populated, or a
   terminal-reaching delete. One-sided windows queue NOTHING — witnesses
   alone never link the rule (ex990x32 sat with three witnesses and a staged
   IF for two epochs), P's alone never drain (ex990x20 cycle 0).
3. **Marked-expired witnesses keep counting AND blocking until their
   quiescence retract** (ex990x32 ep3: the IF blocks on a witness whose
   expire action already exec'd), and quiescence runs AFTER the agenda
   drained — pre-quiescence emissions FIRE (the transient fires the old
   sims could not represent; probes xm1-xm4).
Everything else transfers verbatim: rtm order + global prepend backlog,
reverse(rtm) emission via child-prepend, per-arrival force-flush drains,
bare-P update move-to-tail (never re-fires), explicit deletes at queue
position (cancelling queued activations), witness updates inert.

**The D-133 boundary CORRECTION (a bonus find).** The full-axis population's
due-on-arrival witnesses exposed that the D-133 rule ("deadline < clock ⇒
KEPT forever") conflated two cases — its probes ran at insert-clock 0, where
past ⇔ NEGATIVE. The real boundary (Drools
`PropagationEntry.Insert.scheduleExpiration`, read for names; oracle probes
xq1-xq3): **deadline < 0 ⇒ leak** (DROOLS-455 maps a negative effectiveEnd
to Long.MAX_VALUE = never); **0 ≤ deadline ≤ clock ⇒ the expire action
enqueues in the SAME flush** — the event matches + fires this cycle and
drops at the quiescence drain; **deadline > clock ⇒ scheduled**. Fixed in
`schedule_expiration` (engine.rs); the returned Ordering keeps its contract
(Equal = due-on-arrival, consumed by both shadows). pos_far/pos_ins and the
whole corpus are unaffected (negative-deadline and at-clock cases unchanged);
the not-family populations never generate nonneg-past arrivals (analytically
inert there). Pins: `pr_cep_exp_boundary_{leak,past}`.

**The spec (`tools/model_check_exists.py EMODEL=flush`).** The mechanical
simulator replays the pieces above; `fuzz_existsorder.py` gained
`SEINE_EXPOP_FULL=1` — free op soup adding every axis the banked scaffold
lacks: explicit P deletes, PARTIAL witness deletes (2→1), multi-witness with
staggered ts (partial expiry, deadline-order quiescence), DELAYED first
satisfaction, due-on-arrival witnesses, the leak boundary, witness updates
(inert), pure-P windows, action-interleaved inserts. Validated **0-div on
5,507 oracle scenarios**: all 14 banked D-144/D-147 populations (3,500;
seeds 501-703, 841-843, 851-852 regenerated) + 7 full-axis populations
(2,007; seeds 990-996) + probes xm1-4/xq1-3. The retired EMODEL=epoch key
fails ~46% of a mixed banked population on the same full-sequence check
(structurally blind to expiry transients).

**The port (`ExShadow` in engine.rs).** A sibling of `BfShadow` under the
same D-151 static exclusions (bare patterns, distinct types, non-event P, no
RHS/window touch of gated types ⇒ else no shadow ⇒ plain FIFO): per-rule
state = rtm / staged / e1_alive / pending_exp / join_count / if_staged /
if_through / exec_queued, stepped by the same external-op hooks; `pre_fire`
runs the fire-loop eval, fences the agenda-drained prefix (`q_floor` — a
quiescence unsatisfy cancels only quiescence-born emissions), replays the
registered expirations, and ranks the emission; the gated-exists pick takes
rank-min (FIFO tiebreak) with NO in_cycle guard — the shadow covers in-cycle
stream inserts natively (regime 2 was exactly that).
RETIRED: `not_order_key` (the D-140 epoch key — its last reader),
`RuleNet.last_fire_no` + the fire-boundary commit + `fired_this_cycle`,
`RuleNet.satisfy_seg` + the push_activation stamp, `FactTouch.{epoch,is_upd,
upd_seq,ins_seg}` + `Engine.{upd_seq_next,event_seg}` — `FactTouch` shrinks
to `insert_epoch` (the gated-NOT in_cycle guard's sole field). The exists
keys' entire stamp economy is gone.

**Gates (all green).** Corpus `make diff` 11/**999**/291 byte-identical —
every D-144/D-147 pin (`pr_cep_exists_order_*`, `pr_cep_v4_exists_two_held_
gens`) reproduced by the shadow; +5 pins `pr_cep_exists_flush_{first_defer,
link_gate,transient}` + `pr_cep_exp_boundary_{leak,past}`. Lint **1389**
live / 0 / 0; all cargo suites; bindings pytest 72. ENGINE-vs-oracle sweep:
**5,605 scenarios, 0 fail** (the whole spec surface). A/B against a HEAD
worktree (oracle grafted): HEAD fails **1,012/5,605** — all in the full-axis
populations (banked seeds stay green, confirming the D-144/147 gates were
honest); HEAD+boundary-fix-only fails 738 ⇒ the ExShadow order machinery
fixes ~738, the boundary correction ~274; the port fixes **all 1,012 with
zero regressions**. fuzz_cep 313/401/407 ×400 = 0 divergences; fresh seed
613 ×400 flushed ONE — `cf613x306`, a TEMPORAL-JOIN pair-order case with no
exists rule that **fails on plain HEAD byte-identically** (bisected; filed
`xf_cep_tjorder_613_pair`, the documented item-1b temporal-join latent
family). gen.rs emits no events ⇒ the main axis never builds a shadow.

**Standing scope.** The gated existential families now BOTH run mechanical
shadows; the D-140 in_cycle guard survives only as the not-side RHS fence
(its 23 delete-residuals unchanged). Remaining item-1b tails: plain-not
cf313x4, the A2 windowed-accumulate family, the temporal-join pair-order
latents (cf613x306 kin). Next natural mechanical candidates: none in the
existential family — the retirement arc Bryan opened at D-150 is COMPLETE.

### D-153: the in_cycle guard RETIRED — the not-side spec extended to the same-cycle and staged-IF regimes (full-axis soup, 2,891 scenarios 0-div), the BfShadow gains the exists-arc IF-staging correction, and the unguarded shadow fixes 401 soup divergences with zero regressions; the D-140 stamp economy is now entirely gone

**The slab.** D-151 kept the D-140 in_cycle guard ("a fired P inserted THIS
cycle ⇒ FIFO") as the not-side fence for the same-cycle insert regime the
spec hadn't covered, eating 23 documented delete-family residuals. The
exists arc (D-152) then proved the shadow machinery covers in-cycle streams
natively. This slab extends the not-side spec into that territory and
removes the guard.

**Spec extension (`fuzz_notorder_b.py SEINE_NOTPOP_FULL=1`).** The committed
replacement for D-151's lost scratch delete generator: free op soup
mirroring the exists arc's `SEINE_EXPOP_FULL` — explicit E0 deletes at any
position (delete-unblock + SAME-EPOCH P inserts, the guard's regime),
P deletes, delayed first blocker, multi-blocker staggered ts, due-on-arrival
and DROOLS-455-leaked blockers (the D-152 boundary, now generated on the not
side too), blocker updates (inert), pure-P epochs, action-interleaved
inserts. The model (`MODEL=flush`) needed TWO corrections, both settled by
graft/oracle after hand-derivation misfired again:
1. **The IF left is STAGED at the not until the first QUEUED fire-loop
   eval** — the D-152 exists discovery ported back verbatim. The D-150/151
   populations always had an initial P, making "IF at fire 1"
   indistinguishable from "IF at first queued eval"; the soup's one-sided
   windows split them (nb884x248: a blocker-only initial window leaves the
   IF staged through ep1, so the delete-unblock emission happens at the
   FIRE-LOOP eval and covers post-delete inserts — oracle [3,4,5,1,2] where
   the fire-1 model said [1,2,3,4,5]).
2. **`unlinkNotNodeOnRightInsert` fires only WHILE LINKED — exactly as
   D-150 recorded** (a pure-emptiness re-model of the bit broke 79
   previously-clean scenarios; nb880x7 re-confirmed: a blocker arriving
   before any P leaves the bit SET, so the first P links the segment,
   queues fire 1, and the IF is RESIDENT with per-window drains). The
   asymmetry vs the exists node (whose bit is pure right-population) is
   now graft-pinned on both sides.
Validated **0-div on 2,891 scenarios**: 975 full-axis soup (seeds 880-886)
+ 1,916 regenerated legacy populations (P-first 801-803, mixed 811-812,
pure-bf 861-862/871) — the corrections are invisible on every legacy
regime, exactly as the counterfactual analysis predicted.

**The port.** `BfShadow.eval_window` gains the `fire_loop` flag (staged-IF
processing after the drain/unblock steps); `pre_fire` gates the fire-loop
eval on `exec_queued` (no more unconditional first-fire branch) and adds
the post-expiry eval for a relink that queues with the IF still staged.
The gated-not pick drops the in_cycle guard — pure `emit_rank` (no-shadow
shapes stay FIFO). RETIRED with the guard: `FactTouch` + `fact_touch` +
both stamp sites (its last reader) — nothing of the D-140 stamp economy
remains in the engine.

**Gates (all green).** Corpus `make diff` 11/**1002**/291 byte-identical —
the guard's own pins (`pr_cep_c_del_not`/`_u3`/`_v3`/`_v5`) reproduce
MECHANICALLY, confirming the guard was a shadow of machinery the shadow now
implements; +3 pins `pr_cep_not_flush_{if_staged_del,if_staged_leak,
bit_preblocker}`. Lint **1392**; 9 cargo suites. ENGINE-vs-oracle:
**3,840/3,840 notpop scenarios clean** (soup + legacy). A/B vs a D-152
worktree: D-152 fails **401** (all in the soup regimes — the guard's FIFO
plus the fire-1 IF approximation; the 23 documented delete-residuals are a
subset); D-153 fixes **all 401 with zero regressions**. fuzz_cep
313/401/407 ×400 = 0 divergences; fresh seed 719 flushed TWO — both W2
windowed-accumulate (A2-family) cases that **fail on D-152 identically**
(bisected; filed `xf_cep_winacc_719_{34,242}`). gen.rs unchanged ⇒ main
axis inert.

**Standing scope.** The gated existential order machinery is now fully
mechanical and unguarded on both sides; the 23 delete-residuals are CLOSED
(subsumed by the 401). Remaining item-1b tails: plain-not cf313x4, the A2
windowed-accumulate family (now with two fresh minimizable witnesses), and
the temporal-join pair-order latents (cf613x306 kin).

## D-154 (RECON) — A2 windowed-accumulate: the mechanism, cracked and specified (port GATE-PENDING)

**The family.** All five witnesses (`xf_cep_winacc_719_{34,242}` +
fence-lifted cf401x25/42, cf423x107 — regenerated this arc, deterministic
per seed, no sibling mechanism) are ONE machine with four pieces, each
oracle-validated:

1. **The RightTuple bit.** Per (window-node, event), Drools keeps a
   RightTuple created at the FIRST alpha-pass (insert or update) that
   SURVIVES window-eviction, window-REJECTION, and alpha-fail modifies
   (`WindowNode.assertObject` creates it before `behavior.assertFact`;
   `expireFacts` retracts only the CLONE). It decides which update path
   runs: no-RT → fresh admission; RT → mask-gated modify.
2. **Snapshot admission.** The no-RT path re-runs admission against the
   INSERT-SNAPSHOT ts (`SlidingTimeWindow.assertFact`:
   `startTimestamp + N <= now` rejects — the handle ts is never refreshed,
   D-141-consistent; live ts irrelevant — `wa_fresh_reject_snap`). A
   rejected event propagates NOTHING (no transient — the cf719x242 engine
   extra-fire) but keeps the RT (`wa_stale_ins_revive`).
3. **Mask-gated revival.** On the RT path a modify whose written set hits
   the windowed mask (BINDINGS ONLY — the same structural mask D-139
   found) re-ASSERTS a fold-absent event at LIVE field values
   (`BetaNode.modifyObject`: absent tuple + mask intersect → assert),
   BYPASSING the window queue: revived-after-eviction = ZOMBIE (never
   evicted again — `wa_zombie`; only delete/expiry reap);
   revived-before-eviction stays queued and re-evicts at ts0+N
   (`wa_toggle_reevict`). Mask-miss does NOTHING — even an alpha fail→pass
   toggle stays out (`wa_toggle_stuck`; reconciles
   `pr_cep_c_upd_evict_revive`, which pinned the mask-MISS cell — the
   D-137 class-2 "no revival" was that cell over-generalized; cf719x34 is
   the mask-HIT cell: oracle sum 559 = engine 359 + the revived live
   ts=200).
4. **Deferred entry execution.** Each external update queues its OWN
   propagation entry (own written-mask); entries execute FIFO at the fire
   drain against the LIVE bean = the epoch-FINAL state (BfDump
   PropagationList proxy: `EXEC> Update E(..tag=x)` for a tag=z write —
   the intermediate state never evaluates). No mask merging: two
   same-epoch entries can evaluate DIFFERENTLY as node state evolves
   between them (wf901x261: entry 1 fresh-admission REJECT plants the RT,
   entry 2 mask-hit REVIVES). Killed two wrong hypotheses (sequential
   per-call masks; mask-coalescing) — the m1–m15 matrix discriminates.

**Bonus boundary pin.** The expiry deadline composed with windows is
exactly D-150's `D = ts+@expires+1`: `ts+ex = -1` ⇒ D=0 ⇒ DUE-ON-ARRIVAL
(drops at that epoch's quiescence), NOT a leak; leak = `ts+ex <= -2`
(wf905x127's +41 trailing-fire splits pinned it). Window EVICTION stays
exactly `ts0+N` (no +1) and eager; windowed-source EXPIRY fold-outs stay
deferred to quiescence (trailing fire, `df_win_expire_reins`) — both
landed semantics, now population-re-verified under churn.

**The spec.** `tools/model_check_winacc.py` — a per-node three-bit
(rt/queue/fold) fold simulator + FIFO entry drain + the D-152/D-150 expiry
model; compares per-rule fired-VALUE sequences against banked oracle runs.
**0-div on 3,368 scenarios**: the 30-probe battery
(`tools/gen_winacc_probes.py` — 17 wa_* state-machine cells + 13 m*
deferred-execution cells, every prediction first-shot) + all 5 witnesses
(model reproduces them inside their full rule mixes) + 3,300+ soup
(`tools/fuzz_winacc.py`, seeds 901–909; 904–909 fully out-of-sample;
908–909 generated boundary-aware). Soup axes: boundary-aimed advances
(deadline ±1), stale/due-on-arrival/leak inserts, field-subset updates
incl. same-value writes and multi-update epochs, deletes, 1–3
windowed/plain sum/count rules, tag constraints, entry-points, @expires
above/below/at the window.

**Tooling landed.** AccDump extended with epoch-ACTION replay
(advance/update/delete, property-masked `session.update`) + WindowNode
queue dumps; `fuzz_cep.py` gains the `SEINE_WINUPD_FULL=1` env gate
(lifts the temporal-type UPDATE fence — the D-141-era fence that hid this
family; default runs unchanged).

**Engine gaps this closes (at the port).** (a) `clock_removed` blocks ALL
revival — must become mask-gated "detached" (windowed nodes only; PLAIN
expiry-eager keeps the D-137 guard — `pr_cep_c_upd_after_exp` intact);
(b) the (false,true) update-in path admits with NO window check (the 242
transient) — must run snapshot admission + detached-marking; (c) the
insert path folds stale events in — same check; (d) update evaluation is
immediate per-call — windowed-acc nodes must defer to a pre-fire FIFO
drain (single-update epochs provably equivalent ⇒ corpus-safe; the
multi-update cells are today unreachable in fuzz_cep and reachable via
API). Port plan §"Port plan" of the report; everything gates on
`pat.acc.window_time.is_some()` — plain paths byte-identical by
construction, gen.rs emits no windows ⇒ main axis inert.

**Status: STOPPED AT THE GATE** (per workflow — engine changes to the
landed byte-identical corpus need Bryan's GATE). Report:
`~/.claude/plans/a2-winacc-mechanism-report.md`. Witnesses re-filed with
mechanism-level `_finding`s; they graduate at the port, when the wa/m
battery lands as `pr_cep_winacc_*` pins. Post-port gate battery is
prescribed in the report (§"Planned gates").

## D-155 — A2 windowed-accumulate PORT: the RightTuple entry machine (winacc_step)

**Gate cleared by Bryan ("port it"); the D-154 mechanism is now the engine.**
Four scoped pieces, all keyed on `pat.acc.window_time.is_some()`:

1. **`winacc_admits`** — `SlidingTimeWindow.assertFact`'s snapshot admission
   (`temporal_ts + N > clock`, rejection at <=): runs on the INSERT walk
   (stale-on-arrival events fold NOTHING and mark `clock_removed` = the
   RightTuple plants; no transient) and on the no-RT update transition.
2. **`winacc_step`** — the per-node update machine replacing the D-137/D-139
   arms for windowed nodes: in-fold mask-gated re-fold (bindings-only mask,
   unchanged semantics), alpha-fail exit folds out un-mask-gated AND
   detaches (RT persists), detached + mask-HIT re-asserts at live fields
   (REVIVAL — no queue re-entry: zombies never re-evict; revived-before-
   eviction still pops at ts0+N via the pending deadline entry), no-RT +
   alpha-pass runs fresh admission. `clock_removed` at windowed nodes now
   MEANS "detached" (RT present, fold absent); plain nodes keep the D-137
   expiry-eager no-revival guard untouched (`pr_cep_c_upd_after_exp`).
3. **`winacc_pending` + `drain_winacc_pending`** — EXTERNAL updates of a
   windowed source defer as FIFO queue entries (own written-mask each) and
   execute at fire_all PRE-FIRE against the live = epoch-final fields (the
   deferred-entry semantics BfDump proxied; m1-m15). Single-update epochs
   are provably byte-identical (staging lands before the first agenda pick;
   the pick orders by salience/decl_pos, not queue time). RHS modifies of a
   windowed source (fuzz-unreachable) step immediately. NOT drained at the
   event-insert position: an accumulate emits at rule evaluation either
   way, and mid-epoch staging tripped the per-arrival stream flush's
   segment scoping (pin `pr_cep_winacc_revive_insdrain` guards this).
4. **The stale-deadline guard** in `schedule_window_evictions` — an
   already-due deadline (ts+N <= clock) never schedules: the event can
   never be admitted (the snapshot check is time-monotone), and the stale
   past-key entry would otherwise pop at the next advance and evict a
   REVIVED member that Drools' queue never contained (wf902x184 — the
   zombie must survive; this was the one port bug the population sweep
   caught).

**Gates (all green).** Corpus `make diff` 11/**1035**/**293** byte-identical
(+33 pins: the full wa_*/m* battery as `pr_cep_winacc_*` with per-cell
findings, the insdrain/ep port guards, the updel_inmember control; the two
A2 witnesses GRADUATED to regressions/). Lint **1427**/0/0; cargo 9 suites;
bindings 72. ENGINE-vs-oracle: battery 30/30, all 5 witnesses (both
committed + the 3 fence-lifted legacy) PASS; the 9 winacc soup populations
**3,312/3,335**. A/B vs the pre-port base (worktree at `a6110a5`): base
passes 1,327/3,335 ⇒ **+1,985 fixed / 0 regressed** (every residual fails
on base identically). Fence-lifted `SEINE_WINUPD_FULL=1 fuzz_cep` seeds
401/423/719 + fresh 811, ×400 each: **0 divergences** on the ported tree
(base: the 5 witnesses); re-run post-deadline-guard: clean. gen.rs emits no
windows ⇒ main axis untouched; the not/exists shadow machinery is
rule-disjoint from windowed accs (D-151 static exclusions) and the 1,600
fence-lifted fuzz cases cover the scenario-level mixes.

**The residual (filed, NOT this slab).** 23/3,335 soup cases = ONE family:
same-epoch multi-op batching at NON-windowed nodes vs Drools' PER-ENTRY
incremental flush — (a) update-then-DELETE where the update brings the
fact IN: each oracle entry dirties the accumulate result and the terminal
fires the NET-ZERO value; the engine's batched ins+del staging compensates
silently (22 cases; `xf_cep_acc_updel_flush_{plain,win}`, minimal repros;
the windowed twin's drain skips the dead fact — same no-fire); (b) plain
out-and-back double-update composed with a windowed sibling + insert
(1 case; `xf_cep_acc_multiupd_plain`, minimized from wf906x9; the bare
shape passes). Both PRE-EXISTING (fail on base identically), unreachable
in fuzz_cep (max one op per epoch per target), and the model_check spec
handles them (the spec stays 0-div — only the engine lags). Follow-on:
generalize the deferred-entry drain to per-entry incremental evaluation.

**Standing.** The `SEINE_WINUPD_FULL` fuzz gate stays env-guarded: with A2
closed it now guards only the temporal-join pair-order latents (cf613x306
kin) — lift it by default when that slab lands. The A2 family is CLOSED.

## D-156 — tj pair-order PORT: the self-join arrival's phase membership

Gate cleared by Bryan. ONE mechanism (probe battery t1-t9 + the cf613x306
decode + `model_shared_tjo SEINE_TJO_SELF` 0-div/1,000): a self-join
arrival's LEFT insert propagates BEFORE its self-right exists; the RIGHT
insert then sees the self-left ⇒ arrival batch = [left-role: old rights
newest-first] ++ [right-role: SELF-pair first, old lefts newest-first].
Port (`phreak.rs flush_ins_delta`): the right walk appends the pending
same-fact staged left; the left walk skips the same-flush self-right. trg's
prepend already ordered the phases correctly — only the self-pair moved.
Naturally scoped to the D-136 shared divert (the unshared cascade excludes
both-side arrivals; legacy eval already correct — t1/t7). Single-side
batches byte-identical by construction. GATES: corpus 11/**1044**/**294**
byte-identical (+9 `pr_cep_tjo_self_*` pins; `xf_cep_tjorder_613_pair`
GRADUATED), lint **1437**/0/0, cargo 9, self-join population 250/250
engine-vs-oracle, fuzz_cep seed 613 ×400 **0 div** (was 1). The
`SEINE_WINUPD_FULL` fence now guards nothing known — lift candidate.
Item-1b remaining: plain-not cf313x4 only.

## D-157 — the temporal-type UPDATE fence lifted by default

Every family it guarded is closed (D-141, D-143..153, D-154/155, D-156).
`fuzz_cep.py` now always draws updates of temporal-join event types; the
`SEINE_WINUPD_FULL` env gate is gone. Verified: seeds 42/401/719/902 x400
= 0 divergences fence-lifted-by-default.

## D-158 — plain-not firing order (cf313x4): mechanism CRACKED + spec 0-div at population scale

The LAST item-1b tail: `not D() P()` where D is a PLAIN (non-event) type
inside a STREAM session. Seed-313 regen post-D-157 = exactly ONE divergence
(cf313x4 itself, no fresh siblings); order-only (engine [P2,P1] vs oracle
[P1,P2] on the expiry-unblock re-fire); D-155 per-entry-flush overlap ruled
out (no multi-op-per-handle epochs; the W3 accumulate has zero E0 facts).
Bisect skipped: the family is historically documented pre-existing (the
D-140-era fence-lifted recon).

RECON (probe-first): 21-variant pnb_* battery (2x2s over churn / P-update /
unblock-path / session-type) + 7 BfDump graft dumps (bf_{full,no_churn,
expdel,multiepoch,triple,x7} + battery reruns). Hand-derivation flip-flopped
twice (H=rtm-move-to-tail+reverse died on expdel and triple) ⇒ graft per
doctrine. KEY DUMPS: bf_full [52] (bare-P update = immediate rtm
move-to-tail at its queue position), [59-62] (release = flush staged then
reverse rtm), [38]->[40] (the churn's WM-DELETE is SYNCHRONOUS while the
RHS insertLogical is QUEUED ⇒ del reaches the not FIRST); bf_x7 [45] (an
eval with the left present at START drains the whole staged batch AS-STORED
newest-first, child-less if the not blocked mid-eval), [53] (blocked-P
update repositions rtm).

THE MECHANISM (predict_pflush, model_check_notorder_b.py MODEL=pflush —
the same rtm/staged carrier as D-150 with SIX plain-family deltas):
1. plain ops (P and D alike) STAGE until a network eval — no per-arrival
   force-flush (that is EVENT machinery);
2. the executor evaluates at a fire loop iff QUEUED: a D-DELETE staging
   queues (explicit delete, E1-delete TMS cascade, or churn); a D-INSERT
   or P-INSERT queues only while the segment is LINKED (left present);
   a pure D-ins while blocked queues NOTHING ⇒ multi-epoch staged backlogs;
3. lazy smem init: the FIRST-ever eval drains staged rights into rtm even
   while blocked (nb4001x119/x144);
4. an eval processes the not's staged D ops SEQUENTIALLY in arrival order
   with TRANSIENT releases: count 1->0 releases the left INTO the join —
   the join drains staged P's (reversed-append into rtm) and emits
   [staged-children arrival-order ++ reversed(pre-drain rtm)] (== reversed
   (post-drain rtm) — the two formulations are algebraically identical);
   a later ins in the same batch RE-BLOCKS: unfired emissions cancel, the
   DRAIN persists (bf_full's churn flush). Count 2->1 is ABSORBED — the
   join is untouched (nb4001x85/x54/x67: a second live blocker prevents
   the flush);
5. TMS churn arrival order = del-then-ins (sync WM-DELETE vs queued
   insertLogical) ⇒ a single-blocker churn transiently drains; staged-ins
   ANNIHILATION: a D delete reaching a still-unprocessed staged ins removes
   it — the not never sees either (nb4001x139/x91); a D-ins landing while
   LINKED queues the eval that blocks before later arrivals (nb4103x160);
6. quiescence expiry retracts (deadline order) are each their own
   single-del eval — absorbed ones never drain the join; explicit deletes
   stage and evaluate at the FIRE loop, after later same-epoch entries
   (nb4001x74: the epoch-fact P joins the release batch), NOT at their
   queue position (the event-family E0 rule does not carry over).
Bare-P update: immediate move-to-tail iff flushed into rtm, no-op while
staged, never queues/re-fires (unchanged from D-150 piece 4).

VALIDATION: tools/fuzz_notorder_b.py SEINE_NOTPOP_PLAIN=1 (explicit-D and
logical-D via `J: E1($t:tag) => insertLogical(new D($t))` modes; unique
tags keep nth_inserted indices deterministic — one J fire per E1 insert +
one per tag-update; liveness + prior-epoch-touch constraints in the
docstring). Model 0-div on **1,667 scenarios**: banked 4001/4002/4003
(419), first out-of-sample 4101-4103 (467; two rule fixes consumed this
round — the annihilation-vs-quiescence split and the linked-D-ins queue),
then FROZEN-model validation on 4201-4204 = **781/781 fresh out-of-sample
0-div**. Engine baseline on the 4001 population: 21/200 divergent (the
port target). Session-type control: pnb_plain_sess/pnb_plain_noupd agree
(non-stream sessions unaffected — the fix gates on STREAM).

SCOPE EXCLUSIONS (generator-enforced; the port gate must respect them):
shared TMS justifications (equal tags), due-on-arrival justifiers,
same-epoch E1 touches (coalescing corner), RHS-inserted P's. These fall
to FIFO/HEAD behavior at the port gate, not to the shadow.

PORT PLAN (gate-pending): `PnShadow` beside BfShadow — same emit_rank/
gated-pick plumbing; P hooks at external ops; D hooks at the WM level
regardless of provenance (external, RHS-logical, TMS-retract, expiry
cascade) with churn order imposed del-first per spec; compile gate =
[InitialFact, non-temporal NOT over a PLAIN type, bare positive non-event
P] + STREAM session + BfShadow-style static exclusions. ⚠ blast radius:
plain nots ARE main-axis-reachable — the STREAM-session gate is the
structural protection; main-axis gen.rs fuzz on BOTH trees is a mandatory
port gate (unlike every prior item-1b family).

## D-158 (port) — PnShadow LANDED: the plain-not order family is CLOSED; item-1b DONE

The engine port of the D-158 spec, the mechanical-shadow pattern's third
member (BfShadow D-151 / ExShadow D-152 / PnShadow): a per-gated-rule state
machine ranking the static pick by `emit_rank`. THREE structural deltas vs
its siblings:
- **WM-level D hooks** (`pn_on_wm_insert/delete` inside `on_insert`/
  `on_delete`): the blocker lifecycle is TMS-driven (logical inserts,
  justification retracts, expiry cascades), so the shadow needs NO deadline
  model of its own — the engine's real expiry/TMS machinery already
  delivers D deletes in deadline order. P hooks stay at the external-op
  sites (the gate excludes RHS-touched P types).
- **Online mid-cycle evals**: `in_fire_loop` D events evaluate immediately
  when queued (churn / quiescence-retract semantics) and EXTEND `emit_rank`
  mid-cycle — the pick reads it live per pop. External-phase events wait
  for the boundary eval (`pre_fire(has_items)`).
- **Churn canonicalization** (`pn_seq` + `pn_churn_ctx`): the engine's
  re-fire retracts stale TMS keys in execute_rhs's EPILOGUE (ins-then-del),
  but Drools' WM-DELETE is synchronous at the fire while insertLogical is
  queued (del-then-ins, bf_full [38]->[43]) — the epilogue's del hops
  before the same-RHS staged inses, restoring the spec's transient-release
  drain. Every other provenance appends in arrival order.
Gate: `pn_pos` = [InitialFact, non-temporal NOT over a PLAIN type, positive
P] AND `!event_specs.is_empty()` (STREAM only — the non-stream plain-not
order is main-axis-certified and untouched); static exclusions mirror
BfShadow's except RHS Insert/InsertLogical of the BLOCKER type is ALLOWED
(the logical-justifier mechanism; shared justifications are WM-invisible on
both sides, so the unique-tag spec constraint needs no gate).

GATES (all green): corpus **11/1056/295 byte-identical** (`make diff`; +12
`pr_cep_pn_*` pins, cf313x4 GRADUATED to regressions/), lint **1450**
live/0/0, cargo 9 suites, bindings 72. Plain populations engine-vs-oracle:
**+327 fixed / 0 regressed** (base 364 / ported 37 of 3,100; every residual
SET-level AND pre-existing — classified case-by-case against the pre-port
worktree). fuzz_cep 313/907/911 ×400 = **0 divergences** (base: 313 = 1,
the witness — causation). Event-family sweeps unperturbed: notpop FULL 600
engine-diff 0-fail + MODEL=flush 309/309; expop FULL 600 engine-diff
0-fail. Main axis: gen.rs 42/123/7 ×10k BOTH trees — identical divergence
sets (the pre-existing families; the pn path is analytically dead there:
`event_specs` empty ⇒ `pn_pos` None ⇒ hooks early-return).

RESIDUAL (pre-existing, filed `xf_cep_pn_annihilation_set`, open_divergence):
the engine stream-flushes plain not-blocker INSERTS per-arrival, Drools
stages them lazily until an eval — SET-visible when a same-window explicit
D ins+del pair should ANNIHILATE (Drools: the not never blocks, nothing
re-fires; engine: block-at-insert + release-at-delete re-fires the join
memory). 37/3,100 population scenarios, all this family. Fix class = defer
plain not-blocker staging to the fire eval — a follow-on semantic port,
NOT a pick reorder (the shadow cannot remove real activations).
fuzz_cep CANNOT reach it (its delete targets are event-typed only).

**⇒ ITEM-1B IS CLOSED.** cf313x4 was the last tail; every family (D-141
tj-ts, D-143..153 existential order, D-154/155 A2 winacc, D-156 tj
pair-order, D-158 plain-not) is landed. Remaining CEP latents are the
fenced-by-nature pair (D-134 §6 PriorityQueue ties, fz_42_84
identity-hash) + the two filed per-entry-flush/annihilation SET residuals.

## D-159 — plain-blocker lazy staging: the annihilation residual is CLOSED

The D-158 residual (filed `xf_cep_pn_annihilation_set`): the engine
stream-flushed PLAIN not-blocker right-INSERTS per-arrival while Drools
stages plain ops lazily until a network eval — SET-visible exactly when a
same-staging-window explicit D ins+del pair should ANNIHILATE (TupleSets
addDelete cancels the still-staged insert: the not never blocks, nothing
re-fires; the engine blocked at the ins flush and the deferred del's
release re-fired the whole join memory). The mechanism was SETTLED by the
D-158 graft arc (`predict_pflush` already predicted every cell); this slab
was probe → port → gates only, per the handoff
(`~/.claude/plans/annihilation-handoff.md`).

FIX (engine.rs `stream_flush_ex` stash loop, the non-Join branch): a
`Kind::Not` node whose blocker pattern type is PLAIN
(`!event_specs.contains_key(patterns[env.1].type_id)`) stashes ALL staged
right-ins for the flush walk — the certified restore loop puts them back
and keeps the rule queued+dirty; the staged ins then meets the deferred
del at the fire eval where `Staged::add_del` annihilates natively. EVENT
blockers keep the certified D-102 visibility (an event insert force-flushes
its own eval in Drools — "E1 blocks E2 at the fire-1 flush"). Kind::Not
ONLY (plain-EXISTS churn is pinned COALESCING,
`pr_cep_c_exists_churn_plain`); temporal nots are structurally excluded
(temporal constraints require event types). `touched_node` stays computed
pre-stash (handoff option A): the D-arrival flush still queues a
mostly-empty eval — corpus + populations show ZERO order drift, so no
recompute was needed. ~19 gated lines.

PROBE BATTERY (oracle-first, predictions written before running —
`$CLAUDE_JOB_DIR/tmp/annih/`, predictions.md): 11 probes, 11/11 correct.
Discriminators an1_insdel (engine 12 vs oracle 6) and an9_insdel_mixed
(14 vs 8) failed pre-port exactly as predicted and pass post-port; 9
controls (same-window del→ins transient-cancel ×2, cross-epoch pair split
×2, second-blocker absorbed ×2, mid-epoch ins with queued P's, TMS
same-epoch cell, plain-exists scope) pass on BOTH sides throughout. The
generator-excluded TMS cell (an7: E1 inserted+deleted in one epoch — J's
match cancels at the delete, no logical D ever exists) agrees with
predict_pflush: NO spec gap. SCOPE record (an8): a same-window plain-D
ins+del pair at a plain EXISTS fires nothing on either side (consistent
with lazy annihilation there too, but the shape cannot distinguish it from
eager-satisfy-then-retract-before-fire; engine==oracle ⇒ no action —
widening stays a separate probe-first question, out of this slab).

GATES (ALL green; A/B base = a `3b2aa52` worktree with the oracle symlink):
- corpus `make diff` **11/1060/296 byte-identical** (+4 pins
  `pr_cep_pn_annih_{insdel_mixed,crossepoch,delins,absorbed}`; the witness
  GRADUATED to regressions/), lint **1455 live/0/0**, cargo 9 suites,
  bindings pytest **72** (.so restored).
- plain populations (regenerated, 13 seeds — baseline 10 regeneration-
  identical to D-158's: 1,667 orderable): FIXED tree **4,000/4,000**
  engine-vs-oracle; BASE tree fails **exactly the handoff's 37**
  (2/7/3/6/4/2/3/2/5/3 by seed) + **9** on the fresh out-of-sample seeds
  4401-4403 (2/5/2) ⇒ **+46 fixed / 0 regressed**. pflush model: **ALL
  MATCH on all 13 populations** (1,667 + 503 fresh — the oracle-side spec
  is untouched by the engine fix, incl. nb4001x91/x139, the spec's own
  annihilation citations).
- event-family sweeps unperturbed: notpop FULL 600 seed 888 diff 0-fail +
  MODEL=flush 309/309; expop FULL 600 seed 889 diff 0-fail.
- fuzz_cep ×400 BOTH trees: seeds 313/907/911 + fresh 921/923 = **0
  divergences everywhere** (blast radius only — fuzz_cep's mutation
  targets are event-typed and cannot reach the family).
- main-axis gen.rs 42/123/7 ×10k BOTH trees: **identical flagged name
  sets** (42→{fz_42_258,6358,7682}, 123→{fz_123_763,1589},
  7→{fz_7_776,1936,2990,3185} — the known pre-existing latents; artifacts
  cleaned post-comparison).
- early-warning pins re-diffed explicitly BY NAME: **36/36** (the
  q1/q2/q4 D-106-caveat region untouched, a6/a7*-cascades, ep7*,
  c_upd_tms, 53/721 arrivals, c_exists_churn_plain, all 12 pr_cep_pn_*,
  the w1-w6 ladder).

The PnShadow is untouched (it models Drools, not engine internals). The
D-155 per-entry-flush accumulate residual is a DIFFERENT mechanism and
stays filed. With this, **the plain-not family has NO open divergence**.

## D-160 — per-entry incremental acc drain: the D-155 flush residual is CLOSED

The last filed accumulate divergence (D-155's 23-case residual,
`xf_cep_acc_updel_flush_{plain,win}` + `xf_cep_acc_multiupd_plain`):
same-epoch multi-op sequences over accumulate sources evaluated as ONE
batched staging window while Drools executes each queued entry
INCREMENTALLY. Slab directed by Bryan ("D-155 it is"); no handoff file —
recon from the witnesses, the D-154/155 entries, and the validated spec.

MECHANISM (probe-settled — 7-probe ap battery + the 3 witnesses,
predictions written first, `$CLAUDE_JOB_DIR/tmp/accdrain/`): external
updates/deletes of EVENT-TYPED facts feeding accumulate nodes are queue
entries executing per-entry FIFO at the fire drain against the
EPOCH-FINAL bean, with aliveness decided by ENTRY ORDER — an update
entry followed by a Del entry executes "alive"; each entry that touches
fold membership dirties the result; the terminal fires the final (even
net-zero) value once per boundary. The battery's gating cells came back
DECISIVE: a PLAIN-typed source stays SILENT on the oracle in BOTH
session modes (ap1/ap1b — plain ops batch in one staging window and the
ins+del pair annihilates, coherent with the D-158 plain-staging
laziness; gen.rs REACHES that shape on the main axis, so the gate
protects certified behavior), and an evented same-epoch INSERT+delete
fires net-zero on BOTH sides already (ap2 — the event insert's
per-arrival force-flush materializes the fold-in before the delete, so
no annihilation ever forms; inserts never queue as entries). The
D-154/155 winacc entry queue was this mechanism's windowed special case;
its drain's is_alive skip was the win-twin's bug (an update entry for a
fact deleted LATER in the same epoch was dropped instead of executed).
The spec (`model_check_winacc.simulate`) already modeled every cell —
its domain is evented sources; the ap1/ap1b cells mark its boundary.

PORT (engine.rs, ~180 lines): `AccEntry { Upd(mask), Del }`;
`winacc_pending` → `acc_pending`. on_update: plain-acc nodes over
evented sources defer external updates to the queue (the windowed arm
generalized); RHS modifies and plain-typed sources keep the immediate
D-137/D-139 arms byte-identically. delete_fact: an explicit external
delete of an evented acc-feeding type queues `Del` and
`on_delete_ex(defer_acc)` skips exactly those acc nodes (expiry keeps
`in_expiration_drain`; TMS cascades bypass delete_fact — both stay
immediate). `drain_acc_pending` (fire_all pre-fire, the D-155 site):
FIFO; Upd → `winacc_step` / new `plainacc_step` (the immediate arms
extracted verbatim, D-094 two-pass) evaluating FINAL fields via
`alpha_passes_fields` (the liveness gate split out of `alpha_passes`;
retracted facts' fields stay readable, matching the live Java bean);
an Upd for a fact dead WITHOUT a later Del entry drops (the certified
D-155 expiry compensation). Del → active.remove + add_del at its queue
position; when that add_del ANNIHILATES a drain-staged ins, the entries
still each dirtied Drools' result — force the net-value re-emission by
staging `s_left.add_upd` for every left (Phase D re-derives, Phase G
updates the child unconditionally) + requeue sharing rules.

GATES (ALL green; A/B base = a `7d53106` worktree, oracle symlinked):
- witnesses + battery: 3/3 witnesses PASS (plain 1→2 firings, win 1→2,
  multiupd count 1→2) + ap9 k=2 re-emission through real lefts; all
  scope controls hold. Witnesses GRADUATED; +5 pins
  `pr_cep_acc_drain_{updel_k2,updel_plainsrc,updel_plainsrc_stream,
  insdel_event,updupd_final}`.
- corpus `make diff` **11/1065/299 byte-identical** (incl. all 33
  `pr_cep_winacc_*`, the D-137/139 arm pins, the w-ladder and shadow
  pins), lint **1463 live/0/0**, cargo 9 suites, bindings pytest **72**.
- winacc soup (regenerated, 11 seeds: 901-909 + fresh out-of-sample
  911/913): FIXED tree **4,275/4,275** engine-vs-oracle (incl. 150
  n=400 leftovers from the D-155 session sharing this job's tmp); BASE
  tree fails **30** on the same files (24 across 901-909 + 6 on the
  fresh seeds; spot-checked seed 906: all under-fires = the missing
  incremental re-fires) ⇒ **+30 fixed / 0 regressed**. Spec:
  **4,125/4,125 ok, 0 div** across all 11 regenerated banks.
- fuzz_cep ×400 BOTH trees: 401/423/719/811 (winacc surface, winupd
  axis default-on since D-157) + 313 + fresh 927/929 = **0 divergences
  everywhere** (fuzz_cep caps 1 op/epoch/target ⇒ blast radius only).
- event-family sweeps (fresh seeds): notpop FULL 600/600 (seed 890),
  expop FULL 600/600 (seed 891) — shadow surfaces unperturbed.
- main-axis gen.rs 42/123/7 ×10k BOTH trees: **identical flagged name
  sets** (the known 3/2/4 latents; the evented-only gate is analytically
  dead there — plain-source shapes pinned by pr_cep_acc_drain_updel_
  plainsrc; artifacts cleaned).

SCOPE NOTES. Plain-typed-source lazy staging at acc nodes (the ap1/ap1b
silent cells) is CERTIFIED CURRENT BEHAVIOR, now pinned — any future
evidence to the contrary starts a new probe-first arc, not a gate widen.
RHS modifies of acc sources stay immediate (fuzz-unreachable for
windowed; certified for plain). The accumulate family now has NO open
divergence; every 2026-07 mechanical-shadow-era slab (D-150..D-160) is
closed with the corpus byte-identical throughout.

## D-161 — plain-EXISTS lazy staging: the wedge CLOSED, the order family FILED

Bryan-directed slab ("plain-EXISTS lazy staging next") — the D-159 scope
note's follow-on. NOTHING was filed going in (one coalescing pin + one
0-0 scope probe); the recon opened the largest un-filed family since the
shadow arc: a NEW population axis (`SEINE_EXPOP_PLAIN=1
tools/fuzz_existsorder.py` — gen_plain: bare / constrained (update-churn
axis) / TMS-logical witness drives, gen_plain-style determinism
bookkeeping) measures **205/900 divergences** at seeds 5001-5003.

RECON (12-probe exlazy battery, predictions written first,
`$CLAUDE_JOB_DIR/tmp/exlazy/`): plain-exists witness ops are
NET/FINAL-STATE-wise in EVERY provenance — explicit del+ins churn,
update out-and-back with fired P's (3 firings, no re-fire), TMS
logical-witness churn (satisfied exactly once through a justifier
tag-update), non-stream sessions. The sequential-transient model is
plain-NOT-only; the D-159 handoff's "ins-first/net" exists hypothesis is
CONFIRMED. Two engine defects found:

1. **THE WEDGE (SET, permanent silence — FIXED this slab):** an
   out-and-back alpha update of a plain witness with a P-insert FLUSH
   between the churn and the fire. The per-arrival flush SPLIT the ph=1
   del+ins re-entry pair — the dstash hid the pre-tail del while the
   visible ins evaluated alone — and the exists child died PERMANENTLY
   (ex2: engine 1 vs oracle 3; ex2b: the NEXT epoch's P also never
   fires, engine 1 vs oracle 4). Coherence checks: ex1 (no flush after
   the churn) and ex10 (non-stream, no flushes at all) both pass — the
   mid-epoch flush IS the axis, not session mode.
   FIX: widen the D-159 plain stash gate from `Kind::Not` to
   `Kind::Not | Kind::Exists` (one line + comments) — the plain witness
   ins stays staged through mid-epoch flushes, the pair reaches the
   fire eval intact, and the certified D-081 net machinery handles it.
2. **The satisfy EMISSION-ORDER family (FILED, → D-162):** satisfy
   batches drain newest-first-ish on the engine vs arrival-FIFO on the
   oracle — including INITIAL-fire batches whenever P's precede the
   witness in the facts. ~200/900 population scenarios (~20/seed with
   re-satisfy SET-compounds on top: x75 under-fires 6 vs 8). Witnesses:
   `xf_cep_ex_backlog_order` (the clean 5-P backlog cell),
   `xf_cep_ex_plain_order_set` (order + re-satisfy compound),
   `xf_cep_ex_order_widen_exposed` (passed pre-widen by MECHANISM LUCK —
   the eager flush-satisfy happened to drain arrival-order; its first
   witness now correctly annihilates in staging and the satisfy routes
   through the fire-eval drain, exposing the order bug: ORDER-only).
   Handoff: `~/.claude/plans/plain-exists-order-handoff.md` — a
   D-158-style spec arc (predict_pexists; starting hypothesis =
   arrival-FIFO emission from the all-staged unlinked backlog), 0-div
   at population scale, then Bryan's gate for the port.

GATES (all green; A/B base = a `6cca187` pre-widen worktree):
- populations (the SAME files both trees): base **205** fail → widened
  **200** = **+6 fixed / 1 exposure** (ex5001x5, classified above —
  moved INTO the already-filed order family, not a new mechanism).
- corpus `make diff` **11/1070/299 byte-identical** (+5
  `pr_cep_ex_lazy_{churn_fired,wedge,wedge_next_epoch,tms_churn,
  churn_nonstream}` pins; `pr_cep_c_exists_churn_plain` and the
  w1-w6 event ladder HELD under the widen), lint **1468 live/0/0**,
  cargo 9 suites, bindings pytest **72**.
- event surfaces: expop FULL 600/600 (seed 893), notpop FULL 600/600
  (seed 894), notpop PLAIN 400/400 (seed 4405 — the D-159 surface holds
  under the widen).
- fuzz_cep ×400 BOTH trees: 313/907/911 = 0-div; fresh seed 933 flagged
  `cf933x385` on BOTH trees IDENTICALLY — a PRE-EXISTING order latent in
  `not E1() P()` (EVENT blocker, BfShadow domain) inside a 5-type
  salience mix with temporal joins + windowed accs; quarantined to
  xfail/ with a finding (BfShadow-composition recon when scheduled; NOT
  this family).
- main-axis 42/123/7 ×10k BOTH trees: identical flagged sets (the known
  3/2/4; artifacts cleaned).

SCOPE. Event-typed exists witnesses keep D-102 flush visibility (the
w-ladder). The order family is deliberately NOT chased here — it needs
its own validated emission model (the exposure case makes the ordering
explicit: land the SET fix, spec the ORDER). Next slab = D-162 per the
handoff.

## D-162 — plain-EXISTS satisfy EMISSION-ORDER: mechanism CRACKED, spec 0-div on 1,800 (port gate-pending)

DATE: 2026-07-10 (late). The D-161 handoff (`~/.claude/plans/
plain-exists-order-handoff.md`) executed as a D-158-style spec arc.
Report for the gate: `~/.claude/plans/plain-exists-order-report.md`.

THE MECHANISM (banked-population-derived; discriminators ex5001x{75,79,
88,103,106,125,129,130,170,248}, ex5003x280 — full table in the report):
`predict_pexists` (tools/model_check_exists.py, EMODEL=pexists) = the
D-158 pflush join skeleton (P-ins stage arrival-order; bare-P update =
immediate rtm move-to-tail iff in rtm; P-delete annihilates staged /
leaves rtm + cancels its activation; every eval drains staged rights
reversed-append into rtm, emitting them arrival-order iff THROUGH;
satisfy emission = join_left_ins = staged-arrival ++ reversed(pre-rtm))
× the EXISTS polarity × the D-161 NET witness semantics (staged D ops
apply as ONE net batch per eval; only the net 0→1/1→0 transition fires/
kills) × a link-counter queue economy:
- deletes SYNC / updates DEFERRED (the D-155 principle recurs): explicit
  D-deletes and TMS cascade retracts move the exists link counter at
  entry (annihilating still-staged inses outright); an alpha-EXIT update
  of a processed D stages its del with NO counter move (x125), while an
  alpha-exit of a still-staged ins is a staging-level annihilation
  (x88); alpha-admit stages a fresh ins;
- queue signals in TWO classes: LINK (D-ins counter 0→1 with the join
  populated = satisfy-link; P-ins while counter>0) — DEQUEUED by any
  sync counter 1→0 (segment delink; x88/x79); WM (explicit D-deletes
  only, even annihilating ones — x280/x248) — never dequeued; TMS
  cascade dels carry NO signal (x170 coalesces forever);
- evals run iff queued (fire-loop + mid-drain: the executor re-evaluates
  before its next item when witness ops are staged — the logical drive's
  first satisfaction lands there); the IF left stays STAGED till the
  first fire-loop eval;
- QUIESCENCE eval: staged witness ops unprocessed at the window's end
  evaluate even unqueued — a CROSS-boundary unsatisfy is observed there
  (x276/x56) while a same-window del+ins pair has already coalesced
  mid-drain (x170/x79).
- NO REFRACTION: a re-satisfy re-emits the WHOLE right memory — the x75
  SET-compound (engine 6 vs oracle 8) is this machine, not a separate
  sub-mechanism. FIFO backlogs (xf_cep_ex_backlog_order) = the machine
  when nothing ever queued an eval; newest-drain-batch-first shapes
  (x248/x280) = the same machine when WM signals ran blocked evals.

VALIDATION (all green): banked-format checks 656/656 (5001-5003) +
660/660 FRESH out-of-sample (5004-5006; ZERO refit after seed 5001 —
5002-5006 matched as-derived); FULL-COVERAGE oracle sweeps (no
firing-count filter, incl. the 244+240 sub-2-firing shapes) **1,800/
1,800 ALL MATCH**; EMODEL=flush (D-152 event spec, same file)
regression 60/60; `make lint-probes` 1468/0/0. Populations regenerate
deterministically: `EXPOP_TMP=<tmp>/explain SEINE_EXPOP_PLAIN=1
tools/fuzz_existsorder.py 300 <seed>`.

ENGINE DEFECT (unchanged at `d400c56`, 200/900 banked fails): (1) ORDER
— the fire-eval satisfy drain emits newest-first-ish instead of the
machine order; (2) SET — cross-window re-satisfies under-fire (x75
6-vs-8): the engine refracts where the oracle re-creates every child. A
pick-reorder shadow alone CANNOT fix class 2; the eval path must re-emit
the full memory (port approaches + gates in the report).

STATUS: **SPEC ONLY — engine untouched; Bryan's gate pends for the
port.** Corpus untouched this slab (diff green at session start, no
engine/probe changes; lint re-run green). The 3 xfail witnesses
(`xf_cep_ex_{backlog_order,plain_order_set,order_widen_exposed}`) stay
filed until the port lands. Coverage caveats for the port session: the
generator never reaches zero live P's (satisfy-link's join-populated
corner untested — hand-probe at port time); ex5001x5-class mechanism-
luck shapes need the A/B, not just the model.

## D-162 (PORT) — plain-EXISTS satisfy order LANDED: the quiescence eval + PxShadow; the family is CLOSED

DATE: 2026-07-11. Bryan cleared the gate on the D-162 spec report
(`~/.claude/plans/plain-exists-order-report.md`); this is the engine port,
D-158 discipline (shadow + scoped eval fix, full battery, A/B at the
D-161 commit `d400c56`).

THE TWO PIECES (both gated to the family: PLAIN-witnessed non-temporal
`Kind::Exists` in a STREAM session):

1. **The QUIESCENCE step (the SET fix)** — `next_activation`'s
   agenda-quiescence chain (the flushExpirations slot, after
   `drain_pending_expirations`/`exp_deferred`) re-queues (queued+dirty)
   any rule whose gated exists node holds staged witness ops at the
   window's end; the rescan pop then evaluates them. ROOT (x75 trace):
   a staged plain-witness DEL survived its own boundary — the unlink
   transition queued+dirtied the rule, but a later same-epoch flush hid
   the del in the dstash and its eval cleared the dirty flag
   (evaluateNetwork -> setDirty(false)), so the boundary pop held a
   queued-but-clean item (the faithful D-084/D-091 hold) and the del
   BATCHED with the NEXT window's witness ins into one eval = witness
   HANDOVER (the exists child never died; no re-fire; the same-epoch
   staged-P activations never cancelled: the 6-vs-8 under-fire). The
   engine's wiped-flag hold is otherwise LOAD-BEARING (it is what
   coalesces a same-window del+ins arriving around mid-drain churns —
   x170/x79 — matching Drools); only the WINDOW-END leak is wrong, and
   the quiescence step is exactly the spec's quiescence eval. A pick
   shadow could never fix this class (missing/extra activations).
2. **PxShadow (the ORDER fix)** — the fourth mechanical shadow: the
   validated `predict_pexists` machine in Rust (engine.rs), stepped at
   the WM hooks (P side external ins/upd/del; witness side WM-level
   ins/del for ALL provenances + external update), with the NET witness
   batch, the link-counter queue economy (sync deletes / deferred
   updates; a sync 1->0 dequeues LINK signals; explicit witness deletes
   carry a never-dequeued WM signal; TMS cascades silent), pre-fire
   eval iff queued, mid-drain evals (in-fire witness events with the
   executor queued or NE items pending), its own quiescence eval, and
   `join_left_ins` emissions (staged-arrival ++ reversed(rtm), NO
   refraction). The gated static pick ranks by `emit_rank` (unranked ⇒
   FIFO). Gate `px_pos` = [InitialFact, non-temporal EXISTS over a
   PLAIN type, positive P] + STREAM; build exclusions = the PnShadow
   set EXCEPT the witness may carry ALPHA-only constraints (the cons
   drive — the shadow re-evaluates `alpha_passes_fields` per witness op
   and tracks per-fact alpha state for update classification) and RHS
   Insert/InsertLogical of the WITNESS type is allowed (the logical
   J-drive). P must be bare. Explicit-delete provenance = a
   `px_explicit_victim` stamp in `delete_fact` (cascaded TMS retracts
   inside the same delete are NOT explicit); churn canonicalization
   reuses the pn epilogue hop (`pn_churn_ctx`/`pn_seq`).

PIECES: engine.rs — PxShadow struct + `CompiledRule.px_pos` +
`RuleNet.px` + build gate + WM/external hooks + `px_explicit_victim` +
pre/post_fire + the quiescence step + the gated pick branch. Scenarios:
3 xfail witnesses GRADUATED to regressions/
(`xf_cep_ex_{backlog_order,plain_order_set,order_widen_exposed}`) + 5
mechanism pins `pr_cep_px_{refire_explicit_del,coalesce_logical,
alpha_annih_fifo,wm_drain_order,quiesce_cascade}` (population
discriminators x129/x170/x88/x280/x130).

GATES (all green; A/B base = the `d400c56` worktree, oracle target
symlinked — a fresh worktree lacks the gitignored oracle build):
- populations (the SAME files both trees): FIXED **2,100/2,100** (seeds
  5001-5007 — 5007 generated fresh POST-port) vs BASE fails **479/2,100** — 68+67+65 banked (EXACTLY the D-161 fixedfails counts) + 74+60+72+73 fresh ⇒ **+479 fixed / 0 regressed**;
  spec `EMODEL=pexists` ALL-MATCH on every banked capture incl. fresh
  5007 (210).
- corpus `make diff` **11/1075/302 byte-identical** (+5 pins, +3
  graduations), lint **1476 live/0/0**, cargo 9 suites, bindings
  pytest **72**.
- sweeps (fresh seeds): expop FULL 600/600 (896), notpop FULL 600/600
  (897), notpop PLAIN 400/400 (4406) — the D-152/D-158/D-159 surfaces
  hold under the port.
- fuzz_cep ×400 BOTH trees: 313/907/911 = 0-div; 933 = exactly
  `cf933x385` on both (the D-161-quarantined pre-existing event-not mix
  latent, untouched).
- main-axis 42/123/7 ×10k BOTH trees: identical flagged sets — 3/2/4, the known 9 pre-existing latents (fz_42_{258,6358,7682}, fz_123_{763,1589}, fz_7_{776,1936,2990,3185}); artifacts cleaned.

SCOPE. Event-typed exists keeps the D-152 ExShadow machinery; plain-not
keeps D-158/D-159 (notpop-plain 400/400 under the port); non-stream
plain-exists is main-axis-certified and untouched (`px_pos` None, the
quiescence step gates on `!event_specs.is_empty()`). The quiescence
step is FAMILY-WIDE (not shadow-gated): staged witness ops at gated
nodes always evaluate by their window's end — fuzz_cep composites and
the corpus verify the non-shadowed shapes. Coverage caveats from the
spec stand: zero-live-P satisfy-link corner untested by the generator.

## D-163 — the ORACLE PATCHED to 9.44.0.Final+p1 (upstream #6796 vendored): the D-093 quarantine GRADUATED, the gen.rs wall LIFTED

DATE: 2026-07-11, Bryan-directed ("just add the `|` character in the
oracle") — the D-148 convergence protocol executed via a LOCAL oracle
patch instead of a version bump, eliminating the 9→10 re-certification
risk entirely: everything except the two-line fix stays bit-for-bit
9.44.0.Final.

MECHANISM: `oracle/src/main/java/org/drools/core/phreak/
PhreakAccumulateNode.java` — the 9.44.0.Final source with EXACTLY the
upstream-merged repair (apache/incubator-kie-drools#6796, commit
`275baf9c`, Mario Fusco, 2026-07-08 — Seine's suggested `isDirty |=`
repair verbatim, both arms of `doLeftUpdatesProcessChildren`; hunk
fetched from the merge commit and applied verbatim, TWO sites). The
class SHADOWS the drools-core jar copy via classpath order
(`oracle/target/classes` precedes the jars, harness/src/oracle.rs) —
same mechanism as the graft dumps; no jar surgery, CI builds it with
`mvn package` unchanged. Provenance: loud file header + NOTICE entry;
the vendored file never touches the Rust engine (brief §8 intact).

SCOPE RULE (the slippery-slope fence): only upstream-MERGED fixes for
defects Seine itself reported and quarantined under the D-093 doctrine
(faithfulness axis 2 — value-bearing self-inconsistent defects) may be
vendored as oracle patches. The oracle is labeled **9.44.0.Final+p1**
(README provenance + docs/drools-bug-stale-minmax.md banner).

WALL LIFT (`harness/src/gen.rs`): min/max accumulates re-enter the
mutation pool (the `allow_mutation` func-pool split removed) and
external updates stop rerouting to deletes (`has_minmax` deleted).
NOTE: the lift changes the generator's draw stream — the old main-axis
flag baselines (3/2/4, 9 names) do NOT carry name-for-name; this entry
re-baselines them below.

CAUSATION A/B (same lifted generator, two oracle builds):
- UNPATCHED oracle, seeds 42/123/7 ×10k: 2/2/4 divergences over 30k cases = 8 of the old 9 latent names EXACTLY (fz_42_{258,6358}, fz_123_{763,1589}, fz_7_{776,1936,2990,3185}; fz_42_7682's case stream shifted away under the lifted generator); ZERO defect-family hits — the 7 witnesses carry the causation (0/7 pass vs stock)
- PATCHED oracle, same seeds/programs: flag sets BYTE-IDENTICAL to the
  unpatched legs (2/2/4, the same 8 names) — the patch is INERT outside
  the defect family; the family itself converges at the witnesses.

CONVERGENCE + GATES: the 7 quarantined witnesses (alu6a, alu7a/7d/7f/
7g, fz_123_8426, fz_min_8426) flipped 0/7 (vs stock, same session) to **7/7 PASS** — GRADUATED to
regressions/. Corpus `make diff` **11/1075/309** byte-identical under the patched oracle
(the D-093 wall had kept certified scenarios off the changed surface,
so byte-identity was expected and held). lint **1483/0/0**, cargo 9,
bindings 72. fuzz_cep 313 ×400 0-div (CEP surface unaffected).
Main-axis RE-BASELINE (patched oracle + lifted generator, 42/123/7
×10k + fresh seed 777): 42/123/7 = 2/2/4 (the re-baselined sets above, cleaned); fresh seed 777 ×10k = 4 flags (fz_777_{1086,2897,6781,7035}) — ALL 4 fail against the STOCK oracle identically and NONE contains min/max ⇒ pre-existing fresh-seed latents of the known order classes (deferred to the latent ledger), neither patch- nor wall-lift-caused.

docs/drools-bug-stale-minmax.md carries the +p1 resolution banner;
FEATURES.md accumulate row updated; README provenance notes the
asterisk. The D-148 "next oracle bump" contingency is RETIRED — the
graduation is done; a future real version bump remains its own
re-certification arc if ever wanted for other reasons.

## D-164 — Allen `@expires` INFERENCE: the D-120 fence LIFTED (reach ladder 124/124, constant-interval edges ported)

DATE: 2026-07-11, Bryan-directed ("let's do the Allen inference arc").
The reach VALUES D-119 left unpinned are now oracle-pinned and ported;
the last fenced FEATURE gap inside implemented-CEP is closed.

MECHANISM (sources-hypothesized, oracle-verified cell by cell): every
Allen op carries a PARAM-BLIND CONSTANT interval in Drools' mvel
EvaluatorDefinitions' `getInterval()` — params (dev/min/max) are used
for MATCHING only and never reach inference:
  coincides/starts/startedby [0,0] · meets/overlappedby/finishes
  [0,MAX] · metby/overlaps/includes/finishedby [MIN,0] · during
  [1,MAX].
Fed into the certified D-109 STP machinery as edges (anchor→self)=H
iff H<MAX, (self→anchor)=−L iff L>MIN, the closure reproduces the
D-119 never/finite classification EXACTLY — including `during` leaking
BOTH sides (its −1 backward row-max is the after[lo>0]-class negative
⇒ NEVER) — and the deadline stays endTS + reach + 1 (every finite
Allen reach is ZERO). Two composition facts pinned by mispredict-
then-verify: the dependency matrix is PER-RULE (cross-rule chains do
NOT compose — the ladder's one first-run mispredict), while IN-RULE
chains SUM through the closure (coincides 0 + after[0,100] ⇒ reach
100); a NEVER-marking rule overwrites another rule's finite reach for
the same type (the D-109 overwrite, re-confirmed).

THE LADDER: `probes_pending/cep/e_allen/gen_allen_ladder.py` +
`check_allen_ladder.py` — 124 predictions-first cells (13 ops × both
positions × dur {0,50} × exact deadline boundary pairs for finite /
far-advance for never; 8 param variants incl. the after[3,9] param-FED
control; 3 composition shapes): **ALL PREDICTED** against the oracle,
then **124/124 engine-vs-oracle** after the port.

PORT (engine.rs, the D-120 fence site): the `matches!(op, After |
Before)` edge-emission gate gains the else-branch pushing each Allen
op's constant edges + temporal_pos_type registrations (positives and
the D-132/D-135 phantom slots alike). No closure/deadline/reaper
changes — constants into certified machinery. `tools/fuzz_cep.py`'s
explicit-@expires Allen gate LIFTED (un-annotated types draw freely
under Allen ops).

SCENARIOS: the 17 `xf_cep_e_*` witnesses GRADUATED to regressions/;
9 reach pins added (`pr_cep_allen_inf_*`: both coincides positions,
the meets asymmetry pair, the during leak, param-blindness, in-rule
chain sum, cross-rule non-composition, the mix-never overwrite —
facts-only probes, `expect_inert`).

GATES: corpus **11/1084/326** byte-identical (probes 1075→1084, regressions
309→326), lint **1509 live/0/0**, cargo 9, bindings **72**; fence-lifted
fuzz_cep 313/941/943/945 ×400 = 0-div except ONE seed-313 flag, cf313x346 — bisected PRE-EXISTING (fails identically on a 61f4281 worktree; its during-rule infers NEVER both trees) = a temporal-join firing-choice latent newly REACHED by the lifted draw stream, quarantined to xfail/ with the item-1b tail ledger; expop/notpop sweeps not run — the change surface is @expires reaping of un-annotated Allen types, which the expop/notpop generators never emit; main-axis
untouched by construction (gen.rs emits no temporal ops) —
analytically inert (gen.rs emits no temporal ops; the corpus byte-identity covers the compile path).

The Bryan-noted post-faithfulness enhancement (Allen algebra BEYOND
Drools — coherent inference where Drools is incoherent, e.g. the
during leak and the param-blind intervals) remains a roadmap item
(docs/allen-beyond-drools.md), now with the faithful baseline landed.

## D-165 — tj-tail latent ledger CRACKED: both composition latents are ONE family, UPDATE-RECENCY ordering (2026-07-11)

RECON (no engine change; the port is Bryan-gated). The two quarantined
fuzz_cep composition latents (`cf933x385`, `cf313x346` — both bisected
PRE-EXISTING, both re-verified ORDER-class on HEAD by full firing-
multiset comparison) minimized (tools/minimize.py, keyed variant
pinning the diverging rule so the ORDER divergence could not drift) to
single-digit-line reproducers whose ONLY active ingredient is a fact
UPDATE. Every handoff candidate seam — BfShadow×agenda partition,
shadow×shadow shared-P, tj_epoch drain, salience interleave, chain
enumeration — was a red herring: cf313x346 reduces to ONE unsalienced
rule (`E0() E1(before[0,100]) E2(after[0,100])`), cf933x385 to two
(`not E1() P()` + an undropped window:time accumulate).

MECHANISM (12 divergence cells + 6 control cells, oracle 3×-
deterministic; battery committed to `probes_pending/cep/tj_tail/`,
report `docs/tj-tail-update-recency-mechanism.md`):
- JOIN CELL: Drools enumerates temporal-join partners for later right
  arrivals as most-recently-UPDATED first (durable across epochs,
  permanent across multiple arrivals, value-blind, idempotent, stacking
  most-recent-first), then untouched facts in insertion order; the
  engine's (left_sseq,left_seq) ASC scan keeps pure insertion order
  (re_add-to-tail is erased by the original stamps). Controls: anchor
  (leftmost) updates and rightmost-pattern updates are order-inert.
  Sub-seam: same-epoch multi-update refires FIFO oracle / LIFO engine.
- EVENT-NOT CELL: baseline blocked-left release at blocker expiry is
  recency-DESC and MATCHES (the D-134 reversal); an update to a
  blocked-under left IN the expiry epoch (either side of the advance)
  hoists it release-FIRST oracle-side. EPOCH-LOCAL, unlike the join
  cell.
- NOT raw Drools list placement (TupleList.add is tail-append —
  sources read, no code copied); the internal Drools seam is left
  uncracked deliberately (observable spec is probe-pinned; a BfDump
  graft can settle internals if the port needs it).

STATUS: both cf* witnesses stay QUARANTINED (xfail `_finding`s updated
to point at the report); the family spec = the committed battery.
FIX PLAN (pre-read only): a durable front-stamp on update re-add for
the join scan + an epoch-local hoist in the not-release path (⚠ D-106
caveat there — checker-first); model-check in Python over an
update-heavy fuzz axis BEFORE any Rust, then the Bryan gate.
GATES: no engine change — corpus 11/1084/326 byte-identical, lint
green over the new battery (12 open_divergence + 6 live controls),
cargo 9 (see commit).

## D-166 — the update-recency ORDER port: tj-tail family CLOSED, cf313x346 + cf933x385 GRADUATED (2026-07-11)

Bryan gated the D-165 fix plan; this slab executes it Python-first.

SPEC (model_join_flush.py v3 `usimulate` + the `fuzzu` update-heavy
population generator — **0 divergences on 2,000/2,000** vs the live
oracle, seeds 42/7/101/202; disciplines pinned by a 7-cell u-ladder
`probes_pending/cep/tj_tail/tjt_u*`):
- temporal MATCH uses the INSERT-time ts (handle-stamped; a ts-field
  update changes the printed value only);
- ROOT-pattern (leftmost) updates do NOT re-propagate (u2; re-explains
  the D-165 m8 control);
- a non-root UPDATE moves the fact/tuple to the TAIL of its node
  memory and re-propagates child UPDATEs staged with the SAME
  prepend/once-reversal discipline as inserts (grid-searched:
  UPD_BETA=prepend, UPD_TERM=prepend, RUPD_ORDER=opposite-memory
  scan — the alternatives die on the population at 52-81%);
- each external update action is its OWN propagation batch (FIFO
  across actions); a tuple staged twice in one epoch fires ONCE at its
  first staging position (u5).

ENGINE PORT (engine 0/2,000 on the same population + 7/7 u-ladder;
all four pieces gated to temporal/event paths, plain sessions
byte-identical by construction):
1. phreak.rs `do_join_node` reorder block: a TEMPORAL node refreshes
   the updated left's `lseq` (`refresh_left_seq`) — the certified
   (left_sseq, lseq) partner scan then sees the tuple at its NEW
   memory position chronologically (tail now, before any later fill).
   The first attempts (fresh global sseq; an upd_rank major key) were
   WRONG — rank-major pins updated-after-everything, but a fresh left
   arriving after the update must enumerate after it (mju42x142);
   chronological lseq is the memory order exactly.
2. engine.rs `update_fact`: an EVENT-type update gets the D-125
   per-arrival trigger-scoped `stream_flush` (each update action = its
   own batch ⇒ FIFO refires). Plain-type updates keep the certified
   batch path (the shadows pin it).
3. `stream_flush_ex`: the k=1 stash scopes to PRE-EXISTING upds (the
   9-field stage_snapshot + per-window upd counts) so the update
   trigger sees its own effect; `touched_node` counts upd growth.
   Insert triggers add no upds ⇒ byte-identical.
4. BfShadow: `on_p_update` also hoists the fact inside an
   already-formed emission queue (q = reverse(rtm) ⇒ move-to-tail ≡
   hoist-to-front) + rebuilds emit_rank; and the D-150-era `windowed`
   shadow-construction exclusion is LIFTED (cf933x385's cell needs the
   shadow under a window:time accumulate over the blocker type — the
   window deadline forces an early eval that freezes q before the
   update's queue position). pn/ex/px shadows untouched.

SCENARIOS: cf313x346 + cf933x385 GRADUATED to regressions/ (328); the
12 D-165 battery divergence cells flipped to live pins; +7 u-ladder
pins. NEW AXIS: `SEINE_TJUPD=1 tools/fuzz_cep.py` (update-heavy,
temporal-target-biased; flag-off stream verified byte-identical
200/200). RESIDUAL LEDGER: the axis flushes 6 PRE-EXISTING adjacent
latents (identical on the pre-port base bank — NOT port regressions),
quarantined to xfail/: cf6001x245/cf6003x274 (tj partner-choice),
cf6004x233/cf6005x208 (self-join SELF-PAIR choice), cf6001x384 (the
one SET-class find), cf6002x359 (a reached witness for the KNOWN
D-117-guarded non-termination). Known-name suppression: these 6 are
the axis's expected flags.

GATES: corpus **11/1084/328** byte-identical; lint **1536/0/0**;
cargo 9 suites; bindings 72 (fresh maturin build); fuzz_cep
313/941/943/945 ×400 = 0 div; SEINE_TJUPD 6001-6005 ×400 = exactly
the 6 ledger names; notpop event 590 + plain 210 + expop 600 + plain
293 regenerated fresh seeds 91-94: engine 0/1000+0/1000 AND all four
model specs ALL MATCH (flush/pflush/EMODEL=flush/pexists); mju
population 0/2,000. Main-axis analytically inert (every change gates
on temporal nodes / event types / event-session flushes).
## D-167 — the SEINE_TJUPD residual ledger: all six mechanisms cracked; the SET fix validated (recon; ports Bryan-gated) (2026-07-11)

RECON (no engine change lands in this slab — the validated SET fix is
REVERTED pending the gate). All six D-166 ledger cases re-verified
pre-existing on a cb7d443 (pre-port) worktree A/B — none is a D-166
regression. Full report: `docs/tjupd-ledger-mechanisms.md`; batteries +
the v4 model: `probes_pending/cep/tj_upd/` (6 minimals + s/r/m ladder
cells, 17 files).

THREE mechanisms:
1. **SET (cf6001x384)** — trace-pinned: a stale staged UPD on an
   UNLINKED temporal join fails the D-125 per-arrival eligibility gate
   (clean-insert-only), the arrival falls to the childless
   `self_drain_delta`, and the pair is PERMANENTLY lost across the
   unlink/relink (the relink eval never re-derives from memories).
   SIX-LINE FIX validated in recon (self-drain fallback additionally
   requires empty upd staging — mixed staging stays for the eval):
   minimal + m4_split + the FULL case (19/19) graduate; corpus
   11/1084/328 byte-identical; tjt 26/26; mju 0/200; fuzz_cep
   313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6003 ×400 = only the
   known ORDER/hang names (the SET name gone); cargo green. Fix
   reverted; re-apply per the report diff on the gate.
2. **ORDER (cf6001x245/6003x274/6004x233/6005x208)** — one family: the
   anchored/self-join MODIFY composition. Spec = model_tjupd_v4 (built
   on the certified D-125 Node + D-156 arrival): alpha ENTRY scans the
   PRE-move memory; EXIT kills + CANCELS same-epoch pending fires;
   in-place anchor updates A'-refire the anchor's children in REVERSED
   child-list order (list APPENDS on creation, MOVES-TO-END on refire),
   gated by the $a watch mask ({tag}; ts-only is $a-deaf); $b hears
   every update (listen-all): refires (ltm-scan + prepend) and ALWAYS
   re-adds to the memory tail (phase C, post-scan; r-ladder: a single
   value-identical update coincidentally restores insertion order —
   only doubles expose it); a surviving anchor also tail-re-adds in
   the LEFT memory on tag-touch; per-epoch dedup keeps first position.
   VALIDATED 1,181/1,200 (98.4%, seeds 11/21/31). RESIDUAL (named):
   the same-fact double-touch QUEUE-POSITION sub-rule (u5-chains keep
   FIRST position; self-join re-entries move the refire BEHIND the
   entry) — D-106-adjacent; needs its own discriminator ladder before
   any port. No engine change for this family (doctrine: model 0-div
   first).
3. **HANG (cf6002x359)** — the KNOWN D-117-guarded executor re-add
   spin, minimized to 4 facts (tju_359_spin_min: delete-of-just-
   expired + fresh matching pair in one epoch). Own checker-first arc;
   ⚠ D-106 caveat.

METHOD: the keyed minimizer again (plus an errored-keyed variant for
the hang case — minimize-through-a-spin works, ~18s/diverging-variant);
the s/r/m ladders; the v4 model iterated 68%→30%→25%→3%→2% with a
knob grid at each plateau; the r1-vs-r3 trap (a single update
coincidentally order-neutral) caught by the model, not by hand.

Deliverable order for the gate: (a) land the SET fix (evidence
complete); (b) close the v4 residual then port the ORDER family;
(c) the spin root-cause slab.

GATES this slab (recon artifacts only): corpus 11/1084/328
byte-identical, lint green over the new battery, cargo 9, xfail
findings sharpened, minimals + model committed.
## D-168 — the SET fix LANDED: cf6001x384 graduated; the mju spec's committed default repaired (2026-07-11)

Bryan gated deliverable (a) of the D-167 ledger; this slab lands it.

ENGINE (the D-167 §1 six-liner, applied verbatim from the report): in
`engine.rs stream_flush_ex`, the childless self-drain fallback now
additionally requires EMPTY upd staging (s_right/s_left/s0_in — three
conjuncts). Before: an arrival reaching an UNLINKED temporal join
alongside a stale staged upd failed the D-125 clean-insert-only
eligibility gate, fell to `self_drain_delta` (memory WITHOUT
children), and the pair was PERMANENTLY lost across the unlink/relink
(the relink eval never re-derives from memories). Now mixed staging
stays in place for the eval. The exists early-continue and the D-125
eligible cascade sit upstream of the branch; corpus byte-identical.

SCENARIOS: cf6001x384 GRADUATED xfail→regressions (329); tju_384_min +
tju_m4_split flipped to LIVE pins in probes_pending/cep/tj_upd
(open_divergence dropped, findings kept with the GRADUATED
annotation). The ORDER/hang cells (tju_208/233/245/274/359 minimals,
r3/s4/s8) stay open_divergence — lint re-verifies they STILL diverge,
i.e. the fix did not leak into the ORDER family.

GATES (all green): witnesses tju_384_min + tju_m4_split + the FULL
cf6001x384 (19/19 firings) PASS, controls tju_m2/m3/m5 PASS; corpus
**11/1084/329** byte-identical; tjt battery 25/25; mju ENGINE leg
0/200 (seed 42); fuzz_cep 313/941/943/945 ×400 = 0 div; SEINE_TJUPD
6001-6005 ×400 = EXACTLY cf6001x245/cf6002x359/cf6003x274/cf6004x233/
cf6005x208 (the SET name GONE); cargo 9 suites; lint **1554/0/0**
(+1 = the graduate entering the linted regressions tier); bindings 72
(fresh maturin --release, .so refreshed); shadow populations on fresh
seeds 91-94 (notpop event 197 + plain 97, expop event 200 + plain
150): engine 0-div AND all four model specs ALL MATCH
(flush/pflush/EMODEL=flush/pexists).

SPEC REPAIR (flushed by this slab's gate; engine-independent):
`tools/model_join_flush.py` was COMMITTED at D-166 with
`RUPD_ORDER="childlist"` — a grid LOSER (158/200 = 79% divergent on
mju seed 42, inside the "alternatives die at 52-81%" band the D-166
entry itself records) — while the validated winner is `oppmem`
(opposite-memory scan, exactly what the D-166 text says won). An
8-cell knob grid against one cached oracle pass reproduced the D-166
conclusion (oppmem 0/200; every other combo 104-173/200); fuzzm (the
v2 no-update control) 0/200 shows the base model was never affected.
Default flipped, the docstring's grid-override argv (documented but
never parsed) wired into __main__, re-certified 0/200 on seeds 42 AND
7. The D-166 0/2,000 claim was REAL — the committed default just
didn't match the session's validated config (the session-state trap,
same family as the scratch-only keyed minimizer). The mju ENGINE leg
was green throughout — engine-vs-oracle gates were never affected.

NEXT (unchanged, Bryan-gated): (b) close the model_tjupd_v4
double-touch residual → port the ORDER family (cf6001x245/6003x274/
6004x233/6005x208); (c) the cf6002x359 spin root-cause arc
(checker-first, ⚠ D-106). Handoff:
`~/.claude/plans/tjupd-ledger-handoff.md` (deliverable (a) DONE).
## D-169 — the double-touch residual CLOSED: model_tjupd_v4 0-div on 2,200/2,200; the ORDER-family port is UNBLOCKED (recon/spec; port Bryan-gated) (2026-07-11)

Bryan directed deliverable (b) step 1 (post-D-168 push). NO engine
change — this slab is the discriminator-ladder arc the D-167 handoff
prescribed, executed predictions-first per doctrine.

THE LADDER (5 rounds, 31 cells, all oracle-2×-stable except the one
documented flake; `probes_pending/cep/tj_upd/ladder_dt{,2,3,4,5}.py`):
round 1 (13 cells: the handoff's 4×2 core + 3-action interposers that
the 2-action fuzz generator cannot reach + move-visibility cells)
REFUTED both naive theories — the four residual-bank shapes moved
while int1/int2/dt4 kept first and en1/en3 saw immediate moves; round
2 (6) split exit-vs-plain move timing and pinned drain-at-reentry;
round 3 (5) killed value-identicalness as the key (ts-VI is fully
immediate — the written FIELD is the discriminator) and exposed the
insert-scan/modify-scan split; round 4 (4) pinned the tag-write
movability class (both-write defers with tag; in-place keeps; DR1's
oracle = the predicted drain-at-reentry order exactly); round 5 (3)
pinned the self-slot mid-batch and by-fact scoping. Hand-replays of
bank cells (tu21x329, tu21x244) matched the oracle verbatim before
encoding.

T6 SUB-RULES (full statement in the model docstring; report §2):
(1) MOVABILITY — upd emissions staged by TAG-writing actions (noop
y→y, both-fields, in-place z→z, exit z→y) are movable-by-f; ts-only
actions stage anchored emissions. (2) RELOCATION — re-emitting a
movable emission during a LATER alpha-ENTRY of the SAME fact moves it
behind the entry's ins batch; anchored/different-fact/same-action/
non-entry re-emissions keep first (u5 was the different-facts case all
along); ins-staged absorb (the x126 self-pair). (3) MOVES are
immediate post-scan for every class. (4) SELF-SLOT — an entry's scan
sees the entering fact itself at its pre-epoch slot when its
same-epoch moves were tag-class (exits included: x56/x227/dt2b/x145),
at its moved slot after ts-only moves (en3).

TWO ENCODING ITERATIONS, each caught by evidence: a same-ACTION
relocation overreach (dt4/int2/ip1 — A'-then-phase-B must keep first;
fixed with an action counter) and a non-entry relocation overreach
(tu11x84, a NEW diverger the first encoding created: in-place
re-touches keep first; fixed by requiring the relocating action be an
alpha-ENTRY). Final: ladder 31/31, the 17 buggy-run divergers 17/17,
and **fuzz 0-div on 2,200/2,200 — bank seeds 11/21/31 (the 18 D-167
divergers all green) + fresh out-of-sample 41/51/61 ×400 each**.

⚠ ORACLE FLAKE (documented, quarantined, NOT chased — the fz_42_84
class): the exit-move's visibility to a later same-epoch
DIFFERENT-fact entry scan is JVM-nondeterministic (cell ex9: 16 moved
/ 2 unmoved across JVM instances, each internally consistent; the two
unmoved runs were one python session's pair). The model encodes the
moved majority; ex9 is deliberately NOT in the battery. Related: the
D-167 seed-11 tally was 6 divergers, today's deterministic count is 5
— consistent with one flaky cell in this same boundary.

SCENARIOS: the 30 stable ladder cells graduated into the battery as
`probes_pending/cep/tj_upd/tjdt_*.json` — 17 open_divergence ORDER
pins (engine still on v4-base order pending the port) + 13 live
controls. Lint **1584/0/0**. Corpus untouched (no engine change);
make diff 11/1084/329 byte-identical re-verified.

NEXT (Bryan gates): (b2) the ORDER-family ENGINE PORT — now
doctrine-unblocked (spec 0-div); seam map = D-167 §2 + the T6 delta
(per-arrival update flush must reproduce movability, entry-relocation,
and the self-slot scan; engine seams: alpha-entry pre-move scan, A'
reversed child-list, the D-166 lseq refresh on the mask-miss path,
anchor left-re-add). ⚠ D-106 adjacency stands. (c) the spin arc.
## D-170 — the ORDER-family ENGINE PORT: cf6001x245/6003x274/6004x233/6005x208 GRADUATED; population 719 fixed / 0 regressed (2026-07-11)

Bryan gated (b2) post-D-169. The T6 spec is now IN THE ENGINE; the
whole SEINE_TJUPD ORDER family is closed. Fuzz-driven port: fast
battery (75 cells incl. the 4 FULL witnesses) after every piece, the
2,200-case model population as the wide loop, corpus as the guard.

THE PIECES (all scoped to temporal joins; plain sessions untouched):
1. **The T6 REPLAY** (`phreak.rs temporal_upd_replay`): an eval batch
   carrying staged UPDATES on a temporal 2-PATTERN join (per-action
   update evals + unlinked cross-action accumulations) replays its ops
   in ARRIVAL-STAMP order — LIns (entry: memory append + CURRENT-
   memory scan via the self-slot view), RIns (partners in left-memory
   order), RUpd/LUpd (pure refires: anchors-in-memory-order / child-
   list order via a cross-op STEAL), RMove/LMove (the per-ACTION
   memory moves). Each op's block composes FIFO into the eval's trg
   (per-op local staging + merge — the model's per-action buffer),
   with ins-absorb / keep-first / A'-steal dedup. Clean insert-only
   batches keep the certified D-125/D-156 arms untouched.
2. **PENDING MOVES** (`Node.pending_{r,l}moves`): every tag/ts update
   ACTION records ONE stamped memory-move at staging (dedup-proof — a
   re-touch of a staged upd appends another move, tu11x95), applied by
   the replay at its stamp so moves interleave correctly with
   still-staged inserts (tu11x92). Prior-epoch pendings (stamp ≤ the
   fire-boundary floor) apply silently. LMove refreshes lseq AND jumps
   the left_sseq era (tu21x20); the replay's LIns re-stamps a
   re-entering left freshly (tu11x197).
3. **SELF-SLOT VIEW** (`Node.scan_rights_view` + the per-epoch
   rights0/rlog/self_dirty bookkeeping, reset at fire_all): an entry's
   scan sees the entering fact at its pre-epoch slot under tag-class
   moves (T6-4); the epoch floor is the last pre-fire stamp (strict >).
4. **TERMINAL relocation + consume order** (`engine.rs`
   consume_term_{upds,ins} + tj_mark_movable + RuleNet.act_movable +
   the tj_trigger/tj_entered context set by update_fact): an eval
   whose trigger alpha-ENTERED the rule's anchor consumes INS before
   UPDS (the model's phase A-then-B) and relocates queued MOVABLE
   activations (staged by a tag-writing action of the same fact)
   behind the entry's ins batch; everything else keeps the certified
   updates-then-inserts consume and keep-first (u5/dup1). ⚠ D-106:
   these touch the QUEUE's content order only — the pick, halt and
   executor logic are untouched.
5. **A' child-list discipline**: the AB-arrival walk re-slots the
   self-pair to the END of the arriving anchor's by_left list (the
   model's scan-children-then-self creation order; emissions
   unchanged); the A' snapshot is taken before the right-side
   re_add_left moves (ip1); tag-class staging is marked ph=6 by
   on_update (anchor-type + anchor-listen∩mask).

EVIDENCE-DRIVEN ITERATIONS (each caught by the battery/population,
several by ONE cell): the same-action relocation overreach (dt4/int2/
ip1), the non-entry relocation overreach (tu11x84), the LIFO block
composition (en3/en4/int2), the eager-move-vs-staged-insert interleave
(tu11x92 vs x95 — resolved by the pending-move ops), the stamp
keep-first on dedup (en3), the epoch-floor off-by-one (tu51x131), the
stale re-entry lseq (tu11x197), the anchor sseq era (tu21x20).

RESULTS: fast battery **75/75** (30 tjdt ladder cells, ALL tju
minimals/ladders incl. r3/s4/s8, tj_tail 25/25, the 4 FULL cf*
witnesses); population **2,197/2,200** — the 3 residuals are
A/B-PROVEN PRE-EXISTING on the pre-port tree (quarantined:
tu51x80/tu51x187 = SET losses in an exit→unlinked→re-enter RJ shape,
the relink-eval never re-derives — kin of the D-168 SET family, own
recon slab; tu51x207 = a 3-touch ORDER compound) ⇒ **+719 fixed /
0 regressed**. GATES: corpus **11/1084/333** byte-identical (the 4
witnesses GRADUATED to regressions/); tjt 25/25; mju model 0/200 +
engine 200/200; fuzz_cep 313/941/943/945 ×400 = 0; **SEINE_TJUPD
6001/6003/6004/6005 ×400 = 0 divergences** (the ORDER names GONE) +
6002 = only cf6002x359 (the D-117 spin); notpop/expop fresh
populations engine-clean; cargo 9; lint **1591/0/0** (21 battery
cells flipped live with GRADUATED findings; 3 residuals quarantined);
bindings 72 (fresh --release .so).

REMAINING TJUPD ledger: cf6002x359 (the D-117 spin — deliverable (c),
checker-first, ⚠ D-106) + the new tu51x ledger (the relink SET family
recon). The tju_359_spin_min stays open_divergence.
## D-171 — the relink-SET recon: mechanism CRACKED, fix VALIDATED (reverted; the port pends Bryan's gate) (2026-07-11)

Bryan directed the tu51x recon post-D-170-push. RECON slab — the
validated fix is REVERTED per doctrine; diff verbatim in
`docs/tjupd-ledger-mechanisms.md` §4.

MECHANISM (trace-pinned, EVAL/FLUSH debug + the minimized
`tju_relink_min`): an anchor's alpha-EXIT stages an s0-DEL that
nothing consumes while the rule is unlinked — (1) dels don't count as
flush-TOUCH (`touched_node` checks ins/upd growth only) so the exit's
own trigger flush never evaluates; (2) unlinked evals don't fold s0
(lazy PHREAK — the oracle defers too); (3) at the RE-ENTRY the dstash
("earlier actions' dels batch to the fire") hides the del from the
relink eval, which re-creates the pairs; (4) the del drains at the
NEXT FIRE — after the re-entry — and the value-keyed child/queue kill
destroys the freshly re-created same-VALUE pairs (Drools kills by
tuple OBJECT at the relink drain, del-then-ins in stage order; the
Staged no-fold comment's object-identity distinction made lethal by
the deferral). Also exposed: the backlog arrival's flush eval pairs
against the STALE not-yet-deleted left (dies pre-fire — SET-invisible
but order-relevant).

LADDER (probes_pending/cep/tj_upd/tju_relink_*): the same-epoch
exit+re-entry ALSO diverges (`_sameepoch` — RJ has no $b-side upd, so
unlike the SJ analogs the re-entry stays cascade-eligible and the
dstash bites even within one epoch); gap epochs preserve the loss
(`_gap`); no-backlog converges (`_nobacklog`, live control — the del
drains at its own epoch's pop when no arrival eval interleaves).

FIX (validated, REVERTED): the dstash exempts an s0-del whose fact
RE-ENTERS in the same flush — the eval processes del-then-ins in
stage order (Drools' relink drain). ~15 lines in stream_flush_ex's
dstash loop; flush-layer only, the executor/halt machinery untouched
(⚠ D-106-clean by construction). Evidence with the fix in-tree:
all five family witnesses PASS (min/sameepoch/gap/tu51x80/tu51x187);
corpus 11/1084/333 byte-identical; fast battery 71/71; population
2,199/2,200 (only tu51x207 — the separate 3-touch ORDER compound);
fuzz_cep 313 + SEINE_TJUPD 6001 ×400 = 0; cargo 9.

TOOLING: `tools/minimize_keyed.py` COMMITTED (the per-session-rebuilt
keyed minimizer from the D-165/D-167 recons — KEY-literal-pinned
divergence predicate, unquoted-rule split; the handoff's trap list
retires an entry).

NEXT (Bryan gates): (d) land the relink-SET fix (evidence complete —
re-apply §4's diff, graduate the 5 witnesses + flip _nobacklog
stays-live, full battery); (c) the cf6002x359 spin arc (checker-first,
⚠⚠ D-106); tu51x207's ORDER compound recon (smallest).
## D-172 — the relink-SET fix LANDED; ⚖ THE IDENTITY-MODEL LAW logged (2026-07-11)

Bryan gated deliverable (d) and ruled the D-171 distillation a
STANDING BEHAVIORAL LAW. Both land in this slab.

⚖ **THE IDENTITY-MODEL LAW (Bryan's ruling): the engine kills by
VALUE; Drools kills by tuple OBJECT identity.** A re-created
same-composition tuple is a NEW object in Drools that earlier-staged
deletes cannot touch; the engine's value-keyed children/queue/staging
let a DEFERRED delete reach across a re-creation and kill fresh
state. The two models coincide exactly while processing order
preserves del-before-recreate — every divergence in this class is an
ORDERING deferral made lethal by value-keying, and the faithful fix
restores the STAGE ORDER (as this slab's fix does), not bolted-on
identity. Known instances: the Staged no-fold rule (c13) and the
D-171 relink SET loss. TRIAGE RULE: a SET loss whose trace shows a
deferred del draining after a same-value re-creation is THIS law —
check the del's deferral path (dstash / unlinked staging / fire
batching) first. Expect it to explain future finds wherever deletes
defer: TMS retraction cascades, expiration drains, halt-model
re-adds. Full statement at the top of
`docs/tjupd-ledger-mechanisms.md`; also in the workflow memory's
doctrine list beside the uniform-fold signature.

ENGINE (the D-171 §4 diff, applied verbatim + the law cited in the
comment): `stream_flush_ex`'s dstash exempts an s0-del whose fact
RE-ENTERS in the same flush (a fresh same-fact s0-ins) — the eval
processes del-then-ins in stage order, Drools' relink drain.
Flush-layer only; the executor/halt machinery untouched (D-106-clean).

SCENARIOS: tju_relink_{min,sameepoch,gap} + tu51x80 + tu51x187
flipped to LIVE pins (GRADUATED findings); tju_relink_nobacklog stays
the live control. The battery's open ledger is now EXACTLY
tju_359_spin_min (the D-117 spin) + tu51x207 (the 3-touch ORDER
compound).

GATES (all green): the 10-witness family sweep PASS; corpus
**11/1084/333** byte-identical; population 2,199/2,200 (only
tu51x207); fuzz_cep 313/941/943/945 ×400 = 0; SEINE_TJUPD
6001/6003/6004/6005 ×400 = 0 + 6002 = only cf6002x359; tjt 25/25;
mju model 0/200 + engine 200/200; notpop/expop fresh populations
engine-clean; cargo 9; lint **1595/0/0**; bindings 72 (fresh
--release .so).

NEXT (Bryan gates): (c) the cf6002x359 spin root-cause slab — THE
LAST OPEN TJUPD ITEM but one (checker-first, ⚠⚠ D-106); tu51x207's
ORDER-compound recon (smallest).
## D-173 — the tu51x207 3-touch ORDER recon: mechanism CRACKED, fix VALIDATED (reverted; port pends Bryan's gate) (2026-07-11)

Bryan directed the recon with two standing constraints, both honored:
(1) do NOT fit the identity-model law — verified inapplicable EARLY
(no delete of any kind exists in the composition; the law's
activation condition is categorically absent) and the mechanism found
is orthogonal (a per-action side-effect elision); (2) ladder, not
fuzz — the 3-touch is past the 2-action generator's structural reach
(proven: the discriminator requires a leading VI in the SAME epoch as
the entry PLUS a next-epoch in-place = 3 touches), so the evidence is
5 constructed cells; the population/fuzz runs are reported as
CONTROLS only.

HEAD START (structural): tu51x207 is a model-population cell —
model_tjupd_v4 is 0-div on it — so the oracle mechanism was already
encoded in the validated T6 spec and the recon reduced to
engine-vs-MODEL departure hunting, oracle-confirmed.

MECHANISM (§5 of the report): the $b-refire's CHILDLIST move-to-end
is a per-ACTION side effect — the model re-runs phase B at every
touch against the CURRENT children (the ENTRY action's own phase B
moves the just-created self-child to the childlist end; its emission
dedups away but the move lands). The engine's replay attaches the
re-adds to the ONE dedup'd RUpd op at the FIRST touch's stamp
(keep-first — correct for EMISSIONS only), so a leading same-epoch
tag-VI pins the refire pass before the entry exists and the
self-child never moves; the next epoch's A′ then fires it at the
scan slot instead of the moved end.

DISCRIMINATION (the round-3 bar, per the directive): the
x2l1-vs-x2l2 minimal pair differs ONLY by the leading VI and the
engine flips exactly as stamp-elision predicts (green without the
VI). Controls: x2l3 (cross-epoch VI = vacuously correct — the
prior-epoch move relocates the memory slot so the scan orders the
self-child last), x2l4's second in-place (un-dedup'd refires DO
move), x2l5 (the move precedes later partner appends). All 5 cells
oracle-2×-stable, model==oracle on every one.

FIX (validated, REVERTED; diff verbatim in the report §5): the
childlist re-adds move from the RUpd emission op to the per-ACTION
RMove ops in `temporal_upd_replay` (~10 lines; replay-internal, the
executor untouched — D-106-clean). Evidence in-tree: ladder 5/5
engine==oracle, tu51x207 + tjx207_min PASS, fast battery **81/81**,
corpus **11/1084/333** byte-identical, cargo 9. CONTROLS: the model
population **2,200/2,200** — the whole SEINE_TJUPD update axis
converges — and fuzz 313 + TJUPD 6001 ×400 = 0.

SCENARIOS: tjx207_min + x2l1/x2l4/x2l5 = open_divergence pins (red
at HEAD, findings point at §5); x2l2/x2l3 = live controls;
tu51x207's finding updated to CRACKED. `ladder_x207.py` committed;
`minimize_keyed.py` extended with '!'-negated signature keys (the
ORDER-class minimization this recon needed).

NEXT (Bryan gates): (e) land the §5 fix (evidence complete — takes
the battery's open ledger to EXACTLY tju_359_spin_min); (c) the spin
arc — THE LAST OPEN TJUPD ITEM (checker-first, ⚠⚠ D-106).
## D-174 — the 3-touch fix LANDED; ⚖ THE DEDUP/SIDE-EFFECT LAW logged (2026-07-11)

Bryan gated (e) and ruled the D-173 distillation the SECOND standing
behavioral law, ledgered beside the identity-model law.

⚖ **THE DEDUP/SIDE-EFFECT LAW (Bryan's ruling): staged-op dedup
(TupleSets keep-first) folds EMISSIONS only; per-touch processing
side effects run once per ACTION.** Every touch's processing (memory
re-add/move, child-list re-add, stamp refresh) executes at its own
action against the then-current state — a side effect attached to the
dedup'd staged op runs once at the FIRST touch's stamp against the
FIRST touch's state, silently eliding later touches. Engine
corollary: side effects ride PER-ACTION ops (the D-170 pending-move
pattern); only emissions belong on the dedup'd op. Instances: the rtm
tail-move (tu11x95/D-170), the childlist move-to-end (tu51x207/this
slab). TRIAGE: an ORDER divergence in a multi-touch composition where
the engine acts as if only the FIRST touch happened — check whether a
side effect rode a dedup'd op. Activation needs ≥2 same-fact touches
with a state change between (beyond 2-action fuzz reach ⇒ constructed
ladders). Ledgered at the top of `docs/tjupd-ledger-mechanisms.md`
beside the identity-model law + the workflow memory's doctrine list.

ENGINE (the D-173 §5 diff, applied verbatim + the law cited): the
childlist re-adds move from the RUpd emission op to the per-ACTION
RMove ops in `temporal_upd_replay`. Replay-internal; executor
untouched (D-106-clean).

SCENARIOS: tjx207_min + x2l1/x2l4/x2l5 + tu51x207 flipped to LIVE
pins (GRADUATED findings); x2l2/x2l3 stay live controls. **The
battery's open ledger is now EXACTLY tju_359_spin_min** — the whole
SEINE_TJUPD update axis is closed except the D-117 spin.

GATES (all green): ladder 5/5 engine==oracle; corpus **11/1084/333**
byte-identical; fast battery 81/81; cargo 9; fuzz_cep
313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6005 ×400 = only
cf6002x359; tjt 25/25; mju 0/200 + 200/200; notpop/expop fresh
engine-clean; lint **1601/0/0**; bindings 72. CONTROL: the model
population **2,200/2,200** — the update axis fully converges.

NEXT (Bryan's gate): (c) the cf6002x359 spin root-cause slab — THE
LAST OPEN TJUPD ITEM (checker-first, ⚠⚠ D-106; witness
tju_359_spin_min, 4 facts).

## D-175 — the cf6002x359 SPIN ROOT CAUSE cracked: the TMS teardown CAUSE SPLIT; fix validated-then-REVERTED (Bryan gate); the whole D-117 hang family cured under the fix (2026-07-11)

Deliverable (c), the last open TJUPD item. Framed per Bryan's directive as an
ordinary divergence that manifests as non-termination: Drools halts and fires
a complete sequence; reproducing that sequence was the bar, and it is met.

### 0. Tooling (LANDED, stays in-tree)

- `SEINE_SPIN_GUARD` env override of the D-117 limit (`spin_limit` field,
  default 50M unchanged). MEASURED verdict identity on the witness: guard trip
  at 100k in 0.08s == guard trip at 50M in 26.8s — the guard is a backstop,
  not semantics, now as fact. Recon cost per spin cell: ms instead of ~26s.
- `tools/minimize_keyed.py --errored`: the HANG-variant predicate ("engine
  errored but oracle succeeded") as an explicit flag — the default predicate
  REJECTS errored runs (the handoff's silent-reduce trap). Verified: the
  witness repros under it and is at a LOCAL MINIMUM (no fact/epoch/action
  droppable; the E1s survive only via the index-shift guard — see §2 c4).
- TMS debug lens extended: `tms_on_terminal_del` prints
  expiring/exp_deferred/deferred state; the three exp_deferred drain sites
  print `TMS drain[post-fire-exp|quiescence-exp|quiescence-bare]`.

### 1. The cycle, instrumented (SEINE_EVAL_DEBUG + SEINE_TMS_DEBUG, guard=50)

Epoch 2 (`advance 50` + delete E2(0) + fresh pair): the advance marks
`tms.expiring = {E2(0), E1(13), E1(21)}` (cleared ONLY by
`drain_pending_expirations` at quiescence, engine.rs:6296). The external
delete of E2(0) — ALIVE per c_del_after_exp — kills the fired tuple
[E0(9),E2(0)] at the pop eval; `tms_on_terminal_del`'s mark-keyed check
(engine.rs:9588-region) routes the teardown to `exp_deferred`. The fresh pair
fires TJ1 (the oracle's 2nd firing HAPPENS — the engine dies AFTER completing
the oracle's sequence), then the post-fire drain (engine.rs:6877-region
`while let`) removes the entry, hands it back to `tms_on_terminal_del`, which
sees E2(0) STILL marked (quiescence never runs) and RE-ADDS the entry it was
just handed. 1000-step trace: one `drain[post-fire-exp]` + one re-add per
iteration, same tuple, forever — D-117's sentence verbatim.

### 2. Discrimination (round-3 bar; predictions logged before running)

Six cells from the witness, engine-side at guard=100k:
- c1 drop-the-delete → terminates (no mid-window entry) — delete NECESSARY.
- c2 drop-the-fresh-pair → terminates, 1 firing (no post-fire drain) —
  same-window re-fire NECESSARY.
- c3 delete the NEVER-EXPIRED partner E0(9) → SPINS. ⚖ identity-model law
  KILLED, necessity direction: no delete-of-just-expired, no same-value
  re-creation, phenomenon present.
- c7 same delete OUTSIDE the mark window → terminates. ⚖ identity law killed
  in the SUFFICIENCY direction too: its activation shape (delete-of-expired +
  same-flush re-adds) fully present, phenomenon absent. (Mechanically the law
  cannot bind here: exp_deferred keys by FactId — handle identity, not value —
  and the fresh pair demonstrably fired.)
- c4 no-E1s → SPINS (their deadlines are NOT load-bearing; D-167's minimizer
  simply couldn't drop facts below the delete's target index).
- c6 no-entry-point → SPINS (EP-independent).
- ⚖ dedup/side-effect law: precondition measurably ABSENT (≥2 same-fact
  touches required; the witness has ONE touch of one fact, and the trace shows
  the opposite shape — one touch acted on unboundedly, not N folded to 1).

MECHANISM: spin ⇔ an external delete kills a fired tuple containing ANY
still-expiration-marked fact inside the advance→quiescence mark window, AND
the same rule fires again before quiescence. The original hang-backlog
scenario is the c3 flavor (deletes an unmarked fact whose PARTNER is marked)
— which also proves a delete-clears-own-mark fix shape cannot work.

### 3. The checker: the teardown CAUSE SPLIT (oracle-pinned 3×, new corner)

The certified seam (a7c/a7d/q1/q4) distinguishes teardown cause — expiration
lazy, external-delete eager — but a7d's pin has NO mark window open; the
external-delete-INSIDE-the-window corner was unpinned (both known hang
scenarios have EMPTY by_act — the original's insertLogical rule J3 has no E1
facts, dead code — so neither could decide it). New probes, all 3×-stable:
- spin_deps_extdel (witness + insertLogical + not-D observer @9):
  [TJ1, RN, TJ1, RL] — the belief drop lands EAGERLY at the delete's
  propagation, before ANY epoch-2 firing.
- spin_deps_delpartner (delete the unmarked partner): SAME — the split keys
  on the tuple's DEATH CAUSE, not on which member carries the mark.
- spin_deps_expire (control; tuple dies by expiration): [TJ1, TJ1, RL] — RN
  never fires (D survives through the re-justification) — the a7c lazy
  behavior, reproduced by the same probe design.
- spin_deps_k1 (k=1 justifier, NO temporal join): [J1, RN, J1, RL] — eager;
  and a previously-UNKNOWN latent D-117 family member (SPINS on HEAD).

THE LAW OF THE CORNER: **lazy is the expiration cause only.** At the report
site the faithful reconstruction is: route lazy ⟺ `in_expiration_drain`
(the drain's own synchronous prunes) ∨ (some tuple member marked ∧ ALL
members alive — q1's mid-fire consume of a scheduled-but-alive fact). A
flag-false report with a DEAD member means an external delete killed the
tuple: Drools tears its beliefs down eagerly at the propagation and the
pending expiration later no-ops on the dead handle (store.kill already
models this by clearing store.expired; tms.expiring was the stale mirror).

### 4. The fix (VALIDATED THEN REVERTED — re-apply verbatim at the landing)

Two predicate edits, same cause logic at both exp_deferred producers; flush/
executor/guard untouched (D-106-clean: pick/halt/loop identical; the 19
agenda_open engine outputs byte-identical pre/post, worktree-A/B'd).

In `tms_on_terminal_del` (the direct path):
```
-        if tuple.iter().any(|f| self.tms.expiring.contains(f)) {
+        if self.in_expiration_drain
+            || (tuple.iter().any(|f| self.tms.expiring.contains(f))
+                && tuple.iter().all(|f| self.store.is_alive(*f)))
+        {
```
In `tms_eager_break` (the k=1 scan):
```
-            if act.1.iter().any(|x| self.tms.expiring.contains(x)) {
+            if self.in_expiration_drain
+                || (act.1.iter().any(|x| self.tms.expiring.contains(x))
+                    && act.1.iter().all(|x| self.store.is_alive(*x)))
+            {
```
The re-add edge dies structurally: dead-member entries never enter
exp_deferred, so the post-fire drain cannot bounce them.

### 5. Validation (fix in-tree; ALL of D-174's gate list green)

The witness reproduces the oracle's COMPLETE sequence byte-identically in
27ms (TJ1(E0 9×E2 0), TJ1(E0 128×E2 124), facts [E2(124)]). Full original
`scenarios/xfail/cf6002x359.json` PASSES (1.1s). The original hang-backlog
scenario PASSES. All six cells PASS engine==oracle. spin_deps_k1 PASSES
byte-identical. Battery: corpus 11/1084/333 byte-identical (make diff rc 0);
cargo 9 suites; fuzz_cep 313/941/943/945 ×400 = 0; **SEINE_TJUPD 6001-6005
×400 = ZERO flags — cf6002x359 GONE, the axis fully clean** (no name-keyed
skip in fuzz_cep: the case genuinely ran green); tjt 25/25; tj_upd fast
battery 60/60; mju 0/200; notpop 112 + expop 147 fresh ALL MATCH (seeds
7013/7017); lint 1601/0/0; bindings 72 (release); population CONTROL
2,200/2,200; a7c/a7d/q1/q4 seam pins PASS; agenda_open ×19 byte-identical.

RESIDUAL (new, smaller, filed): spin_deps_extdel/delpartner post-fix
TERMINATE but under-fire RN ([TJ1,TJ1,RL] vs oracle [TJ1,RN,TJ1,RL]) — the
eager drop lands mid-pop-eval AFTER the pick committed (the D-101/cf5x17
static return); Drools reopens the pick pre-fire. That is the halt fine
structure (⚠⚠ D-106) and cf5x17 certifies the OPPOSITE polarity for the
pre-drain shape — needs its own halt-matrix arc; NOT touched here. k=1 has
no such gap (the drop lands at delete-action time, pre-pick).

### 6. Artifacts + the landing checklist (Bryan gate)

Committed NOW (fix reverted): the §0 tooling; live pins
tju_spin_{nodelete,nofresh,window_split,deps_expire}; hang-backlog cells
spin_{c3_delpartner,c4_noE1s,c6_noEP,deps_extdel,deps_delpartner,deps_k1}
(+ README); this entry. The witness stays `open_divergence` (still red on
HEAD).

LANDING (on Bryan's go): (1) re-apply the §4 diff verbatim → rebuild; (2)
rerun the §5 battery; (3) GRADUATE: tju_359_spin_min → live pin, xfail/
cf6002x359 + hang-backlog/pre_existing_temporal_delete_hang + spin_c3/c4/c6
+ spin_deps_k1 → regressions// live pins, spin_deps_extdel/delpartner →
open_divergence pins (the D-106 halt-corner ledger item); (4) update the
fuzz expected-names note (TJUPD axis = zero); (5) D-17x + CURRENT STATE +
memory; commit UNPUSHED. The D-117 guard STAYS IN — the backstop for cycles
not yet met (e.g. a hypothetical all-alive-marked act at its own justifier's
post-fire drain, reachable in principle, no witness).

## D-176 — the D-175 spin fix LANDED (Bryan-gated); THE WHOLE D-117 HANG FAMILY IS CURED; hang-backlog is EMPTY; the TJUPD axis flags NOTHING (2026-07-11)

Bryan's landing call on D-175. The §4 diff re-applied VERBATIM (the two
cause-split predicates in `tms_on_terminal_del` + `tms_eager_break`; comments
now cite D-175/D-176), rebuilt, and the §6 checklist executed.

GRADUATIONS:
- `tju_359_spin_min` → LIVE pin (open_divergence dropped, GRADUATED finding).
- `scenarios/xfail/cf6002x359.json` → `scenarios/regressions/` (the full fuzz
  case; open_divergence dropped). Verified: NO mechanical name-suppression
  consumed it — fuzz_cep.py has no xfail check (the Rust main-axis fuzz's
  `scenarios/xfail/<name>` check concerns fz_* names only), so the move is
  bookkeeping; the axis was already genuinely clean.
- `scenarios/hang-backlog/pre_existing_temporal_delete_hang.json` (the
  ORIGINAL D-116/D-117 repro) + `spin_c3_delpartner` + `spin_c4_noE1s` +
  `spin_c6_noEP` + `spin_deps_k1` → `scenarios/regressions/` (all live,
  engine==oracle).
- `spin_deps_{extdel,delpartner}` → `probes_pending/cep/tj_upd/
  tju_spin_deps_{extdel,delpartner}.json`, `open_divergence: true` — THE NEW
  OPEN LEDGER of the battery: the D-106 halt-fine-structure corner (post-fix
  they TERMINATE but under-fire RN; oracle ground truth [TJ1,RN,TJ1,RL]
  pinned 3× in the files; cf5x17 certifies the opposite polarity for the
  pre-drain shape — own halt-matrix arc required, the pick stays untouched).
- `scenarios/hang-backlog/` is EMPTY (README rewritten: dispositions + the
  dir's standing charter for future guard-caught cycles).

GATES (all green on the landed tree): corpus **11/1084/339** byte-identical
(6 graduates inside the tier); cargo 9; lint **1613 live / 0 ghosts / 0
inert** (1605 + 6 regressions + 2 relocated ledger pins — exact); fuzz_cep
313/941/943/945 ×400 = 0; **SEINE_TJUPD 6001-6005 ×400 = 0 — the axis flags
NOTHING**; tjt 25/25; tj_upd dir 64 pass + exactly the 2 expected
open-ledger fails (66 files); mju 0/200; notpop 118 + expop 139 fresh ALL
MATCH (seeds 8011/8017); population CONTROL 2,200/2,200; agenda_open ×19
byte-identical to the D-175 validated-fix outputs; bindings rebuilt from the
LANDED tree, 72 passed (release .so, ~4.6MB).

STATE AFTER THIS SLAB: **deliverable (c) is CLOSED — the E1-hardening
non-termination root cause is fixed, not just guarded.** The D-117 guard
STAYS IN as the backstop for cycles not yet met (the hypothetical
all-alive-marked act at its own justifier's post-fire drain remains
reachable in principle; no witness). The battery's open ledger is now
EXACTLY the two halt-corner pins `tju_spin_deps_{extdel,delpartner}` — a
VALUE divergence (one RN under-fire), not a hang, discriminated to the
executor's pre-fire pick-reopening (⚠⚠ D-106; do not hand-patch).

NEXT is Bryan's call. Candidates: the halt-corner arc (the new ledger; needs
a D-106-style halt-matrix over the deps shapes + cf5x17 twins), or the other
deferred items (D-080 TMS envelope, class-3 re-entrant churn, window:length,
Allen-beyond-Drools).

## D-177 — the HALT-MATRIX ARC opened and CLOSED in one move: no pick-reopen exists; ⚖ THE LANDING LAW; the D-176 residual's finding text was a DEFECT (corrected); fix validated-then-REVERTED (Bryan gate) (2026-07-11)

Bryan's charter: characterization, not bug hunt — the deliverable is the
polarity CONDITION, the fix falls out of it. Opening move (Bryan-specified,
"may end the arc"): re-derive cf5x17's polarity on the D-175
salience-staggered observer instrument, both polarities on ONE probe design.
It ended the arc.

### 1. The instrument: the salience INTERPOSER

The pinned spin_deps output [TJ1 | RN, TJ1, RL] was UNDERDETERMINED —
consistent with (M1) the belief drop landing PRE-PICK (RN@9 simply outranks
TJ1@5 at the first pick; no reopen exists) AND (M2) reach-landing plus a
pre-fire pick-reopen. The D-176 residual wrote M2 down as fact. An
INTERPOSER rule at a salience strictly between observer and victim converts
landing time into firing order and splits M1/M2/commit three ways.
Predictions were logged BEFORE every run (scratchpad predictions.md); every
oracle row below is 3×-stable; baseline was green first (corpus 11/1084/339,
ledger pins red-as-expected).

### 2. The cells

| cell | mode/cause | oracle | engine@HEAD | verdict |
|---|---|---|---|---|
| hm1  | stream/external, mark-window open | [TJ1\|RN,R7,TJ1,RL] | [TJ1\|R7,TJ1,RL] | PRE-PICK landing; engine RED |
| hm1b | stream/external, window closed    | same as hm1 | RED | window does NOT gate landing |
| hm2b | stream/RHS delete                 | [TJ1\|K,RN,R6,TJ1,RL] | [TJ1\|K,R6,TJ1,RL] | lands at the deleting rule's FIRE; engine RED (new family member) |
| hm3/hm3b | cloud/external (+tag-scoped obs) | [J2\|R6,J2,(RN),RL] | == oracle | reach landing + COMMIT; GREEN |
| hm4/hm4b | cloud/RHS (+tag-scoped obs)      | [J2\|K,R6,J2,(RN),RL] | == oracle | reach landing + COMMIT; GREEN |

hm3b/hm4b are decisive on the executor: the reach-born RN@9 fires AFTER the
victim's committed head (before RL) — commit, not reopen, engine==oracle.
Every standing pin slots in without exception: q1/q2/q4 + a7c +
spin_deps_expire (expiration ⇒ post-fire/quiescence), a7d + spin_deps_k1
(stream k=1 delete ⇒ action time), the certified plain-TMS corpus (cloud
reach, D-076), D-138 class-3 (the law's existential instance), u4 (the
insert-side mirror). A voided cell (hm2, clock at the expiration boundary)
produced an OFF-ARC find — see §6.

### 3. ⚖ THE LANDING LAW (Bryan's ruling; third sibling at
### docs/tjupd-ledger-mechanisms.md top)

**Delete-sourced teardowns land by MODE × CAUSE. Stream ⇒ at the delete's
propagation (external: at the action; RHS: at the firing). Cloud ⇒ at the
victim's item-reach (D-076). Expiration ⇒ post-fire/quiescence. The executor
NEVER reopens a committed pick — cf5x17 is confirmed, the pick's static
return (~7234) is correct and stays.** TRIAGE: a firing-order divergence
around a delete → identify mode and cause first, then check landing site
against the law, BEFORE hypothesizing anything about the executor. The
contested window (activations born during a k≥2 pop) is EMPTY in Drools'
frame for stream deletes and populated-but-committed in cloud. Axis
post-mortem: axis 2 (engine-frame landing site) was the whole story and
dissolved into Drools-side variables exactly as the arc directive warned;
axis 3 (observer lifecycle) discriminated nothing.

### 4. The D-176 residual correction — a DEFECT, not a typo (Bryan's ruling)
### + ⚖ THE METHOD LAW

The two ledger pins asserted "Drools reopens the pick pre-fire" — FALSE, and
aimed the next session at the caveated D-106 region for no reason. Finding
text corrected in both pins with the why: the output was underdetermined and
one reading got pinned without a discriminating cell. **⚖ METHOD LAW (the
third thing this arc taught, logged beside the two behavioral laws): a pin
is an interpretation of a probe; an underdetermined output is not a finding.
Before pinning a mechanism claim, ask what OTHER mechanism produces the same
output; if one exists, build the splitting cell first or pin only the
output.**

### 5. The fix (VALIDATED THEN REVERTED — re-apply verbatim at the landing)

NOT at the pick (engine.rs:7216/7234 caveated region untouched, per the law
and Bryan's tripwire). The single choke point is `tms_eager_break`'s k=1
scope: the k≥2 exclusion is CORRECT for cloud (min3783/t11/t12 pins) and
WRONG for stream explicit deletes. One predicate + a call-site flag
(from_delete: on_delete_ex ⇒ true, the on_update alpha-fail path ⇒ false —
update-sourced breaks stay lazy, unprobed):

```rust
let stream_del_land = from_delete
    && !self.in_expiration_drain
    && self.event_specs.contains_key(&self.store.fact_type(f));
...
if !self.nets[*ri].path.is_empty() && !stream_del_land {
    return false;
}
```

The landing rides the EXISTING eager machinery (tms_drop_act_deps →
belief retract → cascade queues the observer): external deletes land inside
delete_fact's on_delete_ex (at the action), RHS deletes inside execute_rhs's
Delete arm (at the firing) — one edit, both call sites, k=1 behavior
byte-identical (path.is_empty() short-circuits before the new conjunct
matters). The current_act self-guard stays (fz_42_2442; the fenced
RHS-self-churn case stays fenced). A D-138-style force-eval was considered
and REJECTED: evaluate_rule(force=true) sets tms.defer_mode, which routes
the teardown into tms.deferred (reach landing) — it cannot land the drop.

### 6. Validation (fix in-tree; the full D-176 battery)

corpus **11/1084/339 byte-identical** (make diff rc 0); cargo **9 suites**;
fuzz_cep 313/941/943/945 ×400 = **0**; **SEINE_TJUPD 6001-6005 ×400 = 0**;
tjt 25/25; tj_upd dir **66/66 — both ledger fails FLIPPED**; mju 0/200;
notpop 81 + expop 115 fresh ALL MATCH (seeds 8021/8027); population
2,199/2,200 (model_tjupd_v4, seeds 11..111 step 10 — the 1 = tu81x60,
PRE-EXISTING: baseline-vs-fix engine outputs byte-identical, worktree A/B'd;
structurally inert — the tu population has no insertLogical so tms.by_act is
empty; filed as an open pin, kin of the D-170/171 exit→re-enter family);
bindings --release 72 (fixed tree; the .so reverted with the engine — the
landing rebuilds it); **agenda_open ×19 BYTE-IDENTICAL fixed-vs-reverted**
(the D-106-clean receipt); expected flips all confirmed:
tju_spin_deps_{extdel,delpartner} + hm1/hm1b/hm2b PASS under the fix,
spin_deps_expire + the four cloud cells unchanged. Post-revert: ledger pins
red-again, corpus green with the graduations.

### 7. Filings + state after this slab

- GRADUATED (green both trees, the law's cloud row): scenarios/probes/
  pr_halt_cloud_{extdel,rhsdel}_commit{,_obs} — corpus probes 1084→1088.
- probes_pending/halt/: hm1, hm1b, hm2b (open_divergence, PASS under the
  fix, graduate at the landing).
- probes_pending/flag_eager/fe1_fresh_right_flagged_left.json — the OFF-ARC
  find as ITS OWN ARC (Bryan: not a footnote): the engine pairs a fresh
  right-insert with a flag-expired left-memory partner at a plain join in a
  stream session; oracle refuses (D-102 flag-eager); non-TMS, no executor
  involvement. Arc file `~/.claude/plans/flag-eager-pair-arc.md` (minimal
  cell + boundary-vs-walk discriminator + bisect course).
- probes_pending/cep/tj_upd/tu81x60.json — the population latent (§6).
- The battery's open ledger is now EXACTLY: the two corrected halt pins
  (close at the D-177 landing), fe1 (own arc), tu81x60 (own recon).

LANDING (on Bryan's go): (1) re-apply the §5 diff verbatim → rebuild; (2)
rerun the §6 battery; (3) GRADUATE tju_spin_deps_{extdel,delpartner} +
hm1/hm1b/hm2b → regressions//live pins; (4) rebuild bindings from the landed
tree; (5) D-178 + CURRENT STATE + memory; commit UNPUSHED.

## D-178 — the D-177 LANDING-LAW fix LANDED (Bryan-gated); the five graduated; the halt arc is CLOSED end-to-end (2026-07-11)

Bryan's landing call on D-177. The §5 diff re-applied VERBATIM (git apply of
the captured diff; stat-identical 21+/8−; comments cite D-177), rebuilt, and
the §7 checklist executed.

GRADUATIONS (the five, all → `scenarios/regressions/` live pins,
open_divergence dropped, GRADUATED-prefixed findings):
- `tju_spin_deps_extdel` + `tju_spin_deps_delpartner` (the D-176 ledger,
  finding text already corrected at D-177) — **the last TJUPD-family ledger
  items are CLOSED**.
- `hm1_extdel_interposer` + `hm1b_extdel_nowindow` + `hm2b_rhsdel_interposer`
  (the D-177 stream witnesses). `probes_pending/halt/` is now EMPTY and
  removed.

GATES (all green on the landed tree): corpus **11/1088/344** byte-identical;
lint **1622/0/0**; cargo 9; fuzz_cep 313/941/943/945 ×400 = 0; **SEINE_TJUPD
6001-6005 ×400 = 0**; tjt 25/25; tj_upd dir 64 pass + exactly the 1 expected
open pin (tu81x60, 65 files); mju 0/200; notpop 84 + expop 104 fresh ALL
MATCH (seeds 8031/8037); population 2,199/2,200 (seeds 11..111 — the 1 is
the filed pre-existing tu81x60, same as validation); **agenda_open ×19
byte-identical to the D-177 validated-fix captures** (D-106-clean, third
measurement); bindings rebuilt from the LANDED tree, 72 passed (release
.so ~4.6MB).

STATE AFTER THIS SLAB: **the halt arc is closed end-to-end** — the landing
law + method law standing (docs top), the executor untouched and re-certified
(cf5x17 static return, agenda_open ×3 measurements), the whole
spin_deps/halt-corner family engine==oracle. The battery's open ledger is
EXACTLY: `fe1_fresh_right_flagged_left` (the flag-eager pair — OWN ARC,
`~/.claude/plans/flag-eager-pair-arc.md`) and `tu81x60` (the population
latent, D-170/171 exit→re-enter kin — own recon). The D-117 guard stays in
as backstop. NEXT is Bryan's call — candidates: the flag-eager arc, the
tu81x60 recon, D-080 TMS envelope, class-3 re-entrant churn, window:length,
Allen-beyond-Drools.

## D-179 — the FLAG-EAGER PAIR ARC opened: mechanism PINNED in one sitting — the plain-join walks corpse-check only the PARTNER, never the WALKING fact; fix shape stated, port awaits Bryan's gate (2026-07-11)

Bryan's call: initial work on fe1 (the arc filed at D-177). Course per the
arc file: minimize → boundary-vs-walk discriminator → polarity twin →
mechanism hunt. Predictions logged before every run; all oracle rows
3×-stable; no engine changes.

CELLS (probes_pending/flag_eager/, lint 1628/0/0):
- fe2_minimal — 2 types (E2@expires100, plain P), 1 rule K@7
  `E2(ts==0) P()`; E2(0) at epoch 0; advance 51; advance 50 + P(1) (clock
  101 == deadline 101: flagged, alive). Oracle []; engine [K]. The
  divergence survives minimization.
- fe3_pastdeadline — advance 60 (clock 111 ≫ deadline). Oracle []; engine
  [K] ⇒ **boundary arithmetic RULED OUT** — the flag IS set engine-side
  (advance() marks at clock ≥ deadline); the pairing walk never consults it.
- fe4_swap — `P() E2(ts==0)` (corpse on the join node's RIGHT). Oracle [];
  engine [K] ⇒ both walk directions carry the gap.
- fe6_beta — `E2($t:ts, ts==0) P(v > $t)`. Oracle []; engine [K] ⇒ the
  constraint-bearing scan too (not scoped to cross joins).
- fe5_below (control) — advance 30 (clock 81): BOTH [K]. Cells live.
- fe7_temporal (control) — same deferred-fold structure through
  `E3(after[0,200] $b)`: engine [] == oracle [] ⇒ **the temporal arm does
  NOT share the blind spot; the gap is scoped to the PLAIN arm.**

MECHANISM (SEINE_TRACE/EVAL/FLUSH_DEBUG on fe2 + fe4): the corpse's own
memory-fold is DEFERRED past its flagging — the empty partner side keeps the
path UNLINKED so the corpse accumulates in staging (D-101/fz_7_145) until the
fresh partner arrives; both sides then walk in ONE pop eval. The plain-join
insert walks corpse-check only the PARTNER side: the rightIns walk tests
`l.iter().any(is_expired)` on left partners (phreak.rs ~1930) and the leftIns
walk tests `is_expired(f)` on right partners (~1954) — the WALKING
fact/tuple is never flag-checked. fe2/fe6 = corpse walks as the LEFT
(partner P(1) is plain ⇒ check passes); fe4 = corpse walks as a ph=4
pre-link RIGHT after the flush folded the fresh left (trace-pinned). The
D-102 pin ("a pending-expired event makes NO NEW join pairs") is
partner-complete but walker-blind in the engine's plain arm.

FIX SHAPE (stated, NOT ported — Bryan gate per doctrine): corpse-guard the
partner LOOP of both plain walks (walking right `*f`; walking left tuple
`l`); the memory PUSH stays (flag-eager, retraction-lazy — the corpse
occupies memory until the lazy delete, it only makes no NEW pairs). At port
time: cover/probe the stream-AB arm's twin sites (~595/612, same
partner-only pattern, no witness yet); open sub-cells noted (born-expired
inserts — D-133 adjacency; update-refold walks). Gates for the port: make
diff (⚠ the D-112 eviction-vs-expiration flip-flop zone is adjacent — if a
cf1x-family pin moves, STOP, don't hand-tune), b-ladder, cf11x55/8/19/37,
fuzz_cep ×4 + TJUPD ×5, cargo, lint, fe2/3/4/6 flipping green.

Bookkeeping: fe1's finding refreshed (its engine sequence gained RN
post-D-178 — the correct landing-law composition given K fires; the defect
is unchanged). Arc file `~/.claude/plans/flag-eager-pair-arc.md` updated to
D-179 state. The battery's open ledger membership is unchanged (fe-family +
tu81x60); fe now carries a pinned mechanism awaiting the port gate.

## D-180 — the flag-eager WALKER-GUARD port LANDED (Bryan-gated); the fe arc is CLOSED; the battery's open ledger = EXACTLY tu81x60 (2026-07-11)

Bryan's port call on D-179. The fix, exactly the stated shape: the plain-join
insert walks corpse-check the WALKING fact too — two guards in `phreak.rs
do_join_node`'s plain arm, each AFTER the memory push (flag-eager,
retraction-lazy: the corpse still occupies memory until the lazy delete; it
only makes no NEW pairs):
- rightIns walk (~1934): `if env.is_expired(*f) { continue; }` before the
  lefts_bucket partner loop (fe4's corpse-as-ph=4-pre-link-right);
- leftIns walk (~1959): `if l.iter().any(|x| env.is_expired(*x)) { continue; }`
  before the rights_bucket partner loop (fe2/fe6's corpse-as-left).
The ph=1 update-re-entry walk stays UNGUARDED (no witness — the update-refold
open sub-cell stands); the temporal arm needed nothing (fe7 was already
green). Cloud sessions are byte-identical BY CONSTRUCTION (no clock ⇒
store.expired always empty ⇒ the new conjuncts read false).

GATES (all green, port in-tree): corpus **11/1090/349** byte-identical after
graduation (pre-graduation 11/1088/344 also byte-identical — the b-ladder,
cf11x55/8/19/37 flag-eager pins, and the ⚠ D-112 cf1x eviction flip-flop zone
all HELD; no STOP condition); lint **1628/0/0**; cargo 9; fuzz_cep
313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6005 ×400 = 0; tjt 25/25; tj_upd
64 + exactly tu81x60; mju 0/200; notpop 85 + expop 115 fresh ALL MATCH
(seeds 8041/8047); population 2,199/2,200 (only the filed tu81x60 — the
temporal arm untouched, as designed); bindings rebuilt from the landed tree,
72 passed (release .so). All SEVEN fe cells == oracle: fe1 collapses to
[TJ1, R6, TJ1, RL] (K refused ⇒ no RHS delete ⇒ no RN — the whole composed
sequence matches), fe2/3/4/6 = [] (now expect_inert live pins), fe5 [K] and
fe7 [] controls unchanged.

GRADUATIONS: fe1/fe2/fe3/fe4/fe6 → `scenarios/regressions/` live pins
(fe2/3/4/6 with `expect_inert` — deliberately empty post-fix);
fe5→`scenarios/probes/pr_fe_below_boundary`, fe7→`pr_fe_temporal_arm`
(control pins). `probes_pending/flag_eager/` is EMPTY and removed; the arc
file marked CLOSED.

STATE AFTER THIS SLAB: **the flag-eager arc is closed** (D-102 mechanism 4 is
now walker-complete on the plain arm). **The battery's open ledger is
EXACTLY `probes_pending/cep/tj_upd/tu81x60.json`** (the population latent,
D-170/171 exit→re-enter kin — own recon). Open sub-cells noted for future
arcs, not ledgered: born-expired inserts (D-133 adjacency), update-refold
walks (ph=1), the stream-AB arm's partner-only sites (~595/612 — no witness;
fe7 suggests the temporal path guards elsewhere). NEXT is Bryan's call —
candidates: tu81x60 recon, D-080 TMS envelope, class-3 re-entrant churn,
window:length, Allen-beyond-Drools.

## D-181 — tu81x60 CRACKED: the LINGERING-DEL relink flavor (⚖ identity-model law instance #3); the D-171 exemption widened for temporal nodes; fix validated-then-REVERTED (Bryan gate) (2026-07-11)

Bryan's call: the tu81x60 recon (the last ledger item). Predictions logged
pre-run; all oracle rows 3×-stable; the fix validated then reverted.

### 1. The fork, then the minimal

The MODEL predicts the oracle on the case (regeneration verified byte-equal;
simulate = ['15z|38z','80z|38z'] == oracle 3×) ⇒ the D-169 spec has NO gap —
an ENGINE SEAM. Minimal (tu_x60_min1, 5 ingredients): E1(59,z); one epoch
[exit tag→y, re-enter tag→z+ts15] + fresh E0(38,z). Oracle [RJ(15,38)]
(frozen ts0=59: 59−38=21 ∈ [0,50]); engine [] — the only pair lost.

### 2. Discriminators (each 3×)

- c_no_tswrite RED ⇒ the ts write is NOT load-bearing.
- c_noexit GREEN ⇒ the EXIT is required (the in-place D-170 family is fine).
- c_linked GREEN ⇒ **LINKED-NESS is the load-bearing axis** (an inert
  resident E0 flips it green — a linked path's flush evals drain the del).
- c_split RED ⇒ **same-epoch-ness is NOT required**: the exit's del LINGERS
  across a fire boundary (nothing drains unlinked staging, fz_7_145) — the
  CROSS-EPOCH flavor, outside D-171's same-flush exemption.

### 3. The mechanism (SEINE_TRACE on min1 + c_split) — ⚖ identity law #3

The pair IS created: at the linking E0's flush the temporal node leaves the
pre-tail re-entry INS visible (temporal joins stash no lefts, D-102) and the
eval derives [f0,f1]; the terminal queues it. The exit's s0-DEL — staged
EARLIER in stage order — was dstash-hidden ("pre-tail DELS stash at ALL
nodes") and drains at the POP, where the value-keyed kill destroys the fresh
pair. Drools' object-keyed delete would kill only the OLD (childless) tuple.
The D-171 exemption at engine.rs ~5807 keys on `fresh_ins` (ins staged since
THIS flush's snapshot) — the re-entry staged by an earlier ACTION (min1) or
EPOCH (c_split) is pre-tail and invisible to it. Ins visible + del hidden =
the inversion; the stale del made lethal by value-keying — the law verbatim.

### 4. The fix (VALIDATED THEN REVERTED — re-apply verbatim at the landing)

One predicate widening at the D-171 site: at TEMPORAL nodes scan ALL pending
s0-ins for the same-fact re-entry (their pre-tail ins is visible to the eval,
so the del must stay visible with it); PLAIN joins keep the fresh-only scan
(their pre-tail lefts are stashed — del-batching unchanged).

```diff
diff --git a/engine/src/engine.rs b/engine/src/engine.rs
index f598992..85690f9 100644
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -5805,9 +5805,18 @@ impl Engine {
             // OBJECT identity — destroys the re-created children
             // (tu51x80/tu51x187: the relink SET losses).
             let fresh_ins = t.s0_in.ins.len() - p.0.min(t.s0_in.ins.len());
-            if fresh_ins > 0 && !s0_dtail.is_empty() {
+            // D-181 (tu81x60 lingering-del, identity-law instance #3): at a
+            // TEMPORAL node the pre-tail same-fact ins is VISIBLE to this
+            // eval (temporal joins stash no lefts), so a LINGERING del —
+            // staged by an earlier action or epoch on the then-unlinked
+            // path, never drained — must stay visible with it, or it
+            // drains at the pop AFTER the re-entry pair derives and kills
+            // it by value. Plain joins stash pre-tail lefts too, so their
+            // fresh-only scan keeps the certified del-batching.
+            let scan_n = if t.node.temporal { t.s0_in.ins.len() } else { fresh_ins };
+            if scan_n > 0 && !s0_dtail.is_empty() {
                 let fresh: Vec<FactId> =
-                    t.s0_in.ins[..fresh_ins].iter().map(|(f, _, _)| *f).collect();
+                    t.s0_in.ins[..scan_n].iter().map(|(f, _, _)| *f).collect();
                 let mut keep: Vec<(FactId, Origin, u8)> = Vec::new();
                 s0_dtail.retain(|e| {
                     if fresh.contains(&e.0) {
```

### 5. Validation (fix in-tree; ALL green)

corpus 11/1090/349 byte-identical; cargo 9; **tj_upd 65/65 — tu81x60 FLIPPED,
the dir fully green for the first time**; tjt 25/25; mju 0/200; fuzz_cep
313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6005 ×400 = 0; **population
2,200/2,200 — FULLY CONVERGED on the 11-seed set for the first time** (the
seed-81 batch 200/200); notpop 87 + expop 122 fresh ALL MATCH (8051/8057);
bindings 72 (fixed tree; .so reverted with the engine); all 7 recon cells ==
oracle under the fix (min1/min2/c_no_tswrite/c_split flip; c_linked/c_noexit
unchanged); post-revert tu81x60 red-again ([RJ(80,38)]).

### 6. Filings + the landing checklist (Bryan gate)

Committed NOW (fix reverted): tu_x60_{min1,min2,c_no_tswrite,c_split}
(open_divergence — PASS under the fix, graduate at the landing) +
tu_x60_c_{linked,noexit} (green controls) in probes_pending/cep/tj_upd/;
tu81x60's finding → CRACKED. Lint 1634/0/0.

LANDING (on Bryan's go): (1) re-apply the §4 diff verbatim → rebuild; (2)
rerun the §5 battery; (3) GRADUATE tu81x60 + the four red cells →
regressions/ live pins, c_linked/c_noexit → probes/ control pins; (4)
rebuild bindings from the landed tree; (5) D-182 + CURRENT STATE + memory;
commit UNPUSHED. After that landing THE BATTERY'S OPEN LEDGER IS EMPTY.

## D-182 — the D-181 lingering-del fix LANDED (Bryan-gated); the tu81x60 family graduated; **THE BATTERY'S OPEN LEDGER IS EMPTY** (2026-07-11)

Bryan's landing call on D-181. The §4 diff re-applied VERBATIM (the temporal
scan_n predicate at the D-171 exemption site; comment cites D-181), rebuilt,
and the §6 checklist executed.

GRADUATIONS: `tu81x60` + `tu_x60_{min1,min2,c_no_tswrite,c_split}` →
`scenarios/regressions/` live pins (GRADUATED findings);
`tu_x60_c_linked`→`scenarios/probes/pr_tux60_linked`,
`tu_x60_c_noexit`→`pr_tux60_noexit` (control pins). The tj_upd dir is back
to 64 files, ALL green.

GATES (all green on the landed tree): corpus **11/1092/354** byte-identical;
lint **1634/0/0**; cargo 9; **tj_upd 64/64**; tjt 25/25; mju 0/200; fuzz_cep
313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6005 ×400 = 0; **population
2,200/2,200 — all 11 seeds fully converged**; notpop 83 + expop 117 fresh
ALL MATCH (seeds 8061/8067); bindings rebuilt from the LANDED tree, 72
passed (release .so).

STATE AFTER THIS SLAB: **THE BATTERY'S OPEN LEDGER IS EMPTY** — for the
first time since the ledger existed, every filed divergence in the battery
is either fixed-and-graduated or explicitly fenced-by-nature (D-134 §6
heap ties, fz_42_84 identity-hash-order). The identity-model law carries
three closed instances (c13 Staged no-fold, D-171 relink, D-181
lingering-del). Remaining known-open surfaces are SUB-CELLS noted in their
entries, not ledgered divergences: born-expired inserts (D-133 adjacency),
update-refold ph=1 walks, the stream-AB partner-only sites, the D-175
hypothetical all-alive-marked act (D-117 guard stays as backstop for it).
NEXT is Bryan's call — candidates: D-080 TMS envelope, class-3 re-entrant
churn, window:length, Allen-beyond-Drools, or probing the noted sub-cells.

## D-183 — the WINDOW:LENGTH ARC OPENED: Bryan's scope rulings recorded; wl-ladder rungs 1-4 PINNED in one sitting (9 cells, one landed-surface graduation); rungs 5-7 + the model phase remain (2026-07-11)

### 1. Scope rulings (Bryan, 2026-07-11)

(a) **ACC-SOURCE-ONLY** — `window:length` enters the subset on accumulate
sources only; standalone-pattern windows split to their own P3 FEATURES row
and STAY WALLED (the natural parse wall at `over`). (b) **TMS×window stays
FENCED in-arc; D-080 remains its own arc and this arc is NOT deferred behind
it.** (c) (unopposed) the `CepWindowTest` length subset via the drift-checked
extraction pipeline is the spec floor. FEATURES §2 updated (the bundled row
split); handoff `~/.claude/plans/window-length-arc.md` §1 marked RULED.

### 2. wl-ladder rungs 1-4 (oracle 3× per cell; predictions logged pre-run;
### the engine wall drl.rs ~705 verified held — all length cells engine_fenced)

- **POST-ALPHA window** (wl_s1): an alpha-failing insert consumes NO slot
  and causes no re-fire; sums [0,1,3,6].
- **Independent windows per (type,N)** (wl_s2): N=2/N=3 no cross-talk.
- **Eviction+admission = ONE NET re-fire, in-epoch by salience** (wl_t1,
  interposer instrument): [R7@7, W(6)@5, RL@1] — no intermediate evict-only
  fold. Multi-evictions in one epoch batch to one fire (wl_t2: 12).
- **Slot order is INSERT-FIXED** (wl_u1): a value-relevant update re-folds
  (bound field — the D-139 bindings-watch analog) but never re-admits; the
  updated event still evicts first. The D-141 insert-fixed parallel.
- **REVIVAL** (wl_u2 + the time twin): an external update of an EVICTED
  event re-enters the fold — length: {50,2,3}=55, THREE events in an N=2
  window; time (pr_wl_time_revival, GREEN engine==oracle, GRADUATED): 52.
  Both window kinds agree in the pure-eviction shape; the D-137c2
  clock_removed suppression scopes to a different composition (its pins
  stay green — measured before filing, per the method law: the near-filing
  of a false divergence died to one engine run). Post-revival (wl_u2b) the
  revived event persists OUTSIDE the ring: the next admission evicts only
  the oldest ring slot ({50,3,4}=57). Structure tentative until the model.
- **NO backfill on delete** (wl_d1): deleting an in-window event shrinks
  the window; the evicted event does not return.

### 3. State + next

probes_pending/cep/winlen/ holds the 8 engine_fenced pins;
pr_wl_time_revival graduated (corpus probes 1093); lint **1643/0/0**;
corpus otherwise untouched, engine untouched. REMAINING before the port
gate: rung 5 (expiration×length — ⚠⚠ the D-112 flip-flop zone: model-first,
never probe-grind), rung 6 (D-139 discriminator analogs), rung 7
(batch/degenerate N + entry points), then the §3 model extension
(model_check_winacc length dimension, 0-div on ladder + population) → the
§4 port map (note added: the length eviction must ride the same
non-suppressing path the landed time eviction uses for revival) → Bryan's
port gate.

## D-184 — wl-ladder rungs 5-7 PINNED; the population peel CORRECTED two rung-1-4 interpretations (⚖ method law, self-applied); the winlen MODEL is 0-DIV; THE PORT IS AT BRYAN'S GATE (2026-07-11)

### 1. Rungs 5-7 (oracle 3×; e3 5×; predictions pre-logged)

- **Expiration×length** (e1/e2/e3): a single expiration drops the member
  from the fold at the advance epoch; the coincident single-deadline
  expiration+eviction+observer epoch is **5×-STABLE, one net fire by
  salience** — no D-112-style flip-flop on this surface.
- **Property reactivity** (p1/p2/p3 — the D-139 analogs): the windowed
  watch mask = source BINDINGS only, confirmed for length (no-binding
  count() never re-folds on writes; bound-but-fn-unused re-fires at equal
  value); membership changes re-fire at equal value; **revival requires a
  mask write** (a no-mask write on an evicted event does nothing).
- **Degenerates** (b1/b2/b3/b4): batch overfill = one net fire; N=1 fine;
  **N=0 THROWS in Drools (ArithmeticException) — out of subset, the port
  rejects N<1 at parse**; entry-point routing folds normally (clause order:
  `over` before `from entry-point`).
- **Alpha transitions via update** (x1/x2): entry ADMITS; exit drops from
  the FOLD even though tag is unbound — constraint transitions act
  regardless of the bindings mask.

### 2. The population peel — two D-183 interpretations CORRECTED
### (⚖ method law, self-applied: fold-drop observables were slot-ambiguous)

The first population (450 cases) diverged ~7% and the peel discriminated
what the hand cells could not:
- **SLOT RETENTION** (sr1/sr2 in the model ladder): a deleted, alpha-exited,
  or expired member leaves the FOLD but its ring SLOT persists — it still
  counts toward N and still evicts live members on later admissions. Every
  D-183 "slot freed" reading was underdetermined; the fill-beyond
  compositions split it. d1/e1/e2 outputs re-derive identically under
  slot-retention (verified — ladder green both before and after).
- **SAME-SLOT RE-OCCUPATION** (sr4): an exit-then-re-enter returns the
  event to its RETAINED slot (it evicts first), not a new one; only a
  never-slotted entry appends.
- **TWO FENCED TRICKLE CORNERS** (witnesses banked engine_fenced):
  wl_f1_multi_deadline_trickle — two deadlines crossed by one advance + a
  same-epoch insert: the drops TRICKLE across separate fires (the D-112
  zone transposed to length); wl_f2_born_expired_trickle — a born-expired
  insert folds at its epoch then drops as its own later fire (the D-133
  adjacency). Both FENCED from the generator (one advance per scenario in
  an actions-only epoch; ts floor after advances) and deferred to the
  WindowNode sub-recon with the time-window family.

### 3. The executable spec: model_winlen.py is 0-DIV

`probes_pending/cep/winlen/model_winlen.py` — slot-retention ring, fold =
passing slot-occupants ∪ revived, bindings watch mask, re-occupation,
lazy-free expiration (single-deadline), one-net-fire-per-touched-epoch +
the initial fire. **Ladder 19/19** (the 15 hand cells + sr1/sr2/sr3/sr4)
and **0-div on 7 population seeds (~1,050 fresh cases; 601-605, 701-702)**
under the two documented fences. The generator: N∈{1,2,3}, sum/count,
tag-transition updates, deletes, single advances, 0-2 facts × 2-3 epochs.

### 4. THE PORT IS AT THE GATE (nothing engine-side touched; lint 1657/0/0)

Port map (refined by the peel): `drl.rs parse_window_opt` Window::Length(n),
N≥1 enforced, acc-source only; engine `window_len` beside `window_time`
with a SLOT-RETENTION ring per window node (occupants may be corpses;
eviction pops the oldest SLOT; re-occupation on re-entry); revival rides
the SAME non-suppressing path the landed time-window revival uses
(pr_wl_time_revival); the D-139 windowed-ness gate widens to `window_len`;
@expires-inference no-contribution already landed (~5006, pin at port);
expiration×window keeps the lazy single-deadline shape, the trickle corners
stay fenced (xfail-class witnesses wl_f1/wl_f2). Gates: the standing
battery + the wl ladder/population via model_winlen fuzz + df_*/cf1x pins
(⚠ do-not-hand-tune stands) + the CepWindowTest length baselines (§1c) +
the 22 winlen cells flipping from engine_fenced to live. Bryan's go
starts the port.

## D-185 — the WINDOW:LENGTH PORT LANDED (Bryan-gated): the wall lifts on accumulate sources; TWO landing-law inversions in the acc machinery found-by-population and fixed; the wl arc is CLOSED (2026-07-11)

Bryan's port call on D-184. The arc's rulings (D-183 §1) implemented in full.

### 1. The port

- `drl.rs`: `Window::Length(n)` + the wall at parse_window_opt lifts for
  accumulate sources; **N ≥ 1 enforced** with a clean error (N=0 throws in
  Drools — wl_b3_n0 stays the engine_fenced WALL GUARD, the only file left
  in probes_pending/cep/winlen/ beside the model).
- `engine.rs`: `CompiledAcc.window_len`; the SLOT-RETENTION ring
  (`TrieNode.win_ring`) — admission appends (insert walk +
  `winacc_step`'s never-admitted entry, NOT revivals), overflow pops the
  OLDEST SLOT via `stage_acc_removal` (detach; revival stays mask-gated —
  the same non-suppressing path as landed time windows); the seven
  windowed-ness gates widened (`window_time.is_some() || window_len...`):
  shadow exclusions ×3, the acc-entry drain router, the on_update router,
  the D-139 bind-fields eff_mask, the insert walk; the node-share key
  gains `window_len` (wl_s2 independence); `acc_nodes` carries it so
  length-windowed accs SKIP eager expiration removals (lazy through the
  window, like time). The @expires-inference no-contribution was already
  landed (D-110).

### 2. Two landing-law inversions, found by the population (the port's
### hand cells all passed FIRST BUILD — the population caught what they
### could not)

Engine-vs-oracle over the winlen population initially failed ~6.8%:
- **Eviction side** (pr_wl_evict_pending_upd, was wl603x23: 42 vs 3): an
  external upd DEFERS to the D-160 entry drain; an immediate walk-time ring
  pop landed BEFORE it, turning the pre-eviction update into a spurious
  mask-hit revival of the just-evicted member.
- **Admission side** (pr_wl_entry_slot_position, was wl603x54: 104 vs 15):
  a deferred-entry ADMISSION slotted at drain time (last) instead of its
  FIFO position (before the epoch's walk inserts), surviving evictions it
  should take.
FIX (one mechanism): when deferred entries pend, walk-time RING OPS defer
to `win_admit_pending` and land at the END of `drain_acc_pending` — true
action order: entries first at their FIFO positions (their ring ops run
mid-drain with acc_pending already taken ⇒ immediate), then the walk
admissions in arrival order. Pure-insert epochs (acc_pending empty) keep
immediate ring ops — byte-identical to the first build's green cells.

### 3. The spec caught its own corner (⚖ method law, both directions)

model_winlen found wl901x72: deferred update entries evaluate against
EPOCH-FINAL fields (the D-160 doctrine verbatim) — a same-epoch
exit-and-back transient is INVISIBLE. The model was restructured to the
engine's own shape (explicit ACTIVE set vs final-alpha arms) —
pr_wl_transient_exit pins it. Final spec state: **ladder 21/21; 0-div ×5
fresh seeds (750 cases)**.

### 4. Gates (all green on the landed tree)

corpus **11/1124/354** byte-identical; lint **1662/0/0**; cargo 9;
**winlen population 2,695/2,700 engine==oracle** (the 5 = OLD-generator
invalid scenarios where BOTH sides error — update-of-expired: engine
dead-handle vs oracle NPE — mutual refusal, not divergence; every
new-generator case passes); **all 5 D-110-era fenced recon cells passed
UNCHANGED and graduated** (pr_wl_d110_* — cells written 75 D-entries ago,
the lint's fence-regression guard surfaced them); winacc TIME-window
population spec 150/0 (shared paths undisturbed); fuzz_cep ×4 = 0; TJUPD
6001/6003/6005 ×400 = 0 (full ×5 = 0 earlier in the cycle); tu population
spot 800/800 (full 2,200 clean earlier in the cycle; no accs in that
population); tjt 25/25; tj_upd 64/64; mju 0/200; notpop 85 + expop 111
fresh ALL MATCH (8081/8087, post-fix); bindings 72 (landed-tree release
.so). GRADUATIONS: 21 wl cells + x3/x3t + the 5 D-110 cells + the 3
fix-witness pins → scenarios/probes/ (32 new pins incl.
pr_wl_time_{revival,reentry}).

### 5. State + follow-ups

**window:length(N) on accumulate sources is IN THE SUBSET** (FEATURES §1
row to flip from ARC OPEN at the next touch). Follow-ups, not ledgered:
the CepWindowTest length-subset baseline adoption (§1c — the drift-checked
extraction pipeline, its own sitting); the generator axis merge
(SEINE_WINLEN into fuzz_cep — after the baselines); the WindowNode
sub-recon (the D-112/D-133 trickle corners — the ENGINE matches the oracle
on both banked witnesses (pr_wl_f1/f2 graduated LIVE), so the sub-recon is
spec-side only); standalone-pattern windows stay P3-walled. The battery's
open ledger REMAINS EMPTY. NEXT is Bryan's call.

## D-186 — the D-080 TMS ENVELOPE ARC OPENED: laws-first scoping COMPLETE — fresh re-baseline (3 moves, both movers = evaluation-lifecycle commits), the law-read buckets, THE RESIDUE stated; no engine change (2026-07-11)

Bryan's charter: open with the laws, not a witness — the arc starts from
an envelope, so the opening move is scoping, and its deliverable is the
RESIDUE (what the standing laws do NOT explain). Full read:
`~/.claude/plans/tms-envelope-arc.md`; recon home
`probes_pending/tms_envelope/` (fresh baseline table banked).

### 1. Baseline (gates green first: corpus 11/1124/354 byte-identical,
### lint 1662/0/0 on HEAD 687b8ae, clean tree)

Fresh `tools/triage_xfail.py --runs 10` into a NEW cache
(`target/triage_cache_d080arc`; the D-087 cache preserved). The envelope
= exactly the 68 TMS witnesses, byte-unchanged files since D-087.
Results: ZERO classification changes (45 VALUE + 22 RUNAWAY + 1 NONDET),
zero graduation candidates, zero unfenced divergences; the oracle is
BYTE-STABLE on all 68 across the D-163 +p1 vendor (10/10 replicates).
Mechanical table diff vs D-087: exactly THREE rows moved, all
ENGINE-side, worktree-bisected (predicate = firing counts, main-tree
witness files for constant identity):

- fz_42_4442 (SJ runaway) engine 6→4 firings at **D-091 `f70b189`** —
  the RuleExecutor dirty-flag lifecycle port (whose own entry recorded a
  TMS-drain fallout; this is the same coupling's un-gated shadow).
- fz_123_2674 (A,B) −0/+4 → −0/+3 and fz_42_7619 (A,B) −0/+5 → −1/+4 at
  **D-101 `bb6eb6d`** — the drain-at-link / lazy-teardown slab.

THE SCOPING DATUM: across ~110 commits and ten landed arcs — including
two TMS-law fixes (D-172/D-181 identity, D-177 landing, all
temporal/stream-gated by design) — the envelope moved ONLY when the
executor's EVALUATION-LIFECYCLE model moved. The xfail set sits outside
the byte-identical gate; these moves were silent until this re-baseline.

### 2. The law-read (shape-level; candidate classifications, NOT pins —
### method-law discipline: splitting cells before any mechanism claim)

The 45 VALUE witnesses bucket by fence marker × fresh delta direction
(full per-witness lists in the plan §3):

- **L-SD, self-defeat-cause landing ×13** (pure SJ): **11/13 engine
  UNDER-fires** — the park lands the drop earlier than Drools' lazy
  drain; the sibling's glimpse is lost. Kill-cells for any single-row
  hypothesis: the two OVER outliers fz_123_3060 / fz_7_9375.
- **L-MB, mutation-break-cause landing ×18** (A-marked, incl. 2 XD):
  **16/18 engine OVER-fires** — on compounds the engine lands
  justifier-mutation belief-drops LATER than Drools' eager path (the
  certified k=1-scope/own-origin pins all pass; the compounds sit past
  that boundary).
- **I-RD, mixed-key kill path ×12** (RD): direction MIXED 6/6, two with
  fact deltas — Drools immediateDelete-vs-staged-cancellation naming
  OBJECTS vs the engine's value-key; identity-law home turf with a
  landing component.
- **I-ST, static bookkeeping ×1**: fz_7_9902 — firings byte-identical,
  oracle retains one extra stated duplicate. NO ordering component.
- **Compound ×1**: fz_7_9550 (A,SJ).
- **Family II ×22 runaways**: the identity law's MIRROR — object-identity
  belief churn sustains Drools' cascade, the engine's value-merge + park
  terminates. Explained, NOT actionable (no stable oracle answer);
  fenced-by-nature stays. Family III ×1 nondet: fenced-by-nature stays.

Law NON-applications stated early: the dedup/side-effect law's signature
dominates no bucket (per-cell checklist only); the identity law's
ordering clause is the wrong polarity for L-MB's over-direction (stays a
per-cell check in I-RD and the two L-SD outliers).

### 3. THE RESIDUE (the arc's actual work)

- **R1 — the (cloud × belief-loss) landing rows.** The landing law's
  mode×cause table has no rows for self-defeat or justifier-mutation
  teardowns, and the two clusters' direction-coherence (11/13 under,
  16/18 over) says a table EXISTS. This is the fifth-law-shaped hole;
  §1's attribution locates its machinery — the evaluation-lifecycle
  (dirty/link/drain/halt) discipline, ⚠ D-106-adjacent. The D-177
  pattern governs any approach (interposer instrument; the pick is
  never the defect; agenda_open ×19 byte-identical receipts mandatory).
- **R2 — the static stated/justified bookkeeping model.** The
  9902-class sits OUTSIDE the identity law as written (no ordering
  deferral to restore). Law refinement or D-077 model completeness;
  store-layer, D-106-clean; TmsDump-graft ground truth first.

Probing order (plan §5): (1) L-SD interposer ladder on the min812 +
fz_123_9133 spines; (2) the L-MB eager/lazy boundary ladder on the
k/origin/property-hit axes; (3) I-RD TmsDump graft on 4048/9902 before
cells; (4) model + DEDICATED arc fuzzer population (gen.rs walls STAY
UP) on fresh out-of-sample seeds; (5) any port validate-and-revert
behind Bryan's gate. If the cells split with no coherent table, the
honest close is fe1-shaped: a tighter fence statement, no law.

### 4. Filings

`probes_pending/tms_envelope/` (README + triage-2026-07-11.md);
`~/.claude/plans/tms-envelope-arc.md`; this entry; CURRENT STATE.
Engine untouched; corpus untouched; no wall moved; the D-087
`docs/xfail-triage.md` kept as the historical record. NEXT: plan §5
rung 1 unless Bryan re-orders.

## D-187 — the XFAIL ENGINE-DRIFT GATE landed (Bryan-directed) + L-SD rung 1: the (cloud × self-defeat) LANDING ROW PINNED, with the eagerness split that reconciles min608 (2026-07-11)

### 1. The gate (Bryan: "fix the xfail gate")

`tools/xfail_drift.py` + `make xfail-drift`/`xfail-rebank`; `make diff`
grew a fourth tier ("xfail engine-drift"). Mechanism: the quarantine
stays excluded from the ORACLE diff by design (its witnesses diverge —
that is their finding), but the ENGINE's canonical output on every
witness is now gated against a committed snapshot
(`scenarios/xfail-engine-baseline.ndjson`, banked at HEAD post-D-185).
Movement fails the gate and must be deliberate: re-triage
(tools/triage_xfail.py) → `make xfail-rebank` → D-entry. Comparison is
D-003-canonical via triage_xfail's canonicalizer (serializer churn
cannot trip it; semantic movement always does); set drift
(added/removed witnesses without rebank) also fails. LIVENESS PROVEN:
banked 75 → green; corrupted one fz_7_9902 entry → RED naming the
witness ("firings 13 -> 14"), rc=1; rebanked → green; composed
`make diff` green with the tier riding. The bank's location
(scenarios/ root) is invisible to every existing consumer (Makefile
finds target subdirs; lint scans probes/regressions/duckdb/
probes_pending; fuzz suppression checks scenarios/xfail/{name}.json;
triage globs scenarios/xfail/*.json). Closes the D-186 §1 exposure
(D-091/D-101 moved the quarantine silently for five days).

### 2. Rung 1 — 14 cells, 3 designed rounds, all oracle rows 3×-stable;
### predictions logged before every round (the round-1 lead hypothesis
### FALSIFIED by its own grid; rounds 2-3 landed 7/7 exact)

**⚖-candidate ROW (the (cloud × self-defeat) landing table, entry 1):
a LAZY justifier's self-defeat belief drop lands at the justifier's
ITEM POP — same-salience observers glimpse the transient iff their
queue position PRECEDES the justifier's item (declaration order in
same-firing-born shapes); an EAGER justifier (no-loop/dyn-salience)
lands the drop at the firing's eager-flush — no ≤-salience glimpse,
queue position irrelevant; strictly-higher always glimpses (t11),
strictly-lower never; k irrelevant.** The ENGINE lands the lazy case
early (continuation-time) uniformly → it under-fires exactly the
observers declared before a lazy justifier at equal salience — which
IS min812's mechanism (a10), whose accumulate was never load-bearing
(a9: a plain observer diverges identically; a5: acc-after-justifier is
green). Decisive cells: **a13** (two IDENTICAL plain observers at equal
salience split exactly at the justifier's declaration slot — the drain
occupies the justifier's queue position) and **a14×a15** (no-loop
kills the glimpse at k=0; k=1-lazy still glimpses ⇒ EAGERNESS is the
split, not k). Four open_divergence cells filed: a9/a10/a13/a15.

**The min608 reconciliation (⚖ method law applied to a standing pin):**
D-076 drain point (a)'s "equal salience/earlier decl does NOT preempt
(min608 vs t11)" was pinned on fz_7_608, whose justifier carries
NO-LOOP — what it actually pinned is the EAGER flush regime; the
"continuation drains at equal salience" reading over-generalized to
lazy justifiers and the engine implemented the over-generalization.
Both standing pins hold within their real scope (a14 reproduces
min608's row; a3/a6/a12 reproduce t11's); the lazy-equal case was
never discriminated until a9/a13. Underdetermined fine print stated in
the results doc (queue-position vs decl-order realization; the
engine-side code path deliberately NOT pinned from cells — port-phase
work, ⚠ D-106 tripwire + the D-177 landing-not-pick pattern apply).

### 3. Gates + filings

lint **1676/0/0** (14 new cells all live; 4 open_divergence markers);
`make diff` green ×2 this sitting (pre-gate baseline + composed with
the new tier); corpus untouched **11/1124/354**; engine untouched.
Filings: `probes_pending/tms_envelope/` sd_a2..a15 +
rung1-predictions.md (pre-run, all three rounds) + rung1-results.md
(the row, the grid, the reconciliation). The row RETRODICTS the L-SD
bucket's 11/13 under-fire direction; the two OVER-fire outliers
(fz_123_3060, fz_7_9375) are NOT explained by it — rung 2's
kill-cells, alongside the fz_123_9133 fan-out spine (multi-activation
justifier × in-firing continuation). NEXT: rung 2, or Bryan's
re-order; any engine change stays validate-and-revert behind the gate.

## D-188 — RUNG 2 (Bryan's sequencing): the over-fire outliers DISSOLVE into a second clause, the sweep's misfits into a third — the L-SD row is a THREE-CLAUSE TABLE and ALL 13 BUCKET MEMBERS ARE ACCOUNTED FOR (2026-07-11)

Bryan's bar: "a table that explains 11/13 and can't explain 2 is not a
table yet." Executed: cache trace-reads (10-replicate sequences, free)
of 9133/3060/9375 → 9 constructed cells over two prediction rounds
(pre-logged; all oracle rows 3×-stable) → a mechanical 13-witness
retrodiction sweep → trace-reads of the two sweep misfits. Full write-up
`probes_pending/tms_envelope/rung2-results.md`.

**THE (cloud × self-defeat) ROW, complete — three clauses:**
- **A (landing/queue-position, rung 1):** lazy ⇒ drop lands at the
  justifier's ITEM POP, same-salience observers glimpse iff queue
  position precedes it; eager ⇒ flush landing; an item fires its WHOLE
  tuple list at its pop (sd_b7: join observer ⇒ [RJ,RO,RO,RO] — the
  9133 sequence from a 2-rule cell).
- **B (in-firing self-cancellation — the over-fire outliers):** the
  justifier's OWN remaining same-item tuples (fan-out AND or-twin
  branches) die IN-FIRING at the self-break, both regimes. sd_b2
  (leading-not, = 3060) and sd_b4 (or-twin no-loop, = 9375) RED; sd_b1
  green — the ENGINE's cancellation works trailing-not but fires the
  corpse tuples on leading-not/or-twin topologies. The "over-fire
  outliers" were clause-B violations, not counterexamples to A.
- **C (post-drop re-derivation — the sweep's justifier-under-fire
  misfits):** no WM change ⇒ NO refire (sd_c2 green; t10's scope
  confirmed exactly); a left-side WM change (t15 revive) re-derives the
  remaining tuples and the re-queued item competes at its salience —
  strictly-higher preempts the changer after ONE firing ⇒ STRICT
  ALTERNATION (sd_c1: oracle [RJ,RD]×3, = 5213's ten pairs; the D-091
  halt structure). The ENGINE batches the changer and starves refires.

**Bucket accounting (13/13):** 8 pure-A (sweep-verified), 9133 = A+B
(B honored: trailing-not), 3060 = B, 9375 = B, 5213 = A+B+C, 1353 =
A+B+cascade-persistence (the bootstrap glimpse's own insertLogicals
persist per the ordinary D-076 lifecycle — no fourth clause; its
8-firing/5-fact loss is the persistence chain the engine's early drain
never starts). ZERO unexplained members.

**Bonus boundary find (sd_b3):** the bare LAZY or-twin self-justifier
is a genuine Drools RUNAWAY (fire-limit 3/3) — the fz_42_946 family as
a 1-rule constructed minimal; `no-loop` is exactly what makes 9375's
or-twin terminate. The terminate/runaway boundary is now a designed
pair (b3/b4), not just fuzz census.

**Engine gaps, stated (all evaluation-lifecycle; ⚠ D-106 tripwire +
D-177 landing-not-pick apply to any future fix):** (1) early
continuation-drain instead of pop-landing [A]; (2) topology-dependent
in-firing cancellation miss [B: leading-not, or-twin]; (3)
changer-batching instead of halt-alternation [C].

GATES: lint **1685/0/0** (9 new cells: 7 open_divergence incl. the b3
boundary witness, 2 live controls); corpus untouched 11/1124/354;
engine untouched; xfail drift gate green (nothing rebanked). Cell
census for the arc so far: 23 constructed cells, 11 open_divergence
witnesses = the L-SD port battery core. Method-law fine print (clause
B's exact fold site; clause C's equal-salience re-queue case c3 —
unprobed, one cell at the model phase) recorded in the results doc.
NEXT: L-MB (the 18-witness mutation-break cluster) per plan §5, or
L-SD model-phase consolidation first — Bryan's call.

## D-189 — CONSOLIDATION (Bryan-directed): c3 falsified the halt formulation → THE QUEUE-HEAD DISCIPLINE; the model + dedicated fuzzer live; ⚖ Bryan's EPICYCLE STOP mid-arc; the MEMBER-ORDER GRAFT phase 1 — deterministic mechanics, not texture (2026-07-12)

Bryan's ruling after D-188: consolidate before L-MB — "the table has
never been tested out-of-sample." Executed, and the out-of-sample
testing did its job THREE times over.

### 1. c3: the equal-salience re-queue cells (theory-derived predictions)

c3b/c3c hit exactly; **c3a FALSIFIED the strictly-higher halt
formulation** (oracle alternates at EQUAL salience, justifier
decl-first, 3×). Unification: **H-QHEAD — the executor always fires
from the queue head; queue order = (salience desc, declaration
position); an item continues iff still head; a lazy drop lands when
its item returns to the head, an eager (no-loop) drop at run end.**
The undischarged H-REENTRY alternative was killed by the pre-registered
splitter **c3d** (value-level split: finals EMPTY vs P-survivors;
oracle = queue-head exactly; the zombie self-defeated LK stays visible
through the deleter's run). Clause C dissolves into H-QHEAD + t15.
Rung-2's three-clause wording is SUPERSEDED: the table = queue-head
discipline + in-firing self-cancellation + t10-leak/t15-revive +
cascade (below). c3d is also a new doubly-divergent engine witness.

### 2. The executable spec + the dedicated fuzzer (population loop LIVE)

`probes_pending/tms_envelope/model_sd.py` (simulate()) +
`validate_cells.py` (32 banked truths incl. 3 derived runaways) +
`tools/fuzz_tms_sd.py` (the L-SD grammar; gen.rs walls UNTOUCHED;
mismatches oracle-3× flake-filtered; engine census riding along).
Population seed 6001 ×150 fresh found REAL out-of-sample holes and
drove these mechanism-grounded corrections: **the D-076 CASCADE was
missing** (P dies ⇒ its non-defeated LK dies eagerly — x130) with the
**ZOMBIE refinement** (a self-defeated LK's dep is already cancelled
at break time; it is immune to its P's death and dies only at its
drop — c3d); **the k1-LEAD-NL RUNAWAY family** (7/7 population hits;
the d1-d5 boundary ladder pinned it: SELF-CONTAINED — no deleter
needed (d3); trail terminates (d2); lazy-lead terminates (d4);
no-loop blocks t15 revival (d2); a NEW positive-pattern Drools
runaway family beyond the CE-only D-080 census; engine terminates =
strictly better; uncertifiable); **t15 re-derivation clears fired
marks** for tuples that DIED in a defeat churn (d4 — same-value
refire as a NEW object) and ONLY for those (x52/x68/x130: a
non-breaking justifier's live fired tuples never re-derive); **LK
re-creation is a new OBJECT** — observers refire (x14; the
identity-model law applied inside the model). State: 32/32 banked,
**132/150 population clean, ALL 18 residues member-order-sourced**;
engine-vs-oracle census 69/150 (the port A/B baseline).

### 3. ⚖ Bryan's EPICYCLE STOP (method doctrine, logged)

Mid-consolidation the member-order sub-model degenerated into four
fitted toggles (yielded/stale/churned/has_lead_just). Bryan halted it:
**residue cases that keep inverting a model's empirical toggles are
FALSIFYING the formulation, not resisting it; every added conjunct is
an epicycle; a rule that needs a proxy variable is a rule that doesn't
know why that variable matters.** Layer separation ruled: the
pop-level discipline (§1-2) STANDS — every designed splitter confirmed
it; the member-order layer was being hand-derived from firing
sequences (inferring a data structure's layout from its consumption
order) and moved to GROUND TRUTH per D-086. The churned-on-every-pop
model defect found during the stop was fixed FIRST (Bryan's order);
32/32 holds post-fix, honestly (raw population mismatches rose 13→20
when the bug stopped absorbing order cases — then 18 after the §2
principle fixes).

### 4. The member-order GRAFT, phase 1 (SdDump.java; outcomes pre-registered)

`oracle/.../SdDump.java` (ExistsDump clone): every beta memory in
physical iteration order + handle ids + identityHashCode tags, after
every action and firing; 4 reduced residue cores × 3 JVM launches.
**NOT hash texture — cross-launch STABLE 3/3 everywhere** (outcome B
off for these shapes). Three physical rules observed
(`graft-phase1.md`): (1) **add-at-head**, no in-place reordering;
(2) **churn replay REVERSES the list** (break/unbreak re-inserts the
scan at head — the source of every FIFO/LIFO flip the toggles
chased); (3) **sharer split by decl position** (first-declared sharer
consumes staging FIFO, later sharer scans memory LIFO; gt2↔gt4 swap
exactly). NEW fine print opened by the dump, NOT hand-derived: gt3
shows the lazy drop landing AFTER the deleter's first firing — a
clause-B-emptied item apparently DEQUEUES (D-091 empty-item rule) and
the drop rides its re-entry; a13 (k=0) pinned the empty-item case
differently ⇒ the split is phase-2's target, with the per-path
SegmentMemory staged-list/peer dump (rule 3's construction).

### 5. Gates + filings + state

lint **1694/0/0** (9 more cells: c3a/c3c/c3d/d1/d3/d4/d5
open_divergence — d1/d3/d5 = runaway-class, uncertifiable; c3b/d2
live controls); xfail drift gate GREEN; corpus untouched
**11/1124/354**; engine untouched; oracle module gains SdDump
(diagnostic main class — runner/gate paths untouched, classpath
rebuild clean). Arc census: 32 constructed cells, 16 open_divergence.
Filings: model_sd.py + validate_cells.py + fuzz_tms_sd.py +
graft-phase1.md + the predictions file (every round pre-logged,
including the pre-registered graft outcomes). NEXT: graft phase 2
(per-path staged lists/peers; the empty-item drop-landing split),
encode rules 1-3, re-population toward 0-div; then L-MB. The four
in-model order toggles are RETIRED as semantics (stopgap only,
superseded by graft-phase1.md).

## D-190 — L-SD CONSOLIDATION CLOSED AT 0-DIV: graft phase 2 pinned the member-order construction; the toggles are GONE; **750/750 across five fresh seeds** (2026-07-12)

Bryan's mandate executed end-to-end: phase 2 → encode rules → fresh
seeds to 0-div.

### 1. Graft phase 2 (SdDump + per-path staged lefts / blocked / peers)

Decisive dumps (graft-phase1.md, phase-2 section): **gt5** — the
self-defeated justifier's re-adds appear as its OWN path's staged
inserts in PRE-reversal scan order, held STAGED-WITHOUT-DIRTY (the
t10 leak's implementation; t15's revive = the dirty set — the
queue-head discipline needed no amendment); **gt6** — a k0-NL fold
NETS OUT on the deleter's off-path node (same-batch ins+del fold
away: no reversal, t0 staging intact); **gt7/gt8** — the fold-staging
matrix: owner-members ⇒ PRE-reversal scan, the non-owner justifier ⇒
POST, decided by t0-OWNERSHIP not staging-presence (gt8's fold-2 is
the discriminating observation: a non-owner holding leftover staging
still stages POST). Phase-1's "drop rides re-entry" reading RETRACTED
on re-read (stale unevaluated rtm, not a live LK): the drop lands at
head-return, exactly per the queue-head discipline.

### 2. The final order layer (model_sd docstring §4; ZERO toggles)

Physical list: add-at-head, delete-in-place, fold ⇒ REVERSAL. Sharer
split by declaration (t0 owner = staging FIFO; later sharers =
memory-scan) — recurring for obs_join twins (6003x15/x47). Fold
staging by ownership (PRE/POST matrix). Unshared folds pend until a
member's eval consumes the scan as its WHOLE continuation. k0-NL
folds net out iff the justifier is declared before the deleter
(x70-class churns). lead-justifier and del_join consume insertion
order. Every clause traces to a dump or a 3×-stable population
discrimination — no proxy variables (the epicycle-stop discipline
held).

### 3. THE 0-DIV RESULT

**32/32 banked cells; population 750/750 — seeds 6001-6005 × 150,
ZERO divergences** (mismatch protocol: oracle 3× flake filter; none
needed at close). Seeds 6004/6005 were untouched by any fitting
round — genuine out-of-sample confirmation. Engine-vs-oracle census
over the same 750: **300/750 divergent (40%)** = the port's A/B
baseline (69/53/58/62/58 per seed). Lone flagged residual anywhere:
fz_123_3060's T0(5)-first initial pick (two-not + no-loop-observer
structure, outside the v1 grammar; noted, unmodeled). The L-SD
sub-family's executable spec is DONE: the queue-head discipline +
in-firing self-cancellation + t10/t15 + cascade/zombie + the
member-order physics, validated out-of-sample at the bar the arc
set.

### 4. State

Engine untouched throughout (the consolidation is spec-side; the
port battery = the 16 open_divergence cells + the 40% population
census awaits its own gated slab). lint 1694/0/0; corpus untouched;
drift gate green. NEXT per Bryan's sequencing: **L-MB** (the
18-witness mutation-break cluster — plan §5 rung 2 of the residue),
opening with the same discipline: bucket census re-read, ladder with
pre-logged predictions, kill-cells first.

## D-191 — the L-MB POPULATION INSTRUMENT built FIRST (Bryan's ruling: "build it before believing the ladder"); v2 A-shape grammar + certified priors; census run; TWO principle fixes + ONE latent v1 bug caught (2026-07-12)

Bryan's sequencing inversion: L-SD's table only became trustworthy at
750 fresh cases, so L-MB's fuzzer precedes L-MB's ladder.

### 1. The v2 instrument

`tools/fuzz_tms_sd.py` v2: P gains a mutable f1; the k=1 justifier
draws `amut ∈ {None, del, set_break}` (+ a mutfirst RHS-order toggle;
set_break adds the `f1 == 0` alpha and emits the house
`$p.setF1(1); update($p);` form; composable with breaks∈{T,F} — the
breaks=False+amut draw is the PURE dep-death transient). model_sd
gains the certified PRIORS: self-inflicted dep-teardown lands lazy
(fz_42_2442), foreign delete effects eager (recompute + cascade/
zombie), t15 on RHS deletes. gen.rs walls untouched.

### 2. Census + fixes (validate 32/32 held throughout)

First census (7001): 21 divergences → signatures. Two were prior
bugs with clean principles, FIXED: (i) a self-inflicted delete never
t15-revives the ACTOR's own suppressed tuples (oracle keeps them
parked and the P alive); (ii) a set_break justifier's f1-alpha takes
it OFF the shared node — private staging, insertion order. One was a
LATENT v1 MODEL BUG the original 750 draws never exercised
(sdp6003x67): run-end eager drops must land BEFORE the next head
selection commits (the model let a low-salience observer pop while
the dying LK still hid the deleter) — loop fixed; honest note: the
D-190 0-div stands for its 750 draws; populations are evidence, not
proof, and the v2 stream shift caught the corner.

### 3. State: the L-MB target list is population-sourced

Post-fix: **7001 149/150, 7002 145/150, 6001-v2 148/150, 6003-v2
145/150 — 13 residues/600, ALL A-shaped.** Signature spread: set_break
(lazy trail / NL lead / mutfirst variants) + del (NL lead / lazy
trail). Headline target: the set_break shape where the oracle fires
the justifier's SECOND tuple despite the not-break — the mutation's
dep-teardown unbreaks IN-RUN, faster than the fz_42_2442 lazy prior.
Engine census on v2 populations: 47/56/45/54 per 150. NEXT: the L-MB
ladder proper (kill-or-confirm the in-run unbreak; the k2lazy
boundary; mutfirst), each finding checked against the live instrument
the way Bryan mandated. lint 1694/0/0 unchanged (no corpus/engine
touch).

## D-192 — the L-MB LADDER, rung MB-1 + two graft rounds: the census headline retracted (method law), the update-relocation and fold-batch physics pinned; v2 populations at 589/600 with 11 named order residues (2026-07-12)

### 1. MB-1 (7 cells, predictions pre-logged, all 3×-stable)

The D-191 headline ("set_break's dep-teardown unbreaks IN-RUN") was a
CENSUS MISREAD — the solo set_break-trail fires ONCE (mb1_st; x48's
double-fire was its deleter's t15, which the model already carries).
⚖ method law, self-applied: the retraction is the finding. The 2×2
(del/set_break × lead/trail) + nobrk/mutfirst controls all match the
D-191 priors — the model was GREEN on all seven BEFORE any new rule.
ENGINE: RED on both lead cells (mb1_dl/mb1_sl — the lead clause-B
miss extends to A-shapes; both filed open_divergence, port battery).

### 2. Graft rounds (gt9/gt10) — three more physical rules

- **update RELOCATES the fact to the join right-memory TAIL** (gt9:
  P2 visibly moves at gen-2); the not-node ltm does NOT relocate
  (gt10) — per-node-type update semantics, both observed.
- **obs_join sharers consume each generation in MIRROR orders**:
  owner = reversed rtm-scan, later sharer = rtm-scan (retrofits
  gen-1/gen-2/b7 exactly).
- **Fold batching**: an UNSHARED justifier's fold (lead, or alpha'd
  set_break trail) nets out on other nodes when EAGER (same-batch
  ins+del, gt10 — deleter staging survives FIFO even deleter-first);
  when LAZY the later-batch drop genuinely churns them (pending fold,
  the gt3/d4 machinery) — the unified gate fixed the 7001x114-class
  regressions my first gt10 encode introduced.

### 3. State

v2 populations: **7001 149/150, 7002 147/150, 6001 147/150, 6003
146/150 = 589/600 (98.2%)**; the 11 residues are ~5 named ORDER-class
signatures (nb-justifier member orders after partner deletes; the
mf-lazy justifier's own continuation order; lead-NL-nb missing
gen-2 obs fires; nb obs pairing tails) — ZERO landing/mechanism
divergences anywhere in v2. Engine census steady (47/56/45/54).
lint **1701/0/0** (7 mb1 cells: 2 open_divergence, 5 live); drift
gate green; corpus + engine untouched. NEXT: the residue signatures
one graft-dump each (nb/del interplay first), then fresh v2 seeds to
0-div; the I-RD front after.

## D-193 — L-MB continuation: two more dump-grounded rules (del = EAGER dep-cascade; the INTERVENING-ACTION fold law); v2 at 738/750 with three named residue clusters (2026-07-12)

gt12 (nb-del + obs_join): the observer NEVER fires — a self-inflicted
DELETE carries the LK's dep EAGERLY (plain cascade at the action);
the zombie+lazy-drop prior was WRONG for deletes and its spurious
fold machinery caused the x134-trio misorders. fz_42_2442's
self-inflicted-lazy is the UPDATE-break's law, not the delete's.
gt11 (ilfirst-NL + deleter): the deleter's node CHURNS under ilfirst
where gt10's mutfirst did not — **the fold nets out iff NO WM action
intervenes between the LK-ins and the run-end del** (ilfirst puts the
update between them, forcing the staged ins to process; mutfirst
leaves them adjacent to fold). Both encoded; 32/32 held.

STATE: v2 populations **7001 149/150, 7002 148/150, 6001 148/150,
6003 146/150, 7003 (fresh) 148/150 = 738/750 (98.4%)**; the 9
survivors cluster: (i) the lead-NL quartet (x17/x90/x128/x131 —
set_break×NL×lead, missing gen-2 obs fires / deleter orders — the
multi-firing run's drop-batch structure, NOT hand-derived, next
dump target); (ii) the mf-lazy-trail trio (x68/x41/x88 — the
justifier's own post-churn continuation order); (iii) two nb-trail
tails (x103/x0). All order-class; ZERO landing/mechanism divergences
in 750 v2 cases. Engine census 47/56/45/54/37. NEXT: one dump per
cluster, then fresh seeds to v2 0-div; then I-RD.

## D-195 — the TmsDump lens LIVE in-tree; the lead-NL belief-staging cluster CLOSED (⚖-candidate eval-consumption landing + the mutfirst composite gate); v2 at 741/750, zero belief-class residues (2026-07-12)

THE INSTRUMENT (the D-194 handoff's unit, now in `SdDump.java` — one
dump, both lenses): per firing + PRE-FIRE/FIRE-BOUNDARY, every WM
handle's EqualityKey status (STATED/JUSTIFIED) + BeliefSet (size,
staged WorkingMemoryAction, each LogicalDependency's justifier
rule/tuple/act/queued), the TMS-side key map (zombie = belief
without WM), and the session's pending PropagationEntry queue. TMS
lines carry NO identity tags (raw cross-launch diff); the TMS object
is only reached through a live BeliefSet (or the factory AFTER a key
exists) — the instrument cannot create the TMS. Cleanliness proven
in-run: PRE-FIRE pending still shows the two external Inserts AFTER
getFactHandles() (no flush); instrumented firing sequence == the
banked uninstrumented oracle sequence. gt13 + gt14 3/3 stable.

THE TRACE VERDICTS (predictions pre-logged, tmslens-predictions.md;
reading in tmslens-results.md): H2 FALSIFIED — both deps attach
synchronously at insertLogical exec (store handle + JUSTIFIED key +
dep immediate; only the network Insert rides the queue, behind the
RHS-earlier Update). H1's WM form FALSIFIED — the session queue
drains BETWEEN the justifier's own firings; the observers'
non-preemption is an agenda-layer fact (the R1 interposer arc's
lane, not this lens's). H3 ANSWERED = the LK1/LK2 asymmetry:

⚖-CANDIDATE EVAL-CONSUMPTION LANDING (dump-grounded, splitter-
confirmed): the amut update-break's dep-teardown lands when the
JUSTIFIER'S network eval consumes the staged break — (a) mid-run
break ⇒ consumed at the between-firings eval: teardown + inline
retract complete before the next firing, the LK's queued Insert nets
out, observers NEVER see it; (b) last-firing break ⇒ the staged
delete waits while strictly-higher observers run (zombie-justifier
window: belief n=1, justifier match dead — RO2 fired twice on it),
then lands at the justifier item's NEXT POP — strictly-lower
observers never fire (gt14: RO3@3 silent 3/3; quiescence-landing
dead). Sibling of the D-189 lazy-drop-at-pop law.

THE COMPOSITE LANE (breaks=True + set_break) is MUTFIRST-GATED —
found the honest way: the ungated extension regressed 7002 149→145
(x26/x58/x71/x95, all ilfirst) while x51 (mutfirst, fresh-seed
FINALS divergence, del_join@5 firing ×4 on the zombie) demanded the
window. Mechanism = the D-193 intervening-action fold law's
belief-layer sibling: the staged RHS pair drains in RHS order;
ilfirst's insert-caused not-break reaches the tuple FIRST (D-076
eager cascade at propagation — no window even for strictly-higher);
mutfirst's update join-break stages first (lazy eval-consumption —
the pop window). breaks=False has no race (mutfirst-independent).

MODEL PORT (model_sd.py, the only semantic file touched): pure lane
— eager last-firing set_break drop rides drops[] (pop) instead of
eager_pend[] (loses-head); mid-run keeps eager_pend (nets out
pre-next-firing, already order-correct); composite lane — the elif
moves the LAST cycle's drop to drops[] iff eager ∧ mutfirst ∧ no
revivable P; mid-cycle stays loses-head (the suppress/revive
machinery needs the LK dead pre-refire, sd_d* certified). Lazy
routing untouched (no dump evidence).

SCOREBOARD: v2 populations 741/750 — 7001/7002/6001 149/150, 6003
147/150, FRESH 7004 147/150 (its 4 finds bisected vs HEAD model:
x67/x108/x131 pre-existing order-class; x51 = the composite window,
fixed out-of-sample). THE LEAD-NL QUARTET RESOLVED (x17/x131/x128
clean; x90 DEMOTED to order-class — firing set right, deleter-run
P-order reversed). ZERO belief-staging divergences in 750; survivors
= mf-lazy-trail continuation order (x68/x41/x88, kin x90/x67/x108/
x131) + nb-trail tails (x103/x0) — both order-class, one SdDump run
each per the handoff. Engine census steady 47/56/45/54/55.

BANKED: tmslens-predictions.md, tmslens-results.md, gt14_leadnl_
subsal.json, gt15_composite_mutfirst.json (x51 renamed — collision
hazard), lmb-census.md §RESOLVED. Gates: validator 39/39 through
EVERY edit; make diff green incl. xfail drift 75/75; lint 1716/0/0
(gt14+gt15 live). Corpus + engine UNTOUCHED; gen.rs walls UP. NEXT
(Bryan's sequencing): the two order clusters, then I-RD / the R1
interposer ladder.

## D-196 — BOTH ORDER CLUSTERS CLOSED (gt16-gt20 dumps + the 2×2), the del-lane law corrected (gt12 was salience-confounded), v2 at TRUE 0-DIV (1800/1800, twelve seeds); the R1 INTERPOSER LADDER OPENED AND ITS FIRST RUNG IS 6/6 GREEN (2026-07-12)

Bryan's sequencing: "the two order clusters, one SdDump run each,
then 0-div. Then R1's interposer ladder before I-RD."

THE CLUSTER DUMPS (predictions pre-logged, ordlens-predictions.md;
readings in ordlens-results.md; all instrument runs 3×-stable):
- gt16 (x68-core, lazy trail mutfirst composite): ⚖ TWO-PHASE
  UNBREAK / STALE-RTM STARVATION — at the justifier's pop the DROP
  lands (WM retract) but the not-level unbreak stays STAGED behind a
  dead LK's stale right-tuple + blocked chain; blocked tuples revive
  only on a TOUCH (another rule's WM action on a P that reaches the
  node — the ALPHA GATE: pmut'd Ps never do, lead and trail alike);
  revived continuation = the rule's OWN pre-fold private-phys scan
  (trail; new jphys) / insertion (lead, banked). No touch ⇒ STARVE —
  the mb1_st/sl/dt banked truths are this law's no-deleter face.
  Also: D's not processes the full break+unbreak history at ITS
  eval; consume = the pre-reversal scan; the fold reverses at the
  FOLD, not at consume (pf_reversed).
- gt17 (x103-core, nb trail): NO folds ever reach the deleter (nb
  keys never break) — its t0 staged order survives FIFO; the old
  pending_fold churn condition was a cross-node PROXY (removed).
- ⚖ DEL-LANE EVAL-CONSUMPTION (x88/x0/x66/x79/x98): amut=del joins
  the D-195 law — the actor's own dep rides to its next pop (lazy /
  last-firing) or nets at the between-firings eval (eager mid-run);
  gt12's "del = eager cascade" (D-193) was SALIENCE-CONFOUNDED (its
  observer sat below the justifier, where pop-landing is
  output-identical); the foreign cascade stays D-076-eager (x130).
- THE EAGER-COMPOSITE MATRIX (gt18/gt19 dumps + the c2x2 corners +
  seven population witnesses): eager cycles at FOREIGN nodes —
  mutfirst keys never propagate (no fold, any decl); ilfirst LEAD
  nets out everywhere (gt18: D's ltm pristine, zero folds; x131's
  2-fold match was parity coincidence); ilfirst TRAIL folds IFF the
  del_not is DECLARED BEFORE the justifier (the 2×2 corners
  gt20a/gt20b: sink-order shaped). The mutfirst pop-landed LAST key
  folds regardless (its insert long-processed when the delete
  arrives). The nb last-key window is closed ONLY for ilfirst+trail
  (x147); ilfirst+lead keeps it (x6001x131/x7004x92), mutfirst
  keeps it (gt13).

0-DIV: **v2 populations 1800/1800 across TWELVE seeds** —
7001/7002/6001/6003 + 7004..7010 (every former fresh seed re-run
clean) + NEVER-USED 7011 clean on first contact. Validator 39/39
through every edit; the 26-witness registry + 2 corners inlined in
check_witnesses.py (pure model regression, no oracle needed).
Engine census 47/56/45/54/55/47/44/52/51/48/46/54 = the port A/B
baseline. Every fresh-seed find this arc was bisected against the
pre-edit model before attribution; two selfmade regressions
(x66 round-1 del routing; x131/x92 x147-overreach) were caught by
the base-seed re-runs and reverted-by-narrowing same sitting.

THE R1 INTERPOSER LADDER (interposer_ladder.py; predictions =
mechanical model_sd outputs logged pre-run, interposer-predictions
.md): **first rung 6/6 GREEN, 3×-stable, zero adjustments** —
min812-spine lazy k0 (the pop window is a salience THRESHOLD: the
@5 interposer glimpses beside the @10 observer; sub-salience never),
eager k0 (no ≤-salience glimpse, interposer included), 9133-spine
lazy k1 fan-out (clause B + starvation: gen-1 only), and the D-195
(b) BETWEEN-row (RI@6 fires once on the zombie LK after RO2@7's
run, before RJ's pop — gt14's below-row completed). ⇒ the (cloud ×
belief-loss) landing rows are a CONFIRMED TABLE, population-
certified at 0-div and interposer-verified on both named spines.
The ⚠ D-106 region was never entered; engine untouched; walls up.

BANKED: gt16..gt19 + gt20a/gt20b graft targets; ordlens-
{predictions,results}.md; interposer-{predictions,results}.md;
interposer_ladder.py; check_witnesses.py. Gates at close: make diff
green (xfail 75/75), lint 1722/0/0, validator 39/39, witnesses
26/26. NEXT (Bryan's call): the engine A/B on the ladder cells +
the 13+18 xfail witnesses re-read against the completed table (the
port target list); I-RD stays last.

## D-197 — THE PORT SLAB, ROUND 1 (P1+P2): the deferral CAUSE MODEL landed — the interposer ladder is ENGINE-GREEN 6/6, census 599→505 (−15.7%, all 12 seeds), fz_123_941 GRADUATED; corpus byte-identical, agenda_open ×19 receipts clean (2026-07-12)

Bryan: "Push it. Then the port slab: engine A/B on the ladder cells,
re-read the 13 L-SD and 18 L-MB xfail witnesses against the completed
table, and that produces the target list. ... the port is translation."
D-196 pushed (9979cda..000a8e1). The A/B + re-read (port-target-list
.md): ladder 4/6 engine-green pre-port — the two misses named the
mechanisms (ip_a3 = the eager run-end landing missing; ip_c1 = mid-run
net-out + run-continuation missing); the 31 witnesses' signatures:
L-MB 18/18 engine-OVER finals-equal (the P1/P2 family), L-SD 11 under
(P3: the equal-salience queue-position window — min812's decl-first
observer; the engine's min608-over-generalized drain; ⚠ D-106-adjacent,
port LAST), 2 over (P4 clause-B), 5213 (P5 clause-C), 1353 compound.

THE TRANSLATION (engine.rs, the TMS deferral machinery only —
next_activation's flush loops + tms_on_terminal_del + tms_insert_
logical + evaluate_rule_inner; the executor's pick/halt logic
untouched): `tms.deferred`'s bool became a CAUSE-FLAGS u8, populated
from three per-evaluation lanes + one per-act flag:
- bit1 NOT-side (`right_touched`, NOT/SubnetNot node right ops only —
  a join's right is a positive pattern, not a CE): the SELF-DEFEAT
  lane — flush-drains unconditionally at the run end (ip_a3: the
  eager k0 drop now lands before any ≤-salience pop).
- bit0 LIA-hit (`left_touched`, unchanged — watch-gated s0 staging,
  which is what kept the t20 property-reactivity split certified):
  the t20 flush discipline — EXCEPT when bit2 is set.
- bit2 LATE-DEP (`late_acts`): the D-195 RHS-order race read LIVE at
  insertLogical — a MUTFIRST consequence has already broken its own
  tuple's alpha when the dep attaches ⇒ the act's last-firing
  teardown rides to the item's POP (the gt13/ip_c1 zombie window);
  ilfirst deps attach whole and die at the flush (pr_tms_t20d +
  pr_tms_selfbreak_flush stayed green; the a/b/c/selfbreak_lazy
  certified-POP cells held via the LIA watch gate).
- bit3 JOIN-RIGHT (`joinr_touched`): the LEAD topology's P side —
  flush-drains MID-RUN only (run_live = the item's queue non-empty);
  the last firing's entry rides to the pop. ip_c1 exact INCLUDING
  the gt9 pairing order.

RECEIPTS: the interposer ladder 6/6 ENGINE-GREEN; corpus
11/1124/355 byte-identical (the six t20-family pins all green);
**agenda_open ×19 BYTE-IDENTICAL** (⚠ D-106, measured twice — the
worktree-free stash-dance verified with git diff --stat after);
cargo test 9 suites; lint 1723/0/0. **fz_123_941 GRADUATED out of
xfail** (10/10 converged both sides, firings + finals — its I-RD
divergence carried a landing component all along; now a
regressions/ cell with a D-197 comment); fz_123_9175 moved TOWARD
the oracle (5→4 firings; re-triaged 10/10 stable, rebanked — the
drift bank is 74). CENSUS (the port A/B metric, 12 seeds ×150):
**599 → 505 divergent (−94, −15.7%), every seed improved**
(47→38, 56→46, 45→37, 54→47, 55→44, 47→41, 44→35, 52→44, 51→41,
48→44, 46→41, 54→47); model-vs-oracle 12× 150/150 — the 0-div spec
held through the engine change. The 30 remaining envelope witnesses
unchanged, as scoped (P3/P4/P5 + lazy fine structure = round 2+).

DEBUG: the deferral drain sites carry SEINE_TMS_DEBUG site tags
(defer-push with flags, drain[post-fire-continue|flush-pre|
flush-post|pop]) — the diagnosis loop for the next rounds.

NEXT (round 2): P4 clause-B (sd_b2/b4 battery, the two L-SD
over-cells), P5 clause-C alternation (sd_c1, 5213), the lazy L-MB
fine structure (the census's remaining mass), THEN P3 (the
equal-salience queue-position drain split — D-106-adjacent,
receipts-gated, deliberately last). UNPUSHED; Bryan holds the push.

## D-198 — THE PORT SLAB, ROUND 2 (P5 + most of P4): clause-C alternation + the lead-not suppression landed; THREE GRADUATES incl. the L-SD×L-MB compound; census 505→483 net with an honest per-seed mix; one panic caught by the population net (2026-07-12)

THE TRANSLATION (engine.rs, TMS/park machinery only; executor
untouched):
- **P5 clause-C (⚖ t15/d4)**: tms_parked_del's left-death now
  unparks the rule's OTHER parked tuples and re-activates the live
  ones in REVERSED-chain order (gt16's re-add law) — LAZY plain
  rules only (the eager/or-twin exclusion is the model's t15 scope;
  the ungated version flipped fz_777_6816 — caught by the
  regressions tier, scoped same sitting). sd_c1 EXACT
  [RJ1,RD1,RJ3,RD3,RJ2,RD2]; **fz_42_5213 GRADUATED** (the full
  20-firing alternation, 10/10 both sides).
- **P4 clause-B (lead)**: the park's blocked-leak finds LEAD nots
  by ENV LOOKUP (the pos-1 arithmetic is trail-only) and parks the
  blocked LEFT PREFIX; tms_parked_ins matches by starts_with
  (trail parks are full-width ⇒ prefix == exact, certified cells
  unchanged); the leak spans OR-SIBLINGS and prunes their queues.
  The eager list split into evaluate-all-then-drain passes
  (Drools' evaluateEagerList shape). sd_b2 fixed ([RJ] once);
  **fz_123_3060 GRADUATED** (10/10). BONUS: **fz_7_9550 — the
  L-SD × L-MB COMPOUND — GRADUATED** with no dedicated work.
- RESIDUE (named, round 3): sd_b4/fz_7_9375 — the OR-TWIN corner:
  the sibling's blocked list is invisible to node.blocked_of (the
  D-158 PnShadow structure is the suspect; drain-site sibling-eval
  + group park + queue prune are in place and insufficient) — needs
  a PnShadow read. sd_b3 stays fenced (lazy or-twin = Drools
  runaway, Family II).
- Seven Family-II runaway witnesses moved engine-side (fire-limit
  oracles, fenced-by-nature) — rebanked; fz_7_9864 moved TOWARD
  (17→18 vs 19). Drift bank 71.

⚠ LESSON (the population net): a prefix park re-activated through
my revive filter PANICKED on a 3-fact population shape (index out
of bounds) while the corpus, ladder, AND receipts were all green —
the 12-seed census caught it (fixed: full-width parks only revive
directly; prefix parks re-derive via the network). Populations are
the panic net, not just the divergence metric.

RECEIPTS: corpus **11/1124/358** byte-identical (three graduates
this round); drift 71 identical post-rebank; agenda_open ×19
BYTE-IDENTICAL (⚠ D-106); ladder 6/6 held; cargo test 9 suites;
the 26-witness oracle registry untouched (model 0-div held 12×
150/150 through both rounds). CENSUS: **505 → 483 net (−22)** —
HONEST MIX: 7001 38→28, 6003 47→37, 7009 44→39, 7010 41→32, 6001
37→35 improved; 7002 46→50, 7006 37 (+2), 7007 44→48, 7008 41→46,
7004 +1 REGRESSED on non-pinned population shapes — the round's
park/revive/two-pass changes over- or under-apply on some geometry;
those slots are round-3 diagnosis targets alongside the or-twin
corner, the lazy L-MB mass, and P3 (D-106-adjacent, last).
Cumulative from the pre-port baseline: **599 → 483 (−19.4%)**.

## D-199 — THE PORT SLAB, ROUND 3: the shared-node depth-match (P4 CLOSED), the land_eager lead-k1 unpark, the revive ACTOR EXCLUSION, and the lead park-RECORD + foreign-death SWEEP; TWO GRADUATES; the round-2 census regressions CURED; census 483→242 (−49.9%) (2026-07-12)

THE ROUND-3 ENTRY was the handoff's or-twin corner (sd_b4 /
fz_7_9375). The handoff's PnShadow suspicion was WRONG — the trace
showed PnShadow is not even constructed for the shape (its not
carries cmps); the real cause is NODE SHARING: `do_exist[0:Not]` is
ONE shared trie node feeding both terminals, its `blocked` map HOLDS
the block, but the D-198 leak looked the node up by `env == (ri,
pos)` and a shared node carries its FIRST owner's env — the
sibling's lookup missed, no park leaked, the un-break re-fired the
twin.

THE TRANSLATION (five changes, engine.rs TMS/park machinery only;
the executor pick/halt untouched):

1. **The depth-match (sd_b4)**: the leak finds the not's node by
   DEPTH (`trie[ni].env.1 == pos` over `nets[ri].path`), creator-
   agnostic — sharing preserves depth, so this is exactly
   `path[pos-1]` for sharers and unshared alike (the D-198 comment's
   "pos-1 holds for trail layouts only" was a misdiagnosis; the env
   find it installed also encoded the pos==0 guard, kept). sd_b4
   fires ONCE — exact. 19 xfail witnesses moved, ALL toward or onto
   the oracle on the multiset metric (the two firing-count "drops" —
   fz_7_1353 −8/+1→−8/+0, fz_7_9864 −2/+1→−2/+0 — are spurious
   extras eliminated; a COUNT drop is not an away-move when the
   extras die). **fz_7_9375 GRADUATED** (10/10 both sides) and
   **fz_123_9175 GRADUATED** (the D-197 toward-mover converged,
   10/10). 15 Family-II runaways rebanked (drift 69). Per-case flip
   attribution vs a 99b363d worktree: this change broke ZERO cases;
   the five round-2-regressed seeds all dropped below pre-round-2 on
   it alone (7002 46→42, 7004 44→31, 7006 35→28, 7007 44→36, 7008
   41→34).

2. **⚖ land_eager lead-k1 unpark (model_sd land_eager)**: an EAGER
   (no-loop) justifier with exactly one plain NOT strictly upstream
   of exactly one positive join (tms_lead_k1), whose firing
   SELF-KILLED its premise (tms_left_death: a tuple member dead or
   alpha'd out) — the eager landing's unbreak RE-PROPAGATES: unpark
   at the three eager drain sites (post-fire-continue gated on
   no_loop, flush-pre, flush-post), NEVER at the pop (the mutfirst
   last key rides to the pop and lands lazy — no rederive) and never
   for lazy rules (sd_b2's park holds). Fixes the sdp7002x4 class
   (oracle fires once per P on delete($p)/update-out; engine fired
   once total). ⚠ THE FIRST CUT LACKED the left-death gate and made
   the engine FAITHFULLY FOLLOW Drools into the d3/d5 no-amut
   RUNAWAY — sdp7002x40 spun at 99.9% CPU (the oracle's fire limit
   catches the runaway; the engine has no fire cap and the spin
   guard resets per next_activation call, so a real firing loop
   never trips it). The census run STALLED = the population net's
   second panic-class catch in two rounds. The no-amut shape keeps
   the park: the engine TERMINATES and its divergence stays
   Family-II fenced (the terminates-invariant). Census loops now
   wrap each seed in `timeout 900` so a future spin fails loud.

3. **⚖ the revive ACTOR EXCLUSION (model t15_revive actor; kin of
   fz_42_2442)**: a SELF-INFLICTED left-death — the dying parked
   tuple's P deleted/updated-out by the rule's OWN RHS (the staged
   del's origin, rule_parents == ri's) — never revives the actor's
   other parked tuples. tms_parked_del now takes the del's origin
   (both terminal-consume call sites). Fixes the sdp7002x31 class
   OVER-fire (lazy trail mutfirst: the own-tuple park died at its
   stale post-drain terminal-del and revived the whole leaked
   blocked list; the oracle holds the park — fires once). Foreign
   and external deletes revive as certified (sd_c1 exact).

4. **The lead park-RECORD (tms_parked_suppress)**: a PREFIX park (a
   lead not's blocked left) suppresses re-derived children at the
   terminal, but the children stay MATERIALIZED in the join — the
   suppression now RECORDS each suppressed tuple as a full-width
   park entry so left-death events can find them. The t15 revive's
   re-add order is notpos-split: TRAIL keeps the certified
   reversed-chain (sd_c1), LEAD re-derives in INSERTION order (the
   model's land-lane law, banked x108).

5. **⚖ the t15 foreign-death SWEEP, lead lane (model t15_revive)**:
   the model's revive keys on the P DEATH ITSELF — a lead child can
   ANNIHILATE in staging (ins+del fold) and never reach the
   terminal, so the parked-del lane misses the trigger (sdp7007x86:
   the oracle's FIRST foreign delete revives; by the second the
   candidates are gone). New tms_p_death_sweep in on_delete_ex
   (both delete paths, external + RHS): on a foreign fact death,
   every LAZY plain non-ortwin LEAD-k1 justifier whose positive
   pattern ADMITS the dead fact's STALE values (alpha_passes_fields
   — the fact is already killed at the hook; the value-level pmut
   gate = ⚖ the starvation law: an alpha'd-out P's death never
   touches the node) clears its parks; recorded full-width siblings
   re-activate in insertion order, bare prefixes just stop
   suppressing (staged re-derivations queue at their consumption).
   TRAIL stays on the parked-del lane. sdp7007x86 EXACT (the
   lazy-lead alternation, including the pmut'd-P skips); sdp7002x29
   runs the full 12-firing alternation (residue = deleter pick
   order, P6).

THE ROUND-2 CENSUS REGRESSIONS ARE CURED (the handoff's target 2):
per-case flip analysis on the five seeds vs 99b363d (150
cases/seed; the kept census oracle outputs are commit-independent,
only the engines re-ran) found 65 MATCHING→DIVERGENT flips, every
one attributable to D-198's machinery. After this round's changes:
62-63/65 match; the residue is order-only (sdp7008x11/x25 — the P
consume order per firing) plus sdp7007x86 (fixed by change 5).

Park machinery now carries SEINE_TMS_DEBUG tags: park-own,
park-leak, park-record, park-del (left-death + origin), park-revive,
sweep-revive.

RECEIPTS: corpus **11/1124/360** byte-identical after EVERY change
(the two graduates joined regressions/, 10/10 engine-deterministic);
drift bank 69 identical post-graduation (changes 2-5 moved ZERO
xfail witnesses — population-surface only); agenda_open ×19
BYTE-IDENTICAL ×4 vs a bfc363e worktree baseline (⚠ D-106); ladder
6/6 ×4; validate_cells 39/39 ×4; check_witnesses 26/26 ×4; cargo
test 9 suites; lint 1728/0/0 (two park-tag probes' worth of new
lines are debug-only). CENSUS (12 seeds × 150, engine-vs-oracle
divergent): **483 → 242 (−49.9%)**; intermediates: 373 after change
1 alone, 250 after 1-3. Cumulative from the pre-port baseline:
**599 → 242 (−59.6%)**. Model 0-div held 12×150/150 on every run;
no panics, no timeouts. Final table: 7001 16, 7002 25, 6001 16,
6003 19, 7004 19, 7005 20, 7006 17, 7007 23, 7008 22, 7009 25,
7010 16, 7011 24.

THE MASS CLASSIFICATION (target 3, on the 250-state census): SET
123 / RUNAWAY-MISMATCH 64 / ORDER-ONLY 63. The 64 runaway
mismatches are ALL the d3/d5 eager-lead no-amut family — oracle
RUNS AWAY, engine terminates: PERMANENTLY OPEN by the
terminates-invariant (fenced-by-nature; the census's structural
floor is ~64, so the fixable population gap at 242 is ~178). The
ORDER-ONLY mass is 45× k0 + set_break corners = P6's lane (the
model's order layer is the spec). The SET mass was the lazy-LEAD
revive gap (~50, closed by changes 4-5) + P3's equal-salience
window (sdp7002x3: the decl-preceding same-salience observer must
glimpse the transient LK before the drain — the pop-precedence
split) + deleter/justifier pick-order physics (P6) + the x73 class
(lazy-lead-del: a foreign del_not observer under-fires —
undiagnosed, one cluster).

RESIDUE (round 3 continues): P3 NEXT (the drain split — ⚠ D-106
receipts protocol mandatory), then the P6 order sweep; the x73
class; I-RD after the slab (Bryan's order).

## D-200 — THE PORT SLAB, ROUND 3 (part 2): P3 LANDED — the equal-salience pop-precedence drain split, LANE-SCOPED to the NOT-side self-defeat flag; TEN GRADUATES including min812 AND the fz_7_1353 FINALS-DIFF compound; census 242→197; cumulative 599→197 (−67.1%) (2026-07-12)

THE SITE (as scoped by the handoff): next_activation's post-fire
continue — the `higher` gate governed BOTH the D-091 network
re-eval halt AND the TMS deferred drain; min608's over-
generalization drained equal-salience continuations wholesale,
killing the transient before a DECL-PRECEDING same-salience
observer could pop (min812's certified glimpse).

THE SPLIT: the halt keeps the certified strictly-higher gate
UNTOUCHED (the D-091 lane, the executor pick untouched — ⚖ D-177
landing-not-pick). The DRAIN gains pop-precedence: computed beside
`higher`, `eq_decl_preempt` = the fired rule is LAZY (no-loop/dyn
exempt — the model's land_eager runs before every selection) AND
some queued same-group item has (sal == l_sal && decl < l). A
preempted entry LINGERS (the deferred entries keep the item queued)
and lands at drain[pop] when l is actually reached = the model's
land_lazy-at-selection, exactly head()'s (-sal, decl) order.

⚠ THE LANE-SCOPING LESSON (14 certified cells as the tripwire): the
first cut deferred ALL of l's entries under preemption — 14
regression-tier cells broke immediately (baseline/probes/receipts
all green). Their common geometry: justifiers with NO not — the
LIA/t20 lane (bit0: a self-update/delete breaks the own tuple),
whose drain-at-continue discipline is CERTIFIED (pr_tms_t20*). The
pop-precedence deferral belongs to the NOT-side SELF-DEFEAT lane
ONLY — the drain position() now skips exactly entries with (fl & 2)
!= 0 under preemption; bit0/bit2/bit3 entries keep their certified
timing. All 14 recovered; the D-197 cause-flags model is what made
the scoping expressible (the lanes were separable because the flags
exist).

TEN GRADUATES (10/10 oracle-stable, 10/10 engine-deterministic,
370/370 live-oracle green in the regressions tier):
**xf_tms_min812** (the P3 anchor — the certified glimpse now
fires), **fz_123_2135, fz_123_3370, fz_123_4318, fz_123_7637,
fz_123_9133** (the ×3-generation shape: 1→4 firings),
**fz_777_9637, fz_7_812, fz_7_9864** (17→19 = the oracle count),
and **fz_7_1353 — THE FINALS-DIFF COMPOUND (P3+P4+cascade)**: 4→12
firings, exactly as the port-target-list predicted ("the
FINALS-DIFF resolves when its 8 lost firings return"). The ENTIRE
P3 witness list from the target list graduated in this one change.
3 ORACLE-RUNAWAY movers rebanked; fz_7_2864's broad-gate movement
reverted with the lane-scoping (its P3-sensitivity rode a bit0
entry — correctly excluded; rebanked at its pre-P3 output);
fz_7_9360 rebanked toward (+5 firings, −1/+1 residue).

RECEIPTS (⚠ D-106, the full protocol): agenda_open ×19
BYTE-IDENTICAL ×2 at this change (broad + refined; ×6 total this
sitting) vs the bfc363e worktree baseline; the halt matrix +
fz_9001/9003/9004 tripwires ride the probes tier — 1124/1124 green;
baseline 11/11; regressions **370/370** (the 14 t20-lane cells
recovered + the 10 graduates); drift bank **59** identical
post-rebank; ladder 6/6; validate_cells 39/39; check_witnesses
26/26; cargo test 9 suites; lint 1738/0/0.

CENSUS: **242 → 197** (12 seeds × 150; model 0-div 12×150/150, all
exits clean). ROUND-3 CUMULATIVE: **483 → 197 (−59.2% in one
round)**; from the pre-port baseline: **599 → 197 (−67.1%)**.
Final table: 7001 14, 7002 21, 6001 12, 6003 17, 7004 15, 7005 16,
7006 16, 7007 16, 7008 16, 7009 23, 7010 11, 7011 20.
Composition (reclassified on the same case set): RUNAWAY-MISMATCH
64 (the d3/d5 no-amut family — oracle runs away / engine
terminates, PERMANENTLY OPEN, the census floor), ORDER-ONLY 63
(P6's lane — the model's order layer is the spec), SET 70 (down
from 115 — P3 wiped ~40% of the remaining set-mass; what's left:
pick-order physics [P6-adjacent], the x73 class, tails).

⇒ **ROUND 3 IS COMPLETE. P1-P5 ARE ALL LANDED.** The engine's
fixable population gap is 133 (63 order + 70 set) over 1800; the
structural floor is 64. NEXT: P6 (the order-layer sweep — the
model's member-order physics: fold/reversal/insertion-order/
pick-order per lane) or Bryan's call; I-RD after the slab (Bryan's
order).

## D-201 — THE PORT SLAB, P6 (part 1): the k0 fold/churn law — the arc's single biggest mover; the composite last-key RIDE; the del-lane race widening; the trail sweep; census 197→84, cumulative 599→84 (−86.0%) (2026-07-12)

FOUR LANES, all model-translated (model_sd's order layer is the
spec — Bryan: "the order layer is already 0-div in the model, so
it's translation again"):

1. **⚖ the k0 fold/churn law (tms_churn_del_group; model
   fold_on_drop, the gt3/d4 + gt6/x11 dump truths)**: a justifier's
   belief-drop CHURNS the del-group — rules with a positive join
   and a NOT matching the dying belief's type force-evaluate BEFORE
   the drain's retract (consuming the staged blocker-ins: block +
   queued-act cancel), so the un-break re-adds their lefts in the
   blocked list's PREPEND order = the re-derived firing order
   REVERSES (sdp7001x54: the oracle deletes P4..P1; the engine's
   CROSS-BATCH ins+del staging annihilation meant the deleter's not
   never saw the break, preserving t0 order). LAZY justifier: every
   del-group rule churns; EAGER: SINK ORDER — only rules DECLARED
   BEFORE the justifier (gt6/x11 net-out vs the x70-class churn).
   Wired at all four drain sites beside the D-198 sibling-eval.
   THE SINGLE BIGGEST MOVER OF THE ARC: census 197 → 101 on this
   lane alone — it swept the whole k0 ORDER family (45 cases), the
   lazy pick-order SET classes (x29/x52/x114 — the deleter re-derive
   order was the same mechanism), and the lazy set_break clusters.

2. **The composite last-key RIDE (bit16; model land_eager's
   composite re-route, sdp7004x51)**: an EAGER MUTFIRST composite's
   (bit1+bit2) key pushed with NO SURVIVORS — no alive fact still
   passing the positive pattern's alpha (all pmut'd/deleted) — is
   the run's LAST key: flags |= 16 at defer-push; the flush gates
   and the post-fire-continue exclude bit16 entries (they drain at
   drain[pop] ONLY = the model's drops[]/land_lazy). Mid-run keys
   keep the certified flush/selection landings. ⚠ a first cut
   gated the bit1 flush clause on run_live instead — WRONG
   DIRECTION: mid-run keys must land at selections even with an
   empty queue (the model's land_eager is not run_live-gated);
   x51 dropped to R1×1. The survivors-at-push routing is the
   model's exact shape. Fixes the 7-case eager-lead-set_break-mf
   class (oracle: the full R1-run completes, THEN the higher
   deleter fires on the LAST LK).

3. **The del-lane race widening (sdp7007x98/x79)**: tms's LATE
   check (the D-195 mutfirst race read at insertLogical) counted
   only ALPHA-BROKEN live members — a tuple member DELETED before
   the attach is the same race, del flavor (the model's x88/x0
   windows: mid-run LKs net out; the LAST generation rides to the
   pop and STRICTLY-HIGHER observers glimpse it once). Now a dead
   member ⇒ late (bit2) ⇒ with bit1 the composite machinery (lane
   2) routes its last key to the pop.

4. **The trail sweep (sdp7002x121)**: the same staging annihilation
   that starved the LEAD parked-del lane (D-199's sweep) starves
   TRAIL too — a foreign delete folding into the suppressed
   re-derived ins never reaches the terminal, so tms_parked_del
   never triggers. tms_p_death_sweep now covers trail as well;
   re-add order by notpos: LEAD = insertion (the model's land-lane
   law), TRAIL = REVERSED chain (sd_c1/gt16 — certified; sd_c1 and
   the fz_42_5213 alternation graduate stayed green through the
   widening). The parked-del lane remains for deaths that reach
   the terminal first.

RECEIPTS: corpus **11/1124/370** byte-identical after EVERY lane
(the ten D-200 graduates re-verified live-oracle each time); drift
bank **59** identical — ZERO xfail movement across all four lanes
(the entire blast radius landed inside the population surface);
agenda_open ×19 BYTE-IDENTICAL ×3 more (⚠ D-106); ladder 6/6 ×4;
validate_cells 39/39 ×4; check_witnesses 26/26; cargo test 9
suites; lint 1738/0/0. CENSUS: **197 → 84** (churn alone → 101;
+ lanes 2-4 → 84); model 0-div 12×150/150 on every run, all seeds
timeout-clean. Final table: 7001 8, 7002 11, 6001 3, 6003 8, 7004
6, 7005 6, 7006 5, 7007 6, 7008 10, 7009 9, 7010 5, 7011 7.
CUMULATIVE FROM THE PRE-PORT BASELINE: **599 → 84 (−86.0%)**.

COMPOSITION at 84: **64 = the d3/d5 no-amut runaway family
(PERMANENT — oracle runs away / engine terminates, the census
floor); 20 FIXABLE TAILS** — 12 ORDER (4× eager-ortwin-k0 twin
match order [sdp7001x97], 3× eager-trail-set_break-mf
[sdp7002x119], 2× eager-k0, 3 singles) + 8 SET (3× lazy-trail-None
[sdp6003x77], 2× lazy-trail-del incl. the x33 two-deleter
pick-value corner, 3 singles). No cluster exceeds 4; every
remaining case is a fine-structure corner of an already-landed
mechanism.

NEXT: the 20 tails (diminishing returns — each is a
one-to-four-case corner) or call the population slab COMPLETE and
proceed to I-RD per Bryan's order.
