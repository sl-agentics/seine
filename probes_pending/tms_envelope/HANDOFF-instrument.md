# COLD-START HANDOFF: enhancing the member-order/belief graft (D-194)

_Everything a fresh context needs to extend the instrument and attack
the three remaining v2 residue clusters. State as of `1d9f634`+:
v2 populations 738/750; 39/39 banked cells; the L-SD+L-MB executable
spec is `model_sd.py`; ZERO landing/mechanism divergences in 750
A-shape cases — everything open is member-order/belief-staging class._

## The instrument today

- `oracle/src/main/java/dev/seine/oracle/SdDump.java` — after every
  firing and action, dumps each beta node's ltm/rtm in physical
  iteration order (+ blocked chains, peer chains, per-path
  SegmentMemory staged-left lists). Reflection helpers: `call`/`call1`
  (walk superclasses, setAccessible). Build:
  `cd oracle && mvn -q -DskipTests package` (then run commands from
  the REPO ROOT — the cd hazard). Run:
  `java -cp $(cat oracle/target/classpath.txt):oracle/target/classes \
     dev.seine.oracle.SdDump <scenario.json>`
  Determinism: 3 launches, diff — everything so far has been 3/3
  stable (no hash texture).
- Targets banked: `graft_targets/gt1..gt13*.json` (each file's role is
  named in graft-phase1.md and lmb-census.md §dossier).
- The executable spec: `model_sd.py` (module docstring = the full rule
  set); validator: `python3 probes_pending/tms_envelope/validate_cells.py`
  (39/39, truths banked in `truths/*.ndj`); populations:
  `python3 tools/fuzz_tms_sd.py 150 <seed>` (v2 A-shape grammar;
  mismatches oracle-3× flake-filtered; engine census rides along).
  Fresh-seed protocol: any model edit → validator 39/39 → seeds
  7001/7002/6001/6003 (comparison base) → a NEVER-USED seed.
  Population cases regenerate deterministically:
  `from fuzz_tms_sd import draw; rng=random.Random(seed);
   cases=[draw(rng) for _ in range(150)]`.

## THE ENHANCEMENT (next unit of work): the TmsDump lens

The lead-NL quartet (x17/x90/x128/x131-class; dossier at the tail of
`lmb-census.md`) is BELIEF-STAGING structure the beta-memory dump
cannot see. The D-076-era TmsDump was session-scratch and is NOT in
the tree — re-create it INSIDE SdDump (one dump, both lenses):

- Per firing, for every LK2 FactHandle in the WM: reflect
  `((InternalFactHandle) h).getEqualityKey()` → key status
  (STATED/JUSTIFIED), `key.getBeliefSet()` → size + iterate the
  LogicalDependency nodes (justifier rule name + the dep's activation
  tuple facts) — drools-tms classes are on the classpath already
  (`classpath.txt` includes the tms module since D-076).
- Also print WM presence vs belief presence separately (the zombie/
  transient distinction) and the session's pending belief-system
  staging if reachable (TruthMaintenanceSystem via
  `TruthMaintenanceSystemFactory.get().getOrCreateTruthMaintenanceSystem(
   (ReteEvaluator/EntryPoint...))` — reflect defensively like the rest).
- First targets: `graft_targets/gt13_leadnl_run.json` ×3 launches,
  then quartet cores regenerated from the population seeds.

HYPOTHESES TO TEST (from the gt13 dump; marked hypotheses, NOT pins):
(H1) insertLogical's WM/dirty effect lands at the inserting item's
RUN END (a mid-run insert cannot preempt its own run — gt13's RJ
fired both tuples under a strictly-higher observer with staged work).
(H2) mutfirst gates dep attachment (upd-before-ins ⇒ dep attaches
after the tuple broke ⇒ the LK persists un-dropped). (H3) something
distinguishes LK1 (died by the next firing) from LK2 (survived the
observer's firings) in the same run — UNEXPLAINED; the BeliefSet
trace should show whether LK1's dep ever attached.

## The three residue clusters (9 cases; regenerate via draw())

1. lead-NL quartet — sdp7002x17, sdp6001x90, sdp6001x131,
   sdp6003x128 (+ gt13 as the minimal). TmsDump work, above.
2. mf-lazy-trail trio — sdp7002x68, sdp6003x41, sdp6003x88: the
   justifier's OWN continuation order after its churn (model fires 2
   next, oracle 3) — likely one SdDump run on an x68-core (the
   justifier's staged list at its second pop).
3. nb-trail tails — sdp7001x103, sdp6003x0: late deleter/obs order
   after non-breaking generations.

## Discipline (standing)

Predictions logged BEFORE every run (rung1-predictions.md pattern);
oracle 3×; ⚖ epicycle stop (no proxy toggles — a rule that needs one
doesn't know its mechanism; dump or fence); ⚖ method law (retraction
of D-191's headline at D-192 is the template); populations are
evidence, not proof; gen.rs walls STAY UP; engine untouched until the
port slab (validate-and-revert, Bryan's gate, agenda_open ×19
receipts, D-106 tripwire). Gates on resume: `make diff` (incl. the
xfail drift tier) + `make lint-probes` + validator 39/39.
