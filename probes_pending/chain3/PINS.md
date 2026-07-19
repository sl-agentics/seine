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
