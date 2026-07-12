#!/usr/bin/env python3
"""D-184 window:length model — the wl-ladder rules, executable.

Pinned semantics (D-183/D-184, all oracle 3x, e3 5x):
- POST-ALPHA ring: only alpha-passing (tag=="z") events occupy slots; ring is
  a FIFO of LIVE members, maxlen N; admission evicts the oldest RING slot.
- Slot order INSERT-FIXED (updates never re-admit/move); eviction+admission
  and multi-event epochs fold to ONE net fire (end-of-epoch value), fired by
  salience within the epoch.
- WATCH MASK = source BINDINGS only (D-139 analog): a mask write on a FOLD
  member re-fires (even at equal value); a mask write on an EVICTED live
  alpha-passing event REVIVES it into the fold OUTSIDE the ring (u2/u2b);
  no-mask writes do nothing (p1/p3). count() without bindings ⇒ empty mask.
- ALPHA TRANSITIONS act regardless of the mask (x1 entry admits like an
  insert; x2 exit drops from fold/ring). No backfill on delete (d1) — model
  assumes the same for alpha-exit (population-checked).
- EXPIRATION (referenced type: deadline = ts + expires + 1) drops the member
  from the fold AT the advance epoch and frees its ring slot (e1/e2/e3 —
  the coincident cell is 5x-STABLE; no D-112-style flip-flop for length).
- FIRING: one initial fire (value over the initial facts, 0/empty included),
  then one fire per epoch iff the fold was TOUCHED (membership change or a
  mask write on a member).
- N=0 throws in Drools (ArithmeticException) — out of subset; N >= 1 only.

fuzz mode: generates scenarios, runs the LIVE oracle, diffs the model.
Cases land in /tmp/model_winlen/. Usage: model_winlen.py fuzz <n> <seed>
"""
import json, os, random, subprocess, sys

EXPIRES = 100

# ---------------------------------------------------------------- simulate
def simulate(scn):
    n = scn["n"]; binding = scn["binding"]; fn = scn["fn"]
    ev = []            # per fact: dict(ts,v,tag,alive,expired,admitted_ever)
    ring = []          # FIFO of SLOTS: occupants may be corpse/exited/expired
    revived = set(); clock = 0
    fires = []

    detached = set()   # revival gate: only a MASK write re-admits
    active = set()     # EXPLICIT fold membership (the engine's active set)
    def passing(i):
        e = ev[i]
        return e["alive"] and not e["expired"] and e["tag"] == "z"
    def in_fold(i): return i in active
    def value():
        return sum(ev[i]["v"] for i in sorted(active)) if fn == "sum" \
            else len(active)

    def admit(i):
        # a slot appends on post-alpha INSERT or never-admitted alpha ENTRY;
        # overflow pops the OLDEST SLOT regardless of its occupant's fold
        # status, DETACHING the occupant (revivable only by a mask write)
        ev[i]["admitted_ever"] = True
        active.add(i)
        ring.append(i)
        if len(ring) > n:
            old = ring.pop(0)
            active.discard(old); detached.add(old)

    def insert(f):
        ev.append(dict(ts=f[0], v=f[1], tag=f[2], alive=True, expired=False,
                       admitted_ever=False))
        i = len(ev) - 1
        if passing(i):
            admit(i); return True
        return False

    def expire_due():
        t = False
        for i, e in enumerate(ev):
            if e["alive"] and not e["expired"] and e["ts"] + EXPIRES + 1 <= clock:
                if in_fold(i): t = True
                e["expired"] = True     # fold-drop; the SLOT is retained
                active.discard(i); revived.discard(i)
        return t

    def apply_action(a):
        nonlocal clock
        t = False
        if a[0] == "advance":
            clock += a[1]; t = expire_due()
        elif a[0] == "delete":
            i = a[1]
            if ev[i]["alive"]:
                if in_fold(i): t = True
                ev[i]["alive"] = False  # fold-drop; the SLOT is retained
                active.discard(i); revived.discard(i)
        return t

    for f in scn["facts"]: insert(f)
    fires.append(value())                          # the initial fire
    for epoch in scn["epochs"]:
        touched = False
        # D-160/x72: deferred external update entries evaluate against
        # EPOCH-FINAL fields — apply all writes first; a same-epoch
        # transient (exit-and-back) is invisible to the entries.
        for a in epoch["actions"]:
            if a[0] == "upd":
                ev[a[1]].update(a[2])
        for a in epoch["actions"]:
            if a[0] == "upd":
                i = a[1]; fields = a[2]
                pass_now = passing(i)
                was_in = i in active
                mask_hit = binding and "v" in fields
                if was_in and not pass_now:
                    active.discard(i); detached.add(i)   # exit: detach
                    revived.discard(i); touched = True
                elif was_in and pass_now:
                    if mask_hit: touched = True          # member re-fold
                elif not was_in and pass_now:
                    if not ev[i]["admitted_ever"]:
                        if "tag" in fields:
                            admit(i); touched = True     # x1 entry
                    elif i in detached and mask_hit:
                        detached.discard(i); active.add(i)
                        if i not in ring: revived.add(i)
                        touched = True                   # mask-gated revival
            else:
                touched |= apply_action(a)
        for f in epoch["facts"]:
            touched |= insert(f)
        if touched: fires.append(value())
    return fires

