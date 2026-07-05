# Skip-list — upstream Drools tests Seine will never run

The honest-limitations document: Drools 9.44.0.Final regression tests that
are out of scope *by design*, mapped to their FEATURES.md bucket (§3 CANT /
§4 WONT) or to "not a DRL-behavior test" (they test the platform, not the
language). These are never extracted, never counted as acceptance criteria.

Classes under `drools-test-coverage/test-compiler-integration/src/test/java/
org/drools/{compiler|mvel}/integrationtests/…`. Method-level skips inside
otherwise-extractable classes are recorded per-method in
`docs/drools-test-routing.tsv`.

## CANT — architecture excludes the feature (FEATURES.md §3)

| Upstream tests | Blocking constraint |
|---|---|
| mvel MVELTest, JittingTest, operators MathTest, FormulaTest, EvalTest, EvalRewriteTest | No embedded Java/MVEL expression evaluation; general `eval` confirmed CANT (D-061). MathTest/FormulaTest methods inside the D-061 closed arithmetic grammar are ROADMAP acceptance (docs/roadmap-acceptance.md), not skips |
| mvel FunctionsTest; DRL `function` methods throughout | User-authored Java function bodies |
| AccumulateTest custom-function + inline-code methods; AccumulateMvelDialectTest | Custom accumulate functions are user Java |
| CustomOperatorTest, CustomOperatorOnlyDrlTest | Pluggable Java evaluator API |
| MapConstraintTest, session FieldAccessTest, operators MemberOfTest, FromTest, drl NestingTest, compiler/oopath suite | Object-graph facts: nested/dereference access, collection fields, `from` iteration — columnar arena is flat scalar |
| operators InstanceOfTest, PolymorphismTest, drools-traits module | No Java class model: subtype matching, instanceof, traits |
| PropertyChangeSupportTest | No JavaBean eventing on arena rows |
| DynamicEvalTest | Dynamic (`@typesafe(false)`) constraint typing needs MVEL dispatch |

## WONT — exclusion is the feature (FEATURES.md §4)

| Upstream tests | Why excluded |
|---|---|
| CepEspTest (117), StreamsTest, AbstractCepEspTest, AccumulateCepTest, CepEspNegativeCloudTest, NegativePatternsTest, ExpirationTest, TemporalOperatorTest, WindowTest, LengthSlidingWindowTest, PseudoClockEventsTest, CepJavaTypeTest, AnnotationsCepTest, DRLCepTest, SubnetworkCEPTest, LifecycleTest, QueryCep*, CepFireUntilHaltTimerTest, MTEntryPointsTest, session EntryPointTest | CEP runtime (events, clocks, windows, entry points) — clock-dependent semantics vs deterministic replay (pseudo-clock included per D-060: even deterministic time adds a second WM lifecycle) |
| TimerAndCalendar*Test (4 classes), CalendarTest, TimerAndCalendarExceptionTest | Timers/calendars = wall-clock scheduling |
| FireUntilHaltTest, FireUntilHaltAccumulateTest, DroolsFromRHSTest (halt-thread methods), Parallel*Test, PhreakConcurrencyTest, all concurrency/ subdirs, MTEntryPointsTest | Threaded/active execution — single-threaded determinism is the product |
| All Kie*Test (Builder/Container/Services/Repository/Module/DefaultPackage/HelloWorld/Loggers/CompilationCache/BaseIncludes…), ClassLoaderTest, KieBaseIncludeTest, PackageInMultipleResourcesTest, MessageImplTest, KnowledgeBuilderTest, phases/* | KIE platform: packaging, containers, build API — Seine's surface is DRL text + facts |
| CommandsTest, FireAllRulesCommandTest, ListenersTest, RuleEventListenerTest, EnableAuditLogCommandTest, MBeansMonitoringTest, session RuleRuntimeEventTest, AgendaFilterTest | Commands/listeners/monitoring APIs — the deterministic firing log is the observability story |
| marshalling/ subdir, Serialized*Test, WorkingMemoryActionsSerializationTest, AbstractCellTest/CellTest/equalitymode Cell*Test, OutOfMemoryTest | Persistence/marshalling/limits — in-memory only |
| DynamicRules*Test, DynamicRuleLoadTest, DynamicRuleRemovalTest, FailureOnRemovalTest, RuleExtensionTest (incremental methods), MergePackageTest, incrementalcompilation/ subdir | Dynamic KB mutation at runtime — rulebase is immutable per session |
| DslTest, MultiSheetsTest (XLS), drools-decisiontables/templates/DMN/PMML modules | Authoring frontends compile down to rules |
| equalitymode/* (as a config axis), kiebase-config-dependent methods (drools.propertySpecific etc.) | One certified semantics; no config matrix |
| RuleFlowGroupTest, DeclarativeAgendaTest | BPM/ruleflow platform; declarative meta-agenda confirmed WONT (D-069) |
| I18nTest (identifier methods) | Non-ASCII identifiers break the accessor-sort wall (D-050/D-051) |
| Char-literal/char-field methods throughout (e.g. drl LiteralTest char methods) | Char type walled out of the subset (D-067) |
| GlobalTest (Java-object/mutable-global methods), GlobalOnLHSTest (object-global methods) | Java-object globals are side-channel state (D-062); scalar-read methods are ROADMAP acceptance |
| TimerAndCalendar date-effective/date-expires methods (incl. fixed-date variants) | Engine-evaluated calendar attributes WONT even with a virtual date (D-068); dates as fact fields are ROADMAP (D-064) |

## Not DRL-behavior (test the engine's internals or the test harness itself)

| Upstream tests | What they actually test |
|---|---|
| LinkingTest, UnlinkingTest, SegmentCreationTest, SegmentMemorySegmentPrototypeTest, NodePositionInPathTest, PathEndNodeTest, NodesPartitioningTest, ObjectTypeNodeTest, AlphaNodeRangeIndexingTest, JoinNodeRangeIndexingTest, IndexingTest, SharingTest (node-memory methods), AlphaNetworkModifyTest, PropertySpecificTest (mask-introspection methods, 30) | PHREAK/Rete node internals via InternalWorkingMemory — Seine pins the OBSERVABLE semantics instead (D-0xx probe ladders) |
| DRLDumperTest, ParserTest, NewLineAtEoFTest, ConsequenceOffsetTest, RuleMetadataTest, SwitchOverStringTest, KnownExecModelDifferenceTest (exec-model diffs), FromOnlyExecModelTest, EdgeCaseNonExecModelTest (exec-model methods) | Parser/AST/exec-model artifacts, not runtime behavior |
| BigRuleSetCompilationTest, ConditionLimitTest (compile-perf methods), OutOfMemoryTest | Build performance/limits |
| GeneratedBeansTest (FactType API methods), DroolsEventListTest, KieSessionIterationTest, StatelessStressTest | Java API mechanics around the session |

A test being on this list does not mean the *behavior* it guards is
unspecified in Seine — e.g. linking/unlinking semantics are certified via
observable probe ladders (D-028/D-032) rather than via node introspection.
