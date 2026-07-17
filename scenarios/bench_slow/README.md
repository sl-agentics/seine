# bench_slow — correct-but-slow deep-scale witnesses (NOT corpus, NOT linted)

This dir is deliberately OUTSIDE the lint scan set (tools/lint_probes.py)
and the make-diff tier globs, and byte-gate recipes must EXCLUDE it:
these scenarios are certified correct but engine-side QUADRATIC until
the by_act scaling item lands (D-296 open item: execute_rhs's
refire-supersede prologue does a linear `tms.by_act` find PER FIRING —
O(n²) over a deep chain; debug timings 0.22s @ 1000-deep, 15.5s @
9000-deep, ~36 min @ 99k-deep).

- `ub_deep_9000.json` — 9000-deep fixpoint + complete teardown.
- `ub_deep_99k.json` — the fire-limit-maximal class (98,999 grows +
  99,000-deep teardown). THE no-assert witness (D-296).
- `ar_tms_runaway_logical.json` / `ar_tms_cycle_two_type.json` —
  moved from probes_pending/arith_grammar (their D-282 PINS rows
  stand): unbounded cyclic-logical runaways. Post-lift both sides
  run to "fire limit 100000 reached" (error-vs-error parity,
  D-013/j21) — but the engine side grinds the same quadratic by_act
  scan on the way there, so they cannot sit in a linted tree.

Run manually at slab boundaries (oracle needs the D-295 -Xss1g pin,
already in harness/src/oracle.rs):

    cargo run -q -p seine-harness -- diff scenarios/bench_slow/ub_deep_9000.json
    cargo run -q -p seine-harness -- diff scenarios/bench_slow/ub_deep_99k.json   # ~36 min engine-side (debug)

When a perf slab indexes by_act (order-preserving — by_act ORDER is
the certified eager-break scan order, D-293), graduate both into
scenarios/probes/ and delete this dir.
