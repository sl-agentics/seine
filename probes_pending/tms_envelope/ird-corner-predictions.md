# I-RD corner-round predictions (logged BEFORE the runs)

The three assert-unreachable corners from D-206..208, taken to
cells. After the round: the model REPLACES the winning asserts with
encoded behavior, validator green (27 + the new cells), and the
population re-runs with the ex-corner cases NOW INSIDE the 0-div
comparison (~30-40/seed pending-corner + 1-3/seed belief-delete) —
the gate must hold at 0 REAL with the corner space included.

## Corner 3 first: the collapse argument (pre-registered)

stated-delete-on-a-JUSTIFIED-born-mixed-key is unreachable while
the belief lives, for ANY value-matched deleter in the vocabulary:
the belief materializes first (justified-born), so the deleter's
FIFO act lands on the belief before any stated sibling — exactly
the D-207 shadowing. Arm-gated one-shots hit the belief first too.
Paths that remove the belief first (break → L6; delete → corner 2)
change the key state before any stated kill. THEREFORE corner 3 has
no independent cell: it is pinned by c5's SECOND firing if the
belief-delete leaves the key alive, or fenced-by-unreachability if
the belief-delete kills the key whole. Its assert stays in the
model either way (reachable only if a future vocabulary adds a
belief-excluding deleter).

## The cells

- **c2_pending_selfbreak_kill**: [T0, V(v)]; RJ@20 = JL(v,
  brk=update) — the belief lands PENDING on the stated-born key
  (no WM op; d1/l3 dumps), the same-batch self-break empties its
  deps in-flush; RD@5 DEL(v); ROBS@0. What is the pending's fate?
- **c3_pending_foreignbreak_kill**: [T0, V(v), karm]; RJ@20 JL(v)
  no-break; RKILL@10 kills T0+karm (foreign, delete-sourced break);
  RD@5; ROBS@0. Source-invariance check for the same fate.
- **c4_pending_break_rejustify**: [T0, V(v), karm, iarm]; RJ@20
  JL(v); RKILL@10 (break); RINS@9 (iarm → fresh T0(false));
  RJ2@8 (re-justify v); ROBS@0. The l6-rebirth probe applied to a
  PENDING belief — discriminates the key's fate independent of
  deletability.
- **c5_beliefdel_with_sibling**: [T0]; RJ@20 JL(v) (belief b, WM,
  justified-born); RS1@10 ST(v) (sibling s1); RD@5; ROBS@0. RD's
  FIFO act kills b FIRST = corner 2 exactly.

SdDump 3× on ALL FOUR (the pending marks, key-death timing, and
survivor identity are dump-only observables — c5's competing
readings produce identical counts in two branches).

## Competing readings and predicted observables

PENDING-FATE readings (c2/c3/c4):
- **A (CLEAR — the lean)**: deps-empty on a NON-WM pending belief
  clears the bookkeeping quietly; the key survives as pure stated.
  Rationale: L6's key-death rides the WM belief's RETRACTION; a
  pending belief has nothing to retract. Medium confidence.
- **B (KEY-DEATH)**: the L6 event fires regardless — key dies,
  stated siblings ORPHAN (x1-undeletable).
- **C (ZOMBIE)**: the pending survives dep-less; a later stated
  kill still unstages it.

| cell | A (clear) | B (key-death) | C (zombie) |
|------|-----------|----------------|------------|
| c2 | [RJ, RD], ROBS(v)=0, finals no v | [RJ, RD(noop)], ROBS=1, finals v×1 | [RJ, RD, RD], ROBS=1 (unstage-born act survives), finals no v |
| c3 | [RJ, RKILL, RD], ROBS=0, no v | [RJ, RKILL, RD(noop)], ROBS=1, v×1 | [RJ, RKILL, RD, RD], ROBS=1, no v |
| c4 | ROBS(v)=1 (s1 only; the re-justify lands PENDING again on the surviving stated key), finals v×1 | ROBS(v)=2 (s1 orphan + the re-justify re-keys FRESH ⇒ WM belief), finals v×2 | (dump decides) |

c2 vs c3 split ⇒ the pending fate is SOURCE-dependent (new axis —
stop and split further). c4 must agree with c2/c3's column;
disagreement ⇒ the fate is event-specific (deps-empty vs stated
kill see different states) — dump arbitration.

BELIEF-DELETE readings (c5):
- **R-KEY-DEATH (the lean)**: deleting the WM belief kills the key
  WHOLE — s1 ORPHANS. [RJ, RS1, RD, RD(noop)], ROBS=1, finals v×1.
  Rationale: every pinned key-death event (L6, r1, fresh-restart)
  tears whole keys; consistent pattern. Medium.
- **R-BELIEF-ONLY**: the belief dies alone; the key survives
  holding s1. The SECOND RD firing then pins corner 3 live:
  sub-readings s1-ordinary ([RJ,RS1,RD,RD], ROBS=0, finals 0) vs
  s1-undeletable (ROBS=1, finals 1 — IDENTICAL COUNTS to
  R-KEY-DEATH; the dump's survivor identity and key-line timing
  split them).
- **R-NOOP**: the belief itself is undeletable (x1-analog) —
  b alive, then s1's kill = corner 3 on a LIVE mixed key; the dump
  decides everything.

## Post-round protocol (pre-registered)

Encode ONLY dump-confirmed winners; c2/c3/c4 must be
column-consistent before the pending assert is replaced; c5's
winner replaces the belief-delete assert; the corner-3 assert
STAYS (collapse argument above). Validator 27+4; new mutation rows
(pending-fate flipped → fails c2/c3/c4; belief-delete flipped →
fails c5). Population re-run ×5 seeds: corner counts should drop
to ~0 (the belief-delete corner shadows nothing anymore; the
pending corner simulates) and the 0-div gate must hold at 0 REAL
with those ~150 ex-corner cases now scored. Any REAL among
ex-corners ⇒ the encoding misses composition fine structure —
minimize, dump, iterate as cells; never patch from the fuzz case.
