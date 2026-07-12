#!/usr/bin/env python3
"""xfail engine-drift gate (D-187).

scenarios/xfail/ holds DOCUMENTED-OPEN divergences — excluded from the
oracle diff BY DESIGN (they diverge from Drools; that is their finding).
But the ENGINE's output on them is deterministic state, and it moved
SILENTLY twice (D-091 `f70b189`, D-101 `bb6eb6d`) because no gate
watched it — found five days later by the D-186 re-baseline bisects.

This gate compares the engine's canonical output on every xfail witness
against a banked snapshot (scenarios/xfail-engine-baseline.ndjson,
committed). Movement is allowed only DELIBERATELY:

    re-triage (tools/triage_xfail.py) -> --rebank -> D-entry

Comparison is canonical per D-003 (facts multiset, firings ordered,
matches multiset, f64 bit equality) via triage_xfail's canonicalizer —
formatting-only churn in the serializer does not trip the gate; any
semantic movement does. Set drift (a witness added to / removed from
xfail/ without rebanking) also fails the gate.

Usage: python3 tools/xfail_drift.py [--rebank] [--dir D] [--bank F]
Exit: 0 clean, 1 drift/set-mismatch, 2 harness failure.
"""
import argparse, os, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from triage_xfail import entry_state, load_ndjson  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HARNESS = ["cargo", "run", "-q", "-p", "seine-harness", "--"]


def run_engine(files, out_path):
    with open(out_path, "w") as fh:
        r = subprocess.run(HARNESS + ["run"] + files, stdout=fh, cwd=REPO)
    if r.returncode != 0:
        sys.exit(2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rebank", action="store_true",
                    help="overwrite the bank with the current engine output "
                         "(deliberate movement only: re-triage + D-entry first)")
    ap.add_argument("--dir", default=os.path.join(REPO, "scenarios", "xfail"))
    ap.add_argument("--bank",
                    default=os.path.join(REPO, "scenarios",
                                         "xfail-engine-baseline.ndjson"))
    args = ap.parse_args()

    files = sorted(os.path.join(args.dir, f)
                   for f in os.listdir(args.dir) if f.endswith(".json"))
    if args.rebank:
        run_engine(files, args.bank)
        n = len(load_ndjson(args.bank))
        print(f"xfail drift gate: REBANKED {n} witnesses -> {args.bank}")
        return

    if not os.path.exists(args.bank):
        sys.exit(f"xfail drift gate: no bank at {args.bank} — "
                 f"run tools/xfail_drift.py --rebank once (then commit it)")
    banked = load_ndjson(args.bank)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".ndjson",
                                     delete=False) as tf:
        fresh_path = tf.name
    try:
        run_engine(files, fresh_path)
        fresh = load_ndjson(fresh_path)
    finally:
        os.unlink(fresh_path)

    problems = []
    for name in sorted(set(banked) - set(fresh)):
        problems.append(f"  {name}: in bank but not in scenarios/xfail/ "
                        f"(graduated/removed? --rebank deliberately)")
    for name in sorted(set(fresh) - set(banked)):
        problems.append(f"  {name}: in scenarios/xfail/ but not banked "
                        f"(new witness? --rebank deliberately)")
    for name in sorted(set(fresh) & set(banked)):
        bs, bres = entry_state(banked[name])
        fs, fres = entry_state(fresh[name])
        if bs != fs:
            problems.append(f"  {name}: engine state moved {bs} -> {fs}")
        elif bres != fres:
            bf = len(banked[name].get("result", {}).get("firings", []))
            ff = len(fresh[name].get("result", {}).get("firings", []))
            problems.append(f"  {name}: engine output moved "
                            f"(firings {bf} -> {ff}"
                            + ("" if bf != ff else "; same count, "
                               "order/matches/facts differ") + ")")

    if problems:
        print(f"xfail drift gate: ENGINE MOVEMENT on the quarantine "
              f"({len(problems)} witness(es)):")
        print("\n".join(problems))
        print("movement must be deliberate: re-triage "
              "(tools/triage_xfail.py), then --rebank, with a D-entry.")
        sys.exit(1)
    print(f"xfail drift gate: {len(fresh)} witnesses, "
          f"engine output identical to bank")


if __name__ == "__main__":
    main()
