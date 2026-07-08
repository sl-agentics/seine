#!/usr/bin/env python3
"""D-119: does the @expires INFERENCE (D-109) reach through Allen ops?
Insert ONLY the event of interest (inference is structural); advance far and
observe presence via FACTS. Present@big => NEVER (no finite reach); gone => finite."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_allen_inf"
os.makedirs(OUT, exist_ok=True)

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}  # NO expires -> inference

def w(name, op, keep, kdur, advance_to=100000):
    # keep = 'A' or 'B' (the only inserted event); rule references both.
    ts = 0
    facts = [{"type": keep, "fields": {"ts": ts, "dur": kdur}}]
    s = {"name": name, "types": [typ("A"), typ("B")],
         "drl": f"rule R when $a : A() $b : B(this {op} $a) then end\n",
         "facts": facts,
         "epochs": [{"actions": [{"op": "advance", "ms": advance_to}], "facts": []}]}
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(s, f, indent=1)

# bare during vs during[min,max]: does param bound the inferred reach?
w("inf_during_bare_A",  "during",            "A", 100)
w("inf_during_bare_B",  "during",            "B", 30)
w("inf_during_param_A", "during[10ms,50ms]", "A", 100)
w("inf_during_param_B", "during[10ms,50ms]", "B", 30)
# coincides (bounded relation - both endpoints tied)
w("inf_coincides_A", "coincides", "A", 50)
w("inf_coincides_B", "coincides", "B", 50)
# overlaps bare vs param
w("inf_overlaps_bare_A",  "overlaps",          "A", 50)
w("inf_overlaps_param_A", "overlaps[10ms,25ms]","A", 50)
# finishes / meets (equality-anchored)
w("inf_finishes_A", "finishes", "A", 80)
w("inf_meets_A",    "meets",    "A", 50)

print("wrote", len(os.listdir(OUT)), "allen-inference smoke scenarios")
