# DECISIONS.md — running log

Append-only log of semantics probes, tie-break discoveries, design decisions,
and known limitations. Each checkpoint ends with a handoff note.

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
