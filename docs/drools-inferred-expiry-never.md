# Upstream note — inferred `@expires` silently resolves to NEVER (event leak footgun)

**NOT YET FILED** (draft, 2026-07-07). Found by differential testing
against the Seine engine (D-109 in this repository's DECISIONS.md). Unlike
the min/max defect ([kie-issues#2366](https://github.com/apache/incubator-kie-issues/issues/2366)),
this is very likely **intended behavior** — the note is framed as a
*surprising-behavior / documentation* question for upstream, not a bug
claim. Seine reproduces it faithfully (Drools is the spec for engine
semantics); we file it because the memory-growth consequence is a footgun
users hit without any diagnostic.

**Affected versions:** verified on 9.44.0.Final; the relevant code
(`PatternBuilder.attachObjectTypeNode`, `TemporalDependencyMatrix.
getExpirationOffset`) is unchanged on current `main`, so all recent
releases behave identically.

**Component:** drools-core — `org.drools.core.reteoo.builder.PatternBuilder`,
`org.drools.core.time.TemporalDependencyMatrix`.

## Summary

When an event type has **no explicit `@expires`**, its expiration offset is
*inferred* from the temporal constraints (`this after/before[lo,hi] $x`).
In several natural rule shapes the inferred offset silently resolves to
**NEVER_EXPIRES (-1)** — the events of that type are then retained in
working memory **forever**, even in STREAM mode with a pseudo-clock that
advances far past any plausible temporal window. There is no warning, and
`KieSession` memory grows unbounded. Adding an explicit `@expires` (which is
"hard" and immune to this) is the only fix.

The three shapes we observe (all with STREAM + pseudo-clock, event type `E`
with **no** `@expires`):

1. **Bare reference.** `E` appears in *any* pattern that carries no temporal
   constraint — a plain `E()`, a `not E()`, or an `exists E()` — in addition
   to (or instead of) a temporal one. That bare pattern forces `E` to NEVER,
   even if another rule constrains it temporally.

2. **Backward-only temporal.** `E` is only ever the *later* operand of a
   strict `after[lo,hi]` with `lo > 0` (or the earlier operand of
   `before[lo>0,hi]`). Its inferred offset is `-lo < 0` → NEVER. Note the
   discontinuity at `lo`: `after[0,hi]` yields offset 0 (expires promptly),
   `after[1,hi]` yields NEVER.

3. **Self-join.** `rule r when $a: E() $b: E(this after[50ms,100ms] $a)` —
   the `$b` (later) side contributes a negative row value → the *type* `E`
   becomes NEVER, so neither `$a` nor `$b` events ever expire.

## Mechanism

`TemporalDependencyMatrix.getExpirationOffset(pattern)` takes, over the
transitively-closed temporal-distance matrix, the **max upperBound** of the
pattern's row and returns `NEVER_EXPIRES` whenever that max is `< 0`
(a purely-backward reach) — or when the pattern has no temporal neighbour at
all (a bare pattern, size-1 matrix). Then in
`PatternBuilder.attachObjectTypeNode`, the non-hard branch does:

```java
long distance = context.getExpirationOffset( pattern );
if ( distance == NEVER_EXPIRES ) {
    otn.setExpirationOffset( offset );   // OVERWRITES — not max()
} else {
    otn.setExpirationOffset( Math.max( distance, offset ) );
}
```

Because the `distance == NEVER_EXPIRES` branch **overwrites** the shared
`ObjectTypeNode`'s offset (rather than max-ing), a single non-forward
pattern of a type resets that type's expiration to NEVER regardless of the
order patterns are attached or of finite offsets from other rules. Explicit
`@expires` takes the earlier `if (expirationSpec.hard)` branch and is never
reached by this overwrite — which is why the workaround works.

## Reproduction

```drl
declare E @role( event ) @timestamp( ts ) end   // no @expires

rule temporal when
    $a : E() $b : E(this after[50ms, 100ms] $a)  // self-join, lo>0
then end
```

1. STREAM mode, `SessionPseudoClock`.
2. `insert` an `E` at ts = 0; `clock.advanceTime(1_000_000, MS)`.
3. Observe: the `E` fact is **still in working memory** (never expired).
   Replace the pattern with an explicit `@expires(101ms)` on `E` and the
   same event is gone after the advance.

(Analogous repros for the bare-reference and backward-only shapes are in
`scenarios/probes/pr_cep_inf_*` of the Seine repository.)

## Question for upstream

Is the NEVER-on-negative-reach + overwrite behavior intended? If so, a
build-time **warning** ("event type `E` has an inferred expiration of NEVER;
its events will accumulate in working memory — add `@expires` to bound it")
would save users from a silent unbounded-memory condition. If the overwrite
(vs `Math.max`) in the `distance == NEVER_EXPIRES` branch is not intended,
max-ing there would let a finite offset from another rule survive a bare
reference.
