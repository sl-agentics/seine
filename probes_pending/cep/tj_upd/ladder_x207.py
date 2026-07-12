#!/usr/bin/env python3
"""D-173 recon ladder — the tu51x207 3-touch ORDER compound.

Mechanism under test: the $b-refire's CHILDLIST move-to-end side
effect is per-ACTION (the model re-runs phase B each touch against the
CURRENT children), while the engine's replay attaches it to the ONE
dedup'd RUpd op at the FIRST touch's stamp — a leading same-epoch
tag-VI therefore pins the refire pass BEFORE the entry exists, and the
entry's self-child never moves (tu51x207: the ep1 A' block fires the
self-pair at its scan slot instead of the moved end).

Cells (predictions = model_tjupd_v4.simulate — the 0-div spec):
 L1  [VI, entry] ... [in-place]     — the witness shape (RED pre-fix)
 L2  [entry] ... [in-place]         — DISCRIMINATOR: no leading VI, the
     RUpd stamp lands AFTER the entry so the current engine also moves
     the self-child (green pre-fix). If L2 is red, the stamp-elision
     mechanism is WRONG.
 L3  [VI] / [entry] / [in-place]    — the VI in its own epoch (unlinked
     deferral carries the stamp across the boundary; RED pre-fix)
 L4  [VI, entry] / [in-place] / [in-place] — per-action moves compound
 L5  [VI, entry, +partner insert] / [in-place] — appends interleave
     with the move (RED pre-fix)

Not the identity-model law: no delete exists in any cell (no exits) —
the law's activation condition is categorically absent here.

Usage: ladder_x207.py
"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import model_tjupd_v4 as V


def C(name, facts, epochs):
    return {"name": name, "self_join": True, "lo": 0, "hi": 200,
            "facts": [(t, g, "both") for (t, g) in facts],
            "epochs": [{"actions": a, "facts": [(t, g, "both") for (t, g) in f]}
                       for (a, f) in epochs]}

BASE = [(57, "y"), (116, "y"), (86, "y"), (21, "y")]
VI = {"target": 2, "tag": "y"}
ENTRY = {"target": 2, "tag": "z", "ts": 75}
INPLACE = {"target": 2, "tag": "z", "ts": 118}
INPLACE2 = {"target": 2, "tag": "z", "ts": 30}

CELLS = [
    C("x2l1_vi_entry_inplace", BASE, [([VI, ENTRY], []), ([INPLACE], [])]),
    C("x2l2_entry_inplace", BASE, [([ENTRY], []), ([INPLACE], [])]),
    C("x2l3_vi_own_epoch", BASE, [([VI], []), ([ENTRY], []), ([INPLACE], [])]),
    C("x2l4_inplace_twice", BASE,
      [([VI, ENTRY], []), ([INPLACE], []), ([INPLACE2], [])]),
    C("x2l5_append_interleave", BASE,
      [([VI, ENTRY], [(80, "y")]), ([INPLACE], [])]),
]


def main():
    outdir = "/tmp/x207_ladder"
    os.makedirs(outdir, exist_ok=True)
    paths = []
    for s in CELLS:
        h = V.to_harness(s, s["name"])
        p = f"{outdir}/{s['name']}.json"
        json.dump(h, open(p, "w"), indent=1)
        paths.append(p)
    ora1 = V.oracle_seq(paths)
    ora2 = V.oracle_seq(paths)
    import subprocess
    for s in CELLS:
        nm = s["name"]
        stable = "STABLE" if ora1.get(nm) == ora2.get(nm) else "FLAKY!"
        pred = V.simulate(s)
        o = ora1.get(nm)
        r = subprocess.run(["./target/debug/seine-harness", "run",
                            f"{outdir}/{nm}.json"], capture_output=True, text=True)
        eng = None
        try:
            d = json.loads(r.stdout)
            eng = [f"{fr['matches'][0]['fields']['ts']}{fr['matches'][0]['fields']['tag']}|"
                   f"{fr['matches'][1]['fields']['ts']}{fr['matches'][1]['fields']['tag']}"
                   for fr in d["result"]["firings"]]
        except Exception:
            pass
        pm = "model==oracle" if pred == o else "MODEL✗"
        pe = "engine==oracle" if eng == o else "ENGINE✗"
        print(f"===== {nm} ({stable}) [{pm}] [{pe}]")
        print("  model :", pred)
        print("  oracle:", o)
        print("  engine:", eng)


if __name__ == "__main__":
    main()
