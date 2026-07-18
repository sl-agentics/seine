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
    # nested arithmetic now BUILDS (RHS args / LHS constraints take it,
    # D-299); the closed salience grammar rejects it at the point of use
    r = Rule("L")
    p = r.when(Person)
    e = (p.age + 1) * 2
    with pytest.raises(CompileError, match="term op term"):
        r.set_salience(e)


def test_salience_div_and_float_fenced():
    r = Rule("L")
    p = r.when(Person)
    with pytest.raises(CompileError, match=r"term op term"):
        r.set_salience(p.age / 2)
    with pytest.raises(CompileError, match="int literals or numeric bindings"):
        r.set_salience(p.age + 1.5)


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


# --- the expires-vs-window consistency lint (D-219) ------------------

def _ev(expires_ms, name="Anchor"):
    @seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=expires_ms))
    class Anchor:
        ts: int
        acct: int
    Anchor.__name__ = name
    return Anchor


@seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=100_000))
class Later:
    ts: int
    acct: int


def test_expires_truncates_window_rejected():
    Anchor = _ev(5_000)
    r = seine_rs.Rule("nsf-reverses-payoff")
    a = r.when(Anchor)
    with pytest.raises(CompileError, match="silently truncating"):
        r.when(Later, seine_rs.this_after(a, 0, 10_000))


def test_expires_before_window_opens_rejected():
    Anchor = _ev(5_000)
    r = seine_rs.Rule("gap")
    a = r.when(Anchor)
    with pytest.raises(CompileError, match="can never match"):
        r.when(Later, seine_rs.this_after(a, 6_000, 10_000))


def test_expires_equal_to_lo_is_truncation_not_never_match():
    # the round-9 off-by-one: at delta == lo == expires the anchor is
    # alive AT its deadline and the join fires (pr_cep_expwin_atlo,
    # oracle-pinned) — so expires == lo is the single-instant tier-2
    # truncation, and "can never match" would state a falsehood
    Anchor = _ev(10_000)
    r = seine_rs.Rule("at-lo")
    a = r.when(Anchor)
    with pytest.raises(CompileError, match="silently truncating") as ei:
        r.when(Later, seine_rs.this_after(a, 10_000, 20_000))
    assert "before the window opens" not in str(ei.value)


def test_expires_covering_window_allowed():
    Anchor = _ev(10_000)
    r = seine_rs.Rule("ok")
    a = r.when(Anchor)
    r.when(Later, seine_rs.this_after(a, 0, 10_000))
    assert "after[0ms,10000ms]" in r.to_drl()


def test_before_checks_the_this_side():
    # this BEFORE anchor: the THIS pattern is the earlier event
    Anchor = _ev(500_000)
    Short = _ev(5_000, name="Short")
    r = seine_rs.Rule("before-side")
    a = r.when(Anchor)
    with pytest.raises(CompileError, match="Short declares expires_ms=5000"):
        r.when(Short, seine_rs.this_before(a, 0, 10_000))


@seine_rs.fact
class SApp:
    acct: str
    pri: int


@seine_rs.fact
class SDecision:
    acct: str
    kind: str


def _negator(logical=False):
    r = seine_rs.Rule("release")
    a = r.when(SApp)
    r.when_not(SDecision, SDecision.acct == a.acct)
    ins = r.then_insert_logical if logical else r.then_insert
    ins(SDecision, acct=a.acct, kind="release")
    return r


def _blocker(**rule_kw):
    r = seine_rs.Rule("block-bankruptcy", **rule_kw)
    b = r.when(SApp, SApp.acct == "bad")
    r.then_insert(SDecision, acct=b.acct, kind="block")
    return r


def test_unstratified_negation_rejected_in_both_declaration_orders():
    # the round-7 leak: same stratum, outcome flips with list order —
    # the lint must fire regardless of which order was declared
    for rules in ([_blocker(), _negator()], [_negator(), _blocker()]):
        with pytest.raises(seine_rs.CompileError) as ei:
            seine_rs.compile_rules(rules)
        msg = str(ei.value)
        assert "release" in msg and "block-bankruptcy" in msg
        assert "SDecision" in msg and "declared" in msg
        # every remedy the lint's own exemptions know about is offered,
        # including the smallest diff: view -> then_insert_logical
        assert "then_insert_logical" in msg
        assert "salience" in msg and "agenda_group" in msg and "separate session pass" in msg


