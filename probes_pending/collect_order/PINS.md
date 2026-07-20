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

## D-327 THE CEP EVENT-DRAIN CHURN ROUND (2026-07-18)

### The x184 hand-decode (cf326901x184, end-to-end)

Fork: firing[3], W3's windowed collectList — engine [z,z,y],
oracle [z,y,z]. Timeline (only 4 firings, only [3] differs):
epoch 0 builds [z,z] (E0@21 z, E0@18 z); epoch 1 = advance 101
(no evictions: both leave-times 118/121 > 101), update idx4
(ts 18→102, tag z→y), then epoch-fact insert E0@119 z.

ENGINE MECHANISM (read, engine.rs): event-typed acc-source
updates DEFER — on_update pushes AccEntry::Upd (acc_pending,
~9216) and skips staging; drain_acc_pending runs at fire_all
pre-fire (~7835) → winacc_step (true,true) stages add_upd THEN.
But the epoch's fresh INSERT stages at its own call and D-102
stream-flush materializes it immediately. So the staged batches
are split: [ins] flushes at the insert call → [z,z,z]; [upd]
flushes at the boundary → remove-first-z + append-y → [z,z,y].
The D-154 comment at 7830 names the buried assumption: "staging
here instead of at the update call is byte-identical for
single-update epochs" — false once a flush-triggering call
intervenes AND the accumulate is order-visible (collectList).

ORACLE MECHANISM (hypothesis to split): Drools stages the
modify at the CALL; the insert's per-call stream flush processes
everything staged in ONE batch, class-ordered (right-dels,
right-upds, right-inss — PhreakAccumulateNode doNode order, the
same phase order our eval_acc_node already has) → upd-append
lands before ins-append → [z,y,z]. "Epoch-final fields" (D-160)
and "fields at next flush" are INDISTINGUISHABLE in this
scenario grammar (actions precede facts in an epoch) — the
deferral's field semantics were never the divergence; only the
APPEND ORDER is.

### The structural reduction

Every collectList remove targets the FIRST VALUE-EQUAL element
(D-323/D-324) — removes commute with everything (equal victims:
same element either way; distinct victims: disjoint). The
entire fork surface is the ORDER OF APPENDS: upd-new-value
appends vs fresh-insert appends vs re-admission appends vs
move-to-tail appends. Explains why the five members are all
update-flavored and why explicit deletes never fork.

### Round 1 predictions (REGISTERED BEFORE CELLS RUN)

Candidate mechanisms: (A) stage-at-call, one batch at next
flush trigger, phase-ordered, FIFO among distinct-fact upds
(the a5 plain pin); (B) per-update-call flush. A and B agree on
every cell this grammar can express (updates precede inserts in
an epoch); both refute the engine's boundary deferral.

- **ed1_upd_then_ins** (x184 minimal: [z,z]; upd f1 z→y ts102;
  ins z@119): PREDICT oracle [z,y,z], engine [z,z,y] — DIVERGE
  (high). The class reproduced with one rule, three facts.
- **ed2_upd_only** (no epoch fact): PREDICT MATCH [z,y] (high)
  — the certified single-update epoch.
- **ed3_ins_only** (no update): PREDICT MATCH [z,z,z] (high).
- **ed4_two_upds_then_ins** (upd f1 z→y, upd f0 z→w, ins z):
  PREDICT oracle [y,w,z] (med-high: FIFO call order among
  distinct-fact upds, per a5), engine [z,y,w] — DIVERGE. If
  oracle is [w,y,z] the upd class is LIFO instead — record it.
- **ed5_samefact_twice_then_ins** (upd f1 z→y, upd f1 y→w,
  ins z): PREDICT oracle [z,w,z] (med — A and B agree; the
  second modify re-stages/re-reads, one net append), engine
  [z,z,w] — DIVERGE (the D-154 two-entry drain double-upds but
  one Phase C entry nets remove-z+append-w after the ins).

### Round 1 MEASUREMENTS (oracle 3x byte-stable on all divergers)

- ed1 DIVERGE: oracle [z,y,z] — PREDICTED EXACTLY. engine
  [z,z,y] as predicted.
