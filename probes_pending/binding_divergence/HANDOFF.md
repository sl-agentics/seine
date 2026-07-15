# HANDOFF — the binding-divergence family (fz_4242_286 + fz_5150_1857): TWO mechanisms pinned, ports pending

**BOTH LANES LANDED — THE ARC IS CLOSED.**

**LANE 1 LANDED 2026-07-15 (D-262, Bryan-approved with the
no-order-pinning constraint as the deliverable).** The halt-check
peek now walks the group's members in PICK order (item salience
DESC, decl ASC) and STOPS at the first live item — members below it
stay dirty and materialize at their own pop (Drools' lazy timing).
NO ordering was encoded: the fix only narrows WHICH members the peek
evaluates; queue construction and emission are untouched. Four
order-pin guard cells (pr_bd_g1..g4) certify the constraint — g1
(update re-stage), g2 (delete cancel), g3 (join-fed accumulate)
would diverge under any synthetic merge/sort rule; g4 (receiver
decl-first, salience-lower) discriminates decl-order walks.
fz_4242_286 graduated to scenarios/regressions/; bonus oracle-ward
flip fz_9105_8945 (agenda_open, lazy self-join receiver — lane 1's
unplanned out-of-sample instance) graduated too (agenda_open ×15).
All remaining family cells graduated to scenarios/probes/pr_bd_*;
this HANDOFF is the arc record. See D-262.

**LANE 2 LANDED 2026-07-15 (D-261, Bryan-approved: red-first,
validate-and-revert, complete TMS battery — all green, no revert).**
The eager-list flush is extracted to `eager_flush()` and runs at the
same-rule sibling-continue (a firing boundary). fz_5150_1857 +
fz_9103_1436 (agenda_open, dyn-salience variant — an unplanned
out-of-sample instance) graduated to scenarios/regressions/; bd_d4 +
4 lane-2 controls graduated to scenarios/probes/pr_bd_*. See D-261
for the landing receipts.

Recon DONE (2026-07-15, session after D-258/D-259, HEAD `19a3fe2`;
Bryan directed the arc: "Start on fz_4242_286 + fz_5150_1857"). Both
xfail witnesses are cracked. They are the SAME probe family (agenda
focus × RHS inserts × pop order) but TWO DISTINCT engine mechanisms —
both evaluation-TIMING side effects of the ⚠⚠ D-106 agenda-executor
model, NOT halt/continue control-flow errors. Ports are Bryan-gated.

## The shared skeleton

A MAIN rule fires `setFocus("ga")`; a ga rule's RHS inserts facts
matching a multi-activation receiver rule's patterns; the receiver's
firing ORDER of old-vs-new activations diverges. The engine pops
static-salience queues FIFO (`RuleExecutor.getNextTuple` removeFirst —
certified D-043), so order is decided entirely by WHEN each queue
materializes and WHERE staged adds land.

## Mechanism 1 — lazy-materialization pinning (fz_4242_286, `bd_min4242`, `bd_a3`, `bd_pred_a`)

The **D-106 halt-check force-evaluation** (engine.rs ~7201-7210)
materializes the focused group's empty+dirty members' queues at the
setFocus firing — BEFORE the higher-salience sibling's inserts. The
late result tuple then APPENDS to the pre-built batch. Drools
evaluates a lazy (static, non-no-loop) rule's network ONCE at its
pop, AFTER the inserts — and the accumulate lane's one-batch emission
is REVERSE-insertion, so the newest insert fires FIRST.

- Engine: `[early batch t1b,t1a] + append(NEW)` → t1b,t1a,NEW.
- Oracle: one batch, reverse emission → NEW,t1b,t1a.

Invisible for plain/single-pattern/join receivers (forward emission ==
tail-append, `bd_a`/`bd_a2` PASS) and for no-loop receivers (Drools is
eager there too and materializes early itself — `bd_e2` PASS). Needs
the insert to land mid-queue (`bd_c3` drain-first PASS; `bd_b3`
no-focus PASS). **Out-of-sample prediction verified exactly** —
`bd_pred_a` (two inserter siblings @4/@2): oracle NEW6,NEW2,t1b,t1a
(one reverse batch), engine t1b,t1a,NEW6,NEW2.

## Mechanism 2 — the eager-flush skip (fz_5150_1857, `bd_d4`)

The **D-106 same-rule sibling-continue** (engine.rs ~7218,
`else if !higher && !focus_stack.is_empty() { return Some(l) }`)
returns BEFORE the eager-list flush (~7223). A ga rule firing twice
CONSECUTIVELY (two activations of the same rule) therefore never
lets an EAGER (no-loop) receiver evaluate between the firings —
the two staged inserts coalesce into ONE evaluation whose self-join
emission is left-delta-major over the FINAL memory. Drools evaluates
eager rules per firing: per-delta batches, FIFO across batches.

- Engine (one shot): (N⁻²,N⁻²),(N⁻²,N⁵),(N⁻²,t1c),(N⁵,·)×3,(t1c,·)×2
  — note (N⁻²,N⁵) in the first group: N⁵ did not exist at the first
  firing.
- Oracle (per delta): batch1 = (N⁻²,N⁻²),(N⁻²,t1c),(t1c,N⁻²); then
  batch2 = the N⁵ rows.

Trace-pinned: `EVAL[eager]` absent between the two `post-fire-force
rule 0` lines in ga (`bd_d4`), present between every firing in MAIN
(`bd_b4` PASS). Needs the SELF-join (left+right deltas; hetero-join
left-only deltas emission-coincide, `bd_e1` PASS) and the LAZY control
`bd_d3` PASSes (truly-lazy one-batch emission agrees on both sides).
**Out-of-sample prediction verified** — `bd_pred_b` (the two inserts
split across two ga RULES): the pick cycle between different rules
runs the eager list → per-delta restored → PASS.

## The matrix (15 cells filed in scenarios/, all verified on `19a3fe2`)

| cell | receiver | focus | eager | result |
|---|---|---|---|---|
| bd_min4242 | T1+acc | ga | lazy | FAIL — lane 1 (minimized witness) |
| bd_a3 | T1+acc | ga | lazy | FAIL — lane 1 clean cell |
| bd_pred_a | T1+acc, 2 inserters | ga | lazy | FAIL — lane 1 prediction, exact |
| bd_d4 | T1×T1 self-join | ga | no-loop | FAIL — lane 2 (= min fz_5150_1857) |
| bd_a / bd_a2 | single / T1×T0 | ga | lazy | PASS — forward emission masks lane 1 |
| bd_b / bd_b3 / bd_b4 | (each) | none | (each) | PASS — no focus, no divergence |
| bd_c / bd_c3 | drains pre-insert | ga | lazy | PASS — no batch mix |
| bd_d3 | self-join | ga | lazy | PASS — both sides one-batch |
| bd_e1 | T1×T0 hetero | ga | no-loop | PASS — left-only deltas coincide |
| bd_e2 | T1+acc | ga | no-loop | PASS — Drools eager too, re-converges |
| bd_pred_b | self-join, 2 inserter rules | ga | no-loop | PASS — lane 2 prediction |

## Port sketches (Bryan gates; both sit in the ⚠⚠ D-106 region — full
battery per D-254/D-258 protocol; treat sketches as CANDIDATES, not
proven sufficient — D-256 precedent)

