# Concepts

This is a guide to the ideas behind `seine-rs`, written for people who have
**never used a rule engine**. There is no assumed background. If you can read a
Python `if` statement, you can read this.

A rule engine is a different way to organize decision logic. Instead of writing
one long function that checks conditions in an order you choose, you write a set
of small independent rules and hand them to an engine that decides which ones
apply and when. That trade — giving up control of the order in exchange for
logic that stays legible as it grows — is the whole idea. These pages build it
up one piece at a time.

Read them in order:

1. **[Facts](01-facts.md)** — the data your rules reason about.
2. **[Rules](02-rules.md)** — a condition and a consequence, written separately
   from every other rule.
3. **[The agenda](03-agenda.md)** — how the engine decides what fires, and in
   what order, when several rules match at once.
4. **[Truth maintenance](04-truth-maintenance.md)** — how a conclusion can
   *un-conclude* itself when the thing that justified it goes away.
5. **[Time and events](05-time-and-events.md)** — facts that happen at a
   moment, the clock you drive yourself, and expiry.
6. **[Patterns in time](06-temporal-patterns.md)** — sequences and windowed
   counts: "B shortly after A," "three failures inside a minute."
7. **[Math outside the engine](07-math-outside-the-engine.md)** — why patterns
   never compute, and the dataframe-style derivation plane that does.

Each page starts from the `if`/`else` you'd write instead, shows what the rule
version buys you, and is honest about what it costs. The later pages assume
you've seen a dataframe before; nothing more.

## One thing to know up front

`seine-rs` is a **deliberately bounded engine**. It implements a well-defined
subset of a larger rule language and refuses, at compile time, to let you write
things outside that subset. This is a feature: every behavior it *does* accept
has been checked, byte for byte, against a reference implementation. When you
hit an edge of the subset you get a clear error explaining the boundary, not a
silent wrong answer. You'll see this posture throughout — the engine would
rather stop you than guess.

You do **not** need to know what that reference implementation is to use this
library, and these docs never assume you do.
