#!/usr/bin/env python3
"""Triage the xfail quarantine: oracle determinism + divergence signatures.

For every scenario in scenarios/xfail/ this runs the engine once and the
oracle N times (each oracle invocation is a fresh JVM launch), then
classifies each witness:

  bucket             meaning
  ------             -------
  ORACLE-NONDET      Drools' own output varies across JVM launches
                     (order flips, or pass/fire-limit flips) — uncertifiable
                     by any differential harness (D-080)
  ORACLE-RUNAWAY     Drools hits the 100k fire limit in every run while the
                     engine terminates (CE-only self-justify family, D-080)
  ORDER-ONLY         oracle stable; engine differs only in firing order
  VALUE              oracle stable; engine differs in firing multiset or
                     final facts
  PASS               oracle stable and engine matches — graduation candidate
  ENGINE-ERROR/...   error asymmetries, reported verbatim

plus D-078/D-080 fence-shape markers from the scenario text:

  A   a justifier RHS mixes insertLogical with mutation (set/update/modify/delete)
  B   stated insert of the logical type (rule / initial fact / epoch fact)
  RD  a rule deletes a fact bound to the logical type
  SJ  CE-only self-justifier (LHS sees the logical type only via not/exists)
  XU/XD  external update/delete targets a logical-type fact

A VALUE witness with NO fence marker would be a divergence inside the
certified envelope — a pin candidate the triage must flag loudly.

Usage: python3 tools/triage_xfail.py [--runs N] [--cache DIR] [--md out.md]
Comparison follows D-003: facts = multiset, firings ordered, matches within
a firing = multiset, f64 = bit equality, query rows ordered, identifiers set.
"""
import argparse, json, os, re, struct, subprocess, sys
from collections import Counter, defaultdict

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HARNESS = ["cargo", "run", "-q", "-p", "seine-harness", "--"]


def canon_val(v):
    if isinstance(v, float):
        return ("f64", struct.pack(">d", v).hex())
    if isinstance(v, bool):
        return ("bool", v)
    if isinstance(v, int):
        return ("i64", v)
    return v


def canon_fact(f):
    return json.dumps({"type": f["type"],
                       "fields": {k: canon_val(v) for k, v in sorted(f["fields"].items())}},
                      sort_keys=True)


def canon_result(r):
    facts = sorted(canon_fact(f) for f in r.get("facts", []))
    firings = [(fi["rule"], tuple(sorted(canon_fact(m) for m in fi.get("matches", []))))
               for fi in r.get("firings", [])]
    queries = []
    for q in r.get("queries", []) or []:
        rows = [json.dumps({k: canon_fact(v) if isinstance(v, dict) and "type" in v else canon_val(v)
                            for k, v in sorted(row.items())}, sort_keys=True)
                for row in q.get("rows", [])]
        queries.append((q.get("call"), json.dumps([canon_val(a) for a in q.get("args", [])]),
                        frozenset(q.get("identifiers", [])), tuple(rows)))
    return {"facts": facts, "firings": firings, "queries": queries}


def diff_kind(a, b):
    """a = oracle canonical, b = engine canonical -> (kind, human detail)."""
    diffs = []
    if a["facts"] != b["facts"]:
        ca, cb = Counter(a["facts"]), Counter(b["facts"])
        diffs.append(("facts", f"facts -{sum((ca - cb).values())}/+{sum((cb - ca).values())}"))
    fa, fb = a["firings"], b["firings"]
    if fa != fb:
        if Counter(fa) == Counter(fb):
            first = next(i for i, (x, y) in enumerate(zip(fa, fb)) if x != y)
            diffs.append(("firing-order", f"{len(fa)} firings, first swap @{first}"))
        else:
            ca, cb = Counter(fa), Counter(fb)
            diffs.append(("firing-set",
                          f"firings {len(fa)} vs {len(fb)} (-{sum((ca - cb).values())}/+{sum((cb - ca).values())})"))
    if a["queries"] != b["queries"]:
        qa, qb = a["queries"], b["queries"]
        same_sets = len(qa) == len(qb) and all(
            x[0] == y[0] and x[1] == y[1] and x[2] == y[2] and Counter(x[3]) == Counter(y[3])
            for x, y in zip(qa, qb))
        diffs.append(("query-row-order" if same_sets else "query-rows", "query rows"))
    if not diffs:
        return ("equal", "")
    kinds = {k for k, _ in diffs}
    if kinds <= {"firing-order", "query-row-order"}:
        return ("ORDER-ONLY", "; ".join(d for _, d in diffs))
    return ("VALUE", "; ".join(d for _, d in diffs))


