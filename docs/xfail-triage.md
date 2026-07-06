# xfail triage — the quarantine, itemized (D-087)

Triage of every scenario in `scenarios/xfail/` (75 files at HEAD 03aee92,
2026-07-06), executing the D-080 mandate: classify the TMS envelope
witnesses into pinnable / Drools-nondeterministic / ambiguous-micro-timing,
folding in the D-042 order-trio. Method:

- **Engine**: one run (the engine is deterministic by construction).
- **Oracle**: **10 independent replicates, each a fresh JVM launch**
  (Drools 9.44.0.Final), exceeding the D-080 "verify 2–3x" bar.
- **Comparison**: canonical D-003 semantics — facts as a multiset, firing
  log order-significant, matches within a firing as a multiset, f64 by
  IEEE-754 bit equality.
- **Shape analysis**: each scenario's text is tested against the
  D-078/D-080 fence line (marker legend below).

Regenerate with: `python3 tools/triage_xfail.py --runs 10 --md <out>`
(first run performs the engine + oracle runs into `target/triage_cache/`;
delete that directory to force fresh runs).

## Verdict

**Zero in-envelope pin candidates.** Every diverging TMS witness carries at
least one D-078/D-080 fence marker — the fence line as drawn covers the
entire quarantine, and no divergence hides inside the certified envelope.
The correct action for every family is fence-and-document (executed here);
no engine change is warranted by any witness.

| family | count | oracle behavior (10 runs) | classification |
|---|---|---|---|
| I. compound transient-visibility | 45 | 10/10 identical | fence: ambiguous micro-timing (D-080) |
| II. Drools runaway | 22 | 10/10 fire-limit | fence: uncertifiable (engine terminates) |
| III. Drools order-nondeterminism | 1 | 6/10 vs 4/10 order flip | fence: uncertifiable |
| IV. D-042 order-trio | 3 | 10/10 identical | fence: accepted order-only carve-out |
| V. D-084 held-staging fence | 4 | 10/10 identical | fenced pending sources-port (Bryan's ruling) |

Fence-shape marker legend (computed from scenario text):

- **A** — a justifier RHS mixes `insertLogical` with mutation
  (`set*/update/modify/delete`) — D-080 shape (a).
- **B** — stated insert of the logical type (by a rule, an initial fact, or
  an epoch fact) — stated/justified key mixing, D-080 shape (b).
- **RD** — a rule deletes a fact bound to the logical type (immediateDelete
  vs staged-cancellation path divergence, min4048 family).
- **SJ** — CE-only self-justifier: the justifier's LHS sees the logical
  type only via `not`/`exists` (the t10/t21/946 family).
- **XU/XD** — an external action updates/deletes a logical-type fact
  (external deletes are IN-envelope per D-078; reported for completeness).

## Families

### I — compound transient-visibility micro-timing (45, oracle-deterministic)

All 45 sit inside the D-080 fence shapes; marker census: **A** 25, **B** 29,
**RD** 12, **SJ** 17, **XD** 2 (combos led by `A,B` ×14 and pure `SJ` ×13).
The divergences are small firing-multiset deltas in BOTH directions
(oracle-extra `-n` and engine-extra `+n` both occur — differing transient
windows, not a systematic under/over-fire); five witnesses also differ in
final facts (fz_7_1353, fz_7_8757, fz_7_9360, fz_7_9550, fz_7_9902).

Two representative narratives:

- **xf_tms_min812** (SJ, minimized to 2 rules): `not T2()` →
  `insertLogical(new T2(...))` is a parked self-defeat in the engine; Drools
  lets a sibling accumulate rule fire ONCE against the transient T2 before
  the lazily-processed retraction lands (oracle 2 firings, engine 1,
  final facts identical and empty). The single-glimpse transient, observed
  through an accumulate.
- **fz_7_9902** (B): firing logs are byte-identical (14 firings); only the
  final WM differs — the oracle retains one extra stated `T1` duplicate.
  Stated/justified key bookkeeping under value-equality mixing, no timing
  component at all.

Per D-080: every single-mechanism minimization of this family PASSES (the
pinned probes); only the compounds diverge, and each peel exposed another
RuleExecutor internal. Fenced as documented-open; the generator does not
draw these shapes (D-078).

### II — Drools runaway (22, engine terminates)

All 22 are **SJ** — CE-only self-justifiers (with ≥2 deps on one key /
observer rules, the fz_42_946 family from the D-080 second campaign). The
oracle hit the 100k fire limit in **10/10 launches for every witness**; the
engine terminates cleanly on all of them (2–15 firings) via the self-defeat
park (pr_tms_t10/t11/t15/t21 pin the certified single-tuple semantics).

Note on the fz_42_84 family (84/581/2657): D-080 recorded pass-vs-fire-limit
flips across JVM launches (identity-hash-order-dependent cascade churn).
Today's 10 runs produced fire-limit on all three — the flip was not
reproduced, but the class remains launch-dependent per the D-080 record.
Either way the family is uncertifiable by a differential harness: there is
no stable oracle answer to certify against, and the engine's clean
termination is the strictly better behavior.

### III — Drools order-nondeterminism (1)

**fz_123_6887** (shape B,RD): across 10 launches the oracle produced TWO
firing orders — 6/10 one interleave, 4/10 another (identical 14-firing
multiset and identical final facts; the flip is an R5/R3 refire interleave
after firing 10). Drools' own firing order on this input is a function of
the JVM launch, not the program — uncertifiable. (The engine additionally
fires 11 of the 14 — three transient refires short, the same family-I
compound class; final facts match.) A NEW nondeterminism witness beyond
D-080's fz_42_84-family list.

### IV — D-042 order-trio (3, pre-TMS)

**nb3, fz_7_2364, fz_min_7_2364** — no TMS involvement (no insertLogical).
Oracle 10/10 stable; engine diverges in firing ORDER only (first swap at
positions 2–3 of 6–7 firings): the relative refire order of tuples
unblocked together at a not-node inside ≥3-pattern rules under
modify-entering blockers whose delete also removes a blocked left. The
D-042 carve-out (accepted 2026-07-04: rare, order-only,
mechanism-ambiguous after deep source reading) is RE-AFFIRMED with this
10-run evidence; D-083's re-entry discriminator did not dislodge it (the
D-042 siblings fz_999_8145/fz_27182_1227 graduated at D-083; this trio is
the residue). Revisit only per D-042's trigger (a value-bearing variant or
new mechanism evidence), likely alongside the D-084 sources-port.

