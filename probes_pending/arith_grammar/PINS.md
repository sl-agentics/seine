# Arithmetic-grammar probe pins (oracle: Drools 9.44.0.Final+p1, 3×-stable)

The boundary-redraw arc's step 0 (Bryan's doctrine amendment on the
record: **the match grammar never grows a Java or MVEL interpreter** —
it may grow certified arithmetic). 25 probes in this directory,
oracle-only (`seine-harness oracle`), each batch run 3× byte-identical.
The engine parses none of these DRLs today — that is the point.
(§E adds the 18-probe ar_upd_* round: updates/setters with
computation, the D-231 re-examination.)

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

~~Working hypothesis: the literal's int-representability selects
integer vs real division; ONE ANOMALY REMAINS (`k / z > 0` fires at
z=0, `k / z == 0` silently no-fires).~~ **RESOLVED — superseded by
§F (the ar_dz_* matrix, D-290): there is no integer division in the
interpreted path at all.** The quotient is ALWAYS IEEE double; what
looked like integer division is a Java `(long)` NARROWING CAST
applied at the comparison when the comparand is int-typed/int-valued
((long)3.5 = 3, (long)+Inf = Long.MAX, (long)NaN = 0). Both anomaly
cells fall out of one rule: `k/z > 0` → (long)+Inf = Long.MAX > 0
fires; `k/z == 0` → Long.MAX == 0 is false, silently. And the table
above is MODE-1 ONLY — a second, jitted java mode engages
nondeterministically at evaluation volume (§F).

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

## C. Tier-boundary probes (D-282; the shippability question)

| probe | pin |
|---|---|
| ar_fl_runaway_computed | runaway computed PLAIN-insert chain → oracle errors "fire limit 100000 reached (non-terminating?)" — the D-013/j21 parity clause (both-sides-fire-limit = agreement) already covers the differential; no hang |
| ar_tms_runaway_logical | runaway computed LOGICAL chain → same clean fire-limit error (oracle-side; the unbounded tier stays walled in-engine regardless) |
| ar_tms_computed_dedup | `insertLogical(new U($k + 1))` from two premises = TWO justifications on ONE value-keyed belief: delete premise 1 → U survives (W1 fired); delete premise 2 → U retracts (W2N fired). Computed values are ordinary TMS values |
| ar_tms_computed_cascade | 2-stratum computed chain U($k+1) → V($v+1): deleting the root retracts the whole chain (WGone fired, WAlive silent) |

Tier plan confirmed by probe: Tier 1 (computed args on plain `insert`)
creates no justification chains — no D-076 exposure at all; fire-limit
+ D-117 spin guard govern generation. Tier 2 (computed `insertLogical`
behind a stratification CompileError) keeps chain depth rule-count-
bounded, preserving the recursive cascade's existing safety argument.
Unbounded (cyclic computed logical) waits for the D-076 iterative
rewrite.

## D. Java INT-literal typing (D-284; found by the computed-args fuzz axis)

The fuzz axis found it before any user did: `-1000000007 * 3` in an
insert arg is INT×INT in Java — 32-bit WRAPPING arithmetic — while the
engine computed in i64. The model, pinned by pr_ar_rhs_int_literal_wrap
(all six cells exact): an int-range literal is an INT; literal-only ops
wrap at 32 bits (`2000000000 + 2000000000` → -294967296; `-2147483648 /
-1` → -2147483648; `% -1` → 0); ONE long operand (an i64 field/binding)
promotes that op to long (`$a + 2000000000 + 2000000000` → 4000000001,
left-assoc promotes at the FIRST op). The engine's CExpr carries a
per-node ArithTy {I32, I64, F64} computed at compile with exactly
javac's promotion. Graduated regressions: fz_577215_1014 (plain
insert), fz_577215_270 (computed insertLogical).

## E. Updates/setters with computation (ar_upd_*, 18 probes, 3×-stable — the D-231 re-examination)

All engine_fenced (17 by the rhs_arg atom wall, ar_upd_same_value_runaway
by both-sides fire limit — its shape is atom-only and legal TODAY).
Runaways + error walls run as timeout-guarded singles, never batched.

