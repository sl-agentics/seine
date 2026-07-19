#!/usr/bin/env python3
"""D-097 phase 2: the DuckDB differential oracle (D-095 authority for
data-type semantics; scope per D-097 ruling 5 = per-rule MATCH SETS +
accumulate results over INSERT-ONLY scenarios with inert RHS).

For each scenario (oracle: "duckdb"):
  1. Build tables from `types` (nullable columns per the flag), insert
     `facts` with an `idx` column = visible insertion order (the same
     sequence as engine handles, D-047).
  2. Translate each rule's LHS to a SELECT over the tables:
     positive patterns -> FROM aliases, constraints -> WHERE (surface
     `== null` arrives from the engine grammar as-is and maps to
     IS [NOT] NULL; everything else is the direct SQL operator, which
     IS the 3VL semantics); not/exists -> NOT EXISTS/EXISTS;
     accumulate -> a correlated scalar subquery (sum wrapped
     COALESCE(...,0) per D-097 ruling 2; avg/min/max require IS NOT
     NULL — SQL NULL result == engine no-propagate).
  3. Compare against the engine's firings (SEINE_HANDLES=1) as
     order-INSENSITIVE per-rule multisets of handle tuples (+ acc
     values).

Usage: .venv/bin/python tools/diff_duckdb.py scenarios/duckdb/*.json
"""
import json
import re
import subprocess
import sys

import duckdb

assert duckdb.__version__ == "1.5.4", f"oracle version drift: {duckdb.__version__} (pin: 1.5.4 — rerun tools/pin_duckdb.py and re-gate)"

SQLT = {"i64": "BIGINT", "f64": "DOUBLE", "String": "VARCHAR", "bool": "BOOLEAN"}


def sql_type(t):
    if t.startswith("decimal("):
        return "DECIMAL" + t[len("decimal"):]
    return SQLT[t]


def build_db(scn):
    con = duckdb.connect()
    for t in scn["types"]:
        cols = ", ".join(
            f'"{f["name"]}" {sql_type(f["type"])}{"" if f.get("nullable") else " NOT NULL"}'
            for f in t["fields"]
        )
        sep = ", " if cols else ""
        con.sql(f'CREATE TABLE "t_{t["name"]}" (idx BIGINT{sep}{cols})')
    for i, fact in enumerate(scn.get("facts", [])):
        t = next(t for t in scn["types"] if t["name"] == fact["type"])
        names = ["idx"] + [f'"{f["name"]}"' for f in t["fields"]]
        import decimal as _d
        vals = [i] + [
            _d.Decimal(v) if isinstance(v, str) and f["type"].startswith("decimal(") and v is not None
            else v
            for f, v in ((f, fact["fields"].get(f["name"])) for f in t["fields"])
        ]
        ph = ", ".join("?" * len(vals))
        con.execute(f'INSERT INTO "t_{fact["type"]}" ({", ".join(names)}) VALUES ({ph})', vals)
    return con


# ---------------- DRL LHS parsing (the harness-rendered subset) -----------
PAT_RE = re.compile(
    r"(?:(?P<neg>not|exists)\s+)?(?:\$(?P<fb>\w+)\s*:\s*)?(?P<type>[A-Z]\w*)\s*\((?P<body>[^()]*(?:\([^()]*\)[^()]*)*)\)"
)
ACC_RE = re.compile(
    r"accumulate\(\s*(?P<src>[A-Z]\w*)\s*\((?P<body>[^;]*)\)\s*;\s*\$(?P<res>\w+)\s*:\s*(?P<fn>sum|count|average|min|max)\((?P<arg>[^)]*)\)\s*\)"
)


def split_top(s, sep=","):
    out, depth, cur, instr = [], 0, "", False
    for c in s:
        if c == '"':
            instr = not instr
        if not instr:
            if c in "(":
                depth += 1
            elif c == ")":
                depth -= 1
            elif c == sep and depth == 0:
                out.append(cur.strip())
                cur = ""
                continue
        cur += c
    if cur.strip():
        out.append(cur.strip())
    return out


def sql_lit(tok):
    tok = tok.strip()
    if tok == "null":
        return "NULL"
    if tok.startswith('"') and tok.endswith('"'):
        inner = tok[1:-1].replace("'", "''")
        return f"'{inner}'"  # SQL string literal (double quotes are identifiers)
    return tok


