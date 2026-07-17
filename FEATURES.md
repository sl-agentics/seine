# FEATURES.md — Drools 9.44.0.Final coverage matrix

One row per Drools feature, bucketed against Seine's certified subset.
Sources: the Drools 9.44.0.Final documentation (`drools-docs`, language
reference `_drl-rules.adoc` + rule-engine pages), the upstream module
structure, and the upstream regression suite
(`drools-test-coverage/test-compiler-integration`). Oracle = Drools
9.44.0.Final throughout; D-0xx references point into DECISIONS.md.

**Buckets**

- **IMPLEMENTED** — built and differentially certified (1,388-scenario
  corpus: 11 baseline / 1,075 probes / 302 regressions, plus multi-seed fuzz
  campaigns and model-checked population sweeps). Each row maps to the D-0xx
  entries that pinned its semantics.
- **ROADMAP** — not built, architecturally compatible with Seine's design
  (columnar arena, single deterministic evaluator, DRL-text-only surface).
  Carries a priority. Upstream tests for these features are *acceptance
  criteria*, cataloged in `docs/roadmap-acceptance.md` — they define done.
- **CANT** — architecturally impossible given Seine's design; the specific
  blocking constraint is stated. Upstream tests are on the skip-list
  (`docs/drools-test-skiplist.md`).
- **WONT** — deliberately excluded; the exclusion is a design *strength*
  (usually: single-threaded determinism, one certified semantics, no
  embedded-Java evaluation, no KIE/BPM platform surface).

CANT and WONT are kept strictly separate: CANT is "the architecture forbids
it", WONT is "we choose not to, and the choice is the product".
Genuinely ambiguous calls were collected in §5 for an explicit ruling;
all ten were resolved 2026-07-05 (D-060..D-069) and their rows moved
into §1–§4.

**Test references** name upstream classes under
`drools-test-coverage/test-compiler-integration/src/test/java/…`;
`c.i` = `org.drools.compiler.integrationtests`, `m.i` =
`org.drools.mvel.integrationtests`.

---

## §1 IMPLEMENTED

