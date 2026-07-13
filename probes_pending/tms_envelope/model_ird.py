#!/usr/bin/env python3
"""model_ird: executable spec of the three I-RD mechanisms (D-203..205).

THE LAWS AS CODE (predictions + commitments: ird-model-predictions.md):
 1. DYNAMIC (survive-the-delete, D-203): killing an UNSTAGE-BORN handle
    never cancels queued acts — they fire later with the dead handle's
    values. Every other kill cancels acts whose tuple contains the dead
    handle.
 2. STATIC key lifecycle (D-203/D-205): T1/T3 are keyed, T0/T2 premise
    types keyless. Stated inserts append WM-visible handles (key label =
    birth status). Logical insert: no key -> JUSTIFIED key, WM-visible
    belief; JUSTIFIED key -> dep fold; STATED-born key -> NON-WM pending
    belief. Key-death events: (i) last dep breaks -> belief dies, stated
    siblings ORPHAN (L6); (ii) stated delete on a stated-born MIXED key
    -> victim dies, siblings ORPHAN, pending belief UNSTAGES WM-visible
    unstage-born (r1; b1 = 0-sibling case). Orphans: alive, keyless,
    UNDELETABLE (x1). Later inserts of the value re-key FRESH; orphans
    are never adopted.
 3. SAME-BATCH SELF-BREAK (D-205): RHS ops apply in order; a break lands
    IMMEDIATELY (one path for same-batch-self and foreign) EXCEPT when
    dep.act == breaking act AND the dep tuple binds the broken fact >=2x
    (self-join): a lazy-break pseudo-item at the JUSTIFIER's salience.
    Form (modify/update) and source (update/delete) never branch.
 Tie-break: equal salience pops FIFO by act creation. Breaks cascade
 through kills recursively (a2). UNPINNED corners raise AssertionError.

Validation: `python3 model_ird.py` -> 22/22 vs truths/ird_*.ndj.
"""
import json
import os

DIR = os.path.dirname(os.path.abspath(__file__))
KEYED = ("T1", "T3")

# ---------- rule spec constructors ----------


def JL(sal, val="v", brk=None, selfjoin=False, prem="T0", belief=None,
       brkfields=None):
    return dict(kind="jl", sal=sal, val=val, brk=brk, selfjoin=selfjoin,
                prem=prem, belief=belief, brkfields=brkfields or {"f0": False})


def ST(sal, val="v"):
    return dict(kind="st", sal=sal, val=val)


def DEL(sal, val):
    return dict(kind="del", sal=sal, val=val)


def OBS(sal, typ="T1"):
    return dict(kind="obs", sal=sal, typ=typ)


def KS(sal, val, arm):
    return dict(kind="ks", sal=sal, val=val, arm=arm)


def KILLT0(sal, arm):
    return dict(kind="killt0", sal=sal, arm=arm)


def MIDT(sal, trig, trig_f1, val):
    return dict(kind="midt", sal=sal, trig=trig, trig_f1=trig_f1, val=val)


def RINS(sal, arm):
    return dict(kind="rins", sal=sal, arm=arm)


def JL2(sal, val="v"):
    return dict(kind="jl2", sal=sal, val=val)


def RUK(sal, arm):
    return dict(kind="ru", sal=sal, arm=arm)


T0F = ("T0", {"f0": True})


def V(f0="v", f1=True):
    return ("T1", {"f0": f0, "f1": f1})


