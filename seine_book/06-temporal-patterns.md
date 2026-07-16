# Patterns in time

The [previous page](05-time-and-events.md) gave events a timestamp and an
expiry. This page is about constraints *between* timestamps — rules that match
sequences, not snapshots. The industry name for this style is **complex event
processing** (CEP); you don't need the name, just the two moves on this page.

## Move one: "B happened shortly after A"

A card swiped in London, then swiped in Paris twelve seconds later. Either the
card can teleport or two people have it. The DIY version keeps a dict of last
swipe per card and checks deltas by hand — workable, until you're also
expiring old swipes and handling three cards interleaved.

The rule version says it directly:

```python
from seine_rs import fact, Event, Rule, Session, this_after

@fact(event=Event(timestamp="ts", expires_ms=60_000))
class Swipe:
    ts: int
    card: str
    city: str

@fact
class Suspicious:
    card: str
    a: str
    b: str

r = Rule("far-apart-swipes")
s1 = r.when(Swipe)
s2 = r.when(Swipe, Swipe.card == s1.card,
                   Swipe.city != s1.city,
                   this_after(s1, 1, 30_000))
r.then_insert(Suspicious, card=s2.card, a=s1.city, b=s2.city)
```

The first two constraints on `s2` you've seen before — an ordinary join on
`card`, an ordinary inequality on `city`. The new one, `this_after(s1, 1,
30_000)`, reads: *this* swipe's timestamp is between 1 ms and 30 s **after**
`s1`'s. Time became just another thing patterns can constrain.

```python
sess = Session([r], facts={Swipe: [], Suspicious: []})
sess.fire()
sess.insert_row(Swipe, {"ts": 0, "card": "C1", "city": "london"})
sess.fire()
sess.advance(12_000)
sess.insert_row(Swipe, {"ts": 12_000, "card": "C1", "city": "paris"})
res = sess.fire()

print([(x["card"], x["a"], x["b"])
       for x in res.facts["Suspicious"].to_pylist()])
# [('C1', 'london', 'paris')]
```

Had the Paris swipe come 40 seconds later, no match — outside the window. Two
days later, no match *and* no memory cost — the London swipe expired long ago.
Windowed matching and expiry work together: expiry is what keeps "compare
every pair of swipes" from meaning every pair *ever*.

`this_after` has a mirror (`this_before`) and a family of relatives for
interval events — overlaps, during, meets, and the rest. They all work the
same way: a constraint between two matched events' times.

## Move two: "N of these inside a window"

The other temporal shape is a *count*: three failed logins inside a minute.
If you know dataframes, your instinct is a rolling window — that instinct is
right, and here it lives inside a rule:

```python
from seine_rs import count, window_time

@fact
class User:
    name: str

@fact(event=Event(timestamp="ts", expires_ms=120_000))
class Login:
    ts: int
    user: str
    ok: bool

@fact
class Policy:
    limit: int

@fact
class Locked:
    user: str
    fails: int

r = Rule("lockout")
u = r.when(User)
fails = r.accumulate(Login, Login.user == u.name, Login.ok == False,
                     agg=count(), window=window_time(60_000))
pol = r.when(Policy, Policy.limit <= fails)
r.then_insert(Locked, user=u.name, fails=fails)
```

`accumulate` is the engine's aggregation: it matches *all* the `Login` events
that satisfy the constraints and reduces them — here with `count()`, over only
the last 60 seconds of engine time. The result, `fails`, is a bound value like
any field, so the next pattern can compare against it and the consequence can
store it.

Notice where the threshold lives: not hard-coded in the rule, but on a
`Policy` **fact**. `Policy(limit <= fails)` is an ordinary join — which means
the lockout threshold is data. Change it by updating a fact, not by editing a
rule. That's a small idiom worth stealing for anything tunable.

```python
sess = Session([r], facts={User: [{"name": "ana"}], Login: [],
                           Policy: [{"limit": 3}], Locked: []})
sess.fire()
clock = 0
for t, ok in [(0, False), (10_000, False), (30_000, False)]:
    sess.advance(t - clock); clock = t
    sess.insert_row(Login, {"ts": t, "user": "ana", "ok": ok})
    locked = sess.fire().facts["Locked"].to_pylist()
    print(t, [(x["user"], x["fails"]) for x in locked])
```

```
0 []
10000 []
30000 [('ana', 3)]
```

Two failures: nothing. The third, still inside the window: locked. Alongside
`count()` there's `sum_`, `average`, `min_`, `max_`, and list/set collectors,
and windows also come in a by-count flavor (`window_length(n)`: the last *n*
events). A few combinations are deliberately refused — a collector over a
window, for instance, raises a compile error rather than guessing — the same
would-rather-stop-you posture as everywhere else.

## What these two moves cost

Temporal rules inherit every agenda subtlety from the timeless kind, plus one
more thing to hold in your head: matching now depends on *when* facts arrived
relative to the clock, so reproducing a bug means reproducing the event
sequence **and** the `advance` calls. Keep your feed replayable — a list of
(time, rows) you can rerun — and temporal rules stay as testable as plain
ones. You'll see that pattern built out fully on the next page.

One boundary to know exists: the engine constrains *time between events* and
*aggregates over windows*, but its match language still has **no arithmetic**
— you cannot write `Swipe.lat - s1.lat < 0.5` inside a `when`. Why that's a
deliberate refusal, and what to do instead, is the last page:
[math outside the engine](07-math-outside-the-engine.md).