| Feature | Pinned by | Drools-test references | Notes |
|---|---|---|---|
| Rules: `rule…when…then…end`, quoted names, declaration order | D-007, D-008 | m.i `DroolsTest`, c.i drl `DRLTest`, `PatternTest` | Declaration index is a conflict-resolution key. |
| Patterns over declared types; empty pattern `T()` | D-007, D-010, D-013 (j13) | c.i drl `PatternTest` | Types come from scenario schemas (= DRL `declare`). |
| Constraints: `==` `!=` `<` `<=` `>` `>=` over i64/f64/String/bool | D-007, D-010 (pr09/pr10) | c.i operators `EqualsTest`, drl `LiteralTest` | String relationals = byte order (ASCII subset). |
| Numeric cross-type promotion + coercion at literals, joins, indexes | D-020, D-028, D-029 | m.i session `TypeCoercionTest`, m.i `Misc2Test` (coercion methods) | Join `==` coerces to LEFT field's type; literals promote; alpha eq-hash groups coerce (w-series). |
| Field bindings `$x : f`, fact bindings `$p : T(…)` | D-007, D-013 | c.i drl `BindTest` | Bindings-as-RHS-snapshots per D-020. |
| Comma-AND constraint conjunction | D-007 | c.i operators `AndTest` (comma forms) | Inline `&&`/`||`/`!()` groups: D-073. |
| Multi-pattern joins (any k), self-joins, cross-pattern constraints | D-013, D-014, D-015, D-028, D-082, D-083 | m.i session `CrossProductTest`, c.i `BetaTest` | Left-major enumeration + PHREAK staging pinned to firing order; update-entry rights split on out-and-back re-entry (D-083). |
| Node sharing: alpha literal sharing + ≥3 eq-hash threshold | D-029 | m.i `AlphaNodeTest`, c.i `AlphaTest` | Coerced-key hashing, first-built-literal inheritance. |
| Node sharing: beta prefix trie, per-batch sink propagation flips | D-033, D-036, D-037 | c.i `SharingTest` (subset), m.i `AlphaNodeTest#testSharedAlpha` | Bound-field-set + named-var-reference identity. |
| Property reactivity (default ALWAYS): listen masks, update masks | D-013 (j06–j21), D-040, D-041 | m.i `PropertyReactivityTest` (subset; many methods use `@watch`/API) | `@watch`/`@classReactive`/`@propertyReactive` annotations are ROADMAP. |
| RHS: `insert(new T(…))` with literals/bindings/getters | D-007, D-010 | c.i drl `ConsequenceTest`, `RHSTest` | Insert-time propagation (D-046). |
| RHS: setters + `update($x)`, `modify($x){…}` blocks | D-013 (j10), D-023, D-024, D-083 | m.i session `UpdateTest`, `BasicUpdateTest` | Update cascade/requeue semantics fully pinned (D-023/D-024); alpha-entry via modify = plain right insert (D-083). |
| RHS: `delete($x)` (activation cancellation, unblocking) | D-013 (j05/j11), D-031 | m.i session `DeleteTest` | `retract` keyword alias not parsed (ROADMAP, trivial). |
| Bare `update()` ALL-SET class-reactive mask | D-025 (fz_42_3311) | m.i `PropertySpecificTest` (class-reactive methods) | u64::MAX sentinel mask. |
| `no-loop` | D-010, D-013 (j04), D-018 | c.i `ExecutionFlowControlTest#testNoLoop` (ext-DRL) | Eager evaluation windows per D-018. |
| `salience` (static int, negatives) | D-008 | c.i `ExecutionFlowControlTest#testSalience*` | (salience DESC, decl ASC, insertion ASC) + preemption. |
| `salience` expressions over numeric bindings (`+ - *`) | D-043 (se1–se15) | m.i `Misc2Test` (dynamic-salience methods) | Per-activation salience, newest-first dynamic ties, intValue() wrap. |
| `not` / `exists` CEs (bare, any position, InitialFact) | D-031, D-032 | c.i operators `NotTest`, `ExistsTest`, m.i `ExistentialOperatorTest`, `NullCheckOnExistentialNodeTest` | Blocker model, handover, unblock refire order (D-042 carve-out: 3 order-only quarantined cases). |
| Nested `not(…and…)` / `exists(…or…)` CE groups (RIA subnetworks) | D-088, D-089 | m.i `Misc2Test#testNestedNots*` (adapted mirrors: sn_d2 reproduces the fire counts; JDK fact types are not machine-extractable), `FirstOrderLogicTest#testRemoveIdentitiesSubNetwork` (shape adapted; the kbase option is WONT), c.i `SubnetworkTest` (shapes) | Counting machine (not the blocker model); LogicTransformer rewrites at parse (`not(or)`=De Morgan, `exists(or)`=double negation); orders inverted vs bare CEs, phase-keyed (leftIns=arrival, rightDel=reverse); 2-3 inner patterns, inner bindings group-scoped, bare not/exists inside groups (incl. the forall-correlation shape, sn_a10). Fences: RIA-in-RIA (`not(not)`, `not(exists(and))`, composite or-branches), groups+insertLogical (D-076 ext), groups in query bodies. |
| Range (comparison) indexes on not/exists nodes | D-032, D-035 | c.i `JoinNodeRangeIndexingTest` (not/exists subset) | Probe coerces to stored side's type; join-node range index stays off (Drools default). |
| `matches` (String fields, literal regex subset) | D-030 | c.i operators `MatchesTest` | Full-string; tame regex grammar; Drools-legal numeric-field `matches` is walled. |
| `contains` (String substring), `not contains` | D-030 | c.i operators `ContainsTest` (String methods) | Collection `contains` is CANT (no collection fields). |
| `in` / `not in` value lists | D-030 | c.i operators `InTest` | Composite ==-with-promotion branches; no alpha-hash participation (op_i4/i6/i7). |
| `accumulate` inline: `sum/count/average/min/max` | D-038, D-039, D-092, D-093 | c.i `AccumulateTest` (built-in inline methods), `AccumulateConsistencyTest` | Exact float op-sequencing, reverse/re-accumulate, null retraction, result typing walls = faithful Drools compile errors. **Intentional divergence (D-093):** Drools 9.44.0-through-current has a stale-extremum defect on min/max left-update merges (refold skipped unless the extremum's removal dirties last, D-092); Seine computes the correct value — faithfulness is to Drools-the-spec, not durable defects. Upstream report apache/incubator-kie-issues#2366 — **FIXED upstream** (kie-drools#6796, merged 2026-07-08, the report's repair verbatim; D-148) and **vendored into the oracle** (9.44.0.Final+p1, D-163): the seven witnesses CONVERGED and graduated to regressions/, the gen.rs wall is lifted, min/max×mutation is fuzz-covered again. |
| `collect` (`ArrayList()/List() from collect`) | D-038, D-040, D-041, D-085 | c.i `FirstOrderLogicTest#testCollect*` (ext-DRL) | Left-modify gate; subnetwork collect sources fenced (see §3 CANT / mn6 note); shared-prefix result updates keep peer kind (D-085). |
| Multi-fire sessions (insert → fire → insert → fire) | D-046 (mf1–mf6), D-091 | m.i session `StatefulSessionTest` (subset) | Epochs in scenario schema; firing log continues. Held-staging drain across fire boundaries CLOSED by the D-091 sources-port (RuleExecutor dirty-flag lifecycle; 455/4816 families graduated green). |
| External update/delete by handle + property masks | D-047 (xu/xv series), D-083 | m.i `PropertyReactivityTest`, session `UpdateTest` (API-side methods) | 3-arg `session.update(fh, obj, props)` mirror; window queues, slot memory; out-and-back re-entries take the late join pass (D-083). |
| Queries: non-recursive, params, unification `==`, bound/unbound calls | D-049, D-050, D-052, D-053 | m.i `QueryTest`, `Query2Test`, `Query3Test` (subset) | Row ORDER pinned incl. TupleIndexHashTable iteration (seed 993 hash model). |
| Queries: positional syntax, `or` bodies, query calls, recursion (fenced) | D-054, D-055 | m.i `QueryTest` (positional/chained methods), `AbstractBackwardChainingTest` | Fence: 2-branch base-first self-recursion; cyclic data = clean error (Drools hangs). |
| `?query` pull CEs in rules (the backward-chaining bridge) | D-056, D-057, D-058, D-086 | m.i `PassiveQueryTest` | Lazy pull windows, stateful query memories, agenda-item arming (link-gated, D-086), all-unbound CE sharing. |
| Truth maintenance: `insertLogical`, justification, cascading retract — **QUERYABLE justification graph** | D-076..D-080 | c.i `ErrorOnInsertLogicalTest`, m.i `Misc2Test` logical methods (all honestly routed out-of-subset), drools-tms module tests (skiplist) | **The why-engine substrate:** `Engine::justifications()`/`why(fact)` expose supports (rule + tuple + seq) and stated siblings. Value-equality over all declared fields (@key-all oracle declares); two-path unmatch timing with pinned drain points; refire-supersede; unstage materialization; delete quirks modeled. Walls: mutation of logical types, acc/collect/?query justifiers (compile-time). Documented-open fence (D-080; **triaged D-087, zero in-envelope pins**): 68 xfail witnesses = 45 compound transient-visibility (oracle-deterministic, all inside the D-078 fence shapes) + 22 Drools runaways (oracle fire-limit 10/10, engine terminates) + 1 Drools order-nondet — per-witness table in `docs/xfail-triage.md`. |
| `or` CE (infix/prefix, subrule rewrite) + parenthesized CE groups | D-070 | c.i operators `OrTest` (subset; `testEmptyIdentifier` in baseline), m.i `Misc2Test` or-scope routing | Parse-time DNF subrule expansion; branch-major agenda order, per-rule no-loop, plain-rule trie sharing, every-branch binding rule. Groups inside not/exists landed (D-089). |
| Inline `&&`/`||`/`!(…)` constraint groups, abbreviated forms, bind-with-restriction | D-073 | c.i operators `InTest#testInOperator`/`#testNegatedIn`, `OrTest#testConstraintConnectorOr` (baseline) | Top-level `&&` splits comma-equivalent (joins eq-hash groups, shares); `\|\|`/`!()` composites are in-like (double promotion, no hash participation). Query bodies keep the plain grammar. |
| `declare` fact types (scalar fields) | D-004 | m.i `TypeDeclarationTest`, c.i drl `DeclareTest` (plain-declare subset), `GeneratedBeansTest` | Scenario `types` ARE declares; both runners get identical blocks. |
| Boolean accessors are `isX()` only | D-009 | (Drools compile behavior) | Engine leniency documented (accepts getX too; generator emits Drools-legal only). |
| InitialFact (leading-CE rules) | D-031, D-038 (acc1), D-056 (qx0_first) | c.i operators `ExistsTest`/`NotTest` leading-CE methods | Canonicalized rendering in both runners. |
| Deterministic conflict resolution & agenda lifecycle | D-008, D-018, D-028, D-032, D-043 | c.i `ExecutionFlowControlTest`, m.i `RuleExecutionTest` | The certified whole: eager/lazy windows, linking, queue-on-unlink, item lifecycle. |
| fireAllRules with fire-limit parity | D-013 (j21) | (harness-level) | Both runners cap at 100k and error on runaway. |
| Working-memory introspection (final facts, firing audit, handles) | D-003, D-044, D-047 | (API shape differs; behavior via result schema) | Canonical multiset facts + ordered firing log with post-RHS renderings. |
| **Deterministic CEP E1**: `@role(event)` point events (explicit `@expires`), pseudo-clock `advance()`, `after/before[lo,hi]` temporal joins, expiration×TMS, STREAM per-insert flush | D-099..D-102 | c.i `CepEspressoTest`/`PseudoClockEventsTest` (behavioral reference; wall-clock tests stay §4) | Expiration rides the certified TMS/quiescence machinery — no second WM lifecycle. Final campaign 3×1000 = 0 divergences. |
| CEP E2-A: `@expires` INFERENCE (STP transitive closure) | D-109; not §3A D-130/132/133; exists D-135 | c.i CEP tests (inference subset) | Bare patterns infer NEVER; phantom `temporal_pos` records not/exists edges. Boundary corrected D-152: only NEGATIVE deadlines leak (DROOLS-455), nonneg-past = due-on-arrival. Allen-op inference LANDED (D-164): the 11 ops emit param-blind constant interval edges (mvel getInterval, 124-cell reach ladder); only `during` leaks (both sides), per Drools. |
| CEP E2-B: `window:time` (eviction + the A→B seam) | D-110..D-114; admission/revival D-154/D-155 | c.i `CepWindowTest` (time subset) | WindowNode RightTuple survives eviction/rejection; snapshot-ts admission; per-entry FIFO update drain. `window:length` stays ROADMAP (§2). |
| CEP E2-C: event UPDATE / external DELETE re-propagation | D-115, D-137..D-139, D-141, D-160 | m.i session `UpdateTest` × CEP (adapted) | Temporal position is INSERT-FIXED (ts field stays mutable); clock-removed revival guard; delete-time eval for event witnesses; windowed-acc watches source BINDINGS only; per-entry incremental acc drains. RHS re-entrant churn fenced. |
| CEP E2-D: named entry points (`from entry-point`) | D-116 | c.i `CepEspressoTest` (entry-point subset) | One routing dimension on the alpha network (a single `alpha_passes` clause); composes with mutation. |
| CEP E2-E: `@duration` interval events + the FULL Allen predicate algebra | D-118..D-120 | c.i CEP interval tests | `endTS = ts + dur` (dur=0 byte-identical); pure `eval_allen` predicate table; the 13 relations + parameterized after/before. |
| CEP temporal firing-ORDER fidelity: per-arrival join flush, existential admission, not window-close deferral, shared-node epoch batches, self-join phases | D-121..D-125, D-127, D-132..D-134, D-136, D-156 | (order-of-firing — no upstream assertions; model-checked specs `tools/model_*_flush.py`) | Graft-derived flush models, 0-div at population scale before each port. Fenced-by-nature: within-close-time timer ties (java.util.PriorityQueue nondeterminism, D-134 §6). |
| Existential firing-ORDER shadows: `not/exists <EVENT\|PLAIN>() P()` agenda-pick fidelity | BfShadow/ExShadow D-150..D-153; PnShadow + plain lazy staging D-158/D-159; PxShadow + the quiescence eval D-161/D-162 | (order-of-firing; specs `model_check_notorder_b.py` MODEL=flush/pflush, `model_check_exists.py` EMODEL=flush/pexists) | Mechanical replays of the graft-observed Drools propagation (rtm order + staged backlogs + link-counter queue economy) rank the gated picks; the D-140..D-147 key models retired. 20k+ population scenarios 0-div across the four families. |
| `Engine::reset()` — in-place session reset (paged-batch lifecycle) | D-104 | (impl-only API; the 10-probe ladder is the acceptance suite) | Clears WM/agenda/TMS/clock/handles, keeps the KieBase; listener-drop quirk measured; InitialFact re-created. Python `Session.reset()`. |
| `collectList`/`collectSet` accumulate functions + `groupby` (leading position) | D-108 | m.i `AccumulateTest` (behavioral reference) | Counted-set semantics; SetCollection canonicalized SORTED both sides. Joined-position groupby + query-side aggregation walled loudly (§2). |
| `agenda-group` + focus stack + RHS `setFocus` | D-106 | c.i `ExecutionFlowControlTest` (agenda-group subset) | Groups partition BEFORE salience; last-setFocus-on-top; empty-group pop. ⚠ the halt/continue model is a CLOSE APPROXIMATION (standing caveat, 5 witnesses in probes_pending/agenda_open — re-derive from witnesses before touching). `auto-focus`/`lock-on-active` stay §2. |
| Queries across mutation epochs | D-107 | m.i `QueryTest` (update-after-query methods) | The D-051/D-057 walls LIFTED: ?query CEs are PULL-AT-ACTIVATION; queried-side churn never re-evaluates existing matches. |

