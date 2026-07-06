#!/usr/bin/env python3
"""model_check_join2.py — D-083 discriminator elimination.

D-082 pinned that jw3 (rule-produced fresh right) and jr10 (external
update-entry right) have opposite oracle orders and landed a late pass
for ALL update-entry rights — leaving 7 certified rule-origin-modify
scenarios red (u12/u13/u16, fz_42_1176, fz_42_3408, fz_777_3846,
fz_999_3298). This checker replays the full join pipeline (the engine's
certified do_join_node mechanics, ported 1:1) over timelines extracted
from ALL of: the jw fresh-right matrix, the jr1..jr10 external re-entry
ladder, and the 7 counterexamples' oracle logs — with the update-entry
right treatment as the ONLY free dimensions:

  gate       which update-entries take the special late path:
             provenance   external ones only (first D-083 candidate)
             reentry      those whose fact has a staged DEL at the same
                          node in the same batch (out-and-back;
                          D-081's existential signature at joins)
             always_late  all of them (the D-082 landing)
             never        none (pure pre-D-082)
  late_pos   rinsf | late     where selected entries process
  late_walk  memfwd | lseqdesc
  late_emit  lifo | fifo

Non-selected entries are PLAIN right inserts (rightInserts slot,
post-reorder memory-forward walk, LIFO). 4 x 2 x 2 x 2 = 32 machines.

Round 2 (this file's final form): the fuzz gate on the round-1
survivor (gate=provenance) caught fz_42_440 — an EXTERNAL PURE-entry
behaving PLAIN on a linked node — and probes jr11/16/17/18 filled the
pure/re-entry x action/facts-insert matrix: pure entries are plain
regardless of provenance or insert flavor; re-entrants take the late
pass. jr10 (pure, external) is explained by never-linked staging
accumulation (fz_7_145), which the replica models with a link gate.

Certified mechanics held FIXED (all pinned pre-D-082): LIFO staging,
head-first consumption, memory append-on-process, reorder re-appends
staged-upd lefts at the END (staged-list order) with child reAdds,
Rupd/Lupd sync-walks emitting child updates in bucket order (cursor
threading), fresh right-inserts walk the post-reorder lefts bucket
memory-forward with LIFO emission, left-inserts walk rights memory-
forward with LIFO emission, terminal drains dels -> upds -> ins
head-first. Expected sequences are the ORACLE firing logs (engine logs
where both agree).
"""

import itertools
import sys


class Staged:
    def __init__(self):
        self.ins = []  # (item, prov) newest first
        self.upd = []
        self.dele = []

    def add_ins(self, t, prov):
        if any(x == t for x, _ in self.ins) or any(x == t for x, _ in self.upd):
            return
        self.ins.insert(0, (t, prov))

    def add_upd(self, t, prov):
        if (any(x == t for x, _ in self.ins) or any(x == t for x, _ in self.upd)
                or any(x == t for x, _ in self.dele)):
            return
        self.upd.insert(0, (t, prov))

    def add_del(self, t, prov):
        for i, (x, _) in enumerate(self.ins):
            if x == t:
                del self.ins[i]  # cancel unpropagated insert
                return
        for i, (x, _) in enumerate(self.upd):
            if x == t:
                del self.upd[i]
                break
        if any(x == t for x, _ in self.dele):
            return
        self.dele.insert(0, (t, prov))


