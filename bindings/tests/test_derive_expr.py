"""The derive expression layer's certification battery.

Three independent evaluators are compared:
  1. the Rust kernels (seine_rs.derive.with_columns / .filter),
  2. RefEval — a pure-python tree walker in THIS file implementing
     docs/derive-expr-pins.md literally (never checked against Rust
     internals; errors are values),
  3. DuckDB 1.5.4 — the measured data-plane oracle, via a tree->SQL
     translator (int literals CAST AS BIGINT to pin widths; `||`,
     never the null-swallowing concat(); strlen(), not length()).

Plus: the wire format pinned as API, trap-guard and error-message
vectors, Kleene tables, determinism pins, an IEEE side-battery
(Rust vs reference only — the oracle treats NaN as an ordered value),
a polars cross-check on curated vectors, and the derived-batch ->
Session feed test.
"""
import math
import random
import re

import pytest

import seine_rs
from seine_rs.derive import Expr, col, if_else, lit, with_columns
from seine_rs.derive import filter as dfilter

# ---------------------------------------------------------------------
# RefEval: the independent pure-python reference
# ---------------------------------------------------------------------

I64_MIN, I64_MAX = -(2**63), 2**63 - 1


class RefErr(Exception):
    def __init__(self, kind):
        self.kind = kind  # overflow | div0 | cast | domain


def _trunc_div(a, b):
    if b == 0:
        raise RefErr("div0")
    q = abs(a) // abs(b)
    return -q if (a < 0) != (b < 0) else q


def _ckd(v):
    if not I64_MIN <= v <= I64_MAX:
        raise RefErr("overflow")
    return v


def _round_shortest_decimal(x, nd):
    """docs/derive-expr-pins.md §E + ledger row 8: round the shortest
    decimal representation half away from zero."""
    if x != x or x in (float("inf"), float("-inf")) or x == 0.0:
        return x
    neg = math.copysign(1.0, x) < 0
    mant, _, exp = f"{abs(x):e}".partition("e")
    # f-string %e is not shortest — use repr-based shortest form
    r = repr(abs(x))
    if "e" in r or "E" in r:
        mant, _, exp = r.lower().partition("e")
        exp = int(exp)
    else:
        if "." not in r:
            r += ".0"
        ip, fp = r.split(".")
        digits_all = (ip + fp).lstrip("0")
        exp = len(ip.lstrip("0")) - 1 if ip.strip("0") else -(len(fp) - len(fp.lstrip("0")) + 1)
        mant = digits_all
    digits = [int(c) for c in mant.replace(".", "")]
    keep = exp + nd
    if keep < -1:
        out = 0.0
    elif keep == -1:
        out = 10.0 ** (-nd) if digits[0] >= 5 else 0.0
    elif keep >= len(digits) - 1:
        out = abs(x)
    else:
        up = digits[keep + 1] >= 5
        digits = digits[: keep + 1]
        if up:
            i = keep
            while True:
                if i < 0:
                    digits.insert(0, 1)
                    exp += 1
                    break
                if digits[i] == 9:
                    digits[i] = 0
                    i -= 1
                else:
                    digits[i] += 1
                    break
        body = "".join(str(d) for d in digits)
        out = float(f"{body[0]}.{body[1:] or '0'}e{exp}")
    return -out if neg else out


def _ref_type(tree, schema):
    """Mirror of the Rust typecheck (types: i64/f64/bool/utf8)."""
    op = tree["op"]
    if op == "col":
        return schema[tree["name"]]
    if op == "lit":
        v = tree["value"]
        if isinstance(v, bool):
            return "bool"
        if isinstance(v, int):
            return "i64"
        if isinstance(v, float):
            return "f64"
        return "utf8"
    a = [_ref_type(t, schema) for t in tree["args"]]
    if op in ("add", "sub", "mul", "rem", "fill_null"):
        return "f64" if "f64" in a else a[0]
    if op in ("div", "pow", "sqrt"):
        return "f64"
    if op in ("floordiv",):
        return "i64"
    if op in ("eq", "neq", "lt", "lt_eq", "gt", "gt_eq", "and", "or", "not",
              "is_null", "is_not_null", "str_contains", "str_starts_with",
              "str_ends_with", "regexp_matches", "regexp_full_match"):
        return "bool"
    if op in ("neg", "abs", "floor", "ceil", "round"):
        return a[0]
    if op in ("sin", "cos", "tan", "asin", "acos", "atan", "ln", "log10",
              "exp", "degrees", "radians"):
        return "f64"
    if op == "cast":
        return tree["to"]
    if op == "if_else":
        return "f64" if ("f64" in a[1:]) and a[1] != a[2] else a[1]
    if op == "str_len":
        return "i64"
    if op == "concat":
        return "utf8"
    raise AssertionError(op)


