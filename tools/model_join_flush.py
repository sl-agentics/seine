#!/usr/bin/env python3
"""CEP temporal-join firing-ORDER reference model + population validator
(D-122/D-123). `simulate()` is the VALIDATED v2 model — the exact spec for the
engine port. The harness differs it against the gate oracle over the curated
battery and random shuffled-insertion populations.

  model_join_flush.py battery                          # curated 33-case check (self-contained)
  model_join_flush.py fuzz    <n> <seed>               # single-anchor population
  model_join_flush.py fuzzm   <n> <seed>               # MULTI-anchor population (facet-4)

--- v2 (VALIDATED: 0 divergences on 33 curated + ~4300 random cases) ---
The faithful phreak per-propagation flush. Disciplines (drools-core 9.44
sources; ARBITER = oracle):
* node memory (TupleList.add) APPENDS; scan the OPPOSITE memory FORWARD
  (getFirst + it.next).
* emissions -> child's staged-left set via addInsert = PREPEND; the child's
  doLeftInserts reads it in getInsertFirst order (= prepend order) and APPENDS
  to its own memory.
* NET: one emit (eager individual insert) is identity; a BATCH of N emits
  (anchor draining N held partners) is reversed EXACTLY ONCE. That single-vs-
  batch reversal is the whole game — and is what v1 (below) got wrong.

--- v1 DEAD-END (do not reintroduce) ---
v1 "every beta insert scans the opposite memory in REVERSE, appends emissions"
fit all 33 curated cases but diverged ~27% on the population (every failure = a
deeper partner HELD). The curated battery is a TRAP; shuffled insertion is the
honest bar. Kept only as this cautionary note.

--- PORT (engine.rs): reproduce the cascade, THEN re-cert ---
`make diff` (944) byte-identical + `fuzz_chain.py` vs a HEAD worktree = 0
regressions. Faithfulness bar: ZERO new Drools-divergences.
"""
import json, os, sys, re


def parse_windows(drl):
    return [(m.group(1), int(m.group(2)), int(m.group(3)))
            for m in re.finditer(r'(after|before)\[(\d+)ms,\s*(\d+)ms\]', drl)]


def win_match(op, lo, hi, anchor_ts, partner_ts):
    d = partner_ts - anchor_ts if op == "after" else anchor_ts - partner_ts
    return lo <= d <= hi


class Node:
    def __init__(self, op, lo, hi):
        self.op, self.lo, self.hi = op, lo, hi
        self.ltm, self.rtm, self.child, self.firings = [], [], None, []

    # --- v2 (D-122): faithful phreak per-propagation flush ---------------
    # Disciplines from drools-core 9.44 source (arbiter = oracle):
    #  * node memory (TupleList.add) APPENDS; scan the opposite memory
    #    FORWARD (getFirst + it.next).
    #  * emissions go to the child's staged-left set via addInsert = PREPEND.
    #  * the child's doLeftInserts reads that staged set in getInsertFirst
    #    order (= prepend order) and APPENDS to its own memory.
    #  Net: a SINGLE emit (eager individual insert) is identity; a BATCH of
    #  N emits (anchor draining N held partners) is reversed exactly ONCE.
    #  doNode order is doRightInserts before doLeftInserts, but per external
    #  fact only one side is staged, so the entry side alone matters.

    def _emit_set(self, staged_left):
        """doLeftInserts at THIS node over `staged_left` (getInsertFirst
        order): append each to ltm, scan rtm forward, prepend matches into
        this node's trg set, then propagate trg to the child (or fire)."""
        trg = []
        for lt in staged_left:
            self.ltm.append(lt)
            for rt in self.rtm:                     # forward scan
                if win_match(self.op, self.lo, self.hi, lt[-1], rt):
                    trg.insert(0, lt + [rt])        # addInsert PREPEND
        self._propagate(trg)

    def _propagate(self, trg):
        if self.child is None:
            self.firings.extend(trg)                # fire in getInsertFirst order
        else:
            self.child._emit_set(trg)

    def right_insert(self, rt):
        """doRightInserts: append to rtm, scan ltm forward, prepend matches
        into trg, propagate."""
        self.rtm.append(rt)
        trg = []
        for lt in self.ltm:                         # forward scan
            if win_match(self.op, self.lo, self.hi, lt[-1], rt):
                trg.insert(0, lt + [rt])            # addInsert PREPEND
        self._propagate(trg)

    def left_insert(self, lt):
        self._emit_set([lt])


