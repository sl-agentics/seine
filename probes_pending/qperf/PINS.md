# qperf PINS — the query-eval memoization + old-backlog perf round
# (D-367; executing probes_pending/qperf/HANDOFF.md; predictions
# registered BEFORE any ladder run or fix, per doctrine)

## PART 3 LADDER PREDICTIONS (registered pre-measurement)

Each qp_* ladder is engineered so the LEGIT workload is ~linear
(selective second patterns keep join outputs O(N)) and only the
audited site can contribute a super-linear term.

- P1 qp_wave_N (d357_wave_reorder): the shares_before build iterates
  fact_prov (O(N) RHS facts) and per distinct birth-firing k scans
  firing_log[..k] (O(N)) -> predict ~O(N^2) at the wave release.
  parent_of is O(trie^2) but trie is program-bounded — invisible.
  Post-fix (one cumulative prefix-sum pass): ~linear.
- P2 qp_termq_N (d359_termq_reorder): same shares_before shape at the
  lazy Term rule's FIRST selection -> predict ~O(N^2) once.
  firing_log.contains(&ri) is once-per-rule (termq_sorted gate) ->
  predicted NOISE (the handoff's item 5 ruling confirmed by reading:
  the gate flag flips before the call; it cannot run per-selection).
- P3 qp_qce_N (the D-361 fold): del.iter x ins.any = O(del x ins)
  per terminal consume + the cancel loop's remove_first_by_key.
  With N same-tuple ins+del pairs -> predict ~O(N^2). Post-fix
  (HashSet membership; batch removal): ~linear. NOTE
  remove_first_by_key may already be keyed O(1) via the D-298
  StagedList — if so the fold scan alone carries the quadratic.
- P4 qp_enum_N (d363_reorder window rank): windows.position(contains)
  per row with N windows of 1 fact -> predict ~O(N^2) at the
  top-level call. Post-fix (fact->window HashMap): ~linear.
- P5 qp_repos_N (the D-360 reposition arm): pending.ins.position per
  kept-kind clash -> predict ~O(N^2) with N stale x N kept. The arm
  is delicate to engage from a blind ladder — engagement is verified
  with a scratch instrument BEFORE timing; if the ladder cannot
  reach the arm at scale, record that and fall back to bounding the
  scan analytically (pre-existing peer_merge_left pattern).

All fixes MUST be order-invisible: shares_before prefix-sum computes
identical values (pure function of k); the rank map computes
identical (bucket, wi) per fact; the fold's HashSet preserves
del-iteration order of `cancelled`. Byte gate expects ZERO movers.

## PART 1 MEMO PREDICTIONS (registered pre-profile)

- M0 (the cost model): steps-arithmetic from the DAG (T=23 TWr rows,
  k~25 T1s) puts one full site-2 evaluation at ~1.1M steps — enough
  to trip the 1M backstop per evaluation, but NOT enough to explain
  200M steps consumed in 9.5 min. PREDICT: the profile shows the
  dominant cost is re-evaluation of the FULL accumulated left set
  per churn event (not per-firing), multiplied by Env-clone width,
  with a possible second term from an engine-side per-firing scan
  (fact_prov/firing_log growth is O(1) per firing — audit confirms).
  The honest register: where the 200M goes is OPEN until profiled.
- M1 (the law): in a uniform-argvec batch, the callee's qmem output
  is a concatenation of per-flush SEGMENTS; each segment holds one
  contiguous BLOCK per caller, every block byte-identical to the
  single-caller row sequence, block order forward-or-reverse per
  segment (the flat-list machinery is env-by-env expansion + full
  reversals + front-splices — all caller-block-lockstep-preserving).
  Therefore a 2-probe capture (Root::Site(0)/Site(1) markers)
  determines the replay for ANY k. VALIDATED AT RUNTIME: if the
  capture fails to parse into equal-block segments, fall back to
  real evaluation (zero semantic risk; the theory is testable
  per-call).
- M2: intra-run memoization alone (probe once per (callee, argvec)
  per Machine run, replay per caller) makes the witness complete
  within the step budget: post-memo cost ~ output size (~375k rows
  ~= the 374,533 oracle firings) + one probe descent per site.
  Cross-call memoization is NOT needed. (If wrong: the fallback is
  the WM-generation-stamped cross-call memo, recorded before built.)
- M3: memo movers = xf_fz_9201_1660 ALONE (no certified cell pins
  the step-limit error — grepped scenarios/: only the witness and
  the drift-bank baseline mention it; truly-cyclic data still trips
  the limit inside the probe evaluation itself).
- M4: the memo is fenced OFF when any callee-branch pool is
  non-empty at the Call (the run_query top-level self-recursion
  sweeps env0 into nested batches — "pools may be swept early");
  run_site paths always see empty pools, so the witness class memos.

## MEASUREMENTS

