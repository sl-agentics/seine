# bench_slow — correct-but-slow deep-scale witnesses (NOT corpus, NOT linted)

This dir is deliberately OUTSIDE the lint scan set (tools/lint_probes.py)
and the make-diff tier globs, and byte-gate recipes must EXCLUDE it.

History: D-296 parked four cyclic grinders here under the by_act
quadratic. **D-297 fixed by_act** (order-preserving ByAct index) —
ub_deep_9000 graduated to scenarios/probes/pr_ub_deep_9000 (0.77s) and
the two arith_grammar runaways went home (≈4.5s diffs, error-vs-error
parity). One resident remains:

- `ub_deep_99k.json` — the fire-limit-maximal class (98,999 grows +
  99,000-deep teardown; the D-296 no-assert witness). Engine ~52s
  debug under the RESIDUAL quadratic (the D-297 open item): the
  staged del-dedup scan — `phreak::Staged::add_del`'s
  `del.iter().any(...)` walks the accumulated del list per teardown
  delete (the `seen` fast path can't help: consumed facts stay seen).
  Fix sketch = a stale-positive `del_set` (the D-266/D-267 pattern),
  but del lists are MERGED wholesale at ~9 sites (grep
  `del.extend|del.push`) on identity-model-law surface — that audit
  is its own gated slab.

Run manually at slab boundaries (oracle needs the D-295 -Xss1g pin,
already in harness/src/oracle.rs):

    cargo run -q -p seine-harness -- diff scenarios/bench_slow/ub_deep_99k.json   # ~60s wall

When the staged-del item lands, graduate this file into
scenarios/probes/ and delete this dir.