def xlate_cexpr(expr, alias, binds):
    """One constraint expression -> SQL, resolving $vars via binds."""
    expr = expr.strip()
    # strip redundant outer parens
    while expr.startswith("(") and expr.endswith(")"):
        depth = 0
        ok = True
        for i, c in enumerate(expr):
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0 and i < len(expr) - 1:
                    ok = False
                    break
        if ok:
            expr = expr[1:-1].strip()
        else:
            break
    # top-level || / &&
    for op, sql in (("||", " OR "), ("&&", " AND ")):
        parts = split_top_bool(expr, op)
        if len(parts) > 1:
            return "(" + sql.join(xlate_cexpr(p, alias, binds) for p in parts) + ")"
    if expr.startswith("!") and expr[1:].lstrip().startswith("("):
        return f"(NOT {xlate_cexpr(expr[1:].lstrip(), alias, binds)})"
    m = re.match(r"(\w+)\s+not\s+in\s*\((.*)\)$", expr)
    if m:
        items = ", ".join(sql_lit(x) for x in split_top(m.group(2)))
        return f'({alias}."{m.group(1)}" NOT IN ({items}))'
    m = re.match(r"(\w+)\s+in\s*\((.*)\)$", expr)
    if m:
        items = ", ".join(sql_lit(x) for x in split_top(m.group(2)))
        return f'({alias}."{m.group(1)}" IN ({items}))'
    m = re.match(r"(\w+)\s+matches\s+(\".*\")$", expr)
    if m:
        return f'regexp_full_match({alias}."{m.group(1)}", {sql_lit(m.group(2))})'
    m = re.match(r"(\w+)\s+contains\s+(\".*\")$", expr)
    if m:
        return f'contains({alias}."{m.group(1)}", {sql_lit(m.group(2))})'
    m = re.match(r"(\w+)\s*(==|!=|<=|>=|<|>)\s*(.+)$", expr)
    if m:
        field, op, rhs = m.groups()
        rhs = rhs.strip()
        if rhs == "null":
            return f'({alias}."{field}" IS {"NOT " if op == "!=" else ""}NULL)'
        sqlop = {"==": "=", "!=": "<>"}.get(op, op)
        if rhs.startswith("$"):
            ra, rf = binds[rhs[1:]]
            rhs_sql = f'{ra}."{rf}"'
        else:
            rhs_sql = sql_lit(rhs)
        return f'({alias}."{field}" {sqlop} {rhs_sql})'
    raise ValueError(f"untranslatable constraint: {expr!r}")


def split_top_bool(s, op):
    out, depth, cur, instr, i = [], 0, "", False, 0
    while i < len(s):
        c = s[i]
        if c == '"':
            instr = not instr
        if not instr and depth == 0 and s[i : i + 2] == op:
            out.append(cur.strip())
            cur = ""
            i += 2
            continue
        if not instr:
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
        cur += c
        i += 1
    out.append(cur.strip())
    return out


def rule_to_sql(lhs, binds_seed=None):
    """LHS text -> (select_sql, n_positive). binds: $name -> (alias, field)."""
    binds = dict(binds_seed or {})
    aliases, wheres, existentials, accs = [], [], [], []
    body = lhs.strip()
    # accumulate first (its inner parens would confuse PAT_RE)
    for am in ACC_RE.finditer(body):
        accs.append(am)
    body_wo_acc = ACC_RE.sub(" ", body)
    pats = list(PAT_RE.finditer(body_wo_acc))
    # pass 1: register binds of positive patterns in order
    pending = []
    for pm in pats:
        neg, ty, pbody = pm.group("neg"), pm.group("type"), pm.group("body")
        alias = None
        if not neg:
            alias = f"p{len(aliases)}"
            aliases.append((alias, ty))
        cons = []
        for c in split_top(pbody):
            if not c:
                continue
            bm = re.match(r"\$(\w+)\s*:\s*(\w+)$", c)
            if bm:
                if alias:
                    binds[bm.group(1)] = (alias, bm.group(2))
                continue
            cons.append(c)
        pending.append((neg, ty, alias, cons))
    for neg, ty, alias, cons in pending:
        if not neg:
            for c in cons:
                wheres.append(xlate_cexpr(c, alias, binds))
        else:
            sub_alias = "e0"
            sub_w = [xlate_cexpr(c, sub_alias, binds) for c in cons]
            w = f" WHERE {' AND '.join(sub_w)}" if sub_w else ""
            q = f'SELECT 1 FROM "t_{ty}" {sub_alias}{w}'
            existentials.append(f"{'NOT ' if neg == 'not' else ''}EXISTS ({q})")
    sel_cols = [f"{a}.idx" for a, _ in aliases]
    acc_cols = []
    for am in accs:
        src, abody, fn, arg = am.group("src"), am.group("body"), am.group("fn"), am.group("arg").strip()
        sa = "a0"
        sub_binds = dict(binds)
        sub_w = []
        argfield = None
        for c in split_top(abody):
            if not c:
                continue
            bm = re.match(r"\$(\w+)\s*:\s*(\w+)$", c)
            if bm:
                sub_binds[bm.group(1)] = (sa, bm.group(2))
                if arg == f"${bm.group(1)}":
                    argfield = bm.group(2)
                continue
            sub_w.append(xlate_cexpr(c, sa, sub_binds))
        w = f" WHERE {' AND '.join(sub_w)}" if sub_w else ""
        if fn == "count":
            agg = "COUNT(*)"
        else:
            assert argfield, f"acc arg {arg} not bound in source"
            sqlfn = {"sum": "SUM", "average": "AVG", "min": "MIN", "max": "MAX"}[fn]
            agg = f'{sqlfn}({sa}."{argfield}")'
        sub = f'SELECT {agg} FROM "t_{src}" {sa}{w}'
        if fn in ("sum", "count"):
            acc_cols.append(f"COALESCE(({sub}), 0)")  # ruling 2: sum(empty/all-null)=0
        else:
            acc_cols.append(f"({sub})")
            existentials.append(f"({sub}) IS NOT NULL")  # no-propagate == NULL
    conds = wheres + existentials
    wh = f" WHERE {' AND '.join(conds)}" if conds else ""
    frm = ", ".join(f'"t_{t}" {a}' for a, t in aliases) or "(VALUES (1))"
    cols = ", ".join(sel_cols + acc_cols) or "1"
    return f"SELECT {cols} FROM {frm}{wh}", len(sel_cols)


