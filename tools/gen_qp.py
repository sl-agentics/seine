#!/usr/bin/env python3
"""qperf ladder generator (D-367, probes_pending/qperf/HANDOFF.md Part 3).

Five ladders, N in {100, 1000, 5000}, each engineered so the LEGIT
workload stays ~linear (selective anchor patterns keep join outputs
O(N)) and only the audited site can contribute a super-linear term:

  qp_wave_N  — d357_wave_reorder + shares_before (fresh chassis:
               R1 driver x N -> 2N RHS T1s parked at the subnet-not
               behind a blocker; a low-salience delete releases the
               whole set in one wave).
  qp_termq_N — d359_termq_reorder + shares_before (fresh chassis:
               the lazy first-Term R0 shares the join with the
               Ria-guarded R4; the blocker never dies, so R0's queue
               is complete at first selection).
  qp_qce_N   — the D-361 qce same-eval-delete fold (pr_qc_m1704b
               verbatim rules; RelR scaled to N disjoint edges, the
               MarkR recursion seed dropped so TCr rows stay O(N)).
  qp_enum_N  — d363_reorder's window rank (pr_qe_p363a verbatim
               rules; T2 drivers scaled to N -> N one-fact drain
               windows at Q0's b0 site, then one top-level full-walk
               call ranks N rows against N windows).
  qp_repos_N — the D-360 kept-kind reposition arm (pr_pc_m1020b
               verbatim rules; setup T0 scaled to M = isqrt(N) so the
               staged tuple set is M^2 ~ N).

Regenerate with:  python3 tools/gen_qp.py
"""
import json
import math
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "probes_pending", "qperf")
PROBES = os.path.join(ROOT, "scenarios", "probes")
NS = [100, 1000, 5000]

T01 = [
    {"name": "T0", "fields": [{"name": "f1", "type": "i64"}]},
    {
        "name": "T1",
        "fields": [
            {"name": "f0", "type": "i64"},
            {"name": "f1", "type": "String"},
        ],
    },
]

DRIVER = (
    'rule "R1"\n'
    "when\n"
    "    $p1 : T0($b1 : f1)\n"
    "then\n"
    '    insert(new T1($b1, "beta"));\n'
    '    insert(new T1($b1, "a"));\n'
    "end\n"
)

R4 = (
    'rule "R4"\n'
    "salience($a4 - $b4)\n"
    "when\n"
    "    T1($a4 : f0)\n"
    "    T1(f0 == 777777, $b4 : f0)\n"
    "    exists(T1(f0 == 888888) and not(T1(f0 < -3)))\n"
    "then\n"
    "end\n"
)


def anchors():
    return [
        {"type": "T1", "fields": {"f0": 777777, "f1": "s"}},
        {"type": "T1", "fields": {"f0": 888888, "f1": "s"}},
        {"type": "T1", "fields": {"f0": -5, "f1": "s"}},
    ]


def wave(n):
    drl = (
        DRIVER
        + R4
        + 'rule "RD"\n'
        "salience -20\n"
        "when\n"
        "    $pd : T1(f0 < -3)\n"
        "then\n"
        "    delete($pd);\n"
        "end\n"
    )
    facts = [{"type": "T0", "fields": {"f1": i}} for i in range(n)] + anchors()
    return {"name": f"qp_wave_{n}", "drl": drl, "facts": facts, "types": T01}


def termq(n):
    drl = (
        'rule "R0"\n'
        "salience -10\n"
        "when\n"
        "    T1($a0 : f0)\n"
        "    T1(f0 == 777777, $b0 : f0)\n"
        "then\n"
        "end\n" + DRIVER + R4
    )
    facts = [{"type": "T0", "fields": {"f1": i}} for i in range(n)] + anchors()
    return {"name": f"qp_termq_{n}", "drl": drl, "facts": facts, "types": T01}


def qce(n):
    base = json.load(open(os.path.join(PROBES, "pr_qc_m1704b.json")))
    facts = [f for f in base["facts"] if f["type"] not in ("RelR", "MarkR")]
    facts += [
        {"type": "RelR", "fields": {"src": 1000000 + i, "dst": 2000000 + i}}
        for i in range(n)
    ]
    return {
        "name": f"qp_qce_{n}",
        "drl": base["drl"],
        "facts": facts,
        "types": base["types"],
    }


def enum_(n):
    base = json.load(open(os.path.join(PROBES, "pr_qe_p363a.json")))
    facts = [f for f in base["facts"] if f["type"] != "T2"]
    facts += [{"type": "T2", "fields": {"f0": "s%06d" % i}} for i in range(n)]
    return {
        "name": f"qp_enum_{n}",
        "drl": base["drl"],
        "facts": facts,
        "types": base["types"],
        "queries": base["queries"],
    }


def repos(n):
    base = json.load(open(os.path.join(PROBES, "pr_pc_m1020b.json")))
    m = math.isqrt(n)
    facts = [
        {"type": "T0", "fields": {"f0": False, "f1": "", "f2": 2.0, "f3": True}}
        for _ in range(m)
    ]
    return {
        "name": f"qp_repos_{n}",
        "drl": base["drl"],
        "facts": facts,
        "epochs": base["epochs"],
        "types": base["types"],
    }


def main():
    os.makedirs(OUT, exist_ok=True)
    for n in NS:
        for gen in (wave, termq, qce, enum_, repos):
            cell = gen(n)
            path = os.path.join(OUT, cell["name"] + ".json")
            with open(path, "w") as fh:
                json.dump(cell, fh, indent=1)
                fh.write("\n")
            print("wrote", path)


if __name__ == "__main__":
    main()
