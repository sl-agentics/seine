# Upstream bug report — Drools accumulate min/max stale result

**FILED: https://github.com/apache/incubator-kie-issues/issues/2366**
(2026-07-07, status open at filing time). Found and analyzed 2026-07-06
by differential testing against the Seine engine (D-092 in this
repository's DECISIONS.md). The text below is the filed content.

---

**Title:** `accumulate` with `min`/`max` returns a stale result after a
left-tuple update: the refold is skipped whenever the extremum's removal
is not the last "dirtying" step of the left-update merge

**Affected versions:** verified on 9.44.0.Final and 10.1.0 (latest
release line at time of writing); the relevant code on current `main`
is identical, so all intermediate releases are expected to be affected.

**Component:** drools-core — `org.drools.core.phreak.PhreakAccumulateNode`

## Summary

When a left tuple of an `accumulate( ...; min(...) )` (or `max`) node is
UPDATED such that the source match holding the current extremum no
longer satisfies a beta constraint, the node removes the match from the
match list but — depending only on right-memory iteration order — skips
the re-accumulation. The accumulate then keeps returning the removed
extremum indefinitely: the working memory reaches quiescence with a
match set and a result that contradict each other. Reversible functions
(`sum`, `count`, `average`) are unaffected.

## Minimal reproducer (KieHelper, declared type; no POJOs needed)

```java
String drl =
    "package repro;\n" +
    "declare P b : long end\n" +
    "declare S v : long end\n" +
    "declare G g : long end\n" +
    "rule R_acc when\n" +
    "    $p : P($b : b)\n" +
    "    accumulate( S(v >= $b, $s : v); $m : min($s) )\n" +
    "then\n" +
    "    System.out.println(\"min for b=\" + $b + \" -> \" + $m);\n" +
    "end\n" +
    "rule R_low salience -5 when $g : G() $p : P()\n" +
    "then modify($p){ setB(5) } end\n";

KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
KieSession ks = kbase.newKieSession();
FactType P = kbase.getFactType("repro", "P");
FactType S = kbase.getFactType("repro", "S");
FactType G = kbase.getFactType("repro", "G");

Object p = P.newInstance(); P.set(p, "b", -10L);
Object s12 = S.newInstance(); S.set(s12, "v", 12L);
Object sm2 = S.newInstance(); S.set(sm2, "v", -2L);
Object g = G.newInstance(); G.set(g, "g", 1L);

ks.insert(p);
ks.insert(s12);   // NOTE: insertion order matters — see analysis
ks.insert(sm2);
ks.insert(g);
ks.fireAllRules();
```

**Expected output:**
```
min for b=-10 -> -2        // initial: {12, -2}, min -2 — correct
min for b=5   -> 12        // after modify: only {12} matches
```

**Actual output (9.44.0.Final and 10.1.0):**
```
min for b=-10 -> -2
min for b=5   -> -2        // stale: -2 no longer satisfies v >= 5
```

Swapping the two `S` insertions (insert `sm2` before `s12`) makes the
bug disappear — the result depends on right-memory iteration order, not
on the rule semantics.

The internal state at quiescence is self-contradictory (observable by
reflecting into the node's `BetaMemory`): the left tuple's match list
correctly contains only `S(v=12)`, while the accumulation function
context still holds `min = -2` and the result fact still carries `-2`.
Nothing ever corrects it.

## Root cause

`PhreakAccumulateNode.doLeftUpdatesProcessChildren` (the same-bucket
left-update merge) walks the right memory against the left's match
list, and defers all re-accumulation to a single end-of-walk call gated
on a local `isDirty` flag. Two properties interact:

1. Every `removeMatch(...)` inside the walk is called with
   `reaccumulate = false`, so the per-removal refold inside
   `reaccumulateForLeftTuple` is a no-op; correctness depends entirely
   on the end-of-walk gate.
2. `isDirty` is ASSIGNED per walk element, last-writer-wins:
   - removal arm: `isDirty = !reversed;` — and for min/max,
     `MinMaxAccumulateFunction.tryReverse` returns
     `data.min.compareTo(value) < 0`, i.e. it "succeeds" (a no-op
     reverse) for every non-extremal removal and fails only when the
     removed value IS the current extremum;
   - kept-match arm: `isDirty = accumulate.hasRequiredDeclarations();`
     — `false` for all built-in functions;
   - newly-allowed arm (`addMatch`): no write.

Consequently, when the extremum's removal is followed in the same walk
by any kept match or any non-extremal removal, the final `isDirty` is
`false` and the refold never runs. In the reproducer the memory
iteration order is `[-2, 12]`: the extremum `-2` is removed
(`isDirty = true`), then `12` is kept (`isDirty = false` — clobbered),
end of walk, no refold.

The right-side paths (`doRightDeletes` etc.) and the indexed
bucket-change path pass `reaccumulate = true` and are correct; the
defect is exclusive to the left-update merge. `sum`/`count`/`average`
reverse inline in `tryReverse` and are always correct — which is what
lets this defect hide: the match bookkeeping is right, only the
non-reversible fold goes stale.

## Suggested fix

Accumulate the flag instead of assigning it, e.g.:

```java
isDirty |= !reversed;                                  // removal arm
isDirty |= accumulate.hasRequiredDeclarations();       // kept arm
```

(or equivalently: track "any non-reversed removal happened" in a
separate boolean and OR it into the final condition). With the flag
accumulated, the end-of-walk `reaccumulateForLeftTuple(..., true)` runs
whenever an extremum was removed, restoring `min`/`max` correctness at
unchanged cost for the common paths.

## Additional discriminating cases (all verified on both versions)

| memory walk (allowed?) | result |
|---|---|
| remove extremum, keep | stale (bug) |
| keep, remove extremum (last) | correct |
| remove extremum, keep, remove non-extremal | stale (bug — the trailing removal "reverses" as a no-op and clears the flag) |
| keep, remove extremum, keep | stale (bug) |
| remove extremum, newly-add | correct (the add arm doesn't clear the flag) |
| any of the above with `sum` | correct (reversible) |
