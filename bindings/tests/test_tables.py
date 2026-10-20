"""D-391 decision tables: the port of Drools' CSV RuleTable / RuleMatrix compiler.

The certification is text parity with Drools' SpreadsheetCompiler: every table in scenarios/tables/ has the oracle's
output banked beside it (tools/table_oracle.py bank), and `decision_table(...).drl` must match it byte for byte,
errors included. The engine side needs nothing new — `.rules` is ordinary DRL — so these tests only check that the
rules-only form runs, and that what has no Seine counterpart is refused instead of dropped."""
import os

import subprocess
import sys

import pytest
import seine_rs as s

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TABLES = os.path.join(ROOT, "scenarios", "tables")
CORPUS = sorted(f for f in os.listdir(TABLES) if f.endswith(".csv"))


@pytest.mark.parametrize("name", CORPUS)
def test_drl_is_byte_identical_to_drools(name):
    path = os.path.join(TABLES, name)
    with open(path[:-4] + ".drl", encoding="utf-8", newline="") as f:
        expected = f.read()
    try:
        got = s.decision_table(path, matrix=name.endswith(".matrix.csv")).drl
    except s.DecisionTableError as e:
        if e.drools_crashes:
            # Drools crashes on this table (an NPE, ...); Seine refuses it with a located message instead
            assert expected.startswith("ERROR ") and "DecisionTableParseException" not in expected, expected
            assert e.cell and e.cell in e.message
            return
        got = f"ERROR DecisionTableParseException: {e.message}"
    assert got == expected


BASIC = '''RuleTable Discounts
CONDITION,CONDITION,PRIORITY,ACTION
Person...,Person...,,
age > $param,"tier == ""$param""",,insert(new Discount($param));
Age over,Tier,,Discount
30,gold,5,10
,silver,,5
'''


@s.fact
class Person:
    age: int
    tier: str


@s.fact
class Discount:
    pct: int


def test_rules_run_on_the_engine():
    t = s.decision_table(text=BASIC)
    assert t.drl.startswith("package rule_table;\n//generated from Decision Table\n")
    assert not t.rules.startswith("package")              # rules-only: Seine DRL has no header
    res = s.run(t, {Person: [Person(age=40, tier="gold"), Person(age=20, tier="silver")], Discount: []})
    assert sorted(r["pct"] for r in res.facts[Discount].to_pylist()) == [5, 10]
    assert "Person(age > 30, tier == \"gold\")" in t.rules      # "Person...,Person..." merges into one pattern


def test_a_blank_cell_leaves_the_column_out():
    t = s.decision_table(text=BASIC)
    second = t.rules.split('\nrule "')[2]
    assert 'Person(tier == "silver")' in second and "age >" not in second


def test_errors_are_drools_messages():
    with pytest.raises(s.DecisionTableError, match=r"^No RuleTable cells in spreadsheet\.$"):
        s.decision_table(text="RuleSet,x\n")
    with pytest.raises(s.DecisionTableError, match=r"Priority is not an integer literal, in cell A6"):
        s.decision_table(text="RuleTable B\nPRIORITY,C\n,Person\n,age > $param\nl,l\nhigh,1\n")
    assert issubclass(s.DecisionTableError, s.CompileError)


@pytest.mark.parametrize("prop", ["Variables,java.util.List list", "Functions,function int f() { return 1; }",
                                  "Queries,query q() end", "Declare,declare T x : int end", "Unit,U",
                                  "Dialect,mvel", "NO-LOOP,true"])
def test_what_seine_has_no_form_for_is_refused_not_dropped(prop):
    t = s.decision_table(text=prop + "\n" + BASIC)
    assert t.drl                                          # Drools' text is still available
    with pytest.raises(s.DecisionTableError):
        t.rules


def test_imports_and_package_are_dropped_from_rules():
    t = s.decision_table(text="RuleSet,my set\nImport,org.acme.Person\nDialect,java\n" + BASIC)
    assert "import org.acme.Person;" in t.drl and "package my_set;" in t.drl
    assert "import" not in t.rules and "package" not in t.rules


GRID = """AgendaGroup,grid
Precondition,$c : Customer()
HorizontalCondition,Membership(tier == "$param")
VerticalCondition,Customer(age > $param)
Action,insert(new Discount($param));
RuleTable Grid,gold,silver
60,15,10
18,5,
"""


@s.fact
class Customer:
    age: int


@s.fact
class Membership:
    tier: str


