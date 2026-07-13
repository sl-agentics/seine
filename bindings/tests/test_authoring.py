"""Layer-2 authoring tests (D-045): golden DRL, fencing, and parity —
Python-authored rules must be indistinguishable from hand-written DRL
because the engine only ever sees the generated DRL.
"""
import polars as pl
import pytest

import seine_rs
from seine_rs import CompileError, Rule, average, count, fact, max_, min_, sum_


@fact
class Person:
    name: str
    age: int
    score: float
    active: bool


@fact
class Order:
    owner: str
    amount: float
    priority: int


@fact
class Alert:
    owner: str
    total: float


# ------------------------------------------------------------- golden DRL

def test_join_not_exists_golden():
    r = Rule("R", salience=5, no_loop=True)
    p = r.when(Person, Person.age >= 18, Person.name.matches("(a|be).*"))
    r.when(Order, Order.owner == p.name, Order.amount > 10.0)
    r.when_not(Person, Person.score < 10.0)
    r.when_exists(Order, Order.priority.in_(1, 2, 3))
    r.then_delete(p)
    drl = r.to_drl()
    assert drl == (
        'rule "R"\n'
        "salience 5\n"
        "no-loop\n"
        "when\n"
        '    $p0 : Person(age >= 18, name matches "(a|be).*", $b0_0 : name)\n'
        "    Order(owner == $b0_0, amount > 10.0)\n"
        "    not Person(score < 10.0)\n"
        "    exists Order(priority in (1, 2, 3))\n"
        "then\n"
        "    delete($p0);\n"
        "end\n"
    )


def test_class_field_salience_is_guided():
    with pytest.raises(CompileError, match="MATCHED pattern"):
        Rule("Agg", salience=Person.age * 10)


def test_accumulate_and_salience_expr_golden():
    r2 = Rule("Agg2")
    p = r2.when(Person, Person.active == True)  # noqa: E712
    r2.set_salience(p.age - 3)
    total = r2.accumulate(Order, Order.owner == p.name, agg=sum_(Order.amount))
    r2.then_insert(Alert, owner=p.name, total=total)
    drl = r2.to_drl()
    assert drl == (
        'rule "Agg2"\n'
        "salience($b0_1 - 3)\n"
        "when\n"
        "    Person(active == true, $b0_0 : name, $b0_1 : age)\n"
        "    accumulate( Order(owner == $b0_0, $s1 : amount); $a1 : sum($s1) )\n"
        "then\n"
        "    insert(new Alert($b0_0, $a1));\n"
        "end\n"
    )


def test_collect_golden():
    r = Rule("C")
    r.when(Person)
    r.collect(Order, Order.amount > 5.0)
    drl = r.to_drl()
    assert "$l1 : ArrayList() from collect( Order(amount > 5.0) )" in drl


def test_generated_drl_parses_in_engine():
    """Every golden construct must actually compile in the engine."""
    r = Rule("R", no_loop=True)
    p = r.when(Person, Person.age >= 18)
    tot = r.accumulate(Order, Order.owner == p.name, agg=average(Order.amount))
    r.when_not(Alert, Alert.owner == p.name)
    r.collect(Order, Order.priority.in_(1, 2))
    r.then_insert(Alert, owner=p.name, total=tot)
    empty = {
        Person: {"name": ["x"], "age": [1], "score": [0.0], "active": [False]},
        Order: {"owner": ["x"], "amount": [0.0], "priority": [0]},
        Alert: {"owner": ["x"], "total": [0.0]},
    }
    res = seine_rs.run(r, empty)  # engine parse + fire is the assertion
    assert res.fired >= 0


# ------------------------------------------------------------- fencing

def test_lambda_in_constraint_fenced():
    r = Rule("L")
    with pytest.raises(CompileError, match="match loop"):
        r.when(Person, lambda p: p.age > 3)


def test_lambda_salience_fenced():
    with pytest.raises(CompileError, match="match loop"):
        Rule("L", salience=lambda p: p.age)


def test_nested_salience_arithmetic_fenced():
    r = Rule("L")
    p = r.when(Person)
    with pytest.raises(CompileError, match="term op term"):
        _ = (p.age + 1) * 2


