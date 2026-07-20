# Upstream bug report — accumulate rule goes permanently deaf after its source empties (subnetwork shapes)

**Status: DRAFT (2026-07-20, not yet filed).** Found and analyzed by
differential testing against the Seine engine (D-372 in this
repository's DECISIONS records; witness `fz_360001_381`, law grid in
`probes_pending/notrel/PINS.md`). Reproducer verified against vanilla
9.44.0.Final and 10.1.0 (both classpaths, 3× stable). Predecessor
report of this kind: `docs/drools-bug-stale-minmax.md` (filed as
incubator-kie-issues#2366, fixed upstream in one day, PR#6796).

---

**Title:** A rule with `accumulate` followed by a subnetwork CE never
re-activates after its accumulate source empties and is repopulated —
the RuleAgendaItem is never re-queued

**Affected versions:** verified on 9.44.0.Final and 10.1.0 (latest
release line at time of writing). Behavior identical on both.

**Component:** drools-core — PHREAK rule-agenda-item dirty
notification / segment memory (`RuleAgendaItem` queueing on staged
propagation into an accumulate segment)

## Summary

When a rule's LHS is an `accumulate(...)` followed by a CE that
compiles to a **subnetwork** (e.g. `not( C() and C(x == 5) )`), the
rule stops being evaluated forever once its accumulate **source
becomes empty**: any later repopulation of the source stages into the
network but never re-queues the rule's `RuleAgendaItem`, so the
executor never evaluates the network, the reborn accumulate result is
never seen, and the rule silently never fires again (until some
unrelated event happens to notify the same path).

The defect is **self-inconsistent**: with `C` never populated,
`not( C() and C(x == 5) )` and `not C()` are semantically equivalent —
yet the first form loses the firing and the second fires correctly.
Only the network **structure** (the subnetwork's segment split)
differs.

## Minimal reproducer (KieHelper, declared types; no POJOs needed)

```java
import org.kie.api.KieBase;
import org.kie.api.io.ResourceType;
import org.kie.api.runtime.KieSession;
import org.kie.api.definition.type.FactType;
import org.kie.internal.utils.KieHelper;

public class Repro {
    static void run(String label, String notCe) throws Exception {
        String drl =
            "package repro;\n" +
            "declare B v : long end\n" +
            "declare C x : long end\n" +
            "rule \"observer\" salience -3 when\n" +
            "    accumulate( B($v : v); $m : max($v) )\n" +
            "    " + notCe + "\n" +
            "then System.out.println(\"  OBSERVER max=\" + $m); end\n" +
            "rule \"sweeper\" salience -10 when $b : B() then delete($b); end\n";
        KieBase kbase = new KieHelper().addContent(drl, ResourceType.DRL).build();
        KieSession s = kbase.newKieSession();
        FactType bt = kbase.getFactType("repro", "B");
        Object b7 = bt.newInstance(); bt.set(b7, "v", 7L);
        s.insert(b7);
        int f1 = s.fireAllRules();
        Object b11 = bt.newInstance(); bt.set(b11, "v", 11L);
        s.insert(b11);
        int f2 = s.fireAllRules();
        System.out.println(label + ": fire1=" + f1 + " fire2=" + f2);
        s.dispose();
    }
    public static void main(String[] args) throws Exception {
        System.out.println("-- variant A: not( C() and C(x == 5) )  [subnetwork]");
        run("A", "not( C() and C(x == 5) )");
        System.out.println("-- variant B: not C()  [no subnetwork - control]");
        run("B", "not C()");
    }
}
```

No `C` fact is ever inserted; both `not` forms are trivially open the
whole time. The sequence per variant:

1. insert `B(7)`; `fireAllRules()` — the observer fires (`max=7`),
   then the sweeper (lower salience) deletes `B(7)`. The accumulate
   source is now **empty**; the staged source-delete is left
   unprocessed at quiescence (harmless in itself — the observer
   already fired).
2. insert `B(11)`; `fireAllRules()` — the source is repopulated,
   `max` should be re-derived as 11 and the observer (salience −3,
   above the sweeper's −10) should fire before the sweeper.

## Expected vs. actual

Expected (both variants):

```
  OBSERVER max=7
fire1=2
  OBSERVER max=11
fire2=2
```

Actual (9.44.0.Final and 10.1.0, identical):

```
-- variant A: not( C() and C(x == 5) )  [subnetwork]
  OBSERVER max=7
A: fire1=2 fire2=1        <-- the max=11 firing is LOST
-- variant B: not C()  [no subnetwork - control]
  OBSERVER max=7
  OBSERVER max=11
B: fire1=2 fire2=2
```

In variant A the second `fireAllRules` runs only the sweeper. The
observer's activation for `max=11` is never created — not created-
then-cancelled: an `AgendaEventListener` sees **no matchCreated at
all** for the observer in the second cycle.

## Analysis (instrumented observation)

Dumping the observer's `PathMemory`/`RuleAgendaItem` state after every
working-memory event (reflection on a live session) shows, throughout
the entire second cycle in variant A:

```
RTN observer  linkedSegmentMask=3/3  item[queued=false dirty=false]
```

- The path stays **fully linked** the whole time (`3/3` — the
  accumulate segment never unlinks; accumulate nodes, like `not`
  nodes, can produce output with an empty source). This is **not** a
  linking loss.
- The `RuleAgendaItem` is **never re-queued** when `B(11)` propagates
  into the accumulate's right input, so `evaluateNetwork` never runs
  for the rule; the staged insert sits unprocessed in the segment.

Boundary facts, each verified with its own cell:

- If the source never empties (a second `B` fact keeps it populated),
  intermediate re-derivations notify and fire normally — the
  suppression is specific to the **empty → non-empty** transition.
- A **plain pattern** or a **bare `not C()`** after the accumulate
  (no subnetwork, hence no segment split — the terminal shares the
  accumulate's segment) does not exhibit the problem.
- The subnetwork need not be data-relevant: `C` is never inserted in
  the reproducer, and the deafness still occurs. Structure alone
  triggers it.
- The emptiness that matters is the accumulate node's **beta (right)
  memory**, not the fact type's population: a permanent `B` fact
  excluded by an alpha constraint on the accumulate source (e.g.
  source `B(v != 0, ...)` with a resident `B(0)`) does NOT prevent
  the deafness; a permanent fact that **enters the aggregation**
  does.

Consistent reading: the accumulate source's empty↔non-empty
transitions are routed through the node link/unlink notification path,
which is a no-op for a segment whose linked mask does not change (the
accumulate segment is always linked) — so the "queue the
RuleAgendaItem" side effect is swallowed. With the terminal in the
same segment (no subnetwork) a different/additional notification
reaches the item and the firing survives; the subnetwork's segment
split removes that path. We have not pinpointed the exact source line;
the instrumented observations above are the measured facts.

## Impact

Any rule of the shape `accumulate(...)` + subnetwork-CE whose
aggregated source can drain and refill (batch/windowed processing,
periodic sweeps, TMS-maintained sources) silently stops reacting after
the first drain. There is no error and the working memory itself is
correct — the rule is simply never evaluated again, which makes this
hard to notice and diagnose in production.

## Workarounds (each verified against 9.44.0.Final)

- Restructure the trailing CE to avoid a subnetwork where an
  equivalent non-subnetwork form exists (variant B above), or
- keep a sentinel fact **inside the aggregation** so the accumulate's
  right memory never empties (verified: a resident `B(-999)` with
  `max` restores the lost firing). Note this changes behavior: the
  aggregate takes the sentinel's value when the real facts drain, and
  extra firings occur at that value. A sentinel excluded from the
  aggregation by an alpha constraint does NOT work (see the last
  boundary fact above).
