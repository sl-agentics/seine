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

## D-334 recon: cf325901x52 rechecked with the D-333 toolkit (the
## NOT-LEAD shape — lane probes_pending/notlead/)

x52's NW4 is `not DW(v>=1) P()` — the not is pattern 0, so the
network is InitialFact -> not(DW) -> JOIN(P) -> rtn. The blocked
list gates ONE tuple (the InitialFact); the P-witness order on a
release comes from the DOWNSTREAM JOIN's re-propagation, not from
any blocked-list walk. Source composition: released left ->
join doLeftInserts walks P's RIGHT MEMORY forward (TupleList:
tail-append; doUpdatesReorderRightMemory removeAdd = move-to-tail
on update) -> children prepend-staged -> ONE reversal at the
terminal -> consumption = REVERSE P-right-memory order.

x52 replay under this law: P-rtm = [P1(2), P0(3)->tail at the
epoch-2 v-update, P2(2) appended] = [P1,P0,P2]; reversed =
P2(2), P0(3), P1(2) = the oracle's [2,3,2] EXACTLY. The engine
emits [2,2,3] (P0 last) — the engine's not-release re-propagation
does not walk the P memory in the oracle's order.

### Predictions (REGISTERED BEFORE the cells run)

- nl1_base (P1,P2,B(false),P3; R = not B(g==true) P; U modify -5;
  D delete -10): initial batch = reverse P-memory [3,2,1]; U; D;
  release re-propagation = reverse P-memory again: R(3) R(2) R(1).
  Full: R(3) R(2) R(1) U D R(3) R(2) R(1).
  (initial: the InitialFact left-inserts once; the join walks
  P-rtm [1,2,3] forward, one staging reversal => 3,2,1.)
- nl2_update_move (adds M salience -3: modify P(v==1) setT(1)
  between the R batch and U): M's update moves P1 to the rtm TAIL
  => rtm [P2,P3,P1]; ALSO the pass->pass P update re-fires R for
  P1 (terminal update re-add). Full prediction:
  R(3) R(2) R(1) M R(1) U D R(1) R(3) R(2).
  (M's re-fire of R(1): the updated child re-queues; release
  order = reverse [P2,P3,P1] = P1,P3,P2 => R(1) R(3) R(2).)

### nl1/nl2 measurements — BOTH PREDICTIONS MISSED (recorded), and
### the miss decodes cleanly

nl1 oracle (3x): R(1) R(2) R(3) U D R(1) R(2) R(3) — initial AND
release both FORWARD; engine IDENTICAL (the clean not-lead shape
is not divergent at all). nl2: no memory move, no R re-fire.
The corrected composition (two errors in the registered one):
1. The join's right memory is built in STAGED-WALK order (the
   batch processes LIFO, rtm.add appends) => rtm = REVERSED
   insertion per batch, and the join-walk + one staging reversal
   flips consumption back to FORWARD-insertion. nl1's [1,2,3]
   both times ✓.
2. nl2's setT update was MASKED by property reactivity: R binds
   only $v, so a t-update never stages at R's join — no
   removeAdd move, no terminal re-add. (x52's NW4 has a BARE
   P() = watches everything; its v-update DOES stage.)
x52 under the corrected law: per-epoch batches give rtm
[P0],[P0,P1],（v-update moves P0 to tail）[P1,P0],[P1,P0,P2];
release: join walks FORWARD P1,P0,P2, prepend-stages, terminal
reverses => consumption P2(2), P0(3), P1(2) = [2,3,2] = THE
ORACLE, EXACTLY.

### nl3 prediction (REGISTERED BEFORE the cell runs)

nl3_watched_move (P1(1),P2(2),B(false),P3(3); M salience -3
modifies P(v==2) to v=5 — WATCHED field, self-disabling):
- initial: batch-LIFO rtm [P3,P2,P1], walk+flip => R(1) R(2) R(3).
- M(2) fires; the v-update stages at R's join: removeAdd moves P2
  to the rtm tail => [P3,P1,P2]; the child update re-adds the
  fired match at the terminal => R re-fires the updated P: R(5)
  (salience 0 preempts M's -5 continuation... M at -3, R at 0 =>
  R(5) lands immediately after M(2)).
- U, D, release: walk [P3,P1,P2] forward, prepend, terminal flip
  => consumption P2,P1,P3 = R(5) R(1) R(3).
FULL: R(1) R(2) R(3) M(2) R(5) U D R(5) R(1) R(3).

### nl4 prediction (REGISTERED BEFORE the cell runs)

nl4_external_move: B(g=true) from the start (the IF-left blocks
walk-in; NO initial R batch; the P-join's rtm builds [P3,P2,P1]
with no children); epoch 1 = EXTERNAL update P2 v->5 (watched)
=> removeAdd moves P2 to the tail: [P3,P1,P2]; epoch 2 inserts K
=> D deletes B => release => join walks [P3,P1,P2] forward, one
flip => R(5) R(1) R(3).
- ORACLE PREDICT: D then R(5) R(1) R(3).
- ENGINE PREDICT (if x52's residual is the external-update move
  missing at the not-lead join): no move => rtm [P3,P2,P1] =>
  R(1) R(5) R(3) — the fork reproduces WITHOUT events or TMS.

### nl4 measured / nl5 prediction (registered before nl5 runs)

nl4: oracle prediction HIT EXACTLY (D, R(5) R(1) R(3), 3x) — but
the ENGINE prediction MISSED: the engine matches the oracle. The
plain-cloud external-update move is already correct. x52's
residual must ride the EVENT-SESSION flavor: stream mode flushes
per insert (D-102) => the join rtm builds in INSERTION order
(singleton batches), and external updates take the stream-mode
path.

nl5_stream_move = nl4 + one inert event type (session flips to
stream). Composition: rtm [P1,P2,P3] (per-insert flush); update
P2 (watched) => removeAdd => [P1,P3,P2]; release => forward walk
+ one flip => consumption P2,P3,P1.
- ORACLE PREDICT: D then R(5) R(3) R(1).
- ENGINE PREDICT (if the residual = external-update move missing
  on the STREAM path): no move => rtm [P1,P2,P3] => flip =>
  R(3) R(5) R(1).

### nl5 measured / nl6 prediction (registered before nl6 runs)

nl5: oracle D R(5) R(1) R(3) = SAME as nl4 (the per-insert-flush
rtm sub-prediction was wrong — the initial batch composes as ONE
window in stream mode too, D-102); engine matches. Still no
repro. The remaining x52 delta: MULTI-EPOCH P arrivals with the
watched update in its own epoch (x52: Pa epoch 0, Pb epoch 1,
update-Pa epoch 2, Pc + release epoch 3).

