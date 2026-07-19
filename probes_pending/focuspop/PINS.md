# PINS — D-345: fz_342002_1206 (Bryan named the witness) — the
# focus-pop preemption race

2026-07-19. The fork (re-measured): ONE adjacent swap, firings
7/8 — oracle R0(zz) then R2(""); engine R2("") then R0(zz).
Firing SET + facts identical. Composition decode: firing 6 =
R2(zz) does setFocus(gb) + modify(f2:=true); the modify births
R0(zz)'s re-activation (R0 binds $f2 → listens f2; R3 binds
f0/f1 only → correctly silent); gb is EMPTY at the push → pops
→ the engine RESUMES the in-progress R2 executor; Drools
re-selects by (salience, loadOrder) — R0 (load 0) preempts R2
(load 2). The D-333 eq-load-order tie composed ACROSS a focus
push/pop — the halt-check preemption ran against gb (empty),
not MAIN's refreshed queue. TMS/insertLogical/R1/T0 =
predicted scaffolding.

## The grid (predictions REGISTERED before any cell runs;
## oracle 3x)

Shape: A (load 0, sal 0): T1(f0 contains "zz", $x : f2) — the
modify-reborn rule; B (load 1, sal 0): T1(f2 == false) →
setFocus(g) + modify f2:=true; C (load 2, group g): T1() → end.
Facts: T1("zz",F), T1("x",F), T1("y",F) — B has 3 acts; after
B(zz)'s modify the race is A(zz-reborn) vs B(y-remaining).

- mz1_minimal: PREDICT (high) the witness fork reproduces
  scaffolding-free: oracle ...B(zz), A(zz), B(y); engine
  ...B(zz), B(y), A(zz). (TMS + R1 + the second focus cycle are
  scaffolding.)
- mz2_nofocus (B does NOT setFocus; C in MAIN at sal 6):
  PREDICT (high) MATCH both sides — the certified D-333
  eq_decl_preempt path handles the modify-birth race when no
  focus push intervenes. Isolates the FOCUS as the ingredient.
- mz3_empty_group (C constrained to never match — the push is
  a pure push/pop of an empty group): PREDICT (med) the engine
  STILL forks (the defect is the pop-RESUME skipping the
  preemption re-check, not C's firings); a MATCH here would
  relocate the defect into C's firing window.

## Grid MEASUREMENTS (2026-07-19, oracle 3x each)

- mz1_minimal: MATCH both sides — the HIGH prediction MISSED
  (recorded). A real focus cycle (C fires, g drains, pop)
  re-selects correctly; the witness's FIRST focus cycle was
  never the carrier.
- mz2_nofocus: MATCH (hit) — the plain D-333 path is healthy.
- mz3_empty_group: FORKS (the med prediction HIT): oracle
  [A, B(zz), A, B(x), B(y)]; engine [A, B(zz), B(x), B(y), A].
  THE INGREDIENT IS THE EMPTY PUSH: setFocus of a group with no
  live activations → the D-258 late-continue keeps l past an
  equal-salience DECL-PRECEDING own-group member born from this
  firing's modify. Matches the witness exactly (its SECOND
  setFocus pushes gb when R3 has no fresh acts).

## THE NAIVE PORT — MEASURED AND REVERTED (the D-331 protocol)

