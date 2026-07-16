# Rules

A **rule** is a condition paired with a consequence: *when* some facts look a
certain way, *then* do something. Here's the whole of one:

```python
from seine_rs import fact, Rule, run

@fact
class Person:
    name: str
    age: int
    citizen: bool

@fact
class CanVote:
    name: str

r = Rule("can-vote")
p = r.when(Person, Person.age >= 18, Person.citizen == True)
r.then_insert(CanVote, name=p.name)
```

Read it aloud: *"when there is a Person aged 18 or over who is a citizen, then
insert a CanVote for them."* The `when` is the condition; the `then_insert` is
the consequence — it creates a new fact.

Run it over some people:

```python
people = [
    {"name": "Alice", "age": 34, "citizen": True},
    {"name": "Bob",   "age": 16, "citizen": True},
    {"name": "Carol", "age": 71, "citizen": False},
    {"name": "Dave",  "age": 18, "citizen": True},
]
res = run([r], {Person: people, CanVote: []})

for row in res.derived["CanVote"].to_pylist():
    print(row["name"])
```

```
Alice
Dave
```

Bob fails on age, Carol on citizenship, Dave lands exactly on the boundary. The
two constraints inside `when` are **and**-ed together; `p.name` pulls the matched
person's field into the consequence.

(Results are tables, and the loop is just one way to read one. If you'd rather
have a dataframe, `res.derived["CanVote"].to_pandas()` hands you one with
Arrow-backed dtypes; `.to_arrow()` and `.to_polars()` exist too. `to_pylist()`
is simply the only reader that needs no extra install.)

## Why not just write the `if`?

You could. For one condition, an `if` is plainly simpler:

```python
can_vote = []
for person in people:                      # the same list run() just saw
    if person["age"] >= 18 and person["citizen"]:
        can_vote.append(person["name"])
```

The rule earns its keep when there are *many* conditions that interact. Imagine
voting eligibility with a dozen clauses: age, citizenship, registration
deadlines, felony status by state, overseas exceptions. In the `if` version those
pile into one branching function where every new case risks the ordering of the
old ones. In the rule version each clause is a separate, named `Rule` you can
read, test, and change on its own. The engine — not you — works out which ones
apply to each fact.

That's the trade: you give up writing the control flow, and in return each piece
of logic stays small and independent no matter how many you add.

## The two halves

**The `when` side** is one or more patterns. Each pattern names a fact type and
zero or more constraints on its fields:

```python
r.when(Person, Person.age >= 18, Person.citizen == True)
```

When a rule has several patterns, they must **all** match — and they can be tied
together by a shared value. This rule only fires for an order whose customer is
flagged as fraudulent:

```python
@fact
class Order:
    id: int
    customer_id: str

@fact
class Customer:
    id: str
    fraud: bool

r2 = Rule("fraud-order")
o = r2.when(Order)
r2.when(Customer, Customer.id == o.customer_id, Customer.fraud == True)
```

The `o.customer_id` on the second pattern is a **join**: it says the Customer's
`id` must equal *this* Order's `customer_id`. Joins are how facts relate to each
other.

You can also require that a matching fact **not** exist:

```python
@fact
class Account:
    acct_id: str
    status: str

@fact
class Hold:
    acct_id: str

r3 = Rule("release-title")
a = r3.when(Account, Account.status == "PAIDOFF")
r3.when_not(Hold, Hold.acct_id == a.acct_id)   # ...and no hold on it
```

`when_not` is powerful and has one sharp edge, covered on the
[agenda](03-agenda.md) page — when it negates a fact your *own* rules produce,
order starts to matter. The engine has a compile-time check that catches the
dangerous version of this for you.

**The `then` side** creates, changes, or removes facts:

| Action | Meaning |
| --- | --- |
| `then_insert(T, ...)` | add a new fact of type `T` |
| `then_insert_logical(T, ...)` | add a fact that **auto-retracts** — see [truth maintenance](04-truth-maintenance.md) |
| `then_delete(pattern)` | remove a matched fact |

The difference between `then_insert` and `then_insert_logical` is one of the most
useful ideas in the library, and it has its own page.

## Rules don't call each other

This is the mental shift. A rule never invokes another rule. It only reads facts
and writes facts. If rule B should run "after" rule A, you don't wire them
together — you have A produce a fact that B's `when` looks for. Rules coordinate
entirely through working memory, never by calling.

Which raises the obvious question: if nothing controls the order, and several
rules match at once, *what fires first?* That's the [agenda](03-agenda.md).