CELLS = {
    "ird_a1_ord_fresh": ([T0F],
        {"RJ": JL(20, "tgt"), "RD": DEL(5, "tgt"), "ROBS": OBS(0)}),
    "ird_a2_ord_stale": ([T0F],
        {"RJ": JL(20, "tgt"), "RMID": MIDT(10, "tgt", True, "mid"),
         "RD": DEL(5, "tgt"), "ROBS": OBS(0)}),
    "ird_b1_unstage_fresh": ([T0F, V("tgt")],
        {"RJ": JL(20, "tgt"), "RD": DEL(5, "tgt"), "ROBS": OBS(0)}),
    "ird_b2_unstage_stale": ([T0F, V("tgt"), V("ks_arm", False), V("mid_arm", False)],
        {"RJ": JL(20, "tgt"), "RKS": KS(10, "tgt", "ks_arm"),
         "RMID": MIDT(5, "mid_arm", False, "mid"), "RD": DEL(3, "tgt"),
         "ROBS": OBS(0)}),
    "ird_c1_stated_ctl": ([T0F, V("tgt")],
        {"RD": DEL(5, "tgt"), "ROBS": OBS(0)}),
    "ird_l1_stated_x3": ([T0F, V(), V(), V()],
        {"ROBS": OBS(0)}),
    "ird_l2_stated_onto_justified": ([T0F],
        {"RJ": JL(20), "RS1": ST(10), "RS2": ST(8), "ROBS": OBS(0)}),
    "ird_l3_justified_onto_stated": ([T0F, V()],
        {"RJ": JL(20), "ROBS": OBS(0)}),
    "ird_l4_stated_rhs_onto_external": ([T0F, V()],
        {"RS1": ST(10), "RS2": ST(8), "ROBS": OBS(0)}),
    "ird_l5_break_orphan": ([T0F, V("karm", False)],
        {"RJ": JL(20), "RS1": ST(18), "RS2": ST(16),
         "RKILL": KILLT0(10, "karm"), "ROBS": OBS(0)}),
    "ird_l6_break_rejustify": ([T0F, V("karm", False), V("iarm", False)],
        {"RJ": JL(20), "RS1": ST(18), "RS2": ST(16),
         "RKILL": KILLT0(10, "karm"), "RINS": RINS(9, "iarm"),
         "RJ2": JL2(8), "ROBS": OBS(0)}),
    "ird_x1_orphan_del": ([T0F, V("karm", False)],
        {"RJ": JL(20), "RS1": ST(18), "RKILL": KILLT0(10, "karm"),
         "RD": DEL(5, "v"), "ROBS": OBS(0)}),
    "ird_r1_multistated_kill": ([T0F, V()],
        {"RJ": JL(18), "RS2": ST(16), "RD": DEL(5, "v"), "ROBS": OBS(0)}),
    "ird_r2_two_stated_ctl": ([T0F, V()],
        {"RS2": ST(16), "RD": DEL(5, "v"), "ROBS": OBS(0)}),
    "ird_m0_nobreak_ctl": ([T0F],
        {"RJ": JL(20), "ROBS": OBS(25)}),
    "ird_m1_samebatch_self_update": ([T0F],
        {"RJ": JL(20, brk="update"), "ROBS": OBS(25)}),
    "ird_m2_samebatch_self_modify": ([T0F],
        {"RJ": JL(20, brk="modify"), "ROBS": OBS(25)}),
    "ird_m3_samebatch_selfjoin_modify": ([("T2", {"f0": 1, "f1": False})],
        {"RJ": JL(0, brk="modify", selfjoin=True, prem="T2",
                  belief=("T3", {"f0": True, "f1": False}),
                  brkfields={"f1": True}),
         "ROBS": OBS(7, typ="T3")}),
    "ird_m4_laterbatch_foreign_update": ([T0F, V("arm", False)],
        {"RJ": JL(20), "RU": RUK(10, "arm"), "ROBS": OBS(0)}),
    "ird_m5_samebatch_self_del": ([T0F],
        {"RJ": JL(20, brk="delete"), "ROBS": OBS(25)}),
    "ird_m6_samebatch_selfjoin_modify_iso": ([T0F],
        {"RJ": JL(20, brk="modify", selfjoin=True), "ROBS": OBS(25)}),
    "ird_m7_samebatch_selfjoin_update_iso": ([T0F],
        {"RJ": JL(20, brk="update", selfjoin=True), "ROBS": OBS(25)}),
}

