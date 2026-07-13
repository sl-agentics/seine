# I-RD model-extension results (predictions: ird-model-predictions.md)

model_ird.py — the executable spec of all three I-RD mechanisms —
validates **22/22 against the banked truths ON THE FIRST COMPLETE
RUN**: zero semantic edits, zero transcription fixes, no
assert-unreachable corner fired. The pre-registered claim holds:
the D-203/D-204/D-205 laws, encoded exactly as stated (plus the two
flagged underdetermined picks), fully determine every banked cell's
firing sequence (with per-pattern post-RHS match values) and finals.

Gate: `python3 probes_pending/tms_envelope/model_ird.py` → 22/22.

## The mutation matrix (each commitment is load-bearing)

Each law commitment was mutated one at a time; the validator must
fail EXACTLY the cells that pinned that commitment:

| mutation | fails |
|----------|-------|
| dynamic law off (unstage kill cancels acts) | b1, b2, r1 |
| self-join exception off (all breaks immediate) | m3, m6, m7 |
| ALL self-breaks lazy (no in-flush row) | m1, m2, m5 |
| orphan undeletability off | r1, x1 |
| R-LAST instead of R-FIRST (unstage waits for last stated, no orphaning) | r1 |
| key survives the last-dep break (no L6 orphaning; label→STATED) | l6, x1 |
| LIFO tie-break within equal salience | b2 |
| restored | (none) |

Notes: every row fails at least one cell (no dead commitment); each
failure set is precisely the pinning cells (r1 alone separates
R-FIRST/R-LAST — the discriminator doing its job; b2 alone carries
the FIFO evidence, as flagged in the predictions; x1 rides both the
orphan and key-death paths). The truths CONSTRAIN the spec — 22/22
is not a vacuous pass.

## Standing notes carried from the predictions

- The lazy-break landing slot (at-justifier-salience) remains
  underdetermined by the cells (one intervening act in each of
  m3/m6/m7); the pick is doctrine-aligned (D-076/D-178 lazy row).
  A two-observer straddle cell would pin it — port-time if needed.
- Scope: the model covers the CELL vocabulary; the eight witnesses
  are out of model scope (dump-read coverage, D-204); the port
  graduates them against the real engine.
- The engine-side r1 anomaly (RD×2) is NOT modeled — the model is
  the ORACLE spec; the engine recon happens at port time against
  this spec.

## What the port inherits

model_ird.py is the port's executable target: (1) the act-cancel
exemption keyed on unstage-born origin; (2) the key-death-whole
events — last-dep break AND stated-delete-on-mixed-key — with
sibling orphaning, orphan undeletability, and fresh re-key; (3) the
break-landing rule — immediate everywhere except self-break with
tuple-binding ≥2×, which schedules at the justifier's salience.
Boundary cells the port must keep green: r2, m0, m3, m4, m6, m7
(converged) + the 39 sd cells + the full receipt set.