def test_stratified_by_salience_passes():
    assert "rule" in seine_rs.compile_rules([_blocker(salience=10), _negator()])


def test_stratified_by_agenda_group_passes():
    assert "rule" in seine_rs.compile_rules(
        [_blocker(agenda_group="blocks"), _negator()]
    )


def test_self_negation_fire_once_passes():
    # a rule negating the type it itself inserts is the fire-once idiom
    assert "rule" in seine_rs.compile_rules([_negator()])


def test_insert_logical_negator_exempt():
    # TMS retracts the logical product when the negation falsifies
    # later — finals are order-invariant, so the set compiles
    assert "rule" in seine_rs.compile_rules([_blocker(), _negator(logical=True)])


def test_dynamic_salience_stays_silent():
    r = _negator()
    a = r.patterns[0]
    r.set_salience(a.pri)
    assert "rule" in seine_rs.compile_rules([_blocker(), r])


# --- RHS arithmetic wall (round 11) -----------------------------------

def test_rhs_computed_args_golden():
    # D-299: the D-283/D-288-certified computed args are authorable.
    # Nested sub-expressions render PARENTHESIZED (the D-281 oracle
    # bare-precedence defect stays unreachable from authored rules).
    @seine_rs.fact
    class Ctr:
        n: int

    r = seine_rs.Rule("inc")
    c = r.when(Ctr, Ctr.n < 3)
    r.then_insert(Ctr, n=c.n * 2 + 1)
    drl = r.to_drl()
    assert "insert(new Ctr(($b0_0 * 2) + 1));" in drl

    r2 = seine_rs.Rule("bump")
    c2 = r2.when(Ctr, Ctr.n < 3)
    r2.then_modify(c2, n=c2.n + 1)
    assert "setN($b0_0 + 1)" in r2.to_drl()


def test_rhs_class_field_arith_guided():
    @seine_rs.fact
    class Ctr8:
        n: int

    r = seine_rs.Rule("inc8")
    r.when(Ctr8)
    with pytest.raises(CompileError, match="MATCHED fields") as ei:
        r.then_insert(Ctr8, n=Ctr8.n + 1)
        r.to_drl()
    assert "SalExpr" not in str(ei.value)  # no internal type names


def test_salience_arithmetic_still_compiles():
    @seine_rs.fact
    class Ctr2:
        n: int
    r = seine_rs.Rule("sal")
    c = r.when(Ctr2)
    r.set_salience(c.n + 1)
    assert "salience" in seine_rs.compile_rules([r])


# --- LHS whole-slot arithmetic (D-291 agree subset, D-299 sugar) -------

@fact
class ArithT:
    k: int
    j: float


def test_lhs_arith_golden_and_fires():
    # own-field division against an int literal comparand — the D-291
    # int-int agree cell; k/2 == 3 admits k in {6, 7} (Java int division)
    r = Rule("DEq3")
    r.when(ArithT, ArithT.k / 2 == 3)
    drl = r.to_drl()
    assert "ArithT(k / 2 == 3)" in drl
    res = seine_rs.run(r, {ArithT: {"k": [5, 6, 7, 8], "j": [0.0] * 4}})
    assert res.fired == 2


def test_lhs_arith_binding_comparand_golden():
    # the BindArith probe shape: T(k > $a + 1) across two patterns
    r = Rule("BindArith")
    p = r.when(ArithT)
    r.when(ArithT, ArithT.k > p.k + 1)
    drl = r.to_drl()
    assert "ArithT(k > $b0_0 + 1)" in drl
    res = seine_rs.run(r, {ArithT: {"k": [1, 3], "j": [0.0, 0.0]}})
    assert res.fired == 1  # k=3 > k=1 + 1


