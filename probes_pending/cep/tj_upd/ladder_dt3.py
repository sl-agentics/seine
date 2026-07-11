#!/usr/bin/env python3
"""D-169 double-touch ladder ROUND 3 — the deferral-class split.

Working theory (T3) after rounds 1-2:
  IMMEDIATE class (plain real-change update): refires emit at the
    action's position; $b tail-move visible to later scans in-epoch;
    later same-epoch dups keep-first (u5/dup1/int1).
  DEFERRED class (alpha-EXIT; value-identical write?): the $b-side
    right-upd stays STAGED — no move, no emission at the action; it
    drains at the node's NEXT EVAL (a later action's forced eval or
    the epoch-end eval), AFTER that eval's insert-scan/ins emissions.
Cells:
 VI1  value-identical ts write then same-fact entry: refire behind
      entry (deferred) or in front (immediate)?
 VI3  value-identical ts write then DIFFERENT-fact entry: does the
      entry's scan see the VI move? do the VI refires drain after the
      entry's ins batch?
 EX7  exit then same-epoch INSERT: does the insert's scan see the
      exit move? (T3: no — stale upd drains after the scan)
 EX8  exit refire vs TWO later plain updates: stale drain at the next
      eval (position 1-ish) vs epoch end (last)?
 EX9  exit then different-fact ENTRY: scan sees exited fact unmoved?
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_tjupd_v4 as V


def C(name, hi, facts, epochs):
    return {"name": name, "self_join": True, "lo": 0, "hi": hi,
            "facts": [(t, g, "both") for (t, g) in facts],
            "epochs": [{"actions": a, "facts": [(t, g, "both") for (t, g) in f]}
                       for (a, f) in epochs]}

CELLS = [
    # VI1: deferred -> ep: ['20z|20z','20z|15y','60z|20z']
    #      immediate -> ep: ['60z|20z','20z|20z','20z|15y']
    C("vi1_vi_ts_then_entry", 100, [(15, "y"), (20, "y"), (60, "z")],
      [([{"target": 1, "ts": 20}, {"target": 1, "tag": "z"}], [])]),
    # VI3: deferred -> ep: ['30z|30z','30z|22y','30z|20y','30z|15y','60z|30z','60z|20y']
    #      immediate -> ep: ['60z|20y','30z|20y','30z|30z','30z|22y','30z|15y','60z|30z']
    C("vi3_vi_ts_then_other_entry", 100,
      [(15, "y"), (20, "y"), (22, "y"), (30, "y"), (60, "z")],
      [([{"target": 1, "ts": 20}, {"target": 3, "tag": "z"}], [])]),
    # EX7: scan-unmoved -> ep: ['30z|22y','30z|20y','30z|15y','30z|30z']
    #      scan-moved   -> ep: ['30z|20y','30z|22y','30z|15y','30z|30z']
    C("ex7_exit_then_insert_scan", 100, [(15, "y"), (20, "z"), (22, "y")],
      [([{"target": 1, "tag": "y"}], [(30, "z")])]),
    # EX8: drain-at-next-eval -> ep: ['60z|30y','60z|14y','60z|9y']
    #      drain-at-epoch-end -> ep: ['60z|14y','60z|9y','60z|30y']
    C("ex8_exit_refire_drain_point", 100,
      [(10, "y"), (15, "y"), (30, "z"), (60, "z")],
      [([{"target": 2, "tag": "y"}, {"target": 1, "ts": 14},
         {"target": 0, "ts": 9}], [])]),
    # EX9: scan-unmoved -> ep: ['30z|30z','30z|22y','30z|20y','30z|15y']
    #      scan-moved   -> ep: ['30z|20y','30z|30z','30z|22y','30z|15y']
    C("ex9_exit_then_other_entry_scan", 100,
      [(15, "y"), (20, "z"), (22, "y"), (30, "y")],
      [([{"target": 1, "tag": "y"}, {"target": 3, "tag": "z"}], [])]),
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
    ora2 = V.oracle_seq(paths)
    for s in CELLS:
        nm = s["name"]
        stable = "STABLE" if ora1.get(nm) == ora2.get(nm) else "FLAKY!"
        print(f"===== {nm} ({stable})")
        print("  model :", V.simulate(s))
        print("  oracle:", ora1.get(nm))


if __name__ == "__main__":
    main()
