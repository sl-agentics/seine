#!/usr/bin/env python3
"""D-391 decision-table certification: Drools' SpreadsheetCompiler (oracle-tables/) vs the port (seine_rs.tables).

    python3 tools/table_oracle.py bank   [DIR]     run the oracle on every DIR/*.csv, write the expected DRL beside it
    python3 tools/table_oracle.py diff   [DIR]     port vs the banked .drl files (no JVM; the pytest gate does the same)
    python3 tools/table_oracle.py fuzz   [N] [SEED]  N random tables, port vs the LIVE oracle; failures saved to
                                                 scenarios/tables/fuzz_fail/ for minimizing and banking

    python3 tools/table_oracle.py robust [N] [SEED]  N hostile inputs (random bytes, ragged CSV, huge cells, every
                                                 table fragment) at the port alone: anything but a DecisionTableError
                                                 is a failure ("no input crashes Seine")

A file named *.matrix.csv is compiled as a RuleMatrix, anything else as a RuleTable. Expected outputs are the exact
text SpreadsheetCompiler returns, or "ERROR <exception class>: <message>" when Drools rejects the table.
  - DecisionTableParseException: the port's message must be byte-identical;
  - Drools CRASHES (NullPointerException, IndexOutOfBounds, "Script template is null"): the port must refuse the table
    with its own located message (DecisionTableError.drools_crashes);
  - Drools builds SILENTLY WRONG rules (a RuleMatrix with a blank header cell): the port refuses; counted separately.
"""
from __future__ import annotations

import os
import random
import subprocess
import sys
import types

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "scenarios", "tables")
ORACLE = os.path.join(ROOT, "oracle-tables")


def _tables():
    """seine_rs.tables without the native extension: authoring.py and tables.py are pure Python."""
    if "seine_rs.tables" not in sys.modules:
        pkg = types.ModuleType("seine_rs")
        pkg.__path__ = [os.path.join(ROOT, "bindings", "python", "seine_rs")]
        sys.modules.setdefault("seine_rs", pkg)
    import seine_rs.tables as T
    return T


def port(path: str) -> str:
    T = _tables()
    try:
        return T.decision_table(path, matrix=path.endswith(".matrix.csv")).drl
    except T.DecisionTableError as e:
        if e.drools_crashes:
            return f"GUARD {e.message}"
        return f"ERROR DecisionTableParseException: {e.message}"


def oracle(paths: list[str]) -> dict[str, str]:
    cp_file = os.path.join(ORACLE, "target", "classpath.txt")
    if not os.path.exists(cp_file):
        subprocess.run(["mvn", "-q", "-DskipTests", "package"], cwd=ORACLE, check=True)
    cp = os.path.join(ORACLE, "target", "classes") + ":" + open(cp_file).read().strip()
    req = "".join(("matrix" if p.endswith(".matrix.csv") else "rules") + "\t" + p + "\n" for p in paths)
    out = subprocess.run(["java", "-cp", cp, "dev.seine.oracle.TableDump", "batch"], input=req.encode(),
                         capture_output=True, check=True).stdout.decode("utf-8")
    res, cur, buf = {}, None, []
    for line in out.split("\n"):
        if line.startswith("\x01BEGIN "):
            cur, buf = line[len("\x01BEGIN "):], []
        elif line.startswith("\x01END") or "\x01END" in line:
            head, _, _ = line.partition("\x01END")
            buf.append(head)
            res[cur] = "\n".join(buf)
            cur = None
        elif cur is not None:
            buf.append(line)
    return res


def same(expected: str, got: str) -> bool:
    if expected == got:
        return True
    if expected.startswith("ERROR ") and not expected.startswith("ERROR DecisionTableParseException"):
        return got.startswith("GUARD ")          # Drools crashed: the port must refuse with its own message
    if not expected.startswith("ERROR ") and got.startswith("GUARD ") and "Drools would" in got:
        return True                              # Drools builds silently wrong rules: the port refuses
    return False


def _csvs(d: str) -> list[str]:
    return sorted(os.path.join(d, f) for f in os.listdir(d) if f.endswith(".csv"))