The candidate gate (own_tie: equal-salience decl-preceding
queued member of l's OWN group blocks the late-continue) fixed
the witness + mz3 but the byte gate measured TWELVE certified/
failures-tier movers — fz_9001_1795/2060/2573, fz_9002_1011/
5814, fz_9003_2151/2190/4514/486, pr_af_fz_9102_7658,
pr_af_g7_tie_hfirst, agenda_open/fz_9003_6467 — ALL oracle-diff
FAIL with the gate in. ENGINE REVERTED (byte gate 2517/2517
IDENTICAL post-revert). The D-106/D-258/D-320 halt surface is
an EMPIRICALLY CALIBRATED approximation (the 88-witness halt
matrix, 10 configs, "every blocker-pool variant measured
WORSE") — a bare own-group tie check is too coarse: those
witnesses contain equal-salience decl-preceding queued members
whose certified behavior is the CONTINUE.

REFINED HYPOTHESIS for the next round (unverified): the mz3
discriminator is FRESHNESS — the preempting member's activation
is born DURING l's firing (the RHS modify), where the halt
matrix's continue shapes carry members queued BEFORE l's pick
(which took l as best over them... note the pick-order argument
says an equal-sal lower-decl QUEUED member could not have been
passed over — so the matrix members are likely queued-DIRTY
empty-at-pick or born under different focus tops; the model
must say). NEXT-SLAB REQUIREMENTS (its own slab, model-first —
the D-333 model_check pattern): enumerate candidate halt laws
(freshness-gated tie, dirty-stamp windows, af_live composition)
against the FULL halt-matrix population (the 88 witnesses + the
12 movers + mz1-3 + the fz_342002_1206 witness) before any
engine edit. fz_342002_1206 KEEPS ITS XFAIL SEAT.

# D-346: the halt-law MODEL round (Bryan: "do the focus-pop
# halt-law model round") — the law is FINER than every candidate;
# ENGINE UNTOUCHED (two exploratory edits made and REVERTED —
# recorded below for honesty; the round should have led with the
# model, and did after Bryan's course-correction).

## Source reading (drools-core/kiesession 9.44.0.Final, verbatim)

- setFocus from a RHS is DEFERRED (AgendaGroupQueueImpl.setFocus →
  addPropagation(SetFocusAction)); at the post-firing flush,
  internalExecute pushes and — ONLY if the push really changed the
  top (the focusStack.getLast() != group guard; already-top
  setFocus is a no-op) — calls haltGroupEvaluation(), a flag on
  the GROUP EVALUATOR (exits its per-group item loop; checked
  BETWEEN executors, not inside one).
- RuleExecutor.haltRuleFiring (between one rule's firings):
  evaluateEagerList → peekNextRule = focusStack.peekLast().peek()
  — THE TOP GROUP ONLY, empty top peeks null → keep control; halt
  on a foreign-group top item (any salience) or an own-group item
  strictly preceding per RuleAgendaConflictResolver.
- getNextFocus pops empty auto-deactivate tops (plain
  agenda-groups pop when empty — probe-confirmed POP events);
  MAIN never pops. AgendaGroupsManager keeps DUPLICATE stack
  entries (no relocate — the engine's relocate-or-push is an
  approximation).

## The two probe streams (MzProbe / Mz2Probe — the central
## unresolved contradiction)

mz3 shape: FIRE B(zz) → PUSH g → MATCH+ A → POP g → FIRE A —
the executor YIELDED after ONE firing past an EMPTY pushed top.
x35 shape: FIRE R2('') → PUSH g → FIRE R2(x) → FIRE R2('') →
POP g — the executor CONTINUED through its whole list past an
EMPTY pushed top, under the SAME visible configuration (real
push, empty g, equal-salience decl-preceding member re-birthed
by the firing's own modify). Neither the source's flag plumbing
nor any black-box feature tested below explains both.

## The model (tools/model_check_focuspop.py, the D-333 pattern)

Alpha-only agenda scenarios (single-pattern rules, exact match
computation), oracle-diffed; laws: keep (the engine's D-106
peek), yield (real-push yields; no-op never), yieldall, naive
(the reverted D-345 tie gate). 900 cases / 3 seeds
(346001/2/3), population at $CLAUDE_JOB_DIR/tmp/focuspop_pop:
  keep 10+0+1 = 11 div;  yield 8+2+1 = 11;  yieldall same as
  yield; naive = keep. Model bug found+fixed en route (recorded):
  Drools' modify propagates on the SETTER MASK with NO value
  diff — setF2(true) on an already-true field still re-fires
  listeners (x49/x162 composed exactly).
NO candidate fits: the oracle itself forks on near-identical
shapes — x31/x159/x267/x282 (+ mz3, the witness) behave
YIELD-style; x71/x134/x35 (+ the 879 family) KEEP-style.

## Killed discriminator hypotheses (each with its counterexample)

- fresh-INSERT-birth vs UPDATE-refire of the preemptor: x282
  (update-refire, yields) vs x134 (update-refire, keeps).
- the firer's own listen-mask hit: x282 (firer listens f0 only,
  yields) vs x35 (same, keeps); mz3 (listens f2, yields) vs
  x31's R3 f2f (yields) — no split.
- the pushed group's history (never-lived vs fired-and-emptied):
  mz3/x282 never-lived+yield BUT x31 fired-and-emptied+yield.
- salience level, adjacency of decl positions: equal-sal pairs
  on both sides of the fork.

## The engine vs this population

The CURRENT engine (all its af_live/af_linger/tie_preempt fine
structure) FAILS 6 of the 7 discriminators (x31, x159, x267,
x282, x71, x35; passes x134) — WORSE than the plain-keep
abstraction on this space. The alpha-only setFocus×modify
population is an under-covered divergence CLASS, not one
witness; the 7 cells are copied into this lane as data.

## Exploratory engine edits (both REVERTED, byte-verified)

(1) blanket yield-on-real-push: 67 corpus movers (the pick
path's eval windows differ from the calibrated keep-control
machinery even when the re-pick chooses l). (2) narrowed
materialized-tie yield: reverted un-measured when the model
round superseded it. Engine byte-identical to a9c11ee-era
bytes; the reverts are exact.

## NEXT (its own session)

The model needs the state my abstraction lacks: RuleAgendaItem
lifecycle (dequeue/requeue on update-refire, heap positions),
group active flags, and the propagation-flush batching order —
OR a mechanism-level trace (instrumented Drools build / JDI) on
the mz3-vs-x35 pair to SEE what exits B's executor. Only after
the unique survivor: ONE engine port, narrowest-gate
implementation, full battery. fz_342002_1206 KEEPS ITS SEAT;
the 7 population cells are the acceptance grid.

# D-347: THE MECHANISM TRACE ROUND (Bryan: "do the halt-law
# mechanism trace round") — THE LAW CLOSES; witness GRADUATED

## The trace (classpath-shadowed instrumented drools-core —
## the D-093 vendor pattern, scratch-only; RuleExecutor +
## AbstractGroupEvaluator + AgendaGroupQueueImpl instrumented)

mz3: FIRE B(zz) → PUSH g → **peekNextRule=C@g** → halt-check
TRUE → yield. The "empty" pushed group held C's RuleAgendaItem
— queued by AlphaTerminalNode.modifyObject via
byPassModifyToBetaNode: **a modify whose mask misses an alpha
constraint's listened properties BYPASSES the stateless alpha
and touches the rule's path REGARDLESS of the fact's
membership** (stack traces verbatim in the round log). x35:
peekNextRule=null → continue (R1's f2-listening alpha
re-evaluated, failed, queued NOTHING) — and the item-add trace
shows the second delta: **alpha-EXITS never queue the item**
(R2's modify adds R0's item — the ENTERING rule — only).

## THE COMPLETE LAW (all prior contradictions compose)

1. RHS setFocus defers (SetFocusAction); already-top = no-op;
   a real push sets the GROUP EVALUATOR flag (exits its loop
   between executors only).
2. The executor's between-firings check peeks the focus-top
   group's ITEM queue: null → continue; foreign item → halt;
   own-group strictly-preceding → halt.
3. Items queue on: alpha-passing inserts, mask-hit re-evals
   that PASS, and BYPASSED modifies (mask ∩ alpha-constraint
   listen = ∅, membership-blind). Items do NOT queue on
   alpha-exits. removeRuleAgendaItemWhenEmpty on evaluated-empty.
The population fork: yield-style cells all had f0-only-listen
group rules (bypassed by the f2 modify → phantom item → halt);
keep-style cells had f2-listening group rules (re-eval → fail →
no item). Every D-346 contradiction composes.

## Model validation (BEFORE the engine port)

The bypass machine (item lifecycle + evaluator flag) added to
model_check_focuspop.py: **0 divergences over 1400 oracle cases
(seeds 346001/2/3 + 500 @ 347001)** — the unique survivor;
keep and yield each fail their known cells.

## THE PORT (two deltas, both trace-verbatim)

1. on_update epilogue: the BYPASS TOUCH — rules of the
   alpha-terminal class (plain single-positive-pattern; the
   model-certified scope) whose alpha-constraint mask (cmps
   fold; bindings are not alpha nodes) misses the update mask
   get queued/dirty/af_live — no staging; the stateful lazy
   eval finds nothing and unqueues (Drools'
   removeRuleAgendaItemWhenEmpty).
2. on_update exits (pass B): an exit-only notify of an
   alpha-terminal rule does not arm af_live (snapshot/restore
   around the notify) — no Drools item-queue on exits.
The existing D-320 af_flush machinery then yields exactly where
Drools' item-peek halts. Multi-pattern bypass touches are
UNPROBED (recorded — the alpha-terminal scope is what the model
certifies).

## Receipts

Acceptance: the witness + mz1-3 + x31/x159/x267/x282/x71/x134
ALL PASS (10 graduations: pr_fp_*); x35's residual is a NEW
CLASS (rule-level control now correct; R0's within-rule
activation-REQUEUE order at its final run) — BANKED as
scenarios/xfail/fp346003x35.json, bank 17 (1206 out, x35 in).
make diff 11/1541/414 + drift 17 identical (THE ENTIRE
D-106/D-258/D-320 halt matrix + the D-345 12-mover family stay
green — the mechanism-derived law preserves the calibrated
surface); byte gate 2519/2527 = EXACTLY the 8 lane cells; lint
2400/0/0; cargo 74; pytest 260; demo True; SD census 71 EXACT;
agenda_open x10 identical x3; model_ird 31/31; IRD 0-div x5;
fuzz 2x2000 seeds 345001/345002 + cep 3x300 seeds 345901-903
ALL CLEAN; NEXT seeds 348001+.

# D-348: the x35 requeue-order round (Bryan: "do the x35
# requeue-order round")

x35 re-measured (3x stable both sides): R0's FINAL run — oracle
[''#0, x, ''#2] = the order R2's modifies touched R0 (births
''#0, x; requeue ''#2); engine [''#2-first phase consume]. THE
LAW CANDIDATE (from D-347's trace): ALPHA-TERMINAL rules
materialize matches EAGERLY per propagation
(AlphaTerminalNode.assertObject/modifyObject at each flush) —
the tupleList accumulates in PROPAGATION order, ins and upds
INTERLEAVED; the engine batches the staging and consumes
upds-then-ins at one eval.

## The rq grid (predictions REGISTERED before cells; A sal 10
## fires initial T-facts first; B sal 5 modifies all its facts
## uninterrupted (A at 0 cannot preempt); A's re-run = the
## measurement). A: T1(f2 == true) sal ...; B: T1($x : f0)
## salience 5 → modify setF2(true). Facts f0 = '', 'x', 'q'.

- rq1_mixed (facts ''F, xF, qT; A sal 0 so q fires only in the
  final run... no — mirror x35: A must FIRE q BEFORE the
  requeue: A sal 10 → fires q first; then B; then A's re-run):
  effects birth(''), birth(x), requeue(q). PREDICT oracle
  ['', x, q] (propagation order); engine phase-consume (q-upd
  first or ins-head-first) ≠ that — record what lands.
- rq2_ins_only (all F): batch = births only. PREDICT MATCH
  (high): both ['', x, q] FIFO births.
- rq3_upd_only (all T; A fires all 3 first): batch = requeues
  only. PREDICT oracle ['', x, q] (propagation order); engine
  upd-list head-first = REVERSED [q, x, ''] (med — the staged
  upd prepend).
- rq4_interleave (''T, xF, qT): effects requeue(''), birth(x),
  requeue(q). PREDICT oracle ['', x, q]; engine phases split
  them (upds together, ins apart) — any non-['', x, q] order
  confirms the phase-batching fork.

## rq1-rq4 MEASUREMENTS: all four MATCH — the grid design MISSED
## (recorded): A@sal-10 preempts B after every firing, so the
## staging never batches. THE INSIGHT THE MISS BOUGHT: x35's
## batch forms BECAUSE of the setFocus push — the peek-top-only
## law hides MAIN's equal-salience decl-preceding item from the
## halt check, letting B run uninterrupted. The batch-order
## surface is REACHABLE ONLY UNDER A FOCUS PUSH (or salience
## inversion, where the engine already matches). rqb grid: x35's
## exact 3-rule structure (A f2t decl0 sal0; G-in-g f2f decl1
## sal2 setFocus; B bare decl2 sal0 setFocus+modify), varying
## the fact mix:
- rqb_ins (all F): A batch = births only. PREDICT (med) oracle
  ['', x, q] propagation order; engine ins-consume order —
  whatever lands names the engine's phase.
- rqb_upd (all T): A fires all 3 initially; batch = requeues.
  PREDICT oracle ['', x, q]; engine upd-head-first [q, x, ''].
- rqb_mix (''T, xF, qT): requeue(''), birth(x), requeue(q).
  PREDICT oracle ['', x, q]; engine splits the phases.

## rqb MEASUREMENTS + THE D-348 PORT

rqb (x35's exact structure, fact-mix axis, oracle 3x): rqb_ins
MATCH, rqb_upd MATCH (the upd-reversal prediction MISSED —
recorded: pure batches are fine), rqb_mix FORKS (oracle tail
[x, q] = propagation order; engine [q, x] = upds-then-ins).
THE LAW: only MIXED ins+upd batches fork — the alpha-terminal
tupleList accumulates per-propagation with ins/upds interleaved
by arrival; the engine's per-window phase consume merges a
whole RHS run into one window and splits the phases.

THE PORT: every WM-mutating RHS action (Insert/InsertLogical/
Update/Delete) closes the k=1 s0 windows (execute_rhs,
post-match) — one D-047 window per RHS effect records the
arrival interleave; the certified per-window phase consume
(pr08/pr04) then reproduces it untouched. Byte-neutral for
single-phase sequences by construction (per-window consume ≡
merged-window consume unless phases interleave) — ONLY the
mixed class moves.

## D-348 receipts

Acceptance 8/8 (x35 + rqb x3 + rq1-4). Byte gate 2526/2527 vs
pre-port HEAD — the ONE diff is the witness itself; make diff
11/1549/414 + drift 16 identical (pr08/pr04 and every RHS-chain
cell stay green); lint 2408/0/0; cargo 74; pytest 260; demo
True; SD 71 EXACT; agenda_open x10 x3; model_ird 31/31; IRD
0-div x5; fuzz 2x2000 seeds 348001/348002 + cep 3x300 seeds
348901-903 CLEAN; NEXT seeds 349001+. EIGHT graduations
(pr_fp_x35_requeue + pr_fp_rqb_* + pr_fp_rq1..4 — the rq grid's
design-miss cells kept as controls); bank 17→16. THE FOCUSPOP
LANE LEDGER IS EMPTY (D-345/346/347/348: witness → model →
trace → both ports). Round misses this slab: rq grid design
(A@10 preempts — the miss that NAMED the focus-masking
ingredient), rqb_upd reversal prediction.
