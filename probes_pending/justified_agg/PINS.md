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
