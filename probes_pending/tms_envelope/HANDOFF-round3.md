# COLD-START HANDOFF: the port slab, round 3 (post-D-198)

_Everything a fresh context needs. State as of `bcdfe1d` (D-198,
pushed): the D-080 TMS-envelope PROBING phase is COMPLETE (D-186..
D-196 — the landing-row table is CONFIRMED, model_sd.py is the
executable spec at 0-div, 12 seeds × 150); the ENGINE PORT ("the
port is translation" — Bryan) has TWO ROUNDS LANDED. Census:
pre-port 599 → 483 divergent (12 seeds × 150; the A/B metric).
Read `port-target-list.md` (mechanisms P1-P6 + per-round status) and
DECISIONS D-197/D-198 before touching anything._

## What is in the engine now (rounds 1-2, all receipts green)

- **The deferral CAUSE MODEL** (D-197): `tms.deferred: Vec<(usize,
  Tup, u8)>` — flags bit0 LIA-hit (t20 flush discipline; from
  `left_touched`, watch-gated s0 staging), bit1 NOT-side self-defeat
  (`right_touched`, NOT/SubnetNot node right-ops only ⇒ flush-drains
  unconditionally at run end), bit2 LATE-DEP (`late_acts`, set at
  tms_insert_logical when the act's own tuple alpha is ALREADY
  broken = the D-195 mutfirst race ⇒ the last-firing entry rides to
  the POP = the zombie window), bit3 JOIN-RIGHT (`joinr_touched`,
  positive-pattern rights — the LEAD topology's P side ⇒ flush
  MID-RUN only, gated on run_live = queue non-empty). Both flush
  gates in next_activation use: `dyn || bit1 || (bit0 && (run_live
  || !bit2)) || (bit3 && run_live)`.
- **Clause C / t15-d4** (D-198): `tms_parked_del` left-death unparks
  the rule's other parked tuples and re-activates live FULL-WIDTH
  ones in REVERSED-chain order; scope = LAZY plain rules (no-loop/
  dyn/or-twin excluded — fz_777_6816 is the falsifier; prefix parks
  never re-activate directly — a 3-fact population shape PANICKED
  before that guard: populations are the panic net).
- **Clause B lead** (D-198): the park's blocked-leak finds nots by
  ENV LOOKUP (`trie[ni].env == (ri,pos)`; pos-1 arithmetic is
  trail-only), parks the blocked LEFT PREFIX, spans OR-SIBLINGS
  (rule_parents groups) and prunes their queues; `tms_parked_ins`
  matches by `starts_with`. The eager list is TWO-PASS (evaluate all
  eager rules, then drain+re-eval — Drools' evaluateEagerList).
- Sibling-eval before drops at all three drain sites; every drain
  site carries `SEINE_TMS_DEBUG` tags: `defer-push … flags=`,
  `drain[post-fire-continue|flush-pre|flush-post|pop]`.
- Graduates so far: fz_123_941 (D-197), fz_42_5213 + fz_123_3060 +
  fz_7_9550 (D-198). Drift bank = 71. Corpus 11/1124/358.

## ROUND-3 TARGETS (in order)

### 1. The or-twin corner — sd_b4 / fz_7_9375 (READ FIRST, then fix)

Engine fires the twin branch; oracle fires once. THE DIAGNOSIS SO
FAR (do not redo): the drain-site sibling-eval DOES consume the
sibling's in-firing block (its queue prunes — a terminal-del for the
sibling is visible in the debug trace), and the group park + queue
prune are in place — **but `node.blocked_of(blocker)` returns
NOTHING on the sibling's not**, so no park leaks and the un-break
re-derives the sibling's activation (`EVAL[pop] rule 1` fires it).
SUSPECT: the D-158 **PnShadow** (plain-not blocker shadows,
`pn_on_wm_insert` in on_insert) — the plain-not's blocking may live
in the shadow structure, not the node's blocked map. NEXT STEP:
read the PnShadow struct + where blocking is recorded for plain
nots; either leak the park from the shadow, or take the SdDump
route (bank a b4-core as a graft target and dump ×3 — the dumper is
`oracle/src/main/java/dev/seine/oracle/SdDump.java`, run from repo
root, 3 launches, identity-normalized diff). sd_b3 (lazy or-twin)
stays FENCED — Drools runaway, Family II — do not chase.

### 2. The five regressed seeds' slots (bisect, then fix or revert-narrow)

Round 2 moved 7002 46→50, 7006 35→37, 7007 44→48, 7008 41→46, 7004
44→45 (engine-vs-oracle divergent counts) while fixing others.
RECIPE: `python3 tools/fuzz_tms_sd.py 150 7002 --keep` on HEAD, and
the same in a `git worktree` at `99b363d` (D-197 — NEVER stash/
checkout in place; worktree only); per case compare engine-vs-
oracle match booleans; the cases that flipped MATCHING→DIVERGENT
are the regression slots. Expect them to implicate the round-2
park/revive/two-pass on geometries the pins don't cover; fix by
NARROWING scope (the fz_777_6816 pattern), never by adding proxy
conjuncts (⚖ epicycle stop).

### 3. The lazy L-MB mass (~483 divergent total)

Sample survivors from any census run (`--keep`, diff engine vs
oracle per case), classify against port-target-list.md's P-list,
fix the biggest cluster first. Most are expected to be lazy-lane
landing/order (the model — model_sd.py — is the spec at 0-div;
`check_witnesses.py` = 28 banked oracle truths, oracle-free).

### 4. P3 — the equal-salience queue-position drain split (LAST, ⚠ D-106)

The L-SD under-family (~9-11 xfail witnesses: min812,
fz_123_{2135,3370,4318,7637,9133}, fz_777_9637, fz_7_812, fz_7_9864).
THE SITE: `next_activation`'s post-fire continue path — the `higher`
gate (strictly-higher) governs BOTH the D-091 network re-eval halt
AND the TMS deferred drain; the min608 over-generalization is that
the drain also runs at equal salience, killing the transient before
a DECL-PRECEDING same-salience observer pops (min812's certified
glimpse). THE FIX SHAPE: split the gates — the network-re-eval halt
keeps strictly-higher (halt-matrix certified); the DRAIN gate
becomes pop-precedence: drain only if NO queued same-group item has
(sal > l_sal) OR (sal == l_sal AND decl < l_decl). ⚠ D-106: the
halt-check region — agenda_open ×19 byte-identical receipts
MANDATORY before/after (keep `$SCRATCH/ag_open_base.ndjson`-style
baselines via a worktree build, verify `git diff HEAD --stat`
after any stash dance); the halt matrix and fz_9001/9003/9004
witnesses are the tripwires. If receipts move, STOP and report.

## The iteration loop (per change)

```
cargo build -q -p seine-harness
# cells: the ladder + the battery
python3 probes_pending/tms_envelope/interposer_ladder.py --run /tmp/ip3   # 6/6 expected
cargo run -q -p seine-harness -- run probes_pending/tms_envelope/sd_c1_alternation.json ...  # spot cells
make diff          # 4 tiers; xfail movement ⇒ re-triage 10×10 both sides,
                   # graduate CONVERGED (regressions/ + comment + git rm the xfail
                   # + make xfail-rebank + D-entry); Family-II (fire-limit oracle)
                   # movement ⇒ rebank only
make lint-probes; cargo test -q
# receipts (⚠ D-106) — worktree or stash-dance WITH git diff --stat verification:
cargo run -q -p seine-harness -- run probes_pending/agenda_open/*.json > now.ndjson; diff vs baseline
# the metric + the panic net (12 seeds, ~35 min, background):
for seed in 7001 7002 6001 6003 7004 7005 7006 7007 7008 7009 7010 7011; do
  python3 tools/fuzz_tms_sd.py 150 $seed; done   # model MUST stay 12×150/150
python3 probes_pending/tms_envelope/check_witnesses.py               # 28/28
```

## Standing discipline

Bryan's sequencing: the port slab to completion, **I-RD LAST**.
Never push v* tags. Commit per green slab with a D-entry; Bryan
holds pushes (D-196..198 are pushed through `bcdfe1d`). Predictions
before instrument runs; ⚖ method law (an underdetermined output is
not a finding — build the splitter); ⚖ epicycle stop (a rule that
needs a proxy variable doesn't know its mechanism); the identity/
dedup/landing laws + the D-106 caveat are in the workflow memory
and docs/tjupd-ledger-mechanisms.md. gen.rs walls STAY UP
(fuzz_tms_sd is arc-local recon). The oracle is 9.44.0.Final+p1;
oracle rebuilds: `cd oracle && mvn -q -DskipTests package` (then
run from repo root — the cd hazard). Gates on resume: make diff +
lint-probes + `validate_cells.py` (39/39) + check_witnesses (28/28).
