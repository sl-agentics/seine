# xfail — node-sharing window-claim classes (D-035)

These cases combine structurally shared beta prefixes (rules whose
pattern prefixes are node-shared in Drools) with mutation, salience
differences, or linking asymmetries among the sharers.

- fz_7_2081, fz_7_2859, fz_777_7592: sharing × mutation (delete/update
  interleaving the shared join's enumeration).
- fz_42_8472: sharing × salience/linking (static insert-only! — the
  preserved-vs-flipped sink is claimed by the first sharer whose agenda
  item evaluates the shared segment; an unlinked or lower-salience
  sharer never claims).

Drools evaluates a shared node ONCE, at the window of whichever rule's
agenda item reaches it first; its staged output then propagates to every
sink's segment. Seine keeps per-rule network copies, so each copy
evaluates at its own rule's window — under mutation the batch boundaries
(and therefore child-creation and requeue orders) can differ from the
shared single evaluation.

The D-033 sink-order flip handles the static equal-salience all-linked
part of sharing (pinned by pr_ne_s1..s11); the generator wall (D-035)
keeps ALL generated programs free of shared prefixes until shared
segments are modeled properly (one node instance, evaluated once at the
first-reaching item's window). These cases are the open class's
evidence; minimize with tools/minimize.py + SEINE_HANDLES=1 when
picking this up.
