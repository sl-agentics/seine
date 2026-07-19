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

## Round 3 (part 2, source read): THE GROUND TRUTH — and the law is LAZINESS, not list order

Sources read (drools-core 9.44.0.Final-sources.jar): PhreakNotNode,
PhreakJoinNode, PhreakRuleTerminalNode, RuleNetworkEvaluator,
TupleSetsImpl, TupleList, RightTupleImpl, LeftInputAdapterNode,
SegmentPropagator, RuleExecutor, RuleAgendaConflictResolver,
MatchConflictResolver.

### Iteration/build directions (quoted)

- TupleSetsImpl.addInsert/addUpdate/addDelete: HEAD-PREPEND
  ("setNextTuple(tuple, insertFirst); insertFirst = tuple"); every
  consumer walks head-first (getInsertFirst/getStagedNext) => each
  node-emission hop REVERSES batch order. addAll = tail-append,
  order-preserving (segment boundaries add no extra flip).
  CLASH SEMANTICS (the load-bearing find): addDelete on a
  staged-INSERT tuple ANNIHILATES ("case Tuple.INSERT:
  removeInsert(tuple); return"); addDelete on staged-UPDATE
  demotes (removeUpdate, then stage delete); addInsert on
  staged-UPDATE no-ops ("already staged as an update").
- TupleList.add: TAIL-APPEND ("last.setNext(tuple); last=tuple");
  head-first iteration; removeAdd = move-to-tail. All beta memories
  FIFO by add.
- RightTupleImpl.addBlocked: HEAD-PREPEND ("leftTuple.setBlockedNext(
  this.blocked); this.blocked = leftTuple").
- PhreakNotNode.doNormalNode ARM ORDER: leftDeletes ->
  existentialReorderLeftMemory -> existentialReorderRightMemory ->
  rightInserts -> rightUpdates -> rightDeletes -> leftUpdates ->
  leftInserts.
- doRightInserts (block walk): walks the not's LEFT MEMORY head-first,
  SKIPS lefts staged UPDATE ("ignore, as it will get processed via
  left iteration"), blocked left => setBlocker + addBlocked (prepend)
  + ltm.remove + child delete. Also unlinkNotNodeOnRightInsert:
  empty-beta-constraint nots UNLINK the segment on right insert.
- doRightDeletes (release walk): walks rightTuple.getBlocked()
  HEAD-FIRST via getBlockedNext (= LAST-BLOCKED-FIRST); re-block scan
  rtm.getFirst forward, skipping isDeleted; released =>
  insertChildLeftTuple = ltm.add (TAIL re-entry) + trg.addInsert
  (prepend).
- doUpdatesExistentialReorderLeftMemory: staged-update lefts are
  REMOVED from ltm; only UNBLOCKED ones re-added TAIL-APPEND in
  staged-walk order; a blocked left whose blocker is also staged gets
  removeBlocked (forced re-match).
- doLeftUpdates: unblocked in-memory left is ltm.remove'd first
  ("to ensure iteration order"); re-blocked lefts addBlocked (prepend)
  AFTER the right-insert arm => update-blocked lefts sit at the
  blocked HEAD (released first).
- PhreakJoinNode: doLeftInserts walks staged head-first, ltm.add,
  children emitted per rtm scan, trg.addInsert (prepend);
  doRightInserts skips staged-UPDATE lefts; ARM ORDER: rightDeletes ->
  leftDeletes -> reorderRight -> reorderLeft -> rightUpdates ->
  leftUpdates -> rightInserts -> leftInserts.
- PhreakRuleTerminalNode.doLeftInserts: staged walk head-first ->
  executor.addLeftTuple.
- RuleExecutor: tupleList (TupleList) TAIL-APPEND; STATIC salience =>
  getNextTuple = tupleList.removeFirst() = FIFO in terminal-arrival
  order (the BinaryHeapQueue + MatchConflictResolver LIFO tie-break
  exists ONLY for dynamic salience).
- RuleAgendaConflictResolver (rule selection): salience, then LOWEST
  loadOrder first ("lowest order goes first"), then terminal id.
- RuleExecutor.fire loop: fireActivation -> flushPropagations ->
  haltRuleFiring(peekNextRule); on preemption it BREAKS WITHOUT
  evaluateNetworkIfDirty — the preempted rule's network stays DIRTY;
  evaluation resumes only when that rule is next SELECTED.

### THE LAW (supersedes the round-1 LIFO phrasing)

**A rule's beta network advances only when that rule is selected to
fire (or between its own consecutive firings). Effects staged in the
interim coalesce with TupleSets clash semantics — in particular a
DELETE annihilates a STILL-STAGED right INSERT at a not node: no
block, no cancel, no release ever happens.**

The relay corollary: in the modify->delete relays, R0 (same salience,
lower load order) preempts R1 after every R1 firing. R1's network is
dirty with the staged not-right INSERT when R0's delete lands =>
annihilation. The oracle's "release order" is NOT a release — it is
R1's ORIGINAL round-0 activation queue surviving untouched, consumed
FIFO. Queue order = terminal-arrival order = insertion order reversed
once per staging hop: LIA-direct (2 hops) => FORWARD; join-fed
(3 hops) => REVERSE-INSERTION. Both r1/r2 "LIFO" measurements and
nb1's [3,2,1] release fall out of one machine with no order axis free.

nb1 keeps a REAL block+release because selection intervenes: R
(salience 0) is selected between U's modify (-5) and D's delete
(-10), so the staged right insert flushes (blocks) before the delete
arrives. The discriminator between the shapes is selection-between-
arrival-and-delete, not memory orientation. The D-331 join-emission
suspicion is RETIRED; the D-158 shadow suspicion is doubly dead
(fz_7_2364's not is beta-constrained: f1 > $b1_0_0).

### Round 3 predictions (REGISTERED BEFORE the instrument runs)

Full fire sequences (rule(T0-idx or L-id)), lazy source machine:
- nb1: R(1) R(2) R(3) U D R(3) R(2) R(1) — initial batch FORWARD
  (LIA-direct parity), release reversed. [certified cell: engine
  already matches; this pins the initial-batch claim]
- r1: R1(4) R0(4) R1(3) R0(3) R1(1) R0(1) R1(0) R0(0)
  (f0 order 9, -1e9+7, -5, 4 — matches round-1 measurement)
- r2: R0(3) R1(4) R0(4) R1(1) R0(1) R1(0) R0(0) — NO initial-block
  release: R0 fires first, the blocker dies before R1's network ever
  evaluates (annihilation of ALL its staged tuples). Consumption
  -1e9+7, -5, 4 = the round-1 measurement, now explained without any
  release lane.
- r3: R(3) R(2) R(1) after D — a REAL walk-in-block + release
  (blocker pre-exists; D is selected before R? no: R salience 0
  evaluates first, all lefts block at walk-in, zero activations; D
  then deletes; release emits blocked head-first [L1,L2,L3], one
  staging flip => 3,2,1).
- r4: D(5) first (salience 5): L3 dies by annihilation before R
  evaluates; R fires (2,M) then (1,M). Order [2,1].
- fz_min_7_2364: R1(3) R0(3) R1(1) R0(1) R1(0) R0(0) — idx3, idx1,
  idx0 = the recorded oracle fork side.
- nb3: R1(3) R0(3) R1(2) R0(2) R1(1) R0(1) — reverse insertion.
- fz_7_2364: R0(3) R1(4) R0(4) R1(1) R0(1) R1(0) R0(0) — t3 is the
  initial blocker, deleted before R1 evaluates (annihilation); then
  the relay idx4, idx1, idx0.

### Round 3 model grid + r5 (prediction registered before the cell runs)

model_check_notrel.py, 16 machines x 8 timelines: the four lazy
survivors collapse to TWO classes — {rel=head,blk=prepend} ==
{rel=tail,blk=append} is a gauge pair (rev∘rev = id; the observable is
"release visits newest-blocked-first"), but clash annihilate-vs-keep
is NOT discriminated by the 8: every block there hits ALL queued
activations or none. r5_partial_block discriminates: queue [c(f0=0),
b(f0=1), a(f0=10)]; R1 fires c; the staged blocker rt_c (f1=5) would
block b (5>1) but not a (5>10); R0 deletes c.
- ANNIHILATE (source): rt_c never reaches the node; queue untouched.
  PREDICT: R1(0) R0(0) R1(1) R0(1) R1(10) R0(10).
- KEEP (block+release in one batch): b cancelled and re-released to
  the tail -> R1(0) R0(0) R1(10) R0(10) R1(1) R0(1).
- The EAGER engine should also produce the keep order (block real at
  fire-1's flush, release at R0's delete).

### Round 3 verdict (model_check_notrel.py, 16 machines x 9 timelines)

r5 measured: oracle = the ANNIHILATE order (3x-stable), engine = the
keep/eager order — both registered predictions hit. Grid result:
UNIQUE SURVIVOR CLASS = (lazy, annihilate) x the {rel=head,blk=prepend
== rel=tail,blk=append} gauge pair. The EAGER variant of the SAME
machine reproduces the ENGINE's measured sequences on all five fork
cells (r1, r2, fz_min, nb3, r5) — the laziness axis ALONE explains the
divergence; every list direction composes identically in both engines.
Bonus: the eager+inverted-release machines fail exactly {nb1, r3, r5}
— the D-331 naive flip's measured breakage profile, now explained.

THE PORT (named by the survivor): the engine must defer not-node
right-side admissions produced by RHS effects until the owning rule is
next SELECTED to fire, with fact-keyed ins+del ANNIHILATION in the
deferral window (a delete of a fact whose right-insert is still
deferred cancels it outright — no block, no cancel, no release).
Would-graduate: fz_7_2364, fz_min_7_2364, fz_7_9360, nb3 + the r1/r2/
r5 lane cells.

## THE PORT (D-333, landed)

The engine ALREADY carried the whole lazy structure: D-091's halt
(skip the continue-path self re-evaluation) existed for STRICTLY-
higher salience; D-320's tie_preempt is the same halt inside a focused
group; RHS Update/Delete only stage; Staged::add_del already
annihilates a staged ins ("never materialized: cancel"); eager_flush
evaluates only no-loop/dyn rules (= evaluateEagerList); the agenda pop
is lazy in (salience DESC, decl ASC). The ONE gap: the MAIN-group halt
gate ignored the RuleAgendaConflictResolver load-order tie-break.

Three engine.rs edits at the D-091 site:
1. post-fire force-eval skipped when `eq_decl_preempt` (the EXISTING
   D-199 P3 predicate: lazy rule, equal-salience decl-preceding queued
   same-group item) — EXCEPT when the rule has TMS deferred/exp
   entries (their D-198/199/201 drain calibration assumes the eval;
   that whole lane stays byte-identical by construction).
2. the D-258 late-continue force mirrors the new skip
   (`higher || eq_decl_preempt`).
3. the focused-group !tie_preempt continue gains the Drools
   evaluateNetworkIfDirty (dirty-only — byte-neutral on the old flow,
   live only when the new halt skipped the force-eval).

MEASURED: all 9 fitness cells PASS engine-vs-oracle + fz_7_9360 (the
4th would-graduate) + fz_327002_845 (a banked D-327 latent — the
insertLogical boolean value fork was THIS bug composed with TMS: a
BONUS graduation). The 11-cell D-331 counter-set: all PASS. Byte gate
vs a20dd5a: 2455/2462 SAME, 7 diff = EXACTLY the 6 lane/would-graduate
cells + fz_327002_845, 0 moved — surgical. make diff 11/1486/414 (ten
graduations: pr_nr_fz_7_2364, pr_nr_fz_min_7_2364, pr_nr_fz_7_9360,
pr_nr_nb3, pr_nr_fz_327002_845 + the r1-r5 lane cells as pr_nr_*),
drift bank 24 -> 19, lint 2334/0/0, agenda_open x10 identical x3.