def test_lhs_arith_nested_parenthesized():
    r = Rule("Nest")
    r.when(ArithT, (ArithT.k + 1) * 2 > 6)
    assert "ArithT((k + 1) * 2 > 6)" in r.to_drl()


def test_lhs_arith_group_composition_fenced():
    with pytest.raises(CompileError, match="whole-slot"):
        (ArithT.k + 1 > 2) & (ArithT.k == 3)


def test_lhs_arith_cross_class_field_fenced():
    r = Rule("X")
    with pytest.raises(CompileError, match="OWN fields"):
        r.when(ArithT, Person.age + 1 > 3)


def test_lhs_arith_when_any_fenced():
    r = Rule("WA")
    r.when(Person)
    with pytest.raises(CompileError, match="not certified"):
        r.when_any((ArithT, ArithT.k + 1 > 3), (Person, Person.age > 1))


def test_lhs_arith_nonnumeric_field_fenced():
    with pytest.raises(CompileError, match="numeric i64/f64"):
        _ = Person.name + 1


def test_lhs_arith_string_comparand_fenced():
    with pytest.raises(CompileError, match="numeric"):
        _ = ArithT.k + 1 == "seven"


def test_lhs_arith_engine_fence_bubbles():
    # the drl.rs D-291 fences stay the single authority: an int-int
    # division composed into surrounding arithmetic is a fenced cell
    # and must fail the session build loudly, from the engine
    r = Rule("Fenced")
    r.when(ArithT, ArithT.k / 2 + 1 == 4)
    with pytest.raises(Exception, match="div"):
        seine_rs.run(r, {ArithT: {"k": [6], "j": [0.0]}})


# --- string ops on nullable String (round 12) --------------------------

@fact
class NDoc:
    title: "str | None"
    acct: int


def test_string_ops_allowed_on_nullable_string():
    # null makes the constraint UNKNOWN (SQL 3VL, duckdb-tier certified:
    # dk_strings) — the natural nullable-text schema needs no sentinel
    r = Rule("release-via-title")
    d = r.when(NDoc, NDoc.title.is_not_null(), NDoc.title.contains("DEED"))
    r2 = Rule("m")
    r2.when(NDoc, NDoc.title.matches(".*DEED.*"))
    res = seine_rs.run(
        [r, r2], {NDoc: {"title": ["WARRANTY DEED", None, "NOTE"], "acct": [1, 2, 3]}}
    )
    assert res.fired == 2  # both rules, DEED row only; null row never admits


def test_bare_nullable_contains_skips_null_rows():
    r = Rule("bare")
    r.when(NDoc, NDoc.title.contains("NOTE"))
    res = seine_rs.run([r], {NDoc: {"title": [None, "NOTE"], "acct": [1, 2]}})
    assert res.fired == 1


def test_string_ops_still_walled_on_non_string():
    r = Rule("x")
    with pytest.raises(CompileError, match="requires a str field"):
        r.when(Person, Person.age.contains("1"))
    with pytest.raises(CompileError, match="requires a str field"):
        r.when(Person, Person.age.matches("1.*"))


# --- the truthiness footgun (round 13) ---------------------------------

def test_chained_comparison_raises_not_silently_drops():
    # 10 < x < 100 desugars via bool(10 < x) — default truthiness kept
    # only the right operand and the rule silently misfired (amount=5
    # matched a "10 < amount" author intent)
    r = Rule("band")
    with pytest.raises(CompileError, match="truth value .* ambiguous"):
        r.when(Order, 10 < Order.amount < 100)


def test_and_or_between_constraints_raise():
    with pytest.raises(CompileError, match="ambiguous"):
        (Order.amount > 10) and (Order.amount < 100)
    with pytest.raises(CompileError, match="ambiguous"):
        (Order.amount > 100) or (Person.active == True)  # noqa: E712
    # the boolean-field case routes through bool(FieldRef), not
    # bool(_Constraint) — both classes must raise
    with pytest.raises(CompileError, match="ambiguous"):
        Person.active and (Order.amount > 10)


def test_ambiguity_error_names_the_correct_forms():
    with pytest.raises(CompileError) as ei:
        (Order.amount > 10) and (Order.amount < 100)
    msg = str(ei.value)
    assert "&" in msg and "|" in msg and "when(" in msg


