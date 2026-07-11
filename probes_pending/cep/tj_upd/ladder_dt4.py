#!/usr/bin/env python3
"""D-169 double-touch ladder ROUND 4 — the tag-write deferral class.

T4 (post round-3): an update on a self-join fact defers its $b-side
right-upd (no move, no refires at the action) iff it WRITES TAG and is
not an alpha-ENTRY: {noop y->y tag write, EXIT z->y}. ENTRY (y->z) and
every ts-only write process immediately. Deferred staging drains at
the node's next eval: insert-eval drains BEFORE its scan (ex7);
modify-entry-eval scans/emits ins FIRST, drains after (dt2b/ex9/dt1);
plain-modify-eval drains in staging order; epoch-end otherwise.
Cells:
 TV1  tag-VI on F, then DIFFERENT-fact entry: F unmoved at N's scan +
      (A,F) refire drained BEHIND N's ins batch?
 TB1  both-fields write (tag y->y + fresh ts) then same-fact entry:
      deferred (behind) or immediate (ts wins)?
 IP1  in-place (z->z) A' refires then a different fact's entry:
      A'-block behind the entry's ins (deferred) or in front?
 DR1  exit staging drains AT the re-entry action's position (before a
      3rd action's fires), not at epoch end?
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
    # TV1: deferred -> ep: ['30z|30z','30z|22y','30z|20y','30z|15y','60z|30z','60z|20y']
    #      immediate-> ep: ['60z|20y','30z|20y','30z|30z','30z|22y','30z|15y','60z|30z']
    C("tv1_tagvi_then_other_entry", 100,
      [(15, "y"), (20, "y"), (22, "y"), (30, "y"), (60, "z")],
      [([{"target": 1, "tag": "y"}, {"target": 3, "tag": "z"}], [])]),
    # TB1: deferred -> ep: ['19z|19z','19z|15y','60z|19z']
    #      immediate-> ep: ['60z|19z','19z|19z','19z|15y']
    C("tb1_bothwrite_then_entry", 100, [(15, "y"), (20, "y"), (60, "z")],
      [([{"target": 1, "tag": "y", "ts": 19}, {"target": 1, "tag": "z"}], [])]),
    # IP1: deferred -> ep: ['30z|30z','30z|15y','60z|30z','60z|60z','60z|15y']
    #      immediate-> ep: ['60z|30y','60z|60z','60z|15y','30z|30z','30z|15y']
    C("ip1_inplace_then_entry", 100, [(15, "y"), (60, "z"), (30, "y")],
      [([{"target": 1, "tag": "z"}, {"target": 2, "tag": "z"}], [])]),
    # DR1: drain-at-reentry -> ep: ['30z|19y','30z|30z','30z|15y','60z|30z','60z|19y']
    #      drain-at-end     -> ep: ['30z|19y','30z|30z','30z|15y','60z|19y','60z|30z']
    C("dr1_drain_at_reentry", 100,
      [(15, "y"), (30, "z"), (20, "y"), (60, "z")],
      [([{"target": 1, "tag": "y"}, {"target": 1, "tag": "z"},
         {"target": 2, "ts": 19}], [])]),
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