class Node:
    """One join node: lefts are tuples (of fact ids), rights are fact ids."""

    def __init__(self, name, key_left=None, key_right=None):
        self.name = name
        self.lefts = []            # memory order (append on process)
        self.rights = []
        self.children = {}         # cid -> dict(left, right, tuple, dead)
        self.by_left = {}          # left-tuple -> [cid] (append on create)
        self.by_right = {}         # right-fact -> [cid]
        self.next_cid = 0
        self.lseq = {}
        self.lseq_next = 1
        self.key_left = key_left   # fn(tuple) -> key, or None (unindexed)
        self.key_right = key_right

    def create_child(self, l, f, before_left=None, before_right=None):
        t = tuple(l) + (f,)
        cid = self.next_cid
        self.next_cid += 1
        self.children[cid] = dict(left=tuple(l), right=f, tuple=t, dead=False)
        lv = self.by_left.setdefault(tuple(l), [])
        if before_left is not None and before_left in lv:
            lv.insert(lv.index(before_left), cid)
        else:
            lv.append(cid)
        rv = self.by_right.setdefault(f, [])
        if before_right is not None and before_right in rv:
            rv.insert(rv.index(before_right), cid)
        else:
            rv.append(cid)
        return t

    def kill_child(self, cid):
        self.children[cid]["dead"] = True
        return self.children[cid]["tuple"]

    def re_add_left(self, cid):
        lv = self.by_left.get(self.children[cid]["left"], [])
        if cid in lv:
            lv.remove(cid)
            lv.append(cid)

    def re_add_right(self, cid):
        rv = self.by_right.get(self.children[cid]["right"], [])
        if cid in rv:
            rv.remove(cid)
            rv.append(cid)

    def alive_by_left(self, l):
        return [c for c in self.by_left.get(tuple(l), []) if not self.children[c]["dead"]]

    def alive_by_right(self, f):
        return [c for c in self.by_right.get(f, []) if not self.children[c]["dead"]]

    def stamp_left_seq(self, l):
        if tuple(l) not in self.lseq:
            self.lseq[tuple(l)] = self.lseq_next
            self.lseq_next += 1

    def lefts_bucket(self, rkey):
        if self.key_left is None:
            return list(self.lefts)
        return [l for l in self.lefts if self.key_left(l) == rkey]

    def rights_bucket(self, lkey):
        if self.key_right is None:
            return list(self.rights)
        return [f for f in self.rights if self.key_right(f) == lkey]


def do_join(model, node, sl, sr, trg):
    """Port of do_join_node with the RU treatment as model dimensions.

    'ins' staged entries carry prov in {'fresh', 'rule_ru', 'ext_ru'}:
    fresh = plain insert (initial fact, RHS insert, external insert),
    *_ru = alpha entry via update, by provenance.
    """
    # right deletes
    for f, _ in sr.dele:
        if f in node.rights:
            node.rights.remove(f)
        for c in list(node.alive_by_right(f)):
            trg.add_del(node.kill_child(c), None)
    # left deletes
    for l, _ in sl.dele:
        node.lefts = [x for x in node.lefts if tuple(x) != tuple(l)]
        for c in list(node.alive_by_left(l)):
            trg.add_del(node.kill_child(c), None)
    # reorder right memory: staged-upd rights move to END; children reAddLeft
    for f, _ in sr.upd:
        if f in node.rights:
            node.rights.remove(f)
            node.rights.append(f)
        for c in node.alive_by_right(f):
            node.re_add_left(c)
    # reorder left memory: remove all staged-upd lefts, re-append in staged
    # LIST order; children reAddRight
    for l, _ in sl.upd:
        node.lefts = [x for x in node.lefts if tuple(x) != tuple(l)]
    for l, _ in sl.upd:
        node.lefts.append(tuple(l))
        for c in node.alive_by_left(l):
            node.re_add_right(c)

    def staged_left_upd(l):
        return any(tuple(x) == tuple(l) for x, _ in sl.upd)

    # right updates (property-hot rights staying in alpha): sync-walk
    for f, _ in sr.upd:
        rkey = node.key_right(f) if node.key_right else None
        bucket = node.lefts_bucket(rkey)
        alive = node.alive_by_right(f)
        ci = 0
        for l in bucket:
            if staged_left_upd(l):
                continue
            cur = alive[ci] if ci < len(alive) else None
            if cur is not None and node.children[cur]["left"] == tuple(l):
                trg.add_upd(node.children[cur]["tuple"], None)
                node.re_add_left(cur)
                ci += 1
            else:
                t = node.create_child(l, f, before_left=cur)
                trg.add_ins(t, "upd_derived")
    # left updates: sync-walk against the rights bucket
    for l, _ in sl.upd:
        if tuple(l) not in [tuple(x) for x in node.lefts]:
            continue
        lkey = node.key_left(l) if node.key_left else None
        bucket = node.rights_bucket(lkey)
        if node.key_left is not None:
            # stale-children pass (indexed only)
            for c in list(node.alive_by_left(l)):
                if node.children[c]["right"] not in bucket:
                    trg.add_del(node.kill_child(c), None)
        alive = node.alive_by_left(l)
        ci = 0
        for f in bucket:
            cur = alive[ci] if ci < len(alive) else None
            if cur is not None and node.children[cur]["right"] == f:
                trg.add_upd(node.children[cur]["tuple"], None)
                node.re_add_right(cur)
                ci += 1
            else:
                t = node.create_child(l, f, before_right=cur)
                trg.add_ins(t, "upd_derived")

    staged_del_facts = {f for f, _ in sr.dele}

    def takes_late_path(f, prov):
        gate = model["gate"]
        if gate == "provenance":
            return prov == "ext_ru"
        if gate == "reentry":
            return f in staged_del_facts
        if gate == "always_late":
            return True
        return False  # never

    def ru_mode():
        return model["late_pos"], model["late_walk"], model["late_emit"]

    fifo_queue = []  # fifo-emitted RU children (appended to trg.ins at the end)

    def right_insert(f, walk, emit):
        rkey = node.key_right(f) if node.key_right else None
        node.rights.append(f)
        bucket = node.lefts_bucket(rkey)
        if walk == "lseqdesc":
            bucket = sorted(bucket, key=lambda l: -node.lseq.get(tuple(l), 0))
        for l in bucket:
            t = node.create_child(l, f)
            if emit == "lifo":
                trg.add_ins(t, "right")
            else:
                fifo_queue.append(t)

    # plain right inserts + rinsf-positioned selected entries (head-first)
    for f, prov in sr.ins:
        if not takes_late_path(f, prov):
            right_insert(f, "memfwd", "lifo")
        elif ru_mode()[0] == "rinsf":
            right_insert(f, ru_mode()[1], ru_mode()[2])
    # left inserts: stamp lseq oldest-first, process staged head-first
    for l, _ in reversed(sl.ins):
        node.stamp_left_seq(l)
    for l, _ in sl.ins:
        node.lefts.append(tuple(l))
        lkey = node.key_left(l) if node.key_left else None
        for f in node.rights_bucket(lkey):
            trg.add_ins(node.create_child(l, f), "left")
    # late-positioned selected entries
    for f, prov in sr.ins:
        if takes_late_path(f, prov) and ru_mode()[0] == "late":
            right_insert(f, ru_mode()[1], ru_mode()[2])

    for t in fifo_queue:
        trg.ins.append((t, "right"))


