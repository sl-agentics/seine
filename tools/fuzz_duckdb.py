#!/usr/bin/env python3
"""D-097 phase 3: null-rich differential fuzz against the DuckDB
oracle. Draws INSERT-ONLY scenarios with inert RHS over the phase-1
null surface (nullable fields ~30% null density; surface null tests;
null in-list members; groups; existentials; accumulates) and compares
per-rule match sets via tools/diff_duckdb.py machinery.

Engine-axis exclusions (deliberate — this oracle owns DATA-TYPE
semantics only, D-097 ruling 5): no epochs/actions/TMS/queries/
or-rules/salience (set comparison makes order moot); no i64-vs-f64
eq joins (D-020 truncation is a certified Drools-axis quirk); f64
values are multiples of 0.25 so float sums are exact under any
addition order; bools compare only ==/!=; no in-lists inside groups
(translator paren-depth limit).

Usage: .venv/bin/python tools/fuzz_duckdb.py <n> <seed>
"""
import json
import os
import random
import subprocess
import sys

sys.path.insert(0, os.path.dirname(__file__))
from diff_duckdb import duckdb_match_sets  # noqa: E402

BIN = "target/debug/seine-harness"
STRS = ["a", "b", "ab", "zz", "abc", "ba"]
RX = ['"a.*"', '".*b"', '"z.*"']


class Gen:
    def __init__(self, rng):
        self.r = rng
        self.types = []

    def draw_types(self):
        for ti in range(self.r.randint(2, 3)):
            fields = []
            for fi in range(self.r.randint(1, 4)):
                ft = self.r.choice(["i64", "i64", "f64", "String", "bool"])
                fields.append({
                    "name": f"f{fi}",
                    "type": ft,
                    **({"nullable": True} if self.r.random() < 0.55 else {}),
                })
            self.types.append({"name": f"T{ti}", "fields": fields})

    def lit(self, ft):
        r = self.r
        if ft == "i64":
            return str(r.randint(0, 4))
        if ft == "f64":
            return f"{r.randint(0, 12) * 0.25:.2f}"
        if ft == "String":
            return f'"{r.choice(STRS)}"'
        return r.choice(["true", "false"])

    def fact_val(self, f):
        if f.get("nullable") and self.r.random() < 0.3:
            return None
        ft = f["type"]
        if ft == "i64":
            return self.r.randint(0, 4)
        if ft == "f64":
            return self.r.randint(0, 12) * 0.25
        if ft == "String":
            return self.r.choice(STRS)
        return self.r.choice([True, False])

    def cmp_op(self, ft):
        if ft in ("bool",):
            return self.r.choice(["==", "!="])
        return self.r.choice(["==", "!=", "<", "<=", ">", ">="])

    def leaf(self, t, binds, allow_group_unsafe=True):
        """One constraint leaf on type t. binds: [(var, type, field)]."""
        r = self.r
        f = r.choice(t["fields"])
        ft = f["type"]
        kind = r.random()
        if f.get("nullable") and kind < 0.18:
            return f'{f["name"]} {r.choice(["==", "!="])} null'
        if kind < 0.45:
            return f'{f["name"]} {self.cmp_op(ft)} {self.lit(ft)}'
        if kind < 0.6 and allow_group_unsafe and ft in ("i64", "String"):
            items = [self.lit(ft) for _ in range(r.randint(2, 3))]
            if r.random() < 0.35:
                items.append("null")
            neg = "not in" if r.random() < 0.5 else "in"
            return f'{f["name"]} {neg} ({", ".join(items)})'
        if kind < 0.75 and ft == "String":
            if r.random() < 0.5:
                return f'{f["name"]} matches {r.choice(RX)}'
            return f'{f["name"]} contains "{r.choice(["a", "b", "z"])}"'
        # join to an earlier binding of the SAME field type (no i64/f64 mix)
        cands = [v for (v, vt, _) in binds if vt == ft]
        if cands:
            return f'{f["name"]} {self.cmp_op(ft)} ${r.choice(cands)}'
        return f'{f["name"]} {self.cmp_op(ft)} {self.lit(ft)}'

    def group(self, t, binds):
        r = self.r
        n = r.randint(2, 3)
        leaves = [self.leaf(t, binds, allow_group_unsafe=False) for _ in range(n)]
        op = r.choice([" || ", " && "])
        g = f'({op.join(leaves)})'
        if r.random() < 0.4:
            return f"!{g}"
        return g.strip("()") if op == " && " and r.random() < 0.5 else g

    def pattern(self, pi, binds, vcount):
        r = self.r
        t = r.choice(self.types)
        cons = []
        for _ in range(r.randint(0, 2)):
            if r.random() < 0.2:
                cons.append(self.group(t, binds))
            else:
                cons.append(self.leaf(t, binds))
        new_binds = []
        if r.random() < 0.6:
            f = r.choice(t["fields"])
            v = f"v{vcount}"
            cons.append(f"${v} : {f['name']}")
            new_binds.append((v, f["type"], f["name"]))
        return t, f'{t["name"]}({", ".join(cons)})', new_binds

    def acc_ce(self, binds, vcount):
        r = self.r
        t = r.choice(self.types)
        num = [f for f in t["fields"] if f["type"] in ("i64", "f64")]
        fn = r.choice(["sum", "count", "average", "min", "max"])
        if not num:
            fn = "count"
        cons = []
        for _ in range(r.randint(0, 1)):
            cons.append(self.leaf(t, binds, allow_group_unsafe=False))
        av = f"v{vcount}"
        rv = f"v{vcount + 1}"
        if fn == "count":
            f = r.choice(t["fields"])
        else:
            f = r.choice(num)
        cons.append(f"${av} : {f['name']}")
        return f'accumulate( {t["name"]}({", ".join(cons)}); ${rv} : {fn}(${av}) )'

    def rule(self, ri):
        r = self.r
        parts, binds, vcount = [], [], 0
        for pi in range(r.randint(1, 3)):
            kind = r.random()
            if pi > 0 and kind < 0.18:
                t = r.choice(self.types)
                cons = [self.leaf(t, binds) for _ in range(r.randint(0, 2))]
                parts.append(f'{r.choice(["not", "exists"])} {t["name"]}({", ".join(cons)})')
                continue
            if kind < 0.33:
                parts.append(self.acc_ce(binds, vcount))
                vcount += 2
                continue
            t, ptxt, nb = self.pattern(pi, binds, vcount)
            vcount += len(nb)
            binds.extend(nb)
            parts.append(ptxt)
        return f'rule R{ri} when {"  ".join(parts)} then end'

    def scenario(self, name):
        self.draw_types()
        rules = [self.rule(i) for i in range(self.r.randint(2, 4))]
        facts = []
        for _ in range(self.r.randint(4, 18)):
            t = self.r.choice(self.types)
            facts.append({
                "type": t["name"],
                "fields": {f["name"]: self.fact_val(f) for f in t["fields"]},
            })
        return {
            "name": name,
            "oracle": "duckdb",
            "types": self.types + [{"name": "Out", "fields": []}],
            "drl": "\n".join(rules) + "\n",
            "facts": facts,
            "epochs": [],
        }