nl6_multiepoch_move mirrors that timeline (cloud, plain blocker):
Pa(1) epoch 0 (B(true) blocks walk-in), Pb(2) epoch 1, external
update Pa v->3 epoch 2, Pc(4) + K (delete B, release) epoch 3.
rtm: [Pa] / [Pa,Pb] / move Pa -> [Pb,Pa] / [Pb,Pa,Pc]; release
forward walk + one flip => Pc, Pa, Pb.
- ORACLE PREDICT: D then R(4) R(3) R(2)  (= x52's [2,3,2] shape).
- ENGINE PREDICT (if x52's residual reproduces): no external
  move on the epoch-accumulated rtm => [Pa,Pb,Pc] => flip =>
  R(4) R(2) R(3)  (= x52's [2,2,3] shape).

### nl6 measured — BOTH predictions missed; ENGINE MATCHES ORACLE
### on ALL FIVE nl cells. Round verdict.

nl6 oracle (3x) AND engine: D R(2) R(4) R(3) — consumption
[Pb, Pc, Pa]. No single-flip composition over any defensible rtm
order produces this ([Pb,Pa,Pc]-walk-reversed = [4,3,2] was the
registered prediction). The hand-composition of the not-lead
join's release re-propagation is WRONG in a way the nl cells
cannot yet discriminate — a D-083 stop-and-model signal for THIS
shape. Crucially: the engine agrees with the oracle on every nl
cell (nl1-nl6) — the clean not-lead release surface is HEALTHY.

### D-334 RECON VERDICT (the x52 recheck)

1. x52 is NOT a notrel-family member and the D-333 laziness law
   does NOT explain it: its not gates the InitialFact tuple; the
   P-witness order rides the DOWNSTREAM join re-propagation.
2. The clean not-lead surface (5 cells: base, masked update,
   watched RHS-move, external-epoch move, stream-mode move,
   multi-epoch move) is engine==oracle THROUGHOUT — no clean-
   shape witness exists.
3. The registered walk-order composition is falsified by nl6
   (both engines agree on an order it cannot produce) — the
   not-lead join memory/emission law is OPEN; pinning it needs a
   model round (memory build x reorder timing x emission dir)
   BEFORE x52's fork can be decoded.
4. x52's fork therefore lives in the UNREPRODUCED residual: the
   event-session x windowed-acc x TMS-supersede composition (DW
   del+ins churn per epoch at the not right, expirations, the
   value-preserving E0 update). It keeps its xfail seat; the
   next attack is the notlead model round + a TMS-flavored nl7.

Prediction scorecard this recon: nl1/nl2 registered predictions
MISSED (recorded; decodes = batch-LIFO memory build + property-
reactivity masking), nl3/nl4 oracle predictions HIT exactly,
nl5/nl6 partially/fully MISSED. The misses are the finding: the
clean surface is healthy and the composition is subtler than the
D-333 toolkit's straight-line application.

## D-334 round 2: THE NOTLEAD MODEL ROUND (Bryan: "do the notlead
## model round")

### The nl6 decode — D-031 UNLINK closes the composition

The nl6 "impossible" [2,4,3] composes EXACTLY once the D-031
unlink joins the toolkit: an ALPHA-ONLY not UNLINKS the path
while a blocker exists (unlinkNotNodeOnRightInsert at eval;
relink when the right count returns to 0). While unlinked the
rule is never selected, so staged effects ACCUMULATE ACROSS
EPOCHS and all process in ONE relink evaluation:
  nl6: e0 eval blocks the IF-left + unlinks, rtm [Pa]; e1 ins Pb
  and e2 upd Pa just SIT staged; e3 relink eval: reorder walks
  staged upds (Pa in rtm: singleton removeAdd = no-op... Pa IS
  rtm-resident: removeAdd -> [Pa] unchanged), rightIns walks the
  staged ins LIFO [Pc, Pb] -> rtm [Pa, Pc, Pb], leftIns walks
  forward + one flip => consumption [Pb, Pc, Pa] = [2,4,3] ✓.
All six nl cells re-compose exactly under {D-333 lists + D-031
unlink + cross-epoch staged accumulation + the join arm order}.

### nl8 prediction (REGISTERED BEFORE the cell runs) — the
### staged-upd walk + reorder-timing discriminator

nl8_two_upds: Pa(1), Pb(2), B(true) (rtm [Pb, Pa] — initial batch
LIFO walk); e1 upd Pa v->3; e2 upd Pb v->5; e3 Pc(4)+K release.
Source machine (reorder at EVAL, staged-upd walk HEAD-first =
LIFO [Pb, Pa]): removeAdd Pb -> [Pa, Pb]; removeAdd Pa ->
[Pb, Pa]; rightIns Pc -> [Pb, Pa, Pc]; consumption reverse =
[Pc, Pa, Pb] => ORACLE PREDICT: D then R(4) R(3) R(5).
Competing machines: upd-walk TAIL-first (FIFO) => [Pa, Pb, Pc] =>
R(4) R(5) R(3); reorder at ARRIVAL (per-epoch, no accumulation)
=> e1 no-op (Pa at tail), e2 -> [Pa, Pb] => R(4) R(5) R(3) (same
as FIFO — nl8 separates source from BOTH).

### nl7 prediction (REGISTERED) — the TMS-churn flavor (x52's
### missing composition, cloud-clean)

nl7_tms_churn: W: K2($n:n) => insertLogical(B2($n)); R: not
B2(n>=1) P($v). e0: W fires B2(5), R blocks + unlinks. e1: P(2)
ins + K2 n->7: W refires => supersede: ins B2(7) + del B2(5) —
BOTH STAGE at the unlinked not (net right count stays 1, no
relink). e2: K2 n->0: W refires, B2(0) fails the alpha => only
del B2(7) stages => count 0 => RELINK: the accumulated staged
set {ins B2(7), del B2(5), del B2(7)} ANNIHILATES B2(7)
(ins+del, the D-333 clash law) leaving del B2(5): pure release,
NO re-block. Join: rtm [Pa] + staged ins Pb -> [Pa, Pb];
consumption [Pb, Pa].
ORACLE PREDICT: W W W R(2) R(1).
ENGINE: genuinely uncertain — insertLogical routes through the
eager TMS machinery (materialize -> on_insert immediately); if
the engine's B2(7) reaches rtm eagerly at e1, its e2 release
walks a DIFFERENT composition. The cell IS the probe; a fork
here = the clean witness for x52's class.

### D-334 round-2 verdict (model_check_notlead.py, 8 machines x 5
### timelines)

nl7 AND nl8 predictions both HIT EXACTLY (3x-stable): nl8 = the
staged-upd LIFO walk with eval-time reorder (refutes FIFO and
per-epoch-arrival machines); nl7 = the TMS-supersede churn incl.
the D-333 ins+del ANNIHILATION at the unlinked not (W W W R(2)
R(1) — the accumulated {ins B2(7), del B2(5), del B2(7)} nets to
one release, no re-block). The ENGINE matches the oracle on BOTH
— all EIGHT nl cells engine==oracle.

Grid: UNIQUE SURVIVOR = (unlink=on, updwalk=lifo, inswalk=lifo).
Refutations: unlink=off fails nl6+nl8 (per-epoch reorder moves
the resident P too early); ins=fifo fails nl1/3/4/6 (the cross-
epoch staged-ins walk is newest-first); upd=fifo fails nl8.

THE NOT-LEAD LAW (pinned): a leading not gates the InitialFact
tuple; while a blocker exists the ALPHA-ONLY not UNLINKS its path
(D-031) and ALL staged effects accumulate across fireAllRules
boundaries; the relink evaluation processes them in ONE batch —
reorder walks staged upds LIFO (removeAdd = move-to-tail),
rightIns walks staged ins LIFO (tail-append), the released
IF-child then walks the memory FORWARD with one staging flip to
the terminal FIFO. The engine implements this composition
correctly on the entire clean surface, INCLUDING plain-TMS
supersede churn.

x52's residual is now tightly bounded: everything in its shape
EXCEPT the event-session x windowed-accumulate composition
(window:time sum + expirations + the D-154/D-160 entry machinery
+ the value-preserving E0 update) is certified healthy. The next
attack is a windowed nl9 ladder toward W2's exact accumulate —
a clock-plane round, not a notlead one.

## D-334 round 3: THE CLOCK-PLANE ROUND (Bryan: "do the clock-plane
## round") — x52 minimization + the stream-flush law

Minimization (fork survives all): m1 drops TJ0/TJ1/RW3; m2 drops
E0 + the value-preserving update; m3 drops one E2; m3b drops the
P-UPDATE — none load-bearing. m3b fork: engine [2,4,1] vs oracle
[4,2,1] (P0 e0, P1 e1, P2 e3, no updates).

THE DECODE against the pinned notlead law: the ENGINE's [2,4,1]
IS the certified CLOUD composition (unlink accumulation + LIFO
staged-ins walk at relink: [P2,P1] -> rtm [P0,P2,P1] -> flip).
The ORACLE's [4,2,1] = rtm [P0,P1,P2] = INSERTION-ordered — the
stream session's per-insert force-flush (D-102) drives join-side
inserts into node memories AT ARRIVAL even while the path is
unlinked-by-blocker; nothing accumulates.

### m4 prediction (REGISTERED BEFORE the cell runs)

If the law is session-level, the windowed DW is NOT load-bearing:
m4 = plain-B blocker (nl6 shape minus the update) + an inert
event type (stream session) + multi-epoch Ps.
- ORACLE PREDICT: D then R(4) R(2) R(1) (insertion-ordered rtm
  [Pa,Pb,Pc], one flip).
- ENGINE PREDICT: D then R(2) R(4) R(1) (cloud accumulation
  carried into the stream session: staged LIFO [Pc,Pb] -> rtm
  [Pa,Pc,Pb] -> flip).
A fork here = the minimal witness class for x52 with NO windows,
NO TMS, NO clock advances.

### m4 measured — ORACLE PREDICTION MISSED (recorded): plain-B
### stream accumulates like cloud on BOTH sides. The real law: THE
### TRANSIENT RELINK

m4: oracle [2,4,1] = the accumulation composition (engine
identical, no fork). The stream-flush hypothesis is DEAD. The
load-bearing delta in m3b is the DW CHURN: Drools removes the
superseded belief at MATCH-CANCEL time (W2's network evaluation,
BEFORE the refire's insertLogical) — the not's right count
transiently hits ZERO each churn epoch => RELINK + item queued
=> NW4's pop-evaluation drains the accumulated staging THAT
epoch => the join rtm builds INSERTION-ORDERED. Seine's D-076
supersede is an EPILOGUE (ins-then-del): the count never dips,
no transient relink, accumulation to the final release.
COROLLARY: nl7's hit was outcome-correct but mechanism-
underdetermined (2 Ps: accumulation and per-epoch give the same
order) — the fork should reproduce in PURE CLOUD with 3 Ps.

### p3 prediction (REGISTERED BEFORE the cell runs) — the cloud
### minimal witness

p3_cloud_churn3: W: K2($n:n) => insertLogical(B2($n)); R: not
B2(n>=1) P($v). P0(1)+K2(5) initial; e1 upd K2 n->7 + P(2); e2
upd K2 n->9 + P(4); e3 upd K2 n->0 (release).
- ORACLE PREDICT (transient relink per churn epoch): rtm builds
  per-epoch [P0,P1,P2]; release => flip => R(4) R(2) R(1).
- ENGINE PREDICT (epilogue supersede, no transient): staged
  accumulation => rtm [P0,P2,P1] at the release => R(2) R(4)
  R(1).
A fork here = the x52 class witnessed with NO events, NO windows,
NO clock — pure cloud TMS churn.

### p3 measured — BOTH PREDICTIONS EXACT (3x): oracle [4,2,1],
### engine [2,4,1]. THE X52 CLASS IS WITNESSED IN PURE CLOUD.

### p1 control prediction (REGISTERED): plain non-TMS churn

p1_plain_churn: C: $k:K3() $b:B(g==true) => delete($b);
insert(new B(true)); delete($k); (one churn per K3); D: K4()
B(g==true) => delete. P0(1)+B(true) initial; e1 +P(2)+K3; e2
+P(4)+K3; e3 +K4.
A plain delete's not-right count DOES dip to 0 at arrival (the
relink fires before the same-RHS re-insert) => BOTH engines
should transient-relink each churn epoch => insertion-ordered
rtm => release R(4) R(2) R(1), NO FORK. This isolates the fork
to the TMS SUPERSEDE TIMING: Drools removes the superseded
belief at match-update/cancel (count dips, relink); Seine's
D-076 epilogue removes it AFTER the refire's insertLogical
(count never dips).

### p1 measured — CONTROL EXACT, no fork ([4,2,1] both sides)

### D-334 ROUND-3 VERDICT (the clock-plane round)

THE LAW (confirmed by p3 fork + p1 control, all predictions
exact): **the TMS SUPERSEDE-TIMING TRANSIENT RELINK.** When a
justifier's match updates (its accumulate result or bound values
changed), Drools removes the superseded belief AT MATCH-UPDATE/
CANCEL TIME — before the refire's insertLogical — so a not node
blocked solely by that belief sees its right count transiently
hit ZERO: the path RELINKS, the item queues, and its pop-
evaluation drains all accumulated staging THAT epoch (join
memories build insertion-ordered). Seine's D-076 refire-supersede
is an EPILOGUE (the new belief inserts during the actions, the
stale key retracts after): the count never dips, no transient
relink, staging accumulates to the final release and walks LIFO.
Plain (non-TMS) del+ins churn dips the count in BOTH engines at
the delete's arrival — p1 matches — so the fork is EXACTLY the
supersede path.

x52's chain is now fully decoded: windows/clock/events only
drove W2's refires (the churn); every other ingredient was
scaffolding. MINIMAL WITNESS: p3_cloud_churn3 (2 rules, cloud,
3 Ps, 3 K2-update epochs).

Scorecard round 3: m4 oracle prediction MISSED (recorded — it
killed the stream-flush hypothesis and forced the real law);
p3 BOTH predictions exact; p1 control exact.

THE PORT (next slab, gated): reproduce the transient relink
without disturbing the D-076 epilogue's certified SEMANTICS.
Open question the port round must probe first: in Drools, does
the superseded belief die at match-update even when the refire
NEVER happens (halt / salience-starved refire)? — that decides
whether the port is "epilogue del + synthetic relink pulse" or a
true match-update-time retraction. Blast surface: tms_envelope
(SD census), the D-330 park lanes, nl7/p1 (must stay green).

## D-335 probe round: SUPERSEDE TIMING (Bryan: "do the supersede-
## timing probe round")

SOURCE FIRST (drools-tms 9.44.0.Final): the transient-relink law
is TEXTUALLY UNSUPPORTED — PhreakRuleTerminalNode.doLeftTupleUpdate
makes NO TMS call (an updated fired match just re-queues);
removeLogicalDependencies rides cancelActivation (match DELETE
only); and TruthMaintenanceSystemKnowledgeHelper is prologue-
snapshot (setActivation clears deps) + insertLogical re-justify +
reset() cancelRemainingPreviousLogicalDependencies — the SAME
ins-then-del refire shape as Seine's D-076 epilogue. The not's
right count never dips textually. Yet p3 forks. The sharpest
unexplained delta p3-vs-m4: p3's churn epochs CONTAIN FIRINGS
(W refires); m4's epochs are fire-free.

### m4b prediction (REGISTERED BEFORE the cell runs)

m4b = m4 + a trivial G-rule firing per churn epoch (no TMS, no
churn — B(true) sits untouched until the final delete):
- If the oracle's per-epoch drain is FIRING-DRIVEN (something in
  the fire loop drains staged join inserts of unlinked-blocked
  paths): m4b FORKS — oracle R(4) R(2) R(1) (insertion-ordered)
  vs engine R(2) R(4) R(1) (accumulation).
- If the drain is TMS-supersede-specific: m4b MATCHES like m4
  (both accumulate, R(2) R(4) R(1)).

### The TmsProbe measurement (standalone listener probe, oracle
### classpath, p3's exact shape)

Event stream: the supersede flush order is **WM-INS B2(new) THEN
WM-DEL B2(old)** — ins-then-del,同 Seine's epilogue. THE
TRANSIENT-RELINK LAW IS DEAD (recorded miss — the counter never
dips mid-churn). The MATCH+ stream: R's matches are created ONLY
at e3, in order [P(4), P(2), P(1)] = (with the trg-prepend +
terminal-walk flip) join emission [Pa, Pb, Pc] = INSERTION-
ORDERED rtm = R's network EVALUATED EVERY CHURN EPOCH.

### THE LAW (round verdict candidate): not-right DEL dirty-notify

One rule fits all eight measurements: **a staged right-DELETE
arriving at a not node dirty-notifies the owning rule's item (a
release-check) — queueing its lazy evaluation even while the
path is blocked/unlinked.** p3: a del per epoch (the supersede's)
=> per-epoch drain => insertion-ordered rtm. nl6/m4/m4b: no dels
until the end => accumulation. p1: plain-churn dels per epoch =>
both engines drain (Seine's counter hits 0 there — relink-at-
zero coincides). Seine's fork = precisely the NON-ZERO del
(2->1: no relink, no queue, accumulation).

### p5 prediction (REGISTERED BEFORE the cell runs) — the plain
### multi-blocker seal

p5_stagger_teardown: THREE blockers B(5),B(6),B(7); `not
B(n >= 1)`; one deleted per epoch via K5(m)-triggered rule
(counter 3->2->1->0 — never dips to 0 until the END). Ps arrive
per epoch. If the del-notify law holds:
- ORACLE: per-epoch drain => rtm [Pa,Pb,Pc] => release
  consumption [Pc,Pb,Pa] = R(4) R(2) R(1).
- ENGINE (relink only at 0): accumulation => staged LIFO at the
  final eval => consumption [Pb,Pc,Pa] = R(2) R(4) R(1). FORK —
  and the class is then PLAIN (wider than TMS): any staggered
  multi-blocker teardown over a leading not.

### p5 measured — ORACLE PREDICTION MISSED (recorded): staggered
### plain teardown ACCUMULATES ([2,4,1] both sides). Dels do NOT
### wake a blocked not (the del-notify law is dead too). NotNode
### source: BOTH assertObject and doDeleteRightTuple call
### setNodeDirty on a fresh staging batch — so notifies must be
### linkage-gated, and the p3 wake needs another asymmetry.

### p6 prediction (REGISTERED BEFORE the cell runs) — plain
### no-dip churn (ins present, counter never 0 mid-run)

p6_nodip_churn: three blockers; DD deletes one AND inserts a
replacement per epoch (counter 3->2->3: no dip, fresh INS at the
not each epoch); final epoch DDA tears all down (the last del
dips -> release).
- If a not-right INSERT wakes a blocked-unlinked rule (ins-vs-del
  asymmetry): ORACLE drains per epoch => R(4) R(2) R(1); ENGINE
  accumulates => R(2) R(4) R(1). FORK, and the class is plain.
- If not: both accumulate ([2,4,1]) and the p3 wake is
  TMS-belief-machinery-specific.

### p6 measured — both sides accumulate ([2,4,1]): a plain
### not-right INSERT does not wake a blocked-unlinked rule either
### (the "if not" prediction arm hit).

### D-335 ROUND VERDICT (the supersede-timing probe round)

1. THE ORIGINAL QUESTION IS ANSWERED TEXTUALLY AND MOOT FOR THE
   FORK: TruthMaintenanceSystemKnowledgeHelper is prologue-
   snapshot (setActivation clears deps) + insertLogical
   re-justify + reset() removes non-re-established deps — the
   superseded belief dies ONLY via a completed refire (or match
   DELETE via cancelActivation). No refire => no death — the
   SAME semantics as Seine's D-076 epilogue. The TmsProbe
   listener measurement confirms the flush order: WM-INS
   B2(new) THEN WM-DEL B2(old). Seine's epilogue is CORRECT;
   both transient-dip laws are dead (recorded).
2. THE REAL DIVERGENCE IS A WAKE: measured matrix —
   p1 (plain churn, count dips 0): BOTH drain (Seine's
     relink-at-zero coincides).
   p5 (plain dels, no dip): BOTH accumulate.
   p6 (plain ins+del churn, no dip): BOTH accumulate.
   p3 (TMS supersede churn, no dip): ORACLE DRAINS PER EPOCH,
     engine accumulates — THE FORK.
   By elimination: **a TMS belief-supersede churn epoch wakes the
   blocked not's owning rule (its lazy evaluation runs at that
   epoch's pop, draining ALL accumulated staging); no plain
   composition does**. p5's oracle order also re-confirms the
   LIFO staged walk at ordinary relink evaluations, so p3's
   insertion-ordered memory can ONLY be per-epoch draining.
3. X52 COMPOSES EXACTLY under the wake law: per-epoch drains
   through the churn epochs + the P0 reorder at e2 + the final
   release => [2,3,2]; the engine's accumulation => [2,2,3].
4. THE PORT (named, its own gated slab): when a TMS supersede's
   belief churn touches a not-right (the stale-key retraction
   and/or the new belief's insert reaching a not node), queue
   the owning rule's item dirty — the evaluation stays lazy
   (at its pop), only the WAKE is added. Blast surface:
   tms_envelope (SD census 71), the D-330 park lanes, nl7/p1/p5/
   p6 (must stay green). The exact Drools wake SITE remains
   unpinned (candidate: the TMS belief ops' entry-point route
   notifying independent of node linkage) — the port emulates
   the BEHAVIOR; the byte gate and battery judge.

Scorecard D-335: p5 oracle prediction MISSED (recorded — killed
del-notify), p6 "if not" arm HIT, TmsProbe = the decisive
instrument (MATCH+ stream + WM event order).

## D-336: THE WAKE PORT (Bryan: "do the wake port")

### Scope probes p7/p8 (predictions REGISTERED before the cells run)

The wake could ride the TMS INSERT, the TMS DELETE, either, or
only the supersede PAIR. p7 = pure TMS ins churn (fresh justifier
K6 per epoch, distinct beliefs, no dels until the final
teardown); p8 = pure TMS del churn (three justifiers up front,
one killed per epoch via its K6's delete -> match death ->
belief retract; no new ins).
- PREDICT (uncertainty recorded; my guess = the TMS belief ops'
  entry-point route notifies on BOTH): p7 oracle DRAINS =>
  R(4) R(2) R(1); p8 oracle DRAINS => R(4) R(2) R(1); engine
  accumulates on both => R(2) R(4) R(1).
- The four outcomes map: both drain = either-op wake; p7-only =
  ins-wake; p8-only = del-wake; neither = pair-only.

### D-336 measured + LANDED

p7/p8 measured: BOTH accumulate on both sides (prediction missed,
recorded) — THE WAKE IS PAIR-ONLY: one firing retracting a stale
belief key while establishing a new one. The port: in
execute_rhs's refire-supersede epilogue, when (new key this
firing) AND (stale retraction happened), every rule with a `not`
position admitting either fact (type + alpha, checked pre-kill)
gets queued dirty — the WAKE only; evaluation stays lazy at the
item's pop (the exists-requeue precedent's flag shape).

