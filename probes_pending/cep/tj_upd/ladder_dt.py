#!/usr/bin/env python3
"""D-169 double-touch discriminator ladder (the D-167 §2 residual).

Two candidate sub-rules over the v4 base (M = committed v4):
 T1 RETOUCH-REPOSITION: a later action on the SAME fact re-staging an
    already-staged UPD emission MOVES it to the current queue tail
    (ins-staged emissions absorb the re-touch and never move; a dup via
    a DIFFERENT fact keeps its first position — the u5 discipline).
 T2 DEFERRED PHASE-C: the every-update tail re-add ($b memory, and the
    anchor's left-memory re-add) is applied at ACTION-LOOP END (before
    the epoch's fresh inserts), so entry scans by LATER ACTIONS in the
    same epoch see the epoch-start memory; inserts + next epochs see
    the moved memory (r-ladder durability preserved).

Cells: DT* same-fact double-touch; INT* 3-action interposers (out of
the fuzz generator's 2-action reach); EN* move-visibility; controls.
Usage: ladder_dt.py [--oracle-only]
"""
import json, sys, os, random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_tjupd_v4 as V


# ---------- T-variant simulator (v4 + T1 + T2) ----------
class MT(V.M):
    def __init__(self, lo, hi):
        super().__init__(lo, hi)
        self.entries = {}        # key -> buffer entry dict
        self.stager = None       # fact id of the current action target
        self.move_q = []         # deferred phase-C moves: (side, fact)

    # buffer entries: {"a","b","kind","stager"}
    def _emit(self, a, b, kind="ins"):
        key = (a["id"], b["id"])
        if self.pending is not None and key in self.pending:
            e = self.entries.get(key)
            if e is None or e["kind"] == "ins":
                return                      # ins absorbs the re-touch
            if self.stager is not None and e["stager"] == self.stager:
                self.buffer.remove(e)       # T1: same-fact retouch moves
            else:
                return                      # different fact: keep first
        if self.pending is not None:
            self.pending.add(key)
        e = {"a": a, "b": b, "kind": kind, "stager": self.stager}
        self.buffer.append(e)
        self.entries[key] = e

    def render(self):
        for e in self.buffer:
            a, b = e["a"], e["b"]
            self.fired.append(f"{a['ts']}{a['tag']}|{b['ts']}{b['tag']}")
        self.buffer = []
        self.entries = {}

    def left_delete(self, a):
        self.ltm = [x for x in self.ltm if x["id"] != a["id"]]
        killed = [k for k in self.children if k[0] == a["id"]]
        for k in killed:
            del self.children[k]
        self.childlist.pop(a["id"], None)
        kset = set(killed)
        self.buffer = [e for e in self.buffer
                       if (e["a"]["id"], e["b"]["id"]) not in kset]
        for k in kset:
            self.entries.pop(k, None)
        if self.pending is not None:
            self.pending -= kset

    # T2: defer the phase-C moves while inside the action loop
    def right_readd(self, f):
        if self.stager is not None:
            self.move_q.append(("r", f))
        else:
            super().right_readd(f)

    def left_readd(self, f):
        if self.stager is not None:
            self.move_q.append(("l", f))
        else:
            super().left_readd(f)

    def apply_moves(self):
        for side, f in self.move_q:
            if side == "r":
                super().right_readd(f)
            else:
                super().left_readd(f)
        self.move_q = []


def simulate_t(scn):
    node = MT(scn["lo"], scn["hi"])
    facts = {}
    nid = 0
    def insert(ts, tag, is_b, is_a_type):
        nonlocal nid
        f = {"id": nid, "ts": ts, "ts0": ts, "tag": tag,
             "a_type": is_a_type, "b_type": is_b}
        facts[nid] = f; nid += 1
        if is_a_type and tag == "z":
            node.left_insert(f)
        if is_b:
            node.right_insert(f)
        return f
    node.pending = set()
    for (ts, tag, kind) in scn["facts"]:
        insert(ts, tag, kind in ("b", "both"), kind in ("a", "both"))
    node.pending = None
    node.render()
    for ep in scn["epochs"]:
        node.pending = set()
        for act in ep["actions"]:
            f = facts[act["target"]]
            node.stager = f["id"]
            old_tag = f["tag"]
            new_tag = act.get("tag", old_tag)
            if "ts" in act:
                f["ts"] = act["ts"]
            f["tag"] = new_tag
            was_anchor = f["a_type"] and old_tag == "z"
            now_anchor = f["a_type"] and new_tag == "z"
            if not was_anchor and now_anchor:
                node.left_insert(f)
            elif was_anchor and not now_anchor:
                node.left_delete(f)
            elif was_anchor and now_anchor and "tag" in act:
                node.left_refire(f)
            if f["b_type"]:
                node.right_refire(f)
            if f["b_type"]:
                node.right_readd(f)
            if f["a_type"] and f["tag"] == "z" and "tag" in act:
                node.left_readd(f)
            node.stager = None
        node.apply_moves()                  # T2: moves land here
        for (ts, tag, kind) in ep["facts"]:
            insert(ts, tag, kind in ("b", "both"), kind in ("a", "both"))
        node.pending = None
        node.render()
    return node.fired


