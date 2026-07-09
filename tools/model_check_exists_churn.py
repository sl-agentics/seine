#!/usr/bin/env python3
"""CEP E2 item-C CLASS 3 — model_check for the exists delete+reinsert churn.

Doctrine (MODEL-CHECK IN PYTHON before writing Rust): validate a full ordering
model of the existential node's right-tuple churn against ALL the class-3 probe
outputs; wrong sub-models die here in seconds and the surviving model doubles as
the port spec. Ground truth = the oracle firings (probes_pending/cep/e_* +
scenarios/xfail/xf_cep_c_del_churn_exists*) + the ExistsDump graft
(oracle/.../ExistsDump.java): an EVENT right churn re-fires via a fresh
child+CREATE; a PLAIN right coalesces.

ABSTRACT MACHINE — one exists node, its right memory (witnesses, in order), each
right's BLOCKED-left set, the UNBLOCKED lefts, and per-left blocker. A left is
"satisfied" (propagates a child ⇒ the rule can fire) iff it is BLOCKED by some
right. We count CHILD-INSERTs emitted during the epoch = the number of exists
RE-FIRES (a child-DELETE is a retract, not a fire).

  INSERT r: append r; every UNBLOCKED left that matches gets blocked by r and
            emits child_ins (exists becomes satisfied ⇒ fire).
  DELETE r: remove r; every left blocked by r unblocks and RE-SEARCHES the
            current right memory for a new blocker — found ⇒ silently re-block
            (no child event); none ⇒ move to unblocked + child_del (retract).

The ONLY degree of freedom is the ORDER the staged ops hit the node:
  BATCHED  — all INSERTs before all DELETEs (Drools PhreakExistsNode phase order,
             hand-traced): the delete's re-search sees the same-batch insert ⇒
             it re-blocks ⇒ COALESCE.
  ARRIVAL  — ops in arrival order: a delete-first churn empties the memory before
             the insert, so the delete retracts and the insert re-fires.

Candidate models pick the order; the surviving one is the spec for the port.
No engine calls — a pure replica. Run: python3 tools/model_check_exists_churn.py
"""


class ExistsNode:
    """Right memory + per-right blocked lefts + unblocked lefts. `match` is
    trivially true here (bare exists / no beta constraint — every witness
    blocks every left), which is all the class-3 shapes need."""

    def __init__(self, rights, lefts):
        # rights: list of witness ids (order = right-memory order)
        # lefts:  list of left-tuple ids present at the node
        self.rights = list(rights)
        self.blocked = {}          # right -> [lefts]
        self.blocker_of = {}       # left  -> right
        self.unblocked = []        # lefts with no blocker (exists NOT satisfied)
        self.child_ins = 0         # fires emitted
        self.child_del = 0         # retracts emitted
        # initial blocking: each left blocked by the FIRST right (arrival),
        # else unblocked. (post-initial-fire steady state)
        for l in lefts:
            if self.rights:
                b = self.rights[0]
                self.blocker_of[l] = b
                self.blocked.setdefault(b, []).append(l)
            else:
                self.unblocked.append(l)

    def insert(self, r):
        self.rights.append(r)
        for l in list(self.unblocked):
            # every witness matches (no constraint) → block the unblocked left
            self.blocker_of[l] = r
            self.blocked.setdefault(r, []).append(l)
            self.unblocked.remove(l)
            self.child_ins += 1        # exists satisfied → FIRE

    def delete(self, r):
        if r in self.rights:
            self.rights.remove(r)
        for l in list(self.blocked.pop(r, [])):
            del self.blocker_of[l]
            # re-search the CURRENT right memory for a replacement blocker
            newb = self.rights[0] if self.rights else None
            if newb is not None:
                self.blocker_of[l] = newb
                self.blocked.setdefault(newb, []).append(l)
                # silent re-block: exists stays satisfied → NO child event
            else:
                self.unblocked.append(l)
                self.child_del += 1    # exists unsatisfied → retract


