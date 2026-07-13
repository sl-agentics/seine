# I-RD population results (predictions: ird-population-predictions.md)

tools/fuzz_tms_ird.py, five seeds × 150 (comparison 7001/7002/6001/
6003 + fresh 9001). **THE 0-DIV GATE IS RED: 5 REAL mismatches /
750 cases (745 model-clean)** — two distinct findings, BOTH landing
on pre-registered at-risk axes. Per the pre-registered gate meaning:
law-gap cells BEFORE the port; the model is NOT yet population-green.

| seed | clean | REAL | corners (pend-dep / belief-del) | engine-div |
|------|-------|------|--------------------------------|-----------|
| 7001 | 118 | 0 | 32 / 0 | 12 |
| 7002 | 108 | 0 | 39 / 3 | 14 |
| 6001 | 118 | 1 | 29 / 2 | 21 |
| 6003 | 112 | 2 | 35 / 1 | 25 |
| 9001 | 111 | 2 | 37 / 0 | 14 |

Oracle stability: 0 flaky (TMS bar quiet, as predicted). Witnesses
regenerate deterministically: `python3 tools/fuzz_tms_ird.py 150
<seed> --keep` (the SD-arc precedent; workdir printed).

## FINDING A — the mixed-key kill is NOT arrival-order-universal
(irdp6001x25, irdp6003x23, irdp9001x129; finals-only: the model
keeps an orphan w the oracle deletes)

All three shapes have the belief arriving AFTER all existing stated
siblings (x129 minimal: ST×2 via two T0s, then MIDT's logical → the
model's pending sibling; DEL then kills). The model applies the r1
event on the first stated kill (key-dies-whole, sibling orphaned
x1-undeletable, belief unstaged) ⇒ finals keep the orphan. The
ORACLE fires the SAME deleter sequence (×3 — so a third w handle
existed oracle-side too) but ALL THREE deletes land — no orphan, no
survivor. r1/8757 (belief arrived BEFORE the last stated) pinned
the opposite. TWO live sub-hypotheses, cells + SdDump required:
- P-POSITION: belief-before-a-later-stated ⇒ R-FIRST (key-dies-
  whole/orphan); belief-after-all-stateds ⇒ handle-by-handle with
  unstage-at-last (R-LAST-like).
- P-SLOT: pending_vals forms only when the key holds EXACTLY ONE
  stated at logical-insert time (l3/r1 shapes); with ≥2 stateds the
  belief takes a different form (WM-visible? dep-only?) and the r1
  event never arms. (l3's dump pinned pending on 1 stated; no dump
  exists for logical-onto-2-stateds.)
The A cell round: logical-insert onto {1,2} stateds × {stated
appended after, not} × kill, WITH SdDump arms (the fhs/pending
marks discriminate what firings cannot).

## FINDING B — the lazy-break slot beats same-salience queued acts
(irdp6003x41, irdp9001x46; firings: the oracle re-keys and re-fires
where the model dep-folds)

Self-join justifier with TWO T0s fires twice at ONE salience. The
oracle's first (t,t) firing schedules the lazy break; the break
LANDS BEFORE the second same-salience R4 act pops — so the second
insertLogical starts a FRESH key (rebirth) and the observer/deleter
re-fires on the new handle. The model's pseudo-act used FIFO seq ⇒
popped AFTER the earlier-queued same-salience act ⇒ dep-fold, no
rebirth, missing firing. This is the ⚖-flagged underdetermined
LAZY-SLOT pick failing in exactly the tie the cells could not
reach: the slot is AT the justifier's item — before any LATER pop
at the justifier's salience, not just before lower saliences.
SECOND nuance in the same shapes: the oracle's second firing is the
(t2,t2) twin, not the earlier-created (t1,t2) — consistent with
Drools update-propagation CANCELLING+RE-QUEUEING still-valid tuples
that contain the updated fact (the model only cancels alpha-FAILING
acts, an imported D-076-family commitment that is now shown
INCOMPLETE: updates also reorder surviving tuples). The B cell
round: (i) the slot straddle (a second same-salience act queued
before the lazy break schedules — does the break land first?);
(ii) update-requeue order (a still-valid tuple containing the
updated fact — does its act move to the queue tail?).

## Corners (counted, banked by pointer, NOT failures)

- break-empties-a-PENDING-belief's-deps: 29-39 per seed — VERY
  common, as predicted; the same-batch self-break law composed with
  the L3 pending sibling. First witness per seed printed in the log
  (e.g. irdp7001x0).
- belief-delete-with-stated-siblings: 1-3 per seed (predicted rare).
- stated-delete-on-JUSTIFIED-born-mixed-key: ZERO — predicted >0,
  measured 0: SHADOWED by the belief-delete corner (DEL's FIFO act
  on the WM belief pops before its act on the later stated sibling,
  so the belief assert always fires first in this grammar). The
  corner needs a deliberate cell, not more fuzz.

## Census (the port baseline)

engine-vs-oracle divergent: 12+14+21+25+14 = **86/750 (11.5%)** —
the low edge of the predicted 10-40% bracket. This is the I-RD port
baseline; re-measure on the same seeds after the port.

## Scorecard vs predictions

0-div claim: FAILED (5 REAL) — but both failures are the
pre-registered at-risk axes #1 (lazy slot) and, for Finding A, the
composition space the cells under-sampled (the r1 cell fixed the
belief's arrival position; the population varied it). Corner
reachability: 2 of 3 hit as predicted; the third shadowed
(mechanism identified). Census bracket: hit. Flake prediction: hit
(0). The population did exactly its job: two fine-structure gaps
found, witnessed, and named BEFORE the port.

## Next (the pre-registered protocol)

1. The A cell round (position × stated-count, SdDump arms) — split
   P-POSITION from P-SLOT, re-pin the mixed-key kill law's scope.
2. The B cell round (slot straddle + update-requeue) — re-pin the
   lazy slot and widen the imported update commitment.
3. Model re-pin from the cells (never from fuzz cases directly),
   validator 22/22 + new cells green, THEN population re-run on the
   same five seeds → 0 REAL required before the port slab opens.
4. The pending-dep-break corner cell (the highest-volume corner).