MEASURED: p3 AND cf325901x52 flip PASS; all controls hold
(p1/p5/p6/p7/p8/nl7). Byte gate vs 05fd74a: 2475/2477 SAME, 2
diff = EXACTLY the two intended movers, 0 moved — the
tms_envelope, park, and error-parity lanes byte-identical.
SEVENTEEN graduations (pr_nl_*: cf325901x52 + nl1-8 + m4/m4b +
p1/p3/p5/p6/p7/p8); bank 19->18->19 (one fresh pre-existing
quarantine fz_336002_968: collect x acc x insertLogical latent,
worktree-bisected byte-identical pre/post wake, seed re-run
clean). Corpus 11/1503/414 + drift 19; lint 2351/0/0; SD census
71 cell-for-cell EXACT; cargo 74; pytest 257; demo True;
model_ird 31/31; IRD 0-div x5 (engine 0-div); agenda_open x10
identical x3; fuzz 336001 clean + cep 3x300 clean.

THE cf325901x52 LEDGER ITEM CLOSES: unexplained cep blob ->
not-lead law (D-334) -> supersede-wake law (D-335) -> ported and
certified (D-336).

## D-337: fz_327002_1948 — the STARVED deferred teardown (the
## phantom-beliefs witness closes)

THE DECODE: R2 (no-loop, agenda-group "ga", or-triple `not
T1(f1==true)`) fires 3x justifying two beliefs; epoch 2's foreign
T1(f1=true) inserts close the not — the eager (no-loop) flush
evaluates R2 and its three matches die, but the TMS terminal-dels
DEFER-PUSH (flags=0) onto tms.deferred, and the ONLY drain for
that list is the item's POP — which the agenda-group filter
forever denies ("ga" is never refocused). The entries starve; the
beliefs linger as phantoms. Drools' unjustify lands AT the eager
evaluation (removeLogicalDependencies in doLeftDelete,
group-independent).