TRUTH_FILES = ["ird_oracle_r1.ndj", "ird_ladder_oracle_r1.ndj",
               "ird_l56_oracle_r1.ndj", "ird_x1_oracle_r1.ndj",
               "ird_rm_oracle_r1.ndj", "ird_m67_oracle_r1.ndj"]


class Sim:
    def __init__(self, facts, rules):
        self.rules = rules
        self.h = {}
        self.keys = {}
        self.acts = []
        self.pseudo = []
        self.seen = set()
        self.fired = []
        self._seq = 0
        self._hid = 0
        self._act = 0
        for typ, fields in facts:
            self.insert_stated(typ, dict(fields))

    # ---------- plumbing ----------
    def seq(self):
        self._seq += 1
        return self._seq

    def vk(self, typ, fields):
        return (typ, tuple(sorted(fields.items())))

    def new_handle(self, typ, fields, unstage=False):
        self._hid += 1
        self.h[self._hid] = dict(typ=typ, fields=fields, alive=True,
                                 orphan=False, unstage=unstage)
        return self._hid

    # ---------- alphas ----------
    def _pat(self, hid, typ, **eq):
        h = self.h[hid]
        return (h["alive"] and h["typ"] == typ
                and all(h["fields"].get(k) == v for k, v in eq.items()))

    def patterns(self, name):
        """Per-rule pattern predicates, one callable per LHS pattern."""
        r = self.rules[name]
        k = r["kind"]
        if k == "jl":
            prem = r["prem"]
            alpha = ({"f1": False} if prem == "T2" else {"f0": True})
            if r["selfjoin"]:
                return [lambda x: self._pat(x, prem),
                        lambda x: self._pat(x, prem, **alpha)]
            return [lambda x: self._pat(x, prem, **alpha)]
        if k == "st":
            return [lambda x: self._pat(x, "T0", f0=True)]
        if k == "del":
            return [lambda x: self._pat(x, "T1", f0=r["val"], f1=True)]
        if k == "obs":
            return [lambda x: self._pat(x, r["typ"])]
        if k == "ks":
            return [lambda x: self._pat(x, "T1", f0=r["val"], f1=True),
                    lambda x: self._pat(x, "T1", f0=r["arm"], f1=False)]
        if k == "killt0":
            return [lambda x: self._pat(x, "T0", f0=True),
                    lambda x: self._pat(x, "T1", f0=r["arm"], f1=False)]
        if k == "midt":
            return [lambda x: self._pat(x, "T1", f0=r["trig"], f1=r["trig_f1"])]
        if k == "rins":
            return [lambda x: self._pat(x, "T1", f0=r["arm"], f1=False)]
        if k == "jl2":
            return [lambda x: self._pat(x, "T0", f0=False)]
        if k == "ru":
            return [lambda x: self._pat(x, "T0", f0=True),
                    lambda x: self._pat(x, "T1", f0=r["arm"], f1=False)]
        raise AssertionError(k)

    # ---------- activation ----------
    def activate_on(self, hid):
        for name in self.rules:
            pats = self.patterns(name)
            alive = [x for x in self.h if self.h[x]["alive"]]
            slots = []
            for p in pats:
                slots.append([x for x in alive if p(x)])
            if not all(slots):
                continue
            if len(pats) == 1:
                combos = [(x,) for x in slots[0] if x == hid]
            else:
                combos = [(a, b) for a in slots[0] for b in slots[1]
                          if hid in (a, b)]
            for tup in combos:
                key = (name, tup)
                if key in self.seen:
                    continue
                self.seen.add(key)
                self._act += 1
                self.acts.append(dict(id=self._act, rule=name, tuple=tup,
                                      sal=self.rules[name]["sal"],
                                      seq=self.seq(), live=True, fired=False))

    # ---------- WM ops ----------
    def insert_stated(self, typ, fields):
        hid = self.new_handle(typ, fields)
        if typ in KEYED:
            vk = self.vk(typ, fields)
            k = self.keys.get(vk)
            if k is None:
                self.keys[vk] = dict(label="STATED", stated=[hid],
                                     belief=None, pending=None, deps=[])
            else:
                k["stated"].append(hid)
        self.activate_on(hid)
        return hid

    def insert_logical(self, typ, fields, by):
        vk = self.vk(typ, fields)
        k = self.keys.get(vk)
        dep = dict(act=by["id"], rule=by["rule"], tup=list(by["tuple"]))
        if k is None:
            hid = self.new_handle(typ, fields)
            self.keys[vk] = dict(label="JUSTIFIED", stated=[], belief=hid,
                                 pending=None, deps=[dep])
            self.activate_on(hid)
        elif k["label"] == "JUSTIFIED":
            k["deps"].append(dep)
        else:
            k["pending"] = dict(fields)
            k["deps"].append(dep)

    def kill(self, hid, by, cancel):
        h = self.h[hid]
        h["alive"] = False
        if cancel:
            for a in self.acts:
                if a["live"] and not a["fired"] and hid in a["tuple"]:
                    a["live"] = False
        for vk in list(self.keys):
            k = self.keys.get(vk)
            if not k:
                continue
            for dep in [d for d in k["deps"] if hid in d["tup"]]:
                self.land_break(vk, dep, by, dep["tup"].count(hid))

    def rhs_delete(self, hid, by):
        h = self.h[hid]
        if not h["alive"]:
            return
        if h["orphan"]:
            return  # x1: undeletable
        typ = h["typ"]
        if typ in KEYED:
            vk = self.vk(typ, h["fields"])
            k = self.keys.get(vk)
            if k and hid in k["stated"]:
                mixed = k["belief"] is not None or k["pending"] is not None
                if mixed:
                    assert k["label"] != "JUSTIFIED", \
                        "UNPINNED: stated-delete on justified-born mixed key"
                    pend = k["pending"]
                    sibs = [s for s in k["stated"] if s != hid]
                    del self.keys[vk]
                    self.kill(hid, by, cancel=True)
                    for s in sibs:
                        if self.h[s]["alive"]:
                            self.h[s]["orphan"] = True
                    nb = self.new_handle(typ, dict(pend), unstage=True)
                    self.activate_on(nb)
                else:
                    self.kill(hid, by, cancel=True)
                    k["stated"].remove(hid)
                    if not k["stated"]:
                        del self.keys[vk]
                return
            if k and hid == k["belief"]:
                assert not k["stated"], \
                    "UNPINNED: belief-delete with stated siblings"
                del self.keys[vk]
                self.kill(hid, by, cancel=True)
                return
        # keyless: unstage-born handles, or T0/T2 premises
        self.kill(hid, by, cancel=not h["unstage"])

    def dep_alpha_holds(self, dep):
        pats = self.patterns(dep["rule"])
        return all(p(x) for p, x in zip(pats, dep["tup"]))

    def rhs_update(self, hid, newfields, by):
        self.h[hid]["fields"].update(newfields)
        for a in self.acts:
            if a["live"] and not a["fired"] and hid in a["tuple"]:
                pats = self.patterns(a["rule"])
                if not all(p(x) for p, x in zip(pats, a["tuple"])):
                    raise AssertionError(
                        "UNPINNED: update invalidates a queued act")
        for vk in list(self.keys):
            k = self.keys.get(vk)
            if not k:
                continue
            for dep in [d for d in k["deps"] if hid in d["tup"]]:
                if not self.dep_alpha_holds(dep):
                    self.land_break(vk, dep, by, dep["tup"].count(hid))

    def land_break(self, vk, dep, by, bindcount):
        if dep["act"] == by["id"] and bindcount >= 2:
            self.pseudo.append(dict(sal=by["sal"], seq=self.seq(),
                                    vk=vk, dep=dep, lazy=True))
        else:
            self.finalize_break(vk, dep, by)

    def finalize_break(self, vk, dep, by):
        k = self.keys.get(vk)
        if not k or dep not in k["deps"]:
            return
        k["deps"].remove(dep)
        if k["deps"]:
            return
        if k["belief"] is not None:
            b = k["belief"]
            for s in k["stated"]:
                if self.h[s]["alive"]:
                    self.h[s]["orphan"] = True
            del self.keys[vk]
            self.kill(b, by, cancel=True)
        elif k["pending"] is not None:
            raise AssertionError(
                "UNPINNED: break empties a pending belief's deps")

    # ---------- RHS dispatch ----------
    def fire(self, act):
        r = self.rules[act["rule"]]
        k = r["kind"]
        t = act["tuple"]
        if k == "jl":
            typ, bf = r["belief"] or ("T1", {"f0": r["val"], "f1": True})
            self.insert_logical(typ, dict(bf), act)
            target = t[1] if r["selfjoin"] else t[0]
            if r["brk"] in ("update", "modify"):
                self.rhs_update(target, dict(r["brkfields"]), act)
            elif r["brk"] == "delete":
                self.rhs_delete(target, act)
        elif k == "st":
            self.insert_stated("T1", {"f0": r["val"], "f1": True})
        elif k == "del":
            self.rhs_delete(t[0], act)
        elif k == "ks":
            self.rhs_delete(t[0], act)
            self.rhs_delete(t[1], act)
        elif k == "killt0":
            self.rhs_delete(t[0], act)
            self.rhs_delete(t[1], act)
        elif k == "midt":
            self.insert_logical("T1", {"f0": r["val"], "f1": False}, act)
        elif k == "rins":
            self.insert_stated("T0", {"f0": False})
            self.rhs_delete(t[0], act)
        elif k == "jl2":
            self.insert_logical("T1", {"f0": r["val"], "f1": True}, act)
        elif k == "ru":
            self.rhs_update(t[0], {"f0": False}, act)
            self.rhs_delete(t[1], act)
        elif k == "obs":
            pass
        else:
            raise AssertionError(k)
        self.fired.append((act["rule"],
                           tuple((self.h[x]["typ"],
                                  tuple(sorted(self.h[x]["fields"].items())))
                                 for x in t)))

    def run(self):
        while True:
            live = [a for a in self.acts if a["live"] and not a["fired"]]
            cand = live + self.pseudo
            if not cand:
                break
            item = max(cand, key=lambda a: (a["sal"], -a["seq"]))
            if item.get("lazy"):
                self.pseudo.remove(item)
                self.finalize_break(item["vk"], item["dep"],
                                    dict(id=-1, sal=item["sal"]))
                continue
            item["fired"] = True
            self.fire(item)
        finals = sorted((self.h[x]["typ"],
                         tuple(sorted(self.h[x]["fields"].items())))
                        for x in self.h if self.h[x]["alive"])
        return self.fired, finals


def truth_of(entry):
    firings = [(f["rule"],
                tuple((m["type"], tuple(sorted(m["fields"].items())))
                      for m in f["matches"]))
               for f in entry["result"]["firings"]]
    finals = sorted((x["type"], tuple(sorted(x["fields"].items())))
                    for x in entry["result"]["facts"])
    return firings, finals


def main():
    truths = {}
    for fn in TRUTH_FILES:
        for line in open(os.path.join(DIR, "truths", fn)):
            d = json.loads(line)
            truths[d["scenario"]] = truth_of(d)
    ok = 0
    for name, (facts, rules) in CELLS.items():
        want = truths[name]
        try:
            got = Sim(facts, rules).run()
        except AssertionError as e:
            print(f"RAISE {name}: {e}")
            continue
        if got == want:
            print(f"OK  {name}")
            ok += 1
        else:
            print(f"FAIL {name}")
            print(f"   want firings: {want[0]}")
            print(f"   got  firings: {got[0]}")
            print(f"   want finals:  {want[1]}")
            print(f"   got  finals:  {got[1]}")
    print(f"--- {ok}/{len(CELLS)}")
    return 0 if ok == len(CELLS) else 1


if __name__ == "__main__":
    raise SystemExit(main())
