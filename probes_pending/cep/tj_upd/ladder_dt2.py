#!/usr/bin/env python3
"""D-169 double-touch ladder ROUND 2 — timing/durability of the phase-C
move per update class, exit-refire emission position, and the
entry-repositioning with multiple prior refires.

Open questions after round 1 (each cell lists the competing answers):
 EX1  is an alpha-EXIT's $b tail-move DURABLE across epochs?
      (round-1 dt2b + tu-cells prove it is INVISIBLE in-epoch;
       tu11x145's ep0-exit looked durable — confirm in isolation)
 EX2  is an alpha-ENTRY's $b tail-move durable across epochs?
 EX4  is a same-epoch RE-ENTRY's move visible to a LATER entry scan
      in that epoch? (transition-deferral vs plain-immediacy)
 EX6  is a plain ENTRY's move visible to a later entry scan in-epoch?
 EX5  does an EXIT's $b-side refire emit at its own action position
      (per-action FIFO) or defer to epoch end?
 DUP1 entry-repositioning with TWO prior refires: block moves behind
      the entry's ins emissions, original order preserved?
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
    # EX1: F(20,z) exits ep1; N enters ep2 and scans.
    #  exit move durable  -> ['30z|20y','30z|30z','30z|22y','30z|15y'] (epoch part)
    #  exit never moves   -> ['30z|30z','30z|22y','30z|20y','30z|15y']
    C("ex1_exit_durable", 100, [(15, "y"), (20, "z"), (22, "y"), (30, "y")],
      [([{"target": 1, "tag": "y"}], []),
       ([{"target": 3, "tag": "z"}], [])]),
    # EX2: F(20,y) enters ep1; N enters ep2 and scans.
    #  entry move durable -> ['30z|20z','30z|30z','30z|22y','30z|15y']
    #  entry never moves  -> ['30z|30z','30z|22y','30z|20z','30z|15y']
    C("ex2_entry_durable", 100, [(15, "y"), (20, "y"), (22, "y"), (30, "y")],
      [([{"target": 1, "tag": "z"}], []),
       ([{"target": 3, "tag": "z"}], [])]),
    # EX4: exit+reentry of F, then G enters SAME epoch and scans F's slot.
    #  transition moves deferred -> G part: ['25z|25z','25z|20z','25z|15y']
    #  re-entry move immediate   -> ['25z|20z','25z|25z','25z|15y']
    C("ex4_reentry_scan_same_epoch", 100, [(15, "y"), (20, "z"), (25, "y")],
      [([{"target": 1, "tag": "y"}, {"target": 1, "tag": "z"},
         {"target": 2, "tag": "z"}], [])]),
    # EX6: plain entry of F, then G enters SAME epoch and scans.
    #  entry move immediate -> ['25z|20z','25z|25z','25z|15y']
    #  entry move deferred  -> ['25z|25z','25z|20z','25z|15y']
    C("ex6_entry_scan_same_epoch", 100, [(15, "y"), (20, "y"), (25, "y")],
      [([{"target": 1, "tag": "z"}, {"target": 2, "tag": "z"}], [])]),
    # EX5: EXIT of F refires (A,F); then B's plain update refires (A,B).
    #  exit refire at its action position -> ['60z|30y','60z|14y']
    #  exit refire deferred to epoch end  -> ['60z|14y','60z|30y']
    C("ex5_exit_refire_pos", 100, [(15, "y"), (30, "z"), (60, "z")],
      [([{"target": 1, "tag": "y"}, {"target": 0, "ts": 14}], [])]),
    # DUP1: two refires staged, then same-fact ENTRY.
    #  reposition (T') -> ['19z|19z','19z|15y','65z|19z','60z|19z']
    #  keep-first (M)  -> ['65z|19z','60z|19z','19z|19z','19z|15y']
    C("dup1_two_refires_entry", 100, [(15, "y"), (20, "y"), (60, "z"), (65, "z")],
      [([{"target": 1, "ts": 19}, {"target": 1, "tag": "z"}], [])]),
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
