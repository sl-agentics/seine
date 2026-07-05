"""seine — differentially certified Drools-subset rule engine over Arrow.

Layer 1 (D-044): `run(drl, {type: table})` — DRL strings over Arrow
batches, WM-delta results.
Layer 2 (D-045): Pythonic authoring that compiles to DRL text — the
engine only ever sees the certified grammar.
"""
from seine._native import Session as _NativeSession, Result, Table, run as _native_run

from ._rows import is_row_list, rows_to_columns
from .authoring import (
    CompileError,
    Rule,
    average,
    compile_rules,
    count,
    fact,
    max_,
    min_,
    sum_,
)

__all__ = [
    "CompileError",
    "Result",
    "Rule",
    "Session",
    "Table",
    "average",
    "compile_rules",
    "count",
    "fact",
    "max_",
    "min_",
    "run",
    "sum_",
]


def _facts_arg(facts):
    """Accept @fact classes or string names as keys, and Arrow tables,
    dicts of column lists, or LISTS OF ROW OBJECTS (@fact instances,
    dicts, dataclasses, Pydantic models — anything with the fields as
    attributes) as values (D-048). Returns (facts, schemas): @fact
    class keys contribute explicit schemas, so empty row lists still
    declare their type."""
    if facts is None:
        return None, None
    out, schemas = {}, {}
    for k, v in facts.items():
        name = k if isinstance(k, str) else k.__name__
        if hasattr(k, "__seine_fields__"):
            schemas[name] = dict(k.__seine_fields__)
        if is_row_list(v):
            if len(v) == 0:
                if name not in schemas:
                    raise ValueError(
                        f"{name}: cannot infer a schema from an empty row list — "
                        "use a @seine.fact class key or a typed Arrow table"
                    )
                continue  # schema-only declaration
            v = rows_to_columns(k, v)
        out[name] = v
    return out, schemas or None


def _drl_arg(rules):
    """Accept a DRL string, a Rule, or a list of Rules."""
    if isinstance(rules, str):
        return rules
    if isinstance(rules, Rule):
        return rules.to_drl()
    return compile_rules(rules)


class Session:
    """seine.Session(rules, facts): `rules` is a DRL string, a Rule, or
    a list of Rules; `facts` maps type names OR @fact classes to Arrow
    tables, dicts of column lists, or lists of row objects. Thin
    delegating wrapper over the native session — the row sugar reshapes
    into the certified column path, nothing more."""

    def __init__(self, rules, facts=None):
        f, sch = _facts_arg(facts)
        self._native = _NativeSession(_drl_arg(rules), f, sch)

    def insert(self, type_or_name, data):
        """Insert a batch: Arrow table, dict of column lists, or a list
        of row objects. Returns the new facts' handles."""
        name = type_or_name if isinstance(type_or_name, str) else type_or_name.__name__
        if is_row_list(data):
            data = rows_to_columns(type_or_name, data)
        return self._native.insert(name, data)

    def insert_row(self, type_or_name, row):
        """Insert one fact: a dict or a row object. Returns its handle."""
        name = type_or_name if isinstance(type_or_name, str) else type_or_name.__name__
        if not isinstance(row, dict):
            row = rows_to_columns(type_or_name, [row])
            row = {f: vals[0] for f, vals in row.items()}
        return self._native.insert_row(name, row)

    def update(self, handle, **fields):
        return self._native.update(handle, **fields)

    def delete(self, handle):
        return self._native.delete(handle)

    def fire(self, fire_limit=100_000, on_fire=None):
        return self._native.fire(fire_limit, on_fire)


def run(rules, facts, fire_limit=100_000, on_fire=None):
    """Build, insert, fire once, return the Result."""
    f, sch = _facts_arg(facts)
    return _native_run(_drl_arg(rules), f, fire_limit, on_fire, sch)
