# PINS — the justified-aggregation edge (D-304 option b probe round)

Predictions registered 2026-07-18 BEFORE any cell ran (D-308 doctrine).
Mission per HANDOFF.md: reversal-correctness — a logical Release
justified by a stale plain-inserted Balance survives its own reversal.
Fork 0 (then_modify singleton, the cheap safe-pattern candidate) runs
FIRST; fork 1 (does Drools itself support insertLogical-from-accumulate,
f1–f6) runs for the record either way. Stale-release-on-reversal gets
pinned regardless.

## CELLS + REGISTERED PREDICTIONS

### Fork 0 — the then_modify singleton (diffable, in-subset candidate)

Shape: ONE seeded Bal row; `balance` = accumulate sum + guard
`Bal(v != $t)` + `modify($b){ setV($t) }` (the guard kills the
self-loop: once v == $t the rule unmatches). `release` =
`Bal(v <= 0)` → insertLogical(Release). Epoch 0: lines sum to 0 →
Bal set to 0 → Release derives. Epoch 1: +Line(50) → sum 50 →
modify Bal → release LHS unmatches.

- **ja0_singleton_f64** — P0a: BUILDS both sides (risk: the
  accumulate-result binding `$t` used in a LATER pattern's constraint
  is unprobed in our subset — if the engine fences it, that is a
  uniformity finding, not a failure). P0b: epoch-1 modify RETRACTS the
  logical Release via update-driven unmatch teardown (certified
  machinery, D-186..D-211). Final WM: 3 Lines, Bal(50.0), NO Release.
  P0c: engine-vs-oracle IDENTICAL, 3× stable. CONFIDENCE: high on
  P0b/P0c, medium on P0a (engine side).
- **ja0_singleton_dec** — same shape over decimal(10,2), `v <= 0`
  int-literal comparison (the D-309 grid). P0d: same retraction
  result. EXTRA UNPINNED AXIS: DRL-level `sum` over a decimal field —
  engine dec-sum exists (D-097/D-305 machinery) but no scenario cell
  exercises the DRL path, and the ORACLE's built-in sum over
  BigDecimal may degrade to double (unmeasured). Any fence or
  degradation here is its own pin. CONFIDENCE: medium.

### f5 — the stale-fact CONTROL (diffable): plain-insert shape

`balance` plain-inserts `new Bal($t)` per re-accumulation; `release`
as above. Epoch 1 adds Line(50).

- **ja5_stale_plain_f64** / **ja5_stale_plain_dec** — P5: BOTH
  engines leave Bal(0) AND Bal(50) in WM and Release ALIVE at the
  end — the Release survives its own reversal identically. The gap is
  a UNIVERSAL modeling gotcha, not a Seine divergence. Engine-vs-
  oracle IDENTICAL 3×. CONFIDENCE: high.

### Fork 1 — insertLogical from an accumulate rule (ORACLE-ONLY;
### engine_fenced — the D-076 wall rejects these by design)

- **ja1_build** (f1) — `accumulate(...sum...) then insertLogical(new
  Bal($t))`, no epochs. P1: Drools BUILDS it (the wall's rationale is
  OUR TMS revalidation mechanism, not oracle parity;
  ErrorOnInsertLogicalTest was routed for function blocks). Logical
  Bal(0.0) in WM. CONFIDENCE: medium — genuinely unpinned; this is
  the D-304 precision-#2 question.
- **ja2_swap** (f2) — ja1 + epoch [Line(50)]. P2: the old logical
  Bal(0.0) RETRACTS (justifying-match teardown on the accumulate
  re-propagation) and Bal(50.0) derives — the aggregate is
  self-maintaining. Final WM: ONE Bal. CONFIDENCE: medium-low — the
  accumulate result may propagate as a tuple UPDATE whose interaction
  with prior logical insertions from the same match is exactly what
  we cannot predict. This cell is the heart of fork 1.
- **ja3_chain** (f3) — ja2 + release rule. P3: if P2 holds, Release
  retracts through the swap (whole-chain teardown). Final WM:
  Bal(50.0) only, no Release. CONFIDENCE: follows P2.
- **ja4_samevalue** (f4) — ja1 + epoch [Line(7), Line(-7)] (net sum
  unchanged at 0.0). P4: final WM holds exactly ONE Bal(0.0) (value-
  keyed dedup / re-root; no duplicate). WEAK sub-prediction: balance
  re-fires in epoch 1 (accumulate propagates modify without comparing
  values). CONFIDENCE: medium on the WM, low on the re-fire count.
- **ja6_groupby** (f6, only meaningful if P2 holds) — per-key
  accumulate `K($k : k) accumulate(Line(k == $k, ...); sum)` →
  insertLogical(new Bal($k, $t)); epoch adds a line to key 1 only.
  P6: key-1 Bal swaps, key-2 Bal untouched — per-group independent
  maintenance. CONFIDENCE: follows P2.