def run_scenario(model, scen):
    """scen: dict(nodes=[...], batches=[...]).

    Batch keys per node name: (sl_events, sr_events) in WM-EVENT order
    (the replica stages LIFO); a derived node's sl comes from the
    upstream node's trg. Events: ('ins', item[, prov]) / ('upd', item) /
    ('del', item). 'expect': the batch's full firing list. 'acc_dirty':
    a downstream accumulate result changed — refire all existing
    activations (creation order) before this batch's own output.
    'set_values': fact-value mutations applied before the batch (live
    index keys)."""
    nodes = [Node(s.get("name", f"n{i}"), s.get("key_left"), s.get("key_right"))
             for i, s in enumerate(scen["nodes"])]
    pend_sl = [Staged() for _ in nodes]  # never-linked accumulation (fz_7_145)
    activations = []
    ok = True
    outs = []
    for batch in scen["batches"]:
        for f, v in batch.get("set_values", {}).items():
            scen["values"][f] = v
        trg_prev = None
        for i, node in enumerate(nodes):
            sl, sr = pend_sl[i], Staged()
            sl_ev, sr_ev = batch.get(node.name, (None, None))
            for ev in sl_ev or []:
                kind, item = ev[0], ev[1]
                prov = ev[2] if len(ev) > 2 else "fresh"
                item = (item,) if not isinstance(item, tuple) else item
                {"ins": sl.add_ins, "upd": sl.add_upd, "del": sl.add_del}[kind](item, prov)
            if trg_prev is not None:
                for t, _ in reversed(trg_prev.dele):
                    sl.add_del(t, None)
                for t, _ in reversed(trg_prev.upd):
                    sl.add_upd(t, None)
                for t, prov in reversed(trg_prev.ins):
                    sl.add_ins(t, prov)
            for ev in sr_ev or []:
                kind, item = ev[0], ev[1]
                prov = ev[2] if len(ev) > 2 else "fresh"
                {"ins": sr.add_ins, "upd": sr.add_upd, "del": sr.add_del}[kind](item, prov)
            if not node.rights and not (sr.ins or sr.upd or sr.dele):
                # never linked: right side empty and stays empty — the
                # staged left input accumulates (fz_7_145 / jr10)
                trg_prev = Staged()
                continue
            pend_sl[i] = Staged()
            trg = Staged()
            do_join(model, node, sl, sr, trg)
            trg_prev = trg
        # terminal: dels cancel, upds fire (refires), ins fire
        fired = []
        if batch.get("acc_dirty"):
            fired.extend(activations)
        for t, _ in trg_prev.dele:
            if t in activations:
                activations.remove(t)
        for t, _ in trg_prev.upd:
            fired.append(t)
        for t, _ in trg_prev.ins:
            fired.append(t)
            activations.append(t)
        exp = [tuple(e) for e in batch["expect"]]
        outs.append((fired, exp))
        if fired != exp:
            ok = False
    return ok, outs