def ref_eval_row(tree, row, schema):
    """One row, pure python, pins-doc semantics. None = SQL NULL."""
    op = tree["op"]
    if op == "col":
        return row[tree["name"]]
    if op == "lit":
        return tree["value"]

    def sub(i):
        return ref_eval_row(tree["args"][i], row, schema)

    def styp(i):
        return _ref_type(tree["args"][i], schema)

    if op in ("add", "sub", "mul", "div", "floordiv", "rem", "pow"):
        a, b = sub(0), sub(1)
        if a is None or b is None:
            return None
        promoted_f = "f64" in (styp(0), styp(1)) or op in ("div", "pow")
        if promoted_f:
            a, b = float(a), float(b)
            if op == "add":
                return a + b
            if op == "sub":
                return a - b
            if op == "mul":
                return a * b
            if op == "div":
                if b == 0.0:
                    if a != a or a == 0.0:
                        return float("nan")
                    return math.copysign(float("inf"), a) * math.copysign(1.0, b)
                return a / b
            if op == "rem":
                if b == 0.0 or a != a or b != b or a in (float("inf"), float("-inf")):
                    return float("nan")  # IEEE fmod; python's math.fmod raises on inf
                return math.fmod(a, b)
            if op == "pow":
                try:
                    r = a**b
                except OverflowError:
                    r = float("inf")
                except ValueError:  # (-x)**0.5: python raises, powf -> NaN
                    r = float("nan")
                if isinstance(r, complex):
                    r = float("nan")
                return float(r)
            raise AssertionError(op)
        # integer, checked
        if op == "add":
            return _ckd(a + b)
        if op == "sub":
            return _ckd(a - b)
        if op == "mul":
            return _ckd(a * b)
        if op == "floordiv":
            return _ckd(_trunc_div(a, b))
        if op == "rem":
            if b == 0:
                raise RefErr("div0")
            # arrow documents i64::MIN % -1 == 0 (no error)
            return a - _trunc_div(a, b) * b if not (a == I64_MIN and b == -1) else 0
        raise AssertionError(op)
    if op in ("eq", "neq", "lt", "lt_eq", "gt", "gt_eq"):
        a, b = sub(0), sub(1)
        if a is None or b is None:
            return None
        if "f64" in (styp(0), styp(1)) and styp(0) != styp(1):
            a, b = float(a), float(b)
        return {
            "eq": a == b,
            "neq": a != b,
            "lt": a < b,
            "lt_eq": a <= b,
            "gt": a > b,
            "gt_eq": a >= b,
        }[op]
    if op == "and":
        a, b = sub(0), sub(1)
        if a is False or b is False:
            return False
        if a is None or b is None:
            return None
        return True
    if op == "or":
        a, b = sub(0), sub(1)
        if a is True or b is True:
            return True
        if a is None or b is None:
            return None
        return False
    if op == "not":
        a = sub(0)
        return None if a is None else not a
    if op == "is_null":
        return sub(0) is None
    if op == "is_not_null":
        return sub(0) is not None
    if op == "fill_null":
        a, b = sub(0), sub(1)
        out = b if a is None else a
        if out is not None and "f64" in (styp(0), styp(1)) and styp(0) != styp(1):
            out = float(out)
        return out
    if op == "if_else":
        # EAGER like the vectorized kernel: both branches evaluate for
        # every row, so a row-level error in the unselected branch still
        # errors (ledger row 9 — SQL CASE is lazy, dataframes are not)
        c = sub(0)
        t_val, f_val = sub(1), sub(2)
        out = t_val if c is True else f_val  # NULL cond -> otherwise
        if out is not None and _ref_type(tree, schema) == "f64":
            out = float(out)
        return out
    if op == "neg":
        a = sub(0)
        if a is None:
            return None
        if isinstance(a, int):
            return _ckd(-a)
        return -a
    if op == "abs":
        a = sub(0)
        if a is None:
            return None
        if isinstance(a, int):
            return _ckd(abs(a))
        return abs(a)
    if op in ("floor", "ceil"):
        a = sub(0)
        if a is None or styp(0) == "i64":
            return a
        if a != a or a in (float("inf"), float("-inf")):
            return a  # IEEE specials pass through
        return float(math.floor(a) if op == "floor" else math.ceil(a))
    if op == "sqrt":
        a = sub(0)
        if a is None:
            return None
        a = float(a)
        if a != a:
            return a  # NaN propagates
        if a < 0.0:
            raise RefErr("domain")
        return math.sqrt(a)
    if op == "round":
        a = sub(0)
        if a is None or styp(0) == "i64":
            return a
        return _round_shortest_decimal(a, tree["ndigits"])
    if op in ("sin", "cos", "tan", "asin", "acos", "atan", "ln", "log10",
              "exp", "degrees", "radians"):
        a = sub(0)
        if a is None:
            return None
        a = float(a)
        if a != a and op not in ():  # NaN propagates through the whole row
            return a
        if op in ("sin", "cos", "tan"):
            if a in (float("inf"), float("-inf")):
                raise RefErr("domain")
            return getattr(math, op)(a)
        if op in ("asin", "acos"):
            if a < -1.0 or a > 1.0:
                raise RefErr("domain")
            return getattr(math, op)(a)
        if op == "atan":
            return math.atan(a)
        if op in ("ln", "log10"):
            if a == 0.0 or a < 0.0:
                raise RefErr("domain")
            if a == float("inf"):
                return a
            return math.log(a) if op == "ln" else math.log10(a)
        if op == "exp":
            try:
                return math.exp(a)
            except OverflowError:
                return float("inf")
        if op == "degrees":
            return a * (180.0 / math.pi)
        return a * (math.pi / 180.0)
    if op == "cast":
        a = sub(0)
        if a is None:
            return None
        if tree["to"] == "f64":
            return float(a)
        if isinstance(a, int):
            return a
        if a != a or a in (float("inf"), float("-inf")):
            raise RefErr("cast")
        r = _round_half_even(a)
        if not I64_MIN <= r <= I64_MAX:
            raise RefErr("cast")
        return r
    if op == "str_len":
        a = sub(0)
        return None if a is None else len(a.encode("utf-8"))
    if op == "concat":
        a, b = sub(0), sub(1)
        return None if a is None or b is None else a + b
    if op in ("str_contains", "str_starts_with", "str_ends_with"):
        a, b = sub(0), sub(1)
        if a is None or b is None:
            return None
        if op == "str_contains":
            return b in a
        if op == "str_starts_with":
            return a.startswith(b)
        return a.endswith(b)
    if op in ("regexp_matches", "regexp_full_match"):
        # pins §N: search vs whole-string; the pattern is a build-time
        # literal (kernel-validated), so RefEval sees only valid ones
        a = sub(0)
        if a is None:
            return None
        if op == "regexp_matches":
            return re.search(tree["pattern"], a) is not None
        return re.fullmatch(tree["pattern"], a) is not None
    raise AssertionError(op)


def _round_half_even(x):
    f = math.floor(x)
    d = x - f
    if d > 0.5:
        return f + 1
    if d < 0.5:
        return f
    return f if f % 2 == 0 else f + 1


def ref_eval(tree, data, schema):
    """-> ("ok", [values]) or ("err", kind)."""
    n = len(next(iter(data.values()))) if data else 0
    out = []
    try:
        for i in range(n):
            row = {k: v[i] for k, v in data.items()}
            out.append(ref_eval_row(tree, row, schema))
    except RefErr as e:
        return ("err", e.kind)
    return ("ok", out)


# ---------------------------------------------------------------------
# Rust + DuckDB harnesses
# ---------------------------------------------------------------------

def _err_kind(msg):
    m = str(msg)
    if "integer overflow" in m or "abs(" in m and "verflow" in m:
        return "overflow"
    if "division by zero" in m:
        return "div0"
    if "out of range for i64" in m or "has no i64 value" in m:
        return "cast"
    if "square root of a negative" in m:
        return "domain"
    if ("trig of an infinite" in m or "is undefined outside" in m
            or "of zero is undefined" in m or ") is undefined" in m):
        return "domain"
    return f"other:{m[:60]}"


def rust_eval(expr, data):
    try:
        t = with_columns(data, __out__=expr)
    except ValueError as e:
        return ("err", _err_kind(e))
    return ("ok", [r["__out__"] for r in t.to_pylist()])


SQLT = {"i64": "BIGINT", "f64": "DOUBLE", "bool": "BOOLEAN", "utf8": "VARCHAR"}


