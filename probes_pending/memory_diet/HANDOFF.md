# HANDOFF — the memory diet (SmallVec/inline tuples): cold start

Filed 2026-07-15 after D-269 (the OOM endurance race). Repo state at
filing: HEAD `2d2fc7c` (D-269), local-unpushed on top of the pushed
`v0.4.28` (`5f6d623`). Working tree clean. Bryan named this the
follow-up; THE ENGINE EDIT IS BRYAN-GATED — measurement and a
validated worktree prototype are not.

## The goal

D-269's race (equal 16GB kernel cgroup caps, disjoint join, N facts
per side, zero firings): the engine OOMs at ceiling ∈ [4.25M, 4.5M)
pairs; Drools survives to [4.5M, 4.75M). The engine is ~20% lighter
at every N BELOW the wall (3.8 vs 4.7GB at 1M) and loses only the
last mile to G1's compaction. Steady state: **~1.9KB engine RSS per
single-i64-field fact**. Target: cut per-fact RSS enough to push the
16GB ceiling past 4.75M pairs (take the endurance crown) without
giving back any of the D-266..268 speed (join_10000 ≈ 62-72ms,
all sweeps linear through 16k).

## Step 0 — MEASURE FIRST (the attribution is a hypothesis)

The "small Vec allocs dominate" story is INFERRED, not measured.
valgrind/massif/heaptrack are absent and perf_event is sysctl-blocked
(perf_event_paranoid=4, ptrace_scope=1) — the working pattern in this
repo is feature-gated in-process instrumentation (precedent: the
`prof` feature, pprof-rs, `--features prof` + SEINE_FLAME=out.svg for
CPU flamegraphs; D-268). For allocations: add a counting global
allocator behind a feature (count + bytes by size class, dump at
exit), run the D-269 workload at N=1M, and get the real breakdown
BEFORE dieting. If the top consumer isn't the tuple/key allocs below,
follow the measurement, not this file.

## The allocation surface (verified sites, engine/src/)

- `pub type Tup = Vec<FactId>` — phreak.rs:24. ~230 uses across
  engine.rs (109) / phreak.rs (114) / queries.rs (5). Every k=1 rule
  carries ONE-ELEMENT heap Vecs per fact in every structure that
  touches tuples.
- Join beta memory (phreak.rs ~368): `lefts: Vec<(Tup, Option<Vec<Value>>)>`
  — one Vec alloc per tuple + one per stored key;
  `rights: Vec<(FactId, Option<Vec<Value>>)>` — one per key.
- `Child { tuple: Tup, left: Tup, right: Option<FactId>, dead }`
  (phreak.rs ~352) — TWO Vec allocs per join child.
- Tup-keyed maps, one key-Vec alloc per entry: `lseq: HashMap<Tup, u64>`,
  `left_sseq: HashMap<Tup, u64>`, `by_left: HashMap<Tup, Vec<usize>>`
  (phreak.rs ~395-402, ~374), `act_num` (engine.rs, per-net), the
  D-266 `seen: HashSet<T>` per Staged (T = Tup on the left side).
- Store side (store.rs): fact rows + `fields: Vec<(String, Value)>`
  surfaces — measure before assuming they matter (they're per-FACT
  not per-tuple-structure, but rows may already be columnar — check).

## The diet sketch (candidates, in expected value order)

1. `Tup` → `SmallVec<[FactId; 2]>` (smallvec crate, new engine dep).
   Inline ≤2 covers k=1 and k=2 rules (measure the corpus k
   distribution first — likely >90% of tuple instances). Mechanics:
   the alias swap compiles most sites; expect fixes at `vec![f]`
   constructors (→ smallvec![f] / SmallVec::from), slice patterns
   (`l.as_slice()`, `[f] =>` matches), functions taking `&[FactId]`
   (Deref covers), Hash/Eq/Ord derive (smallvec has them; hashing
   must equal Vec's — it hashes as a slice, same as Vec ✓ so HashMap
   keys stay compatible IF all keys swap together).
2. Stored join keys `Option<Vec<Value>>` → `Option<SmallVec<[Value; 1]>>`
   — keys are 1-element in the certified eq-index scope already.
3. The D-266 `seen` sets: consider capacity hygiene (they only grow;
   fine) — or measure whether per-entry Tup keys dominate; covered
   by (1).
4. NOT candidates without their own recon: replacing map keys with
   hashes (collision semantics), columnar store rewrites.

⚠ SPEED IS A CO-GATE: SmallVec makes moves/clones memcpy-heavier for
inline variants — re-run the D-268 sweeps (doubling ratios must stay
~2.0) and tools/bench_oracle.py --scale after; join_10000 must stay
≤ its current 62-72ms band.

## The protocol (the D-266..268 pure-optimization pattern, verbatim)

1. Baseline byte capture (THE gold gate — re-run after EVERY step):
   `find scenarios probes_pending -name '*.json' | sort > /tmp/all.txt`
   `./target/release/seine-harness run $(cat /tmp/all.txt | tr '\n' ' ') > byteid_pre.ndjson 2>/dev/null`
   (2,028 files at filing). cmp after each change: BYTE-IDENTICAL or
   stop and understand.
2. Certified gates per checkpoint: `make diff` (11/1209/404 + drift
   37 identical), `cargo test` (53), maturin develop --release +
   `pytest bindings/tests` (171), demo selfcheck (LIVE==REPLAY True).
3. Fresh fuzz 2×2000 on unused seeds; any find → worktree bisect at
   the pre-change commit; byte-identical ⇒ pre-existing ⇒ quarantine
   per D-255 (xf_ prefix, open_divergence, drift rebank).
4. Memory receipts: `tools/bench_oom.sh 16 4000000 4500000 4750000
   5000000 6000000` before/after (plus a 1M run for the per-fact
   number). The crown = engine ceiling > 4.75M.
5. Speed receipts: bench_oracle --scale 3 passes + the 1k..16k
   doubling sweep (generate via the D-268 method or tools/bench_oom
   -style gen). Alloc-count receipts from step 0's allocator.
6. D-entry (D-270+), commit local, NO push/tag/bump without Bryan.

## Env crumbs

- `export PATH="$HOME/.cargo/bin:$PATH"`; run from repo root.
- Oracle prebuilt at oracle/target/classpath.txt; worktrees need
  `ln -sfn <main>/oracle/target <wt>/oracle/target`.
- perf/gdb blocked (see above) — feature-gated instrumentation only;
  CPU flamegraph: `cargo build --release -p seine-harness --features
  prof` + `SEINE_FLAME=out.svg`.
- cgroup runs: `systemd-run --user --scope -p MemoryMax=16G
  -p MemorySwapMax=0 <cmd>` (works unprivileged).
- SEINE_TIME=1 → per-scenario ms on stderr, both runners.
- Machine at filing: 125GB RAM, 20 cores, no swap.
