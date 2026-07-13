"""Python-surface ergonomics (D-213): Table conversion methods and the
uniform property-style Result accessors."""
import polars as pl
import pyarrow as pa
import pytest

import seine_rs


DRL = """
rule "R"
when
    $p : P(v > 1)
then
    insert(new Out($p.getV()));
end
"""


@pytest.fixture()
def res():
    return seine_rs.run(DRL, {
        "P": pa.table({"v": [1, 2, 3]}),
        "Out": pa.table({"v": pa.array([], pa.int64())}),
    })


def test_table_to_arrow(res):
    t = res.derived["Out"]
    at = t.to_arrow()
    assert isinstance(at, pa.Table)
    assert at.num_rows == len(t)


def test_table_to_polars(res):
    df = res.derived["Out"].to_polars()
    assert isinstance(df, pl.DataFrame)
    assert df.height == 2


def test_table_discoverable(res):
    t = res.firings
    assert "to_arrow" in dir(t) and "to_polars" in dir(t)
    assert "seine_rs.Table" in repr(t)


def test_result_uniform_properties(res):
    # every accessor is an attribute — no bound-method reprs
    assert isinstance(res.fired, int)
    assert isinstance(res.firings, seine_rs.Table)
    assert isinstance(res.derived, dict)
    assert isinstance(res.facts, dict)
    assert isinstance(res.deleted_handles, list)
    assert "seine_rs.Result" in repr(res)


def test_version_present():
    assert isinstance(seine_rs.__version__, str)
    assert seine_rs.__version__.count(".") == 2


def test_certification_interrogable():
    c = seine_rs.certification()
    assert "9.44.0.Final" in c["oracle"]
    assert c["corpus_baseline"] > 0
    assert c["corpus_probes"] > 0
    assert c["corpus_regressions"] > 0
    assert c["engine_version"] == seine_rs.__version__
    assert isinstance(c["commit"], str) and c["commit"]
    # the payload is self-describing: scope names the gate, marks the
    # quarantine as NOT certified, and excludes WIP instruments
    assert "make diff" in c["scope"]
    assert "NOT certified" in c["scope"]
    assert "probes_pending" in c["scope"]


def test_py_typed_marker():
    import pathlib
    pkg = pathlib.Path(seine_rs.__file__).parent
    assert (pkg / "py.typed").exists()


def test_table_to_pylist(res):
    rows = res.derived["Out"].to_pylist()
    assert isinstance(rows, list) and len(rows) == 2
    assert "handle" in rows[0]


def test_handle_column_named_plainly(res):
    cols = res.derived["Out"].to_arrow().column_names
    assert "handle" in cols and "_handle" not in cols


def test_no_tracker_ids_in_public_docs():
    import re
    texts = [seine_rs.__doc__ or ""]
    for name in dir(seine_rs):
        if name.startswith("_"):
            continue
        obj = getattr(seine_rs, name)
        texts.append(getattr(obj, "__doc__", "") or "")
    offenders = [t[:60] for t in texts if re.search(r"\bD-\d{2,3}\b", t)]
    assert not offenders, offenders


RESERVED_DRL = """
rule "R"
when
    P(v > 0)
then
end
"""


def test_handle_field_rejected_at_fact():
    # `handle` is the engine's result column; a user field with that
    # name would duplicate it and collapse silently in to_pylist/polars
    with pytest.raises(seine_rs.CompileError, match="reserved"):
        @seine_rs.fact
        class Bad:
            handle: int
            v: int


def test_handle_field_rejected_in_dict_table():
    with pytest.raises(Exception, match="reserved"):
        seine_rs.run(RESERVED_DRL, {"P": {"handle": [1], "v": [1]}})


def test_handle_field_rejected_in_arrow_table():
    with pytest.raises(Exception, match="reserved"):
        seine_rs.run(RESERVED_DRL, {"P": pa.table({"handle": [1], "v": [1]})})


@seine_rs.fact
class NV:
    tag: str
    v: "int | None"


def test_to_pylist_is_dependency_free(res, monkeypatch):
    # the clean-install contract: a zero-dep wheel must be able to READ
    # its own results — to_pylist works with pyarrow AND polars absent
    import sys
    monkeypatch.setitem(sys.modules, "pyarrow", None)
    monkeypatch.setitem(sys.modules, "polars", None)
    rows = res.derived["Out"].to_pylist()
    assert len(rows) == 2 and "handle" in rows[0] and rows[0]["v"] in (2, 3)


def test_optional_dep_errors_are_actionable(res, monkeypatch):
    import sys
    monkeypatch.setitem(sys.modules, "pyarrow", None)
    with pytest.raises(ModuleNotFoundError, match=r"pip install pyarrow"):
        res.derived["Out"].to_arrow()
    monkeypatch.setitem(sys.modules, "polars", None)
    with pytest.raises(ModuleNotFoundError, match=r"pip install polars"):
        res.derived["Out"].to_polars()


def test_to_pylist_native_fidelity():
    # every result dtype: i64, f64, bool, str, decimal, null
    import decimal
    r = seine_rs.run(
        'rule R when P(v > 0) then end',
        {"P": {"v": [1], "f": [1.5], "b": [True], "s": ["x"]}},
    )
    row = r.facts["P"].to_pylist()[0]
    assert row["v"] == 1 and row["f"] == 1.5 and row["b"] is True and row["s"] == "x"

    rule = seine_rs.Rule("n")
    rule.when(NV)
    s = seine_rs.Session([rule], facts={NV: {"tag": ["a"], "v": [None]}})
    nrow = s.fire().facts["NV"].to_pylist()[0]
    assert nrow["v"] is None and nrow["tag"] == "a"

    d = seine_rs.run(
        'rule D when M() then end',
        {"M": pa.table({"amt": pa.array([decimal.Decimal("1.50")], pa.decimal128(10, 2))})},
    )
    drow = d.facts["M"].to_pylist()[0]
    assert drow["amt"] == decimal.Decimal("1.50") and isinstance(drow["amt"], decimal.Decimal)
