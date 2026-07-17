# D-076 unbounded-tier probe pins (step B recon; oracle 3×, guarded singles)

All probes `engine_fenced` (the D-284 stratification wall rejects the
computed cycle engine-side — lint verifies the wall; the wall's lift is
the gated step-B port). Every cyclic scenario ran as a TIMEOUT-GUARDED
SINGLE — never batched (JVM hang hazard). All results 3×-byte-stable
unless noted. Predictions were registered BEFORE the oracle round
(git: this file's first commit-in-progress draft); 8/8 predicted
probes hit exactly; the deep tier produced the round's FINDING.

## §1. Semantics pins (the fixpoint algebra — all predicted, all hit)

| probe | oracle 3× | pin |
|---|---|---|
| ub_fix_k6 | Grow ×5 (on 1..5), final T(1..6) | bounded fixpoint `T(n < K) → insertLogical(T(n+1))` terminates at the guard; one firing per derived value |
| ub_teardown_k6 | + WGone, final = P only | external root delete tears the WHOLE chain down; WAlive silent |
| ub_order_k6 | Obs firing order = **Obs4 Obs1 Obs6 Obs3 Obs2 Obs5** (the shuffled DECL order), Marks 1..6 all present | teardown-driven not-unblocks land as ONE quiescence batch; equal-salience observers fire in DECL order ⇒ **teardown propagation order is agenda-INVISIBLE at equal salience** (deep-teardown order needs no cross-engine pinning beyond the SD-census shapes already banked) |
| ub_anchor_belief | Seed2 + Grow ×5; final = B, T(3..6) | a MULTI-JUSTIFIED key stops the teardown: key-3 keeps the Seed2 belief when Grow-on-2's act dies; everything below the anchor survives |
| ub_tworoot_pend | Grow fires 1,4,2,5,3,6,7 (chains INTERLEAVE); final = T(4..8) | the D-211 pending model composes at depth: logical value-4 lands PENDING on the stated T(4) (no materialization, no re-fire — Grow never fires twice on 4); root-1 delete kills 2,3 and CLEARS the pending; the stated anchor + its chain survive |
| ub_supersede_chain | Seed(1) Grow(1..5) Seed(3); final = A(3), T(3..6) | refire-supersede RE-ROOTS a chain: the update's refire re-establishes key-3, the epilogue drops stale key-1, the cascade eats 1,2 and STOPS at the re-established key |
| ub_ungrounded | Grow(1..4) Shrink(4) Shrink(5); epoch-0 final T(1..5); post-root-delete final = **T(3),T(4),T(5)** | grow/shrink cycle: (a) FINITE value set ⇒ terminates by value-keyed dedup (dup beliefs, no dup facts); (b) ⚖ **UNGROUNDED mutual-support clusters SURVIVE root deletion** — 3←Shrink(4), 4←Grow(3)+Shrink(5), 5←Grow(4) hold each other up; Drools TMS is SUPPORT-COUNTING, not well-founded. The engine's value-keyed act model reproduces this naturally (acts live while their tuple facts live) |

## §2. The deep tier — THE ROUND'S FINDING: the oracle's teardown is call-recursive too, and dies FIRST

| probe | oracle | pin |
|---|---|---|
| ub_grow_1000 | OK ×3: 999 firings, final = T(1..1000) | chain GROWTH is agenda-iterative — no depth limit on the way up |
| ub_teardown_200 | OK ×3: complete teardown, final = P | 200-deep teardown completes |
| ub_teardown_rhs300 | OK ×3: Grow ×299 + Del + WGone, final = P | RHS-delete-caused 300-deep teardown completes — same recursion class as external cause |
| ub_teardown_500 | **StackOverflowError ×3** | 500-deep external teardown BLOWS THE ORACLE'S STACK |
| ub_deep_1000 / ub_deep_9000 | **StackOverflowError** (1000: ×3; 9000: ×1, same class) | scenario-level clean error (the batch survives; the JSON error line is deterministic) |
| (scratch bisect, this env) | 300 OK / 400 SOE | ceiling ≈ [300, 400] on the DEFAULT JVM stack — the runner sets no -Xss, so the ceiling is a JVM-config RESOURCE LIMIT, not a semantic constant |
| ar_tms_runaway_logical (re-run ×3) | "IllegalStateException: fire limit 100000 reached (non-terminating?)" ×3 | the fire limit is the only INFINITE-cycle governor; the whole scenario errors (batch state discarded) → parity is error-vs-error (D-013/j21). Engine twins: fire limit + D-117 spin guard |

Consequences for the step-B port (Bryan's gate):

1. **Drools cannot witness deep teardowns.** RemoveLogicalDependencies
   recurses; beyond ≈350 frames (env-dependent) the oracle SOEs. The
   "legitimate 100k-fire fixpoint tears down 100k deep" scenario is
   REAL on the growth side (fire limit permits it) but its teardown is
   oracle-UNOBSERVABLE. Post-lift certification therefore needs a
   **teardown-depth residency precondition** (the D-290 mode-1
   residency precedent): byte-certified cells live at depth ≤ 200
   (conservative, stable side of the bracket); deeper cells are
   engine-guaranteed (worklist boundedness) with the oracle divergence
   class DOCUMENTED (SOE-vs-success — the engine outliving the oracle,
   opposite-polarity witnesses like ub_teardown_500).
2. **The 8192 assert goes WITH the lift, not before.** Under the wall
   it stays unreachable (D-284's bound); post-lift a legitimate deep
   fixpoint teardown would false-panic it. The worklist already
   removed the thing the assert protected (engine stack).
3. **Semantics the lift must certify** (all pinned above, all already
   natural under the engine's value-keyed act model): guard
   termination, whole-chain teardown, multi-justifier anchors,
   pending-on-stated at depth, supersede re-rooting, dedup-bounded
   finite cycles, ungrounded-cluster survival, fire-limit parity on
   infinite cycles.
4. **gen.rs stays ACYCLIC** (handoff directive) — cyclic computed
   shapes are designed runaways/deep-teardowns; probes carry the
   surface.
