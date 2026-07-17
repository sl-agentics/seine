# D-076 unbounded-tier probe pins (step B recon; oracle 3×, guarded singles)

**D-296: THE LIFT LANDED.** The probe files this record describes are
no longer in this dir: 12 graduated to `scenarios/probes/pr_ub_*`
(engine-vs-oracle corpus members, all PASS), and the deep/runaway
grinders (`ub_deep_9000`, `ub_deep_99k`, plus arith_grammar's
`ar_tms_runaway_logical` / `ar_tms_cycle_two_type`) live in
`scenarios/bench_slow/` — correct but engine-side QUADRATIC (the
by_act linear-scan open item, see D-296) so they cannot sit in a
linted tree. The tables below are the probe-round record (all rows
oracle-verified as written).

At probe time all probes were `engine_fenced` (the D-284 wall rejected
the computed cycle engine-side). Every cyclic scenario ran as a
TIMEOUT-GUARDED SINGLE — never batched (JVM hang hazard). All results 3×-byte-stable
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

## §2b. The stack-bump round (Bryan's directive 2026-07-17: "try bumping stack space on the oracle")

Mechanism: `JDK_JAVA_OPTIONS` (JDK 21 launcher env, no code change)
for the experiments. Predictions registered PRE-RUN: with `-Xss1g`,
ub_teardown_500 / ub_deep_1000 / ub_deep_9000 and a NEW
fire-limit-class ub_deep_99k (guard n < 99000 — 98,999 grows, the
maximal legitimate single-fireAllRules chain is ~100k) ALL complete:
WGone fires once, final = P only, 3×-byte-stable; time ~linear in
depth; the SOE bracket was pure stack arithmetic (≈2.6–3.3 KB/level
from the default-1MB [300,400] bracket ⇒ 1g covers ≈350k levels,
3× margin over the fire-limit max).

### Results — ALL PREDICTIONS HIT; the ceiling was pure stack arithmetic

Sanity: `-Xss64m` completes ub_teardown_500 (default SOE'd ×3) — the
launcher runs fireAllRules on a thread the flag governs.

| probe (`-Xss1g`, 3× each) | result | wall | maxRSS |
|---|---|---|---|
| ub_teardown_500 | OK ×3: 500 firings, WGone, final = P | ~1.1s | ~250 MB |
| ub_deep_1000 | OK ×3: complete 1000-deep teardown | ~1.1s | ~250 MB |
| ub_deep_9000 | OK ×3: complete 9000-deep teardown | ~1.2s | ~270 MB |
| ub_deep_99k (NEW: guard n < 99000 — the fire-limit-maximal class) | OK ×3: 98,999 grows + WGone, final = P, complete **99,000-deep teardown** | ~3s | ~600 MB |

All 3×-byte-stable; semantics identical to the shallow pins (complete
teardown, WGone once, final = P only). Time ~linear, memory modest —
no cliff anywhere. **The §2 SOE rows are the DEFAULT-STACK record**
(historical: what a stock `java` invocation does; reproduce by
removing the runner pin).

**RUNNER PINNED (D-295)**: `harness/src/oracle.rs` now passes
`-Xss1g` unconditionally. Receipts: pinned-runner outputs
byte-identical to the env-var runs on all 4 deep probes; full
`make diff` 11/1257/406 green + drift bank 46 IDENTICAL (the pin
changes nothing banked); lint 1990/0/0; cargo 54. Deep teardowns are
now oracle-OBSERVABLE through the ordinary runner up to ≥99k —
covering the whole fire-limit-reachable envelope with ~3× stack
margin (≈2.6–3.3 KB/level × 100k ≈ 300 MB ≪ 1 GB).

Consequences for the step-B port (Bryan's gate) — REVISED after the
stack bump (the original ≤200-residency proposal is in git history;
it is OBSOLETE):

1. **No depth residency precondition needed.** With the pinned
   runner, teardown at the fire-limit-maximal depth (~100k) is
   byte-certifiable directly — growth, teardown, and belief state are
   all oracle-observable. The one certification note that remains:
   the oracle's teardown depth ceiling is a JVM-config resource limit
   (now pinned at 1g in-repo), not a semantic constant — record the
   pin next to any deep-cell receipts.
2. **The 8192 assert goes WITH the lift, not before.** Under the wall
   it stays unreachable (D-284's bound); post-lift a legitimate deep
   fixpoint teardown (ub_deep_99k is 99k deep) would false-panic it.
   The worklist already removed the thing the assert protected
   (engine stack).
3. **Semantics the lift must certify** (all pinned above, all already
   natural under the engine's value-keyed act model): guard
   termination, whole-chain teardown, multi-justifier anchors,
   pending-on-stated at depth, supersede re-rooting, dedup-bounded
   finite cycles, ungrounded-cluster survival, fire-limit parity on
   infinite cycles.
4. **gen.rs stays ACYCLIC** (handoff directive) — cyclic computed
   shapes are designed runaways/deep-teardowns; probes carry the
   surface.