## DECISION TABLE (pre-registered)

- P0b YES → the safe pattern EXISTS in today's subset: document it
  (sum_ docstring + pinned probes + builder end-to-end test); the
  engine feature drops to uniformity polish. Fork 1 results are
  receipts for the docs.
- P0b NO → fork 1 in earnest; the port design (justifying-tuple
  revalidation for accumulate conditions) goes in the report for
  Bryan's gate, not in code.
- P1 NO (Drools rejects the build) → our D-076 wall is ALSO error
  parity; the wall text can say so.

## MEASUREMENTS (2026-07-18; all cells 3× stable)

Prediction scorecard: **ALL HIT** — P0a/P0b/P0c/P0d, P5, P1, P2, P3,
P4 (incl. the weak re-fire sub-prediction), P6.

### Fork 0 — the safe pattern EXISTS (diff PASS 4/4 ×3)

- **ja0_singleton_f64 / ja0_singleton_dec**: engine-vs-oracle
  IDENTICAL. Epoch 0: balance fires (sum 0), release fires, Release
  derived. Epoch 1 (+Line 50): balance re-fires, modify → the logical
  Release RETRACTS via update-driven unmatch teardown. Final WM: 3
  Lines + Bal(50) — NO Release. The `Bal(v != $t)` guard kills the
  modify self-loop on both sides (firing counts identical: 3 total).
- The accumulate-result binding used in a LATER pattern's constraint
  (`$t` in `Bal(v != $t)`) is IN-SUBSET engine-side (P0a's flagged
  risk did not materialize) — first corpus cell exercising it.
- NEW PIN (bonus): DRL-level `sum` over decimal is EXACT BigDecimal
  on BOTH sides — scale-preserved ("0.00", "50.00"), no double
  degradation. First scenario-level decimal-sum cell.
- GRADUATED: all four → scenarios/probes/pr_ja0_*/pr_ja5_*.

### f5 — the stale control (diff PASS ×3): the gotcha is UNIVERSAL

- **ja5_stale_plain_f64 / ja5_stale_plain_dec**: both engines leave
  Bal(0) AND Bal(50) in WM with Release(1) ALIVE. Plain insert is
  plain insert — Drools has the identical reversal gap. Receipts for
  the docs: this is a modeling gotcha, not a Seine divergence.

### Fork 1 — Drools SUPPORTS self-maintaining logical aggregates
### (oracle-only, 3× byte-stable each; engine fences all five with
### the D-076 wall text — verified)

- **ja1_build**: insertLogical from an accumulate rule BUILDS in
  Drools (no compile error). Logical Bal(0.0) lands in WM. **Our
  D-076 wall is a Seine-side scope cut, NOT error parity.**
- **ja2_swap**: SELF-MAINTAINING — after +Line(50), WM holds ONE
  Bal(50.0); the old logical Bal(0.0) retracted on re-accumulation
  (justifying-match teardown), the new value derived. 2 firings.
- **ja3_chain**: the FULL reversal chain — final WM Bal(50.0) only,
  NO Release; the downstream logical retracts through the swap.
- **ja4_samevalue**: +Line(7)/+Line(-7) (net sum unchanged) → ONE
  Bal(0.0) at quiescence, no duplicate, no observable flicker;
  balance re-fired exactly ONCE in epoch 1 (both staged inserts
  collapse into a single recompute-and-propagate).
- **ja6_groupby**: per-group independence — key-1 Bal swaps to 50.0,
  key-2 Bal(3.0) untouched; 3 firings (2 initial + 1 for key 1).

### Port-scope edge cells (added at Bryan's "do the port", predictions
### registered BEFORE running — the lift covers only measured shapes)

- **ja7_gbce** — the groupby CE as justifier:
  `groupby( Line($k : k, $a : amount); $k; $t : sum($a) )` →
  insertLogical(new Bal($t)) (the key binding is RHS-unusable in
  Drools — pr_ga lane). P7: builds; per-group logical rows (Bal(0.0)
  group 1, Bal(3.0) group 2); epoch +Line(k=1,50) swaps group 1's
  Bal to 50.0, group 2 untouched — same maintenance as ja2/ja6.
  CONFIDENCE: medium-high (same accumulate machinery under the CE).
- **ja8_collect** — `$l : List() from collect( Line(amount > 0.0) )`
  → insertLogical(new Flag(1)). Collect always matches (empty list
  included), so no unmatch reversal is observable; the measurable
  slice is BUILD + same-value dedup across re-collections. P8:
  builds; exactly ONE Flag before and after the epoch insert; one
  re-fire per collection change. CONFIDENCE: medium.
- NOT probed, stays WALLED with precise texts: windowed accumulate
  justifiers (CEP tier, unprobed) and ?query-CE justifiers.

