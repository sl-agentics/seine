# Data-types arc design — nulls (SQL 3VL) + exact decimals (D-096)

Status: DESIGN CHECKPOINT — pre-implementation, awaiting Bryan's
review. Authority per D-095: the columnar ecosystem (DuckDB 1.5.4 +
pyarrow 24.0.0, pinned in `.venv`); measured ground truth lives in
`docs/duckdb-datatype-pins.md` (regenerate: `tools/pin_duckdb.py`).

## 1. Schema surface

- **Nullability is per-field opt-in**: `{"name": "x", "type": "i64",
  "nullable": true}`. Default stays non-nullable — the certified
  corpus and every existing scenario are untouched, and non-nullable
  fields keep today's loud null rejection (D-044).
- **Decimal is a new field type**: `"type": "decimal(18,2)"` —
  precision ≤ 38 (Arrow decimal128), scale ≤ precision. Scenario JSON
  carries decimal VALUES AS STRINGS (`"1.25"`) or integers; JSON
  floats are rejected for decimal fields (no IEEE round-trip).
- Scenario JSON `null` is legal exactly for nullable fields.
- DRL surface: `declare` blocks mirror the schema; decimal literals
  in constraints are lexed exactly (digits + '.') and typed
  DECIMAL(p,s) by their literal shape, like SQL (pin J: `typeof(1.23)
  = DECIMAL(3,2)`).

## 2. Storage

- Nulls: per-nullable-column validity bitmap beside the value arena
  (Arrow model). Value slots of null cells are poisoned in debug.
- Decimals: i128 scaled fixed-point at the FIELD's declared (p,s)
  (the D-064 note, now DECIMAL(p,s)-shaped). Comparisons align scales
  by exact rescale (widening only — never lossy); arithmetic is out
  of the subset today (no constraint arithmetic yet), so only
  compare/bind/insert/aggregate paths exist.
- Ingestion casts to the field's scale with HALF-UP rounding when the
  source has more scale (pin J), and ERRORS on overflow (pin J:
  conversion error, never wrap/saturate).

## 3. Semantics (conform to the pins; deltas called out)

- **Constraint evaluation is 3VL**: tests yield TRUE/FALSE/UNKNOWN;
  a pattern admits a fact only on TRUE (pin D: WHERE-TRUE; UNKNOWN
  is excluded from both a test and its negation). Composite groups
  (`&&`/`||`/`!`) follow the pin-B tables — note `NULL AND FALSE =
  FALSE` means no naive short-circuit-to-UNKNOWN.
- **Null-test surface**: `field == null` / `field != null` in DRL
  parse as IS NULL / IS NOT NULL (definite, two-valued — pin A).
  Everything else involving a null operand is UNKNOWN. This mapping
  (Drools' surface syntax, SQL's semantics) is the one place the two
  worlds meet; flagged for review.
- **`in`/`not in`**: pin C — `x in (…)` with null x is UNKNOWN;
  a null LIST member makes non-membership UNKNOWN (the `not in`
  trap reproduces faithfully: `1 not in (1, null)` is FALSE,
  `1 not in (2, null)` is UNKNOWN → excluded).
- **String ops**: `matches`/`contains` with a null operand → UNKNOWN
  (pin E).
- **Join equality keys**: null never equi-joins (pin F) — the hash
  index simply never indexes null keys; range indexes skip null.
- **Existential CEs / groups**: compose through WHERE-TRUE admission;
  no new machinery.
- **Accumulate**: null CONTRIBUTIONS are skipped (pin G — sum/avg/
  min/max ignore nulls; count($x) skips null args). Result-side, the
  certified Drools behavior stays for the empty/all-null set (sum→0
  fires; avg/min/max don't propagate — which coincides with SQL's
  NULL results for avg/min/max). **The one axis conflict: SQL says
  sum(empty)=NULL, Drools-certified says 0 — proposal: keep 0
  (aggregates are engine-axis; null-handling is data-axis).
  FLAGGED for ruling.**
