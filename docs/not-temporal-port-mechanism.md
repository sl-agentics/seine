# not×temporal — ENGINE PORT mechanism report

Status: **LANDED (2026-07-09, D-134).** The port is complete: §3A (arc-B
REAPING) landed D-132/D-133; **§3B (arc-A FIRING DEFERRAL) landed D-134** —
implemented NOT as the report's removal-driven `fire_deadlines` (§3B below), but
as a **hold-in-lefts + `pending_release` re-fire** design (see DECISIONS D-134
for why the removal/phantom-blocker design was set aside). All gates green:
`make diff` 11/956/284 byte-identical, `fuzz_not_temporal` 0 firing-SET
divergences across ~4600 cases (engine==validated-model on every case), the ~0.6%
within-close-time ORDER residual quarantined to `scenarios/xfail/` (§6). The 5
fenced recon witnesses graduated to `scenarios/probes/pr_cep_not_*`.

The sections below are the PRE-PORT plan (kept for provenance); where the landed
design differs from §3B's recommendation, DECISIONS D-134 is authoritative.
It consolidates the validated models (D-129 arc A, D-130 arc B, D-131 chains) with
a drools-core source read and a three-way engine-code map. Read alongside
DECISIONS.md D-128..D-134.

---

## 1. Goal & current fence

Unfence temporal `not` — `not E(this after|before[lo,hi] $a)` — so the engine
matches the Drools oracle. Today it hard-errors at compile:

- **Fence:** `engine.rs:2276` — inside `Constraint::Temporal` handling,
  `if p.ce == CeKind::Not { return Err("temporal constraints on `not` CEs are a
  follow-on slab") }`. This returns BEFORE the D-109 STP-inference-edge code
  (~2313), so temporal `not` currently records no expiry edges and never builds
  a node.

The port is the LAST fence in the CEP E2 arc (positive joins D-125 and exists
D-127 already landed). `not` is the analog but is a genuinely different machine
(a firing DEFERRAL scheduler, not the D-127 admission reorder).

## 2. What the engine must reproduce (validated semantics)

Three validated Python specs are the executable contract. The engine must match
the **firing SET and the cross-close-time order** exactly; one within-close-time
order residual is a scheduler heap artifact (§6).

### 2a. Firing / deferral — arc A (`tools/model_not_defer.py`, 0-div)
A temporal `not` does NOT fire when merely satisfied-so-far (that is Seine's
current bug once unfenced); Drools DEFERS to the pseudo-clock proving no blocker
can still arrive.
- **fire_time** per anchor: `after[lo,hi]` ⇒ `anchor.ts + hi`; `before[lo,hi]`
  ⇒ `anchor.ts − lo`.
- **IMMEDIATE** iff `fire_time < anchor.ts` (i.e. `before` with `lo>0`: the whole
  blocker window is strictly in the past) — fires at the initial fire in
  FIFO/insertion order. Else **DEFER** to `fire_time` (fires when `clock ≥
  fire_time`; `fire_time ≤ 0` fires at the initial fire).
- An in-window **blocker cancels** the firing. Pseudo-clock starts at 0, moves
  only via explicit `advance`.
- A due `advanceTime` batch fires in **reverse close-time** (descending
  fire_time — the PREPEND discipline); across separate advances, in advance
  order.

### 2b. @expires INFERENCE — arc B (`tools/model_not_infer.py`, 0-div firing set)
Absent an explicit `@expires`, Drools infers a per-type expiration offset from
the temporal constraint (the D-109 `TemporalDependencyMatrix` mechanism —
`docs/drools-inferred-expiry-never.md`). For `not E1(this OP[lo,hi] $a)` the
constraint is `E1 OP[lo,hi] E0`; the STP edges are the SAME as a positive
pattern's:

|                 | offset(E0 anchor) | offset(E1 not-pattern) |
|-----------------|-------------------|------------------------|
| `after[lo,hi]`  | hi                | lo==0 ? 0 : NEVER      |
| `before[lo,hi]` | lo==0 ? 0 : NEVER | hi                     |

Reaped when `clock ≥ ts + offset + 1`; NEVER (backward reach, `−lo<0`) = never
reaped; explicit `@expires=E` overrides to `offset=E`.

**KEY (D-130): the inference is INVISIBLE to firings.** A blocked anchor stays
silent whether its blocker is inferred-mortal (arc B) or explicit-immortal
(arc A) — the window-close deferral only fires an anchor that had NO in-window
blocker at insertion, and by the time any inferred blocker expires the only
post-expiry fire-point has everything reaped. ⇒ **the port's FIRING logic needs
no arc-B branch**; arc B is entirely about getting the `facts` (reaping) right,
which is the inferred-offset edges above.

