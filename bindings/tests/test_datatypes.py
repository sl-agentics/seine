# D-098 phase 5: the ratified typing-module authoring surface + the
# Arrow/row boundary for nulls (SQL 3VL) and exact decimals.
#
# This file deliberately enables PEP 563 — the shipped 0.2.0 @fact broke
# under it (raw __annotations__ saw strings); get_type_hints fixes it.
from __future__ import annotations

from decimal import Decimal
from typing import Annotated, Optional

import pytest
import seine_rs


# --- the PEP-563 regression: plain types under stringized annotations ---
def test_fact_works_under_pep563():
    @seine_rs.fact
    class Plain:
        name: str
        age: int

    assert Plain.__seine_fields__ == {"name": "String", "age": "i64"}


# --- ratified point 2: Optional/Annotated normalize at any nesting ---
def test_optional_and_annotated_compose():
    @seine_rs.fact
    class Loan:
        balance: Annotated[Decimal, seine_rs.Decimal(18, 2)]
        rate: Optional[Annotated[Decimal, seine_rs.Decimal(9, 6)]]
        note: Annotated[Optional[Decimal], seine_rs.Decimal(10, 2)]
        score: Optional[float]
        age: int | None

    assert Loan.__seine_fields__ == {
        "balance": "decimal(18,2)",
        "rate": "decimal(9,6)?",
        "note": "decimal(10,2)?",
        "score": "f64?",
        "age": "i64?",
    }


# --- ratified point 1 (EMPHATIC): bare Decimal is a loud error ---
def test_bare_decimal_names_the_fix():
    with pytest.raises(seine_rs.CompileError) as ei:
        @seine_rs.fact
        class Bad:
            amount: Decimal

    msg = str(ei.value)
    assert "Annotated[Decimal, seine_rs.Decimal(p, s)]" in msg
    assert "walled" in msg


# --- ratified point 4: marker validates at construction ---
def test_marker_validation():
    with pytest.raises(seine_rs.CompileError):
        seine_rs.Decimal(39, 0)
    with pytest.raises(seine_rs.CompileError):
        seine_rs.Decimal(10, 11)
    with pytest.raises(seine_rs.CompileError):
        seine_rs.Decimal(0, 0)


def test_null_3vl_end_to_end():
    sess = seine_rs.Session(
        "rule R when T(v > 2) then insert(new Out()); end",
        schemas={"T": {"v": "i64?", "w": "i64"}, "Out": {}},
    )
    sess.insert_row("T", {"v": 1, "w": 0})
    sess.insert_row("T", {"v": None, "w": 0})
    sess.insert_row("T", {"v": 5, "w": 0})
    res = sess.fire()
    fdf = __import__("polars").DataFrame(res.firings())
    assert fdf["seq"].n_unique() == 1  # pin D: only v=5 (UNKNOWN excluded)


def test_none_rejected_for_non_nullable():
    sess = seine_rs.Session(
        "rule R when T(v > 2) then end",
        schemas={"T": {"v": "i64"}},
    )
    with pytest.raises(ValueError) as ei:
        sess.insert_row("T", {"v": None})
    assert "Optional" in str(ei.value)


# --- ratified point 5 (EMPHATIC): the declaration IS the NaN choice ---
def test_nan_normalizes_to_null_for_nullable_floats():
    sess = seine_rs.Session(
        "rule R when T(x > 0.0) then end\n"
        "rule S when T(x == null) then end",
        schemas={"T": {"x": "f64?"}},
    )
    sess.insert_row("T", {"x": float("nan")})
    sess.insert_row("T", {"x": 1.5})
    res = sess.fire()
    fdf = __import__("polars").DataFrame(res.firings())
    by_rule = sorted(fdf["rule"].to_list())
    # NaN became NULL: R fires once (1.5); S fires once (the NaN row)
    assert by_rule == ["R", "S"]


def test_nan_stays_a_value_for_bare_floats():
    sess = seine_rs.Session(
        "rule R when T(x != 0.0) then end",
        schemas={"T": {"x": "f64"}},
    )
    sess.insert_row("T", {"x": float("nan")})
    res = sess.fire()
    # bit-exact NaN is a VALUE (certified D-044): != 0.0 is Java-false
    # for NaN comparisons in the engine (ord None) — zero firings, and
    # crucially NO null conversion happened (insert succeeded).
    fdf = __import__("polars").DataFrame(res.firings())
    assert fdf.height == 0


def test_decimal_round_trip_and_wall():
    sess = seine_rs.Session(
        "rule R when M(amount == 1.25) then end",
        schemas={"M": {"amount": "decimal(10,2)"}},
    )
    sess.insert_row("M", {"amount": Decimal("1.25")})
    sess.insert_row("M", {"amount": Decimal("1.005")})  # half-up -> 1.01
    res = sess.fire()
    fdf = __import__("polars").DataFrame(res.firings())
    assert fdf["seq"].n_unique() == 1

    with pytest.raises(TypeError) as ei:
        sess.insert_row("M", {"amount": 1.25})  # float -> decimal walled
    assert "decimal.Decimal" in str(ei.value)

    with pytest.raises(ValueError):
        sess.insert_row("M", {"amount": Decimal("123456789.00")})  # p=10 overflow


def test_arrow_ingest_nullable_and_decimal():
    pa = pytest.importorskip("pyarrow")
    tbl = pa.table({
        "v": pa.array([1, None, 5], type=pa.int64()),
        "x": pa.array([1.5, float("nan"), None], type=pa.float64()),
        "amount": pa.array([Decimal("1.25"), Decimal("2.50"), None],
                           type=pa.decimal128(10, 2)),
    })
    sess = seine_rs.Session(
        "rule R when T(v > 2) then end\n"
        "rule S when T(x == null) then end\n"
        "rule Q when T(amount == 1.25) then end",
        facts={"T": tbl},
        schemas={"T": {"v": "i64?", "x": "f64?", "amount": "decimal(10,2)?"}},
    )
    res = sess.fire()
    fdf = __import__("polars").DataFrame(res.firings())
    by_rule = sorted(fdf["rule"].to_list())
    # R: v=5 only; S: the NaN row AND the None row (both NULL); Q: 1.25
    assert by_rule == ["Q", "R", "S", "S"]


def test_arrow_null_still_rejected_when_not_nullable():
    pa = pytest.importorskip("pyarrow")
    tbl = pa.table({"v": pa.array([1, None], type=pa.int64())})
    with pytest.raises(Exception) as ei:
        seine_rs.Session("rule R when T(v > 0) then end",
                      facts={"T": tbl}, schemas={"T": {"v": "i64"}})
    assert "null" in str(ei.value).lower()


def test_results_round_trip_nulls_and_decimals():
    pl = pytest.importorskip("polars")
    sess = seine_rs.Session(
        "rule R when M(amount > 1) then insert(new M(0.99, null)); end",
        schemas={"M": {"amount": "decimal(10,2)", "opt": "i64?"}},
    )
    sess.insert_row("M", {"amount": Decimal("2.00"), "opt": None})
    res = sess.fire()
    df = pl.DataFrame(res.facts()["M"])
    assert str(df.schema["amount"]).startswith("Decimal")
    vals = sorted(str(v) for v in df["amount"].to_list())
    assert vals == ["0.99", "2.00"]
    assert df["opt"].null_count() == 2