def test_intended_combinators_untouched_by_bool_wall():
    r = Rule("ok")
    r.when(Order, Order.amount > 10.0, Order.amount < 100.0)  # varargs AND
    r2 = Rule("ok2")
    r2.when(Order, (Order.amount > 10.0) & (Order.amount < 100.0))
    r3 = Rule("ok3")
    r3.when(Order, (Order.amount > 100.0) | (Order.priority == 1))
    r4 = Rule("ok4")
    r4.when(Order, ~(Order.amount > 100.0))
    for x in (r, r2, r3, r4):
        assert "rule" in x.to_drl()


def test_precedence_trap_raises_compile_error_not_type_error():
    # & / | bind tighter than comparisons: `a > 10 & b < 100` parses as
    # `a > (10 & b) < 100` — the bare-field operand must get the
    # teachable error, not a cryptic TypeError (round 14)
    with pytest.raises(CompileError, match="bind TIGHTER"):
        Order.amount > 10 & Order.amount  # noqa: B015
    with pytest.raises(CompileError, match="bind TIGHTER"):
        Person.active | Order.amount  # noqa: B015
    # legitimate _Constraint & _Constraint never touches a field expression
    r = Rule("ok")
    r.when(Order, (Order.amount > 10.0) & (Order.amount < 100.0))
    assert "rule" in r.to_drl()


# --- the logical-cycle lint (round 17 / Bryan's build directive) --------

@fact
class CRoot:
    v: int


@fact
class CMid:
    k: str


@fact
class CLeaf:
    k: str


def _chain(back_edge=True, logical_back=True):
    r1 = Rule("RM")
    r1.when(CRoot, CRoot.v > 0)
    r1.then_insert_logical(CMid, k="m")
    r2 = Rule("RML")
    r2.when(CMid, CMid.k == "m")
    r2.then_insert_logical(CLeaf, k="l")
    out = [r1, r2]
    if back_edge:
        r3 = Rule("RLM")
        r3.when(CLeaf, CLeaf.k == "l")
        (r3.then_insert_logical if logical_back else r3.then_insert)(CMid, k="m")
        out.append(r3)
    return out


def test_distinct_type_logical_cycle_rejected_both_orders():
    # the pr_tms_cycle contract: an all-logical cycle orphans permanently
    # (justification sets are counted, not grounded) — reject at compile
    for order in (_chain(), list(reversed(_chain()))):
        with pytest.raises(CompileError, match="logical derivation cycle") as ei:
            seine_rs.compile_rules(order)
        msg = str(ei.value)
        assert "RML" in msg and "RLM" in msg
        assert "CMid" in msg and "CLeaf" in msg
        assert "DAG" in msg  # the remedy is named


def test_three_type_logical_cycle_rejected():
    @fact
    class CM3:
        k: str
    r1 = Rule("a"); r1.when(CMid); r1.then_insert_logical(CLeaf, k="l")
    r2 = Rule("b"); r2.when(CLeaf); r2.then_insert_logical(CM3, k="x")
    r3 = Rule("c"); r3.when(CM3); r3.then_insert_logical(CMid, k="m")
    with pytest.raises(CompileError, match="logical derivation cycle"):
        seine_rs.compile_rules([r1, r2, r3])


def test_acyclic_chain_and_stated_back_edge_pass():
    assert "rule" in seine_rs.compile_rules(_chain(back_edge=False))
    # a stated back-edge creates no TMS support edge — no logical cycle
    assert "rule" in seine_rs.compile_rules(_chain(logical_back=False))


def test_self_loop_exempt_bounded_escalation():
    # the load-bearing exemption: constraint-guarded escalation over ONE
    # type is valid (and exactly where type-level over-approximation
    # would bite) — T -> T stays silent by design
    @fact
    class CAlarm:
        sev: int
    r = Rule("esc")
    r.when(CAlarm, CAlarm.sev == 1)
    r.then_insert_logical(CAlarm, sev=2)
    assert "rule" in seine_rs.compile_rules([r])


