# Memo: a deterministic CEP subset as a TMS special case (D-099)

Status: INVESTIGATION — the P3 roadmap row, memo-first per D-079.
No implementation. Sources read: drools-core 9.44.0.Final
(SlidingTimeWindow, WindowNode, ObjectTypeNode expiration,
PseudoClockScheduler, DefaultTimerJobInstance, ExpireJob/
WorkingMemoryReteExpireAction paths).

## 1. The thesis, tested against the source

The framing "CEP is a special case of TMS" holds SEMANTICALLY and
fails MECHANICALLY — and both halves are useful:

- **Semantically**: an event with `@expires(30s)` inserted at t is a
  fact whose existence is justified by `clock < t + 30s`; advancing
  the clock past the deadline breaks the justification and the fact
  (plus everything derived from it) must go. That is exactly the
  TMS's logical-retraction contract with a time-valued justifier.
- **Mechanically**: Drools does NOT route expiration through its
  TMS. `SlidingTimeWindow.expireFacts` and the `@expires` ExpireJob
  both call `ObjectTypeNode.doRetractObject(...)` — the ordinary
  retraction path, identical in kind to an external delete. The
  TruthMaintenanceSystem is never consulted.

**Consequence for Seine**: the faithful port is NOT new TMS
machinery. It is a deterministic RETRACTION SCHEDULER (a
deadline-ordered queue) driving the ALREADY-CERTIFIED delete
cascade. The TMS connection materializes for free at one remove:
when a rule `insertLogical`s facts justified by events, an event's
expiration triggers the certified D-076/D-083 TMS cascade with no
new code. CEP-as-TMS is the right REVIEW LENS (it predicts the
composition behaviors we must probe); the retraction scheduler is
the right IMPLEMENTATION.

## 2. Reduction table (mechanism -> Seine substrate)