def to_sql(tree):
    op = tree["op"]
    if op == "col":
        return f'"{tree["name"]}"'
    if op == "lit":
        v = tree["value"]
        if isinstance(v, bool):
            return "TRUE" if v else "FALSE"
        if isinstance(v, int):
            return f"CAST({v} AS BIGINT)"
        if isinstance(v, float):
            return f"CAST({v!r} AS DOUBLE)"
        return "'" + v.replace("'", "''") + "'"
    a = [to_sql(t) for t in tree.get("args", [])]
    if op in ("add", "sub", "mul"):
        sym = {"add": "+", "sub": "-", "mul": "*"}[op]
        return f"({a[0]} {sym} {a[1]})"
    if op == "div":
        return f"(CAST({a[0]} AS DOUBLE) / CAST({a[1]} AS DOUBLE))"
    if op == "floordiv":
        return f"({a[0]} // {a[1]})"
    if op == "rem":
        return f"({a[0]} % {a[1]})"
    if op == "pow":
        return f"pow({a[0]}, {a[1]})"
    if op in ("eq", "neq", "lt", "lt_eq", "gt", "gt_eq"):
        sym = {"eq": "=", "neq": "<>", "lt": "<", "lt_eq": "<=", "gt": ">", "gt_eq": ">="}[op]
        return f"({a[0]} {sym} {a[1]})"
    if op in ("and", "or"):
        return f"({a[0]} {op.upper()} {a[1]})"
    if op == "not":
        return f"(NOT {a[0]})"
    if op == "is_null":
        return f"({a[0]} IS NULL)"
    if op == "is_not_null":
        return f"({a[0]} IS NOT NULL)"
    if op == "fill_null":
        return f"COALESCE({a[0]}, {a[1]})"
    if op == "if_else":
        return f"(CASE WHEN {a[0]} THEN {a[1]} ELSE {a[2]} END)"
    if op == "neg":
        return f"(-({a[0]}))"
    if op in ("abs", "floor", "ceil", "sqrt", "sin", "cos", "tan", "asin",
              "acos", "atan", "ln", "log10", "exp", "degrees", "radians"):
        return f"{op}({a[0]})"
    if op == "round":
        return f"round({a[0]}, {tree['ndigits']})"
    if op == "cast":
        return f"CAST({a[0]} AS {SQLT[tree['to']]})"
    if op == "str_len":
        return f"strlen({a[0]})"
    if op == "concat":
        return f"({a[0]} || {a[1]})"
    if op == "str_contains":
        return f"contains({a[0]}, {a[1]})"
    if op == "str_starts_with":
        return f"starts_with({a[0]}, {a[1]})"
    if op == "str_ends_with":
        return f"ends_with({a[0]}, {a[1]})"
    if op in ("regexp_matches", "regexp_full_match"):
        pat = tree["pattern"].replace("'", "''")
        return f"{op}({a[0]}, '{pat}')"
    raise AssertionError(op)


def _duck():
    duckdb = pytest.importorskip("duckdb")
    assert duckdb.__version__ == "1.5.4", (
        f"the derive-expr pins are measured against duckdb 1.5.4, found "
        f"{duckdb.__version__} — re-run tools/pin_derive_expr.py, diff the doc, "
        "and update deliberately"
    )
    return duckdb


def duck_eval(duckdb, tree, data, schema):
    con = duckdb.connect()
    cols = ", ".join(f'"{k}" {SQLT[schema[k]]}' for k in data)
    con.sql(f"CREATE TABLE t (idx BIGINT, {cols})")
    n = len(next(iter(data.values())))
    for i in range(n):
        vals = ", ".join(_sql_val(data[k][i], schema[k]) for k in data)
        con.sql(f"INSERT INTO t VALUES ({i}, {vals})")
    try:
        rows = con.sql(f"SELECT {to_sql(tree)} FROM t ORDER BY idx").fetchall()
    except Exception as e:
        return ("err", str(e).splitlines()[0])
    return ("ok", [r[0] for r in rows])


def _sql_val(v, t):
    if v is None:
        return f"CAST(NULL AS {SQLT[t]})"
    if t == "bool":
        return "TRUE" if v else "FALSE"
    if t == "utf8":
        return "'" + v.replace("'", "''") + "'"
    if t == "f64":
        return f"CAST({v!r} AS DOUBLE)"
    return f"CAST({v} AS BIGINT)"


def _values_match(a, b):
    if len(a) != len(b):
        return False
    for x, y in zip(a, b):
        if x is None or y is None:
            if x is not y:
                return False
            continue
        if isinstance(x, float) or isinstance(y, float):
            xf, yf = float(x), float(y)
            if math.isnan(xf) and math.isnan(yf):
                continue
            if xf != yf:
                return False
        elif x != y:
            return False
    return True


# ---------------------------------------------------------------------
# Fixed data
# ---------------------------------------------------------------------

SCHEMA = {
    "ia": "i64", "ib": "i64", "inn": "i64",
    "fa": "f64", "fb": "f64", "fnn": "f64",
    "ba": "bool", "bb": "bool",
    "sa": "utf8", "sb": "utf8",
}
DATA = {
    "ia": [-7, 3, None, 0, 2, -1, 7, None, 5, -3],
    "ib": [2, -3, 4, None, 1, 2, 0, 3, -2, 6],
    "inn": [1, 2, 3, 4, 5, -6, 7, 8, -9, 10],
    "fa": [2.5, -1.5, None, 0.0, 2.675, -7.5, 1.25, None, 3.5, -0.5],
    "fb": [1.0, 2.0, 0.5, None, -2.0, 3.0, 0.0, 1.5, -1.0, 2.5],
    "fnn": [0.5, 1.5, -2.5, 3.5, 0.125, 2.665, -1.0, 4.0, 1234.5, 0.0],
    "ba": [True, False, None, True, False, None, True, False, True, None],
    "bb": [None, True, False, True, None, False, True, None, False, True],
    "sa": ["abc", "", None, "zz", "beta", "a|b", None, "ABC", "über", "x"],
    "sb": ["b", "a", "c", None, "be", "|", "q", "AB", "ü", ""],
}


# ---------------------------------------------------------------------
# The wire format is API
# ---------------------------------------------------------------------

