#!/usr/bin/env python3
"""D-119: full Allen-operator bare-form recon. this=B, anchor=A; the rule
`$a:A() $b:B(this <op> $a)` tests the relation "B <op> A". Event X occupies
[X.ts, X.ts+X.dur]; probes give intervals as [start,end] and convert."""
import json, os
OUT = "/home/bryan/.claude/jobs/577ad61a/tmp/probes_allen_full"
os.makedirs(OUT, exist_ok=True)

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}

def scen(name, op, A, B):
    # A,B are [start,end]
    drl = f"rule R when $a : A() $b : B(this {op} $a) then end\n"
    return {"name": name, "types": [typ("A"), typ("B")], "drl": drl,
            "facts": [{"type": "A", "fields": {"ts": A[0], "dur": A[1]-A[0]}},
                      {"type": "B", "fields": {"ts": B[0], "dur": B[1]-B[0]}}],
            "epochs": []}

def w(name, op, A, B):
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(scen(name, op, A, B), f, indent=1)

# ============ coincides: B.start==A.start && B.end==A.end ============
w("coincides_fire",    "coincides", A=[10,60], B=[10,60])
w("coincides_ns_off",  "coincides", A=[10,60], B=[11,60])  # start off by 1 -> inert?
w("coincides_ne_off",  "coincides", A=[10,60], B=[10,61])  # end off by 1 -> inert?

# ============ meets: B.end == A.start ============
w("meets_fire",   "meets", A=[50,90], B=[0,50])   # B.end=50==A.start=50
w("meets_lo",     "meets", A=[50,90], B=[0,49])   # B.end=49 -> inert?
w("meets_hi",     "meets", A=[50,90], B=[0,51])   # B.end=51 -> inert?

# ============ metby: B.start == A.end ============
w("metby_fire",   "metby", A=[40,90], B=[90,120]) # B.start=90==A.end=90
w("metby_lo",     "metby", A=[40,90], B=[89,120])
w("metby_hi",     "metby", A=[40,90], B=[91,120])

# ============ overlaps: B.start<A.start<B.end<A.end ============
w("overlaps_fire",   "overlaps", A=[30,100], B=[0,50])   # 0<30<50<100
w("overlaps_before", "overlaps", A=[30,100], B=[0,20])   # B.end=20<A.start=30 (disjoint) inert
w("overlaps_during", "overlaps", A=[0,100],  B=[20,50])  # B inside A -> during, not overlaps? inert
w("overlaps_eqstart","overlaps", A=[30,100], B=[30,50])  # B.start==A.start (starts) -> inert?

# ============ overlappedby: A.start<B.start<A.end<B.end ============
w("overlappedby_fire", "overlappedby", A=[0,50], B=[30,100])

# ============ during: A.start<B.start && B.end<A.end (B strictly inside A) ==
w("during_fire",    "during", A=[0,100], B=[20,80])
w("during_eqstart", "during", A=[0,100], B=[0,80])    # B.start==A.start -> inert (starts)?
w("during_eqend",   "during", A=[0,100], B=[20,100])  # B.end==A.end -> inert (finishes)?
w("during_asincl",  "during", A=[0,100], B=[0,100])   # identical -> inert?

# ============ includes: B.start<A.start && A.end<B.end (A inside B) ========
w("includes_fire",  "includes", A=[20,80], B=[0,100])

# direction cross-checks: during-config with includes op and vice versa
w("xdir_duringcfg_includes", "includes", A=[0,100], B=[20,80])  # should be inert
w("xdir_includescfg_during", "during",   A=[20,80], B=[0,100])  # should be inert

# ============ starts: B.start==A.start && B.end<A.end ============
w("starts_fire",    "starts", A=[10,90], B=[10,50])
w("starts_ns_off",  "starts", A=[10,90], B=[11,50])   # start!=  -> inert
w("starts_eqend",   "starts", A=[10,90], B=[10,90])   # B.end==A.end (coincides) -> inert?
w("starts_endgt",   "starts", A=[10,50], B=[10,90])   # B.end>A.end (startedby) -> inert?

# ============ startedby: B.start==A.start && B.end>A.end ============
w("startedby_fire", "startedby", A=[10,50], B=[10,90])

# ============ finishes: B.end==A.end && B.start>A.start ============
w("finishes_fire",  "finishes", A=[10,90], B=[50,90])
w("finishes_ne_off","finishes", A=[10,90], B=[50,89])  # end!= -> inert
w("finishes_eqstart","finishes", A=[10,90], B=[10,90]) # B.start==A.start (coincides) -> inert?

# ============ finishedby: B.end==A.end && B.start<A.start ============
w("finishedby_fire","finishedby", A=[50,90], B=[10,90])

print("wrote", len(os.listdir(OUT)), "allen-full scenarios")