- ed2 MATCH [z,y], ed3 MATCH [z,z,z] — as predicted.
- ed4 DIVERGE: oracle [y,w,z] — PREDICTED EXACTLY (FIFO call
  order among distinct-fact upds). Engine [z,w,y] (predicted
  [z,y,w] — the staged upd list is push_front/LIFO and Phase C
  iterates front-first, so the boundary batch reversed; the
  engine-internal detail was misread, the law was not).
- ed5 DIVERGE: oracle [z,w,z] — PREDICTED EXACTLY. engine
  [z,z,w] as predicted.
- ed6 (plain non-windowed acc over event source) DIVERGE:
  oracle [z,y,z], engine [z,z,y] — the D-160 plain-event
  deferral carries the same latent.

MECHANISM SPLIT: ed4 kills batch-at-next-flush (A): one batch
would process the push_front upd list LIFO → [w,y,z] ≠ measured
[y,w,z]. THE LAW IS PER-CALL FLUSH (B): **in a stream session
every external call flushes its own propagation batch; an
event-source update's accumulate effect (remove-first-value-
equal old + append new / revival append / move-to-tail) lands
AT ITS OWN CALL, in call order.** D-102 (per-insert flush) and
D-166 (per-update flush, already in the engine at the session
update entry — "each update action is its own propagation
batch") already pinned per-call flush for every OTHER surface;
the acc arms' D-154/D-160 deferral is the one hold-out. The
deferral's "epoch-final fields" are indistinguishable from
"fields at own call" (the store is written before propagation);
only the APPEND ORDER differs, visible only through collect*.

### The five members re-decoded under the law (element-exact)

- x184: [z,z]; upd(z→y)+ins(z) → oracle [z,y,z] / eng [z,z,y] ✓
- x221: [x,x,x]; upd(x→y)+ins(z) → oracle [x,x,y,z] / eng
  [x,x,z,y] ✓
- x88: [z,x,y]; upd(x→y)+ins(z)+ins(y) → oracle [z,y,y,z,y] /
  eng [z,y,z,y,y] ✓ (initial build = per-call arrival order,
  both sides agree)
- x239: [y,x]; upd(x→y)+ins(x) → oracle [y,y,x] / eng [y,x,y] ✓
- x55: E0@0 EVICTED by the advance, then upd(x→y,ts113) =
  detached mask-hit REVIVAL; +ins(x@103) → oracle [y,x] (the
  revival append lands at the upd's call, before the ins) /
  eng [x,y] ✓ — the law covers revival appends unchanged.

ALL FIVE are the one mechanism. No flip-flop-zone involvement
(no eviction interleaves BETWEEN the racing appends; x55's
eviction precedes both calls). D-083 not implicated.

### THE PORT (design)

on_update's two event-acc deferral arms stop deferring and step
immediately (transitions VERBATIM — winacc_step/plainacc_step
untouched): the windowed arm calls winacc_step at pass 0 (the
RHS-modify branch's exact call — the origin.is_none() split is
DELETED); the plain-event arm calls plainacc_step per pass. The
already-certified D-166 per-call stream flush then materializes
the effect at the call. AccEntry::Upd + the drain's Upd arm +
del_pos die (external deletes keep the Del deferral — deletes
are order-invisible in collect*: removes always target the
first value-equal element and commute). Expected byte movement:
the 5 members flip PASS; xf_cep_acc_updel_flush_{plain,win}
(regression-tier engine pins of the drain's approximation of
"the oracle's per-entry incremental flush" — per-call flush IS
that) may move toward the oracle → re-bank or graduate; any
other movement = investigate before accepting.

### Round 2: the revival composition (x55's wrinkle)

Post-port, x55 WORSENED shape-wise: engine [x] vs oracle [y,x] —
the revived y VANISHED (and a firing with it). ed7 (minimal:
E0(x@0); advance 101 = eviction; upd →y ts113 = detached
mask-hit revival; ins x@103) reproduces. WA-instrumented trace:
the advance's eviction del is STASHED by the trigger-scoped
delta flushes (certified D-125/D-322 stash mechanics) and only
processes at the BOUNDARY Phase B — where it reversed BOTH of
f1's contributions (the stale x AND the revival's y, folded at
the upd call): [x]→[x,y]→[x,y,x]→(boundary del reverses twice)
→[x]. Pre-port the revival ins also sat at the boundary BEHIND
the del in phase order, which masked the composition.

THE LAW (the D-326 identity-fold, reaching this arm now that
stepping is per-call): a staged del + a same-fact revival nets
to ONE UPD — Phase C reverses the STORED contribution (x) and
appends the new (y) at the revival's call. Under per-call flush
a staged del can coexist with a (false,true) step ONLY via the
non-flushing del sources (eviction/expiry stash) — a same-batch
alpha exit's del always flushes at its own call.

PREDICTIONS: winacc_step (false,true) admit arm folds
del+ins→UPD (remove_first_by_key + add_upd, the D-326 port's
exact shape) → ed7 PASS [y,x]; x55 PASS; ed1-ed6 unchanged
PASS; the wa_*/D-112/D-137 lanes hold (count/sum order-blind).

### Round 3: the m-matrix pushes back — MECHANISM C′

The naive per-call port broke 10 certified cells (m3/m8/m10-m15/
updupd_final/wl_transient — byte gate). m3/m8/m12 forced the
entry queue back verbatim: the pair's FIRST entry must evaluate
at EPOCH-FINAL fields with ITS OWN mask ({tag,ts} — the
tag-only second entry cannot revive, mask-miss). m12 proved
advances don't drain. Then m11/m13 (an update of a SECOND fact
between the pair) killed drain-at-update-calls too: the
intervening call would consume entry 1 at the z-state. And
after_insert's OLD comment names the true law measured in the
D-150 era: "Drools force-flushes them at an event insert's
queue position" — boundary-only was a knowing approximation
("position-independent"), true only for order-blind observables.

