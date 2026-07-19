# HANDOFF — the CEP event-drain churn round (cold start)

> **CONSUMED (2026-07-18): the round ran as D-327** — the five
> members graduated (pr_co_cf*), the law + port are recorded in
> PINS.md (mechanism C′: entries drain at insert positions,
> FIFO effects, the revival fold) and DECISIONS.md D-327.
> Residuals from the standing ledger: xf_fz_662607_47
> (collectSet), cf325901x52, cf318902x167.

Written 2026-07-18 at the end of the D-311..D-326 session. The
target: the FIVE-member collectList order-fork class on
EVENT-typed sources — cf324903x55, cf325902x221, cf325902x88,
cf326901x184, cf326902x239 (all in scenarios/xfail/, all banked
with engine output pinned in the drift bank). Every member:
firing tuples identical except ONE windowed-collectList
Collection whose elements are the same MULTISET in a different
ORDER (adjacent duplicate-tag swaps like [..,z,y] vs [..,y,z]),
in fuzz_cep scenarios with advance/update/delete churn over
E-types. All worktree-bisected PRE-EXISTING (none introduced by
the D-320..D-326 engine work). Sibling sub-item, same lane dir:
xf_fz_662607_47 (collectSet FIRST-INSTANCE order, gen.rs plane,
NO clock — probably a separate small law; c8/c8b/c12 minimal set
cells all MATCH, so its ingredient is also composition-level).
NOT family: cf325901x52 (a not-DW P-witness ORDER fork —
reclassified out, unexplained agenda-order latent).

## WHAT IS ALREADY LAW (do not re-derive — pr_co_* cells pin it)

1. **D-323**: on a MATERIALIZED collectList, reverse removes the
   FIRST VALUE-EQUAL element (java List.remove(Object)),
   regardless of which fact retracted. Build = LIFO-arrival
   append for batches; sequential arrivals append in order.
   Pre-materialization ins+del pairs annihilate in staging.