| Null field values — SQL three-valued logic | **IMPLEMENTED** (D-095–D-097) | Authority: **DuckDB 1.5.4/SQL 3VL** (docs/duckdb-datatype-pins.md), NOT Drools — a deliberate deviation. Opt-in `"nullable": true` / `Optional[X]`; UNKNOWN never admits (incl. under `!()`); `== null` ⇒ IS NULL; the `not in` null trap; null keys never equi-join; TMS keys collapse nulls; aggregates skip null contributions (sum(all-null)=0 fires, ruling 2). Oracle: tools/diff_duckdb.py + fuzz (12k+ cases clean). Queries/salience over nullable walled (liftable). |
| Exact decimal field type — Arrow Decimal128 | **IMPLEMENTED** (D-095/D-098) | Authority: **DuckDB/Arrow DECIMAL**, NOT Java BigDecimal. `decimal(p,s)` / `Annotated[Decimal, seine.Decimal(p,s)]`; i128 scaled storage; exact cross-scale compare; half-up ingest, loud overflow; floats NEVER meet decimals (compile wall, ruling 4); sum exact →DECIMAL(38,s), avg→f64, min/max preserve. 6k-case decimal fuzz clean. Queries over decimal walled (liftable). |

## §2 ROADMAP

Priorities: **P1** next probe phase candidates, **P2** high-value later,
**P3** worthwhile, **P4** trivia / long tail. Every row's upstream tests are
expected-to-fail acceptance criteria (see `docs/roadmap-acceptance.md`).