**C′: entries queue per external update call (own masks, FIFO,
the D-154 machine VERBATIM); the queue drains at every external
INSERT call (after the stage snapshot — inside the trigger
delta, the segment-scoping trap that killed the D-150-era
insdrain attempt) and at fire_all pre-fire; updates, advances,
deletes leave the queue alone. Drained effects process FIFO
(staged via push-back — the event drain is FIFO-effect; the
PLAIN inline arms keep push_front/LIFO, a5/a7's pinned order).
The revival identity-fold (round 2) stands: staged eviction del
+ same-fact revival nets to ONE UPD at the drain position.**

Fields at any drain = epoch-final for drained entries BY
GRAMMAR (actions precede facts within an epoch) — the m-matrix
semantics and the collect-order law were never in conflict.

Round-3 predictions (before cells):
- **ed9_two_upds_no_ins** (windowed pair f1→y, f0→w, no epoch
  facts — boundary drain): PREDICT oracle [y,w] (FIFO-effect at
  the boundary too — one uniform drain order). [w,y] would mean
  boundary keeps LIFO — record it, split the staging call.
- **ed8_plainacc_two_upds_then_ins** (plain acc over EVENT
  source, upd f1→y, upd f0→w, ins z): PREDICT oracle [y,w,z]
  (the D-160 queue drains FIFO at the ins call, same as
  windowed).
- The 22-cell set (ed1-7, five members, m-matrix 8, updupd,
  m10, wl_transient) all PASS under C′.

### Round 3 MEASUREMENTS — C′ CONFIRMED

- ed9 oracle [y,w] — PREDICTED EXACTLY (FIFO-effect at the
  boundary drain; one uniform drain order, no split).
- ed8 oracle [y,w,z] — PREDICTED EXACTLY (the plain-event D-160
  queue drains FIFO at the ins call, same as windowed).
- The FULL 24-cell set PASSES: ed1-ed9, the five cf* members,
  m3/m8/m10/m11/m12/m13/m14/m15, updupd_final, wl_transient.

THE PORT (final shape, 4 edits):
1. after_insert: drain_acc_pending() after the stage snapshot,
   before on_insert (the delta placement).
2. The external-update path: comment only (updates do NOT
   drain — m11/m13's discriminant).
3. phreak Staged::add_upd_back (push_back, add_upd's dedup
   verbatim); winacc_step (true,true)-hit + revival-fold +
   plainacc_step (true,true) flip to it. Plain inline arms
   keep add_upd (push_front/LIFO — a5/a7).
4. winacc_step (false,true) revival: staged del + re-assert
   nets to UPD (remove_first_by_key + add_upd_back) — the
   round-2 fold, kept.
Everything else (AccEntry queue, masks, FIFO evaluation,
aliveness, del_pos/drain_dead, the Del arm, winlen landing
law) — VERBATIM pre-port.

OPEN CORNER (noted, unmeasured, no witness): a drained
(false,true) FRESH admission (never-admitted event entering via
ts/alpha update) staged push_front races a same-flush fresh
insert's push_front — Phase E order between them unpinned by
any cell; the admission-vs-ins append order may need its own
pin round if fuzz surfaces it.

### D-327 CLOSE-OUT

Full battery green: byte gate 2415/0/5 (the five witnesses are
the only movement); make diff 11/1445/414 + drift 47; lint
2283/0/0; cargo 73; pytest 257; demo True; model_ird 31/31; IRD
0-div ×5; SD 72 EXACT ×12; agenda_open ×10 ×3; fuzz 327001
clean, 327002's 2 finds pre-existing → banked (fz_327002_845
value-fork, fz_327002_1948 TMS-phantom — NOT this family);
fuzz_cep 3×300 clean. Graduated: 5 members (pr_co_cf*) + the
ed grid (pr_co_ed1..ed9). The collect-order family is CLOSED
end-to-end: D-323 (reverse) + D-324 (windows) + D-326
(identity-fold) + D-327 (drain positions).

## D-328: THE collectSet SUB-ITEM (2026-07-19)

### The hand-decode (xf_fz_662607_47, end-to-end)

Fork: firing[25], R5's collectSet — ONE adjacent swap: engine
[-1.0, -1000000007.0, ...], oracle [-1000000007.0, -1.0, ...];
the other eight elements identical, firings 13/19 identical.
Epoch-1 set ops are PURE ADDS (6.0, a duplicate -1.0,
-1000000007.0, 5.5) — no reverse touches -1.0, so no insertion-
history mechanism can reorder it.

THE ORACLE SOURCE (drools-core 9.44
CollectSetAccumulateFunction): a COUNTED map
(HashMap<Object,MutableInt>; accumulate = get-or-put +
counter++, reverse = --counter, remove at 0; getResult =
keySet()) — dup adds NEVER move a key, confirming the engine's
ga15 counted-set model. Drools' own doc: "the order of the
elements in the set is not guaranteed."

THE ACTUAL FORK — the CANONICALIZATION KEYS DIFFER, not the
engines: D-108 decided "both sides canonicalize SORTED", but
the oracle runner sorts by Jackson-rendered JSON toString
(OracleRunner.java: Comparator.comparing(Object::toString)) in
which Java prints -1000000007.0 as "-1.000000007E9" — sorting
BEFORE "-1.0" ('0' < '}' after the shared "...value\":-1.0"
prefix) — while the engine sorts by its plain-decimal render
("-1.0" < "-1000000007.0", '.' < '0'). The keys disagree
exactly when Java's Double.toString goes SCIENTIFIC (|v| >=
1e7, |v| < 1e-3) and collides in prefix with another element.
c8/c8b/c12 matched because their values never entered the
scientific range.

