#!/usr/bin/env python3
"""D-164: the Allen @expires-inference REACH-VALUE ladder — predictions first.

Hypothesis (from the MVEL evaluators' getInterval(), to be verified cell by
cell against the oracle): every Allen op carries a PARAM-BLIND constant
interval on the anchor->self distance —

    coincides/starts/startedby [0,0]   meets/overlappedby/finishes [0,MAX]
    metby/overlaps/includes/finishedby [MIN,0]          during [1,MAX]

fed into the D-109 STP matrix (edge anchor->self = H iff H<MAX; edge
self->anchor = -L iff L>MIN; reach = row-max of the closure, >=0 finite,
else NEVER), with the deadline = endTS + reach + 1 (the certified
after/before arithmetic). Predicted per-op reach:

    op            reach(A=$a)  reach(B=this)
    coincides         0             0
    starts            0             0
    startedby         0             0
    meets           NEVER           0
    metby             0           NEVER
    overlaps          0           NEVER
    overlappedby    NEVER           0
    during          NEVER         NEVER
    includes          0           NEVER
    finishes        NEVER           0
    finishedby        0           NEVER

Probes: finite cells get present-at(dur+reach) + gone-at(dur+reach+1);
NEVER cells get present-at(100000). Param variants must behave EXACTLY
like the bare op (param-blind intervals — Drools ignores dev/min/max in
getInterval). after[3ms,9ms] rides as a positive control (reach A=9, B
leak lo>0 => NEVER). Compositions verify the closure sums. The manifest
(name -> predicted present/gone) drives check_allen_ladder.py.
"""
import json, os, sys

OUT = os.environ.get("ALLEN_TMP", "/home/bryan/.claude/jobs/577ad61a/tmp/allen_ladder")
os.makedirs(OUT, exist_ok=True)

REACH = {  # op -> (reach_A, reach_B); None = NEVER
    "coincides": (0, 0), "starts": (0, 0), "startedby": (0, 0),
    "meets": (None, 0), "metby": (0, None),
    "overlaps": (0, None), "overlappedby": (None, 0),
    "during": (None, None), "includes": (0, None),
    "finishes": (None, 0), "finishedby": (0, None),
}
PARAM_FORMS = {  # param variants must match the BARE op's reach exactly
    "coincides[7ms]": REACH["coincides"], "starts[7ms]": REACH["starts"],
    "meets[7ms]": REACH["meets"], "metby[7ms]": REACH["metby"],
    "overlaps[5ms]": REACH["overlaps"], "during[2ms,10ms]": REACH["during"],
    "finishes[7ms]": REACH["finishes"],
    "after[3ms,9ms]": (9, None),  # control: param-FED (lo>0 => B leaks)
}

def typ(name):
    return {"name": name,
            "fields": [{"name": "ts", "type": "i64"}, {"name": "dur", "type": "i64"}],
            "event": {"timestamp": "ts", "duration": "dur"}}

manifest = {}

def w(name, op, keep, dur, adv, present):
    s = {"name": name, "types": [typ("A"), typ("B")],
         "drl": f"rule R when $a : A() $b : B(this {op} $a) then end\n",
         "facts": [{"type": keep, "fields": {"ts": 0, "dur": dur}}],
         "epochs": [{"actions": [{"op": "advance", "ms": adv}], "facts": []}]}
    json.dump(s, open(f"{OUT}/{name}.json", "w"), indent=1)
    manifest[name] = {"type": keep, "present": present}

def cells(op, tag, reach_a, reach_b):
    for keep, reach in (("A", reach_a), ("B", reach_b)):
        for dur in (0, 50):
            base = f"al_{tag}_{keep}_d{dur}"
            if reach is None:
                w(f"{base}_far", op, keep, dur, 100000, True)
            else:
                dl = dur + reach + 1          # endTS + reach + 1
                w(f"{base}_last", op, keep, dur, dl - 1, True)
                w(f"{base}_gone", op, keep, dur, dl, False)

for op, (ra, rb) in REACH.items():
    cells(op, op, ra, rb)
for op, (ra, rb) in PARAM_FORMS.items():
    tag = op.replace("[", "_p").replace("]", "").replace(",", "_").replace("ms", "")
    cells(op, tag, ra, rb)

# --- closure compositions (multi-hop sums + the never-overwrite) ---
def w3(name, drl, ins, adv, expect):
    s = {"name": name, "types": [typ("A"), typ("B"), typ("C")],
         "drl": drl,
         "facts": [{"type": t, "fields": {"ts": 0, "dur": 0}} for t in ins],
         "epochs": [{"actions": [{"op": "advance", "ms": adv}], "facts": []}]}
    json.dump(s, open(f"{OUT}/{name}.json", "w"), indent=1)
    manifest[name] = {"type": ins[0], "present": expect}

# IN-RULE chain (one rule, 3 patterns): the per-rule matrix closes
# A->B (coincides, 0) + B->C (after, 100) => reach(A)=100: survive 100,
# die at 101.
CHAIN1 = ("rule R when $a : A() $b : B(this coincides $a) "
          "$c : C(this after[0ms,100ms] $b) then end\n")
w3("al_chain_inrule_last", CHAIN1, ["A"], 100, True)
w3("al_chain_inrule_gone", CHAIN1, ["A"], 101, False)
# CROSS-RULE chain: matrices are PER RULE (verified by the first ladder
# run: the summed prediction MISPREDICTED) — A's reach comes only from
# R1's coincides (0) => deadline 1: gone already at clock 1.
CHAIN2 = ("rule R1 when $a : A() $b : B(this coincides $a) then end\n"
          "rule R2 when $b : B() $c : C(this after[0ms,100ms] $b) then end\n")
w3("al_chain_crossrule_gone1", CHAIN2, ["A"], 1, False)
# mixed same-type never-overwrite: rule1 gives A finite 0 (coincides), rule2
# marks A NEVER (during anchor) — Drools' matrix OVERWRITES to NEVER.
MIX = ("rule R1 when $a : A() $b : B(this coincides $a) then end\n"
       "rule R2 when $a : A() $c : C(this during $a) then end\n")
w3("al_mix_never", MIX, ["A"], 100000, True)

json.dump(manifest, open(f"{OUT}/manifest.json", "w"), indent=1)
print(f"wrote {len(manifest)} ladder scenarios -> {OUT}")
