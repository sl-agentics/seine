# R1 interposer ladder — PREDICTIONS, logged pre-run

_2026-07-12, opened on Bryan's sequencing ("R1's interposer ladder
before I-RD") after the order-cluster 0-div work. Instrument + cells:
`interposer_ladder.py` (6 cells; equal-salience decl cells exist as
sd_a2/sd_a9). Every prediction below is model_sd's mechanical output
(`--predict`, captured before any oracle run) — the ladder tests the
D-187..D-195 laws' TRANSFER to the min812 (k0) and 9133 (k1 fan-out)
spines and the update-break row, with the D-177 salience-interposer
as the landing-time discriminator. Oracle 3× per cell; flake ⇒
quarantine per fz_42_84._

## Cells + model predictions (= the laws' joint implication)

| cell | shape | model prediction | law under test |
|---|---|---|---|
| ip_a1 | lazy k0 J@0; RO@10; RI@5 | [RJ, RO, RI] | lazy drop lands at J's POP ⇒ every strictly-higher rule (10 AND the interposer 5) glimpses the transient, in salience order |
| ip_a2 | lazy k0 J@0; RO@10; RI@-5 | [RJ, RO] | sub-salience never glimpses (the pop precedes -5's turn) |
| ip_a3 | EAGER k0 J@0; RO@10; RI@5 | [RJ] | eager drop lands at run end (loses-head) ⇒ NO ≤-salience glimpse at all — neither 10 nor 5 fires |
| ip_b1 | lazy k1 J@0, P×3; RO@10; RI@5 | [RJ(1), RO, RI] | clause B on the fan-out spine: the first insert self-cancels the remaining tuples; NO deleter ⇒ starvation (gen-1 only); both higher rules glimpse gen-1's LK once |
| ip_b2 | lazy k1 J@0, P×3; RO@10; RI@-5 | [RJ(1), RO] | as b1, sub-salience blind |
| ip_c1 | gt13 + RI obs_lk@6 | [RO1(1), RO1(2), RJ(1), RJ(2), RO2(2), RO2(1), RI] | the D-195 (b) row's BETWEEN cell: the zombie window is open to EVERY rule strictly above RJ@5 — RO2@7 first, then RI@6 fires once on LK2(2), then RJ's pop kills it (gt14 pinned the below-row; this pins between) |

## Falsifier bookkeeping (pre-committed)

- ip_a1 without RI (or RI after RO in the wrong order) ⇒ the pop
  window is not uniformly open to all strictly-higher salience — the
  queue-head discipline's glimpse clause needs a row split.
- ip_a3 with any glimpse ⇒ the eager run-end landing does not
  transfer to k0-with-interposer — the D-187 eager row is
  shape-dependent (big: re-opens min812's reconciliation).
- ip_b1 with RJ firing >1 ⇒ clause B does not govern the fan-out
  spine (the 9133 bucket needs its own mechanism); with RO/RI firing
  ×3 ⇒ per-generation windows exist for lazy k1 WITHOUT deleters —
  the starvation reading of mb1_st would need re-scoping.
- ip_c1 with RI silent ⇒ the zombie window is salience-top-only (not
  every-strictly-higher) — the D-195 (b) row splits by queue
  position, not salience threshold. RI firing BEFORE RO2's second
  firing ⇒ the window interleaves runs — queue-head violation.

## Notes

All six also re-verify finals (a-cells: LK gone at boundary; b-cells:
Ps remain, LK gone; c1: P-only). Any oracle flake ⇒ quarantine the
cell and say so. After the runs: verdicts + any residue to
interposer-results.md; cells graduate to the port battery
(open_divergence) only where the ENGINE diverges from the confirmed
oracle truth — engine census rides along in a later phase (the port
A/B baseline), not this rung.