| Feature | Priority | Drools-test references | Rationale / notes |
|---|---|---|---|
| `window:length(N)` on ACCUMULATE SOURCES | P2 — ARC OPEN (D-183; Bryan rulings 2026-07-11: acc-source-only; TMS×window stays FENCED in-arc, D-080 remains its own arc; not deferred behind it) | c.i `CepWindowTest` (length subset) | Keeps the N most-recently-inserted events; does NOT feed `@expires` inference (D-110 recon). Handoff: `~/.claude/plans/window-length-arc.md`. |
| Standalone-pattern windows (`over` outside an accumulate source) | P3 | c.i `CepWindowTest` | Split from the acc-source row by the 2026-07-11 ruling; own grammar + node semantics; stays WALLED (natural parse wall at `over`). |
| Negative-lo temporal windows (`after[-500ms, 500ms]` — the symmetric coincidence/straddle window) | P2 — DEFERRED ARC, gated, **recon-first** (Bryan ruling 2026-07-13, D-234) | temporal-operator suites' negative-bound methods; exact sweep at recon-open | A real expressivity gap with NO in-subset workaround (split rules double-fire the delta-0 overlap; D-233 archaeology) — a sensor-correlation/near-simultaneity CEP staple, so not WONT by default. Structure: {oracle recon → decide → arc}. HARD PRECONDITION: graft the oracle's negative-lo join behavior into probes_pending/ BEFORE any grammar work — the lo<0 semantics against the deadline/not-fire machinery (which assumes lo≥0 today) must come from the pinned oracle, never inferred from the operator's documented meaning. If recon shows the oracle's negative-lo behavior is identity-hash/ordering-underdetermined, this flips to WONT with receipts. Not scheduled; rides behind the current correctness hardening. |
| `forall` | P2 | c.i operators `ForAllTest` (29) | Reducibility assessed at D-089: the MULTI-pattern form (`forall(base rem)`) is a pure parse rewrite onto the D-089 substrate — `not(base and not(rem))`, correlation shape probe-backed (sn_a10). NOT free: the flagship SINGLE-pattern form injects a `this == base` identity join (no such operator in subset — needs its own design), and multi-remaining forms need RIA-in-RIA (fenced). Keep as its own phase. |

