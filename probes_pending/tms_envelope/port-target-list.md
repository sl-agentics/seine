# The port target list (D-196 A/B + the 31-witness re-read)

_2026-07-12, post-0-div. Inputs: the engine A/B on the interposer
cells (4/6 green — the lazy rows a1/a2/b1/b2 are ALREADY engine-
correct) + all 31 envelope VALUE witnesses run through oracle+engine
with divergence signatures (this sitting; signatures reproducible
from scenarios/xfail/). Census baseline: 47-56 divergent/150 per
seed. Every mechanism below is a confirmed-table row; the port is
translation (Bryan). Finals are equal on 30/31 (fz_7_1353 the one
FINALS-DIFF — the known A+B+cascade compound)._

## Mechanisms, ranked by witness coverage

- **P1 — the eager run-end landing.** A no-loop justifier's
  belief-drop lands when its firing run ends; pending ≤-salience
  activations on the transient are cancelled before any fires
  (ip_a3: engine lets BOTH @10 and @5 fire; oracle neither).
  Covers the L-MB over-family's core (~16/18 witnesses) with P2.
- **P2 — mid-run drain, net-out, and run-continuation.** The
  justifier's RHS pair drains between its own firings; the break's
  teardown at the between-firings eval nets the LK's queued network
  insert (observers never see mid-run generations); observers
  dirtied mid-run do not preempt the run (ip_c1: the engine fires
  RO2+RI on EACH generation and lets them preempt RJ). The RHS-order
  race (mutfirst/ilfirst) and the last-key pop window ride on this
  machinery (gt13/gt14/x147/x131-class).
- **P3 — the equal-salience queue-position window.** The lazy drop's
  pop-landing spares SAME-salience observers whose queue position
  (decl order) precedes the justifier's; the engine instead drains
  equal-salience continuations wholesale (the min608
  over-generalization, D-187). Covers the L-SD under-family:
  min812, fz_123_{2135,3370,4318,7637,9133}, fz_777_9637, fz_7_812,
  fz_7_9864 (~9-11 witnesses; 9133 = ×3 generations of it).
  ⚠ D-106-ADJACENT: the drain lives in the executor region —
  agenda_open ×19 receipts mandatory; port LAST.
- **P4 — clause-B in-firing self-cancellation** on leading-not /
  or-twin topologies (the justifier's own insert breaks its not
  mid-firing; remaining same-item tuples never fire). The two L-SD
  over-cells fz_123_3060 / fz_7_9375; battery cells sd_b2/sd_b4.
- **P5 — clause-C post-drop re-derivation alternation.** A left-side
  WM change re-derives + strictly-higher re-queue preempts the
  changer after ONE firing (strict alternation; engine batches).
  fz_42_5213 (R3 −7); battery cell sd_c1.
- **P6 — member-order physics** (folds, staged-consume orders, gt9
  relocation, the decl-axis) — the ORDER-only residue once P1-P5 fix
  the firing SETS; model_sd's order layer is the spec.
- fz_7_1353 = P3+P4+cascade-persistence compound (the FINALS-DIFF
  resolves when its 8 lost firings return).

## Round-1 status (same sitting): P1+P2 LANDED

The engine's deferral gained the table's CAUSE MODEL — three lanes +
the race flag on `tms.deferred` (bit0 LIA-hit / bit1 NOT-side / bit2
late-dep / bit3 join-right):

- **NOT-side (self-defeat)** entries flush-drain unconditionally at
  the run end (ip_a3: the eager k0 drop now lands before ANY
  ≤-salience pop — new `right_touched`, NOT-kind nodes only).
- **LIA-hit** entries keep the certified t20 discipline (flush) —
  EXCEPT a LATE-dep act's last entry, which rides to the pop. The
  late flag is the D-195 race read LIVE at insertLogical: a MUTFIRST
  consequence already broke its own tuple's alpha when the dep
  attaches (gt13/ip_c1 zombie window); ilfirst deps attach whole and
  die at the flush (pr_tms_t20d / pr_tms_selfbreak_flush stayed
  certified-green, and the property-reactivity split inside the t20
  family — a/b/c/selfbreak_lazy certified POP — held via the
  watch-gated LIA staging that feeds left_touched).