1. **Lane 2 (likely smaller):** run the eager-list flush before (or
   on) the same-rule sibling-continue return, mirroring Drools'
   per-firing eager evaluation. ⚠ the eager list also runs
   `tms_flush_drain` — the P3/D-199/D-201 landing laws live there;
   expect TMS battery sensitivity.
2. **Lane 1 (subtler):** the halt-check needs member EMPTINESS without
   pinning lazy members' batch order. Candidates: peek-evaluate into
   scratch without committing queue order; or mark queues
   materialized-by-peek and rebuild at first pop. Touches certified
   D-106 fine structure (the 88-witness matrix) — re-verify the whole
   halt matrix.
3. The two xfail witnesses stay quarantined until a port lands; then
   graduate + rebank per the D-258 flow (they are the drift-bank
   anchors for this family).

## Env crumbs

- Repro: `cargo run -q -p seine-harness -- diff probes_pending/binding_divergence/scenarios/bd_*.json`
  (FAIL cells carry `open_divergence: true` for lint).
- Traces: `SEINE_EVAL_DEBUG=1` (`EVAL[eager]`/`EVAL[pop]` timing);
  the queue push/pop sites are `push_activation` (~9530) and the
  static-FIFO pop (~6742).
- `tools/minimize.py <xfail> <out>` reproduced both minimal shapes in
  one pass (~5 min each).
