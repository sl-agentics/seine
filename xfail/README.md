# xfail — shared-prefix × mutation window timing (D-035)

Both cases combine structurally shared beta prefixes (two rules whose
pattern prefixes are node-shared in Drools) with RHS mutation (delete)
interleaving the shared join's enumeration.

Drools evaluates a shared node ONCE, at the window of whichever rule's
agenda item reaches it first; its staged output then propagates to every
sink's segment. Seine keeps per-rule network copies, so each copy
evaluates at its own rule's window — under mutation the batch boundaries
(and therefore child-creation and requeue orders) can differ from the
shared single evaluation.

The D-033 sink-order flip handles the INSERT-ONLY part of sharing
(pinned by pr_ne_s1..s11); the generator wall (D-035) keeps mutation
programs free of shared prefixes until shared segments are modeled
properly. These two cases are the open class's evidence; minimize with
tools/minimize.py + SEINE_HANDLES=1 when picking this up.
