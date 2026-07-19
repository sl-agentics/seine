# Changelog

A rules engine whose pitch is auditability keeps an auditable release
history. Entries start at the why-machine arc; earlier releases are
recorded in DECISIONS.md.

## 0.4.44

- **Activation order after logical-belief supersede churn now matches
  Drools.** When a rule refires and replaces its own logical belief
  (`insertLogical` superseding a prior derivation) while that belief
  blocks another rule's `not`, Drools wakes the blocked rule's
  evaluation each churn cycle; Seine let its staging accumulate to the
  final release, firing the released activations in a different order.
  Seventeen scenarios graduated to the certified corpus, closing a
  long-banked fuzz witness (cf325901x52).

- **Logical beliefs of rules in never-refocused agenda groups now
  retract when their matches die.** An eager (no-loop) rule in an
  inactive agenda group whose `not` closes had its dead matches'
  belief teardowns deferred to an agenda pop that could never come —
  the beliefs lingered as phantom facts. They now drain at agenda
  quiescence, matching Drools (which unjustifies at the eager
  evaluation itself).

## 0.4.43

- **Firing order under `not` release now matches Drools in
  preempted-rule shapes.** When a rule's activation queue survives a
  would-be blocker that is inserted and deleted across an intervening
  higher-priority firing (e.g. a modify→delete relay between two
  rules at equal salience), Drools' lazy per-rule network evaluation
  never sees the blocker — the surviving activations fire in their
  original queue order. Seine evaluated eagerly and re-created those
  activations in release order instead. Ten scenarios graduated to
  the certified corpus, including a long-open fuzz witness family.

- **Self-contradictory logical derivations are now detected and
  raised** — a rule whose `insertLogical` falsifies its own `not`
  support is a Russell loop: no stable assignment of the derived
  fact exists. Seine previously *settled silently* on these
  (leaving a half-derived working memory with no signal anything
  was wrong) while real Drools oscillates to its fire limit. The
  engine now raises the same catchable "fire limit reached" error
  Drools produces — and, unlike Drools, says why: the error names
  the rule(s) whose derivations defeat their own support. A lone
  self-defeating match still terminates (both engines park it);
  the error arises when two or more such matches share a `not`
  (or-branches, sibling rules, or multiple matched facts) and
  relay each other. Twenty-two quarantined fuzz witnesses now
  agree with the oracle error-for-error, and a join-index
  quadratic found during the round was fixed (100k-firing runs
  complete in ~2.5s).

  **Compatibility note, loud:** rulesets that previously "worked"
  by settling on such a shape will now raise. Those rules were
  self-contradictory — the old quiet state was not a stable model
  of them, just a stopping point — and the same rules loop
  production Drools. Seine defaults to reporting the
  contradiction identically to Drools rather than picking an
  arbitrary resting state; a stabilizing mode could exist as an
  opt-in divergence if there is ever demand.
- **Shipped wheels self-identify**: `certification()["commit"]` on
  CI-built wheels now reports the source commit (it was `"unknown"` —
  the containerized builds could not run git). An engine whose pitch
  is auditability should let you audit *which engine you have*: the
  one-move answer to "does this installed wheel carry fix X" now
  works on PyPI artifacts. Builds from the sdist (no git context)
  still stamp `"unknown"`, honestly.

## 0.4.42

- **Event updates land in call order in windowed and plain
  accumulates over event sources** — an external update of an
  event feeding an accumulate now takes effect at its own queue
  position (drained at the next insert's flush point, exactly as
  Drools does) instead of at the fire boundary: with `collectList`,
  an updated element's new value now appears **before** elements
  inserted later in the same batch, updates apply in call order
  among themselves, and a window-evicted event revived by an
  update re-enters at the update's position. Five quarantined
  fuzz witnesses now match the oracle byte-for-byte; sums,
  counts, and every certified update-semantics probe (masks,
  epoch-final evaluation, expiry aliveness) are byte-identical.
- **A fact leaving and re-entering an accumulate in one batch is
  one update** — when an update pushes a fact out of an
  accumulate's source constraint and a later update in the same
  batch brings it back, the collected effect now lands as a
  single in-place update at the update's position (before the
  batch's fresh inserts), exactly as Drools' identity-folded
  staging does. Fixes `collectList` element order under
  out-and-back updates.
- **Eager rules in unfocused agenda groups preempt correctly** —
  a `no-loop` (or dynamic-salience) rule in a not-yet-focused
  agenda group leaves a pending agenda entry after its eager
  evaluation, exactly as Drools does; the group's first focus now
  yields to fresh higher-priority activations once before
  continuing. Closes the last quarantined witnesses of the
  focus-preemption family.
