# PINS — D-345: fz_342002_1206 (Bryan named the witness) — the
# focus-pop preemption race

2026-07-19. The fork (re-measured): ONE adjacent swap, firings
7/8 — oracle R0(zz) then R2(""); engine R2("") then R0(zz).
Firing SET + facts identical. Composition decode: firing 6 =
R2(zz) does setFocus(gb) + modify(f2:=true); the modify births
R0(zz)'s re-activation (R0 binds $f2 → listens f2; R3 binds
f0/f1 only → correctly silent); gb is EMPTY at the push → pops
→ the engine RESUMES the in-progress R2 executor; Drools
re-selects by (salience, loadOrder) — R0 (load 0) preempts R2
(load 2). The D-333 eq-load-order tie composed ACROSS a focus
push/pop — the halt-check preemption ran against gb (empty),
not MAIN's refreshed queue. TMS/insertLogical/R1/T0 =
predicted scaffolding.

## The grid (predictions REGISTERED before any cell runs;
## oracle 3x)

Shape: A (load 0, sal 0): T1(f0 contains "zz", $x : f2) — the
modify-reborn rule; B (load 1, sal 0): T1(f2 == false) →
setFocus(g) + modify f2:=true; C (load 2, group g): T1() → end.
Facts: T1("zz",F), T1("x",F), T1("y",F) — B has 3 acts; after
B(zz)'s modify the race is A(zz-reborn) vs B(y-remaining).

- mz1_minimal: PREDICT (high) the witness fork reproduces
  scaffolding-free: oracle ...B(zz), A(zz), B(y); engine
  ...B(zz), B(y), A(zz). (TMS + R1 + the second focus cycle are
  scaffolding.)
- mz2_nofocus (B does NOT setFocus; C in MAIN at sal 6):
  PREDICT (high) MATCH both sides — the certified D-333
  eq_decl_preempt path handles the modify-birth race when no
  focus push intervenes. Isolates the FOCUS as the ingredient.
- mz3_empty_group (C constrained to never match — the push is
  a pure push/pop of an empty group): PREDICT (med) the engine
  STILL forks (the defect is the pop-RESUME skipping the
  preemption re-check, not C's firings); a MATCH here would
  relocate the defect into C's firing window.

## Grid MEASUREMENTS (2026-07-19, oracle 3x each)

- mz1_minimal: MATCH both sides — the HIGH prediction MISSED
  (recorded). A real focus cycle (C fires, g drains, pop)
  re-selects correctly; the witness's FIRST focus cycle was
  never the carrier.
- mz2_nofocus: MATCH (hit) — the plain D-333 path is healthy.
- mz3_empty_group: FORKS (the med prediction HIT): oracle
  [A, B(zz), A, B(x), B(y)]; engine [A, B(zz), B(x), B(y), A].
  THE INGREDIENT IS THE EMPTY PUSH: setFocus of a group with no
  live activations → the D-258 late-continue keeps l past an
  equal-salience DECL-PRECEDING own-group member born from this
  firing's modify. Matches the witness exactly (its SECOND
  setFocus pushes gb when R3 has no fresh acts).

## THE NAIVE PORT — MEASURED AND REVERTED (the D-331 protocol)

The candidate gate (own_tie: equal-salience decl-preceding
queued member of l's OWN group blocks the late-continue) fixed
the witness + mz3 but the byte gate measured TWELVE certified/
failures-tier movers — fz_9001_1795/2060/2573, fz_9002_1011/
5814, fz_9003_2151/2190/4514/486, pr_af_fz_9102_7658,
pr_af_g7_tie_hfirst, agenda_open/fz_9003_6467 — ALL oracle-diff
FAIL with the gate in. ENGINE REVERTED (byte gate 2517/2517
IDENTICAL post-revert). The D-106/D-258/D-320 halt surface is
an EMPIRICALLY CALIBRATED approximation (the 88-witness halt
matrix, 10 configs, "every blocker-pool variant measured
WORSE") — a bare own-group tie check is too coarse: those
witnesses contain equal-salience decl-preceding queued members
whose certified behavior is the CONTINUE.

REFINED HYPOTHESIS for the next round (unverified): the mz3
discriminator is FRESHNESS — the preempting member's activation
is born DURING l's firing (the RHS modify), where the halt
matrix's continue shapes carry members queued BEFORE l's pick
(which took l as best over them... note the pick-order argument
says an equal-sal lower-decl QUEUED member could not have been
passed over — so the matrix members are likely queued-DIRTY
empty-at-pick or born under different focus tops; the model
must say). NEXT-SLAB REQUIREMENTS (its own slab, model-first —
the D-333 model_check pattern): enumerate candidate halt laws
(freshness-gated tie, dirty-stamp windows, af_live composition)
against the FULL halt-matrix population (the 88 witnesses + the
12 movers + mz1-3 + the fz_342002_1206 witness) before any
engine edit. fz_342002_1206 KEEPS ITS XFAIL SEAT.
