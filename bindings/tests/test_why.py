"""Session.why() / Session.justifications() — the justification graph
at the Python surface (the engine-level graph is pinned in
engine/tests/tms_queryable.rs and tms_why_adversarial.rs; these tests
pin the BINDING: dict shapes, handle types, decimal rendering, and the
None contract)."""
from decimal import Decimal as D

import seine_rs
from seine_rs import Session


def test_why_supports_and_retraction_contract():
    drl = (
        "rule J1 when A() then insertLogical(new LK(9)); end\n"
        "rule J2 when B() then insertLogical(new LK(9)); end\n"
    )
    s = Session(
        drl,
        facts={"A": {"a": [1]}, "B": {"b": [2]}},
        schemas={"LK": {"v": "i64"}},
    )
    s.fire()
    js = s.justifications()
    assert len(js) == 1
    j = js[0]
    assert j["type"] == "LK" and j["fields"] == {"v": 9}
    # "handle" is the canonical key (the result tables' vocabulary);
    # "fact" stays as the compatibility alias — same value
    assert j["handle"] == j["fact"] and isinstance(j["handle"], int)
    assert s.why(j["handle"])["handle"] == j["handle"]
    assert [x["rule"] for x in j["supports"]] == ["J1", "J2"]
    assert all(isinstance(h, int) for x in j["supports"] for h in x["tuple"])
    assert s.why(j["fact"]) is not None

    # the retraction contract: drop every support and the fact retracts
    s.delete(j["supports"][0]["tuple"][0])
    s.fire()
    assert [x["rule"] for x in s.why(j["fact"])["supports"]] == ["J2"]
    cascade = s.delete(j["supports"][1]["tuple"][0])
    assert j["fact"] in cascade, "last support gone -> the fact retracts"
    assert s.why(j["fact"]) is None


def test_why_explains_the_ungrounded_orphan():
    # the support-counting orphan (certified): after the root dies, the
    # cycle survives and the graph says exactly why — each member's only
    # support is the other one
    drl = (
        "rule Seed when Root() then insertLogical(new M1(1)); end\n"
        "rule R12 when M1($v : v) then insertLogical(new M2($v)); end\n"
        "rule R21 when M2($v : v) then insertLogical(new M1($v)); end\n"
    )
    s = Session(
        drl,
        facts={"Root": {"r": [0]}},
        schemas={"M1": {"v": "i64"}, "M2": {"v": "i64"}},
    )
    s.fire()
    js = {j["type"]: j for j in s.justifications()}
    assert set(js) == {"M1", "M2"}
    s.delete(0)  # the only Root
    m1, m2 = s.why(js["M1"]["fact"]), s.why(js["M2"]["fact"])
    assert m1 is not None and m2 is not None, "the orphan cycle survives"
    assert [x["rule"] for x in m1["supports"]] == ["R21"]
    assert m1["supports"][0]["tuple"] == [js["M2"]["fact"]]
    assert [x["rule"] for x in m2["supports"]] == ["R12"]
    assert m2["supports"][0]["tuple"] == [js["M1"]["fact"]]


def test_acc_sources_closes_the_audit_chain():
    # the full title-release audit walk: why(Release) -> Balance; the
    # balance firing's match carries the aggregation result;
    # acc_sources(result) -> the line-item leaves, whose contributions
    # sum EXACTLY to the balance
    drl = (
        "rule balance when accumulate( Line($a : amount); $t : sum($a) ) "
        "then insert(new Balance($t)); end\n"
        "rule release when Balance($v : v, v <= 0.00) "
        "then insertLogical(new Release(1)); end\n"
    )
    s = Session(
        drl,
        facts={"Line": {"amount": ["100.10", "50.20", "-150.30"]}},
        schemas={"Line": {"amount": "decimal(18,2)"},
                 "Balance": {"v": "decimal(38,2)"},
                 "Release": {"k": "i64"}},
    )
    caught = []
    s.fire(on_fire=lambda rule, matches: caught.append((rule, matches)))
    rel = s.justifications()[0]
    assert rel["type"] == "Release"
    bal_firing = next(c for c in caught if c[0] == "balance")
    result_h = next(h for (t, h) in bal_firing[1] if t == "BigDecimal")
    src = s.acc_sources(result_h)
    assert len(src) == 3
    assert sum(v for (_, v) in src) == D("0.00")  # exact — accounts for the value
    assert all(isinstance(h, int) and isinstance(v, D) for (h, v) in src)
    # None contract: bogus and non-result handles never fabricate
    assert s.acc_sources(414141) is None
    assert s.acc_sources(src[0][0]) is None

    # recompute on deletion: the snapshot follows the current value
    s.delete(src[0][0])
    s.fire()
    src2 = s.acc_sources(result_h)
    assert len(src2) == 2 and src[0][0] not in [h for (h, _) in src2]


def test_why_renders_decimals_and_answers_none():
    # D-315: decimal literals cannot construct facts (error parity);
    # the justified Money carries a bound field instead
    drl = "rule J when A($v : v) then insertLogical(new Money($v)); end\n"
    s = Session(
        drl,
        facts={"A": {"v": [D("10.50")]}},
        schemas={"A": {"v": "decimal(18,2)"}, "Money": {"v": "decimal(18,2)"}},
    )
    s.fire()
    js = s.justifications()
    assert len(js) == 1
    assert js[0]["fields"]["v"] == D("10.50")
    assert isinstance(js[0]["fields"]["v"], D)
    # stated facts and bogus handles answer None, never fabricate
    assert s.why(0) is None  # the stated A
    assert s.why(424242) is None
