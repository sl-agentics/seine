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