### E1. Setter args are the SAME clean Java as insert args

| probe | pin |
|---|---|
| ar_upd_java_int | in setter position the §D ArithTy lattice holds verbatim: `setW(2000000000 + 2000000000)` → **-294967296** (int wrap, widened to long); `setD($x / 2)` x=7 → 3; `setR(-7 % 3)` → -1; `setP($x + 2000000000 + 2000000000)` → **4000000007** (left-assoc promotes at the first op) |
| ar_upd_dbl | `setA($x / 2.0)` → 3.5; `setB(0.1 + 0.2)` → 0.30000000000000004; `setI(1.0 / 0.0)` → Infinity, rendered as the JSON **string "Infinity"** (the D-283 serializer pins already cover this path) |
| ar_upd_div_zero | `setN($n / 0)` → **ConsequenceException: java.lang.ArithmeticException: / by zero** at fire — the D-283 judge parity clause covers it |
| ar_upd_narrowing_wall | `setK($d + 0.5)` k:long → **BUILD error** "The method setK(long) in the type C is not applicable for the arguments (double)" — javac assignability, the mirror of D-283's compile rule (computed f64 cannot narrow into i64) |

### E2. Eval sources — the fz_7_2525 law composes into arithmetic

| probe | pin |
|---|---|
| ar_upd_snap_bind | `setK(100); setM($k + 1);` k=5 → **m=6** — a BINDING inside arithmetic is the consequence-entry SNAPSHOT |
| ar_upd_getter_live | `setK(100); setM($p.getK() + 1);` → **m=101** — a GETTER reads LIVE, sees the just-set value |
| ar_upd_mix_snap_live | `setM($k + $p.getK())` after setK(100) → **m=105** — both sources compose in ONE expression (the out-of-sample prediction cell: predicted, then measured) |
| ar_upd_block_seq | `modify($p) { setK(100), setM($p.getK() + 1), setW($k + 1) }` → m=101, w=6 — the modify block is sugar for SEQUENTIAL statements; bindings stay snapshot inside it |

Engine mirror already exists: `Src::SnapField` (bindings) vs
`Src::Field` (getters), and `CExpr::Atom` reuses `Src` — the computed
port inherits the certified two-source model with no new machinery.

### E3. The mask model

| probe | pin |
|---|---|
| ar_upd_form_parity | modify-block and setter+update are behaviorally IDENTICAL with computed args (m=16 both facts, one fire each, no loops) — the oracle property-masks a bare `update()` from the setters textually preceding it, exactly the engine's pending-mask model |
| ar_upd_same_value_runaway | `C(n == 0) => modify { setN(0) }` → **fire limit** — the mask is DECLARED (setters-called), never value-diffed. ATOM-only: this runaway is in-subset TODAY, on both sides |

(Completing pins from the standing record: bare update with NO setters
= ALL-SET mask, class-reactive, refires even empty-listen patterns —
fz_42_3311/j13; EXTERNAL update = CHANGED-FIELDS mask — D-047.)

### E4. Re-trigger algebra — the D-231 hazard, mapped