| Drools CEP mechanism (9.44 source) | Reduces to | New? |
|---|---|---|
| `@expires` / inferred expiration (`ObjectTypeNode.expirationOffset`, merged by MAX; ExpireJob -> WorkingMemoryReteExpireAction -> doRetractObject) | Scheduled WM-WIDE delete at `t_insert + offset`, composing through the certified delete cascade + D-047 action ordering + TMS cascade for dependent logical facts | Scheduler only |
| `over window:time(N)` (WindowNode: **`cloneAndLink()` — the behavior queue holds a CLONE**; SlidingTimeWindow = FIFO queue + head-scheduled job; expiry retracts THE CLONE) | Scheduled PER-WINDOW-SUBTREE retraction: the event unmatches that pattern's subtree but stays in the WM for other rules until OTN expiration | Window membership structure + scheduler |
| `over window:length(N)` (SlidingLengthWindow) | Count-based FIFO eviction on the window structure — no clock at all | Window structure |
| Temporal operators (`after[a,b]`, `before`, `coincides`, `during`, `meets`, `overlaps`, `starts`, `finishes` + negations) | A CLOSED family of interval tests over (start, end) pairs. For POINT events (duration 0) the whole family collapses to delta-range checks: `B(this after[1s,5s] $a)` = `b.ts - a.ts ∈ [1000, 5000]`. Specialized `Test::Temporal{op, lo, hi}` variants — **no general constraint arithmetic needed**; range-indexable via the existing D-032 machinery | Test variants |
| Pseudo-clock `advanceTime` (PseudoClockScheduler.runCallBacksAndIncreaseTimer: pops due jobs IN FIRE-TIME ORDER, **sets the clock to each trigger's own fire time** before executing, then to end time) | Drain our deadline queue in order; each batch = retractions + certified propagation + agenda evaluation at that intermediate instant. Mid-advance states are REAL and observable (an expiration at t=5 during advance-to-10 runs with clock=5) | Clock + drain loop |
| STREAM mode + fireAllRules-per-advance | Our multi-fire lifecycle (D-046/D-047/D-091) with `advance` as a new external action kind | Scenario action |
| `@timestamp(field)` | Read the event's timestamp from a declared i64 field — fully deterministic, no ingestion clock | Schema metadata |

## 3. Determinism analysis (the fence-relevant findings)

1. **Equal fire-time ties are UNSPECIFIED in Drools.**
   `DefaultTimerJobInstance.compareTo` compares the fire Date ONLY,
   and `java.util.PriorityQueue` is a binary heap with no stable
   tie-break — two expirations due at the same instant run in
   heap-shape-dependent order. This is a D-035/D-052-class
   unspecified-order surface. Options, in preference order:
   (a) probe whether the order is empirically stable for realistic
   shapes (same-JVM heap behavior often is) and pin if so;
   (b) fence: "distinct expiry instants per advance" in the
   certified subset, with the generator enforcing it;
   (c) if tied batches prove stable-by-accident, reproduce; if not,
   quarantine tied-instant scenarios as unspecified-order.
2. **Mid-advance clock visibility is part of the spec.** The clock
   is rolled to each job's fire time before its callback — rules
   firing from an expiration at t=5 observe t=5, not the advance
   target. Our drain loop must do the same (natural for a
   deadline-ordered queue).
3. **Inferred expiration is a compile-time MAX** over temporal
   reach and window sizes (merged across OTN users). Deterministic,
   reproducible — but the exact inference rules need oracle probes
   (the D-083 lesson: fuzz-gate the discriminators).
4. **`@timestamp` from a field removes all wall-clock dependence.**
   The primary subset should REQUIRE it (point events, explicit i64
   timestamps). Insertion-clock timestamps are a secondary,
   still-deterministic-under-scripted-advances extension.

## 4. Differential-oracle feasibility

Fully scriptable: KieHelper + `EventProcessingOption.STREAM` +
`ClockTypeOption.PSEUDO`; epochs gain `{"advance_ms": N}` actions
mapping to `session.getSessionClock().advanceTime(N, MILLISECONDS)`;
scenario types gain `"event": {"timestamp": "ts", "expires_ms": N}`
metadata rendered as `@role(event) @timestamp(ts) @expires(...)`
declarations. Canonical output extends unchanged — expired events
VANISH from the final WM dump, which is exactly the observable that
separates window-unmatch (fact survives) from expiration (fact
gone). The RunnerDump/AccDump graft pattern extends to WindowNode
memories if ladders need ground truth.

## 5. Thesis fit (why this is worth promoting past P3)

The financial-decisioning domain is time-window-native: 30/60/90-day
delinquency buckets ARE `window:time` constructs; payment-sequence
rules (`Payment after[0d,30d] Statement`) ARE point-event temporal
joins; rate-lock and cure-period logic ARE expirations. The
deterministic subset (pseudo-clock, field timestamps) is not a
compromise for the domain — batch decisioning over event histories
is exactly how these systems replay. And the D-095 ecosystem axis
composes: event timestamps arrive as Arrow timestamp columns
(epoch-i64 — the P3 date-type row shares this encoding).

## 6. Proposed fences (initial)

OUT of the deterministic subset: wall clock / realtime mode;
`fireUntilHalt` (threading); entry points (streams multiplexing —
its own feature); `@duration` (interval events) initially — point
events collapse the temporal family and defer the (start,end)
algebra; cron/interval RULE timers (`timer(...)` attributes — a
different feature family); equal-instant expiration ties pending
the 3(a) probe.

## 7. Phased plan (future arc, gated on Bryan's go)

- **E0**: oracle plumbing (STREAM + pseudo-clock + advance actions)
  + the probe ladder: tie-order stability (3.1), mid-advance firing
  composition vs D-047/D-091 (agenda state across intra-advance
  batches), window-clone scope (fact survives other rules —
  §2 row 2), inferred-expiration rules, expiration × TMS cascade
  (an event justifying an insertLogical chain), expiration × D-076
  defer-drain (the strictly-higher gate at expiration boundaries).
- **E1**: point events + `@expires` + `after/before` delta-range
  tests + the deadline queue; differential + fuzz gate (generator
  draws advance schedules; distinct-instant fence enforced).
- **E2**: `window:time` / `window:length` (clone-scoped membership).
- **E3**: the rest of the temporal family; revisit `@duration`.

Estimated new-machinery surface: the clock + BTreeMap deadline
queue, window membership structures, ~8 temporal Test variants,
schema/scenario/oracle metadata — small relative to the reuse
(delete cascade, TMS cascade, D-091 lifecycle, D-032 indexes,
multi-fire composition all land for free).

## 8. Recommendation

Promote to a P2 arc with E0 as its own supervised recon phase (the
probe ladder above), keeping the memo's fences until probes say
otherwise. The reduction is favorable: CEP's hard parts in Drools
(threading, wall clocks, session lifecycles) are exactly what the
deterministic subset excludes, and what remains is a scheduler over
machinery we have already certified twice over.
