#!/usr/bin/env python3
"""Validate model_sd against the 27 banked cell ORACLE truths (D-189).

Truth = the 3x-stable oracle outputs captured at rungs 1-3 (r1 files;
stability was verified at capture time). Comparison: rule-name sequence
+ the P-value of each firing (f0 of the first P-typed match, else None)
+ final facts as sorted (type, f0) with LK/LK2 folded to "LK" + the
runaway flag (b3 = oracle fire-limit).

Usage: python3 validate_cells.py [truth_dir]   (default: the banked
probes_pending/tms_envelope/truths/ — rung*_oracle_r1.ndj)
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from model_sd import simulate


def J(name="RJ", sal=0, k=1, notpos="trail", eager=False, ortwin=False,
      breaks=True, amut=None, mutfirst=False):
    return {"kind": "justifier", "name": name, "sal": sal, "k": k,
            "notpos": notpos, "eager": eager, "ortwin": ortwin,
            "breaks": breaks, "amut": amut, "mutfirst": mutfirst}


def OL(sal, name="RO"): return {"kind": "obs_lk", "name": name, "sal": sal}
def OJ(sal, name="RO"): return {"kind": "obs_join", "name": name, "sal": sal}
def DN(sal, name="RD"): return {"kind": "del_not", "name": name, "sal": sal}
def DJ(sal, name="RD"): return {"kind": "del_join", "name": name, "sal": sal}


CELLS = {
    "sd_a2_plain_eq":        ([],        [J(k=0), OL(0)]),
    "sd_a3_plain_hi":        ([],        [J(k=0), OL(10)]),
    "sd_a4_plain_lo":        ([],        [J(k=0), OL(-10)]),
    "sd_a5_acc_eq":          ([],        [J(k=0), OL(0)]),
    "sd_a6_acc_hi":          ([],        [J(k=0), OL(10)]),
    "sd_a7_acc_lo":          ([],        [J(k=0), OL(-10)]),
    "sd_a8_dual_eq":         ([],        [J(k=0), OL(0), OL(0, "RO2")]),
    "sd_a9_declorder_plain": ([],        [OL(0), J(k=0)]),
    "sd_a10_declorder_acc":  ([],        [OL(0), J(k=0)]),
    "sd_a11_declfirst_lo":   ([],        [OL(-10), J(k=0)]),
    "sd_a12_declfirst_hi":   ([],        [OL(10), J(k=0)]),
    "sd_a13_dual_declsplit": ([],        [OL(0), J(k=0), OL(0, "RO2")]),
    "sd_a14_noloop_declfirst": ([],      [OL(0), J(k=0, eager=True)]),
    "sd_a15_k1_declfirst":   ([1],       [OL(0), J()]),
    "sd_b1_fanout_trailing": ([1, 2],    [J()]),
    "sd_b2_fanout_leading":  ([1, 2],    [J(notpos="lead")]),
    "sd_b3_ortwin_lazy":     ([],        [J(k=0, ortwin=True)]),
    "sd_b4_ortwin_noloop":   ([],        [J(k=0, ortwin=True, eager=True)]),
    "sd_b5_fanout3_obs":     ([1, 2, 3], [OL(0), J()]),
    "sd_b6_leading_obs":     ([1, 2],    [OL(0), J(notpos="lead")]),
    "sd_b7_join_obs_fanout3": ([1, 2, 3], [OJ(0), J()]),
    "sd_c1_alternation":     ([1, 2, 3], [J(sal=7), DN(0)]),
    "sd_c2_no_deleter":      ([1, 2, 3], [J(sal=7)]),
    "sd_c3a_eq_jfirst":      ([1, 2, 3], [J(), DN(0)]),
    "sd_c3b_eq_dfirst":      ([1, 2, 3], [DN(0), J()]),
    "sd_c3c_gap1":           ([1, 2, 3], [J(), DN(-1)]),
    "sd_c3d_join_deleter":   ([1, 2, 3], [DJ(0), J()]),
    "sd_d1_nl_lead_del":     ([1, 2],    [J(notpos="lead", eager=True), DN(-5)]),
    "sd_d2_nl_trail_del":    ([1, 2],    [J(eager=True), DN(-5)]),
    "sd_d3_nl_lead_nodel":   ([1, 2],    [J(notpos="lead", eager=True)]),
    "sd_d4_lazy_lead_del":   ([1, 2],    [J(notpos="lead"), DN(-5)]),
    "sd_d5_nl_lead_obs":     ([1, 2],    [J(notpos="lead", eager=True), OL(5)]),
    "mb1_dt":                ([1, 2],    [J(amut="del")]),
    "mb1_dl":                ([1, 2],    [J(notpos="lead", amut="del")]),
    "mb1_st":                ([1, 2],    [J(amut="set_break")]),
    "mb1_sl":                ([1, 2],    [J(notpos="lead", amut="set_break")]),
    "mb1_st_nb":             ([1, 2],    [J(amut="set_break", breaks=False)]),
    "mb1_st_mf":             ([1, 2],    [J(amut="set_break", mutfirst=True)]),
    "mb1_dt_nb":             ([1, 2],    [J(amut="del", breaks=False)]),
}


def truth_of(entry):
    if "error" in entry:
        err = str(entry["error"])
        return {"runaway": "fire limit" in err, "firings": None, "finals": None}
    seq = []
    for f in entry["result"]["firings"]:
        pv = None
        for m in f.get("matches", []):
            if m["type"] == "P":
                pv = m["fields"].get("f0")
                break
        seq.append((f["rule"], pv))
    finals = sorted(("LK" if fa["type"] in ("LK", "LK2") else fa["type"],
                     fa["fields"].get("f0")) for fa in entry["result"]["facts"])
    return {"runaway": False, "firings": seq, "finals": finals}


def main():
    tdir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "truths")
    truths = {}
    for fn in os.listdir(tdir):
        if fn.startswith("rung") and (fn.endswith("_oracle_r1.ndjson")
                                      or fn.endswith("_oracle_r1.ndj")):
            for line in open(os.path.join(tdir, fn)):
                j = json.loads(line)
                truths[j["scenario"]] = truth_of(j)
    n_ok = 0
    for name, (facts, rules) in sorted(CELLS.items()):
        got = simulate(facts, rules)
        t = truths.get(name)
        if t is None:
            print(f"MISSING TRUTH {name}"); continue
        if t["runaway"]:
            ok = got["runaway"]
            detail = f"runaway: model={got['runaway']}"
        else:
            ok = (not got["runaway"] and got["firings"] == t["firings"]
                  and got["finals"] == t["finals"])
            detail = ""
            if not ok:
                detail = (f"\n    model  {got['firings']}  finals={got['finals']}"
                          f"\n    oracle {t['firings']}  finals={t['finals']}")
        print(f"{'OK ' if ok else 'DIV'} {name}{detail}")
        n_ok += ok
    print(f"--- {n_ok}/{len(CELLS)}")
    sys.exit(0 if n_ok == len(CELLS) else 1)


if __name__ == "__main__":
    main()
