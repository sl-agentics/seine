#!/usr/bin/env python3
"""model_check_notlead.py — D-334: the not-lead release re-propagation
(the cf325901x52 recheck, round 2).

The shape: `not B(...) P(...)` — the not is pattern 0, so it gates ONE
left tuple (the InitialFact) and the P-witness order on a release comes
from the DOWNSTREAM join's re-propagation. The D-334 recon (nl1-nl6)
found the clean surface engine==oracle throughout but falsified the
straight-line composition on nl6 ([2,4,3], BOTH engines). The missing
piece is D-031: an ALPHA-ONLY not UNLINKS its path while a blocker
exists (unlinkNotNodeOnRightInsert; relink when the right count
returns to 0), so staged effects ACCUMULATE ACROSS EPOCHS and process
in ONE relink evaluation.

This checker: the source-exact machine with THREE axes:

  unlink   on    D-031 unlink while blocked; staged effects accumulate
                 across fireAllRules boundaries (source)
           off   the path stays selectable; per-epoch evaluation
  updwalk  lifo  doUpdatesReorderRightMemory walks getUpdateFirst
                 head-first = newest-staged-first (source)
           fifo  oldest-first
  inswalk  lifo  doRightInserts walks getInsertFirst head-first =
                 newest-staged-first (source)
           fifo  oldest-first

2 x 2 x 2 = 8 machines over FIVE oracle timelines (all 3x-stable;
nl8's prediction was registered before the cell ran and hit exactly):
nl1_base, nl3_watched_move, nl4_external_move, nl6_multiepoch_move,
nl8_two_upds.

Fixed source mechanics: staged sets head-prepend with head-first walks
(one reversal per emission hop), TupleList memories tail-append with
removeAdd = move-to-tail, join arm order (rightDel, leftDel,
reorderRight, rightUpd, rightIns, leftIns — leftIns LAST), terminal
FIFO tupleList with update re-add for unqueued matches, selection =
salience then decl order, the lazy fire loop (D-333).

Expected survivor: (unlink=on, updwalk=lifo, inswalk=lifo) — nl6
discriminates unlink+inswalk, nl8 discriminates updwalk and the
reorder-timing (a per-epoch-arrival machine collapses onto fifo).
"""

import itertools
import sys


class Fact:
    def __init__(self, ftype, fields):
        self.ftype = ftype
        self.fields = dict(fields)

    def __repr__(self):
        return f"{self.ftype}({self.fields})"


