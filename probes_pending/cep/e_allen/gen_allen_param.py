#!/usr/bin/env python3
"""D-119: Allen operator PARAMETERIZED forms. this=B, anchor=A; "B op A".
Pin what distance each optional parameter bounds and its boundary inclusivity."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_allen_param"
os.makedirs(OUT, exist_ok=True)

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}

def w(name, op, A, B):
    s = {"name": name, "types": [typ("A"), typ("B")],
         "drl": f"rule R when $a : A() $b : B(this {op} $a) then end\n",
         "facts": [{"type": "A", "fields": {"ts": A[0], "dur": A[1]-A[0]}},
                   {"type": "B", "fields": {"ts": B[0], "dur": B[1]-B[0]}}],
         "epochs": []}
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(s, f, indent=1)

# === during[max]: A=[0,100],B=[20,80] -> distStart=Bs-As=20, distEnd=Ae-Be=20
w("during_max25", "during[25ms]", A=[0,100], B=[20,80])  # 20<=25 both -> fire?
w("during_max20", "during[20ms]", A=[0,100], B=[20,80])  # boundary 20<=20 -> fire?
w("during_max15", "during[15ms]", A=[0,100], B=[20,80])  # 20>15 -> inert?
# during[min,max]
w("during_20_25", "during[20ms,25ms]", A=[0,100], B=[20,80])  # 20 in[20,25] -> fire?
w("during_21_25", "during[21ms,25ms]", A=[0,100], B=[20,80])  # 20<21 -> inert?
# during[lo1,hi1,lo2,hi2]: distStart in[lo1,hi1], distEnd in[lo2,hi2]
w("during_4p_match",  "during[15ms,25ms,15ms,25ms]", A=[0,100], B=[20,80])  # both in -> fire?
w("during_4p_endout", "during[15ms,25ms,0ms,10ms]",  A=[0,100], B=[20,80])  # distEnd=20 not in[0,10] -> inert?
# asymmetric to prove which is start vs end: B=[20,70] -> distStart=20, distEnd=30
w("during_4p_asym_ok",  "during[15ms,25ms,25ms,35ms]", A=[0,100], B=[20,70])  # 20in[15,25],30in[25,35] fire?
w("during_4p_asym_swap","during[25ms,35ms,15ms,25ms]", A=[0,100], B=[20,70])  # swapped -> inert?

# === overlaps[maxDist]: A=[30,100],B=[0,50] -> overlap region [30,50]
w("overlaps_max25", "overlaps[25ms]", A=[30,100], B=[0,50])  # overlap=Be-As=20 <=25 -> fire?
w("overlaps_max15", "overlaps[15ms]", A=[30,100], B=[0,50])  # 20>15 -> inert?
w("overlaps_min_max","overlaps[10ms,25ms]", A=[30,100], B=[0,50])  # 20 in[10,25] -> fire?
w("overlaps_min_hi", "overlaps[21ms,25ms]", A=[30,100], B=[0,50])  # 20<21 -> inert?

# === coincides[dev] / [startDev,endDev]: A=[10,60]
w("coincides_dev1_startoff","coincides[1ms]", A=[10,60], B=[11,60])  # |ds|=1<=1 -> fire?
w("coincides_dev0_startoff","coincides[0ms]", A=[10,60], B=[11,60])  # strict -> inert?
w("coincides_2dev_endoff",  "coincides[0ms,1ms]", A=[10,60], B=[10,61]) # ds=0,de=1<=1 -> fire?
w("coincides_2dev_startoff","coincides[0ms,1ms]", A=[10,60], B=[11,60]) # ds=1>0 -> inert?

# === meets[dev] / metby[dev]
w("meets_dev1", "meets[1ms]", A=[50,90], B=[0,49])  # |Be-As|=1<=1 -> fire?
w("meets_dev0", "meets[0ms]", A=[50,90], B=[0,49])  # strict -> inert?
w("metby_dev1", "metby[1ms]", A=[40,90], B=[89,120]) # |Bs-Ae|=1<=1 -> fire?

# === starts[dev] / finishes[dev]
w("starts_dev1",   "starts[1ms]",   A=[10,90], B=[11,50])  # startdev 1<=1 & Be<Ae -> fire?
w("finishes_dev1", "finishes[1ms]", A=[10,90], B=[50,89])  # enddev 1<=1 & Bs>As -> fire?

print("wrote", len(os.listdir(OUT)), "allen-param scenarios")
