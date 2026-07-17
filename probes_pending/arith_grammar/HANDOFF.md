# HANDOFF — the boundary-redraw arc, remaining work (cold start)

Filed 2026-07-16 at the end of the D-274..286 session. Repo state:
everything through v0.4.32 PUSHED and PyPI-live (tag `8b3210b`);
D-entries D-274..286 in the log. Working tree clean. Read
DECISIONS.md CURRENT STATE first; this file is the operational map
for what remains. ALL engine ports below are Bryan-gated: probe →
mechanism report → gate → port → full battery, per [[seine-workflow]].

## What landed this session (context, not tasks)

- The derive EXPRESSION LAYER (D-274..279, v0.4.30/31/32): Python
  `with_columns`/`filter` + `col/lit/if_else/Expr`, calculator row
  included (D-285). Oracle = DuckDB 1.5.4, pins =
  docs/derive-expr-pins.md (12-row divergence ledger), battery =
  bindings/tests/test_derive_expr.py (three-way: Rust / RefEval /
  DuckDB SQL). ADS-B is reproducible WITHOUT the bespoke kernels
  (D-286, test_derive_expr_adsb.py) — kernels are now an
  optimization, not a capability.
- ⚖ Doctrine (D-280, Bryan): "the match grammar never grows a Java
  or MVEL INTERPRETER" replaced "never grows arithmetic".
- MATCH-plane arithmetic Tiers 1+2 (D-283/D-284): RHS computed args
  on `insert` AND `insertLogical`, Java semantics (ArithTy {I32,I64,
  F64} — int-range literals compute in 32-BIT WRAPPING arithmetic,
  one long operand promotes the op; pinned by
  pr_ar_rhs_int_literal_wrap). Stratification pass: computed edge in
  a logical-derivation cycle = CompileError; copy cycles stay legal.
  tms.cascade_depth panic guard at 8192. Judge parity clauses:
  "fire limit" (D-013) and "/ by zero" (D-283). Non-finite doubles
  render as Java strings ("Infinity"/"NaN") in BOTH serializer paths.
- Implementation map: drl.rs RhsExpr grammar + lexer '/' '%';
  engine.rs CExpr/ArithTy compile_cexpr + eval_cexpr + the
  stratification pass at the end of add_rules_drl; harness judge in
  main.rs; f64_to_json in runner.rs; gen.rs computed-args fuzz axis.

## THE QUEUE (Bryan's stated order: updates next)

### 1. Updates/setters with computation — D-231's core, NOT yet probed

Bryan sequenced this immediately after insertLogical. D-231 ruled
modify-with-computation WONT because self-referential counters
(`modify($c){ setN($c.getN() + 1) }`) are imperative loops. The
re-examination needs its own probe round FIRST:
- What Drools does with `$p.setX($a + 1); update($p)` — arithmetic in
  SETTER args (the engine's Action::Set arg is still atom-only).
- The no-loop/property-reactivity interaction when the computed value
  feeds the rule's own LHS (the loop hazard D-231 named) vs a
  DIFFERENT rule's LHS (the useful case).
- Whether a narrower opening exists: computed setter args allowed
  when the rule cannot re-trigger itself (a static check like the
  stratification pass — "self-feeding computed modify" = the D-222
  lint shape again).
- Fire-limit as the backstop, same as Tiers 1/2 (D-282 showed the
  parity clause covers runaway loops).
Probes go in this directory (ar_upd_*); oracle 3×; PINS.md gets a §E.

### 2. LHS constraint arithmetic — the coercion swamp (D-280 §B)

The certifiable prize (`k > $a + 1` works in Drools TODAY — binding
arithmetic probes all green). Blockers, in order:
- The UNRESOLVED div0 cell: `k / z > 0` with z=0 FIRES while
  `k / z == 0` silently no-fires. Needs its own 2×2 (operator ×
  literal-representability × zero-divisor) before ANY division port.
- The comparand-literal hypothesis (int-representability selects
  integer vs real division; fits 8/9 cells — the SAME FACT fires
  both `k/2 == 3` AND `k/2 == 3.5`) needs the remaining cells:
  binding comparands (`k / 2 == $a`), field comparands, `!=`, `in`.
- Port shape: the COHERENT SUBSET — same-type operands, division
  restricted or comparand-pinned; swamp cells FENCED with
  authoring-lint steering. ⚖ D-281: the engine does NOT copy the
  9.44 `field + lit * lit` eval-throw (correct precedence everywhere;
  opposite-polarity xfail witnesses; generator gate; re-adjudicate on
  oracle bump).
- Prereq note from roadmap-acceptance.md:33 — D-061 lists the D-076
  iterative cascade as prerequisite for arithmetic generally; D-282's
  probes narrowed that to the UNBOUNDED tier only, but re-verify the
  reasoning if LHS arithmetic composes with insertLogical justifiers.
- Acceptance battery: Drools MathTest/FormulaTest (already mapped).

### 3. D-076 iterative cascade → the unbounded tier

The recursive TMS cascade (engine.rs tms_drop_act_deps → on_delete
recursion) is rule-count-bounded today; the stratification pass
keeps it that way. Going iterative (explicit worklist) unlocks cyclic
computed insertLogical (recursive derivations: transitive closure
with computed values, fixpoint numerics). The cascade_depth guard
(panic at 8192) marks every entry point. This is a standalone
engine slab with the FULL battery incl. both censuses.

### 4. authoring.py sugar for computed args

The Python rule builder (bindings/python/seine_rs/authoring.py) still
emits atom-only insert args — DRL-string users have arithmetic,
authoring users don't. BoundField already has `_arith` (+ - * for
salience, SalExpr closed grammar); the insert-args surface needs its
own expression object (mirror the derive Expr trap guards; render to
the DRL arithmetic the engine now parses). Bindings-only; the D-243
three-point wiring hazard does not apply (no new natives) but the
docs-lint (no D-numbers in public docstrings) does.