def run(rights, lefts, ops, order):
    """ops = ordered [('ins'|'del', witness)]. order ∈ {'batched','arrival'}.
    Returns the number of child-inserts (re-fires) emitted."""
    node = ExistsNode(rights, lefts)
    if order == "batched":
        seq = [o for o in ops if o[0] == "ins"] + [o for o in ops if o[0] == "del"]
    else:  # arrival
        seq = ops
    for kind, w in seq:
        (node.insert if kind == "ins" else node.delete)(w)
    return node.child_ins


# --- the class-3 probe table: ground truth = oracle firing counts ---------
# rights   = witnesses live after the INITIAL fire
# lefts    = ['L'] (the single exists left: InitialFact, or the [P]-prefix)
# is_event = the witness type is an @event
# explicit = the delete is an explicit session.delete / rule-RHS delete
#            (False ⇒ expiration, which defers/coalesces — D-102)
# ops      = ARRIVAL-ordered staged right ops for the epoch
# refires  = NEW exists fires during the epoch, from the oracle
PROBES = [
    # name                         rights      is_event explicit ops(arrival)                    refires
    ("xf_cep_c_del_churn_exists", ["W"],       True,    True,   [("del","W"),("ins","W2")],       1),
    ("e_bare_churn",              ["W"],       True,    True,   [("del","W"),("ins","W2")],       1),
    ("e_rule_churn",              ["W"],       True,    True,   [("del","W"),("ins","W2")],       1),
    ("e_plain_churn",             ["W"],       False,   True,   [("del","W"),("ins","W2")],       0),
    ("e_evt_insfirst",            ["W"],       True,    True,   [("ins","W2"),("del","W")],       0),
    ("e_evt_2wit",                ["W","W0"],  True,    True,   [("del","W"),("ins","W2")],       0),
    ("e_evt_delonly",             ["W"],       True,    True,   [("del","W")],                    0),
    ("e_exp_churn",               ["W"],       True,    False,  [("del","W"),("ins","W2")],       0),
]

# --- candidate models: each maps a probe to the processing order ----------
MODELS = {
    "always_batched":       lambda p: "batched",
    "always_arrival":       lambda p: "arrival",
    # THE HYPOTHESIS: an EVENT right with an EXPLICIT delete is arrival-ordered;
    # everything else (plain fact, or expiration) batches.
    "event_explicit_arrival": lambda p: "arrival" if (p["is_event"] and p["explicit"]) else "batched",
}


def main():
    results = {m: [] for m in MODELS}
    for (name, rights, is_event, explicit, ops, refires) in PROBES:
        p = {"is_event": is_event, "explicit": explicit}
        for m, pick in MODELS.items():
            got = run(rights, ["L"], ops, pick(p))
            results[m].append((name, got, refires, got == refires))
    print(f"{'probe':<28} exp " + " ".join(f"{m:>22}" for m in MODELS))
    for i, (name, *_ ) in enumerate(PROBES):
        exp = PROBES[i][5]
        cells = []
        for m in MODELS:
            _, got, _, ok = results[m][i]
            cells.append(f"{got}{'ok ' if ok else 'XX '}")
        print(f"{name:<28} {exp:>3}  " + " ".join(f"{c:>22}" for c in cells))
    print()
    for m in MODELS:
        errs = sum(0 if ok else 1 for *_, ok in results[m])
        verdict = "SURVIVES (0-div)" if errs == 0 else f"DIES ({errs} mispredictions)"
        print(f"  model {m:<26} {verdict}")
    survivors = [m for m in MODELS if all(ok for *_, ok in results[m])]
    print()
    if survivors == ["event_explicit_arrival"]:
        print("✓ UNIQUE SURVIVOR = event_explicit_arrival — the port spec: an exists/not")
        print("  node over an EVENT type processes an EXPLICIT (non-expiration) right")
        print("  DELETE in ARRIVAL order (unblock+retract before a same-batch insert")
        print("  re-blocks); plain facts and expiration keep the batched ins-before-del")
        print("  coalesce. Validated 0-div vs the class-3 probe/graft ground truth.")
        return 0
    print(f"✗ survivors = {survivors} — model not yet unique; refine before porting.")
    return 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
