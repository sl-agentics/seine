"""Layer-1 boundary tests (D-044).

The binding must add ZERO semantics: exact marshaling, loud rejection of
anything outside the certified subset, and parity with the native
harness on corpus scenarios pushed through the Python API.
"""
import json
import math
import os
import struct
import subprocess
import sys

import polars as pl
import pyarrow as pa
import pytest

import seine_rs

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


# ----------------------------------------------------------------- fidelity

def test_f64_bit_exact_roundtrip():
    vals = [0.1, 0.2, 0.30000000000000004, -0.0, 1e-308, 6.5e9]
    df = pl.DataFrame({"v": vals})
    res = seine_rs.run("rule R when T($x : v) then end\n", {"T": df})
    out = pl.DataFrame(res.facts["T"])["v"].to_list()
    assert [struct.pack(">d", a) for a in out] == [struct.pack(">d", a) for a in vals]


def test_i64_extremes_and_strings():
    df = pl.DataFrame({
        "n": [2**63 - 1, -(2**63), 0],
        "s": ["", "héllo — ünïcode", "line\nbreak\t\"quote\""],
    })
    res = seine_rs.run("rule R when T($x : n) then end\n", {"T": df})
    t = pl.DataFrame(res.facts["T"])
    assert t["n"].to_list() == df["n"].to_list()
    assert t["s"].to_list() == df["s"].to_list()


def test_widening_is_exact():
    tbl = pa.table({
        "a": pa.array([1, -7, 2**31 - 1], type=pa.int32()),
        "b": pa.array([1.5, -2.25, 3.0], type=pa.float32()),
    })
    res = seine_rs.run("rule R when T($x : a) then end\n", {"T": tbl})
    t = pl.DataFrame(res.facts["T"])
    assert t["a"].dtype == pl.Int64 and t["a"].to_list() == [1, -7, 2**31 - 1]
    assert t["b"].to_list() == [1.5, -2.25, 3.0]  # f32->f64 exact for these


# ----------------------------------------------------------------- rejection

def test_nulls_rejected_loudly():
    df = pl.DataFrame({"v": [1.0, None, 3.0]})
    with pytest.raises(ValueError, match="null"):
        seine_rs.run("rule R when T($x : v) then end\n", {"T": df})


def test_unsupported_dtype_rejected():
    tbl = pa.table({"d": pa.array([1, 2], type=pa.date32())})
    with pytest.raises(TypeError, match="outside the certified subset"):
        seine_rs.run("rule R when T($x : d) then end\n", {"T": tbl})


def test_out_of_subset_drl_is_a_parse_error():
    df = pl.DataFrame({"v": [1.0]})
    bad = "rule R when accumulate( T($x : v); $s : variance($x) ) then end\n"
    with pytest.raises(ValueError, match="not in subset|accumulate function"):
        seine_rs.run(bad, {"T": df})


def test_none_scalar_rejected_in_dict_path():
    with pytest.raises(ValueError, match="None"):
        seine_rs.run("rule R when T($x : v) then end\n", {"T": {"v": [1.0, None]}})


# ----------------------------------------------------------------- lifecycle

def test_multi_fire_deltas():
    s = seine_rs.Session("rule R when T(v > 1.0) then end\n", {"T": {"v": [2.0]}})
    r1 = s.fire()
    assert r1.fired == 1
    # quiescent refire: nothing new
    r2 = s.fire()
    assert r2.fired == 0
    # incremental insert -> only the NEW fact fires
    s.insert("T", {"v": [3.0, 0.5]})
    r3 = s.fire()
    assert r3.fired == 1
    audit = pl.DataFrame(r3.firings)
    assert json.loads(audit["values_json"][0])["v"] == 3.0


def test_multi_fire_derived_is_per_fire():
    drl = "rule R when $t : T(v >= 2.0) then insert(new U($t.getV())); end\n"
    s = seine_rs.Session(drl, {"T": {"v": [2.0]}, "U": {"v": [0.0]}})
    r1 = s.fire()
    assert pl.DataFrame(r1.derived["U"])["v"].to_list() == [2.0]
    s.insert("T", {"v": [5.0]})
    r2 = s.fire()
    # ONLY this fire's derivation
    assert pl.DataFrame(r2.derived["U"])["v"].to_list() == [5.0]


