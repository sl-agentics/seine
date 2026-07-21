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
    acc = rule.when(Account, Account.balance <= 0)   # when() returns the MATCH (bindings)
    rule.then_insert_logical(Eligible, account_id=acc.id)  # rule methods stay on `rule`

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


def test_machinery_errors_steer():
    # the machinery-level errors steer like the semantic ones do
    # (cold-start round 2: bare KeyError/AttributeError cost probes)
    rule = s.Rule("e2")
    acc = rule.when(Account, Account.balance <= 0)
    rule.then_insert_logical(Eligible, account_id=acc.id)
    sess = s.Session([rule])
    sess.insert_row(Account(id=1, balance=0))
    res = sess.fire()

    import pytest
    # the steering bar: each message contains the LITERAL next call,
    # built from actual state (attribute name + a type really present)
    with pytest.raises(KeyError, match=r"not a sequence.*Try res\.derived\['Account'\]"):
        res.derived[0]
    with pytest.raises(KeyError, match=r"no fact type 'Nope'.*'Eligible'.*Try res\.facts\['Account'\]"):
        res.facts["Nope"]
    assert res.facts.get("Nope") is None      # probing stays silent
    for guess in ("query_facts", "query_all", "live", "working_memory", "wm"):
        # the steer names a REAL registered type, not a placeholder —
        # the same literal-next-call bar as the KeyError family
        with pytest.raises(AttributeError, match=r"sess\.fire\(\)\.facts\['Account'\]"):
            getattr(sess, guess)
    with pytest.raises(AttributeError, match="did you mean"):
        sess.inserf_row
    with pytest.raises(AttributeError, match="methods: "):
        sess.zzz_nothing_close
    with pytest.raises(AttributeError, match="facts"):
        res.no_such_attr
    # dunders pass through untouched: hasattr/copy/pickle/inspect safe
    assert not hasattr(sess, "__deepcopy__")
    assert not hasattr(sess, "__wrapped__")


def test_pattern_miss_steers_to_rule():
    # the cold-start review's AttributeError: to_drl on when()'s return
    r = s.Rule("x")
    p = r.when(Account, Account.balance <= 0)
    import pytest
    with pytest.raises(AttributeError, match="lives on the Rule"):
        p.to_drl
    with pytest.raises(AttributeError, match="lives on the Rule"):
        p.then_insert
    assert p.balance is not None            # field access still works


def test_pattern_in_session_steers():
    # UAT #2's gotcha: Session([r.when(...)]) passes the MATCH where a
    # Rule belongs — the wall now names the fix, not just the type
    import pytest
    r = s.Rule("y")
    p = r.when(Account)
    with pytest.raises(s.CompileError, match=r"when\(\)'s return.*Session\(\[r\]\)"):
        s.Session([p])


def test_jvm_type_aliases_and_colon_eq_steer():
    # UAT #3's two findings: (a) schemas= accepts JVM long/double/boolean
    # (no more mid-file vocabulary switch with Layer-1 DRL); width-
    # ambiguous names steer. (b) ':=' unification steers to the D-051 form.
    import pytest
    drl = 'query "adults" (long $min)\n    Person(age == $min, $n : name)\nend\n'
    sess = s.Session(drl, schemas={"Person": {"name": "String", "age": "long", "score": "double", "vip": "boolean"}})
    assert s._normalize_schemas({"T": {"v": "double?"}}) == {"T": {"v": "f64?"}}
    sess.insert_row("Person", {"name": "ada", "age": 36, "score": 91.5, "vip": True})
    sess.fire()
    assert [r["$n"] for r in sess.query("adults", 36)] == ["ada"]
    with pytest.raises(s.CompileError, match="width-ambiguous"):
        s.Session(drl, schemas={"Person": {"age": "int"}})
    with pytest.raises(ValueError, match=r"D-051.*== \$var"):
        s.Session('query "q" (String $who)\n    Person($who := name)\nend\n',
                  schemas={"Person": {"name": "String"}})


def test_handle_panic_fixed_and_next_tier_steers():
    # D-382 GATE: a fabricated handle was a Rust PanicException
    # (BaseException — slips `except Exception`); now a ValueError steer.
    # Plus the next-tier scan's cheap pair: field-typo steer on @fact
    # classes and the insert-shape steer.
    import pytest
    rule = s.Rule("e3")
    acc = rule.when(Account, Account.balance <= 0)
    rule.then_insert_logical(Eligible, account_id=acc.id)
    sess = s.Session([rule])
    h = sess.insert_row(Account(id=1, balance=0))
    sess.fire()
    for op in (lambda: sess.update(999, balance=5), lambda: sess.delete(999),
               lambda: sess.update(-1, balance=5)):
        with pytest.raises(ValueError, match=r"no fact was ever created.*live handles"):
            op()
    # in-range DEAD handles keep certified semantics (D-047)
    sess.delete(h)
    sess.fire()
    assert sess.delete(h) == []                    # delete-of-dead no-op
    # field typo on the @fact class steers with the field list
    with pytest.raises(AttributeError, match=r"has no field 'blance'.*did you mean 'balance'"):
        Account.blance
    assert not hasattr(Account, "__origin__")      # dunders pass through
    # insert-shape: dict of scalars steers to insert_row
    with pytest.raises(TypeError, match=r"COLUMN lists.*insert_row"):
        sess.insert("Account", {"id": 1, "balance": 0})


def test_on_fire_raise_crosses_ffi_cleanly():
    # the last FFI-boundary probe: an observer raising propagates AS
    # ITSELF (no PanicException, no swallow); the run already completed
    # so effects persist and the session stays usable
    import pytest
    rule = s.Rule("e4")
    acc = rule.when(Account, Account.balance <= 0)
    rule.then_insert_logical(Eligible, account_id=acc.id)
    sess = s.Session([rule])
    sess.insert_row(Account(id=5, balance=0))

    def boom(rule_name, matches):
        raise RuntimeError("observer exploded")

    with pytest.raises(RuntimeError, match="observer exploded"):
        sess.fire(on_fire=boom)
    res = sess.fire()
    assert [x["account_id"] for x in res.facts[Eligible].to_pylist()] == [5]