class Machine:
    def __init__(self, cell, unlink, updwalk, inswalk):
        self.cell = cell
        self.unlink = unlink
        self.updwalk = updwalk
        self.inswalk = inswalk
        # R: not B(g==true) P($v) — the not-lead rule (salience 0, decl 0)
        self.p_ins = []      # staged P right-inserts, head = newest
        self.p_upd = []      # staged P right-updates, head = newest
        self.b_ins = []      # staged B admissions at the not right
        self.b_del = []
        self.rtm = []        # join P right memory (TupleList order)
        self.not_rtm = []    # not right memory (B facts)
        self.if_blocked = False
        self.if_staged = True     # the InitialFact's left-insert, pending
        self.if_in_join = False   # the IF-child is in the join's ltm
        self.linked = True
        self.queue = []      # R's terminal FIFO: (generation, pfact)
        self.fired_children = set()  # live terminal tuples (fired or queued)
        self.helper_fired = set()
        self.wm = []
        self.fired = []

    # ---- routing (external + RHS effects) --------------------------------

    def stage_p_ins(self, p):
        self.p_ins.insert(0, p)

    def stage_p_upd(self, p):
        if p in self.p_ins or p in self.p_upd:
            return
        self.p_upd.insert(0, p)

    def stage_b(self, b, admitted):
        if admitted:
            self.b_ins.insert(0, b)
        else:
            if b in self.b_ins:
                self.b_ins.remove(b)  # annihilation (TupleSets clash)
                return
            self.b_del.insert(0, b)
        # relink at arrival when the net right count returns to 0
        live = len(self.not_rtm) + len(self.b_ins) - len(self.b_del)
        if live == 0:
            self.linked = True

    def r_dirty(self):
        return bool(self.p_ins or self.p_upd or self.b_ins or self.b_del)

    # ---- R's evaluation (not-lead path) ----------------------------------

    def evaluate_r(self):
        term_del, term_upd, term_ins = [], [], []
        # NOT node: right inserts block the IF-left; right deletes release
        b_ins, self.b_ins = self.b_ins, []
        b_del, self.b_del = self.b_del, []
        if_join_ins = if_join_del = False
        for b in b_ins:
            self.not_rtm.append(b)
            if not self.if_blocked and not self.if_staged:
                self.if_blocked = True
                if_join_del = True  # block: the live IF-child dies
            elif self.if_staged:
                self.if_blocked = True  # will block at walk-in below
            if self.unlink:
                self.linked = False  # unlinkNotNodeOnRightInsert
        for b in b_del:
            if b in self.not_rtm:
                self.not_rtm.remove(b)
            if self.if_blocked and not self.not_rtm:
                self.if_blocked = False
                if_join_ins = True
        if self.if_staged:  # the not's leftIns arm: findLeftTupleBlocker
            self.if_staged = False
            if self.not_rtm:
                self.if_blocked = True  # blocked at walk-in, no join entry
                if_join_ins = False
            elif not self.if_blocked:
                if_join_ins = True

        # JOIN arms: rightDel(none), leftDel, reorderRight, rightUpd,
        # rightIns, leftIns
        if if_join_del:
            self.if_in_join = False
            for gen_p in list(self.queue):
                self.queue.remove(gen_p)  # cancel queued activations
            self.fired_children.clear()
        p_upd, self.p_upd = self.p_upd, []
        p_ins, self.p_ins = self.p_ins, []
        upd_walk = p_upd if self.updwalk == "lifo" else list(reversed(p_upd))
        for p in upd_walk:  # doUpdatesReorderRightMemory: removeAdd
            if p in self.rtm:
                self.rtm.remove(p)
                self.rtm.append(p)
        if self.if_in_join:  # rightUpd: child updates toward the terminal
            for p in upd_walk:
                if p in self.rtm:
                    term_upd.insert(0, p)
        ins_walk = p_ins if self.inswalk == "lifo" else list(reversed(p_ins))
        for p in ins_walk:  # doRightInserts
            self.rtm.append(p)
            if self.if_in_join:
                term_ins.insert(0, p)
        if if_join_ins:  # leftIns: the released IF-child walks the rtm
            self.if_in_join = True
            for p in self.rtm:
                term_ins.insert(0, p)

        # TERMINAL: dels, upds (re-add unqueued), ins (FIFO append)
        for p in term_upd:
            if p not in [q for q in self.queue]:
                self.queue.append(p)
        for p in term_ins:
            self.queue.append(p)

    # ---- helpers ---------------------------------------------------------

    def helper_matches(self, name):
        spec = self.cell["helpers"][name]
        out = []
        for f in self.wm:
            if f.ftype == spec["type"] and spec["alpha"](f.fields):
                if spec.get("needs_k") and not any(
                        g.ftype == "K" for g in self.wm):
                    continue
                if (name, id(f)) not in self.helper_fired:
                    out.append(f)
        return out

    def fire_helper(self, name, f):
        spec = self.cell["helpers"][name]
        self.helper_fired.add((name, id(f)))
        self.fired.append((name, "w"))
        act = spec["action"]
        if act[0] == "modify_b":
            was = f.fields["g"] is True
            f.fields["g"] = act[1]
            now = f.fields["g"] is True
            if not was and now:
                self.stage_b(f, True)
            elif was and not now:
                self.stage_b(f, False)
        elif act[0] == "delete_b":
            self.wm.remove(f)
            self.stage_b(f, False)
        elif act[0] == "modify_p":
            f.fields["v"] = act[1]
            self.stage_p_upd(f)

    # ---- the epoch driver + lazy fire loop -------------------------------

    def run(self):
        facts = [Fact(t, fl) for t, fl in self.cell["facts"]]
        epochs = [{"actions": [], "facts": facts}] + [
            {"actions": e.get("actions", []),
             "facts": [Fact(t, fl) for t, fl in e.get("facts", [])]}
            for e in self.cell["epochs"]]
        seq = []  # visible insertion sequence for update targets
        for ep in epochs:
            for op, *args in ep["actions"]:
                if op == "update":
                    tgt, fields = args
                    f = seq[tgt]
                    f.fields.update(fields)
                    if f.ftype == "P":
                        self.stage_p_upd(f)
                    elif f.ftype == "B":
                        pass  # not used by the cells
            for f in ep["facts"]:
                seq.append(f)
                self.wm.append(f)
                if f.ftype == "P":
                    self.stage_p_ins(f)
                elif f.ftype == "B" and f.fields["g"] is True:
                    self.stage_b(f, True)
            self.fire_all()
        return self.fired

    def fire_all(self):
        for _ in range(100):
            # candidates: (salience, decl, kind, payload)
            cands = []
            if self.queue or (self.r_dirty() and self.linked):
                cands.append((0, self.cell["r_decl"], "R", None))
            for name, spec in self.cell["helpers"].items():
                if self.helper_matches(name):
                    cands.append((spec["sal"], spec["decl"], name, None))
            if not cands:
                return
            cands.sort(key=lambda c: (-c[0], c[1]))
            fired = False
            for sal, decl, kind, _ in cands:
                if kind == "R":
                    if self.r_dirty() and self.linked:
                        self.evaluate_r()
                    if not self.queue:
                        continue  # evaluated empty: next candidate
                    p = self.queue.pop(0)
                    self.fired.append(("R", str(p.fields["v"])))
                    fired = True
                    break
                self.fire_helper(kind, self.helper_matches(kind)[0])
                fired = True
                break
            if not fired:
                return


