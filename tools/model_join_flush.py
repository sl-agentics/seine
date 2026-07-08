#!/usr/bin/env python3
"""CEP temporal-join firing-ORDER reference model + population validator
(D-122). The MODEL here is a DISPROVEN v1 kept as a documented dead-end;
the reusable asset is the `validate` / `fuzz` harness that differs any model
against the gate oracle over the curated battery and a random chain
population. Build v2 by editing `simulate()`, then re-run `fuzz` — the bar is
ZERO divergences (the faithfulness bar).

  model_join_flush.py battery  <dir> <gate_oracle.txt>   # curated 33-case check
  model_join_flush.py fuzz     <n> <seed>                # population differ vs oracle

--- WHAT GROUND TRUTH SAYS (faithful AccDump, STREAM+pseudo-clock) ---
Invariant across all 33 curated cases: firing = reverse(node2 left-memory).
node1's right-memory is IDENTICAL for e0first vs e0last ([26,23,25]); what
differs is node2's left-memory order — set by node1's EMISSION provenance,
i.e. fact-handle recency + whether a partner was held (arrived before its
anchor, drained as a reversed batch) or eager (arrived after, appended).

--- WHY v1 IS DISPROVEN ---
v1 rule "every beta insert scans the opposite memory in REVERSE, appends
emissions" reproduces all 33 curated cases (all partner-last) but DIVERGES on
~27% of the random shuffled-insertion population — every failure has a DEEPER
partner HELD (E2 arriving before its E0/E1 context). The curated battery is a
trap: 33/33 masks a 27% population gap. This is the D-121 "family of
interdependent facets" made concrete.

--- SOURCE DISCIPLINES for v2 (drools-core 9.44 sources, ARBITER = oracle) ---
* TupleSetsImpl.addInsert  -> PREPENDS (insertFirst = new; LIFO staged list).
* TupleList.add (node mem) -> APPENDS (this.last = new; FIFO memory).
* PhreakJoinNode.doLeftInserts:  ltm.add(lt); iterate rtm via getFirstRightTuple
  + it.next; each match -> insertChildLeftTuple -> trgLeftTuples.addInsert.
* PhreakJoinNode.doRightInserts: rtm.add(rt); iterate ltm via getFirstLeftTuple
  + it.next; each match -> addInsert.
v2 must model the OTN/LIA staging into node1's src sets + the segment flush
order (doNode) + the memory iterator direction (recency), NOT a scan-direction
knob. Validate to 0 divergences on `fuzz` BEFORE porting to engine.rs.
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
        self.left, self.right, self.child, self.firings = [], [], None, []

    def left_insert(self, t):
        self.left.append(t)
        for p in reversed(self.right):
            if win_match(self.op, self.lo, self.hi, t[-1], p):
                self._emit(t + [p])

    def right_insert(self, p):
        self.right.append(p)
        for t in reversed(self.left):
            if win_match(self.op, self.lo, self.hi, t[-1], p):
                self._emit(t + [p])

    def _emit(self, t):
        if self.child:
            self.child.left_insert(t)
        else:
            self.firings.append(t)


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


def load_oracle_tbl(path):
    tbl = {}
    for line in open(path):
        line = line.rstrip()
        if not line:
            continue
        name = line.split()[0]
        rest = line[len(name):].strip()
        tbl[name] = eval(rest) if rest != "ERR" else "ERR"
    return tbl


def battery(dir_, oracle_txt):
    tbl = load_oracle_tbl(oracle_txt)
    ok = bad = 0
    for fn in sorted(os.listdir(dir_)):
        if not fn.endswith(".json") or fn == "_manifest.json":
            continue
        name = fn[:-5]
        pred = simulate(json.load(open(os.path.join(dir_, fn))))
        if pred == tbl.get(name):
            ok += 1
        else:
            bad += 1
            print(f"  MISMATCH {name}\n    model : {pred}\n    oracle: {tbl.get(name)}")
    print(f"MODEL vs GATE ORACLE: {ok} ok / {bad} MISMATCH")


def fuzz(n, seed):
    import random, subprocess
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import fuzz_chain
    REPO = "/home/bryan/rust-rules"
    OUT = "/tmp/model_join_fuzz"
    os.makedirs(OUT, exist_ok=True)

    def gate(paths):
        env = dict(os.environ)
        env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
        r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
                           cwd=REPO, capture_output=True, text=True, env=env)
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

    rng = random.Random(seed)
    done = ndiff = 0
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 200, n)):
            scn, _, _ = fuzz_chain.gen(rng, f"mjf{seed}x{i}", rng.choice([2, 2, 3]))
            p = os.path.join(OUT, f"mjf{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = gate(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            if simulate(scns[nm]) != ora.get(nm):
                ndiff += 1
        done += len(paths)
    print(f"model-vs-oracle: {n} cases seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")


if __name__ == "__main__":
    if sys.argv[1] == "battery":
        battery(sys.argv[2], sys.argv[3])
    elif sys.argv[1] == "fuzz":
        fuzz(int(sys.argv[2]), int(sys.argv[3]))
