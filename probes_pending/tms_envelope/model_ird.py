#!/usr/bin/env python3
"""model_ird: executable spec of the I-RD mechanisms (D-203..205, D-208).

THE LAWS AS CODE (predictions/commitments: ird-model-predictions.md,
ird-population-predictions.md, ird-ab-predictions.md):
 1. DYNAMIC (survive-the-delete, D-203): killing an UNSTAGE-BORN handle
    never cancels queued acts — they fire later with the dead handle's
    values. Every other kill cancels acts whose tuple contains the dead
    handle.
 2. STATIC key lifecycle (D-203/D-205/D-208): T1/T3 keyed, T0/T2
    keyless premises. ⚖ ACTIVATION-BACKFILL (D-208, d1/d2 dumps): TMS
    EqualityKeys form at TMS ACTIVATION = the session's first
    insertLogical. Stated facts already in WM get PER-HANDLE keys with
    only the LAST one VALUE-MAPPED; post-activation equal-valued
    stated inserts JOIN the mapped key; logical inserts join/create
    the mapped key (JUSTIFIED-born: WM-visible belief; onto a
    STATED-born key: NON-WM pending belief; onto JUSTIFIED: dep fold).
    Key-death events, PER-KEY, both orphaning that key's remaining
    stated siblings: (i) the last dep breaks (L6); (ii) a stated
    delete on a stated-born MIXED key — victim dies, siblings orphan,
    the pending belief UNSTAGES WM-visible unstage-born (r1; b1 =
    0-sibling; d1/d2 = the two-key split, NOT a position law).
    Orphans: alive, keyless, UNDELETABLE (x1). A dead key's map entry
    clears — later inserts of the value re-key FRESH (L6 rebirth).
 3. SAME-BATCH SELF-BREAK (D-205, slot+shape re-pinned D-208): RHS ops
    apply in order; a break lands IMMEDIATELY (same-batch-self and
    foreign alike) EXCEPT when the breaking act IS the justifying act
    AND the justifier's LHS is a SELF-JOIN (≥2 patterns on the broken
    fact's type — RULE-SHAPE-keyed, s2, not tuple-bindcount): then a
    LAZY-BREAK lands at the justifier's salience BEFORE any other
    same-salience pop (s1 — the pseudo beats earlier-queued acts).
    Form (modify/update) and source (update/delete) never branch.
 4. IMPORTED commitments (engine-corpus doctrine, cell-checked here):
    an alpha-breaking update CANCELS queued unfired acts (D-076
    family); an alpha-KEEPING update leaves surviving acts IN PLACE —
    no re-queue (s3).
 Tie-break: equal salience pops FIFO by act creation; lazy-breaks beat
 same-salience acts. Breaks cascade through kills recursively (a2).
 UNPINNED corners raise AssertionError (three: stated-delete on a
 justified-born mixed key; belief-delete with stated siblings; a break
 emptying a PENDING belief's deps).

Validation: `python3 model_ird.py` -> 27/27 vs truths/ird_*.ndj.
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


def T0OBS(sal):
    return dict(kind="t0obs", sal=sal)


def KS(sal, val, arm):
    return dict(kind="ks", sal=sal, val=val, arm=arm)


def KILLT0(sal, arm):
    return dict(kind="killt0", sal=sal, arm=arm)


def MIDT(sal, trig, trig_f1, val, bf1=False):
    return dict(kind="midt", sal=sal, trig=trig, trig_f1=trig_f1, val=val,
                bf1=bf1)


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
    "ird_d1_2stated_belief_last": ([T0F, V()],
        {"RS2": ST(20), "RJ": JL(18), "RD": DEL(5, "v"), "ROBS": OBS(0)}),
    "ird_d2_2stated_belief_mid": ([T0F, V()],
        {"RS2": ST(20), "RJ": JL(18), "RS3": ST(16), "RD": DEL(5, "v"),
         "ROBS": OBS(0)}),
    "ird_s1_slot_straddle": ([T0F],
        {"RJ": JL(10, brk="modify", selfjoin=True), "RD": DEL(10, "v")}),
    "ird_s2_slot_twin": ([T0F, T0F],
        {"RJ": JL(10, brk="modify", selfjoin=True), "ROBS": OBS(15)}),
    "ird_s3_update_requeue": ([T0F, T0F, V("arm", False)],
        {"RB": RUK(20, "arm"), "RO": T0OBS(10)}),
}

TRUTH_FILES = ["ird_oracle_r1.ndj", "ird_ladder_oracle_r1.ndj",
               "ird_l56_oracle_r1.ndj", "ird_x1_oracle_r1.ndj",
               "ird_rm_oracle_r1.ndj", "ird_m67_oracle_r1.ndj",
               "ird_ab_oracle_r1.ndj"]


class Sim:
    def __init__(self, facts, rules):
        self.rules = rules
        self.h = {}
        self.keys = {}       # kid -> key dict
        self.value_map = {}  # vk -> kid (the MAPPED key per value)
        self.tms_active = False
        self.acts = []
        self.pseudo = []
        self.seen = set()
        self.fired = []
        self._seq = 0
        self._hid = 0
        self._act = 0
        self._kid = 0
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
                                 orphan=False, unstage=unstage, key=None)
        return self._hid

    def new_key(self, vk, mapped, **kw):
        self._kid += 1
        k = dict(vk=vk, label=kw.get("label", "STATED"),
                 stated=kw.get("stated", []), belief=kw.get("belief"),
                 pending=kw.get("pending"), deps=kw.get("deps", []))
        self.keys[self._kid] = k
        for s in k["stated"]:
            self.h[s]["key"] = self._kid
        if k["belief"] is not None:
            self.h[k["belief"]]["key"] = self._kid
        if mapped:
            self.value_map[vk] = self._kid
        return self._kid

    def drop_key(self, kid):
        k = self.keys.pop(kid)
        if self.value_map.get(k["vk"]) == kid:
            del self.value_map[k["vk"]]

    # ---------- alphas ----------
    def _pat(self, hid, typ, **eq):
        h = self.h[hid]
        return (h["alive"] and h["typ"] == typ
                and all(h["fields"].get(k) == v for k, v in eq.items()))

    def patterns(self, name):
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
        if k == "t0obs":
            return [lambda x: self._pat(x, "T0")]
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
            slots = [[x for x in alive if p(x)] for p in pats]
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
        if typ in KEYED and self.tms_active:
            vk = self.vk(typ, fields)
            kid = self.value_map.get(vk)
            if kid is None:
                self.new_key(vk, mapped=True, stated=[hid])
            else:
                self.keys[kid]["stated"].append(hid)
                self.h[hid]["key"] = kid
        self.activate_on(hid)
        return hid

    def tms_activate(self):
        """⚖ ACTIVATION-BACKFILL (D-208, d1/d2 dumps): pre-activation
        stated handles get PER-HANDLE keys; the LAST per value wins the
        value map."""
        self.tms_active = True
        for hid in sorted(self.h):
            h = self.h[hid]
            if h["alive"] and h["typ"] in KEYED and h["key"] is None:
                self.new_key(self.vk(h["typ"], h["fields"]), mapped=True,
                             stated=[hid])

    def insert_logical(self, typ, fields, by):
        if not self.tms_active:
            self.tms_activate()
        vk = self.vk(typ, fields)
        dep = dict(act=by["id"], rule=by["rule"], tup=list(by["tuple"]))
        kid = self.value_map.get(vk)
        if kid is None:
            hid = self.new_handle(typ, fields)
            self.new_key(vk, mapped=True, label="JUSTIFIED", belief=hid,
                         deps=[dep])
            self.activate_on(hid)
        elif self.keys[kid]["label"] == "JUSTIFIED":
            self.keys[kid]["deps"].append(dep)
        else:
            self.keys[kid]["pending"] = dict(fields)
            self.keys[kid]["deps"].append(dep)

    def kill(self, hid, by, cancel):
        h = self.h[hid]
        h["alive"] = False
        if cancel:
            for a in self.acts:
                if a["live"] and not a["fired"] and hid in a["tuple"]:
                    a["live"] = False
        for kid in list(self.keys):
            k = self.keys.get(kid)
            if not k:
                continue
            for dep in [d for d in k["deps"] if hid in d["tup"]]:
                self.land_break(kid, dep, by)

    def rhs_delete(self, hid, by):
        h = self.h[hid]
        if not h["alive"]:
            return
        if h["orphan"]:
            return  # x1: undeletable
        kid = h["key"]
        if kid is not None:
            k = self.keys[kid]
            if hid in k["stated"]:
                mixed = k["belief"] is not None or k["pending"] is not None
                if mixed:
                    assert k["label"] != "JUSTIFIED", \
                        "UNPINNED: stated-delete on justified-born mixed key"
                    pend = k["pending"]
                    sibs = [s for s in k["stated"] if s != hid]
                    self.drop_key(kid)
                    self.kill(hid, by, cancel=True)
                    for s in sibs:
                        if self.h[s]["alive"]:
                            self.h[s]["orphan"] = True
                            self.h[s]["key"] = None
                    nb = self.new_handle(h["typ"], dict(pend), unstage=True)
                    self.activate_on(nb)
                else:
                    self.kill(hid, by, cancel=True)
                    k["stated"].remove(hid)
                    if not k["stated"]:
                        self.drop_key(kid)
                return
            if hid == k["belief"]:
                assert not k["stated"], \
                    "UNPINNED: belief-delete with stated siblings"
                self.drop_key(kid)
                self.kill(hid, by, cancel=True)
                return
        # keyless: unstage-born, pre-activation stated, or T0/T2 premises
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
                    # D-207 IMPORTED (D-076 family): alpha-breaking
                    # update cancels queued acts. Alpha-KEEPING updates
                    # leave survivors IN PLACE (s3, D-208 — no requeue).
                    a["live"] = False
        for kid in list(self.keys):
            k = self.keys.get(kid)
            if not k:
                continue
            for dep in [d for d in k["deps"] if hid in d["tup"]]:
                if not self.dep_alpha_holds(dep):
                    self.land_break(kid, dep, by)

    def land_break(self, kid, dep, by):
        # ⚖ D-208 (s2): the lazy exception is RULE-SHAPE-keyed —
        # the justifier's LHS is a self-join — not tuple-bindcount.
        if dep["act"] == by["id"] and self.rules[dep["rule"]].get("selfjoin"):
            self.pseudo.append(dict(sal=by["sal"], seq=self.seq(),
                                    kid=kid, dep=dep, lazy=True))
        else:
            self.finalize_break(kid, dep, by)

    def finalize_break(self, kid, dep, by):
        k = self.keys.get(kid)
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
                    self.h[s]["key"] = None
            self.drop_key(kid)
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
        elif k in ("ks", "killt0"):
            self.rhs_delete(t[0], act)
            self.rhs_delete(t[1], act)
        elif k == "midt":
            self.insert_logical("T1", {"f0": r["val"], "f1": r["bf1"]}, act)
        elif k == "rins":
            self.insert_stated("T0", {"f0": False})
            self.rhs_delete(t[0], act)
        elif k == "jl2":
            self.insert_logical("T1", {"f0": r["val"], "f1": True}, act)
        elif k == "ru":
            self.rhs_update(t[0], {"f0": False}, act)
            self.rhs_delete(t[1], act)
        elif k in ("obs", "t0obs"):
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
            # ⚖ D-208 (s1): a lazy-break beats same-salience acts
            # regardless of queue seq; FIFO otherwise.
            item = max(cand, key=lambda a: (a["sal"],
                                            1 if a.get("lazy") else 0,
                                            -a["seq"]))
            if item.get("lazy"):
                self.pseudo.remove(item)
                self.finalize_break(item["kid"], item["dep"],
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