# ---- cells (oracle timelines, 3x-stable; nl8 registered-then-hit) --------

CELLS = [
    {"name": "nl1_base", "r_decl": 0,
     "helpers": {
         "U": {"type": "B", "sal": -5, "decl": 1,
               "alpha": lambda f: f["g"] is False, "action": ("modify_b", True)},
         "D": {"type": "B", "sal": -10, "decl": 2,
               "alpha": lambda f: f["g"] is True, "action": ("delete_b",)},
     },
     "facts": [("P", {"v": 1}), ("P", {"v": 2}), ("B", {"g": False}),
               ("P", {"v": 3})],
     "epochs": [],
     "expected": [("R", "1"), ("R", "2"), ("R", "3"), ("U", "w"),
                  ("D", "w"), ("R", "1"), ("R", "2"), ("R", "3")]},
    {"name": "nl3_watched_move", "r_decl": 0,
     "helpers": {
         "M": {"type": "P", "sal": -3, "decl": 1,
               "alpha": lambda f: f["v"] == 2, "action": ("modify_p", 5)},
         "U": {"type": "B", "sal": -5, "decl": 2,
               "alpha": lambda f: f["g"] is False, "action": ("modify_b", True)},
         "D": {"type": "B", "sal": -10, "decl": 3,
               "alpha": lambda f: f["g"] is True, "action": ("delete_b",)},
     },
     "facts": [("P", {"v": 1}), ("P", {"v": 2}), ("B", {"g": False}),
               ("P", {"v": 3})],
     "epochs": [],
     "expected": [("R", "1"), ("R", "2"), ("R", "3"), ("M", "w"),
                  ("R", "5"), ("U", "w"), ("D", "w"), ("R", "5"),
                  ("R", "1"), ("R", "3")]},
    {"name": "nl4_external_move", "r_decl": 0,
     "helpers": {
         "D": {"type": "B", "sal": 0, "decl": 1, "needs_k": True,
               "alpha": lambda f: f["g"] is True, "action": ("delete_b",)},
     },
     "facts": [("P", {"v": 1}), ("P", {"v": 2}), ("B", {"g": True}),
               ("P", {"v": 3})],
     "epochs": [
         {"actions": [("update", 1, {"v": 5})]},
         {"facts": [("K", {"v": 0})]},
     ],
     "expected": [("D", "w"), ("R", "5"), ("R", "1"), ("R", "3")]},
    {"name": "nl6_multiepoch_move", "r_decl": 0,
     "helpers": {
         "D": {"type": "B", "sal": 0, "decl": 1, "needs_k": True,
               "alpha": lambda f: f["g"] is True, "action": ("delete_b",)},
     },
     "facts": [("P", {"v": 1}), ("B", {"g": True})],
     "epochs": [
         {"facts": [("P", {"v": 2})]},
         {"actions": [("update", 0, {"v": 3})]},
         {"facts": [("P", {"v": 4}), ("K", {"v": 0})]},
     ],
     "expected": [("D", "w"), ("R", "2"), ("R", "4"), ("R", "3")]},
    {"name": "nl8_two_upds", "r_decl": 0,
     "helpers": {
         "D": {"type": "B", "sal": 0, "decl": 1, "needs_k": True,
               "alpha": lambda f: f["g"] is True, "action": ("delete_b",)},
     },
     "facts": [("P", {"v": 1}), ("P", {"v": 2}), ("B", {"g": True})],
     "epochs": [
         {"actions": [("update", 0, {"v": 3})]},
         {"actions": [("update", 1, {"v": 5})]},
         {"facts": [("P", {"v": 4}), ("K", {"v": 0})]},
     ],
     "expected": [("D", "w"), ("R", "4"), ("R", "3"), ("R", "5")]},
]


def normalize_epochs(cell):
    out = []
    for e in cell["epochs"]:
        acts = [(a[0], a[1], a[2]) for a in e.get("actions", [])]
        out.append({"actions": acts, "facts": e.get("facts", [])})
    return out


def main():
    survivors = []
    for unlink, updwalk, inswalk in itertools.product(
            [True, False], ["lifo", "fifo"], ["lifo", "fifo"]):
        fails = []
        for cell in CELLS:
            c = dict(cell)
            c["epochs"] = normalize_epochs(cell)
            got = Machine(c, unlink, updwalk, inswalk).run()
            if got != cell["expected"]:
                fails.append(cell["name"])
        tag = (f"unlink={'on ' if unlink else 'off'} "
               f"upd={updwalk} ins={inswalk}")
        if fails:
            print(f"  {tag}  FAIL {len(fails)}/5: {', '.join(fails)}")
        else:
            print(f"  {tag}  SURVIVES 5/5")
            survivors.append((unlink, updwalk, inswalk))
    print()
    if survivors == [(True, "lifo", "lifo")]:
        print("UNIQUE SURVIVOR = the source machine "
              "(D-031 unlink + cross-epoch accumulation, LIFO staged walks)")
        return 0
    print(f"UNEXPECTED survivors: {survivors}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
