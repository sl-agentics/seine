# TmsDump lens (BeliefSet per firing) — PREDICTIONS, logged pre-run

_2026-07-12, the D-194 handoff's "next unit of work". Written and saved
BEFORE the enhanced SdDump was first run on any target (house
discipline; the rung1-predictions.md pattern). Instrument: the
BeliefSet lens grafted INTO SdDump (one dump, both lenses) — per
firing + PRE-FIRE/FIRE-BOUNDARY, for every WM handle: equality-key
status (STATED/JUSTIFIED), BeliefSet size, each LogicalDependency's
justifier rule + activation tuple + isActive, the belief set's staged
WorkingMemoryAction; plus the TMS-side equality-key map (belief
presence WITHOUT WM presence = zombie) and the session's pending
propagation-entry queue. First target: `graft_targets/gt13_leadnl_run.json`
×3 JVM launches (TMS lines carry no identity tags ⇒ raw diff must be
clean)._

## The question this run asks

gt13 (x128-core: RJ no-loop@5 lead-NL, amut=set_break, mutfirst,
breaks=False; obs_p RO1@7; obs_join RO2@7; P×2):

- ORACLE (banked, 3×): [RO1(1), RO1(2), RJ(1), RJ(2), RO2(_,2), RO2(_,1)]
  — LK1 := LK2(1,true) DIED before RO2's eval; LK2 := LK2(2,true)
  SURVIVED RO2's two firings.
- MODEL (model_sd @ 1d9f634, measured 2026-07-12 pre-run):
  [RO1(1), RO1(2), RJ(1), RJ(2)] and finals P-only — the model
  tears down BOTH beliefs at the update-break flush; the oracle tears
  down EXACTLY ONE. The asymmetry (H3) is the cluster's core unknown;
  the beta-memory dump cannot see belief staging, hence this lens.

## Observables the lens adds (per firing F0..F5 + boundaries)

1. ObjectStore presence of LK1/LK2 handles (WM view, sorted by id).
2. Per LK equality key: STATED vs JUSTIFIED, logical-FH id, BeliefSet
   size, dep list (justifier rule, justifier tuple facts, isActive),
   the key's staged WorkingMemoryAction (non-null = a logical
   retract/insert callback is QUEUED but not drained).
3. TMS-side key map vs WM store (zombie = key+belief without store
   handle; transient = store handle whose belief set is empty/pending).
4. The session's pending PropagationEntry queue (BeliefSystemLogical-
   Callback sightings = the retract in flight).
5. P1/P2 equality keys — expected NULL throughout (plain inserts never
   touch the TMS). A non-null P key would be a discovery.

## Pre-registered readings + what each predicts the trace shows

Common ground (high confidence, all readings): at F2 (dump runs after
RJ(1)'s consequence) LK1's handle is ALREADY in the ObjectStore
(insertLogical's store/TMS action is synchronous; only network/agenda
effects can defer — the D-194 dossier already saw RO2's staged work
appear at F2), key JUSTIFIED, BeliefSet size 1, dep justifier = RJ
with a (P1)-tuple. Symmetrically at F3 for LK2. I.e. BOTH deps attach
at insertLogical exec — H3's "did LK1's dep ever attach" resolves YES.
FALSIFIER: size 0 or missing key at the inserting firing's own dump ⇒
dep attachment is itself deferred, and H2 gets a stronger form.

- **P-A (composite; the reading I give highest prior). RUN-EVAL vs
  POST-RUN teardown split**: RJ(2)'s admission eval processes P1's
  staged update mid-run (the executor re-evaluates RJ's network between
  its firings); the tuple-object for RJ(1) breaks THERE and the dep
  cascade is taken at eval ⇒ F3's dump shows LK1 already dead (store-
  absent, or belief 0 + retract callback pending/drained). P2's update
  flushes only at RJ's RUN END, when RJ(2)'s match has already fired
  and its item demotes; the update-break teardown path for a fired,
  no-longer-queued match does NOT cascade the dep (the D-193 "lazy
  prior scoped to update-breaks") ⇒ LK2's key stays JUSTIFIED size-1
  through F4, F5 AND FIRE-BOUNDARY, its justifier tuple DEAD in the
  beta lens (a zombie justifier; the belief leaks). wmAction stays
  null for LK2; the pending queue never carries a callback for it.
  ⇒ model law if confirmed: amut update-break dep-cascade lands at
  the next network EVAL that consumes the staged update; a break whose
  flush lands at/after the justifier item's run end never cascades.
- **P-B. Uniform run-end landing, order-split inside the flush**: both
  teardowns land at RJ's run end, but the flush interleaves so LK1's
  dep is attached-then-cascaded while LK2's retract callback is
  created and then STARVED/CANCELLED by RO2's higher-salience run.
  Trace: at F3 LK1 still size-1 (NOT dead — the anti-P-A marker); at
  F4 LK1 gone AND LK2 shows size 0 + wmAction NON-NULL (or a
  BeliefSystemLogicalCallback visible in the pending queue) while RO2
  fires on it; FIRE-BOUNDARY then shows LK2 either drained (dies at
  boundary — oracle finals must lack it) or the callback leaked.
- **P-C. Teardown landed for both, retract drained late**: as P-B for
  LK2 (size 0 + pending callback during F4/F5) but the callback DRAINS
  after RO2's run ⇒ LK2 ABSENT at FIRE-BOUNDARY. P-C and P-B(leak)
  are separated by the boundary dump + oracle finals.
- **P-D (H2-strong). LK2's dep never effectively attached** (orphaned
  by the mutfirst staged break): F3 shows LK2 key JUSTIFIED but size 0
  (or dep whose justifier match is already dead/inactive at attach) ⇒
  nothing ever cascades, LK2 leaks. Separated from P-A by SIZE at
  F3..boundary (P-A: 1; P-D: 0) — this is why the lens prints size,
  not just presence.

H1 (run-end WM landing) refinement expected: store-presence at F2 +
staged-but-unevaluated network work (already seen) ⇒ H1 narrows to
AGENDA/EVAL landing, not WM landing. FALSIFIER: LK1 absent from
getFactHandles at F2 ⇒ true deferred store landing (strong H1).

## Discipline

Oracle 3× (3 JVM launches, diff; TMS lines tag-free so the diff is
raw). Any cross-launch instability ⇒ quarantine per fz_42_84 doctrine.
No model edit before the trace is read against ALL FOUR readings; the
⚖ method law (what OTHER mechanism produces the same trace?) applies
before any pin. gen.rs walls stay up; engine untouched.
