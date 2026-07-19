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
