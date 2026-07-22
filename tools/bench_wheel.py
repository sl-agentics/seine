"""Doubling ladder through the PUBLIC Python API — run under any
installed seine_rs wheel (`python tools/bench_wheel.py`). Mirrors
tools/bench_oracle.py --scale's workloads; a consecutive ratio ~2 =
linear, ~4 = quadratic. This ladder run against the PUBLISHED 0.4.56
wheel is what caught D-390 (the bindings' cascade capture rendered the
full store twice per update/delete — 682s for 32k updates where the
engine lane took 340ms); the README's performance table comes from it.
"""
import time

import seine_rs as s

SIZES = [2000, 4000, 8000, 16000, 32000]
V = s.certification()["engine_version"]


@s.fact
class T0:
    k: int
    v: int


@s.fact
class T1:
    k: int
    w: int


def alpha(n):
    r = s.Rule("R0")
    r.when(T0, T0.v >= 0)
    sess = s.Session([r])
    sess.insert("T0", {"k": list(range(n)), "v": list(range(n))})
    t0 = time.monotonic()
    sess.fire()
    return time.monotonic() - t0


def join(n):
    r = s.Rule("R0")
    a = r.when(T0)
    r.when(T1, T1.k == a.k)
    sess = s.Session([r])
    sess.insert("T0", {"k": list(range(n)), "v": list(range(n))})
    sess.insert("T1", {"k": list(range(n)), "w": list(range(n))})
    t0 = time.monotonic()
    sess.fire()
    return time.monotonic() - t0


def acc(n):
    drl = ('rule "R0"\nwhen\n    T1($k : k)\n'
           '    accumulate( T0(v >= 0, $s : v); $a : sum($s) )\nthen\nend\n')
    sess = s.Session(drl, schemas={"T0": {"k": "long", "v": "long"},
                                   "T1": {"k": "long", "w": "long"}})
    sess.insert("T0", {"k": list(range(n)), "v": list(range(n))})
    sess.insert_row("T1", {"k": 0, "w": 0})
    t0 = time.monotonic()
    sess.fire()
    return time.monotonic() - t0


def tms(n):
    drl = (f'rule "Grow"\nwhen\n    T($n : n, n < {n})\nthen\n'
           '    insertLogical(new T($n + 1));\nend\n')
    sess = s.Session(drl, schemas={"T": {"n": "long"}})
    h = sess.insert_row("T", {"n": 1})
    t0 = time.monotonic()
    sess.fire()
    grow = time.monotonic() - t0
    t0 = time.monotonic()
    sess.delete(h)
    sess.fire()
    tear = time.monotonic() - t0
    return grow + tear


def churn(n):
    r = s.Rule("R0")
    a = r.when(T0)
    r.when(T1, T1.k == a.k)
    sess = s.Session([r])
    hs = sess.insert("T0", {"k": list(range(n)), "v": list(range(n))})
    sess.insert("T1", {"k": list(range(n)), "w": list(range(n))})
    sess.fire()
    t0 = time.monotonic()
    for i, h in enumerate(hs):
        sess.update(h, k=i, v=i + 1)
    sess.fire()
    return time.monotonic() - t0


print(f"=== wheel {V} (ms; ratios ~2 linear, ~4 quadratic) ===")
for name, fn in [("alpha", alpha), ("join", join), ("acc", acc),
                 ("tms", tms), ("churn", churn)]:
    ms = []
    for n in SIZES:
        ms.append(fn(n) * 1e3)
    rs = "  ".join(f"{ms[i] / ms[i - 1]:4.2f}" for i in range(1, len(ms)))
    print(f"{name:6s} " + "".join(f"{v:9.1f}" for v in ms) + f"   {rs}")