### The fix (harness canon, engine untouched)

Complete D-108 at the one layer where both sides share a
rendering: harness canon_fact sorts SetCollection element
renderings (post-canonicalization, f:hex keys) before joining.
Content comparison is intact (equal sorted arrays <=> equal
multisets); collectList/Collection stay ORDER-SIGNIFICANT
(D-323). Engine `run` bytes untouched (canon is compare-only)
=> no byte-gate movement, regressions/drift-bank unaffected;
the oracle runner's Java sort becomes harmless. PREDICT:
xf_fz_662607_47 flips PASS; the corpus holds; D-295-scale
receipts (harness-only slab).

## D-328 addendum: Bryan's external wheel verification (2026-07-19)

Bryan ran the first out-of-harness collectList check against the
PyPI 0.4.42 wheel and observed by-FACT removal — reading
**acc_sources**. RESOLUTION (all three of his candidates ruled
out; the fourth surface was in play): acc_sources is the
PROVENANCE channel (D-305: acc_provenance snapshots
ctx.matches — per-fact bookkeeping, removal by FactId — that IS
its contract: "which facts produced this value"). The D-323 law
lives on the FIRED COLLECTION's element order (vlist). Measured
on the shipped PyPI wheel, his exact geometry ([-4,-1e9,-4],
delete first-inserted -4 vs last-inserted -4):
- Collection elements: BOTH deletes → [-1e9,-4] — the
  first-value-equal law, the fix IS in the wheel;
