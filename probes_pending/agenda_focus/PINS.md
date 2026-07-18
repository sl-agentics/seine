# PINS — the setFocus × salience agenda-order family hunt (D-318;
# Bryan relaying the other instance: "three latents converging on
# agenda ordering isn't three unlucky corners, it's a smell")

Predictions registered 2026-07-18 BEFORE any cell ran.

## THE FORK SIGNATURE (from the banked witnesses)

fz_315901_311 forks at firing[4]: after R3 (sal -7, RHS =
setFocus("ga") + inserts) fires ONCE, the oracle interleaves — the
fresh higher-salience R0 activations (on the just-inserted facts)
preempt R3's REMAINING activations; the engine runs all R3s first.
fz_316001_1681: identical shape with insertLogical (R1 sal -5 +
setFocus("gb") vs R0 sal 6 on the derived facts). Both converge to
the same firing MULTISET — pure order. HYPOTHESIS: the certified
late-continue path (D-258/D-259: after firing R, continue with R's
next activation without a full re-pick) fails to YIELD when the RHS
changed the FOCUS STACK — the pick that should see the fresh
higher-salience MAIN activation instead continues the current
rule's run.

## SPLITTER CELLS (minimal, diffable)

- **s1_ctl** — NO setFocus: R_low (sal -5) ×3 seeds, RHS inserts X;
  R_high (sal 5) on X. PREDICT MATCH (interleaved
  low/high/low/high/low/high — the certified salience-preemption
  surface; if THIS diverges the hypothesis is wrong and the bug is
  broader).
- **s2_focus** — s1 + R_low's RHS ALSO does setFocus("g") (group "g"
  declared by an unmatchable rule). PREDICT DIVERGENCE with the
  witness signature: oracle interleaves, engine runs all R_lows
  first. s1-vs-s2 is the splitter: if only s2 diverges, the
  mechanism is the setFocus × late-continue interaction, isolated.
- **s3_logical** — s2 with insertLogical instead of insert.
  PREDICT: same divergence (the 1681 witness shape; TMS staging is
  not the variable).
- **s4_grouphit** — setFocus to a group WITH a matching rule.
  PREDICT: both engines fire the group rule immediately after each
  R_low (the certified D-106 surface) — MATCH; maps the adjacent
  lane so the fix (if any) does not disturb it.
- **s5_nofresh** — s2 but R_high's premise is pre-seeded (no fresh
  activation from the RHS; high fires before any low by salience).
  PREDICT MATCH — the divergence needs a FRESH arrival during the
  focus-changed continue.

## DECISION TABLE

Only s2/s3 diverge → the law: "a focus-stack change in the RHS must
force a full agenda re-pick (late-continue yields)"; the fix is the
yield condition; SD census 72 + agenda_open ×15 are the order gates
for any engine change. s1 diverges too → broader agenda bug, STOP
and report. Nothing diverges → the witnesses need a bigger minimal
shape (or-branches / no-loop / multiple groups) — iterate the
splitter before touching anything.

## MEASUREMENTS

(filled after the run)

## MEASUREMENTS (2026-07-18, same day) — THE FAMILY IS ONE MECHANISM

First splitter round: s1..s5 ALL MATCH (both engines continue the
current rule's run after a dead-group push — the s2/s3 divergence
prediction MISSED, which was the tell that the witnesses carry an
extra ingredient). Delta-minimization of fz_316001_1681 (semantic-
divergence predicate) landed a 3-rule cell; ablation grid: no-loop,
duplicate insertLogical, epoch-vs-initial, value coverage, plain-vs-
logical ALL irrelevant; **setFocus removal kills it** (m5). The s7/s8
pair then split "group watches the inserted type": necessary (s8)
but not sufficient (s7, alpha-only). The s9/s10/s11 grid landed it:

**THE LAW (oracle side): when an RHS pushes focus to a group whose
rules contain NOT-CE networks receiving the staged inserts, the
focused-group evaluation FLUSHES staged propagation — fresh
higher-salience MAIN activations become visible to the next pick and
PREEMPT the current rule's remaining activations.** No not-CE in the
group (alpha-only s7, plain join s11) → no flush → both engines
continue (today's certified behavior). Fine structure: the group-not
form interleaves FULLY (s9: L,H,L,H,L,H); the simple-not form
PARTIALLY (s10: L,H,L,L,H,H — segment-linking territory, unmapped).
The engine models none of this — it always continues the run.

**ALL FOUR banked witnesses are members** (setFocus-ablation kills
each): fz_313002_319 (the "computed-salience" read was wrong),
fz_315901_311, fz_316001_1681, fz_316002_1902 (the "collect-order
adjacent" read was wrong too). The family upgrade: from four
unexplained latents to ONE NAMED LANE — the setFocus × not-CE
staged-flush preemption.

DISPOSITION: s1/s2/s3/s4/s5/s7/s8/s11 MATCH → graduated (pr_af_*,
they pin the agree-boundary around the lane); s9/s10 + the minimized
1681 → xfail as CANONICAL witnesses (minimal, named — better than
the fuzz blobs, which stay banked too). THE PORT IS GATED: an agenda-
pick landing law on the most order-sensitive surface, with the s10
fine structure unmapped — a probe grid (flush-per-evaluation? which
segment states?) belongs before any engine change. AT BRYAN'S GATE.
