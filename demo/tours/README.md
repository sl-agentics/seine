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

Authored during the v0.4.46 cross-machine test drive (2026-07-20); all
five verified green against the 0.4.46 wheel on both machines.
