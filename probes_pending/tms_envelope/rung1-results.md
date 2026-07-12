# Rung 1 — L-SD results: the (cloud × self-defeat) landing row is
# PINNED, with an eagerness split that reconciles min608 (D-187)

_2026-07-11. 14 cells, 3 designed rounds, every oracle row 3×-stable
(fresh JVM per replicate). Predictions logged before every round
(rung1-predictions.md — round 1 falsified its own lead hypothesis;
rounds 2–3 landed 7/7 exact). Cells: `sd_a*.json` here; 4 are
open_divergence (a9/a10/a13/a15), 10 are live controls._

## THE ROW (first confirmed entry of the (cloud × self-defeat) table)

**A self-defeat belief drop (a lazy justifier's insertLogical breaking
its own not-CE) lands at the justifier's ITEM POP, not at its
post-firing continuation. Same-salience observers therefore glimpse the
transient iff their queue position PRECEDES the justifier's item —
declaration order, in same-firing-born shapes. An EAGER justifier
(no-loop/dyn-salience) instead lands the drop at the firing's
eager-flush: no same-or-lower-salience observer ever glimpses,
queue position irrelevant. Strictly-higher observers glimpse under
both regimes (t11); strictly-lower never. k is irrelevant (a15).**

The ENGINE lands the lazy case early (continuation-time), uniformly —
so it under-fires exactly the observers declared before a lazy
justifier at equal salience. That is min812's mechanism (a10), and the
accumulate in min812 was never load-bearing (a9 diverges identically
with a plain observer; a5's acc-after-justifier is green).

## The evidence grid

| cell | shape | oracle 3× | engine | verdict |
|---|---|---|---|---|
| a2/a5 | plain/acc observer AFTER justifier, eq sal | [RJ] | [RJ] | GREEN |
| a3/a6 | observer @+10 | [RJ, RO] | [RJ, RO] | GREEN |
| a4/a7 | observer @−10 | [RJ] | [RJ] | GREEN |
| a8 | acc+plain both AFTER, eq | [RJ] | [RJ] | GREEN |
| **a9** | plain observer BEFORE, eq | **[RJ, RO]** | [RJ] | **RED** |
| **a10** | acc observer BEFORE, eq (min812 minimal) | **[RJ, RO]** | [RJ] | **RED** |
| a11/a12 | observer BEFORE @−10/@+10 | [RJ] / [RJ, RO] | same | GREEN |
| **a13** | plain BEFORE + plain AFTER, both eq | **[RJ, RO]** (RO2 silent) | [RJ] | **RED** |
| a14 | observer BEFORE, eq, justifier **no-loop** | [RJ] | [RJ] | GREEN |
| **a15** | observer BEFORE, eq, justifier k=1 lazy | **[RJ, RO]** | [RJ] | **RED** |

a13 is the decisive cell: two IDENTICAL plain observers at equal
salience split exactly at the justifier's declaration slot — the drain
occupies the justifier's queue position. a14×a15 discriminate the
min608 reconciliation: eagerness splits the row, k does not.

## The min608 reconciliation (⚖ method law, applied to a standing pin)

D-076 drain point (a) — "the post-firing continuation drains unless a
strictly-higher item waits; equal salience/earlier decl does NOT
preempt (min608 vs t11)" — was pinned on the fz_7_608 family, whose
justifier carries **no-loop**. What min608 actually pinned is the EAGER
flush drain (point (b)'s regime); the "continuation drains at equal
salience" reading over-generalized to lazy justifiers, and the engine
implemented the over-generalization. Both pins stand within their real
scope: min608 = the eager row (a14 reproduces it), t11 = the
strictly-higher rule (a3/a6/a12 reproduce it). The lazy-equal case was
never discriminated until a9/a13.

## Method-law fine print (underdetermined, stated as such)

- "Queue position = declaration order" is pinned only for
  same-firing-born shapes, where loadOrder and within-firing FIFO
  coincide; a splitter would need an observer item that enters the
  equal bucket at a different time than its decl order implies, which
  the D-091 empty-item-removal rule makes hard to construct. The row is
  stated at the queue-position level; decl order is its observed
  realization here.
- The ENGINE-side mechanism (which code path drains early) is NOT
  pinned from these cells — that is the port phase's job, reading
  engine.rs against the row. ⚠ Executor-adjacent: any fix carries the
  D-106 tripwire (agenda_open ×19 byte-identical receipts), and the
  D-177 pattern (move the LANDING, never touch the pick).

## Standing after rung 1

- The row retrodicts the L-SD bucket's 11/13 under-fire census
  direction. The two OVER-fire outliers (fz_123_3060, fz_7_9375) are
  NOT explained by this row (it predicts engine-under only) — they are
  rung 2's kill-cells, alongside the fz_123_9133 fan-out spine
  (multi-activation justifier × in-firing continuation semantics).
- Not yet touched: L-MB (mutation-break) rows, I-RD graft, the
  9133-class per-tuple fan-out. The row above covers the single-shot
  self-defeat shape only.
