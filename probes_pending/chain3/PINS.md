# The cf318902x167 recon (D-338 candidate) — reclassified to the
# D-102 ph-class boundary surface (2026-07-19)

## The witness

cf318902x167 (banked since D-318): ONE adjacent swap in the last
two CH2 firings — engine [E1:616, E1:614], oracle [614, 616];
facts identical.

## Minimization (fork survives every rung)

- m1: CH2 alone (TJ0/TJ1/NE3 dropped) — FORKS. The exists/
  agenda flavor is scaffolding.
- m2: entry-point removed (E2 in MAIN) — FORKS. NOT an
  entry-point composition.
- m3: temporal constraints removed (plain E0() E1() E2() chain)
  — FORKS identically. NOT temporal.
- m5: THE MINIMAL WITNESS — a 2-pattern PLAIN EVENT JOIN
  `E0() E1()`: two E1 rights arrive in epoch 1, the E0 left in
  epoch 2. Engine (644,614) first; ORACLE (644,616) first.

## The composition

ORACLE (both m3 and m5): ARRIVAL-ORDERED right memories + the
standard D-333 staging flips compose the measured orders EXACTLY
(m5: rtm [614,616], leftIns walk fwd, one flip => 616-first;
m3: two hops => 614-first).

ENGINE: the certified D-102 ph-class law — rights staged while
the path is UNLINKED are ph=4 "pre-link" and the fire walk
orders them LIFO — applied here gives exactly the engine's
orders (pre-LIFO [616,614] + flip => 614-first in m5).

## The open refinement (the next probe round)

