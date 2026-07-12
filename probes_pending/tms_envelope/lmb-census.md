# L-MB opening census (D-190 tail, 2026-07-12): the mirror hypothesis

Cache sweep of all 18 A-marked witnesses (justifier RHS mixes
insertLogical with mutation — setter and/or delete):

- The over-firing rule is NEVER the A-rule itself — in 18/18 it is a
  third-party OBSERVER (deltas all engine-positive on observers;
  D-186's 16/18 over-fire census now fully attributed).
- OPENING HYPOTHESIS (the mirror of L-SD): the A-rule's RHS mutation
  triggers Drools' EAGER teardown — the D-076 tuple-member-break path,
  cancelling within the breaking WM action — while the engine defers
  the teardown to its uniform flush/pop timing. Engine transient
  outlives the oracle's ⇒ observers over-fire. L-SD showed the engine
  EARLY on the lazy row; L-MB shows it LATE on the eager row: ONE
  uniform drain where Drools has the certified two-path split. The
  uniform-fold signature.
- Ladder axes (next sitting; predictions pre-logged per house rule):
  k of the justifier × break kind (RHS delete vs alpha-breaking
  setter) × own-tuple vs foreign-tuple break × observer
  salience/decl-position (the L-SD queue-position instrument
  transfers). The certified k=1-scope pin (pr_tms_k2lazy: k≥2 dies
  LAZY even in Drools) is the first boundary to re-probe under
  compounds.
- Grammar note: A-shapes are OUTSIDE fuzz_tms_sd's v1 grammar — the
  population phase needs a v2 axis (justifier RHS setter/delete; an
  alpha-breakable field). gen.rs walls stay up regardless.

## The lead-NL cluster dossier (D-194 tail; gt13 observations —
## HYPOTHESES marked, not pins)

gt13 (x128-core: mf~nb~NL lead set_break @5, obs_p@7 + obs_j@7, P×2)
oracle: [RO1(1), RO1(2), RJ(1), RJ(2), RO2(LK2,P2), RO2(LK2,P1)].
Dump-grounded observations:
- RJ fired BOTH tuples back-to-back although RO2@7 gained staged work
  from F2's insertLogical — a mid-run insert did NOT preempt the run.
  HYPOTHESIS: insertLogical's WM/dirty effect lands at the inserting
  item's RUN END (consistent with every banked cell — no banked shape
  had a multi-fire run with a mid-run observer; deletes stay eager,
  c3a unaffected).
- LK1 died before RO2's eval; LK2 survived RO2's two firings —
  structurally identical inserts, different lifetimes. HYPOTHESIS:
  the mutfirst order gates dep attachment (upd-before-ins ⇒ the dep
  attaches after the tuple broke), with an LK1-vs-LK2 asymmetry still
  unexplained (first-update vs last-firing-of-run).
- RO2's pairing = reversed rtm-scan with P2 relocated to tail (the
  gt9 relocation law, reconfirmed).
NEXT INSTRUMENT: the TmsDump graft (BeliefSet per firing) — the
beta-memory dump cannot see belief staging; do NOT resume black-box
inference on this cluster (epicycle discipline).

## RESOLVED (same day, next sitting): the TmsDump lens ran

The BeliefSet lens (grafted into SdDump, D-194 handoff recipe) +
the gt14 sub-salience splitter settled the cluster — full trace
reading + the pinned ⚖ eval-consumption landing law + the model
port in `tmslens-predictions.md` / `tmslens-results.md`. Headlines:
H2 (mutfirst gates attachment) FALSIFIED (both deps attach at
insertLogical exec); H1's WM form FALSIFIED (the queue drains
BETWEEN the justifier's own firings — non-preemption is an
agenda-layer fact, the interposer arc's lane); H3 ANSWERED: the
LK1/LK2 asymmetry = mid-run between-firings eval (immediate
teardown, LK never observed) vs last-firing pop-landing (zombie-
justifier window: strictly-higher observers fire on it, sub-salience
never — gt14: RO3@3 silent 3/3). model_sd ported (eager last-firing
set_break → drops[]); validator 39/39 held; gt13+gt14 exact.