THE FIX (one block): an AGENDA-QUIESCENCE drain of tms.deferred,
placed after the exp_deferred bare-drain. MAIN/focused items with
deferred entries always reach their pop first (any deferred entry
makes the item reachable), so ONLY the group-starved class
survives to quiescence — the certified t20/D-196/D-199/D-201
drain timing is untouched by construction; the rescan orders
drain-activated observers by salience (the quiescence-exp
precedent).

MEASURED: fz_327002_1948 flips PASS (graduated
pr_nl_fz_327002_1948; bank 19->18). Byte gate vs 96357b1:
2478/2480 SAME, 2 diff — the witness + ONE deliberate
oracle-CLOSER mover (agenda_open/fz_9004_214: a documented-open
recon witness whose engine under-fired 8-vs-9; the starved drain
lands the missing retraction, final FACTS now equal the oracle,
tail order R2/R1 still swapped = the quiescence-vs-eager-eval
timing approximation, recorded open). t20 pins green. Corpus
11/1504/414 + drift; lint 2351; SD census 71 EXACT; cargo 74;
pytest 257; demo True; model_ird 31/31; IRD 0-div x5; agenda_open
x9 identical + the deliberate mover; fuzz 337001 clean, 337002 ->
1 PRE-EXISTING quarantined (fz_337002_1104, worktree-bisected,
seed re-run clean; bank 19); cep 3x300 clean.

