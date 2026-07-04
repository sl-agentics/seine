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
