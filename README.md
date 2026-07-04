# Seine

A Rust reimplementation of a **bounded subset** of the Drools DRL
forward-chaining rule semantics, proven faithful by **differential testing
against real Drools** (pinned: **9.44.0.Final**) as a live oracle.

> Status: Phases 0–2; curated corpus (106 scenarios, incl. 50+ named fuzz
> regressions) at 100%. One documented open divergence class remains
> (~2 per 10k fuzz cases): activation requeue placement after repeated
> updates in join rules (`xfail/`, D-025 — next step is a faithful port of
> the PHREAK node algorithm's staging structures). See `DECISIONS.md` for
> the running log of every oracle-pinned semantic (D-001…D-025).

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
  staging batches, eager/lazy agenda evaluation, and the unified update
  cascade — see `DECISIONS.md` D-013…D-025). Subset wall: programs using
  `update`/`modify` are proven for rules of up to 2 patterns; 3-pattern
  rules are proven for insert/delete-only programs (D-017/D-025).
- Phase 3 (stretch): `not`/`exists`, `accumulate`/`collect`, `matches`/
  `contains`/`in`.

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
`scenarios/regressions/` (41 of them so far — each one pinned a real PHREAK
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
