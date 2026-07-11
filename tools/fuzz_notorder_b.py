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
the port gate; MODEL=flush is the D-150 mechanical spec). Usage:
fuzz_notorder_b.py <n> <seed>  ->  writes
<tmp>/notpop_b_<seed>.json = [{"scenario":..., "order":[v...]}].

SEINE_NOTPOP_FULL=1 (D-153): the FULL-AXIS generator — free op soup replacing
the single-unblock scaffold, mirroring the exists arc's SEINE_EXPOP_FULL
(D-152). Adds the axes the scaffold and the (uncommitted) D-151 delete
population lacked or lost: explicit E0 deletes at ANY position
(delete-unblock + SAME-EPOCH P inserts — the in_cycle-guard residual regime),
explicit P deletes (staged annihilation / rtm removal / queued-activation
cancel), DELAYED first blocker (P's fire unblocked first), multi-blocker
staggered ts (partial expiry), DUE-ON-ARRIVAL blockers (nonneg-past deadline
=> registers same flush — blocks transiently) and the DROOLS-455 leak
(negative deadline => immortal blocker, D-152 boundary), blocker UPDATES
(inert — bare mask), pure-P epochs, and action-interleaved inserts. Never
touches an explicitly-deleted handle; never updates an expired one (oracle
NPE, D-151 note).

SEINE_NOTPOP_PLAIN=1 (D-158 arc): the cf313x4 PLAIN-fact blocker family —
`not D() P()` with D a plain type in a STREAM session, driven explicitly
and/or logically (insertLogical from an @expires event justifier). See
gen_plain's docstring for the determinism constraints (unique tags; no
due-on-arrival justifiers; prior-epoch-only E1 touches).
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


def gen_full(r, name):
    """The D-153 full-axis generator (SEINE_NOTPOP_FULL=1): 2-5 epochs of free
    op soup over live-handle tracking, the not-family mirror of the exists
    arc's gen_full. The generator mirrors the runners' deadline arithmetic
    (deadline = ts + expires + 1; negative deadline = immortal) so it never
    touches a dead handle."""
    clock = [0]
    gidx = [0]
    nextv = [1]
    p_live = {}        # v -> handle idx (live P's)
    e_live = {}        # handle idx -> deadline (live E0s by the generator's books)
    facts = []

    def mk_p(into):
        v = nextv[0]; nextv[0] += 1
        into.append({"type": "P", "fields": {"v": v}})
        p_live[v] = gidx[0]; gidx[0] += 1

    def mk_e(into, initial=False):
        roll = r.random()
        if roll < 0.06:
            ts = r.randint(-400, -102)               # deadline < 0: the leak
        elif roll < 0.12:
            ts = r.randint(-101, max(-101, clock[0] - 101))  # nonneg past
        else:
            lo = 0 if initial else -130
            ts = max(0, clock[0] + r.randint(lo, 30))
        into.append({"type": "E0", "fields": {"ts": ts}})
        dl = ts + 100 + 1
        if dl < 0:
            e_live[gidx[0]] = float("inf")           # leaked: alive forever
        elif dl > clock[0]:
            e_live[gidx[0]] = dl
        gidx[0] += 1

    slots = ["p"] * r.randint(0, 3) + ["e"] * r.choice([0, 0, 1, 1, 2])
    r.shuffle(slots)
    for s in slots:
        (mk_p if s == "p" else lambda i: mk_e(i, True))(facts)
    epochs = []
    for _ep in range(r.randint(2, 5)):
        actions = []
        for _ in range(r.randint(0, 5)):
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
PDRL_J = ("rule J when $e : E1($t : tag) then insertLogical(new D($t)); end\n"
          "rule NE when not D() P() then end\n")
PDRL = "rule NE when not D() P() then end\n"


