# PINS — exact decimal average (D-314; Bryan: "do the exact decimal
# average slab, half_up default")

Predictions registered 2026-07-18 BEFORE any cell ran.

## THE DESIGN QUESTION AND WHY THE ORACLE CAN STILL DECIDE IT

Drools has NO decimal average — its `average` is IEEE double
(D-098 pin J, certified). So `averageExact` is engine-native with no
standard DRL spelling and can never enter the auto-diff corpus. The
semantics we implement are java.math's:

    result = sum.divide(BigDecimal.valueOf(count), scale, mode)

and THAT is expressible in raw Drools: a multi-function accumulate
`$s : sum($x), $c : count()` plus an RHS
`$s.divide(new java.math.BigDecimal($c), SCALE,
java.math.RoundingMode.MODE)` insert. The campaign runs the SAME
fact vectors through both spellings — the oracle's explicit-divide
program and our `averageExact($x, scale, mode)` — and compares the
result values. Neither spelling runs on the other side (our engine
walls RHS method calls; Drools rejects the unknown accumulate
function), so the comparison is value-for-value in this pin round,
not an auto-diff; the graduated protection is engine tests + pytest
(the why()/acc_sources precedent).

## THE GRID

Modes (all of java.math.RoundingMode except UNNECESSARY):
up, down, ceiling, floor, half_up, half_down, half_even.

Vectors (chosen so modes DISAGREE and signs flip):
- V1 half-boundary positive: (0.02, 0.03) → avg 0.025 @ scale 2
  predictions: up/ceiling/half_up 0.03; down/floor/half_down 0.02;
  half_even 0.02 (2 is even).
- V2 half-boundary negative: (-0.02, -0.03) → avg -0.025 @ scale 2
  predictions: up/half_up -0.03 (away from zero); floor -0.03;
  down/ceiling/half_down -0.02; half_even -0.02.
- V3 non-terminating positive: (1.00, 1.00, 1.01) → 3.01/3 =
  1.00333… @ scale 2: up/ceiling 1.01; all others 1.00.
- V4 non-terminating negative: (-1.00, -1.00, -1.01) → -1.00333… @
  scale 2: up/floor -1.01; down/ceiling/half_* -1.00.
- V5 exact division (no rounding): (1.10, 2.20) → 1.65 — ALL modes
  1.65 (the mode must be a no-op when the division terminates at
  scale).

P1: our i128 rounded division equals the oracle's BigDecimal.divide
on ALL 35 grid cells (7 modes × 5 vectors), 3× stable.
CONFIDENCE: high — java.math RoundingMode semantics are documented;
the grid exists to catch OUR sign-handling bugs (negative truncation
in Rust is toward zero, and half-comparisons must be on magnitudes).

P2 (empty/all-null): count == 0 → NO propagation, like `average`
(oracle: the multi-acc spelling still fires with count 0 and RHS
divide would throw div-by-zero — a spelling artifact, not a
semantic: our averageExact follows `average`'s certified
empty-blocks-propagation contract; documented, not diffed).

P3 (null skips): nullable decimal sources skip null contributions in
BOTH sum and count (the D-097 skip is uniform in AccCtx); average of
(1.00, null, 2.00) @ scale 2 = 1.50.