def test_wire_format_pinned():
    assert (col("price") * col("qty")).to_tree() == {
        "op": "mul",
        "args": [{"op": "col", "name": "price"}, {"op": "col", "name": "qty"}],
    }
    nested = if_else(col("d") > 5000, lit("far"),
                     if_else(col("c").is_null(), "unknown", "near")).to_tree()
    assert nested == {
        "op": "if_else",
        "args": [
            {"op": "gt", "args": [{"op": "col", "name": "d"},
                                  {"op": "lit", "value": 5000}]},
            {"op": "lit", "value": "far"},
            {"op": "if_else", "args": [
                {"op": "is_null", "args": [{"op": "col", "name": "c"}]},
                {"op": "lit", "value": "unknown"},
                {"op": "lit", "value": "near"}]},
        ],
    }
    r = col("x").round(2).to_tree()
    assert r == {"op": "round", "args": [{"op": "col", "name": "x"}], "ndigits": 2}
    c = col("x").cast("i64").to_tree()
    assert c == {"op": "cast", "args": [{"op": "col", "name": "x"}], "to": "i64"}


def test_bool_lifts_before_int():
    t = (col("f") == True).to_tree()  # noqa: E712 - the point of the test
    v = t["args"][1]["value"]
    assert v is True and isinstance(v, bool)
    t = (col("f") == 1).to_tree()
    v = t["args"][1]["value"]
    assert v == 1 and not isinstance(v, bool)


# ---------------------------------------------------------------------
# Trap guards & build-time errors
# ---------------------------------------------------------------------

def test_trap_guards():
    with pytest.raises(TypeError, match="ambiguous"):
        bool(col("a") > 1)
    with pytest.raises(TypeError, match="ambiguous"):
        1 < col("a") < 5  # noqa: B015 - chained comparison is the trap
    with pytest.raises(TypeError, match="bind TIGHTER"):
        col("a") > 10 & col("b")
    with pytest.raises(TypeError, match="bind TIGHTER"):
        5 | col("b")
    with pytest.raises(TypeError, match="is_null"):
        col("a") == None  # noqa: E711 - the point of the test
    with pytest.raises(TypeError, match="str_contains"):
        "x" in col("a")
    with pytest.raises(TypeError, match="!="):
        col("a") ^ col("b")
    with pytest.raises(TypeError, match="unhashable"):
        hash(col("a"))
    with pytest.raises(TypeError, match="must be an Expr"):
        dfilter({"a": [1]}, True)
    with pytest.raises(TypeError, match="must be an Expr"):
        if_else(True, 1, 2)


def test_literal_lifting_errors():
    with pytest.raises(ValueError, match="does not fit i64"):
        lit(2**63)
    with pytest.raises(ValueError, match="non-finite"):
        lit(float("nan"))
    with pytest.raises(ValueError, match="fill_null"):
        lit(None)
    with pytest.raises(ValueError, match="v1 subset"):
        col("a").cast("utf8")
    with pytest.raises(TypeError, match="ndigits"):
        col("a").round(1.5)
    with pytest.raises(TypeError, match="non-empty column name"):
        col(7)


