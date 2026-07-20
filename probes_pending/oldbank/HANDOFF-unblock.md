# HANDOFF — the mass-unblock normalization port round
# (cold start; written 2026-07-19 at 9f3dc14, Bryan-directed.
# Read this + PINS.md's D-355/D-356/D-356b sections first.)

## THE TARGET

Two banked witnesses, ONE unifying law, anchors already
minimized and committed in this lane:

- scenarios/xfail/xf_fz_141421_123 — anchor
  probes_pending/oldbank/m123.json (4 rules, 4 base facts,
  forks at firing[3], oracle 3x stable).
- scenarios/xfail/xf_fz_8087_1020 — anchor
  probes_pending/oldbank/m1020b.json (3 rules, 1 base fact,
  forks at firing[5], oracle 3x stable). NOTE m1020.json
  (2-sink form) PASSES — the third sink is an ingredient.

## THE LAW AS PINNED (D-356b) — and what is NOT yet pinned

PINNED (invariant, both anchors + the witness): the not-node
MASS-UNBLOCK emission (a right-delete releasing many blocked
lefts at once) reaches EVERY sink in FACT-INSERTION order
(oldest generation first, insertion order within a generation),
invariant to sink position/count. The ENGINE's order flips with
structure: m123 (R4's exists = 2nd sink of the shared join) =
generations LIFO, within-gen [beta,a]; the 141421 witness (R4 =
4th sink, after R3's or-twin terms) = generations FIFO,
within-gen [a,beta]. The oracle gives [beta-g1, a-g1, beta-g2,
a-g2] in BOTH.

NOT YET PINNED — extract BEFORE any edit: the COMPLETE wave law
inside equal-dyn-salience groups. The witness's oracle
firings 159-165 = (21,21),(21,22),(22,21),(22,22),(23,23),
(23,24),(23,21)... — (23,23) precedes (23,21), so the order is
NOT pure fact-id-lexicographic. Hand-extract BOTH anchors' and
the witness's FULL waves (recipes below), build the group
table, and find the exact within-group law before designing
the port. Chase whether it is per-generation blocks x
something, or salience-group boundaries I mis-drew (recompute
each tuple's dyn salience $a-$b explicitly).

## DECODE FACTS A FRESH CONTEXT NEEDS

m123 handle map (oracle #, engine F aligns): T1(-4,"beta")=1,
T1(2,"ab")=2, T0s=3,4; R1-gen-1: beta=5, a=6; gen-2: beta=7,
a=8. Sequence: R1 fires twice (two T0s), R5 deletes T1(-4) →
the subnet exists(T1() and not(T1(f0 < -3))) flips for ALL
parked R4 tuples at once → the R4 wave = the mass-unblock,
stable-sorted by R4's dynamic salience ($a-$b).

m123 oracle wave head (firings 3-12): (5,2),(6,2),(7,2),(8,2),
(2,2),(5,5),(5,6),(5,7),(5,8),(6,5)...
m123 ENGINE emission (SEINE_TRACE term[2] consume ins, in
consume order): (7,7),(7,8),(7,5),(7,6),(7,2),(8,7),(8,8),
(8,5),(8,6),(8,2),(5,7),(6,7),(2,7),(5,8),(6,8),(2,8),(5,5),
(5,6),(5,2),(6,5),(6,6),(6,2),(2,5),(2,6),(2,2).

Witness [x,2ab] salience-9 group: engine [a22,beta21,a24,
beta23] vs oracle [beta21,a22,beta23,a24].

m1020b's fork element: the [A,A] refire (an exists child BORN
at the leftUpd phase — exists was false at base, no prior
child) fires FIRST engine-side vs LAST oracle-side at the PEER
sink (b2) only; the direct sink (b1) has it FIRST both sides.
Working hypothesis: the same normalization seen from the
UPDATE side (the child rides the ins channel at peers). Verify
against the extracted law before assuming.

ENGINE STRUCTURE (SEINE_TRACE on m123): the subnet lowers to a
Not node holding 3-tuples [l1, l2, innerT1] blocked by T1(-4);
the unblock emission routes through the RIA hop -> the outer
exists counting machine (eval_subnet_node, D-351) -> term. The
parity chain [blocked-walk -> not-trg staging -> RIA add_ins ->
sn_right staged walk -> child_ins -> term staging -> consume]
is where the engine's structure-dependence lives.

DROOLS SOURCE (already extracted to the session tmp — RE-UNZIP,
job tmps do not survive): PhreakNotNode.doRightDeletes walks
rightTuple.getBlocked() from the HEAD = most-recently-blocked-
first (SAME direction as the engine); children stage via
trg.addInsert PREPEND; terminal drains head-first. Drools'
normalization is emergent from its hop conventions, NOT a sort.
Source jar: ~/.m2/repository/org/drools/drools-core/9.44.0.Final/
drools-core-9.44.0.Final-sources.jar (unzip PhreakNotNode.java,
PhreakExistsNode.java, RuleNetworkEvaluator.java as needed).

## LOG RECIPES (regenerate — session tmps are gone)

Handle-tagged firing log, either side:
  SEINE_HANDLES=1 ./target/release/seine-harness run <cell> |
    python3 (print rule + per-match f0,f1 + fields['__h'])
  SEINE_HANDLES=1 java -Xss1g -cp "oracle/target/classes:$(cat
    oracle/target/classpath.txt)" dev.seine.oracle.OracleRunner
    <cell>  # oracle __h = "0:<id>:..." -> split(':')[1]; QUERY/
    synthetic facts have no __h — guard the parser.
Engine node trace: SEINE_TRACE=1 ... run <cell> 2>&1 >/dev/null
  | grep -E "do_exist|do_node|term\[" (term[N] consume lines
  show the exact staged drain order).

## THE PORT — doctrine and constraints

MODEL-FIRST IS MANDATORY here (D-331/D-345/D-352 protocol; the
D-352 naive port broke 14 certified cells and was reverted).
The target surface is D-031 (blocked-list PREPEND pinned; the
D-127 exists-flush most-recently-blocked-first admit; the
sd_/tms parked lanes ride blocked lists). Candidate design: a
NORMALIZATION at the mass-unblock emission or at the terminal
consume for the mass-unblock class (the D-343 sort-at-consume
precedent: fid-desc-lex for not-MID release evals — study that
site, engine.rs ~10226, before choosing where to sort). The
class gate must be narrow: a right-delete unblocking >1 left
(mass), non-temporal; single-unblock cells must stay
byte-identical.

REGRESSION SET (must stay green; the D-352 counterexample
list): pr_or_a28, pr_or_a29, pr_ib15, pr_ib15b, pr_ib28,
fz_123_3482, fz_123_8822, fz_42_4816, fz_42_580, fz_42_952,
fz_999_6009, fz_min_580, fz_min_8822, fz_9005_450 + jr11/jr17
sanity. Then the FULL byte gate (expect movers = the two
witnesses + the two anchors only), then the full battery.

Round order: (1) extract the complete wave law (both anchors +
witness, group tables with explicit dyn-salience recomputed);
(2) model the engine's parity chain + the candidate
normalization over those timelines (a verifier like
tools/model_check_join3.py — candidate fits oracle, engine
model fits engine, cross-fit fails); (3) port ONCE, gates,
graduate m123+m1020b+both witnesses, rebank 13->11.

## VERIFICATION CARD (all green at handoff, 9f3dc14 PUSHED)

make diff 11/1578/414 + drift bank 13 identical; lint-probes
2437/0/0; cargo 74; pytest 260 (cd bindings &&
../.venv/bin/maturin develop --release, run pytest from REPO
ROOT with absolute paths, then git checkout
bindings/python/seine_rs/_native.abi3.so BEFORE commits); demo
True; model_ird 31/31 + check_witnesses 26/26 + validate_cells
39/39 (cd probes_pending/tms_envelope); IRD
tools/fuzz_tms_ird.py 150 <seed> x {7001,7002,6001,6003,9001}
all 0-div FROM REPO ROOT; SD census tools/fuzz_tms_sd.py 150 x
{7001,7002,6001,6003,7004..7011} = 6,10,3,4,6,5,5,6,8,7,4,7 =
71 EXACT (debug build first, never rebuild mid-census);
agenda_open x10 identical x3 (release/debug/pre-edit worktree);
fresh fuzz NEXT seeds 352001/352002 + fuzz_cep 3x300 NEXT
352901-903 (finds: bisect vs pre-edit worktree; pre-existing ->
mv scenarios/xfail/ + tools/xfail_drift.py --rebank + re-run).
Byte gate: git worktree add wt_preNNN <sha>, release build
both, compare `run` outputs over scenarios+probes_pending
xargs -P 8; REMOVE the worktree before git add -A.

## REPO/DOCTRINE STATE

ALL PUSHED through 9f3dc14 (main, untagged; latest release
v0.4.44; publish-crates standing red = Bryan's crates.io TP).
CHANGELOG carries ELEVEN release-ready Unreleased entries —
release ONLY on Bryan's explicit release word; "push, no tag" =
push main untagged; NEVER push unprompted. Commit-per-green-
slab; predictions registered in PINS.md BEFORE cells run;
misses recorded. PITFALLS (bit this session repeatedly): bash
cwd PERSISTS — never `cd X && ...` then relative paths in later
calls; oracle __h parser must guard synthetic facts; maturin
clobbers the tracked .so; worktrees must be removed before
`git add -A`. Other open old-bank families after this round:
the QUERY pair (xf_fz_296001_1704/xf_fz_296002_1494, adjacent
to the ?query-justifiers wall) + the fz_123_6887 ORACLE-FLAPPER
census (standalone passes are NOT graduation evidence).