## D-370 candidate: fz_337002_1104 — the LAZY-SEGMENT unjustify law
## (the D-337 drain's complementary witness)

THE DECODE (all 26 firings IDENTICAL both sides; the fork is
FINAL FACTS only — the oracle KEEPS two R4-justified T3 beliefs):
Drools' logical unjustification for a dead justifier match lands
when the justifying rule's SEGMENT NEXT EVALUATES (doLeftDelete →
removeLogicalDependencies); if that evaluation never comes — the
rule's agenda group starved to session end — the belief SURVIVES.
The witness contains its own control: the BASE-round beliefs died
oracle-side because the epoch's setFocus("gb") re-evaluated the
segment (processing the earlier staged deletes); the EPOCH-round
beliefs, with no later refocus, survive. D-337's fz_327002_1948
is the complementary cell: there the no-loop EAGER FLUSH evaluated
the segment (Drools unjustified) and only the engine's TMS-effect
deferral starved — the quiescence drain fixed it. The drain is
thus an APPROXIMATION that over-fires on the never-evaluated
class: the engine unjustifies at quiescence what Drools never
unjustifies at all. Minimized m1104 = 3 rules (R3 setFocus, R4
gb/dyn-salience insertLogical, R1 sal-5 delete-all-T2), 2 facts,
NO epochs.

### GRID (predictions registered 2026-07-20 BEFORE cells run;
### engine kills the belief in every cell — oracle-KEEPS = DIVERGE)

- **g1_anchor** — m1104 verbatim. PREDICT DIVERGE (oracle keeps
  T3; the witness's class, 3× stability check rides here).
- **g0_static_sal** — R4 with `salience 1` instead of dynamic.
  PREDICT DIVERGE (dynamic salience not load-bearing).
- **g2_late_refocus** — + R5 (MAIN, salience -10): T1() →
  setFocus("gb"), firing AFTER R1's deletes. The refocus
  evaluates R4's segment → unjustify. PREDICT MATCH (no T3
  either side) — the witness-internal control, isolated.
- **g3_main_group** — R4 with no agenda-group. The dead match is
  pop-reachable in MAIN → unjustify. PREDICT MATCH.
- **g4_delete_in_gb** — R1 moved INTO "gb" (salience -5). The
  deletes land while gb is focused; the segment evaluates in
  focus. PREDICT MATCH.
- **g5_eager_flush** — + epoch external MODIFY of the T1
  (value-identical) after the deletes: D-353 — an external modify
  flushes its beta segment AT CALL, group-independent → the
  evaluation runs → unjustify. PREDICT MATCH (the 1948-boundary
  pin: eager-flush evaluations DO unjustify).