| Push (reactive) query CEs + open/live queries | P2 | m.i `QueryTest` (open query methods) | qx2_late_push pinned the basic refire; row lifecycle unprobed (D-057). |
| Negation-as-failure inside query bodies | P2 | m.i `QueryTest#testQueryWithNot`-style | Q-phase follow-on per Q2 handoff. |
| `activation-group` (XOR groups) | P2 | c.i `ExecutionFlowControlTest#testActivationGroup*` | Pure agenda bookkeeping over the certified item lifecycle. |
| `auto-focus` / `lock-on-active` | P3 | c.i `ExecutionFlowControlTest`, `CompositeAgendaTest` | The agenda-group/focus/setFocus core landed (D-106, §1); these are the remaining attribute gates. `ruleflow-group` itself is WONT (BPM). |
| Accumulate extensions: multi-function, post-constraints, `from accumulate` result pattern | P3 | c.i `AccumulateTest` (multi-function/from-accumulate methods) | Same node, wider grammar (`collectList`/`collectSet` landed, D-108 §1); custom functions stay CANT. |
| `groupby` — joined-position keys + query-side aggregation | P3 | drools-model GroupByTest | The leading-position form landed (D-108, §1); the joined-position slice is walled loudly. |
| Rule `extends` (condition inheritance) | P3 | m.i `ExtendsTest` (25) | Compile-time prefix concatenation; fits trie sharing naturally. |
| Named consequences `then[x]` / `do[x]` / `if…break` | P3 | m.i `NamedConsequencesTest` (39), `EdgeCaseNonExecModelTest` | Docs mark it legacy-ish but the test surface is large; terminal-per-label model. |
| Constraint arithmetic (`age + 1 > $x`, closed grammar) | **LANDED (D-291)** | c.i operators `MathTest` (in-grammar methods), `FormulaTest` (agree-subset rows) | The AGREE SUBSET of the two oracle modes (D-290): `+ - * %` over i64/f64 with cross-pattern bindings; `/` restricted (int-int division needs a nonzero int-literal divisor, an int comparand, and stands alone on its side); mode-divergent cells FENCED with steering (double comparands on int division, field/binding divisors, division==binding equality, `%` on doubles, in/matches over expressions, not/exists/acc/group contexts). Mode-1 residency is a logged precondition; the harness tags volume divergences (jit-race suspects). General `eval` stays CANT. |
| Date field type (epoch-i64 encoding, date-literal parsing) | P3 | m.i `DateComparisonTest` (3) | D-064: dates as fact data compared against. Engine-evaluated `date-effective`/`date-expires` stays WONT (§4). |
| `declare` extras: field defaults, `@key` constructors, declared enums | P3 | m.i `TypeDeclarationTest`, `EnumTest`, c.i `AnnotationsTest` | Scalar-only defaults are easy; `@key` interacts with the D-066 value-equality story. |
| `@watch` / `@classReactive` / `@propertyReactive` annotations | P3 | m.i `PropertySpecificTest` (59), `PropertyReactivityBlockerTest` | Mask machinery already exists (D-013/D-040); this is surface syntax + mode gates. |
| Positional patterns in rule LHS (queries already have them) | P4 | m.i `Misc2Test` positional methods | Parser + `@position` ordering; semantics identical to query positional form (D-054). |
| `retract(…)` keyword alias for `delete` | P4 | (pervasive in older tests) | Parser alias, zero semantics. |
| Plain-identifier bindings (`cheese : Cheese()` without `$`) | P4 | c.i drl `PatternTest`, old-style tests throughout | Parser trivia; Drools-legal, engine currently rejects. |
| `str[startsWith\|endsWith\|length]` operator | P4 | m.i `StrEvaluatorTest` (10) | Simple String evaluator triple. |
| `soundslike` | P4 | c.i operators `SoundsLikeTest` | Soundex; tiny, low demand. |
| `enabled` attribute (boolean literal) | P4 | c.i operators `EnabledTest` | Static skip flag; expression form, if ever, follows the D-061 closed grammar. |
| `halt()` from RHS | P4 | m.i `DroolsFromRHSTest` | Deterministic agenda stop; trivial in the fire loop. |
| Read-only scalar globals in constraints | P4 | c.i drl `GlobalTest` (scalar-read methods) | D-062(b): per-session constant environment. RHS sink globals already stripped at extraction (D-059); Java-object globals WONT (§4). |

