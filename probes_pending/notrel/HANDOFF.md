# HANDOFF — the not-release order round, part 2: SOURCE + MODEL
# (cold start; D-331 was part 1, commit f50989a)
# >>> COMPLETED as D-333 (2026-07-19): the law was Phreak LAZINESS
# (haltRuleFiring's no-eval break + staged-ins annihilation), not a
# list direction. All four would-graduates + fz_327002_845 graduated
# (pr_nr_*); see PINS.md rounds 3+ and tools/model_check_notrel.py.

Written 2026-07-19. Bryan's directive: read the Drools source
for the not-node machinery, MODEL it (D-083 style), then port.
Part 1 (D-331, probes_pending/notrel/PINS.md — read it FIRST)
pinned the consumption law and the engine-side mechanism but
stopped at a six-order composition on certified surface.

## THE TARGET

Would-graduate on the modeled port: fz_7_2364 + fz_min_7_2364
(the banked witness pair), fz_7_9360, nb3 (scenarios/xfail/).
The COUNTER-SET that any port must keep byte-green (the naive
flip broke all 11 — measured, reverted): scenarios/probes/
nb1, nb2, nb6, pr_ne_n4; scenarios/regressions/ fz_123_3370,
fz_27182_1227, fz_42_5213, fz_42_7768, fz_7_9864, fz_min_7768,
fz_min_999_8145. Plus 13 probes_pending/tms_envelope cells
that moved bytes (gt11/16/19/20b/3/5/7/8, sd_c1/c3a/c3c/d2/d4)
— the SD census (72 EXACT ×12) is their gate.

## WHAT IS ALREADY MEASURED (do not re-derive)

- The consumption LAW (r1/r2 predictions hit EXACTLY): a
  release consumes its unblocked lefts NEWEST-INSERTED-FIRST
  (LIFO) — in BOTH the LIA-direct (nb1: [3,2,1]) and join-fed
  (2364 relay) shapes.
- The engine serves both through ONE path and gets only the
  join-fed shape wrong. Instrumented traces (in PINS.md):
  the not node's left-MEMORY order is upstream-dependent —
  LIA-direct arrives REVERSED (memory [2,1,0] → correct);
  join-fed arrives FORWARD (memory [0,1,3] → wrong). The
  block walk follows memory order; blocked lists prepend
  (insert(0), all 9 sites); the release iterates list order;
  child_ins ph=2 staging flips once; queue pick is FIFO
  (push_back/removeFirst, static salience).
- r3 (fresh release, LIA-direct) and r4 (same-type dual-role
  one-shot, join-shaped!) are ALREADY CORRECT engine-side —
  r4 means the join-fed orientation is not WRONG universally:
  the relay's MULTI-ROUND re-add path is implicated too
  ("re-add the unblocked ones at the END" in release order).

## THE SOURCE PLAN (step 1 — ground truth before the model)

Sources: ~/.m2/repository/org/drools/drools-core/9.44.0.Final/
drools-core-9.44.0.Final-sources.jar (extraction dir from this
session: /home/bryan/.claude/jobs/557e9581/tmp/drl-src/ —
`unzip -o -q <jar> "<glob>" -d .`). Read, in order:
1. org/drools/core/phreak/PhreakNotNode.java — doRightDeletes
   (the release walk: blocked-list iteration order + where
   released lefts stage), doRightInserts (the block walk:
   left-memory iteration + blocked-list build order),
   doLeftInserts (walk-in blocking).
2. org/drools/core/reteoo/NotNode.java + BetaNode.java — the
   blocked/blocker linked-list structure (addBlocked semantics:
   head-prepend or tail-append) and retractRightTuple.
3. org/drools/core/common/TupleSetsImpl.java — addInsert/
   addUpdate prepend semantics per staged type (the D-323
   round already pinned addFirst for inserts; confirm for the
   ph-classes the not release uses).
4. The JOIN emission: PhreakJoinNode.doLeftInserts /
   doRightInserts — the order children stage toward downstream
   nodes (THE suspected divergent variable), and
   LeftTupleSets/SegmentPropagator batch composition.
Extract: for each machine step, the ITERATION DIRECTION and
LIST BUILD DIRECTION. The composition of these IS the model's
transition table — write them into PINS.md as quoted source
lines (the D-320 style).

## THE MODEL PLAN (step 2 — D-083 style)

tools/model_check_notrel.py, in the model_check_join2.py mold
(32 machines × oracle timelines, unique survivor): state = not
node {left memory order, blocked lists per blocker, queue},
ops = {left-ins batch, blocker-ins (block walk), blocker-del
(release), left-del}, candidate axes = {memory build dir,
block walk dir, blocked build dir, release iter dir, staging
flip on/off, re-add position}. Fitness timelines: nb1, the
2364 relay (BOTH rounds), r3, r4, fz_7_9360, nb3 + oracle
runs of any new grid cells (SEINE_FIRE_LIMIT + the
seine-harness oracle idiom; oracle 3× per cell). The survivor
machine's deltas vs the engine name the port EXACTLY.

## VERIFICATION SET FOR THE PORT

The 4 would-graduates flip PASS; the 11 counter-cells hold
byte-identical (byte gate vs f50989a); SD census 72 EXACT ×12
(seeds 7001,7002,6001,6003,7004..7011 → 6,10,3,5,6,5,5,6,8,7,
4,7); full battery per the D-330 receipts block in DECISIONS
(corpus 11/1476/414, drift 24, lint 2324 w/ expect_error,
cargo 74, pytest 257, model_ird 31/31, IRD ×5, agenda ×10×3,
fuzz next seeds 329001/329002, fuzz_cep 329901+; make diff
wall ~7m38s is NORMAL — the pr_rw_ relay cells). Byte-gate
script: jobs tmp bytegate2.sh (sed the wt name in FIRST;
currently wt_pre331 — stale).

## REPO STATE (2026-07-19)

Pushed through 958d31f (v0.4.42 tag). LOCAL unpushed:
ad6bf6e (D-328 canon) → 8ac7b62 (verification addendum) →
3766fad (D-329 wheel stamp) → 5b0083c (D-330 runaway class) →
f50989a (D-331 round record). Engine byte-exact at 5b0083c.
CHANGELOG Unreleased: the D-330 detection entry (+ loud compat
note) + D-329 wheel self-identification — release-ready.
Doctrine: oracle decides; predictions in PINS BEFORE cells;
hand-decode/source-read before gridding; commit per green
slab; NEVER push without Bryan's word; maturin develop
CLOBBERS the tracked bindings .so (git checkout before
commits); cwd persists across Bash calls — use absolute paths.