- **g6_two_groups** — R4 in "gb" + R4b (same shape, belief
  T3(true,...)) in "gc"; a post-delete refocus of gb ONLY.
  PREDICT DIVERGE with EXACTLY the gc belief surviving
  oracle-side (per-group split — the law's strongest cell).

### ROUND-1 MEASUREMENTS + THE MECHANISM SHARPENS (LINKING)

- g1 DIVERGE ✓ (anchor; oracle keeps T3). g2/g4 MATCH ✓ (refocus
  and in-focus deletes unjustify). g6 DIVERGE ✓ EXACTLY as
  predicted (per-group split: only the un-refocused gc belief
  survives). g3 = cell bug (setFocus at an undeclared group,
  error-parity walls) — REPLACED by the g7/g8 pair below.
- **g0_static_sal PASS — prediction MISS, the reframe**: with
  STATIC salience the belief survives on BOTH sides. The engine
  is already lazy here (no evaluation → no terminal-del → no
  deferred entry). The defect is dyn-salience-specific.
- **g5_eager_flush DIVERGE — prediction MISS**: an external
  value-identical modify does NOT evaluate the starved segment
  oracle-side (the D-353 flush law does not extend here).
- SEINE_TMS_DEBUG on g1: R4's terminal-del processes INLINE with
  deferred=0 — the kill site is eager_flush's dyn-salience
  evaluation (evaluate_rule eager + dyn_sal → TMS dels inline,
  the fz_999_3020 pin), which the engine runs UNCONDITIONALLY.
  Drools' evaluateEagerList only evaluates LINKED networks —
  deleting the LAST T2 UNLINKS R4's segment, so the staged
  delete never processes (doLeftDelete never runs). The witness's
  base round is the internal control: the epoch's fresh T2s
  RELINKED the segment, so the refocus evaluation processed the
  earlier staged deletes and those beliefs died.

REVISED LAW: logical unjustification lands when the justifying
rule's network next EVALUATES; eager-listed rules (no-loop /
dyn-salience) evaluate at every firing boundary ONLY WHILE
LINKED; an unlink (positive input emptied) freezes the staged
deletes — beliefs survive unless a relink + evaluation later
processes them.

### Round-2 boundary cells (predictions BEFORE run)

- **g7_partial_delete** — two T2s (f3 false/true), R1 deletes
  only f3==false. Segment STAYS LINKED → the eager evaluation
  processes the dead match. PREDICT MATCH: T3(...,false) dies,
  T3(...,true) survives, BOTH sides.
- **g8_relink_no_refocus** — full delete, then a late epoch
  inserts a fresh T2 (NO refocus). Relink → the next firing
  boundary's eager evaluation processes the old staged deletes.
  PREDICT MATCH: the old belief dies both sides (no new belief —
  gb never refocuses, the new activation never fires).

### Round-2 measurements + round 3 (predictions BEFORE g3fix/g9 run)

- g7 MATCH ✓ (linked-starved evaluates both sides).
- g8 FAIL — the OLD belief died BOTH sides (the relink prediction
  core HELD); the miss was cell design: R3 re-fired on the fresh
  T2, refocused gb, and R4 minted a NEW belief whose T2 then died
  post-unlink — a fresh instance of the anchor class (oracle
  keeps T3(...,true)). The law composes; recorded as measured.