# --- the self-loop satisfiability boundary (round 18) ------------------

@fact
class SLM:
    n: int


def _self_loop(guard, insert_n):
    r = Rule("self")
    if guard is None:
        r.when(SLM)
    else:
        r.when(SLM, guard)
    r.then_insert_logical(SLM, n=insert_n)
    return r


def test_self_loop_proven_live_rejected():
    # A: unconstrained — trivially re-satisfies (a one-node cycle)
    with pytest.raises(CompileError, match="self-justifying"):
        seine_rs.compile_rules([_self_loop(None, 0)])
    # B: plateau — the terminus re-matches its own guard (n=1 < 2)
    with pytest.raises(CompileError, match="self-justifying"):
        seine_rs.compile_rules([_self_loop(SLM.n < 2, 1)])
    # copied field: satisfies whatever the match satisfied, by construction
    r = Rule("copy")
    m = r.when(SLM, SLM.n < 2)
    r.then_insert_logical(SLM, n=m.n)
    with pytest.raises(CompileError, match="self-justifying"):
        seine_rs.compile_rules([r])


def test_self_loop_strict_progress_and_undecidable_pass():
    # C: the insert falls OUTSIDE the guard — cascades cleanly at runtime
    assert "rule" in seine_rs.compile_rules([_self_loop(SLM.n < 2, 2)])
    # undecidable (value from another pattern): silence, per the
    # dynamic-salience precedent — only PROVEN-live rejects
    @fact
    class SLOther:
        v: int
    r = Rule("x")
    o = r.when(SLOther)
    r.when(SLM, SLM.n < 2)
    r.then_insert_logical(SLM, n=o.v)
    assert "rule" in seine_rs.compile_rules([r])


def test_self_loop_group_boundary():
    r = Rule("g")
    r.when(SLM, (SLM.n < 0) | (SLM.n > 10))
    r.then_insert_logical(SLM, n=99)
    with pytest.raises(CompileError, match="self-justifying"):
        seine_rs.compile_rules([r])
    r2 = Rule("g2")
    r2.when(SLM, (SLM.n < 0) | (SLM.n > 10))
    r2.then_insert_logical(SLM, n=5)
    assert "rule" in seine_rs.compile_rules([r2])


# --- the self-feeding modify lint (the D-231 hazard, mapped by probe:
# --- written ∩ own-listened + a write that provably keeps matching)


def test_self_feeding_modify_same_value_rejected():
    # write a listened field a value that provably keeps the rule
    # matching -> the rule re-triggers itself to the fire limit
    @fact
    class SFC1:
        n: int

    r = Rule("same")
    c = r.when(SFC1, SFC1.n == 0)
    r.then_modify(c, n=0)
    with pytest.raises(CompileError) as ei:
        seine_rs.compile_rules([r])
    msg = str(ei.value)
    assert "same" in msg and "SFC1" in msg and "re-triggers itself" in msg
    # every exemption the lint knows is offered as a remedy
    assert "no_loop" in msg and "acc_sum" in msg and "guard" in msg


def test_self_feeding_modify_guard_flip_passes():
    # the corpus idiom: the write falsifies the rule's own constraint,
    # so the rule exits its own match (setG(true) under g == False)
    @fact
    class SFC2:
        g: bool

    r = Rule("flip")
    c = r.when(SFC2, SFC2.g == False)  # noqa: E712 — authoring constraint
    r.then_modify(c, g=True)
    assert "rule" in seine_rs.compile_rules([r])


def test_self_feeding_modify_still_true_numeric_rejected():
    @fact
    class SFC3:
        n: int

    r = Rule("still")
    c = r.when(SFC3, SFC3.n > 5)
    r.then_modify(c, n=7)  # 7 > 5: provably still matching
    with pytest.raises(CompileError, match="re-triggers itself"):
        seine_rs.compile_rules([r])


def test_self_feeding_modify_falsifying_numeric_passes():
    @fact
    class SFC4:
        n: int

    r = Rule("exit")
    c = r.when(SFC4, SFC4.n == 0)
    r.then_modify(c, n=1)  # 1 == 0 is False: exits the match
    assert "rule" in seine_rs.compile_rules([r])


