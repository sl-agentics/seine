# Rung 2 — the L-SD row completed: THREE CLAUSES, and every bucket
# member accounted for (D-188)

_2026-07-11, Bryan's sequencing ("the row isn't done until its own
outliers are accounted for"). 9 new cells (sd_b1..b7, sd_c1/c2), every
oracle row 3×-stable; predictions pre-logged per round
(rung1-predictions.md, rung-2 sections); cache-driven trace reads for
the three named witnesses + a mechanical 13-witness retrodiction
sweep. The over-fire outliers DISSOLVED into the table (they were the
missing second clause), and the sweep's two justifier-under-fire
misfits became the third._

## THE THREE-CLAUSE ROW — (cloud × self-defeat) landing

**A — landing / queue position (rung 1, reconfirmed b5/b6/b7).** A
LAZY justifier's self-defeat drop lands at its ITEM POP; same-salience
observers glimpse the transient iff their queue position PRECEDES the
justifier's item (decl order in same-firing-born shapes); an EAGER
(no-loop/dyn-sal) justifier lands at the firing's flush (no ≤-salience
glimpse); strictly-higher always glimpses, strictly-lower never; k
irrelevant. An item that pops fires its WHOLE tuple list before the
drain (b7: [RJ, RO, RO, RO] from a join observer over P×3).

**B — in-firing self-cancellation (new; the over-fire outliers).** The
justifier's OWN remaining same-item tuples — fan-out activations AND
or-twin branch activations — die IN-FIRING at the self-break, before
the next tuple fires, in BOTH regimes (b2 lazy, b4 no-loop). The
ENGINE's cancellation is topology-dependent: trailing-not works (b1
green), leading-not (b2) and or-twin (b4) fire the corpse tuples.

**C — post-drop re-derivation (new; the justifier-under-fire
misfits).** After the drop lands: with NO WM change there is NO refire
(c2 green — t10's dead-blocker leak, scope confirmed). A left-side WM
change (t15's revive) re-derives the justifier's remaining tuples, and
its re-queued item competes at its salience — a strictly-higher
re-queue preempts the changer after ONE firing, producing STRICT
ALTERNATION (c1: oracle [RJ,RD]×3 pairs; the D-091 halt structure is
the same mechanism). The ENGINE batches the changer's item and starves
the refires (c1: [RJ, RD, RD, RD]).

Consequences propagate ordinarily: a glimpsed firing's own
insertLogical PERSISTS unless it breaks its own guard (D-076
lifecycle) — fz_7_1353's whole 8-firing/5-fact loss is clause A on the
bootstrap + this persistence; no fourth clause.

## Bonus boundary find

sd_b3: the bare LAZY or-twin self-justifier is a genuine DROOLS
RUNAWAY (fire-limit 3/3) — the fz_42_946 family in a 1-rule minimal;
`no-loop` is exactly what makes 9375's or-twin terminate (b4). The
terminate/runaway boundary is now a constructed pair, not just fuzz
census.

## The bucket, fully accounted (13/13)

| witness | account |
|---|---|
| min812, 2135, 3370, 4318, 7637, 9637, 812, 9864 | clause A (observer decl-before a lazy justifier, equal salience) — sweep-verified count-level |
| fz_123_9133 | A (join-observer ×3 glimpse, b7) + B honored by engine (trailing-not) |
| fz_123_3060 | **B** (leading-not fan-out; b2 minimal) — was "over-fire outlier" |
| fz_7_9375 | **B** (or-twin, eager; b4 minimal) — was "over-fire outlier" |
| fz_42_5213 | A + B + **C** (strict alternation; c1 minimal) |
| fz_7_1353 | A (bootstrap glimpse) + B (or-twin ×1) + cascade persistence |

Sweep tool: mechanical count-level classification over the 10-replicate
cache (justifier over-fire → B; decl-before observer under-fire → A;
justifier under-fire → C-candidate → trace-read). 11/13 auto-fit,
2 trace-read, 0 unexplained.

## Method-law fine print

- Clause B's "in-firing" site is pinned at the output level (the next
  same-item tuple never fires); whether Drools folds at the WM action
  or at a between-tuples network eval is NOT determined by these cells
  — port-phase reading, not a behavioral difference in this shape
  class.
- Clause C's alternation is pinned for strictly-higher re-queues; the
  equal-salience re-queue case (does the changer's item finish its
  list before the re-derived justifier pops?) is UNPROBED — flag for
  the model phase, one cell (c3) when needed.
- The b5 prediction slip (RO×3 on a single-activation observer) was a
  cell-design error caught by the run, not an oracle surprise; b7 is
  the clean whole-tuple-list witness.

## Standing after rung 2

The L-SD sub-family is CLOSED at the recon level: three clauses, every
bucket member accounted, 6 new open_divergence witnesses
(b2/b4/b5/b6/b7/c1) + rung 1's 4 (a9/a10/a13/a15) = the port battery's
core. Engine gaps, stated: (1) early drain at the continuation instead
of pop-landing (A); (2) topology-dependent in-firing cancellation miss
(B: leading-not, or-twin); (3) changer-batching instead of
halt-alternation (C). All three sit in the evaluation-lifecycle region
(⚠ D-106 tripwire: agenda_open ×19 receipts; the D-177
landing-not-pick pattern). NEXT per plan §5: L-MB (the 18-witness
mutation-break cluster) — or the model phase for L-SD first if Bryan
prefers consolidation before the second front.
