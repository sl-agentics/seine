# HANDOFF — the agenda late-continue latent (fz_9901_1221): validated fix, port pending

**LANDED 2026-07-15 (D-258, Bryan-directed).** The patch is in the engine
with a D-entry-grade comment; the TMS-drain scope question was answered NO
by 5 recon cells (`pr_alc_tms_*`); the 9 alc_ cells graduated to
`scenarios/probes/pr_alc_*`; fz_9901_1221 + the two agenda_open witnesses
(fz_9104_5192, fz_9202_2058) graduated to `scenarios/regressions/`; drift
bank rebanked 35→34. agenda_open is now ×17. This file + the patch stay as
the arc record; see D-258 in DECISIONS.md.

Cold-start file for the Bryan-gated engine port. Recon is DONE (2026-07-15,
session after D-257/v0.4.25, HEAD `9c4e23a`): mechanism trace-pinned, fix
candidate written and validated against the full battery in a scratch
worktree. Nothing is committed for this arc except this directory.

## What this is

`scenarios/xfail/xf_fz_9901_1221.json` (filed D-254, quarantined D-255):
firing count engine 4 vs oracle 2 on a 4-fact composition, bisected
pre-existing at `4ee9c02`. Cracked: it is a **D-106 agenda-executor defect**,
not a query/resize issue. Two of the standing D-106 disproof witnesses in
`probes_pending/agenda_open/` (`fz_9104_5192`, `fz_9202_2058`) are the SAME
family and flip FAIL→PASS under the fix.

## Mechanism (trace-pinned via SEINE_EVAL_DEBUG=1 SEINE_AG_DEBUG=1)

The **D-106 late-continue** at `engine/src/engine.rs` ~7215
(`if top_empty && pre_force_qlen > 0 { return Some(l) }`) returns the
just-fired rule WITHOUT the **D-091 continue-path self re-evaluation**.
The branch is only reachable when `higher=true` (a higher-salience item
waits in the focus-top group) — exactly the condition that made the D-091
halt path skip `evaluate_rule(l, true, false)`. Sequence in the witness:

1. R4 (MAIN) fires; its RHS runs `setFocus("gb")` (pushes gb; R0@10 waits
   there, dirty/never-evaluated) + `delete($p)` (stages a WM delete that
   should cancel R4's own remaining activations).
2. `next_activation(Some(R4))`: `higher=true` (R0@10 in top gb) → post-fire
   force SKIPPED → R4's queue keeps 2 stale items. The halt-check
   force-evaluates gb's empty+dirty members: R0 evaluates to empty (its
   staged inserts coalesce with the staged delete) → `top_empty` →
   **`return Some(l)`** with l's dirty network never drained.
3. Exactly one stale activation fires (the LIFO-next). The NEXT
   `next_activation` call has `higher=false`, runs the post-fire force, and
   cancels the rest. Hence one extra firing per occurrence (per epoch in the
   witness: 4 vs 2).

Drools halts to the agenda instead; `evaluateNetworkIfDirty` at each item
pop cancels the siblings first. The sibling engine path at ~7218
(`else if !higher && ...`) is safe — only reachable AFTER the self re-eval.

The defect is not delete-specific: `fz_9104_5192` is the `update` variant
(stale pre-update bindings fire). Trigger shape = same-firing RHS does
setFocus(G) + WM mutation, and G's items all evaluate to empty at the
halt-check.

## Discriminating matrix (filed as scenarios/, all verified on HEAD)

| cell | HEAD | shows |
|---|---|---|
| alc_base_3t1 | FAIL 2v1 | base: setFocus+delete, 3 T1s — one stale sibling fires |
| alc_min_2t1 | FAIL 2v1 | minimal: 2 activations, both fire |
| alc_delfirst | FAIL 2v1 | RHS order (delete-then-setFocus) irrelevant |
| alc_survivor_pick | FAIL 2v1 | extra MAIN R8@-5 on T0: R8 cancelled; survivor = exactly the next pick |
| alc_nofocus_pass | PASS | delete without setFocus cancels correctly |
| alc_bare_pass | PASS | delete-only RHS |
| alc_focusonly_pass | PASS | setFocus without delete |
| alc_gbalive_pass | PASS | live rule in gb → normal pick path reconciles |
| alc_gbnever_pass | PASS | gb never populated → empty push is a no-op |