- **`collectList` removal order matches Drools exactly** — when a
  collected fact retracts (or its accumulated value changes), the
  list now loses its **first value-equal** element, exactly as
  Drools' `java.util.List.remove(Object)` reverse does — not the
  retracted fact's own entry. With duplicate values the two differ:
  five previously quarantined order-divergence witnesses now match
  the oracle byte-for-byte. Distinct-valued lists are unaffected.
- **Expired-event `not` releases no longer fire deleted facts** —
  when an event blocking a `not` expires (or is deleted) and the
  rule unblocks, activations now cover only facts that are still
  alive, matching the oracle: a fact deleted while the rule was
  re-blocked can no longer produce a phantom firing at a later
  release. Root cause was an internal staging-bookkeeping
  invariant broken across stream flush boundaries; nine
  previously quarantined divergence witnesses now match the
  oracle byte-for-byte.
- **Agenda focus preemption matches Drools** — when a rule's
  right-hand side pushes focus (`drools.setFocus`) to a group whose
  rules have pending network evaluations, the evaluation flushes
  staged propagation and freshly activated higher-priority MAIN
  rules preempt the pushing rule's remaining activations, exactly
  as the oracle's focused-group evaluation does (salience first,
  declaration order at ties, all fresh activations drained). The
  same law covers preemption inside a focused group: a fresh
  same-salience activation of an earlier-declared rule (for
  example an accumulate re-firing after a staged delete) now
  interrupts the running rule's remaining activations. Fixes an
  order-only divergence class: eleven previously quarantined
  fuzz witnesses now match the oracle byte-for-byte, on top of
  the 47-cell probe grid that mapped the law.
- **Windowed logical aggregates** — `insertLogical` from a windowed
  accumulate (`over window:time/length`) is in-subset: window
  eviction retracts the superseded logical result and derives the
  new one, downstream logical facts retract through the swap, and an
  emptied window keeps a `sum` matched at its identity — measured
  against Drools' own maintenance (an evicted event that is still
  alive in working memory triggers the same swap). Only `?query`
  justifiers remain a build error.
- **Decimal ingestion is verbatim** (measured against the oracle): a
  string keeps its own scale — `"1.1"` stays `1.1`, and `"1.005"`
  into a `decimal(10,2)` field is no longer silently rounded to
  `1.01`; integers ingest at scale 0. The declared `(p,s)` remains
  the Arrow column contract (and precision is still enforced) —
  it no longer rewrites working-memory values.
- **Truth-maintenance identity is scale-sensitive** (like
  `BigDecimal.equals`): logically inserting `2.50` and `2.500`
  yields two distinct justified facts, exactly as the oracle's
  generated equality does.
- **Numeric literals cannot construct or set decimal fields** in
  DRL (`insert(new Bal(2.5))`, `setV(2.5)`) — now a build error
  matching the oracle's, with steering to bindings and ingested
  data. Comparisons and in-lists over decimal fields are unchanged.

## 0.4.41

- **Exact decimal average** — `average_exact(field, scale=…,
  rounding="half_up")`: sum and count accumulate exactly, one
  division at the chosen scale with java.math rounding semantics
  (`BigDecimal.divide(count, scale, mode)` — certified
  value-for-value against oracle programs computing exactly that,
  across all seven RoundingModes and both signs). `scale` defaults
  to the source field's scale; modes: up, down, ceiling, floor,
  half_up (default), half_down, half_even. Decimal sources only —
  `average` stays IEEE double. Nulls skip both sum and count; an
  empty or all-null source doesn't fire (like `average`).

## 0.4.40

- **Decimal sum identity matches BigDecimal exactly** (found by the
  new decimal fuzz axis): a sum over an empty or all-null decimal
  source is now `Decimal("0")` — `BigDecimal.ZERO`, scale 0 — not
  `0.00` at the field's scale; a sum drained back to zero keeps its
  contribution scale (`0.00`), exactly like BigDecimal subtraction.
  Runtime decimal values also keep their own scale when stored into
  fields (declared precision still enforced); only string/int
  ingestion normalizes to the declared scale. Numeric comparisons
  are unaffected (compareTo semantics); only rendered scale moves.