## Open ledger (small/standing)

- **crates.io Trusted Publishing** — Bryan's one-time setup; every
  tag's publish-crates fails on it (v0.4.5 → v0.4.32). Steps in the
  D-215-era release memory: manual first `cargo publish -p
  seine-engine` + TP config repo=sl-agentics/seine workflow=ci.yml
  env=crates-io.
- **Collect-order latent family**: xf_fz_662607_47 + fz_4649_1144
  class (SetCollection/collectList element ORDER, pre-existing, no
  computed args). Own triage someday; drift bank carries them.
- **Open divergences in xfail/** (all bisect-verified pre-existing):
  xf_fz_31415_774, xf_fz_62831_359, xf_fz_141421_123,
  xf_fz_141421_1206 — plus the older ledger (fz_7331_973 etc.).
- **Derive-plane v2 ledger**: regex ops (dialect pin campaign),
  utf8/bool casts, typed null literals, decimal columns, aggregates
  (accumulate owns them). Also the deferred grid-cell candidate pass
  for pair_candidates at scale.
- **LHS-swamp probes in this dir** are `engine_fenced` (lint verifies
  the walls stay up) — when LHS arithmetic ports, unfence + promote
  the coherent ones, exactly as Tier 1/2 did.
- Cosmetic: the v0.4.31 Actions run shows red from the GitHub outage
  reruns (artifacts all published; a re-run of failed jobs greens it).

## Environment crumbs (beyond [[seine-workflow]])

- Full engine-edit battery (the D-283/284 shape): all-scenarios byte
  gate (find scenarios probes_pending -name '*.json' MINUS this dir →
  run → cmp), make diff, make lint-probes, cargo test, maturin
  develop --release -m bindings/Cargo.toml + pytest (220), demo
  selfcheck, fresh fuzz 2×2000 ×2 seeds with worktree bisect on any
  find, model_ird (probes_pending/tms_envelope/model_ird.py, 31/31),
  agenda_open ×15 byte-compare vs pre-edit worktree, IRD census
  (tools/fuzz_tms_ird.py 150 <seed> — seeds 7001/7002/6001/6003/9001,
  expect 0-div), SD census (tools/fuzz_tms_sd.py 150 <seed> — seeds
  7001/7002/6001/6003/7004..7011, expect divergent-sum == 72 EXACT).
- Derive-plane changes: pytest + make diff/lint + demo only; regenerate
  docs/derive-expr-pins.md via tools/pin_derive_expr.py when semantics
  move (duckdb 1.5.4 hard-asserted).
- Release flow: bump both versions (workspace Cargo.toml +
  bindings/pyproject.toml), rebuild + maturin + pytest, commit ".so
  included", tag vX.Y.Z, push main + tag; publish-crates red is
  expected; verify PyPI JSON. GitHub-outage flakes: HttpClientError
  during job SETUP = infra, rerun when healthy (three sustained API
  probes); real failures show maturin/cargo output.
- cargo-bloat is installed (the wheel-size tool; measure before
  dieting — the guessed culprit was wrong twice this session).