- acc_sources: tracks the dead fact's own entry — exactly his
  numbers, correct provenance semantics.
Oracle cross-check: ed10/ed11 (his geometry as cells) PASS
engine-vs-oracle. Also ruled out BY MECHANISM: the maturin-
develop .so clobber CANNOT reach PyPI wheels (CI compiles from
source at the tag; the tracked .so is a source-tree
convenience).

NEW OPEN ITEM (found by his check): **the CI-built wheel's
certification()["commit"] is "unknown"** (local builds embed
a74d345) — the release workflow loses the git context, so a
shipped wheel cannot self-identify its commit. The one-move
answer to "does this wheel carry fix X" doesn't work on PyPI
artifacts until the workflow passes the commit through.

## D-368 candidate: the DRIVER re-pair walk order (fz_356002_1512)

The family reopens on a NEW axis. The banked D-363-round quarantine
fz_356002_1512 (3× oracle-stable) forks NOT on the Collection's
element order (in-list order AGREES both sides at every delta) but
on the ORDER OF THE DRIVER-SIDE R0 RE-FIRINGS when a collect delta
re-pairs all left tuples. Minimized (m1512: 2 rules, 3 facts —
driver `T0(f1 in ...)` + `from collect`, plus a modify pump):

- delta-1 walk h3,h2,h1 BOTH (newest-insert-first);
- delta-2 (h2 just modified) walk h3,h1,h2 BOTH;
- delta-3 (h3 just modified) walk ORACLE h1,h2,h3 — **h2 STAYS
  displaced from delta-2** — vs ENGINE h2,h1,h3 (h2 reverts to its
  insertion slot; only the current delta's modified tuple lands
  last).

MECHANICAL MODEL (fits all three deltas): the oracle's left memory
is a list walked front-to-back where **insert = push-front**
(newest-insert-first) and **modify = move-to-BACK, persistently**
(a modify physically re-locates the tuple — the identity-model law
family: Drools kills/re-adds the tuple object; the engine's
value-keyed in-place update keeps the slot). The engine's delta-2
agreement is an artifact: the modified tuple's own update staging
orders after the right-delta walk, so ONE recent modify looks
correct; the SECOND delta exposes the lost relocation.

Uninformative corner noted: a modify of the tuple already at
walk-back (m1512's h1) is positionally invisible — no relocation
evidence either way.

### THE GRID (predictions registered 2026-07-20 BEFORE cells run)

Shared shape: drivers T0(a),T0(b),T0(c) [insert order], right/
collect subject T1; driver pattern watches f0; walks read off R0's
firing order per delta. Oracle 3× on any surprise.

- **mo1_ctrl** — no modify; epoch inserts T1 j2 → delta walk.
  PREDICT MATCH c,b,a both (high) — newest-insert-first.
- **mo2_relocate** — update b (epoch), then insert j2 → walk.
  THE LAW CELL. PREDICT ORACLE c,a,b / ENGINE c,b,a → DIVERGE
  (high). Also behaviorally verifies epoch-update target indexing.
- **mo3_ab** — update a (walk-back already: invisible), then
  update b, then insert j2. PREDICT ORACLE c,a,b / ENGINE c,b,a →
  DIVERGE (med-high); the a-update must NOT change the oracle walk
  (move-to-back idempotent at back).
- **mo4_ins_after_mod** — update b; insert T0 d; insert j2 → walk.
  PREDICT ORACLE d,c,a,b / ENGINE d,c,b,a → DIVERGE (med-high) —
  insert lands walk-FIRST even after a modify.
- **mo5_vkeep** — update c with IDENTICAL fields; insert j2. Does a
  value-identical external update relocate? PREDICT ORACLE b,a,c
  (med-LOW — no value-diff check expected) vs stable c,b,a; ENGINE
  c,b,a either way. MEASURE.
- **mo6_plainjoin** — NO collect: driver + plain T1(g0=="on")
  pattern; update b; insert T1 on → walk of the three join pairs.
  PREDICT ORACLE c,a,b (med) — the law is generic beta-memory, not
  accumulate-specific. A c,b,a MATCH here = the law is
  collect-delta-specific (port site narrows).
- **mo7_shrink** — update b; DELETE T1 j1 → empty-collection delta
  → walk. PREDICT ORACLE c,a,b / ENGINE c,b,a → DIVERGE (med-high)
  — delta direction (grow/shrink) is irrelevant.
- **mo8_rhsmod** — the witness's confound bridge: RHS modify (rule
  at salience 10 rewrites b→b2 at fire time) instead of an epoch
  update; then insert j2 → walk. PREDICT ORACLE c,a,b2 / ENGINE
  c,b2,a → DIVERGE (med).

