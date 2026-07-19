# The constrained-not release-order round (D-331) — fz_7_2364

## The hand-decode (fz_min_7_2364 — the banked minimized twin)

R1 `T0() T1() not T0(f2==true)` modifies one T0 to f2=true
(blocking its own not); R0 deletes it; the release re-arms;
repeat — a modify-delete relay consuming three T0s. Same firing
count, same final state BOTH sides; the fork is pure CONSUMPTION
ORDER: engine idx3,idx0,idx1 vs oracle idx3,idx1,idx0 — the
oracle is STRICT REVERSE-INSERTION (LIFO); the engine's first
pick agrees (initial activation batch is LIFO) but its
RELEASED re-activations queue in insertion order. The full
fz_7_2364 forks at the identical juncture (idx0-vs-idx1 after
round 1) — one law covers both.

MECHANISM SUSPECT: the D-158 PnShadow (which ranks release
orders for the gated pick) is built only for BARE nots (the
D-199 note: "not even constructed for the shape — its not
carries cmps"); both cells' nots are CONSTRAINED (f2==true) →
no shadow → the release falls to an unranked default. The
temporal deferral lane already implements reverse-release
("each released child PREPENDS at the not node (addInsert), so
the agenda is the REVERSE of the push order" — the D-134 site).

## THE LAW CANDIDATE

**Released-not activations re-enter the agenda LIFO (Drools'
addInsert prepend): a release re-activates its blocked lefts
such that the pick consumes them newest-inserted-first.**

## Round 1 predictions (REGISTERED BEFORE CELLS RUN)

- **r1_four_candidates** (the min shape + a 4th false T0 at the
  END): PREDICT oracle consumes STRICT reverse insertion —
  T0#4, T0#3(-1e9+7)... wait, by tuple: the LAST-inserted T0
  first each round, i.e. f0 order 9, -1e9+7, -5, 4 (high).
- **r2_initial_block** (one T0 f2=true present INITIALLY; R0
  deletes it in round 1, releasing three initially-blocked
  lefts at once): PREDICT the released batch consumes
  reverse-insertion too (med-high — same release lane whether
  the block formed initially or mid-relay).
- **r3_bare_not_control** (the relay via a BARE not on a
  blocker TYPE — R1 `T0() T1() not B()` + modify inserts... a
  bare-not relay needs a stated B; instead: verify any existing
  bare-not release cell logic stands — SKIP as a cell; the pn
  lane is certified surface, untouched by the port).

## Round 1 MEASUREMENTS + THE PORT

- r1: oracle [9, -1e9+7, -5, 4] — strict reverse-insertion,
  PREDICTED EXACTLY. Engine was [9, 4, -1e9+7, -5].
- r2 (initial block): oracle [-1e9+7, -5, 4] — reverse-
  insertion, PREDICTED. Engine was [-1e9+7, 4, -5]. The law
  holds across block provenance (initial vs mid-relay).

THE LAW: **released-not activations re-enter the agenda LIFO
(Drools' addInsert prepend) — a release consumes its unblocked
lefts newest-inserted-first.**

THE PORT (phreak, one loop + one retirement): the right-del
release now REV-ITERATES the prepend-built blocked list
(oldest-first emission; the staged push_front flip lands
newest-first consumption). The D-201 mutfirst pre-reversal
(blocked_reverse_of + its tms_mf_teardown_reverse call) is
RETIRED — rev-iteration of the unreversed list is IDENTICAL
emission (rev∘id ≡ id∘rev), so the x119/x30 t0-order pins hold
by construction. Post-port: r1/r2 + fz_min_7_2364 + fz_7_2364
ALL PASS. Blast radius = every plain-not release — byte gate
vs 5b0083c + full battery decide.

OPEN (noted): right-UPDATE-driven unblocks (a mask-changed
blocker releasing lefts) ride a different arm — same law
presumably; no witness forces it yet.

## Round 2: THE NAIVE PORT FALLS — and the true mechanism is UPSTREAM

The flip was MEASURED AND REVERTED (engine back to 5b0083c
byte-exact): it fixed the 2364 pair + fz_7_9360 + nb3 (both
would GRADUATE under a correct port — noted) but broke ELEVEN
certified cells (nb1/nb2/nb6, pr_ne_n4, regressions
fz_123_3370/fz_27182_1227/fz_42_5213/fz_42_7768/fz_7_9864/
fz_min_7768/fz_min_999_8145). The byte gate caught it; every
mover was oracle-diffed.

THE INSTRUMENTED DECODE (QPUSH/QPICK + REL + per-site BLK
traces, all stripped after):
- nb1's pinned release order [3,2,1] is ALSO LIFO — both shapes
  want the same consumption law; the engine gets nb1 right and
  the relay wrong through ONE code path.
- Same block arm (site0, the right-ins walk) builds OPPOSITE
  list orientations because the walk follows the not node's
  LEFT-MEMORY order, which is UPSTREAM-DEPENDENT:
  - nb1 (LIA -> not directly): memory [2,1,0] (the reversed
    LIA batch walk) -> blocked [0,1,2] -> release+staging-flip
    -> consumption [3,2,1] ✓.
  - the relay (L×M JOIN -> not): memory [0,1,3] (the join's
    FORWARD child emission) -> blocked [1,0] -> consumption
    f0-first ✗ (oracle wants f1 = newest-first).
- r3 (fresh release, LIA-direct): engine already CORRECT.
- r4 (same-type dual-role one-shot, join-shaped): engine
  already CORRECT (!) — its memory build happened to orient
  right; the relay's multi-round re-add path differs.

CONCLUSION: the divergent variable is the JOIN-CHILD EMISSION
ORDER into downstream left memories (forward where Drools'
equivalent walk is newest-first) — HEAVILY certified adjacent
surface (the jr pins, D-125 flush models, D-027 phase classes).
Composing orders: memory build × block walk × prepend list ×
release iteration × staging flip × queue pick. **This is a
D-083 STOP-AND-MODEL composition: the next session should build
model_check_notrel.py (the not-node block/release/memory machine
vs oracle timelines) with this round's traces as seed data —
NOT hand-tune.** The three would-graduate witnesses (fz_7_2364,
fz_min_7_2364, fz_7_9360, nb3 — four, counting the pair as two)
stay banked until the modeled port.