- **Join-right** entries (the LEAD topology's P side) flush-drain
  MID-RUN only (run_live = the item's queue is non-empty); the last
  firing's entry rides to the pop — ip_c1's mid-run net-out +
  last-key window both exact, INCLUDING the gt9 pairing order.

Receipts: ladder 6/6 ENGINE-GREEN; corpus 11/1124/355 byte-identical
(the t20 six all green); agenda_open ×19 byte-identical (⚠ D-106,
measured twice); cargo test 9 suites; lint 1723/0/0. **fz_123_941
GRADUATED out of xfail** (10/10 converged both sides — its I-RD
divergence had a landing component; now a regressions/ cell);
fz_123_9175 moved toward the oracle (5→4 firings, rebanked — drift
bank now 74). Census: 7001 47→38, 6003 54→47 at mid-round; the full
12-seed post-round baseline in the D-197 entry. The 30 remaining
envelope witnesses are P3/P4/P5 + lazy-fine-structure targets as
scoped — round 2.

## Round-2 status (same sitting): P5 LANDED, P4 mostly landed

- **P5 clause-C (t15/d4 sibling revive)**: tms_parked_del's
  left-death now unparks the rule's other parked tuples and
  re-activates the live ones in reversed-chain order (gt16's re-add
  law) — LAZY plain rules only (the eager/or-twin exclusion is the
  model's t15 scope; the ungated version flipped fz_777_6816, caught
  by the regressions tier and scoped same sitting). sd_c1 EXACT
  ([RJ1,RD1,RJ3,RD3,RJ2,RD2]); **fz_42_5213 GRADUATED** (the full
  20-firing alternation, 10/10).
- **P4 clause-B**: the park's blocked-leak now finds LEAD nots (env
  lookup instead of the pos-1 arithmetic) and parks the blocked
  LEFT PREFIX; tms_parked_ins matches by starts_with (trail parks
  are full-width, so prefix == exact — certified cells unchanged).
  sd_b2 fixed ([RJ] once); **fz_123_3060 GRADUATED** (10/10). The
  eager list split into evaluate-all-then-drain passes (Drools'
  evaluateEagerList shape). BONUS: **fz_7_9550 — the L-SD × L-MB
  COMPOUND — GRADUATED** with no dedicated work.
- RESIDUE (named): sd_b4/fz_7_9375 — the OR-TWIN corner: the twin's
  blocked list is invisible to node.blocked_of (the D-158 PnShadow
  structure is the suspect); the drain-site sibling-eval + group
  park + queue prune are in place but the leak finds nothing on the
  sibling's not. Needs a PnShadow read or a dump — next round.
  sd_b3 stays fenced (the lazy or-twin is a Drools runaway,
  Family II).
- Seven Family-II runaway witnesses moved engine-side (fire-limit
  oracles, fenced-by-nature — no semantic gate) — rebanked; drift
  bank 71. fz_7_9864 moved toward (17→18 vs 19).

## Round-3 status (D-199): P4 CLOSED (the or-twin), the t15 lanes completed

- **The or-twin corner CRACKED — it was NODE SHARING, not PnShadow**:
  the leak's env-lookup missed a shared not node (env carries the
  FIRST owner); the depth-match (`env.1 == pos`) fixes it. sd_b4
  exact; **fz_7_9375 + fz_123_9175 GRADUATED**; 19 xfail movers ALL
  toward/onto the oracle (multiset); 15 Family-II rebanked (drift 69).
  ⇒ **P4 is CLOSED** (all its cells/witnesses exact or graduated).
- **The round-2 census regressions CURED** (the 65 broken slots →
  order-only residue): per-case flip attribution vs 99b363d showed
  all 65 were D-198 machinery; three model-translated lanes fixed
  them — the ⚖ land_eager lead-k1 unpark (self-killed premises only
  — the ungated cut SPUN the engine on the d3/d5 no-amut runaway,
  caught by the census net; census loops now `timeout 900`/seed),
  the ⚖ revive ACTOR EXCLUSION (self-inflicted left-death never
  revives the actor), and the lead park-RECORD + ⚖ foreign-death
  SWEEP (t15's WM-level trigger; stale-value alpha admit = the
  starvation law; lead revives INSERTION-order, trail keeps sd_c1's
  reversed chain).