# ---------------------------------------------------------------- timelines
# Facts are small ints; join prefixes are tuples of them. Event lists are in
# WM-EVENT order. Expected sequences come from the oracle firing logs
# (SEINE_HANDLES runs, 2026-07-06).

def _1j(fires):
    return dict(
        nodes=[dict(name="j")],
        batches=[dict(j=(sl, sr), expect=exp) for sl, sr, exp in fires],
    )


SCENARIOS = {}

# --- jw fresh-right matrix (rule-origin fresh inserts; pr_hw_joinwin*) ---
SCENARIOS["jw3"] = _1j([
    ([("ins", 1)], [("ins", 2)], [(1, 2)]),
    ([("ins", 3)], [("ins", 4)], [(3, 4), (3, 2), (1, 4)]),
])
SCENARIOS["jw4"] = _1j([
    ([("ins", 2)], [("ins", 1)], [(2, 1)]),
    ([("ins", 4)], [("ins", 3)], [(4, 3), (4, 1), (2, 3)]),
])
SCENARIOS["jw5"] = _1j([
    ([("ins", 1)], [("ins", 2)], [(1, 2)]),
    ([("ins", 3)], [("ins", 4)], [(3, 4), (3, 2), (1, 4)]),
    ([("ins", 5)], [("ins", 6)], [(5, 6), (5, 4), (5, 2), (3, 6), (1, 6)]),
])

# --- jr external re-entry ladder (pr_hw_jr*; epoch actions, merged batch) ---
SCENARIOS["jr1"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([], [("del", 5), ("ins", 5, "ext_ru")], [(1, 5)]),
])
SCENARIOS["jr3"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2)], [("del", 5), ("ins", 5, "ext_ru")], [(1, 5), (2, 5)]),
])
SCENARIOS["jr5"] = SCENARIOS["jr3"]  # same staged state, different action order
SCENARIOS["jr6"] = SCENARIOS["jr3"]
SCENARIOS["jr7"] = _1j([
    ([("ins", 1), ("ins", 2)], [("ins", 5)], [(1, 5), (2, 5)]),
    ([], [("del", 5), ("ins", 5, "ext_ru")], [(1, 5), (2, 5)]),
])
SCENARIOS["jr8"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2), ("ins", 3)], [("del", 5), ("ins", 5, "ext_ru")],
     [(1, 5), (2, 5), (3, 5)]),
])
SCENARIOS["jr9"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2)], [("del", 5)], []),
])
SCENARIOS["jr10"] = _1j([
    ([("ins", 1)], [], []),
    ([("ins", 2)], [("ins", 5, "ext_ru")], [(1, 5), (2, 5)]),
])

# --- round-2 probes: the pure-entry / re-entry x insert-flavor matrix ---
# jr11: PURE external entry + same-batch ACTION-insert (linked node)
SCENARIOS["jr11"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2)], [("ins", 6, "ext_ru")], [(2, 6), (2, 5), (1, 6)]),
])
# jr16: PURE entry + two action-inserts (walk-direction sensitive)
SCENARIOS["jr16"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2), ("ins", 3)], [("ins", 6, "ext_ru")],
     [(2, 6), (2, 5), (3, 6), (3, 5), (1, 6)]),
])
# jr17: RE-entry + FACTS-insert
SCENARIOS["jr17"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2)], [("del", 5), ("ins", 5, "ext_ru")], [(1, 5), (2, 5)]),
])
# jr18: PURE entry + FACTS-insert (fz_42_440 minimal)
SCENARIOS["jr18"] = _1j([
    ([("ins", 1)], [("ins", 5)], [(1, 5)]),
    ([("ins", 2)], [("ins", 6, "ext_ru")], [(2, 6), (2, 5), (1, 6)]),
])