P4 (scale default, authoring): `average_exact(Line.amount)` over
decimal(18,2) defaults to scale 2 (the SOURCE scale) and HALF_UP
(Bryan's ruling); result subset_type decimal(38,2) — results are
never null (like sum, per D-306's result-typing).

## MEASUREMENTS (2026-07-18, same day — nothing below existed before the cells ran)

**P1 HIT: 35/35 grid cells MATCH** — oracle (multi-acc + RHS
BigDecimal.divide, 3× stable per cell) vs engine (averageExact),
value-for-value across 7 modes × 5 vectors. Sign handling exact:
-0.025 rounds away-from-zero to -0.03 under up/half_up, toward zero
under down/ceiling/half_down, floor takes -0.03, half_even takes the
even neighbor -0.02 — identically on both sides. V5 confirms every
mode is a no-op on terminating division.

Engine vectors additionally pin (engine/tests/dec_avg.rs): half_even
parity at 0.035 → 0.04; scale narrowing (0.025 @ scale 0 → "0"
half_up, "1" up/ceiling); scale widening (0.0250 @ 4); P2 HIT
(empty AND all-null block propagation, like average); P3 HIT (nulls
skip both sum and count: avg(1.00, null, 2.00) = 1.50). Walls loud:
i64 source steers to average; unknown mode lists the seven; scale
caps at 38.

Cells stay PENDING by design: the oracle spelling engine-fences on
multi-function accumulate + RHS method calls; the engine spelling is
oracle-unknown. The graduated protection = the vector suite + pytest
(the why()/acc_sources precedent).

# THE WINDOWED ROUND (2026-07-19, Bryan: "D-314" — the windowed
# average_exact authoring fence, the ledger's own ox/ex round)

Predictions REGISTERED before any cell runs. Same doctrine as the
D-314 grid: the oracle cannot run averageExact (unknown function)
and the engine walls the oracle's spelling (multi-acc + RHS
BigDecimal.divide), so certification is value-for-value on the
ORDERED B-fact sequences — each refire inserts a B; the sequence
pins every intermediate window state. All consecutive expected
values are DISTINCT by vector design (sidesteps refire-on-equal
comparisons between the two spellings, which differ there by
construction: the oracle's sum/count pair always changes).

Mechanical basis (code-read): AccCtx is uniform — AverageExact has
a subtract-based try_reverse (exact, no refold), so window eviction
rides the same certified machinery as windowed sum/count/average.
The DRL parser accepts `over window:*` + averageExact (no
restriction); the fence is AUTHORING-LAYER ONLY (authoring.py).
PREDICTION WP6: the lift needs ZERO engine-side change.

- WP1 window:length(2) ring (events @10/@20/@30, vals 0.01, 0.04,
  0.05; scale 2): oldest-out slot retention; states {0.01} →
  {0.01,0.04} (avg 0.025, the half boundary) → {0.04,0.05} (avg
  0.045, again a half boundary). PREDICT sequences — half_up:
  [0.01, 0.03, 0.05]; half_even: [0.01, 0.02, 0.04] — engine
  (ewl_*) == oracle (owl_*, 3x) cell-for-cell.
- WP2 window:time(100ms) (events 1.00@10, 1.03@20, 1.07@30;
  advances 10/10/10/85/10 → evict @10 at t=115, @20 at t=125):
  eviction refires with the survivor average. PREDICT — half_up:
  [1.00, 1.02, 1.03, 1.05, 1.07]; half_down: [1.00, 1.01, 1.03,
  1.05, 1.07] (the 1.015 midpoint splits the modes; 1.05 exact
  division = mode no-op) — engine (ewt_*) == oracle (owt_*).
- WP3 scale-ratchet invisibility (window:length(2), vals 0.02
  (s=2), 0.0350 (s=4), 0.05 (s=2), half_up @ scale 2): the sum's
  ratcheted internal scale never shows — divide re-normalizes to
  the SPELLED scale every firing, both sides. PREDICT [0.02, 0.03,
  0.04] (0.0275 → 0.03; eviction of 0.02 → avg 0.0425 → 0.04).
- WP4 empty window + refill (ENGINE-ONLY, vs the certified
  windowed-average CONTROL): advancing past the last event's
  window BLOCKS propagation (count 0 — P2 extends to windows,
  NO firing on the eviction-to-empty), a later event re-derives.
  ewe (averageExact): PREDICT [1.00, 2.00] with no firing at the
  empty transition. cwe (average, DIFFABLE — certifies the firing
  PATTERN oracle-side): PREDICT same pattern, values 1.0/2.0.
  The oracle twin of ewe is IMPOSSIBLE by the P2 artifact: the
  multi-acc spelling fires count=0 and the RHS divide throws.
- WP5 null slot (pytest, post-lift): a null-field event OCCUPIES a
  window:length slot but contributes to neither sum nor count
  (add-skip and reverse-skip are both D-097-uniform). Ring
  {null, 0.04} → avg 0.04. Oracle-unknowable (count() counts
  matches, not contributions — the P3 artifact).
- WP6: the fence lift = authoring.py only (remove the window arm;
  group_by fence STAYS — separate uncertified surface); pytest
  fence test flips to positive coverage.

## THE WINDOWED MEASUREMENTS + A NAMED FINDING (same session)

The count-0 hazard hit EARLIER than designed: the harness fires
once at the BASE (empty session) before epoch 0 — the oracle
multi-acc cells threw / by zero there. Restructure (recorded):
the first event moved into the BASE facts on all 10 o/e WL/WT/WSC
cells — no count-0 fire ever; sequences unchanged.

WP1 HIT (owl/ewl, oracle 3x): hu [0.01,0.03,0.05], he
[0.01,0.02,0.04] — engine == oracle cell-for-cell.
WP2 HIT (owt/ewt): the oracle FIRING order (read off the bound
$s sums 1.00→2.03→3.10→2.10→1.07) reconstructs per-firing values
hu [1.00,1.02,1.03,1.05,1.07] / hd [1.00,1.01,...] — engine
IDENTICAL. (The WM facts-list ORDER of the B inserts differs
between the runners' dumps — a canon artifact of never-diffed
cells, not semantics; the firing sequence is the surface.)
WP3 HIT (owsc/ewsc): [0.02, 0.03, 0.04] both sides — the ratchet
never shows.
WP4 ewe HIT: [1.00, 2.00], 2 firings, NO firing at the
empty transition.

**THE NAMED FINDING — pin J FALSIFIED for decimal sources** (the
cwe control caught it): `average($a)` over a BigDecimal-typed
field in Drools 9.44.0.Final resolves to
BigDecimalAverageAccumulateFunction (drools-core sources, read
verbatim), NOT the double function:
1. result type = BigDecimal (engine today: F64 per pin J);
2. result = total.divide(count, RoundingMode.HALF_EVEN) — the
   2-arg divide keeps the DIVIDEND's scale = the running sum's
   ratcheted scale (the D-313 ratchet law), banker's rounding;
3. count == 0 → BigDecimal.ZERO (scale 0, "0") and it FIRES —
   measured: the windowed control fires "0" BOTH at session start
   and at eviction-to-empty (engine: blocks, 2 firings vs the
   oracle's 4);
4. null contributions skip both total and count; reverse is
   subtract-based (window-exact).
The double function (read verbatim): count == 0 ? null :
total/count — null BLOCKS, which is the certified engine
behavior for i64/f64 sources. The asymmetry is per-source-type.
Pin J's provenance: the D-098 "AVG(decimal)→DOUBLE" row was a
DUCKDB measurement, adopted by a D-097-checkpoint alignment
ruling ("matches the certified average→f64" — certified on
i64/f64 only); ZERO corpus cells combine average( with decimal
types, so the claim was never oracle-protected. GATE ITEM FOR
BRYAN (it revisits his D-097 ruling): (a) port the Drools law
(BigDecimal result @ sum-scale HALF_EVEN, empty fires "0"), (b)
WALL average-over-decimal at authoring+DRL steering to
averageExact (the money-never-meets-floats thesis option), or
(c) keep + document the deviation. Until ruled, average(decimal)
stays as-is and UNCERTIFIED.

cwe REDESIGN (the control's job is the WP4 firing PATTERN on
certified surface, not the decimal law): source field becomes
f64 (values 1.0 / 2.0) — windowed average over f64, empty
blocks per the double function. PREDICT: diff PASS, 2 firings
[1.0, 2.0], no empty-transition firing.

## THE ROUND'S CLOSE (receipts)

cwe PREDICTION HIT: the f64 control PASSES the real diff —
GRADUATED to scenarios/probes/pr_cep_win_avg_empty_refill.json
(the one diffable cell of the round; corpus 11/1519/414).
WP5 HIT (pytest): the null-px event occupies its length-window
slot, contributes to neither sum nor count — ring {null, 0.04}
→ 0.04 (a null-skipping ring would give 0.03).
WP6 HIT: the lift = authoring.py only (the window arm removed;
group_by fence stays); ZERO engine-source change (git diff --
engine/ EMPTY — no byte gate needed, the certified binary is
the D-339 one).
One recorded test-authoring miss (not a semantic miss): the
first pytest advance was 115 — evicting only the @10 event
(@20 lives to 120); the "failure" value 1.52 = (1.03+2.00)/2
half_up, exactly right for the window actually left. Fixed to
advance(125); the block assertion tightened to the per-fire
delta contract (fire() derived shows THIS fire's rows).
pytest 257→260; lint 2378/0/0; cargo 74; make diff 11/1519/414
+ drift 18 identical; demo True. The 11 o/e cells stay PENDING
by design (the D-314 doctrine: value-for-value, never diffed).
CHANGELOG Unreleased carries the windowed-average_exact entry.
OPEN ON THIS LANE: the pin-J gate item above (Bryan's ruling
on average-over-decimal) — until ruled, average(decimal) is
as-was and uncertified; group_by × average_exact stays fenced.

# D-341: BRYAN'S RULING — WALL IT (2026-07-19)

"per diem rate as well as interest rates can't be floats, then,
if we wall it, which seems like correct behavior to wall." —
option (b): average over a decimal source is OUT OF SUBSET,
steering to averageExact. The D-097 item-4 thesis extends to
aggregation: money never meets floats. Pin J's engine behavior
(AVG(decimal)→F64) is REMOVED, not ported-to-Drools — neither
the silent float coercion nor Drools' BigDecimalAverage
(sum-scale HALF_EVEN, empty fires ZERO) survives; the exact
tool (averageExact) is the steer.

THE PORT SURFACE (recon, all reachable average+decimal paths):
1. engine add_rules_drl (the acc validation, next to the
   averageExact dual wall) — the one engine-side gate; covers
   DRL accumulate AND groupby (shared validation, upstream of
   the group_key arm).
2. engine/tests/d098_decimals.rs pin-J assertion (AVG(decimal)
   is DOUBLE) — FLIPS to the wall assertion.
3. authoring.py average() construction — the decimal wall,
   mirroring average_exact's dual (its startswith("decimal")
   idiom covers Optional decimals too, proven by WP5); covers
   accumulate AND group_by (construction-time).
4. bindings/tests/test_authoring.py:451 (subset_type == "f64"
   for average over decimal) — FLIPS to raises.
5. tools/fuzz_duckdb.py acc_ce: average may draw a decimal
   field — steered to i64/f64 (the harness gen.rs already
   restricts decimal to sum-only since D-313; unchanged).
6. Corpus + pending cells: ZERO combine average( with decimal
   (measured D-340); the harness fuzzer never emits the combo.
   Blast radius prediction: byte gate 100% identical.
7. OUT OF SCOPE (recorded): seine_rs.derive's mean over decimal
   — the DataFrame layer is the SQL/DuckDB-aligned contract
   (Bryan's D-097 item-5 scope), not the rules accumulate.

## D-341 receipts

The wall landed at all 5 surfaces. ONE unpredicted mover, decoded:
scenarios/duckdb/dk_dec_acc.json (the duckdb TIER, which the
average+decimal recon greps missed — scenarios/duckdb/ is neither
probes nor phase*) carried R2 = the pin-J average cell; R2
removed, the other four rules stay DuckDB-certified. BONUS FIND
while re-certifying: the diff-duckdb comparator itself had been
STALE-BROKEN since the D-308 BigDecimal rename (acc results type
"BigDecimal" now; the comparator's tuple said "Decimal" →
KeyError on the handle arm) — diff-duckdb is NOT in the standard
battery and had not been run since. Comparator fixed (one tuple
entry), gate 11/11 GREEN again.

Battery: byte gate vs bea549e 2505/2506 — the ONE diff is the
edited dk_dec_acc itself (verified, gate-green); blast-radius
prediction HIT (nothing else in 2506 outputs moves). make diff
11/1519/414 + drift 18 identical; diff-duckdb 11/11; lint
2378/0/0 (the ghost dk_dec_acc threw was the wall working —
fixed by the R2 removal); cargo 74; pytest 260; demo True; SD
census 71 EXACT; fuzz 2x2000 seeds 340001/340002 CLEAN (NEXT
341001+... note duckdb-fuzz consumed 341001 as a seed label);
duckdb-fuzz 200 cases seed 341001 with the steered generator:
0 divergences. CHANGELOG Unreleased: the wall entry added
(FOUR pending entries). Pin J's row in engine tests/docs now
reads as superseded by D-341.
