# PINS — exact decimal average (D-314; Bryan: "do the exact decimal
# average slab, half_up default")

Predictions registered 2026-07-18 BEFORE any cell ran.

## THE DESIGN QUESTION AND WHY THE ORACLE CAN STILL DECIDE IT

Drools has NO decimal average — its `average` is IEEE double
(D-098 pin J, certified). So `averageExact` is engine-native with no
standard DRL spelling and can never enter the auto-diff corpus. The
semantics we implement are java.math's:

    result = sum.divide(BigDecimal.valueOf(count), scale, mode)

and THAT is expressible in raw Drools: a multi-function accumulate
`$s : sum($x), $c : count()` plus an RHS
`$s.divide(new java.math.BigDecimal($c), SCALE,
java.math.RoundingMode.MODE)` insert. The campaign runs the SAME
fact vectors through both spellings — the oracle's explicit-divide
program and our `averageExact($x, scale, mode)` — and compares the
result values. Neither spelling runs on the other side (our engine
walls RHS method calls; Drools rejects the unknown accumulate
function), so the comparison is value-for-value in this pin round,
not an auto-diff; the graduated protection is engine tests + pytest
(the why()/acc_sources precedent).

## THE GRID

Modes (all of java.math.RoundingMode except UNNECESSARY):
up, down, ceiling, floor, half_up, half_down, half_even.

Vectors (chosen so modes DISAGREE and signs flip):
- V1 half-boundary positive: (0.02, 0.03) → avg 0.025 @ scale 2
  predictions: up/ceiling/half_up 0.03; down/floor/half_down 0.02;
  half_even 0.02 (2 is even).
- V2 half-boundary negative: (-0.02, -0.03) → avg -0.025 @ scale 2
  predictions: up/half_up -0.03 (away from zero); floor -0.03;
  down/ceiling/half_down -0.02; half_even -0.02.
- V3 non-terminating positive: (1.00, 1.00, 1.01) → 3.01/3 =
  1.00333… @ scale 2: up/ceiling 1.01; all others 1.00.
- V4 non-terminating negative: (-1.00, -1.00, -1.01) → -1.00333… @
  scale 2: up/floor -1.01; down/ceiling/half_* -1.00.
- V5 exact division (no rounding): (1.10, 2.20) → 1.65 — ALL modes
  1.65 (the mode must be a no-op when the division terminates at
  scale).

P1: our i128 rounded division equals the oracle's BigDecimal.divide
on ALL 35 grid cells (7 modes × 5 vectors), 3× stable.
CONFIDENCE: high — java.math RoundingMode semantics are documented;
the grid exists to catch OUR sign-handling bugs (negative truncation
in Rust is toward zero, and half-comparisons must be on magnitudes).

P2 (empty/all-null): count == 0 → NO propagation, like `average`
(oracle: the multi-acc spelling still fires with count 0 and RHS
divide would throw div-by-zero — a spelling artifact, not a
semantic: our averageExact follows `average`'s certified
empty-blocks-propagation contract; documented, not diffed).

P3 (null skips): nullable decimal sources skip null contributions in
BOTH sum and count (the D-097 skip is uniform in AccCtx); average of
(1.00, null, 2.00) @ scale 2 = 1.50.

P4 (scale default, authoring): `average_exact(Line.amount)` over
decimal(18,2) defaults to scale 2 (the SOURCE scale) and HALF_UP
(Bryan's ruling); result subset_type decimal(38,2) — results are
never null (like sum, per D-306's result-typing).

## MEASUREMENTS (2026-07-18, same day — nothing below existed before the cells ran)

**P1 HIT: 35/35 grid cells MATCH** — oracle (multi-acc + RHS
BigDecimal.divide, 3× stable per cell) vs engine (averageExact),
value-for-value across 7 modes × 5 vectors. Sign handling exact:
-0.025 rounds away-from-zero to -0.03 under up/half_up, toward zero
under down/ceiling/half_down, floor takes -0.03, half_even takes the
even neighbor -0.02 — identically on both sides. V5 confirms every
mode is a no-op on terminating division.

Engine vectors additionally pin (engine/tests/dec_avg.rs): half_even
parity at 0.035 → 0.04; scale narrowing (0.025 @ scale 0 → "0"
half_up, "1" up/ceiling); scale widening (0.0250 @ 4); P2 HIT
(empty AND all-null block propagation, like average); P3 HIT (nulls
skip both sum and count: avg(1.00, null, 2.00) = 1.50). Walls loud:
i64 source steers to average; unknown mode lists the seven; scale
caps at 38.

Cells stay PENDING by design: the oracle spelling engine-fences on
multi-function accumulate + RHS method calls; the engine spelling is
oracle-unknown. The graduated protection = the vector suite + pytest
(the why()/acc_sources precedent).