### 2c. Chains — D-131 (`model_not_infer.py`, firing set 0-div)
`chain_not` (`$a E0() $b E1(op1 $a) not E2(op2 $b)`) and `not_mid`
(`$a E0() not E1(op1 $a) $c E2(op2 $a)`): the positive temporal join (D-125) →
the `not` filters (blocked⇒silent) and defers to its window-close (chain_not's
not-anchor is `$b`; not_mid's is `$a`). Firing renders the join TUPLE (the not
contributes no element, D-031). Fully composes the D-125 join with the arc-A/B
not; no new firing semantics beyond §2a/§2b applied per surviving tuple.

## 3. The port — engine mechanism

Two independent halves: **REAPING (arc B, §3A)** and **FIRING/DEFERRAL (arc A,
§3B)**. Both mapped to exact engine sites below (§4 is the reuse index).

### 3A. Reaping — record the not's inference edges (arc B, the `facts` fix)

The offset/reaper infrastructure already exists and is correct (D-109); the port
just has to route the `not` through it. Three edits:

1. **Lift the fence** — `engine.rs:2276` (`if p.ce == CeKind::Not { return Err }`).
2. **Give the `not` pattern an STP-matrix position and record its edges.** Today
   `tpos` (the tuple-element position) is assigned only to `CeKind::Positive`
   (`engine.rs:2234`), so `not`/`exists` are positionless and the edge-recording
   guard `if let Some(self_pos) = tpos` (`engine.rs:2319`) skips them. The edges
   pushed there (`engine.rs:2319-2330`) are exactly what §2b needs:
   `temporal_edges.push((earlier, later, hi))` and `((later, earlier, −lo))` +
   `temporal_pos_type.insert(pos, type)`. The `not`'s type MUST get a matrix
   position (distinct from any tuple-element slot — the not still contributes no
   tuple element) so `E0` receives `ub(E0→E1)=hi` and `E1` receives
   `ub(E1→E0)=−lo`. Then Floyd-Warshall (`accumulate_temporal_closure`,
   `engine.rs:3378`, row-max→NEVER-when-<0 at `3419-3444`) and
   `infer_event_expiry` (`engine.rs:3468`) fold them into `spec.expires`
   unchanged, giving the §2b table.
3. **Stop the bare-NEVER override for the `not`'s type.** The bare-pattern loop
   `if !cp.tpos.is_some_and(|tp| temporal_pos_type.contains_key(&tp)) {
   never_inferred.insert(cp.type_id) }` (`engine.rs:3005-3007`) currently forces
   any not/exists event type to NEVER. Once the `not`'s type has a
   `temporal_pos_type` entry (step 2) it stops being "bare" and this override no
   longer fires it — **but verify** the anchor `E0` (a real bare `$a:E0()` with
   NO temporal constraint of its own) still gets its offset ONLY via the edge,
   not clobbered to NEVER. (Drools: the anchor is bare in the tuple but has the
   join edge, so it expires at `ts+hi+1` — measured D-130. The overwrite hazard
   from `docs/drools-inferred-expiry-never.md` is real; mirror Drools'
   `NEVER→overwrite` vs `finite→max` exactly.)

Reaper is untouched: `schedule_expiration` (`engine.rs:3497-3516`) keys
`deadlines` at `ts+dur+offset+1` (rule-referenced), and `advance()`
(`engine.rs:3946-3981`) reaps everything `≤ clock_ms`. Net: reap at
`clock ≥ ts+dur+offset+1` = the §2b spec, no change needed.

**This 3A block is the whole arc-B port** — because the inference is invisible to
firings (§2b), getting these offsets right is *sufficient* for the `facts` to
match; no firing-side arc-B logic.

### 3B. Firing / deferral (arc A) — the window-close deferral

**The gap.** A `not` currently fires the moment it is UNBLOCKED. The link gate
`pos_linked` (`engine.rs:4951-4953`) links a `not` when `pat.beta ||
node.active.is_empty() || node.pulse`; the per-left blocker model in
`phreak.rs do_existential_node` (`phreak.rs:957`, doc `1451-1454`) emits the
child while unblocked (`create_ce_child`) and retracts it when a blocker arrives
(`kill_child`). For a temporal `not` this fires IMMEDIATELY at insert — the bug.
It must instead defer to `fire_time` (§2a).

