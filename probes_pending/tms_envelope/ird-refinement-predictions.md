# I-RD refinement + third-mechanism splitter predictions
# (logged BEFORE the runs)

Two cell programs from D-204's dump-reads: the 8757 multi-stated
refinement (r-cells) and the same-batch self-break landing splitters
(m-cells). Plus one pre-registered CONFLICT the m-cells must carry.

## ⚠ THE 2442 CONFLICT (pre-registered before any cell runs)

fz_min_42_2442 (certified regression, engine byte-matches) is the
SAME same-batch self-break shape as the D-204 witness trio: RHS =
insertLogical(belief) then break-own-premise, one TMS batch. Its
pinned oracle behavior: the HIGHER-salience observer act FIRES on
the belief before the retract (the engine's current_act lazy
exclusion exists because of it). The witness trio shows NO act ever
firing. All four scenarios have observer salience ABOVE the
justifier (2442: 7>0; 2956: 5>0; 1591: 10>8; 5988: 5>0) — salience
does not separate them. The two remaining concrete axes:
- **FORM**: 2442 breaks via a modify-BLOCK; all three witnesses via
  setter + update() call.
- **SELF-JOIN**: 2442's justifier binds the modified fact TWICE
  ($a : T2(), $b : T2(f1==false), one fact) — the tuple contains
  the broken fact twice; the witnesses bind it once.
The m-cells isolate each. If NEITHER axis flips the behavior, the
conflict is unresolved — STOP and dump both corners (2×2 discipline)
before any model/port work.

## The m-cells (same-batch self-break landing)

Observable: does ROBS fire on the belief value "v" (T3 for m3)?
Scaffold: observer salience ABOVE the justifier (witness-faithful).

| cell | shape | oracle pred | engine pred |
|------|-------|-------------|-------------|
| m0_nobreak_ctl | insertLogical(v), no break | ROBS(v)=1, v in finals | same (converged) |
| m1_samebatch_self_update | insertLogical(v); setF0; update($t) — the witness minimal | **ROBS(v)=0**, no v (HIGH — three witnesses) | ROBS(v)=1 (lazy) |
| m2_samebatch_self_modify | as m1 but modify-block | FORM-IRRELEVANT ⇒ 0; FORM-AXIS ⇒ 1. Lean: 0 (modify compiles to update) | 1 (lazy either way) |
| m3_samebatch_selfjoin_modify | 2442 verbatim minimal (self-join justifier, modify, ROBS sal 7) | **ROBS(T3)=1** (the 2442 pin MUST reproduce; 0 ⇒ pin stale — STOP) | 1 (certified) |
| m4_laterbatch_foreign_update | belief batch 1; foreign RU (sal 10, arm-consumed) updates the premise later; ROBS sal 0 queued | 0 (D-076 eager-unmatch names "DELETE or alpha-breaking UPDATE ... inside the same WM action"; t5 pinned the delete flavor) — MEDIUM | 1 (the engine keeps update-sourced breaks lazy) — if oracle=0 this cell is a NEW divergence witness |
| m5_samebatch_self_del | insertLogical(v); delete($t) — delete-sourced same-batch self | 0 (flush-eager like m1) — MEDIUM (source axis untested) | 1 (current_act exclusion) |

Outcome mapping:
- m1=0, m2=0, m3=1 ⇒ the SELF-JOIN (tuple containing the broken
  fact twice) is the axis that preserves 2442's lazy landing; form
  irrelevant. Port scope: same-batch self-breaks land in-flush
  EXCEPT the self-join shape.
- m1=0, m2=1 (⇒ m3=1) ⇒ the FORM (modify vs update) is the axis.
  Port scope keys on the update route.
- m1=0, m2=0, m3=0 ⇒ 2442's pin does not reproduce — its
  certification is stale or rides an unidentified detail; STOP,
  re-dump 2442 itself, re-open the current_act exclusion's basis.
- m1=1 ⇒ the witness read is wrong (the trio's cancellation rides
  something my minimal lacks — e.g. the deleter walls); rebuild
  toward a witness shape ingredient-by-ingredient.
