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
  the certified grammar is a definition-time `CompileError`.
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

Install: `pip install seine-rs` — the import is `import seine_rs`.

Source, scenario corpus, the guided tours (`demo/tours/`), and the
full decision log (D-001 onward):
https://github.com/sl-agentics/seine
