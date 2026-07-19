#!/usr/bin/env python3
"""model_check_notrel.py — D-333 (part 2 of D-331): the not-release order
is Phreak LAZINESS, not a list direction.

D-331 pinned the consumption law (r1/r2 predictions exact) but the naive
LIFO flip broke 11 certified cells, and the instrumented decode pointed
upstream. The source read (drools-core 9.44.0.Final: PhreakNotNode,
PhreakJoinNode, PhreakRuleTerminalNode, TupleSetsImpl, TupleList,
RightTupleImpl, RuleExecutor, RuleAgendaConflictResolver) fixed every
list direction and exposed the actual mechanism:

  * RuleExecutor.fire: after a firing, haltRuleFiring peeks; on
    preemption it BREAKS WITHOUT evaluateNetworkIfDirty — the preempted
    rule's beta network stays dirty until that rule is next SELECTED.
  * TupleSetsImpl.addDelete on a staged-INSERT tuple ANNIHILATES
    ("case Tuple.INSERT: removeInsert(tuple); return").

In the modify->delete relays (R0 lower load order preempts R1 at equal
salience) the staged not-right INSERT dies before R1's network ever
evaluates: NO block, NO cancel, NO release. The oracle's "release
order" is R1's untouched round-0 FIFO queue — reverse-insertion for
join-fed shapes (3 staging hops, odd reversals), forward for LIA-direct
(2 hops). nb1 keeps a REAL block+release because R (salience 0) is
selected between U's modify (-5) and D's delete (-10).

This checker: one source-exact machine core with FOUR free axes:

  evalmode  lazy    evaluate a rule's network only at its selection /
                    between its own consecutive firings (source)
            eager   evaluate every dirty network after every firing
                    (the engine's global eagerness)
  clash     annihilate  addDelete kills a staged INSERT (source)
            keep        the delete stages alongside; block+release
                        both happen at the next evaluation
  relwalk   head    doRightDeletes walks blocked head-first =
                    last-blocked-first (source)
            tail    oldest-blocked-first
  blkbuild  prepend addBlocked head-prepend (source)
            append  tail-append

2 x 2 x 2 x 2 = 16 machines over NINE oracle timelines (3x-stable,
predictions registered first, 8/8 hit): nb1, r1_four_candidates,
r2_initial_block, r3_fresh_release, r4_sametype_release,
fz_min_7_2364, nb3, fz_7_2364, r5_partial_block (the clash-axis
discriminator).

Fixed source mechanics (not axes): staged sets head-prepend with
head-first walks (one reversal per node-emission hop; addAll
order-preserving), TupleList memories tail-append/head-first,
existential reorder moves updated unblocked lefts to the memory tail,
doRightInserts skips staged-UPDATE lefts (re-blocked later in
doLeftUpdates = blocked-list head), released lefts re-enter memory at
the tail, terminal walks head-first into a per-rule FIFO tupleList
(static salience: removeFirst), rule selection = salience then LOWEST
load order (RuleAgendaConflictResolver).
"""

import itertools
import sys


class Staged:
    """TupleSetsImpl: head-prepend lists, head-first walks, clash rules."""

    def __init__(self, clash="annihilate"):
        self.ins = []
        self.upd = []
        self.dele = []
        self.clash = clash

    def add_ins(self, t):
        if t in self.upd:
            return  # addInsert on staged UPDATE: no-op
        if t not in self.ins:
            self.ins.insert(0, t)

    def add_upd(self, t):
        if t in self.ins or t in self.upd or t in self.dele:
            return  # addUpdate when staged != NONE: no-op
        self.upd.insert(0, t)

    def add_del(self, t):
        if t in self.ins:
            if self.clash == "annihilate":
                self.ins.remove(t)
                return  # case Tuple.INSERT: removeInsert; return
            # keep: the insert stays staged; block+release both happen
        if t in self.upd:
            self.upd.remove(t)
        if t not in self.dele:
            self.dele.insert(0, t)

    def take_all(self):
        out = (self.ins, self.upd, self.dele)
        self.ins, self.upd, self.dele = [], [], []
        return out

    def is_empty(self):
        return not (self.ins or self.upd or self.dele)


