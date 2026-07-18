# PINS — the expiration unblock-landing probe round (D-321; Bryan:
# "do the ND/NE landing probe round"; the D-317 named open item)

Predictions registered 2026-07-18 BEFORE any cell ran.

## THE REVISED HYPOTHESIS (from decoding all four witnesses first)

The D-317 read ("the oracle lands the not-D unblock at a different
drain point than the not-E unblock, arrival beats salience") does
NOT survive contact with the witness timelines:

- cf317902x205: epoch 3 = advance(600) [both E2 blockers expire
  mid-advance] + DELETE P1. Oracle fires NE(P2) only; engine fires
  NE(P2) AND NE(P1) — the P1 activation was born mid-advance and
  fired despite the same-epoch delete.
- cf317901x11: E1 expires mid-advance in epoch 1 while the same
  epoch DELETES P1 and INSERTS a fresh E1 (re-blocking the not).
  The engine's phantom NE(P1) act survives the re-block and fires
  an EPOCH LATER; the oracle never materializes it. The ND/NE
  SALIENCE ORDER MATCHES on both sides (NE@0 then ND@-6) — no
  ordering split at all; cf11x24's co-landing salience pin stands.

**HYPOTHESIS: the oracle lands expiration-driven not-unblock
activations at the FIRE BOUNDARY, against the epoch's FINAL state
(post-deletes, post-re-blocking-inserts); the engine's unblock
lands EAGERLY mid-advance (the D-112 eager-expiration lane),
manufacturing acts on lefts the epoch then kills — which fire as
dead-handle/phantom acts (D-211-adjacent) or ride re-blocks into
later epochs. The "ND vs NE" surface fork is a SYMPTOM: the
TMS-cascade side (not-D) is already quiescence-timed = boundary-
correct engine-side; only the direct not-E lane is eager.**

## THE GRID (all cells: E expires 100ms; P(v=1) [+P(v=2)] lefts;
## NE = `not E1() P()`; J/D for TMS cells; 3× oracle stability)

- **n1_ctl**: epoch 1 = advance(200) only (E1 ts=0 expires
  mid-advance; nothing else). PREDICT MATCH (high): the unblock
  lands, NE fires per left; no cancellation window exists.
- **n2_delP**: epoch 1 = advance(200) + delete P1 (the x205
  distillate). PREDICT DIVERGE (high): oracle NO NE firing
  (boundary state has no P); engine one phantom NE(P1).
- **n2b_oporder**: epoch 1 = delete P1 THEN advance(200) (op
  order flipped). PREDICT MATCH (med): the delete lands before
  the engine's eager expiry too — both sides fire nothing.
- **n3_reblock**: epoch 1 = advance(200) + insert fresh E1
  (re-blocking; P1 stays alive). PREDICT DIVERGE (med-high):
  oracle sees boundary state blocked → no NE fire; the engine's
  mid-advance phantom fires (or rides to a later epoch).
- **n4_del_later**: epoch 1 = advance(200); epoch 2 = delete P1.
  PREDICT MATCH (high): NE(P1) fires at epoch 1 on both sides —
  the control for n2 (the delete must be SAME-epoch to cancel).
- **n5_tms_delP**: J `E1($t:tag) => insertLogical(D($t))`, ND
  `not D() P()`; epoch 1 = advance(200) + delete P1. PREDICT
  MATCH (med): the TMS teardown routes through the quiescence
  drain = boundary-timed already; both sides fire nothing.
- **n6_order_a / n6_order_b**: NE and ND both unblock in the
  same epoch (E1 expires → D dies with it), P alive, saliences
  ND=5/NE=0 and ND=0/NE=5. PREDICT MATCH both (med-high):
  co-landing unblocks order by salience (the cf11x24 pin).
- **n7_freshP**: epoch 1 = advance(200) + insert P2 (P1 stays).
  PREDICT MATCH (med): boundary state has both Ps; both sides
  fire NE(P1) and NE(P2) (order may differ — record it).
- **n8_ride**: epoch 1 = advance(200) + delete P1 + insert
  fresh E1' (re-block); epoch 2 = advance(300) (E1' expires,
  nothing else). The x11 distillate. PREDICT DIVERGE (med):
  oracle fires NE(P2-era lefts only... with only P2 alive at
  epoch 2's boundary — here only fresh state); engine's
  epoch-1 phantom NE(P1) RIDES the re-block and fires in
  epoch 2 alongside.

DECISION TABLE: n2/n3/n8 diverge with n1/n2b/n4/n5/n6/n7
matching → the law is "boundary landing against final epoch
state; TMS lane already correct" — a narrow engine fix in the
D-112 eager-advance lane (GATED, Bryan decides; the D-134 §3B
deferral release and D-211 dead-handle pins are adjacent
certified surfaces to protect). Anything else → iterate before
naming.

## ROUND 1 MEASUREMENTS (2026-07-18, all 3× oracle-stable;
## n5/n6 "diverges" in the runner were the renderer's field-
## order artifact — harness diff says PASS)

THE BOUNDARY-LANDING HYPOTHESIS FALLS TOO — in the narrowing
direction: n1_ctl, n2_delP, n2b_oporder, n3_reblock,
n4_del_later, n5_tms_delP, n6_order_a/b, n7_freshP ALL MATCH.
The engine already cancels same-epoch-deleted acts (n2), sees
same-epoch re-blocks (n3), lands the TMS lane at the boundary
(n5), and orders co-landing ND/NE by salience both ways (n6 —
the cf11x24 pin reconfirmed). ONLY **n8_ride DIVERGES** —
oracle NE(2), engine NE(2)+NE(1): the phantom needs the FULL
CYCLE — the act born at the mid-advance unblock is HELD when a
same-epoch insert re-blocks the not, SURVIVES the left's
(P1's) same-epoch deletion inside that hold, and is RELEASED
an epoch later when the re-blocker expires, firing with the
dead left. The oracle's later release covers only LIVE lefts.

## ROUND 2 — which ingredient makes the ride
## Predictions registered 2026-07-18 BEFORE any cell ran.

- **n8b_noDel**: n8 without the P1 delete (P1 alive
  throughout). PREDICT MATCH (med): the ride itself is legal —
  epoch-2 release fires both Ps on both sides; the DELETE
  inside the hold is the divergent ingredient.
- **n8c_delLater**: re-block WITHOUT delete in epoch 1; epoch 2
  = advance (E1' expires) + delete P1. PREDICT DIVERGE
  (med-low): the engine's held P1 act releases mid-advance
  before the epoch-2 delete lands; the oracle's boundary
  release sees P1 dead. (A MATCH here narrows the defect to
  deletes INSIDE the hold epoch only.)
- **n8e_preDel**: epoch 1 = delete P1 BEFORE the advance, plus
  the re-block insert; epoch 2 = advance. PREDICT MATCH
  (med-high): P1 dies before the mid-advance unblock — no act
  is ever born, nothing rides.

## ROUND 2 MEASUREMENTS (2026-07-18, 3× stable)

- n8b_noDel: MATCH (hit) — the ride is legal; both sides fire
  NE(2),NE(1) at the epoch-2 release. THE DELETE is the
  divergent ingredient.
- n8c_delLater: DIVERGE (hit) — delete in the release epoch
  still leaves the engine's phantom.
- n8e_preDel: DIVERGE (**MISS — the informative one**): the
  phantom fires even when P1 died BEFORE the mid-advance
  unblock. So the stale entry is NOT an act born at the
  unblock — it is P1's BLOCKED-LEFT from epoch 0, retained in
  the not machinery ACROSS its owner's deletion whenever a
  re-blocker spans the delete; the later unblock-release
  emits it as a dead-handle firing. (n2/n2b/n3 match because
  a same-epoch release without the re-block hold prunes
  correctly — a different code path.)

**THE LAW (revised): the oracle's expiration-driven not-unblock
release fires LIVE lefts only; the engine's release emits
stale (deleted) lefts held in the not's blocked set when a
re-block cycle spans the deletion.**

## ROUND 3 — scope: epoch-independence + the TMS lane
## Predictions registered 2026-07-18 BEFORE any cell ran.

- **n8h_delMid**: re-block in epoch 1 (no delete); delete P1
  alone in epoch 2 (no advance); release in epoch 3. PREDICT
  DIVERGE (med-high): the stale left persists regardless of
  WHICH epoch the delete lands in — only delete-before-release
  under a spanning re-block matters.
- **n9_tms_ride**: the same ride through the TMS lane — J:
  E1($t)→insertLogical(D($t)); ND: not D() P(); epoch 1 =
  advance (E1 dies → D torn down → momentary unblock) +
  re-block via fresh E1(205) (J re-derives D at the boundary)
  + delete P1 + insert P2; epoch 2 = advance (E1(205) dies →
  D dies → release). PREDICT DIVERGE (low-med): the stale-left
  retention lives in the generic not machinery, so the not-D
  release shows it too. A MATCH narrows the defect to the
  direct not-E expiration release path only.

## ROUND 3 MEASUREMENTS (2026-07-18, 3× stable, harness-diff
## authoritative on all 15 cells)

- n8h_delMid: DIVERGE (hit) — the delete's epoch is free.
- n9_tms_ride: **MATCH** (the low-med DIVERGE prediction
  missed, narrowing) — the TMS-mediated not-D release fires
  live lefts only; the defect is CONFINED to the direct
  not-EVENT release path.

# ═══════════════════════════════════════════════════════════
# THE LAW — FINAL (D-321, 15 cells, 3 rounds, all 3× stable)
# ═══════════════════════════════════════════════════════════

**The oracle's expiration-driven not-unblock release fires
LIVE lefts only. The engine's plain-not-over-EVENT machinery
retains a deleted left's blocked entry whenever a re-block
cycle spans the deletion (n8b: the ride itself is legal; n2:
same-epoch release without a re-block hold prunes correctly),
and a LATER expiration release fires the dead left as a
phantom (delete position free: before the unblock advance
[n8e], same epoch [n8], a bare middle epoch [n8h], or the
release epoch [n8c]).** Everything the D-317 read guessed is
refuted in the narrowing direction: there is NO ND/NE landing
split (n6 both orders = salience, the cf11x24 pin
reconfirmed), no boundary-vs-eager landing difference in the
simple shapes (n1-n5, n7), and the TMS/J lane is CLEAN (n5,
n9) — the four cf blobs' ND-vs-NE surface forks are all the
NE-side phantom displacing the salience order.

MATCH 11/15 → graduate (pr_ndne_*); DIVERGE 4/15 → xfail
canonical witnesses (xf_ndne_n8_ride THE minimal + n8c/n8e/
n8h variants); the 4 cf blobs stay banked as class members
(cf317902x205 double-checked: its "count 4v3" fork is the
same phantom). THE PORT IS GATED (Bryan): scope = the not-
over-event release path's liveness filter; adjacent certified
surfaces to protect = D-140/D-151/D-158 not-unblock order
shadows, D-134 §3B deferral release, D-102 held-staged
drains, D-211 dead-handle pin (whose lane is NON-expiration
staged deletes — distinct). The D-317 fuzz_cep exists-only
J-fence is LIFTED this slab (it guarded the refuted variable
and cost acc-justifier × not coverage); the true class is
named in the generator comment.
