# Seine

A Rust reimplementation of a **bounded subset** of the Drools DRL
forward-chaining rule semantics, proven faithful by **differential testing
against real Drools** (pinned: **9.44.0.Final**) as a live oracle.

> Status: Phases 0–2 complete, plus the Phase-3 stretch items
> `matches`/`contains`/`in`, `not`/`exists` and `accumulate`/`collect`
> with the built-in functions (custom accumulate functions not
> started). Curated corpus (360 scenarios, incl. 137 named fuzz
> regressions) at 100% with NO subset wall (mutation, 3-pattern rules and
> CEs mix freely). The engine core is a behavioral port of the PHREAK
> node algorithm (`engine/src/phreak.rs`) — staging sets, beta memories
> with child-list cursor threading, existential blocker lists with
> comparison (range) indexes, property-miss reAdd, and agenda-item
> lifecycle incl. queue-on-unlink, each pinned by probe scenarios. See
> `DECISIONS.md` (D-001…D-037) for every oracle-pinned semantic.

## What this is

- `engine/` — `seine-engine`: the Rust forward-chaining engine (arena-backed,
  id-based, columnar-friendly working memory from the first commit).
- `harness/` — `seine-harness`: Rust scenario runner + comparator.
- `oracle/` — Java reference runner: loads the same scenario, runs it through
  real Drools 9.44.0.Final, emits the same canonical result JSON.
- `scenarios/` — the test corpus (curated golden-master scenarios; fuzz
  generators arrive in Phases 1–2).

Equivalence bar: identical **final fact set** AND identical **ordered firing
log** for every in-subset program.

## Supported subset (target; grows by phase)

- Phase 1: single-pattern rules; typed fields (`i64`, `f64`, `String`, `bool`);
  operators `== != < <= > >=`; variable/field bindings; `insert` on the RHS;
  `salience`; `no-loop`; oracle-pinned conflict resolution.
- Phase 2: multi-pattern joins on bound variables (up to 3 patterns,
  self-joins included); `update`/`modify`/`delete` with oracle-pinned
  re-evaluation and re-firing semantics (PHREAK property reactivity,
  staging batches, eager/lazy evaluation windows, beta-memory child
  sync-walks, and agenda-item lifecycle — see `DECISIONS.md`
  D-013…D-028). The former mutation/3-pattern subset wall (D-017) is
  lifted.
- Phase 3 (stretch, landed): operators `matches` (full-string
  java.util.regex semantics over a tame regex subset: literals, `.`,
  classes with ranges/negation, groups, `|`, `* + ?`), `contains`
  (String substring), `in`/`not in` (literal lists) — String fields only
  for matches/contains, literal-only operands (D-030); `not`/`exists`
  conditional elements, including first-position CEs (matched on
  `InitialFact`), constrained CEs with hash- or range-indexed blocker
  search, and oracle-pinned cancellation/refire lifecycle (D-031/D-032).
  Bindings inside CE patterns are rejected; the type name `InitialFact`
  is reserved. Node sharing is modeled with a TRUE shared prefix trie
  (D-037): rules with structurally equal pattern prefixes — identity
  includes the bound-field SET and the names of any variables referenced
  in constraints (D-036/D-037) — share one node instance that evaluates
  once per agenda window; the first-built sink receives each batch
  preserved, later sinks reversed (identical-LHS rules fire their
  activations in opposite orders, faithfully). No sharing wall remains.
- Phase 3b landed: `accumulate` (sum/count/average/min/max) + `from collect`
  with bit-exact float op sequencing (D-038..D-041). One documented-open
  order corner (D-042, scenarios/xfail/).
- Salience expressions landed (D-043): computed salience over bindings with
  the full dynamic-agenda lifecycle, certified zero divergences over 5 seeds.
- Python bindings, Layer 1 (D-044): `pip`-able `seine` module — Arrow
  tables in (polars/pyarrow zero-copy), one-shot sessions, WM-delta +
  firing-audit results, observer callbacks. The boundary adds zero
  semantics (bit-exact marshaling, loud null/type rejection, native-
  parity tests).
