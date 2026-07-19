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

# ═══════════════════════════════════════════════════════════════
# THE FINE-STRUCTURE GRID (D-319; Bryan: "begin phase 1")
# Predictions registered 2026-07-18 BEFORE any cell ran.
# ═══════════════════════════════════════════════════════════════

Base shape = s10 (rule L sal -5 on S: setFocus("g") + insert X; rule
H sal 5 on X; GD agenda-group "g": X + `not S(k == 1)` BLOCKED).
Oracle baseline at 3 seeds: L,H,L,L,H,H (ONE flush, on the first L
firing). Engine everywhere: all-L-then-all-H. Every DIVERGENT
prediction below carries engine = no-interleave; the content of the
prediction is the ORACLE's pattern. All cells ordinary DRL, 3×
oracle stability required.

## g1 — scale (s10 shape, n = 2/4/5 seeds)

Competing fine-structure hypotheses for the s10 partial: (A)
FIRST-EVALUATION-ONLY flush (the group's not-segment links on the
first focused evaluation; later pushes find it linked → no flush);
(B) periodic/alternating; (C) flush-per-push with a one-firing lag.

- **g1_n2**: PREDICT oracle L,H,L,H (consistent with A but
  n=2 cannot split A-vs-full — recorded for the scale curve).
- **g1_n4**: PREDICT oracle L,H,L,L,L,H,H,H (hypothesis A: one
  flush then none — med-high; B/C would show extra H's mid-run).
- **g1_n5**: PREDICT oracle L,H,L,L,L,L,H,H,H,H (same).

## g2 — pre-push (repeat-push / D-106 relocate axis)

Starter P sal 100 on S(k==1), RHS = setFocus("g") ONLY; then the s10
shape. The pre-push should be POPPED-EMPTY before any L fires
(GD has no X yet), so L's later setFocus is a fresh push again.
PREDICT (a) P,L,H,L,L,H,H — the baseline signature survives; the
popped pre-push consumes nothing because NO staged inserts existed
at its evaluation (low-med). Alternative (b) P,L,L,L,H,H,H — the
pre-push consumed the first-link (GD's not-node links on the RIGHT
side data S(k==1) alone) → no flush ever → MATCH engine. A (b)
result names the flush as LINK-TIME, not staged-data-time.

## g3 — unblocked not (the group FIRES)

s10 with GD `not S(k == 99)` (no such S → GD matches each X).
PREDICT MATCH (med-high): both sides L,GD,H per seed —
L,GD,H,L,GD,H,L,GD,H (the s4-certified group-hit surface extends to
a matching not-CE; the pushed group fires immediately, then pop,
then fresh H beats remaining L on both sides). A divergence here
would mean the engine's group-hit yield is alpha-only.

## g4 — two fresh highs (drain depth)

s10 + H2 sal 8 on X. PREDICT (high on the qualitative): the flush
makes ALL fresh MAIN activations visible and the pick drains by
salience — oracle L,H2,H,L,L,H2,H2,H,H (drain BOTH highs on X1
before returning to L; tail = remaining by salience). "One
activation per flush" would be an agenda anomaly — if seen, the law
is not a flush-then-normal-pick.

## g5 — exists dual

s10 with GD `exists S(k == 99)` (no such S → GD blocked, exists
form). PREDICT DIVERGENCE with the s10 signature L,H,L,L,H,H (med):
phreak's ExistsNode is NotNode's twin (same left-link machinery) —
if exists does NOT flush, the law is not-SPECIFIC (control-tuple
speciality) and the port condition narrows. NOTE: either way, the
D-317 fuzz_cep exists-only fence is a DIFFERENT lane (expiration
ND/NE) — not touched this slab.

## g6 — which network shapes flush

- **g6_beta**: GD `not S(k == $v)` (beta-correlated simple not;
  every X(k) blocked by its S(k)). PREDICT DIVERGENCE (med-high),
  pattern = s10 partial L,H,L,L,H,H (low-med on the pattern —
  correlation shouldn't change left-linking).
- **g6_grpjoin**: GD `not(S(k == 1) and S(k == 2))` (group-form not
  over a join; conjunction non-empty → blocked). PREDICT DIVERGENCE
  with the s9 FULL interleave L,H,L,H,L,H (med): group-form not =
  subnetwork; s9 (nested group-not) interleaved fully — if the
  simple/group split is really subnetwork-vs-simple, this lands
  full.

## g7 — salience ties (the pick law at equality)

H at salience -5 == L, s10 GD. Two cells:
- **g7_tie_hfirst**: H declared BEFORE L. PREDICT oracle
  L,H,L,L,H,H (med): after the flush the -5 tie breaks by decl
  order (the D-294 quiescence observation) → H preempts. Engine
  L,L,L,H,H,H → DIVERGE.
- **g7_tie_hlast**: H declared AFTER L. PREDICT MATCH
  L,L,L,H,H,H (med): decl-order tie-break keeps L first → the
  flush is invisible. If BOTH diverge, the tie-break is recency
  (fresh-beats-old), not decl order — also a clean law, port
  condition unchanged (the pick itself is already certified
  surface; only visibility timing is at stake).

## g8 — mixed group (alpha rule alongside the not-CE)

s10 + GA agenda-group "g" on X(v == 99) (alpha, never matches).
PREDICT DIVERGENCE with s10 baseline signature L,H,L,L,H,H
(med-high): the dead alpha rule neither adds nor removes the
GD-segment link; s7 already showed alpha-ONLY groups don't flush —
this shows alpha rules don't SHIELD a not-CE from flushing.

## g9 — two groups, stack evolution

L1 sal -5 on S (setFocus "g", insert X), L2 sal -6 on T (setFocus
"g2", insert X), H sal 5 on X, GD in "g" + GD2 in "g2" both
`not S(k == 1)` blocked. Facts S(1),S(2),T(11),T(12). PREDICT
(qualitative, med): the flush state is PER-GROUP — g2's first push
flushes AGAIN even after g's flush is spent. Oracle
L1,H,L1,L2,H,H,L2,H (L1(S1) flush → H(X1); L1(S2) spent; L2(T11)
pushes g2 → fresh flush → drains BOTH pending H's; L2(T12) spent;
tail H(X12)). A global-once result (L1,H,L1,L2,L2,H,H,H) would
make the port condition a session-level latch instead.

## DECISION TABLE (port condition by outcome)

Coherent A-pattern (g1 one-flush + g2(a) + g9 per-group) → port
condition: "on setFocus push of a group holding not/exists-CE
networks over staged types, flush + full re-pick ONCE per group
first-evaluation" — with g5/g6 naming the CE set and s9/g6_grpjoin
possibly upgrading simple-vs-group form to per-evaluation flush.
Incoherent (patterns nondeterministic across the 3× runs, or
mutually contradictory cells) → STOP AND REPORT, no port.

## ROUND 1 MEASUREMENTS (2026-07-18, all 3× oracle-stable)

REFERENCE FRAME (from the graduated boundary, re-run): s1_ctl
(NO push) = FULL interleave L,H,L,H,L,H on BOTH sides — mid-run
preemption by fresh higher salience is the certified NORMAL law;
s2 (dead-group push) = NO interleave both sides. So the push
SUPPRESSES the normal preemption, and the lane's divergence is
where the ORACLE's suppression breaks while the engine's never
does. Results:

- g1_n2/n4/n5: PREDICTED PATTERNS EXACT (one flush, first L push
  only: n=5 → L,H,L,L,L,L,H,H,H,H). Hypothesis A holds to n=5.
- g2_prepush: outcome (a) EXACT — P,L,H,L,L,H,H. The popped-empty
  pre-push (no staged data at its evaluation) consumes nothing.
- g3_unblocked: MATCH as predicted — L,GD,H ×3 both sides. The
  engine's group-hit yield covers matching not-CE.
- g4_twohighs: PREDICTED EXACT — L,H2,H,L,L,H2,H2,H,H. The flush
  exposes ALL fresh MAIN activations; drain is pure salience.
- g5_exists: MATCH (prediction MISSED, narrowing direction):
  exists does NOT flush. The law is NOT-specific.
- g6_beta (not S(k == $v)): DIVERGE, pattern FULL L,H,L,H,L,H —
  divergence predicted (hit), s10-partial pattern MISSED.
  Correlation upgrades partial→full.
- g6_grpjoin (not(S(k==1) and S(k==2))): DIVERGE FULL as
  predicted — group-form not = full, s9 class confirmed.
- g7_tie_hfirst: DIVERGE L,H,L,L,H,H; g7_tie_hlast: MATCH — the
  tie-break at equal salience is DECL ORDER, both hits. The flush
  only reorders when the fresh rule wins the ordinary pick.
- g8_mixed: DIVERGE L,H,L,L,H,H — dead alpha rule in the group
  neither shields nor upgrades (predicted).
- g9_twogroups: DIVERGE but FULL — L1,H,L1,H,L2,H,L2,H; the
  per-group one-flush prediction MISSED. CONFOUND NOTED: GD2 has
  an IDENTICAL LHS to GD → phreak node sharing (the twin segment
  split may be the upgrade, not the second group) → round 2.

Standing after round 1 — PARTIAL class: sole uncorrelated
alpha-not (s10, g1, g2, g7, g8). FULL class: correlated not
(g6_beta), group-form not (s9, g6_grpjoin), twin-shared not (g9).
NONE class: exists (g5), alpha-only (s7), plain join (s11), dead
(s2). The partial/full boundary is segmentation fine structure —
round 2 isolates the variables.

# ═══════════════════════════════════════════════════════════════
# ROUND 2 — sharing/correlation/latch-scope splitters
# Predictions registered 2026-07-18 BEFORE any round-2 cell ran.
# ═══════════════════════════════════════════════════════════════

- **g10_twinsame**: GD + GD2 identical LHS (X + not S(k==1)),
  BOTH in group "g". PREDICT FULL (med) — g9's upgrade is the
  twin node-sharing (segment split at the shared not node), not
  the second group. PARTIAL here → the second GROUP is the
  variable and g9 needs a different read.
- **g11_twinmain**: s10 + R2 with identical LHS in MAIN (sal -20,
  blocked, never fires). PREDICT FULL (med-low) — pure network
  twinning upgrades even with no second group. PARTIAL → the
  twin must be group-resident to matter.
- **g12_barenot**: GD `not B()` (bare uncorrelated, no right
  alpha filter; B(9) seeded). PREDICT PARTIAL (med-low) — the
  right-side alpha filter is not the variable. FULL → s10's
  partial requires the filtered right input.
- **g13_alphaH**: s10 with H = `X(v != 99)` (H gets its own alpha
  node — breaks any H/GD lia sharing). PREDICT PARTIAL (med) —
  the observation channel's sharing is not the mechanism.
- **g6_betaneq**: GD `not S(k != $v)` (correlated but
  non-hash-indexable; all X blocked at n=3). PREDICT FULL
  (med-low) — correlation itself upgrades, not the index type.
  PARTIAL → the upgrade is INDEXED correlation only.
- **g15_twonots**: GD `not S(k==1)` + GE `not S(k==2)` both in
  "g" (different not nodes, shared lia, both blocked). PREDICT
  FULL (low-med) — any segment split on the shared left path
  upgrades. PARTIAL → only twin/self sharing matters.
- **g14_base**: s10-shape with insertLogical + blocker moved to
  `not B(b == 1)` (B(1) seeded), n=3, single epoch. PREDICT
  PARTIAL L,H,L,L,H,H (med-high — s3/min1681 logical parity;
  this is the control for the epoch pair).
- **g14_ctl**: g14_base + epoch 2 inserting S(4),S(5),S(6) (no
  deletes — GD's left segment stays linked across the batch
  boundary). Epoch-2 pattern PREDICT L,L,L,H,H,H (low-med) —
  the latch is NOT reset by a new fireAllRules batch (it is
  link-scoped, not batch-scoped). L,H,L,L,H,H → batch-scoped.
- **g14_relink**: g14_base + epoch 2 deletes S(1),S(2),S(3)
  (targets 0,1,2 — the logical X's tear down, GD's left UNLINKS)
  then inserts S(4),S(5),S(6) (relink at the first epoch-2 L).
  Epoch-2 pattern PREDICT L,H,L,L,H,H (med-low) — the latch
  resets on link transition. Combined readout with g14_ctl:
  ctl-noflush + relink-flush = LINK-SCOPED (port needs link
  tracking); both-flush = batch-scoped (port latch per
  fire_all); both-noflush = session-permanent latch;
  ctl-flush + relink-noflush = INCOHERENT → stop.

## ROUND 2 MEASUREMENTS (2026-07-18, all 3× oracle-stable)

- g10_twinsame: PARTIAL (prediction FULL **missed**) — twin
  not-CE in the same group does not upgrade.
- g11_twinmain: PARTIAL (**missed**) — twin in MAIN doesn't
  either. Node sharing is NOT the upgrade variable.
- g12_barenot: PARTIAL (hit) — right alpha filter irrelevant.
- g13_alphaH: PARTIAL (hit) — H's own alpha node irrelevant.
- g6_betaneq: FULL-CLASS flush at pushes 1 and 2 (hit) —
  correlation upgrades even non-hash-indexable.
- g15_twonots: PARTIAL (**missed**) — two DIFFERENT alpha-nots
  in the group still one flush (both latch at push 1).
- g14_base: PARTIAL under insertLogical + B-blocker (hit).
- g14_ctl epoch 2: NO flush → the latch SURVIVES the
  fireAllRules batch boundary (hit).
- g14_relink epoch 2: NO flush → the latch even survives full
  teardown (all X retracted) + relink (**missed**, decision
  table row: both-noflush = the latch is ONCE-EVER, permanent
  for the session; consistent with phreak not re-queuing a
  once-evaluated rule on staged additions, and no eager unlink
  on empty).

**ROUND-1 REVISION (g9)**: the "FULL" read of g9 was WRONG — L1
and L2 each have only TWO activations, so H(2)/H(12) after the
second L firings are RUN-EXHAUSTION picks (certified normal
surface), not flushes. g9 is per-group-ONCE: g's flush at
L1(1), g2's at L2(11). No contradiction with g10/g11. Likewise
every "FULL" n=3 cell shows flushes only at pushes 1,2 (push
3's H is run-end) — per-push at n=3 needs the n=5 scale check.

# ═══════════════════════════════════════════════════════════════
# ROUND 3 — latch identity, per-push scale, evaluation-keying
# Predictions registered 2026-07-18 BEFORE any round-3 cell ran.
# ═══════════════════════════════════════════════════════════════

- **g16_latejoin**: GD (X + not S(k==1)) and GE (Y + not
  S(k==1)) both in "g"; L1 sal -5 on S inserts X ×3; L2 sal -6
  on T inserts Y ×2 (GE's segment first LINKS during L2's run,
  long after g's first flush). PREDICT (med): the latch is
  PER-RULE (first evaluation of each rule), so L2(11) flushes:
  L1(1),H(1),L1(2),L1(3),H(2),H(3),L2(11),H2(11),L2(12),H2(12).
  Per-GROUP latch → no L2 flush:
  ...,L2(11),L2(12),H2(11),H2(12).
- **g17_prelink**: exact s10 DRL; facts S(1..3) + X(0) (GD's
  segment links at initial insert, BEFORE any push; GD stays
  never-evaluated because "g" is never focused until L(1)).
  PREDICT (med): flush at L(1)'s push anyway —
  H(0),L(1),H(1),L(2),L(3),H(2),H(3) — the latch keys on FIRST
  EVALUATION, not on this-RHS-caused-the-link. A no-flush
  result (H(0),L,L,L,H,H,H = MATCH engine) would tie the flush
  to RHS-time linking.
- **g6_beta_n5** / **g6_grpjoin_n5**: the correlated and
  group-form cells at n=5. PREDICT (med): truly per-push —
  L,H,L,H,L,H,L,H,L,H both cells (flush at every push, no
  latch). A partial pattern (e.g. flushes at pushes 1-2 only)
  would mean these classes latch at a different count and the
  law needs a counter, not a boolean.

## ROUND 3 MEASUREMENTS (2026-07-18, all 3× oracle-stable)

ALL FOUR PREDICTIONS HIT EXACTLY:
- g16_latejoin: per-RULE latch — GE (linking during L2's run)
  flushes at L2(11) long after GD's latch spent.
- g17_prelink: flush at first PUSH-evaluation even though the
  segment linked at initial insert — the latch keys on first
  evaluation, not RHS-time linking.
- g6_beta_n5 / g6_grpjoin_n5: L,H ×5 — correlated and
  group-form are per-push, no latch.

THE LINKING RE-READ: every no-flush shape has a rule that can
never LINK — s11's join right is alpha-filtered to k==99
(EMPTY memory; joins link on both sides), g5's exists right
likewise empty (exists links like join), s7's alpha filter
drops every X before the lia (empty terminal segment), s2's
group rule is type-dead. The not-CE links on LEFT data alone —
that alone may be why "not" appeared special. SHARP TEST: a
populated-but-unmatched exists or join (linked, zero matches)
should FLUSH if the linking read is right, refuting the
"not-specific" round-1 conclusion.

# ═══════════════════════════════════════════════════════════════
# ROUND 4 (final) — linked-exists, linked-join, multi-push
# Predictions registered 2026-07-18 BEFORE any round-4 cell ran.
# ═══════════════════════════════════════════════════════════════

- **g18_existsbeta**: L inserts X($k+10); GD in "g" =
  `X($v : v) exists S(k == $v)` — right memory = ALL S
  (populated, LINKED), no S(11..14) so GD stays blocked; n=4.
  PREDICT FLUSH per-push (low-med, the linking read):
  L,H,L,H,L,H,L,H. NO flush (engine-match) → exists genuinely
  never flushes and the law stays not-CE-specific.
- **g19_joinlinked**: GD = `X($v : v) S(k == $v + 100)` —
  right memory = ALL S (populated, linked), keys never match;
  n=4. PREDICT FLUSH per-push (low-med): L,H,L,H,L,H,L,H.
  NO flush → joins never flush even linked; law stays not-CE.
  (A populated UNcorrelated unmatched join is structurally
  impossible — no beta constraint means cross-product matches,
  which is the certified group-hit lane.)
- **g20_twopush**: L's RHS pushes BOTH "g" (GD not S(k==1))
  and "g2" (GD2 not S(k==2)), then inserts X; n=3. PREDICT
  (med-high): both alpha-not rules first-evaluate and latch at
  push 1 → ONE visible flush → L,H,L,L,H,H. Extra H's mid-run
  would mean per-push-per-group compounding.

RELOCATE NOTE (why no relocate × flush cell): a blocked group
pops at its first pick and cannot stay buried on the stack —
setFocus makes the pushed group top instantly, so a
blocked-not group is never relocated from a buried position;
the D-106 relocate lane only involves non-empty groups, which
are the certified group-hit surface.

## ROUND 4 MEASUREMENTS (2026-07-18, all 3× oracle-stable)

ALL THREE PREDICTIONS HIT:
- g18_existsbeta: FLUSH per-push (L,H ×4) — a LINKED exists
  flushes. The round-1 "law is not-specific" conclusion is
  REFUTED; g5's silence was the empty right memory (never
  linked), not the exists form.
- g19_joinlinked: FLUSH per-push (L,H ×4) — a linked,
  zero-match join flushes too. s11's silence was its empty
  alpha-filtered right memory.
- g20_twopush: ONE visible flush, both groups' alpha-not
  latches spend at push 1 — L,H,L,L,H,H.

# ═══════════════════════════════════════════════════════════════
# THE LAW — FINAL (D-319, 29 cells, 4 rounds, every cell 3×
# oracle-stable, zero incoherence; the port condition)
# ═══════════════════════════════════════════════════════════════

**When a firing's RHS pushes focus to a group G, the oracle
evaluates G's QUEUED rule networks before continuing the current
rule's activation run. That evaluation FLUSHES staged
propagation: every fresh MAIN activation becomes visible, and
the next pick is ordinary — salience first (g4: ALL fresh highs
drain before returning), declaration order at ties (g7 pair) —
so it preempts the current run whenever something fresh beats
it. If G has nothing queued, both engines continue the run
(the entire pr_af_* agree-boundary).**

A rule in G is QUEUED at push time iff its network is LINKED
and has unevaluated (staged) input:

1. **LINK rules** (which no-flush boundary cells confirm):
   alpha-terminal needs alpha-PASSING data (s7); join and
   exists need BOTH side memories populated post-alpha-filter
   (s11, g5 — empty right = never linked = never flushes);
   NOT links on LEFT data alone (why not-CE looked special).
2. **Re-queue rules** (the partial/full fine structure):
   - simple alpha-not (uncorrelated; right constraint
     alpha-only or bare): queued ONCE EVER — first evaluation
     latches permanently (g1 scale, g14_ctl across
     fireAllRules batches, g14_relink even across full
     teardown+relink). The latch is PER RULE (g16: a
     late-linking second rule flushes again) and keys on first
     EVALUATION, not on who caused the link (g17: pre-linked
     segment still flushes at first push).
   - correlated not (beta constraint, indexed or not — g6_beta
     n5, g6_betaneq), group-form not (subnetwork — s9,
     g6_grpjoin n5), linked exists (g18), linked unmatched
     join (g19): queued on EVERY staged addition → flush at
     EVERY push.
3. Matching group rules fire instead — the certified group-hit
   surface, including matching not-CE (g3), both engines agree.
4. Multiple pushes in one RHS evaluate each pushed group; the
   latches spend together, one visible flush (g20).
5. Sharing/placement are NOT variables: twins in same group
   (g10), twins in MAIN (g11), extra dead alpha in group (g8),
   H's own alpha node (g13), a second different alpha-not
   (g15) — all preserve the class.

Divergence inventory: 26 of 29 grid cells DIVERGE (engine
always continues), 3 MATCH (g3_unblocked, g5_exists,
g7_tie_hlast). All 29 stay pending here until the port; the
divergent ones graduate with it (byte-gate expected-divergence
at port time = these 26 + the 7 xfailed witnesses).

VERDICT: **COHERENT** — the port contingency is satisfied.
Engine-side condition sketch (phase 2): at CompiledAction::
SetFocus, check the pushed group's rules for linked-dirty
non-matching CE networks per rules 1-2 (a per-rule once-ever
latch for the simple-alpha-not class; per-push for
correlated/subnetwork/exists/join); if any queued → force
staged flush + full agenda re-pick (the late-continue yields)
instead of continuing the current rule's run.

# ═══════════════════════════════════════════════════════════════
# PHASE 2 (D-320, Bryan: "Phase 2") — THE PORT
# ═══════════════════════════════════════════════════════════════

DISCOVERY AT THE ENGINE MAP: the halt-check peek (engine.rs
~8432, the D-262 lane) ALREADY walks the pushed group's members
in pick order and evaluates queued-dirty ones — and the engine's
D-031/D-091 agenda-item model (rule_linked / refresh_linked =
notifyRuleLinkSegment→queueRuleAgendaItem, note_link_effects =
doUnlinkRule) ALREADY reproduces the entire measured fine
structure, verified empirically via SEINE_AG_DEBUG peek states:
s10's GD is q=true,d=true at push 1 and q=false at push 2+ (the
once-ever latch, via the alpha-not right-data link state);
g6_beta's GD is q=true,d=true at EVERY push (per-push class).
THE PORT IS ONE BOOLEAN: if the peek evaluated any queued-dirty
member, that evaluation is the oracle's staged-propagation flush
→ suppress the late-continue (fall through to the ordinary
agenda pop). No class analysis, no latch bookkeeping, no linked
scan — the certified linking model IS the law's substrate.

## The two ungridded corners — predictions FROM the engine link
## model (registered 2026-07-18 BEFORE the oracle cells ran)

- **g21_staledirt**: GD (X + not S(k==1)) dirty from an INITIAL
  X(0); L's RHS inserts only Y (observer H2 sal 5 on Y) +
  pushes "g" — the push carries NO dirt of its own toward GD.
  Engine peek: q=true,d=true at push 1 (stale dirt), q=false
  after. PREDICT (med): flush at push 1 from STALE dirt —
  L,H2,L,L,H2,H2. A no-flush oracle (L,L,L,H2,H2,H2) would mean
  the oracle's flush needs THIS-RHS staged input and the
  one-boolean port over-flushes here.
- **g22_joinnot**: GD = X + Y2 (join, Y2(7) seeded) + not
  S(k==1), n=5. Engine peek: q=true,d=true at push 1 ONLY.
  PREDICT (med): ONCE-EVER — L,H,L,L,L,L,H,H,H,H. Per-push
  (L,H ×5) would mean the join upgrades re-queueing in the
  oracle and the engine's link model diverges from Drools here.

## CORNER MEASUREMENTS (2026-07-18, 3× oracle-stable)

BOTH PREDICTIONS HIT EXACTLY: g21 = L,H2,L,L,H2,H2 (the flush
fires from STALE dirt — the push needs no dirt of its own);
g22 = L,H,L,L,L,L,H,H,H,H (join + alpha-not is once-ever; the
alpha-not link state dominates). The engine's certified link
model predicted the oracle on both unmeasured corners — the
one-boolean port inherits the fine structure from machinery
that is already differential-certified. 31 grid cells total.

## PORT ROUND 1 (the one-boolean yield): 31/31 grid cells PASS,
## 8/8 pr_af boundary PASS, 5/7 witnesses PASS. The two
## surviving fuzz blobs name TWO UNGRIDDED INGREDIENTS:

- fz_315901_311: the pushed group's rule R4 is NO-LOOP (eager
  list) + accumulate + bare not — the engine's eager_flush
  consumes its dirt BEFORE the halt-check peek (AG_DEBUG:
  q=false at the push), so the one-boolean sees nothing; the
  oracle still preempts after the first push firing.
- fz_316002_1902: fork INSIDE the already-focused group — the
  deleter R3's staged delete re-fires the decl-earlier
  same-salience accumulate rule R2 mid-run; the oracle lets R2
  preempt R3's remaining run; the engine's same-group continue
  halts only on STRICTLY-higher (`higher`), and the D-199
  eq_decl_preempt gate is TMS-landing-only (min608: wholesale
  equal-salience halting was measured WRONG — so the 1902 law
  must be finer, plausibly fresh-mid-run-requeue vs
  already-queued).

## ROUND 5 — the eager and in-group corners
## Predictions registered 2026-07-18 BEFORE any cell ran.

- **g23_noloop**: s10 with GD no-loop. PREDICT (med, from the
  315901 witness): the eager status does not rob the push
  flush — oracle L,H,L,L,H,H. Engine currently: eager_flush
  consumed dirt → peek clean → continue → DIVERGE expected
  pre-fix.
- **g25_accnot**: GD = no-loop + accumulate(count over X) +
  `not S(k == 1)` (blocked); L inserts X per firing (the acc
  re-dirties on EVERY push). PREDICT once-ever L,H,L,L,H,H
  (low-med; the g22 alpha-not-dominance analogy). Per-push
  L,H,L,H,L,H would mean acc upgrades re-queueing.
- **g26_grp_accrefire** (the 1902 distillate): P pushes "g";
  GA in g = collectList over T + join E(e==1), decl-FIRST;
  GB in g = deleter of T, decl-second, same salience. PREDICT
  (med, from 1902): oracle interleaves — P, GA[1,2,3], GB(1),
  GA[2,3], GB(2), GA[3], GB(3), GA[] — each staged delete
  re-fires GA which preempts GB's run at the decl-order tie.
- **g27_main_accrefire**: the SAME two rules in MAIN, no
  groups, no push. PREDICT MATCH = continue (med-low): GA
  fires, GB's full run, GA re-fires once with the final []
  — the certified MAIN late-continue holds at ties even for
  acc-refires. A DIVERGE here = a GENERAL halt-law gap (not
  focus-scoped) and the fix moves out of the focus lane.

## ROUND 5 MEASUREMENTS + THE FULL PORT (2026-07-18)

- g23_noloop: MATCH already (no-loop alone doesn't rob the
  peek — its X-lia is empty at init so the initial eager eval
  early-exits unlinked, leaving the pulse for the push).
- g25_accnot: DIVERGE — oracle L,H,L,L,H,H (once-ever flush),
  engine no flush: the acc's InitialFact lia makes the rule
  initially evaluable, so the no-loop EAGER evaluation
  consumes the not-pulse + initial item BEFORE any push.
- g26_grp_accrefire: DIVERGE exactly as predicted — the
  in-group interleave (P, GA[3,2,1], GB, GA[3,2], GB, GA[3],
  GB, GA[]).
- g27_main_accrefire: MATCH (the engine's MAIN path does a
  FULL RE-PICK每 firing — the keep-control branches require a
  non-empty focus stack, so the tie preemption at MAIN was
  already right; prediction hit).

THE PORT (three edits attempted, two land):
1. **af_flush one-boolean** (D-258 top≠l_grp branch): a
   queued-dirty member evaluated by the halt-check peek =
   the oracle's staged flush → the late-continue yields.
2. **tie_preempt** (D-261 same-group branch): the
   between-firings halt is the QUEUE COMPARATOR — a queued
   same-group member at EQUAL salience with EARLIER decl_pos
   yields (any such item mid-run is necessarily fresh; the
   pop evaluates it lazily, D-262 untouched). Fixes
   g26/fz_316002_1902's agenda component.
3. **eager inactive-group gate: REVERTED** — it fixed
   g25/fz_315901_311 but broke fz_9005_450 (a certified
   halt-matrix cell whose or-form no-loop group rule NEEDS
   the eager queue-construction timing; its fork was an
   activation CHOICE inside the or-queue, not a flush).
   The two constraints conflict at exactly one shape:
   **no-loop + accumulate + not in a pushed group** (the
   acc's InitialFact lia + no-loop puts the rule on the
   initial eager list, which consumes the pulse the push
   flush needs). NAMED OPEN CORNER, quarantined:
   xf_af_g25_accnot (minimal) + xf_af_fz_315901_311 (blob).
   g25b (the SAME shape minus no-loop) PASSES and graduates
   as the boundary pin — the corner is no-loop-scoped.

fz_316002_1902 DISPOSITION: the agenda component is FIXED
(firing[7] = R2 both sides post-port); the residual is a
COLLECT-ORDER divergence (collectList element order on
delete-refire: engine [0,1,3] vs oracle [1,3,0]) that
reproduces in MAIN with no focus machinery (g28) and is
byte-identical pre/post the port = PRE-EXISTING, a
collect-order family member. g28 → xf_co_refire_1902 (the
family's minimal witness); the 1902 blob stays banked under
the corrected read. The D-318 'collect-order adjacent' first
read was half-right after all.

FINAL LANE DISPOSITION: 40 cells graduate (35 grid + g25b +
... see commit), 6 witnesses un-xfail, bank 63→60
(−6 witnesses, +xf_af_g25_accnot, +xf_af_fz_315901_311,
+xf_co_refire_1902).

## THE af_live REFINEMENT (the 879 ↔ g9 needle) + 11 BONUS
## RESOLUTIONS (2026-07-18, same day)

The first byte gate flushed 14 divergent cells — 12 of them
were OPEN LATENTS the port RESOLVES (5 agenda_open + 6 xfail
incl. fz_313902_761, the unexplained D-315 latent, and the
whole xf_fz_141421/31415/606060/62831 order family) and one
was fz_9003_879, a PASSING halt-matrix guard the raw
one-boolean REGRESSED. Three iterations on the yield
predicate, each measured against the full lane + agenda_open:

1. linked-gate (member must be rule_linked): fixed 879, broke
   g9 (GD2's queued entry is unlinked — the twin-shared pulse
   was spent by GD's evaluation — yet the oracle's item IS in
   g2's queue and flushes at L2's push).
2. attempt-zombie (reached-while-unlinked = dead): fixed g9,
   broke 879 again (the critical member r4 was queued by the
   UNLINK TRANSITION during R5's deletes — never attempted).
3. **af_live (LANDED): entry ORIGIN decides.** A queued-dirty
   member is live to the flush-peek iff its entry was born at
   a staging-notify-while-LINKED (refresh_linked = Drools'
   queueRuleAgendaItem) and no evaluation attempt has reached
   the rule since (Drools consumes-and-removes there — the
   engine's unlinked staging is undrainable, so queued+dirty
   would persist as a zombie). Unlink-transition entries
   (doUnlinkRule-born) are dead to the peek — 879's oracle
   run continues past its unlink-remnant; source-verified
   against drools-core 9.44 (RuleExecutor.fire:
   flushPropagations → haltRuleFiring{evaluateEagerList;
   peekNextRule = focusStack.top().peek()} →
   evaluateNetworkIfDirty; halt = different-group ||
   conflict-order; removeRuleAgendaItemWhenEmpty at fire()
   end).

Result: ZERO unexpected fails across the 58-cell pr_af lane +
agenda_open ×10 + fz_9005_450; the 6 remaining agenda_open
latents byte-identical ×3 (debug/release/pre-edit worktree);
fz_9104_1328's brief flip resolved back to its EXACT pre-port
open state (its earlier "resolution" was the same over-breadth
that broke 879).

# ═══════════════════════════════════════════════════════════════
# THE g25 CORNER ROUND (D-325; Bryan: "g25 no-loop-acc corner")
# Predictions registered 2026-07-18 BEFORE any cell ran.
# ═══════════════════════════════════════════════════════════════

THE LINGER HYPOTHESIS (from the D-322 source reading):
evaluateEagerList → evaluateNetwork does NOT call
removeRuleAgendaItemWhenEmpty — removal happens only at fire().
So g25's GD (no-loop + acc ⇒ InitialFact lia ⇒ initially linked
via the not-pulse ⇒ queued ⇒ eager-listed) is evaluated at the
initial eager pass and left LINGERING clean-empty in the
INACTIVE group's queue (the group is never focused, so no pop
ever removes it). The first push's peekNextRule finds the item
(non-null, different group) → HALT → the pop consumes it (fire()
with empty tuples → removeWhenEmpty) → H preempts ONCE; later
pushes peek an empty group → continue. The engine's eager pass
unqueues emptied items immediately (the fz_42_8775 pin) — so its
peek never sees anything. Predictions:

- **p1_g25_rerun**: xf_af_g25_accnot + the blob re-diff (control:
  still DIVERGE pre-port).
- **p2_unblocked_init**: the g25 shape with `not S(k == 99)` (no
  such S → GD MATCHES at init on count=0) → GD FIRES at init →
  its item is consumed by fire()'s removeWhenEmpty on BOTH sides
  → no linger → pushes continue. PREDICT MATCH (med-high):
  GD-at-init then L,L,L,H,H,H both sides.
- **p3_prepush**: the g25 shape + starter P sal 100 whose RHS
  only pushes "g" BEFORE any L fires. The oracle's linger item is
  consumed by THAT push's pop cascade → L's own pushes find an
  empty group → NO flush → all-L-then-H. PREDICT DIVERGE→MATCH
  INVERSION vs g2_prepush (med): the plain-not g2 KEPT the flush
  (its queue event was the fresh X1 link, not a linger); the
  acc-linger variant LOSES it. A g25-like flush surviving the
  pre-push REFUTES the linger hypothesis.
- **p4_two_lingers**: TWO no-loop-acc-not rules in "g". PREDICT
  (med): still ONE visible flush at push 1 (both linger items
  consumed in the same pop cascade).
- **p5_dynsal**: the g25 shape with `salience ($c * 0)` (dynamic
  ⇒ eager) instead of no-loop. PREDICT (med): same linger → same
  once-ever flush → currently DIVERGES like g25.

PORT SKETCH (if the round lands): af_linger — a one-shot flag set
when the eager pass unqueues an emptied item whose agenda group
is INACTIVE (≠ MAIN, ≠ focus top); the halt-peek treats a
lingering member as the oracle's queued item (yield + consume the
flag); a pop of the group also consumes it. No queue-state
surgery — the fz_42_8775 window-claiming pin and the fz_9005_450
or-queue construction timing are untouched (the eager EVALUATION
and the unqueue both stay).

## D-325 MEASUREMENTS + THE PORT (2026-07-18)

ALL FOUR PREDICTIONS HIT — p3's INVERSION is the clincher: the
acc-linger variant LOSES the flush under a pre-push
(P,L,L,L,H,H,H) where g2's plain-not variant KEPT it — exactly
the "item consumed by the earlier push's pop" signature. p2
(unblocked-at-init → fires per push, group-hit lane, MATCH);
p4 (two lingers, ONE flush); p5 (dyn-salience lingers like
no-loop).

THE PORT — af_linger, a one-shot flag: set when the eager pass
unqueues an emptied item whose group is INACTIVE (Drools'
evaluateEagerList leaves the item queued; only fire() removes);
consumed by the halt-peek (one yield) or by the group popping
off the focus stack; superseded by any fresh queue event. The
fz_42_8775 window-claiming unqueue and the fz_9005_450 or-queue
construction timing are UNTOUCHED (evaluation and unqueue both
stay; only the flag is new).

Post-port: p2-p5 all MATCH; xf_af_g25_accnot +
xf_af_fz_315901_311 flip PASS and graduate; fz_9005_450 holds;
the full pr_af lane + agenda_open ×10 clean. Byte gate vs
e5fba33: 2400 same / 2 moved / 0 diff — the two graduated
witnesses are the only movement in the corpus universe. The
agenda_focus ledger is EMPTY.
