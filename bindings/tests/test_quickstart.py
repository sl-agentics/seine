"""The README quickstart, verbatim — if this test moves, update the
README blocks (repo README.md + bindings/README.md) in the same
commit. It deliberately exercises the cold-start path: no facts= at
session build (auto-schema), single-arg insert_row, class-keyed result
read, why(), and the TMS auto-retract."""
import seine_rs as s


@s.fact
class Account:
    id: int
    balance: int            # cents; <= 0 == paid off


@s.fact
class Eligible:             # insertLogical: auto-retracts with its support
    account_id: int


def test_quickstart_block():
    rule = s.Rule("eligible")
    acc = rule.when(Account, Account.balance <= 0)
    rule.then_insert_logical(Eligible, account_id=acc.id)

    sess = s.Session([rule])                 # schemas auto-registered from the rule
    h = sess.insert_row(Account(id=42, balance=0))
    res = sess.fire()

    assert res.facts[Eligible].to_pylist() == [{"handle": 1, "account_id": 42}]
    why = sess.why(1)
    assert why["handle"] == 1 and [x["rule"] for x in why["supports"]] == ["eligible"]

    sess.delete(h)
    res = sess.fire()                        # support gone -> Eligible auto-retracts
    assert res.facts[Eligible].to_pylist() == []


def test_builder_chains_and_single_arg_insert():
    r = s.Rule("chain")
    a = r.when(Account, Account.balance <= 0)
    assert r.when_not(Eligible, Eligible.account_id == a.id) is r
    assert r.when_exists(Account) is r
    r.then_insert(Eligible, account_id=a.id)

    sess = s.Session([r])
    hs = sess.insert([Account(id=1, balance=0), Account(id=2, balance=5)])
    assert len(hs) == 2
    res = sess.fire()
    assert [x["account_id"] for x in res.derived[Eligible].to_pylist()] == [1]
