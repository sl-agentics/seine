# Beyond-Drools: a fully coherent Allen interval algebra (roadmap note)

**Status:** ROADMAP / vision — **not current work.** Recorded per Bryan's ruling
(2026-07-08, DECISIONS §D-119). Do **not** start this before Seine is
*certifiably faithful to Drools* on the whole CEP surface (the current
prime-directive goal). This note exists so the idea is not lost.

## The idea

Once Seine faithfully reproduces Drools 9.44's `@duration` interval-event
behavior (item E slab 1: the Allen predicates + `endTS = ts+dur`, with Allen-op
`@expires` **inference fenced**), a follow-on would **fully implement the Allen
interval algebra** — including the places where **Drools itself is incomplete or
incoherent**, providing clean, complete interval semantics as a *superset* of
Drools.

## Why this is a deliberate doctrine break

Seine's prime directive is PROBE-FIRST: the Drools oracle settles every
semantic. **This enhancement has no oracle** — by definition it covers behavior
Drools does *not* have (or has inconsistently). So it must be **spec-driven**,
with **Allen's interval algebra** (Allen 1983, the 13 base relations and their
composition table) as the authoritative spec instead of Drools. That is the
same shape as the existing axis-2/axis-3 rulings (D-093/D-095): where the
faithful oracle is wrong or silent, adopt the *correct* external spec and
document the intentional divergence with witnesses — except here the "external
spec" is a mathematical algebra, and the divergences are *additions*, not
corrections.

Because it is off-oracle, it must be **strictly opt-in and quarantined** from
the faithful core: gated so the default (Drools-faithful) paths stay
byte-identical, its divergences filed as expected (opposite-polarity witnesses),
and never allowed to regress the certified corpus.

## Concrete Drools gaps this would close (found during D-118/D-119 recon)

- **The `@expires` inference LEAK.** Drools' inferred-expiry reach through Allen
  ops is op/position/param-specific and leaks to NEVER for whole classes
  (`during` both positions; `meets`/`finishes` anchor side; etc. — full
  never/finite table in D-119). A coherent implementation would derive a
  *principled* retention bound from each relation's endpoint constraints (the
  STP closure) rather than inheriting Drools' `lo>0`-style leaks — so
  un-annotated interval events used under any Allen op get a sound, finite
  expiry.
- **Operator/param completeness.** Ensure every one of the 13 relations (+ their
  parameterized tolerance/distance forms) composes uniformly with windows,
  accumulate, not/exists, and entry points — no silent op×feature gaps.
- **Coherence of the composition table.** Support Allen-relation *composition*
  reasoning (if `A during B` and `B before C` then `A before C`, …) where it
  buys query power, rather than treating each constraint in isolation.

## What "done" looks like (someday)

A mode/flag under which Seine offers the complete, internally-consistent Allen
algebra over `@duration` intervals, spec-tested against the algebra's own laws
(reflexivity of `coincides`, converse pairs, the composition table), with the
Drools-faithful behavior still the default and still certified. Until Seine is
faithful first, this stays a note.

**See also:** DECISIONS §D-118 (the `endTS=ts+dur` model), §D-119 (Allen
predicates/params + the inference never/finite classification), §D-109 (the STP
inference machinery this generalizes), `docs/drools-inferred-expiry-never.md`.
