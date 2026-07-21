# Seine

A Rust reimplementation of a **bounded subset** of the Drools DRL
forward-chaining rule semantics, proven faithful by **differential testing
against real Drools** (pinned: **9.44.0.Final+p1** — stock 9.44.0.Final
plus exactly one vendored upstream-merged fix for a defect Seine
reported; see `NOTICE` and DECISIONS D-163) as a live oracle.

> **Status: v0.4.48 on PyPI** (`pip install seine-rs`, import `seine_rs`).
> The certified corpus is **11 baseline / 1,681 probe / 414 regression
> scenarios, byte-identical against real Drools** across joins, node
> sharing, property reactivity, `not`/`exists` (incl. nested CE groups),
> `accumulate`/`collect`/`groupby`, recursive + pull queries, truth
> maintenance with a queryable justification graph, agenda groups,
> deterministic CEP (point + interval events, `window:time`, entry
> points, the full Allen algebra) — and the *order* of every firing, down
> to graft-derived replays of Drools' internal propagation. The coverage
> matrix lives in `FEATURES.md`; every semantic is pinned by an
> oracle-probe D-entry in `DECISIONS.md`.

## Quickstart

```python
import seine_rs as s

@s.fact
class Account:
    id: int
    balance: int            # cents; <= 0 == paid off

@s.fact
class Eligible:             # insertLogical: auto-retracts with its support
    account_id: int

rule = s.Rule("eligible")
acc = rule.when(Account, Account.balance <= 0)
rule.then_insert_logical(Eligible, account_id=acc.id)

sess = s.Session([rule])                 # schemas auto-registered from the rule
h = sess.insert_row(Account(id=42, balance=0))
res = sess.fire()

print(res.facts[Eligible].to_pylist())   # [{'handle': 1, 'account_id': 42}]
print(sess.why(1))                       # the justification: rule + support tuple
sess.delete(h); sess.fire()              # support gone -> Eligible auto-retracts
```

**Coming from Drools?** DRL here is **rules-only** — don't write
`package` or `declare`. Fact types live in Python (`@fact` classes or
the `facts=`/`schemas=` mappings) and the engine infers schemas from
them; DRL owns only the logic.

This block is pinned verbatim as `bindings/tests/test_quickstart.py`.
Rules author in Python but compile to DRL text (`rule.to_drl()` shows
it), so the differential certification covers Python-authored rules
with no translation gap — and anything outside the certified grammar
is a definition-time `CompileError` that names the fix. For a tool or
an LLM driving the API, that wall is the headline property: **you
cannot emit a wrong-but-accepted rule** — it compiles to certified
semantics or it errors with the correction. Guided walkthroughs of
every surface live in `demo/tours/`.

## What this is

- `engine/` — `seine-engine`: the Rust forward-chaining engine (arena-backed,
  id-based, columnar working memory; a behavioral port of the PHREAK
  node algorithm plus a deterministic pseudo-clock CEP runtime).
- `harness/` — `seine-harness`: Rust scenario runner + comparator + the
  main-axis fuzz generator.
- `oracle/` — Java reference runner: loads the same scenario, runs it through
  real Drools 9.44.0.Final, emits the same canonical result JSON (plus
  reflection-graft dump instruments used during recon).
- `scenarios/` — the tiered corpus: `baseline/` (adapted Drools-suite
  spec tests), `probes/` (D-entry pins), `regressions/` (graduated fuzz
  finds), `xfail/` (filed open divergences + quarantines, lint-enforced).
- `bindings/` — the Python package (`pip install seine-rs`): Arrow
  tables in, WM-delta + firing audit out; `@seine.fact` classes and a
  Rule builder that compile to DRL text so the differential guarantees
  cover Python-authored rules verbatim.
- `tools/` — fuzz generators and the executable order-model specs
  (`model_check_*.py`) that every firing-order port is validated
  against before it lands.

Equivalence bar: identical **final fact set** AND identical **ordered firing
log** for every in-subset program.

## The certified surface

`FEATURES.md` is the authoritative coverage matrix (implemented /
roadmap / can't / won't, one row per Drools feature with its pins).
The headline areas, each differentially certified:

- **Core matching** — typed scalar fields, the full comparison operator
  set with cross-type promotion, `matches`/`contains`/`in`, inline
  `&&`/`||`/`!()` groups, bindings, multi-pattern joins and self-joins,
  TRUE shared-prefix node sharing, property reactivity, `no-loop`,
  static + expression `salience`, deterministic conflict resolution.
- **Conditional elements** — `not`/`exists` anywhere in the LHS (blocker
  model, range indexes), nested `not(…and…)`/`exists(…or…)` RIA
  subnetworks, `or` with subrule expansion, InitialFact semantics.