| Non-ASCII string VALUES | P4 | m.i `I18nTest` (value subset) | Needs UTF-16-order comparison shim above BMP; identifiers stay walled (accessor-sort rule, D-050). |

## §3 CANT

| Feature | Blocking constraint | Drools-test references |
|---|---|---|
| Java/MVEL expressions in constraints or RHS: method calls, ternaries, `this` expressions, inline maps/lists, `throw` | **No embedded JVM / no expression interpreter** (⚖ D-280: the boundary is the INTERPRETER, not arithmetic). Seine's single evaluator executes a closed, pre-compiled grammar; arbitrary Java/MVEL is the boundary of the product. (Closed-grammar arithmetic is CERTIFIED SUBSET: LHS constraints = the D-290/D-291 agree subset; RHS insert/setter args = D-283/D-288.) | c.i operators `MathTest`, `FormulaTest`, m.i `MVELTest` (33), `JittingTest` |
| DRL `function` blocks, `import static` functions | Same constraint: user-authored Java bodies cannot execute. | m.i `FunctionsTest`, c.i drl (function methods) |
| Custom accumulate functions (`AccumulateFunction` impls), inline-code accumulate (`init/action/reverse/result`) | Same: user Java. Built-ins are ported bit-exactly instead (D-038). | c.i `AccumulateTest` (custom-function methods) |
| Custom operators (pluggable evaluator API) | Same: Java plugin surface. | c.i `CustomOperatorTest`, `CustomOperatorOnlyDrlTest` |
| Object-graph facts: nested property access (`address.city`), map/list fields, `[]` access, `memberOf` against collection bindings, `contains` on collections, `from $x.collection` iteration, null-safe deref `!.` (D-063) | **Columnar arena stores flat scalar fields** (i64/f64/String/bool per column). There is no reference graph between facts, no collection-typed values, and no dereference chain evaluator. | m.i `MapConstraintTest`, session `FieldAccessTest`, `NullSafeDereferencingTest`, c.i operators `MemberOfTest`, `FromTest` (24), c.i drl `NestingTest` |
| OOPath expressions (`/persons[…]`, reactive `?/`, backreferences) | Same object-graph constraint (OOPath is dereference-chain syntax). | m.i oopath tests (compiler/oopath) |
| Fact-model classes from the app classpath (POJOs, inheritance, interfaces, traits, `instanceof`, inline casts `#`, `isA`) | Facts exist only as arena rows of declared scalar types; there is no Java class model to match against. | c.i operators `InstanceOfTest`, m.i `PolymorphismTest`, drools-traits module |
| Declared-type inheritance (`declare X extends Y`) + supertype matching | **One-type-one-arena invariant** (D-065): alpha/beta indexes key on (type, field), property masks are per-type bit positions, node-sharing identity assumes one arena per pattern type. Supertype matching over a union of subtype arenas is an arena redesign, not a feature. | m.i `TypeDeclarationTest`/`ExtendsTest` (declare-extends methods), c.i drl `DeclareTest` (inheritance methods) |
| `eval(…)` over arbitrary expressions | Interpreter constraint — confirmed CANT with no subset-grammar carve-out (D-061). | c.i operators `EvalTest` (16), `EvalRewriteTest` |
| >96 distinct keys per indexed join key (hash-table resize) | TupleIndexHashTable resize re-buckets with chain reversal — deliberately unmodeled; the 96-key wall is part of the certified envelope (D-051). | (surfaced by fuzz, not upstream tests) |
| `@propertyChangeSupport` (JavaBeans listeners mutating WM) | Facts are arena rows; there is no bean eventing to listen to. | c.i `PropertyChangeSupportTest` |
| Dynamic/`@typesafe(false)` constraint typing | Requires MVEL dynamic dispatch; the engine compiles typed column accessors. | m.i `DynamicEvalTest` |

