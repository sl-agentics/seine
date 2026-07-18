"""seine_rs — differentially certified Drools-subset rule engine over Arrow.

Layer 1: `run(drl, {type: table})` — DRL strings over Arrow batches,
WM-delta results.
Layer 2: Pythonic authoring that compiles to DRL text — the engine
only ever sees the certified grammar.

The certification claim is interrogable: `seine_rs.certification()`
reports the pinned Drools oracle, the differential corpus this build
was stamped beside, and the source commit.
"""
from seine_rs._native import (
    Session as _NativeSession,
    Result,
    Table,
    run as _native_run,
    certification,
    __version__,
)

from . import derive
from ._rows import is_row_list, rows_to_columns
from .authoring import (
    CompileError,
    Event,
    this_after,
    this_before,
    this_coincides,
    this_during,
    this_finishedby,
    this_finishes,
    this_includes,
    this_meets,
    this_metby,
    this_overlappedby,
    this_overlaps,
    this_startedby,
    this_starts,
    Decimal,
    Rule,
    average,
    compile_rules,
    count,
    fact,
    max_,
    min_,
    sum_,
    collect_list,
    collect_set,
    window_length,
    window_time,
)

__all__ = [
    "CompileError",
    "Event",
    "this_after",
    "this_before",
    "this_coincides",
    "this_during",
    "this_finishedby",
    "this_finishes",
    "this_includes",
    "this_meets",
    "this_metby",
    "this_overlappedby",
    "this_overlaps",
    "this_startedby",
    "this_starts",
    "Result",
    "Rule",
    "Session",
    "Table",
    "average",
    "compile_rules",
    "count",
    "derive",
    "fact",
    "max_",
    "min_",
    "run",
    "sum_",
    "window_length",
    "window_time",
    "collect_list",
    "collect_set",
]


def _facts_arg(facts):
    """Accept @fact classes or string names as keys, and Arrow tables,
    dicts of column lists, or LISTS OF ROW OBJECTS (@fact instances,
    dicts, dataclasses, Pydantic models — anything with the fields as
    attributes) as values. Returns (facts, schemas): @fact
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
                        "use a @seine_rs.fact class key or a typed Arrow table"
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
    """seine_rs.Session(rules, facts): `rules` is a DRL string, a Rule, or
    a list of Rules; `facts` maps type names OR @fact classes to Arrow
    tables, dicts of column lists, or lists of row objects. Thin
    delegating wrapper over the native session — the row sugar reshapes
    into the certified column path, nothing more."""

    def __init__(self, rules, facts=None, schemas=None):
        f, sch = _facts_arg(facts)
        if schemas:
            sch = {**(sch or {}), **schemas}
        events = _collect_events(rules, facts)
        self._native = _NativeSession(_drl_arg(rules), f, sch, events or None)

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

    def advance(self, ms):
        """Advance the session pseudo-clock BY ms — a DELTA, not an
        absolute time (two advance(600) calls put the clock at 1200).
        Expired events leave working memory at the next fire's
        quiescence."""
        return self._native.advance(ms)

    def reset(self):
        """In-place reset for paged batches: clears all facts,
        the agenda, TMS state, the pseudo-clock and handle numbering;
        keeps the compiled rules and queries. The session behaves like
        a fresh one afterwards."""
        return self._native.reset()

    def fire(self, fire_limit=100_000, on_fire=None):
        return self._native.fire(fire_limit, on_fire)

    def why(self, handle):
        """Why does this fact hold? For a fact derived by
        ``insertLogical``: a dict — its handle, type, field values,
        ordered ``supports`` (each the justifying rule, the matched
        tuple's fact handles, and the firing seq), and any live
        ``stated_siblings`` of the same value. The support list is
        also the retraction contract: remove every support and the
        fact retracts. Returns None for stated facts, dead handles,
        and unknown ids — the graph never fabricates an answer."""
        return self._native.why(handle)

    def justifications(self):
        """The whole justification graph: every derived fact's
        :meth:`why` answer, ordered by fact handle."""
        return self._native.justifications()

    def query(self, name, *args):
        """Run a DRL query against current working memory (direct
        invocation). Positional args follow the query's parameter list;
        pass None for an UNBOUND parameter — its bindings come back in
        the rows. Returns rows as dicts keyed by the query's
        identifiers: facts as {"type", "handle", fields...}, scalars as
        plain values, or-branch-unbound as None.

        ROW ORDER: certified for a query invoked against a quiescent
        state (the fresh-call order; insertion-ordered with reinserts
        appended). Repeated calls interleaved with insert/delete churn
        may return the same row SET in arrival-window order instead —
        the engine's query state memory is shared with rule-side
        ?query conditions, whose cross-fire ordering is certified; the
        cross-CALL ordering under churn is not yet oracle-pinned."""
        return self._native.query(name, *args)


def _collect_events(rules, facts):
    """Event declarations from @fact(event=...) classes reachable via
    the Rule objects' patterns and the facts mapping's class keys."""
    out = {}
    def add(cls):
        ev = getattr(cls, "__seine_event__", None)
        if ev is not None:
            out[cls.__name__] = ev
    rlist = rules if isinstance(rules, (list, tuple)) else [rules]
    for r in rlist:
        for p in getattr(r, "patterns", []):
            add(p.cls)
        for a in getattr(r, "actions", []):
            c = a.kw.get("cls") if hasattr(a, "kw") else None
            if c is not None:
                add(c)
    if isinstance(facts, dict):
        for k in facts:
            if isinstance(k, type):
                add(k)
    return out


def run(rules, facts, fire_limit=100_000, on_fire=None, schemas=None):
    """Build, insert, fire once, return the Result."""
    f, sch = _facts_arg(facts)
    if schemas:
        sch = {**(sch or {}), **schemas}
    events = _collect_events(rules, facts)
    return _native_run(_drl_arg(rules), f, fire_limit, on_fire, sch, events or None)
