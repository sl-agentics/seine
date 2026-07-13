"""Row-object ingestion sugar tests (D-048): the sugar reshapes rows
into the certified column path and adds zero semantics."""
import dataclasses

import polars as pl
import pytest

import seine_rs
from seine_rs import Rule, fact


@fact
class Person:
    name: str
    age: int
    score: float


@fact
class Flagged:
    name: str
    score: float


def _rules():
    r = Rule("Adults")
    p = r.when(Person, Person.age >= 18)
    r.then_insert(Flagged, name=p.name, score=p.score)
    return r


def test_fact_instances_end_to_end():
    people = [
        Person("ada", 36, 91.5),
        Person("kurt", 17, 99.0),
        Person("grace", 45, 88.0),
    ]
    res = seine_rs.run(_rules(), {Person: people, Flagged: []})
    out = pl.DataFrame(res.derived["Flagged"]).sort("name")
    assert out["name"].to_list() == ["ada", "grace"]


def test_dict_rows_and_plain_objects():
    class Row:  # duck-typed, pydantic-like: fields as attributes
        def __init__(self, name, age, score):
            self.name, self.age, self.score = name, age, score

    people_dicts = [{"name": "ada", "age": 36, "score": 91.5}]
    people_objs = [Row("grace", 45, 88.0)]
    s = seine_rs.Session(_rules(), {Person: people_dicts, Flagged: []})
    s.insert(Person, people_objs)
    res = s.fire()
    assert res.fired == 2


def test_insert_row_object_and_handles():
    s = seine_rs.Session(_rules(), {Person: [Person("ada", 36, 91.5)], Flagged: []})
    h = s.insert_row(Person, Person("grace", 45, 88.0))
    assert isinstance(h, int)
    res = s.fire()
    assert res.fired == 2
    s.update(h, age=17)  # wrapper passthrough
    s.delete(h)
    assert s.fire().fired == 0


def test_empty_row_list_with_fact_key_declares_type():
    # [] under a @fact class key declares the type from the class schema
    res = seine_rs.run(_rules(), {Person: [Person("ada", 36, 91.5)], Flagged: []})
    assert res.fired == 1


def test_empty_row_list_with_string_key_errors():
    with pytest.raises(ValueError, match="empty row list"):
        seine_rs.run("rule R when T($x : v) then end\n", {"T": []})


def test_none_in_row_object_still_rejected():
    with pytest.raises(ValueError, match="None"):
        seine_rs.run(_rules(), {Person: [Person("ada", None, 1.0)], Flagged: []})


def test_missing_field_in_dict_row():
    with pytest.raises(ValueError, match="missing field"):
        seine_rs.run(_rules(), {Person: [{"name": "ada", "age": 36}], Flagged: []})


def test_mixed_rows_share_schema_order():
    # dicts + instances in one list: schema order comes from the key class
    rows = [{"score": 91.5, "age": 36, "name": "ada"}, Person("grace", 45, 88.0)]
    res = seine_rs.run(_rules(), {Person: rows, Flagged: []})
    assert res.fired == 2


def test_row_sugar_equals_column_path():
    people_rows = [Person("ada", 36, 91.5), Person("kurt", 17, 99.0)]
    people_cols = {"name": ["ada", "kurt"], "age": [36, 17], "score": [91.5, 99.0]}
    r1 = seine_rs.run(_rules(), {Person: people_rows, Flagged: []})
    r2 = seine_rs.run(_rules(), {Person: people_cols, Flagged: []})
    a1 = pl.DataFrame(r1.firings)
    a2 = pl.DataFrame(r2.firings)
    assert a1["values_json"].to_list() == a2["values_json"].to_list()
