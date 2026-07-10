#!/usr/bin/env python3
"""CEP item-1b Family B — population capture for the EVENT-not EXPIRY-with-UPDATE
firing-order regime (the D-140 clean model breaks when a P is UPDATED in the
UNBLOCK epoch). Same clean single-unblock shape as fuzz_notorder.py, but the
final (unblock) epoch also carries P UPDATES, interleaved with the UNBLOCK
advance at a random position — the axis fuzz_notorder never exercised.

model_check_notorder_b.py validates a simulator against this population (0-div is
the port gate). Usage:  fuzz_notorder_b.py <n> <seed>  ->  writes
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
    # P-FIRST regime (the divergent one — the real witnesses cf401x362/notB_min
    # insert P before the blocker; a blocker inserted BEFORE a P promotes an
    # unblock-epoch update, AFTER a P does not — the key Family-B discriminator).
    # D-140's fuzz_notorder put the blocker at idx0 (blocker-first), so it only
    # ever saw the easy regime. Here: initial P's first, then the blocker.
    facts = []
    gidx = 0
    vpos = {}          # v -> global insertion index (only once inserted)
    nextv = [1]

    def add_initial():
        v = nextv[0]; nextv[0] += 1
        facts.append({"type": "P", "fields": {"v": v}})
        nonlocal gidx
        vpos[v] = gidx; gidx += 1
    for _ in range(r.randint(1, 3)):
        add_initial()
    facts.append(dict(BFACT)); gidx += 1   # blocker AFTER the initial P's
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