class Fact:
    _next = [0]

    def __init__(self, ftype, fields):
        self.ftype = ftype
        self.fields = dict(fields)
        self.fid = Fact._next[0]
        Fact._next[0] += 1

    def __repr__(self):
        return f"{self.ftype}#{self.fid}"


class Child:
    """A join child / not left tuple; also the terminal match identity.
    New entity per (re-)creation: a release makes a NEW activation."""

    def __init__(self, lf, rf):
        self.lf = lf
        self.rf = rf

    def __repr__(self):
        return f"<{self.lf},{self.rf}>"


class NotRule:
    def __init__(self, spec, clash):
        self.spec = spec
        self.lia = Staged(clash)          # left tuples = lfact
        self.join_right = Staged(clash)   # join right facts (None join: unused)
        self.jltm = []                    # join left memory (facts)
        self.jrtm = []                    # join right memory (facts)
        self.children = {}                # (lf,rf) -> Child (live join children)
        self.not_right = Staged(clash)    # right facts at the not
        self.nltm = []                    # not left memory (Child)
        self.nrtm = []                    # not right memory (facts)
        self.blocked = {}                 # rfact -> [Child] (list head = index 0)
        self.blocker = {}                 # Child -> rfact
        self.term_child = {}              # Child -> match live at terminal
        self.queue = []                   # FIFO tupleList of Child
        self.dirty = True

    def is_dirty(self):
        return not (self.lia.is_empty() and self.join_right.is_empty()
                    and self.not_right.is_empty())


class SimpleRule:
    def __init__(self, spec, clash):
        self.spec = spec
        self.lia = Staged(clash)  # facts
        self.admitted = set()
        self.queue = []           # FIFO of facts
        self.dirty = True

    def is_dirty(self):
        return not self.lia.is_empty()