**No fire-scheduler exists — but the shape to imitate does.** `advance()`
(`engine.rs:3946-4005`) only drains time-keyed DELETES: `deadlines` (expirations)
and `window_deadlines` (accumulate evictions), both `BTreeMap<i64,…>` pulled via
`.range(..=clock_ms)`; it emits NO activations — firings "on advance" are
produced by the downstream re-evaluation reacting to those deletes. D-125 joins
and D-127 exists both fire on insert-ARRIVAL (stream-flush), never from
`advance()`. So time-keyed deferred FIRING is genuinely new work.

**Recommended design — schedule the window-close as a REMOVAL, reuse the existing
re-fire path.** This mirrors BOTH the engine's "advance only deletes → re-eval
fires" invariant AND Drools' actual mechanism (source read D-131: the temporal
not fires via `PhreakNotNode.doRightDeletes` when the window-close/blocker is
removed). Concretely:
1. At insert, a temporal `not`-left is held **not-yet-eligible** until its
   `fire_time` (§2a) — represent the open window as a synthetic hold (a
   phantom "window blocker" in the `do_existential_node` model, or a per-left
   `deferred_until` clock checked at eval). It must NOT emit `create_ce_child`
   yet.
2. Add a third time-keyed structure — `fire_deadlines: BTreeMap<i64,Vec<…>>`
   keyed at `fire_time` (the SAME pattern as `deadlines`/`window_deadlines`),
   populated at insert alongside `schedule_expiration`.
3. In `advance()`, after the existing reap, drain `fire_deadlines.range(..=
   clock_ms)`: for each due held not-left, release the hold and let the existing
   `do_existential_node` re-fire path run — it re-searches for a REAL blocker
   (`find_blocker`, `phreak.rs:1735`) and emits `create_ce_child` iff none
   remains. A real in-window blocker (present or already-expired-but-was-present:
   see §2b "blocked⇒silent") keeps it suppressed.
4. **Ordering falls out of the existing PREPEND discipline.** `fire_deadlines`
   drains ascending `fire_time`; each released firing PREPENDs its child (the
   `insert(0,…)`/addInsert discipline already in `do_existential_node`), so the
   net agenda order is DESCENDING fire_time = arc A's "reverse close-time"
   (§2a). The `before,lo>0` IMMEDIATE regime = `fire_time < anchor.ts`: fire at
   the initial fire (no scheduling), FIFO — a `fire_time ≤ clock` fast-path.
5. **Reap-after-fire is automatic:** the fire job at `ts+hi` sits in a lower
   `BTreeMap` key than the anchor's reap at `ts+hi+1`, so the crossing advance
   fires before it reaps (matches the measured `after[0,hi]` fires-at-A+hi,
   reaps-at-A+hi+1).

**Alternative** (if the phantom-hold proves awkward): a dedicated held-firing
queue that emits directly into the agenda in `advance()` — but that breaks the
"advance only deletes" invariant and re-implements the re-fire logic; prefer the
removal-driven design above. Decide at port time.

**Chains (D-131):** the not sits downstream of the D-125 positive join
(`chain_not`: not on `$b`; `not_mid`: not on `$a`). The join already produces
its tuples on arrival (D-125); the temporal `not` defers each surviving tuple by
its own `fire_time`. Confirm the deferral composes with the join's `Kind` (plain
`Kind::Not` at `engine.rs:1502` vs `Kind::SubnetNot` at `:1496` when the not
wraps a subnetwork).

## 4. Reuse map (parallel to D-127 exists / D-125 joins)

### Reaping / inference infra (all present, engine.rs)
- `EventSpec { ts_fi, expires: Option<i64>, dur_fi }` (1022) — `expires`
  Some=finite / None=NEVER.
- `temporal_ub: HashMap<TypeId,i64>` (1050), `never_inferred: HashSet` (1058),
  `explicit_expiry: HashSet` (1044) — the inference state.
- `temporal_edges` + `temporal_pos_type` → `accumulate_temporal_closure` (3378,
  Floyd-Warshall closure so multi-hop chains SUM, per-rule) → `infer_event_expiry`
  (3468) → `spec.expires`.
- `deadlines: BTreeMap<i64,Vec<FactId>>` (1061); `schedule_expiration`
  (3497-3516, the `+1` for rule-referenced types); `advance()` reaper
  (3946-3981) draining `range(..=clock_ms)`.

### Non-temporal `not` machinery — REUSED UNCHANGED (kind-agnostic)
- Link gate `pos_linked` not-arm (`engine.rs:4953`), `maybe_pulse` unconstrained-
  not force-link (`engine.rs:4676-4682`), pulse spend (`engine.rs:5241`).