def bank(d: str):
    paths = _csvs(d)
    for p, out in oracle(paths).items():
        with open(p[:-4] + ".drl", "w", encoding="utf-8", newline="") as f:
            f.write(out)
    print(f"banked {len(paths)} tables in {d}")


def diff(d: str) -> int:
    bad = 0
    paths = _csvs(d)
    for p in paths:
        exp = open(p[:-4] + ".drl", encoding="utf-8", newline="").read()
        got = port(p)
        if not same(exp, got):
            bad += 1
            print(f"DIFF {os.path.relpath(p, ROOT)}")
            import difflib
            for line in difflib.unified_diff(exp.splitlines(), got.splitlines(), "drools", "port", lineterm="", n=1):
                print("   ", line)
    exact = sum(1 for p in paths if open(p[:-4] + ".drl", encoding="utf-8", newline="").read() == port(p))
    print(f"{len(paths) - bad}/{len(paths)} tables pass: {exact} byte-identical to Drools, "
          f"{len(paths) - bad - exact} refused with a located error where Drools crashes or builds wrong rules")
    return 1 if bad else 0


# ---------------------------------------------------------------- the fuzzer

_CELLS = ["", " ", "x", "10", "-3", "(5 + 1)", "true", "y", "on", "no", '"quoted"', 'say "hi"', "a, b", "1,2,3",
          "x...", "...", "$param", "$1", "“curly”", " padded ", "line\\nbreak", "eval", "é", "﻿bom"]
_HEADERS = ["CONDITION", "ACTION", "NAME", "DESCRIPTION", "PRIORITY", "NO-LOOP", "AGENDA-GROUP", "ACTIVATION-GROUP",
            "METADATA", "C", "A", "P", "X", "@", "DURATION", "TIMER", "ENABLED", "LOCK-ON-ACTIVE", "AUTO-FOCUS",
            "CALENDARS", "RULEFLOW-GROUP", "DATE-EFFECTIVE", "DATE-EXPIRES", "Bogus", "condition"]
_OBJECTS = ["", "Person", "Person()", "$p : Person", "Person() from $list", "Person from $list", "eval",
            "p: /people", "Person @watch(age)", "Cheese(type == 'x')", "Person..."]
_SNIPPETS = ["age > $param", "name == \"$param\"", "age >", "name", "flag?", "age in ($1, $2)",
             "forall(||){age == $}", "forall(&&){name != \"$\"}", "insert(new Hit(\"$param\"));",
             "setAge($param)", "$param", "update($p);", "list.add($param);", "x == $1 && y == $2", "type"]
_PROPS = [["RuleSet", "my rules"], ["Import", "a.B, c.D"], ["Variables", "java.util.List list"],
          ["Sequential", "true"], ["EscapeQuotes", "false"], ["SequentialMaxPriority", "100"],
          ["SequentialMinPriority", "98"], ["NO-LOOP", "true"], ["PRIORITY", "7"], ["Dialect", "java"],
          ["Functions", "function int f() { return 1; }"], ["AGENDA-GROUP", "g"], ["Unit", "U"],
          ["Declare", "declare T x : int end"], ["Queries", "query q() end"], ["NumericDisabled", "true"]]


def _q(cell: str) -> str:
    return '"' + cell.replace('"', '""') + '"' if ("," in cell or '"' in cell) else cell


