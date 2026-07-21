"""seine_rs — differentially certified Drools-subset rule engine over Arrow.

Layer 1: `run(drl, {type: table})` — DRL strings over Arrow batches,
WM-delta results.
Layer 2: Pythonic authoring that compiles to DRL text — the engine
only ever sees the certified grammar.

COMING FROM DROOLS? DRL here is RULES-ONLY. Don't write `package` or
`declare` — fact types live in Python (`@fact` classes or the
`facts=`/`schemas=` mappings), and the engine infers schemas from
them. One source of truth for schema; DRL owns only the logic.

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
    average_exact,
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
    "SessionResult",
    "Table",
    "average",
    "average_exact",
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


class _TypeTables(dict):
    """Result tables keyed by type NAME, also readable by @fact class —
    ``res.facts[Person]`` == ``res.facts["Person"]``. In/out symmetry
    with the ``facts=`` argument, which accepts both key kinds too.
    A miss raises a STEERING KeyError containing the literal next call
    (registered-but-empty types return an empty Table, never raise —
    so a miss is always a category error or an unknown/typo'd name)."""

    def __init__(self, data, label="facts"):
        super().__init__(data)
        self._label = label

    @staticmethod
    def _key(k):
        return k if isinstance(k, str) else getattr(k, "__name__", k)

    def __missing__(self, k):
        types = sorted(self)
        have = "[" + ", ".join(repr(t) for t in types) + "]" if types else "(none)"
        ex = f"res.{self._label}[{types[0]!r}]" if types else f"res.{self._label}['TypeName']"
        if not isinstance(k, str):
            raise KeyError(
                f"result tables are keyed by fact type name (str) or @fact "
                f"class, not position — a map, not a sequence. Types in "
                f"this result: {have}. Try {ex}"
            )
        raise KeyError(
            f"no fact type {k!r} in this result. Types here: {have}. "
            f"Try {ex} (or .get({k!r}) for None-if-absent)"
        )

    def __getitem__(self, k):
        return super().__getitem__(self._key(k))

    def get(self, k, default=None):
        return super().get(self._key(k), default)

    def __contains__(self, k):
        return super().__contains__(self._key(k))


class SessionResult:
    """A fire()/run() result — ONE object, two layers: ``facts`` (all
    live) and ``derived`` (this fire's new facts) are per-type Arrow
    table MAPS (not sequences) readable by type name OR @fact class;
    ``fired`` / ``firings`` / ``deleted_handles`` and every other
    native Result attribute reach through by delegation."""

    def __init__(self, native):
        self._native = native
        self.facts = _TypeTables(native.facts, "facts")
        self.derived = _TypeTables(native.derived, "derived")

    def __getattr__(self, name):
        try:
            return getattr(self._native, name)
        except AttributeError:
            raise AttributeError(
                f"SessionResult has no attribute {name!r}. The surface: "
                "facts (ALL live) / derived (this fire's delta) — "
                "type->Table maps keyed by name or @fact class — plus "
                "fired, firings, deleted_handles from the native result"
            ) from None

    def __repr__(self):
        return repr(self._native)


class Session:
    """seine_rs.Session(rules, facts=None): `rules` is a DRL string, a
    Rule, or a list of Rules; `facts` (optional) maps type names OR
    @fact classes to Arrow tables, dicts of column lists, or lists of
    row objects. Schemas for every @fact class the rules reference are
    registered automatically — ``Session([rule])`` alone works;
    ``facts=``/``schemas=`` entries take precedence.

    THE STATE MODEL: working-memory state is read off fire() results,
    not the session — ``res = sess.fire()`` then ``res.facts`` (ALL
    live facts) / ``res.derived`` (THIS fire's new facts), both
    per-type Arrow-table maps keyed by name or @fact class. The
    session side holds the mutators (insert/update/delete by handle)
    and the audit channels (why/justifications/acc_sources/query).
    Thin delegating wrapper over the native session — the row sugar
    reshapes into the certified column path, nothing more."""

    def __init__(self, rules, facts=None, schemas=None):
        f, sch = _facts_arg(facts)
        # auto-register schemas for every @fact class the rules
        # reference — explicit facts=/schemas= entries take precedence
        sch = {**_collect_schemas(rules), **(sch or {}), **(schemas or {})}
        events = _collect_events(rules, facts)
        self._native = _NativeSession(_drl_arg(rules), f, sch or None, events or None)

    def insert(self, type_or_name, data=None):
        """Insert a batch: Arrow table, dict of column lists, or a list
        of row objects. Returns the new facts' handles. With a list of
        @fact instances the type argument may be omitted —
        ``insert([Account(...), Account(...)])``."""
        if data is None:
            rows = type_or_name
            if not (isinstance(rows, list) and rows
                    and hasattr(type(rows[0]), "__seine_fields__")):
                raise TypeError(
                    "insert(rows) needs a non-empty list of @fact instances; "
                    "otherwise call insert(type_or_name, data)"
                )
            type_or_name, data = type(rows[0]), rows
        name = type_or_name if isinstance(type_or_name, str) else type_or_name.__name__
        if is_row_list(data):
            data = rows_to_columns(type_or_name, data)
        return self._native.insert(name, data)

    def insert_row(self, type_or_name, row=None):
        """Insert one fact; returns its handle. A @fact instance knows
        its own type, so ``insert_row(Account(id=42, balance=0))``
        suffices; the 2-arg form remains for dict rows and name-based
        insertion."""
        if row is None:
            if not hasattr(type(type_or_name), "__seine_fields__"):
                raise TypeError(
                    "insert_row(row) needs a @fact instance; otherwise call "
                    "insert_row(type_or_name, row)"
                )
            type_or_name, row = type(type_or_name), type_or_name
        name = type_or_name if isinstance(type_or_name, str) else type_or_name.__name__
        if not isinstance(row, dict):
            row = rows_to_columns(type_or_name, [row])
            row = {f: vals[0] for f, vals in row.items()}
        return self._native.insert_row(name, row)

    def update(self, handle, **fields):
        """Update a live fact in place by HANDLE (no type argument —
        the engine tracks types internally): ``sess.update(h,
        balance=20.0)``. Only the named fields change. Takes effect at
        the next fire(), where property reactivity decides what
        re-evaluates."""
        return self._native.update(handle, **fields)

    def delete(self, handle):
        """Delete a live fact by HANDLE. Returns the SYNCHRONOUS TMS
        retraction cascade (handles of beliefs that died with their
        support; often empty). Some unjustifications land lazily at
        the justifying rule's next network evaluation instead — the
        next fire()'s WM-delta is the complete record."""
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
        """Run the rules to quiescence; returns a :class:`SessionResult`
        (this fire's WM-delta). ``on_fire(rule, matches)`` is a
        post-quiescence OBSERVER invoked per firing in firing order —
        two arguments: the rule name and the match tuple as a list of
        (type, handle) pairs. Observers receive plain data and cannot
        call back into the session; collect handles there, query after
        fire() returns."""
        return SessionResult(self._native.fire(fire_limit, on_fire))

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

    def acc_sources(self, handle):
        """Which facts fed this aggregation result? ``handle`` is an
        accumulate/groupby RESULT fact — the hidden fact a firing's
        match tuple carries, visible in ``fire(on_fire=...)`` as a
        (type, handle) pair. Returns ``[(source_handle, contribution),
        ...]`` in match order, snapshotted at the computation that
        produced the result's current value — the contributions always
        account for that value. An aggregation over an empty source
        answers ``[]``. Returns None for dead or non-result handles —
        the audit never fabricates. Closes the aggregation gap in the
        :meth:`why` chain: walk why() through the logical layer, then
        acc_sources() through the summation to the line-item leaves."""
        return self._native.acc_sources(handle)

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

    # The names an API explorer reaches for on the statistical prior
    # "read facts from the session." They 404 ON PURPOSE — a live
    # mid-epoch WM read has no certified semantics — but the miss
    # STEERS to the certified path instead of a bare AttributeError.
    _READ_GUESSES = frozenset({
        "query_facts", "query_all", "get_facts", "all_facts",
        "facts", "derived", "live", "live_facts", "get_all",
        "working_memory", "wm",
    })

    def __getattr__(self, name):
        # dunders/privates pass through untouched — intercepting them
        # would quietly break hasattr/copy/pickle/inspect
        if name.startswith("_"):
            raise AttributeError(name)
        if name in self._READ_GUESSES:
            raise AttributeError(
                f"Session has no {name!r}. To read live facts, fire and "
                "index the result: sess.fire().facts['TypeName'] (ALL "
                "live) / .derived (this fire's new facts). To run a DRL "
                "query: sess.query(name, *args). The session holds "
                "mutators (insert/update/delete) and audit channels "
                "(why/justifications/acc_sources)"
            )
        import difflib
        methods = sorted(m for m in dir(type(self)) if not m.startswith("_"))
        close = difflib.get_close_matches(name, methods, n=2)
        hint = (
            f"; did you mean {' or '.join(close)}?"
            if close
            else f"; methods: {', '.join(methods)}"
        )
        raise AttributeError(f"Session has no attribute {name!r}{hint}")


def _collect_schemas(rules):
    """Schemas of every @fact class the rule objects reference
    (patterns and actions) — the same walk as _collect_events. Lets
    Session([rule]) work with no facts= at all: authoring a rule
    BEFORE having data is the normal first move, and the rule already
    holds the class objects."""
    out = {}
    rlist = rules if isinstance(rules, (list, tuple)) else [rules]
    for r in rlist:
        for p in getattr(r, "patterns", []):
            cls = getattr(p, "cls", None)
            if hasattr(cls, "__seine_fields__"):
                out[cls.__name__] = dict(cls.__seine_fields__)
        for a in getattr(r, "actions", []):
            c = a.kw.get("cls") if hasattr(a, "kw") else None
            if hasattr(c, "__seine_fields__"):
                out[c.__name__] = dict(c.__seine_fields__)
        for grp in getattr(r, "or_groups", []):
            for cls, _ in grp:
                if hasattr(cls, "__seine_fields__"):
                    out[cls.__name__] = dict(cls.__seine_fields__)
    return out


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
    """Build, insert, fire once, return the :class:`SessionResult`."""
    f, sch = _facts_arg(facts)
    if schemas:
        sch = {**(sch or {}), **schemas}
    sch = {**_collect_schemas(rules), **(sch or {})}
    events = _collect_events(rules, facts)
    return SessionResult(
        _native_run(_drl_arg(rules), f, fire_limit, on_fire, sch or None, events or None)
    )
