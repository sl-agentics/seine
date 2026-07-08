#!/usr/bin/env python3
"""CEP E2 item E — expiration boundary probes (Probe 3)."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_exp"
os.makedirs(OUT, exist_ok=True)

def scen(name, interval, dur, expires, advance_to):
    ev = {"timestamp": "ts", "expires_ms": expires}
    if interval:
        ev["duration"] = "dur"
    etype = {"name": "E",
             "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
             "event": ev}
    ptype = {"name": "P", "fields": [{"name": "v", "type": "i64"}]}
    return {
        "name": name,
        "types": [etype, ptype],
        # R fires on E at insert; N fires once E has expired (not E()).
        "drl": "rule R when E() then end\nrule N salience -1 when not E() P() then end\n",
        "facts": [
            {"type": "E", "fields": {"ts": 0, "dur": dur}},
            {"type": "P", "fields": {"v": 1}},
        ],
        "epochs": [{"actions": [{"op": "advance", "ms": advance_to}], "facts": []}],
    }

def write(s):
    with open(f"{OUT}/{s['name']}.json", "w") as f:
        json.dump(s, f, indent=1)

# --- POINT baseline: E ts=0 @expires(100). Pin the exact expiry boundary.
for T in (99, 100, 101, 102):
    write(scen(f"ex_pt_exp100_at{T}", False, 0, 100, T))

# --- INTERVAL: E ts=0 dur=50 @expires(100).
#   H1 (end+expires): expiry at 0+50+100 = 150.
#   H2 (start+expires, dur ignored): expiry at 0+100 = 100.
# Coarse discriminator at T=120 (between 100 and 150):
write(scen("ex_int_d50_exp100_at120", True, 50, 100, 120))
# Fine boundary near 150 (H1) and near 100 (H2):
for T in (99, 100, 101, 149, 150, 151, 152):
    write(scen(f"ex_int_d50_exp100_at{T}", True, 50, 100, T))

# --- INTERVAL dur=0 with @expires(100): must equal the point baseline.
for T in (100, 101):
    write(scen(f"ex_int_d0_exp100_at{T}", True, 0, 100, T))

print("wrote", len(os.listdir(OUT)), "expiry scenarios")
