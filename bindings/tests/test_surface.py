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
