# Time and events

Everything so far has been timeless. A `Person` is 34; the engine doesn't care
when you learned that, and the fact sits in working memory until something
deletes it. Plenty of data isn't like that. A sensor reading, a card swipe, a
login attempt — these happen *at a moment*, and their usefulness fades. A
reading from three days ago shouldn't trip today's alert.

An **event** is a fact with a timestamp. You declare one by telling the engine
which field carries the time:

```python
from seine_rs import fact, Event

@fact(event=Event(timestamp="ts", expires_ms=10_000))
class Reading:
    ts: int          # milliseconds — the field named above
    sensor: str
    temp: int
```

Two things changed. The `ts` field is now *meaningful to the engine*, not just
another integer. And `expires_ms=10_000` declares that a `Reading` stops
mattering ten seconds after its timestamp — the engine will remove it on its
own, the way truth maintenance removed the unjustified alert.

One rule to react to it, nothing new here:

```python
from seine_rs import fact, Event, Rule, Session

@fact
class Hot:
    sensor: str

r = Rule("hot")
rd = r.when(Reading, Reading.temp > 100)
r.then_insert(Hot, sensor=rd.sensor)
```

## The clock belongs to you

Here's the part that surprises people: the engine has a clock, but it is **not
your computer's clock**. It starts at zero and only moves when you move it:

```python
sess = Session([r], facts={Reading: [], Hot: []})
sess.fire()

sess.insert_row(Reading, {"ts": 0, "sensor": "S1", "temp": 120})
sess.fire()
sess.advance(5_000)      # five seconds pass — because you said so
```

`advance()` is relative: two `advance(5_000)` calls put the clock at 10,000.

Why would a library make you drive the clock by hand? Because it makes runs
**reproducible**. If the engine read wall-clock time, the same events replayed
tomorrow — or in a test, or in a debugger — could expire in a different order
and produce different alerts. With an explicit clock, a run is a pure function
of the sequence you fed it: same inserts, same `advance` calls, same answer,
every time. You'll feel this as mild friction in examples and as a superpower
the first time you replay a production incident locally.

## Watch a fact age out

The clock sits at 5,000 and the reading was stamped 0, so it has five seconds
left to live:

```python
def readings(res):
    return [x["sensor"] for x in res.facts["Reading"].to_pylist()]

print(readings(sess.fire()))    # ['S1']  — five seconds in, still fresh

sess.advance(6_000)             # clock now at 11,000 — past ts + 10,000
res = sess.fire()
print(readings(res))            # []      — expired, removed by the engine
print([x["sensor"] for x in res.facts["Hot"].to_pylist()])   # ['S1']
```

Look at that last line. The `Reading` is gone, but the `Hot` fact it produced
is still there — `then_insert` makes durable records, exactly as the
[truth maintenance](04-truth-maintenance.md) page divided things. If you wanted
the conclusion to age out *with* its evidence, you already know the tool:
`then_insert_logical`, and the conclusion follows its justification.

Expiry is the same "view of the present" idea applied to inputs. Working
memory stays a picture of *now*, and old events leave it without you writing
cleanup — you declare how long an event matters, once, on its type.

## What this sets up

A timestamp on a single event only buys you expiry. The real payoff is
*relating* events in time: this swipe within thirty seconds of that one, three
failures inside a minute. Those are constraints **between** timestamps, and
they're the next page: [patterns in time](06-temporal-patterns.md).
