"""Comparative wall-clock benchmarks: the seine engine vs the Drools oracle.

Both runners batch scenario files in ONE process and, under SEINE_TIME=1,
emit per-scenario "TIME <name> <ms>" lines on stderr (parse + build/compile
+ run + serialize — like for like, end to end). This script feeds the same
file list to both sides, passes it R times per process so the oracle's JIT
warmup is visible (pass 1 = cold, last pass = warm), and reports:

  - per-side totals, process-startup overhead (wall - sum of scenario times)
  - cold vs warm distribution stats (median / p90 / max)
  - per-scenario engine-vs-oracle(warm) ratios, worst offenders

Workloads:
  --corpus N      deterministic sample (seed 42) of N certified scenarios
                  from scenarios/probes + scenarios/regressions
  --scale         parametric synthetic suite (alpha / join / accumulate at
                  growing fact counts) written to a scratch dir
  <paths...>      any explicit scenario files

Timings are wall-clock on whatever machine this runs on — comparative, not
absolute. Nothing here touches the certified gates: the TIME lines are
env-gated and stderr-only on both runners.

Usage:
  python3 tools/bench_oracle.py --corpus 300
  python3 tools/bench_oracle.py --scale
  python3 tools/bench_oracle.py --repeats 3 scenarios/probes/pr_bd_*.json
"""
import argparse
import glob
import json
import os
import random
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENGINE = os.path.join(ROOT, "target/release/seine-harness")
CP_FILE = os.path.join(ROOT, "oracle/target/classpath.txt")


def run_side(cmd, files, repeats):
    """One process, files x repeats; returns (wall_s, [pass][name]=ms)."""
    argv = cmd + files * repeats
    env = dict(os.environ, SEINE_TIME="1")
    t0 = time.monotonic()
    proc = subprocess.run(argv, env=env, cwd=ROOT,
                          capture_output=True, text=True)
    wall = time.monotonic() - t0
    if proc.returncode != 0:
        sys.exit(f"command failed ({argv[0]}): {proc.stderr[-500:]}")
    times = [t for t in (l.split() for l in proc.stderr.splitlines())
             if len(t) == 3 and t[0] == "TIME"]
    per_pass, n = [], len(files)
    for r in range(repeats):
        chunk = times[r * n:(r + 1) * n]
        per_pass.append({name: float(ms) for _, name, ms in chunk})
    return wall, per_pass


def stats(ms):
    q = statistics.quantiles(ms, n=10) if len(ms) >= 10 else [max(ms)] * 9
    return (f"total {sum(ms):9.1f}ms  median {statistics.median(ms):8.2f}ms  "
            f"p90 {q[8]:8.2f}ms  max {max(ms):8.2f}ms")


def bench(files, repeats, label):
    print(f"\n=== {label}: {len(files)} scenarios x {repeats} passes ===")
    ew, epass = run_side([ENGINE, "run"], files, repeats)
    cp = open(CP_FILE).read().strip()
    ow, opass = run_side(
        ["java", "-cp", f"{ROOT}/oracle/target/classes:{cp}",
         "dev.seine.oracle.OracleRunner"], files, repeats)

    e_warm, o_cold, o_warm = epass[-1], opass[0], opass[-1]
    e_ms, oc_ms, ow_ms = [list(d.values()) for d in (e_warm, o_cold, o_warm)]
    print(f"engine        {stats(e_ms)}")
    print(f"oracle cold   {stats(oc_ms)}")
    if repeats > 1:
        print(f"oracle warm   {stats(ow_ms)}")
    print(f"process wall  engine {ew:6.2f}s (startup ~"
          f"{ew - sum(sum(p.values()) for p in epass) / 1e3:5.2f}s)   "
          f"oracle {ow:6.2f}s (JVM+compile overhead ~"
          f"{ow - sum(sum(p.values()) for p in opass) / 1e3:5.2f}s)")
    ratios = sorted(((o_warm[k] / e_warm[k], k) for k in e_warm if e_warm[k] > 0),
                    reverse=True)
    med = statistics.median(r for r, _ in ratios)
    print(f"oracle-warm / engine ratio: median {med:8.1f}x   "
          f"min {ratios[-1][0]:6.1f}x   max {ratios[0][0]:8.1f}x")
    print("  largest gaps:")
    for r, k in ratios[:3]:
        print(f"    {k:36s} engine {e_warm[k]:9.3f}ms  oracle {o_warm[k]:9.1f}ms  ({r:,.0f}x)")
    print("  smallest gaps:")
    for r, k in ratios[-3:]:
        print(f"    {k:36s} engine {e_warm[k]:9.3f}ms  oracle {o_warm[k]:9.1f}ms  ({r:,.0f}x)")
    slopes = list(_slope_lines(e_warm))
    if slopes:
        print("  engine growth ratios (doubling sizes: ~2 linear, ~4 quadratic):")
        for line in slopes:
            print(line)