# --- fz_42_440 (R2's node): T0-lefts x T1-rights; external PURE entry of
# T1#3 with a same-epoch facts-insert T0#8; second epoch adds T0#16 ---
SCENARIOS["fz_42_440"] = _1j([
    ([("ins", 0)], [("ins", 4)], [(0, 4)]),
    ([("ins", 8)], [("ins", 3, "ext_ru")], [(8, 3), (8, 4), (0, 3)]),
    ([("ins", 16)], [], [(16, 3), (16, 4)]),
])

# --- u12: A() x A($b:g) x A(g==true); flip = rule modify A1 (g F->T) ---
SCENARIOS["u12"] = dict(
    nodes=[dict(name="n1"), dict(name="n2")],
    batches=[
        dict(n1=([("ins", 0), ("ins", 1), ("ins", 2)],
                 [("ins", 0), ("ins", 1), ("ins", 2)]),
             n2=(None, [("ins", 0), ("ins", 2)]),
             expect=[(2, 2, 0), (2, 2, 2), (2, 1, 0), (2, 1, 2), (2, 0, 0), (2, 0, 2),
                     (1, 2, 0), (1, 2, 2), (1, 1, 0), (1, 1, 2), (1, 0, 0), (1, 0, 2),
                     (0, 2, 0), (0, 2, 2), (0, 1, 0), (0, 1, 2), (0, 0, 0), (0, 0, 2)]),
        dict(n1=([], [("upd", 1)]),
             n2=(None, [("ins", 1, "rule_ru")]),
             expect=[(2, 1, 0), (2, 1, 2), (1, 1, 0), (1, 1, 2), (0, 1, 0), (0, 1, 2),
                     (2, 1, 1), (1, 1, 1), (0, 1, 1),
                     (2, 2, 1), (2, 0, 1), (1, 2, 1), (1, 0, 1), (0, 2, 1), (0, 0, 1)]),
    ],
)

# --- u13: B() x A($b:g) x A(g==true); flip = rule modify A3 ---
SCENARIOS["u13"] = dict(
    nodes=[dict(name="n1"), dict(name="n2")],
    batches=[
        dict(n1=([("ins", 100), ("ins", 101)],
                 [("ins", 2), ("ins", 3), ("ins", 4)]),
             n2=(None, [("ins", 2), ("ins", 4)]),
             expect=[(101, 4, 2), (101, 4, 4), (101, 3, 2), (101, 3, 4),
                     (101, 2, 2), (101, 2, 4), (100, 4, 2), (100, 4, 4),
                     (100, 3, 2), (100, 3, 4), (100, 2, 2), (100, 2, 4)]),
        dict(n1=([], [("upd", 3)]),
             n2=(None, [("ins", 3, "rule_ru")]),
             expect=[(101, 3, 2), (101, 3, 4), (100, 3, 2), (100, 3, 4),
                     (101, 3, 3), (100, 3, 3), (101, 4, 3), (101, 2, 3),
                     (100, 4, 3), (100, 2, 3)]),
    ],
)

# --- u16: B() x A($b:g) x A(g==true); two flips (A2, then A4) ---
SCENARIOS["u16"] = dict(
    nodes=[dict(name="n1"), dict(name="n2")],
    batches=[
        dict(n1=([("ins", 100)], [("ins", 1), ("ins", 2), ("ins", 3), ("ins", 4)]),
             n2=(None, [("ins", 1), ("ins", 3)]),
             expect=[(100, 4, 1), (100, 4, 3), (100, 3, 1), (100, 3, 3),
                     (100, 2, 1), (100, 2, 3), (100, 1, 1), (100, 1, 3)]),
        dict(n1=([], [("upd", 2)]),
             n2=(None, [("ins", 2, "rule_ru")]),
             expect=[(100, 2, 1), (100, 2, 3),
                     (100, 2, 2), (100, 4, 2), (100, 3, 2), (100, 1, 2)]),
        dict(n1=([], [("upd", 4)]),
             n2=(None, [("ins", 4, "rule_ru")]),
             expect=[(100, 4, 2), (100, 4, 1), (100, 4, 3),
                     (100, 4, 4), (100, 2, 4), (100, 3, 4), (100, 1, 4)]),
    ],
)

# --- fz_42_1176: T0($b:f1) x T1 x T0(f1 != false, f1 == $b); INDEXED node2.
# T0.f1: 0=T 1=F(->T at the flip) 2=T 5=T; rights (T1): 3, 4, 6, 7.
_V1176 = {}

