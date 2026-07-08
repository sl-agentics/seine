# DECISIONS.md — running log

The body below is an **append-only** log of semantics probes, tie-break
discoveries, design decisions, and known limitations (each checkpoint ends
with a handoff note). The block just below — **CURRENT STATE** — is the ONE
mutable part: a living summary, **overwrite it each checkpoint** so a fresh
session orients here instead of reading 4900 lines. Keep it short; put the
detail in a D-entry below and the active-slab detail in the plan file.

---

## CURRENT STATE  (living summary — overwrite each checkpoint)

_Last updated: 2026-07-07 (run `git log --oneline -8` for the live HEAD —
this line lags by its own commit)._

**Repo:** Seine — differential-tested Rust port of a bounded Drools
9.44.0.Final subset. **Prime directive: PROBE-FIRST** — the oracle settles
every semantic; never hand-derive PHREAK/temporal staging (it flip-flops).
Workflow, env quirks, and doctrine live in memory `seine-workflow.md`.

**Git:** on `main`, **several commits UNPUSHED** (don't push without Bryan).
Key commits: `8018ea2` item A inference, `79c6b95` item B recon+parser,
plus this CURRENT-STATE block. Build clean. Gates green: baseline 11 /
probes 729 / regressions 281 byte-identical; lint 1069; 8 Rust suites;
72 pytest. Verify with `make diff` / `make lint-probes` / `cargo test`;
oracle prebuilt (`oracle/target/classpath.txt`). If any gate is red on
resume, something drifted — investigate before building on it.

**Landed:** v0.4.0 (`5b23e7c`) = CEP E1 + Engine::reset + agenda groups +
queries×mutation + structured aggregation. Data-types arc (nulls/decimals,
D-096–098). TMS, P1c group CEs, hardening waves — see the log.
CEP **E2 item A** @expires inference (D-109, `8018ea2`): reach + transitive
STP closure + the never-overwrite (bare/backward → NEVER).

**ACTIVE FRONTIER — CEP E2 item B (windows).** Recon + parser plumbing
committed (D-110, `79c6b95`); `window:time` WALLED at engine-compile.
**RESUME HERE — build the window runtime:** (1) per-subtree EVICTION = a
scoped right-delete at the windowed accumulate node (Phase-B @
`engine.rs:5162` → Phase G), scheduled at `ts+N` via a new window-deadline
BTreeMap drained in `advance()`, WITHOUT killing the fact; (2) the A→B SEAM
(fold `window:time` size−1 into `temporal_ub` + exempt windowed patterns
from `never_inferred`). Then un-fence `probes_pending/cep/win*`, `make diff`,
extend `fuzz_cep.py`. Full detail: `~/.claude/plans/graceful-waddling-stallman.md`.
Deferred to a model-check sub-recon (own gate): window × STREAM-flush / TMS /
node-sharing / length-under-mutation.

**Open/deferred:** E1-hardening — 2 temporal-join-order xfails
(`scenarios/xfail/xf_cep_tjorder_*`, bisect-confirmed pre-existing); D-080
TMS envelope; E2 remaining after B: C event update/delete, D entry-points,
E @duration. Upstream: #2366 filed (min/max), `docs/drools-inferred-expiry-never.md`
drafted (window/inference never-leak). Window:length is walled (follow-on).

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
