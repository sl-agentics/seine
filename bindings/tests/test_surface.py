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

@seine_rs.fact
class OvAcct:
    v: int


def test_int_overflow_names_the_overflow_not_the_schema():
    # a Python int past i64 must fail AS an overflow at ingestion —
    # not decay to float and resurface as "table schema differs"
    with pytest.raises(ValueError, match="does not fit a 64-bit") as ei:
        seine_rs.run("rule R when P(v > 0) then end", {"P": {"v": [2**63]}})
    assert "schema differs" not in str(ei.value)
    r = seine_rs.Rule("r")
    r.when(OvAcct)
    with pytest.raises(ValueError, match="does not fit a 64-bit") as ei:
        seine_rs.Session([r], facts={OvAcct: {"v": [2**63]}})
    assert "schema differs" not in str(ei.value)


def test_int_boundaries_still_ingest():
    res = seine_rs.run(
        "rule R when P(v != 0) then end",
        {"P": {"v": [2**63 - 1, -(2**63)]}},
    )
    vals = sorted(row["v"] for row in res.facts["P"].to_pylist())
    assert vals == [-(2**63), 2**63 - 1]
    assert res.fired == 2


@seine_rs.fact
class OvF:
    f: float


def test_big_int_promotes_into_declared_float_field():
    r = seine_rs.Rule("r")
    r.when(OvF)
    s = seine_rs.Session([r], facts={OvF: {"f": [1.0]}})
    s.insert_row(OvF, {"f": 2**63})
    rows = s.fire().facts["OvF"].to_pylist()
    assert float(2**63) in [row["f"] for row in rows]


@seine_rs.fact
class RowT:
    a: int
    b: int


def test_insert_row_unknown_field_rejected():
    # the one schema violation this path accepted silently (round 9 C)
    r = seine_rs.Rule("r")
    r.when(RowT)
    s = seine_rs.Session([r], facts={RowT: {"a": [1], "b": [2]}})
    with pytest.raises(ValueError, match="unknown field c"):
        s.insert_row(RowT, {"a": 1, "b": 2, "c": 3})

# --- the WM-delta symmetry (round 10): TMS retractions are observable --

@seine_rs.fact
class Hot:
    sensor: int


@seine_rs.fact
class Alarm:
    sensor: int


def _alarm_session(hots):
    r = seine_rs.Rule("alarm-while-hot")
    h = r.when(Hot)
    r.then_insert_logical(Alarm, sensor=h.sensor)
    sess = seine_rs.Session([r], {Hot: hots, Alarm: []})
    return sess, sess.fire()


def test_tms_retraction_reaches_deleted_handles():
    # a logical fact leaving WM must be as observable as one entering it:
    # the cascade of a between-fire delete() lands on the NEXT fire
    sess, res = _alarm_session([Hot(7)])
    hot = res.facts["Hot"].to_pylist()[0]["handle"]
    alarm = res.facts["Alarm"].to_pylist()[0]["handle"]
    sess.delete(hot)
    res2 = sess.fire()
    assert res2.facts["Alarm"].to_pylist() == []
    assert alarm in res2.deleted_handles
    assert hot not in res2.deleted_handles  # Python's own action, not echoed


def test_session_delete_returns_the_cascade():
    sess, res = _alarm_session([Hot(7)])
    hot = res.facts["Hot"].to_pylist()[0]["handle"]
    alarm = res.facts["Alarm"].to_pylist()[0]["handle"]
    assert sess.delete(hot) == [alarm]


def test_shared_justification_cascades_only_on_last_premise():
    sess, res = _alarm_session([Hot(7), Hot(7)])
    hots = [x["handle"] for x in res.facts["Hot"].to_pylist()]
    alarm = res.facts["Alarm"].to_pylist()[0]["handle"]
    assert sess.delete(hots[0]) == []      # still justified by the other
    assert sess.delete(hots[1]) == [alarm]
    assert sess.fire().deleted_handles == [alarm]


def test_update_driven_retraction_reaches_deleted_handles():
    r = seine_rs.Rule("alarm-while-hot")
    h = r.when(Hot, Hot.sensor > 0)
    r.then_insert_logical(Alarm, sensor=h.sensor)
    sess = seine_rs.Session([r], {Hot: [Hot(7)], Alarm: []})
    res = sess.fire()
    hot = res.facts["Hot"].to_pylist()[0]["handle"]
    alarm = res.facts["Alarm"].to_pylist()[0]["handle"]
    sess.update(hot, sensor=-1)
    res2 = sess.fire()
    assert res2.facts["Alarm"].to_pylist() == []
    assert alarm in res2.deleted_handles

# --- the epoch-shape guard (round 21) -----------------------------------

@seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=1000))
class EvA:
    ts: int
    k: int


@seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=100000))
class EvB:
    ts: int
    k: int


def _tj_session():
    r = seine_rs.Rule("R")
    a = r.when(EvA)
    r.when(EvB, EvB.k == a.k, seine_rs.this_after(a, 0, 1000))
    return seine_rs.Session([r], {EvA: [], EvB: []})


def test_prefire_actions_walled_inserts_allowed():
    # pre-fire external actions ran against the engine's staging batch —
    # a shape no certified scenario produces (round 21: it flipped a
    # temporal-join-under-expiry outcome vs the oracle). Inserts stay
    # legal: they ARE the certified initial batch.
    sess = _tj_session()
    sess.insert_row(EvA, {"ts": 0, "k": 1})
    sess.insert_row(EvB, {"ts": 500, "k": 1})
    for act in (lambda: sess.advance(2000),
                lambda: sess.update(0, k=2),
                lambda: sess.delete(0)):
        with pytest.raises(RuntimeError, match="certified epoch shape"):
            act()


def test_guided_flow_matches_certified_outcome():
    # fire-then-advance = the certified epoch sequence: the join formed
    # at insert survives the expiry-crossing advance (pr_cep_expjoin_*),
    # REGARDLESS of insert order — the round-21 order-sensitivity was an
    # artifact of the walled pre-fire shape
    for order in (("A", "B"), ("B", "A")):
        sess = _tj_session()
        for step in order:
            if step == "A":
                sess.insert_row(EvA, {"ts": 0, "k": 1})
            else:
                sess.insert_row(EvB, {"ts": 500, "k": 1})
        fired_initial = sess.fire().fired
        sess.advance(2000)
        assert fired_initial == 1 and sess.fire().fired == 0


def test_reset_rearms_the_guard():
    sess = _tj_session()
    sess.fire()
    sess.advance(10)          # fine post-fire
    sess.reset()
    with pytest.raises(RuntimeError, match="certified epoch shape"):
        sess.advance(10)      # staging state again after reset