SCENARIOS["fz_42_1176"] = dict(
    values=_V1176,
    init_values={0: True, 1: False, 2: True, 5: True},
    nodes=[dict(name="n1"),
           dict(name="n2",
                key_left=lambda l: _V1176[l[0]],
                key_right=lambda f: _V1176[f])],
    batches=[
        dict(n1=([("ins", 0), ("ins", 1), ("ins", 2), ("ins", 5)],
                 [("ins", 3), ("ins", 4)]),
             n2=(None, [("ins", 0), ("ins", 2), ("ins", 5)]),
             expect=[(5, 4, 0), (5, 4, 2), (5, 4, 5), (5, 3, 0), (5, 3, 2), (5, 3, 5),
                     (2, 4, 0), (2, 4, 2), (2, 4, 5), (2, 3, 0), (2, 3, 2), (2, 3, 5),
                     (0, 4, 0), (0, 4, 2), (0, 4, 5), (0, 3, 0), (0, 3, 2), (0, 3, 5)]),
        dict(n1=([], [("ins", 6)]),
             expect=[(5, 6, 0), (5, 6, 2), (5, 6, 5),
                     (2, 6, 0), (2, 6, 2), (2, 6, 5),
                     (0, 6, 0), (0, 6, 2), (0, 6, 5)]),
        dict(n1=([], [("ins", 7)]),
             expect=[(5, 7, 0), (5, 7, 2), (5, 7, 5),
                     (2, 7, 0), (2, 7, 2), (2, 7, 5),
                     (0, 7, 0), (0, 7, 2), (0, 7, 5)]),
        dict(n1=([("upd", 1)], []),
             n2=(None, [("ins", 1, "rule_ru")]),
             set_values={1: True},
             expect=[(1, 4, 1), (1, 3, 1), (1, 6, 1), (1, 7, 1),
                     (5, 7, 1), (2, 7, 1), (0, 7, 1),
                     (5, 6, 1), (2, 6, 1), (0, 6, 1),
                     (5, 4, 1), (5, 3, 1), (2, 4, 1), (2, 3, 1), (0, 4, 1), (0, 3, 1),
                     (1, 4, 0), (1, 4, 2), (1, 4, 5), (1, 3, 0), (1, 3, 2), (1, 3, 5),
                     (1, 6, 0), (1, 6, 2), (1, 6, 5), (1, 7, 0), (1, 7, 2), (1, 7, 5)]),
    ],
)

# --- fz_42_3408: T0() x T0($b:f1) x T0(f1==true); two rule-update flushes ---
SCENARIOS["fz_42_3408"] = dict(
    nodes=[dict(name="n1"), dict(name="n2")],
    batches=[
        dict(n1=([("ins", 1), ("ins", 3), ("ins", 4), ("ins", 5)],
                 [("ins", 1), ("ins", 3), ("ins", 4), ("ins", 5)]),
             n2=(None, [("ins", 3), ("ins", 4)]),
             expect=[(5, 5, 3), (5, 5, 4), (5, 4, 3), (5, 4, 4), (5, 3, 3), (5, 3, 4),
                     (5, 1, 3), (5, 1, 4),
                     (4, 5, 3), (4, 5, 4), (4, 4, 3), (4, 4, 4), (4, 3, 3), (4, 3, 4),
                     (4, 1, 3), (4, 1, 4),
                     (3, 5, 3), (3, 5, 4), (3, 4, 3), (3, 4, 4), (3, 3, 3), (3, 3, 4),
                     (3, 1, 3), (3, 1, 4),
                     (1, 5, 3), (1, 5, 4), (1, 4, 3), (1, 4, 4), (1, 3, 3), (1, 3, 4),
                     (1, 1, 3), (1, 1, 4)]),
        dict(n1=([], [("upd", 1)]),
             n2=(None, [("ins", 1, "rule_ru")]),
             expect=[(5, 1, 3), (5, 1, 4), (4, 1, 3), (4, 1, 4),
                     (3, 1, 3), (3, 1, 4), (1, 1, 3), (1, 1, 4),
                     (5, 1, 1), (4, 1, 1), (3, 1, 1), (1, 1, 1),
                     (5, 5, 1), (5, 4, 1), (5, 3, 1),
                     (4, 5, 1), (4, 4, 1), (4, 3, 1),
                     (3, 5, 1), (3, 4, 1), (3, 3, 1),
                     (1, 5, 1), (1, 4, 1), (1, 3, 1)]),
        dict(n1=([], [("upd", 5)]),
             n2=(None, [("ins", 5, "rule_ru")]),
             expect=[(5, 5, 1), (5, 5, 3), (5, 5, 4),
                     (4, 5, 1), (4, 5, 3), (4, 5, 4),
                     (3, 5, 1), (3, 5, 3), (3, 5, 4),
                     (1, 5, 1), (1, 5, 3), (1, 5, 4),
                     (5, 5, 5), (4, 5, 5), (3, 5, 5), (1, 5, 5),
                     (5, 1, 5), (4, 1, 5), (3, 1, 5), (1, 1, 5),
                     (5, 4, 5), (5, 3, 5), (4, 4, 5), (4, 3, 5),
                     (3, 4, 5), (3, 3, 5), (1, 4, 5), (1, 3, 5)]),
    ],
)

