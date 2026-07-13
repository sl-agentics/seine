# I-RD model-extension predictions (logged BEFORE the model runs)

Deliverable: model_ird.py — the executable spec of the three I-RD
mechanisms (D-203/D-204/D-205), the model_check/model_sd pattern: a
small replica validated against ALL banked ird truths (22 cells
across truths/ird{,_ladder,_l56,_x1,_rm,_m67}_oracle_r1.ndj).
Scope: the CELL vocabulary only — the witnesses (or-CEs,
accumulate, epochs, salience formulas) are out of model scope;
their coverage claim rests on the D-204 dump-reads and later on the
port's graduation. Comparison: exact firing sequence (rule + ALL
per-pattern match values, POST-RHS as the harness serializes — m1's
RJ match reads f0=false) + finals as a MULTISET.

## The laws as code (the model's semantic commitments)

1. DYNAMIC: kill of an unstage-born handle never cancels queued
   acts (they fire later with the dead handle's values); every
   other kill cancels acts whose tuple contains the dead handle.
2. STATIC key model (T1/T3 keyed; T0/T2 premise types keyless):
   stated insert appends a WM-visible handle to the value's key
   (label = birth status, unchanged by appends); logical insert
   onto no-key births a JUSTIFIED key with a WM-visible belief,
   onto a JUSTIFIED key folds a dep, onto a STATED-born key sets a
   NON-WM pending belief (+dep). Key-death events: (i) the last dep
   breaking — belief dies, stated siblings ORPHAN, key vanishes
   (L6); (ii) a stated delete on a stated-born MIXED key — the
   deleted handle dies (acts cancel), remaining stated ORPHAN, the
   pending belief UNSTAGES as a WM-visible unstage-born handle, key
   vanishes (r1; b1 = the 0-sibling case). Orphans: alive, keyless,
   UNDELETABLE (delete no-ops; acts on them fire normally — x1).
   A later insert of the value starts a FRESH key; orphans are
   never adopted (l6 pins the fresh-key part; non-adoption is the
   9902 dump's fhs evidence).
3. SAME-BATCH SELF-BREAK: RHS ops apply in order; a premise
   update/delete that breaks a dep lands the break IMMEDIATELY
   (in-flush ≡ eager — one code path for same-batch-self AND
   foreign) — belief dies, its queued acts cancel — EXCEPT
   dep.act == breaking act AND the dep's tuple binds the broken
   fact ≥2× (self-join): then a LAZY-BREAK pseudo-item is scheduled
   at the JUSTIFIER's salience and the belief dies when it pops
   (queued higher-salience acts fire first). Form (modify vs
   update) and source (update vs delete) do not branch anywhere.
4. Tie-break: equal salience pops FIFO by act creation; act
   creation follows flush order (initial facts in array order).
5. Breaks cascade through kills recursively (a2: killing a belief
   that premises another rule's dep breaks that dep eagerly).

## Underdetermined picks (⚖ method law — flagged, not silently chosen)

- The lazy-break's landing slot: m3/m6/m7 each have ONE intervening
  act, so [at-justifier-salience] vs [end-of-agenda] vs
  [before-next-lower-salience] all fit. The model picks
  at-justifier-salience (matches the D-076/D-178 lazy doctrine "at
  the justifier's agenda-item evaluation"). A future cell with two
  observers straddling the justifier's salience would pin it.
- FIFO-within-salience is directly evidenced only by b2 (mid_arm →
  tgt → mid) and l6; the model generalizes it globally.

## Assert-unreachable corners (unpinned; the model RAISES if hit)

- stated delete on a JUSTIFIED-born mixed key (l2-shape + deleter);
- delete of the WM belief while stated siblings exist;
- a break emptying a PENDING (non-WM) belief's deps;
- an update invalidating a queued unfired act's alpha.
No cell reaches these; if one does, the encoding is wrong somewhere
— stop and re-read, don't pick a behavior.

## Per-cell predicted outputs (hand-traced from the laws)

| cell | firing sequence | finals |
|------|-----------------|--------|
| a1 | RJ RD | T0 |
| a2 | RJ RMID RD | T0 |
| b1 | RJ RD RD ROBS(tgt) | T0 |
| b2 | RJ RKS RMID RD ROBS(mid_arm) ROBS(tgt) ROBS(mid) | T0, mid_arm, mid |
| c1 | RD | T0 |
| l1 | ROBS ×3 | T0, v×3 |
| l2 | RJ RS1 RS2 ROBS ×3 | T0, v×3 |
| l3 | RJ ROBS | T0, v |
| l4 | RS1 RS2 ROBS ×3 | T0, v×3 |
| l5 | RJ RS1 RS2 RKILL ROBS ×2 | v×2 |
| l6 | RJ RS1 RS2 RKILL RINS RJ2 ROBS ×3 | T0(false), v×3 |
| x1 | RJ RS1 RKILL RD ROBS | v |
| r1 | RJ RS2 RD RD RD ROBS ×2 | T0, v |
| r2 | RS2 RD RD | T0 |
| m0 | RJ ROBS | T0, v |
| m1 | RJ | T0(false) |
| m2 | RJ | T0(false) |
| m3 | RJ ROBS | T2(1,true) |
| m4 | RJ RU | T0(false) |
| m5 | RJ | (empty) |
| m6 | RJ ROBS | T0(false) |
| m7 | RJ ROBS | T0(false) |

## The pre-registered claim

The model reproduces ALL 22 banked truths (exact firing sequences
with match values + finals multisets) with NO semantic content
beyond the commitments above. Transcription errors (cell registry
typos) surface as FAILs and get fixed as typos; any fix that
requires CHANGING A SEMANTIC COMMITMENT is a FINDING — the laws
under-determine or mis-state something — and gets logged in the
results file explicitly, never patched silently. If an
assert-unreachable corner fires, the encoding (not the corner) is
under suspicion first.