# ---------------------------------------------------------------- harness io
def to_harness(scn, name):
    types = [{"name": "E0",
              "fields": [{"name": "ts", "type": "i64"}, {"name": "v", "type": "i64"},
                         {"name": "tag", "type": "String"}],
              "event": {"timestamp": "ts", "expires_ms": EXPIRES}}]
    src = 'E0(tag == "z", $v : v)' if scn["binding"] else 'E0(tag == "z")'
    agg = "$s : sum($v)" if scn["fn"] == "sum" else "$c : count()"
    drl = f'rule W when accumulate( {src} over window:length({scn["n"]}); {agg} ) then end\n'
    def fact(f): return {"type": "E0", "fields": {"ts": f[0], "v": f[1], "tag": f[2]}}
    def act(a):
        if a[0] == "advance": return {"op": "advance", "ms": a[1]}
        if a[0] == "delete":  return {"op": "delete", "target": a[1]}
        return {"op": "update", "target": a[1], "fields": a[2]}
    return {"name": name, "types": types, "drl": drl,
            "facts": [fact(f) for f in scn["facts"]],
            "epochs": [{"actions": [act(a) for a in e["actions"]],
                        "facts": [fact(f) for f in e["facts"]]} for e in scn["epochs"]]}

def oracle_seq(paths):
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle", *paths],
        capture_output=True, text=True).stdout
    res = {}
    for line in out.splitlines():
        line = line.strip()
        if not line: continue
        d = json.loads(line)
        r = d.get("result")
        if r is None: res[d["scenario"]] = None; continue
        vals = []
        for fr in r["firings"]:
            for m in fr["matches"]:
                if "value" in m["fields"]: vals.append(m["fields"]["value"])
        res[d["scenario"]] = vals
    return res

