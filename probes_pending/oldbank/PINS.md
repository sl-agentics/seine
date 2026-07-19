# PINS — the OLD-BANK quarantine families round (opened 2026-07-19)

Round record for the Item-7 aged-witness families (see HANDOFF.md
for the triage). Doctrine: hand-decode end-to-end FIRST, minimize,
predictions registered here BEFORE cells run, model on calibrated
surfaces, revert naive ports on gate evidence (D-331/D-345/D-333).

## Family 1 — ORACLE-NPE pair: NO ROUND NEEDED (ledger correction)

xf_fz_31337_698 + xf_fz_8087_1043. The handoff's step 1 ("build a
10.1.0 oracle, run both, gate with Bryan") was STALE: D-263
(2026-07-15) already ran EXACTLY this check on EXACTLY this pair —
both clean 3x byte-stable on a throwaway 10.1.0 oracle while the
pinned 9.44.0.Final oracle still NPEs; nothing to file; Bryan's
disposition recorded then: STAY BANKED (pinned-oracle envelope,
adjudicable on a future oracle bump). Both files' _finding carry
the full disposition text. Verified 2026-07-19; zero new work.

## Family 2 — the COUNT pair: NOT one law. fz_777_1278 decoded
## (this slab); xf_fz_296002_626 is a DIFFERENT class (no no-loop,
## no or-branches — setFocus/dyn-salience/acc shape; own decode
## after this slab).

### fz_777_1278 — HAND-DECODE (2026-07-19, complete)

Engine R0+R2x3 (4) vs oracle R0+R2x2 (3); final facts IDENTICAL.
R2 = no-loop, THREE or-branches, RHS insertLogical(new T2("a")).
Branches 1/3 satisfied at setup (T2("") passes !(contains "b"));
branch 2's `exists T2(f0 contains "a")` right memory EMPTY at
setup → path unlinked. SEINE_TRACE shows the fork: branch 1's
firing stages the logical T2("a") right ins WITH origin Some(2),
the same eval relinks branch 2 and stages the LIA left fill
([F0], None); phase order runs rightIns (lefts empty, no emit)
then leftIns — the blocking arm emits the exists child with the
LEFT's origin None (phreak.rs:3071-3074) → consume_term_ins's
parent-scoped no-loop check (engine.rs:11185) sees None → the
extra activation.

DROOLS SOURCE (9.44.0.Final, verbatim):
- PhreakExistsNode.doLeftInserts:128 — the child takes
  leftTuple.getBlocker().getPropagationContext() (the BLOCKER's
  pctx; upd-path re-blocks stamp it too, lines 276/399/493).
- Tuple.findMostRecentPropagationContext — max propagation
  NUMBER over the tuple's own pctx and all parents'.
- PhreakRuleTerminalNode.doLeftTupleInsert:102-106 — no-loop:
  suppress iff sameRules(rtn, mostRecent.getTerminalNodeOrigin())
  where sameRules = rule NAME + package + consequence name — TRUE
  across or-subrules (shared name). The engine's parent-scoped
  check is the right model; only the origin threading gaps.

THE LAW (candidate): under no-loop, the suppression origin is
the MOST RECENT propagation context in the tuple chain; an
exists child born by BLOCKING carries the blocker's pctx, so a
left whose blocker arrived in the SAME evaluation inherits the
blocker's rule origin. The engine gap: the exists leftIns
blocking arm keeps the left's origin — on the relink fill
(origin None) the same-batch blocker's Some(o) is lost.

PORT SKETCH (after the ladder): phreak.rs leftIns blocking arm,
exists side only — if the left's origin is None and blocker `b`
is in THIS batch's sr.ins (any phase, incl. ph=1 upd-entry) or
sr.upd, emit the child with that staged right's origin. None-only
inheritance = can only ADD suppression in exactly the
Drools-suppressed shape. NOTED RESIDUAL (not touched): the D-127
temporal exists_flush_admit path has the same theoretical gap
(no-loop x temporal-exists x or-sibling x rule-born right — no
witness; model_exists_flush-calibrated surface).

### The ladder — PREDICTIONS REGISTERED BEFORE ANY CELL RUNS

All cells: types T0(i64,i64,bool)/T1(bool,bool)/T2(String) as in
the witness; base facts T1(t,t) + T2("") unless stated. R2 = 2
or-branches (b1: T1() and exists T2(!(f0 contains "b")); b2:
T1() and exists T2(f0 contains "a")), no-loop, RHS
insertLogical(new T2("a")) unless stated.