| probe | pin |
|---|---|
| ar_upd_pr_getter | `C(a > 0) => modify { setB($c.getB() + 1) }` → fires ONCE, b=1; watcher on b==1 fires after. Getter reads do NOT listen; **written ∩ own-listened = ∅ ⇒ self-modify terminates** (statically decidable) |
| ar_upd_pr_bind_runaway | same increment but with `$b : b` BOUND in the LHS → **fire limit**. Bound fields ARE listened (the engine's line-3956 model, oracle-confirmed) |
| ar_upd_bounded_counter | `C($n : n, n < 5) => modify { setN($n + 1) }` → exactly **5 fires**, n=5 — guard falsification terminates the listened case, but the bound is DYNAMIC (not statically checkable) |
| ar_upd_noloop_self | no-loop + `setN($n + 1)` unguarded → **1 fire, n=1** — no-loop suppresses the rule's OWN re-activation |
| ar_upd_noloop_pingpong | two no-loop rules both incrementing n → **fire limit** — no-loop never blocks cross-rule cycles |
| ar_upd_setter_arith | cross-fact computed modify: `A($x : x) C(n == 0) => modify($c) { setN($x * 2 + 1) }` → n=15, one fire, downstream watcher fires — the USEFUL case (computed value feeds a different rule's LHS) is ordinary propagation |

### E5. TMS + no-propagation edges

| probe | pin |
|---|---|
| ar_upd_tms_rederive | RHS computed modify drives re-derivation exactly like the external-update pin (pr_ar_tms_update_rederive): Derive→U(2), W2, Bump modifies A.k 1→5, Derive RE-FIRES → U(6) justified and **U(2) refire-superseded** (retracted); W6 fires; final = U(6) only |
| ar_upd_set_no_update | setter with NO update(): object mutated (final view m=15) but **zero propagation** — the watcher never fires. Oracle serializer and engine store agree by construction |

### The port — LANDED (D-288, Bryan's gate: (a) + symmetric (b); not (c); (d) not built)

Shipped exactly as §E implied: both setter sites `rhs_arg` →
`rhs_expr`; `CompiledAction::Set` carries CExpr (atoms =
CExpr::Atom, byte-identical eval_src passthrough); computed args via
compile_cexpr + the javac assignability CompileError; gen.rs setter
axis under the guard-field discipline. 13 probes GRADUATED
(pr_ar_upd_*); the 5 runaway/error walls stay here as engine_fenced
recon (narrowing wall now rejects with the assignability error;
div_zero and the 3 fire-limit runaways error loudly on both sides).

⚖ The restriction ruling (Bryan, after counter-review): NO engine
wall — walls are legal where an ENGINE BOUND exists (D-284's
CompileError protects the recursive TMS cascade's rule-count depth
bound); the update loop is agenda-iterative under fire limit + the
D-117 spin guard, so there is nothing to protect. The cross-rule
update-edge cycle check is NOT built (D-284's pass GATES, so
"matching its contract" would re-create (c) one rule-count out; the
D-219/D-222 authoring altitude — per-rule, local, no chain
reasoning — rules out the advisory form). Authoring-layer guidance
is D-289: the D-222 template (blocking-with-exemptions at
compile_rules), SYMMETRIC over atom and computed self-feeds, with
the falsifying-write carve-out for the corpus's guard-flip idiom.

## F. LHS division: the TWO-MODE model — the div0 anomaly resolved, the swamp mapped (ar_dz_*, 29 probes, D-290)

25 deterministic probes 3×-byte-stable; 4 probes are DESIGNED race
witnesses (marked). All engine_fenced (LHS arithmetic is unparseable
engine-side — this family is the future port's battery). Every
prediction cell was written before its first run; 7/7 out-of-sample
predictions hit, plus the two mode-2 volume confirmations.

### The model

**MODE 1 — MVEL-interpreted** (every evaluation until the async jit
lands; ALL small scenarios live here entirely):
- `/` over integer operands computes in **IEEE double** — divisor
  source irrelevant (literal / same-fact field / cross-fact binding
  all identical: ar_dz_field_ops ≡ ar_dz_lit_ops ≡ ar_dz_bind_div).
  Division by zero NEVER throws: ±Inf/NaN per IEEE (even `k / 0`
  with a LITERAL zero: ar_dz_zero_lit fires, no error).
- `+ - * %` stay **long-exact** (ar_dz_prec: k=2^53+1 → `k + 1 ==
  2^53+2` and `k * 2 == 2^54+2` both fire — a double path would lose
  the +1; ar_dz_mod_prec: `k % 2^53 == 1` fires). `% 0` THROWS
  ConstraintEvaluationException even at a single evaluation
  (ar_dz_mod_zero — a LOUD error, the "/ by zero"-style parity shape).
- The COMPARISON then picks a representation:
  - **equality family (== / != / in)**: literal comparand with
    integral VALUE (3, 3.0) → narrow BOTH sides with a Java `(long)`
    cast and compare as longs; non-integral literal (3.5) → double
    compare. FIELD comparand → by declared TYPE (i64 narrows, f64
    doubles — ar_dz_field_cmp30: `== d2` with d2=3.0 f64 NO-fires,
    so value-representability is LITERAL-only). BINDING comparand →
    **TYPE-STRICT boxed equals: `k / 2 == $a` fires for NEITHER an
    i64=3 nor an f64=3.5 binding** (ar_dz_bind_cmp) — the beta
    equality never coerces; its own fence-worthy quadrant.
  - **relational family (> >= < <=)**: narrow-to-long iff the
    comparand's TYPE is integral (literal 3, i64 field/binding);
    double-typed comparands (3.0! 3.5, f64) compare as doubles —
    `k / 2 > 3` no-fires while `k / 2 > 3.0` FIRES on the same fact.
    Binding relational coerces normally (pred_bind_rel: `>= $b`
    fires) — strictness is equality-only.
  - The narrowing cast is Java's: (long)3.5=3, (long)+Inf=Long.MAX,
    (long)−Inf=Long.MIN, **(long)NaN=0 — so `0/0 == 0` FIRES and
    `0/0 != 0` does not** (ar_dz_zero_nan), and the same NaN fact
    fires `k/z < 1` but NOT `k/z < 1.0` (pred_nan_rel).
- The PDiv precision cell proves the quotient always transits double:
  `k / 1 == 9007199254740992` FIRES at k=2^53+1 (true long division
  would keep the +1 and no-fire).

**MODE 2 — jitted java** (per-constraint, after ~jitThreshold
evaluations + async compile latency): full java typing — long
division truncates and **THROWS on zero** (ConstraintEvaluation-
Exception, batch-fatal); long==double promotes java-style (3L ==
3.5 false).

**THE RACE (the reason the swamp resisted modeling):** the mode
switch is asynchronous and run-NONDETERMINISTIC. Witnesses:
- ar_dz_jit_zero (30 facts, `k / z > 0`, z=0): fired 30/30 in 3 of
  4 recorded runs; run 2 of the stability batch THREW mid-scenario —
  same input, two outcomes.
- ar_dz_race_eq35 (5000 facts, `k / 2 == 3.5`): fires a CONTIGUOUS
  PREFIX then stops cold — ids 1..128 / 1..127 / 1..135 across three
  runs (mode-1 fires, mode-2 refuses, cut point = compile latency).
- ar_dz_race_zero (5000 facts, `k / z > 0`, z=0): ERRORS all 3 runs
  (the jit always lands within 5000 evaluations; mode-2 throws).
Volume semantics for `/` are therefore NOT byte-certifiable against
this oracle configuration; small-scenario semantics (the entire
corpus and fuzz population) are pure mode 1 and fully deterministic.

### The AGREE subset (both modes give identical outcomes) — the certifiable core

- `+ - *` int operands: long both modes ✓ (the already-green binding
  arithmetic family).
- `%`: long both modes; `% 0` throws LOUDLY in both → a judge parity
  clause covers it (the D-283 "/ by zero" precedent).
- `/` with INT-TYPED comparands (eq and rel): mode-1 (long)(a/(double)b)
  ≡ mode-2 a/b for all |operands| < 2^53 and b ≠ 0 (cast and
  trunc-div agree, including negatives — both truncate toward zero).
  DISAGREE at ≥2^53 (PDiv) and at b = 0 (silent table vs throw).
- DISAGREE generically (fence candidates): `/` with double-typed
  comparands (the race_eq35 cliff class); expression == binding
  (mode-1 degenerate always-false); `/` with a runtime-zero-reachable
  divisor; `/` at ≥2^53 operands.

### The port — LANDED (D-291, Bryan's directive: agree subset + residency precondition + volume detector)

Shipped as §F implied, plus the two hardening requirements:
- Grammar: ArithCmp whole-slot constraints (drl.rs aexpr; the legacy
  slot grammar byte-preserved — 2060-scenario byte gate). Engine:
  compile_aexpr (LHS lattice: int literals are I64 per
  pr_ar_dz_lhs_i32/i32b — NOT the RHS javac I32) + Test::Arith with
  the D-037/D-113 identity key; eval is TOTAL (nonzero literal
  divisors compile-checked). `+ - * %` free (incl. `%` composition);
  int-int `/` = whole-side only, int comparand, nonzero int literal
  divisor; f64 `/` free (IEEE, any divisor). NaN: ==/rel false, !=
  true (pr_ar_lhs_nan_ne — probed, not assumed).
- Fences (CompileError with steering, all verified loudly): double
  comparands on int division; field/binding divisors; literal-zero
  divisors; division==binding equality; composed int division; `%`
  on doubles; bind-with-arith slots; `in`/matches over expressions;
  not/exists/accumulate/group-CE/query contexts.
- **⚖ MODE-1 RESIDENCY IS A LOGGED PRECONDITION**: every
  differential receipt for this feature is mode-1 evidence (the
  corpus and generator live at ≤6 facts, far under the ~20-eval jit
  threshold). The admitted grid is mode-invariant BY CONSTRUCTION +
  volume-confirmed on the == int cell (pr_ar_dz_jit_eq3, both modes
  agree at any volume); everything mode-divergent is fenced. A
  future `/`-bearing divergence at volume is RACE-SUSPECT first.
- **The volume detector**: harness diff + fuzz failure paths tag
  divergences whose LHS-division constraints could exceed ~16
  evaluations ("MODE1-RESIDENCY EXCEEDED (D-290 jit-race suspect)"),
  so the quarantine class detects its own members instead of
  presenting as a flaky gate. Generator: agree-subset single-op
  shapes only, structural residency cap.
- Expected-divergence witnesses (drift-banked): xf_ar_lhs_div_2p53
  (the ≥2^53 double-transit corner + MIN/-1 saturation, opposite
  polarities) and xf_ar_lhs_precedence_defect (the D-281 bare
  `a + b * c` eval throw, both polarities). Race witnesses stay here
  as engine_fenced recon, never promoted.
- 24 probes graduated (pr_ar_lhs_* / pr_ar_dz_*); corpus
  11/1257/406; the one fuzz find (fz_606060_555) minimized to ZERO
  arithmetic and bisected PRE-EXISTING → quarantined
  (xf_fz_606060_555, acc/setFocus/agenda-group latent family).
- Re-adjudicate the whole §F table on any oracle bump (jit behavior
  is engine-version-specific).

## G. The fenced contexts, probed (ar_ctx_*, 5 probes, 3×-stable — D-292)

Bryan's scope check after D-291: the not/exists, accumulate-source,
group, and query fences were walls of IGNORANCE (loud, doctrine-legal,
unprobed). Measured now — **the mode-1 model is CONTEXT-INVARIANT**:

| probe | pin |
|---|---|
| ar_ctx_not_exists | `not T(k / 2 == 3)` blocks at k=7 (narrow 3==3 matches); `not (== 4)` fires; exists mirrors — the cell semantics compose into CEs unchanged |
| ar_ctx_not_zero | `not N(k / z > 0)` at z=0 BLOCKS ((long)+Inf = MAX > 0 matches inside the CE); `not (< 0)` fires — the narrowing cast reaches through negation |
| ar_ctx_acc | source-pattern division filters the accumulate: k∈{7,6,9} → quotients {3,3,4} → count()==2 and sum==13 both fire |
| ar_ctx_group | `k / 2 == 3 \|\| k == 0` fires (left disjunct); `== 4 \|\|` silent; `!(k / 2 > 3)` fires — same cells inside `\|\|`/`!` composites |
| ar_ctx_query | `query qdiv() $t : T(k / 2 == 3)` returns exactly the k=7 row (k=9's quotient 4 excluded) |

No build errors, no context-specific coercion anywhere: the walls sit
on MODEL-CONSISTENT ground — they are scope cuts, not divergence
covers. Lifting any of them is ordinary Bryan-gated port work (the
join-level eval paths already handle Test::Arith; not/exists is a
compile-arm allowance; groups need AExpr inside the GExpr grammar;
queries need queries.rs plumbing), and the D-291 agree-subset
restrictions + mode-1 residency precondition would carry over
unchanged (the jit race lives in the same constraint machinery
regardless of context). Probes stay here as engine_fenced recon —
the walls must keep rejecting until a lift is gated.