# ---------- the ladder ----------
# every cell: self-join, lo=0; facts = (ts, tag, "both")
def C(name, hi, facts, epochs):
    return {"name": name, "self_join": True, "lo": 0, "hi": hi,
            "facts": [(t, g, "both") for (t, g) in facts],
            "epochs": [{"actions": a, "facts": [(t, g, "both") for (t, g) in f]}
                       for (a, f) in epochs]}

CELLS = [
    # -- same-fact double-touch core (2-action) --
    C("dt1_noop_entry", 100, [(20, "y"), (60, "z")],
      [([{"target": 0, "tag": "y"}, {"target": 0, "tag": "z"}], [])]),
    C("dt2_exit_reentry", 100, [(30, "z"), (60, "z")],
      [([{"target": 0, "tag": "y"}, {"target": 0, "tag": "z"}], [])]),
    C("dt2b_exit_reentry_x227", 100, [(14, "y"), (77, "z"), (43, "z"), (0, "y")],
      [([{"target": 1, "tag": "y", "ts": 107}, {"target": 1, "tag": "z"}], [])]),
    C("dt3_entry_noop", 100, [(15, "y"), (20, "y"), (60, "z")],
      [([{"target": 1, "tag": "z"}, {"target": 1, "ts": 110}], [])]),
    C("dt4_inplace_inplace", 100, [(15, "y"), (60, "z")],
      [([{"target": 1, "tag": "z"}, {"target": 1, "tag": "z"}], [])]),
    C("dt6_noop_entry_x199", 50, [(28, "y"), (76, "y"), (74, "z"), (51, "y"), (44, "y")],
      [([{"target": 0, "tag": "y"}, {"target": 0, "tag": "z"}], [])]),
    # -- 3-action interposers (outside the fuzz generator's reach) --
    C("int1_samefact_move", 100, [(15, "y"), (20, "y"), (60, "z"), (65, "z")],
      [([{"target": 1, "ts": 19}, {"target": 0, "ts": 14}, {"target": 1, "ts": 18}], [])]),
    C("int2_difffact_keep", 100, [(15, "y"), (20, "y"), (60, "z"), (65, "z")],
      [([{"target": 1, "ts": 19}, {"target": 0, "ts": 14}, {"target": 2, "tag": "z"}], [])]),
    C("int3_entry_interposed", 100, [(15, "y"), (20, "y"), (25, "y"), (60, "z")],
      [([{"target": 1, "ts": 19}, {"target": 2, "tag": "z"}, {"target": 1, "ts": 18}], [])]),
    # -- phase-C move visibility --
    C("en1_move_vs_other_entry", 100, [(15, "y"), (20, "y"), (22, "y"), (30, "y")],
      [([{"target": 1, "ts": 19}, {"target": 3, "tag": "z"}], [])]),
    C("en3_move_vs_own_entry", 100, [(30, "y"), (15, "y"), (22, "y")],
      [([{"target": 0, "ts": 29}, {"target": 0, "tag": "z"}], [])]),
    C("en4_move_vs_insert", 100, [(15, "y"), (20, "y"), (22, "y")],
      [([{"target": 1, "ts": 19}], [(30, "z")])]),
    C("en2_move_crossepoch", 100, [(15, "y"), (20, "y"), (22, "y"), (30, "y")],
      [([{"target": 1, "ts": 19}], []),
       ([{"target": 3, "tag": "z"}], [])]),
]


def main():
    outdir = "/tmp/tjupd_ladder"
    os.makedirs(outdir, exist_ok=True)
    paths = []
    for s in CELLS:
        h = V.to_harness(s, s["name"])
        p = f"{outdir}/{s['name']}.json"
        json.dump(h, open(p, "w"), indent=1)
        paths.append(p)
    ora1 = V.oracle_seq(paths)
    ora2 = V.oracle_seq(paths)          # determinism check
    verdicts = []
    for s in CELLS:
        nm = s["name"]
        m = V.simulate(s)
        t = simulate_t(s)
        o = ora1.get(nm)
        stable = "STABLE" if ora1.get(nm) == ora2.get(nm) else "FLAKY!"
        vm = "M✓" if m == o else "M✗"
        vt = "T✓" if t == o else "T✗"
        verdicts.append((nm, vm, vt, stable))
        print(f"===== {nm}  [{vm} {vt}] ({stable})")
        print("  model :", m)
        print("  theory:", t)
        print("  oracle:", o)
    print("\n---- summary")
    for nm, vm, vt, st in verdicts:
        print(f"  {nm:28s} {vm} {vt} {st}")
    mt = sum(1 for _, vm, _, _ in verdicts if vm == "M✓")
    tt = sum(1 for _, _, vt, _ in verdicts if vt == "T✓")
    print(f"  M {mt}/{len(verdicts)}   T {tt}/{len(verdicts)}")


if __name__ == "__main__":
    main()