# --- fz_999_3298 (slice): T0($b:f1) x T0(f0 != false); rule flush updates 4 ---
SCENARIOS["fz_999_3298"] = dict(
    nodes=[dict(name="n1")],
    batches=[
        dict(n1=([("ins", 1), ("ins", 3), ("ins", 4), ("ins", 5)], [("ins", 3)]),
             expect=[(1, 3), (3, 3), (4, 3), (5, 3)]),
        dict(n1=([("upd", 4)], [("ins", 4, "rule_ru")]),
             expect=[(4, 3), (4, 4), (1, 4), (3, 4), (5, 4)]),
    ],
)

# --- fz_777_3846 (slice): T1(f1==true) x T1(f1==true, not-in) [+acc pass-thru]
# Left AND right entries via rule modify; the acc count change refires all
# existing activations (creation order).
SCENARIOS["fz_777_3846"] = dict(
    nodes=[dict(name="n1")],
    batches=[
        dict(n1=([("ins", 0, "rule_ru")], [("ins", 0, "rule_ru")]),
             expect=[(0, 0)]),
        dict(n1=([("ins", 1, "rule_ru")], []), acc_dirty=True,
             expect=[(0, 0), (1, 0)]),
        dict(n1=([], []), acc_dirty=True,
             expect=[(0, 0), (1, 0)]),
        dict(n1=([("ins", 5, "rule_ru")], [("ins", 5, "rule_ru")]), acc_dirty=True,
             expect=[(0, 0), (1, 0), (5, 5), (5, 0), (1, 5), (0, 5)]),
        dict(n1=([("ins", 7, "rule_ru")], []), acc_dirty=True,
             expect=[(0, 0), (1, 0), (5, 5), (5, 0), (1, 5), (0, 5),
                     (7, 5), (7, 0)]),
    ],
)


DIMS = dict(
    gate=["provenance", "reentry", "always_late", "never"],
    late_pos=["rinsf", "late"],
    late_walk=["memfwd", "lseqdesc"],
    late_emit=["lifo", "fifo"],
)


def check(model, collect=False):
    fails = []
    for name, scen in SCENARIOS.items():
        if "values" in scen:
            scen["values"].clear()
            scen["values"].update(scen["init_values"])
        ok, outs = run_scenario(model, scen)
        if not ok:
            first_bad = next((o for o in outs if o[0] != o[1]), None)
            fails.append((name, first_bad if collect else None))
    return fails


def main():
    keys = list(DIMS)
    survivors, results = [], []
    for combo in itertools.product(*(DIMS[k] for k in keys)):
        model = dict(zip(keys, combo))
        fails = check(model)
        results.append((model, fails))
        if not fails:
            survivors.append(model)
    print(f"{len(results)} models, {len(survivors)} survivors")
    for m in survivors:
        print("  SURVIVOR:", m)
    if not survivors:
        results.sort(key=lambda r: len(r[1]))
        for m, fails in results[:6]:
            print(f"  near-miss ({len(fails)} fail: {[f[0] for f in fails]}):")
            print(f"    {m}")
            name = fails[0][0]
            _, outs = run_scenario(m, prep(name))
            for got, exp in outs:
                if got != exp:
                    print(f"    {name} GOT {got}\n    {name} EXP {exp}")
                    break
    return 0 if survivors else 1


def prep(name):
    scen = SCENARIOS[name]
    if "values" in scen:
        scen["values"].clear()
        scen["values"].update(scen["init_values"])
    return scen


if __name__ == "__main__":
    sys.exit(main())
