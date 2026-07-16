# Facts

A **fact** is one piece of data the engine reasons about. If you're picturing a
row in a table, or one dict in a list of dicts, you've got it exactly.

You describe the *shape* of a fact with a class:

```python
from seine_rs import fact

@fact
class Person:
    name: str
    age: int
    citizen: bool
```

That's not an object you instantiate. It's a **schema** — a declaration that
"a `Person` fact has a string `name`, an integer `age`, and a boolean
`citizen`." The actual facts are plain data you hand in later:

```python
people = [
    {"name": "Alice", "age": 34, "citizen": True},
    {"name": "Bob",   "age": 16, "citizen": True},
]
```

## Why a schema at all?

Because the engine checks it, strictly, and tells you when the data doesn't
match. Declare `age: int` and pass a string, and you get an error at load time
naming the field — not a wrong answer three rules later. The type you write is
a promise the engine holds you to.

This strictness is the same posture the whole library takes: it would rather
stop and tell you something is off than quietly do the wrong thing.

## Facts come in, facts come out

Here's the part that's different from a table. Your rules don't just *read*
facts — they *create* them. When a rule reaches a conclusion, that conclusion is
itself a new fact, and it goes back into the same pool where other rules can see
it.

So you'll declare two kinds of fact class: the ones you put **in** (the data you
start with) and the ones the rules produce as **output**:

```python
@fact
class Person:          # input: what you know
    name: str
    age: int
    citizen: bool

@fact
class CanVote:         # output: what the rules conclude
    name: str
```

Nothing about `CanVote` marks it as "output" — that distinction is just how you
use it. Any fact a rule creates can be read by another rule, which is what lets
you build a conclusion in steps: rule A produces a fact, rule B reads it and
produces another. The whole pool of facts, input and derived together, is called
**working memory**.

That's all a fact is: typed data, some given, some derived, all living in the
same pool. Next: the [rules](02-rules.md) that read and write it.
