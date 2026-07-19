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
