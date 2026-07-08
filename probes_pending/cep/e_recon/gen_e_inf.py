#!/usr/bin/env python3
"""CEP E2 item E — @duration x @expires-inference seam (Probe 4).

E is the EARLIER event in `after[0,100]` with NO explicit @expires, so D-109
infers offset = hi = 100. Question: is that inferred offset applied from the
interval END (ts+dur+100+1) like the explicit case, and does @duration change
the COMPUTED offset (it shouldn't — offset comes from constraint bounds)?
"""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_inf"
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
    ptype = {"name": "P", "fields": [{"name": "v", "type": "i64"}]}
    # TJ makes E the earlier event in after[lo,hi] -> E inherits inferred offset=hi.
    # N observes E's disappearance. F is never inserted (inference is structural).
    drl = (f"rule TJ when $a : E() $b : F(this after[{lo}ms,{hi}ms] $a) then end\n"
           "rule R when E() then end\n"
           "rule N salience -1 when not E() P() then end\n")
    return {
        "name": name,
        "types": [etype, ftype, ptype],
        "drl": drl,
        "facts": [
            {"type": "E", "fields": {"ts": 0, "dur": e_dur}},
            {"type": "P", "fields": {"v": 1}},
        ],
        "epochs": [{"actions": [{"op": "advance", "ms": advance_to}], "facts": []}],
    }

def write(s):
    with open(f"{OUT}/{s['name']}.json", "w") as f:
        json.dump(s, f, indent=1)

# P4b POINT control: inferred offset 100 -> gone@101 (D-109 re-confirm).
for T in (100, 101):
    write(scen(f"inf_pt_off100_at{T}", False, 0, T))

# P4a INTERVAL dur=50, inferred offset 100. Applied-from-END => gone@151.
#   from-START (dur ignored) => gone@101. Discriminate across the sweep.
for T in (101, 120, 150, 151, 152):
    write(scen(f"inf_int_d50_off100_at{T}", True, 50, T))

# P4c does duration feed the COMPUTED offset? interval dur=50 earlier in
# after[0,100]; if offset stayed 100 the boundary is exactly point+dur.
# (covered by comparing inf_int_* to inf_pt_* — difference should be exactly dur.)

print("wrote", len(os.listdir(OUT)), "inference-seam scenarios")
