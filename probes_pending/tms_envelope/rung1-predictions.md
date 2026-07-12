# Rung 1 — L-SD (self-defeat landing) ladder: PREDICTIONS, logged pre-run

_2026-07-11, D-186 plan §5 rung 1. Written and saved BEFORE any cell was
run (house discipline; the D-177 predictions.md pattern). Skeleton = the
min812 spine at k=0: justifier RJ = `not LK() -> insertLogical(LK)`
firing off the InitialFact; the observer's node class and salience are
the two axes. All cloud, all 2-type (LK logical, no others needed),
facts empty. Oracle 3x per cell (the D-080 TMS bar)._

## The question this rung asks

min812's oracle glimpse (sibling accumulate fires once on the transient
at EQUAL salience) contradicts a naive transfer of the certified drain
point (a) (min608: equal salience does not preempt the post-firing
continuation, which drains). Either (i) the equal-salience drain does
not cover the justifier's OWN defeat (a k=0/self-break cause split), or
(ii) the observer's NODE CLASS (accumulate vs plain join) changes what
survives, or (iii) the equal-salience bucket's pop order is the real
variable. The 2x3 grid separates (i) from (ii); the a8 dual-observer
cell splits the two readings any single RED cell leaves underdetermined
(method law: the splitter is designed BEFORE the mechanism claim).

## Cells + predictions

| cell | observer | sal | ORACLE prediction | ENGINE prediction | basis |
|---|---|---|---|---|---|
| sd_a2_plain_eq | plain `LK($v:f0)` | 0 | [RJ] (1 firing) | [RJ] | min608: equal does not preempt; continuation drains. LOW-MED confidence — if [RJ,RO], reading (i) wins immediately and the acc axis dies |
| sd_a3_plain_hi | plain | +10 | [RJ, RO] (2) | [RJ, RO] | t11 transfer: strictly-higher witness fires on the transient; engine implements drain point (a). GREEN control expected |
| sd_a4_plain_lo | plain | −10 | [RJ] (1) | [RJ] | nothing higher waits; continuation drains. GREEN control, highest confidence |
| sd_a5_acc_eq | `$l:LK($v:f0)` + `accumulate(LK(f0==$v); count())` | 0 | [RJ, RO] (2) | [RJ] (1) | THE min812 SHAPE itself (the xfail record). RED expected — this cell renames the recorded behavior into a clean ladder cell |
| sd_a6_acc_hi | acc | +10 | [RJ, RO] | [RJ, RO] | min_1310 + t11: acc on a transient at higher salience is certified. GREEN control expected |
| sd_a7_acc_lo | acc | −10 | [RJ] | [RJ] | as a4. GREEN control expected |
| sd_a8_dual_eq | acc RO @0 AND plain RO2 @0 (decl order RJ, RO, RO2) | 0/0 | M1 ⇒ [RJ, RO, RO2] (some order); M2 ⇒ [RJ, RO] only | [RJ] | THE SPLITTER — no single prediction pinned (method law). M1 = the equal-salience drain is skipped for the self-break entirely (plain glimpses too when an acc coexists is NOT required — M1 proper predicts RO2 fires because the drain never landed before the equal bucket drained). M2 = the drain lands at the continuation but the acc's materialized activation survives it (a cancellation-reach gap) — then RO2 (plain) dies and only RO fires |

Grid logic: a2 vs a5 isolates the node-class axis at equal salience;
a3/a4/a6/a7 are the transfer controls (all four predicted GREEN
engine==oracle — if any is RED the certified-pin transfer itself fails
and the row is bigger than the compound story). a8 splits M1/M2 after
a5 confirms.

Falsifier bookkeeping (pre-committed): if a2 comes back [RJ, RO]
(plain glimpses at equal salience), then min608's drain-point-(a) does
NOT govern self-defeat breaks at all — reading (i) — and the acc axis
is an artifact; a5/a8 then only corroborate. If a5 comes back [RJ]
(no glimpse), the min812 record's mechanism is NOT node-class and NOT
salience — the compound needs a different load-bearing ingredient
(min812's R1 also carries a positive LK pattern binding + eq-join into
the acc; sd_a5 preserves that: `$l:LK($v:f0)` + `f0==$v` join).

Oracle stability: any cross-launch flip on any cell → quarantine the
cell per the fz_42_84 doctrine and say so; a flaky cell is not
evidence.

## Round-2 (written AFTER round-1 results, BEFORE round-2 runs)

