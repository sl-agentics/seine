# The agenda

When you run an engine, several rules can match at the same moment. The
**agenda** is the engine's to-do list of matches waiting to fire, and this page
is about how it's ordered — because that order is the one thing a newcomer most
often gets surprised by.

## The loop

The engine repeats a simple cycle:

1. Find every rule whose `when` currently matches. Each match goes on the agenda.
2. Pick one match and fire it — run its `then`.
3. Firing may create or remove facts, which may add or cancel matches.
4. Go back to 1. Stop when the agenda is empty.

That empty-agenda state is called **quiescence**: nothing left to fire. A run is
finished when it goes quiet.

Because firing a rule changes the facts, and changed facts change what matches,
you cannot in general predict the whole sequence by reading top to bottom the way
you would a script. You reason about it the way the engine does: what matches
*now*, what does firing change, what matches *next*.

## When several match at once: salience

Suppose two rules both match the same order:

```python
@fact
class Order:
    id: int
    total: float
    vip: bool

@fact
class Discount:
    id: int
    pct: int

r_vip = Rule("vip-gets-20", salience=10)
o = r_vip.when(Order, Order.vip == True)
r_vip.then_insert(Discount, id=o.id, pct=20)

r_big = Rule("big-gets-10", salience=5)
o = r_big.when(Order, Order.total >= 100.0)
r_big.then_insert(Discount, id=o.id, pct=10)
```

A £250 VIP order matches both. Which fires first? The one with higher
**salience** — a priority number, default 0. Higher goes first, and you can
see it in the order the conclusions arrive:

```python
res = run([r_vip, r_big],
          {Order: [{"id": 7, "total": 250.0, "vip": True}], Discount: []})
print([(d["id"], d["pct"]) for d in res.derived["Discount"].to_pylist()])
# [(7, 20), (7, 10)]   — the salience-10 rule fired first
```

Salience is how you express "this rule matters more" without making one rule call
another. It's a knob on *priority*, not a wire between rules.

## When several match and you don't care: it's still deterministic

If two matches have equal salience, the engine still picks a definite order — you
just haven't told it which you prefer. That's fine when the rules don't interfere
with each other. It stops being fine the moment one rule's outcome depends on
whether another has fired yet. Which brings us to the sharp edge.

## The sharp edge: negating your own conclusions

Recall `when_not` from the [rules](02-rules.md) page — "fire only if no matching
fact exists." Now combine it with the fact that rules produce facts. Picture a
title-release system:

- **block rules** insert a `Decision` of "BLOCK" when an account has a hold, is
  in an NSF window, still owes a balance, etc.
- a **release rule** fires `when_not(Decision)` — release the title only if
  *nothing* has decided against it.

Here's the trap. The release rule asks "has anything blocked this **yet**?" — and
"yet" depends entirely on whether the block rules have already fired. If a block
rule and the release rule sit at the same priority, the answer comes down to
which the engine happened to pick first. Same rules, same data, and flipping the
order they were declared can flip a blocked account to released:

```
block declared first  -> BLOCK           (correct)
release declared first -> BLOCK, RELEASE  (a title released on a blocked account)
```

That is a real, silent, order-dependent wrong answer, and it's the kind of thing
that's very hard to spot by reading the rules.

## The engine catches this for you

`seine-rs` runs a compile-time check for exactly this pattern. If a rule negates
a fact type that another rule inserts *at the same priority*, it refuses to
compile and tells you why. Here's the shape it refuses, cut to the bone:

```python
@fact
class Acct:
    acct_id: str
    status: str

@fact
class Decision:
    acct: str
    verdict: str

block = Rule("block-bankruptcy")
b = block.when(Acct, Acct.status == "BANKRUPT")
block.then_insert(Decision, acct=b.acct_id, verdict="BLOCK")

release = Rule("release")
a = release.when(Acct, Acct.status == "PAIDOFF")
release.when_not(Decision, Decision.acct == a.acct_id)
release.then_insert(Decision, acct=a.acct_id, verdict="RELEASE")

compile_rules([block, release])     # raises CompileError
```

> rule "release" negates Decision, but rule "block-bankruptcy" inserts Decision
> in the default agenda group at salience 0 — the negation may be evaluated
> before that insert, so this rule's outcome depends on the order rules were
> declared.

The message then lays out the ways to fix it. All of them come down to one real
question: **is this conclusion allowed to be wrong later?**

- If the release is a **view** — something that should un-conclude the moment a
  block appears — make its insert logical (next page). Truth maintenance handles
  the retraction and order stops mattering.
- If the release is a **durable record** — an audit row that must survive — then
  put the block rules and the release rule in genuinely separate **stages**:
  give the blocks higher salience, or run them in a separate pass and feed their
  output back in as input. Now "all blocks are decided before any release" is
  structural, not a coincidence of priority numbers.

The general lesson, worth carrying past this one case: a rule that concludes
something based on the *absence* or *total* of facts your own rules are still
producing is making a bet that they've finished. Separate the stages so the bet
is always safe.

Next: [truth maintenance](04-truth-maintenance.md), the "view" option in full.