### V — D-084 held-staging fence (4)

**fz_7_455, fz_min_455, fz_42_4816, fz_min_4816** — the boundary-drain
fence per Bryan's D-084 ruling (resume via drools-core sources-port ONLY;
six black-box rounds exhausted). Oracle 10/10 stable — the class is fully
deterministic mechanics, not nondeterminism. fz_42_4816 is ORDER-ONLY
(64 firings, first swap @51); the other three carry small equal-count
firing/fact swaps. Validation harness for the port: pr_rl2..rl10 +
fz_42_4035/fz_123_2742/fz_123_3482/fz_999_6009 (green regressions) + these
four.

## Per-witness table

Bucket = behavioral classification (see tools/triage_xfail.py header);
`-n/+m` = n oracle-only / m engine-only entries.

| scenario | bucket | fence shape | oracle runs | engine vs oracle |
|---|---|---|---|---|
| fz_123_1338 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (6 firings) |
| fz_123_2135 | VALUE | SJ | 10/10 OK | firings 2 vs 1 (-1/+0) |
| fz_123_2349 | VALUE | A,B | 10/10 OK | firings 3 vs 4 (-0/+1) |
| fz_123_2674 | VALUE | A,B | 10/10 OK | firings 12 vs 16 (-0/+4) |
| fz_123_274 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_123_2934 | VALUE | A,B | 10/10 OK | firings 1 vs 3 (-0/+2) |
| fz_123_3060 | VALUE | SJ | 10/10 OK | firings 3 vs 4 (-0/+1) |
| fz_123_3370 | VALUE | SJ | 10/10 OK | firings 18 vs 17 (-1/+0) |
| fz_123_3406 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (3 firings) |
| fz_123_3767 | VALUE | A,B,XD | 10/10 OK | firings 10 vs 11 (-0/+1) |
| fz_123_3988 | VALUE | A,B | 10/10 OK | firings 5 vs 6 (-0/+1) |
| fz_123_4036 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_123_4318 | VALUE | SJ | 10/10 OK | firings 4 vs 3 (-1/+0) |
| fz_123_4866 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_123_4904 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (8 firings) |
| fz_123_6887 | ORACLE-NONDET | B,RD | 6/10 OK \| 4/10 OK | order flip |
| fz_123_7219 | VALUE | B,RD | 10/10 OK | firings 11 vs 10 (-1/+0) |
| fz_123_7637 | VALUE | SJ | 10/10 OK | firings 3 vs 2 (-1/+0) |
| fz_123_9133 | VALUE | SJ | 10/10 OK | firings 4 vs 1 (-3/+0) |
| fz_123_9175 | VALUE | B,RD,SJ | 10/10 OK | firings 2 vs 5 (-0/+3) |
| fz_123_9269 | VALUE | A,B | 10/10 OK | firings 3 vs 5 (-0/+2) |
| fz_123_941 | VALUE | A,B,RD | 10/10 OK | firings 9 vs 10 (-0/+1) |
| fz_123_9462 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (12 firings) |
| fz_123_9804 | VALUE | A,B | 10/10 OK | firings 5 vs 6 (-1/+2) |
| fz_42_166 | VALUE | A | 10/10 OK | firings 9 vs 13 (-0/+4) |
| fz_42_2657 | ORACLE-RUNAWAY | B,RD,SJ | 10/10 FIRE-LIMIT | engine terminates (8 firings) |
| fz_42_2829 | VALUE | A,B | 10/10 OK | firings 3 vs 7 (-0/+4) |
| fz_42_4300 | VALUE | A,B | 10/10 OK | firings 5 vs 6 (-0/+1) |
| fz_42_4442 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (6 firings) |
| fz_42_4816 | ORDER-ONLY | — | 10/10 OK | 64 firings, first swap @51 |
| fz_42_5213 | VALUE | SJ | 10/10 OK | firings 20 vs 13 (-7/+0) |
| fz_42_581 | ORACLE-RUNAWAY | B,SJ | 10/10 FIRE-LIMIT | engine terminates (3 firings) |
| fz_42_6368 | VALUE | B,RD | 10/10 OK | firings 6 vs 5 (-1/+0) |
| fz_42_7619 | VALUE | A,B | 10/10 OK | firings 7 vs 12 (-0/+5) |
| fz_42_8206 | VALUE | A,B | 10/10 OK | firings 5 vs 7 (-0/+2) |
| fz_42_84 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_42_9324 | VALUE | A,B | 10/10 OK | firings 1 vs 2 (-0/+1) |
| fz_42_946 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (5 firings) |
| fz_777_1278 | VALUE | B,RD,SJ | 10/10 OK | firings 3 vs 4 (-0/+1) |
| fz_777_1296 | VALUE | A,B,XD | 10/10 OK | firings 5 vs 6 (-0/+1) |
| fz_777_2956 | VALUE | A,B,RD | 10/10 OK | firings 7 vs 8 (-0/+1) |
| fz_777_5036 | VALUE | A,B | 10/10 OK | firings 12 vs 13 (-0/+1) |
| fz_777_6662 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (10 firings) |
| fz_777_6762 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (6 firings) |
| fz_777_7168 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (3 firings) |
| fz_777_9637 | VALUE | SJ | 10/10 OK | firings 2 vs 1 (-1/+0) |
| fz_7_1353 | VALUE | SJ | 10/10 OK | facts -5/+0; firings 12 vs 5 (-8/+1) |
| fz_7_1441 | VALUE | A,B | 10/10 OK | firings 15 vs 20 (-0/+5) |
| fz_7_1591 | VALUE | A,B,RD | 10/10 OK | firings 8 vs 11 (-0/+3) |
| fz_7_1879 | VALUE | A | 10/10 OK | firings 2 vs 6 (-0/+4) |
| fz_7_1914 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (6 firings) |
| fz_7_2142 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_7_2364 | ORDER-ONLY | — | 10/10 OK | 7 firings, first swap @3 |
| fz_7_2864 | VALUE | A,B,RD | 10/10 OK | firings 26 vs 23 (-3/+0) |
| fz_7_3803 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (15 firings) |
| fz_7_4048 | VALUE | B,RD | 10/10 OK | firings 8 vs 7 (-1/+0) |
| fz_7_455 | VALUE | — | 10/10 OK | facts -4/+4; firings 11 vs 11 (-4/+4) |
| fz_7_5988 | VALUE | A,B,RD | 10/10 OK | firings 6 vs 7 (-0/+1) |
| fz_7_6923 | VALUE | A,B | 10/10 OK | firings 10 vs 14 (-0/+4) |
| fz_7_812 | VALUE | SJ | 10/10 OK | firings 2 vs 1 (-1/+0) |
| fz_7_8360 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (5 firings) |
| fz_7_8623 | ORACLE-RUNAWAY | A,B,SJ | 10/10 FIRE-LIMIT | engine terminates (3 firings) |
| fz_7_8757 | VALUE | B,RD | 10/10 OK | facts -1/+0; firings 6 vs 5 (-1/+0) |
| fz_7_930 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (2 firings) |
| fz_7_9360 | VALUE | A,B,RD,SJ | 10/10 OK | facts -2/+2; firings 18 vs 13 (-5/+0) |
| fz_7_9375 | VALUE | SJ | 10/10 OK | firings 3 vs 4 (-0/+1) |
| fz_7_9550 | VALUE | A,SJ | 10/10 OK | facts -1/+1; firings 2 vs 3 (-0/+1) |
| fz_7_9628 | ORACLE-RUNAWAY | SJ | 10/10 FIRE-LIMIT | engine terminates (6 firings) |
| fz_7_9864 | VALUE | SJ | 10/10 OK | firings 19 vs 17 (-2/+0) |
| fz_7_9902 | VALUE | B | 10/10 OK | facts -1/+0 |
| fz_min_455 | VALUE | — | 10/10 OK | facts -2/+2; firings 6 vs 6 (-2/+2) |
| fz_min_4816 | VALUE | — | 10/10 OK | firings 10 vs 10 (-1/+1) |
| fz_min_7_2364 | ORDER-ONLY | — | 10/10 OK | 6 firings, first swap @2 |
| nb3 | ORDER-ONLY | — | 10/10 OK | 6 firings, first swap @2 |
| xf_tms_min812 | VALUE | SJ | 10/10 OK | firings 2 vs 1 (-1/+0) |