def entry_state(e):
    if "error" in e:
        err = str(e["error"])
        return ("FIRE-LIMIT", None) if "fire limit" in err else ("ERROR:" + err[:60], None)
    return ("OK", canon_result(e["result"]))


def split_rules(drl):
    return [p for p in re.split(r"(?=rule\s)", drl) if p.strip()]


def shape_markers(d):
    drl = d.get("drl", "")
    logical = sorted(set(re.findall(r"insertLogical\(new (\w+)", drl)))
    L = set(logical)
    markers = set()
    for rule in split_rules(drl):
        m = re.search(r"\bwhen\b(.*?)\bthen\b(.*)$", rule, re.S)
        if not m:
            continue
        lhs, rhs = m.group(1), m.group(2)
        il_types = set(re.findall(r"insertLogical\(new (\w+)", rhs))
        if il_types:
            if re.search(r"\b(update|modify|delete)\s*\(|\.set[A-Z]", rhs):
                markers.add("A")
            for lt in il_types:
                neg = re.findall(r"(?:not|exists)\s+(?:\$\w+\s*:\s*)?%s\s*\(" % lt, lhs)
                allocc = re.findall(r"%s\s*\(" % lt, lhs)
                if neg and len(neg) == len(allocc):
                    markers.add("SJ")
        for lt in L:
            if re.search(r"\binsert\(new %s\b" % lt, rhs):
                markers.add("B")
        for var, ty in re.findall(r"\$(\w+)\s*:\s*(\w+)\s*\(", lhs):
            if ty in L and re.search(r"delete\(\s*\$%s\s*\)" % var, rhs):
                markers.add("RD")
    vis = [f["type"] for f in d.get("facts", [])]
    if any(t in L for t in vis):
        markers.add("B")
    for ep in d.get("epochs", []) or []:
        for f in ep.get("facts", []):
            if f["type"] in L:
                markers.add("B")
    vis = [f["type"] for f in d.get("facts", [])]
    for ep in d.get("epochs", []) or []:
        for a in ep.get("actions", []) or []:
            t = a.get("target")
            if t is not None and t < len(vis) and vis[t] in L:
                markers.add("XU" if a["op"] == "update" else "XD")
        for f in ep.get("facts", []):
            vis.append(f["type"])
    return logical, sorted(markers)


def load_ndjson(path):
    m = {}
    with open(path) as fh:
        for line in fh:
            j = json.loads(line)
            m[j["scenario"]] = j
    return m