def test_collect_join_constraint_fenced_d041():
    r = Rule("L")
    p = r.when(Person)
    with pytest.raises(CompileError, match="subnetwork"):
        r.collect(Order, Order.owner == p.name)


def test_minmax_float_downstream_fenced_d039():
    r = Rule("L")
    r.when(Person)
    m = r.accumulate(Order, agg=max_(Order.amount))
    with pytest.raises(CompileError, match="opaque Number"):
        r.then_insert(Alert, owner="x", total=m)


def test_acc_result_salience_fenced_d043():
    r = Rule("L")
    r.when(Person)
    c = r.accumulate(Order, agg=count())
    with pytest.raises(CompileError, match="against the oracle"):
        Rule("L2", salience=c)


def test_bindings_inside_not_fenced():
    r = Rule("L")
    r.when_not(Person, Person.age > 3)
    with pytest.raises(CompileError, match="scope"):
        # accessing fields of a not() has no Drools meaning
        r.patterns[0].age  # noqa: B018


def test_insert_field_coverage_enforced():
    r = Rule("L")
    r.when(Person)
    with pytest.raises(CompileError, match="missing"):
        r.then_insert(Alert, owner="x")


def test_wrong_owner_constraint_fenced():
    r = Rule("L")
    with pytest.raises(CompileError, match="does not belong"):
        r.when(Person, Order.amount > 3.0)


def test_unsupported_annotation_fenced():
    with pytest.raises(CompileError, match="outside the certified subset"):
        @fact
        class Bad:
            when: list


def test_min_over_int_is_usable_downstream():
    # min over i64 -> Long: valid as an RHS arg per D-039 (Long widens
    # to double); salience stays fenced for ALL accumulate results (D-043)
    r = Rule("OK")
    p = r.when(Person)
    m = r.accumulate(Order, agg=min_(Order.priority))
    r.then_insert(Alert, owner=p.name, total=m)
    drl = r.to_drl()
    assert "insert(new Alert($b0_0, $a1));" in drl
    seine_rs.run(r, {
        Person: {"name": ["x"], "age": [1], "score": [0.0], "active": [False]},
        Order: {"owner": ["x"], "amount": [0.0], "priority": [3]},
        Alert: {"owner": ["x"], "total": [0.0]},
    })


# ------------------------------------------------------------- parity

def test_authored_equals_hand_drl():
    """The authored rules and hand-written DRL produce identical
    firing sequences and derived facts."""
    hand = (
        'rule "Adults"\n'
        "when\n"
        "    Person(age >= 18, $n : name, $s : score)\n"
        "then\n"
        "    insert(new Alert($n, $s));\n"
        "end\n"
        'rule "Boost" salience -1\n'
        "when\n"
        "    $a : Alert(total < 90.0)\n"
        "then\n"
        "    modify($a) { setTotal(90.0) }\n"
        "end\n"
    )
    adults = Rule("Adults")
    p = adults.when(Person, Person.age >= 18)
    adults.then_insert(Alert, owner=p.name, total=p.score)
    boost = Rule("Boost", salience=-1)
    a = boost.when(Alert, Alert.total < 90.0)
    boost.then_modify(a, total=90.0)

    people = pl.DataFrame({
        "name": ["ada", "grace", "alan", "kurt"],
        "age": [36, 45, 41, 17],
        "score": [91.5, 88.0, 79.5, 99.0],
        "active": [True, True, False, True],
    })
    alerts = pl.DataFrame({"owner": ["x"], "total": [0.0]}).clear()

    r1 = seine_rs.run(hand, {"Person": people, "Alert": alerts})
    r2 = seine_rs.run([adults, boost], {Person: people, Alert: alerts})
    a1 = pl.DataFrame(r1.firings)
    a2 = pl.DataFrame(r2.firings)
    assert a1["rule"].to_list() == a2["rule"].to_list()
    assert a1["values_json"].to_list() == a2["values_json"].to_list()
    d1 = pl.DataFrame(r1.derived["Alert"]).sort("owner")
    d2 = pl.DataFrame(r2.derived["Alert"]).sort("owner")
    assert d1.drop("handle").equals(d2.drop("handle"))
