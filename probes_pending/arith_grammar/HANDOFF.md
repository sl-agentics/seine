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

### 1. Updates/setters with computation — DONE (D-287 probes → D-288 port; D-289 authoring check)

Bryan gated (a)+(b) after counter-review: faithful port, no engine
restriction (⚖ capability-bound rationale in D-288 — walls only
where an engine bound exists), symmetric authoring-layer check
(D-289). 13 probes graduated; 5 walls stay fenced here; full
battery green (byte gate 2047, corpus 11/1233/406, fuzz clean with
the setter axis, SD 72 EXACT). Remaining §1 surface = queue item 4
(authoring sugar owns the stale `_rhs_arg` wall message).

Original probe-round record below, for the mechanism map:

### 1-record. The D-287 probe round (was: PROBED, AT BRYAN'S GATE)

Probe round DONE 2026-07-16: 18 ar_upd_* probes in this directory,
3×-stable, 18/18 predictions hit — PINS.md §E is the record, D-287
the log entry. Answers to the four questions this section asked:
- Setter args are the SAME clean Java as insert args (ArithTy §D
  verbatim; narrowing = build error; div0 = parity shape). Bindings
  snapshot / getters live composes into arithmetic (fz_7_2525 law).
- The loop hazard is EXACTLY the self-feeding shape: written ∩
  own-listened ≠ ∅ (bound fields count as listened; getter reads do
  not). Self-modify of an unlistened field terminates WITHOUT
  no-loop; the useful case (feeding another rule's LHS) is ordinary.
- The narrower opening EXISTS and is static: per-rule compile check
  on written ∩ own-listened; note the hazard shape is atom-legal
  TODAY (ar_upd_same_value_runaway), so a computed-only wall is
  asymmetric. no-loop covers self only, never cross-rule ping-pong.
- Fire limit backstops every runaway cleanly (parity clause).
BRYAN DECIDES the restriction level: (a) none / (b) authoring lint /
(c) CompileError on self-feeding computed setters, ± the update-edge
cycle check (D-284 stratification shape). Port shape in PINS.md §E
(mechanical: rhs_arg → rhs_expr both setter sites, Set carries CExpr,
D-283 machinery verbatim; fuzz = computed setter args under the
guard-field discipline). Full engine battery applies when ported.

### 2. LHS constraint arithmetic — DONE (D-290 map → D-291 agree-subset port)

Bryan directed the agree-subset port with mode-1 residency logged as
a precondition and a volume detector on the quarantine. LANDED: see
PINS.md §F "The port — LANDED" for the full record (grammar, fences,
residency precondition, detector, witnesses, receipts). Remaining
LHS surface = the fenced cells (each a mode-divergence, each with
steering) and the §F re-adjudication note on oracle bump.

Original D-290 record below, for the mechanism map:

### 2-record. The D-290 round (was: AT BRYAN'S GATE)

Probe round DONE 2026-07-16: 29 ar_dz_* probes (25 deterministic
3×-stable + 4 designed race witnesses), 7/7 out-of-sample
predictions hit. PINS.md §F is the full model; §B's hypothesis is
struck. The answers:
- NO integer division exists interpreted: `/` is always IEEE double
  (div0 never throws, even literal `k / 0`); the "integer" cells
  were a Java (long) NARROWING CAST at the comparison ((long)3.5=3,
  (long)+Inf=MAX, (long)NaN=0 — `0/0 == 0` FIRES). Both anomaly
  cells fall out of one rule.
- Comparison typing: eq-family by literal VALUE / field TYPE;
  binding-eq TYPE-STRICT (always-false — its own fence quadrant);
  relational by comparand TYPE (`> 3` no-fires, `> 3.0` fires).
  `+ - * %` long-exact; `% 0` throws LOUDLY (parity shape).
- THE FLOOR: an async MVEL→java jit RACE — constraints flip to java
  semantics (trunc div, div0 THROWS, 3L==3.5 false) after ~20+
  evaluations, run-nondeterministically (prefix cliffs 127/128/135
  at 5000 facts; z=0 volume errors every run). Volume `/` is NOT
  byte-certifiable against this oracle config.
- AGREE SUBSET (both modes identical — the certifiable core):
  `+ - * %` (+ `% 0` loud parity error) and `/` with int-typed
  comparands, |operands| < 2^53, nonzero LITERAL divisor. Fences:
  double comparands on `/`, field/binding divisors, expr==binding,
  huge operands. ⚖ D-281 (precedence defect not copied) stands.
- D-076 prereq: unchanged (unbounded tier only, D-282); re-verify if
  LHS arithmetic composes with insertLogical justifiers at port.
- Acceptance battery: Drools MathTest/FormulaTest (mapped) — filter
  to the agree subset at port time.
BRYAN DECIDES the port shape (PINS.md §F "port shape" block). Race
witnesses stay recon-only (fz_42_84 quarantine precedent);
re-adjudicate the table on any oracle bump.

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
- **xf_fz_606060_555 + xf_min_606060_555** (D-291 fuzz flush,
  bisected PRE-EXISTING — pre/post-D-291 engines byte-identical on
  the min repro): FIRE-COUNT divergence (oracle 8, engine 6) on a
  3-rule shape — acc(sum) + bare join + acc(count) rule, a
  setFocus+insert rule, an agenda-group not-CE rule; ZERO
  arithmetic. Smells D-106-adjacent (agenda/focus × accumulate
  re-eval), NOT the collect-order class. Min witness is
  drift-banked; own triage someday.
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
