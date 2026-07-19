# HANDOFF — the D-125 cascade round: the x167 temporal residual
# (cold start; D-338 rounds 1-3 are the prior arc, commits
# a303f3b / f553baf / 1febfce, all pushed on main untagged)

Written 2026-07-19. Bryan's directive: port the intermediate-tuple
handoff order into downstream TEMPORAL joins within one arrival
cascade — the last step between cf318902x167 (scenarios/xfail/,
the final banked witness of its class) and graduation. Read
probes_pending/chain3/PINS.md FIRST (D-338 rounds 1-3: the
event-ness law, the ph=5 port, the round-3 scoping).

## THE TARGET

Would-graduate: cf318902x167. The residual witnesses live IN THIS
LANE: x167_m1.json (entry-point + temporal chain) and
x167_m2.json (temporal chain only — THE CLEANER CARRIER; the
entry-point is not load-bearing, proven round 1). The fork on
both: ONE adjacent swap — engine CH2(644,616,648) before
CH2(644,614,648); oracle 614-first; facts identical; 3x-stable.

## WHAT IS ALREADY MEASURED (do not re-derive)

- The D-338 law (PORTED, certified): EVENT-typed rights held on
  unlinked paths walk ARRIVAL-ordered (ph=5, Drools' per-insert
  stream force-flush rides event inputs); PLAIN facts keep the
  certified pre-LIFO (ph=4). Plain chains of the x167 shape all
  PASS (pr_ch_b3/b4/b5, pr_ch_x167_m3/m5).
- tm5 (2-pattern TEMPORAL held: E0() E1(before), rights one
  epoch early): engine==oracle ALREADY — graduated
  pr_ch_tm5_temporal_held. The temporal branch's own staged walk
  is HEALTHY.
- THE CARRIER (round-3 scoping, instrumented-print-silent proof):
  the (E0,E1) children of the FIRST temporal join reach the
  SECOND temporal join OUTSIDE do_node's staged-lefts walk — a
  stamp-loop eprintln in phreak.rs's temporal else-arm (the
  `for (l,_,_) in &sl.ins` stamping) NEVER FIRED on m1 or m2.
  They ride the D-125 v2 flush-model cascade (the per-arrival
  path). do_node is phreak.rs:1679, the temporal split ~2295.
- THE ORACLE COMPOSITION (composes m1/m2 exactly): join1 rtm
  arrival [614,616]; E0@644's cascade emits (E0,614),(E0,616);
  ONE staging flip into join2 (prepend-build + head-walk) =>
  join2 lefts/lseq order [616,614]; E2@648's partner scan
  (arrival) => emission 616-first => one flip at the terminal =>
  consumption 614-first. THE ENGINE hands the children to join2
  UN-FLIPPED (614 first) => consumption 616-first. One missing
  flip, location = the cascade handoff.

## THE PLAN

1. LOCATE the cascade: where does a temporal join's child
   emission reach the next temporal join during a per-arrival
   flush? Candidates: stream_flush_ex (engine.rs ~7003+, the
   evaluate path), the tj_epoch machinery (fire_all ~7872), or
   an inline hop in evaluate_rule_inner. Instrument (env-gated
   eprintln) to dump the arrival order of join2's incoming
   children on m2 — expect [614-child first] engine-side.
2. MODEL-CHECK BEFORE PORTING if the site is shared with
   certified compositions: tools/model_check_stream.py and
   model_check_twalk.py are the D-102/D-125 molds; the jr pins
   and D-125 flush-model records are the certified adjacent data.
3. Design the pin ladder predictions-FIRST (the m2 shape +
   variants: E2 arriving in an earlier/same epoch, 3 E1s, the
   second join PLAIN while the first is temporal, and the
   reverse) — each 3x oracle.
4. THE PORT: add the missing flip (or the lseq-stamp order
   correction) at the cascade handoff. Blast surface: the jr
   pins, the TJ/TJUPD corpus, the D-125 flush-model cells, every
   cep regression. The byte gate decides; oracle-diff EVERY
   mover.
5. Graduate cf318902x167 (+ m1/m2 as pr_ch_*), rebank (19->18).

## VERIFICATION SET FOR THE PORT

Byte gate: /home/bryan/.claude/jobs/<job>/tmp/bytegate2.sh-style
(re-create if the job tmp is gone: worktree add wt_preNNN at the
pre-edit sha, release build BOTH, compare `run` outputs over
scenarios+probes_pending; sed the worktree name FIRST). Full
battery (current values): make diff 11/1510/414 + drift 19
identical (~60s); lint 2363-ish/0/0; SD census 12x150 model
0-div, engine 71 EXACT (6,10,3,4,6,5,5,6,8,7,4,7 — debug build
first, NEVER rebuild mid-census); cargo 74; maturin develop
--release from bindings/ then git checkout the tracked .so BEFORE
commits; pytest 257; demo True; model_ird 31/31 (cd
probes_pending/tms_envelope); IRD 0-div x5 (seeds
7001/7002/6001/6003/9001, run tools/fuzz_tms_ird.py FROM REPO
ROOT); agenda_open x10 identical x3 (release/debug/pre-edit
worktree); fresh fuzz 2x2000 NEXT seeds 339001/339002 + fuzz_cep
3x300 seeds 339901-903 (bisect any find vs the pre-edit worktree:
pre-existing => quarantine to scenarios/xfail/ + rebank + re-run
the seed).

## REPO STATE (2026-07-19)

Pushed through 1febfce (main, UNTAGGED — latest release v0.4.44
at 1b9a20c). CHANGELOG Unreleased is EMPTY: the D-338 ph=5
event-arrival fix needs its user-visible entry (+ this round's,
when it lands) BEFORE the next release. Ledger after this arc:
crates.io TP (Bryan's manual step), ?query justifiers (unprobed
wall), THIS ROUND, fz_336002_968 + fz_337002_1104 (fresh
pre-existing quarantines), the Phase E admission-vs-ins corner
(no witness), windowed average_exact fence, the fz_9004_214
tail-order note (agenda_open lane, oracle-closer since D-337).
Doctrine: oracle decides; predictions in PINS BEFORE cells run;
hand-decode witnesses end-to-end; commit per green slab on main;
NEVER push without Bryan's word ("push, no tog" = push main
untagged; releases only on his explicit release word); cwd
persists across Bash calls — use absolute paths; the harness
oracle idiom: java -Xss1g -cp "oracle/target/classes:$(cat
oracle/target/classpath.txt)" dev.seine.oracle.OracleRunner
<cell.json> (3x per measurement).