- m4=0 ⇒ update-sourced FOREIGN breaks land eager too (the engine's
  blanket-lazy update path is wrong beyond same-batch; new witness).
  m4=1 ⇒ foreign-lazy certified by cell; port scope stays narrow.
- m5 mirrors m1 on the delete source; m5=1 with m1=0 ⇒ source axis.

## The r-cells (the 8757 multi-stated refinement)

Key composition at kill time: stated-born key, belief non-WM
sibling, TWO stated WM handles (s1 initial fact, belief via RJ sal
18, s2 via RS2 sal 16 — mirroring 8757's arrival order s1, b, s2).
RD (sal 5) value-deleter; ROBS (sal 0) bare counter.

| cell | reading | RD count | ROBS(v) | finals(v) |
|------|---------|----------|---------|-----------|
| r1_multistated_kill | **R-FIRST** (8757 read: first stated kill ⇒ key dies WHOLE, orphan s2 + unstage b in one event; orphan x1-undeletable; b's ROBS act survives per the dynamic law) | 3 (one a no-op) | **2** (s2 alive + b's surviving act) | **1** (the orphan) |
| r1 alternative | **R-LAST** (the pre-8757 assumption: unstage waits for the last stated; no orphan) | 3 | 1 (only b's surviving act) | 0 |
| r2_two_stated_ctl | control: two stated, NO belief — key loses handles one at a time, both deletes land | 2 | 0 | 0 |

r1 discriminates R-FIRST vs R-LAST by ROBS(v) 2-vs-1 and finals
1-vs-0 (RD=3 both ways; robust to which stated dies first). r2
guards the scope: if r2 shows a no-op delete + survivor (ROBS=1,
finals=1), the key-death-whole event is belief-INDEPENDENT (any
multi-handle key) — a much wider law than the 8757 read.
Engine (diagnostic only): expected R-LAST-like minus the survive
(ROBS 0, finals 0, RD 3) — LOW confidence given the 8757 R4×2
anomaly; the cell gives the clean shape for port-time recon.

Bar: oracle 3× identity-stable per cell; engine 1× diagnostic;
predictions above are the pre-registration record.

## Round 2 (logged BEFORE the m6/m7 runs): isolating the self-join axis

Round-1 results: r1 = R-FIRST exactly (2/3/1; engine 0/2/0 = the
8757 anomaly reproduced minimal); r2 control holds (2/0/0 both);
m0 converged; m1=0, m2=0 (form-irrelevant), m3=1 (2442 reproduces),
m4=0 CONVERGED both sides (foreign row: the engine is already
right — its lazy path never covered foreign breaks), m5=0/1
(source-irrelevant oracle-side; engine lazy on delete-self too).

The m1=0,m2=0,m3=1 row points at the SELF-JOIN — but m3 is
2442-VERBATIM, differing from m2 in types, saliences (justifier 0
vs 20, observer 7 vs 25), a two-setter modify, and the join shape.
Per the 2×2 discipline, isolate the join axis in the SAME
vocabulary as m1/m2:

| cell | shape | pred if SELF-JOIN axis | pred if 2442-other |
|------|-------|------------------------|--------------------|
| m6_samebatch_selfjoin_modify_iso | m2 + a second T0 binding ($a : T0(), $b : T0(f0==true), one fact; modify $b) | ROBS(v)=1 | 0 |
| m7_samebatch_selfjoin_update_iso | m6 with setter+update() | ROBS(v)=1 (form already shown irrelevant) | 0 |

- m6=1, m7=1 ⇒ SELF-JOIN axis pinned in-vocabulary; the exception
  is join-shape-keyed, form-independent (consistent with m1/m2).
- m6=1, m7=0 ⇒ join×form interaction — split further before pinning.
- m6=0 (⇒ expect m7=0) ⇒ the 2442 exception rides something else
  (types/salience/two-setter) — compare m6→m3 ingredient-by-
  ingredient; do NOT pin.