# ---------------------------------------------------------------- generator
def gen(rng):
    n = rng.choice([1, 2, 2, 3])
    binding = rng.random() < 0.75
    fn = "sum" if binding else "count"
    nxt_ts = [0]
    clock_seen = [0]
    def ts():
        # FENCE (wl604x44): no born-expired inserts — the deadline must
        # clear the clock (the born-expired x window trickle is deferred
        # with the D-133 adjacency)
        floor = clock_seen[0] - EXPIRES + 30
        nxt_ts[0] = max(nxt_ts[0], floor)
        nxt_ts[0] += rng.randint(5, 40); return nxt_ts[0]
    def f(): return (ts(), rng.randint(1, 9), "z" if rng.random() < 0.8 else "y")
    facts = [f() for _ in range(rng.randint(0, 2))]
    all_facts = list(facts)
    live = list(range(len(facts)))
    clock = [0]
    epochs = []
    advance_used = [False]
    n_epochs = rng.randint(2, 3)
    adv_epoch = rng.randrange(n_epochs) if rng.random() < 0.4 else -1
    for eidx in range(n_epochs):
        acts = []
        if eidx == adv_epoch and not advance_used[0]:
            # FENCE: one advance per scenario, actions-only epoch (the
            # multi-deadline trickle corner is deferred — x31 witness)
            acts.append(("advance", rng.randint(20, 120)))
            clock[0] += acts[-1][1]
            clock_seen[0] = clock[0]
            advance_used[0] = True
            epochs.append({"actions": acts, "facts": []})
            continue
        for _ in range(rng.randint(0, 2)):
            kind = rng.choice(["upd_v", "upd_v", "upd_tag", "delete"])
            cands = [i for i in live
                     if all_facts[i][0] + EXPIRES + 1 > clock[0] + 130]
            if not cands: continue
            i = rng.choice(cands)
            if kind == "delete":
                acts.append(("delete", i)); live.remove(i)
            elif kind == "upd_v":
                acts.append(("upd", i, {"v": rng.randint(10, 99)}))
            else:
                acts.append(("upd", i, {"tag": rng.choice(["z", "y"])}))
        efacts = [f() for _ in range(rng.randint(0, 2))]
        base = len(all_facts)
        live.extend(range(base, base + len(efacts)))
        all_facts.extend(efacts)
        epochs.append({"actions": acts, "facts": efacts})
    return {"n": n, "binding": binding, "fn": fn, "facts": facts, "epochs": epochs}

def fuzz(count, seed):
    rng = random.Random(seed)
    OUT = "/tmp/model_winlen"; os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0; diffs = []
    while done < count:
        scns, paths = {}, []
        for i in range(done, min(done + 150, count)):
            s = gen(rng); name = f"wl{seed}x{i}"
            json.dump(to_harness(s, name), open(f"{OUT}/{name}.json", "w"))
            scns[name] = s; paths.append(f"{OUT}/{name}.json")
        ora = oracle_seq(paths)
        for name, s in scns.items():
            if simulate(s) != ora.get(name):
                ndiff += 1; diffs.append(name)
        done += len(paths)
    print(f"winlen model-vs-oracle: {count} seed {seed}: {ndiff} div")
    if diffs: print("  ", " ".join(diffs[:15]))

