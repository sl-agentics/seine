#!/usr/bin/env python3
"""CEP E2 item E recon probe generator (oracle-only ground truth)."""
import json, os

OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes"
os.makedirs(OUT, exist_ok=True)

def typ(name, interval, dur_field="dur"):
    ev = {"timestamp": "ts"}
    if interval:
        ev["duration"] = dur_field
    return {
        "name": name,
        "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
        "event": ev,
    }

def scen(name, a_interval, b_interval, a_ts, a_dur, b_ts, b_dur, lo, hi, drl_win=None):
    win = drl_win if drl_win else f"{lo}ms,{hi}ms"
    drl = f"rule R when $a : A() $b : B(this after[{win}] $a) then end\n"
    return {
        "name": name,
        "types": [typ("A", a_interval), typ("B", b_interval)],
        "drl": drl,
        "facts": [
            {"type": "A", "fields": {"ts": a_ts, "dur": a_dur}},
            {"type": "B", "fields": {"ts": b_ts, "dur": b_dur}},
        ],
        "epochs": [],
    }

def write(s):
    with open(f"{OUT}/{s['name']}.json", "w") as f:
        json.dump(s, f, indent=1)

# ---- Probe 1: the 2x2 core. A.ts=100 B.ts=200; point distance=100.
# interval A dur=30 -> A.end=130 -> distance = 200-130 = 70.
# Rule window [60,80]: point A (100) INERT; interval A (70) FIRES.
# Prediction if `after` uses A.end = A.ts + A.dur:
#   pp inert, ip FIRE, pi inert (B.dur irrelevant), ii FIRE.
write(scen("e_p1_pp", False, False, 100, 30, 200, 30, 60, 80))
write(scen("e_p1_ip", True,  False, 100, 30, 200, 30, 60, 80))
write(scen("e_p1_pi", False, True,  100, 30, 200, 30, 60, 80))
write(scen("e_p1_ii", True,  True,  100, 30, 200, 30, 60, 80))

# ---- Probe 1b: boundary sweep on the POINT baseline (distance=100).
# Pin inclusive/exclusive of after[lo,hi] before trusting interval math.
write(scen("e_p1b_pt_100_100", False, False, 100, 0, 200, 0, 100, 100))  # lo=hi=100 exact
write(scen("e_p1b_pt_101_200", False, False, 100, 0, 200, 0, 101, 200))  # lo just above -> inert?
write(scen("e_p1b_pt_0_99",    False, False, 100, 0, 200, 0, 0,   99))    # hi just below -> inert?
write(scen("e_p1b_pt_0_100",   False, False, 100, 0, 200, 0, 0,   100))   # hi inclusive -> fire?

# ---- Probe 1c: interval endpoint EXACTNESS. A dur=30 -> distance should be 70.
# Window [70,70]: fires iff A.end == A.ts+A.dur exactly (no +/-1).
write(scen("e_p1c_int_70_70", True, False, 100, 30, 200, 0, 70, 70))   # exact -> fire?
write(scen("e_p1c_int_69_69", True, False, 100, 30, 200, 0, 69, 69))   # off-by-one low -> inert?
write(scen("e_p1c_int_71_71", True, False, 100, 30, 200, 0, 71, 71))   # off-by-one high -> inert?

# ---- Probe 2: dur=0 equivalence. @duration(dur) with dur=0 must == point.
# Reuse the [90,110] window: point distance=100 FIRES; @duration(0) must also.
write(scen("e_p2_pt_90_110",   False, False, 100, 0, 200, 0, 90, 110))  # point control -> fire
write(scen("e_p2_dur0_90_110", True,  False, 100, 0, 200, 0, 90, 110))  # @duration(dur)=0 -> must == control

print("wrote", len(os.listdir(OUT)), "scenarios to", OUT)
