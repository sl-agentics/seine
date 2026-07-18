# BigDecimal inline-arithmetic pin campaign (D-308) — predictions FIRST

Oracle: Drools 9.44.0.Final+p1 (pinned; -Xss1g). Step 0 taught the
OracleRunner decimal(p,s) → BigDecimal (exact string construction both
ways; render = scale-preserving toString). All probes oracle-only,
3×-stable (volume cells 5×). Predictions pre-registered below BEFORE
any run; measurements land in the tables after.

## Predictions (pre-registered 2026-07-18)

- **A. RHS javac** (`insert(new Out($p + $f))`, computed setter args):
  BUILD ERROR — Java has no operator overloading on BigDecimal; the
  RHS is javac. Confidence HIGH. If this holds, the engine's RHS
  decimal wall is ERROR-PARITY (stronger than a scope cut).
- **B. LHS MVEL + - ***: WORK, exact BigDecimal arithmetic; `==` on
  computed results behaves like compareTo (scale-insensitive:
  `1.10 + 2.20 == 3.3` FIRES). Confidence MEDIUM-HIGH.
- **b5 THE DISCRIMINATOR** (`amount + 0.10 == 3.40`, amount 3.30):
  MVEL types 0.10 as Double, then promotes via
  BigDecimal.valueOf(double) (string path → exactly 0.1) → FIRES.
  The alternative (new BigDecimal(double), raw binary) → NO FIRE.
  Confidence LOW — this cell decides the literal-promotion law.
- **C. Division**: terminating divides WORK; NON-terminating (1/3)
  throws ArithmeticException("Non-terminating decimal expansion")
  surfacing as a scenario error (BigDecimal.divide unguarded);
  zero divisor throws at fire time. Confidence MEDIUM.
- **D. The jit axis** (5000-fact populations, the D-290 race): + - *
  agree between modes; DIVISION is the suspect cell — if the jitted
  path coerces to double or supplies a different MathContext, volume
  runs diverge from mode-1 and/or go run-nondeterministic.
  Confidence LOW — this axis is the campaign's reason to exist.

## Measurements

(recorded after the runs)

## Measurements (2026-07-18; three rounds + volume, all 3×-stable, volume 5×)

### A. RHS (javac) — PREDICTION HELD (HIGH)
`insert(new Out($p + $f))`, `insert(new Out($p + 1))`,
`modify($m){ setAmount($p + $f) }` → ALL kbase BUILD ERRORS.
**Seine's RHS decimal-arithmetic wall is ERROR-PARITY, not a scope
cut.**

### B. LHS `+ - *` — exact BigDecimal, compareTo comparisons
- `a + b >= 3.30` fires (1.10+2.20); `fee - amount`, `a * b` compute
  exactly.
- `==` vs BigDecimal FIELDS: scale-INSENSITIVE (compareTo): computed
  3.30 == t2(3.30) AND == t4(3.3000) both fire; 1.10*3.00 (scale 4)
  == t2(3.30) and t4(3.3000) both fire.
- `==` vs INT literals: compareTo (a - b == 0 fires on 0.00).

### C. THE DOUBLE-LITERAL POISON — the campaign's headline
- Double LITERALS coerce RAW-BINARY: `a + b == 3.30` NEVER fires on an
  exactly-3.30 result (also 3.3, 2.2, 1.10, 4.30 — every == cell of
  round 1). Boundaries are ASYMMETRIC: `a + b >= 3.30` FIRES while
  `a + b <= 3.30` does NOT, on the same exact-equal boundary
  (literal 3.30 → 3.2999999999999998…).
- Double FIELDS coerce VALUE-FAITHFULLY (toString path): `a + b == d`
  with d=3.3 fires; `a / b == d` with d=0.3333333333333333 fires.
  LITERAL vs FIELD behave DIFFERENTLY inside the same operator.

### D. `/` and `%` — silent degradation to IEEE double
- 1.00/3.00 evaluates (no non-terminating throw — the divide is NOT
  exact BigDecimal); result compares as the IEEE double (== double
  field 0.3333333333333333 fires; == decimal(20,16) 0.3333333333333333
  does NOT — compareTo against the true double's raw binary).
- 4.40/2.20 == 2 fires (double 2.0); 5.50 % 2.00 == 1.5/1.50/t16(1.5)
  all fire (double 1.5, exactly representable). -5.50 % 2.00 == -1.5
  (dividend sign).
- ZERO divisor: ConstraintEvaluationException at eval time (a runtime
  scenario error, not a build error).

### E. Volume / the D-290 jit axis — CLEAN
5000-fact populations, 5 runs each: fire COUNTS and firing SEQUENCES
byte-identical across all runs (bd_vol_add 2550 — reproducing the
raw-binary literal boundary at volume, so both modes share the
poison; bd_vol_div 2500). The only run-to-run jitter is the WM-dump
order (session.getObjects hash iteration; canonicalized in the diff
pipeline; not semantic). NO mode divergence observed on decimal
cells — unlike D-290's int-division cliffs.

## The port recommendation (Bryan's gate)

1. RHS wall: KEEP — upgrade the recorded rationale to ERROR-PARITY.
2. AGREE-SUBSET candidate (the D-291 shape): LHS `+ - *` over decimal
   fields, comparisons restricted to DECIMAL operands and INT
   literals — compareTo semantics, exact, volume-stable, and the
   engine's existing i128 machinery computes it (add/sub → max scale,
   mul → s1+s2; results are never scale-compared, so representation
   is free).
3. FENCES (loud, steering): double literals AND f64 fields in
   decimal-arithmetic comparisons (the poison cells — composing
   naturally with ruling 4's existing decimal-vs-float wall: "money
   never meets floats" now provably includes "…because the oracle's
   own coercion is poisoned"); `/` and `%` on decimals (silent
   double degradation is the exact thing the money doctrine forbids —
   the oracle here is the anti-spec, and ledger row 4's loud-error
   doctrine wins).
4. No mode-1 residency precondition needed (volume agreed), recorded
   with the absence-of-evidence caveat.