## §4 WONT

| Feature | Why exclusion is a strength |
|---|---|
| Multithreaded evaluation (`drools.parallelExecution`, partitioned networks), `fireUntilHalt` active mode, session pools, thread-safety machinery | **Single-threaded determinism is the product.** Same inputs → same firing log, byte-for-byte, across runs and platforms; the differential guarantee depends on it. Upstream needs test suites for race conditions; Seine cannot race. |
| Timers, calendars (`timer(int/cron/expr)`, Quartz), `duration`, `date-effective`/`date-expires` (incl. virtual/fixed evaluation date, D-068) | Wall-clock scheduling makes rule firing a function of *when you ran it*. A ruleset whose behavior depends on the calendar is that same nondeterminism even with a fixed evaluation date. Dates as **fact data compared against** = ROADMAP (D-064); dates as **engine-evaluated effective/expiry** = WONT. |
| CEP wall clocks: realtime session clock, `fireUntilHalt` live streams (the E2 A–E surface — `@expires` inference, `window:time`, event mutation, entry points, `@duration`/Allen — is IMPLEMENTED on the pseudo-clock, D-109..D-120, §1) | Wall-clock firing makes rule behavior a function of *when you ran it* — the same nondeterminism the pseudo-clock exists to remove. The deterministic `advance()` surface is the product boundary. |
| MVEL dialect (`dialect "mvel"`) | One dialect, one semantics: every certified behavior is pinned against java-dialect Drools; a second dialect doubles the oracle surface without adding engine capability. |
| KIE platform: KieContainer/KieBase/kmodule.xml, KieBuilder, classloaders, KJARs, KieScanner/maven, kie-server, commands/BatchExecutor, stateless-vs-stateful session API | Seine's surface is DRL text + typed facts in, results out (plus Arrow/Python bindings). No build-system, packaging, or container lifecycle to misconfigure; the harness IS the integration story. |
| BPM/ruleflow: `ruleflow-group`, jBPM/process integration, declarative agenda over process state | Out-of-domain platform (the brief's no-BPM wall). Agenda-group-style partitioning is ROADMAP §2 without the process engine. |
| Persistence, marshalling/serialization, JPA, clustering, reliability | In-memory only: a session is cheap to rebuild from facts; serialized-session compatibility is a permanent tax on every internal data structure. |
| Event listeners/channels/audit APIs (AgendaEventListener etc.), MBeans/metrics | The deterministic firing log + WM delta is strictly stronger observability than callback ordering, and it's diffable. Python `on_fire` covers the observer use case after quiescence. |
| Authoring frontends: decision tables (XLS), DSL/DSLR, templates, DMN, PMML, scenario-simulation | They all compile down to rules; Seine certifies the rule semantics underneath. The Python authoring layer (D-045) plays this role with definition-time wall errors. |
| Rule units (`unit`, DataStore/DataStream, Kogito REST) | Alternative session/data-source API aimed at Kogito microservices; orthogonal to engine semantics and superseded by the bindings' session model. |
| Alternate engine modes: sequential mode, propagation modes (`@Propagation(IMMEDIATE/EAGER)` as user surface), equality assert mode as a *config*, `drools.*` tuning knobs (alpha range-index threshold, beta range index, jitting thresholds) | **One certified semantics.** Every config axis multiplies the differential surface (each combination is its own oracle); Seine pins Drools' defaults and certifies those exhaustively instead of shallowly certifying a matrix. (Equality-assert mode as config confirmed WONT by D-066; the value-equality *mechanism* landed with TMS — D-076, §1.) |
| ~~RHS arithmetic in action args~~ — **SUPERSEDED (the D-280 boundary redraw)**: `insert(new T($x + 1))` landed D-283 (+ insertLogical under stratification, D-284); `modify($c){ setN($n + 1) }` landed D-288 (Bryan's gate: walls only where an engine bound exists — the update loop is agenda-iterative under fire limit + the D-117 spin guard, so the D-231 WONT dissolved). Self-feeding modifies are caught at the authoring layer (D-289, symmetric over atom and computed, falsifying-write carve-out); the declarative forms (LHS accumulate, two-pass update) remain the recommended idioms. Cyclic computed insertLogical LANDED (D-293/296: the iterative-cascade rewrite removed the engine bound; the stratification wall is lifted — fixpoint numerics and transitive closure are in-subset, fire-limit-governed both sides, certified by pr_ub_* incl. a 1000-deep chain; deep-scale witnesses in scenarios/bench_slow/). |
| `drools.getKieRuntime()` / kcontext RHS API (beyond halt/focus) | RHS is declarative by design (insert/update/delete only): consequences cannot reach engine internals, so every WM mutation is visible to the differential harness. |
| Consequence exception handling config | Subset RHS cannot throw; error surface is parse/compile time. |
| Java-object globals (mutable services/collections reachable from rules) | D-062(c): side-channel state invisible to the differential harness. RHS sink globals are stripped at extraction with the firing log as the stronger assertion (done, D-059); read-only scalar globals are ROADMAP-P4 (§2). |
| Char fields / char literals | D-067: niche type, odd DRL stringification of `'x'` literals, no target-domain demand. Out of subset; revisit only if a real corpus needs it (then: 1-char String vs i64 code point). |
| Declarative agenda (rules controlling other rules' matches) | D-069: meta-control couples agenda internals to user rules; small upstream surface (m.i `DeclarativeAgendaTest`, 16). Agenda-groups (ROADMAP-P3) cover the real use cases. |

## §5 AMBIGUOUS

All ten items resolved 2026-07-05 — rulings recorded as **D-060..D-069**,
rows moved into §1–§4:

| # | Item | Ruling | Now in |
|---|---|---|---|
| 1 | CEP pseudo-clock | WONT (D-060) -> superseded: E1 IMPLEMENTED via the TMS reduction (D-099..D-102) | §1 |
| 2 | Bounded expression grammar | Constraint arithmetic LANDED (agree subset, D-290/D-291); RHS insert/setter args LANDED (D-283/D-288); general `eval` CANT — D-061 | §2 / §3 |
| 3 | Globals | Sinks stripped (done); scalar read-only ROADMAP-P4; Java-object WONT — D-062 | §2 / §4 |
| 4 | Null field values | ROADMAP-P2 (raised), own phase; `!.` CANT — D-063 | §2 / §3 |
| 5 | Date / BigDecimal | Date ROADMAP-P3; BigDecimal ROADMAP-P4-hard via i128 scaled fixed-point, NOT CANT — D-064 | §2 |
| 6 | Declared-type inheritance | CANT (one-type-one-arena invariant) — D-065 | §3 |
| 7 | Fact equality / TMS | Value-equality over declared fields; TMS flagged PRODUCT-CRITICAL; equality-mode config WONT — D-066 (landed: D-076) | §1 / §4 |
| 8 | Char fields/literals | WONT (out of subset) — D-067 | §4 |
| 9 | Virtual date-effective/expires | WONT (calendar-dependent behavior) — D-068 | §4 |
| 10 | Declarative agenda | WONT (meta-control) — D-069 | §4 |

---

*Maintenance:* when a ROADMAP feature lands, move its row to §1 with its
D-0xx pins and promote its acceptance tests into `scenarios/baseline/`.
§5 is fully resolved (D-060..D-069); if a new ambiguity surfaces, park it
there for an explicit ruling rather than guessing a bucket.
Last reconciled against DECISIONS at D-162 / v0.4.1 (2026-07-11).