ROUND-1 RESULT: all 7 GREEN 3×-stable — **a5 (acc,eq) did NOT glimpse**
(prediction falsified). Per the pre-committed falsifier: the min812
ingredient is neither node-class nor salience. Skeleton diff vs min812:
min812 declares the OBSERVER FIRST (R1) and the justifier SECOND (R2);
every round-1 cell declared RJ first. New hypothesis H-DECL: the
equal-salience bucket pops by RULE DECLARATION ORDER (Drools
RuleAgendaItem loadOrder), and the self-break's belief drop lands at
the justifier's ITEM POP, not at its post-firing continuation — so an
observer declared BEFORE the justifier pops first and glimpses; declared
after, the justifier's pop drains it first. (min608's continuation-drain
pin must then scope to a different break origin/shape — the cause-split
fine print this row exists to write.)

| cell | shape | ORACLE prediction (H-DECL) | ENGINE prediction | note |
|---|---|---|---|---|
| sd_a9_declorder_plain | plain RO @0 declared FIRST, then RJ | [RJ, RO] | [RJ] (continuation-drain as implemented) | RED expected — the row's first confirmed cell if it lands |
| sd_a10_declorder_acc | acc RO @0 declared first (min812-minimal) | [RJ, RO] | [RJ] | RED expected — renames the min812 record |
| sd_a11_declfirst_lo | plain RO @−10 declared first | [RJ] | [RJ] | salience dominates decl order; GREEN control |
| sd_a12_declfirst_hi | plain RO @+10 declared first | [RJ, RO] | [RJ, RO] | GREEN control (t11 transfer unchanged) |
| sd_a13_dual_declsplit | plain RO @0 declared BEFORE RJ; plain RO2 @0 declared AFTER RJ | [RJ, RO] (RO fires, RO2 does not) | [RJ] | the queue-position splitter: same class, same salience, only decl position differs |

If a9/a10 come back [RJ] (no glimpse even observer-first): H-DECL dies
too; the next candidates from the skeleton diff are (in order) the
acc-source alpha (`f2 > 0`), the count==0-vs-1 fold, the unused third
type, and the multi-field type — one cell each, no compound guessing.

## Round-3 (written AFTER round-2 results, BEFORE round-3 runs)

