# HANDOFF — the OLD-BANK quarantine families round (cold start;
# written 2026-07-19 at c757528+, Bryan: "start work on the
# standing Item 7 old quarantine families")

Read this first, then probes_pending/focuspop/PINS.md's D-345..349
arc for the freshest round-doctrine examples (decode → model →
trace → port; misses recorded; naive ports reverted on gate
evidence).

## THE TARGET — ten aged witnesses, FRESH-TRIAGED 2026-07-19
## (all re-measured this day; classes below are today's forks)

ORACLE-NPE family (2) — Drools CRASHES, engine succeeds:
- xf_fz_31337_698, xf_fz_8087_1043: oracle NPE
  `Tuple.getStagedType() ... "tuple" is null`. PRECEDENT: D-263
  (an oracle NPE resolved upstream in Drools 10.1.0; no filing).
  10.1.0 jars EXIST in ~/.m2 (org/drools/*/10.1.0). The round:
  build a 10.1.0 classpath variant of the oracle runner, run
  both witnesses — if clean there, these are oracle-defect
  records; disposition (reclassify out of the bank vs keep) is
  BRYAN'S CALL.

ORACLE-FLAPPER (1):
- fz_123_6887 (TMS-envelope era): passes a STANDALONE diff 4/4
  today but FAILED once inside make diff's parallel run — the
  ORACLE side flips (the engine-side drift bank has been stable
  throughout; no gate hole). DO NOT graduate on standalone
  passes; the round needs a 10x stability census (standalone AND
  in-corpus-context), then either the D-131 undefined-class
  treatment or a real decode of the flipping surface.

QUERY family (2) — VALUE forks on ?query outputs (ADJACENT to
the ?query-justifiers unprobed wall; consider folding into that
future round):
- xf_fz_296001_1704: a QOut fact differs (engine-only rows,
  d-field values fork).
- xf_fz_296002_1494: queries[1] row[2] binding values differ.

COUNT class (2) — firing-count forks (value-class, highest
semantic yield per the shop's experience):
- xf_fz_296002_626: engine 7 vs oracle 5.
- fz_777_1278: engine 4 vs oracle 3.

ORDER class (3):
- xf_fz_141421_123: firing[141] swap (DEEP — expect heavy
  minimization; R4 2-pattern T1xT1 self-join-ish).
- xf_fz_7331_973: firing[16] swap (R2, T0xT1).
- xf_fz_8087_1020: firing[5] swap (R2 matching TWO EQUAL T0s —
  a twin-fact order; possibly the fz_42_84 identity-hash class,
  check before grinding).

## THE PLAN (one family per slab; commit-per-green-slab)

1. START with the ORACLE-NPE pair (cheapest disposition): the
   10.1.0-classpath check, then gate with Bryan.
2. Then the COUNT pair — hand-decode each witness end-to-end
   FIRST (the D-323/D-333 doctrine), minimize (drop rules/facts
   while the fork survives), register predictions in a new
   probes_pending/oldbank/PINS.md BEFORE cells run, model if the
   surface is shared/calibrated (the D-345 lesson: naive gates
   on calibrated surfaces break certified cells — REVERT on gate
   evidence, model, then port ONCE).
3. The ORDER trio and QUERY pair follow, each its own round; the
   query pair may fold into the ?query-justifiers probe round.
4. Every port: byte gate vs pre-edit HEAD (expect ONLY the
   would-graduate movers), oracle-diff every mover, graduate +
   rebank, full battery.

## VERIFICATION CARD (current values, all green at handoff)

make diff 11/1554/414 + drift bank 16 identical; lint 2413/0/0;
cargo 74; pytest 260 (maturin develop --release from bindings/
via ../.venv/bin/maturin, then git checkout the tracked
bindings/python/seine_rs/_native.abi3.so BEFORE commits); demo =
.venv/bin/python3 demo/adsb_convergence.py → True; model_ird
31/31 (cd probes_pending/tms_envelope); IRD 0-div x5 (seeds
7001/7002/6001/6003/9001, tools/fuzz_tms_ird.py FROM REPO ROOT);
SD census `python3 tools/fuzz_tms_sd.py 150 <seed>` seeds
7001,7002,6001,6003,7004..7011 → 6,10,3,4,6,5,5,6,8,7,4,7 = 71
EXACT (debug build first, NEVER rebuild mid-census); agenda_open
x10 identical x3 (release/debug/pre-edit worktree); fresh fuzz
2x2000 NEXT seeds 349001/349002 + fuzz_cep 3x300 NEXT seeds
349901-903 (finds: bisect vs the pre-edit worktree; pre-existing
→ mv scenarios/xfail/ + `python3 tools/xfail_drift.py --rebank`
+ re-run the seed). Byte gate recipe: `git worktree add
wt_preNNN <sha>`, release build BOTH, compare `run` outputs over
scenarios+probes_pending via xargs -P 8 (a bytegate2.sh lives in
the current job tmp but job tmps DO NOT survive — re-create).
Oracle idiom: java -Xss1g -cp "oracle/target/classes:$(cat
oracle/target/classpath.txt)" dev.seine.oracle.OracleRunner
<cell.json>, 3x per measurement.

## REPO/DOCTRINE STATE

Pushed through c757528 (main, untagged; latest release v0.4.44).
CHANGELOG carries EIGHT release-ready Unreleased entries
(D-338..D-348) — releases ONLY on Bryan's explicit release word;
"push, no tog"/"push, no tag" = push main untagged; NEVER push
unprompted. Other open items: the Phase E admission-vs-ins
corner (witness-less), ?query justifiers (unprobed wall),
group_by x average_exact + collect fences, crates.io TP
(Bryan's), diff-duckdb outside the standard battery (one-line
decision; its comparator was fixed in D-341), fz_9004_214 +
SD-71 recorded approximations, the 128B x 1M perf note,
query-resize churn/>96 corners. PITFALLS THAT BIT THIS SESSION
(twice each): bash cwd PERSISTS across calls — use absolute
paths, never bare `cd X && ...` loops; the engine DRL subset
REJECTS `$a.field` cross-pattern access (use bindings — and a
binding ADDS a listen bit, which can silently change what a
probe measures); git worktrees must be REMOVED before `git add
-A` (an embedded-repo add slipped into a commit once and needed
an amend).