def engine_sets(path):
    r = subprocess.run(
        [BIN, "run", path],
        capture_output=True, text=True, env={**os.environ, "SEINE_HANDLES": "1"},
    )
    if r.returncode != 0:
        raise RuntimeError(f"engine: {r.stderr[-300:]}")
    out = json.loads(r.stdout)["result"]
    h2idx = {}
    for i, fact in enumerate(out["facts"]):
        if "__h" in fact["fields"]:
            h2idx[fact["fields"]["__h"]] = i
    sets = {}
    for f in out["firings"]:
        row = []
        for m in f["matches"]:
            if m["type"] == "InitialFact":
                continue
            if m["type"] in ("Long", "Double"):
                row.append(("acc", round(float(m["fields"]["value"]), 9)))
            elif "__h" in m["fields"]:
                row.append(("h", h2idx[m["fields"]["__h"]]))
        sets.setdefault(f["rule"], []).append(tuple(sorted(row)))
    return {k: sorted(v) for k, v in sets.items()}


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    tmp = os.environ.get("FUZZ_TMP", "/tmp") + f"/dkfuzz_{seed}"
    os.makedirs(tmp, exist_ok=True)
    subprocess.run(["cargo", "build", "-q", "-p", "seine-harness"], check=True)
    div = comp_err = 0
    for i in range(n):
        rng = random.Random(seed * 1_000_003 + i)
        scn = Gen(rng).scenario(f"dkf_{seed}_{i}")
        path = f"{tmp}/case.json"
        json.dump(scn, open(path, "w"), indent=1)
        try:
            eng = engine_sets(path)
            ora = duckdb_match_sets(scn)
        except Exception as ex:
            comp_err += 1
            keep = f"{tmp}/err_{i}.json"
            os.rename(path, keep)
            print(f"[{i}] HARNESS-ERROR {ex} -> {keep}")
            if comp_err > 20:
                print("too many harness errors; aborting")
                sys.exit(2)
            continue
        keys = set(eng) | set(ora)
        bad = [k for k in keys if eng.get(k, []) != ora.get(k, [])]
        if bad:
            div += 1
            keep = f"{tmp}/div_{i}.json"
            os.rename(path, keep)
            print(f"[{i}] DIVERGENCE rules {bad} -> {keep}")
            for k in bad[:2]:
                print(f"    engine {eng.get(k, [])[:6]}")
                print(f"    duckdb {ora.get(k, [])[:6]}")
        if i % 200 == 199:
            print(f"  ...{i + 1}/{n} (div={div} err={comp_err})", flush=True)
    print(f"--- duckdb-fuzz complete: {n} cases, seed {seed}, {div} divergences, {comp_err} gen-rejects")
    sys.exit(1 if div else 0)


if __name__ == "__main__":
    main()
