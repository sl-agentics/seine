#!/usr/bin/env python3
"""Two-node temporal-chain battery: generate scenarios and compare
engine vs oracle firing ORDER.

Chain rule: $a:E0() $b:E1(this after[0,50] $a) $c:E2(this after[0,100] $b)
  node1 = E0(left) joins E1(right)   -> tuple (E0,E1)
  node2 = (E0,E1)(left) joins E2(right)

We vary insertion order / multiplicity to probe how batches reverse
through the two nodes. Distinguishing field = each fact's ts.

Usage:
  battery.py gen  <dir>
  battery.py cmp  <engine.ndjson> <oracle.ndjson>
"""
import json
import sys
import os

EXP = 100000
CHAIN_DRL = ("rule CH when $a : E0() $b : E1(this after[0ms,50ms] $a) "
             "$c : E2(this after[0ms,100ms] $b) then end\n")


def etype(name):
    return {"name": name, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": EXP}}


def scenario(name, drl, facts, types=("E0", "E1", "E2")):
    return {
        "name": name,
        "types": [etype(t) for t in types],
        "drl": drl,
        "facts": [{"type": t, "fields": {"ts": ts}} for (t, ts) in facts],
        "epochs": [],
    }


def perms(xs):
    if len(xs) <= 1:
        yield list(xs)
        return
    for i, x in enumerate(xs):
        for rest in perms(xs[:i] + xs[i + 1:]):
            yield [x] + rest


def gen(outdir):
    os.makedirs(outdir, exist_ok=True)
    cases = []

    def add(name, facts):
        cases.append(scenario(name, CHAIN_DRL, facts))

    # ---- Group A: e0last core + E1-insertion-order permutations ----
    # E0@1 inserted after the E1 batch, single E2@110 last.
    for i, p in enumerate(perms([23, 25, 26])):
        facts = [("E1", t) for t in p] + [("E0", 1), ("E2", 110)]
        add(f"A_e1perm_{'_'.join(map(str, p))}", facts)
    # wider spread
    for i, p in enumerate(perms([10, 30, 50])):
        facts = [("E1", t) for t in p] + [("E0", 1), ("E2", 110)]
        add(f"A_wide_{'_'.join(map(str, p))}", facts)

    # ---- Group B: node1 batch size (E0 last, E2 last) ----
    for p in perms([20, 40]):
        add(f"B_two_{'_'.join(map(str, p))}",
            [("E1", t) for t in p] + [("E0", 1), ("E2", 110)])
    for p in [[10, 20, 30, 40], [40, 30, 20, 10], [20, 40, 10, 30]]:
        add(f"B_four_{'_'.join(map(str, p))}",
            [("E1", t) for t in p] + [("E0", 1), ("E2", 130)])

    # ---- Group C: E0 position (first / middle / last) ----
    add("C_e0first", [("E0", 1), ("E1", 26), ("E1", 23), ("E1", 25), ("E2", 110)])
    add("C_e0mid1", [("E1", 26), ("E0", 1), ("E1", 23), ("E1", 25), ("E2", 110)])
    add("C_e0mid2", [("E1", 26), ("E1", 23), ("E0", 1), ("E1", 25), ("E2", 110)])
    add("C_e0last", [("E1", 26), ("E1", 23), ("E1", 25), ("E0", 1), ("E2", 110)])

    # ---- Group D: node2 right batch (multiple E2) ----
    # single E1, several E2 joining one left (pure node2 multi-right)
    for p in perms([60, 70, 80]):
        add(f"D_e2perm_{'_'.join(map(str, p))}",
            [("E1", 30), ("E0", 1)] + [("E2", t) for t in p])
    # 3 E1 batch AND 2 E2
    add("D_3x2", [("E1", 26), ("E1", 23), ("E1", 25), ("E0", 1),
                  ("E2", 110), ("E2", 120)])
    add("D_3x2b", [("E1", 26), ("E1", 23), ("E1", 25), ("E0", 1),
                   ("E2", 120), ("E2", 110)])

    # ---- Group E: multiple E0 (node1 left batch) ----
    add("E_2e0_last", [("E1", 26), ("E1", 23), ("E1", 25),
                       ("E0", 1), ("E0", 2), ("E2", 110)])
    add("E_2e0_first", [("E0", 1), ("E0", 2), ("E1", 26), ("E1", 23),
                        ("E1", 25), ("E2", 110)])
    add("E_2e0_2e1", [("E1", 40), ("E1", 20), ("E0", 1), ("E0", 2), ("E2", 110)])

    # ---- Group F: single E1 sanity (no batch at node1) ----
    add("F_1e1", [("E1", 26), ("E0", 1), ("E2", 110)])

    manifest = {}
    for c in cases:
        path = os.path.join(outdir, c["name"] + ".json")
        with open(path, "w") as f:
            json.dump(c, f)
        # record E1 insertion order for interpretation
        e1s = [ts for (t, ts) in
               [(x["type"], x["fields"]["ts"]) for x in c["facts"]] if t == "E1"]
        manifest[c["name"]] = e1s
    with open(os.path.join(outdir, "_manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print(f"wrote {len(cases)} scenarios to {outdir}")


def load_ndjson(path):
    out = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            name = obj.get("scenario")
            res = obj.get("result")
            if res is None:
                out[name] = None  # error
            else:
                out[name] = res.get("firings", [])
    return out


def firing_seq(firings):
    """Render each firing as a compact string of ts by type; keep order."""
    seq = []
    for fr in firings:
        parts = []
        for m in fr["matches"]:
            parts.append(f'{m["type"]}{m["fields"]["ts"]}')
        seq.append("+".join(sorted(parts)))
    return seq


def cmp_(engine_path, oracle_path):
    eng = load_ndjson(engine_path)
    ora = load_ndjson(oracle_path)
    names = sorted(set(eng) | set(ora))
    ndiff = 0
    for n in names:
        e = eng.get(n)
        o = ora.get(n)
        es = firing_seq(e) if e else None
        os_ = firing_seq(o) if o else None
        tag = "OK   " if es == os_ else "DIFF "
        if es != os_:
            ndiff += 1
        # compact: show E1 order (the distinguishing field for chains)
        def e1only(seq):
            if seq is None:
                return None
            out = []
            for s in seq:
                for tok in s.split("+"):
                    if tok.startswith("E1"):
                        out.append(tok[2:])
            return out
        print(f"{tag}{n}")
        print(f"      eng E1: {e1only(es)}")
        print(f"      ora E1: {e1only(os_)}")
        if es != os_:
            print(f"      eng full: {es}")
            print(f"      ora full: {os_}")
    print(f"\n{ndiff} DIFFER / {len(names)} total")


if __name__ == "__main__":
    if sys.argv[1] == "gen":
        gen(sys.argv[2])
    elif sys.argv[1] == "cmp":
        cmp_(sys.argv[2], sys.argv[3])