# column kinds that make a VALID table, with the data cells each kind accepts
_VALID = {
    "CONDITION": (["Person", "Person()", "$p : Person", "Person() from $list", "Person from $list", "eval",
                   "p: /people", "Person @watch(age)", "", "Cheese(type == 'x')"],
                  ["age > $param", 'name == "$param"', "age >", "name", "flag?", "age in ($1, $2)",
                   "forall(||){age == $}", "forall(&&){name != \"$\"}", "x == $1 && y == $2", "type",
                   "age >= $param", "name matches", "tags contains"],
                  ["10", "-3", "x", '"quoted"', 'say "hi"', "a, b", "1,2,3", "\u201ccurly\u201d", " padded ",
                   "line\\nbreak", "é", "(1)", "[2]", "'s'", ""]),
    "ACTION": (["", "$p", "list"],
               ['insert(new Hit("$param"));', "setAge($param)", "update($p);", "list.add($param);", "$param",
                "modify($p){ setN($1) }", "retract($p);"],
               ["10", "x", '"q"', "a, b", 'say "hi"', "", "é"]),
    "PRIORITY": ([""], [""], ["5", "-1", "(10 + 1)", "x", "2147483648", ""]),
    "NAME": ([""], [""], ["r one", 'q"uote', '"already"', "", "Ünï"]),
    "DESCRIPTION": ([""], [""], ["about this", "", "x"]),
    "METADATA": ([""], ["author(\"$param\")", "tag($param)"], ["bob", "1", ""]),
}
_ATTR_COLS = {"NO-LOOP": ["true", "y", "no", ""], "AGENDA-GROUP": ["g1", '"g2"', ""],
              "ACTIVATION-GROUP": ["x", ""], "DURATION": ["100", "x", ""], "TIMER": ["int: 1s", "(cron: 0)", ""],
              "ENABLED": ["false", "on", ""], "LOCK-ON-ACTIVE": ["true", ""], "AUTO-FOCUS": ["yes", ""],
              "CALENDARS": ["weekdays", ""], "RULEFLOW-GROUP": ["rf", ""], "DATE-EFFECTIVE": ["1-Jan-2020", ""],
              "DATE-EXPIRES": ["1-Jan-2030", ""]}


def gen_valid(rng: random.Random) -> list:
    rows = []
    for _ in range(rng.randint(0, 3)):
        rows.append(rng.choice(_PROPS[:10]))
    if rng.random() < 0.3:
        rows.append([])
    cols = []
    for _ in range(rng.randint(1, 6)):
        k = rng.choice(list(_VALID) * 3 + list(_ATTR_COLS))
        if k in ("PRIORITY", "NAME") + tuple(_ATTR_COLS) and any(c[0] == k for c in cols):
            continue
        cols.append((k, rng.choice(["long", "short"])))
    lead = [""] * rng.choice([0, 0, 1])
    rows.append(lead + ["RuleTable " + rng.choice(["T", "Pricing rules", "x"])])
    hdr, obj, code, lab = [], [], [], []
    for k, form in cols:
        hdr.append(k if form == "long" else {"CONDITION": "C", "ACTION": "A", "PRIORITY": "P", "NAME": "N",
                                             "DESCRIPTION": "I", "METADATA": "@"}.get(k, k))
        if k in _VALID:
            o, sn, _ = _VALID[k]
            obj.append(rng.choice(o))
            code.append(rng.choice(sn))
        else:
            obj.append("")
            code.append("")
        lab.append(rng.choice(["label", "", "Age"]))
    rows += [lead + hdr, lead + obj, lead + code, lead + lab]
    for _ in range(rng.randint(0, 6)):
        rows.append(lead + [rng.choice(_VALID[k][2] if k in _VALID else _ATTR_COLS[k]) for k, _ in cols])
    return rows


def gen_matrix(rng: random.Random) -> list:
    rows = [["AgendaGroup", rng.choice(["grid", '"q"', "a b"])],
            ["Action", rng.choice(['insert(new Hit("$param"));', "x($param);", "y($1, $2);", "forall(,){z($)}"])],
            ["HorizontalCondition", rng.choice(['H(v == "$param")', "H(v > $param)", "H($1)"])],
            ["VerticalCondition", rng.choice(['V(v == "$param")', "V(w < $param)"])]]
    if rng.random() < 0.8:
        rows.append(["Precondition", rng.choice(["P()", "$p : P(ok)"])])
    rng.shuffle(rows)
    for _ in range(rng.randint(0, 2)):
        rows.append(rng.choice(_PROPS[:3]))
    w = rng.randint(1, 4)
    rows.append([""] * rng.choice([0, 1]) + ["RuleTable " + rng.choice(["Grid", "m"])]
                + [rng.choice(["a", "b", "c, d", " sp "]) for _ in range(w)])
    lead = len(rows[-1]) - w - 1
    for _ in range(rng.randint(1, 5)):
        rows.append([""] * lead + [rng.choice(["r1", "r2", " r3 "])] + [rng.choice(["1", "2", "", "x y", "a,b"])
                                                                         for _ in range(rng.randint(0, w))])
    return rows


