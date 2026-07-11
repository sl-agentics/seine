#!/usr/bin/env python3
"""CEP item-1b Family B (exists) — `exists E1() P()` witness-toggle RE-FIRE order
population. Mirror of fuzz_notorder_b.py for the EXISTS witness: P's fire when the
witness E1 EXISTS (each satisfy transition re-fires the whole held memory).

Structure: initial P's + witness E1 (fire cycle 1 = all initial P's); then 1-2
TOGGLES, each = the witness LEAVES (delete or expiry-advance) + P's churn while
absent (inserts + updates) + the witness RE-ARRIVES (>=1 E1) => a re-fire. The
divergence is the re-fire ORDER (the D-140 EPOCH model). Captures the FULL firing
sequence; the port gate is engine-vs-oracle `diff` 0-fail on the emitted files
(model_check_exists.py, incl. the D-147 regime-2 rule). Usage: fuzz_existsorder.py <n> <seed> -> <tmp>/existspop_<seed>.json.

SEINE_EXPOP_FULL=1 (D-152): the FULL-AXIS generator for the mechanical-model
spec — free op soup instead of the toggle scaffold. Adds every axis the banked
populations lack: explicit P DELETES (staged annihilation / rtm removal /
queued-activation cancel), PARTIAL witness deletes (count 2->1), MULTI-witness
with staggered ts (partial expiry; deadline-order quiescence), DELAYED first
satisfaction (no initial witness — multi-epoch P backlog), DUE-ON-ARRIVAL
witnesses (ts already past deadline => transient satisfy + same-flush expiry),
pure-P epochs (no witness op — the boundary-drain axis), witness UPDATES
(inert — bare mask), action-interleaved inserts, and the D-133-corrected
expiration boundary (rare deep-negative ts => deadline < 0 = the DROOLS-455
leak, immortal; ts in [-101, clock-101] => nonneg-past deadline = due on
arrival). Never updates/deletes a dead handle (oracle NPE, D-151 note).
The spec gate: `EMODEL=flush model_check_exists.py <existspop_*.json>`.
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
        # D-147 (regime 2): P's inserted IN the satisfying epoch, interleaved
        # before/after the re-arrival witnesses — before-witness joins the
        # re-fire batch, after-witness fires fresh (cf407x121's NE6 shape).
        for _ in range(r.randint(0, 2)):
            v = nextv[0]; nextv[0] += 1
            rf.insert(r.randint(0, len(rf)), {"type": "P", "fields": {"v": v}})
        existing = list(vpos.keys()); r.shuffle(existing)
        for pv in existing:
            if r.random() < 0.3:
                actions2.append({"op": "update", "target": vpos[pv], "fields": {"v": pv}})
        epochs.append({"actions": actions2, "facts": rf})
        for fct in rf:
            if fct["type"] == "E1":
                live_e1.append(gidx)
            else:
                vpos[fct["fields"]["v"]] = gidx
            gidx += 1
    return {"name": name, "types": TYPES, "drl": DRL, "facts": facts, "epochs": epochs}


def gen_full(r, name):
    """The D-152 full-axis generator (SEINE_EXPOP_FULL=1): 2-5 epochs of free
    op soup over live-handle tracking. The generator mirrors the runners'
    clock/deadline arithmetic (deadline = ts + expires + 1) so it never
    touches a dead handle."""
    clock = [0]
    gidx = [0]
    nextv = [1]
    p_live = {}        # v -> handle idx (live P's)
    e_live = {}        # handle idx -> deadline (live E1s by the generator's books)
    facts = []

    def mk_p(into):
        v = nextv[0]; nextv[0] += 1
        into.append({"type": "P", "fields": {"v": v}})
        p_live[v] = gidx[0]; gidx[0] += 1

    def mk_e(into, initial=False):
        # ts around the clock; negative offsets make DUE-ON-ARRIVAL witnesses;
        # rare deep-negative ts make the DROOLS-455 leak (deadline < 0 ⇒
        # immortal, xq1) and nonneg-past deadlines (xq2/xq3)
        roll = r.random()
        if roll < 0.06:
            ts = r.randint(-400, -102)               # deadline < 0: the leak
        elif roll < 0.12:
            ts = r.randint(-101, max(-101, clock[0] - 101))  # nonneg past
        else:
            lo = 0 if initial else -130
            ts = max(0, clock[0] + r.randint(lo, 30))
        into.append({"type": "E1", "fields": {"ts": ts}})
        dl = ts + 100 + 1
        if dl < 0:
            e_live[gidx[0]] = float("inf")           # leaked: alive forever
        elif dl > clock[0]:
            e_live[gidx[0]] = dl
        gidx[0] += 1

    # initial facts: P's and (0-2) E1's interleaved — E1 ABSENT sometimes
    # (delayed first satisfaction), positions random
    slots = ["p"] * r.randint(0, 3) + ["e"] * r.choice([0, 0, 1, 1, 2])
    r.shuffle(slots)
    for s in slots:
        (mk_p if s == "p" else lambda i: mk_e(i, True))(facts)
    epochs = []
    for _ep in range(r.randint(2, 5)):
        actions = []
        n_ops = r.randint(0, 5)
        for _ in range(n_ops):
            op = r.choice(["upd", "upd", "pdel", "edel", "adv", "ins_p", "ins_e", "eupd"])
            if op == "upd" and p_live:
                v = r.choice(list(p_live))
                actions.append({"op": "update", "target": p_live[v], "fields": {"v": v}})
            elif op == "pdel" and len(p_live) > 1 and r.random() < 0.6:
                v = r.choice(list(p_live))
                actions.append({"op": "delete", "target": p_live.pop(v)})
            elif op == "edel" and e_live:
                h = r.choice(list(e_live))
                del e_live[h]
                actions.append({"op": "delete", "target": h})
            elif op == "adv":
                ms = r.randint(40, 250)
                clock[0] += ms
                for h in [h for h, dl in e_live.items() if dl <= clock[0]]:
                    del e_live[h]
                actions.append({"op": "advance", "ms": ms})
            elif op == "ins_p":
                mk_p(actions)
                actions[-1]["op"] = "insert"
            elif op == "ins_e":
                mk_e(actions)
                actions[-1]["op"] = "insert"
            elif op == "eupd" and e_live:
                h = r.choice(list(e_live))
                actions.append({"op": "update", "target": h,
                                "fields": {"ts": r.randint(0, clock[0] + 30)}})
        efacts = []
        slots = ["p"] * r.randint(0, 3) + ["e"] * r.choice([0, 0, 0, 1, 1, 2])
        r.shuffle(slots)
        for s in slots:
            (mk_p if s == "p" else mk_e)(efacts)
        epochs.append({"actions": actions, "facts": efacts})
    return {"name": name, "types": TYPES, "drl": DRL, "facts": facts, "epochs": epochs}


PTYPES = [
    {"name": "E1", "fields": [{"name": "ts", "type": "i64"},
                              {"name": "tag", "type": "String"}],
     "event": {"timestamp": "ts", "expires_ms": 100}},
    {"name": "P", "fields": [{"name": "v", "type": "i64"}]},
    {"name": "D", "fields": [{"name": "tag", "type": "String"}]},
]
PDRL_BARE = "rule R when exists D() P() then end\n"
PDRL_CONS = 'rule R when exists D(tag == "x") P() then end\n'
PDRL_J = ('rule J when $e : E1($t : tag) then insertLogical(new D($t)); end\n'
          "rule R when exists D() P() then end\n")


def gen_plain(r, name):
    """SEINE_EXPOP_PLAIN=1 (D-161): the PLAIN-witness family — `exists D()
    P()` with D a PLAIN type in a STREAM session (E1 declared keeps the
    session STREAM; inserted only under the TMS drive). Three rule drives:
      bare:    explicit D ins/del churn only (alpha can't exit by update);
      cons:    D(tag=="x") — enables the UPDATE-CHURN axis (out-and-back /
               exit-only / admit-only witness updates: the ex2 wedge family);
      logical: J: E1($t:tag) => insertLogical(new D($t)) with UNIQUE tags
               (bare R; D dies with its justifier — explicit E1 delete at
               position or tag-update churn; no advances ⇒ no expiry axis,
               plain witnesses never expire).
    gidx bookkeeping mirrors fuzz_notorder_b.gen_plain: logical D inserts
    consume nth_inserted slots at fire boundaries (unique tags ⇒ one J fire
    per E1 insert + one per tag-update); E1s are only touched from PRIOR
    epochs, at most once per epoch."""
    drive = r.choice(["bare", "cons", "cons", "logical"])
    gidx = [0]
    nextv = [1]
    nexttag = [1]
    p_live = {}     # v -> insertion idx
    d_live = {}     # idx -> current tag (explicit D's only)
    e1_live = {}    # idx -> born-epoch
    pend_j = [0]
    facts = []

    def mk_p(into):
        v = nextv[0]; nextv[0] += 1
        into.append({"type": "P", "fields": {"v": v}})
        p_live[v] = gidx[0]; gidx[0] += 1

    def mk_d(into, tag=None):
        t = tag if tag is not None else \
            (r.choice(["x", "z"]) if drive == "cons" else f"t{nexttag[0]}")
        if t.startswith("t"):
            nexttag[0] += 1
        into.append({"type": "D", "fields": {"tag": t}})
        d_live[gidx[0]] = t; gidx[0] += 1

    def mk_e1(into, epoch_no):
        tag = f"t{nexttag[0]}"; nexttag[0] += 1
        into.append({"type": "E1", "fields": {"ts": 0, "tag": tag}})
        e1_live[gidx[0]] = epoch_no
        gidx[0] += 1; pend_j[0] += 1

    slots = ["p"] * r.randint(1, 3)
    if drive != "logical" and r.random() < 0.55:
        slots += ["d"]
    if drive == "logical":
        slots += ["e"] * r.choice([0, 1, 1])
    r.shuffle(slots)
    for s in slots:
        {"p": mk_p, "d": mk_d, "e": lambda i: mk_e1(i, 0)}[s](facts)
    epochs = []
    for ep_no in range(1, r.randint(2, 5) + 1):
        gidx[0] += pend_j[0]; pend_j[0] = 0
        actions = []
        upd_this_ep = set()
        for _ in range(r.randint(0, 5)):
            op = r.choice(["insp", "delp", "updp", "insd", "deld", "updd",
                           "updd", "inse", "upde", "dele"])
            if op == "insp":
                mk_p(actions); actions[-1]["op"] = "insert"
            elif op == "delp" and len(p_live) > 1 and r.random() < 0.5:
                v = r.choice(list(p_live))
                actions.append({"op": "delete", "target": p_live.pop(v)})
            elif op == "updp" and p_live:
                v = r.choice(list(p_live))
                actions.append({"op": "update", "target": p_live[v],
                                "fields": {"v": v}})
            elif op == "insd" and drive != "logical" and r.random() < 0.6:
                mk_d(actions); actions[-1]["op"] = "insert"
            elif op == "deld" and d_live:
                h = r.choice(list(d_live)); del d_live[h]
                actions.append({"op": "delete", "target": h})
            elif op == "updd" and drive == "cons" and d_live:
                h = r.choice(list(d_live))
                t = r.choice(["x", "z"])
                d_live[h] = t
                actions.append({"op": "update", "target": h,
                                "fields": {"tag": t}})
            elif op == "inse" and drive == "logical":
                mk_e1(actions, ep_no); actions[-1]["op"] = "insert"
            elif op == "upde" and drive == "logical":
                ok = [h for h, born in e1_live.items()
                      if born < ep_no and h not in upd_this_ep]
                if ok:
                    h = r.choice(ok); upd_this_ep.add(h)
                    tag = f"t{nexttag[0]}"; nexttag[0] += 1
                    actions.append({"op": "update", "target": h,
                                    "fields": {"tag": tag}})
                    pend_j[0] += 1
            elif op == "dele" and drive == "logical":
                ok = [h for h, born in e1_live.items()
                      if born < ep_no and h not in upd_this_ep]
                if ok:
                    h = r.choice(ok); del e1_live[h]
                    actions.append({"op": "delete", "target": h})
        efacts = []
        for _ in range(r.randint(0, 3)):
            mk_p(efacts)
        if drive != "logical" and r.random() < 0.2:
            mk_d(efacts)
        epochs.append({"actions": actions, "facts": efacts})
    drl = {"bare": PDRL_BARE, "cons": PDRL_CONS, "logical": PDRL_J}[drive]
    return {"name": name, "types": PTYPES, "drl": drl,
            "facts": facts, "epochs": epochs}


def order_of(result):
    return [next((m["fields"]["v"] for m in f["matches"] if m["type"] == "P"), None)
            for f in result["firings"]]


def main():
    n = int(sys.argv[1]); seed = int(sys.argv[2])
    r = random.Random(seed)
    if os.environ.get("SEINE_EXPOP_PLAIN"):
        g = gen_plain
    elif os.environ.get("SEINE_EXPOP_FULL"):
        g = gen_full
    else:
        g = gen
    made = []
    for i in range(n):
        s = g(r, f"ex{seed}x{i}")
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