### THE PORT (D-312, Bryan: "do the port") — measured same-day

Edge-cell scorecard: P7 HIT (groupby CE self-maintains per group,
3 firings, group 2 untouched), P8 HIT (collect builds, ONE Flag,
2 firings — value-dedup across re-collections). 3× stable each.

THE LIFT IS MECHANISM-FREE: the accumulate result fact is updated IN
PLACE (eval_acc_node set_value — same FactId), so the act key
(ri, Tup) is STABLE across re-accumulations; the D-076
refire-supersede prologue/epilogue (execute_rhs) already supersedes
keys not re-established by the re-fire, and null-result/left-death
unmatches ride tms_on_terminal_del. The wall was prophylactic —
verbatim the D-296 pattern. Engine change = replacing the blanket
`acc.is_some() || qce.is_some()` wall with two precise fences
(?query justifiers; windowed-accumulate justifiers — both unprobed,
pinned in tms_queryable::d312_acc_justifier_walls).

ALL SEVEN previously-fenced cells + ja9_dec_swap (the decimal money
shape: logical Bal("50.00"), Release retracts through the swap,
exact scale) PASS engine-vs-oracle on FIRST CONTACT, 3× stable →
ALL EIGHT GRADUATED (pr_ja1..pr_ja9, corpus 1292 with D-311's four).
The D-304 audit dead-end CLOSES: the aggregate is logical, so why()
walks Release → Bal; builder end-to-end pytest pins the chain.

P0b YES → the safe pattern (then_modify singleton + `v != $t` guard)
is certified in today's subset: documented + graduated + builder
end-to-end test. P1/P2 YES → the wall is a capability gap vs the
oracle, not parity; a port must solve justifying-tuple revalidation
for accumulate conditions. THE PORT DESIGN QUESTION GOES TO BRYAN'S
GATE — no engine code in this slab (the only engine byte moved is the
ACC_DECIMAL render label, "Decimal"→"BigDecimal", aligning the
accumulate-result box name with the oracle's Java simple name; found
by ja0_singleton_dec, the first cell to ever render one).

## STABLE-ACT-KEY STRESS CELLS (Bryan: "pin the two cells where
## 'stable activation key' is most stressed") — predictions
## registered BEFORE running

The mechanism under stress: the acc result FactId (and so the act
key (ri, Tup)) is REUSED — supersede, terminal-del, and
re-justification all book against one key. The two maximal stresses:

- **ja10_min_unmatch** — the key survives a FULL unmatch/rematch
  cycle. `accumulate( Line($a : amount); $m : min($a) )` →
  insertLogical(new Bal($m)) + downstream logical Release on
  Bal(v <= 0). Epoch 1 deletes BOTH Lines (min over EMPTY = no
  result → propagateDelete → the terminal tuple dies → deps drop at
  the key); epoch 2 inserts Line(5). P10: after epoch 1 the WM has
  NO Bal and NO Release (the whole chain retracts); after epoch 2 a
  fresh Bal(5.0) derives (no Release, 5 > 0). Engine-side the result
  FactId is RETAINED through the unmatch (ctx.result), so the act
  key is bit-identical across the death/rebirth boundary — any stale
  by_act/had_justified bookkeeping shows here. Engine-vs-oracle
  IDENTICAL 3×. CONFIDENCE: high on semantics; the cell exists to
  catch bookkeeping staleness.
- **ja11_self_feed** — the key supersedes ITSELF to fixpoint.
  `accumulate( Bal($v : v); $c : count() )` → insertLogical(new
  Bal($c)) — the rule's own output feeds its aggregate; the SAME act
  re-fires as count moves, each firing superseding the previous
  belief at the same key (D-296 cyclic × D-312 justifier). P11:
  CONVERGES — count=0 → Bal(0) → count=1 → refire supersedes to
  Bal(1) → count still 1 → Bal(1) re-established → quiescent with
  exactly ONE Bal(1). Both sides identical; if instead it runs away,
  fire-limit error-vs-error parity is the acceptable landing.
  CONFIDENCE: medium — the convergence argument is clean, but the
  staging order of the retract/insert pair inside one supersede is
  exactly what could differ between the engines.

MEASURED (same day, 3× stable, both diff PASS → GRADUATED
pr_ja10/pr_ja11): P10 HIT — epoch 1 tears down the WHOLE chain (no
Bal, no Release, zero firings — teardown only), epoch 2 re-derives
Bal(5.0) at the reused key; one reshape was needed first, and it was
ERROR PARITY (min over double: our D-039 wall vs Drools' "constructor
Bal(Number) is undefined" — both reject; the graduated cell uses the
certified i64-min shape). P11 HIT — converges in exactly THREE
firings (derive Bal(0) → supersede to Bal(1) → re-establish Bal(1)),
final WM = ONE Bal(1), interleaving identical on both sides.

## THE D-313 FUZZ AXES (Bryan: "do the fuzzer enhancements") — what
## the first 4200 cases measured

The decimal axis PAID IN THE FIRST 200 CASES: 6 finds, ONE class —
the empty/all-null decimal sum identity. The oracle returns
BigDecimal.ZERO — scale 0, "0"; our "0 at the field's scale" was a
D-098 ruling-2 COMPOSITION, never measured, now falsified. Fix =
result_value returns the ratcheting fold's own (u, s); the follow-on
storage find (fz_313902_80... fz_313901_80): runtime decimals keep
their OWN scale when stored into a field (the oracle's POJO fields
are plain BigDecimals) — coerce's Dec→Dec arm no longer rescales to
the declared scale (ingestion arms unchanged, precision enforced).
The d098_decimals pin updated to the measured value; all 6 finds
fixed and moved to scenarios/regressions as tripwires.

Deep shakedown (3×2000): two more finds, BOTH bisected PRE-EXISTING
(engine outputs bit-identical at 040bccc): fz_313902_761 = a
dec-composition agenda-order latent in the D-080 documented-open
SHAPE (or-branches + exists-over-logical + setFocus + TMS mutation);
fz_313902_1661 = the standing-ledger collect-order family. Both
quarantined to scenarios/xfail/ (bank 52), seeds re-run CLEAN.

Corners the axes deliberately AVOID (unmeasured — future pin
candidates): oracle string-scale ingestion ("1.1" into decimal(10,2)
— the POJO keeps scale 1?), int-JSON values into decimal fields,
decimal eq-literals inside the oracle's D-029 alpha hash groups
(equals() is scale-sensitive; possibly fz_761's mechanism), decimal
setter args.

## THE WINDOWED-JUSTIFIER PROBE ROUND (D-316 candidate; Bryan:
## "other claude is looking at D-312: CEP window maintenance is
## unprobed") — predictions registered BEFORE running

The D-312 wall's stated reason is honest: window EVICTION is a
teardown trigger the ja1..ja11 grid never measured — an event
leaves the window while staying ALIVE in WM, so no fact-death or
delete propagation is involved; the accumulate result changes
"spontaneously" (length: pushed out by an admission; time: the
clock passes ts+N). IF the D-312 mechanism story is complete,
eviction is just another source of change: the result updates in
place, the act key is stable, the refire supersedes. The cells
measure whether DROOLS agrees (oracle-only; the wall fences all
five engine-side by design):

- **w1_build**: length-window sum + insertLogical, no epochs. P-w1:
  builds; logical B(30). CONFIDENCE high (ja1 precedent).
- **w2_len_evict**: + a third event pushes the oldest out of
  window:length(2) — the evicted event STAYS in WM. P-w2: old B(30)
  retracts, B(60) derives, ONE B at the end, the evicted E0 still
  present. CONFIDENCE medium-high — THE cell; if Drools ties
  logical deps to something eviction does not touch, this is where
  it shows.
- **w3_time_evict**: window:time(50ms), events at ts 0/30, advance
  to 60 (ts0 out, ts30 in). P-w3: B(30) → B(20) swap. CONFIDENCE
  follows w2 (same propagation class, clock-driven).
- **w4_empty**: advance past everything — the window EMPTIES while
  both events live in WM. P-w4: sum's identity keeps the rule
  matched → single B(0) at the end (the sum-still-fires contract,
  certified unwindowed). CONFIDENCE medium.
- **w5_chain**: w2 + a downstream logical Release on B(v <= 30).
  P-w5: the Release retracts through the eviction swap (the ja3
  chain, eviction-driven). CONFIDENCE follows w2.

Gate discipline: measurements → this report → BRYAN'S GATE before
the wall moves (the D-312 lift itself was gated; a new wall lift is
a new gate).

MEASURED (same day; all five 3× byte-stable, engine fences verified
by the lint tier): **P-w1..P-w5 ALL HIT.** w1 builds (logical
B(30)). w2 — THE cell — the length-window eviction swaps the
logical: ONE B(60) at the end, the evicted E0(v=10) STILL ALIVE in
WM; Drools' logical maintenance keys on the accumulate result
change, not on any fact-death path. w3: the time-window swap via
clock advance (B(30) → B(20), ts0 evicted-but-alive). w4: the
window EMPTIES while both events live — sum's identity keeps the
rule matched, single B(0) (the certified unwindowed contract holds
under windows). w5: the full chain — release fires at B(30), R
retracts through the eviction swap. VERDICT: the D-312 wall's
stated reason is measured away — window eviction is exactly
"another source of change" to the stable act key; the lift is
predicted mechanism-free (stage_acc_removal → unapply → in-place
result update → refire-supersede is the same certified path). AT
BRYAN'S GATE for the wall lift.