def gen_table(rng: random.Random) -> tuple[str, bool]:
    if rng.random() < 0.15:
        eol = rng.choice(["\n", "\r\n"])
        return eol.join(",".join(_q(c) for c in r) for r in gen_matrix(rng)) + eol, True
    if rng.random() < 0.7:
        rows = gen_valid(rng)
        eol = rng.choice(["\n", "\n", "\r\n"])
        return eol.join(",".join(_q(c) for c in r) for r in rows) + rng.choice([eol, ""]), False
    matrix = rng.random() < 0.3
    rows = []
    for _ in range(rng.randint(0, 4)):
        rows.append(rng.choice(_PROPS + ([["AgendaGroup", "grid"], ["Action", "insert(new Hit(\"$param\"));"],
                                           ["HorizontalCondition", "H(v == \"$param\")"],
                                           ["VerticalCondition", "V(v == \"$param\")"],
                                           ["Precondition", "P()"]] * 3 if matrix else [])))
    if rng.random() < 0.3:
        rows.append([])
    if matrix:
        w = rng.randint(1, 4)
        rows.append(["RuleTable " + rng.choice(["Grid", "m", ""])] + [rng.choice(["a", "b", "c", "d", ""]) for _ in range(w)])
        for _ in range(rng.randint(0, 4)):
            rows.append([rng.choice(["r1", "r2", ""])] + [rng.choice(["1", "2", "", "x"]) for _ in range(rng.randint(0, w + 1))])
    else:
        lead = [""] * rng.choice([0, 0, 1])
        ncols = rng.randint(1, 5)
        rows.append(lead + ["RuleTable " + rng.choice(["T", "Pricing rules", "x", ""])])
        rows.append(lead + [rng.choice(_HEADERS) for _ in range(ncols)])
        rows.append(lead + [rng.choice(_OBJECTS) for _ in range(ncols)])
        rows.append(lead + [rng.choice(_SNIPPETS + [""]) for _ in range(ncols)])
        rows.append(lead + [rng.choice(["label", "", "Age"]) for _ in range(ncols)])
        for _ in range(rng.randint(0, 5)):
            if rng.random() < 0.1:
                rows.append([])
            rows.append(lead + [rng.choice(_CELLS) for _ in range(rng.randint(0, ncols + 1))])
        if rng.random() < 0.2:
            rows.append(["RuleTable Second"])
            rows.append(["CONDITION", "ACTION"])
            rows.append(["Person", ""])
            rows.append(["age > $param", "insert($param);"])
            rows.append(["", ""])
            rows.append([rng.choice(_CELLS), rng.choice(_CELLS)])
    eol = rng.choice(["\n", "\n", "\r\n", "\r"])
    text = eol.join(",".join(_q(c) for c in r) for r in rows) + rng.choice([eol, ""])
    return text, matrix


def fuzz(n: int, seed: int) -> int:
    rng = random.Random(seed)
    work = os.path.join(DIR, "fuzz_work")
    os.makedirs(work, exist_ok=True)
    paths = []
    for i in range(n):
        text, matrix = gen_table(rng)
        p = os.path.join(work, f"f{seed}_{i:05d}" + (".matrix.csv" if matrix else ".csv"))
        with open(p, "w", encoding="utf-8", newline="") as f:
            f.write(text)
        paths.append(p)
    exp = oracle(paths)
    fails = [p for p in paths if not same(exp[p], port(p))]
    keep = os.path.join(DIR, "fuzz_fail")
    if fails:
        os.makedirs(keep, exist_ok=True)
        for p in fails[:50]:
            os.replace(p, os.path.join(keep, os.path.basename(p)))
            with open(os.path.join(keep, os.path.basename(p))[:-4] + ".drl", "w", encoding="utf-8", newline="") as f:
                f.write(exp[p])
    for p in paths:
        if os.path.exists(p):
            os.remove(p)
    os.rmdir(work)
    kinds = {}
    for p in paths:
        k = "error" if exp[p].startswith("ERROR") else ("matrix" if p.endswith(".matrix.csv") else "drl")
        kinds[k] = kinds.get(k, 0) + 1
    import re as _re
    feats = {"2+ attributes": r"\n\t[a-z-]+ .*\n\t[a-z-]+ .*\n\twhen", "salience": r"\tsalience ",
             "forall join": r" (\|\||&&) ", "from": r"\) from", "eval(": r"eval\(", "@watch": r"@watch",
             "metadata": r"\n\t@", "description": r"\n/\* ", "escaped quote": r'\\"', "global": r"\nglobal ",
             "import": r"\nimport ", "matrix rule": r'rule "rule_\d+_\d+"'}
    cov = {k: sum(1 for p in paths if not exp[p].startswith("ERROR") and _re.search(v, exp[p])) for k, v in feats.items()}
    print("coverage (tables whose Drools DRL shows it):", cov)
    print(f"fuzz seed {seed}: {n - len(fails)}/{n} match ({kinds}); failures kept in {keep}" if fails
          else f"fuzz seed {seed}: {n}/{n} match the live oracle ({kinds})")
    return 1 if fails else 0


