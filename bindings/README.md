# seine-rs

A rules/decisioning engine over Arrow dataframes — a Rust port of a
bounded Drools (DRL) subset, **differentially certified against real
Drools 9.44.0.Final**: every supported behavior is pinned by running
the same scenarios through both engines, plus multi-seed fuzz
campaigns (zero divergences to completion).

```python
import polars as pl
import seine_rs
from seine_rs import Rule, fact

@fact
class Person:
    name: str
    age: int
    score: float

@fact
class Flagged:
    name: str
    score: float

adults = Rule("Adults")
p = adults.when(Person, Person.age >= 18)
adults.then_insert(Flagged, name=p.name, score=p.score)

people = pl.DataFrame({"name": ["ada", "kurt"], "age": [36, 17], "score": [91.5, 99.0]})
res = seine_rs.run([adults], {Person: people, Flagged: []})

pl.DataFrame(res.derived["Flagged"])   # facts the rules created
pl.DataFrame(res.firings)              # full audit trail
```

- **Bulk, columnar boundary**: Arrow tables in (polars / pyarrow /
  pandas ≥ 2.2, zero-copy via the PyCapsule interface), Arrow tables
  out. Row-object lists (`@fact` instances, dicts, dataclasses,
  Pydantic models) work too.
- **Rules in Python or DRL**: the Python builder compiles to DRL text,
  so both paths run the identical certified engine. Anything outside
  the certified grammar is a definition-time `CompileError`. Coming
  from Drools: DRL here is rules-only — no `package`, no `declare`;
  fact types live in Python and schemas are inferred from them.
- **Full working-memory lifecycle**: `insert → fire → update/delete by
  handle → fire`, each certified differentially.
- **No Python in the hot path**: conditions, aggregates and salience
  are native; callbacks are observers over immutable results.
- **The why-machine**: `sess.why(handle)` answers with the justifying
  rule, matched tuple and firing seq; `justifications()` walks the
  whole graph; `acc_sources()` walks an aggregate to the exact source
  facts that sum to its value. The audit surface is certified
  alongside the semantics.
- **Schemas auto-register** from the `@fact` classes your rules
  reference — `Session([rule])` alone works; `insert_row(Account(...))`
  needs no type argument.

- **Linear where it counts**: alpha, joins, aggregates, TMS
  chains/teardowns, and bulk update churn all measure linear under a
  doubling-size ladder run through this public API
  (`tools/bench_wheel.py` in the repo). The headline: 32k updates
  through a join took 712 s on the 0.4.55 wheel (quadratic — engine
  and bindings each contributed a lane) and 219 ms on 0.4.57.
  Honest counterpoint: deep logical-chain TMS still runs ~3× behind
  warm real-Drools on the same box — the decomposition is in the
  repo's commit record.

Install: `pip install seine-rs` — the import is `import seine_rs`.

Source, scenario corpus, the guided tours (`demo/tours/`), and the
full decision log (D-001 onward):
https://github.com/sl-agentics/seine