def test_matrix_rules_fire_once_their_agenda_group_has_focus():
    t = s.decision_table(text=GRID, matrix=True)
    assert t.rules.count('agenda-group "grid"') == 3      # one rule per filled cell; the blank one makes none
    focus = 'rule "focus"\nwhen\n    Customer()\nthen\n    drools.setFocus("grid");\nend\n'
    res = s.run(focus + t.rules, {Customer: [Customer(age=70)], Membership: [Membership(tier="gold")], Discount: []})
    assert sorted(r["pct"] for r in res.facts[Discount].to_pylist()) == [5, 15]


@pytest.mark.parametrize("prop", ["AgendaGroup", "Precondition", "Action", "HorizontalCondition",
                                  "VerticalCondition"])
def test_matrix_missing_property_is_a_located_error_not_a_crash(prop):
    """Drools throws a NullPointerException (or "Script template is null") here; Seine names the property and cell."""
    text = "\n".join(l for l in GRID.split("\n") if not l.startswith(prop))
    with pytest.raises(s.DecisionTableError, match=rf"no {prop} property, needed for cell [A-Z]+\d+") as ei:
        s.decision_table(text=text, matrix=True)
    assert ei.value.drools_crashes and ei.value.cell


def test_matrix_blank_column_header_is_refused():
    """Drools silently shifts every later column onto the wrong condition; Seine refuses the grid."""
    text = GRID.replace("RuleTable Grid,gold,silver", "RuleTable Grid,gold,,silver").replace("60,15,10", "60,15,,10")
    with pytest.raises(s.DecisionTableError, match=r"column header cell C\d+ is blank but D\d+ after it is not"):
        s.decision_table(text=text, matrix=True)


def test_matrix_blank_row_header_is_refused():
    """Drools silently reuses the previous row's condition; Seine refuses the grid."""
    text = GRID.replace("18,5,", ",5,")
    with pytest.raises(s.DecisionTableError, match=r"row header A\d+ is blank"):
        s.decision_table(text=text, matrix=True)


def test_warnings_flag_drools_compatible_surprises():
    t = s.decision_table(text="RuleTable T\nCONDITION,BOGUS,ACTION\nPerson...,...,\nage > $param,,insert($param);\n"
                              "l,l,l\n1,true,2\n")
    assert any("BOGUS" in w and "ENABLED" in w for w in t.warnings)
    assert any('bare "..."' in w for w in t.warnings)
    assert not s.decision_table(text=BASIC).warnings


def test_unreadable_file_is_a_decision_table_error(tmp_path):
    with pytest.raises(s.DecisionTableError, match="cannot read the table") as ei:
        s.decision_table(tmp_path / "missing.csv")
    assert ei.value.source.endswith("missing.csv") and str(ei.value).startswith(ei.value.source)
    with pytest.raises(s.DecisionTableError, match="cannot read the table"):
        s.decision_table(tmp_path)                  # a directory


def test_bytes_and_binary_garbage_never_escape_as_other_exceptions():
    for data in (b"\xff\xfe\x00garbage", b"", b"\x00" * 50, "RuleTable".encode(), "RuleTable X\n\u0000".encode()):
        for matrix in (False, True):
            try:
                s.decision_table(text=data, matrix=matrix).rules
            except s.DecisionTableError as e:
                assert not e.message.startswith("internal error"), e.message


def test_command_line(tmp_path):
    env = {**os.environ, "PYTHONPATH": os.pathsep.join(sys.path)}
    ok = tmp_path / "t.csv"
    ok.write_text(BASIC)
    r = subprocess.run([sys.executable, "-m", "seine_rs", "table", "--rules", str(ok)], capture_output=True, text=True,
                       env=env)
    assert r.returncode == 0 and r.stdout.startswith("// rule values") and r.stderr == ""
    bad = tmp_path / "bad.csv"
    bad.write_text("RuleSet,x\n")
    r = subprocess.run([sys.executable, "-m", "seine_rs", "table", str(bad)], capture_output=True, text=True, env=env)
    assert r.returncode == 1 and r.stdout == ""
    assert r.stderr == f"error: {bad}: No RuleTable cells in spreadsheet.\n"
    r = subprocess.run([sys.executable, "-m", "seine_rs", "frobnicate"], capture_output=True, text=True, env=env)
    assert r.returncode == 2 and "unknown command" in r.stderr


def test_path_or_text_not_both():
    with pytest.raises(TypeError):
        s.decision_table()
    with pytest.raises(TypeError):
        s.decision_table("x.csv", text="y")
