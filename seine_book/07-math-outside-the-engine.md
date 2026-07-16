# Math outside the engine

The [last page](06-temporal-patterns.md) ended on a refusal: the match
language has no arithmetic. You can compare a field to a value or to another
field, but you cannot compute inside a `when` — no `lat1 - lat2`, no distance
formulas, no derived quantities in the pattern. This page is about why that's
a feature, and about the door the library gives you instead. If you know your
way around a dataframe, you're about to feel at home.

## Why refuse arithmetic?

Because every behavior the match language accepts is certified — checked, byte
for byte, against a reference implementation. Formulas inside patterns would
be arbitrary code the certification can't reach, and engines that allow it end
up with business math buried in match conditions, invisible to tests that
don't happen to trigger the rule. The engine would rather hold a hard line:
**patterns match on fields; they never compute them.**

But real rules need computed values. Proximity alerting needs a distance in
meters, and no aircraft transmits its distance to every other aircraft —
positions come in, distance is *derived*.

## The two-plane pattern

The answer is to split the work into two planes:

- **The derivation plane** computes. It's columnar math over batches of rows —
  dataframe thinking — that turns raw inputs into **honest fields**: a `dist`
  column that plainly *is* the distance, a `closing` flag that plainly *is*
  "nearer than last time."
- **The match plane** decides. Rules constrain those fields exactly like every
  other page of this book: `Pair.dist < 5000` is just a field comparison. The
  engine never knows a formula existed.

Derive, then assert, then fire. The math happens *before* working memory, in a
place you can test like any pure function.

## The kernels

`seine_rs.derive` ships the columnar pieces, and they compose like dataframe
operations — tables in, tables out:

```python
import seine_rs as s

positions = {                      # one batch of raw rows, as columns
    "icao": ["AC1", "AC2", "AC3"],
    "lat":  [40.0, 40.0, 41.0],
    "lon":  [0.00, 0.04, 2.00],
}

cand = s.derive.pair_candidates(positions, id="icao", radius_m=25_000)
withd = s.derive.haversine(cand, lat1="lat_a", lon1="lon_a",
                           lat2="lat_b", lon2="lon_b", out="dist")

print([(r["key"], r["dist"]) for r in withd.to_pylist()])
# [('AC1|AC2', 3407)]
```

`pair_candidates` cross-joins the batch against itself and prunes pairs that
can't possibly be within 25 km (it's careful about the places naive pruning
goes wrong — the antimeridian, the poles); `haversine` adds a great-circle
distance column in meters. AC3 never survives the prune. What's left is a
table of **facts with honest fields**, ready to assert:

```python
@s.fact
class Near:
    key: str
    dist: int

@s.fact
class Callout:
    key: str

r = s.Rule("close-pair")
n = r.when(Near, Near.dist < 5000)          # plain field constraint —
r.then_insert(Callout, key=n.key)           # the formula is long gone

sess = s.Session([r], facts={Near: [], Callout: []})
sess.fire()
for row in withd.to_pylist():
    sess.insert_row(Near, {"key": row["key"], "dist": row["dist"]})

print([x["key"] for x in sess.fire().derived["Callout"].to_pylist()])
# ['AC1|AC2']
```

There's also `closing`, which compares each pair's distance against the
previous batch's — cross-batch state the *caller* owns, in a plain dict it
hands back on every call. The kernels themselves keep no state at all.

## The loop, and why replay still works

Put it in motion and each tick of a live system is the same four steps:

```
raw rows  ->  derive  ->  assert  ->  fire
```

Everything in that pipeline is deterministic: the kernels are pure functions
of the batch (plus caller-owned state), and the engine is a pure function of
the asserted sequence and the clock. So if you log just the **raw** inputs and
the clock advances, you can replay the log and re-derive your way to
byte-identical alerts. Store raw, recompute the rest — the derivation plane
doesn't weaken the reproducibility story from the [time](05-time-and-events.md)
page; it joins it.

Facts the *rules* produce come back out the same door. `fire()` returns
derived facts as tables — the same shape the kernels accept — so engine output
can flow into kernels and back in as next tick's input. The planes stay
separate; the seam is always the tick boundary, never the middle of a match.

## Where the trust comes from

One honest question remains: the match plane is certified against a reference
— what certifies the *math*? Not the engine. The kernels carry their own
oracle: an independent plain-Python reference implementation and a property
battery that checks the fast columnar path against it, including the
adversarial geometry (pairs straddling the antimeridian, pairs near the
poles). Same posture, different referee: every plane answers to an
independent implementation of itself.

For a complete working system on this pattern — live feed, alerts, write-ahead
log, replay proving byte-identical outputs — read `demo/adsb_convergence.py`
in this repository top to bottom. It's under 250 lines, and after these seven
pages, every line of it should read as something you've already met.

---

That completes the arc. Four core ideas — [facts](01-facts.md),
[rules](02-rules.md), the [agenda](03-agenda.md), and
[truth maintenance](04-truth-maintenance.md) — then [time](05-time-and-events.md)
and [temporal patterns](06-temporal-patterns.md) over them, and finally the
[two-plane split](07-math-outside-the-engine.md) that keeps computation out of
matching without giving it up.
