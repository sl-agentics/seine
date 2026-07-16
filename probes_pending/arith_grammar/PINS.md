# Arithmetic-grammar probe pins (oracle: Drools 9.44.0.Final+p1, 3×-stable)

The boundary-redraw arc's step 0 (Bryan's doctrine amendment on the
record: **the match grammar never grows a Java or MVEL interpreter** —
it may grow certified arithmetic). 25 probes in this directory,
oracle-only (`seine-harness oracle`), each batch run 3× byte-identical.
The engine parses none of these DRLs today — that is the point.

## A. RHS arithmetic (insert args): CLEAN JAVA, certifiable

| probe | pin |
|---|---|
| ar_rhs_insert_arith | `insert(new U($a / 2))` with a=7 → **3** (Java int division); `$a + 1` → 8 |
| ar_rhs_long_wrap | `$a + 1` at Long.MAX → **-9223372036854775808** (silent wrap) |
| ar_rhs_more | `-7 + 2 * 3` → -1 (precedence ✓); `-7 % 3` → -1 (dividend sign ✓); `-$a` → 7 |
| ar_rhs_dbl_div | `$a / 2.0` a=7 → 3.5 (mixed promotes) |
| ar_rhs_div_zero | RHS `1/0` → **ConsequenceException** (java.lang.ArithmeticException), batch errors |
| ar_rhs_double_edge | `1.0 / 0.0` → Infinity; the oracle RENDERS it as the JSON **string "Infinity"** (our serializer emits null for non-finite — a rendering pin the port must resolve) |

Verdict: the RHS is javac — deterministic, coercion-free, matching our
kernels bit-for-bit on f64 `+ - * /` and comparisons. i64 overflow
WRAPS (Java) where the derive plane errors — an in-plane divergence the
port must decide (wrap-to-match-oracle inside the match plane is the
byte-certifiable choice).

## B. LHS constraint arithmetic: works, but division is a COERCION SWAMP

Solid ground (all 3×-stable):
- Binding arithmetic works: `k > $a + 1`, `k == $a + $b`, `k == -$a`,
  `k > $a * 2`, `k - 1 == 4` (ar_lhs_binding_arith, ar_lhs_bind_bind,
  ar_lhs_bind_mul, ar_lhs_neg_bind).
- Doubles are IEEE: `0.1 + 0.2 == 0.3` no-fire / `== 0.30000000000000004`
  fires (ar_lhs_ieee_sum); `0.0/0.0 == 0.0/0.0` NO-fire — **NaN != NaN,
  standard IEEE, not totalOrder** (ar_lhs_double_div_zero) — matches the
  derive plane's hand-rolled comparisons exactly.
- Long overflow wraps in constraints too (`k + 1 < 0` fires at MAX,
  ar_lhs_long_overflow).
- `%` is dividend-sign (ar_lhs_rem_sign). Mixed int+double promotes
  (ar_lhs_mixed_promotion). Cross-type `l == d` promotes long→double
  LOSSILY (2^53+1 == 2^53.0 FIRES, ar_lhs_cross_type_eq).

The swamp — division semantics depend on the COMPARAND LITERAL
(ar_lhs_int_div, ar_lhs_div_ctx, ar_lhs_div_ctx_neg; k=7 / k=-7):

| constraint | fired? | implied semantics |
|---|---|---|
| `k / 2 == 3` | YES | integer division (3) |
| `k / 2 == 3.5` | YES | real division (3.5) — **same fact fires both** |
| `k / 2 == 3.0` | YES | integer division widened (3.0) |
| `k / 2 >= 3.5` | YES | real division |
| `k / 2 < 4` | YES | consistent either way |
| `k / 2 > 3` | no | integer division (3 > 3) |
| `-7: k / 2 == -3.5` | YES | real |
| `-7: k / 2 == -3.0` | YES | integer (trunc) — **again both** |

Working hypothesis (fits every cell above): the literal's
int-representability selects integer vs real division. ONE ANOMALY
REMAINS: `k / z > 0` with z=0 FIRES (ar_lhs_div_zero_int — suggests
real division → Infinity) while `k / z == 0` silently no-fires
(ar_lhs_div_zero_eqint — suggests integer division → exception →
false). Unresolved; needs its own 2×2 before any LHS-division port.

Compiler defect — ⚖ Bryan's ruling on the record: **we are NOT copying
the broken order of operations.** Bare `k + 2 * 3 == 13` throws
**ConstraintEvaluationException at EVAL time** (ar_lhs_precedence),
while `k + (2 * 3) == 13`, `2 * 3 + k == 13`, and `k * 2 + 3 == 17`
all evaluate correctly (ar_lhs_prec2) — a self-inconsistent 9.44
defect, not a semantic. Handling is the established defect doctrine
(the accumulate stale-min/max precedent): the engine evaluates the
shape with CORRECT precedence; expected-divergence witnesses of
opposite polarity go to xfail/; the fuzz generator excludes the defect
surface; re-adjudicate against a newer oracle on any bump (it may be
fixed upstream) and draft an upstream report if not.

## The port shape this implies (Bryan-gated, not started)

1. RHS computed insert args first (clean Java; supersedes the D-231
   WONT for pure computed fields on NEW facts only; modify-with-
   computation stays WONT).
2. LHS arithmetic as a COHERENT SUBSET: same-type operands, division
   restricted or comparand-pinned; the mixed/coercion cells FENCED with
   authoring-lint steering (the D-061 closed grammar, narrowed to where
   Drools is self-consistent).
3. Prereq on record before either: the D-076 TMS cascade goes iterative
   (arithmetic unlocks unbounded justification chains).
4. Rendering: oracle emits Infinity/NaN as JSON strings in fact output;
   our serializer emits null — must be pinned before any byte gate.
