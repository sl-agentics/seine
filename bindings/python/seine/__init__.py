"""seine — differentially certified Drools-subset rule engine over Arrow.

Layer 1 (D-044): `run(drl, {type: table})` — DRL strings over Arrow
batches, WM-delta results.
Layer 2 (D-045): Pythonic authoring that compiles to DRL text — the
engine only ever sees the certified grammar.
"""
from seine._native import Session as _NativeSession, Result, Table, run as _native_run

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
    """Accept @fact classes or string names as fact-dict keys."""
    if facts is None:
        return None
    out = {}
    for k, v in facts.items():
        name = k if isinstance(k, str) else k.__name__
        out[name] = v
    return out


def _drl_arg(rules):
    """Accept a DRL string, a Rule, or a list of Rules."""
    if isinstance(rules, str):
        return rules
    if isinstance(rules, Rule):
        return rules.to_drl()
    return compile_rules(rules)


def Session(rules, facts=None):
    """seine.Session(rules, facts): `rules` is a DRL string, a Rule, or
    a list of Rules; `facts` maps type names OR @fact classes to Arrow
    tables / dicts of column lists."""
    return _NativeSession(_drl_arg(rules), _facts_arg(facts))


def run(rules, facts, fire_limit=100_000, on_fire=None):
    """One-shot: build, insert, fire, return the Result."""
    return _native_run(_drl_arg(rules), _facts_arg(facts), fire_limit, on_fire)