### Baseline @ HEAD=17189d9 (engine byte-identical to 29eeebf;
### release, 3x, seconds; engagement verified by scratch instruments
### first: d357 ins=202/prov=304/firings=101, d359 queue=203 once,
### d361 del=100 ins=200 cancelled=100, d363 rows=104 windows=101,
### d360 kept_scan x10 pending=121 @ N=100)

| cell     | N=100 | N=1000 | N=5000 | slope 1k->5k |
|----------|-------|--------|--------|--------------|
| qp_wave  | 0.00  | 0.04   | 0.54   | 13.5x  ~N^1.9 |
| qp_termq | 0.00  | 0.03   | 0.31   | 10.7x  ~N^1.7 |
| qp_qce   | 0.01  | 0.26   | 6.07   | 23x    ~N^2   |
| qp_enum  | 0.00  | 0.06   | 1.05   | 17.5x  ~N^1.8 |
| qp_repos | 0.00  | 0.01   | 0.11   | 11x    ~N^1.5 (ladder shape: M scans x M^2 pending, N=M^2) |

P1-P4 slopes CONFIRMED super-linear; P5 engaged at its designed
N^1.5 shape.

### PROFILE FINDINGS (gdb multi-sample; perf blocked, paranoid=4)

- qp_qce baseline 4/6 samples: fire_all -> next_activation ->
  drain_query -> drain_pattern (the `seen` HashSet + live_facts_of
  rebuild). NOT the D-361 fold (P3's named site is subdominant).
  THE LAW OF THE BLOW-UP: every WM event re-drains every armed
  linked query's patterns, and live_facts_of walked EVERY handle
  ever inserted (all types) — O(events x handles).
- M0 RESOLVED — the witness (scratch 200M-step binary,
  SEINE_FIRE_LIMIT=2M): 6/6 samples in the SAME drain_pattern
  stack; completes in ~100-130s pre-fix (the D-366 9.5-min figure
  does not reproduce on this box; the class is confirmed, the
  constant was environmental). The 374k QOut inserts x full-store
  drains ARE the wall-clock wall. The prediction's "re-evaluation
  of accumulated left sets" part was WRONG — run_site is called
  O(1) times per churn event with delta lefts only (instrumented:
  2 calls total on qp_qce). Recorded as a miss.
- qp_qce post-drain-fix 4/4 samples: run_site -> Machine::walk
  candidate scan — the D-053 single-field unification index leaves
  a bound NON-index eq beta unindexed; the unbound-index descent
  full-order-scanned (and CLONED) per env. Fixed by the aux
  equality bucket (gated: exact type match, non-F64 — join-eq ==
  exact equality there; F64/cross-type keep the scan since D-020
  coercion and bit-pattern equality diverge from bucket equality).
- qp_wave/termq residual after shares_before: DIFFUSE pre-existing
  phreak frames (peer_merge_left hash staging, do_join_node
  allowed, push_activation, vec growth) — no single quadratic
  frame; disposition pending the pre-oldbank slope check (below).

### Slab A fixes (all order-invisible by construction)

1. store: per-type handle index (live_facts_of O(type), same
   sequence) + type_gen (insert/kill/set_value) + type_mut_gen
   (kill/set_value only) generations.
2. drain_pattern: 3-way — clean skip (type_gen equal), insert-only
   incremental (mut_gen equal: retain provably no-op, test only
   post-hwm handles), else full walk. Split update/read so
   drain_query stops cloning the memory per drain.
3. query_linked: memoized per query on the summed member-type gens
   (mark_queries_pending).
4. shares_before x2 (d357/d359): one ascending firing_log walk.
5. D-361 fold: keyed contains (the StagedList index — the scan was
   simply not using it).
6. d363 rank: fact->first-window map.
7. eval_fact_level: borrowed candidate slices (no per-env
   full_order/arrival clones) + the aux equality bucket.

### Post-Slab-A (release, seconds)

| cell     | N=1000 | N=5000 | slope | vs baseline @5k |
|----------|--------|--------|-------|-----------------|
| qp_wave  | 0.03   | 0.33   | 11x   | 1.6x (residual = pre-existing phreak staging, see above) |
| qp_termq | 0.02   | 0.19   | 9.5x  | 1.6x (same class) |
| qp_qce   | 0.02   | 0.18   | 9x    | 34x  |
| qp_enum  | 0.02   | 0.08   | 4x ~linear | 13x |
| qp_repos | 0.01   | 0.11   | 11x   | 1.0x (pre-existing peer_merge_left pattern, mild absolute) |

Witness check on Slab A: reaches the 1M step limit in 1.3s (was
~100s+ of drain grind to get there) — the Machine-side step count
is untouched, so the memo slab (Part 1) remains the graduation
gate, exactly M2's split.

## PART 1 — THE RESULT MEMO (D-055/D-367), LANDED

Design (M1 as registered, one correction): at a Call node with a
UNIFORM bound-arg vector across the batch, evaluate the callee ONCE
with TWO probe callers (Root::Site ids) on a swapped-out machine
state, capture its qmem emission, greedy-parse it into lockstep
segments (maximal same-probe run opens a segment; the partner's
EQUAL block must follow — anything else stores None and the call
evaluates for real, permanently for that key). Replay emits one
block per real caller per segment, forward or reverse per the
captured direction. Fences: probe_depth>0 (the machine stays fully
iterative below one probe — the D-055 native-stack guarantee),
non-empty callee pools (run_query's swept-env self-recursion),
fact-valued args, non-uniform batches, duplicate unbound arg slots.
Probes share level_cache/pattern memories/steps — a probe's drains
are exactly the drains the real evaluation would perform, and
cyclic data trips the step limit INSIDE the probe with the
identical error.

THE ONE BUG THE GATE CAUGHT (recorded miss, D-331 protocol
honored): fills were first stored by CALLER SLOT from the probing
site; qc3_sibling reaches the same (callee, argvec) from
direct($x,$y) AND direct($z,$y) — the second site's replay wrote
$x, left $z unbound, and the open recursion looped to the step
limit. Fix: fills are keyed by CALLEE ARG POSITION (site-
independent); the replay maps positions onto the REPLAYING site's
slots with the terminal handler's own unbound-first-per-slot rule.
qc3_sibling byte-restored; the byte gate is the arbiter that made
this a 30-minute find instead of a shipped divergence.

RECEIPTS: xf_fz_9201_1660 GRADUATES as
scenarios/probes/pr_qm_fz_9201_1660.json (fire_limit 400000,
open_divergence/_finding dropped): engine completes in 2.5s /
746MB RSS under the STANDARD 1M step limit (D-366: no completion
at 200M steps in 9.5 min; the oracle's 374,533 firings + 12 query
outputs), harness diff vs the live oracle PASS (3x pre-fix + 1x
post-fix, oracle re-derived each run). Bank 11 -> 10 (rebanked).
The D-055 error text refreshed: the 'cyclic recursion data?' guess
is gone (a finite DAG proved able to trip it); the "step limit"
substring stays (cargo pin). Slab A alone: byte gate 2623/2623
ZERO movers. Slab A + memo + graduation: expected movers = the
graduated cell alone (gate rerun below).

## BATTERY (D-367 commit receipts, all green 2026-07-20)

Byte gates: Slab A 2623/2623 ZERO movers vs 17189d9; final gate =
pr_qm_fz_9201_1660 ALONE (the graduation). make diff 11/1631/414
+ drift bank 10 identical (clean full run obtained; one flapper
occurrence recorded below). lint-probes 2503/0/0 (+15 qp cells,
+1 graduation). cargo 74 (workspace). pytest 260 (maturin
develop --release, .so restored). demo True. model_ird 31/31 +
check_witnesses 26/26 + validate_cells 39/39. IRD 150x5 seeds
7001/7002/6001/6003/9001 0-div. SD census 150x12 =
6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT cell-for-cell. agenda_open
10 cells x10 stable x3 binaries (release/debug/pre-edit). Fresh
fuzz 2x2000 seeds 358001/358002 CLEAN (0 div, 0 xfail) +
fuzz_cep 3x300 seeds 358901-903 CLEAN. NEXT fuzz seeds 359001+ /
cep 359901+.

### THE FLAPPER RETURNS (D-365 correction, recorded 2026-07-20)

pr_co_fz_123_6887 FLAPPED in 2 of 3 full `make diff` runs during
this round's battery (heavy parallel load: byte gates + oracle
fleets concurrent), then passed 3x sequentially. The flap detail:
firing[10] — the ORACLE's agenda order itself swapped (R3's
2-element collect firing vs R5), the classic ArrayList/hash-order
oracle-side mechanism. The engine is not implicated (drift gate
engine-vs-bank identical; byte gates clean; the oracle is
untouched by this round). D-365's "zero flaps on the current
stack" census conclusion is FALSIFIED — the cell is a LIVE
oracle-side load-sensitive flapper. Disposition (re-open vs
accept rare flaps) is Bryan's call; the cell stays certified (a
clean full diff 11/1631/414 was obtained this round).

### THE PRE-OLDBANK VERDICT (the handoff's introduced-quadratics
### question, CLOSED)

wt_c757528 (pre-D-350, before every old-bank port) on the same
cells: qp_wave 0.03/0.32, qp_termq 0.02/0.18 @ N=1k/5k — IDENTICAL
to post-Slab-A current. The residual wave/termq slope is entirely
PRE-EXISTING phreak staging machinery; the D-357..D-364 additions
are cost-invisible at these scales after Slab A. (qp_wave_1000
output also byte-matches c757528 — the reorders are no-ops there
by parity.) qp_repos's 0.11 @5k likewise matches the pre-existing
peer_merge_left pattern — left as recorded, per the fix-what-moves
rule.