def test_eval_error_voices():
    with pytest.raises(ValueError, match=r'missing column "nope" \(columns: x\)'):
        with_columns({"x": [1]}, z=col("nope"))
    with pytest.raises(TypeError, match=r"use \.concat\(\) for strings"):
        with_columns({"s": ["a"], "n": [1]}, z=col("s") + col("n"))
    with pytest.raises(ValueError, match="already exists"):
        with_columns({"x": [1]}, x=col("x") + 1)
    with pytest.raises(ValueError, match="reserved"):
        with_columns({"x": [1]}, handle=col("x"))
    with pytest.raises(ValueError, match=r'integer overflow.*cast\("f64"\)'):
        with_columns({"n": [2**62]}, z=col("n") * 4)
    with pytest.raises(ValueError, match="division by zero"):
        with_columns({"n": [1]}, z=col("n") // 0)
    with pytest.raises(TypeError, match="integer-only"):
        with_columns({"x": [1.0]}, z=col("x") // 2)
    with pytest.raises(ValueError, match="square root of a negative"):
        with_columns({"x": [-1.0]}, z=col("x").sqrt())
    with pytest.raises(TypeError, match="decimal columns are outside"):
        import decimal
        import pyarrow as pa
        t = pa.table({"d": pa.array([decimal.Decimal("1.5")], pa.decimal128(4, 2))})
        with_columns(t, z=col("d"))
    with pytest.raises(TypeError, match="predicate resolves to i64"):
        dfilter({"x": [1]}, col("x") + 1)
    with pytest.raises(TypeError, match="booleans have no order"):
        with_columns({"b": [True]}, z=col("b") > False)
    with pytest.raises(TypeError, match=r"\.cast\(\"f64\"\) first"):
        with_columns({"n": [1]}, z=col("n").round(2))


def test_missing_sibling_column_is_input_only():
    # polars semantics: with_columns exprs see the INPUT columns only
    with pytest.raises(ValueError, match='missing column "total"'):
        with_columns({"p": [1.0], "q": [2]},
                     total=col("p") * col("q"), big=col("total") > 10)
    # chaining works
    t = with_columns({"p": [1.0], "q": [2]}, total=col("p") * col("q"))
    t = with_columns(t, big=col("total") > 1.0)
    assert t.to_pylist()[0]["big"] is True


# ---------------------------------------------------------------------
# Semantics pins (values measured in docs/derive-expr-pins.md)
# ---------------------------------------------------------------------

def _one(expr, **cols):
    data = {k: [v] for k, v in cols.items()}
    t = with_columns(data, out=expr)
    return t.to_pylist()[0]["out"]


def test_division_family_pins():
    # / is true division, always f64 (§A)
    assert _one(col("a") / col("b"), a=7, b=2) == 3.5
    assert _one(col("a") / col("b"), a=-7, b=2) == -3.5
    # // truncates (§A: DuckDB/C; differs from python's floor)
    assert _one(col("a") // col("b"), a=-7, b=2) == -3
    assert _one(col("a") // col("b"), a=7, b=-2) == -3
    assert _one(col("a") // col("b"), a=-7, b=-2) == 3
    # % takes the dividend's sign (§A)
    assert _one(col("a") % col("b"), a=-7, b=3) == -1
    assert _one(col("a") % col("b"), a=7, b=-3) == 1
    # the div/mod identity holds
    for a, b in [(-7, 2), (7, -2), (-7, -3), (7, 3)]:
        q = _one(col("a") // col("b"), a=a, b=b)
        r = _one(col("a") % col("b"), a=a, b=b)
        assert q * b + r == a
    # float % is fmod (§A)
    assert _one(col("a") % col("b"), a=-7.5, b=3.0) == -1.5
    # float division by zero is IEEE (§B)
    assert _one(col("a") / col("b"), a=1.0, b=0.0) == float("inf")
    assert _one(col("a") / col("b"), a=-1.0, b=0.0) == float("-inf")
    assert math.isnan(_one(col("a") / col("b"), a=0.0, b=0.0))
    # int / 0 promotes to f64 first -> inf, matching the oracle's DOUBLE '/'
    assert _one(col("a") / col("b"), a=1, b=0) == float("inf")


def test_rounding_pins():
    # §E + ledger row 8: shortest-decimal, half away from zero
    vecs = [(2.5, 0, 3.0), (3.5, 0, 4.0), (-2.5, 0, -3.0), (0.5, 0, 1.0),
            (2.345, 2, 2.35), (2.675, 2, 2.68), (2.665, 2, 2.67),
            (0.125, 2, 0.13), (0.135, 2, 0.14), (-2.675, 2, -2.68),
            (1234.5, -2, 1200.0), (1250.0, -2, 1300.0)]
    for x, nd, want in vecs:
        assert _one(col("x").round(nd), x=x) == want, (x, nd)
        assert _round_shortest_decimal(x, nd) == want, (x, nd, "reference")
    # cast f64 -> i64 rounds HALF TO EVEN with range check (§D)
    assert _one(col("x").cast("i64"), x=2.5) == 2
    assert _one(col("x").cast("i64"), x=3.5) == 4
    assert _one(col("x").cast("i64"), x=-2.5) == -2
    assert _one(col("x").cast("i64"), x=2.6) == 3
    for bad in (1e300, float("nan"), float("inf")):
        with pytest.raises(ValueError):
            _one(col("x").cast("i64"), x=bad)
    # round on i64 at ndigits=0 is the identity
    assert _one(col("n").round(), n=7) == 7


def test_kleene_tables():
    tvals = [True, False, None]
    rows = [(a, b) for a in tvals for b in tvals]
    data = {"a": [r[0] for r in rows], "b": [r[1] for r in rows]}
    t = with_columns(data, k_and=col("a") & col("b"), k_or=col("a") | col("b"),
                     k_not=~col("a"))
    got = t.to_pylist()
    AND = {(True, True): True, (True, False): False, (True, None): None,
           (False, True): False, (False, False): False, (False, None): False,
           (None, True): None, (None, False): False, (None, None): None}
    OR = {(True, True): True, (True, False): True, (True, None): True,
          (False, True): True, (False, False): False, (False, None): None,
          (None, True): True, (None, False): None, (None, None): None}
    for r in got:
        key = (r["a"], r["b"])
        assert r["k_and"] == AND[key], ("and", key)
        assert r["k_or"] == OR[key], ("or", key)
        assert r["k_not"] == (None if r["a"] is None else not r["a"])


def test_null_propagation_vectors():
    t = with_columns(
        {"a": [1, None], "f": [1.5, None], "s": ["x", None], "b": [True, None]},
        add=col("a") + 1, mul=col("f") * 2.0, cmp=col("a") > 0,
        ln=col("s").str_len(), cc=col("s").concat("!"),
        ct=col("s").str_contains("x"), ab=col("a").abs(),
        isn=col("a").is_null(), inn=col("a").is_not_null(),
        fill=col("a").fill_null(9),
    )
    r0, r1 = t.to_pylist()
    assert (r0["add"], r0["mul"], r0["cmp"], r0["ln"], r0["cc"], r0["ct"], r0["ab"]) == \
        (2, 3.0, True, 1, "x!", True, 1)
    assert all(r1[k] is None for k in ("add", "mul", "cmp", "ln", "cc", "ct", "ab"))
    assert (r0["isn"], r1["isn"], r0["inn"], r1["inn"]) == (False, True, True, False)
    assert (r0["fill"], r1["fill"]) == (1, 9)


def test_if_else_null_condition_takes_otherwise():
    t = with_columns({"c": [True, False, None]},
                     z=if_else(col("c"), lit("then"), lit("else")))
    assert [r["z"] for r in t.to_pylist()] == ["then", "else", "else"]


def test_filter_semantics():
    # SQL WHERE: TRUE passes, FALSE and NULL drop (§L)
    t = dfilter({"v": [1, None, 5]}, col("v") > 2)
    assert [r["v"] for r in t.to_pylist()] == [5]
    # handle columns ride through for delete/update pipelines
    t = dfilter({"handle": [10, 11, 12], "v": [1.0, 8.0, 9.0]}, col("v") > 5.0)
    assert [r["handle"] for r in t.to_pylist()] == [11, 12]


def test_empty_batch_types_without_data():
    pl = pytest.importorskip("polars")
    empty = pl.DataFrame({"p": pl.Series([], dtype=pl.Float64)})
    t = with_columns(empty, d=col("p") * 2, s=lit("x"), b=col("p") > 0)
    assert len(t) == 0
    out = pl.DataFrame(t)
    assert out.schema["d"] == pl.Float64
    assert out.schema["s"] == pl.String
    assert out.schema["b"] == pl.Boolean


def test_scalar_broadcast_and_ordering():
    t = with_columns({"x": [1, 2]}, z=lit(9), y=col("x") * 2)
    assert [list(r) for r in map(dict.keys, t.to_pylist())] == [["x", "z", "y"]] * 2
    assert [r["z"] for r in t.to_pylist()] == [9, 9]


def test_determinism_byte_identical():
    exprs = dict(
        m=col("fa") * col("fb"), r=col("fnn").round(2),
        q=col("ia") % col("inn"), s=col("sa").concat(col("sb")),
        k=(col("ba") | col("bb")) & (col("fa") > 0),
    )
    a = with_columns(DATA, **exprs).to_pylist()
    b = with_columns(DATA, **exprs).to_pylist()
    assert a == b


# ---------------------------------------------------------------------
# The three-way differential battery
# ---------------------------------------------------------------------

def _error_kinds(tree, schema):
    """Count the DISTINCT error kinds a tree could raise (conservative:
    over-counting only skips the secondary kind-equality assert, never
    the strict does-it-error one)."""
    kinds = set()

    def walk(t):
        if not isinstance(t, dict):
            return
        for x in t.get("args", []):
            walk(x)
        op = t.get("op")
        if op in ("add", "sub", "mul", "neg", "abs"):
            kinds.add("overflow")  # i64 checked arithmetic
        elif op in ("floordiv", "rem"):
            kinds.update(("div0", "overflow"))
        elif op == "cast" and t.get("to") == "i64":
            kinds.add("cast")
        elif op in ("sqrt", "asin", "acos", "ln", "log10", "sin", "cos", "tan"):
            kinds.add("domain")
        elif op == "pow":
            kinds.update(("overflow", "domain"))

    walk(tree)
    return len(kinds)


def _three_way(expr, data, schema, duckdb):
    tree = expr.to_tree()
    rust = rust_eval(expr, data)
    ref = ref_eval(tree, data, schema)
    # Rust vs reference: strict on WHETHER it errors; the error KIND is
    # asserted only when the tree carries a single potential kind — with
    # two error sources (e.g. a div0 subtree and a sqrt-domain subtree)
    # the vectorized kernels surface the first erroring NODE while the
    # row-wise reference surfaces the first erroring ROW, and first-error
    # SELECTION is not certified surface (the ledger-9 eager-evaluation
    # family; flushed when the regex axis reshuffled the fuzz draws).
    if rust[0] == "err" or ref[0] == "err":
        assert rust[0] == "err" and ref[0] == "err", (repr(expr), rust, ref)
        if _error_kinds(tree, schema) <= 1:
            assert rust[1] == ref[1], (repr(expr), rust, ref)
    else:
        assert _values_match(rust[1], ref[1]), (repr(expr), rust[1], ref[1])
    # vs the oracle
    duck = duck_eval(duckdb, tree, data, schema)
    if rust[0] == "ok" and duck[0] == "ok":
        assert _values_match(rust[1], duck[1]), (repr(expr), rust[1], duck[1])
    elif rust[0] == "err" and duck[0] == "ok":
        # ledger rows 4/6/9: the kernels error the whole BATCH loudly
        # where the oracle yields per-ROW NULL (// and % by zero) or
        # evaluates branches LAZILY (COALESCE/CASE skip errors in rows
        # that never select the branch — our fill_null/if_else are
        # vectorized-eager)
        assert rust[1] in ("div0", "domain", "overflow"), (repr(expr), rust, duck)
    # rust ok + duck err: only the documented MIN%-1 class; the fuzz
    # pools cannot produce it (no i64::MIN literal/column value)


def test_regex_vector_pins():
    # pins §N verbatim (ASCII rows agree three ways; the pins doc holds
    # the measured DuckDB values)
    cases = [  # (string, pattern, full, expected)
        ("abc123", "[0-9]+", False, True), ("abc123", "[0-9]+", True, False),
        ("123", "[0-9]+", True, True), ("abc", "^b", False, False),
        ("abc", "^a", False, True), ("abc", "c$", False, True),
        ("1234", "^[0-9]{3}$", False, False), ("123", "[0-9]{3}", True, True),
        ("xxabcdxx", "(ab|cd)+", False, True), ("aBc", "(?i)abc", False, True),
        ("ABC", "(?i)abc", True, True), ("a.c", "a\\.c", False, True),
        ("abc", "a\\.c", False, False), ("a3", "\\d", False, True),
        ("foo bar", "\\bbar\\b", False, True), ("éx", ".", False, True),
        ("\n", ".", False, False), ("abc", "", False, True), ("", "", True, True),
        ("É", "(?i)é", False, True), ("straße", "(?i)STRASSE", False, False),
    ]
    for s, p, full, want in cases:
        e = col("a").regexp_full_match(p) if full else col("a").regexp_matches(p)
        assert _one(e, a=s) == want, (s, p, full)
        ref = re.fullmatch(p, s) if full else re.search(p, s)
        assert (ref is not None) == want, ("reference disagrees", s, p)
    # ledger row 12, pinned POSITIVE: kernel + reference are
    # Unicode-aware on perl classes where RE2/DuckDB is ASCII
    # (regexp_matches('٣', '\d') measured False in §N)
    assert _one(col("a").regexp_matches("\\d"), a="٣") is True
    assert re.search("\\d", "٣") is not None
    # null in -> null out (§H doctrine; DATA's sa[2] is None)
    st, vals = rust_eval(col("sa").regexp_matches("x"), DATA)
    assert st == "ok" and vals[2] is None
    # invalid patterns are LOUD at expression build — the same three
    # classes the oracle errors on (§N)
    for bad in ["(", "(?=a)", "(a)\\1"]:
        with pytest.raises(Exception, match="invalid regex pattern"):
            _one(col("a").regexp_matches(bad), a="x")
    # the pattern is a literal, never a column
    with pytest.raises(TypeError, match="literal str pattern"):
        col("a").regexp_matches(col("b"))


def test_curated_three_way():
    duckdb = _duck()
    exprs = [
        col("ia") + col("ib"), col("ia") - col("inn"), col("ia") * col("ib"),
        col("ia") / col("ib"), col("fa") + col("ib"), col("fa") * col("fb"),
        col("ia") % col("inn"), col("fa") % col("fnn"),
        col("fa").abs(), (-col("ia")).abs(), col("fnn").floor(), col("fnn").ceil(),
        col("fnn").round(1), col("fnn").round(-1), col("fa").fill_null(0.0),
        col("ia").fill_null(col("ib")), col("fa").cast("i64"), col("ia").cast("f64"),
        col("ia") > col("ib"), col("fa") <= col("fb"), col("sa") == col("sb"),
        col("sa") < col("sb"), col("ba") & col("bb"), col("ba") | ~col("bb"),
        col("ia").is_null(), col("sa").str_len(), col("sa").concat(col("sb")),
        col("sa").str_contains(col("sb")), col("sa").str_starts_with("a"),
        col("sa").str_ends_with("c"),
        col("sa").regexp_matches("[ab]"), col("sa").regexp_matches("^a"),
        col("sa").regexp_full_match("[a-z]*"), col("sa").regexp_matches("b\\|"),
        col("sa").regexp_matches(""), col("sa").regexp_full_match("(a|b)+c?"),
        col("sb").regexp_matches("(?i)B.*"), col("sa").regexp_matches("[0-9]{1,2}"),
        if_else(col("ba"), col("ia"), col("ib")),
        if_else(col("fa") > 0, lit("pos"), lit("nonpos")),
        (col("ia") + col("ib")) * col("inn") - 3,
        col("inn") ** 2,
        col("fnn").sin(), col("fnn").cos(), col("fnn").tan(),
        col("fa").atan(), col("inn").atan(), col("fnn").exp(),
        col("fnn").degrees(), col("fa").radians(),
        col("fnn").abs().fill_null(1.0).ln(),
        (col("inn").abs() * 100).log10(),
        (col("fa").abs() / 10.0).asin(),
        col("fnn").sin() ** 2 + col("fnn").cos() ** 2,
    ]
    for e in exprs:
        _three_way(e, DATA, SCHEMA, duckdb)


# -- typed fuzzer ------------------------------------------------------

INT_COLS = ["ia", "ib", "inn"]
F64_COLS = ["fa", "fb", "fnn"]
BOOL_COLS = ["ba", "bb"]
STR_COLS = ["sa", "sb"]
INT_LITS = [-3, -1, 0, 1, 2, 7]
F64_LITS = [-1.5, 0.0, 0.5, 2.5]
STR_LITS = ["", "a", "b|", "z"]
# regex fuzz pool: ASCII core where all three dialects agree (no perl
# classes — those are vector-pinned; ledger row 12 covers the split)
REGEX_POOL = ["a", "^a", "c$", "[ab]", "[^ab]", "[a-c]x?", "a.c", "(a|b)+",
              "b\\|", "[0-9]{1,2}", "a*", "(?i)ab", "x|y|z", ""]


def gen_expr(rng, want, depth):
    leaf_p = 0.25 + 0.25 * depth
    if rng.random() < leaf_p or depth >= 4:
        if want == "i64":
            return col(rng.choice(INT_COLS)) if rng.random() < 0.7 else lit(rng.choice(INT_LITS))
        if want == "f64":
            return col(rng.choice(F64_COLS)) if rng.random() < 0.7 else lit(rng.choice(F64_LITS))
        if want == "bool":
            return col(rng.choice(BOOL_COLS)) if rng.random() < 0.8 else lit(rng.random() < 0.5)
        return col(rng.choice(STR_COLS)) if rng.random() < 0.7 else lit(rng.choice(STR_LITS))
    g = lambda w: gen_expr(rng, w, depth + 1)  # noqa: E731
    num = lambda: g(rng.choice(["i64", "f64"]))  # noqa: E731
    if want == "i64":
        k = rng.choice(["add", "sub", "mul", "floordiv", "rem", "neg", "abs",
                        "if_else", "fill_null", "cast", "str_len"])
        a, b = g("i64"), g("i64")
        if k == "add":
            return a + b
        if k == "sub":
            return a - b
        if k == "mul":
            return a * b
        if k == "floordiv":
            return a // b
        if k == "rem":
            return a % b
        if k == "neg":
            return -a
        if k == "abs":
            return a.abs()
        if k == "if_else":
            return if_else(g("bool"), a, b)
        if k == "fill_null":
            return a.fill_null(b)
        if k == "cast":
            return num().cast("i64")
        return g("utf8").str_len()
    if want == "f64":
        k = rng.choice(["add", "sub", "mul", "div", "rem", "pow", "sqrt",
                        "floor", "ceil", "round", "neg", "abs", "if_else",
                        "fill_null", "cast", "sin", "cos", "tan", "atan",
                        "exp", "degrees", "radians", "asin", "ln"])
        a, b = g("f64"), num()
        if k == "add":
            return a + b
        if k == "sub":
            return a - b
        if k == "mul":
            return a * b
        if k == "div":
            return num() / num()
        if k == "rem":
            return a % g("f64")
        if k == "pow":
            return a ** lit(rng.choice([1, 2, 0.5]))
        if k == "sqrt":
            return a.abs().sqrt() if rng.random() < 0.7 else a.sqrt()
        if k in ("sin", "cos", "tan", "atan", "exp", "degrees", "radians"):
            return getattr(a, k)()
        if k == "asin":
            return a.asin()  # domain errors compare three-way
        if k == "ln":
            return a.ln()
        if k == "floor":
            return a.floor()
        if k == "ceil":
            return a.ceil()
        if k == "round":
            return a.round(rng.choice([-1, 0, 1, 2]))
        if k == "neg":
            return -a
        if k == "abs":
            return a.abs()
        if k == "if_else":
            return if_else(g("bool"), a, g("f64"))
        if k == "fill_null":
            return a.fill_null(b)
        return num().cast("f64")
    if want == "bool":
        k = rng.choice(["cmp_num", "cmp_str", "cmp_bool", "and", "or", "not",
                        "is_null", "str_pred", "regex", "if_else", "fill_null"])
        if k == "cmp_num":
            sym = rng.choice(["__lt__", "__le__", "__gt__", "__ge__", "__eq__", "__ne__"])
            return getattr(num(), sym)(num())
        if k == "cmp_str":
            sym = rng.choice(["__lt__", "__gt__", "__eq__", "__ne__"])
            return getattr(g("utf8"), sym)(g("utf8"))
        if k == "cmp_bool":
            sym = rng.choice(["__eq__", "__ne__"])
            return getattr(g("bool"), sym)(g("bool"))
        if k == "and":
            return g("bool") & g("bool")
        if k == "or":
            return g("bool") | g("bool")
        if k == "not":
            return ~g("bool")
        if k == "is_null":
            e = g(rng.choice(["i64", "f64", "utf8", "bool"]))
            return e.is_null() if rng.random() < 0.5 else e.is_not_null()
        if k == "str_pred":
            m = rng.choice(["str_contains", "str_starts_with", "str_ends_with"])
            return getattr(g("utf8"), m)(g("utf8"))
        if k == "regex":
            # ASCII core only: perl classes (\d \b ...) are vector-pinned —
            # over non-ASCII data they are the ledger-12 dialect split
            m = rng.choice(["regexp_matches", "regexp_full_match"])
            return getattr(g("utf8"), m)(rng.choice(REGEX_POOL))
        if k == "if_else":
            return if_else(g("bool"), g("bool"), g("bool"))
        return g("bool").fill_null(g("bool"))
    # utf8
    k = rng.choice(["concat", "if_else", "fill_null", "leaf"])
    if k == "concat":
        return g("utf8").concat(g("utf8"))
    if k == "if_else":
        return if_else(g("bool"), g("utf8"), g("utf8"))
    if k == "fill_null":
        return g("utf8").fill_null(g("utf8"))
    return col(rng.choice(STR_COLS))


@pytest.mark.parametrize("seed", range(20))
def test_fuzz_three_way(seed):
    duckdb = _duck()
    rng = random.Random(seed)
    for _ in range(25):
        want = rng.choice(["i64", "f64", "bool", "utf8"])
        expr = gen_expr(rng, want, 0)
        _three_way(expr, DATA, SCHEMA, duckdb)


def test_fuzz_filter_vs_where():
    duckdb = _duck()
    rng = random.Random(424242)
    kept = 0
    for _ in range(60):
        expr = gen_expr(rng, "bool", 0)
        rust = None
        try:
            t = dfilter(DATA, expr)
            rust = [r["inn"] for r in t.to_pylist()]
        except ValueError:
            continue  # int div0 etc. — covered by the three-way battery
        con = duckdb.connect()
        cols = ", ".join(f'"{k}" {SQLT[SCHEMA[k]]}' for k in DATA)
        con.sql(f"CREATE TABLE t (idx BIGINT, {cols})")
        n = len(DATA["inn"])
        for i in range(n):
            vals = ", ".join(_sql_val(DATA[k][i], SCHEMA[k]) for k in DATA)
            con.sql(f"INSERT INTO t VALUES ({i}, {vals})")
        try:
            rows = con.sql(
                f'SELECT "inn" FROM t WHERE {to_sql(expr.to_tree())} ORDER BY idx'
            ).fetchall()
        except Exception:
            continue
        assert rust == [r[0] for r in rows], repr(expr)
        kept += 1
    assert kept >= 30  # the sweep must actually exercise the comparison


def test_fuzz_ieee_side_battery():
    # NaN/inf as column DATA: Rust vs reference only (ledger row 1 —
    # the oracle orders NaN as a value)
    data = {
        "fa": [float("nan"), float("inf"), float("-inf"), 1.0, None],
        "fb": [1.0, float("nan"), 2.0, float("inf"), 0.5],
        "fnn": [-0.0, 0.0, float("inf"), -2.5, float("nan")],
    }
    schema = {"fa": "f64", "fb": "f64", "fnn": "f64"}
    rng = random.Random(31415)
    checked = 0
    for _ in range(40):
        want = rng.choice(["f64", "bool"])
        expr = gen_expr(rng, want, 1)
        tree = expr.to_tree()
        if any(c in repr(expr) for c in ("ia", "ib", "inn", "ba", "bb", "sa", "sb")):
            continue
        rust = rust_eval(expr, data)
        ref = ref_eval(tree, data, schema)
        assert rust[0] == ref[0], (repr(expr), rust, ref)
        if rust[0] == "ok":
            assert _values_match(rust[1], ref[1]), (repr(expr), rust[1], ref[1])
        else:
            assert rust[1] == ref[1], repr(expr)
        checked += 1
    assert checked >= 10
    # NaN comparisons are IEEE
    assert _one(col("x") == col("x"), x=float("nan")) is False
    assert _one(col("x") != col("x"), x=float("nan")) is True


def test_sentinel_pins_still_measure_true():
    """Drift alarm: the values RefEval hardcodes re-measured live."""
    duckdb = _duck()
    con = duckdb.connect()
    sentinels = [
        ("CAST(-7 AS BIGINT) // CAST(2 AS BIGINT)", -3),
        ("CAST(-7 AS BIGINT) % CAST(3 AS BIGINT)", -1),
        ("round(CAST(2.675 AS DOUBLE), 2)", 2.68),
        ("round(CAST(1250.0 AS DOUBLE), -2)", 1300.0),
        ("CAST(CAST(2.5 AS DOUBLE) AS BIGINT)", 2),
        ("CAST(CAST(3.5 AS DOUBLE) AS BIGINT)", 4),
        ("CASE WHEN CAST(NULL AS BOOLEAN) THEN 'a' ELSE 'b' END", "b"),
        ("CAST(1.0 AS DOUBLE) / CAST(0.0 AS DOUBLE)", float("inf")),
        ("sin(CAST(0.5 AS DOUBLE))", 0.479425538604203),
        ("ln(CAST(2.718281828459045 AS DOUBLE))", 1.0),
        ("degrees(CAST(3.141592653589793 AS DOUBLE))", 180.0),
    ]
    for sql, want in sentinels:
        got = con.sql(f"SELECT {sql}").fetchall()[0][0]
        assert got == want, (sql, got, want)


def test_polars_cross_check():
    pl = pytest.importorskip("polars")
    df = pl.DataFrame(
        {k: v for k, v in DATA.items() if k in ("ia", "ib", "fa", "fb", "ba", "sa", "sb")},
        schema={"ia": pl.Int64, "ib": pl.Int64, "fa": pl.Float64, "fb": pl.Float64,
                "ba": pl.Boolean, "sa": pl.String, "sb": pl.String},
    )
    cases = [
        (col("fa") * col("fb") + 1, pl.col("fa") * pl.col("fb") + 1),
        (col("ia") + col("ib"), pl.col("ia") + pl.col("ib")),
        ((col("fa") > 0) & col("ba"), (pl.col("fa") > 0) & pl.col("ba")),
        (col("fa").abs().sqrt(), pl.col("fa").abs().sqrt()),
        (col("sa").str_contains(col("sb")), pl.col("sa").str.contains(pl.col("sb"), literal=True)),
        (col("fa").fill_null(0.0), pl.col("fa").fill_null(0.0)),
        (if_else(col("ba"), col("ia"), col("ib")),
         pl.when(pl.col("ba").fill_null(False)).then(pl.col("ia")).otherwise(pl.col("ib"))),
    ]
    for ours, theirs in cases:
        got = [r["z"] for r in with_columns(df, z=ours).to_pylist()]
        want = df.select(z=theirs)["z"].to_list()
        assert _values_match(got, want), (repr(ours), got, want)


def test_derived_batch_feeds_session():
    # the full pipeline: derive a column, filter the garbage rows
    # (nulls are the EXPRESSION plane's to handle — the match plane
    # rejects them at insert), assert, fire on the computed field
    raw = {"price": [2.5, 40.0, None], "qty": [4, 3, 2]}
    t = with_columns(raw, total=col("price") * col("qty"))
    t = dfilter(t, col("total").is_not_null())
    assert len(t) == 2
    drl = 'rule "Big"\nwhen\n    Order(total > 100.0)\nthen\nend\n'
    res = seine_rs.run(drl, {"Order": t})
    assert len(res.firings) == 1
    assert '"total":120.0' in res.firings.to_pylist()[0]["values_json"]


def test_dict_ingest_nulls_and_inference():
    t = with_columns({"a": [1, None, 3]}, b=col("a") + 1)
    assert [r["b"] for r in t.to_pylist()] == [2, None, 4]
    # int -> float promotion within a column
    t = with_columns({"a": [1, 2.5]}, b=col("a") * 2)
    assert [r["b"] for r in t.to_pylist()] == [2.0, 5.0]
    with pytest.raises(ValueError, match="cannot infer a type"):
        with_columns({"a": [None, None]}, b=col("a").is_null())
    with pytest.raises(ValueError, match="ragged"):
        with_columns({"a": [1, 2], "b": [1]}, z=col("a"))
    with pytest.raises(TypeError, match="mixed value types"):
        with_columns({"a": [1, "x"]}, z=col("a"))


def test_depth_cap():
    e = col("x")
    for _ in range(300):
        e = e + 1
    with pytest.raises(ValueError, match="nesting exceeds"):
        with_columns({"x": [1]}, z=e)
