# HANDOFF — D-076 iterative cascade → the unbounded computed-logical tier (cold start)

Filed 2026-07-16 at the close of the D-287..292 session, for a fresh
context. Read DECISIONS.md CURRENT STATE first; this file is the
operational map. Per [[seine-workflow]]: probe → mechanism report →
**Bryan GATE** → port → full battery. Nothing below is pre-approved
except step A's *shape* (Bryan queued "D-076 iterative cascade → the
unbounded tier" as boundary-redraw item 3; each engine edit still
gates).

## Repo state at filing

- Pushed through `4cb325d` (D-287..291 + witnesses). LOCAL unpushed:
  `f13be11` (D-292 fenced-context recon) + this handoff commit.
  Working tree clean; `git log --oneline` for live HEAD.
- Gates at filing: corpus 11/1257/406, drift bank 46, full lint
  ~1976/0/0, cargo 54, pytest 229, demo True, SD census 72 EXACT,
  IRD 0-div ×5, model_ird 31/31, agenda_open ×15 stable.
- The boundary-redraw arc so far: computed insert args (D-283),
  computed insertLogical under STRATIFICATION (D-284), computed
  setter args (D-288), authoring self-feed check (D-289), agree-subset
  LHS arithmetic (D-290/291), fenced-context recon (D-292). Queue
  item 3 is THIS file; item 4 (authoring sugar) waits behind it.

## THE TASK — two steps, separately gated

### Step A — the recursion becomes a worklist (behavior-preserving)

The recursive TMS teardown is the LAST rule-count-bounded recursion:

- `tms_drop_act_deps` (engine.rs:11647) collects `to_retract`, sorts
  `by_key(|(s, f)| (*s, f.0))` per level, bumps `tms.cascade_depth`
  (11689–11702; **panic assert at 8192** naming D-284), then for each
  victim: `store.kill` + `self.on_delete(jf, None)` — and on_delete
  re-enters TMS drains → recursion. `tms_route_delete_ex` (10876),
  `tms_materialize` (10991), and the refire-supersede epilogue in
  `execute_rhs` feed the same path.
- The rewrite: an explicit LIFO worklist that REPLAYS THE RECURSION'S
  EXACT ORDER (depth-first, per-level sort preserved). Teardown order
  is CERTIFIED — the SD census (72 EXACT) is the order-sensitive gate
  that will catch any deviation; treat any SD drift as "the refactor
  changed teardown order", never as census noise.
- FIRST move for the cold session: MAP THE RECURSION GRAPH before
  editing (which on_delete paths re-enter tms_drop_act_deps; whether
  expiration drains / halt re-adds recurse through the same frames —
  the identity-model-law triage note says deletes defer in "TMS
  cascades, expiration drains, halt-model re-adds"). Scope step A to
  the TMS cascade unless the map says otherwise; surface anything
  bigger to Bryan before widening.
- Step A ships with the FULL battery (below) and NO semantic change:
  all-scenarios byte gate, SD 72 EXACT, agenda_open ×15 identical.
  The 8192 assert stays in step A (it should be unreachable exactly
  as before).

### Step B — the unbounded tier (probe round FIRST, then Bryan's gate)

Unlock = lift the D-284 stratification CompileError for computed
cycles (engine.rs ~2811, block "(4) D-284 STRATIFICATION" inside
`add_rules_drl`) so cyclic computed insertLogical — transitive
closure with computed values, fixpoint numerics — runs instead of
rejecting. Known oracle facts: runaway computed logical chains hit
the oracle's fire limit CLEANLY ("fire limit 100000 reached", the
D-013/j21 parity clause covers both-sides-limit = agreement;
ar_tms_runaway_logical / ar_fl_runaway_computed pins, D-282).

Probe round must pin (probes in THIS dir, oracle 3×, guarded singles
for anything cyclic — **cyclic scenarios HANG the oracle JVM batch;
NEVER batch them**, timeout-guard each):
- Bounded fixpoints: `T(n < K)` guarding `insertLogical(new T($n+1))`
  — terminal state, firings, belief-set contents, teardown on root
  delete at depth (does the oracle tear down a 1000-deep chain
  completely? memory/time behavior).
- Value-keyed dedup at depth (the t10/copy-cycle family already
  certified — what changes when values COMPOUND).
- Refire-supersede under deep chains (D-076 epilogue semantics when
  a premise update re-derives a long chain).
- The fire-limit ceiling as the ONLY runaway governor (D-117 spin
  guard is the engine twin); what the oracle's belief state looks
  like AT the limit error (is the batch state discarded — yes,
  scenario errors — so parity is error-vs-error).
- Deep-teardown ORDER observability (SD-census-style shapes at
  depth if reachable in small scenarios).
