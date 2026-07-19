# PINS — the collect-order family probe round (D-323; Bryan: "the
# collect-order family")

Predictions registered 2026-07-18 BEFORE any cell ran.

## THE FAMILY INVENTORY (all order-only Collection forks, same
## multiset, banked)

- xf_fz_298001_1216 — THE ROSETTA CELL (no epochs, 3 facts, one
  delete): initial [-4,-1e9,-4] matches both sides; R2 deletes the
  FIRST -4; refire: oracle [-1e9,-4] (removed the first -4 — java
  List.remove(Object) first-equal), engine [-4,-1e9] (removed the
  LAST equal). With distinct values the difference is invisible;
  with DUPLICATES the wrong instance leaves.
- xf_fz_4649_1144 (move-to-tail signature, duplicate 5.5s),
  fz_313902_1661 (duplicate 3.0s swap), fz_315002_1364 (distinct-
  value tail swap — possibly a different sub-mechanism),
  xf_fz_662607_47 (collectSet order), xf_co_refire_1902 +
  fz_316002_1902 residual (delete-refire rebuild, oracle [1,3,0]
  not explained by remove-first alone — composition TBD).

## HYPOTHESIS

**Drools maintains the collectList accumulator IN PLACE: append
on accumulate, java List.remove(Object) on reverse — i.e. the
FIRST value-equal instance leaves, regardless of WHICH fact
retracted. The engine removes a different instance (the 1216
evidence: the LAST equal).** Updates = reverse+accumulate =
remove-first-equal + append (a value's move-to-tail).

## THE GRID (type T(v,k) — k the instance tag; C: collectList(v)
## over T; targeted deletes via alpha on k; 3× oracle stability)

- **c1_build**: insert [7(k1), 8(k2), 9(k3)] distinct, no ops.
  PREDICT MATCH (high) — insertion order, certified surface.
- **c2_dup_del_first**: [7(k1), 8(k2), 7(k3)]; delete k1 (the
  first 7). Value-first AND identity removal agree → oracle
  [8,7]; the engine's last-equal removal → [7,8]. PREDICT
  DIVERGE (high — the 1216 signature verbatim).
- **c3_dup_del_second**: same facts; delete k3 (the SECOND 7).
  THE ORACLE SPLITTER: value-first-equal → [8,7] (the FIRST 7
  leaves though k3 died); fact-identity → [7,8]. PREDICT oracle
  [8,7] (med-high — java List.remove semantics in the standard
  collectList accumulator). Engine: measure (its last-equal
  would give [7,8] → DIVERGE... or MATCH if both remove the
  dead instance — then the law is identity and c2 was the
  engine removing by value).
- **c4_del_distinct**: [7,8,9]; delete k2. PREDICT MATCH (high)
  — all removal laws agree on distinct values.
- **c5_upd_move**: [7(k1), 8(k2), 9(k3)] + epoch update of k1's
  UNRELATED field. PREDICT (med): oracle = reverse+accumulate =
  7 moves to TAIL → [8,9,7]; engine behavior unknown — a MATCH
  means updates are already modeled, a stable-order result
  [7,8,9] means the engine skips the move.
- **c6_upd_dup**: [7(k1), 8(k2), 7(k3)] + update k3's unrelated
  field. Value-first reverse removes the FIRST 7 then appends →
  [8,7(k3-value),7?] — precisely: [7,8,7] → remove-first-7 →
  [8,7] → append 7 → [8,7,7]. PREDICT oracle [8,7,7] (med).
- **c7_reinsert**: [7(k1), 8(k2)]; delete k1; insert 7(k4).
  PREDICT MATCH [8,7] both (med-high) — appends compose.
- **c8_set_dups**: collectSet over [7,8,7,-3] with a delete of a
  duplicate. The D-108 comment claims both sides canonicalize
  SORTED, yet xf_fz_662607_47 diverges — PREDICT DIVERGE (low-
  med) and measure what the oracle's set order actually is.
- **c9_1902_recheck**: after the law lands, re-derive
  xf_co_refire_1902's oracle [1,3,0] from it (initial [0,3,1,0],
  delete the first-0 fact → remove-first-0 → [3,1,0] ≠ [1,3,0]
  — the 1902 shape has a JOIN partner T1(f2==false) whose
  update/churn may add reverse+accumulate cycles; if the law +
  composition explain it, done; else iterate).

## ROUND 1 MEASUREMENTS (all 8 MATCH — the frame rewrites)

- c1_build: [9,8,7] BOTH — the list is LIFO-arrival APPEND (the
  batch-staged drain order), not insertion order.
- c2/c3/c4 (deleter at salience 5 = BEFORE the accumulate ever
  fired): MATCH — pre-materialization staged folds remove the
  dead instance on BOTH sides.
