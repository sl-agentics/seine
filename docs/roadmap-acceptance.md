# ROADMAP acceptance criteria — upstream Drools tests that define "done"

For every ROADMAP row in FEATURES.md §2, the upstream Drools 9.44.0.Final
tests below are the acceptance criteria: when the feature lands, run these
through the extraction pipeline (tools/extract_baseline.py +
tools/baseline_gate.py) and the resulting scenarios must pass the
differential gate and join `scenarios/baseline/`. They are expected-to-fail
today and are NOT run as regressions.

Paths are classes under
`drools-test-coverage/test-compiler-integration/src/test/java/org/drools/…`;
counts are @Test methods (grep-approximate). Method-level evidence for the
already-extracted-but-out-of-subset candidates is in
`docs/drools-test-routing.tsv` (gate_route = out-of-subset, detail = parse
error).

| ROADMAP feature (priority) | Acceptance tests | ~methods |
|---|---|---|
| CEP-as-TMS investigation (P3, D-079 — memo, not implementation) | PseudoClockEventsTest (reference behavior only; extraction blocked on CEP runtime regardless) | n/a |
| Nested/multi-pattern not/exists (P1) | Misc2Test#testNestedNots1..3, compiler.integrationtests.FirstOrderLogicTest (not/exists group methods, ext-DRL) | ~12 |
| forall (P2) | operators.ForAllTest | 29 |
| Negative-lo temporal windows (P2, D-234 — deferred, recon-first) | temporal-operator suites' negative-bound methods (exact sweep at recon-open); acceptance is CONDITIONAL — the arc opens only if the probes_pending/ oracle recon shows determinate negative-lo join semantics, else WONT with receipts | ~TBD at recon |
| Null field values (P2, D-063) | mvel.integrationtests.NullTest; NullCheckOnExistentialNodeTest (null-value methods) | 10 + ~3 |
| Push/open/live queries (P2) | mvel.integrationtests.QueryTest (open-query methods), CepQueryTest (non-CEP methods) | ~8 |
| Query + mutation (P2) | QueryTest (update-after-query methods) | ~5 |
| Negation inside queries (P2) | QueryTest (query-with-not methods) | ~3 |
| activation-group (P2) | mvel.integrationtests.ExecutionFlowControlTest#testActivationGroups etc. (ext-DRL) | ~4 |
| agenda-group / focus / auto-focus / lock-on-active (P3) | ExecutionFlowControlTest (agenda/lock methods, ext-DRL); compiler.integrationtests.CompositeAgendaTest | ~10 + 2 |
| Accumulate extensions (P3) | compiler.integrationtests.AccumulateTest (multi-function, from-accumulate, collectList/Set methods) | ~30 of 84 |
| groupby (P3) | drools-model GroupByTest | module |
| Rule extends (P3) | mvel.integrationtests.ExtendsTest | 25 |
| Named consequences (P3) | mvel.integrationtests.NamedConsequencesTest; EdgeCaseNonExecModelTest | 39 + 2 |
| Constraint arithmetic, closed grammar (P3, D-061) | operators.MathTest (in-grammar methods), operators.FormulaTest (agree-subset rows) | **LANDED (D-290/D-291)**: the agree subset of the oracle's two modes — `+ - * %` over i64/f64 + restricted `/`; mode-divergent cells fenced; mode-1 residency logged as the certification precondition. The D-076 prereq was narrowed by probe (D-282) to the unbounded LOGICAL tier only — LHS arithmetic adds no justification chains |
| Date field type (P3, D-064) | mvel.integrationtests.DateComparisonTest | 3 |
| declare extras: defaults/@key/enums (P3) | mvel.integrationtests.TypeDeclarationTest, EnumTest; compiler.integrationtests.AnnotationsTest | 3 + 4 + 5 |
| @watch/@classReactive/@propertyReactive (P3) | mvel.integrationtests.PropertySpecificTest, PropertyReactivityBlockerTest, PropertyReactivityTest (annotation methods) | 59 + 5 + ~20 |
| Positional patterns in rules (P4) | Misc2Test positional methods | ~4 |
| retract alias (P4) | pervasive in older suites (e.g. session.DeleteTest retract methods) | ~5 |
| Plain-identifier bindings (P4) | compiler.integrationtests.drl.PatternTest (cheese : Cheese() forms) | ~10 |
| str[startsWith/endsWith/length] (P4) | mvel.integrationtests.StrEvaluatorTest | 10 |
| soundslike (P4) | operators.SoundsLikeTest | 4 |
| enabled attribute (P4) | operators.EnabledTest | 2 |
| halt() (P4) | mvel.integrationtests.DroolsFromRHSTest | 2 |
| Read-only scalar globals (P4, D-062) | compiler.integrationtests.drl.GlobalTest (scalar-read methods) | ~3 |
| BigDecimal/BigInteger fields (P4 hard, D-064) | Misc2Test, drl.LiteralTest, operators.MathTest (BigDecimal/BigInteger methods) | ~8 |
| Non-ASCII string values (P4) | mvel.integrationtests.I18nTest (value-only methods) | ~4 |

The former FEATURES.md §5 ambiguities were resolved 2026-07-05
(D-060..D-069): the ROADMAP outcomes (constraint arithmetic, nulls, Date,
BigDecimal, scalar globals) have acceptance rows above; the CANT/WONT
outcomes (pseudo-clock CEP, declared-type inheritance, char, virtual
dates, declarative agenda, Java-object globals) are on the skip-list
(docs/drools-test-skiplist.md).
