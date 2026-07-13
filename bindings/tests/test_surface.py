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