def test_self_feeding_modify_bound_only_rejected():
    # bound fields are listened: using c.n as an insert arg binds it,
    # and with no constraint to exit through the write is a proven loop
    @fact
    class SFC5:
        n: int

    @fact
    class SFOut5:
        v: int

    r = Rule("bound")
    c = r.when(SFC5)
    r.then_insert(SFOut5, v=c.n)
    r.then_modify(c, n=5)
    with pytest.raises(CompileError, match="no constraint to exit"):
        seine_rs.compile_rules([r])


def test_self_feeding_modify_no_loop_exempt():
    # the engine suppresses the rule's own re-activation: fires once
    @fact
    class SFC6:
        n: int

    r = Rule("once", no_loop=True)
    c = r.when(SFC6, SFC6.n == 0)
    r.then_modify(c, n=0)
    assert "rule" in seine_rs.compile_rules([r])


def test_self_feeding_modify_unlistened_passes():
    # writing a field the rule neither constrains nor binds cannot
    # re-stage its own match
    @fact
    class SFC7:
        a: int
        b: int

    r = Rule("quiet")
    c = r.when(SFC7, SFC7.a > 0)
    r.then_modify(c, b=5)
    assert "rule" in seine_rs.compile_rules([r])


def test_self_feeding_modify_undecidable_silent():
    # cross-field copy: statically unknowable -> silence (only PROVEN
    # outcomes act, the shared three-valued bias)
    @fact
    class SFC8:
        n: int
        m: int

    r = Rule("copy8")
    c = r.when(SFC8, SFC8.n > 5)
    r.then_modify(c, n=c.m)
    assert "rule" in seine_rs.compile_rules([r])


def test_self_feeding_modify_computed_unguarded_rejected():
    # D-299 (the D-289 skip removed): the classic unguarded computed
    # counter — modify setN($n + 1) with no exit constraint — is the
    # self-feed lint's case now that computed args render.
    @fact
    class SFC9:
        n: int

    r = Rule("inc9")
    c = r.when(SFC9)
    r.then_modify(c, n=c.n + 1)
    with pytest.raises(CompileError, match="re-triggers itself"):
        seine_rs.compile_rules([r])


def test_self_feeding_modify_computed_guarded_silent():
    # a guarded computed write is statically UNKNOWN: silent, and the
    # engine runs the bounded loop (guard n < 3 terminates it)
    @fact
    class SFC10:
        n: int

    r = Rule("inc10")
    c = r.when(SFC10, SFC10.n < 3)
    r.then_modify(c, n=c.n + 1)
    assert "setN($b0_0 + 1)" in seine_rs.compile_rules([r])
    res = seine_rs.run(r, {SFC10: {"n": [0]}})
    assert res.fired == 3


# --- Tier B exposure (round 22): group_by / collect_list/set / when_any

@fact
class GT:
    k: int
    v: int


@fact
class GOut:
    g: int
    total: int


def test_group_by_result_downstream():
    r = Rule("G")
    tot = r.group_by(GT, key=GT.k, agg=sum_(GT.v))
    r.then_insert(GOut, g=0, total=tot)
    assert "groupby( GT($k0 : k, $s0 : v); $k0; $a0 : sum($s0) )" in r.to_drl()
    res = seine_rs.run([r], {GT: {"k": [1, 1, 2], "v": [10, 20, 5]}, GOut: []})
    assert sorted(x["total"] for x in res.derived["GOut"].to_pylist()) == [5, 30]


def test_group_by_key_never_exposed():
    # the oracle REJECTS the key binding in the RHS ("$k cannot be
    # resolved", pr_ga_downstream's finding) — the API never hands it out
    r = Rule("G")
    r.group_by(GT, key=GT.k, agg=sum_(GT.v))
    with pytest.raises(CompileError, match="scoped inside"):
        r.patterns[0].k  # noqa: B018


