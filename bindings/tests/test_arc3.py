"""Arc 3 (D-105): python sugar — insertLogical, CEP events/temporal/
advance, null tests, inline boolean groups."""
import pytest
import seine_rs
from seine_rs import CompileError


@seine_rs.fact
class Person:
    name: str
    age: int


@seine_rs.fact
class Eligible:
    name: str


@seine_rs.fact
class Blocker:
    name: str


def test_insert_logical_golden_and_tms():
    r = seine_rs.Rule("eligible_if_unblocked")
    p = r.when(Person)
    r.when_not(Blocker, Blocker.name == p.name)
    r.then_insert_logical(Eligible, name=p.name)
    drl = r.to_drl()
    assert "insertLogical(new Eligible($b0_0));" in drl, drl

    s = seine_rs.Session([r], facts={Person: {"name": ["a"], "age": [30]},
                                  Blocker: {"name": ["b"]},
                                  Eligible: {"name": []}})
    s.fire()
    # TMS auto-retraction: inserting the blocker kills the justification
    s.insert_row(Blocker, {"name": "a"})
    res = s.fire()
    t = res.facts().get("Eligible")
    assert t is None or len(t) == 0, res.facts()


def test_insert_logical_wall_names_rules():
    @seine_rs.fact
    class W:
        v: int

    r1 = seine_rs.Rule("Logi")
    p = r1.when(W, W.v > 0)
    r1.then_insert_logical(Eligible, name="x")
    r2 = seine_rs.Rule("Muta")
    q = r2.when(Eligible)
    r2.then_modify(q, name="y")
    with pytest.raises(Exception) as ei:
        s = seine_rs.Session([r1, r2], facts={W: {"v": [1]}, Eligible: {"name": []}})
        s.fire()
    assert "Logi" in str(ei.value) or "Muta" in str(ei.value), str(ei.value)


@seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=100))
class Ping:
    ts: int
    tag: str


def test_event_temporal_and_advance():
    r = seine_rs.Rule("pair")
    a = r.when(Ping, Ping.tag == "x")
    r.when(Ping, seine_rs.this_after(a, 0, 50))
    drl = r.to_drl()
    assert "this after[0ms,50ms] $p0" in drl, drl

    s = seine_rs.Session([r], facts={"Ping": {"ts": [0, 30], "tag": ["x", "y"]}})
    res = s.fire()
    firings = res.firings()
    assert len(firings) >= 1
    # expiration: advance past both deadlines; a fresh late pair works
    s.advance(500)
    s.insert(Ping, {"ts": [500, 520], "tag": ["x", "y"]})
    res2 = s.fire()
    assert len(res2.firings()) >= 1


def test_event_requires_explicit_expiry():
    with pytest.raises(CompileError) as ei:
        seine_rs.Event(timestamp="ts", expires_ms=-1)
    assert "expires_ms" in str(ei.value)


def test_temporal_needs_event_types():
    r = seine_rs.Rule("bad")
    a = r.when(Person)
    with pytest.raises(CompileError) as ei:
        r.when(Person, seine_rs.this_after(a, 0, 50))
    assert "event" in str(ei.value)


@seine_rs.fact
class MaybeV:
    tag: str
    v: "int | None"


def test_null_tests_and_none_guard():
    r = seine_rs.Rule("nulls")
    r.when(MaybeV, MaybeV.v.is_null())
    drl = r.to_drl()
    assert "v == null" in drl, drl

    with pytest.raises(CompileError) as ei:
        MaybeV.v == None  # noqa: E711
    assert "is_null" in str(ei.value)

    s = seine_rs.Session([r], facts={MaybeV: {"tag": ["a", "b"], "v": [None, 2]}})
    res = s.fire()
    assert len(res.firings()) == 1


def test_inline_boolean_groups():
    r = seine_rs.Rule("grp")
    r.when(Person, (Person.age > 65) | (Person.age < 18), ~(Person.name == "x"))
    drl = r.to_drl()
    assert "(age > 65 || age < 18)" in drl, drl
    assert "!(name == \"x\")" in drl, drl

    s = seine_rs.Session([r], facts={"Person": {"name": ["a", "x", "b"], "age": [70, 70, 30]}})
    res = s.fire()
    assert len(res.firings()) == 1


def test_group_cross_pattern_rejected():
    @seine_rs.fact
    class Other:
        v: int

    with pytest.raises(CompileError):
        r = seine_rs.Rule("bad")
        r.when(Person, (Person.age > 1) | (Other.v > 1))
        r.to_drl()


def test_agenda_groups_sugar():
    """D-106: agenda_group attr + then_set_focus round-trip."""

    @seine_rs.fact
    class AP:
        v: int

    f = seine_rs.Rule("F", salience=10)
    f.when(AP)
    f.then_set_focus("work")
    g = seine_rs.Rule("G", agenda_group="work")
    g.when(AP, AP.v > 0)
    m = seine_rs.Rule("M", salience=-10)
    m.when(AP)

    drl = seine_rs.compile_rules([f, g, m])
    assert 'agenda-group "work"' in drl, drl
    assert 'drools.setFocus("work");' in drl, drl

    s = seine_rs.Session([f, g, m], facts={AP: {"v": [1]}})
    res = s.fire()
    import polars as pl
    order = pl.DataFrame(res.firings())["rule"].to_list()
    assert order == ["F", "G", "M"], order


def test_set_focus_undeclared_walled():
    @seine_rs.fact
    class AQ:
        v: int

    f = seine_rs.Rule("F")
    f.when(AQ)
    f.then_set_focus("nowhere")
    import pytest as _pt
    with _pt.raises(Exception) as ei:
        s = seine_rs.Session([f], facts={AQ: {"v": [1]}})
        s.fire()
    assert "nowhere" in str(ei.value) and "agenda-group" in str(ei.value), str(ei.value)