- After the lift, the 8192 assert MUST GO (a legitimate 100k-fire
  fixpoint tears down ~100k deep — the worklist's boundedness
  replaces the assert); decide with Bryan whether a soft
  diagnostics counter stays.
- Generator: keep gen.rs shapes ACYCLIC (cyclic computed = designed
  runaways; probes carry that surface, fuzz does not). If a cyclic
  axis is ever wanted, it needs fire-limit-safe construction —
  don't build it in this slab.

### Laws that govern this region (verbatim pointers — violating any is a defect)

- ⚖ identity-model law (D-172), ⚖ dedup/side-effect law (D-174),
  ⚖ landing law (D-177) — full statements atop
  `docs/tjupd-ledger-mechanisms.md`. Landing law triage: a
  firing-order divergence around a delete → identify mode × cause
  BEFORE touching the executor (the D-106 region is downstream and
  has never been this family's defect).
- The D-076 quirk model in `tms_route_delete_ex`: justified-key
  delete kills the JUSTIFIED handle whichever handle was named; a
  stated sibling of a once-justified key is a silent no-op (dump3);
  a stated handle with a pending logical belief UNSTAGES it (dump7).
- Refire-supersede (execute_rhs prologue/epilogue): deps not
  re-established by THIS firing are removed; emptied belief sets
  retract their justified facts.
- ⚠ D-106 halt-model caveat: do not patch the agenda executor's
  halt/continue approximation while chasing anything here.
- probes_pending/tms_envelope/HANDOFF-ird.md OWNS the SD/IRD census
  meaning (SD floor 72, seeds, what "divergent" counts) and carries
  its own open ledger (RP2, SD floor, fz_123_6887, order/value
  xfails) — read it before interpreting census output; do not
  conflate its open items with this slab.

## The battery (engine-core edit = the heaviest gate class)

All from repo root, `~/.cargo/bin` on PATH; oracle prebuilt at
oracle/target/classpath.txt (rebuild after Java edits: cd oracle &&
mvn -q -DskipTests package). Redirect oracle runs to FILES
(`2>/dev/null > out.json` — pipes intermittently deliver empty
stdout).

1. Pre-edit worktree: `git worktree add <scratch>/wt_pre <HEAD>` +
   build seine-harness there. NEVER stash/checkout-bisect in place;
   verify `git branch --show-current` == main before every commit.
2. All-scenarios byte gate: `git ls-tree -r <pre> --name-only |
   grep -E '^(scenarios|probes_pending)/.*\.json$'` (minus the new
   probe dir) → run through BOTH binaries → cmp. (D-291 ran 2060.)
3. `make diff` (11/1257/406 green + drift 46 identical) /
   `make lint-probes` / `cargo test` (54).
4. `.venv/bin/maturin develop --release -m bindings/Cargo.toml` +
   pytest (229) + `demo/adsb_convergence.py --selfcheck` (True).
5. Fresh fuzz 2×2000 on 2 NEW seeds; any find → minimize
   (tools/minimize.py) + BISECT vs the pre-edit worktree; only
   in-scope finds are fixes — pre-existing go to scenarios/xfail/
   per D-255 (xf_ name for suppression) + `make xfail-rebank` +
   D-entry. Finding seed must re-run clean.
6. model_ird 31/31 (`.venv/bin/python
   probes_pending/tms_envelope/model_ird.py`).
7. agenda_open ×15: both binaries on probes_pending/agenda_open ×15
   runs, all byte-identical.
8. IRD census: `tools/fuzz_tms_ird.py 150 <seed>` seeds
   7001/7002/6001/6003/9001 → 0-divergent each.
9. **SD census**: `tools/fuzz_tms_sd.py 150 <seed>` seeds
   7001/7002/6001/6003/7004..7011 → divergent
   6+10+3+5+6+5+5+6+8+7+4+7 = **72 EXACT, cell-for-cell**.
10. Step-B deep-chain probes: timeout-guarded singles; watch RSS on
    the deep-teardown cells.

## Open ledger inherited (do not rediscover)

- crates.io Trusted Publishing — Bryan's one-time setup; every tag's
  publish-crates fails until then.
- Collect-order latent family (xf_fz_662607_47 / fz_4649_1144).
- xf_fz_606060_555 + xf_min_606060_555 — acc/setFocus/agenda-group
  FIRE-COUNT latent, pre-existing, D-106-adjacent smell; min repro
  drift-banked.
- Fenced LHS-arithmetic cells re-adjudicate on any oracle bump (the
  D-290 jit race is version-specific); mode-1 residency is a logged
  certification precondition — volume `/` divergences are
  race-suspect first (harness tags them).
- Queue item 4 after this slab: authoring sugar — owns THREE
  refreshes (stale `_rhs_arg` wall message; D-289 SalExpr-skip
  removal; LHS-arith authoring surface).
- Cosmetic: v0.4.31 Actions reds = GitHub-outage reruns.
