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


if __name__ == "__main__":
    if sys.argv[1] == "battery":
        battery()
    elif sys.argv[1] == "fuzz":
        fuzz(int(sys.argv[2]), int(sys.argv[3]))
    elif sys.argv[1] == "fuzzm":
        fuzzm(int(sys.argv[2]), int(sys.argv[3]))