def rules_from_drl(drl):
    out = []
    for m in re.finditer(r'rule\s+"?([\w-]+)"?.*?when(.*?)then', drl, re.S):
        out.append((m.group(1), m.group(2)))
    return out


def engine_match_sets(path):
    r = subprocess.run(
        ["cargo", "run", "-q", "-p", "seine-harness", "--", "run", path],
        capture_output=True, text=True, env={**__import__("os").environ, "SEINE_HANDLES": "1"},
    )
    if r.returncode != 0:
        raise RuntimeError(f"engine run failed: {r.stderr[-400:]}")
    out = json.loads(r.stdout)["result"]
    # visible insertion order (D-047): result.facts lists visible facts
    # in handle order — map engine __h -> duckdb idx.
    h2idx = {}
    for i, fact in enumerate(out["facts"]):
        if "__h" in fact["fields"]:
            h2idx[fact["fields"]["__h"]] = i
    sets = {}
    for f in out["firings"]:
        row = []
        for mfact in f["matches"]:
            if mfact["type"] == "InitialFact":
                continue
            # "BigDecimal" = the D-308 oracle-parity name for decimal
            # acc results (this comparator predates the rename)
            if mfact["type"] in ("Long", "Double", "Decimal", "BigDecimal"):
                v = mfact["fields"]["value"]
                row.append(("acc", round(float(v), 9)))
            elif "__h" in mfact["fields"]:
                row.append(("h", h2idx[mfact["fields"]["__h"]]))
        sets.setdefault(f["rule"], []).append(tuple(sorted(row)))
    return {k: sorted(v) for k, v in sets.items()}


def duckdb_match_sets(scn):
    con = build_db(scn)
    sets = {}
    for rname, lhs in rules_from_drl(scn["drl"]):
        sql, npos = rule_to_sql(lhs)
        rows = con.sql(sql).fetchall()
        rows_t = []
        for row in rows:
            r = [("h", int(h)) for h in row[:npos]]
            for v in row[npos:]:
                r.append(("acc", round(float(v), 9)))
            rows_t.append(tuple(sorted(r)))
        sets[rname] = sorted(rows_t)
    return sets


def main():
    fails = 0
    for path in sys.argv[1:]:
        scn = json.load(open(path))
        assert scn.get("oracle") == "duckdb", f"{path}: not a duckdb-oracle scenario"
        try:
            eng = engine_match_sets(path)
            ora = duckdb_match_sets(scn)
        except Exception as ex:
            print(f"FAIL {path}: {ex}")
            fails += 1
            continue
        # rules with zero matches may be absent on the engine side
        keys = set(eng) | set(ora)
        bad = [k for k in keys if eng.get(k, []) != ora.get(k, [])]
        if bad:
            fails += 1
            print(f"FAIL {path}")
            for k in bad:
                print(f"  rule {k}:\n    engine {eng.get(k, [])}\n    duckdb {ora.get(k, [])}")
        else:
            print(f"PASS {path.split('/')[-1]}")
    n = len(sys.argv) - 1
    print(f"---\n{n - fails} passed, {fails} failed, {n} total")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