def simulate(scenario):
    wins = parse_windows(scenario["drl"])
    nodes = [Node(*w) for w in wins]
    for i in range(len(nodes) - 1):
        nodes[i].child = nodes[i + 1]
    for f in scenario["facts"]:
        lvl, ts = int(f["type"][1:]), f["fields"]["ts"]
        if lvl == 0:
            nodes[0].left_insert([ts])
        else:
            nodes[lvl - 1].right_insert(ts)
    return ["-".join(map(str, t)) for t in nodes[-1].firings]


def battery():
    """Self-contained: generate the 33-case A-F battery via cep_join_battery,
    run the gate oracle LIVE, compare to the model. No external fixture."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import cep_join_battery
    OUT = "/tmp/model_join_battery"
    cep_join_battery.gen(OUT)
    paths = [os.path.join(OUT, fn) for fn in sorted(os.listdir(OUT))
             if fn.endswith(".json") and fn != "_manifest.json"]
    ora = _gate_oracle(paths)
    ok = bad = 0
    for p in paths:
        name = os.path.basename(p)[:-5]
        pred = simulate(json.load(open(p)))
        if pred == ora.get(name):
            ok += 1
        else:
            bad += 1
            print(f"  MISMATCH {name}\n    model : {pred}\n    oracle: {ora.get(name)}")
    print(f"MODEL vs GATE ORACLE: {ok} ok / {bad} MISMATCH")


EXP = 100000


def _etype(n):
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": EXP}}


def _gen_multi(rng, name):
    """MULTI-ANCHOR generator (facet-4 stress): 2-3 node chain, 1-3 E0
    anchors, 1-4 partners/level, random after/before windows, shuffled."""
    nnodes = rng.choice([2, 2, 3])
    ops = [(rng.choice(["after", "before"]), rng.choice([50, 100, 150, 200]))
           for _ in range(nnodes)]
    conj = ["$a0 : E0()"]
    for i in range(1, nnodes + 1):
        op, hi = ops[i - 1]
        conj.append(f"$a{i} : E{i}(this {op}[0ms,{hi}ms] $a{i-1})")
    facts = []
    e0_ts = rng.sample(range(0, 6), rng.choice([1, 2, 2, 3]))
    facts += [("E0", t) for t in e0_ts]
    base = min(e0_ts)
    for i in range(1, nnodes + 1):
        op, hi = ops[i - 1]
        used, tss = set(), []
        for _ in range(rng.randint(1, 4)):
            for _try in range(20):
                ts = base + rng.randint(0, hi) if op == "after" else base - rng.randint(0, hi)
                if ts not in used:
                    used.add(ts); tss.append(ts); break
        facts += [(f"E{i}", t) for t in tss]
        base = tss[0]
    rng.shuffle(facts)
    return {"name": name, "types": [_etype(f"E{i}") for i in range(nnodes + 1)],
            "drl": f"rule CH when {' '.join(conj)} then end\n",
            "facts": [{"type": t, "fields": {"ts": ts}} for t, ts in facts], "epochs": []}


def _gate_oracle(paths):
    import subprocess
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
                       cwd="/home/bryan/rust-rules", capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try:
            o = json.loads(line)
        except Exception:
            continue
        res = o.get("result")
        if not res:
            out[o["scenario"]] = None
            continue
        seq = []
        for fr in res["firings"]:
            d = {m["type"]: m["fields"]["ts"] for m in fr["matches"]}
            seq.append("-".join(str(d[f"E{i}"]) for i in sorted(int(k[1:]) for k in d)))
        out[o["scenario"]] = seq
    return out


def _fuzz(n, seed, gen_fn, tag):
    import random
    rng = random.Random(seed)
    OUT = "/tmp/model_join_fuzz"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 200, n)):
            scn = gen_fn(rng, f"{tag}{seed}x{i}")
            p = os.path.join(OUT, f"{tag}{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = _gate_oracle(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            if simulate(scns[nm]) != ora.get(nm):
                ndiff += 1
                diffs.append(nm)
        done += len(paths)
    print(f"{tag} model-vs-oracle: {n} cases seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")
    if diffs:
        print("  kept:", " ".join(diffs[:30]))


def fuzz(n, seed):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import fuzz_chain
    _fuzz(n, seed, lambda rng, nm: fuzz_chain.gen(rng, nm, rng.choice([2, 2, 3]))[0], "mjf")


def fuzzm(n, seed):
    _fuzz(n, seed, _gen_multi, "mjm")





# ======================================================================
# v3 (D-166 recon): UPDATE-RECENCY extension — the tj-tail family spec.
# Probe-pinned ingredients (u-ladder, D-165/D-166 recon):
#  * temporal MATCH uses the INSERT-time ts (the event handle is stamped
#    at insert; a ts-field update changes the printed value only).
#  * ROOT-pattern (leftmost) updates do NOT re-propagate (u2/m8).
#  * a non-root UPDATE moves the fact/tuple to the TAIL of its memory
#    and re-propagates child UPDATEs; the existing v2 prepend machinery
#    then yields the observed updated-first enumeration (u6).
#  * a tuple staged twice in one epoch fires ONCE, keeping its first
#    staging position (u5).
# Two disciplines are NOT hand-derivable and are grid-searched over the
# population (fuzzu grid): the staging order of an update batch into a
# beta child vs into the terminal, and right-update child iteration.
#   model_join_flush.py fuzzu <n> <seed> [UPD_BETA UPD_TERM RUPD_ORDER]
# ======================================================================

# D-168: the D-166-validated grid winner is RUPD_ORDER=oppmem (opposite-
# memory scan; DECISIONS D-166 "the alternatives die at 52-81%") — the
# file was committed with the losing childlist default (79% on mju42).
CFG = {"UPD_BETA": "prepend", "UPD_TERM": "prepend", "RUPD_ORDER": "oppmem"}


class UNode:
    def __init__(self, op, lo, hi):
        self.op, self.lo, self.hi = op, lo, hi
        self.ltm, self.rtm, self.child, self.node_id = [], [], None, None
        self.children = {}       # (id(lt), rt-id) -> child tuple (list)
        self.childlist = {}      # right fact id -> [child tuples] creation-PREPEND
        self.term_firings = []   # terminal only (rendered strings)
        self.term_buffer = []    # tuples awaiting epoch-end rendering
        self.term_pending = None # per-epoch dedup set (terminal only)

    def _match(self, lt, rt):
        return win_match(self.op, self.lo, self.hi, lt[-1]["ts0"], rt["ts0"])

    # --- staging into the child (or terminal) --------------------------
    def _forward(self, trg):
        """trg = list of ('ins'|'upd', tuple) in final processing order."""
        if self.child is None:
            for kind, t in trg:
                key = tuple(x["id"] for x in t)
                if self.term_pending is not None and key in self.term_pending:
                    continue                      # u5: double-touch fires once
                if self.term_pending is not None:
                    self.term_pending.add(key)
                self.term_buffer.append(t)
        else:
            self.child._process(trg)

    def _stage(self, ins, upd):
        """Combine an ins batch (v2: PREPEND = reversed) and an upd batch
        (grid: prepend|append) preserving Drools stage-set semantics."""
        term = self.child is None
        mode = CFG["UPD_TERM"] if term else CFG["UPD_BETA"]
        ins_part = list(reversed(ins))            # addInsert prepend
        upd_part = list(reversed(upd)) if mode == "prepend" else list(upd)
        self._forward([('upd', t) for t in upd_part] + [('ins', t) for t in ins_part])

    # --- left side ------------------------------------------------------
    def _process(self, staged):
        ins_out, upd_out = [], []
        for kind, lt in staged:
            if kind == 'ins':
                self.ltm.append(lt)
                for rt in self.rtm:
                    if self._match(lt, rt):
                        ct = lt + [rt]
                        self.children[(id(lt), rt["id"])] = ct
                        self.childlist.setdefault(rt["id"], []).insert(0, ct)
                        ins_out.append(ct)
            else:                                  # left-tuple UPDATE
                if lt in self.ltm:
                    self.ltm.remove(lt)
                    self.ltm.append(lt)            # tail re-add
                for rt in self.rtm:
                    key = (id(lt), rt["id"])
                    if self._match(lt, rt) and key in self.children:
                        ct = self.children[key]
                        cl = self.childlist.get(rt["id"], [])
                        if ct in cl:               # re-add child to END
                            cl.remove(ct); cl.append(ct)
                        upd_out.append(ct)
        # ONE staged-batch flush (v2 discipline: the whole trg reverses once)
        self._stage(ins_out, upd_out)

    def left_insert(self, lt):
        self._process([('ins', lt)])

    def left_update(self, lt):
        self._process([('upd', lt)])

    # --- right side -----------------------------------------------------
    def right_insert(self, rt):
        self.rtm.append(rt)
        ins_out = []
        for lt in self.ltm:
            if self._match(lt, rt):
                ct = lt + [rt]
                self.children[(id(lt), rt["id"])] = ct
                self.childlist.setdefault(rt["id"], []).insert(0, ct)
                ins_out.append(ct)
        self._stage(ins_out, [])

    def right_update(self, rt):
        self.rtm.remove(rt)
        self.rtm.append(rt)                        # tail re-add
        if CFG["RUPD_ORDER"] == "childlist":
            kids = list(self.childlist.get(rt["id"], []))
        else:                                      # opposite-memory scan order
            kids = [self.children[(id(lt), rt["id"])] for lt in self.ltm
                    if (id(lt), rt["id"]) in self.children]
        for ct in kids:                            # re-add each child to END
            cl = self.childlist[rt["id"]]
            cl.remove(ct); cl.append(ct)
        self._stage([], kids)


def usimulate(scenario):
    wins = parse_windows(scenario["drl"])
    nodes = [UNode(*w) for w in wins]
    for i in range(len(nodes) - 1):
        nodes[i].child = nodes[i + 1]
    term = nodes[-1]
    facts, lt_of, nidx = {}, {}, 0

    def insert(fobj):
        nonlocal nidx
        lvl, ts = int(fobj["type"][1:]), fobj["fields"]["ts"]
        f = {"id": nidx, "ts": ts, "ts0": ts, "lvl": lvl}
        facts[nidx] = f
        nidx += 1
        if lvl == 0:
            lt = [f]
            lt_of[f["id"]] = lt
            nodes[0].left_insert(lt)
        else:
            nodes[lvl - 1].right_insert(f)

    def render():
        term.term_firings.extend("-".join(str(x["ts"]) for x in t)
                                 for t in term.term_buffer)
        term.term_buffer = []

    def epoch_body(actions, efacts):
        term.term_pending = set()
        for act in actions:
            if act["op"] != "update":
                continue
            f = facts[act["target"]]
            if "ts" in act["fields"]:
                f["ts"] = act["fields"]["ts"]      # printed value only
            if f["lvl"] == 0:
                pass                               # root updates: no re-propagation
            else:
                nodes[f["lvl"] - 1].right_update(f)
        for fobj in efacts:
            insert(fobj)
        term.term_pending = None
        render()

    term.term_pending = set()
    for fobj in scenario["facts"]:
        insert(fobj)
    term.term_pending = None
    render()
    for ep in scenario.get("epochs", []):
        epoch_body(ep.get("actions", []), ep.get("facts", []))
    return list(term.term_firings)



def _gen_upd(rng, name):
    """Update-heavy population: the _gen_multi shape + 1-3 epochs of
    UPDATE actions over the initial facts (half ts-preserving churn,
    half printed-ts re-draws — matching is insert-stamped either way)
    + fresh arrivals AFTER updates (the enumeration observation point).
    No advances; EXP keeps everything alive."""
    scn = _gen_multi(rng, name)
    nnodes = len(parse_windows(scn["drl"]))
    used = {t["name"]: set() for t in scn["types"]}
    for fobj in scn["facts"]:
        used[fobj["type"]].add(fobj["fields"]["ts"])
    lo_ts = min(f["fields"]["ts"] for f in scn["facts"])
    hi_ts = max(f["fields"]["ts"] for f in scn["facts"])

    def fresh_ts(t):
        for _ in range(50):
            ts = rng.randint(lo_ts - 60, hi_ts + 60)
            if ts not in used[t]:
                used[t].add(ts)
                return ts
        return hi_ts + len(used[t]) + 1

    epochs = []
    for _ in range(rng.randint(1, 3)):
        actions = []
        for _ in range(rng.randint(1, 2)):
            idx = rng.randrange(len(scn["facts"]))
            t = scn["facts"][idx]["type"]
            if rng.random() < 0.5:
                ts = scn["facts"][idx]["fields"]["ts"]
            else:
                ts = fresh_ts(t)
            actions.append({"op": "update", "target": idx, "fields": {"ts": ts}})
        efacts = []
        for _ in range(rng.randint(0, 2)):
            lvl = rng.randrange(nnodes + 1)
            efacts.append({"type": f"E{lvl}", "fields": {"ts": fresh_ts(f"E{lvl}")}})
        epochs.append({"actions": actions, "facts": efacts})
    scn["epochs"] = epochs
    return scn


def fuzzu(n, seed):
    _orig = globals()["simulate"]
    globals()["simulate"] = usimulate
    try:
        _fuzz(n, seed, _gen_upd, "mju")
    finally:
        globals()["simulate"] = _orig


if __name__ == "__main__":
    if sys.argv[1] == "battery":
        battery()
    elif sys.argv[1] == "fuzz":
        fuzz(int(sys.argv[2]), int(sys.argv[3]))
    elif sys.argv[1] == "fuzzm":
        fuzzm(int(sys.argv[2]), int(sys.argv[3]))
    elif sys.argv[1] == "fuzzu":
        # optional grid overrides (docstring interface): fuzzu n seed [UPD_BETA UPD_TERM RUPD_ORDER]
        for key, val in zip(("UPD_BETA", "UPD_TERM", "RUPD_ORDER"), sys.argv[4:7]):
            CFG[key] = val
        fuzzu(int(sys.argv[2]), int(sys.argv[3]))