### GRID MEASUREMENTS (2026-07-20; oracle 3× on the DIVERGE cells)

The naive move-to-back model died well — the real law is sharper:

- mo1_ctrl MATCH c,b,a — predicted.
- mo2_relocate MATCH **c,a,b2 BOTH** — prediction WRONG about the
  engine: it reproduces the relocation on GROW deltas.
- mo3_ab MATCH c,a2,b2 both (a's back-position update invisible —
  predicted for the oracle, engine matches too).
- mo4_ins_after_mod MATCH **c,a,b2,d both** — prediction WRONG
  (insert lands walk-LAST, not first): an epoch insert is a TOUCH
  like any other; the walk is least-recently-touched-first.
- mo5_vkeep MATCH **b,a,c both** — a value-IDENTICAL external
  update relocates (med-low "no relocation" alternative dead).
- mo6_plainjoin MATCH a,b2,c — plain-join right-insert walks
  INSERTION order on both sides; the law (and the fork) is
  specific to the collect/acc delta path.
- mo8_rhsmod MATCH c,b2,a — a pre-materialization RHS modify does
  NOT displace (the D-323 pre-materialization lesson again).
- **mo7_shrink DIVERGE** (3× stable): E c,b2,a / O c,a,b2.
- **mo10_shrink_nonempty DIVERGE**: E ...,c,b2,a / O ...,c,a,b2 —
  not empty-collection-specific.
- **mo11_grow_then_shrink DIVERGE** — THE DEFINITIVE CELL: the
  grow delta walks c,a,b2 correctly on BOTH sides, then the
  immediately-following shrink walks E c,b2,a / O c,a,b2. The
  engine HAS the touch order and uses it on grow; the shrink path
  ignores it.

THE LAW (final): on a collect/acc delta, re-paired driver children
stage in PERSISTENT TOUCH ORDER — least-recently-touched driver
fires first; any WM touch (external update incl. value-identical,
insert) re-seats the driver at the walk's back; initial batch =
newest-insert-first. Plain joins are a different (matching)
sub-mechanism; pre-materialization RHS modifies don't displace.

THE ENGINE SITE (read, not yet ported): eval_acc_node Phase E
(right inserts) walks lefts_bucket_pub — the left memory where
re_add_left_tuple has relocated touched lefts → touch order →
grow deltas CORRECT. Phase B (right deletes) walks acc_by_right[f]
— match-ARRIVAL order, never updated on left touches → shrink
deltas lose every prior relocation. The witness's delta-3 rides
Phase B too (the alpha-exit right update propagates as a right
del). NOTE the Phase B comment: visit order pinned by 25 round-2
regressions — those pin the accumulator FOLD order (in-list,
remove-first-equal), separable from the temp.add_upd STAGING
order that drives the firing walk; a port must reorder ONLY the
staging (or two-loop it) and byte-gate against those 25. Phase
C's index-moved arm also walks arrival order (untested corner —
a right-UPDATE-delta cell would pin it if the port needs it).

