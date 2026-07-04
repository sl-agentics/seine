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