def test_fire_limit_is_an_error_not_a_hang():
    # unguarded self-update loop: every fire re-triggers the rule
    drl = (
        "rule R when $t : T($x : v) then $t.setV($t.getV()); update($t); end\n"
    )
    with pytest.raises(RuntimeError, match="fire limit"):
        seine_rs.run(drl, {"T": {"v": [1.0]}}, fire_limit=500)


def test_insert_row_and_incremental_insert_before_fire():
    s = seine_rs.Session("rule R when T(v > 1.0) then end\n", {"T": {"v": [0.5]}})
    s.insert("T", {"v": [1.5, 2.5]})
    s.insert_row("T", {"v": 9.0})
    res = s.fire()
    assert res.fired == 3


def test_observer_matches_audit_order():
    seen = []
    df = pl.DataFrame({"v": [3.0, 1.0, 2.0]})
    res = seine_rs.run(
        "rule R salience($x) when T($x : v) then end\n",
        {"T": df},
        on_fire=lambda rule, matches: seen.append((rule, tuple(matches[0]))),
    )
    audit = pl.DataFrame(res.firings)
    got = list(zip(audit["rule"].to_list(), zip(audit["type"].to_list(), audit["handle"].to_list())))
    assert seen == got
    # dynamic salience fires descending
    assert [json.loads(v)["v"] for v in audit["values_json"].to_list()] == [3.0, 2.0, 1.0]


def test_wm_delta_and_deletions():
    drl = (
        "rule Promote when $t : T(v >= 2.0) then insert(new U($t.getV())); end\n"
        "rule Drop salience -5 when $t : T(v < 2.0) then delete($t); end\n"
    )
    s = seine_rs.Session(drl, {"T": {"v": [1.0, 2.0, 3.0]}, "U": {"v": [0.0]}})
    res = s.fire()
    derived_u = pl.DataFrame(res.derived["U"])
    assert sorted(derived_u["v"].to_list()) == [2.0, 3.0]
    assert len(res.deleted_handles) == 1


def test_external_update_and_delete():
    drl = "rule R when T(v > 1.0, $x : v) then end\n"
    s = seine_rs.Session(drl, {"T": {"v": [2.0, 0.5]}})
    r1 = s.fire()
    assert r1.fired == 1
    handles = pl.DataFrame(r1.facts["T"])
    h_low = handles.filter(pl.col("v") == 0.5)["_handle"][0]
    h_hi = handles.filter(pl.col("v") == 2.0)["_handle"][0]
    # raise the low fact above the threshold -> fires
    s.update(h_low, v=3.0)
    r2 = s.fire()
    assert r2.fired == 1
    # delete the high fact: external deletes are the CALLER's action
    # (per-fire deleted_handles covers RULE deletions only), but the
    # fact is gone from the final view
    s.delete(h_hi)
    r3 = s.fire()
    assert r3.fired == 0
    assert h_hi not in pl.DataFrame(r3.facts["T"])["_handle"].to_list()


def test_external_action_order_is_certified():
    # session-action order composes at k=1 terminals (D-047/xv2..xv5)
    drl = "rule R when T($b : g) then end\n"
    s = seine_rs.Session(drl, {"T": {"g": [True]}})
    s.fire()
    s.insert("T", {"g": [False]})
    h0 = 0
    s.update(h0, g=True)  # same-value update of the FIRED fact
    r = s.fire()
    audit = pl.DataFrame(r.firings)
    got = [json.loads(v)["g"] for v in audit["values_json"].to_list()]
    assert got == [False, True]  # insert first, then the re-added update


def test_dead_handle_errors():
    s = seine_rs.Session("rule R when T($x : v) then end\n", {"T": {"v": [1.0]}})
    s.fire()
    s.delete(0)
    # D-115: delete of an already-dead handle is a Drools-faithful
    # graceful no-op (session.delete leniency, c_double_del pin) —
    # only UPDATE of a dead handle errors.
    s.delete(0)
    with pytest.raises(ValueError, match="dead handle"):
        s.update(0, v=2.0)


# ----------------------------------------------------------------- parity

def _native_firings(scn_path):
    out = subprocess.run(
        ["cargo", "run", "-q", "-p", "seine-harness", "--", "run", scn_path],
        capture_output=True, text=True, cwd=REPO, check=True,
    ).stdout
    d = json.loads(out.splitlines()[0])
    return d["result"]["firings"]