def ensure_runs(files, cache, runs):
    os.makedirs(cache, exist_ok=True)
    eng_path = os.path.join(cache, "engine.ndjson")
    if not os.path.exists(eng_path):
        with open(eng_path, "w") as fh:
            subprocess.run(HARNESS + ["run"] + files, stdout=fh, cwd=REPO, check=True)
    procs = []
    for i in range(1, runs + 1):
        p = os.path.join(cache, f"oracle_r{i}.ndjson")
        if not os.path.exists(p):
            fh = open(p, "w")
            procs.append((subprocess.Popen(HARNESS + ["oracle"] + files, stdout=fh,
                                           stderr=subprocess.DEVNULL, cwd=REPO), fh))
    for p, fh in procs:
        p.wait()
        fh.close()
        if p.returncode != 0:
            sys.exit("oracle replicate failed")
    return eng_path, [os.path.join(cache, f"oracle_r{i}.ndjson") for i in range(1, runs + 1)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--cache", default=os.path.join(REPO, "target", "triage_cache"))
    ap.add_argument("--dir", default=os.path.join(REPO, "scenarios", "xfail"))
    ap.add_argument("--md", default=None, help="write a markdown table here")
    args = ap.parse_args()

    files = sorted(os.path.join(args.dir, f) for f in os.listdir(args.dir) if f.endswith(".json"))
    eng_path, orc_paths = ensure_runs(files, args.cache, args.runs)
    eng = load_ndjson(eng_path)
    orcs = [load_ndjson(p) for p in orc_paths]

    rows = []
    for path in files:
        d = json.load(open(path))
        name = d["name"]
        logical, markers = shape_markers(d)
        es = entry_state(eng[name])
        ostates = [entry_state(o[name]) for o in orcs]

        # partition oracle runs by canonical equality
        groups = []  # (state, result, count)
        for st, res in ostates:
            for g in groups:
                if g[0] == st and (res is None or diff_kind(g[1], res)[0] == "equal"):
                    g[2] += 1
                    break
            else:
                groups.append([st, res, 1])
        stability = " | ".join(f"{c}/{len(ostates)} {st}" for st, _, c in groups)

        if len(groups) > 1:
            flavors = []
            ok_groups = [g for g in groups if g[0] == "OK"]
            if len({g[0] for g in groups}) > 1:
                flavors.append("pass/limit flip" if any(g[0] == "FIRE-LIMIT" for g in groups)
                               else "state flip")
            if len(ok_groups) > 1:
                kinds = {diff_kind(ok_groups[0][1], g[1])[0] for g in ok_groups[1:]}
                flavors.append("order flip" if kinds <= {"ORDER-ONLY"} else "value flip")
            eng_match = any(g[0] == "OK" and es[0] == "OK" and diff_kind(g[1], es[1])[0] == "equal"
                            for g in groups)
            bucket = "ORACLE-NONDET"
            detail = ", ".join(flavors) + ("; one variant == engine" if eng_match else "")
        else:
            st, res, _ = groups[0]
            if st == "FIRE-LIMIT" and es[0] == "OK":
                bucket = "ORACLE-RUNAWAY"
                detail = f"engine terminates ({len(es[1]['firings'])} firings)"
            elif st == "OK" and es[0] == "OK":
                kind, detail = diff_kind(res, es[1])
                bucket = "PASS" if kind == "equal" else kind
            elif st == es[0]:
                bucket, detail = "BOTH-" + st, ""
            else:
                bucket, detail = "ERR-ASYM", f"oracle={st} engine={es[0]}"
        rows.append({"name": name, "bucket": bucket, "stability": stability,
                     "detail": detail, "logical": ",".join(logical),
                     "markers": ",".join(markers)})

    counts = Counter(r["bucket"] for r in rows)
    print("=== bucket counts ===")
    for k, v in counts.most_common():
        print(f"  {k:16s} {v}")
    pins = [r for r in rows if r["bucket"] in ("VALUE", "ORDER-ONLY") and not r["markers"]
            and r["logical"]]
    print(f"\n!!! in-envelope TMS pin candidates (diverging, no fence marker): "
          f"{[r['name'] for r in pins] if pins else 'NONE'}")
    print("\n=== per witness ===")
    for r in rows:
        print(f"{r['name']:18s} {r['bucket']:15s} [{r['markers']:10s}] "
              f"{r['stability']:22s} {r['detail'][:60]}")

    if args.md:
        with open(args.md, "w") as fh:
            fh.write("| scenario | bucket | fence shape | oracle runs | engine vs oracle |\n")
            fh.write("|---|---|---|---|---|\n")
            for r in rows:
                fh.write(f"| {r['name']} | {r['bucket']} | {r['markers'] or '—'} "
                         f"| {r['stability']} | {r['detail'] or '—'} |\n")
        print(f"\nmarkdown table -> {args.md}")


if __name__ == "__main__":
    main()