2. **D-324**: the same reverse law holds through WINDOW
   evictions (time and length; the w2 splitter: the evicted
   instance's VALUE slot can outlive its owner). Windowed
   batch build is LIFO too. ts-move-only updates are invisible;
   ts+value updates = remove-first + append (w6/w7).
3. **D-326**: a same-batch alpha EXIT+RE-ENTRY folds to ONE
   update (Drools stages by fact identity) draining at the
   UPDATE position — before fresh inserts, LIFO among updates;
   value-preserving folds move-to-tail (the a8 pair). PORTED at
   on_update's INLINE (false,true) arm for PLAIN-typed acc
   sources only (engine.rs, grep "D-326"). Joins keep del+ins
   ph=1 (jr pins — join children are new objects per c13).
4. Updates drain LIFO among themselves (a5/a7); plain
   update-then-insert already lands update-first (a1) — the
   simple two-queue-order hypothesis is DEAD, don't revive it.

## WHY THE CEP MEMBERS ARE DIFFERENT (the open composition)

EVENT-typed acc sources do NOT take the inline arm — external
updates ride `acc_pending` (AccEntry::Upd per call, D-154: ONE
entry per call with ITS OWN mask, NO merging — a pinned
anti-fold!) drained at the fire boundary by `drain_acc_pending`
→ `plainacc_step`/`winacc_step` (engine.rs ~6640-6830), each
entry evaluated against EPOCH-FINAL fields (D-160 — so a
two-call exit+re-entry may read as (true,true)+(true,true), no
exit at all: the D-326 fold may be structurally N/A here).
Candidate divergence sources to split FIRST (predictions
before cells):
- the ORDER of window re-admissions (winacc_step's
  (false,true) admit/revive, D-154 wa_* pins) vs upd entries
  vs same-epoch fresh inserts;
- ts-changing updates that move an event ACROSS the window
  boundary within the churn (w6/w7 covered single-event
  minimal forms — the blobs have MULTIPLE events + advances
  BETWEEN ops);
- the D-112 eager eviction interleaving with queued upd
  entries (the FLIP-FLOP ZONE — cf1x65/cf1x233/df_* pins;
  D-083: do NOT hand-tune; extend model_check_stream if the
  law needs a model).

## METHOD (the D-318/D-326 playbook)

1. Decode ONE blob end-to-end BY HAND first (x184 or x239 —
   the newest, likely smallest). Full firing timelines both
   sides (the harness `oracle`/`run` JSON firings), the E-type
   window states at the fork, which events are in-window, which
   op produced each element. The D-326 hand-decode found the
   law before any cell ran.
2. Delta-minimize if the decode stalls: greedy rule/fact/epoch
   deletion, predicate `'FAIL' in out and 'errored' not in out`
   (script pattern in the job tmp of the old session — rebuild:
   ~30 lines around `seine-harness diff`).
3. Grid with clock cells: the event JSON idiom is
   {"name":"E","fields":[{"name":"ts","type":"i64"},...],
   "event":{"timestamp":"ts","expires_ms":N}}; epochs carry
   {"op":"advance","ms":N} + update/delete by target index +
   "facts" (applied AFTER actions); collectList over a
   3-value tag pool for duplicate pressure. Runner pattern:
   extract Collections from firing matches (see the D-324/326
   PINS one-liners). Oracle 3× per cell.
4. Predictions REGISTERED in this file's PINS.md before every
   round. Engine port only after a coherent law; the D-154/
   D-160/D-137/D-139/wa_*/D-185/D-112 pins are the adjacent
   certified surfaces — if the law demands touching the
   flip-flop zone, STOP and extend model_check_stream instead
   (D-083, re-proven twice).

## VERIFICATION SET WHEN A PORT LANDS

The 5 members must flip PASS; the full pr_co_* lane (40+ cells)
+ pr_jw_*/pr_cep_df_* must hold; byte gate expected-divergence =
exactly the flipped members. Full battery (values as of
a72cef9): make diff 1856 PASS (11/1431/414) + drift 50; lint
2260/0/0; cargo 73; maturin FROM bindings/ (cwd persists —
return to repo root; .venv/bin/python from root only) + pytest
257; demo True; model_ird 31/31 (subshell cd
probes_pending/tms_envelope); IRD 0-div ×5 (7001/7002/6001/
6003/9001, tools/fuzz_tms_ird.py 150 N); SD census 72 EXACT ×12
(tools/fuzz_tms_sd.py 150 N, seeds 7001,7002,6001,6003,
7004..7011 → divergents 6,10,3,5,6,5,5,6,8,7,4,7 — NEVER
rebuild mid-census); agenda_open ×10 byte-identical ×3
(probes_pending/agenda_open/, debug + release + pre-edit
worktree); fresh fuzz 2×2000 (./target/debug/seine-harness
fuzz 2000 N — next seeds 327001/327002); fuzz_cep 3×300
(.venv/bin/python tools/fuzz_cep.py 300 N — next seeds
327901+; no bank suppression, known-class finds RE-REPORT:
bisect via a worktree at the pre-edit commit, `git worktree
add wt_preNNN <sha>` + release build; sed the wt name into
the bytegate script BEFORE launching — jobs tmp bytegate2.sh).

## REPO STATE (2026-07-18)

Pushed through e5fba33 (no tag). LOCAL unpushed: caedbbf
(D-325 af_linger — the g25 corner) + a72cef9 (D-326 the
identity-fold). CHANGELOG Unreleased carries FIVE user-visible
fixes (focus preemption, ndne phantom, collectList removal
order, eager-linger preemption, exit+re-entry fold) — a "bump
tag one point, and push to release" may arrive; flow: CHANGELOG
→ versions (Cargo.toml + bindings/pyproject.toml) → maturin
build --release from bindings/ → unzip wheel .so over tracked
bindings/python/seine_rs/_native.abi3.so (maturin develop
CLOBBERS the tracked .so — git checkout it before commits!) →
fresh-venv sanity (to_pylist, no polars) → commit "vX: version
bump" → lightweight tag → push main+tag; publish-crates ALWAYS
red (Bryan's crates.io TP, standing). Standing ledger beyond
this round: 662607 collectSet sub-item; cf325901x52 (agenda-
order, unexplained); ?query justifiers (honestly unprobed
wall); crates.io TP; windowed average_exact authoring fence.
Doctrine: probe-first, predictions before cells, oracle
decides, engine ports gate with Bryan (recent slabs: an
unscoped item-directive covers measure-and-land), commit per
green slab, NEVER push without Bryan's word.