class Machine:
    def __init__(self, cell, evalmode, clash, relwalk, blkbuild):
        self.evalmode = evalmode
        self.clash = clash
        self.relwalk = relwalk
        self.blkbuild = blkbuild
        self.cell = cell
        self.rules = []
        for spec in cell["rules"]:
            r = (NotRule if spec["kind"] == "not" else SimpleRule)(spec, clash)
            self.rules.append(r)
        self.wm = []
        self.fired = []

    # ---- alpha routing (flushPropagations) -------------------------------

    def route_insert(self, f):
        self.wm.append(f)
        for r in self.rules:
            s = r.spec
            if s["kind"] == "simple":
                if f.ftype == s["type"] and s["alpha"](f.fields):
                    r.lia.add_ins(f)
            else:
                lt, lalpha = s["left"]
                if f.ftype == lt and lalpha(f.fields):
                    r.lia.add_ins(f)
                if s.get("join"):
                    jt, jalpha = s["join"]
                    if f.ftype == jt and jalpha(f.fields):
                        r.join_right.add_ins(f)
                nt, nalpha, _ = s["notp"]
                if f.ftype == nt and nalpha(f.fields):
                    r.not_right.add_ins(f)

    def route_delete(self, f):
        if f in self.wm:
            self.wm.remove(f)
        for r in self.rules:
            s = r.spec
            if s["kind"] == "simple":
                if f.ftype == s["type"] and (f in r.lia.ins or f in r.lia.upd
                                             or f in r.admitted):
                    r.lia.add_del(f)
                    r.admitted.discard(f)
            else:
                lt, _ = s["left"]
                if f.ftype == lt and (f in r.lia.ins or f in r.lia.upd
                                      or f in r.jltm):
                    r.lia.add_del(f)
                if s.get("join"):
                    jt, _ = s["join"]
                    if f.ftype == jt and (f in r.join_right.ins or f in r.jrtm):
                        r.join_right.add_del(f)
                nt, _, _ = s["notp"]
                if f.ftype == nt and (f in r.not_right.ins
                                      or f in r.not_right.upd or f in r.nrtm):
                    r.not_right.add_del(f)

    def route_modify(self, f, old_fields):
        for r in self.rules:
            s = r.spec
            positions = []
            if s["kind"] == "simple":
                positions.append((s["type"], s["alpha"], r.lia, "simple", r))
            else:
                lt, lalpha = s["left"]
                positions.append((lt, lalpha, r.lia, "left", r))
                if s.get("join"):
                    jt, jalpha = s["join"]
                    positions.append((jt, jalpha, r.join_right, "jright", r))
                nt, nalpha, _ = s["notp"]
                positions.append((nt, nalpha, r.not_right, "nright", r))
            for ftype, alpha, staged, _kind, rr in positions:
                if f.ftype != ftype:
                    continue
                before = alpha(old_fields)
                after = alpha(f.fields)
                if before and after:
                    staged.add_upd(f)
                elif not before and after:
                    staged.add_ins(f)
                elif before and not after:
                    staged.add_del(f)
                    if _kind == "simple":
                        rr.admitted.discard(f)

    # ---- evaluation ------------------------------------------------------

    def evaluate(self, r):
        if isinstance(r, SimpleRule):
            ins, upd, dele = r.lia.take_all()
            for f in dele:  # terminal deletes cancel queued matches
                if f in r.queue:
                    r.queue.remove(f)
            for f in upd:   # re-add if not queued (doLeftTupleUpdate)
                if f not in r.queue:
                    r.queue.append(f)
            for f in ins:
                r.admitted.add(f)
                r.queue.append(f)
            return

        s = r.spec
        beta = s["notp"][2]
        lins, lupd, ldel = r.lia.take_all()
        if s.get("join"):
            # ---- join level (arms: rDel, lDel, reorder, rUpd, lUpd, rIns, lIns)
            jins, jupd, jdel = r.join_right.take_all()
            trg = Staged(self.clash)  # children staged toward the not
            for f in jdel:
                if f in r.jrtm:
                    r.jrtm.remove(f)
                for (lf, rf) in [k for k in r.children if k[1] is f]:
                    trg.add_del(r.children.pop((lf, rf)))
            for f in ldel:
                if f in r.jltm:
                    r.jltm.remove(f)
                for (lf, rf) in [k for k in r.children if k[0] is f]:
                    trg.add_del(r.children.pop((lf, rf)))
            # doUpdatesReorderLeftMemory: staged-upd lefts move to the tail
            for f in lupd:
                if f in r.jltm:
                    r.jltm.remove(f)
            for f in lupd:
                r.jltm.append(f)
            for f in lupd:  # child updates toward the not (cross joins: kept)
                for (lf, rf) in [k for k in r.children if k[0] is f]:
                    trg.add_upd(r.children[(lf, rf)])
            for f in jins:  # right inserts: walk ltm, skip staged-upd lefts
                r.jrtm.append(f)
                for lf in list(r.jltm):
                    if lf in lupd:
                        continue
                    c = Child(lf, f)
                    r.children[(lf, f)] = c
                    trg.add_ins(c)
            for f in lins:  # left inserts: append memory, walk rtm
                r.jltm.append(f)
                for rf in list(r.jrtm):
                    c = Child(f, rf)
                    r.children[(f, rf)] = c
                    trg.add_ins(c)
            cins, cupd, cdel = trg.ins, trg.upd, trg.dele
        else:
            # LIA-direct: the segment staged feeds the not's arms with NO
            # intermediate emission hop (children materialize at walk-in)
            cdel = [r.children.pop((f, None)) for f in ldel
                    if (f, None) in r.children]
            cupd = [r.children[(f, None)] for f in lupd
                    if (f, None) in r.children]
            cins = []
            for f in lins:
                c = Child(f, None)
                r.children[(f, None)] = c
                cins.append(c)

        # ---- not level ---------------------------------------------------
        nins, nupd, ndel = r.not_right.take_all()
        term = Staged(self.clash)  # toward the terminal
        for c in cdel:  # doLeftDeletes (before right arms)
            rf = r.blocker.pop(c, None)
            if rf is not None:
                r.blocked[rf].remove(c)
            else:
                if c in r.nltm:
                    r.nltm.remove(c)
                if c in r.term_child:
                    term.add_del(r.term_child.pop(c))
        # doUpdatesExistentialReorderLeftMemory
        for c in cupd:
            if c in r.nltm:
                r.nltm.remove(c)
        for c in cupd:
            rf = r.blocker.get(c)
            if rf is None:
                r.nltm.append(c)
            elif rf in ndel or rf in nupd:
                r.blocked[rf].remove(c)
                del r.blocker[c]  # forced fresh re-match in doLeftUpdates
        def block(c, rf):
            r.blocker[c] = rf
            lst = r.blocked.setdefault(rf, [])
            if self.blkbuild == "prepend":
                lst.insert(0, c)
            else:
                lst.append(c)
        for rf in nins:  # doRightInserts: walk not-ltm head-first, skip upd
            r.nrtm.append(rf)
            for c in list(r.nltm):
                if c in cupd:
                    continue
                if beta(c.lf.fields, rf.fields):
                    block(c, rf)
                    r.nltm.remove(c)
                    if c in r.term_child:
                        term.add_del(r.term_child.pop(c))
        for rf in ndel:  # doRightDeletes: release walk per relwalk axis
            if rf in r.nrtm:
                r.nrtm.remove(rf)
            walk = r.blocked.pop(rf, [])
            if self.relwalk == "tail":
                walk = list(reversed(walk))
            for c in walk:
                del r.blocker[c]
                for rf2 in r.nrtm:  # re-block scan, memory order
                    if rf2 not in ndel and beta(c.lf.fields, rf2.fields):
                        block(c, rf2)
                        break
                if c not in r.blocker:  # released
                    r.nltm.append(c)
                    term.add_ins(c)
        for c in cupd:  # doLeftUpdates (after the right arms)
            rf = r.blocker.get(c)
            if rf is not None and rf in r.nrtm and beta(c.lf.fields, rf.fields):
                continue  # still blocked by same blocker
            if rf is not None:
                r.blocked[rf].remove(c)
                del r.blocker[c]
            newb = None
            for rf2 in r.nrtm:
                if beta(c.lf.fields, rf2.fields):
                    newb = rf2
                    break
            if newb is not None:
                if c in r.nltm:
                    r.nltm.remove(c)
                block(c, newb)
                if c in r.term_child:
                    term.add_del(r.term_child.pop(c))
            else:
                if c not in r.nltm:
                    r.nltm.append(c)
                if c in r.term_child:
                    term.add_upd(r.term_child[c])
                else:
                    term.add_ins(c)
        for c in cins:  # doLeftInserts: findLeftTupleBlocker
            newb = None
            for rf2 in r.nrtm:
                if beta(c.lf.fields, rf2.fields):
                    newb = rf2
                    break
            if newb is not None:
                block(c, newb)
            else:
                r.nltm.append(c)
                term.add_ins(c)

        # ---- terminal (doNode: deletes, updates, inserts; FIFO tupleList)
        for m in term.dele:
            if m in r.queue:
                r.queue.remove(m)
        for m in term.upd:
            if m not in r.queue:
                r.queue.append(m)
        for m in term.ins:
            r.term_child[m] = m
            r.queue.append(m)

    # ---- agenda ----------------------------------------------------------

    def items(self):
        return [r for r in self.rules if r.queue or r.is_dirty()]

    def pick(self, cands):
        # RuleAgendaConflictResolver: salience desc, then load order asc
        return max(cands, key=lambda r: (r.spec["sal"], -r.spec["load"]))

    def fire_one(self, r):
        m = r.queue.pop(0)
        f = m if isinstance(r, SimpleRule) else m.lf
        self.fired.append((r.spec["name"], f))
        act = r.spec.get("action")
        if act:
            if act[0] == "delete":
                self.route_delete(f)
            else:
                old = dict(f.fields)
                f.fields.update(act[1])
                self.route_modify(f, old)
        if self.evalmode == "eager":
            for rr in self.rules:
                if rr.is_dirty():
                    self.evaluate(rr)

    def run(self, limit=200):
        for ftype, fields in self.cell["facts"]:
            self.route_insert(Fact(ftype, fields))
        fires = 0
        while fires < limit:
            cands = self.items()
            if not cands:
                break
            r = self.pick(cands)
            if r.is_dirty():
                self.evaluate(r)
            if not r.queue:
                continue
            while r.queue and fires < limit:
                self.fire_one(r)
                fires += 1
                cands = self.items()
                if cands:
                    top = self.pick(cands)
                    if top is not r and (
                            (top.spec["sal"], -top.spec["load"]) >
                            (r.spec["sal"], -r.spec["load"])):
                        break  # preemption: NO evaluateNetworkIfDirty
                if self.evalmode == "lazy" and r.is_dirty():
                    self.evaluate(r)  # between own consecutive firings
        return [(n, f.fields.get("id", f.fields.get("f0", "?")))
                for n, f in self.fired]