The D-102 law was certified on shapes where the held rights and
the link trigger share a batch (hw_hb4/hb5, fz_min_1144,
u1c/987). m5's rights arrive in an EARLIER EPOCH — a
fireAllRules boundary lies between the rights and the link
trigger. Candidate refinement: **pre-LIFO applies only within
the link trigger's own batch; rights held across earlier fire
boundaries are ARRIVAL-ordered** (cf. D-081's "re-entries after
an intervening fireAllRules place at the head like any fresh
add"). The probe round: a boundary × batch grid with the D-102
certified cells as the counter-set; the port is on the ph-class
assignment or the fire-walk ordering — gated, byte-gate-decided.

## D-338 round 2: THE BOUNDARY GRID (predictions REGISTERED
## before the cells run)

Machines over the join right-walk order at the link eval:
A (boundary-scoped): held-across-boundary rights ARRIVAL-ordered,
  the trigger batch's own rights pre-LIFO on top;
B (all-FIFO): everything arrival-ordered;
C (the engine today): everything pre-LIFO.
m5 already killed C oracle-side but cannot split A/B.

- b4_certified_control (rights + trigger in ONE post-boundary
  epoch: E1@700, E1@702, E0@744): the D-102 certified shape.
  PREDICT both engines consumption [700, 702] (pre-LIFO walk
  [702,700] + one flip).
- b3_split (E1@614, E1@616 epoch 1; E1@700, E1@702 + E0@744
  epoch 2): A => walk [614,616,702,700] => consumption
  [700,702,616,614]; B => [702,700,616,614]; C =>
  [614,616,700,702]. ORACLE PREDICT: A. ENGINE PREDICT: C.
- b5_two_boundaries (E1@614 epoch 1; E1@616 epoch 2; E0@744
  epoch 3): arrival [614,616] => consumption [616,614] under A
  and B alike (per-boundary sub-LIFO would differ only with 2+
  per epoch — b3 covers that). ORACLE PREDICT [616,614]; ENGINE
  (all-LIFO = same walk [616,614] reversed => [614,616]).

## D-338 round-2 measurements + THE PORT (landed)

b4 AND b3 predictions MISSED (recorded) — the oracle is
ARRIVAL-ordered in EVERY plain shape (machine B; b3 [702,700,
616,614] 3x; even the same-batch control b4). The boundary
hypothesis died with them. The wholesale ph=4 walk flip then
broke pr_nl_m4_stream_epochs (byte gate caught it — the one
certified mover) — and that mover DECODED the true law:

**Drools' per-insert stream force-flush rides EVENT-TYPED inputs
(isStreamMode is a node/type property): event rights reach the
memory in ARRIVAL order even while the path is unlinked; PLAIN
facts in a stream session accumulate and keep the certified
pre-LIFO walk** (pr_nl_m4's [2,4,1], nl5, m3b/x52 — all plain-P
shapes — stay exactly as certified).

THE PORT: held EVENT rights stamp ph=5 (plain keep ph=4); the
mixed walk = ph4 LIFO first, then {ph5, ph0} together .rev() =
global arrival (the LIFO-built list reversed); the stash-
exemption has_ph4 check includes ph=5 (hold semantics untouched
— only the walk order changed). MEASURED: b3/b4/b5 + x167_m3/m5
ALL PASS; pr_nl_m4 + nl5 hold; byte gate vs a303f3b 2481/2483 —
the ONLY diffs are the two lane witnesses; corpus 11/1504/414 +
drift 19 identical (x167's engine bytes unchanged — its residual
is the TEMPORAL branch); lint 2357; SD census 71 EXACT; cargo
74; pytest 257; demo True; model_ird 31/31; IRD 0-div x5; agenda
x10 x3; fuzz 2x2000 (338001/338002) + cep 3x300 CLEAN.

RESIDUAL (x167 keeps its xfail seat): the same adjacent swap
through the TEMPORAL join branch (m1/m2 — the per-fact-AB phased
temporal insert split has its OWN walk; D-125-certified surface).
The next round extends the arrival law to temporal held-event
rights with the jr/TJ pins as the counter-set. Five graduations:
pr_ch_b3_split, pr_ch_b4_certified_control,
pr_ch_b5_two_boundaries, pr_ch_x167_m3, pr_ch_x167_m5.

## D-338 round 3: the temporal residual — SCOPED, not yet ported

tm5_temporal_held (2-pattern temporal `E0() E1(before)` with the
held-rights epoch split): engine==oracle ALREADY — graduated
pr_ch_tm5_temporal_held. The basic temporal held case is healthy.

The m1/m2 carrier is NOT do_node's temporal else-arm: an
instrumented stamp-loop print never fired on either witness —
the (E0,E1) children reach the second temporal join through the
D-125 v2 FLUSH-MODEL path (the per-arrival cascade outside
do_node's staged walk). The residual round therefore lives on
the D-125 lane: the intermediate-tuple order INTO a downstream
temporal join within one arrival cascade (oracle wants the
staging flip the plain branch has; the engine's cascade hands
them over un-flipped — m1/m2's stable adjacent swap). Counter-set
for that port: the jr pins + the D-125 flush-model cells + the
TJ corpus. Needs its own session; cf318902x167 keeps its xfail
seat until then.

# The D-125 cascade round (2026-07-19) — the x167 residual port

## Round 4: LOCATE — the code-trace composition (predictions
## REGISTERED before the instrument runs)

The candidate site fell out of the code read, not the instrument:
the D-125 flush loop (engine.rs ~7331) drains join1 via
`flush_ins_delta` (eligible: unshared, Sink::Node) and routes
`trg` into join2's `s_left` via `append_into_pending` — then
join2's OWN iteration (ascending ni) picks an arm:

- `flush_ins_delta` walks staged lefts HEAD-FIRST
  (`s0_folds.chain(lins.iter())`) = the prepend-built list = ONE
  staging flip — the oracle's composition;
- `self_drain_delta` walks `.rev()` = arrival order, NO flip.
- The RIGHT walk is `.rev()` in BOTH arms — the fork is
  exclusively the LEFT-side cascade batch.

The arm gate (eligibility, engine.rs ~7387): join2 is
Term-sinked, so `has_l && rights_is_empty()` must hold. But m2's
join2 rights memory still holds the three seeded E2 CORPSES
(@35/@31/@28, expired at the epoch-1 advance): their expiration
dels batch to the fire, and the UNLINKED rule never evaluated —
the memory never emptied. Ineligible → the else-arm
`self_drain_delta` → un-flipped.

PREDICTIONS (the m2 instrument, engine debug build):
- P1: join1's arm = flush_ins_delta; trg emission order
  (644,614) then (644,616); trg head-first [(644,616),(644,614)].
- P2: join2's s_left after the handoff = [(644,616),(644,614)]
  head-first.
- P3: join2's rights memory at E0@644's flush = the 3 corpse
  E2s (non-empty) → Term-sink eligibility FAILS.
- P4: join2's arm = self_drain_delta; lseq stamp order 614-child
  FIRST (the .rev() walk) — the measured engine order.

Reachability note (the port's scope argument): children
cascading into join2 with LIVE rights ⟹ every pattern populated
⟹ rule LINKED ⟹ the eval walk consumes (not this loop). The
loop sees a cascade batch at an ineligible Term-sink join ONLY
when the downstream right memory is corpses-or-empty. Candidate
port: the Term-sink `has_l` arm treats ALL-EXPIRED rights as
empty (flush_ins_delta then drains with the flip and emits
nothing — corpse rights make no NEW pairs, D-102).

## Round 4 instrument MEASUREMENTS: 4/4 predictions HIT

m2 debug run (fids: E1@32=0, E2@35/31/28=1/2/3, E1@614=4,
E1@616=5, E0@644=6, E2@648=7):
- P1 ✓ join1 (ni=0) eligible, trg head-first [[6,5],[6,4]].
- P2 ✓ join2 (ni=1) s_left after handoff [[6,5],[6,4]].
- P3 ✓ join2 rights_mem=[1,2,3] (the corpses) → eligible=false.
- P4 ✓ arm=self_drain, lefts_mem_after=[[6,4],[6,5]] (614-child
  first) → consumption 616-first = the measured fork.

Linking mechanics confirming the scope argument (pos_linked,
engine.rs ~9694): linked-ness gates on the ALPHA active set,
which empties EAGERLY at expiration-flag time, while the beta
rights memory keeps corpses until the quiescence-lazy
retraction. Linked ⟺ all alphas hold live facts — so the
ineligible-cascade class ⟺ all-corpse (or empty) right memory.
The asymmetry (eager alpha unlink vs lazy beta corpse) IS the
reachability of the broken arm.

MODEL-CHECK CALL: no separate model round — the site's semantics
are pure list mechanics (both arms' walk directions read
directly off the code, 4/4 instrument predictions), and the
shared-surface question is empirical: the byte gate enumerates
every certified cell whose behavior the widened gate touches;
oracle-diff every mover. The pin ladder is the certification.

## Round 5: the pin ladder (predictions REGISTERED before any
## cell runs; each oracle measurement 3x)

The port under test: the Term-sink `has_l` eligibility arm
widens from `rights_is_empty()` to "rights memory admits no NEW
pairs" = all entries expired (empty iterator ⇒ all() true, so
the old condition is subsumed). The has_r arm stays: with
all-corpse lefts both arms push rights .rev() identically and
corpse-left partner scans skip — behaviorally a no-op (recorded,
not ported).

- lc1_linked_live_e2 (E2@620 arrives in EPOCH 1, live at E0@644;
  no epoch-2 E2): the rule LINKS at E0's insert → the eval walk
  (do_node temporal arm, tm5-certified) composes everything.
  PREDICT: oracle 614-first [E0@644,E1@614,E2@620] then 616;
  engine SAME already (pre-port PASS — linked-path control).
- lc2_three_e1s (m2 + E1@618 "w" in epoch 1): the N=3 cascade
  batch. Oracle: join1 rights [614,616,618] → cascade prepend →
  join2 staged [618,616,614] → head-walk lseq → E2 partner scan
  → emissions 618,616,614 → terminal flip → consumption
  614,616,618. Engine pre-port: self_drain .rev() → lseq
  614,616,618 → consumption 618,616,614 (the FULL REVERSAL, not
  an adjacent swap — the N=3 discriminator). Post-port: engine
  = oracle.
- lc3_second_plain (E2 pattern loses its temporal constraint;
  join2 PLAIN, join1 temporal): the D-125 loop skips plain
  join2; the children HOLD in s_left; at E2@648 the linked eval
  walks staged lefts head-first (certified plain surface).
  PREDICT: both 614-first (pre-port PASS — control).
- lc4_first_plain (E1 pattern plain, E2 keeps after[0,50] E1;
  join1 plain, join2 temporal): plain join1 doesn't cascade in
  the D-125 loop; children form at the linked eval (E2@648) and
  hand into join2's temporal do_node arm (tm5-certified).
  PREDICT: both 614-first (pre-port PASS — control).
- lc5_e2_before_e0 (epoch 2 order [E2@648, E0@644]): at E2's
  insert join2 is has_r with EMPTY lefts memory → already
  eligible → arrival drain (live); at E0's insert the rule links
  → eval path. PREDICT: both 614-first (pre-port PASS —
  control).
- The witnesses m1/m2: oracle 614-first (measured 3x in the
  D-318 era + round 1); engine 616-first pre-port; post-port
  PASS.

## Round 5 MEASUREMENTS: 10/10 ladder predictions HIT + THE PORT

Oracle (3x each, all stable): lc1 [614,616]; lc2 [614,616,618];
lc3 [614,616]; lc4 [614,616]; lc5 [614,616]. Engine pre-port:
lc1/lc3/lc4/lc5 PASS (the controls); lc2 [618,616,614] — the
FULL REVERSAL, confirming one missing flip on the whole batch
(not an adjacent-swap artifact). The fork is confined to the
ineligible-cascade class exactly as scoped.

THE PORT (engine.rs, the D-125 loop's Term-sink eligibility):
the `has_l` arm widens from `rights_is_empty()` to "every right
expired-flagged OR dead" (`n.rights_ids().all(is_expired ||
!is_alive)`; empty ⇒ all() true subsumes the old condition).
flush_ins_delta then drains the cascade batch through its
head-first walk (the flip) and emits nothing.

ONE RECORDED MISS on the port mechanism: the first gate attempt
used `is_expired` ALONE and did NOT move the witnesses — the
instrument showed eligible=false still. The corpses are DEAD,
not expired-flagged: `kill()` CLEARS the expired flag when the
epoch-1 quiescence retracts the due events, while the unlinked
join's beta memory keeps the entries (the lazy route-delete
never ran). The reachable corpse-only state is dead-lingering,
so the gate needs expired-OR-dead. (Join1's E1@32 corpse never
exposed this: its exclusion in the trg was the [0,50] temporal
CONSTRAINT, not the corpse skip.)

Dead-right pairing note (recorded, not ported): `allowed` and
flush_ins_delta's corpse skip check `is_expired` only — a
dead-lingering right that passes constraints would emit a
phantom child. This is PRE-EXISTING behavior shared with the
always-eligible Sink::Node arm (m2's join1 holds dead fid0
throughout) and downstream firing filters drop dead-member
tuples; the widened gate routes no new semantics — the new
cases behave exactly like every other flush_ins_delta call.
The has_r symmetric arm stays unwidened: both arms push rights
`.rev()` identically and corpse-left scans skip — a no-op.

POST-PORT: cf318902x167 + m1 + m2 + all 5 ladder cells PASS
engine-vs-oracle (full diff, 8/8).

## The D-339 port receipts

Byte gate vs 83c09d9 (wt_pre339, release both): 2486/2489 SAME,
0 moved, 3 diff — the diffs are EXACTLY the three would-graduate
witnesses (x167_m1, x167_m2, cf318902x167), all oracle-PASS.
Zero certified movers: the widened gate touches nothing else in
the corpus.

GRADUATIONS (8): cf318902x167 → pr_ch_cf318902x167 (the last
banked witness of the D-318 family), x167_m1/m2 → pr_ch_x167_m1/
m2, the five ladder cells → pr_ch_lc1_linked_live_e2 /
lc2_three_e1s / lc3_second_plain / lc4_first_plain /
lc5_e2_before_e0. Drift bank REBANKED 19→18.

Battery: make diff 11/1518/414 + drift 18 identical; lint
2366/0/0; cargo 74; pytest 257 (tracked .so restored); demo
True; model_ird 31/31; IRD 0-div x5 (7001/7002/6001/6003/9001);
SD census 12x150 = 6,10,3,4,6,5,5,6,8,7,4,7 = 71 EXACT;
agenda_open x10 identical x3 (release/debug/wt_pre339); fuzz
2x2000 seeds 339001/339002 CLEAN (0 divergences, 0 xfail) +
fuzz_cep 3x300 seeds 339901-903 CLEAN. NEXT fuzz seeds 340001+.
Arc scorecard: rounds 4+5 = 14 predictions, 14 hits, 1 recorded
port-mechanism miss (the is_expired-only gate — dead-not-flagged
corpses). CHANGELOG Unreleased now carries BOTH entries (the
D-338 event-arrival fix + this round's cascade-order fix).
THE COLLECT/CHAIN TEMPORAL-ORDER LEDGER IS EMPTY — cf318902x167
was the last banked witness of the D-318 family.
