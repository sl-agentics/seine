# Baseline tier — extraction from the Drools 9.44.0.Final regression suite

The **baseline** corpus tier (`scenarios/baseline/`) contains scenarios
adapted from Drools' own JUnit regression tests
(`drools-test-coverage/test-compiler-integration`, Apache-2.0 — see NOTICE).
They are third-party-authored specifications of DRL behavior: any in-subset
divergence against the live oracle is a real faithfulness bug, with no
possibility that our own probe-writing blind spots shaped the test.

## Pipeline

```
Drools test sources (read-only clone, ~/drools-9.44-src, tag 9.44.0.Final)
  │  tools/gen_bean_catalog.py   — model beans -> tools/bean_catalog.json
  ▼
tools/extract_baseline.py        — Java test method -> scenario JSON candidate
  │    * resolves inline DRL string concatenation (incl. Class.getCanonicalName())
  │    * drops package/import/global statements (token-based)
  │    * strips WM-INERT RHS statements only (println, global-list adds, local vars)
  │    * lifts inline scalar `declare` blocks into scenario `types`
  │    * translates `new Bean(...)`/setter inserts via the bean catalog
  │    * records provenance: source path, class#method, JUnit-expected fire count
  ▼
tools/baseline_gate.py           — four-stage gate
  1. ENGINE PARSE GATE   parse/compile error = out-of-subset (routing datum)
  2. ORACLE RUN          Drools error = invalid translation (quarantined)
  3. DRIFT CHECK         oracle firing count != JUnit-expected count
                         = translation drift (quarantined, never tiered)
  4. DIFFERENTIAL        make-diff semantics; PASS -> baseline member,
                         FAIL -> faithfulness bug (REPORTED, not fixed)
```

The Seine parser is the **subset arbiter**: extraction never decides
in/out-of-subset itself. The drift check is the honesty guard — a scenario
only enters the tier if real Drools still behaves exactly as the original
JUnit assertion expected, so adaptation (stripped statements, declare
lifting, bean translation) demonstrably did not change the tested behavior.
Scenarios keep full provenance in a top-level `provenance` key (ignored by
both runners).

## Current state (2026-07-05, v1 extraction)

Scanned: **903 test methods** across 88 inline-DRL test classes
(`operators/`, `drl/`, selected root + `mvel/integrationtests` +
`session/` classes; class-level API/CEP/thread suites excluded up front).

| Stage | Count |
|---|---|
| Extracted candidates | 71 |
| — in-subset (parse gate) | 7 |
| — out-of-subset (feature walls; routed to ROADMAP/CANT/WONT evidence) | 64 |
| Baseline members (differential PASS) | **7** |
| **Faithfulness bugs (differential FAIL)** | **0** |
| Quarantined translations (drift/invalid) | 0 (3 caught and fixed during pipeline bring-up) |

Per-method routing: `docs/drools-test-routing.tsv`
(`extraction_route` = why a method did/didn't produce a candidate;
`gate_route` = what the gate decided for extracted ones).

### Why the yield is 7/903 — and why that's the honest number

The upstream suite deliberately exercises the whole Drools language; most
methods touch features beyond the certified subset, and each such method is
*routing evidence* for FEATURES.md rather than a lost test:

| extraction_route (top) | ~count | disposition |
|---|---|---|
| stmt-unrecognized (helper indirection, misc Java) | 229 | extractor v2 (per-class helper inlining) |
| bean-unknown / ctor-* / setter-* (object-model beans) | ~140 | mostly CANT (object graphs) |
| fact-nonbean (String/Integer/boxed facts) | 73 | out of subset (no boxed-type facts) |
| external-wm-api (FactHandle update/delete mid-test) | 71 | extractor v2 -> epochs actions (D-047 semantics ARE certified) |
| engine-internals (node/mask introspection) | 58 | skip-list (tests Drools internals, not DRL behavior) |
| no-fire-call (compile-only tests) | 54 | skip-list (compile validity, not behavior) |
| loop-in-test (fact-insert loops) | 21 | extractor v2 (unroll counted loops) |
| kiebase-config (property-reactivity modes etc.) | 21 | WONT (config matrix) |
| dialect-mvel | 16 | WONT (MVEL dialect) |
| multi-fire | 6 | extractor v2 -> epochs |
| query-api (getQueryResults translation) | 4 | extractor v2 -> scenario `queries` section |

Of the 64 extracted-but-out-of-subset candidates, the parse-gate reasons map
to FEATURES.md rows: inline `&&`/`||`/`!()` groups, `or`/`(or …)` CEs,
nested `not(… and …)`, `eval`, `from`, functions, plain-identifier bindings,
`retract`, char literals, map/`[]` access, `Object()` patterns, type-coercion
constraints (`==` String literal vs numeric field), null literals.
The three P1 ROADMAP rows (or-CE, constraint groups, nested existentials)
account for the largest block — landing them converts those candidates into
baseline members mechanically.

### v2 yield-expansion options (extractor/harness plumbing only, no engine)

1. **Epochs translation** for `external-wm-api` + `multi-fire` (~77 methods):
   map FactHandle-holding tests onto scenario `epochs` update/delete actions.
   Caveat: JUnit's 2-arg `session.update(fh, obj)` is the ALL-SET
   class-reactive mask; the epochs schema currently expresses field-mask
   updates only, so a faithful translation needs a `bare-update` action op in
   BOTH runners first (small, certifiable harness change).
2. **Per-class helper inlining** (`check(drl, n)`-style) — recovers a slice
   of the 229 stmt-unrecognized methods.
3. **Counted-loop unrolling** for `for (int i…) insert(…)` (21 methods).
4. **Query-call translation** into the scenario `queries` section (4+ methods
   in v1 scope; QueryTest/BackwardChainingTest have many more once helper
   inlining works). Guard: recursive-query scenarios must be oracle-run with
   a timeout (cyclic data hangs Drools — D-055).
5. **External `.drl` resource tests** (ExecutionFlowControlTest,
   FirstOrderLogicTest, session suites): same pipeline, DRL read from
   `src/test/resources` instead of string concatenation.

## Tier semantics

`make diff` runs the baseline tier alongside probes and fuzz regressions —
same harness, same oracle, same canonical comparison. A baseline failure is
by definition a **faithfulness bug against third-party-validated behavior**:
minimize it, report it, and only then decide the fix. Failing members are
quarantined under `scenarios/baseline-quarantine/` (excluded from the gate,
like `scenarios/xfail/`) so the repo gate stays green while the bug is
triaged.
