# HANDOFF — the query-eval MEMOIZATION + old-backlog PERF round
# (cold start; written 2026-07-19 at 29eeebf, Bryan-directed:
# "prepare a handoff to perform the memoize and perf testing
# also test for introducing any quadratics in the areas we've
# touched in the old-backlog." Read probes_pending/oldbank/
# PINS.md D-350..D-366 for the session's laws and ports.)

## PART 1 — THE D-055 MEMOIZATION SLAB

THE WITNESS: scenarios/xfail/xf_fz_9201_1660 (_finding in-file,
D-366). GROUND TRUTH RECORDED: at SEINE_FIRE_LIMIT=2000000 the
ORACLE completes — 374,533 firings + 12 query outputs, 3x
byte-stable (~1 min). The ENGINE cannot complete even at a
scratch STEP_LIMIT of 200M in 9.5 min: the D-055 Machine
(engine/src/queries.rs) re-derives UNBOUND RECURSIVE pulls per
activation — level_cache memoizes per-site fact DRAINS only,
never RESULTS. The blow-up shape: TWr($w) = TCr($w,$f) with both
TCr args open, pulled twice per QR2 activation over T1-pair
cross-products, on an 8-edge DAG.

THE TARGET: result-level memoization. HARD CONSTRAINTS (all
certified surfaces):
- multiplicity is PER-DERIVATION-PATH (the D-361/1704 records:
  TCr duplicate edges yield duplicate rows x14) — a memo must
  REPLAY multiplicities, never dedup;
- emission ORDER within and across branches is pinned
  (D-054/D-055 stack machine; D-363's d363_reorder sits ON TOP
  at the pub run_query path);
- the D-055 walls stay (base-first 2-branch self-recursion, no
  left/mutual recursion, the STEP_LIMIT backstop for truly
  cyclic data).
Design sketch: memo key = (callee qi, bound-arg vector) -> the
ORDERED row list, valid within one Machine run (WM frozen there
— the same freeze that justifies level_cache); a cross-CALL memo
needs a WM-generation stamp (fact_prov/on_insert give one
cheaply). Start intra-run; measure; only go cross-call if the
witness still cannot complete.
SUCCESS CRITERIA: (1) xf_fz_9201_1660 with a `fire_limit`
field (D-332) >= 400000 completes AND byte-matches the recorded
oracle output (re-derive it: SEINE_FIRE_LIMIT=2000000 java ...
OracleRunner, 3x) -> the cell GRADUATES and the bank drops to
10; (2) the D-055 error text's "cyclic recursion data?" guess is
refreshed (D-366 proved a DAG can trip it); (3) the full query
battery (pr_jq*, pr_qe_*, pr_qc_*, the D-054/055 cargo tests)
byte-identical — the memo must be observationally invisible.

## PART 2 — THE QUADRATIC AUDIT (sites touched D-357..D-364)

Each site below was added this session and carries a named
complexity concern. The D-298 lesson applies: MULTI-SAMPLE
before naming a residual; measure first, fix only what moves.
1. shares_before prefix counts — engine.rs d357_wave_reorder
   (~10630) AND d359_termq_reorder (~10508): per DISTINCT birth
   firing k, `firing_log[..k].filter(share.contains)` = O(N_rhs
   x F). Fix sketch: one cumulative prefix-sum pass over
   firing_log per call = O(F + N).
2. The D-361 qce fold — engine.rs ~10443: `src.del.filter(|t|
   src.ins.any(== t))` = O(del x ins) per terminal consume.
   Fix sketch: HashSet of ins tuples (fact-id vectors hash
   cheaply).
3. The D-363 window rank — queries.rs ~1159:
   `windows.position(|w| w.contains(f))` per row = O(rows x
   windowed-facts). Fix sketch: per-site fact->window-index
   HashMap built once per reorder.
4. The D-360 kept-kind reposition — phreak.rs ~1219:
   `pending.ins.position(== t)` linear scans (a PRE-EXISTING
   pattern in peer_merge_left; the D-298 StagedList has keyed
   first-occurrence lookups if it measures hot).
5. d359's `firing_log.contains(&ri)` at first selection: O(F)
   x rules — likely noise; measure anyway.
6. The provenance structures (fact_prov HashMap, firing_log
   Vec, QueryMem.1 window lists): O(1) amortized growth — audit
   MEMORY only (window lists retain dead fact ids by design;
   the reorder filters to live — confirm no unbounded retention
   across epochs on long runs).

