#!/usr/bin/env python3
"""CEP E2 item E — composition probes: not/exists x interval (P6),
mutation x duration (P8), window:time x interval smoke (P5)."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_comp"
os.makedirs(OUT, exist_ok=True)

def typ(name, interval=True, extra=None):
    ev = {"timestamp": "ts"}
    if interval:
        ev["duration"] = "dur"
    if extra:
        ev.update(extra)
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": ev}

def write(name, types, drl, facts, epochs=None):
    s = {"name": name, "types": types, "drl": drl, "facts": facts, "epochs": epochs or []}
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(s, f, indent=1)

# ---- P6: exists / not with temporal constraint over an interval anchor.
# `exists B(this after[60,80] $a:A)`: A interval dur=30 -> A.end=130,
# B.ts=200 -> distance 70 in[60,80] -> exists holds. Point A -> distance 100 -> not.
AB = [typ("A"), typ("B")]
write("cp_exists_int_fire", AB,
      "rule EX when $a : A() exists B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}])
write("cp_exists_pt_inert", [typ("A", interval=False), typ("B")],
      "rule EX when $a : A() exists B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}])
# `not`: fires when NO B in window. Interval A (dist 70, B present) -> B in window -> not FAILS (inert).
write("cp_not_int_inert", AB,
      "rule NE when $a : A() not B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}])
write("cp_not_pt_fire", [typ("A", interval=False), typ("B")],
      "rule NE when $a : A() not B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}])

# ---- P8: mutation x duration. Insert A with dur=0 (distance 100, NO match in
# [60,80]); then UPDATE A.dur=30 (distance would be 70 -> match IF re-read).
# Fixed-at-insert => still inert; re-read-on-update => fires.
write("cp_mut_dur_0to30", AB,
      "rule R when $a : A() $b : B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 0}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}],
      epochs=[{"actions": [{"op": "update", "target": 0, "fields": {"dur": 30}}], "facts": []}])
# Control: A inserted with dur=30 from the start -> fires (baseline for the mutation).
write("cp_mut_ctl_30", AB,
      "rule R when $a : A() $b : B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}])
# Reverse: insert dur=30 (fires), then update dur=0 (distance 100 -> would NOT match).
write("cp_mut_dur_30to0", AB,
      "rule R when $a : A() $b : B(this after[60ms,80ms] $a) then end\n",
      [{"type": "A", "fields": {"ts": 100, "dur": 30}},
       {"type": "B", "fields": {"ts": 200, "dur": 0}}],
      epochs=[{"actions": [{"op": "update", "target": 0, "fields": {"dur": 0}}], "facts": []}])

# ---- P5: window:time x interval smoke — does an interval event flow through
# a window:time accumulate without error, and does membership track start/end?
# accumulate count over window:time(100ms). E interval dur=50 at ts=0.
# Observe count via a threshold rule at two clock times (start-evict@101 vs end-evict@151).
for T in (120, 160):
    write(f"cp_win_int_d50_at{T}", [typ("E")],
          "rule W when accumulate($e : E() over window:time(100ms); $c : count($e); $c >= 1) then end\n",
          [{"type": "E", "fields": {"ts": 0, "dur": 50}}],
          epochs=[{"actions": [{"op": "advance", "ms": T}], "facts": []}])

print("wrote", len(os.listdir(OUT)), "composition scenarios")
