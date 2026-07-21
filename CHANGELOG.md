# Changelog

A rules engine whose pitch is auditability keeps an auditable release
history. Entries start at the why-machine arc; earlier releases are
recorded in DECISIONS.md.

## 0.4.46

- **A no-loop rule's accumulate-justified logical belief now retracts
  in the same teardown wave as directly-justified beliefs.** When a
  fact deletion kills both join matches and an accumulate result (the
  aggregate re-derives to null), Drools lands the accumulate-path
  retraction at the no-loop rule's firing-boundary evaluation — before
  lower-priority rules fire — so an observer of the beliefs (e.g. a
  collect) never sees the intermediate state. The engine deferred that
  one retraction to the rule's own agenda pop, firing the observer once
  on the intermediate collection (one extra firing; surfaced by fuzzing
  as fz_336002_968, now a certified probe with a 7-cell law battery).
  Without no-loop the intermediate firing is correct Drools behavior on
  both sides, and the self-defeat retraction-timing laws (D-196/199/201)
  are untouched.

- **Logical beliefs justified by a starved agenda group's dead matches
  now survive to session end, matching Drools' lazy evaluation.** When
  a rule in an agenda group loses its justifying match after the group
  left the focus stack, Drools only retracts the logical insertion if
  the rule's network evaluates again — via a refocus, or (for no-loop /
  dynamic-salience rules on the eager list) a firing-boundary
  evaluation that runs only while the rule's positive inputs are
  populated (linked). The engine evaluated eager-listed rules
  unconditionally, retracting beliefs Drools keeps (surfaced by
  fuzzing as fz_337002_1104, now a certified probe with a 13-cell law
  battery covering the full group × linking × salience matrix).
  MAIN-group rules, refocused groups, and linked eager rules retract
  exactly as before.

- **Windowed-accumulate collections now append a queued update's
  re-admission before a same-call fresh insert.** When an external
  update revives an event whose window eviction was flushed in an
  earlier epoch, and the same epoch then inserts a fresh event, Drools
  drains the update queue FIFO ahead of the insert's own append; the
  engine staged all three effects LIFO, reversing the collection's
  element order (the corner D-327 recorded as open, surfaced by CEP
  fuzzing as cf355901x129 — now a certified probe with a 7-cell law
  battery). Same-epoch eviction+revival, boundary drains, and
  in-window updates were already correct and are unchanged; aggregate
  values and firing sets were never affected — element order only.