- c5/c6/c7/c8: MATCH (updates pre-materialization, reinsert,
  set).
- RE-DERIVATION: under "oracle reverse = java List.remove(Object)
  = FIRST value-equal of the MATERIALIZED list", xf_co_refire_1902
  is EXPLAINED EXACTLY: [0(ab),1,3,0(a)] − remove-first-0 (the
  head 0(ab), NOT the dead T1(a,0)) → [1,3,0] ✓ oracle; the
  engine's identity removal of the dead tail → [0,1,3] ✓ engine.

**THE LAW (revised): divergence requires the reverse to hit a
MATERIALIZED list; there the oracle removes the FIRST value-equal
element regardless of which fact died; the engine removes the
dead fact's own instance. Identical without duplicates.**

## ROUND 2 — post-materialization ops (deleter salience -5, the
## accumulate fires first). Predictions registered BEFORE runs.

- **c2b_dup_del_first**: [7k1,8k2,7k3] (list [7k3,8,7k1]); delete
  k1 (tail instance) post-mat. PREDICT DIVERGE (high): oracle
  remove-first-7 → [8,7]; engine identity → [7,8].
- **c3b_dup_del_second**: delete k3 (head instance) post-mat.
  PREDICT MATCH (high): first-equal IS the dead instance — both
  [8,7].
- **c4b_del_distinct**: distinct, delete k2 post-mat. PREDICT
  MATCH (high).
- **c5b_upd_move**: [7,8,9] (list [9,8,7]); post-mat update of
  k2 (v=8, unrelated field). PREDICT (med): oracle reverse+
  accumulate = remove-first-8 + APPEND → [9,7,8]; engine
  unknown — a stable [9,8,7] = no-move, identity-in-place.
