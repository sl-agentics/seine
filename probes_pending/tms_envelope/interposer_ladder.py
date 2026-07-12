#!/usr/bin/env python3
"""R1 interposer ladder (the D-177 instrument on the D-186 plan §5.1).

A salience INTERPOSER strictly between justifier and observer converts
belief-drop landing time into firing order. The cells probe the
landing-law belief-loss rows the D-187..D-195 laws predict; every
prediction is model_sd's own output (logged pre-run) — the ladder
tests the laws' TRANSFER to the min812 (k0) and 9133 (k1 fan-out)
spines plus the D-195 update-break row.

Usage:
  python3 interposer_ladder.py --predict          # print model outputs
  python3 interposer_ladder.py --run [workdir]    # oracle 3x + verdicts
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(REPO, "tools"))
from model_sd import simulate                      # noqa: E402
from fuzz_tms_sd import drl_of, truth_of, P_T, LK_T  # noqa: E402

HARNESS = ["cargo", "run", "-q", "-p", "seine-harness", "--"]


def J(name="RJ", sal=0, k=1, notpos="trail", eager=False, ortwin=False,
      breaks=True, amut=None, mutfirst=False):
    return {"kind": "justifier", "name": name, "sal": sal, "k": k,
            "notpos": notpos, "eager": eager, "ortwin": ortwin,
            "breaks": breaks, "amut": amut, "mutfirst": mutfirst}


def OL(sal, name="RO"): return {"kind": "obs_lk", "name": name, "sal": sal}
def OJ(sal, name="RO"): return {"kind": "obs_join", "name": name, "sal": sal}
def OP(sal, name="RO"): return {"kind": "obs_p", "name": name, "sal": sal}


# The 6 cells. a-spine = min812 class (k0 self-defeat); b-spine =
# 9133 class (k1 fan-out); c = the D-195 update-break row. RI is the
# interposer. Equal-salience decl cells exist already (sd_a2/sd_a9).
CELLS = {
    "ip_a1_lazy_k0_between": ([],        [J(k=0), OL(10, "RO"), OL(5, "RI")]),
    "ip_a2_lazy_k0_below":   ([],        [J(k=0), OL(10, "RO"), OL(-5, "RI")]),
    "ip_a3_eager_k0_between": ([],       [J(k=0, eager=True), OL(10, "RO"), OL(5, "RI")]),
    "ip_b1_lazy_k1_between": ([1, 2, 3], [J(), OL(10, "RO"), OL(5, "RI")]),
    "ip_b2_lazy_k1_below":   ([1, 2, 3], [J(), OL(10, "RO"), OL(-5, "RI")]),
    "ip_c1_updbreak_between": ([1, 2],   [J(sal=5, eager=True, notpos="lead",
                                            amut="set_break", mutfirst=True,
                                            breaks=False),
                                          OP(7, "RO1"), OJ(7, "RO2"),
                                          OL(6, "RI")]),
}


def scenario_of(name, facts, rules):
    return {"name": name, "drl": drl_of(rules),
            "facts": [{"type": "P", "fields": {"f0": v, "f1": 0}} for v in facts],
            "types": [P_T, LK_T]}


def predict():
    for name, (facts, rules) in CELLS.items():
        got = simulate(facts, rules)
        print(f"{name}:")
        print(f"   firings {got['firings']}")
        print(f"   finals  {got['finals']}  runaway={got['runaway']}")


def run(workdir):
    files = []
    for name, (facts, rules) in CELLS.items():
        p = os.path.join(workdir, f"{name}.json")
        json.dump(scenario_of(name, facts, rules), open(p, "w"))
        files.append(p)
    launches = []
    for i in range(3):
        out = os.path.join(workdir, f"oracle_r{i}.ndjson")
        r = subprocess.run(HARNESS + ["oracle"] + files, stdout=open(out, "w"),
                           stderr=subprocess.DEVNULL, cwd=REPO)
        if r.returncode != 0:
            sys.exit("oracle batch failed")
        launches.append({json.loads(l)["scenario"]: truth_of(json.loads(l))
                         for l in open(out)})
    n_ok = 0
    for name, (facts, rules) in CELLS.items():
        truths = [l[name] for l in launches]
        if not all(t == truths[0] for t in truths):
            print(f"FLAKY {name}: {truths}")
            continue
        want = truths[0]
        got = simulate(facts, rules)
        got_cmp = {"runaway": got["runaway"],
                   "firings": [tuple(x) for x in got["firings"]] if not got["runaway"] else None,
                   "finals": got["finals"] if not got["runaway"] else None}
        ok = got_cmp == want
        n_ok += ok
        print(f"{'OK ' if ok else 'RED'} {name}")
        if not ok:
            print(f"     model  {got_cmp}")
            print(f"     oracle {want}")
    print(f"--- {n_ok}/{len(CELLS)}")
    return n_ok == len(CELLS)


if __name__ == "__main__":
    if "--predict" in sys.argv:
        predict()
    elif "--run" in sys.argv:
        args = [a for a in sys.argv[2:] if not a.startswith("--")]
        wd = args[0] if args else tempfile.mkdtemp(prefix="ip_ladder_")
        os.makedirs(wd, exist_ok=True)
        print(f"workdir: {wd}")
        sys.exit(0 if run(wd) else 1)
    else:
        print(__doc__)