ROUND-2 RESULT: H-DECL landed 5/5 exact (a9/a10/a13 RED with the oracle
glimpsing observer-declared-first; a11/a12 GREEN salience controls;
a13's split — RO fires, RO2 does not, same class same salience — puts
the drain exactly at the justifier's queue slot). 3×-stable throughout.

RECONCILIATION REQUIRED before any row is written: D-076 drain point
(a) was pinned "equal salience/earlier decl does NOT preempt" on the
min608 family — apparent head-on contradiction with a9. The family
source fz_7_608 (green regression) differs from a9 in TWO ways: its
justifier R4 carries **no-loop** (⇒ the EAGER drain point (b) governs,
flush-time, queue-order-independent) and it is **k=1** (positive T0
pattern) where a9 is k=0. One of these splits the row:

| cell | shape | H-EAGER prediction | H-K prediction | ENGINE prediction |
|---|---|---|---|---|
| sd_a14_noloop_declfirst | a9 + `no-loop` on RJ (k=0) | oracle [RJ] (flush drain; no glimpse) | oracle [RJ, RO] (k unchanged ⇒ glimpse) | [RJ] |
| sd_a15_k1_declfirst | a9 + positive `P()` pattern on RJ (k=1, NO no-loop; P(1) initial fact) | oracle [RJ, RO] (lazy ⇒ pop landing ⇒ glimpse) | oracle [RJ] (k=1 ⇒ drained) | [RJ] |

H-EAGER ⇒ (a14=[RJ], a15=[RJ,RO]); H-K ⇒ (a14=[RJ,RO], a15=[RJ]);
both-[RJ] ⇒ the split is conjunctive (needs both) — further cells;
both-glimpse ⇒ fz_7_608's non-glimpse needs a different ingredient
(its observers' own alpha misses? R2 DOES match the inserted T1 —
re-derive from its record before more cells).

# Rung 2 — the outliers + the fan-out spine (predictions BEFORE runs)

CACHE READ (no new runs; 10-replicate sequences, all 10/10 stable):
fz_123_9133 oracle = [R2, R1, R1, R1] — the justifier fires ONCE (its
remaining 2 activations die in-firing at its own self-break) and the
decl-before observer glimpses with ALL THREE of its activations;
engine = [R2] (clause-A loss). fz_123_3060 oracle = [R0,R0,R3], engine
fires R3 TWICE — R3 is a LEADING-not k=1 lazy justifier with fan-out 2;
the engine fails the in-firing cancellation. fz_7_9375 oracle =
[R3,R2,R1], engine fires R2 twice — R2 is an OR-TWIN CE-only
self-justifier (no-loop): one item, two branch activations; engine
fires both. 9133's trailing-not justifier engine-cancels correctly ⇒
the engine's gap is TOPOLOGY-dependent (leading-not / or-twin miss).

H-CLAUSE-B: the self-break's drop hits the justifier's OWN remaining
same-item tuples IN-FIRING (before the next tuple fires), in BOTH
eager and lazy regimes; clause A (item-pop landing, queue-position
glimpse) governs other rules. The over-fire outliers are clause-B
engine violations, not counterexamples to clause A.

| cell | shape | ORACLE pred | ENGINE pred | basis |
|---|---|---|---|---|
| sd_b1_fanout_trailing | RJ = `P() not LK() -> iL` lazy, P(1) P(2) | [RJ] ×1 | [RJ] ×1 | 9133's trailing-not: engine cancels; GREEN control |
| sd_b2_fanout_leading | RJ = `not LK() P() -> iL` lazy, P×2 | [RJ] | [RJ, RJ] | 3060 minimal; RED |
| sd_b3_ortwin_lazy | RJ = `(not LK(f0==7)) or (not LK(f0==7)) -> iL` | [RJ] | [RJ, RJ] | or-twin gap extrapolated to lazy; MED confidence — if GREEN, the or-twin gap is eager-only |
| sd_b4_ortwin_noloop | b3 + no-loop | [RJ] | [RJ, RJ] | 9375 minimal; RED |
| sd_b5_fanout3_obs | trailing-not k=1 fan-out-3 + plain RO decl-FIRST @0 | [RJ, RO, RO, RO] | [RJ] | 9133 minimal (clause A: the observer's item fires its WHOLE tuple list at its pop); RED |
| sd_b6_leading_obs | b2 + plain RO decl-FIRST @0 | [RJ, RO] | [RJ, RJ] | both clauses in one cell; RED both ways |

Engine predictions are falsifiable as minimality checks: an engine
mismatch means the constructed cell missed the compound's load-bearing
ingredient. After verdicts: a desk retrodiction sweep of ALL 13 L-SD
bucket members' cached sequences against the two clauses — the row is
not done until every bucket member is accounted for (Bryan's bar).

## Rung-2 round 2 — the third regime (written after the sweep + the
## 5213 trace read, BEFORE the cells run)

SWEEP RESULT: 11/13 fit clauses A+B at the count level. The two
misfits share one signature: the JUSTIFIER under-fires engine-side
(5213 R3 −7; 1353 R0/R2 −4 each, triple-justifier cascade). The 5213
trace decodes as STRICT ALTERNATION: per round, the lazy justifier@7
fires once (clause B cancels its other tuples), the drop lands at its
item pop, the no-loop deleter R4@0 re-derives and fires exactly ONCE
— then halts because the justifier's re-queue is strictly higher (the
D-091 halt rule) — and the deleted T0 is a left-side WM event that
re-propagates (the certified t15 revive), refreshing the dead-blocker
leak. Ten [R3, R4] pairs. The engine batches R4 and starves R3's
refires (×3 vs ×10). t10's no-refire pin is NOT violated — it scopes
to no-WM-change cycles.

H-CLAUSE-C (the refire-alternation regime): after the drop lands, a
left-side WM change (t15) re-derives the justifier's remaining
tuples; its re-queued item competes at its salience — strictly-higher
re-queues preempt the changer's item after ONE firing (D-091 halt).

| cell | shape | ORACLE pred | ENGINE pred |
|---|---|---|---|
| sd_c1_alternation | RJ@7 lazy `P($x:f0) not LK2(f1 != true) -> iL(LK2($x,false))`; RD@0 no-loop `$p:P() not LK2(f1 != true) -> delete($p)`; P×3 | strict pairs: [RJ,RD]×3 (RJ ×3, RD ×3) | RJ under-fires, RD batches — [RJ, RD, RD, RD]-like, RJ count < 3 → RED |
| sd_c2_no_deleter | RJ@7 alone, P×3 | [RJ] once (t10: no WM change ⇒ no refire; dead-blocker leak) | [RJ] → GREEN control |

If c1's oracle does NOT alternate strictly (e.g. RD fires twice in a
row), the halt-gating hypothesis is wrong and only the t15-refire part
stands — pin the output, split later. If c2 refires, t10's scope is
narrower than pinned — surface loudly (standing-pin contradiction).