- **Aggregation** — `accumulate` (sum/count/average/min/max, bit-exact
  float sequencing), `from collect`, `collectList`/`collectSet`,
  leading-position `groupby`.
- **Queries** — non-recursive + recursive (transitive closure), `or`
  bodies, positional patterns, `?query` pull CEs in rules, queries
  across mutation epochs, exact `getQueryResults` row order.
- **Truth maintenance** — `insertLogical` with value-equality keys,
  cascading retracts, and a **queryable justification graph**
  (`why(fact)` — the why-engine substrate).
- **Working-memory lifecycle** — multi-fire sessions, external
  update/delete by handle with property masks, `Engine::reset()`,
  agenda groups + focus stack.
- **Deterministic CEP** — `@role(event)` point + `@duration` interval
  events on a pseudo-clock (`advance()`), `after/before[lo,hi]` and the
  full Allen predicate algebra, `@expires` + inference (STP closure),
  `window:time`, named entry points, event mutation re-propagation,
  expiration×TMS composition — with **no wall clock anywhere**: same
  inputs, same firings, every run.
- **Firing-order fidelity** — beyond set-equality, the *order* of every
  activation is certified: per-arrival temporal flush models and four
  mechanical "shadows" replaying Drools' internal propagation
  (right-memory list order, staged backlogs, link-counter queue
  economy), each validated 0-divergence at population scale (tens of
  thousands of oracle scenarios) before its port landed.
- **Data types beyond Drools** — opt-in SQL three-valued nulls and
  exact `decimal(p,s)` (i128 fixed-point), certified against
  **DuckDB** as the ecosystem oracle (a deliberate, documented
  deviation from Java semantics).
- **Python bindings** — Arrow/polars zero-copy in, WM-delta + firing
  audit out, Pythonic rule authoring that compiles to DRL text, so the
  differential guarantee covers Python-authored rules verbatim.

One intentional divergence is documented and upstreamed: the Drools
min/max stale-extremum defect (apache/incubator-kie-issues#2366) —
Seine computes the correct value; the fix was merged upstream
(kie-drools#6796) and graduates here at the next oracle bump.

## Explicit non-goals (hard walls)

The full ledger with per-row rationale is `FEATURES.md` §3 (can't) and
§4 (won't). The short version:

- Embedded Java/MVEL: no expression interpreter, no `eval`, no user
  functions or custom accumulate functions — the closed constraint
  grammar is the boundary of the product.
- Object-graph facts: no nested property access, collections, OOPath,
  or class-model matching — facts are flat scalar arena rows.
- Wall-clock behavior: no timers/calendars, no realtime session clock,
  no `fireUntilHalt` live streams — CEP runs entirely on the
  deterministic pseudo-clock.
- Multithreaded evaluation: **single-threaded determinism is the
  product**; same inputs → same firing log, byte-for-byte.
- KIE platform, BPM/ruleflow, DMN/decision tables, persistence,
  marshalling, listeners-as-API, alternate engine modes/config knobs —
  one certified semantics, no configuration matrix.
- Anything requiring network calls or external state at rule-fire time.

## Running the harness

Prereqs: Rust stable, JDK 17+, Maven with access to Maven Central (Drools
9.44.0.Final and transitives).

```sh
make diff          # every corpus scenario through both engines, byte-compared
make test          # pure-Rust unit + characterization tests (no JVM)
make fuzz          # 10k-case differential fuzz (SEED=n CASES=n to vary)
make lint-probes   # probe liveness lint (fail loud, never pass silent)
make oracle        # build the Java oracle runner (once)
```

The fuzzers are seeded and deterministic (case k of seed s is always the
same program); `tools/fuzz_cep.py` and the `tools/fuzz_*order*.py`
population generators cover the CEP and firing-order axes. Divergent
cases are saved to `scenarios/failures/` automatically; every resolved
divergence graduates to a named regression scenario in
`scenarios/regressions/` (302 of them — each one pinned a real PHREAK
semantic documented in `DECISIONS.md`), and filed open divergences are
quarantined in `scenarios/xfail/` with their findings.

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

Engine changes additionally run the fuzz campaigns before merging
(main-axis `cargo run -p seine-harness -- fuzz 10000 <seed>` plus the
CEP and order-population generators under `tools/` — zero new
divergences on both the changed tree and an A/B baseline worktree; see
DECISIONS.md for the certification records). Semantics are pinned
probe-first: never implement a Drools behavior from intuition — write a
scenario, run it through the oracle, record the pin as a D-entry, then
implement. Firing-order ports go further: an executable Python model
(`tools/model_check_*.py`) must reach zero divergence against banked +
fresh oracle populations before the Rust port begins.
CI mirrors gates 1–3 and builds `seine-rs` wheels for
linux-x86_64 / macos-arm64 / macos-x86_64 / windows-x86_64 plus an
sdist.