@pytest.mark.parametrize("scenario", [
    "scenarios/probes/se1.json",
    "scenarios/probes/acc4.json",
    "scenarios/phase2/j07_refire_order_after_updates.json",
    "scenarios/probes/mf3.json",
    "scenarios/probes/mf5.json",
    "scenarios/probes/xu4.json",
])
def test_parity_with_native_harness(scenario):
    """Corpus scenarios pushed through the Python boundary must fire
    identically to the native harness (rule sequence + rendered values):
    the binding adds no semantics."""
    scn = json.load(open(os.path.join(REPO, scenario)))
    schemas = {
        t["name"]: {f["name"]: f["type"] for f in t["fields"]} for t in scn["types"]
    }
    # declare every type via an empty arrow table with the right schema
    def arrow_type(t):
        return {"i64": pa.int64(), "f64": pa.float64(),
                "bool": pa.bool_(), "String": pa.string()}[t]
    tables = {
        name: pa.table({f: pa.array([], type=arrow_type(ty)) for f, ty in fields.items()})
        for name, fields in schemas.items()
    }
    s = seine_rs.Session(scn["drl"], tables)
    # scenario facts are ORDER-SIGNIFICANT across types: insert row-wise,
    # recording handles for action targeting
    visible = []
    for fact in scn["facts"]:
        visible.append(s.insert_row(fact["type"], fact["fields"]))
    res = s.fire()
    audits = [pl.DataFrame(res.firings)]
    total = res.fired
    # multi-fire epochs (D-046) + external actions (D-047): replay
    # through the boundary, mapping visible insertion indices to real
    # handles via insert_row's return value.
    for epoch in scn.get("epochs", []):
        for action in epoch.get("actions", []):
            if action["op"] == "insert":
                visible.append(s.insert_row(action["type"], action["fields"]))
            elif action["op"] == "update":
                s.update(visible[action["target"]], **action["fields"])
            else:
                s.delete(visible[action["target"]])
        for fact in epoch.get("facts", []):
            visible.append(s.insert_row(fact["type"], fact["fields"]))
        r = s.fire()
        a = pl.DataFrame(r.firings)
        if len(a):
            a = a.with_columns((pl.col("seq") + total).alias("seq"))
        audits.append(a)
        total += r.fired
    audit = pl.concat([a for a in audits if len(a)]) if any(len(a) for a in audits) else audits[0]

    native = _native_firings(os.path.join(REPO, scenario))
    assert total == len(native)
    if total:
        by_seq = audit.group_by("seq", maintain_order=True).agg(
            pl.col("rule").first(), pl.col("values_json")
        )
        for row, nat in zip(by_seq.iter_rows(named=True), native):
            assert row["rule"] == nat["rule"]
            got = sorted(sorted(json.loads(v).items()) for v in row["values_json"])
            want = sorted(sorted(m["fields"].items()) for m in nat["matches"])
            assert got == want, f"seq {row['seq']}: {got} != {want}"


def test_positioned_drl_error_reaches_python():
    """D-103: raw-DRL parse errors carry line/col + caret through PySession."""
    import pytest
    import seine_rs

    with pytest.raises(Exception) as ei:
        seine_rs.Session(
            "rule R when BPos(v > ) then end",
            facts={"BPos": {"v": [1]}},
        )
    msg = str(ei.value)
    assert "line 1" in msg, msg
    assert "^" in msg, msg


def test_reset_paged_batches():
    """D-104: page1 / reset / page2 == fresh(page2)."""
    import seine_rs

    @seine_rs.fact
    class RP:
        v: int

    rule = seine_rs.Rule("r")
    rule.when(RP, RP.v > 0)
    s = seine_rs.Session([rule], facts={"RP": {"v": [1, 2]}})
    r1 = s.fire()
    assert len(r1.firings) == 2
    s.reset()
    s.insert(RP, [RP(v=3)])
    r2 = s.fire()
    assert len(r2.firings) == 1

    fresh = seine_rs.Session([rule], facts={"RP": {"v": [3]}})
    rf = fresh.fire()
    assert len(rf.firings) == len(r2.firings)
