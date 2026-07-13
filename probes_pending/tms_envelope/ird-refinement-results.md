# I-RD refinement + splitter results
# (predictions: ird-refinement-predictions.md, both rounds logged first)

All oracle runs 3× identity-stable. Truths banked:
truths/ird_rm_oracle_r1.ndj (r1/r2/m0-m5),
truths/ird_m67_oracle_r1.ndj. Ten cells total.

## Results vs predictions

| cell | oracle | engine | pre-registered? |
|------|--------|--------|-----------------|
| r1_multistated_kill | ROBS=2 RD=3 finals=1 | 0 / 2 / 0 | **R-FIRST row exact** |
| r2_two_stated_ctl | 0 / 2 / 0 | same | control holds |
| m0_nobreak_ctl | ROBS=1, v justified in finals | same | scaffold sane |
| m1_samebatch_self_update | **ROBS=0** | 1 | HIGH pred hit |
| m2_samebatch_self_modify | **ROBS=0** | 1 | form-irrelevant |
| m3_samebatch_selfjoin_modify (2442 verbatim) | **ROBS=1** | 1 | 2442 pin reproduces |
| m4_laterbatch_foreign_update | **ROBS=0** | **0** | CONVERGED — foreign row already right |
| m5_samebatch_self_del | **ROBS=0** | 1 | source-irrelevant |
| m6_selfjoin_modify_iso | **ROBS=1** | 1 | self-join axis, in-vocabulary |
| m7_selfjoin_update_iso | **ROBS=1** | 1 | join×form: no interaction |

## ⚖ THE SAME-BATCH SELF-BREAK LAW (the third mechanism, pinned)

When a justifier's RHS breaks its own premise in the same firing
that staged the belief's insert, the break lands WITHIN the flush:
no act on the belief ever fires. FORM-irrelevant (modify-block ≡
setter+update; m1/m2) and SOURCE-irrelevant (update ≡ delete;
m1/m5). **EXCEPTION (the 2442 shape): when the justifying tuple
binds the broken fact MORE THAN ONCE (self-join), the break lands
LAZILY — already-queued higher-salience acts fire on the belief
before the retract** (m3 verbatim; m6/m7 isolate the axis in the
m1/m2 vocabulary: adding ONE redundant binding flips the behavior,
under both forms). Foreign breaks (a different activation updating
the premise) cancel the belief's queued acts eagerly — m4, where
the ENGINE ALREADY CONVERGES; its lazy path never covered foreign.

Engine status by row: single-binding same-batch self = WRONG (lazy;
m1/m2/m5 divergent — the fix target, covering witnesses
fz_777_2956 + fz_7_1591 + fz_7_5988); self-join same-batch self =
RIGHT (m3/m6/m7); foreign = RIGHT (m4). Port surface: the
current_act exclusion in tms_eager_break must NARROW — lazy only
when the justifying tuple contains the broken fact ≥2×;
single-binding self-breaks land in-flush like every other break.
(Drools's real discriminator may be something that merely
correlates with the double-binding — e.g. the ModifyPreviousTuples
pass meeting the fact under a second binding — but the OBSERVABLE
law is the double-binding, and the cells pin it under both forms
and both sources.)

## ⚖ THE MIXED-KEY KILL REFINEMENT (static face, pinned by r1/r2)

r1 came out R-FIRST exactly (oracle ROBS=2, RD=3, finals=1): on a
stated-born mixed key, a stated delete kills the key WHOLE — the
belief UNSTAGES into WM and ALL remaining stated siblings ORPHAN
(WM-alive, x1-undeletable — the finals survivor; the unstage-born
handle's queued acts survive per the dynamic law — the +1 ROBS).
UNIFIED STATEMENT: **any stated delete on a mixed key (belief
sibling present) kills the key whole — belief unstages, all other
handles orphan**; 4048/b1's single-stated case is the degenerate
no-orphan instance, 8757/r1 the general one. r2 guards the scope:
NO belief ⇒ the key loses stated handles one at a time, both
deletes land (2/0/0 both sides) — the event is belief-dependent.
The static law's key-death-whole events are now TWO:
(i) last-justification-break (L6), (ii) stated-delete-on-mixed-key
(r1). Both orphan the survivors.

## ⚠ The engine r1 anomaly (port-time recon, now minimal)

Engine r1: RD ×2, finals 0, ROBS 0 — the second stated delete
should hit the unstage gate (stated empty + beliefs non-empty) and
materialize the belief for a third RD kill; instead the belief
vanishes without materializing. Hypothesis list for the port recon
(do NOT armchair-fix): pending_vals None at the second delete
(reset by the s2 append? never set because the belief arrived
between two stateds?), or the had_justified no-op eating the
second delete. The clean cell replaces 8757 as the recon shape.

## Ledger impact

- Third mechanism: PINNED with cells (m0-m7); witnesses
  {2956, 1591, 5988} fully explained; m4/m3/m6/m7 certify the
  boundary rows the port must NOT change.
- Static face: the r1 refinement folds into the key-lifecycle law;
  witness 8757 fully explained (orphan survivor = its finals +1).
- Divergent cells (open_divergence): r1, m1, m2, m5 — port targets
  alongside b1/b2/l6.
- Next per the plan: model extension over all three mechanisms
  (dynamic law + static law incl. both key-death events + the
  same-batch self-break law with its self-join exception),
  population, port validate-and-revert (Bryan gates).