# ---- cells (oracle timelines measured 3x-stable, 2026-07-18) -------------

TRUE_ = lambda f: True

def relay_rules(l_alpha=TRUE_, r0_alpha=None, beta=None):
    r0_alpha = r0_alpha or (lambda f: f["f2"] is True)
    beta = beta or (lambda lf, rf: True)
    return [
        {"name": "R0", "load": 0, "sal": 0, "kind": "simple", "type": "T0",
         "alpha": r0_alpha, "action": ("delete",)},
        {"name": "R1", "load": 1, "sal": 0, "kind": "not",
         "left": ("T0", l_alpha), "join": ("T1", TRUE_),
         "notp": ("T0", lambda f: f["f2"] is True, beta),
         "action": ("modify", {"f2": True})},
    ]

CELLS = [
    {"name": "nb1",
     "rules": [
         {"name": "R", "load": 0, "sal": 0, "kind": "not",
          "left": ("L", TRUE_), "join": None,
          "notp": ("B", lambda f: f["g"] is True, lambda lf, rf: True),
          "action": None},
         {"name": "U", "load": 1, "sal": -5, "kind": "simple", "type": "B",
          "alpha": lambda f: f["g"] is False, "action": ("modify", {"g": True})},
         {"name": "D", "load": 2, "sal": -10, "kind": "simple", "type": "B",
          "alpha": lambda f: f["g"] is True, "action": ("delete",)},
     ],
     "facts": [("L", {"id": 1}), ("L", {"id": 2}), ("L", {"id": 3}),
               ("B", {"g": False})],
     "expected": [("R", 1), ("R", 2), ("R", 3), ("U", "?"), ("D", "?"),
                  ("R", 3), ("R", 2), ("R", 1)]},
    {"name": "r1_four_candidates",
     "rules": relay_rules(),
     "facts": [("T0", {"f0": 4, "f2": False}), ("T0", {"f0": -5, "f2": False}),
               ("T1", {"f0": 4}), ("T0", {"f0": -1000000007, "f2": False}),
               ("T0", {"f0": 9, "f2": False})],
     "expected": [("R1", 9), ("R0", 9), ("R1", -1000000007),
                  ("R0", -1000000007), ("R1", -5), ("R0", -5),
                  ("R1", 4), ("R0", 4)]},
    {"name": "r2_initial_block",
     "rules": relay_rules(),
     "facts": [("T0", {"f0": 4, "f2": False}), ("T0", {"f0": -5, "f2": False}),
               ("T1", {"f0": 4}), ("T0", {"f0": 7, "f2": True}),
               ("T0", {"f0": -1000000007, "f2": False})],
     "expected": [("R0", 7), ("R1", -1000000007), ("R0", -1000000007),
                  ("R1", -5), ("R0", -5), ("R1", 4), ("R0", 4)]},
    {"name": "r3_fresh_release",
     "rules": [
         {"name": "R", "load": 0, "sal": 0, "kind": "not",
          "left": ("L", TRUE_), "join": None,
          "notp": ("B", lambda f: f["g"] is True, lambda lf, rf: True),
          "action": None},
         {"name": "D", "load": 1, "sal": -10, "kind": "simple", "type": "B",
          "alpha": lambda f: f["g"] is True, "action": ("delete",)},
     ],
     "facts": [("L", {"id": 1}), ("L", {"id": 2}), ("L", {"id": 3}),
               ("B", {"g": True})],
     "expected": [("D", "?"), ("R", 3), ("R", 2), ("R", 1)]},
    {"name": "r4_sametype_release",
     "rules": [
         {"name": "R", "load": 0, "sal": 0, "kind": "not",
          "left": ("L", TRUE_), "join": ("M", TRUE_),
          "notp": ("L", lambda f: f["g"] is True, lambda lf, rf: True),
          "action": None},
         {"name": "D", "load": 1, "sal": 5, "kind": "simple", "type": "L",
          "alpha": lambda f: f["g"] is True, "action": ("delete",)},
     ],
     "facts": [("L", {"id": 1, "g": False}), ("L", {"id": 2, "g": False}),
               ("M", {"m": 7}), ("L", {"id": 3, "g": True})],
     "expected": [("D", 3), ("R", 2), ("R", 1)]},
    {"name": "fz_min_7_2364",
     "rules": relay_rules(),
     "facts": [("T0", {"f0": 4, "f2": False}), ("T0", {"f0": -5, "f2": False}),
               ("T1", {"f0": 4}), ("T0", {"f0": -1000000007, "f2": False})],
     "expected": [("R1", -1000000007), ("R0", -1000000007), ("R1", -5),
                  ("R0", -5), ("R1", 4), ("R0", 4)]},
    {"name": "nb3",
     "rules": [
         {"name": "R0", "load": 0, "sal": 0, "kind": "simple", "type": "L",
          "alpha": lambda f: f["g"] is True, "action": ("delete",)},
         {"name": "R1", "load": 1, "sal": 0, "kind": "not",
          "left": ("L", TRUE_), "join": ("M", TRUE_),
          "notp": ("L", lambda f: f["g"] is True, lambda lf, rf: True),
          "action": ("modify", {"g": True})},
     ],
     "facts": [("L", {"id": 1, "g": False}), ("L", {"id": 2, "g": False}),
               ("M", {"m": 7}), ("L", {"id": 3, "g": False})],
     "expected": [("R1", 3), ("R0", 3), ("R1", 2), ("R0", 2),
                  ("R1", 1), ("R0", 1)]},
    # r5: the clash-axis discriminator — the staged blocker (f1=5) would
    # block the queue head b (f0=1) but not a (f0=10); annihilation keeps
    # the queue untouched, keep/eager cancels+re-releases b to the tail.
    {"name": "r5_partial_block",
     "rules": relay_rules(l_alpha=lambda f: f["f2"] is False,
                          beta=lambda lf, rf: rf["f1"] > lf["f0"]),
     "facts": [("T0", {"f0": 10, "f1": 0, "f2": False}),
               ("T0", {"f0": 1, "f1": 0, "f2": False}),
               ("T1", {"f0": 4}),
               ("T0", {"f0": 0, "f1": 5, "f2": False})],
     "expected": [("R1", 0), ("R0", 0), ("R1", 1), ("R0", 1),
                  ("R1", 10), ("R0", 10)]},
    # fz_7_2364: left alpha f2==false (modify EXITS the join), beta-
    # constrained not (f1 > left f0), initial blocker t3(-4,6,true).
    # R2 is inert (its T1(f3 != true) alpha fails on the only T1) — omitted.
    {"name": "fz_7_2364",
     "rules": relay_rules(
         l_alpha=lambda f: f["f2"] is False,
         r0_alpha=lambda f: f["f1"] != 8 and f["f2"] is True,
         beta=lambda lf, rf: rf["f1"] > lf["f0"]),
     "facts": [("T0", {"f0": 4, "f1": 3, "f2": False}),
               ("T0", {"f0": -5, "f1": 3, "f2": False}),
               ("T1", {"f0": 4, "f1": -0.5, "f2": 3, "f3": True}),
               ("T0", {"f0": -4, "f1": 6, "f2": True}),
               ("T0", {"f0": -1000000007, "f1": 5, "f2": False})],
     "expected": [("R0", -4), ("R1", -1000000007), ("R0", -1000000007),
                  ("R1", -5), ("R0", -5), ("R1", 4), ("R0", 4)]},
]