(A 10th cell — setFocus to a group no rule declares — hits the certified
D-106 wall on both sides; not filed.)

## The validated fix candidate

`late-continue.patch` in this directory (15 lines, applies to `9c4e23a`;
`git apply probes_pending/agenda_late_continue/late-continue.patch`).
Shape: at the late-continue, if `higher && nets[l].dirty` run
`evaluate_rule(l, true, false)` (the same call the halt path skipped);
return `Some(l)` only if l's queue survives; else fall through, with the
D-091 `removeRuleAgendaItemWhenEmpty` unqueue when `!dirty`.
⚠ The patch comment says "HYPOTHESIS TEST" — rewrite it as a proper
D-entry-grade comment when landing.

Receipts (all run in the scratch worktree on `9c4e23a` + patch):
- corpus `make diff` 11/1176/397 ALL GREEN (every certified D-106 pin holds);
  xfail drift gate flags EXACTLY `fz_9901_1221: firings 4 -> 2` (the expected
  deliberate movement — needs re-triage + `make xfail-rebank` + D-entry).
- `cargo test` green (22 + integration suites).
- agenda_open ×19: 17 byte-identical, `fz_9104_5192` + `fz_9202_2058`
  FAIL→PASS (both setFocus + same-firing RHS mutation; part of the D-106
  disproof set).
- fuzz 3×2000 (seeds 9901/4242/777): 0 in-scope divergences. Two flags are
  KNOWN pre-existing latents, byte-identical on clean HEAD: `fz_4242_286`
  (different family — binding divergence, untouched by this patch) and
  `fz_777_1086` (documented D-163-era seed-777 latent).
- xf_fz_9901_1221 and all alc_* FAIL cells → PASS under the patch.

## Port checklist (Bryan gates the engine edit)

1. Apply the patch to main; rewrite the comment (D-091 continue-path
   re-evaluation at the D-106 late-continue; cite the new D-entry).
2. OPEN DECISION: should the late-continue also run the `!higher` branch's
   TMS deferred-drain block (engine.rs ~7060-7136)? Untested — no TMS shape
   in this family. Recon with a TMS×setFocus probe or document the scope.
3. Graduate `xf_fz_9901_1221` → `scenarios/probes/` (drop the
   open_divergence/_finding markers), `make xfail-rebank` 35→34.
4. Promote the 4 alc_ FAIL cells to `scenarios/probes/pr_*`; keep or promote
   the 5 PASS controls (they certify the neighboring behavior).
5. Decide graduation of `fz_9104_5192`/`fz_9202_2058` out of
   `probes_pending/agenda_open/` (they become certified pins). NOTE: every
   receipts line that says "agenda_open ×19 byte-identical" changes meaning
   — 2 of 19 now PASS; re-baseline that check.
6. Full battery per D-254 precedent (engine edit): make diff + drift, make
   lint-probes, cargo test, model_ird 31/31, witnesses 26/26, SD census 72
   EXACT + 0-div, ird 0×5, bindings pytest, demo selfcheck, multi-seed fuzz
   with fresh seeds. D-entry; NO push, NO tag, NO bump.

## Side findings (independent of the port)

- **xfail fuzz suppression is broken for the D-255 re-files:** the fuzzer
  checks `scenarios/xfail/<name>.json` (harness/src/main.rs ~173) but D-255
  renamed the files `xf_<name>.json` — re-fuzzing seed 4242 reports
  fz_4242_286 as DIVERGENCE (and copies it into the GATED scenarios/failures/
  — the exact D-255 CI trap) instead of XFAIL. Fix the lookup (also try
  `xf_{name}.json`) or the naming; small harness change, worth its own entry.
- `fz_4242_286` (binding-divergence family) and `fz_31337_698` (oracle-side
  NPE) are NOT this family — both byte-identical under the patch, stay open.

## Env crumbs (repeat offenders)

- `export PATH="$HOME/.cargo/bin:$PATH"`; run from repo root.
- A scratch worktree needs `ln -sfn <main>/oracle/target <wt>/oracle/target`
  (classpath.txt is cwd-relative).
- Pipes from `cargo run` intermittently deliver empty stdout — redirect to a
  file and read it (workflow memory, 2026-07-11 quirk).
- Repro: `cargo run -q -p seine-harness -- diff scenarios/xfail/xf_fz_9901_1221.json`
  and `... -- diff probes_pending/agenda_late_continue/scenarios/*.json`.
