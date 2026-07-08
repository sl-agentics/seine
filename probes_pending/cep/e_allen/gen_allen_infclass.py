#!/usr/bin/env python3
"""D-119: FULL never/finite classification of the @expires inference reach for
every Allen op x position. Insert ONE event, advance far, observe presence.
PRESENT=never (no inference edge; ports free); GONE=finite (reach must be pinned)."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_allen_infclass"
os.makedirs(OUT, exist_ok=True)

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}  # no expires -> inference

def w(name, op, keep):
    s = {"name": name, "types": [typ("A"), typ("B")],
         "drl": f"rule R when $a : A() $b : B(this {op} $a) then end\n",
         "facts": [{"type": keep, "fields": {"ts": 0, "dur": 50}}],
         "epochs": [{"actions": [{"op": "advance", "ms": 100000}], "facts": []}]}
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(s, f, indent=1)

OPS = ["after[0ms,100ms]", "before[0ms,100ms]", "coincides", "meets", "metby",
       "overlaps", "overlappedby", "during", "includes", "starts", "startedby",
       "finishes", "finishedby"]
for op in OPS:
    tag = op.split("[")[0]
    w(f"ic_{tag}_keepA", op, "A")
    w(f"ic_{tag}_keepB", op, "B")

print("wrote", len(os.listdir(OUT)), "inference-classification scenarios")
