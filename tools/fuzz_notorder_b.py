#!/usr/bin/env python3
"""CEP item-1b Family B — population capture for the EVENT-not EXPIRY-with-UPDATE
firing-order regime (the D-140 clean model breaks when a P is UPDATED in the
UNBLOCK epoch). Same clean single-unblock shape as fuzz_notorder.py, but the
final (unblock) epoch also carries P UPDATES, interleaved with the UNBLOCK
advance at a random position — the axis fuzz_notorder never exercised.

D-146: the initial-fact BLOCKER POSITION is now RANDOM among the initial P's
(0-3 P's before it, 0-2 after), so one population spans all three regimes:
P-FIRST (D-143's shape), MIXED (post-blocker epoch-0 initials — the D-145
`xf_cep_not_order_mixed_initial` corner, incl. epoch-0-initials updated across
an ARRIVAL), and BLOCKER-FIRST **with arrivals** (D-140's population had none).

model_check_notorder_b.py validates a simulator against this population (0-div is
the port gate; MODEL=seg2 is the D-146 unified rule). Usage:
fuzz_notorder_b.py <n> <seed>  ->  writes
<tmp>/notpop_b_<seed>.json = [{"scenario":..., "order":[v...]}].
"""
import json, os, sys, random, subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TMP = os.environ.get("NOTPOPB_TMP", "/tmp/seine_notpopb") + "/notpopb"
os.makedirs(TMP, exist_ok=True)
# EVENT blocker E0 (@expires), EXPIRY unblock (advance past the deadline).
BTYPE = {"name": "E0", "fields": [{"name": "ts", "type": "i64"}],
         "event": {"timestamp": "ts", "expires_ms": 100}}
TYPES = [BTYPE, {"name": "P", "fields": [{"name": "v", "type": "i64"}]}]
DRL = "rule NE when not E0() P() then end\n"
BFACT = {"type": "E0", "fields": {"ts": 0}}
UNBLOCK = {"op": "advance", "ms": 300}  # clock 0 -> 300, past E0's ts+100(+1)


def gen(r, name):
    """Blocker E0 present at clock 0 throughout (expires only at the final
    advance = single unblock). N uniquely-valued P facts inserted / updated
    across batches while blocked; the FINAL epoch ALSO updates prior P's, with
    the UNBLOCK advance interleaved at a random position among those updates."""
    # D-146 MIXED initial positions: n_before P's, the blocker, n_after P's.
    # DEFAULT n_before>=1 = the P-first/MIXED regimes (the D-143 seg model + the
    # D-145 initials-last tail) — the seg2 0-div gate. SEINE_NOTPOP_BF=1 allows
    # n_before==0 = BLOCKER-FIRST **with arrivals**, a regime D-140 never
    # validated (its population had none) and whose within-segment composition
    # is UNCRACKED (D-146 recon: d140/class/seg-d140 all ~26-47% divergent) —
    # kept out of the spec population until its own model arc.
    facts = []
    gidx = 0
    vpos = {}          # v -> global insertion index (only once inserted)
    nextv = [1]

    def add_initial():
        v = nextv[0]; nextv[0] += 1
        facts.append({"type": "P", "fields": {"v": v}})
        nonlocal gidx
        vpos[v] = gidx; gidx += 1
    if os.environ.get("SEINE_NOTPOP_BF_ONLY"):
        lo, hi = 0, 0      # PURE blocker-first (the D-149 recon population)
    elif os.environ.get("SEINE_NOTPOP_BF"):
        lo, hi = 0, 3
    else:
        lo, hi = 1, 3
    n_before = r.randint(lo, hi)
    n_after = r.randint(0, 2)
    if n_before + n_after == 0:
        if hi == 0:
            n_after = 1     # BF_ONLY: keep the blocker first
        else:
            n_before = 1
    for _ in range(n_before):
        add_initial()
    facts.append(dict(BFACT)); gidx += 1   # the blocker, mid-initials
    for _ in range(n_after):
        add_initial()
    n_epochs = r.randint(1, 3)
    epochs = []
    for _ep in range(n_epochs):
        actions, efacts = [], []
        existing = list(vpos.keys()); r.shuffle(existing)
        for pv in existing:
            if r.random() < 0.35:
                actions.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
        # non-final-epoch BLOCKER ARRIVALS (the Family-B trigger): an E0 event
        # arriving mid-run triggers a per-arrival stream-flush that re-stages the
        # P's. ts in [0,150] ⇒ alive at clock 0, expires by the final advance 300.
        # Interleaved with P inserts (facts order = arrival order).
        n_p = r.randint(0, 3)
        n_e = r.randint(0, 2)
        slots = ["e"] * n_e + ["p"] * n_p
        r.shuffle(slots)
        for s in slots:
            if s == "e":
                efacts.append({"type": "E0", "fields": {"ts": r.randint(0, 150)}})
            else:
                v = nextv[0]; nextv[0] += 1
                efacts.append({"type": "P", "fields": {"v": v}})
        epochs.append({"actions": actions, "facts": efacts})
        # global insertion indices follow FACTS order (E0 + P interleaved); P's
        # become updatable only from the NEXT epoch on (vpos recorded after).
        for fct in efacts:
            if fct["type"] == "P":
                vpos[fct["fields"]["v"]] = gidx
            gidx += 1
    # FINAL unblock epoch: prior-P UPDATES with the UNBLOCK interleaved at a
    # random spot (before / among / after the updates) — the Family-B axis.
    fexisting = list(vpos.keys()); r.shuffle(fexisting)
    fupds = [pv for pv in fexisting if r.random() < 0.5]
    at = r.randint(0, len(fupds))
    factions = []
    for i, pv in enumerate(fupds):
        if i == at:
            factions.append(dict(UNBLOCK))
        factions.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
    if at == len(fupds):
        factions.append(dict(UNBLOCK))
    epochs.append({"actions": factions, "facts": []})
    return {"name": name, "types": TYPES, "drl": DRL, "facts": facts, "epochs": epochs}


def order_of(result):
    return [next((m["fields"]["v"] for m in f["matches"] if m["type"] == "P"), None)
            for f in result["firings"]]


def main():
    n = int(sys.argv[1]); seed = int(sys.argv[2])
    r = random.Random(seed)
    made = []
    for i in range(n):
        s = gen(r, f"nb{seed}x{i}")
        p = os.path.join(TMP, s["name"] + ".json")
        json.dump(s, open(p, "w"), indent=1)
        made.append((p, s))
    files = [p for p, _ in made]
    out = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + files,
                         capture_output=True, text=True, cwd=ROOT)
    byname = {}
    for ln in out.stdout.splitlines():
        ln = ln.strip()
        if ln.startswith("{"):
            j = json.loads(ln); res = j.get("result")
            if res is not None:
                byname[j["scenario"]] = order_of(res)
    # keep only clean multi-P unblock batches (>=2 fired P's) — the orderable ones
    pop = [{"scenario": s, "order": byname[s["name"]]}
           for _, s in made if s["name"] in byname and len([v for v in byname[s["name"]] if v is not None]) >= 2]
    outp = os.path.join(os.path.dirname(TMP), f"notpop_b_{seed}.json")
    json.dump(pop, open(outp, "w"))
    print(f"captured {len(pop)} orderable scenarios (of {len(made)}) -> {outp}")


if __name__ == "__main__":
    main()
