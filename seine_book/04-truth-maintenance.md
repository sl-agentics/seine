# Truth maintenance

Every conclusion so far has been permanent. A rule inserts a fact, and that fact
stays until something explicitly deletes it. That's usually what you want — but
sometimes a conclusion is only true *as long as its reason is true*, and should
disappear on its own the moment the reason goes away.

That's what **truth maintenance** does, and you opt into it by changing one word:
`then_insert` becomes `then_insert_logical`.

## The problem it solves

A temperature sensor should raise an alert while it's overheating — and the alert
should clear itself when the sensor cools. With a plain insert, you'd have to
write that cleanup yourself: one rule to raise the alert, another to notice the
temperature dropped and hunt down the alert to delete it. Two rules, and a bug
waiting in the gap between them.

With a logical insert, you write only the raising rule:

```python
from seine_rs import fact, Rule, Session

@fact
class Sensor:
    id: str
    temp: int

@fact
class Alert:
    id: str

r = Rule("overheat")
sen = r.when(Sensor, Sensor.temp > 100)
r.then_insert_logical(Alert, id=sen.id)      # logical, not plain
```

The alert is now **justified by** the match that created it. While a `Sensor`
over 100 exists, the `Alert` exists. When that stops being true, the engine
retracts the `Alert` for you — you never wrote a deletion.

## Watch it happen

```python
s = Session([r], {Sensor: [{"id": "S1", "temp": 120}], Alert: []})

def alerts():
    return [a["id"] for a in s.fire().facts["Alert"].to_pylist()]

print("start hot (120):", alerts())   # ['S1']  — justified, alert exists
s.update(0, temp=70)
print("cooled   (70): ", alerts())    # []      — justification gone, retracted
s.update(0, temp=130)
print("re-heated (130):", alerts())   # ['S1']  — justified again, alert returns
```

Nothing deleted the alert. It came and went with the truth of its justification.
That's the whole feature: **you assert the reason, the engine maintains the
conclusion.**

## Reading working memory: `facts` vs `derived`

The example above uses `.facts`, and the reason is worth pausing on, because it's
a common early stumble.

A `fire()` gives you a result with two different views:

- **`.derived`** is the *delta* — only the facts this particular `fire()` newly
  inserted.
- **`.facts`** is *all of working memory* — every fact currently present, input
  and derived alike.

To ask "does an alert exist right now?" you want the current state, so you read
`.facts`. If you read `.derived` after a fire that didn't newly create the alert,
you'll see an empty list and think it vanished when it's actually still there.
Delta versus state — reach for `.facts` when you're checking what *is*, and
`.derived` when you're checking what a run just *produced*.

## When to reach for it

The dividing question is the same one the [agenda](03-agenda.md) page ended on:

> Is this conclusion allowed to be wrong later?

- **Yes — it's a view of the current state.** An alert that should track the
  sensor, a "release eligible" flag that should vanish the instant a hold
  appears, any derived status you recompute against live data. Use
  `then_insert_logical`. The conclusion self-corrects and you write no cleanup.

- **No — it's a durable record.** An audit row, a logged decision, anything that
  must survive even after the situation that produced it changes. Use plain
  `then_insert`, and if it interacts with negation, separate the stages as the
  agenda page describes.

Neither is more correct. They're two different kinds of fact — computed state
versus written record — and picking the right one is really deciding which kind
your conclusion is.

## The tidy bonus

Because a logically-inserted fact retracts itself when its justification fails,
it also sidesteps the order-dependence trap from the agenda page. A release rule
that inserts *logically* can't strand a released title on a blocked account: the
instant a block appears, the block falsifies the release's justification and the
engine withdraws it — no matter which rule fired first. The engine's own
same-priority check knows this, and quietly permits the logical version of a
pattern it would reject in plain form.

---

That's the last concept. You now have the four ideas the engine is built on:
[facts](01-facts.md) are typed data in a shared pool, [rules](02-rules.md) read
and write that pool, the [agenda](03-agenda.md) orders what fires, and truth
maintenance lets conclusions track the truth of their reasons. Everything else
in `seine-rs` is a refinement of these four.