- **c6b_upd_dup**: [7k1,8,7k3] (list [7k3,8,7k1]); post-mat
  update of k1. PREDICT oracle [8,7,7] (remove-first-7 = k3's
  instance + append k1's 7) (med).
- **c10_triple**: [7k1,7k2,8,7k4] (list [7k4,7k2? — LIFO
  [7k4,8,7k2,7k1]]); delete k2 post-mat. PREDICT oracle removes
  head-first-equal 7k4 → [8,7,7]; engine identity →
  [7k4,8,7k1] = [7,8,7]. DIVERGE (med-high).
- **c8b_set_dup_del**: collectSet [7k1,8,7k3,-3]; delete k3
  post-mat. Counted-set semantics (ga15: a duplicate survives a
  sibling's delete) — PREDICT MATCH [something with 7 kept]
  (med); measure the canonical order.

## ROUND 2 MEASUREMENTS + THE PORT (2026-07-18)

- c2b/c10/c11 DIVERGED exactly as predicted (post-materialization
  delete of a non-first duplicate; triple; value-changing update);
  c3b/c4b/c5b/c6b/c8b MATCH (c5b's move-to-tail prediction missed
  in the narrowing direction: a value-PRESERVING update never
  touches the list — property reactivity on the accumulated
  binding; the reverse+append cycle needs the VALUE to change,
  c11).
- c13_rhs_pair MATCH — within-one-RHS insert pairs arrive
  identically (LIFO) on both sides.

**THE LAW (final): the collectList accumulator is maintained IN
PLACE — append on accumulate (batch arrivals drain LIFO), and on
reverse the FIRST VALUE-EQUAL element leaves (java
List.remove(Object)) regardless of which fact retracted. The
engine removed the retracted fact's own instance — identical
without duplicates, wrong instance with them. Value-preserving
updates are invisible; value-changing updates = reverse+append.
Pre-materialization ins+del pairs annihilate in staging and never
reach the reverse (c2/c3 matched pre-port).**

THE PORT: one comparator — Acc::reverse's CollectList arm removes
by VALUE-first (`position(|(_, x)| x == v)`) instead of by FactId;
vlist's FactId component is consumed nowhere downstream. Closes
FIVE witnesses byte-for-byte: xf_co_refire_1902 (re-derived by
hand from the law before the port: [0,3,1,0] − first-0 = [1,3,0]),
fz_316002_1902 (the whole compound blob — its D-320 agenda half +
this = fully explained), xf_fz_298001_1216 (the Rosetta cell),
xf_fz_4649_1144, fz_313902_1661.

RESIDUALS (2, reclassified as their own named sub-items, banked,
byte-UNCHANGED by this port): fz_315002_1364 — a distinct-value
arrival-order swap (5/−5) that c13's minimal within-RHS pair does
NOT reproduce (the ingredient is elsewhere: cross-rule arrival or
churn); xf_fz_662607_47 — collectSet first-instance order (c8/
c8b/c12 churn all match; the blob's ingredient unfound). Both
need their own delta-minimization hunts.

Byte gate 2372/5/0 vs 014b067 — the five witnesses are the ONLY
cells that moved in the corpus universe.

# ═══════════════════════════════════════════════════════════════
# THE WINDOWED-EVICTION PIN ROUND (D-324; Bryan: "do the pin
# round" — the D-323 fix's window composition is unpinned and
# fuzz-unpatrolled: fuzz_cep draws no collect functions)
# Predictions registered 2026-07-18 BEFORE any cell ran.
# ═══════════════════════════════════════════════════════════════

The question: window EVICTION retracts an event from the
accumulate — does the collectList lose the FIRST VALUE-EQUAL
element (the D-323 law, same reverse arm) or the evicted
INSTANCE's own entry? Distinguishable only when the list order
puts a duplicate ahead of the evicted instance (the LIFO batch
build makes that arrangeable: batch [7k1, 9k2, 7k3] builds
[7k3, 9, 7k1]; evicting k1 by value-first removes 7k3 → [9,7,+8],
by instance → [7,9,+8]).

- **w1_len_dup**: window:length(3), batch [E7k1, E9k2, E7k3],
  then E8k4 (evicts the oldest ADMISSION). PREDICT (med): the
  eviction routes through the SAME reverse arm → value-first →
  MATCH post-D-323 (both engines value-first). The cell also
  pins WHICH admission the length-ring calls oldest under a
  LIFO-drained batch — recorded either way.
- **w2_time_dup**: window:time(100ms), staggered ts [E7k1@0,
  E9k2@10, E7k3@20], expires huge; advance(105) evicts k1 ONLY.
  The list is [7k3, 9, 7k1]; value-first removes 7k3 → [9,7];
  instance removes k1's → [7k3, 9] = [7,9]. THE SPLITTER.
  PREDICT MATCH at [9,7] (med — same reverse arm both sides).
  A [7,9]-oracle = instance-based eviction → the D-323 law does
  NOT extend through windows and the port needs a window-side
  distinction.
- **w4_set_win**: windowed collectSet with duplicates + eviction
  (counted-set: the dup survives one eviction). PREDICT MATCH
  (med-high — ga15 semantics are window-agnostic).
- **w5_distinct_ctl**: window:time, distinct values, eviction.
  PREDICT MATCH (high) — all laws agree on distinct.

Plus the patrol wiring: fuzz_cep W rules gain a collectList draw
(the firing tuple then carries the Collection → order-diffable);
shakedown 3×300.

## D-324 MEASUREMENTS (2026-07-18, all cells 3×-run stable via diff)

ALL SIX PIN CELLS MATCH — the D-323 law extends through windows
with no engine change needed:
- w2_time_dup (THE SPLITTER): the time-eviction of k1 removed the
  head 7k3 (FIRST VALUE-EQUAL), not k1's own entry — [9,7] both
  sides. The list can keep a value slot whose window-resident
  owner is gone; both engines agree.
- w1_len_dup: length-ring eviction, same law — [9,7,8] both.
  Also pins LIFO batch build for windowed accs ([7,9,7]).
- w4_set_win: counted-set dup survives one eviction ✓ ga15
  through windows. w5_distinct_ctl ✓.
- w6_upd_tsval / w7_upd_tsonly (post-find probes): a ts-moving
  value-changing update through the window = remove-first +
  append ([9,7,8]); ts-move alone = invisible. Both MATCH.

PATROL: fuzz_cep W rules now draw collectList(tag) at 25% (tags
from a 3-symbol pool = duplicate pressure; collect draws skip the
DW justifier — Collection can't feed DW(v i64)). Shakedown 3×300
seeds 324901-903: 2 clean + ONE find cf324903x55 — worktree-
bisected PRE-EXISTING (byte-identical on the pre-D-323 engine),
an update-churn windowed-collect order fork whose minimal forms
(w6/w7) do NOT reproduce it → banked beside fz_315002_1364 and
xf_fz_662607_47 as the third minimal-cell-resistant member of
the arrival/update-order sub-family.

# ═══════════════════════════════════════════════════════════════
# THE ARRIVAL/UPDATE-ORDER HUNT (D-326; Bryan: "do the
# arrival/update-order hunt" — the 5-member sub-family)
# Predictions registered 2026-07-18 BEFORE any cell ran.
# ═══════════════════════════════════════════════════════════════

HAND-DECODE OF fz_315002_1364 (the cleanest member): R5 collects
f1 over T0(f0 < 11). The epoch: update(target0){f0:100,f1:-5} =
ALPHA EXIT (f1=6 leaves); update(target0){f0:1} = ALPHA RE-ENTRY
(f1=-5 arrives); then a FACT INSERT (f1=5). Oracle tail [-5, 5]
= per-op call order (each external op's acc effect lands at its
own position); engine tail [5, -5] = the fresh insert's arrival
BEAT the update's re-entry.

**HYPOTHESIS: the oracle applies epoch ops' accumulate effects in
PER-OP CALL ORDER; the engine routes fresh inserts through WM
staging but external updates through the acc_pending queue
(D-154/D-160), and the two queues' relative drain order does not
preserve call order — inserts land before update-driven effects.**

## THE GRID (T(v,g,k); C collects v over T(g < 10); k = tag)

- **a1_upd_then_ins**: initial [T(1,g1,k1)]; epoch: update k1
  {v:2} THEN insert T(3,g1,k2). Oracle = call order: remove-1 +
  append-2, then append-3 → [2,3]. Engine (insert-first drain) →
  [3,2]. PREDICT DIVERGE (med-high — the 1364 signature).
- **a2_ins_then_upd**: epoch: insert T(3) THEN update k1 {v:2}.
  Call order → [3,2]; insert-first drain → also [3,2]. PREDICT
  MATCH (med-high). The a1/a2 pair is the splitter: only the
  update-BEFORE-insert order can expose the queue split.
- **a3_reentry_then_ins**: the 1364 distillate — update k1
  {g:20} (exit), update k1 {g:1, v:2} (re-entry), insert
  T(3). Oracle [2,3]; engine [3,2]. PREDICT DIVERGE (med-high).
- **a4_two_epoch_facts**: epoch inserts T(3,k2) and T(4,k3) (no
  updates). PREDICT MATCH (med): same-queue ordering is already
  certified (initial LIFO, c1; epoch facts — record the order).
- **a5_two_upds**: epoch updates k1{v:2} then k2{v:5} (two
  acc_pending entries; initial [T(1,k1),T(4,k2)]). PREDICT MATCH
  (med-high): the D-154 FIFO pin covers same-queue order.
- **a6_del_then_ins**: epoch: DELETE k1 then insert T(3).
  Deletes ride WM staging (a third path). PREDICT MATCH
  (med-low): remove-first + append agree under either drain
  order here ([1]-1=[] then +3 = [3] both). Recorded for the
  op-coverage row.

If a1/a3 diverge and a2/a4/a5 match → the law is per-op call
order and the engine fix is the acc-effect ordering between the
two queues (likely: route update-driven acc effects through the
same boundary drain position as inserts, or interleave
acc_pending with the staged-insert drain by op sequence). The
five witnesses re-diff after any fix; the cep members (x55/x221/
x88) add the window/ts axis on top — verify before claiming
them.

## D-326 MEASUREMENTS + THE PORT (2026-07-18)

Round 1 (a1-a6): a1 MATCH (the queue-split hypothesis fell — a
plain value-update + insert already lands in call order); **a3
DIVERGE — the divergent ingredient is the ALPHA EXIT +
RE-ENTRY**, not updates generally; a2/a4/a5/a6 MATCH. a7's
oracle [5,2] then broke pure call-order and landed the real law:
**Drools folds same-fact staged ops by identity — an exit +
re-entry pair coalesces into ONE net in-place UPDATE whose acc
effect drains at the update position (before fresh inserts, LIFO
among updates)**; the a8 pair confirmed it holds for
value-PRESERVING re-entries too (a8: move-to-tail identical;
a8b: the move drains before the epoch insert — oracle [1,4,3]
vs engine [1,3,4]).

THE PORT: in on_update's inline (false,true) re-entry arm, when
the pattern is an ACC source and a staged del for the same fact
exists → fold: remove the del, stage an UPD (the drain's
existing update processing does reverse-old + append-new /
move-to-tail). Joins keep the certified del+ins ph=1 late-pass
(jr pins — join CHILDREN are new objects per c13; the FACT is
the same object). Closes a3/a8b + **fz_315002_1364** (the
hand-decoded member: exit + re-entry(-5) + insert(5) → the
oracle's [.., -5, 5]).

RESIDUALS (4): the cep members (cf324903x55/cf325902x221/x88)
ride the EVENT-typed entry drain (per-entry against epoch-final
fields, D-160) + window/ts churn — their forks are byte-
unchanged by this fix and the composition is guarded by the
D-154/D-160 pins (updel/multiupd/ap1, wa_* revival): they need
their own clock-plane probe round, not a blind fold extension.
xf_fz_662607_47 (collectSet first-instance order) likewise.
cf325901x52 is not a Collection fork at all (not-DW P-witness
order) — reclassified OUT of this family, unexplained.
