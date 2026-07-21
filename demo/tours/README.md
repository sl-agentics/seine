# Feature tours

Five self-contained walkthroughs, one per major surface. Each is a plain
script — run it, read the printed narrative top to bottom. They are
examples, not regression tests: the certified corpus under
`probes_pending/` is the regression net; these exist to show the API
idioms in working form.

Run with the repo venv (or any env with `seine_rs` installed):

    .venv/bin/python demo/tours/tour1_tms.py

| Script | Surface | What it shows |
|---|---|---|
| `tour1_tms.py` | Truth maintenance | `then_insert_logical` → `why()` → delete a support → auto-retraction (one account's derived fact dies, the other survives) |
| `tour2_cep.py` | Temporal CEP | Allen `after` sequence detection; sliding `window_time` count → threshold chain; clock advance expires the cluster |
| `tour3_provenance.py` | Aggregation provenance | `group_by` sum inserted logically, then the full audit chain: `why()` support tuple → `acc_sources()` → line-item leaves that re-sum to the total |
| `tour4_query.py` | DRL queries | Bound param, unbound param (pass `None`, binds per-row), and no-param queries over `@fact` schemas |
| `tour5_lifecycle.py` | Lifecycle + walls | `update`/`delete` by handle re-deriving through TMS; salience ordering via `on_fire`; five out-of-subset constructs rejected with `CompileError` at definition time |
| `tour6_decimal.py` | Exact decimals vs IEEE | Ten $0.10 payments: engine f64 sum == the naive Java left-fold (0.9999…) while CPython 3.12+'s `sum()` silently compensates to 1.0 — and the exact-decimal lane lands $1.00; `average_exact` rounding modes incl. the 8.125 half_up/half_even tie split; the D-341 average-over-decimal wall |
| `tour7_neg_exists_or.py` | Group CEs | `when_not` with a join; `when_exists` firing once vs the plain join's row multiplication; `when_any` alpha-only OR across classes |
| `tour8_allen.py` | Allen interval algebra | All 13 relations against a `@duration` anchor `[100,200]` — one probe interval per relation, every operator fires for exactly its own probe (the full diagonal) |
| `tour9_cascade.py` | Forward chaining | `new → validated → shipped → archived+deleted` in a single `fire()` via `then_modify`/`then_insert`/`then_delete`; the audit trail of the cascade |
| `tour10_tms_edges.py` | TMS edges | Multi-support survival (a belief outliving one of two justifying rules) and the stated/logical interplay — see the order-sensitivity idiom below |

Idioms worth stealing:

- **Aggregate → threshold chain** (tour2): the certified way to act on an
  aggregate is to insert it as a fact and match it downstream.
- **Provenance walk** (tour3): a `group_by` firing's match element
  renders as the `("QueryArgs", handle)` `[result, key]` composite
  (D-108); since 0.4.47 the composite's handle IS the group-result
  fact, so `acc_sources(handle)` answers directly — the `why()`
  support-tuple walk shown in the tour also works and carries the
  justification context.
- **`on_fire` is a post-quiescence observer** (tour5): it receives plain
  data after the run; session methods (including `acc_sources`) are not
  callable from inside it.
- **The audit trail is post-RHS** (tour9): `firings`/`values_json`
  snapshot each matched fact AFTER the firing's own `then_modify` — a
  `Validate` rule matching `state=="new"` logs `state:"validated"`.
  Oracle-pinned (D-013/j03: match rendering lists facts in declaration
  order, values POST-RHS); match-time values are not recoverable from
  the trail by design.
- **Stated/logical cardinality is order-sensitive** (tour10):
  logical-then-stated mints a second fact (two coexist; deleting the
  justification retracts only the derived one), while
  stated-then-logical is a handle no-op (one fact; the would-be
  justification never attaches). The 2-vs-1 asymmetry is certified
  Drools identity-mode TMS (the `pr_tms_ls_fwd`/`pr_tms_ls_rev` probe
  pair) — anything downstream that counts such facts sees it.

Authored during the v0.4.46 cross-machine test drive (2026-07-20); all
five verified green against the 0.4.46 wheel on both machines.