- The per-left blocker model `do_existential_node` (`phreak.rs:957`, doc
  `1451-1460`): `node.lefts` (unblocked), `node.blocked`/`blocker_of` (PREPEND),
  the `is_not` arms — suppress on right-insert (`phreak.rs:1641-1645`), re-fire
  on right-delete via `create_ce_child` (`phreak.rs:1728-1756`), leftUpd
  (`1828-1857`). **This IS the un-block→fire path the deferral will trigger.**

### Temporal flush machinery — the D-127 exists parallel to imitate
- `stream_flush_ex` (`engine.rs:3603`; sites 3166/4126/6326): eval phase
  (`3760-3779`, consumes existential staging) + per-arrival self-drain cascade
  (`3802-3894`).
- Exists is SKIPPED in the self-drain cascade — `engine.rs:3822`
  `if kind == Kind::Exists { continue }` — and admitted one level down by
  `exists_flush_admit` (`phreak.rs:1918-2002`) under the pure-insert gate
  (`phreak.rs:1488-1494`: temporal + `!is_not` + no upd/del). **A temporal `not`
  likely needs the analogous cascade decision** (don't self_drain_delta a
  temporal not; let eval + the §3B deferral own it) — confirm during the port.
- exists window-close is realized ONLY via reaping (no admission timer); the
  `not` window-close DEFERRAL (fire when no blocker can still arrive) is the one
  piece with no exists precedent — hence §3B's new `fire_deadlines`.

### Clock / reaper — REUSED UNCHANGED (kind-agnostic)
- `clock_ms` (`engine.rs:1031`), `advance()` (`3946-4005`), lazy
  `drain_pending_expirations` at quiescence (`4033-4047`, from the fire_all pop
  loop `4542`). The §3B `fire_deadlines` drain slots into `advance()` beside the
  existing `deadlines`/`window_deadlines` drains.

## 5. Gate / verification plan

- `tools/fuzz_not_temporal.py <n> <seed> <outdir> <repo>` — engine-vs-oracle,
  0-div **modulo the §6 heap ties**. Multi-seed on fresh seeds (population-
  measure, not the discriminating matrix).
- `make diff` — baseline 11 / probes 951 / regressions 284 must stay
  byte-identical (non-`not` and non-temporal paths untouched; gate on
  `!event_specs.is_empty()` + `CeKind::Not` + temporal, like every prior slab).
- `make lint-probes`, `cargo test`, bindings pytest — green.
- Graduate the fenced witnesses `probes_pending/cep/e_recon/cp*not*` (incl.
  `cp_not_chain_defer`) to `scenarios/probes/pr_cep_*`.
- Watch the **D-117 non-termination** region (temporal + re-add cycles): the
  engine spin-guard should catch a runaway, but a deferral scheduler that
  re-schedules on its own firing is exactly the re-add-cycle shape — test with
  the `scenarios/hang-backlog/` shapes.

## 6. The within-close-time order — a scheduler HEAP ARTIFACT (D-131), do NOT chase

Source read (drools-core 9.44): temporal `not` firings drain
`PseudoClockScheduler`'s `PriorityQueue<TimerJobInstance>`, ordered SOLELY by
fire-time (`DefaultTimerJobInstance.compareTo`, no secondary key). Same-close-
time jobs are equal ⇒ their order is a binary-heap artifact of the add/poll
sequence — NOT a clean semantic (same class as the `fz_42_84` identity-hash
quarantine). The models fence it; **~0.6% of chain cases** differ only here.
Port stance: the engine matches these ONLY if Seine's scheduler reproduces
Drools' PQ tie-order; otherwise they graduate to `scenarios/xfail/` as heap-
order expected-divergences — **NOT a firing-set error**. Decide this explicitly
at port time; do not grind the tie.

## 7. Risks / open questions

- **tpos for `not`** (§3.2) — the load-bearing arc-B detail.
- **Scheduler tie-order** (§6) — match or xfail.
- **Non-termination** (§5) — re-add cycle risk in the deferral scheduler.
- **Chains in the engine** — how `chain_not`/`not_mid` compile (subnet-not vs
  not; `Kind::SubnetNot` at `engine.rs:1496` vs `Kind::Not` at 1502) and whether
  the deferral composes with the D-125 join flush.
- **Corpus safety** — re-verify non-temporal `not` byte-identical after each
  change (the known `fz_42_84`-class and `cf313x13` latents are pre-existing).