def test_collect_list_set_fire_only():
    r = Rule("C")
    lst = r.accumulate(GT, agg=seine_rs.collect_list(GT.v))
    with pytest.raises(CompileError, match="collections"):
        r.then_insert(GOut, g=0, total=lst)
    r2 = Rule("C2")
    r2.accumulate(GT, agg=seine_rs.collect_set(GT.v))
    drl = seine_rs.compile_rules([r2])
    assert "collectSet($s0)" in drl
    with pytest.raises(CompileError, match="not certified"):
        Rule("C3").accumulate(GT, agg=seine_rs.collect_list(GT.v),
                              window=seine_rs.window_length(2))


@fact
class OA:
    x: int


@fact
class OB:
    y: int


def test_when_any_or_branches():
    r = Rule("O")
    r.when_any((OA, OA.x > 1), (OB, OB.y == 3))
    assert "( OA(x > 1) or OB(y == 3) )" in r.to_drl()
    assert seine_rs.run([r], {OA: {"x": [0]}, OB: {"y": [3]}}).fired == 1
    assert seine_rs.run([r], {OA: {"x": [5]}, OB: {"y": [0]}}).fired == 1
    # both branches alive = two subrule firings (the DNF expansion)
    assert seine_rs.run([r], {OA: {"x": [5]}, OB: {"y": [3]}}).fired == 2


def test_when_any_walls():
    r = Rule("w")
    p = r.when(GT)
    with pytest.raises(CompileError, match="alpha-only"):
        r.when_any((OA, OA.x == p.k), (OB,))
    with pytest.raises(CompileError, match="at least two"):
        Rule("w2").when_any((OA,))


# --- Tier C exposure (round 22): Allen operators + @duration intervals

@fact(event=seine_rs.Event(timestamp="ts", duration="dur"))
class IvA:
    ts: int
    dur: int


@fact(event=seine_rs.Event(timestamp="ts", duration="dur"))
class IvB:
    ts: int
    dur: int


def test_interval_events_and_overlaps():
    # @duration + expires inference (both certified engine machinery,
    # D-109/D-118) reach the Python surface together with the Allen set
    r = Rule("R")
    a = r.when(IvA)
    r.when(IvB, seine_rs.this_overlaps(a))
    assert "IvB(this overlaps $p0)" in r.to_drl()
    hit = seine_rs.run([r], {IvA: {"ts": [50], "dur": [100]}, IvB: {"ts": [0], "dur": [100]}})
    assert hit.fired == 1
    miss = seine_rs.run([r], {IvA: {"ts": [0], "dur": [10]}, IvB: {"ts": [50], "dur": [10]}})
    assert miss.fired == 0


def test_allen_params_render_and_fire():
    r = Rule("C")
    a = r.when(IvA)
    r.when(IvB, seine_rs.this_coincides(a, 5))
    assert "this coincides[5ms]" in r.to_drl()
    res = seine_rs.run([r], {IvA: {"ts": [100], "dur": [50]}, IvB: {"ts": [103], "dur": [50]}})
    assert res.fired == 1
    r2 = Rule("D")
    a2 = r2.when(IvA)
    r2.when(IvB, seine_rs.this_during(a2, 1, 10, 1, 10))
    assert "during[1ms,10ms,1ms,10ms]" in r2.to_drl()


def test_allen_arity_and_param_walls():
    r = Rule("w")
    a = r.when(IvA)
    with pytest.raises(CompileError, match="0/1 duration"):
        seine_rs.this_meets(a, 1, 2, 3)
    with pytest.raises(CompileError, match="non-negative"):
        seine_rs.this_during(a, -1)
    with pytest.raises(CompileError, match="0/1/2/4"):
        seine_rs.this_during(a, 1, 2, 3)


def test_event_duration_field_typed():
    with pytest.raises(CompileError, match="duration field"):
        @fact(event=seine_rs.Event(timestamp="ts", duration="tag"))
        class BadIv:
            ts: int
            tag: str


def test_expires_lint_scoped_to_after_before():
    # Allen ops carry no [lo,hi] window — the D-219 lifetime lint must
    # not fire on them (and inference-events have no declared expires)
    r = Rule("ok")
    a = r.when(IvA)
    r.when(IvB, seine_rs.this_includes(a))
    assert "this includes" in r.to_drl()
