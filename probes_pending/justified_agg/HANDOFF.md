# HANDOFF — the justified-aggregation edge (D-304 option b), cold start

Written 2026-07-18 at the end of the D-293..D-310 session (see
DECISIONS.md CURRENT STATE). Everything through v0.4.39 is PUSHED and
RELEASED (PyPI live). Corpus 11/1280/406 + drift 50; lint 2034/0/0;
cargo 11 suites; pytest 253. The Drools oracle understands
decimal(p,s) fields since D-308 (OracleRunner → BigDecimal).

## THE MISSION, reframed by review

This is NOT cosmetics/uniformity. It is a REVERSAL-CORRECTNESS gap in
the flagship money story:

    rule balance when accumulate( Line($a : amount); $t : sum($a) )
                 then insert(new Balance($t)); end
    rule release when Balance($v : v, v <= 0.00)
                 then insertLogical(new Release(1)); end

Insert lines summing to 0.00 → Balance(0.00) → Release derives. NOW
INSERT A NEW LINE (the balance goes positive): the accumulate
re-fires, the balance rule PLAIN-INSERTS Balance(50.00) — but
Balance(0.00) is a plain fact and PERSISTS (stale). Release stays
justified by the stale Balance. THE RELEASE SURVIVES ITS OWN
REVERSAL. The D-076 wall ("insertLogical from accumulate/collect/
?query rules is out of subset — justifying-tuple revalidation cannot
re-run those conditions", enforced in engine.rs add_rules compile)
forces the plain insert, so today's subset cannot express the
self-maintaining version.

## THE TWO FORKS — run these BEFORE designing any port (review's
## pins, agreed)

FORK 0 (cheap, run FIRST): **then_modify singleton**. Keep exactly
one Balance fact, UPDATED in place instead of re-inserted:
  - authoring: match Balance, `then_modify(bal, v=total)`; seed one
    Balance row. Or raw DRL `modify($b){ setV($t) }`.
  - QUESTION: when the balance goes positive, does the update
    re-evaluate `release`'s LHS and retract the logical Release
    (update-driven unmatch → removeLogicalDependencies — this is
    certified machinery, D-186..D-211)?
  - Prediction to register: YES, Release retracts — update-unmatch
    teardown is the certified TMS path. If yes: the SAFE PATTERN
    exists in today's subset → document it (sum_ docstring +
    CHANGELOG + a pinned probe), and the engine feature drops to
    uniformity polish (the fuzz-axis slot wins instead).
  - If NO (release survives): the engine feature is earning its keep
    → FORK 1 in earnest.

FORK 1: **the Drools probe round** (D-280/D-308 shape: predictions
first in PINS.md, oracle-only cells, 3× stable):
  - f1: does `insertLogical(new Balance($t))` from an accumulate rule
    even BUILD in Drools? (UNPINNED — our wall's rationale is our own
    TMS mechanism; ErrorOnInsertLogicalTest was routed for
    function-blocks/exceptions, NOT this. D-304 precision #2.)
  - f2 (if it builds): on re-accumulation, does the OLD logical
    Balance retract (justifying-activation cancel → teardown) and the
    new derive — i.e., is the aggregate self-maintaining?
  - f3: does the downstream logical (Release) retract through the
    swap? The full reversal chain.
  - f4: value-keyed dedup interaction — re-accumulate to the SAME
    value (does the Balance stay put / re-root / flicker?).
  - f5: STALE-FACT CONTROL — the plain-insert shape in Drools:
    confirm Drools ALSO leaves the stale Balance + stale Release
    (prediction: yes, plain insert is plain insert — making the gap a
    UNIVERSAL modeling gotcha, not a Seine divergence; receipts for
    the docs).
  - f6: multi-group groupby variant if f2 works (per-group logical
    rows).
  Run via `./target/debug/seine-harness oracle <file>` (build first:
  `make oracle` if oracle/ changed; decimal fields fine). 3× byte
  compare. Volume/jit axis only if the semantics look mode-suspect
  (D-308's decimal cells were jit-clean).

REGARDLESS OF FORK: **stale-release-on-reversal becomes a pinned,
documented test** — engine-vs-oracle diffed if Drools agrees (f5),
plus the safe-pattern probe (fork 0's shape) graduated, plus docs
naming the gotcha where the pattern lives (the sum_ docstring
precedent, D-307).

## GATE DISCIPLINE

Probe round → mechanism report in PINS.md + DECISIONS entry → BRYAN'S
GATE before any engine port. If fork 1 lands "Drools supports it",
the port must solve justifying-tuple revalidation for accumulate
conditions (the original wall rationale) — that design goes in the
report, not in code.

## REPO STATE + OPERATIONAL NOTES (this session's banked lessons)

- Battery for engine-core edits (the full list with receipts shapes:
  see any of D-297..D-310 entries): pre-edit worktree byte gate over
  `find scenarios probes_pending -name '*.json'` (2194 files; use
  expected-divergence analysis when the edit ADDS capability — D-309
  shows the pattern), make diff (11/1280/406 + drift 50), make
  lint-probes (2034/0/0; fence-shape pending probes need
  `"engine_fenced": true`), cargo test, maturin develop --release
  **FROM bindings/** (root invocation FAILS on the workspace manifest
  and a piped tail masks it — D-301; the tracked
  bindings/python/seine_rs/_native.abi3.so IS the import target and
  rides engine commits), pytest (253), demo
  (demo/adsb_convergence.py → True), model_ird (cd
  probes_pending/tms_envelope && python model_ird.py → 31/31; CD
  BACK — a stuck cwd once wrote DECISIONS.md into that dir),
  agenda_open ×15 both binaries, IRD ×5 seeds 7001/7002/6001/6003/
  9001 (0-div), SD census ×12 seeds (divergent 6,10,3,5,6,5,5,6,8,7,
  4,7 = 72 EXACT — the order gate), fresh fuzz 2×2000 on two new
  seeds (next: 311001/311002; finds → bisect vs the pre-edit
  worktree; pre-existing → scenarios/xfail/ + make xfail-rebank).
- Oracle runs: batch fine for non-cyclic; cyclic = timeout-guarded
  singles. Redirect to files. -Xss1g is pinned in oracle.rs.
- Decimal eval overflow errors are TYPED via a thread-local
  (eval_error_set/take, D-310) — new eval paths that can fail should
  use the same slot, and new public entry points need the take.
- Release flow: CHANGELOG Unreleased→version, bump Cargo.toml +
  bindings/pyproject.toml, wheel from bindings/, extract .so over the
  tracked copy, fresh-venv sanity, commit "vX: version bump",
  lightweight tag, push main+tag; tag CI gates publish on the
  differential job; publish-crates ALWAYS red (Bryan's one-time
  crates.io TP config, standing); PyPI JSON lags the CDN ~2 min.
- Standing ledger besides this slab: exact decimal average
  (rounding-mode ruling), gen.rs decimal fuzz axis, crates.io TP.

## FILES

- The wall: engine/src/engine.rs (search "insertLogical from
  accumulate" — the D-076 compile check).
- The teardown machinery the fork-0 prediction rests on: D-186..D-211
  (eager dep-removal on update-unmatch), D-293 worklist.
- The audit channels the fix story feeds: Session.why /
  Session.justifications (D-303), Session.acc_sources (D-305).
- D-304 (the gap's discovery + the two fix shapes), D-308 (the probe
  round template with predictions-first PINS.md).
