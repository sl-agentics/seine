#!/usr/bin/env python3
"""CEP E2 item E — @duration x inference seam (Probe 4, REDONE).

Observe expiry via the FACTS output ONLY. A bare `E()` / `not E()` observer
rule adds a NON-temporal reference to E -> inferred expiry NEVER (max-merge
leak), which polluted the first attempt. Insert ONLY E; TJ makes E the earlier
event in after[0,100] so D-109 infers offset=hi=100. No F inserted.
"""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_inf2"
os.makedirs(OUT, exist_ok=True)

def scen(name, e_interval, e_dur, advance_to, lo=0, hi=100):
    ev = {"timestamp": "ts"}                 # NO expires_ms -> inference
    if e_interval:
        ev["duration"] = "dur"
    etype = {"name": "E",
             "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
             "event": ev}
    ftype = {"name": "F",
             "fields": [{"name": "ts", "type": "i64"}],
             "event": {"timestamp": "ts"}}
    # ONLY the temporal rule. No bare E() / not E() (they leak the inference).
    drl = f"rule TJ when $a : E() $b : F(this after[{lo}ms,{hi}ms] $a) then end\n"
    return {
        "name": name,
        "types": [etype, ftype],
        "drl": drl,
        "facts": [{"type": "E", "fields": {"ts": 0, "dur": e_dur}}],  # only E
        "epochs": [{"actions": [{"op": "advance", "ms": advance_to}], "facts": []}],
    }

def write(s):
    with open(f"{OUT}/{s['name']}.json", "w") as f:
        json.dump(s, f, indent=1)

# POINT control: inferred offset 100 -> present@100, gone@101 (D-109 pin).
for T in (100, 101):
    write(scen(f"i2_pt_off100_at{T}", False, 0, T))

# INTERVAL dur=50, inferred offset 100.
#   from-END => present@150, gone@151.   from-START => gone@101.
for T in (101, 120, 150, 151):
    write(scen(f"i2_int_d50_off100_at{T}", True, 50, T))

# INTERVAL dur=0 must equal the point control.
for T in (100, 101):
    write(scen(f"i2_int_d0_off100_at{T}", True, 0, T))

print("wrote", len(os.listdir(OUT)), "inference-seam v2 scenarios")