def robust(n: int, seed: int) -> int:
    T = _tables()
    rng = random.Random(seed)
    frags = ["RuleTable X", "RuleTable", "ruletable y", "CONDITION", "ACTION", "C", "A", "PRIORITY", "NAME", "@",
             "Person", "Person...", "...", "$param", "$1", "$2", "forall(||){x == $}", "forall(", "){", "eval",
             "from", "@watch(", "p: /", "\"", "\"\"", "'", "\\", "\\n", ",", "(", ")", "[", "]", "{", "}",
             "AgendaGroup", "Action", "HorizontalCondition", "VerticalCondition", "Precondition", "Sequential",
             "true", "SequentialMinPriority", "SequentialMaxPriority", "-1", "99999999999", "Import", "Variables",
             "Variables,x", "RuleSet", "Unit", "Dialect", "\u0000", "\ufeff", "\u2028", "\U0001f600", "\udc80", " ", ""]
    bad = 0
    for i in range(n):
        r = rng.random()
        if r < 0.15:
            data = bytes(rng.randrange(256) for _ in range(rng.randrange(0, 400)))
        elif r < 0.25:
            data = ("x" * rng.randrange(1, 200000)).encode()
        else:
            lines = []
            for _ in range(rng.randrange(0, 14)):
                lines.append(",".join(rng.choice(frags) + (rng.choice(frags) if rng.random() < 0.3 else "")
                                      for _ in range(rng.randrange(0, 7))))
            data = rng.choice(["\n", "\r\n", "\r"]).join(lines).encode("utf-8", "surrogatepass")
        for matrix in (False, True):
            try:
                t = T.decision_table(text=data, matrix=matrix)
                try:
                    t.rules
                except T.DecisionTableError as e:
                    if e.message.startswith("internal error"):
                        raise RuntimeError(e.message) from None
            except T.DecisionTableError as e:
                if e.message.startswith("internal error"):     # the catch-all fired: a port defect
                    e2 = RuntimeError(e.message)
                    bad += 1
                    print(f"INTERNAL case {i} matrix={matrix}: {e2}") if bad <= 10 else None
            except Exception as e:   # noqa: BLE001 — exactly what this hunts
                bad += 1
                if bad <= 10:
                    print(f"CRASH case {i} matrix={matrix}: {type(e).__name__}: {e}")
                    os.makedirs(os.path.join(DIR, "fuzz_fail"), exist_ok=True)
                    with open(os.path.join(DIR, "fuzz_fail", f"robust_{seed}_{i}.bin"), "wb") as f:
                        f.write(data)
    print(f"robust seed {seed}: {2 * n - bad}/{2 * n} inputs ended in a DecisionTableError or DRL, {bad} crashes")
    return 1 if bad else 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "diff"
    if cmd == "bank":
        bank(sys.argv[2] if len(sys.argv) > 2 else DIR)
    elif cmd == "diff":
        sys.exit(diff(sys.argv[2] if len(sys.argv) > 2 else DIR))
    elif cmd == "fuzz":
        sys.exit(fuzz(int(sys.argv[2]) if len(sys.argv) > 2 else 500, int(sys.argv[3]) if len(sys.argv) > 3 else 42))
    elif cmd == "robust":
        sys.exit(robust(int(sys.argv[2]) if len(sys.argv) > 2 else 5000, int(sys.argv[3]) if len(sys.argv) > 3 else 1))
    else:
        sys.exit(__doc__)
