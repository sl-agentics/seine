#!/usr/bin/env python3
"""CEP item-1b Family B (exists) — `exists E1() P()` witness-toggle RE-FIRE order
population. Mirror of fuzz_notorder_b.py for the EXISTS witness: P's fire when the
witness E1 EXISTS (each satisfy transition re-fires the whole held memory).

Structure: initial P's + witness E1 (fire cycle 1 = all initial P's); then 1-2
TOGGLES, each = the witness LEAVES (delete or expiry-advance) + P's churn while
absent (inserts + updates) + the witness RE-ARRIVES (>=1 E1) => a re-fire. The
divergence is the re-fire ORDER (the D-140 EPOCH model). Captures the FULL firing
sequence; the port gate is engine-vs-oracle `diff` 0-fail on the emitted files
(model_check_exists.py derives the order rule on the clean delete-single-toggle
subset). Usage: fuzz_existsorder.py <n> <seed> -> <tmp>/existspop_<seed>.json.
"""
import json, os, sys, random, subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TMP = os.environ.get("EXPOP_TMP", "/tmp/seine_expop") + "/expop"
os.makedirs(TMP, exist_ok=True)
WTYPE = {"name": "E1", "fields": [{"name": "ts", "type": "i64"}],
         "event": {"timestamp": "ts", "expires_ms": 100}}
TYPES = [WTYPE, {"name": "P", "fields": [{"name": "v", "type": "i64"}]}]
DRL = "rule R when exists E1() P() then end\n"


def gen(r, name):
    facts = []
    gidx = 0
    vpos = {}
    nextv = [1]

    def add_initial():
        v = nextv[0]; nextv[0] += 1
        facts.append({"type": "P", "fields": {"v": v}})
        nonlocal gidx
        vpos[v] = gidx; gidx += 1
    for _ in range(r.randint(1, 3)):
        add_initial()
    facts.append({"type": "E1", "fields": {"ts": 0}}); e1_idx = gidx; gidx += 1
    clock = [0]
    epochs = []
    n_toggle = r.randint(1, 2)
    live_e1 = [e1_idx]
    for _t in range(n_toggle):
        actions = []
        if r.random() < 0.5 and live_e1:
            for ei in live_e1:
                actions.append({"op": "delete", "target": ei})
            live_e1 = []
        else:
            clock[0] += 200
            actions.append({"op": "advance", "ms": 200})
            live_e1 = []
        existing = list(vpos.keys()); r.shuffle(existing)
        for pv in existing:
            if r.random() < 0.35:
                actions.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
        efacts = []
        for _ in range(r.randint(0, 2)):
            v = nextv[0]; nextv[0] += 1
            efacts.append({"type": "P", "fields": {"v": v}})
        epochs.append({"actions": actions, "facts": efacts})
        for fct in efacts:
            vpos[fct["fields"]["v"]] = gidx; gidx += 1
        actions2 = []
        rf = []
        for _ in range(r.randint(1, 2)):
            ts = clock[0] + r.randint(0, 20)
            rf.append({"type": "E1", "fields": {"ts": ts}})
        existing = list(vpos.keys()); r.shuffle(existing)
        for pv in existing:
            if r.random() < 0.3:
                actions2.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
        epochs.append({"actions": actions2, "facts": rf})
        for _ in rf:
            live_e1.append(gidx); gidx += 1
    return {"name": name, "types": TYPES, "drl": DRL, "facts": facts, "epochs": epochs}


def order_of(result):
    return [next((m["fields"]["v"] for m in f["matches"] if m["type"] == "P"), None)
            for f in result["firings"]]


def main():
    n = int(sys.argv[1]); seed = int(sys.argv[2])
    r = random.Random(seed)
    made = []
    for i in range(n):
        s = gen(r, f"ex{seed}x{i}")
        p = os.path.join(TMP, s["name"] + ".json")
        json.dump(s, open(p, "w"), indent=1); made.append((p, s))
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
    pop = [{"scenario": s, "firings": byname[s["name"]]}
           for _, s in made if s["name"] in byname
           and len([v for v in byname[s["name"]] if v is not None]) >= 2]
    outp = os.path.join(os.path.dirname(TMP), f"existspop_{seed}.json")
    json.dump(pop, open(outp, "w"))
    print(f"captured {len(pop)} of {len(made)} -> {outp}")


if __name__ == "__main__":
    main()
