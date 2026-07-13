# I-RD A/B cell-round results (predictions: ird-ab-predictions.md)

All five cells oracle 3× identity-stable; SdDump 3× on d1/d2/s2 + an
r1 baseline dump (all banked in graft_targets/ird/); truths banked
(truths/ird_ab_oracle_r1.ndj). Model re-pinned FROM the cells;
validator **27/27**; the mutation matrix extended (below);
population re-run appended at the bottom.

## ROUND A — both pre-registered hypotheses were WRONG; the dump
## found the real law

d1 came out P-POSITION's column exactly (RD×3, ROBS=1, finals 0 —
so P-SLOT died: pending forms on 2-stated keys too). d2 came out
matching NEITHER column (RD×4 ✓ but ROBS=2/finals=1 — ONE orphan,
not two) — the pre-registered stop-and-dump row fired, and the dump
showed something neither hypothesis named:

⚖ **THE ACTIVATION-BACKFILL LAW (the real Finding-A mechanism)**:
TMS EqualityKeys form at TMS ACTIVATION — the session's first
insertLogical. Stated facts already in WM at that moment get
PER-HANDLE keys (d1/d2 dumps: `STATED fhs[@2+]` alone AND `STATED
fhs[@4!@3+]` — two keys, one value), and only the LAST backfilled
key per value is VALUE-MAPPED. Post-activation inserts (stated or
logical) join the MAPPED key. The kill/unstage/orphan event is the
ORIGINAL r1 law, unchanged, operating PER-KEY:
- d1: @2's kill = plain singleton delete (its key was never mixed);
  @3's kill = the r1 event on the mapped key (Delete@3+Insert@4,
  zero siblings) — the "R-LAST-like" appearance in the population
  was an illusion of the two-key split.
- d2: s3 (post-activation) joined the MAPPED key ⇒ the r1 event at
  @3's kill orphans ONLY s3. One orphan. Exactly ROBS=2/finals=1.
- r1's dump (baseline): RJ fired BEFORE RS2 ⇒ TMS active when s2
  arrived ⇒ ONE key ⇒ the original r1 read stands verbatim.
- x129 and both other D-207 A-witnesses: both stateds pre-activation
  ⇒ split keys ⇒ singleton kill + 0-sibling r1 event ⇒ all deletes
  land. Fully explained; no position-scoped kill law exists.
Model: tms_active flag + backfill in tms_activate() + value_map;
the D-206 "stated inserts append to the value's key" rule is now
scoped to POST-ACTIVATION inserts.

## ROUND B — slot pinned, requeue refuted, the D-205 exception
## re-phrased

- **s1 (slot)**: RD NEVER fires — the lazy-break pseudo-item beats
  an earlier-queued act AT THE SAME salience. Pinned: the break
  lands at the justifier's item, before any other same-salience
  pop. (Engine converged.)
- **s3 (requeue)**: RO fires t1-then-t2 — original FIFO — an
  alpha-KEEPING update leaves surviving acts IN PLACE. The requeue
  hypothesis is REFUTED; the imported D-076-family commitment stays
  cancel-alpha-failers-only. (Engine converged.)
- **s2 (the twin)**: [RJ, ROBS, RJ, ROBS] — with requeue refuted,
  the second (mixed-tuple) firing's self-break MUST have landed
  lazily ⇒ ⚖ **the D-205 lazy exception is RULE-SHAPE-keyed**: the
  justifier's LHS is a self-join (≥2 patterns on the broken fact's
  type), NOT tuple-bindcount (m3/m6/m7's single-fact twins could
  not distinguish these; s2 does). (Engine converged.) The
  reachable-grammar altitude note: a T0+T1-pattern justifier
  breaking its T0 premise is outside the vocabulary; the pin holds
  at self-join-on-the-type.

Engine picture: d1/d2 DIVERGENT (open_divergence — d1 reproduces
the D-205 RD×2 anomaly family; d2: engine RD×3/finals 0, no orphan)
— port targets. s1/s2/s3 CONVERGED — boundary pins the port must
not move.

## Model re-pin + mutation matrix (post-edit validator 27/27)

| new mutation | fails |
|--------------|-------|
| backfill off (all stateds join the mapped key) | d1, d2 |
| lazy by tuple-bindcount (old D-205 phrasing) | s2 |
| lazy-slot plain FIFO | s1, s2 |
| update re-queues surviving acts | s3 |
| restored | (none) |

All four new commitments load-bearing, each pinned by exactly its
cells; the original seven mutations remain covered by the D-206
matrix (spot-checked dynamic-off → {b1,b2,r1}).

## Population re-run (the 0-div gate, same five seeds)

**GREEN — 0 REAL after 3× on ALL FIVE SEEDS** (0 flaky). The five
D-207 REAL cases moved into the clean column (6001 118→119 clean,
6003 112→114, 9001 111→113; 7001/7002 unchanged at 118/108).
Corner counts byte-identical to D-207 (same seeds, same draws —
the corner space untouched by the re-pin, as expected). Census
unchanged: 12+14+21+25+14 = **86/750 (11.5%)** — the I-RD port
baseline stands. THE LAWS GENERALIZE OVER THE POPULATION; the
pre-registered protocol is complete and THE PORT SLAB MAY OPEN
(Bryan gates), with model_ird.py (27 cells + 11 mutation rows) as
the executable target, d1/d2 + b1/b2 + l6/r1 + m1/m2/m5 as the
divergence targets, and s1/s2/s3 + r2/m0/m3/m4/m6/m7 as converged
boundaries.