- CENSUS: 483 → 373 (depth-match alone) → 250 (+unpark/actor) →
  see D-199 for the final table. The floor: 64 of the survivors are
  the d3/d5 no-amut eager-lead family — oracle RUNAWAY vs engine
  terminates, permanently open by the terminates-invariant.
- Classification of the residue: ORDER-ONLY ~63 (45× k0 + set_break
  corners — P6), SET remainder = P3's equal-salience window
  (sdp7002x3: the decl-preceding same-salience observer's glimpse)
  + deleter/justifier pick-order physics (P6) + the x73 class
  (lazy-lead-del foreign observer under-fire, undiagnosed).

## Round-3 part 2 (D-200): P3 LANDED — round 3 COMPLETE, P1-P5 all landed

- **P3 = the pop-precedence drain split** at post-fire-continue:
  the halt keeps certified strictly-higher (D-091/pick untouched);
  the DRAIN defers under equal-salience decl-preceding preemption,
  **LANE-SCOPED to bit1 (NOT-side self-defeat) entries** — ⚠ the
  whole-drain first cut broke 14 certified t20-lane cells (bit0
  justifiers, no not: their continue-drain is pr_tms_t20*-
  certified); the D-197 cause flags made the scoping expressible.
- **TEN GRADUATES in one change** — the ENTIRE P3 witness list:
  min812 (the anchor), fz_123_{2135,3370,4318,7637,9133},
  fz_777_9637, fz_7_812, fz_7_9864, and **fz_7_1353 the FINALS-DIFF
  compound** (4→12 — the 8 lost firings returned as predicted).
  Corpus 11/1124/370, drift 59.
- CENSUS 242→197 (cumulative **599→197, −67.1%**). Composition: 64
  RUNAWAY-MISMATCH (d3/d5 no-amut — permanent floor), 63 ORDER-ONLY
  (P6), 70 SET (pick-order physics, the x73 class, tails).

## Port order (updated post-D-200)

~~P1 → P2~~ (D-197) → ~~P4/P5~~ (D-198/D-199) → ~~P3~~ (D-200) →
**P6 NEXT** — the order-layer sweep (the model's order layer is the
spec; the x29/x52/x114 pick-order class, the x11/x25 consume-order
class, the k0 mass) + the x73 class (lazy-lead-del foreign observer
under-fire, undiagnosed). Invariants each step: corpus
byte-identical, xfail drift tier, agenda_open ×19 receipts, full
battery, populations vs model_sd (12×150 0-div).

## P6 part 1 (D-201): the k0 fold/churn law + three composite lanes

- **⚖ the k0 FOLD/CHURN law** (tms_churn_del_group, all four drain
  sites): del-group rules force-evaluate before the drain's retract
  — the cross-batch staging annihilation had blinded their nots to
  the break; the un-break re-adds REVERSED (blocked-list prepend).
  Lazy justifier: all del-group rules; eager: sink order (rj < l).
  **The arc's single biggest mover: 197→101 alone** — the k0 ORDER
  family and the lazy pick-order/set_break SET classes (x29/x52/
  x114) were ONE mechanism.
- The composite last-key RIDE (bit16 via survivors-at-push; flush +
  post-fire exclude, drain[pop] only) — the x51 class.
- The del-lane race widening (dead premise at insertLogical ⇒ bit2)
  — the x98/x79 zombie windows.
- The trail sweep (annihilation starves trail parked-del too;
  re-add notpos-split: lead=insertion, trail=reversed) — x121.
- CENSUS **197 → 84 (cumulative 599→84, −86.0%)**. At 84: 64
  PERMANENT (d3/d5 no-amut runaways) + **20 fixable tails** (12 ORD
  + 8 SET, no cluster > 4): 4× eager-ortwin-k0 twin match order
  (sdp7001x97), 3× eager-trail-set_break-mf (sdp7002x119), 3×
  lazy-trail-None SET (sdp6003x77), 2× eager-k0 ORD, 2×
  lazy-trail-del SET (the x33 two-deleter pick corner), 6 singles.

## Port order (post-D-201)

~~P1→P2~~ (D-197) → ~~P4/P5~~ (D-198/D-199) → ~~P3~~ (D-200) →
~~P6 part 1~~ (D-201) → the 20 tails or SLAB-COMPLETE (Bryan's
call) → I-RD (Bryan's order).