DISPOSITION: fz_356002_1512 is an ORDER fork (firing multiset AND
final facts multiset equal; harness verdict = firing[17] adjacent
swap) — the D-363 "value fork" label was wrong, corrected here.
Minimized witness m1512 (2 rules, 3 facts) reproduces the class.
Port = Bryan gate; expected byte movement on a port: mo7/mo10/
mo11 + the witness flip PASS; the 25 arrival-order regressions
must NOT move.

### mo12_idx_move (the Phase C corner, port-scoping cell)

`from collect` with a driver-bound source constraint is WALLED
(subnetwork, D-041) — but indexed ACCUMULATE is in-subset, and the
corner is reachable there. mo12: two same-key drivers (a1,a2) +
accumulate(T1(g0 == $k); count); touch a2 (epoch f1 update); then
move the T1's key OUT (g0 k1→k9 — an index-move right update).
MEASURED (pre-port): initial + touch-refire AGREE; the index-move
re-pair walks ORACLE a1,a2t (touch order — the mo-grid law
verbatim) vs ENGINE a2t,a1 (arrival order) → DIVERGE, the Phase C
index-moved arm confirmed as a second site. PREDICT: the port
flips mo12 PASS alongside mo7/mo10/mo11/m1512/fz_356002_1512.

### D-368 CLOSE-OUT (2026-07-20, the port landed)

