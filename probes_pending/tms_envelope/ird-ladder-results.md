# I-RD 9902 ladder results (predictions: ird-ladder-predictions.md)

All oracle runs 3× identity-stable; SdDump 3× identity-normalized
stable on L2/L3/L6. Truths banked: truths/ird_ladder_oracle_r1.ndj
(L1-L4), truths/ird_l56_oracle_r1.ndj, truths/ird_x1_oracle_r1.ndj.

## Round 1 — the insert-route counts (L1-L4)

| rung | oracle pred | ORACLE | engine |
|------|-------------|--------|--------|
| ird_l1_stated_x3 | 3 | **3** | 3 |
| ird_l2_stated_onto_justified | 3 | **3** | 3 |
| ird_l3_justified_onto_stated | 1 | **1** | 1 |
| ird_l4_stated_rhs_onto_external | 3 | **3** | 3 |

Oracle: all four predictions hit. Engine: MATCHES THE ORACLE on
every rung — ⚠ the banked "the engine's value-keyed store dedups"
read (graft_targets/ird/README) is WRONG as a general statement.
Measured 9902 delta: firings 14/14 identical; finals differ by
EXACTLY ONE T1(false,true) (oracle 7 handles, engine 6) — the value
whose key died and was reborn across epoch 1.

## Round 2 — the break/re-justify discriminator (L5/L6)

| rung | oracle pred | ORACLE | engine pred | engine |
|------|-------------|--------|-------------|--------|
| ird_l5_break_orphan | 2/2 | **2/2** | 2/2 | 2/2 |
| ird_l6_break_rejustify | 3/3 | **3/3** | 2/2 | **2/2** |

Both predictions hit. The L6 SdDump shows every joint directly:
F0-F2 `JUSTIFIED fhs[@4+@5+@6+]` (stated siblings append, label
stays JUSTIFIED); F4 `TMS keys:` EMPTY (the break kills the WHOLE
key — the stated siblings are ORPHANED alive in WM); F5 `JUSTIFIED
fhs[@8+]lfh=@8` (the re-justification starts a FRESH key, its
handle WM-VISIBLE; the orphans never join it).

## ⚖ THE KEY-LIFECYCLE LAW (the identity law's static face, pinned)

1. Stated inserts of one value COEXIST as separate WM-visible
   handles; external and RHS routes identical (L1/L4).
2. A key born JUSTIFIED keeps the JUSTIFIED label; its belief
   handle is WM-visible; later stated inserts APPEND WM-visible
   siblings (L2; 9902's fhs[@4+@8+@14+@20+]).
3. A key born STATED takes a justified insert as a NON-WM sibling
   (lfh=belief, pending_vals; L3; 4048-F1) — the unstage precursor.
4. When the LAST justification breaks, the key DIES WHOLE: the
   belief handle retracts and the stated siblings are ORPHANED —
   WM-alive, dropped from key bookkeeping. A later re-justification
   of the value starts a FRESH key under rule 2 (L6; 9902's @6/@7
   orphans + fresh @12 key).

THE ENGINE'S ONE MISS = rule 4: it keeps the key alive as STATED
(had_justified) after the break, so a re-justification takes rule
3's non-WM route — one WM handle short per key rebirth. That is the
whole fz_7_9902 divergence. Engine fix altitude: the TMS key model
(key death at belief-empty-with-siblings + an orphan set), not the
executor.

## Round 3 — the unification cell (x1): REFUTED, with a bonus pin

ird_x1_orphan_del asked whether a break-orphan's delete also skips
act-cancel (which would unify both faces under one TMS-map-absence
law — the seventh law). Outcome: NEITHER predicted branch —
**the delete itself NO-OPS** (finals keep v ×1 BOTH sides;
sequence identical). The orphaned stated sibling is UNDELETABLE:
- oracle: the TMS-dropped orphan's delete is absorbed;
- engine: the dump3 had_justified quirk no-ops it — different
  internal model, same observable, NO divergence.

Consequences: the deletable TMS-dropped population = unstage-born
handles ONLY, so P-ORIGIN vs P-MAP-ABSENCE cannot be split further
and the SURVIVE-THE-DELETE LAW stays in its observable ORIGIN-KEYED
form (ird-results.md). No unified seventh law: the dynamic face
(what a delete can cancel) and the static face (what a key can
hold) remain two laws sharing one root (Drools's per-handle TMS
bookkeeping drops handles the engine keeps tracking).

## I-RD ledger impact (to re-check after the model/port)

The two laws' predicted coverage of the ~8 open witnesses:
fz_7_4048 = the dynamic law exactly (one missing post-delete
firing). fz_7_9902 = the static law rule 4 exactly (one-short
finals). The remaining six (fz_123_7219, fz_42_6368,
fz_777_{1278,2956}, fz_7_{1591,5988,8757}) need their dumps read
against both laws before any port — per the plan: model extension
first, population, then port validate-and-revert with FULL receipts
(red-first; Bryan gates).
