# I-RD corner-round results (predictions: ird-corner-predictions.md)

Four cells, oracle 3× identity-stable, SdDump 3× on all four
(banked in graft_targets/ird/), truths banked
(truths/ird_c_oracle_r1.ndj). **THE ENGINE CONVERGES ON ALL FOUR**
— the corner space adds boundary pins, not port targets.

## ⚖ PENDING-CLEAR (corner 1, pinned by c2/c3/c4)

When a NON-WM pending belief's dep set empties, the bookkeeping
clears quietly and **the key survives as PURE STATED** — no key
death, no orphaning, no unstage. Source-invariant (c2 same-batch
self-update ≡ c3 foreign premise-delete). The decisive dump joints
(c4): post-break `STATED fhs[@2+]` (the pending mark gone, the key
alive), and the re-justify landing as a NEW pending sibling on the
SAME key (`fhs[@7!@2+]`) — key-survives, not key-dissolves. ⚠ c2's
post-kill empty keys line initially read as dissolution-at-break;
c3/c4 corrected the attribution (the flush order hid the pure-
stated interval behind the kill — the predictions' column-A
phrasing stands, the dissolution misread does not).
Column A was the pre-registered lean; all three cells landed in it
consistently, as the protocol required before encoding.

## ⚖ BELIEF-DELETE KEY-DEATH (corner 2, pinned by c5)

Deleting the WM belief of a mixed justified-born key kills the key
WHOLE — the dump shows the keys line emptying at the belief's
delete with the stated sibling still listed; the sibling ORPHANS
(x1-undeletable, the RD no-op, the finals survivor, ROBS=1). The
zero-sibling case degenerates to a1's plain justified delete. This
was the pre-registered lean (every pinned key-death event tears
whole keys); the count-ambiguous alternative (belief-only +
undeletable-sibling) is excluded by the dump's key-line timing.

## Corner 3: FENCED-BY-UNREACHABILITY (the collapse argument held)

stated-delete on a LIVE justified-born mixed key cannot arise:
value-matched deleters always hit the WM belief first (FIFO by
materialization — the D-207 shadowing), and c5 shows the
belief-delete kills the key whole, so no stated kill ever lands on
a live mixed justified-born key. The model's assert STAYS as the
fence (it fires only if a future vocabulary adds a belief-excluding
deleter — at which point it becomes a cell, not a guess).

## Model + matrix + population

Both winners encoded (finalize_break's pending branch; rhs_delete's
belief branch); validator **31/31**; new mutation rows exact:
pending-flipped-to-key-death → {c2,c3,c4}; belief-delete-flipped-
to-belief-only → {c5}; restored clean.

Population re-run (same five seeds, corners now INSIDE the 0-div
comparison — ~30-40 ex-corner cases/seed):

**PERFECT SCORE — 150/150 model-clean on ALL FIVE SEEDS, 0 raw
mismatches (not even flake-filtered), corners: NONE.** Every
ex-corner case (~150 across the seeds) simulates and matches the
oracle exactly; the fenced corner-3 assert never fired in 750 more
cases (the unreachability argument holds empirically); census
byte-stable at 12+14+21+25+14 = **86/750 (11.5%)** — the port
baseline. THE MODEL IS TOTAL OVER THE POPULATION GRAMMAR:
model_ird.py (31 cells, 13 exact mutation rows) is a complete
executable oracle spec for the I-RD vocabulary. The port slab's
prerequisites are all green (Bryan gates).
