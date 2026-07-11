#!/usr/bin/env python3
"""D-169 double-touch ladder ROUND 5 — final corners before encoding.

 TV2   childless tag-VI then a DIFFERENT fact's entry: is the VI move
       visible to that scan (rule: yes, tag-move immediate-for-others)?
 TV3   childed tag-VI then the SAME fact's entry, self mid-batch:
       self-slot = pre-epoch (emit [(F,C),(F,F),(F,B)]) or moved-tail
       (emit [(F,F),(F,C),(F,B)])?
 EX10  exit-with-surviving-refire then a DIFFERENT fact's entry: the
       exit refire stays at its own position (no relocation) or drains
       behind the other entry's ins batch?
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
    # TV2: moved  -> ep: ['30z|20y','30z|30z','30z|22y','30z|15y']
    #      unmoved-> ep: ['30z|30z','30z|22y','30z|20y','30z|15y']
    C("tv2_childless_tagvi_other_entry", 100,
      [(15, "y"), (20, "y"), (22, "y"), (30, "y")],
      [([{"target": 1, "tag": "y"}, {"target": 3, "tag": "z"}], [])]),
    # TV3: self-pre-epoch-slot -> ep: ['20z|18y','20z|20z','20z|15y','60z|20z']
    #      self-moved-tail     -> ep: ['20z|20z','20z|18y','20z|15y','60z|20z']
    C("tv3_childed_tagvi_own_entry", 100,
      [(15, "y"), (20, "y"), (18, "y"), (60, "z")],
      [([{"target": 1, "tag": "y"}, {"target": 1, "tag": "z"}], [])]),
    # EX10: refire stays at act1 -> ep: ['60z|30y','20z|20z','20z|15y','60z|20z']
    #       drains behind entry  -> ep: ['20z|20z','20z|15y','60z|20z','60z|30y'] (or interleaved)
    C("ex10_exit_refire_other_entry", 100,
      [(15, "y"), (30, "z"), (20, "y"), (60, "z")],
      [([{"target": 1, "tag": "y"}, {"target": 2, "tag": "z"}], [])]),
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