- **Self-maintaining logical aggregates** — `insertLogical` from
  accumulate, groupby, and collect rules is in-subset (previously a
  build error): re-accumulation retracts the superseded logical
  result and derives the new one, downstream logical facts retract
  through the swap, groups maintain independently, and same-value
  recomputation dedups. Certified engine-vs-oracle (measured against
  Drools' own maintenance), exact over decimal sums, and expressible
  from the rule builder (`then_insert_logical(Bal, v=total)`) — the
  derived balance and everything under it stays `why()`-auditable.
  `?query` and windowed-accumulate justifiers remain build errors.
- **The reversible balance-gate idiom is documented and pinned**: a
  sum inserted as a new fact per recomputation leaves superseded
  results in memory, so logical facts derived from the old value
  survive their own reversal (Drools behaves identically — measured).
  The safe idiom — one result row updated in place behind a
  not-equal guard — is documented on `sum_`, certified
  engine-vs-oracle, and covered end-to-end from the rule builder.
- Accumulate-result match elements over decimal now render as
  `BigDecimal` (the oracle's Java simple name), like the other boxed
  scalars.

## 0.4.39

- **Decimal overflow is a typed, catchable error** — inline multiply
  and accumulate-sum overflow past `DECIMAL(38)` now raise a plain
  engine error (`except Exception` catches it; no Rust backtrace, no
  `PanicException`), and the session stays usable afterwards. Both
  previously surfaced as panics at eval time.

## 0.4.38

- **Inline decimal arithmetic in rule constraints** — the certified
  agree subset: `principal + fee >= limit` over `decimal(p,s)` fields
  computes exactly (i128, java.math scale rules) with
  compareTo-exact comparisons against decimal fields and int
  literals. Measured against Drools' BigDecimal/MVEL semantics
  (33-cell pin campaign) and certified cell-for-cell.
- **The poison is fenced, loudly**: decimal `/` and `%` (the oracle
  silently degrades them to IEEE double), doubles anywhere in
  decimal arithmetic, and double-literal comparands (the oracle
  coerces literals raw-binary — `== 3.30` can never fire on an
  exactly-3.30 result there; boundaries poison asymmetrically).
  Every fence names its reason and steers to the exact idiom.
- RHS decimal arithmetic remains a build error — now certified as
  error parity: the oracle rejects it too.

## 0.4.37

- **Wheel coverage**: Linux wheels now build in `manylinux_2_28`
  containers (RHEL 8 / UBI8, Ubuntu 20.04, Debian 11 get wheels
  instead of silent sdist builds), and the matrix adds
  `linux-aarch64` (Graviton / Axion / arm64 Docker) and
  `musllinux_1_2` x86_64 + aarch64 (Alpine images). macOS
  (arm64 + x86_64) and Windows unchanged.
- **Nullable aggregation** reaches the Python rule builder:
  `sum_`/`min_`/`max_`/`average` accept nullable numeric and decimal
  fields (null contributions are skipped, per the certified
  null-skipping semantics). Aggregate results type non-nullable.
- `sum_` documents the empty/all-null identity-0 footgun and the
  existence-guard idiom.

## 0.4.36

- **`Session.acc_sources(handle)`** — aggregation provenance: for an
  accumulate/groupby result fact (its handle is in the firing's match
  tuple, visible via `fire(on_fire=...)`), returns the
  `(source_handle, contribution)` pairs that produced the result's
  current value, snapshotted at computation. Null-skipped matches
  appear as `(handle, None)`. Closes the audit walk end-to-end:
  `why()` through the logical layer, `acc_sources()` through the
  summation to the line-item leaves.

## 0.4.35

- **`Session.why(handle)` / `Session.justifications()`** — the
  justification graph in Python: per derived (`insertLogical`) fact,
  its supports (justifying rule, matched tuple handles, firing seq)
  and live stated siblings; `None` for stated/dead/unknown handles.
  The support list is the retraction contract.
- **Decimal aggregation in the rule builder**: `sum_` over
  `decimal(p,s)` fields computes exactly (result widens to
  `decimal(38,s)`); `min_`/`max_` preserve the type; `average` is
  f64 by design (exact decimal division requires an explicit
  rounding mode).

## 0.4.34

- **Derive-plane regex**: `col().regexp_matches(pat)` (search) and
  `col().regexp_full_match(pat)`, dialect-pinned against DuckDB/RE2;
  patterns are build-time literals with loud invalid-pattern errors.
- Query-machine performance: the batch-prepend quadratic is gone
  (large `?query` pulls are ~linear).

## 0.4.33

- Deep logical-derivation chains run in ~linear time (the teardown
  and staging quadratics are gone); cyclic computed `insertLogical`
  (fixpoints, transitive closure) is in-subset.
- LHS whole-slot arithmetic and computed RHS args are expressible
  from the Python rule builder.