- **Decimal comparisons**: exact after scale alignment; vs i64 exact;
  vs f64 by casting decimal→double (pin J — with the documented
  hazard that 0.1::DECIMAL == 0.1::DOUBLE is TRUE by construction).
- **Decimal aggregates**: sum exact at DECIMAL(38,s) (pin J);
  average → f64 (pin J: AVG is DOUBLE — happily matching our
  certified average→f64); min/max preserve the field type.
- **TMS value-equality keys**: null == null for key identity (pin H:
  GROUP BY/DISTINCT collapse nulls); decimal keys compare by aligned
  exact value.
- **Boundary (bindings)**: Arrow validity-null → engine null
  (nullable fields only; non-nullable keep rejecting). pandas/Arrow
  float NaN → NULL for NULLABLE float fields (pin I rationale:
  NaN-as-value semantics must not leak into 3VL); NaN into a
  non-nullable float stays a value (bit-exact, certified D-044
  behavior). Arrow decimal128(p,s) → decimal fields (exact, scale
  checked).

## 4. The DuckDB differential oracle

- **What it validates**: data-type semantics — per-rule MATCH SETS
  and accumulate RESULTS over insert-only scenarios with inert RHS,
  plus the operator truth tables (already pinned). Firing ORDER,
  agenda, chaining, mutation, TMS remain Drools-certified engine
  semantics and are out of the DuckDB oracle's scope by design.
- **Mechanics**: scenario gains `"oracle": "duckdb"` (default
  drools). `tools/duckdb_oracle.py` (pinned venv) translates: types →
  tables (nullable columns, DECIMAL columns), facts → rows, each
  rule's LHS → a SELECT (patterns → FROM/JOIN, constraints → WHERE
  with the direct SQL operator mapping, `== null` → IS NULL,
  accumulate → correlated aggregate subquery, not/exists → NOT
  EXISTS/EXISTS with WHERE-TRUE semantics). Output: per-rule row
  sets, canonically sorted.
- **Comparator mode**: order-INSENSITIVE per-rule match-set equality
  (+ accumulate values exact). The engine side runs the same
  insert-only scenario and reports its firing set per rule.
- **Generator**: a duckdb-mode (`fuzz --oracle duckdb` or a seed
  flag) drawing null/decimal-rich insert-only scenarios: nullable
  fields at ~30% null density, decimal fields with varied (p,s),
  full operator coverage incl. groups/in-lists/existentials/
  accumulates, inert RHS. The classic Drools generator is untouched.
- Version pinning: duckdb 1.5.4 + pyarrow 24.0.0 recorded in the
  pins doc and requirements; `tools/pin_duckdb.py` re-run + diff is
  the version-bump ritual (same discipline as the Drools 9.44 pin).

## 5. Phasing (each phase commits green)

1. **Nulls in the engine**: schema/parser/storage/3VL evaluation +
   unit tests generated FROM the pin tables (truth-table conformance
   suite); certified corpus must stay byte-identical (nullability is
   opt-in).
2. **DuckDB runner + harness routing + set-comparator.**
3. **Null probe corpus + duckdb-differential fuzz** (own gate line).
4. **Decimals** (storage/compare/aggregate) through the same cycle.
5. **Bindings**: Arrow nullable ingestion, NaN normalization,
   decimal128; boundary tests.
6. FEATURES/docs promotion to §1 with the D-095 authority noted.

## 6. Checkpoint rulings (Bryan, 2026-07-07 — D-097)

1. `field == null` ⇒ IS NULL mapping: **APPROVED**.
2. sum(empty/all-null): **0, fires** (Drools-certified engine axis;
   the arc's one deliberate deviation from the DuckDB oracle — the
   comparator maps engine-0 ≡ SQL-NULL for empty/all-null sum
   groups). Null contributions still skip per the pins.
3. Per-field opt-in nullability: **APPROVED**.
4. Decimal-vs-f64: **WALLED — compile error** (stricter than DuckDB;
   money never meets floats). decimal-vs-i64 stays exact.
5. DuckDB-oracle scope (match sets + aggregates, insert-only):
   **APPROVED**.
