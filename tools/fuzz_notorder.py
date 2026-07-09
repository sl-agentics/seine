#!/usr/bin/env python3
"""CEP item #2 — population capture for the non-temporal not-unblock STAGING
order model. Generates random CLEAN-regime `not D() P()` scenarios (one plain
blocker D deleted at the end to unblock; N uniquely-valued P facts inserted /
updated across batches while blocked), and records the ORACLE firing order.

model_check_notorder.py validates a simulator against this population (0-div is
the port gate, D-125 methodology: validate on SHUFFLED populations, not a
curated battery). Usage:  fuzz_notorder.py <n> <seed>  ->  writes
<tmp>/notpop_<seed>.json = [{"scenario":..., "order":[v...]}].
"""
import json, os, sys, random, subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TMP = os.environ.get("NOTPOP_TMP", "/tmp/seine_notpop") + "/notpop"
os.makedirs(TMP, exist_ok=True)
# BLOCKER: "plain" (D) or "event" (E0, @expires). TRIGGER: "delete" or "expiry"
# (expiry = advance past the blocker's deadline; requires an event blocker).
BLOCKER = os.environ.get("BLOCKER", "event")
TRIGGER = os.environ.get("TRIGGER", "expiry")
EVENT = BLOCKER == "event"
BT = "E0" if EVENT else "D"
BTYPE = ({"name": "E0", "fields": [{"name": "ts", "type": "i64"}], "event": {"timestamp": "ts", "expires_ms": 100}}
         if EVENT else {"name": "D", "fields": [{"name": "k", "type": "i64"}]})
TYPES = [BTYPE, {"name": "P", "fields": [{"name": "v", "type": "i64"}]}]
DRL = f"rule NE when not {BT}() P() then end\n"
BFACT = {"type": BT, "fields": ({"ts": 0} if EVENT else {"k": 0})}
UNBLOCK = ({"op": "advance", "ms": 300} if TRIGGER == "expiry" else {"op": "delete", "target": 0})

def gen(r, name):
    """A clean-regime scenario: blocker D present throughout (deleted only in the
    final epoch = single unblock). N uniquely-valued P facts inserted / updated
    across batches while blocked. `target` indices are GLOBAL insertion indices
    (runner: initial facts, then each epoch's facts in insertion order)."""
    facts = [dict(BFACT)]   # blocker at global idx 0
    gidx = 1
    vpos = {}          # v -> global insertion index (only once inserted)
    nextv = [1]
    def add_initial():
        v = nextv[0]; nextv[0] += 1
        facts.append({"type": "P", "fields": {"v": v}})
        nonlocal gidx
        vpos[v] = gidx; gidx += 1
    for _ in range(r.randint(1, 3)):
        add_initial()
    n_epochs = r.randint(1, 3)
    epochs = []
    for _ep in range(n_epochs):
        actions, efacts = [], []
        # updates run FIRST (actions before facts) — target P's that already
        # exist (inserted in a PRIOR batch), each keyed to its global index.
        existing = list(vpos.keys()); r.shuffle(existing)
        for pv in existing:
            if r.random() < 0.3:
                actions.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
        new_this = []
        for _ in range(r.randint(0, 3)):
            v = nextv[0]; nextv[0] += 1
            efacts.append({"type": "P", "fields": {"v": v}}); new_this.append(v)
        epochs.append({"actions": actions, "facts": efacts})
        # assign global indices AFTER the epoch's facts insert (so they become
        # updatable only from the NEXT epoch on)
        for v in new_this:
            vpos[v] = gidx; gidx += 1
    epochs.append({"actions": [dict(UNBLOCK)], "facts": []})
    return {"name": name, "types": TYPES, "drl": DRL, "facts": facts, "epochs": epochs}

def order_of(result):
    return [next((m["fields"]["v"] for m in f["matches"] if m["type"] == "P"), None)
            for f in result["firings"]]

def main():
    n = int(sys.argv[1]); seed = int(sys.argv[2])
    r = random.Random(seed)
    made = []
    for i in range(n):
        s = gen(r, f"np{seed}x{i}")
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
    pop = [{"scenario": s, "order": byname[s["name"]]} for _, s in made if s["name"] in byname]
    outp = os.path.join(os.path.dirname(TMP), f"notpop_{seed}.json")
    json.dump(pop, open(outp, "w"))
    print(f"captured {len(pop)} scenarios -> {outp}")

if __name__ == "__main__":
    main()