- Python bindings, Layer 2 (D-045): Pythonic authoring — @seine.fact
  classes and a Rule builder that COMPILE TO DRL TEXT, so the
  differential guarantees cover Python-authored rules verbatim; every
  certified wall is a definition-time CompileError.
- Multi-fire certified (D-046): insert -> fire -> insert -> fire on one
  session (epoch scenarios, 5-seed campaign clean); sessions are no
  longer one-shot and every fire() returns its own WM delta.
- External update/delete by handle certified (D-047): the full WM
  lifecycle crosses the Python boundary (session.update/delete between
  fires, changed-fields property masks, action-ordered k=1 windows).
- Row-object sugar + CI (D-048): lists of @fact instances/dicts/
  dataclass-or-Pydantic objects ingest directly; `pip install seine-rs`
  (import stays `seine`); GitHub Actions runs the differential gate and
  builds wheels.

## Explicit non-goals (hard walls)

- MVEL dialect (only the minimal Java-like expression subset above).
- DMN, CEP / temporal operators, complex event processing.
- Backward chaining, queries, truth maintenance beyond Phase-2 mutation needs.
- Workbench / KIE tooling / full DRL6 grammar / decision tables / templates.
- Persistence, marshalling, session clustering, multithreaded firing.
- Beyond-RAM / disk-backed working memory (the columnar id-based layout keeps
  it *reachable*; building it is out of scope).
- Anything requiring network calls or external state at rule-fire time.

## Running the harness

Prereqs: Rust stable, JDK 17+, Maven with access to Maven Central (Drools
9.44.0.Final and transitives).

```sh
make diff          # run every curated scenario through both engines and compare
make test          # pure-Rust unit + characterization tests (no JVM)
make fuzz          # 10k-case differential fuzz (SEED=n CASES=n to vary)
make oracle        # build the Java oracle runner (once)
```

The fuzzer is seeded and deterministic (case k of seed s is always the same
program). Divergent cases are saved to `scenarios/failures/` automatically;
every resolved divergence graduates to a named regression scenario in
`scenarios/regressions/` (137 of them — each one pinned a real PHREAK
semantic documented in `DECISIONS.md`).

## Provenance & licensing

Licensed **Apache-2.0** (see `LICENSE`, `NOTICE`). This is a behavioral
reimplementation: semantics are captured from the observable behavior of
Drools 9.44.0.Final via probe scenarios (all kept in-repo as regression
tests). The upstream Drools sources (Apache-2.0) have been consulted to
understand internal data structures where black-box probing hit its limits
(see `DECISIONS.md` D-026); no Drools source code is copied or
transliterated here, and every implemented behavior is pinned by an oracle
probe or regression scenario, not by the source text. "Drools" is a Red
Hat trademark; this project is not affiliated with or endorsed by Red Hat
or the KIE project.

## Development

Three gates, all green before merge:

```sh
# 1. native tests (no JVM needed)
make test

# 2. the differential gate: every scenario through BOTH engines.
#    Requires JDK 17+ and maven (the oracle is real Drools 9.44.0.Final):
cd oracle && mvn -q -DskipTests package && cd ..
make diff

# 3. Python bindings (any venv):
pip install maturin polars pyarrow pytest
maturin develop --release -m bindings/Cargo.toml
pytest bindings/tests/
```

Engine changes additionally run the fuzz campaign before merging
(`cargo run -p seine-harness -- fuzz 10000 <seed>` over seeds
42/7/123/777/999 — zero divergences to completion; see DECISIONS.md for
the certification records). Semantics are pinned probe-first: never
implement a Drools behavior from intuition — write a scenario, run it
through the oracle, record the pin as a D-entry, then implement.
CI mirrors gates 1–3 and builds `seine-rs` wheels for
linux-x86_64 / macos-arm64 / macos-x86_64 / windows-x86_64 plus an
sdist.
