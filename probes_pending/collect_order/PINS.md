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