1. obn_min (the 2-CE minimization): PREDICT engine 2 firings vs
   oracle 1 (b2 suppressed); facts identical. Contingency: if the
   k=2 shape does not fork (unlink/fill mechanics differ),
   obn_min3 is the minimal anchor.
2. obn_min3 (b1/b2 each + `and not T0()` — the witness's 3-CE
   shape minus branch 3 and dead rules): PREDICT engine 2 vs
   oracle 1.
3. obn_noloop_off (obn_min minus no-loop): PREDICT 2 == 2 (the
   whole fork is no-loop suppression; terminates — dup
   insertLogical is WM-silent).
4. obn_split (b1/b2 as SEPARATE no-loop rules R2a/R2b, same
   RHS): PREDICT 2 == 2 (sameRules compares rule names — R2b's
   activation born of R2a's firing is NOT suppressed). Also the
   post-port regression control (origin Some(R2a), different
   parent → still fires).
5. obn_plain_ins (obn_min with plain insert): PREDICT fork
   persists — oracle 1 firing/3 facts vs engine 2 firings/4
   facts (pctx carries origin for plain inserts equally).
6. obn_ext_seed (obn_min + T2("a") stated at setup): PREDICT
   2 == 2 (both branches externally activated; insertLogical
   over stated is WM-silent).
7. obn_late_left (obn_min + epoch 2 inserts a second T1): THE
   MECHANISM DISCRIMINATOR — most-recent-pctx vs blocker-sticky.
   PREDICT oracle 3 (b1@e1; b1+b2 both fire for T1#2 — the new
   left's external pctx outranks the OLD rule-born blocker) vs
   engine 4 (the e1 fork + both e2 firings). If oracle is 2, the
   law is blocker-sticky and the port design changes (origin
   stored on rights permanently).
8. obn_upd_entry (REDESIGNED pre-build — the original one-branch
   $m binding violates the or-branch binding rule): b1: $m :
   T2(f0 != "x") and T1(); b2: $m : T2(f0 != "x") and T1() and
   exists T2(f0 contains "a"); RHS modify($m){ setF0("a") };
   facts T1 + T2("z"). The modify keeps $m matched, EXITS
   nothing, and upd-ENTERS b2's exists alpha (ph=1). PREDICT
   oracle 1 (blocker pctx = the modify's, origin R2 →
   suppressed; b1's own re-activation is the calibrated j04
   suppression both sides). Engine sub-prediction (LOWER
   confidence — depends on whether the relink fill stages the
   upd-touched left as a None-origin ins or an origin-carrying
   upd): 2 via the fill, else 1. Post-port: 1 both.
9. obn_foreign (R9 salience 10: T0() → insertLogical(new
   T2("a")); + obn_min's R2; + a T0 fact): PREDICT 3 == 3 (R9
   fires first; T2("a") born origin R9 → sameRules false → b2
   fires both sides). Post-port control: inherited Some(R9),
   foreign parent → still fires.

Oracle runs 3x per cell (count stability).

### LADDER RESULTS (2026-07-19) — 9/9 PREDICTIONS HIT

1. obn_min: engine 2 vs oracle 1 — HIT (2-CE anchor forks).
2. obn_min3: engine 2 vs oracle 1 — HIT.
3. obn_noloop_off: PASS (2==2) — HIT (no-loop is the whole fork).
4. obn_split: PASS — HIT (rule-name scoping).
5. obn_plain_ins: engine-extra T2("a") fact + count fork — HIT.
6. obn_ext_seed: PASS — HIT.
7. obn_late_left: engine 4 vs oracle 3 — HIT. THE DISCRIMINATOR:
   most-recent-pctx confirmed (the late external left FIRES over
   the old rule-born blocker; NOT blocker-sticky). Kills the
   store-origin-on-rights design; the same-batch lookup is right.
8. obn_upd_entry: engine 2 vs oracle 1 — HIT (oracle main
   prediction; engine landed on the fill-path sub-prediction).
9. obn_foreign: PASS (3==3) — HIT.

All five forking cells 3x byte-stable oracle-side (1/3, 1/3,
1/3, 3/4, 1/2 firings/facts). THE LAW STANDS AS PINNED.

NOTED RESIDUALS (witness-less, recorded not ported): (a) the
D-127 temporal exists_flush_admit path; (b) the left-UPD arm's
blocker-found re-block (an externally-updated left blocked by a
same-batch rule-born right would need the same inheritance —
phase order rightIns-before-leftUpd makes it reachable in
principle; no witness, calibrated left-upd surface untouched).

### THE PORT (D-350, landed 2026-07-19)

ONE edit: phreak.rs do_existential_node leftIns blocking arm
(exists side) — the child origin is the left's, or on a
None-origin left the same-batch blocker's (lookup in sr.ins any
phase + sr.upd). None-only inheritance: adds suppression in
exactly the Drools-suppressed shape, never overrides a live
origin. Post-port: 9/9 ladder cells PASS + the witness flips
PASS (R0+R2x2 both sides).

RECEIPTS: byte gate 2538/2539 vs 6bee9e8 (wt_pre350) — the ONE
diff is fz_777_1278 itself, ZERO certified movers; TEN
graduations (pr_obn_fz_777_1278 + the 9 pr_obn_* ladder cells);
bank 16 -> 15 (rebanked); make diff 11/1564/414 + drift 15
identical; lint-probes 2423/0/0; cargo 74; pytest 260 (fresh
.so, then tracked .so restored) + demo True; model_ird 31/31 (+
check_witnesses 26/26, validate_cells 39/39); IRD 150x5 seeds
7001/7002/6001/6003/9001 all 0-div; SD census 12x150 =
6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT; agenda_open x10 stable x3
(release/debug/wt_pre350); fresh fuzz 2x2000 seeds 349001/349002
CLEAN + fuzz_cep 3x300 seeds 349901-903 CLEAN. NEXT fuzz seeds
350001+/350901+. CHANGELOG Unreleased carries the entry (NINE
total).

Prediction scorecard for the round: 9/9 ladder hits, 1 recorded
lower-confidence sub-prediction hit (obn_upd_entry engine=2 via
the fill path); no misses.

## Remaining families (next slabs)

- ORDER trio, QUERY pair, the fz_123_6887 flapper census: per
  HANDOFF.md.

### xf_fz_296002_626 — DECODE START (2026-07-19, post-D-350)

Engine 7 vs oracle 5; the FIRST FIVE firings IDENTICAL (R1, R4,
R0, R4, R2). The extras: two more R4 firings — [T0#0(false),
T1(-3,12,t,f)] and [T0#0(false), T1(3,-1e9,t,f)]. LOCALIZED BY
TRUNCATION: base-only PASSES, base+epoch1 PASSES — the fork is
EPOCH 2, whose actions are a T0#0 f0 false->true->false
ROUND-TRIP + an inert T1(f2=false) insert. After epoch 1 the
subnetwork-not blocker (T1(f0>=2.0, f3==false) and T0(f0!=false)
— satisfied by T1(3,-1e9) x the epoch-1 T0(true)) STANDS and
correctly blocks all R4 tuples BOTH sides (the e1-only PASS).
Class hypothesis: the outer T0's alpha exit+re-entry re-forms
the R4 tuples and the engine's SUBNETWORK-NOT does not consult
the STANDING blocker for the re-entry tuples; the oracle keeps
them blocked. (Kind::SubnetNot machinery, phreak.rs ~651 — NOT
the D-350 law; no no-loop involved.)

MIN CELL PREDICTION (registered before the run) — m626: one
rule `T0(f0 != true, $b : f0) T1(f2 != false) not(T1(f0 >= 2,
f3 == $b) and T0(f0 != false))`, RHS empty; facts T0(false) +
T1(3,0,true,false); epoch 1 inserts T0(true); epoch 2 updates
T0#0 -> true then -> false. PREDICT oracle 1 firing (base only;
the re-entry tuple stays blocked) vs engine 2 (the re-entry
re-fire). If NO fork, the missing ingredient list to ladder
next: the epoch-1 T1 update churn, the second T1, the
dyn-salience/focus machinery, both epoch-2 actions in one
window vs split.

RESULT: HIT — m626 forks engine 2 vs oracle 1, oracle 3x
byte-stable (1 firing / 3 facts). The minimal anchor is
probes_pending/oldbank/m626.json (one rule, two facts, two
epochs; setFocus/salience/acc all shed — NOT ingredients).

### THE SUBNET RE-ENTRY ROUND (D-351) — full decode

ENGINE MECHANISM (SEINE_TRACE + source): the epoch-2 T0#0
false->true->false round-trip stages as left DEL+INS (exit +
re-entry; joins keep del+ins by the D-326/jr pins). The subnet
tuple [F1,F2,F2,F3] CONTAINS F1, so it dies+re-forms in the
SAME batch: the tip emits del (leftDel phase, staged first)
then ins (staged second) — they coexist in the tip trg. THE
CONFLATION: the RIA hop (engine.rs ~10206) routes INS FIRST
then DEL, and Staged::add_del CANCELS a staged same-VALUE ins
("never materialized", phreak.rs:589) — but this ins/del pair
is OLD-generation del + NEW-generation ins of the same VALUE
(Drools: different tuple OBJECTS, never folded). sn_right nets
EMPTY -> eval_subnet_node: leftDel wipes sn_matches, leftIns
sees no matches -> child ins -> the extra firing.

DROOLS MECHANISM (PhreakSubnetworkNotExistsNode, verbatim): (1)
matches live ON the start tuple object (getStartTuple /
setContextObject) — a dead start's del no-ops (context nulled
by deleteLeft), a re-created start is a NEW object; (2)
insertRight runs BEFORE insertLeft ("so 'not' knows if there
are matches before creating the child") — a re-entering left
with a same-batch re-formed blocker is born BLOCKED; (3)
deleteRight runs LAST; updateRight is a NO-OP; (4) upstream,
Drools stages WM updates BY FACT IDENTITY with values read at
propagation time — the round-trip is ONE value-preserving
update, tuples never die (the D-326 fold).

THE DESIGN TRAP (found in case analysis, pre-registered): an
INNER-fact round-trip (blocker exits+re-enters, start ALIVE) is
accidentally-correct TODAY precisely because of the hop
annihilation (nets empty = Drools' updateRight no-op). Removing
the cancel without a generation guard would flip that case to a
spurious unblock (rightDel would eat the re-added match). So
the fix is TWO-PART:
  (i) RIA hop routes DELS FIRST then INSS then UPDS within one
      trg batch — same-trg old-del+new-ins both survive; the
      cross-batch never-materialized cancel (sn_c5b) is
      UNCHANGED (within one tip call, del phases precede ins
      phases, so a same-call same-value pair is always
      old-del+new-ins — a genuine create+delete never coexists
      in one call);
  (ii) eval_subnet_node: rightIns collects `readded` (every
      staged subnet value, UNCONDITIONALLY — including the
      value-idempotency skip path); rightDel SKIPS s in readded
      (the value-keyed stand-in for Drools' dead-object
      null-context no-op). Kind-agnostic (not AND exists).

### The D-351 ladder — PREDICTIONS REGISTERED BEFORE ANY RUN

All cells derive from m626 (T0(bool); T1(i64,i64,bool,bool);
rule T0(f0 != true, $b : f0) / T1(f2 != false) / not(T1(f0 >=
2, f3 == $b) and T0(f0 != false)); base T0#0(false) + T1(3,0,
t,f)). Round-trip = update target0 ->true then ->false.

1. obs_unblocked_rt (NO blocker ever; round-trip at epoch 1):
   PREDICT 2==2 PASS pre-port (both refire: Drools folded
   update -> live child update -> re-fire; engine del+ins ->
   child del+ins -> re-fire). Unchanged post-port.
2. obs_exists_rt (exists(...) instead of not; epoch-1 blocker;
   epoch-2 round-trip): PREDICT PRE-PORT FAIL engine 1 vs
   oracle 2 — the SAME annihilation SUPPRESSES the legitimate
   exists re-fire (child del survives, re-ins lost). Post-port
   2==2. The exists mirror is a SECOND witness class of the
   same bug, opposite direction.
3. obs_split_epochs (->true in epoch 2, ->false in epoch 3):
   PREDICT 1==1 PASS pre-port — the clean re-entry is ALREADY
   blocked (rightIns-before-leftIns re-forms matches first).
   Isolates the fork to same-batch staging conflation, NOT
   re-entry per se.
4. obs_win_split (one epoch: actions [insert T0true, upd
   ->true, upd ->false]): PREDICT pre-port FAIL engine 2 vs
   oracle 1 (the round-trip re-forms the blocker in-batch =
   m626 after window 1); post 1==1.
5. obs_two_blockers (epoch-1 inserts TWO T0(true); epoch-2
   round-trip): PREDICT pre-port FAIL engine 2 vs oracle 1;
   post 1==1 (multi-match bookkeeping: both wiped, both
   re-added, both dels skipped).
6. obs_inner_rt (epoch-1 T0true; epoch-2 round-trips the INNER
   fact, target 2: ->false then ->true): PREDICT 1==1 PASS
   pre-port (annihilation accidentally-correct) AND post-port
   (the readded guard's justification cell — MUST NOT regress).
7. obs_blocker_late (one epoch: actions [upd ->true, upd
   ->false, insert T0true]): PREDICT 1==1 PASS pre-port and
   post (block cancels the pending re-fire both sides; child
   del reaches a fired activation = no-op).
8. The witness xf_fz_296002_626: post-port PREDICT 5==5 (both
   extra R4 firings are start round-trips with same-batch
   re-formed blockers — the subnet tuples contain T0#0).

Oracle 3x per forking cell.

### D-351 LADDER RESULTS — 6/7 HIT, 1 recorded miss

1. obs_unblocked_rt: PASS (2==2) — HIT.
2. obs_exists_rt: FAIL engine 1 vs oracle 2 — HIT (the exists
   mirror forks the OPPOSITE direction, 3x stable 2/3).
3. obs_split_epochs: PASS — HIT (clean re-entry already
   blocked; the fork IS the same-batch staging conflation).
4. obs_win_split: PASS — MISS (predicted 2v1 fork). Decode: the
   three actions stage ACROSS windows without intermediate
   evals; window-2's subnet del cancels window-1's STILL-STAGED
   blocker ins at the hop (cross-call = genuinely
   never-materialized, the CORRECT sn_c5b semantics), window-3
   re-stages it — the eval then sees the clean-re-entry shape
   and blocks. Composed behavior correct both sides; post-port
   route identical (cross-call cancel unchanged). Benign miss,
   mechanism recorded.
5. obs_two_blockers: FAIL engine 2 vs oracle 1 — HIT (3x
   stable 1/4).
6. obs_inner_rt: PASS — HIT (the trap cell: annihilation
   accidentally-correct; post-port MUST stay 1).
7. obs_blocker_late: PASS — HIT.

TRACE NOTE (m626): the term's extra ins arrived ph=0
(leftIns-created) — the fork route is the HOP ANNIHILATION
(both windows composed into ONE tip call). The rightDel route
(fix part ii) is reachable when del and ins arrive in SEPARATE
hop calls with a live start — no current witness, but the
obs_inner_rt post-port analysis proves the guard is required
once part (i) lands.

### THE PORT (D-351, landed 2026-07-19)

Two edits, exactly the pre-registered design: (i) the RIA hop
routes dels FIRST (engine.rs Sink::Ria arm); (ii)
eval_subnet_node rightIns collects `readded` unconditionally,
rightDel skips readded values. Post-port: 9/9 PASS (both forks
flip — obs_exists_rt 2==2, obs_two_blockers 1==1; the trap
cell obs_inner_rt HOLDS at 1==1; m626 1==1; the witness
xf_fz_296002_626 flips 5==5 exactly as predicted — both extra
R4 firings were start round-trips with same-batch re-formed
blockers).

RECEIPTS: byte gate 2547/2549 vs f66778a (wt_pre351) — the 2
diffs ARE the movers (m626 + the witness), zero certified
movement; NINE graduations (pr_obs_fz_296002_626 + pr_obs_m626
+ 7 pr_obs_* ladder cells); bank 15 -> 14; make diff
11/1573/414 + drift 14 identical; lint-probes 2432/0/0; cargo
74; pytest 260 (fresh .so, tracked .so restored) + demo True;
model_ird 31/31 (+26/26, +39/39); IRD 150x5 0-div; SD census
6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT; agenda_open x10 stable x3
(release/debug/wt_pre351); fuzz 2x2000 seeds 350001/350002
CLEAN + cep 3x300 seeds 350901-903 CLEAN. NEXT seeds
351001+/351901+. Round scorecard: 6/7 ladder hits + 1 benign
recorded miss (obs_win_split cross-window composition), all
post-port predictions hit incl. the witness 5==5.

THE COUNT FAMILY IS CLOSED (fz_777_1278 D-350 +
fz_296002_626 D-351). Remaining: ORDER trio, QUERY pair,
fz_123_6887 flapper census.

## Family 4 — the ORDER trio (D-352 round, opened 2026-07-19)

ONE family: identical or-branches sharing >= 2-pattern prefixes
with plain rules (the or-branch expansion leaked through the
D-262-era generator's shared-prefix wall): fz_7331_973 (R2 b1==
b3 == R0's LHS), fz_8087_1020 (R2 branches + no-loop R4 share
T0(f3,f3) x T0(f2) self-join), fz_141421_123 (R0/R3b1/R3b3/R4
share T1 x T1). All three oracle-10x-stable (fz_8087_1020
checked — NOT the fz_42_84 identity-hash nondeterminism class).

### fz_7331_973 — HAND-DECODE COMPLETE (handle-tagged logs both
### sides, jobs tmp e973h/o973h)

BASE batch: IDENTICAL both sides, all four sinks (b1, b2-node,
b3, R0-salience-last). The base pins sink orientation: b1/b3 =
CREATION order, R0 = reversed — one batch, so whole-vs-per-batch
flip is indistinguishable there.

EPOCH batch (T0#1 "beta"->""->"b" double-update re-entry + two
T1 left inserts): forks on every sink, one root composition:

ORACLE (all six lists fit ONE machine, uniquely):
- creation = PLAIN slots: rightIns(re-entrant, memory-appended
  at tail) walks lefts memory-forward [L1,L3] FIRST, then
  leftIns staged-head-first [L8,L7] x rights [R4, R2-at-tail].
- sink distribution PER PHASE-BATCH: first-BUILT sink (R0,
  decl-first) gets each phase batch PREPEND-within, batches
  appended FIFO -> [(L1,R2),(L3,R2)] ++ [(L7,R2),(L7,R4),
  (L8,R2),(L8,R4)]; later sinks (b1/b3) get each batch in
  CREATION order, same batch order -> [(L3,R2),(L1,R2)] ++
  [(L8,R4),(L8,R2),(L7,R4),(L7,R2)]. Observed EXACTLY.
ENGINE: D-083 late pass (leftIns x pre-batch memory first, then
re-entrant x lseq-desc) + WHOLE-trg flip for later sinks —
self-consistent, wrong on both counts for this shape.

THE LAW (candidate, pre-registered): (1) multi-sink propagation
is per-phase-batch granular (first-built prepend-within/append-
across; later sinks creation-order-within/append-across); (2)
the D-083 re-entrant late pass is SINGLE-SINK-scoped — on shared
nodes re-entrant rights take the plain slot. All 22
model_check_join2 timelines are single-sink probes -> untouched
by construction; jr3/jr17's late orders stay (single-rule).

RECONCILIATION NOTE: D-083's gate=reentry survivor was fitted
entirely on single-sink data; 973 is the first shared-node
re-entry observation. The scope refinement does not overturn
the single-sink law.

### PREDICTIONS (registered BEFORE the port/model run)

P1. A verification model (tools/model_check_join3.py) encoding
    {candidate, engine-current} over 973's six lists: candidate
    fits 6/6, engine-current fits its own engine logs 6/6 and
    the oracle 2/6 (base only).
P2. Post-port fz_7331_973 flips PASS entirely (all 38 firings).
P3. fz_8087_1020 and fz_141421_123 MOVE TOWARD the oracle;
    full PASS = the family is one law (strong form); partial =
    additional laws recorded (weak form, still progress).
P4. Byte gate: movers confined to shared-prefix cells with
    multi-phase mutation batches; ne_s1..s11 (insert-only,
    single-batch) byte-identical; all single-sink cells
    byte-identical by construction.

### D-352 ROUND RESULTS — LAW VERIFIED, PORT REVERTED (protocol)

P1 HIT: model_check_join3.py — candidate fits oracle 6/6,
engine-model fits engine logs 6/6, cross-fit correctly fails.

P2 HIT (after one wiring fix): with the port landed (plain slot
on multi-sink joins + first-sink phase-block swap; multi_sink
stamped at the lists_built site — the first attempt's sweep sat
in stream_flush_ex, event-sessions only, and the half-engaged
port was caught by the witness), fz_7331_973 PASSED entirely.
The law IS the complete account of that witness.

P3 PARTIAL (the weak form, as pre-registered): fz_8087_1020
moved (5->1) not fixed — its [A,A] refire rides the oracle's
UPD channel at wave head; the engine's composition differs
(self-join both-sides staging + in-batch left-upd + downstream
exists; own decode needed). fz_141421_123: a DIFFERENT-FACT
fork (which R1-born T1 lands in the tuple) — a different law.

P4 MISS — THE ROUND'S BIG FINDING: 14 CERTIFIED shared-prefix
cells hold the OPPOSITE order and FAILED the oracle under the
port: pr_or_a28, pr_or_a29, pr_ib15, pr_ib15b, pr_ib28,
fz_123_3482, fz_123_8822, fz_42_4816, fz_42_580, fz_42_952,
fz_999_6009, fz_min_580, fz_min_8822 (+ fz_9005_450 in
failures/). BOTH behaviors are oracle-certified in different
shared shapes — the D-082->D-083 pattern repeating at the
SHARING level. PORT REVERTED (D-331/D-345 protocol): engine
byte-identical to 168a467, movers re-verified PASS, witness
re-banked, make diff 11/1573/414 + drift 14 identical.

THE NEXT ROUND (model-first, its own slab): extend
model_check_join3.py into a full eliminator — timelines =
973's six lists + jr17 + hand-extracted oracle logs from the
14 counterexamples; discriminator dimensions to enumerate:
eager (no-loop) sharer present, Term-vs-Node sink kinds,
first-built vs first-evaluated direct-sink assignment,
salience split among sharers, or-sibling vs cross-rule
sharing, linked history, batch composition (upd-in-batch vs
ins-only). The engine's D-083-late + whole-flip is provably
right on the 14 and provably wrong on 973 — the discriminator
is IN that delta.

## D-353 — the eliminator round (Bryan: "start the modeling")

### Step 1 — FACTORIZATION (mechanical, done first)

Variant A (plain slot only, no block swap) rebuilt and run over
all 15 cells: ALL 14 counterexamples PASS, 973 still FAILS.
=> Every certified breakage came from the BLOCK SWAP; the
plain-slot change alone breaks nothing; 973 needs more than
the plain slot. Engine re-reverted byte-identical after the
experiment.

### Step 2 — THE WINDOW REFRAME (fits all six lists + explains
### the swap's accident)

Re-deriving under the factorization constraint, the six 973
lists fit a WINDOW-GRANULAR mechanism with NO block swap:
- each external ACTION = one window; epoch FACTS = one combined
  window (w2 = the re-entrant right's window, w3 = both T1s);
- each window's node eval distributes its trg separately:
  first-BUILT sink APPENDS (FIFO across windows), later sinks
  get peer-prepend copies (creation order within a window's
  block, double reversal);
- EAGER (no-loop) sinks consume their staged copies PER WINDOW
  (b1/b3 drained w2's block before w3 landed); LAZY sinks
  accumulate (R0, salience -4, drains everything at its fire).
Fit check by hand: b1/b3 = [w2-block creation ++ w3-block
creation] (eager, FIFO-of-consumes) EXACT; R0 = [w2-append ++
w3-append] (staged form within) EXACT. The D-352 "phase-block"
fit was an ARTIFACT: 973's phase blocks coincide with its
window blocks (the right's ops and the lefts' ops arrive in
different windows). The 14 counterexamples break the swap
because their phase blocks do NOT align with windows.

### Step 3 — ORACLE PROBES (predictions BEFORE runs; no engine
### changes needed to falsify the window law)

p353a (973 with the two T1 inserts as separate insert ACTIONS —
three windows instead of two): PREDICT the oracle b1/b3 wave
tail FLIPS to L7-block-then-L8-block ([(-2,R4),(-2,R2),(6,R4),
(6,R2)] in value terms, eager FIFO-of-window-consumes); R0
stays [(1.5,R2),(3.5,R2),(-2,R2),(-2,R4),(6,R2),(6,R4)]-class
(append order w3 then w4 = L7 then L8). If the tail does NOT
flip, the facts-vs-actions window split is wrong and the law
needs re-derivation.
p353b (973 with no-loop REMOVED from R2 — all sinks lazy):
PREDICT b1/b3 = LIFO-across-windows head-first = [(6,R4),
(6,R2),(-2,R4),(-2,R2),(3.5,R2),(1.5,R2)]; R0 unchanged
(append-across is consume-time-independent).

### Step 3 RESULTS — BOTH PROBES MISSED (the window law is DEAD)

p353a: output IDENTICAL to the witness (3x stable) — actions-vs-
facts window structure changes NOTHING oracle-side. p353b:
IDENTICAL too — no-loop/eagerness is NOT the mechanism. Both
misses recorded. [SUPERSEDED one step later: the window IDEA was
wrong in its facts-vs-actions FORM, but a per-CALL form survives
— see step 4; both p353a/b misses are CONSISTENT with it.]

### Step 4 — THE FLUSH-AT-MODIFY MECHANISM (fits ALL data)

Timeline extraction from pr_ib15 + pr_or_a28 (handle-tagged):
their R0 (first-built, salience -1) receives WHOLE-EVAL-reversed
blocks FIFO across the R2-firing-driven evals; their R1 receives
creation order — the engine's current certified composition
EXACTLY. Composing this with 973's six lists, ONE mechanism fits
everything:
  (1) an external MODIFY call FLUSHES the network AT CALL TIME
      (BetaNode.doDeleteRightTuple / modify-assert ->
      setNodeDirty -> shouldFlush -> flushLeftTupleIfNecessary);
      activations queue at the call;
  (2) an external INSERT only STAGES (LIA); it drains in ONE
      fire-time batch, staged-LIFO — PROVEN by the witness's own
      lins-block ([6-children before -2-children] in b1);
  (3) per flush-batch: first-built sink = addAll (drain =
      prepend-list head-first), later sinks = per-entry-prepend
      (drain = creation order); term QUEUES accumulate FIFO
      across flushes.
Under (1)-(3): 973's b1 = [rins-batch at update-call ++
fire-batch lins] EXACT; R0 EXACT; ib15/or_a28 (RHS-driven
per-firing flushes) = the current engine EXACT; p353a's miss
(actions vs facts identical) EXACT — both forms flush the
updates at call; p353b's miss (no-loop irrelevant) EXACT — the
flush is call-driven, not eagerness-driven. The D-352
"phase-block swap" fit 973 because its phase blocks COINCIDED
with call-flush batches; it broke the 14 because their
RHS-driven batches are whole-eval blocks.

### Step 5 — the law-1 discriminating probes (predictions FIRST)

p353c (973 epoch = the two updates ONLY, no T1 inserts):
PREDICT R2 waves = [(3.5,b),(1.5,b)] per branch (the rins batch
queued at the update-2 call flush; creation order — peers),
branch2 wave EMPTY, R0 wave = [(1.5,b),(3.5,b)] (addAll
prepend-list). Total epoch firings 6: b1 2, b3 2, R0 2.
p353d (973 epoch actions REORDERED: [insert T1(-2), insert
T1(6), upd A->"", upd A->"b"], facts empty): the update-1 call
flush DRAINS the staged T1s too (a flush evaluates the whole
network) with A absent (exited); update-2's flush adds the
re-entrant's children over ALL four lefts. PREDICT b1 wave =
[(6,R4),(-2,R4)] ++ [(3.5,b),(1.5,b),(6,b),(-2,b)]; R0 wave =
[(-2,R4),(6,R4)] ++ [(-2,b),(6,b),(1.5,b),(3.5,b)]. These are
DISTINCTIVE (neither whole-LIFO nor the witness's order); a hit
here is near-conclusive for flush-at-modify.

### Step 5 RESULTS — p353c FULL HIT; p353d miss REFINES the law

p353c: b1 [(3.5,b),(1.5,b)], b3 same, R0 [(1.5,b),(3.5,b)],
branch2 empty, 6 epoch firings — EVERY registered prediction
exact (3x stable). p353d: output IDENTICAL to the witness —
action order (inserts before updates) changes NOTHING. The miss
decodes cleanly: flushLeftTupleIfNecessary is SEGMENT-scoped —
the modify's flush evaluates the JOIN's own smem (its staged
rights x MEMORY lefts); LIA-staged lefts are a DIFFERENT
segment, untouched, draining at fire regardless of call order.

### THE FINAL LAW (D-353; all data fits, zero residuals)

1. An external MODIFY call flushes the AFFECTED BETA SEGMENT at
   call time (BetaNode.doDeleteRightTuple / modify-assert ->
   setNodeDirty -> shouldFlush -> flushLeftTupleIfNecessary):
   staged rights evaluate against MEMORY lefts, children
   propagate to term smems, ACTIVATIONS QUEUE AT CALL TIME.
2. An external INSERT only stages (LIA segment); it drains at
   fire in ONE batch, staged-LIFO.
3. RHS-driven staging drains per-firing flush (the certified
   current composition).
4. Per flush-batch sink distribution: first-BUILT sink = addAll
   (drain = prepend-list head-first); later sinks = per-entry
   prepend (drain = creation order). Terminal QUEUES accumulate
   FIFO across flushes.

THE UNIFICATION: under this law, jr3's certified "late" order
falls out naturally ((1,5) queued at the back-flush, (2,5) at
fire) and jw3 stays whole-LIFO (one RHS flush batch) — THE
ENTIRE D-082/D-083 LATE-PASS MACHINERY IS THE COALESCED-MODEL
APPROXIMATION of per-call segment flushes: exact on single-sink
shapes (where queue-time is invisible), breaking only on
multi-sink shared nodes where flush boundaries become visible
(fz_7331_973). The 14 counterexamples are RHS-driven (law 3) —
untouched by any of this.

Evidence ledger: 973's six lists (exact), ib15 + or_a28
timelines (exact), jr3/jw3 unification (exact), p353c full hit,
p353a/b/d misses all explained by the final form. Verifier:
tools/model_check_join3.py (the D-352 fit) + this analysis.

### PORT ASSESSMENT (gate with Bryan — ARCHITECTURE decision)

The faithful port = evaluate affected join segments at each
external UPDATE window (queueing activations at window time),
which would eventually RETIRE the D-082/D-083 late pass (a
simplification, but the blast radius = the whole update-order
certified corpus: jr/jw ladders, u12/u13/u16, the D-047 window
machinery, agenda queue-time interactions, TMS-under-epochs).
Alternatives: (a) full flush-at-window restructure (faithful,
big — its own arc like D-076); (b) a targeted multi-sink-only
approximation (the D-352 block-swap was one attempt; a
window-boundary-aware variant could work but needs the
eliminator run over ALL timelines first); (c) leave the trio
banked with the law recorded (they are ORDER-class, value-safe).
fz_8087_1020/fz_141421_123 need their own decodes either way
(different compositions). NO ENGINE CHANGES THIS ROUND.