- **Collect/accumulate re-firing order now honors Drools' persistent
  touch order on shrink and index-move deltas.** When a collection or
  accumulate result changes, every driver match re-fires; Drools walks
  those re-firings in the beta memory's touch order, where a modified
  driver keeps its re-seated position across ALL later deltas. The
  engine already did this for growth deltas but reverted modified
  drivers to insertion order when the collection shrank (or an indexed
  accumulate's source moved between buckets) — an adjacent-swap
  firing-order divergence surfaced by fuzzing (fz_356002_1512, now a
  certified probe with a 13-cell law battery). Aggregate values,
  firing sets, and final working memory were never affected — order
  only.

## 0.4.45

- **Recursive query workloads that enumerate large derivation
  spaces now complete instead of hitting the evaluation backstop.**
  A rule pulling an open recursive query once per activation
  re-derived the same enumeration for every caller; the evaluator
  now derives each distinct call once and replays the result for
  every caller — validated at runtime against the machine's own
  emission discipline, and falling back to full evaluation for any
  call shape outside the validated class, so certified behavior is
  byte-identical everywhere. Alongside it, a set of engine-side
  scaling fixes: per-type fact indexing (pattern drains no longer
  walk every fact ever inserted), change-tracked query memories
  (unrelated working-memory churn no longer re-drains query
  patterns), and linear-time provenance ranking in the ordering
  laws. A stress scenario that previously could not finish at 200×
  the step budget now completes in seconds and joins the certified
  corpus, matching the oracle's 374,533 firings exactly. The
  backstop's error message no longer guesses "cyclic data" — a
  finite workload can legitimately exceed the budget.

- **Rules pulling from a `?query` can now derive self-maintaining
  facts with `insertLogical`.** The last query wall is lifted:
  probing showed Drools treats the pull as a pure snapshot for
  truth maintenance exactly as it does for activations — changes
  to the pulled facts (deletes, updates, later inserts, even links
  of a recursive query's derivation chain) never touch the derived
  belief, while the rule's own matched facts participate normally
  (retraction on death, support counting across justifiers). The
  engine already behaved this way on both sides of that line, so
  the lift certifies with no behavioral change. Nine scenarios
  graduated to the certified corpus.

- **A query called after the session quiesces now returns its rows
  in Drools' order when a rule pulled the same query mid-run.** A
  mid-run `?query` pull leaves the query's network memories
  populated; a later top-level call on a multi-branch query then
  enumerates an unbound branch bucketed by its first key, walking
  the facts inserted after the pull ahead of the facts the pull saw.
  Seine continued its accumulated window order instead. The reorder
  applies exactly where the law was pinned — an unbound single-
  pattern branch with a populated sibling, one pull site, a boolean
  key — and every other shape keeps its certified order. Ten
  scenarios graduated to the certified corpus, closing the last
  banked witness of the query family.

- **A rule pulling from a `?query` no longer fires for a fact
  deleted in the same evaluation.** A `?query` pull expands its
  matches directly at the rule's terminal, so when another rule
  deletes the driving fact before the pulling rule's turn, the
  children's inserts and deletes could meet in one evaluation —
  and the doomed activations fired anyway, producing extra
  consequences from a dead fact. Drools unstages the pending
  activation outright; Seine now cancels the pair the same way.
  Four scenarios graduated to the certified corpus, closing two
  long-quarantined fuzz witnesses.

- **An `or` branch that re-fires an updated match now fires it in
  Drools' position when the rule shares its patterns with a later
  rule.** When an external update revives a match whose activation
  was never consumed (its `exists` guard only became true in the
  same batch), Drools moves the pending activation into the current
  batch — so at the second `or` branch it fires after the batch's
  new matches, not before. Seine's peer staging kept the stale
  queue position. The reposition applies exactly when a further
  rule's terminal follows the branch on the shared join — the
  shapes without one (two-branch, three-branch, terminal-first,
  or an already-consumed activation) keep their certified order
  unchanged. Seven scenarios graduated to the certified corpus,
  closing the last banked witness of the shared-prefix `or` family.

- **A low-priority rule whose matches piled up while higher-priority
  rules ran now fires them in Drools' order.** When a rule shares
  its leading patterns with an `exists`/`not` guard chain and, by
  salience, fires only after everything else has quiesced, Drools
  builds its activation queue in one lazy segment flush — merging
  consequence-inserted facts across firings into a single batch
  unless a rule sharing the same join evaluated between them. Seine
  accumulated the queue eagerly per batch, which ordered those
  activations differently whenever the batches merged. The queue now
  re-sorts to the lazy accumulation order at the rule's first
  firing, gated to shapes where the lazy premise provably holds
  (sibling chains that cannot fire mid-accumulation, no external
  modifies); all previously certified orderings are unchanged. Nine
  scenarios graduated to the certified corpus.

- **A delete that un-blocks many parked activations at once now
  releases them in Drools' order.** When a rule guards on
  `exists(... and not(...))` and a single delete flips that guard
  for every waiting activation simultaneously (a mass un-block),
  Drools emits the wave in the order its lazy segment flushes
  accumulated the tuples: batch by batch in creation order —
  setup facts as their own batch, consequence-inserted facts
  merging into one batch unless a rule sharing the same join
  evaluated between the insertions — with each batch walking
  new-fact rows first and older facts most-recent-batch-first,
  before dynamic salience orders the queue. Seine released the
  parked set by blocked-list walk order, which flipped with
  network shape. The release is now normalized to the accumulation
  law (verified by an exact replay model over every extracted
  wave). Three scenarios graduated to the certified corpus,
  closing a long-quarantined fuzz witness.

- **Rules sharing identical pattern prefixes now fire in Drools'
  order when an external update flips a shared fact out of a
  pattern and back.** When several rules (or `or` branches) share
  the same leading patterns and an external batch both re-enters a
  fact into the shared join and inserts new facts, Drools' segment
  flushes make the re-entered fact's re-fires queue ahead of the
  new facts' activations, in an order that differs per sharing
  rule. Seine composed the whole batch as one block, interleaving
  those groups differently. Batches driven by rule consequences,
  insert-only batches, and unshared rules were already correct and
  are unchanged. Five scenarios graduated to the certified corpus,
  closing a long-quarantined fuzz witness.

- **A fact that leaves and re-enters a pattern within one batch of
  changes no longer slips past a standing `not (A and B)` block —
  and no longer suppresses a due `exists (A and B)` re-fire.** When
  external updates flip a fact out of a pattern and back (or churn
  it while it participates in a `not`/`exists` group's inner
  conjunction), the group's support tuples die and re-form in the
  same evaluation. Seine's staging cancelled the re-formed support
  against the dying generation's delete, so a rule blocked by
  `not (A and B)` could fire spuriously on the round-trip, and the
  mirrored `exists` case could miss a re-fire Drools produces.
  Drools tracks these by tuple identity and keeps the block (and
  the re-fire); Seine now does too. Nine scenarios graduated to the
  certified corpus, closing a long-quarantined fuzz witness.

- **`no-loop` now suppresses across `or` branches when a branch's own
  consequence satisfies a sibling branch's `exists`.** In Drools,
  `or` branches compile to sub-rules sharing the rule's name, and
  `no-loop` suppresses any activation whose most recent cause is the
  same rule's firing — including a sibling branch activated because
  the consequence inserted (or modified) the fact its `exists`
  needed. Seine lost the causing rule's identity on that path (the
  newly-satisfied branch's join was filled fresh, with no origin), so
  the sibling fired once more than Drools. Separate rules, external
  or foreign-rule insertions, and later matches over the old blocker
  are unaffected (certified by controls). Ten scenarios graduated to
  the certified corpus, closing a long-quarantined fuzz witness.

- **Activations born from a consequence's mixed inserts and modifies now
  fire in effect order.** A rule whose consequence both inserts facts
  and modifies existing ones re-activates a single-pattern rule's
  matches in the order the effects touched them, as Drools does
  (matches materialize per propagation); Seine previously batched the
  whole consequence and fired all modify-born re-activations before
  insert-born ones. Consequences whose effects are all inserts or all
  modifies were already correct and are unchanged. Eight scenarios
  graduated to the certified corpus.

- **Rule selection after `setFocus` to a quiet agenda group now matches
  Drools.** Drools' property-reactivity bypass makes a modify touch
  every rule whose constraints don't listen to the changed fields —
  queueing those rules' agenda items even when no fact of theirs
  changed — and an executor firing `setFocus` yields to any such item
  in the focused group. Seine's stateful agenda missed those phantom
  item wakes (and conversely woke rules on alpha-exits, which Drools
  never does), so equal-salience rules re-activated by a `modify`
  could fire later than Drools fires them. Eleven scenarios graduated
  to the certified corpus, closing a fuzz witness and a
  model-discovered divergence family.

- **A rule that deletes and re-inserts an event witnessed by another
  rule's `exists`/`not` now re-fires that rule, matching Drools.**
  When a rule's consequence deletes an event that supports another
  rule's `exists` (or gates its `not`) and then inserts a
  replacement, the support genuinely drops to zero and re-establishes
  — the witnessing rule re-fires. Seine previously coalesced the
  delete/insert pair so the churn was invisible. Re-inserting before
  deleting (support never reaching zero) still coalesces, as in
  Drools. Five scenarios graduated to the certified corpus, closing
  the last value-class witness of the CEP delete-churn family.

- **Firing order after a mid-chain temporal `not` releases now matches
  Drools.** When a temporal `not` sits between positive patterns and
  its window closes (no blocker ever arrived), the rule's pending
  matches fire in the order they would have been created had the
  `not` never gated them — interleaved across anchors and following
  each event's arrival — where Seine previously re-derived them at
  the release and fired them scrambled by the network's staging
  order. Rules whose temporal `not` is the last pattern were already
  correct and are unchanged. Seven scenarios graduated to the
  certified corpus, closing a witness banked since the CEP arc.

- **`average()` over a decimal field is now a compile error** steering
  to `average_exact` — money never meets floats. Previously the engine
  silently coerced decimal contributions through IEEE double (and
  Drools coerces them differently again: BigDecimal at the running
  sum's scale with banker's rounding, firing `0` on an empty window —
  measured, neither semantic is what a money average should quietly
  do). Averages of per-diem rates, interest rates, and prices now
  require an explicit scale and rounding mode via
  `average_exact(field, scale=..., rounding=...)`. `average()` over
  `int`/`float` fields is unchanged.

- **`average_exact` now works with windows.** The windowed authoring
  fence is lifted: `accumulate(..., agg=average_exact(...),
  window=window_time(ms)/window_length(n))` is certified — window
  eviction refolds the running sum and count exactly (subtract-based,
  no drift), the result re-rounds to the spelled scale and mode at
  every firing, an emptied window blocks propagation (like
  `average`), and a null contribution occupies its window slot while
  counting toward neither sum nor count. Certified value-for-value
  against Drools' explicit `sum/count` + `BigDecimal.divide` spelling
  across eviction churn in both window kinds.

- **Firing order for events held on unlinked stream paths now matches
  Drools.** Drools' per-insert stream flush rides event-typed inputs:
  event facts reach a join's memory in arrival order even while the
  rule's network path is unlinked, where Seine walked the whole held
  batch newest-first at the eventual evaluation. Plain (non-event)
  facts in stream sessions keep the certified accumulate-then-LIFO
  order — the distinction is the fact's event-ness, not the fire
  boundary. Five scenarios graduated to the certified corpus.

- **Firing order for intermediate matches cascading through a chain of
  temporal joins now matches Drools.** When one event's arrival
  completes matches at a temporal join whose downstream temporal join
  cannot yet fire (its own events expired or not yet arrived), the
  intermediate matches were handed downstream without the per-hop
  staging reversal Drools' propagation applies — the eventual firings
  came out reversed. Eight scenarios graduated to the certified
  corpus, closing the last banked witness of the D-318 fuzz family
  (cf318902x167).

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
