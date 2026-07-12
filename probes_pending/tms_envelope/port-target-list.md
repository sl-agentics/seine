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

## Port order (proposed)

P1 → P2 (shared drain/landing machinery; ladder-certified; zero
D-106 contact expected) → P4/P5 (RuleExecutor-local) → P3 (the
D-106-adjacent drain change, receipts-gated, after everything else
is green so its diff is minimal) → P6 order-layer sweep. Progress
metric: the census (47-56/150 → 0) + the ladder cells + the 31
witnesses; invariants each step: corpus byte-identical, xfail drift
tier (re-bank as witnesses converge — each graduation is a
xfail-rebank + D-entry), agenda_open ×19 receipts, full battery,
populations vs model_sd (which is now the executable spec at 0-div).
