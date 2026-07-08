#!/usr/bin/env python3
"""CEP E2 item E — `before` over intervals + Allen-operator scope sampling.

Rule form: `$a : A() $b : B(this <op> $a)`  => the relation is "B <op> A".
Event X occupies [X.ts, X.ts+X.dur]. Each op gets a FIRE case (relation holds)
and a NEAR-MISS control (duration tweaked so it no longer holds).
"""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_allen"
os.makedirs(OUT, exist_ok=True)

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}

def scen(name, op, a_ts, a_dur, b_ts, b_dur):
    drl = f"rule R when $a : A() $b : B(this {op} $a) then end\n"
    return {"name": name, "types": [typ("A"), typ("B")], "drl": drl,
            "facts": [{"type": "A", "fields": {"ts": a_ts, "dur": a_dur}},
                      {"type": "B", "fields": {"ts": b_ts, "dur": b_dur}}],
            "epochs": []}

def write(s):
    with open(f"{OUT}/{s['name']}.json", "w") as f:
        json.dump(s, f, indent=1)

# --- before[60,80] over intervals: B earlier (this), A later.
# distance = A.start - B.end = A.ts - (B.ts+B.dur). B=[100,dur], A.ts=200.
# dur=30 -> 70 in[60,80] FIRE ; dur=0 -> 100 NOT in -> inert (B.dur matters).
write(scen("al_before_int_fire", "before[60ms,80ms]", 200, 0, 100, 30))
write(scen("al_before_pt_inert", "before[60ms,80ms]", 200, 0, 100, 0))

# --- during: B strictly inside A (as<bs && be<ae). A=[0,100], B=[20,50].
write(scen("al_during_fire",  "during", 0, 100, 20, 30))   # 0<20 & 50<100 -> B during A
write(scen("al_during_miss",  "during", 0, 100, 0,  30))   # bs=0 not > as=0 -> inert
# during is IMPOSSIBLE for point events (a point can't be strictly inside):
write(scen("al_during_points","during", 0, 0,   20, 0))    # both points -> inert

# --- coincides: same start AND same end. A=[0,50], B=[0,50].
write(scen("al_coincides_fire", "coincides", 0, 50, 0, 50))  # identical intervals
write(scen("al_coincides_miss", "coincides", 0, 50, 0, 40))  # end differs -> inert

# --- overlaps: B.start<A.start<B.end<A.end. B=[0,50], A=[20,100].
write(scen("al_overlaps_fire", "overlaps", 20, 80, 0, 50))   # 0<20<50<100 -> B overlaps A
write(scen("al_overlaps_miss", "overlaps", 20, 80, 0, 10))   # B.end=10<A.start=20 disjoint -> inert

# --- meets: B.end == A.start. B=[0,50], A=[50,80].
write(scen("al_meets_fire", "meets", 50, 30, 0, 50))         # B.end=50==A.start=50 -> meets
write(scen("al_meets_miss", "meets", 50, 30, 0, 40))         # B.end=40 != 50 -> inert

print("wrote", len(os.listdir(OUT)), "allen scenarios")
