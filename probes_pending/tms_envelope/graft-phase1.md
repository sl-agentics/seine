# Member-order graft, phase 1 (D-189): NOT hash texture — three
# physical rules observed, one new fine-print question opened

_2026-07-12. Instrument: `oracle/src/main/java/dev/seine/oracle/SdDump.java`
(ExistsDump clone) — every beta node's left+right tuple memories in
physical iteration order, with fact values, handle ids, and
identityHashCode tags, after every action and every firing. Targets
gt1-gt4 (reduced residue cores), 3 JVM launches each. Protocol +
pre-registered outcomes: rung1-predictions.md §graft._

## Verdict: outcome (A) trajectory — deterministic mechanics, no texture

Cross-launch memory order STABLE 3/3 on all four shapes (identity-tag
diffing). The fence-with-evidence outcome (B) is OFF for these shapes;
the order layer is real mechanics.

## The three observed physical rules

1. **Add-at-head, no reordering in place.** The shared `[P × not-LK2]`
   NotNode left memory reads `(4)(3)(2)(1)` after inserting 1,2,3,4 and
   NEVER changes across firings; deletes remove in place (gt1/gt2/gt4).
2. **Churn replay REVERSES the list.** A break/unbreak cycle on the
   not re-inserts the surviving members at head in scan order —
   `(3)(2)(1)` becomes `(1)(2)(3)` (gt3, NotNode11 between rounds).
   Every FIFO/LIFO flip the retired toggles chased is this reversal
   plus rule 3.
3. **Sharer split by declaration position.** Two rules sharing one
   beta prefix consume the SAME physical list in OPPOSITE orders: the
   first-declared sharer's item list carries staging order (insertion
   FIFO); the later sharer's carries memory-scan order (LIFO of the
   current layout). gt2 vs gt4 swap the assignments exactly with the
   decl swap; all 16 firings fit. Per-path segment/peer mechanics —
   the per-path staged-list dump (phase 2) pins the construction.

## New fine print OPENED by the dump (do not hand-derive)

gt3 FIRING 1 shows `LK2(1,false)` STILL ALIVE in the justifier's own
NotNode right memory while the deleter fires — the lazy drop had NOT
landed at the justifier's expected re-pop. Reading: a clause-B-emptied
item DEQUEUES (the D-091 `!dirty && empty` removal) and the drop rides
its RE-ENTRY, not a phantom pop. Rung 1's a13 (k=0) pinned the drain
at the justifier's slot with an equally-empty item — so the k=0/k=1
(or InitialFact-vs-P-tuple) difference in empty-item drop landing is
UNDETERMINED. Phase-2 targets: (i) per-path SegmentMemory staged-left
lists + peer chains (rule 3's construction + c1-round-1's post-churn
order); (ii) the empty-item drop-landing split (a13-shape vs gt3-shape
with an interposer observer).

## Layer separation (standing, per Bryan's ruling)

- POP-LEVEL mechanism layer: 32/32 banked cells, 132/150 population
  post-bugfix, with F1 (t15 clears fired marks only for tuples that
  DIED in a defeat churn) and F2 (LK re-creation is a NEW object —
  observers refire; the identity law inside the model) as
  principle-grounded corrections, NOT toggles.
- ORDER layer: all 18 remaining population divergences are
  member-order-sourced (12+2 multiset-equal, 4 pick-cascades). The
  four FIFO/LIFO toggles in model_sd are RETIRED as semantics claims —
  they remain only as the in-sample stopgap until rules 1-3 + phase 2
  replace them; the model's docstring order-notes are superseded by
  this file.

# Phase 2 (same instrument + per-path staged lefts/blocked/peers): the
# construction pinned; 0-DIV REACHED

Extended SdDump (PathMemory → SegmentMemories → stagedLeftTuples;
blocked chains; peer chains). Key dumps: gt5 (shared+churn — the
justifier's re-adds visible as seg1 stagedIns in PRE-scan order; the
t10 leak = staged-WITHOUT-dirty, revived by t15's dirty), gt6 (k0-NL:
the off-path fold NETS OUT — no reversal, t0 staging intact), gt7/gt8
(the fold-staging matrix: owner⇒PRE, non-owner justifier⇒POST — by
OWNERSHIP, not staging-presence; gt8 fold-2 is the discriminating
observation). The gt3 "drop rides re-entry" reading from phase 1 was
RETRACTED on re-read: the justifier's not-node rtm holding LK2 at F1
is a stale unevaluated memory; the drop landed at head-return as the
queue-head discipline says.

FINAL STATE: model_sd order layer = the four physical rules (see the
module docstring), zero empirical toggles. **32/32 banked cells;
population 750/750 (0-div) on seeds 6001-6005**, of which 6004/6005
were untouched by any fitting round. Engine-vs-oracle census over the
same 750: 300 divergent (40%) — the port A/B baseline. The lone
flagged order anomaly remaining anywhere: fz_123_3060's T0(5)-first
initial pick (its two-not + no-loop-observer structure; out of the
v1 grammar; noted, not modeled).