def gen_plain(r, name):
    """SEINE_NOTPOP_PLAIN=1 — the cf313x4 PLAIN-fact blocker family: `not D()
    P()` where D is a PLAIN type inside a STREAM session (event types declared
    keep the session STREAM either way). Two D drives, mixable per scenario:
      explicit: D via explicit action-inserts / deletes;
      logical (has_j): `J: E1($t:tag) => insertLogical(new D($t))` with UNIQUE
        tags (no shared justifications — the TMS envelope stays out of scope);
        a D dies with its justifier (expiry at quiescence / explicit E1 delete
        at position) or on tag-update churn (J re-fires: new-D ins BEFORE
        old-D retract, the bf_full graft order).
    gidx bookkeeping: logical D inserts consume nth_inserted indices; unique
    tags make the count DETERMINISTIC (one J fire per E1 insert + one per E1
    tag-update), added at each fire boundary. To keep it deterministic the
    generator (a) never inserts a due-on-arrival E1 (deadline always > clock
    at insert), (b) only tag-updates / deletes E1s from PRIOR epochs (a
    same-epoch touch would coalesce with the pending J match), (c) at most
    one tag-update per E1 per epoch."""
    has_j = r.random() < 0.6
    clock = [0]
    gidx = [0]
    nextv = [1]
    nexttag = [1]
    p_live = {}     # v -> insertion idx
    e1_live = {}    # idx -> {"dl": deadline, "epoch": born-epoch}
    d_live = {}     # idx -> True (explicit D's only)
    pend_j = [0]    # J fires pending at the next fire boundary
    facts = []

    def mk_p(into):
        v = nextv[0]; nextv[0] += 1
        into.append({"type": "P", "fields": {"v": v}})
        p_live[v] = gidx[0]; gidx[0] += 1

    def mk_e1(into, epoch_no):
        # deadline strictly future at insert: ts + 101 > clock
        ts = max(0, clock[0] - r.randint(0, 90)) if r.random() < 0.4 \
            else clock[0] + r.randint(0, 30)
        tag = f"t{nexttag[0]}"; nexttag[0] += 1
        into.append({"type": "E1", "fields": {"ts": ts, "tag": tag}})
        e1_live[gidx[0]] = {"dl": ts + 100 + 1, "epoch": epoch_no}
        gidx[0] += 1; pend_j[0] += 1

    def mk_d(into):
        tag = f"t{nexttag[0]}"; nexttag[0] += 1
        into.append({"type": "D", "fields": {"tag": tag}})
        d_live[gidx[0]] = True; gidx[0] += 1

    slots = ["p"] * r.randint(1, 3)
    if has_j:
        slots += ["e"] * r.choice([0, 1, 1, 2])
    if r.random() < 0.25:
        slots += ["d"]
    r.shuffle(slots)
    for s in slots:
        {"p": mk_p, "e": lambda i: mk_e1(i, 0), "d": mk_d}[s](facts)
    epochs = []
    n_ep = r.randint(2, 5)
    for ep_no in range(1, n_ep + 1):
        gidx[0] += pend_j[0]; pend_j[0] = 0   # prior boundary's J fires
        actions = []
        upd_this_ep = set()
        for _ in range(r.randint(0, 5)):
            op = r.choice(["updp", "updp", "delp", "insp", "inse", "upde",
                           "dele", "insd", "deld", "adv", "adv"])
            if op == "updp" and p_live:
                v = r.choice(list(p_live))
                actions.append({"op": "update", "target": p_live[v],
                                "fields": {"v": v}})
            elif op == "delp" and len(p_live) > 2 and r.random() < 0.5:
                v = r.choice(list(p_live))
                actions.append({"op": "delete", "target": p_live.pop(v)})
            elif op == "insp":
                mk_p(actions); actions[-1]["op"] = "insert"
            elif op == "inse" and has_j:
                mk_e1(actions, ep_no); actions[-1]["op"] = "insert"
            elif op == "upde" and has_j:
                ok = [h for h, m in e1_live.items()
                      if m["dl"] > clock[0] and m["epoch"] < ep_no
                      and h not in upd_this_ep]
                if ok:
                    h = r.choice(ok); upd_this_ep.add(h)
                    tag = f"t{nexttag[0]}"; nexttag[0] += 1
                    actions.append({"op": "update", "target": h,
                                    "fields": {"tag": tag}})
                    pend_j[0] += 1
            elif op == "dele" and has_j:
                ok = [h for h, m in e1_live.items()
                      if m["dl"] > clock[0] and m["epoch"] < ep_no
                      and h not in upd_this_ep]
                if ok:
                    h = r.choice(ok); del e1_live[h]
                    actions.append({"op": "delete", "target": h})
            elif op == "insd" and r.random() < 0.5:
                mk_d(actions); actions[-1]["op"] = "insert"
            elif op == "deld" and d_live:
                h = r.choice(list(d_live)); del d_live[h]
                actions.append({"op": "delete", "target": h})
            elif op == "adv":
                ms = r.randint(40, 250)
                clock[0] += ms
                for h in [h for h, m in e1_live.items() if m["dl"] <= clock[0]]:
                    del e1_live[h]
                actions.append({"op": "advance", "ms": ms})
        efacts = []
        eslots = ["p"] * r.randint(0, 3)
        if has_j:
            eslots += ["e"] * r.choice([0, 0, 1])
        if r.random() < 0.15:
            eslots += ["d"]
        r.shuffle(eslots)
        for s in eslots:
            {"p": mk_p, "e": lambda i: mk_e1(i, ep_no), "d": mk_d}[s](efacts)
        epochs.append({"actions": actions, "facts": efacts})
    return {"name": name, "types": PTYPES,
            "drl": PDRL_J if has_j else PDRL, "facts": facts, "epochs": epochs}


def order_of(result):
    return [next((m["fields"]["v"] for m in f["matches"] if m["type"] == "P"), None)
            for f in result["firings"]]


def main():
    n = int(sys.argv[1]); seed = int(sys.argv[2])
    r = random.Random(seed)
    if os.environ.get("SEINE_NOTPOP_PLAIN"):
        g = gen_plain
    elif os.environ.get("SEINE_NOTPOP_FULL"):
        g = gen_full
    else:
        g = gen
    made = []
    for i in range(n):
        s = g(r, f"nb{seed}x{i}")
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
    stem = "notpop_plain" if os.environ.get("SEINE_NOTPOP_PLAIN") else "notpop_b"
    outp = os.path.join(os.path.dirname(TMP), f"{stem}_{seed}.json")
    json.dump(pop, open(outp, "w"))
    print(f"captured {len(pop)} orderable scenarios (of {len(made)}) -> {outp}")


if __name__ == "__main__":
    main()
