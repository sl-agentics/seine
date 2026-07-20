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

### D-354 — THE PORT LANDS (Bryan: "let's go with a")

Reconciling the faithful law with jr11 (pure entries never
flush; link-transition triggers can't reach the shared node)
collapsed option (a)'s OBSERVABLE to an eval-time distribution
gate: the flush-at-modify composition appears exactly when an
external batch carries a MODIFY-BORN re-entrant. THE PORT
(D-352's edits + TWO gate bits): (i) multi-sink plain joins
route EXTERNAL ph=1 (origin-None) rights through the plain
fresh-right walk (late pass stays for single-sink + RHS
staging); (ii) the first-sink phase-block swap engages ONLY
when sinks>1 AND Join AND !temporal AND every trg ins-child is
origin-None AND the batch's staged rights contained an external
ph=1 re-entrant (`ext_reentry_batch`). The gate discovery
sequence (recorded): origin-None alone broke fz_42_4816 +
fz_999_6009 — their external batches are PURE INSERTS = one
fire-time flush = whole-LIFO correct; the modify-born signature
restored them. Physical reading: per-phase blocks exist iff the
epoch had separate flush events (the modify's segment flush +
the fire drain); a pure-insert epoch has one.

RESULTS: fz_7331_973 PASSES + all 14 counterexamples PASS +
jr11/jr17 sanity PASS. Byte gate 2556/2560 vs 6082206
(wt_pre354): the 4 diffs = the witness + p353a/b/d (same-shape
probes, all now oracle-PASS; p353c single-block = byte-neutral
✓ consistent). FIVE graduations (pr_ot_fz_7331_973 +
pr_ot_p353a-d); bank 14 -> 13; make diff 11/1578/414 + drift 13
identical; lint 2437/0/0; cargo 74; pytest 260 + demo True;
model_ird 31/31 (+26/26+39/39); IRD 150x5 0-div; SD census
6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT; agenda_open x10 stable x3;
fuzz 2x2000 seeds 351001/351002 + cep 3x300 351901-903 CLEAN.
NEXT seeds 352001+/352901+.

### fz_8087_1020 — DECODE (D-355 round, post-D-354 re-measure)

Handle-tagged logs both sides: base R4 [A,A] + R2-b1's wave
(firings 1-4 [AA,AB,BA,BB]) + R4's wave (9-12) are IDENTICAL;
the fork is R2-b2's wave ALONE (5-8): engine [AA,BB,BA,AB] vs
oracle [BB,BA,AB,AA] — same multiset, and the delta reduces to
ONE element: the [A,A] refire (the A f3-update's child) fires
FIRST engine-side, LAST oracle-side, AT B2 ONLY (b1 has it
FIRST on BOTH sides).

Structure decoded: the shared T0xT0 join's sinks = [b1-exists
(first-built, direct staged = reverse-creation), b2-exists
(peer = creation), R4-term (peer)]. The T1 batch arrives at the
exists RIGHTS; the ins-children compose exists-phase (leftUpd
BEFORE leftIns) x term-LIFO into both waves EXACTLY (b1 ins
[AB,BA,BB], b2 ins [BB,BA,AB] both sides). The ONLY open bit:
the [A,A] child is BORN at the exists leftUpd phase (no prior
child — exists was false at base, so the left-upd creates a
fresh child) and the CHANNEL it rides differs by sink kind:
b1 (direct) = the UPD channel (fires before ins, both sides
agree); b2 (peer) = oracle INS-class-last vs engine UPD-first.
THE QUESTION: Drools' peer staging of an update for a tuple
with no prior peer child (updateChildLeftTuple / peer
same-kind semantics) vs the engine's peer_upd marker路径.

MINIMAL ANCHOR PREDICTION (registered before run) — m1020:
strip to 2 or-branches + the shared join + base T0(A) +
epoch [insert B, upd A(dead-field), T1 facts]; drop R4 and
dead rules. PREDICT the fork survives: engine b2-wave AA-first
vs oracle b2-wave AA-last; b1-wave identical both sides.

RESULT: MISS — m1020 (2 sinks) PASSES, and its wave orders
SWAP vs the witness's (b1 [BB,BA,AB,AA], b2 [AA,AB,BA,BB]) —
the 2-sink orientation composes correctly on both sides. THE
THIRD SINK IS AN INGREDIENT: m1020b (+ no-loop R4 declared
after R2 — the witness's third sink) FORKS AT firing[5]
EXACTLY like the witness (engine [A,A] vs oracle [B,B]),
oracle 3x stable. THE ANCHOR: probes_pending/oldbank/
m1020b.json — three rules, ONE base fact.

NEXT SLAB (fresh budget): the exists-node peer-upd channel
round — Drools source read (PhreakExistsNode.doLeftUpdates
child-creation staging + peer same-kind semantics /
updateChildLeftTuple) + a b1-vs-b2 channel probe ladder off
m1020b, then the port. The open question is crisp: at the
2-sink form both sides agree; adding the THIRD sink flips the
oracle's b2 [A,A]-refire to the ins-class-last channel while
the engine keeps upd-first. fz_141421_123 NOT touched this
round (deep different-fact fork; own decode).

### fz_141421_123 — DECODE (D-356 round; Bryan: "you have room")

Handle-tagged logs both sides (239 firings each — counts
MATCH; the "different-fact" read was wrong, it is pure ORDER).
The fork: firings 141-167 = R4's wave, which is the
SUBNET-EXISTS MASS-UNBLOCK — R5 deletes T1(-4,"beta") at
firing 140, flipping `exists(T1() and not(T1(f0 < -3)))` for
ALL parked R4 tuples at once; the wave is stable-sorted by
R4's dynamic salience ($a-$b), and WITHIN each equal-salience
group the four f0=11 facts (two R1-firing generations x the
beta-then-a RHS insert pair) order:
  engine [a#22, beta#21, a#24, beta#23] = join-CREATION order
    per generation (RHS staged LIFO: a processed first),
    generations FIFO;
  oracle [beta#21, a#22, beta#23, a#24] = INSERTION order per
    generation, generations FIFO.
ONE parity difference in the chain [R1-flush trg -> R4's sink
(4th, peer) -> subnet-exists left arrival -> 0->1 emission ->
queue -> dyn-salience stable sort]. R4's sink structure: the
shared T1xT1 prefix (R0-term first-built, R3b1/b3 terms,
R4-exists 4th); the same shared-prefix family as the rest of
the trio.

MINIMAL ANCHOR PREDICTION (registered before run) — m123:
R0 (sal -10, the share) + R1 (T0 -> insert T1(f1,"beta");
insert T1(f1,"a")) + R4 (salience($a-$b), T1 x
T1(!(f0 <= -1000000007)) + exists(T1() and not(T1(f0 < -3))))
+ R5 (delete T1(f0 < 2)); base T1(-4,"beta") + T1(2,"ab") +
TWO T0(f1=11). PREDICT the fork survives: engine fires
[11,a] before [11,beta] within each generation in R4's wave;
oracle the reverse. Contingency (the m1020 lesson): if the
2-sink form passes, add R3-like or-twins to restore R4's
4th-sink position.

RESULT: HIT FIRST SHOT — m123 forks at firing[3] with the
witness's exact shape (engine [11,a] vs oracle [11,beta]),
oracle 3x byte-stable, 53 firings; the oracle wave = [beta#5,
a#6, beta#7, a#8] x [2,ab] — INSERTION order per generation,
generations FIFO, exactly the witness's law. The 2-sink form
SUFFICES here (unlike m1020 — no contingency needed): four
rules, four base facts. THE ANCHOR:
probes_pending/oldbank/m123.json.

NEXT SLAB: the subnet-exists mass-unblock emission-order
round — the chain [RHS-flush trg -> R4's peer sink ->
subnet-exists left arrival -> 0->1 emission -> queue ->
dyn-salience stable sort] has ONE parity off engine-side
(engine emits join-creation order per generation, oracle
insertion order). Candidate mechanism: the sn/exists left
arrival at a PEER (Node) sink — the same peer-chain-parity
class as m1020b's channel question; the two anchors may fall
to ONE law (both are non-Term peer sinks of a shared join).
Source targets: the sn-machine 0->1 left-walk order +
peer_merge_left's ins-order semantics vs SegmentPropagator.

### D-356 continuation — the trace + source round (the law)

ENGINE TRACE (m123): the subnet lowers to a Not node holding
3-tuples [l1, l2, innerT1] blocked by T1(-4); R5's delete
unblocks ALL; the engine's term[2] consume order = gens LIFO
(gen-2 pairs first), within-gen [beta, a]. The WITNESS's
engine wave = gens FIFO, within-gen [a, beta] — TWO different
engine compositions for near-identical shapes (R4 = 2nd sink
in m123, 4th in the witness): the ENGINE'S PARITY FLIPS WITH
SINK POSITION.

DROOLS SOURCE (PhreakNotNode.doRightDeletes): the unblock
walks rightTuple.getBlocked() from the HEAD = most-recently-
blocked-first (SAME walk direction as the engine); its trg
addInsert PREPEND + terminal head-first drain normalize the
net order.

THE LAW (both anchors + the witness, invariant): the
not-unblock MASS EMISSION reaches every sink in FACT-INSERTION
order (oldest generation first, insertion order within),
INVARIANT to sink position/count — the oracle gives
[beta-g1, a-g1, beta-g2, a-g2] in BOTH shapes while the engine
flips parity with structure.

PORT SKETCH (next slab — the D-031 surface, heavily pinned:
blocked-list PREPEND, the D-127 exists-flush most-recently-
blocked-first admit, sd_/tms parked lanes all ride it): a
NORMALIZATION at the mass-unblock emission (the D-343 sort
precedent) — emit the unblock batch in creation/insertion
order rather than relying on parity composition; byte gate
decides the blast radius. m1020b's channel question (upd-vs-
ins class at the peer) is likely the SAME normalization seen
from the update side. Both witnesses stay banked until the
port round.
(unprobed — BetaNode's delete path also flushes; no witness);
left-side-modify flush behavior (unconstrained by data);
RHS-driven re-entrants on multi-sink nodes (late pass retained
there — no witness); the rins-walk memory order on shapes where
engine append-order diverges from add-at-head (973's happened to
coincide). fz_8087_1020 (upd-channel refire composition) and
fz_141421_123 (different-fact fork) stay banked — own decodes.
THE ORDER TRIO: 1 of 3 closed, 2 recorded. The D-082/D-083 late
pass UNIFICATION (it is the coalesced approximation of
flush-at-modify) stands as the arc's theory — full retirement =
a future arc if ever needed.

### D-357 — the mass-unblock port round (per HANDOFF-unblock.md)

FULL-WAVE EXTRACTION (2026-07-19, logs regenerated, oracle 3x
byte-stable all three cells): complete group tables for m123 (25
pairs) + xf_fz_141421_123 (49 pairs) with recomputed dyn salience.
THE FINDING THAT BREAKS D-356b'S MECHANISM READ: m123's oracle
sal-0 group is MERGED across generations ((5,7) precedes (6,5) —
causally impossible under per-batch parking + FIFO emission), while
the witness's is BATCH-MAJOR; and the witness's sal -1 group has
(1,3) before (3,21), killing row-major single-batch structure.
D-356b's invariant ("fact-insertion order, invariant to sink
position") survives only as the per-generation projection; the
full law is richer. doRightDeletes/blocked-walk applies at most to
the setup-parked S-pairs; the wave's G-batches NEVER PARK.

THE LAW (D-357, verified EXACT on both cells,
tools/model_check_unblock.py, 8 ablations refuted):
the oracle's post-setup pairs accumulate as STAGED LEFTS at the
R4-subnet join (lazy segment schedule — that segment evaluates
only when R4's executor pops, post-delete); the delete's rightDel
phase precedes leftIns, so the blocker is gone when the staged
lefts flow through: NO parking, no blocked-walk for them. Wave
order = stable-dyn-salience sort over the ACCUMULATION order B:
  B = concat over join1-flush batches (FIFO), where a batch
  boundary = an evaluation of a join1-SHARING rule with pending
  staged facts (witness: R3's gb passes split S/G1/G2; m123: R0
  sal -10 never pops pre-delete, so G1+G2 MERGE; setup S is its
  own batch — t=0 sweep, forced by the witness's (1,3)<(3,21));
  per batch, new facts N in drain order (external LIFO, RHS FIFO,
  merged batches concatenated in firing order) over old facts O
  (per-batch blocks, most-recent-batch-first, block-internal in
  drain order):
    B_batch = [(a,b) for a in N for b in N++O]   (leftIns rows)
           ++ [(a,b) for b in N for a in O]      (rightIns-born)
UNDERDETERMINED by the two cells: O-block order (most-recent-first
vs oldest-first never co-occur in one salience group) — the
recent-first form is the coherent extrapolation (new-block-first
is pinned by both cells' row walks); p357c discriminates.

PROBE PREDICTIONS (registered BEFORE any cell runs; cells to be
authored as probes_pending/oldbank/p357{a,b,c}.json; predictions
computed mechanically from the law; if a probe's PRE-WAVE timeline
deviates from the assumed one, batches recompute from the observed
timeline first — a timeline surprise is not by itself a law miss):
- p357a = m123 with base T1(2,ab) -> T1(11,ab): all 25 pairs tie
  at sal 0; the wave exposes B directly (merged form). PREDICT
  oracle R4-wave = (2,2),(5,5),(5,6),(5,7),(5,8),(5,2),(6,5),
  (6,6),(6,7),(6,8),(6,2),(7,5),(7,6),(7,7),(7,8),(7,2),(8,5),
  (8,6),(8,7),(8,8),(8,2),(2,5),(2,6),(2,7),(2,8).
- p357b = m123 + R3b (salience 5, T1(f1=="beta") T1(f1=="a"),
  empty RHS — a join1-sharing rule that fires between R1's
  firings; assumed timeline R1,R3b,R1,R3b x3,R5): batches become
  [S],[G1],[G2] in the m123 chassis. PREDICT oracle R4-wave =
  (5,2),(6,2),(7,2),(8,2),(2,2),(5,5),(5,6),(6,5),(6,6),(7,7),
  (7,8),(7,5),(7,6),(8,7),(8,8),(8,5),(8,6),(5,7),(6,7),(5,8),
  (6,8),(2,5),(2,6),(2,7),(2,8).
- p357c = p357b + all-11 base: one sal group, split B fully
  observable; discriminates O-block order AND S-separation.
  PREDICT oracle R4-wave = (2,2),(5,5),(5,6),(5,2),(6,5),(6,6),
  (6,2),(2,5),(2,6),(7,7),(7,8),(7,5),(7,6),(7,2),(8,7),(8,8),
  (8,5),(8,6),(8,2),(5,7),(6,7),(2,7),(5,8),(6,8),(2,8).
  (oldest-first O-blocks would instead give ...(7,2),(7,5),(7,6)
  row order and (2,7),(5,7),(6,7) rIns order — the discriminator.)

D-357 PROBE RESULTS (round 1): p357a HIT EXACT (oracle 3x stable;
the merged-B form confirmed pair-for-pair, incl. new-block-first
row walk and the rIns-born tail). p357b MISS + p357c MISS — BOTH
DIAGNOSTIC: observed waves are the MERGED form (p357b == m123's
original wave; p357c == p357a's predicted wave EXACTLY). R3b
(T1(f1=="beta") x T1(f1=="a")) has different alpha constraints ->
its own beta node, NOT join1: evaluating R3b never flushes join1's
segment. THE SHARPENED CONDITION: a batch boundary requires an
evaluation of a rule sharing THE JOIN NODE ITSELF (identical
prefix patterns -> same beta node), not merely the fact type. With
batches correctly computed under that condition ([S],[G12-merged]
for both probes), the law fits p357b and p357c EXACTLY as well —
4 of 4 probe waves are law-conformant; the misses are probe-design
misses, recorded as such.

ROUND 2 PREDICTIONS (registered before cells): p357d = m123 + R3c
(salience 5, the EXACT join1 prefix T1($x : f0) x
T1(!(f0 <= -1000000007)), empty RHS). R3c genuinely shares join1;
assumed timeline R3c x4 (S-pairs), R1, R3c x12, R1, R3c x20, R5,
wave, R0 x25 (89 firings). Batches [S],[G1],[G2]. PREDICT oracle
R4-wave = the p357b-registered sequence: (5,2),(6,2),(7,2),(8,2),
(2,2),(5,5),(5,6),(6,5),(6,6),(7,7),(7,8),(7,5),(7,6),(8,7),
(8,8),(8,5),(8,6),(5,7),(6,7),(5,8),(6,8),(2,5),(2,6),(2,7),
(2,8). p357e = p357d + all-11 base: PREDICT oracle R4-wave = the
p357c-registered sequence: (2,2),(5,5),(5,6),(5,2),(6,5),(6,6),
(6,2),(2,5),(2,6),(7,7),(7,8),(7,5),(7,6),(7,2),(8,7),(8,8),
(8,5),(8,6),(8,2),(5,7),(6,7),(2,7),(5,8),(6,8),(2,8) — the
O-block discriminator ((7,5),(7,6) before (7,2); (5,7),(6,7)
before (2,7) = recent-first).

D-357 PROBE RESULTS (round 2): p357d HIT EXACT + p357e HIT EXACT
(oracle 3x stable, 89 firings each, timelines exactly as assumed:
R3c x4 pre-R1, x12 between the R1 firings, x20 after, then R5).
THE LAW IS FULLY PINNED — every convention now discriminated by
data: (1) batch boundaries = evaluations of a rule sharing THE
join node (p357d flips m123's chassis to batch-major; p357b's
type-sharing R3b did NOT); (2) S always its own batch (p357e's
(2,2)-first + witness (1,3)<(3,21)); (3) drain ext-LIFO/RHS-FIFO,
merged batches concatenated in firing order (p357a); (4) row walk
N-first then O-blocks MOST-RECENT-FIRST, block-internal drain
order (p357e: (7,5),(7,6) before (7,2) and (5,7),(6,7) before
(2,7) — oldest-first REFUTED); (5) leftIns rows before
rightIns-born (witness G2 + p357e tail); (6) stable dyn-salience
sort on top. ENGINE observations: p357d engine wave == m123
engine wave and p357e == p357a (the engine's order is insensitive
to the intervening sharer — its parity is structural, per
D-356b). Law-conformant waves: 6/6 (m123, witness, p357a-e with
correct batch partitions). tools/model_check_unblock.py carries
the checks + ablations.

D-357 PORT CALIBRATION + THE SECOND SURFACE: the release reorder
(engine.rs d357_wave_reorder, phreak StagedList::reorder_by_key)
landed with ONE direction flip (ascending emission arrived
group-reversed at the terminal -> emit rev(B); the Ria/counting
chain applies one net whole-list reversal). R4 WAVES now MATCH
the oracle on m123 + p357a-e + xf_fz_141421_123 (handle-tagged
full-log comparison). RESIDUAL: m123/p357a-e still fork on the
R0 TAIL (salience -10, fires post-wave) — PRE-EXISTING (byte-
identical pre/post port), invisible in the truncated D-356-era
diff reads. Extracted: m123's oracle R0 sequence == B EXACTLY
(the merged-batch accumulation law at the FIRST-BUILT Term sink,
un-reversed); the engine's R0 sequence == SPLIT-B exactly (the
eager per-batch composition — law-conformant per batch, wrong
batch partition). The witness's R0 tail MATCHES both sides (R3's
firings split the oracle's batches there = engine's eager
partition). Same law, second surface: the lazy-merge term-queue
order.

MICRO-PROBE p357f (registered before run): m123 minus R5/R4
(keep R0 sal -10 + R1 only; two T0s, base T1s) — does the
merged-vs-split R0 fork need the delete/subnet context? PREDICT
PASS (the certified corpus is dense in insert-RHS + low-salience
all-pairs shapes; a bare fork here would have been fuzzer-caught
long ago — so the engine's plain path must already compose
merged-B, and m123's split-B R0 queue is a side effect of the
delete/subnet context). A MISS = a broad uncovered surface, its
own decision point.

D-357 PROBE p357f RESULT: PASS (prediction HIT; the raw diff was
the 0-based-handle artifact — no InitialFact without a CE rule).
NOTE the composition finding: p357f's oracle R0 order is PURE
INSERTION-LEX — a THIRD composition, different from the m123-
context B-form. The B-form at a lazy term needs the delete
context (rightDel-first mega-flush). The when-B-vs-lex law at
lazy TERM sinks is UNEXTRACTED — that is the term-queue round's
first question.

D-357 FINAL STATE (the port): PASS FULL engine-vs-oracle
(handle-tagged, artifact-aware): xf_fz_141421_123, p357d, p357e,
p357f, m1020 (untouched). RESIDUAL R0-TAIL forks (merged-batch
lazy-term surface, PRE-EXISTING): m123, p357a, p357b, p357c —
these stay in the lane as the term-queue round's witnesses.

D-358 ATTEMPT + REVERT (the m1020b/8087 channel element): the
"same normalization from the update side" hypothesis is REFUTED
at the mechanism level — the wave law does not cover it. Decode
facts for the channel round (all from SEINE_TRACE on m1020b):
the or-branches build ONE shared exists node (do_exist[1]); b1
and b2 are its TERM sinks (peer_merge_term per-entry-prepend),
and the join's other sink is R4-term (join sinks = 2, not 3 —
the >= 3-sink stamp never engaged); the node evaluates TWICE per
epoch flush (once per branch path) with DIFFERENT staged lists:
eval-1 (b1) has [A,A] as sl.INS ph=2 near the tail; eval-2 (b2)
has it sl.INS ph=0 AT the tail — [A,A] rides the leftIns arm in
BOTH branches (never the leftUpd arm — an exists-eval deferral
can never reposition it), and peer_merge_left's memory-clash
rule (`t` already in node.lefts -> memory reorder, no staging)
plus the shared s_left across branch evals are the machinery in
play. The needed observable: [A,A] at the HEAD of eval-2's
staged list (processed first -> consumed LAST at b2). The
deferral edit never engaged and was REVERTED clean (byte-
identical re-verified on m1020/m1020b/witness-123). fz_8087_1020
+ m1020b STAY BANKED for the channel round (D-355's original
plan: source read of updateChildLeftTuple/peer same-kind + a
b1-vs-b2 ladder), now with the two-eval staging map above.

GRADUATION DECISION (this slab): graduate xf_fz_141421_123 +
p357d + p357e (full-pass, law-certified); m123 + p357a/b/c/f
stay in probes_pending/oldbank (term-queue round witnesses);
m1020b + xf_fz_8087_1020 stay banked (channel round). Rebank
13 -> 12.

D-357 RECEIPTS (all green, 2026-07-19): byte gate 2569 cells vs
wt_pre357 (HEAD bfe6734) — movers EXACTLY the 7 expected
(xf_fz_141421_123, m123, p357a-e; p357f + m1020 + m1020b +
xf_fz_8087_1020 + the D-352 regression 14 + jr11/jr17 all
byte-identical); graduations pr_mu_fz_141421_123 +
pr_mu_p357d + pr_mu_p357e; make diff 11/1581/414 + drift bank
REBANKED 13 -> 12 identical; lint-probes 2447/0/0; cargo 74;
pytest 260 (maturin rebuild, tracked .so restored); demo True;
model_ird 31/31 + check_witnesses 26/26 + validate_cells 39/39;
IRD 150x5 seeds 7001/7002/6001/6003/9001 all 0-div; SD census
150x12 = 6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT; agenda_open x10
identical x3 binaries (release/debug/pre-edit); fresh fuzz
2x2000 seeds 352001/352002 + fuzz_cep 3x300 seeds 352901-903 =
(recorded below on completion). Engine deltas: phreak.rs
StagedList::reorder_by_key; engine.rs FactProv provenance
(on_insert funnel, firing_log at the firing record, ext_batch_no
at fire_all exit) + d357_wave_reorder (gated: non-temporal Not,
single Ria sink, pure ph=2 triple release, known provenance,
no late-external facts) + the rev(B) emission calibration.
FUZZ RESULTS: 2x2000 seeds 352001/352002 CLEAN (0 divergences, 0
xfail; 72s/67s) + fuzz_cep 3x300 seeds 352901/352902/352903 all 0
divergences. THE BATTERY IS FULLY GREEN.

### D-359 — the term-queue round (Bryan: "do the term-queue round")

THE QUESTION (from D-357): a lazy Term sink's queue composes as B
(m123's R0 = the accumulation law exactly) or as PURE
INSERTION-LEX (p357f's R0 — rows and walks both in insertion
order, ext included FIFO, no batch structure) — the flip
condition is unextracted. Candidate ingredients: (i) a DELETE in
the accumulated batch (m123 has R5's, p357f none); (ii) WHO
drives the join flush (m123: R4's wave-eval flushes join1 before
R0 pops; p357f: R0's own eval is first-and-only); (iii) an early
separate LIA drain (m123: R5's firing-2 eval drains the T1-LIA
before join1 flushes; p357f: one combined cascade at R0's eval);
(iv) the InitialFact / subnet presence (CE-first compile).

LADDER (all off p357f's chassis = m123's 4 facts + R0@-10 + R1;
predictions registered BEFORE any cell runs, per best hypothesis
= the flush-DRIVER/early-LIA-drain family, weakly held):
- p359a = p357f + R5 (the delete, NO subnet). If the R0 tail
  goes B-composed (ext-LIFO rows, batch structure, new-first
  walks) the delete or its early-LIA-drain side effect drives
  the flip. PREDICT: B-composition (forks vs current engine).
- p359d = p357f + R5nd: "T1(f0 == -4) then end" (a T1-LIA rule
  that FIRES but deletes nothing — drains the LIA early exactly
  like R5's pop does, no delete staged). Discriminates
  delete-in-batch vs early-LIA-drain. PREDICT (weakly): B-
  composition if the early drain is the trigger; pure-lex if the
  delete itself is.
- p359b = p357f + R4 (the subnet, NO delete; R4 parks forever,
  zero R4 firings). Discriminates the flush-driver/InitialFact
  ingredient without any delete. PREDICT (weakly): pure-lex
  (R4's item, having no activations, does not pop before R0).
- p359e = m123 with R5 deleting T1(f0 == 2) instead (kills the
  ab fact, NOT the blocker; R4 stays parked forever, no wave).
  A delete WITHOUT a mass-unblock. PREDICT: B-composition on
  the surviving pairs (the delete ingredient present).
A miss anywhere is data; the law is extracted from the full
quartet + the two existing points, then verified in the model
before any edit.

D-359 LADDER RESULTS (oracle 3x stable all four): p359a = PURE-LEX,
engine MATCHES (registered prediction B — MISS: the delete does NOT
flip the composition); p359d = PURE-LEX (kills the early-LIA-drain
sub-hypothesis; the conditional prediction's delete arm was already
dead via p359a); p359b = B-COMPOSITION, ENGINE FORKS (weak
prediction lex — MISS: the flip is the SUBNET'S PRESENCE with zero
R4 firings); p359e = B over survivors, engine forks (prediction
HIT). THE EXTRACTED LAW: the composition flips on JOIN1'S SINK
COUNT — single-sink (the term FUSED into the join's segment; p357f,
p359a, p359d) = pure insertion-lex, engine already correct;
multi-sink (segment split; the term accumulates per-flush COPIES;
m123, p357a-c, p359b, p359e, the witness's R0) = the accumulation
law. THE TERM-SURFACE VARIANT B': p359b's S-block = rows [1,2]
FIFO, walks FIFO — B' = B with ALL drains FIFO (ext included; the
first-sink term chain carries one fewer net reversal than the
subnet/wave chain, visible only in the parked-S block since RHS
blocks are FIFO on both surfaces). Hand-checked EXACT on m123-R0 +
p359e; the witness's R0 (split batches, engine matches oracle) is
the split-B' identity check for the model.

D-359 PORT + THE FENCE ARC (byte-gate-driven, 4 iterations —
each mover family named and each fence data-anchored):
tools/model_check_termq.py green (m123-R0/p359b/p359e EXACT + the
witness's split-R0 identity + 3 ablations); port = termq_sorted
one-shot at first selection + d359_termq_reorder (B' key = the
D-357 key with ext-FIFO ranks; ascending = queue order, no flip
needed). Byte gate iteration 1: 19 movers incl. 13 CERTIFIED (the
D-352 shared-prefix family + peers) -> fence 1 = ext_upd_seen (an
external modify's call-time segment flush breaks the lazy
premise; D-353). Iteration 2: or-twin and interleave fences
(fz_42_890/fz_7_5773 = or-twins; pr_or_a28/fz_123_3482 = rules
selected while other queues hold work). Iteration 3: the
remaining pair (fz_999_6009 R2, fz_123_3482 R0) pinned the REAL
class boundary: sibling chains that can FIRE during accumulation
(any Term reachable without crossing a Ria — plain-join and
accumulate chains included) are the certified peer/flush
compositions; a Ria-GUARDED subnet chain parks without firing, so
the lazy premise provably holds. Overshoot recorded: the first
Ria-guard DFS also fenced the targets (join1 feeds the OUTER
counting node its lefts DIRECTLY — Term(R4) reachable through
Node(SubnetExists)); fix = subnet-kind nodes are exists-gated =
guarded stops. FINAL: byte gate 2573 cells, movers EXACTLY the
six intended (m123, p357a/b/c, p359b, p359e); all 12 law cells
PASS engine-vs-oracle (incl. the three pr_mu_* graduates and the
single-sink controls).
D-359 RECEIPTS (all green, 2026-07-19): byte gate 2573 vs
wt_pre359 (0ea912c) movers EXACTLY the six intended; NINE
graduations pr_tq_{m123,p357a,p357b,p357c,p357f,p359a,p359b,
p359d,p359e}; make diff 11/1590/414 + drift bank 12 identical;
lint 2451/0/0; cargo 74; pytest 260; demo True; model_ird 31/31
+ 26/26 + 39/39; IRD 0-div x5; SD census 71 EXACT; agenda_open
x10 identical x3; fuzz 2x2000 seeds 353001/353002 CLEAN +
fuzz_cep 3x300 353901-903 CLEAN. THE OLDBANK LANE now holds only
records + the channel-round anchors (m1020/m1020b).

### D-360 — the channel round (Bryan: "do the channel round")

D-358 CORRECTION: the trace tag do_exist[1] is the PATTERN
POSITION, not a trie index — m1020b builds TWO exists nodes
(different alpha constraints per branch); the join's sinks =
[Node(e_b1), Node(e_b2), Term(R4)] = THREE (m1020: two). The
"one shared exists node" reading in the D-358 note is WRONG.

THE MECHANISM (trace-exact, current engine): both exists nodes
hold a STALE setup-era staged insert for (A,A) — R2 had no
activations at setup (no T1s -> exists false), so the staging
was never consumed. At the epoch, the join's leftUpd for (A,A)
finds the first-sink pending insert -> kept-kind (child_upd
D-071: re-staged as INS ph2 + peer_upd mark). e_b1 (first sink,
append_into_pending) MERGES the kept-kind entry over the stale
one -> creation position -> b1 = [AA,AB,BA,BB] = oracle. e_b2
(peer_merge_left): the kept-kind arm's staged-clash SKIP leaves
the STALE entry at its old tail position (ph0) -> eval-2 staged
= [AB,BA,BB,AA] -> b2 consume = [AA,BB,BA,AB] = the fork. A
plain prepend walk (= repositioning the stale insert into the
current batch, exactly updateChildLeftTuple's "a child staged as
INSERT moves into the current batch KEEPING its insert kind")
gives staged [AA,AB,BA,BB] -> b2 consume [BB,BA,AB,AA] = THE
ORACLE. m1020 (2-sink) passes BECAUSE OF the skip (D-355's
recorded parity swap between the forms) -> the fix needs the
third-sink discriminator.

LADDER (predictions registered BEFORE cells run; hypothesis =
reposition-on-kept-ins-clash gated on a Term sink AFTER the
exists peers):
- p360a = m1020b + R5 (another T0 f3,f3 x T0 f2 plain term,
  declared after R4; 4th sink). PREDICT the fork persists
  identically pre-port (engine b2 [AA] first, oracle [AA] last);
  post-port both PASS (gate robust to more sinks).
- p360c = m1020b with R4 replaced by a THIRD or-branch of R2
  (exists T1(f0 != 1, ...) — three exists sinks, NO Term).
  THE DISCRIMINATOR: Term-later gate predicts NO reposition
  (oracle b2 = [AA]-first class, engine PASSES pre-port);
  sink-COUNT gate predicts reposition (oracle b2 = [AA]-last,
  engine forks). Registered prediction (weak, per the Term-later
  hypothesis): oracle keeps [AA] FIRST at b2.
- p360d = m1020b + setup T1(3) (exists TRUE at setup; R2 fires
  its setup activations; no stale staged entry at the epoch).
  PREDICT: the epoch's (A,A) rides the UPD channel at both
  sinks (children exist -> child_upd, no kept-kind), and the
  cell PASSES pre-port both sides (upd-of-existing-child order
  is the certified surface).
- p360b = m1020b with R4 declared FIRST (join sinks =
  [Term(R4), e_b1, e_b2] — both exists = peers). The first-sink
  assignment probe; NO registered prediction beyond "oracle 3x
  stable" (unknown surface; the result seeds the law's
  first-sink clause).

D-360 LADDER RESULTS (oracle 3x stable all four): p360a HIT (the
fork persists identically with a 4th sink); p360c HIT — THE
DISCRIMINATOR: three exists sinks with NO Term = engine MATCHES
(b1 = [BB,BA,AB,AA], peers = [AA,AB,BA,BB]) — Term-later
CONFIRMED, sink-count REFUTED; p360d HIT (setup T1 consumes the
staging; the epoch upd rides the certified upd channel; no
surface); p360b recorded (R4-term declared FIRST: both exists =
peers, both [AA,BB,BA,AB], engine matches — a Term BEFORE the
exists does not flip). THE LAW: at a PEER exists sink with a
Term sink at a LATER index of the same join, a kept-kind insert
(peer_upd-marked) clashing with a STALE staged insert
REPOSITIONS it into the current batch keeping its kind
(updateChildLeftTuple verbatim) — head position; without the
later Term (m1020, p360c) or without the stale entry (p360d) or
with the Term first (p360b), current behavior is
oracle-certified. PORT: node stamp kept_ins_reposition at
lists_built (Sink::Node(c) at si with any Term sink at index >
si) + the reposition arm in peer_merge_left's kept-kind branch
(ins-clash only; upd/del-clash and no-clash keep the
fz_999_3298 skip).
D-360 RECEIPTS (all green, 2026-07-19): port = kept_ins_reposition
stamp (Sink::Node peer with any Term sink at a later index) + the
reposition arm in peer_merge_left's kept-kind branch (ins-clash
only); ALL SEVEN law cells PASS first-shot (m1020, m1020b,
xf_fz_8087_1020, p360a-d). Byte gate 2577 vs wt_pre360 (22423b4)
movers EXACTLY the three expected (witness + m1020b + p360a;
m1020/p360b/c/d byte-identical). SEVEN graduations pr_pc_{fz_8087_
1020,m1020,m1020b,p360a,p360b,p360c,p360d}; rebank 12 -> 11; make
diff 11/1597/414 + drift 11 identical; lint 2456/0/0; cargo 74;
pytest 260; demo True; model_ird 31/31 + 26/26 + 39/39; IRD 0-div
x5; SD 71 EXACT; agenda_open x10 x3; fuzz 2x2000 seeds
354001/354002 CLEAN + cep 3x300 354901-903 CLEAN. THE ORDER TRIO
3/3 CLOSED; the oldbank lane = records only.

### D-361 — the query round (Bryan: "do the query round")

RE-MEASURE (post-D-360 engine): both witnesses still fork.
xf_fz_296001_1704 = MEMBERSHIP fork: engine 60 vs oracle 46
firings; the 14 extras = EXACTLY the full 14-row ?TCr
enumeration x the DELETED T1(11) (R3@-6 deletes it before
QR0@-8 pops; pull-at-activation is the certified D-107
semantics, so the activations legitimately existed — the DELETE
CANCELLATION misses qce-rule queue entries). Suspect site:
engine.rs ~10010 deactivation prune (positives filter excludes
qce patterns; tuple layout [T1, row-synthetic] vs the D-056
rendering order needs verification). xf_fz_296002_1494 = ORDER
fork in the top-level Q0 enumeration: the f1==true block rows =
engine [firing1-FIFO, firing2-FIFO, setup] vs oracle [firing2,
firing1, setup] — ORACLE = blocks MOST-RECENT-FIRST,
within-block FIFO, setup (oldest) last = THE SAME memory-block
convention D-357 pinned for join right-memory walks; the false
block (all setup) is identical both sides, confirming the
within-block law.

MINIMIZATION PREDICTIONS (registered before runs):
- m1704 = R0 (T0 -> insert T1(11)) + R3 (the delete pair rule)
  + QR0@-8 (T1 x ?TCr) + TCr + T0(f1=11) + T1(6) + RelR
  {7->13, 7->16, 16->22} + MarkR {16} (closure = (7,13),(7,16),
  (16,22),(7,22) = 4 rows). PREDICT: engine 4 extra QR0 firings
  with the deleted T1(11) (8 vs 4), same class as the witness.
- m1494 = R0 (two-firing T1 inserts) + Q0 (branch 1 only) + the
  two T0s + one setup T1(ab,true); no R3/R4/TCr. PREDICT: the
  Q0 true-block row order forks exactly as the witness (engine
  [firing1, firing2, setup] vs oracle [firing2, firing1,
  setup]).

D-361 MINIMIZATION ROUND 1: BOTH MISSES (m1704 PASS, m1494 PASS
after adding the queries input spec my first cut dropped).
m1494's converged order (both sides) = [setup][firing2][firing1]
— setup FIRST then blocks recent-first: a THIRD composition;
the witness's oracle puts setup LAST. Iteration predictions
(registered): m1494b = +the 3 extra setup T1s (the false block)
-> weakly predict still-PASS (more setup facts alone); m1494c =
+R3 and its delete of T1("",false) -> PREDICT the fork returns
(a T1 delete restructures the memory blocks); m1704b = +QRtwin
(the second qce rule, salience 0, fires pre-delete) -> PREDICT
the fork returns (the twin's earlier evaluation shares qce
state; without it QR0's lazy pop composes correctly).
D-361 ITERATION 2: m1704b FORKS (PREDICTION HIT — QRtwin, the
second qce rule firing pre-delete, is the ingredient; engine 18
vs oracle 14 = the 4-row closure x deleted T1(11) = the witness
class exactly). THE m1704b ANCHOR STANDS (2 qce rules + R0 + R3
+ TCr, 6 facts). m1494b/c still PASS (misses recorded — extra
setup T1s and the R3 delete are NOT the ingredients). Next
prediction (registered): m1494d = full 3-arg Q0 with BOTH
branches + the PRECEDING queries[0] call (Q0(false,null,false)
before the forked Q0(null,null,false)) -> PREDICT the fork
returns (query evaluation mutates drain-window state consumed by
the next call, or the or-branch enumeration is the ingredient).
D-361 ITERATION 3 (1494): m1494d PASS (miss — or-branch + the
preceding spec call are not the ingredients). DELTA-DOWN found a
rule the truncated decode read MISSED: QR0 salience 7 = T0 x
?Q0(false, $x0, false) x ?TCr x T1(f0 == $x0) — a MID-RUN ?Q0
pull. Registered prediction: m1494e = m1494d + QR0 + TCr + one
RelR (v0->v3) -> PREDICT the fork returns (the salience-7 ?Q0
pull consumes/advances the query drain-window state that the
end-of-run enumeration then continues from — the engine's
continuation order diverges).

D-361 THE 1704 MECHANISM (trace-exact on m1704b): QRtwin's
salience-0 firing materializes BOTH qce rules' expansions?? NO —
per-net staging; the twin's role = R3's delete then lands as
same-eval del+ins at QR0's LATER consume: the qce expansion
stages child INSERTS directly at the terminal, so QR0's single
post-delete eval consumes ins=[..(F6,row)..] AND
del=[(F6,row)..] IN ONE BATCH — and the consume processes dels
FIRST (queue-retain finds nothing; the acts are still in
src.ins) then queues ALL ins: the deleted-parent children fire.
Drools unstages the pending insert by tuple OBJECT identity
(deleteChildLeftTuple); row fact-ids are mint-fresh per
materialization, so VALUE identity == object identity for this
class. THE FIX: at the terminal consume of a qce rule, a del
whose tuple matches a still-staged ins CANCELS the pair
(both removed; no queue-retain/TMS hooks for the never-queued
act). Without the twin (m1704), QR0's acts materialize at its
own post-delete eval where the staged del+ins annihilate
upstream — PASS pre-port, the control cell.

1494 DEFERRED TO ITS OWN ROUND (D-362): the enumeration-order
law spans at least THREE observed compositions ([setup][f2][f1]
in the no-pull minimal; blocks-recent-first setup-LAST in the
pull context; the engine's window continuation) — an
exploratory oracle matrix + Machine-pipeline decode is needed
before any port; deferring per commit-per-green-slab.

D-361 JUSTIFIERS-WALL CHECK (ledger item "?query justifiers,
unprobed wall" vs the D-107 lift comments): probe jq1 = a rule
with a ?query CE whose RHS insertLogical's a fact, plus a
support-retraction path (the justifying premise deleted ->
Drools retracts the belief). Registered prediction (per the
D-107 qm5 lift note "TMS retraction composes with the pull"):
the cell RUNS on both sides (no wall) and PASSES — the ledger
item is either STALE or means a finer corner; a fork or a
one-sided error names the real wall.
D-361 JUSTIFIERS RESULT: prediction MISS (recorded) — the wall is
REAL and PRECISE: the engine compile-walls "insertLogical from
?query rules" (D-076/D-312 wording: revalidation over query pulls
is unprobed) while the ORACLE RUNS the cell — the D-107 lift
covered the OTHER direction (logically-inserted facts as pull
targets, qm5). The surface is oracle-observable, so a lift is
POSSIBLE but needs its own envelope round (belief revalidation
when pull premises change). jq1 stays in the lane as the seed
cell. The ledger item is CONFIRMED, sharpened from "?query
justifiers" to "insertLogical FROM qce rules (revalidation
semantics)".
D-361 RECEIPTS (all green, 2026-07-19): port = the qce-gated
del-cancels-staged-ins fold at the terminal consume; m1704 +
m1704b + xf_fz_296001_1704 PASS first-shot, BONUS: the open qmut
witness fz_9103_4499 flips FAIL->PASS (the same class — its
quarantine round predates the fold). Byte gate 2590 vs wt_pre361
(c69503c): movers = the witness + m1704b + the qmut cell (all
intended/oracle-ward). FOUR graduations pr_qc_{fz_296001_1704,
m1704,m1704b,fz_9103_4499}; rebank 11 -> 10 -> 11 (one NEW
pre-existing quarantine: cf355901x129, a CEP temporal-join x
window x not x acc ORDER latent from the fresh seed, bisected
PRE-EXISTING vs c69503c; the cep fuzzer regenerates per-seed so
the seed row stays a KNOWN-find record, not re-run-clean). make
diff 11/1601/414 + drift 11 identical; lint 2460/0/0; cargo 74;
pytest 260; demo True; model_ird 31/31 + 26/26 + 39/39; IRD
0-div x5; SD 71 EXACT; agenda_open x10 x3; fuzz 2x2000
355001/355002 CLEAN; cep 355902/355903 CLEAN + 355901 = the one
banked find. Lane: m1494f (the D-362 anchor) + jq1 (the
justifiers seed) stay.

### D-362 — the query-enumeration-order round (the 1494 half)

EXPLORATORY MATRIX (the law is unknown — recorded as
MEASUREMENTS, not predictions; three compositions already
observed: m1494 no-pull two-spec-calls = [setup][f2][f1];
m1494f early-pull one-call = [f2][f1][setup]; the engine's
window continuation = [setup][f1][f2]-ish). Dimensions: pull
timing (none / early@7 / after-inserts@-20), spec-call count
(1 / 2), setup-T1 presence. Cells e1..e6 below; oracle 3x each;
the fit follows the data.

D-362 MATRIX RESULTS (oracle 3x stable throughout): e1-e10 ALL
MATCH engine==oracle — the compositions: no-pull/late-pull =
[setup][f1][f2] (insertion blocks FIFO); early-pull =
[setup][f1][f2]-window-continuation (e3 — THE ORACLE USES THE
WINDOW CONTINUATION TOO in the simple shape, certifying D-086's
model beyond its prior envelope); the FLIP to [f2][f1][setup]
(the witness/m1494f fork shape) needs the CONJUNCTION or-branch
x false-setup-T1s (f5 = the minimal anchor: no TCr, pull-arg
irrelevant — f1/f6 fork identically; singles e7/e8/e9/e10 all
no-flip). The flip = a whole block-list REVERSAL of the window
structure. Mechanism hypothesis (unverified): the shared query
network's node LINK TIMING (a late-linking branch node refills
from the OTN in a different order) — Drools dquery internals
(QueryElementNode/link lifecycle), needs a source round.
DISCRIMINATOR e11 (registered prediction per the
populated-at-arm hypothesis): f5 with the false T1s inserted by
a rule AFTER the pull (salience 5 < QR0's 7) -> PREDICT NO flip
(branch-2 empty at arm time).
D-362 e11 RESULT: FORKS — prediction MISS (recorded): the
false-T1s inserted AFTER the pull still flip the oracle. The
populated-at-arm hypothesis is REFUTED alongside the naive
link-timing form: the conjunction (or-branch x false-T1s
EXISTING, whenever born) flips the composition,
timing-independent. ROUND DECISION (D-352 discipline): no port
on a 3-positive/7-control phenomenological base with the
mechanism unexplained — xf_fz_296002_1494 STAYS BANKED; the
continuation needs a Drools source round (DroolsQuery /
QueryElementNode / dquery node lifecycle — WHY does a sibling
branch's alpha population reverse the shared T1 site's
enumeration blocks). THE ROUND'S ENVELOPE WIN: e1-e10 all PASS
engine==oracle and pin the enumeration laws (insertion-blocks
for fresh calls, window-continuation for early-pull — the
oracle CERTIFIES D-086's window model in the simple shapes) —
GRADUATED as pr_qe_e1..e10. The fork quartet (f1, f5, f6, e11)
+ m1494f stay in the lane as the continuation's witnesses; f5 =
the minimal anchor.

### D-363 — the 1494 source round (Bryan: "do the 1494 source
round")

SOURCE FINDINGS (drools-core 9.44 + kiesession, extracted):
getQueryResults = PropagationEntry.ExecuteQuery: agenda flush
x2, insert a DroolsQuery handle at the query's LIA, then FOR
EACH PathMemory (one per or-branch, list order) evaluate-and-
fire — rows = arrival order at the query terminals, branch
blocks in path order (matches the observed branch-1-then-
branch-2 output). TupleList.add APPENDS (iteration = arrival);
TupleIndexHashTable full-iterator = slot-major/chain/key-list —
REFUTED as this fork's mechanism by ADJACENCY (equal-key facts
would enumerate adjacently; observed rows interleave them in
fact order — both sides do a per-fact window walk). The fork =
the WINDOW-WALK DIRECTION: e3 = [pull-window][top-window] =
oldest-first FIFO-within; f5/witness = newest-first FIFO-within.

DISCRIMINATOR p363a (prediction registered per the full-window-
reversal reading of f5's [f2][f1][setup]): f5's chassis with
FIVE separate RHS T1-insert firings (five windows w1..w5, one
distinct f0 value each: "q","r","s","t","u"). PREDICT the
oracle's true-block = [w5][w4][w3][w2][w1][setup] — full window
reversal, FIFO within each; a partial pattern (rotation,
last-window promotion) = a different law, recorded either way.
D-363 p363a RESULT: FORKS with a REFINEMENT — oracle =
[q,r,s,t,u,ab] = post-pull windows FIFO, FIFO within, THE
PULL-WINDOW BLOCK MOVED TO THE TAIL (my full-reversal prediction
MISSED — recorded); engine = [u,t,s,r,q,ab] (windows LIFO +
setup-tail — its own composition). Re-reading f5 under this law:
R0 fires zz-first there (activation order), so f5's oracle
[zz,beta]["",beta][ab] = the SAME law (post-windows FIFO,
pull-window last). THE PHENOMENOLOGICAL LAW: at a top-level call
on a MULTI-BRANCH query whose SIBLING branch produces rows at
the call (equivalently: its patterns are alpha-populated — e8's
empty branch-2 = no flip; e11's late-born rows = flip, so
EXISTENCE at call time, not arm time), branch-1 enumerates
[post-pull windows FIFO][the first (pull-time) window LAST];
without the conjunction, [first window][post windows]
(certified). FRESH PREDICTIONS (registered): p363b = p363a
minus the false setup T1s (branch-2 empty) -> PREDICT NO flip,
oracle [ab,q,r,s,t,u]; p363c = p363a with QR0 at salience -20
(the pull AFTER all R0 firings; the first drain window =
everything) -> PREDICT no observable flip (single window).
D-363 FRESH DISCRIMINATORS: p363c HIT (late pull = single window
= no flip, insertion FIFO both sides). p363b = NO FORK (the
no-conjunction half held) but the SHAPE prediction MISSED
(recorded): with interleaved per-firing windows the
no-conjunction composition = [posts-LIFO][pull-tail] — matching
the ENGINE's own composition; e3's [pull][posts-FIFO] is another
engine-matching shape. THE SHARP LAW: the engine's (varying)
composition tracks the oracle in EVERY no-conjunction shape; the
FORK = the POST-WINDOW WALK DIRECTION under the conjunction
only — oracle walks post-pull windows FIFO (FIFO within), the
engine LIFO; the pull-window sits at the tail on BOTH sides
(p363a: [q,r,s,t,u,ab] vs [u,t,s,r,q,ab]; f5 re-read under
zz-first firing order = the same shape). Branch-2's rows match
both sides because its bound-arg walk is INDEX-driven (not the
full walk) — the port must touch only the top-level FULL-WALK
enumeration. PORT DESIGN: QueryMem tracks per-site window
boundaries; at the pub run_query path only (the qce pull/run_site
path untouched), gate = >=2 branches AND >=2 windows AND some
OTHER branch alpha-populated -> reorder the site walk so the
emission = [post windows FIFO, FIFO within][pull window];
direction calibrated on p363a (one flip constant if needed).
D-363 PORT ROUNDS: the coarse window-reorder fixed e11 but left
the rest forking; p363a's residual exposed THE BUCKET STRUCTURE:
the top-call walk is index-BUCKETED by the first unification key
(false bucket before true — p363a puts the pull-consumed ab-T at
the tail of the TRUE bucket while the witness's false trio heads
the FALSE bucket; the pull-consumed hypothesis died against the
witness in the same step). FINAL LAW: under the gate (>=2
branches, >=2 windows, another branch alpha-populated,
single-fact-pattern branch, bool first key), the enumeration =
buckets by key value (false<true), within-bucket [post-pull
windows FIFO, FIFO within][pull-window members last]; every
no-conjunction shape keeps the engine's certified composition.
PORT: QueryMem window records (drain_pattern) + d363_reorder at
the pub run_query path only (run_site untouched); non-bool keys
and unknown facts FENCE. ALL NINE law cells PASS including
xf_fz_296002_1494.
D-363 RECEIPTS (all green, 2026-07-19): three data-pinned fences
(call-boundness — fz_9101_7133's bound-arg call; single-pull-site
— its queries[1] under multi-pull windows; bool-key), each
anchored by a byte-gate counterexample. Byte gate 2599 vs
wt_pre363 (2bf1fe0): final movers EXACTLY the seven law cells
(fz_9101_7133 unmoved after the fences). TEN graduations
pr_qe_{fz_296002_1494,f1,f5,f6,e11,m1494f,p363a,p363b,p363c};
rebank 11 -> 10 -> 11 (NEW pre-existing quarantine
fz_356002_1512: a collect-order-family value fork from the fresh
seed, bisected vs 2bf1fe0). make diff 11/1620/414 + drift
identical; lint 2479/0/0; cargo 74; pytest 260; demo True;
model_ird 31/31 + 26/26 + 39/39; IRD 0-div x5; SD 71 EXACT;
agenda_open x10 x3; fuzz 356001 CLEAN + 356002 = the banked
find; cep 356901-903 CLEAN. THE QUERY FAMILY IS CLOSED
(D-361/362/363); the oldbank lane = records + jq1 (fenced).

### D-364 — the insertLogical-from-?query lift (Bryan: "do the
lift")

THE GAP (source-grounded): the engine's driving-fact death
already routes qce-tuple teardown through the terminal-del path
(the D-361 fold certified that consume); PULLED-premise death
produces no terminal del — the network does not know the row
depends on the fact. Drools' ?query CE = an OPEN dquery
(reactive rows). PORT SHAPE: record per-row supporting fids at
materialization; on_delete(f) synthesizes the terminal del for
affected qce tuples through the SAME consume path.

LADDER (oracle-only until the wall drops; predictions per the
open-reactive hypothesis, registered before any run):
- jq2 premise DELETE post-establishment -> PREDICT the belief
  retracts (final facts exclude T2).
- jq3 premise UPDATE flipping out of the pull constraint ->
  PREDICT retracts.
- jq4 premise UPDATE on an irrelevant field -> PREDICT survives.
- jq5 TWO premises yielding rows with the SAME bound value (two
  justifiers, one belief value); delete one -> PREDICT survives
  (support counting).
- jq6 RECURSIVE callee (TCr); delete a mid-chain link ->
  PREDICT pairs derived through the link retract (weakly held —
  recursive reactivity may differ; a miss walls recursion).
- jq7 DRIVING-fact delete (control) -> retracts (standard TMS).
- jq8 premise dies then RE-INSERTED next epoch -> PREDICT the
  belief re-derives (a fresh firing; count recorded).
- jq9 two RULES pulling the same query, same belief value; one
  driving fact dies -> PREDICT survives via the other justifier.
D-364 LADDER RESULTS (oracle 3x stable all eight): THE
OPEN-REACTIVE HYPOTHESIS IS REFUTED — jq2 (premise delete)
SURVIVES, jq3 (flip-out update) SURVIVES, jq6 (recursive chain
link delete) BOTH beliefs survive, jq8 (premise re-insert) NO
re-fire (misses recorded, 4 of them); jq4 (irrelevant update)
survives HIT, jq7 (driving-fact delete) RETRACTS HIT, jq9
(multi-justifier) survives HIT, jq5 two-rows recorded. THE LAW:
Drools' ?query CE pull is a PURE SNAPSHOT for TMS exactly as for
activations (qm4 extends fully): pulled-premise changes NEVER
touch the belief lifecycle; only DRIVING (network) facts
participate in justification. THE ENGINE'S EXISTING BEHAVIOR IS
ALREADY CORRECT ON BOTH SIDES OF THAT LINE (driving-fact death
routes the terminal del through the D-361-certified consume;
pulled facts route nothing). THE PORT = the wall removal alone —
no row-support recording, no on_delete hook. The recursion scope
lifts too (jq6 conforms).
D-364 RECEIPTS (all green, 2026-07-19): the lift = the wall
check removed + the d312_acc_justifier_walls pin flipped to a
positive (the D-316 pattern) — ZERO new engine code; all NINE
ladder cells PASS engine-vs-oracle first-shot (incl. jq1
unfenced and jq6's recursion — the recursive scope lifts too).
Byte gate 2608 vs wt_pre364 (96a53cb): movers = exactly the nine
jq cells (error -> runs; zero certified movement). NINE
graduations pr_jq1..pr_jq9 (corpus 11/1629/414); make diff
11/1629/414 + drift 11 identical; lint 2487/0/0; cargo 74;
pytest 260; demo True; model_ird 31/31 + 26/26 + 39/39; IRD
0-div x5; SD 71 EXACT; agenda_open x10 x3; fuzz 2x2000 seeds
357001/357002 CLEAN + cep 3x300 357901-903 CLEAN. THE QUERY
LEDGER IS EMPTY (walls: none; witnesses: none). The oldbank lane
= records only.