def _slope_lines(warm):
    """Per-workload consecutive-growth ratios over a size ladder — only
    meaningful when --sizes doubles (the D-385 quadratic detector)."""
    import collections
    import re
    fam = collections.defaultdict(list)
    for k, ms in warm.items():
        m = re.fullmatch(r"(.+)_(\d+)", k)
        if m:
            fam[m.group(1)].append((int(m.group(2)), ms))
    for name in sorted(fam):
        pts = sorted(fam[name])
        if len(pts) < 3:
            continue
        rs = ["%.2f" % (b / a) for (_, a), (_, b) in zip(pts, pts[1:]) if a > 0]
        yield f"    {name:12s} " + "  ".join(rs)


# ------------------------------------------------------------ scale suite

def _scn(name, drl, types, facts):
    return {"name": name, "drl": drl, "facts": facts, "epochs": [], "types": types}


T0 = {"name": "T0", "fields": [{"name": "k", "type": "i64"}, {"name": "v", "type": "i64"}]}
T1 = {"name": "T1", "fields": [{"name": "k", "type": "i64"}, {"name": "w", "type": "i64"}]}


def gen_scale(outdir, sizes):
    paths = []
    for n in sizes:
        cases = {
            f"alpha_{n}": _scn(
                f"alpha_{n}",
                'rule "R0"\nwhen\n    T0(v > 1)\nthen\nend\n',
                [T0],
                [{"type": "T0", "fields": {"k": i, "v": i % 4}} for i in range(n)]),
            f"join_{n}": _scn(
                f"join_{n}",
                'rule "R0"\nwhen\n    T0($k : k)\n    T1(k == $k)\nthen\nend\n',
                [T0, T1],
                [{"type": "T0", "fields": {"k": i, "v": i}} for i in range(n)]
                + [{"type": "T1", "fields": {"k": i, "w": i}} for i in range(n)]),
            f"acc_{n}": _scn(
                f"acc_{n}",
                'rule "R0"\nwhen\n    T1($k : k)\n    accumulate( T0(v >= 0, $s : v); $a : sum($s) )\nthen\nend\n',
                [T0, T1],
                [{"type": "T0", "fields": {"k": i, "v": i}} for i in range(n)]
                + [{"type": "T1", "fields": {"k": 0, "w": 0}}]),
            # D-297/298 lane: logical chain to depth n, then the seed
            # delete tears the whole belief chain down in one wave.
            f"tms_{n}": _scn(
                f"tms_{n}",
                (f'rule "Grow"\nwhen\n    T($n : n, n < {n})\nthen\n'
                 '    insertLogical(new T($n + 1));\nend\n'
                 'rule "WGone"\nwhen\n    P(tag == "e1")\n    not T()\nthen\nend\n'),
                [{"name": "T", "fields": [{"name": "n", "type": "i64"}]},
                 {"name": "P", "fields": [{"name": "tag", "type": "String"}]}],
                [{"type": "T", "fields": {"n": 1}}]),
            # D-385 lane: n updates in ONE epoch through a join — the
            # shape where the window/queue/join-drain quadratics lived.
            f"churn_{n}": _scn(
                f"churn_{n}",
                'rule "R0"\nwhen\n    T0($k : k)\n    T1(k == $k)\nthen\nend\n',
                [T0, T1],
                [{"type": "T0", "fields": {"k": i, "v": i}} for i in range(n)]
                + [{"type": "T1", "fields": {"k": i, "w": i}} for i in range(n)]),
        }
        cases[f"tms_{n}"]["epochs"] = [
            {"actions": [{"op": "delete", "target": 0}],
             "facts": [{"type": "P", "fields": {"tag": "e1"}}]}]
        cases[f"churn_{n}"]["epochs"] = [
            {"actions": [{"op": "update", "target": i, "fields": {"k": i, "v": i + 1}}
                         for i in range(n)],
             "facts": []}]
        for name, scn in cases.items():
            p = os.path.join(outdir, f"{name}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
    return paths


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("paths", nargs="*")
    ap.add_argument("--corpus", type=int, default=0,
                    help="sample N certified scenarios (seed 42)")
    ap.add_argument("--scale", action="store_true",
                    help="parametric alpha/join/accumulate suite")
    ap.add_argument("--sizes", default="100,1000,5000")
    ap.add_argument("--repeats", type=int, default=2)
    args = ap.parse_args()

    if not os.path.exists(ENGINE):
        sys.exit("build the release harness first: cargo build --release -p seine-harness")
    if args.corpus:
        pool = sorted(glob.glob(f"{ROOT}/scenarios/probes/*.json")
                      + glob.glob(f"{ROOT}/scenarios/regressions/*.json"))
        files = sorted(random.Random(42).sample(pool, min(args.corpus, len(pool))))
        bench(files, args.repeats, f"corpus sample ({len(files)})")
    if args.scale:
        sizes = [int(s) for s in args.sizes.split(",")]
        with tempfile.TemporaryDirectory() as d:
            files = gen_scale(d, sizes)
            bench(files, args.repeats, f"scale suite (n = {args.sizes})")
        # Doubling ladders (e.g. --sizes 2000,4000,8000,16000,32000)
        # read the growth exponent directly: consecutive ratio ~2 =
        # linear, ~4 = quadratic (how D-385's churn quadratic was found).
    if args.paths:
        bench(args.paths, args.repeats, "explicit files")


if __name__ == "__main__":
    main()