## PART 3 — DEEP-SCALE PROBE DESIGNS (the D-297/D-298 method)

Author as probes_pending/qperf/qp_*.json; scale ladders N in
{100, 1000, 5000} (generate with a python script committed
alongside); measure release wall time 3x per rung; ~linear (or
N log N) slopes pass, super-linear = gdb-sample the frame
(MULTI-sample), fix, re-measure, byte gate.
- qp_wave_N: the pr_tq_m123 chassis at N R1-driver T0s (2N RHS
  T1s, one mass unblock through the subnet) — exercises
  d357_wave_reorder + the release path + shares_before.
- qp_termq_N: the pr_tq_p359b chassis at N windows (N T2
  drivers) — exercises d359_termq_reorder + shares_before.
- qp_qce_N: a pr_qc_m1704b-shape qce rule pulling an N-row
  closure with a same-eval delete of the driving fact —
  exercises the D-361 fold's del x ins.
- qp_enum_N: the pr_qe_p363a chassis at N windows — exercises
  d363_reorder's window rank + the QueryMem growth.
- qp_repos_N: an m1020b-shape epoch with N stale staged inserts
  clashing kept-kind — exercises the D-360 reposition scans.
BASELINE FIRST: run the ladder on 29eeebf BEFORE any fix so
slopes have a reference; keep the timing table in this lane's
PINS.md.

## GATES (unchanged doctrine)

Every fix: full byte gate vs pre-edit worktree over
scenarios+probes_pending (EXPECT ZERO movers — perf work must
be order-invisible; any mover = a bug, revert per D-331/D-345/
D-352), then the full battery per the card below. The
memoization slab additionally: the Part-1 success criteria.
Commit-per-green-slab; predictions/timings recorded in
probes_pending/qperf/PINS.md BEFORE fixes land.

## VERIFICATION CARD (all green at handoff, 29eeebf PUSHED)

make diff 11/1630/414 + drift bank 11 identical (all
banked-by-design); lint-probes 2487/0/0; cargo 74; pytest 260
(.venv/bin/maturin develop --release -m bindings/Cargo.toml
from the REPO ROOT, pytest with absolute paths, then git
checkout bindings/python/seine_rs/_native.abi3.so); demo True;
model_ird 31/31 + check_witnesses 26/26 + validate_cells 39/39
(cd probes_pending/tms_envelope); IRD 150x5 seeds
7001/7002/6001/6003/9001 0-div; SD census 150x12 seeds
7001,7002,6001,6003,7004..7011 = 6,10,3,4,6,5,5,6,8,7,4,7 = 71
EXACT (debug build first, never rebuild mid-census);
agenda_open x10 identical x3 binaries; fresh fuzz 2x2000 NEXT
seeds 358001/358002 + fuzz_cep 3x300 NEXT 358901-903 (finds:
bisect vs the pre-edit worktree; pre-existing -> mv
scenarios/xfail/ + tools/xfail_drift.py --rebank; the CEP
fuzzer regenerates per-seed — a banked find's seed stays a
known-find record). Byte gate: git worktree add wt_preNNN
<sha>, release build both, compare `run` outputs xargs -P 8;
REMOVE the worktree before git add -A.

## REPO/DOCTRINE STATE

ALL PUSHED through 29eeebf (main, untagged; latest release
v0.4.44; publish-crates standing red = Bryan's crates.io TP).
CHANGELOG carries SEVENTEEN release-ready Unreleased entries —
release ONLY on Bryan's explicit release word; "push, no tag" =
push main untagged; NEVER push unprompted. THE OLD-BANK TRIAGE
IS FULLY DISPOSED (D-350..D-366; records in
probes_pending/oldbank/PINS.md — the lane is records-only).
Remaining banked-by-design: the D-263 oracle-NPE pair,
xf_fz_9201_1660 (Part 1's witness), cf355901x129 +
fz_356002_1512 (latent order finds, undecoded), heaptie
(accepted-undefined). Bryan-gate items parked: the
message-class-blind expect_error contract extension (would flip
9201_1660 without the perf slab); the D-354-family scoped
residuals. PITFALLS (bit this session): bash cwd PERSISTS (use
absolute paths; subshell any cd); maturin clobbers the tracked
.so; worktrees out before git add -A; the oracle __h parser
must guard synthetic facts; head -N truncation hid the m123 R0
fork for two rounds — read FULL diffs when decoding.