- **g3fix_main_dynsal** (replaces the errored g3): R1 + R4 in
  MAIN with DYNAMIC salience, no groups at all. If the law is
  pure LINKING, the unlink freezes the staged delete regardless
  of agenda groups. PREDICT DIVERGE pre-port (oracle keeps,
  engine's unconditional eager flush kills) — the
  groups-are-scaffolding pin.
- **g9_main_static** — same with STATIC salience: not
  eager-listed either side. PREDICT MATCH (both keep).

### Round-3 measurements + the completed law + round 4 (predictions first)

- g3fix PASS (both KILL) + g9 PASS (both KILL) — prediction MISS
  that COMPLETES the law: MAIN rules' dead matches ALWAYS
  evaluate before session end (pop/quiescence reach), static or
  dynamic. The freezer is GROUP STARVATION; linking gates only
  the eager-list route INTO a starved group:
    MAIN                        -> unjustify (g3fix/g9)
    group refocused / in-focus  -> unjustify (g2/g4/witness base)
    starved + dyn-sal + LINKED  -> unjustify (g7; eager list
                                   evaluates linked networks,
                                   group-independent)
    starved + dyn-sal + UNLINKED-> FROZEN, belief survives
                                   (g1/witness/g8-new)
    starved + static + UNLINKED -> FROZEN both (g0; engine
                                   already faithful)
  Engine's sole defect: the eager flush evaluates UNLINKED
  networks (Drools' evaluateEagerList does not).
- **g10_static_linked_starved**: R4 static in gb, partial delete
  (stays linked), no refocus. Nothing evaluates (not eager-
  listed, group starved). PREDICT MATCH — the dead-match belief
  SURVIVES both sides. (Confirms static+linked is already
  faithful; fences the port from touching it.)
- **g11_noloop_unlinked_starved**: R4 no-loop STATIC in gb, full
  delete, no refocus. No-loop shares the eager list; if the
  linked gate is the list's (not dyn-sal's), the staged delete is
  FROZEN oracle-side. PREDICT oracle KEEPS; engine = measure
  (its no-loop eager evaluation defers the terminal-del into
  tms.deferred, whose starved class the D-337 quiescence drain
  kills → likely DIVERGE pre-port).

### D-370 CLOSE-OUT (2026-07-20, the port landed)

Round-4: g10 MATCH ✓ (static+linked+starved survives both — the
port must not and does not touch it); g11 DIVERGE ✓ (no-loop
shares the eager list's linked gate).

THE PORT (eager_flush, 3 gate sites + one new predicate): an
eager-listed rule's flush evaluation is SKIPPED iff the rule is
NOT rule_eager_linked AND its agenda group is UNREACHABLE (not
MAIN, not on the focus stack). rule_eager_linked = positive-input
(segment) linking — Positive/Exists need non-empty inputs, a NOT
never unlinks (rule_linked's not-arm is the D-084/D-091
agenda-reachability concept, a DIFFERENT thing). TWO byte-gate
iterations, both recorded:
1. gate on rule_linked — BROKE fz_327002_1948 + both t20 pins
   (not-closed shapes read as unlinked; Drools evaluates them).
2. + rule_eager_linked — 1948 restored; the t20 SELF-DEFEAT pins
   still broke: MAIN rules whose self-break EMPTIES their own
   positive input are pop-reachable — Drools prunes at the pop,
   the flush is the engine's site for that observable — hence
   the group-reachability exemption.
FINAL byte gate 2677 files = EXACTLY the 7 intended movers
(g1/g5/g6/g8/g11/m1104/fz_337002_1104); the three pins
byte-identical again.

RECEIPTS: FOURTEEN graduations (pr_nl_fz_337002_1104 from xfail
+ pr_nl_m1104 + pr_nl_g0..g11 incl. g3fix); rebank 9 -> 8; make
diff 11/1667/414 + drift 8 identical (one fz_123_6887
parallel-load flap, sequential PASS per the ruling); lint
2572/0/0; cargo 74; pytest 260; demo True; model_ird 31/31 +
witnesses 26/26 + cells 39/39; IRD 0-div x5; SD 71 EXACT
cell-for-cell; agenda_open x10 identical x3 binaries
(debug/release/pre-edit — the sensitive lane this round) +
reruns; fresh fuzz 2x2000 seeds 361001/361002 CLEAN (0 xfail
draws) + cep 3x300 361901-903 CLEAN; NEXT seeds 362001+.
CHANGELOG Unreleased +1 (now 3). Bank 8 = 6 ruled/walled +
fz_336002_968 (Bryan's last actionable pick) + fz_360001_381
(the un-triaged count fork).

### D-370 ADDENDUM: the EXTERNAL-DELETE axis (2026-07-21, the
### tour-QA question routed back)

The second-machine QA sweep built a starved-group belief-survival
check API-side and saw RETRACTION — reported honestly as "my
construction doesn't hit the trigger; what does?". The g-grid had
only RHS-delete cells; the external-op landing was unprobed.
Measured (pr_nl_g12_extdel_starved, oracle diff PASS 3x): an
EXTERNAL delete of the last justifying premise freezes exactly
like the g0/g1 RHS cells — static+starved+unlinked KEEPS the
belief on both sides; the Python binding's synchronous
delete-cascade correctly excludes it (cascade []). API flip
matrix, same session: no_loop+unlinked+starved survives (g11
arm); linked+static+starved keeps both beliefs (g10 arm); a
FRESH PREMISE whose insert re-activates the setFocus kick
REFOCUSES the group and drains the old staged delete — old
belief dies, new one minted (g2/g8 arm). That last shape is the
one that masquerades as "D-370 not reproduced": any fire that
re-grants focus (or any still-matchable kick rule) makes the
group reachable and the law lands the unjustify. The QA guess
("the starved group can't fire to CORRECT the stale belief")
was wrong in a load-bearing way: survival is frozen staged
DELETES (unjustification never runs), not un-fired maintenance.
Corpus 1680; lint 2599.
## (the D-337-receipts "quiescence-vs-eager-eval" open item's cell)

THE DECODE (build-up firings all MATCH; the fork is ONE extra R0
firing): R4 deletes the lone T0 → all four logical T1s must
retract. The DIRECT join-death beliefs (R3/R5's) land in wave 1;
the ACC-path belief T1(7) (R1 no-loop: average(T0) → NULL → match
death) lands engine-side at R1's ITEM POP — after R0's pop — so
R0 fires an intermediate C[7] the oracle never shows (oracle: the
whole cascade lands before R0's pop; count 15 vs 14). R1 is
no-loop = EAGER-LISTED: its flush evaluation runs at the
post-R4-firing boundary BOTH sides (MAIN, group-reachable — the
D-370 gate does not apply); the difference is the TMS effect's
landing: the engine's no-loop eager evaluations DEFER terminal
dels (defer_mode, min3783/t20 calibration) and the deferred
entry's flags don't qualify for the flush drains → it rides to
the pop. Drools lands the removal AT the evaluation.

### GRID (predictions registered 2026-07-20 BEFORE cells run)

- **m968 anchor** — minimized (R0 collect + R1 no-loop acc + R4
  delete + R5 or-justifier, 1 fact). DIVERGE (measured shape).
- **d2_no_noloop** — R1 without no-loop: not eager-listed; the
  acc re-derivation waits for R1's own pop on BOTH sides; R0
  (declared first, equal salience) pops first and sees the
  intermediate. PREDICT MATCH — oracle fires C[7] then C[] too
  (the intermediate becomes CORRECT).
- **d3_plain_justifier** — R5 as a plain (non-or) rule. PREDICT
  DIVERGE unchanged (or-ness inert).
- **d4_dynsal** — R1 with dynamic salience instead of no-loop
  (still eager-listed; the engine's dyn-sal eager evaluations
  process TMS dels INLINE — fz_999_3020). PREDICT MATCH (both
  coalesce) — isolates the engine's no-loop-defers calibration
  as the sole defect surface.
- **d5_acc_only** — drop R5 (no wave-1 belief). PREDICT MATCH
  (no intermediate state exists).

### GRID MEASUREMENTS — 5/5 HITS — + round 2 (predictions first)

m968 DIVERGE ✓; d2_no_noloop MATCH ✓ (the intermediate C[7] is
CORRECT without no-loop — eager-list membership IS the
discriminator); d3 DIVERGE ✓ (or-ness inert); d4_dynsal MATCH ✓
(dyn-sal eager evals process TMS inline — both coalesce);
d5_acc_only MATCH ✓. Flags decode: the R1 entry is
FOREIGN-ORIGIN (R4's delete) → flags=0 → no flush-drain arm
matches for non-dyn rules → rides to the pop. The D-196/199/201
lattice is all OWN-ORIGIN (self-defeat) lanes — this is a NEW
lane, not a re-calibration of those.

- **d6_noloop_join** — R1 as a no-loop PLAIN JOIN justifier
  (T0() => insertLogical), foreign R4 delete, R5 wave-1 belief,
  R0 observer. Does the oracle coalesce here too (retract at the
  eager flush — doLeftDelete is unconditional in Drools) or show
  the intermediate (pop-landed)? PREDICT coalesce → DIVERGE
  (med — if MATCH instead, the law is ACC-specific and the fix
  narrows to acc-carrying rules).
- **d7_static_join** — same, R1 static (not eager-listed).
  PREDICT MATCH (intermediate correct both sides — the d2
  analog).

### Round-2 measurements — the asymmetry decodes

- d7 MATCH ✓ (static join control).
- **d6 MATCH — prediction MISS that completes the mechanism**:
  the no-loop JOIN-path foreign death coalesces on BOTH sides
  already. Join deaths land INLINE at the delete propagation
  (outside evaluate_rule — defer_mode off); only the ACC
  RE-DERIVATION death is evaluation-internal, so it alone gets
  swept into the eager evaluation's defer_mode and rides
  flags=0 (foreign-origin — no self-touch bits) to the pop.
  Drools lands it at the evaluation (doLeftDelete).

THE PORT (one drain-arm): tms_flush_drain admits flags==0
(foreign-origin) entries at the flush sites — the entry still
defers DURING the evaluation (in-eval staging order preserved)
but lands before the next pop. The OWN-ORIGIN zombie lattice
(D-196/199/201, flags!=0) is untouched by construction.

### D-371 CLOSE-OUT (2026-07-20, the port landed)

THREE byte-gate iterations, all recorded:
1. arm `*fl == 0` — missed (the entry carries the LATE bit:
   flags=4; SEINE_TMS_DEBUG pinned push + pop-drain).
2. arm `(*fl & 11) == 0` — fixed the witness but BROKE four
   cells: min3783 (fz_7_3783, the no-loop-defers calibration
   graduate), pr_tms_k2lazy, fz_9004_214, fz_9105_5693 — pure-
   late non-acc lanes' pop landing is load-bearing.
3. FINAL: `(*fl & 11) == 0 && rule has an acc pattern` — the acc
   re-derivation death is the only foreign class born inside the
   evaluation; all four cells restored BYTE-IDENTICAL, final
   gate 2697 = EXACTLY the 3 intended movers (m968/
   d3_plain_justifier/fz_336002_968). fz_9004_214's tail-order
   residual stays recorded-open (its rule is not acc-carrying —
   the improvement seen under arm 2 did not survive the
   narrowing; still the quiescence-vs-eager-eval item).

RECEIPTS: EIGHT graduations (pr_nl_fz_336002_968 from xfail +
pr_nl_m968 + pr_nl_d2..d7); rebank 8 -> 7; make diff 11/1675/414
+ drift 7 identical (one fz_123_6887 parallel-load flap,
sequential PASS per the ruling); lint 2587/0/0; cargo 74; pytest
260; demo True; model_ird 31/31 + witnesses 26/26 + cells 39/39;
IRD 0-div x5; SD 71 EXACT cell-for-cell; agenda_open x10
identical x3 binaries + reruns; fresh fuzz 2x2000 seeds
362001/362002 CLEAN (0 xfail draws) + cep 3x300 362901-903
CLEAN; NEXT seeds 363001+. CHANGELOG Unreleased +1 (now 4).
Bank 7 = 6 ruled/walled + fz_360001_381 (the un-triaged count
fork) — BRYAN'S ACTIONABLE TRIAGE LIST IS EMPTY.

## D-372 candidate: fz_360001_381 — the epoch acc-rebirth skip

m381 (3 rules, 1 fact, 1 epoch action-insert): initial round =
R4 (insertLogical T1), R3 fires max=7, R5 deletes the T0 (T1
retracts, max -> NULL, the acc-result tuple DIES). Epoch: a new
T0 -> R4 -> T1(11) -> max reborn at 11. ENGINE fires R3(11) at
its salience slot (-3, before R5's -10); ORACLE NEVER RE-FIRES
R3 — despite firing it for the identical shape in the initial
round. Firing-set fork (engine +2 in the full witness), final
facts equal. Candidates: the acc NULL-GAP rebirth (tuple death +
re-assert suppressed), the not(and) subnet, path linking.

### GRID (predictions registered 2026-07-20 BEFORE cells run)

- **p1_no_not** — R3 without the not(and) (acc only). If the
  oracle STILL skips the epoch firing -> the acc rebirth is the
  actor. PREDICT oracle skips -> DIVERGE persists (med-low —
  genuinely uncertain).
- **p3_no_nullgap** — a permanent stated T1(f0=5) so max never
  NULLs (7 -> 5 -> 11; the result tuple never dies). PREDICT
  oracle FIRES the epoch R3s -> MATCH (med) — isolates the
  null-gap/tuple-death as the actor.
- **p5_epoch_fact** — the epoch insert as an epoch FACT instead
  of an action insert. PREDICT same as m381 (DIVERGE — the
  insert路径 inert) (med).

### THE GRAFT FINDING (AgendaDump, D-086 idiom) + round 2

p-grid round 1: p1_no_not PASS (both FIRE — single-segment path
notifies), p3_no_nullgap PASS (both fire; mid-population deltas
7->5 notify normally), p5_epoch_fact FAIL (insert route inert).
AgendaDump on m381: the epoch T1(11) insert reaches the acc but
R3's RuleAgendaItem NEVER RE-QUEUES (queued=false dirty=false
throughout; lmask=3/3 CONSTANT — not a linking loss). MECHANISM
READ: the acc source's EMPTY<->NON-EMPTY transitions ride the
link/unlink notification path — a no-op for the always-linked
acc segment — so the dirty notification is SWALLOWED; the rule
goes DEAF to the repopulation until another source notifies.
The subnet-not's role = the SEGMENT SPLIT (seg1[IF+acc]
seg2[not+RTN]); without it the RTN shares the acc's segment and
the notification lands (p1).

Round-2 boundary cells (predictions BEFORE run):
- **p7_plain_follower** — acc followed by a PLAIN populated
  pattern (T2(), one stated T2 fact) instead of the not. Plain
  patterns do NOT split segments. PREDICT MATCH (both fire).
- **p8_bare_not_follower** — acc followed by a bare `not T2()`
  (no subnet; T2 never populated). Bare nots don't split
  segments either. PREDICT MATCH (both fire).
- **p9_inert_subnet** — acc followed by not(T2() and T2(f0==5))
  with T2 NEVER populated (the subnet inert as data, present as
  STRUCTURE). The deafness is about the acc's segment position.
  PREDICT DIVERGE (oracle deaf) — if so, the law = "empty->
  non-empty acc-source transitions do not re-queue a rule whose
  acc sits in a non-terminal (subnet-split) segment".

### Round-2 measurements + upstream check + DISPOSITION (Bryan's gate)

p7 MATCH ✓, p8 MATCH ✓, p9 DIVERGE ✓ — 3/3 predicted. THE LAW:
an empty->non-empty acc-source transition does NOT re-queue a
rule whose accumulate sits in a subnet-split segment; the rule
is DEAF to the repopulation. A DATA-INERT subnet reproduces it
(p9), so `not T2()` vs `not(T2() and T2(f0==5))` with T2 never
populated — semantically equivalent — FORK the firing set: the
D-092/D-114 SELF-INCONSISTENCY signature. UPSTREAM (throwaway
10.1.0 oracle, the D-263 scratch-pom pattern, 3x stable): m381
and p9 firing sets IDENTICAL to 9.44 — the defect is LIVE in
the current Drools line. The engine fires the reborn activation
at its salience-correct slot (arguably MORE correct).

NO ENGINE CHANGE this round (triage + law + graft only). NEW
PERMANENT GRAFT: oracle/.../AgendaDump.java (AccDump + match
lifecycle events + the epoch action-insert op). FOUR law-pin
graduations: pr_nl_381p1/381p3/381p7/381p8 (the notification
boundary, certified both sides). The witness fz_360001_381
STAYS BANKED with the full finding (bank 7, count unchanged).
Lane cells m381/p5/p9 = diverging recon records.

DISPOSITION = BRYAN'S GATE: (a) WALL — correct-in-engine +
expected-divergence witness + upstream report (the axis-2
doctrine; p9 is the minimal filing repro); porting the deafness
means modeling a permanent-deafness bug incl. its wake/backlog
semantics — the stop-rule shape; or (b) PORT the deafness
(axis-1 quirks-included).

### D-372 addendum — the upstream draft (Bryan: "agreed. please
### draft upstream bug report")

WALL disposition confirmed by Bryan. Draft written:
docs/drools-bug-acc-requeue-deafness.md (the drools-bug-stale-minmax
format). Reproducer HARDENED to filing grade: a single
self-contained KieHelper class, TWO variants in one run — A
(subnet not) loses the max=11 firing, B (bare not, semantically
identical) fires — verified on VANILLA 9.44.0.Final (classpath
only, no vendored shadow) and 10.1.0, identical output. TWO new
boundary facts measured for the report: (1) an ALPHA-EXCLUDED
sentinel does NOT prevent the deafness — the emptiness that
matters is the acc node's beta (right) memory, not type
population; (2) an IN-AGGREGATION sentinel restores the firing
(with sentinel-value side firings) — the verified workaround.
Filing target: github.com/apache/incubator-kie-issues (the #2366
precedent). Not yet filed — Bryan files or gives the word.
