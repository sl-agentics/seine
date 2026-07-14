# HANDOFF — the `seine_rs.derive` arc (cold start)

**Goal**: replace the demo's hand-rolled polars derivation stage with a
Rust/arrow-rs kernel module exposed as `seine_rs.derive`, under the
two-plane contract of `docs/derivation-plane.md` (D-249). This is pure
exposure of an already-designed contract — read that doc FIRST; this
file is the operational half.

**Arc status**: NOT OPENED. Bryan gates the opening, every push, and
every tag (never push v* tags without explicit direction — a tag push
publishes to PyPI with no manual gate).

## What already exists (all landed, released through v0.4.20)

- `docs/derivation-plane.md` — the contract: two planes, two oracles.
  The match plane (engine/, the certified grammar) NEVER changes in
  this arc. The derivation plane's oracle is a reference implementation
  + property tests — the Drools oracle has no opinion here.
- `demo/adsb_convergence.py` — the working polars prototype: metric-
  space candidate pass (wrapped lon delta, cos(lat)-scaled threshold
  saturating to lat-only at the poles), vectorized haversine, TTL'd
  closing state (pure function of the raw epoch sequence — WAL-replay
  determinism holds end to end). Its `_selfcheck()` runs at import and
  IS the seed battery (below).
- `scenarios/demo/adsb_convergence.json` — the match-plane half,
  byte-checked 3x against the pinned Drools oracle. The derived values
  in it came from the demo verbatim; if kernel outputs ever change by
  ±1m (rounding), the twin must be regenerated AND re-diffed 3x.
- History: D-249 (design + prototype), D-250 (round-27 fixes: the
  degree-space-prune, kernel-only-selfcheck, and stale-closing-state
  defects — read that entry; the Rust kernels must not reintroduce
  them), commit `8fecbaf` (the reviewer's battery vectors, executable).

## The v1 kernel set (ADS-B-driven; resist scope growth)

1. `haversine` — columnar (lat1, lon1, lat2, lon2) -> dist_m (Float64
   in, Int64 meters out, round-half-away like the demo; EARTH_R =
   6_371_000.0 — keep bit-compatible with the demo so the scenario twin
   survives).
2. `pair_candidates` — cross-join + metric-space prune over one
   position table (icao/lat/lon or generic id/lat/lon): `a < b` dedup,
   lat delta < BBOX_M/111320, WRAPPED lon delta < BBOX_M/(111320*
   max(cos(mean_lat), 1e-6)) clipped to 180 (the D-250 geometry,
   exactly).
3. `closing` — stateful decreasing-distance flag keyed by pair, with a
   TTL swept by epoch timestamp (state is the CALLER's object so replay
   re-derives; do not hide state in module globals).

API shape: functions over Arrow data (accept anything `run()` accepts —
`__arrow_c_stream__` tables or dicts of column lists — return the same),
so derived batches feed `Session.insert()` directly. arrow-rs is already
a dependency of the bindings crate; the wheel stays ZERO-DEP (D-221:
no pyarrow/polars requirement — that constraint is load-bearing and
reviewer-verified).

## The certification battery (port from the demo's `_selfcheck`)

Ground-truth-driven, per D-250's hardening: every vector carries an
explicit must_emit flag; the assert is UNCONDITIONAL (a candidate-
geometry miss turns red — never a kernel-only symmetry check).

```
# (lat1, lon1, lat2, lon2, ~true_dist_m, must_emit)
(40.0,   -0.117, 40.0,  0.117, 19932, True)   # demo opening separation
(40.0,    0.00,  40.0,  0.04,   3407, True)   # benign control
(40.0,  179.98,  40.0, -179.98, 3407, True)   # antimeridian straddle
(89.9,    0.0,   89.9,  1.0,     194, True)   # polar lon-compression
(89.95,   0.0,   89.95, 3.0,     291, True)   # extreme polar compression
(0.0,   179.98,  0.0, -179.98,  4452, True)   # antimeridian at equator
(40.0,    0.0,   40.0,  0.0,       0, True)   # identity
(40.0,    0.0,   40.0,  0.5,   42704, False)  # comfortably outside
(40.0,    0.0,   40.0,  1.0,   85394, False)  # outside
# state-TTL: derive t=0 (far) -> t=5000 (closer, closing=True)
#            -> gap to t=605000 (closer still, closing MUST be False)
```

Plus: symmetry d(a,b)==d(b,a); |kernel - reference| <= 1m against an
INDEPENDENT implementation (the demo's `_haversine_ref` is pure-python —
keep it as the cross-check, do not port it to Rust and compare Rust
against Rust); determinism = same batch in, byte-identical batch out,
repeated. New tests live in `bindings/tests/test_derive.py`; also run
the demo (its selfcheck must stay green on the polars stage — the two
implementations should agree on every vector).

## Hard rules and gates

- `engine/src/` untouched in this arc. If ANYTHING forces an engine
  edit, stop: that's a separate Bryan-gated decision with the FULL
  battery (corpus 3 tiers + drift, SD census 12 seeds x150 = 72 EXACT,
  ird census 5 seeds = 0, model_ird 31/31, witnesses 26/26, agenda_open
  x19 vs a clean worktree, lint-probes, cargo test).
- Bindings-only changes still run: `make diff` (expect corpus counts
  unchanged), `make lint-probes`, full pytest.
- READ gate output before writing receipts into any commit message
  (bitten twice: D-242's lint, the demo's 1156-vs-1155). Echo `$?`
  explicitly — a `| tail` pipeline masks Python exit codes (bit once,
  commit `8fecbaf`'s first attempt had to be amended).
- The Python `Session`/module surface is a WRAPPER: a new native
  pymethod is invisible until also wired in
  `bindings/python/seine_rs/__init__.py` (bit once at D-243).
- Rebuild: `cd bindings && ../.venv/bin/maturin develop --release`
  (python-source edits are live without rebuild; Rust edits need it).

## Environment quick-start

```
cd /home/bryan/rust-rules
.venv/bin/python demo/adsb_convergence.py        # selfcheck + demo, exit 0
cd bindings && ../.venv/bin/python -m pytest tests/ -q   # 142 passed
cd .. && make diff                                # 11/1156/397 + drift 32
make lint-probes                                  # 1826/0/0
```

polars 1.42 is in the venv (dev-only; never a wheel dependency).
DECISIONS.md is append-only history; new work = new D-number entry +
the one-line MEMORY.md arc update.

## Adjacent open items — NOT this arc (do not absorb)

- Entry points (Tier D exposure) — deferred by reviewer ranking.
- The banked groupby-key engine-accepts-more reject (engine change).
- Query-churn recon: a mid-epoch query action in BOTH runners (D-248).
- Bryan's one-time crates.io TP steps (publish-crates is red on every
  tag until then; expected, not a regression).