# ------------------------------------------------------- the ladder gate
LADDER = [
 ("s1", {"n":2,"binding":True,"fn":"sum","facts":[],
   "epochs":[{"actions":[],"facts":[(10,1,"z")]},{"actions":[],"facts":[(20,2,"z")]},
             {"actions":[],"facts":[(30,100,"y")]},{"actions":[],"facts":[(40,4,"z")]}]},
  [0,1,3,6]),
 ("t2", {"n":2,"binding":True,"fn":"sum","facts":[],
   "epochs":[{"actions":[],"facts":[(10,1,"z")]},{"actions":[],"facts":[(20,2,"z")]},
             {"actions":[],"facts":[(30,4,"z"),(40,8,"z")]}]},
  [0,1,3,12]),
 ("u1", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("upd",0,{"v":10})],"facts":[]},{"actions":[],"facts":[(30,3,"z")]}]},
  [3,12,5]),
 ("u2", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[],"facts":[(30,3,"z")]},{"actions":[("upd",0,{"v":50})],"facts":[]}]},
  [3,5,55]),
 ("u2b", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[],"facts":[(30,3,"z")]},{"actions":[("upd",0,{"v":50})],"facts":[]},
             {"actions":[],"facts":[(40,4,"z")]}]},
  [3,5,55,57]),
 ("d1", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[],"facts":[(30,3,"z")]},{"actions":[("delete",1)],"facts":[]}]},
  [3,5,3]),
 ("e1", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("advance",111)],"facts":[]},{"actions":[],"facts":[(30,3,"z")]}]},
  [3,2,5]),
 ("e2", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("advance",111)],"facts":[]},{"actions":[],"facts":[(30,3,"z"),(40,4,"z")]}]},
  [3,2,7]),
 ("e3", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("advance",111)],"facts":[(120,4,"z")]}]},
  [3,6]),
 ("p1", {"n":2,"binding":False,"fn":"count","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("upd",0,{"ts":11})],"facts":[]}]},
  [2]),
 ("p3", {"n":2,"binding":False,"fn":"count","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[],"facts":[(30,3,"z")]},{"actions":[("upd",0,{"v":50})],"facts":[]}]},
  [2,2]),
 ("b1", {"n":2,"binding":True,"fn":"sum","facts":[],
   "epochs":[{"actions":[],"facts":[(10,1,"z"),(20,2,"z"),(30,3,"z"),(40,4,"z")]}]},
  [0,7]),
 ("b2", {"n":1,"binding":True,"fn":"sum","facts":[],
   "epochs":[{"actions":[],"facts":[(10,1,"z")]},{"actions":[],"facts":[(20,2,"z")]},
             {"actions":[],"facts":[(30,3,"z")]}]},
  [0,1,2,3]),
 ("x1", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z")],
   "epochs":[{"actions":[],"facts":[(20,2,"y")]},
             {"actions":[("upd",1,{"tag":"z"})],"facts":[]}]},
  [1,3]),
 ("x2", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("upd",0,{"tag":"y"})],"facts":[]}]},
  [3,2]),
 # slot-retention discriminators (population peel, D-184; oracle from the run)
 ("sr1_del_slot", {"n":2,"binding":True,"fn":"sum","facts":[(20,3,"z"),(58,2,"z")],
   "epochs":[{"actions":[("upd",1,{"v":58}),("delete",1)],"facts":[(80,6,"y"),(100,9,"z")]},
             {"actions":[("upd",3,{"v":48})],"facts":[(140,5,"z")]}]},
  [5,9,53]),
 ("sr2_exit_slot", {"n":3,"binding":True,"fn":"sum","facts":[(5,3,"z"),(30,7,"z")],
   "epochs":[{"actions":[],"facts":[(54,4,"y")]},
             {"actions":[("delete",0),("upd",2,{"v":17})],"facts":[(82,4,"z")]},
             {"actions":[("upd",3,{"tag":"y"})],"facts":[(116,7,"z"),(125,2,"z")]}]},
  [10,11,9]),
 ("sr3_outfold_exit", {"n":1,"binding":False,"fn":"count","facts":[],
   "epochs":[{"actions":[("advance",35),("advance",30)],"facts":[(12,8,"z"),(32,1,"z")]},
             {"actions":[("upd",0,{"tag":"y"})],"facts":[]}]},
  [0,1]),
 ("x72_transient_exit", {"n":2,"binding":True,"fn":"sum","facts":[],
   "epochs":[{"actions":[],"facts":[(5,2,"z"),(30,7,"z")]},
             {"actions":[("upd",1,{"tag":"y"}),("upd",1,{"tag":"z"})],"facts":[(47,7,"z")]},
             {"actions":[("delete",1),("upd",2,{"tag":"z"})],"facts":[]}]},
  [0,9,14,7]),
 ("x3_reentry_lone", {"n":2,"binding":True,"fn":"sum","facts":[(10,1,"z"),(20,2,"z")],
   "epochs":[{"actions":[("upd",0,{"tag":"y"})],"facts":[]},
             {"actions":[("upd",0,{"tag":"z"})],"facts":[]},
             {"actions":[],"facts":[(30,5,"z")]}]},
  [3,2,7]),
 ("sr4_reentry_slot", {"n":2,"binding":True,"fn":"sum","facts":[(21,4,"z")],
   "epochs":[{"actions":[],"facts":[(30,1,"z"),(58,5,"z")]},
             {"actions":[("upd",1,{"tag":"z"})],"facts":[(69,4,"z")]},
             {"actions":[("upd",2,{"tag":"y"}),("upd",2,{"tag":"z"})],"facts":[(103,5,"z")]}]},
  [4,6,9,9]),
]

def ladder():
    bad = 0
    for name, scn, want in LADDER:
        got = simulate(scn)
        ok = got == want
        bad += not ok
        print(f"  {'OK ' if ok else 'DIV'} {name}: model {got} vs pinned {want}")
    print(f"ladder: {len(LADDER)-bad}/{len(LADDER)}")
    return bad == 0

if __name__ == "__main__":
    if sys.argv[1] == "ladder":
        sys.exit(0 if ladder() else 1)
    elif sys.argv[1] == "fuzz":
        fuzz(int(sys.argv[2]), int(sys.argv[3]))