The 2-site port (eval_acc_node Phase B + Phase C/index-moved:
fold arrival-order, stage bucket-order — Phase E's proven shape;
staged-left gate untouched; safety sweep for out-of-bucket
matches). Byte gate 2636 files vs the pre-edit binary: movers =
EXACTLY the six intended (mo7/mo10/mo11/mo12/m1512/
fz_356002_1512), zero others — the 25 pinned arrival-order
regressions all held. All 14 cells + the witness PASS. FOURTEEN
graduations (pr_co_fz_356002_1512 from xfail + pr_co_m1512 +
pr_co_mo1..mo12); rebank 10 -> 9. make diff 11/1645/414 + drift
9 (one fz_123_6887 parallel-load flap, sequential re-run PASS —
the D-367 ruling's remedy); lint 2530/0/0; cargo 74; pytest 260;
demo True; model_ird 31/31 + witnesses 26/26 + cells 39/39; IRD
0-div ×5; SD 71 EXACT cell-for-cell; agenda_open ×10 identical
×3 binaries (debug/release/pre-edit) + rerun ×3; fresh fuzz
2×2000 seeds 359001/359002 CLEAN (0 xfail draws) + cep 3×300
359901-903 CLEAN; NEXT seeds 360001+. Full narrative: DECISIONS
D-368.

## D-369 candidate: the ADMISSION-vs-INS append order — the D-327
## recorded-open corner SURFACES (cf355901x129)

The D-361-round quarantine cf355901x129 decodes to the corner the
D-327 close-out explicitly left open ("admission-vs-ins append
order may need its own pin round if fuzz surfaces it"). NOT the
temporal-join-order class the D-361 bank note guessed: the CEP
scaffolding (temporal joins, entry points, not-D) is inert — the
fork is W3's windowed collectList element order, one adjacent
swap, oracle 3× stable. Hand-minimized m129 = ONE rule
(windowed collectList over E2) + ONE base fact + two epochs:
evict y@25; then {update y (value-identical, queues per C′) +
insert fresh z@372} in ONE epoch. ORACLE [y,z] — the queued
update's re-admission appends BEFORE the same-call fresh insert;
ENGINE [z,y]. (Both sides agree the evicted y RE-ENTERS on the
tag-only update with ts still outside the window — the admission
SEMANTICS are shared; only the append order forks.) Side note:
tools/minimize.py's rule-splitter expects `rule "` and skips the
CEP fuzzer's unquoted rule names — it minimized epochs only;
m129 was hand-derived.

### GRID (predictions registered 2026-07-20 BEFORE cells run)

- **w2_upd_prev_epoch** — update y in its OWN epoch, insert z the
  NEXT epoch. The boundary drain (ed9-certified) lands y before
  the next epoch's insert. PREDICT [y,z] BOTH → MATCH.
- **w3_ins_then_upd** — insert z, then update y in a LATER epoch
  (no insert after): boundary drain appends y last. PREDICT [z,y]
  BOTH → MATCH.
- **w4_two_ins** — update y + insert z1,z2 in one epoch. ORACLE
  [y,z1,z2] (drain at the FIRST insert call, before its effect).
  ENGINE PREDICT [z1,y,z2] (y rides z1's call but lands AFTER
  z1's own admission — the m129 mechanism localized to one call)
  → DIVERGE (med-high; [z1,z2,y] instead would say the engine
  defers the drain past ALL same-epoch inserts).
- **w5_two_upds** — update y1, update y2, insert z (one epoch,
  y1/y2 = two evicted events). ORACLE [y1,y2,z] (FIFO queue
  drains before the insert). ENGINE PREDICT [z,y1,y2] → DIVERGE.
- **w6_mask_change** — the update changes the tag (y→w), not
  value-identical. Same class. PREDICT ORACLE [w,z] / ENGINE
  [z,w] → DIVERGE (med-high).
- **w7_inwindow_ctrl** — update an event still IN the window +
  same-epoch insert (no revival). Certified surface. PREDICT
  MATCH (the D-326 identity-fold/move-to-tail law).

### GRID MEASUREMENTS (2026-07-20) — 6/6 oracle predictions HIT

- w2/w3/w7 MATCH as predicted (boundary drains + in-window
  updates = certified surface, untouched).
- w4 DIVERGE: ORACLE [y,z1,z2] / ENGINE [z1,y,z2] — the exact
  med-high engine prediction (the misorder is confined to the
  drain's own call; z2's later call appends after).
- w5 DIVERGE: ORACLE [y1,y2,z] / ENGINE [z,y2,y1] — the oracle
  FIFO hit; the engine went FULLY LIFO (all three effects
  push_front), sharpening the mechanism beyond the [z,y1,y2]
  prediction.
- w6 DIVERGE: [w,z] vs [z,w] — not value-identical-specific.

THE MECHANISM (read after the grid): winacc_step's (false,true)
admit arm staged add_ins_ph (push_front INS) whenever the
eviction del was NOT still staged (reentry=false — eviction
flushed in an EARLIER epoch). ed1/ed7 always passed because
their same-epoch eviction leaves the del staged -> the D-326
reentry fold stages add_upd_back -> folds at Phase C BEFORE the
fresh insert's Phase E. THE PORT: the admission stages
add_upd_back unconditionally (one arm), restoring queue-FIFO-
before-insert. REGRESSION CAUGHT BY THE BYTE GATE + FIXED:
xf_cep_acc_updel_flush_win (D-160 graduate) — the update-admit +
same-epoch-delete annihilation checked s_right.ins only; an
admission-upd (staged upd with NO stored match) now annihilates
identically (upd removed, no del staged, net-zero re-emission
forced); the certified in-window upd+del fold is distinguished
by stored matches (acc_by_right non-empty). Final byte gate 2656
= EXACTLY 5 intended movers (m129/w4/w5/w6/cf355901x129);
first-gate iteration recorded: 6 movers incl. the updel break.

### D-369 CLOSE-OUT (2026-07-20)

EIGHT graduations: pr_co_cf355901x129 (from xfail) + pr_co_m129 +
pr_co_w2..w7; rebank 9 -> 8 -> 9 (one fresh pre-existing
quarantine fz_360001_381: COUNT fork engine 25 vs oracle 23,
byte-identical across current/pre-369/pre-368 engines — neither
port implicated; seed re-run 0 div + 1 xfail draw). make diff
11/1653/414 + drift identical (one fz_123_6887 parallel-load
flap, sequential PASS — the ruling's remedy); lint 2545/0/0;
cargo 74; pytest 260; demo True; model_ird 31/31 + witnesses
26/26 + cells 39/39; IRD 0-div ×5; SD 71 EXACT cell-for-cell;
agenda_open ×10 identical ×3 binaries + reruns; fuzz 360002
CLEAN + cep 3×300 360901-903 CLEAN; NEXT seeds 361001+.
CHANGELOG Unreleased +1. The D-327 open-corner note is RESOLVED
by this round.