# The engine's measured sequences on the fork cells (a20dd5a, engine at
# 5b0083c): eager evaluation makes the block+release REAL — recorded here
# as the divergence the port removes, not as a fitness target.
ENGINE_FORKS = {
    "r5_partial_block": [("R1", 0), ("R0", 0), ("R1", 10), ("R0", 10),
                         ("R1", 1), ("R0", 1)],
    "r1_four_candidates": [("R1", 9), ("R0", 9), ("R1", 4), ("R0", 4),
                           ("R1", -1000000007), ("R0", -1000000007),
                           ("R1", -5), ("R0", -5)],
    "r2_initial_block": [("R0", 7), ("R1", -1000000007),
                         ("R0", -1000000007), ("R1", 4), ("R0", 4),
                         ("R1", -5), ("R0", -5)],
    "fz_min_7_2364": [("R1", -1000000007), ("R0", -1000000007), ("R1", 4),
                      ("R0", 4), ("R1", -5), ("R0", -5)],
    "nb3": [("R1", 3), ("R0", 3), ("R1", 1), ("R0", 1), ("R1", 2), ("R0", 2)],
}


def main():
    axes = list(itertools.product(["lazy", "eager"], ["annihilate", "keep"],
                                  ["head", "tail"], ["prepend", "append"]))
    survivors = []
    for evalmode, clash, relwalk, blkbuild in axes:
        fails = []
        for cell in CELLS:
            got = Machine(cell, evalmode, clash, relwalk, blkbuild).run()
            if got != cell["expected"]:
                fails.append(cell["name"])
        tag = f"{evalmode:5s} {clash:10s} rel={relwalk:4s} blk={blkbuild:7s}"
        if fails:
            print(f"  {tag}  FAIL {len(fails)}/9: {', '.join(fails)}")
        else:
            print(f"  {tag}  SURVIVES 9/9")
            survivors.append((evalmode, clash, relwalk, blkbuild))
    print()
    gauge = [("lazy", "annihilate", "head", "prepend"),
             ("lazy", "annihilate", "tail", "append")]
    if survivors == gauge:
        print("UNIQUE SURVIVOR CLASS = the source machine (lazy, annihilate)"
              " x the {head+prepend == tail+append} gauge pair (rev∘rev=id:"
              " both mean 'release visits newest-blocked-first')")
    else:
        print(f"UNEXPECTED survivors: {survivors}")
        return 1
    # the eager source machine must reproduce the engine's fork sequences
    print()
    ok = True
    for cell in CELLS:
        if cell["name"] not in ENGINE_FORKS:
            continue
        got = Machine(cell, "eager", "annihilate", "head", "prepend").run()
        m = "matches" if got == ENGINE_FORKS[cell["name"]] else "DIFFERS"
        ok = ok and got == ENGINE_FORKS[cell["name"]]
        print(f"  eager-mode vs engine on {cell['name']}: {m}")
        if got != ENGINE_FORKS[cell["name"]]:
            print(f"    eager: {got}")
            print(f"    engine: {ENGINE_FORKS[cell['name']]}")
    print()
    print("eager source machine %s the engine's fork sequences"
          % ("REPRODUCES" if ok else "does NOT fully reproduce"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
