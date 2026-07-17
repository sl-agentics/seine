"""Pythonic rule authoring that COMPILES TO DRL TEXT.

Every construct here builds a declarative AST at rule-definition time
and renders it into the certified DRL subset (engine/src/drl.rs is the
frozen grammar). Nothing evaluates Python in the match loop: the same
differential guarantees that cover hand-written DRL cover these rules
verbatim, because the engine only ever sees the generated DRL.

Anything the closed grammar cannot express is a `CompileError` at
authoring time — the same fencing philosophy as custom accumulate
functions and MVEL salience bodies on the DRL side.
"""
from __future__ import annotations

import dataclasses
import decimal as _pydecimal
import typing
from typing import Any, Optional, Union


class CompileError(Exception):
    """The construct falls outside the certified rule grammar."""


class Decimal:
    """Field metadata marker: exact decimal with declared precision/scale
    Use inside Annotated on a `decimal.Decimal` field:

        @seine_rs.fact
        class Loan:
            balance: Annotated[Decimal, seine_rs.Decimal(18, 2)]
            rate: Optional[Annotated[Decimal, seine_rs.Decimal(9, 6)]]

    Arrow Decimal128-compatible: 1 <= p <= 38, 0 <= s <= p (validated
    here, matching the engine's i128 storage limits). Prefer the
    module-qualified spelling `seine_rs.Decimal` — a bare `from
    seine_rs import Decimal` shadows `decimal.Decimal`.
    """

    def __init__(self, p: int, s: int):
        if not (isinstance(p, int) and isinstance(s, int)):
            raise CompileError("seine_rs.Decimal(p, s): p and s must be ints")
        if not (1 <= p <= 38 and 0 <= s <= p):
            raise CompileError(
                f"seine_rs.Decimal({p}, {s}): needs 1 <= p <= 38 and 0 <= s <= p "
                "(Arrow Decimal128 / engine i128 limits)"
            )
        self.p, self.s = p, s


def _resolve_field_type(cls_name: str, fname: str, hint: Any) -> str:
    """Normalize a type hint into a subset type string:
    Optional and Annotated unwrap AT ANY NESTING; Annotated metadata is
    collected wherever it appears; the result is one of i64/f64/String/
    bool/decimal(p,s), with a trailing '?' when nullable.

    Nullability semantics are LEGIBLE API semantics: the type
    declaration IS the NaN-vs-NULL choice. `Optional[float]` ingests
    pandas/Arrow NaN as NULL (SQL 3VL); bare `float` keeps NaN
    as a bit-exact IEEE value (certified behavior).
    """
    nullable = False
    metas: list = []
    t = hint
    while True:
        origin = typing.get_origin(t)
        if origin is typing.Annotated:
            args = typing.get_args(t)
            metas.extend(args[1:])
            t = args[0]
            continue
        if origin in (Union, getattr(__import__("types"), "UnionType", ())):
            args = [a for a in typing.get_args(t) if a is not type(None)]
            if len(args) != len(typing.get_args(t)):
                nullable = True
            if len(args) != 1:
                raise CompileError(
                    f"@fact {cls_name}.{fname}: only Optional[X] unions are in the subset"
                )
            t = args[0]
            continue
        break
    dec_meta = [m for m in metas if isinstance(m, Decimal)]
    if t is _pydecimal.Decimal:
        if not dec_meta:
            raise CompileError(
                f"@fact {cls_name}.{fname}: bare Decimal has no precision — declare it "
                f"Annotated[Decimal, seine_rs.Decimal(p, s)] (e.g. seine_rs.Decimal(18, 2)). "
                "Silent money precision is walled."
            )
        m = dec_meta[0]
        return f"decimal({m.p},{m.s})" + ("?" if nullable else "")
    if dec_meta:
        raise CompileError(
            f"@fact {cls_name}.{fname}: seine_rs.Decimal(p, s) metadata belongs on a "
            "decimal.Decimal field"
        )
    if t not in _PY_TO_SUBSET:
        raise CompileError(
            f"@fact {cls_name}.{fname}: type {getattr(t, '__name__', t)!r} is outside "
            "the certified subset (int, float, str, bool, Decimal)"
        )
    return _PY_TO_SUBSET[t] + ("?" if nullable else "")


_PY_TO_SUBSET = {int: "i64", float: "f64", str: "String", bool: "bool"}
_SUBSET_ARROW = {"i64": "int64", "f64": "float64", "String": "string", "bool": "bool"}
_CMP = {"eq": "==", "ne": "!=", "lt": "<", "le": "<=", "gt": ">", "ge": ">="}


def _lit(v: Any) -> str:
    """Render a Python literal into DRL constraint syntax."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, _pydecimal.Decimal):
        s = str(v)
        if "E" in s or "e" in s:
            s = format(v, "f")
        return s
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        s = repr(v)
        # the DRL lexer takes plain decimal floats only
        if "e" in s or "E" in s or "inf" in s or "nan" in s:
            raise CompileError(
                f"float literal {v!r} has no plain-decimal DRL rendering; "
                "use a binding or rescale the value"
            )
        return s
    if isinstance(v, str):
        if '"' in v or "\\" in v or "\n" in v:
            raise CompileError(
                f"string literal {v!r}: quotes/backslashes/newlines are outside "
                "the certified literal syntax"
            )
        return f'"{v}"'
    raise CompileError(f"unsupported literal type {type(v).__name__}")


def _reject_callable(x: Any, where: str) -> None:
    if callable(x) and not isinstance(x, (FieldRef, BoundField, SalExpr)):
        raise CompileError(
            f"{where}: Python callables cannot run in the match loop — express "
            "the condition with field operators (Type.field >= 10) so it "
            "compiles into the certified grammar"
        )


def _ambiguous_bool(kind: str):
    """A raising __bool__ for expression-AST nodes (the numpy/pandas/
    SQLAlchemy move): Python's `and`/`or` and chained comparisons
    (10 < x < 100 desugars to `10 < x and x < 100`) decide
    short-circuiting with bool(), so default truthiness would silently
    DROP one operand from the compiled rule — a misfire, not an error."""
    def __bool__(self):
        raise CompileError(
            f"the truth value of a {kind} is ambiguous: Python's "
            "`and`/`or` and chained comparisons (10 < x < 100) "
            "short-circuit through bool() and would silently drop a "
            "constraint. Combine with `&` / `|` / `~`, pass several "
            "constraints to when(...) for AND, and write a range as two "
            "constraints (x > 10, x < 100)."
        )
    return __bool__


def _precedence_trap(kind: str):
    """Raising &/| for field expressions: `a > 10 & b < 100` parses as
    `a > (10 & b) < 100` because &/| bind TIGHTER than comparisons —
    the bare-field operand is always that precedence slip, never a
    legitimate combinator (legitimate `&`/`|` join two _Constraints
    and never touch a field expression)."""
    def _op(self, other):
        raise CompileError(
            f"`&`/`|` reached a {kind}: Python's `&` and `|` bind TIGHTER "
            "than comparisons, so `a > 10 & b < 100` parses as "
            "`a > (10 & b) < 100`. Parenthesize each comparison — "
            "(Order.amount > 10) & (Order.amount < 100) — and write a "
            "bare boolean field as `field == True`."
        )
    return _op


# ---------------------------------------------------------------------
# Fact classes and field expressions
# ---------------------------------------------------------------------

class FieldRef:
    """A typed field of a fact CLASS; operator overloads build
    constraint AST nodes, never evaluate."""

    __bool__ = _ambiguous_bool("field expression")
    __and__ = __rand__ = __or__ = __ror__ = _precedence_trap("field expression")

    def __init__(self, owner: type, name: str, subset_type: str):
        self.owner = owner
        self.name = name
        self.subset_type = subset_type

    # -- comparisons -> Constraint
    def _cmp(self, op: str, other: Any):
        if other is None:
            good = "is_null()" if op == "==" else "is_not_null()"
            raise CompileError(
                f"{self.owner.__name__}.{self.name} {op} None: null tests are "
                f"EXPLICIT three-valued logic - use .{good} (the "
                "field must be declared Optional to be nullable)"
            )
        _reject_callable(other, f"{self.owner.__name__}.{self.name} {op}")
        if isinstance(other, BoundField):
            return _Constraint(self, op, other)
        if isinstance(other, FieldRef):
            raise CompileError(
                f"{self.owner.__name__}.{self.name} {op} {other.owner.__name__}."
                f"{other.name}: compare against a MATCHED pattern's field "
                "(the object returned by rule.when(...)), not the class"
            )
        return _Constraint(self, op, other)

    def __eq__(self, other):  # type: ignore[override]
        return self._cmp("==", other)

    def __ne__(self, other):  # type: ignore[override]
        return self._cmp("!=", other)

    def __lt__(self, other):
        return self._cmp("<", other)

    def __le__(self, other):
        return self._cmp("<=", other)

    def __gt__(self, other):
        return self._cmp(">", other)

    def __ge__(self, other):
        return self._cmp(">=", other)

    def matches(self, regex: str):
        # nullable String is fine: a null operand makes the constraint
        # UNKNOWN (SQL 3VL, certified in the duckdb tier) — the null row
        # is simply never admitted
        if not isinstance(regex, str):
            raise CompileError(".matches() takes a literal regex string")
        if self.subset_type not in ("String", "String?"):
            raise CompileError(f".matches() requires a str field, {self.name} is {self.subset_type}")
        return _Constraint(self, "matches", regex)

    def contains(self, needle: str):
        if not isinstance(needle, str):
            raise CompileError(".contains() takes a literal string")
        if self.subset_type not in ("String", "String?"):
            raise CompileError(f".contains() requires a str field, {self.name} is {self.subset_type}")
        return _Constraint(self, "contains", needle)

    def is_null(self):
        """SQL 3VL null test: renders `field == null`."""
        return _Constraint(self, "==", _NULL)

    def is_not_null(self):
        return _Constraint(self, "!=", _NULL)

    def in_(self, *items):
        return _Constraint(self, "in", list(items))

    def not_in(self, *items):
        return _Constraint(self, "not in", list(items))

    def _no_class_arith(self, *_a, **_k):
        raise CompileError(
            f"{self.owner.__name__}.{self.name}: salience expressions use fields "
            "of a MATCHED pattern (the object returned by rule.when(...)), "
            "not the class"
        )

    __add__ = __radd__ = __sub__ = __rsub__ = __mul__ = __rmul__ = _no_class_arith

    def __hash__(self):
        return hash((self.owner, self.name))

    def __repr__(self):
        return f"{self.owner.__name__}.{self.name}"


class Event:
    """CEP event declaration for @fact: the fact type
    becomes a point event on the session pseudo-clock.

        @seine_rs.fact(event=seine_rs.Event(timestamp="ts", expires_ms=5_000))
        class Reading:
            ts: int
            value: float

    `timestamp` names an int field holding the event time in ms.
    `expires_ms` declares the lifetime explicitly; omit it (None) and
    the engine INFERS expiration from the rules' temporal constraints —
    the certified Drools behavior. `duration` names an int field holding
    the event's length in ms, making it an INTERVAL event: the Allen
    operators (this_overlaps, this_during, this_coincides, ...) compare
    interval endpoints, and over point events most of them degenerate
    toward before/after/coincides."""

    def __init__(
        self,
        timestamp: str,
        expires_ms: "int | None" = None,
        duration: "str | None" = None,
    ):
        if not isinstance(timestamp, str) or not timestamp:
            raise CompileError("seine_rs.Event: timestamp must name an int field")
        if expires_ms is not None and (
            not isinstance(expires_ms, int) or isinstance(expires_ms, bool) or expires_ms < 0
        ):
            raise CompileError(
                "seine_rs.Event: expires_ms must be a non-negative int, or None "
                "to let the engine infer expiration from the temporal constraints"
            )
        if duration is not None and (not isinstance(duration, str) or not duration):
            raise CompileError(
                "seine_rs.Event: duration must name an int field (ms) — it makes "
                "the type an interval event"
            )
        self.duration = duration
        self.timestamp = timestamp
        self.expires_ms = expires_ms


def fact(cls: type = None, *, event: "Event | None" = None) -> type:
    """Declare a fact type from an annotated class:

        @seine_rs.fact
        class Person:
            name: str
            age: int

    Annotations map onto the certified subset (int -> i64, float -> f64,
    str -> String, bool -> bool). The class becomes a dataclass (usable
    for row construction) and its class attributes become FieldRefs for
    rule expressions.
    """
    # get_type_hints resolves PEP-563 stringized annotations (the raw
    # __annotations__ read broke under `from __future__ import
    # annotations`) and include_extras keeps Annotated metadata (D-098).
    if cls is None:
        # parameterized: @fact(event=...)
        def _wrap(c: type) -> type:
            return fact(c, event=event)
        return _wrap
    if event is not None and not isinstance(event, Event):
        raise CompileError("@fact(event=...) takes a seine_rs.Event")
    try:
        ann = typing.get_type_hints(cls, include_extras=True)
    except Exception as ex:
        raise CompileError(f"@fact {cls.__name__}: cannot resolve annotations: {ex}")
    ann = {k: v for k, v in ann.items() if typing.get_origin(v) is not typing.ClassVar}
    if not ann:
        raise CompileError(f"@fact {cls.__name__}: no annotated fields")
    fields = {}
    for name, py_t in ann.items():
        if name == "handle":
            raise CompileError(
                f"@fact {cls.__name__}: the field name 'handle' is reserved — "
                f"result tables carry the engine's fact handle in a column of "
                f"that name; rename the field"
            )
        fields[name] = _resolve_field_type(cls.__name__, name, py_t)
    dc = dataclasses.dataclass(cls)
    dc.__seine_fields__ = fields  # ordered: annotation order = constructor order
    if event is not None:
        ts_t = fields.get(event.timestamp)
        if ts_t is None:
            raise CompileError(
                f"@fact {cls.__name__}: event timestamp field "
                f"{event.timestamp!r} is not declared on the class"
            )
        if ts_t != "i64":
            raise CompileError(
                f"@fact {cls.__name__}: event timestamp field "
                f"{event.timestamp!r} must be int (ms), it is {ts_t}"
            )
        if event.duration is not None:
            dur_t = fields.get(event.duration)
            if dur_t != "i64":
                raise CompileError(
                    f"@fact {cls.__name__}: event duration field "
                    f"{event.duration!r} must be int (ms), it is {dur_t}"
                )
        dc.__seine_event__ = (event.timestamp, event.expires_ms, event.duration)
    for name, st in fields.items():
        setattr(dc, name, FieldRef(dc, name, st))
    return dc


class BoundField:
    """A field of a MATCHED pattern (`p.age` where `p = r.when(Person)`),
    usable in later constraints, RHS args, accumulate args and salience
    expressions. Compiles to a `$binding : field` declaration."""

    __bool__ = _ambiguous_bool("field expression")
    __and__ = __rand__ = __or__ = __ror__ = _precedence_trap("field expression")

    def __init__(self, pattern: "_Pattern", name: str, subset_type: str):
        self.pattern = pattern
        self.name = name
        self.subset_type = subset_type

    # salience / arithmetic (closed grammar: single binary op)
    def _arith(self, op: str, other, reflected=False):
        if self.subset_type not in ("i64", "f64"):
            raise CompileError(f"salience arithmetic requires numeric fields, {self.name} is {self.subset_type}")
        if isinstance(other, BoundField):
            if other.subset_type not in ("i64", "f64"):
                raise CompileError(f"salience arithmetic requires numeric fields, {other.name} is {other.subset_type}")
            a, b = (other, self) if reflected else (self, other)
            return SalExpr(a, op, b)
        if isinstance(other, int) and not isinstance(other, bool):
            a, b = (other, self) if reflected else (self, other)
            return SalExpr(a, op, b)
        raise CompileError(
            f"salience terms are int literals or numeric bindings, got {type(other).__name__}"
        )

    def __add__(self, other):
        return self._arith("+", other)

    def __radd__(self, other):
        return self._arith("+", other, reflected=True)

    def __sub__(self, other):
        return self._arith("-", other)

    def __rsub__(self, other):
        return self._arith("-", other, reflected=True)

    def __mul__(self, other):
        return self._arith("*", other)

    def __rmul__(self, other):
        return self._arith("*", other, reflected=True)

    def __hash__(self):
        return hash((id(self.pattern), self.name))

    def __repr__(self):
        return f"<{self.pattern.type_name}.{self.name} of pattern {self.pattern.index}>"


class SalExpr:
    """A salience expression: term or term-op-term, closed grammar."""

    __bool__ = _ambiguous_bool("salience expression")

    def __init__(self, a, op: Optional[str] = None, b=None):
        for t in (a, b):
            if t is not None and not isinstance(t, (BoundField, int)):
                raise CompileError("salience terms are int literals or numeric bindings")
        if isinstance(a, SalExpr) or isinstance(b, SalExpr):
            raise CompileError(
                "salience expressions are a single `term op term` in the certified "
                "grammar — nested arithmetic does not compile"
            )
        self.a, self.op, self.b = a, op, b

    def _arith(self, *_a, **_k):
        raise CompileError(
            "salience expressions are a single `term op term` in the certified "
            "grammar — nested arithmetic does not compile"
        )

    __add__ = __sub__ = __mul__ = _arith


class AccResult(BoundField):
    """The result binding of an accumulate — a BoundField whose pattern
    is the accumulate CE. Aggregate typing walls apply."""

    def __init__(self, pattern: "_Pattern", func: str, arg: Optional[BoundField]):
        st = {
            "count": "i64",
            "average": "f64",
            "sum": arg.subset_type if arg is not None else "i64",
            "min": arg.subset_type if arg is not None else "i64",
            "max": arg.subset_type if arg is not None else "i64",
            "collectList": "List",
            "collectSet": "Set",
        }[func]
        super().__init__(pattern, f"__acc_{func}", st)
        self.func = func
        self.arg = arg
        if func in ("min", "max") and st == "f64":
            self.opaque = True  # D-039: compiles nowhere downstream
            self._why = (
                f"{func}() over a float field yields an opaque Number in "
                "Drools: it cannot be used in {use}. Aggregate an int field, "
                "or keep the result unused."
            )
        elif func in ("collectList", "collectSet"):
            # the collection value lives in the match (and the firings
            # audit); no subset field type can carry it downstream
            self.opaque = True
            self._why = (
                f"{func}() results are collections — no certified field "
                "type can carry one into {use}. Use count()/sum() for a "
                "scalar, or the collect() CE if you need the collection "
                "observable in the firings audit."
            )
        else:
            self.opaque = False
            self._why = ""

    def _guard_opaque(self, use: str):
        if self.opaque:
            raise CompileError(self._why.format(use=use))

    def _arith(self, op, other, reflected=False):
        raise CompileError(
            "accumulate results in salience expressions are not certified "
            "against the oracle; compute the aggregate into a fact "
            "and reference that instead"
        )


# aggregate constructors -------------------------------------------------

class _Agg:
    def __init__(self, func: str, arg: Optional[FieldRef]):
        self.func, self.arg = func, arg


def collect_list(field: FieldRef) -> _Agg:
    """`collectList($v)` — the ordered value collection (match-visible
    and in the firings audit; collections have no downstream field type)."""
    return _Agg("collectList", field)


def collect_set(field: FieldRef) -> _Agg:
    """`collectSet($v)` — the counted value set (a duplicate must be
    removed as many times as it was added before the set shrinks)."""
    return _Agg("collectSet", field)


class _Window:
    """Sliding window on an ACCUMULATE SOURCE — the certified placement
    (standalone-pattern windows are outside the subset). time keeps
    events whose timestamp is within the last N ms of the pseudo-clock;
    length keeps the N most recently inserted."""

    def __init__(self, kind: str, n: int):
        if not isinstance(n, int) or isinstance(n, bool):
            raise CompileError(f"window_{kind}: expected an int, got {type(n).__name__}")
        if n < 1:
            raise CompileError(
                f"window_{kind}({n}): needs n >= 1"
                + (" (window:length(0) throws in Drools)" if kind == "length" else "")
            )
        self.kind = kind
        self.n = n

    def render(self) -> str:
        if self.kind == "time":
            return f"over window:time({self.n}ms)"
        return f"over window:length({self.n})"


def window_time(ms: int) -> _Window:
    """`over window:time(N ms)` for an accumulate source: aggregate only
    events whose timestamp is within the last N ms of the session clock.
    The source class must be a declared event (@fact(event=...))."""
    return _Window("time", ms)


def window_length(n: int) -> _Window:
    """`over window:length(N)` for an accumulate source: aggregate only
    the N most recently inserted events."""
    return _Window("length", n)


def sum_(field: FieldRef) -> _Agg:
    return _Agg("sum", field)


def count() -> _Agg:
    return _Agg("count", None)


def average(field: FieldRef) -> _Agg:
    return _Agg("average", field)


def min_(field: FieldRef) -> _Agg:
    return _Agg("min", field)


def max_(field: FieldRef) -> _Agg:
    return _Agg("max", field)


# ---------------------------------------------------------------------
# Constraint / pattern / rule AST
# ---------------------------------------------------------------------

class _Null:
    """Render sentinel for is_null()/is_not_null()."""


_NULL = _Null()


class _Constraint:
    __bool__ = _ambiguous_bool("constraint")

    def __init__(self, field: FieldRef, op: str, rhs: Any):
        self.field, self.op, self.rhs = field, op, rhs

    # -- inline boolean groups (D-073; rendered with explicit parens) --
    def __or__(self, other):
        return _Group("||", [self, other])

    def __and__(self, other):
        return _Group("&&", [self, other])

    def __invert__(self):
        return _Group("!", [self])

    def render(self, rule: "Rule") -> str:
        f = self.field.name
        if isinstance(self.rhs, _Null):
            return f"{f} {self.op} null"
        if self.op in ("in", "not in"):
            items = ", ".join(_lit(v) for v in self.rhs)
            return f"{f} {self.op} ({items})"
        if self.op in ("matches", "contains"):
            return f"{f} {self.op} {_lit(self.rhs)}"
        if isinstance(self.rhs, BoundField):
            var = rule._binding_for(self.rhs, use="a join constraint")
            return f"{f} {self.op} {var}"
        return f"{f} {self.op} {_lit(self.rhs)}"


class _Group:
    """Inline boolean constraint group: `(a || b)`, `(a && b)`,
    `!(a)` - same-pattern fields only."""

    __bool__ = _ambiguous_bool("constraint group")

    def __init__(self, op, children):
        for c in children:
            if not isinstance(c, (_Constraint, _Group)):
                raise CompileError(
                    "boolean groups combine field constraints of ONE pattern "
                    f"(got {type(c).__name__})"
                )
        self.op = op
        self.children = list(children)

    def __or__(self, other):
        return _Group("||", [self, other])

    def __and__(self, other):
        return _Group("&&", [self, other])

    def __invert__(self):
        return _Group("!", [self])

    def owners(self):
        out = set()
        for c in self.children:
            if isinstance(c, _Group):
                out |= c.owners()
            else:
                out.add(c.field.owner)
        return out

    def render(self, rule):
        if self.op == "!":
            return f"!({self.children[0].render(rule)})"
        inner = f" {self.op} ".join(c.render(rule) for c in self.children)
        return f"({inner})"


class _Pattern:
    def __init__(self, rule: "Rule", index: int, cls: type, constraints, ce: str,
                 agg: Optional[_Agg] = None):
        self.rule = rule
        self.index = index
        self.cls = cls
        self.type_name = cls.__name__
        self.constraints = list(constraints)
        self.ce = ce  # "", "not", "exists", "accumulate", "collect"
        self.agg = agg
        self.fact_var: Optional[str] = None       # $pN when needed
        self.bindings: dict[str, str] = {}        # field -> $bN_j
        self.acc_result_var: Optional[str] = None

    def __getattr__(self, name: str) -> BoundField:
        fields = self.cls.__seine_fields__
        if name in fields:
            if self.ce in ("not", "exists"):
                raise CompileError(
                    f"bindings inside {self.ce}() patterns do not exist in Drools "
                    "scope — match the fact with when() if you need its fields"
                )
            if self.ce in ("accumulate", "collect", "groupby"):
                raise CompileError(
                    "fields of an accumulate/collect SOURCE are scoped inside the "
                    "aggregate; use the aggregate's result instead"
                )
            return BoundField(self, name, fields[name])
        raise AttributeError(name)


# valid parameter counts per Allen operator (oracle-pinned, D-119):
# after/before REQUIRE [lo,hi]; endpoint ops take a tolerance/bounds list
_ALLEN_ARITY = {
    "after": (2,), "before": (2,),
    "coincides": (0, 1, 2),
    "meets": (0, 1), "metby": (0, 1),
    "starts": (0, 1), "startedby": (0, 1),
    "finishes": (0, 1), "finishedby": (0, 1),
    "overlaps": (0, 1, 2), "overlappedby": (0, 1, 2),
    "during": (0, 1, 2, 4), "includes": (0, 1, 2, 4),
}


class _Temporal:
    """`this <op>[params] $anchor` — the certified temporal join family
    (after/before plus the Allen interval set). The anchor is a MATCHED
    event pattern from an earlier when()."""

    __bool__ = _ambiguous_bool("temporal constraint")

    def __init__(self, op, anchor, params):
        if not isinstance(anchor, _Pattern) or anchor.ce != "":
            raise CompileError(
                f"this_{op}: the anchor is a positive when() match of an "
                "event type"
            )
        if getattr(anchor.cls, "__seine_event__", None) is None:
            raise CompileError(
                f"this_{op}: anchor {anchor.type_name} is not an event type "
                "(declare @fact(event=seine_rs.Event(...)))"
            )
        arities = _ALLEN_ARITY[op]
        if len(params) not in arities:
            forms = "/".join(str(a) for a in arities)
            raise CompileError(
                f"this_{op}: takes {forms} duration parameter(s), got {len(params)}"
            )
        for i, v in enumerate(params):
            if not isinstance(v, int) or isinstance(v, bool) or v < 0:
                raise CompileError(
                    f"this_{op}: parameter {i} must be a non-negative int (ms)"
                )
        if op in ("after", "before"):
            if params[1] < params[0]:
                raise CompileError(f"this_{op}: hi_ms < lo_ms")
            self.lo_ms, self.hi_ms = params
        self.op = op
        self.anchor = anchor
        self.params = tuple(params)

    def render(self, rule):
        var = rule._fact_var_for(self.anchor)
        if not self.params:
            return f"this {self.op} {var}"
        inner = ",".join(f"{p}ms" for p in self.params)
        return f"this {self.op}[{inner}] {var}"


def this_after(anchor, lo_ms, hi_ms):
    """Constraint: this event's timestamp is in [lo_ms, hi_ms] AFTER the
    anchor match's. Use inside when(EventType, ...)."""
    return _Temporal("after", anchor, (lo_ms, hi_ms))


def this_before(anchor, lo_ms, hi_ms):
    return _Temporal("before", anchor, (lo_ms, hi_ms))


def this_coincides(anchor, *devs):
    """Allen `coincides`: both endpoints match within tolerance —
    bare (exact), [dev] (shared), or [start_dev, end_dev]."""
    return _Temporal("coincides", anchor, devs)


def this_meets(anchor, *tol):
    """Allen `meets`: this ends where the anchor starts (± tolerance)."""
    return _Temporal("meets", anchor, tol)


def this_metby(anchor, *tol):
    return _Temporal("metby", anchor, tol)


def this_overlaps(anchor, *bounds):
    """Allen `overlaps`: this starts first and the intervals overlap —
    bare, [max], or [min, max] on the overlap distance."""
    return _Temporal("overlaps", anchor, bounds)


def this_overlappedby(anchor, *bounds):
    return _Temporal("overlappedby", anchor, bounds)


def this_during(anchor, *bounds):
    """Allen `during`: this lies strictly inside the anchor interval —
    bare, [max], [min, max], or [lo1, hi1, lo2, hi2] on the start/end
    distances."""
    return _Temporal("during", anchor, bounds)


def this_includes(anchor, *bounds):
    return _Temporal("includes", anchor, bounds)


def this_starts(anchor, *tol):
    """Allen `starts`: same start (± tolerance), this ends first."""
    return _Temporal("starts", anchor, tol)


def this_startedby(anchor, *tol):
    return _Temporal("startedby", anchor, tol)


def this_finishes(anchor, *tol):
    """Allen `finishes`: same end (± tolerance), this starts later."""
    return _Temporal("finishes", anchor, tol)


def this_finishedby(anchor, *tol):
    return _Temporal("finishedby", anchor, tol)


class _RhsAction:
    def __init__(self, kind: str, **kw):
        self.kind = kind
        self.kw = kw


class Rule:
    """Builder for one rule. Patterns declare in order; `when` returns a
    pattern object whose attributes are usable in later constraints and
    in the RHS. `to_drl()` shows exactly what the engine will run."""

    def __init__(self, name: str, salience: Union[int, BoundField, SalExpr, None] = None,
                 no_loop: bool = False, agenda_group: "str | None" = None):
        if not name or any(c in name for c in '"\n'):
            raise CompileError(f"bad rule name {name!r}")
        _reject_callable(salience, "salience")
        if isinstance(salience, AccResult):
            raise CompileError(
                "accumulate results in salience expressions are not certified "
                "against the oracle"
            )
        if salience is not None and not isinstance(salience, (int, BoundField, SalExpr)):
            raise CompileError(
                "salience must be an int, a numeric bound field, or a single "
                "`term op term` expression over bindings — Python callables "
                "cannot run in the match loop"
            )
        if agenda_group is not None and (
            not isinstance(agenda_group, str) or not agenda_group
            or any(c in agenda_group for c in '"\n')
        ):
            raise CompileError(f"bad agenda_group {agenda_group!r}")
        self.name = name
        self.salience = salience
        self.no_loop = no_loop
        self.agenda_group = agenda_group
        self.patterns: list[_Pattern] = []
        self.actions: list[_RhsAction] = []
        self.or_groups: list = []
        self._bind_seq = 0

    def set_salience(self, salience: Union[int, BoundField, SalExpr]) -> "Rule":
        """Set salience after patterns exist (needed when the expression
        references a matched field)."""
        _reject_callable(salience, "salience")
        if isinstance(salience, AccResult):
            raise CompileError(
                "accumulate results in salience expressions are not certified "
                "against the oracle"
            )
        if not isinstance(salience, (int, BoundField, SalExpr)):
            raise CompileError(
                "salience must be an int, a numeric bound field, or a single "
                "`term op term` expression over bindings"
            )
        self.salience = salience
        return self

    # -- LHS ------------------------------------------------------------
    def _add_pattern(self, cls, constraints, ce, agg=None) -> _Pattern:
        if not hasattr(cls, "__seine_fields__"):
            raise CompileError(f"{cls!r} is not a @seine_rs.fact class")
        for c in constraints:
            _reject_callable(c, f"{cls.__name__} constraint")
            if isinstance(c, _Temporal):
                if getattr(cls, "__seine_event__", None) is None:
                    raise CompileError(
                        f"{cls.__name__}: temporal constraints need an event "
                        "type - declare @fact(event=seine_rs.Event(...))"
                    )
                # The declared-lifetime-vs-window consistency lint: the
                # EARLIER event of a temporal join must live until the
                # window's upper bound, or matches past its expiry are
                # silently impossible. Expiration inference stays outside
                # the certified subset — this only cross-checks the
                # user's own explicit declarations, per constraint (no
                # transitive/STP reasoning).
                if c.op not in ("after", "before"):
                    continue  # Allen interval ops: no [lo,hi] window to check
                if c.op == "after":
                    early_cls = c.anchor.cls  # this AFTER anchor: anchor is earlier
                else:
                    early_cls = cls           # this BEFORE anchor: this is earlier
                ev = getattr(early_cls, "__seine_event__", None)
                if ev is not None and ev[1] is not None and ev[1] < c.hi_ms:
                    expires = ev[1]
                    where = (
                        f"{early_cls.__name__} declares expires_ms={expires} but is the "
                        f"earlier event of a this_{c.op}[{c.lo_ms}, {c.hi_ms}] window "
                        f"in rule {self.name!r}"
                    )
                    # strictly below lo: the event is alive AT its
                    # deadline instant (pr_cep_expwin_atlo — delta ==
                    # lo == expires fires), so expires == lo is tier-2
                    # truncation to that single instant, not never-match
                    if expires < c.lo_ms:
                        raise CompileError(
                            f"{where} — it always expires before the window opens, so "
                            f"this constraint can never match. Raise expires_ms to at "
                            f"least {c.hi_ms} or narrow the window."
                        )
                    raise CompileError(
                        f"{where} — partners arriving after {expires}ms can never "
                        f"match, silently truncating the declared window. Raise "
                        f"expires_ms to at least {c.hi_ms} or narrow the window."
                    )
                continue
            if isinstance(c, _Group):
                owners = c.owners()
                if owners != {cls}:
                    other = ", ".join(sorted(o.__name__ for o in owners if o is not cls))
                    raise CompileError(
                        f"boolean groups combine constraints of ONE pattern - "
                        f"this {cls.__name__} group also references {other} "
                        "(inline groups cannot join across patterns)"
                    )
                continue
            if not isinstance(c, _Constraint):
                raise CompileError(
                    f"{cls.__name__}: constraints are field expressions "
                    f"(e.g. {cls.__name__}.<field> >= 10), got {type(c).__name__}"
                )
            if c.field.owner is not cls:
                raise CompileError(
                    f"constraint on {c.field.owner.__name__}.{c.field.name} does not "
                    f"belong in a {cls.__name__} pattern"
                )
        p = _Pattern(self, len(self.patterns), cls, constraints, ce, agg)
        self.patterns.append(p)
        return p

    def when(self, cls: type, *constraints) -> _Pattern:
        """Positive pattern; returns the match for later use."""
        return self._add_pattern(cls, constraints, "")

    def when_any(self, *branches) -> None:
        """OR across patterns: `( A(...) or B(...) )` — each branch is a
        tuple (Class, *constraints). The certified DNF expansion (each
        branch becomes a subrule). v1 keeps branches ALPHA-ONLY: same-
        class literal constraints, no bindings out (a binding would not
        exist in the subrules built from the other branches)."""
        if len(branches) < 2:
            raise CompileError("when_any needs at least two branches")
        parts = []
        for b in branches:
            if not isinstance(b, tuple) or not b or not hasattr(b[0], "__seine_fields__"):
                raise CompileError(
                    "when_any branches are tuples: (FactClass, *constraints)"
                )
            cls, cs = b[0], b[1:]
            rendered = []
            for c in cs:
                _reject_callable(c, f"{cls.__name__} or-branch constraint")
                if isinstance(c, _Temporal):
                    raise CompileError(
                        "temporal constraints inside when_any branches are "
                        "not certified — lift the temporal join to its own "
                        "pattern"
                    )
                if not isinstance(c, (_Constraint, _Group)):
                    raise CompileError(
                        f"when_any branch constraints must be field "
                        f"constraints (got {type(c).__name__})"
                    )
                owners = c.owners() if isinstance(c, _Group) else {c.field.owner}
                if owners != {cls}:
                    raise CompileError(
                        f"when_any branches are alpha-only: constraints must "
                        f"be on {cls.__name__} itself (cross-pattern joins "
                        "into or-branches are not certified)"
                    )
                if isinstance(c, _Constraint) and isinstance(c.rhs, BoundField):
                    raise CompileError(
                        "when_any branches are alpha-only: no bindings from "
                        "other patterns (a join into one branch has no "
                        "meaning in the subrules built from the others)"
                    )
                rendered.append(c)
            parts.append((cls, rendered))
        self.or_groups.append(parts)

    def when_not(self, cls: type, *constraints) -> None:
        self._add_pattern(cls, constraints, "not")

    def when_exists(self, cls: type, *constraints) -> None:
        self._add_pattern(cls, constraints, "exists")

    def accumulate(
        self, cls: type, *constraints, agg: _Agg, window: "_Window | None" = None
    ) -> AccResult:
        """Inline accumulate over a source pattern. Join constraints
        against earlier patterns are allowed (no subnetwork is built for
        inline accumulates)."""
        if not isinstance(agg, _Agg):
            raise CompileError(
                "agg must be one of seine_rs.sum_/count/average/min_/max_ — custom "
                "accumulate functions are outside the certified subset"
            )
        if agg.arg is not None:
            if agg.arg.owner is not cls:
                raise CompileError(
                    f"aggregate argument {agg.arg!r} must be a field of {cls.__name__}"
                )
            if agg.arg.subset_type not in ("i64", "f64") and agg.func not in (
                "count", "collectList", "collectSet"
            ):
                raise CompileError(
                    f"{agg.func}() requires a numeric field, "
                    f"{agg.arg.name} is {agg.arg.subset_type}"
                )
        if window is not None:
            if not isinstance(window, _Window):
                raise CompileError(
                    "accumulate(window=...): pass seine_rs.window_time(ms) or "
                    "seine_rs.window_length(n)"
                )
            if agg.func in ("collectList", "collectSet"):
                raise CompileError(
                    f"{agg.func}() with a window is not certified against the "
                    "oracle — use an unwindowed collect, or a windowed scalar "
                    "aggregate (count/sum/average/min/max)"
                )
            if window.kind == "time" and getattr(cls, "__seine_event__", None) is None:
                raise CompileError(
                    f"window_time over {cls.__name__}: time windows need event "
                    "timestamps — declare @fact(event=seine_rs.Event(...)) on the "
                    "source type"
                )
        p = self._add_pattern(cls, constraints, "accumulate", agg)
        p.window = window
        arg_bf = BoundField(p, agg.arg.name, agg.arg.subset_type) if agg.arg is not None else None
        return AccResult(p, agg.func, arg_bf)

    def group_by(
        self, cls: type, *constraints, key: FieldRef, agg: _Agg
    ) -> AccResult:
        """`groupby( T(..., $k : key, $s : arg); $k; $a : func($s) )` —
        the leading-position key form (the certified slice): the rule
        fires once per group with the aggregate bound. The RESULT is
        usable downstream (pr_ga_downstream); the KEY BINDING is not —
        Drools rejects it in the RHS ("$k cannot be resolved"), so the
        authoring layer never exposes it."""
        if not isinstance(key, FieldRef) or key.owner is not cls:
            raise CompileError(
                f"group_by key must be a field of {cls.__name__}"
            )
        if agg.func in ("collectList", "collectSet"):
            raise CompileError(
                "group_by with collect functions is not certified against "
                "the oracle — use a scalar aggregate"
            )
        if agg.arg is not None:
            if agg.arg.owner is not cls:
                raise CompileError(
                    f"aggregate argument {agg.arg!r} must be a field of {cls.__name__}"
                )
            if agg.arg.subset_type not in ("i64", "f64") and agg.func != "count":
                raise CompileError(
                    f"{agg.func}() requires a numeric field, "
                    f"{agg.arg.name} is {agg.arg.subset_type}"
                )
        p = self._add_pattern(cls, constraints, "groupby", agg)
        p.group_key = key
        arg_bf = BoundField(p, agg.arg.name, agg.arg.subset_type) if agg.arg is not None else None
        return AccResult(p, agg.func, arg_bf)

    def collect(self, cls: type, *constraints) -> None:
        """`List() from collect(...)`. The source must be ALPHA-only:
        a collect source referencing other patterns builds an RIA
        subnetwork, which is outside the certified subset."""
        for c in constraints:
            if isinstance(c, _Constraint) and isinstance(c.rhs, BoundField):
                raise CompileError(
                    "collect sources cannot reference other patterns (that builds "
                    "an RIA subnetwork, outside the certified subset); "
                    "use accumulate() for joined aggregation"
                )
        self._add_pattern(cls, constraints, "collect")

    # -- RHS ------------------------------------------------------------
    def then_insert(self, cls: type, **field_values) -> "Rule":
        if not hasattr(cls, "__seine_fields__"):
            raise CompileError(f"{cls!r} is not a @seine_rs.fact class")
        fields = cls.__seine_fields__
        missing = set(fields) - set(field_values)
        extra = set(field_values) - set(fields)
        if missing or extra:
            raise CompileError(
                f"insert {cls.__name__}: missing={sorted(missing)} extra={sorted(extra)} "
                "(all declared fields, no others)"
            )
        for k, v in field_values.items():
            _reject_callable(v, f"insert {cls.__name__}.{k}")
            if isinstance(v, AccResult):
                v._guard_opaque("an insert argument")
        self.actions.append(_RhsAction("insert", cls=cls, values=field_values))
        return self

    def then_set_focus(self, group: str) -> "Rule":
        """drools.setFocus(group): push the agenda group onto
        the focus stack. The group must be some rule's agenda_group -
        the engine walls undeclared targets at build (Drools NPEs at
        runtime on them)."""
        if not isinstance(group, str) or not group:
            raise CompileError("then_set_focus takes a group name string")
        self.actions.append(_RhsAction("set_focus", group=group))
        return self

    def then_insert_logical(self, cls: type, **field_values) -> "Rule":
        """insertLogical(new Cls(...)): the fact is JUSTIFIED by this
        rule's match (truth maintenance) - it auto-retracts when the match goes
        away. Unit walls apply: insertLogical cannot coexist with ?query
        CEs, and mutating a logically-inserted type is rejected at
        build (the engine names the offending rules)."""
        if not hasattr(cls, "__seine_fields__"):
            raise CompileError(f"{cls!r} is not a @seine_rs.fact class")
        fields = cls.__seine_fields__
        missing = set(fields) - set(field_values)
        extra = set(field_values) - set(fields)
        if missing or extra:
            raise CompileError(
                f"insertLogical {cls.__name__}: missing={sorted(missing)} "
                f"extra={sorted(extra)} (all declared fields, no others)"
            )
        for k, v in field_values.items():
            _reject_callable(v, f"insertLogical {cls.__name__}.{k}")
            if isinstance(v, AccResult):
                v._guard_opaque("an insertLogical argument")
        self.actions.append(_RhsAction("insert_logical", cls=cls, values=field_values))
        return self

    def then_modify(self, pattern: _Pattern, **field_values) -> "Rule":
        if not isinstance(pattern, _Pattern) or pattern.ce != "":
            raise CompileError("then_modify targets a positive when() match")
        for k, v in field_values.items():
            if k not in pattern.cls.__seine_fields__:
                raise CompileError(f"{pattern.type_name} has no field {k}")
            _reject_callable(v, f"modify {pattern.type_name}.{k}")
            if isinstance(v, AccResult):
                v._guard_opaque("a modify argument")
        self.actions.append(_RhsAction("modify", pattern=pattern, values=field_values))
        return self

    def then_delete(self, pattern: _Pattern) -> "Rule":
        if not isinstance(pattern, _Pattern) or pattern.ce != "":
            raise CompileError("then_delete targets a positive when() match")
        self.actions.append(_RhsAction("delete", pattern=pattern))
        return self

    # -- compilation ------------------------------------------------------
    def _binding_for(self, bf: BoundField, use: str) -> str:
        p = bf.pattern
        if isinstance(bf, AccResult):
            bf._guard_opaque(use)
            if p.acc_result_var is None:
                p.acc_result_var = f"$a{p.index}"
            return p.acc_result_var
        if p.rule is not self:
            raise CompileError(f"{bf!r} belongs to a different rule")
        if bf.name not in p.bindings:
            var = f"$b{p.index}_{self._bind_seq}"
            self._bind_seq += 1
            p.bindings[bf.name] = var
        return p.bindings[bf.name]

    def _fact_var_for(self, p: _Pattern) -> str:
        if p.fact_var is None:
            p.fact_var = f"$p{p.index}"
        return p.fact_var

    def _rhs_arg(self, v: Any) -> str:
        if isinstance(v, BoundField):
            return self._binding_for(v, "an RHS argument")
        if isinstance(v, SalExpr):
            raise CompileError(
                "RHS arithmetic is outside the certified subset: an action "
                "value must be a bound field or a literal — `field op term` "
                "expressions are certified only in set_salience. To keep a "
                "running value, accumulate it on the LHS (acc_sum/acc_count) "
                "and insert the result, or read the fact and update() the "
                "new value from Python between fires."
            )
        return _lit(v)

    def to_drl(self) -> str:
        # RHS first: it may demand bindings/fact vars on patterns
        rhs_lines: list[str] = []
        for a in self.actions:
            if a.kind == "insert":
                cls = a.kw["cls"]
                args = ", ".join(
                    self._rhs_arg(a.kw["values"][f]) for f in cls.__seine_fields__
                )
                rhs_lines.append(f"    insert(new {cls.__name__}({args}));")
            elif a.kind == "set_focus":
                rhs_lines.append(f"    drools.setFocus(\"{a.kw['group']}\");")
            elif a.kind == "insert_logical":
                cls = a.kw["cls"]
                args = ", ".join(
                    self._rhs_arg(a.kw["values"][f]) for f in cls.__seine_fields__
                )
                rhs_lines.append(f"    insertLogical(new {cls.__name__}({args}));")
            elif a.kind == "modify":
                p = a.kw["pattern"]
                var = self._fact_var_for(p)
                setters = ", ".join(
                    f"set{f[0].upper()}{f[1:]}({self._rhs_arg(v)})"
                    for f, v in a.kw["values"].items()
                )
                rhs_lines.append(f"    modify({var}) {{ {setters} }}")
            elif a.kind == "delete":
                var = self._fact_var_for(a.kw["pattern"])
                rhs_lines.append(f"    delete({var});")

        # salience needs its bindings too
        sal_attr = ""
        if isinstance(self.salience, int) and not isinstance(self.salience, bool):
            if self.salience != 0:
                sal_attr = f"salience {self.salience}\n"
        elif isinstance(self.salience, BoundField):
            v = self._binding_for(self.salience, "salience")
            sal_attr = f"salience({v})\n"
        elif isinstance(self.salience, SalExpr):
            e = self.salience

            def term(t):
                return str(t) if isinstance(t, int) else self._binding_for(t, "salience")

            sal_attr = f"salience({term(e.a)} {e.op} {term(e.b)})\n"

        # pre-pass: join constraints demand bindings on EARLIER patterns;
        # collect them all before rendering so declarations land in the
        # patterns that own them
        for p in self.patterns:
            for c in p.constraints:
                if isinstance(c, _Constraint) and isinstance(c.rhs, BoundField):
                    self._binding_for(c.rhs, "a join constraint")

        lhs_lines: list[str] = []
        # temporal anchors demand their fact vars BEFORE any LHS line
        # renders (the anchor pattern precedes the temporal one)
        for p in self.patterns:
            for c in p.constraints:
                if isinstance(c, _Temporal):
                    self._fact_var_for(c.anchor)
        for p in self.patterns:
            body = [c.render(self) for c in p.constraints]
            # field bindings demanded by later constraints / RHS / salience
            for fname, var in p.bindings.items():
                body.append(f"{var} : {fname}")
            inner = f"{p.type_name}({', '.join(body)})"
            if p.ce == "not":
                lhs_lines.append(f"    not {inner}")
            elif p.ce == "exists":
                lhs_lines.append(f"    exists {inner}")
            elif p.ce == "accumulate":
                rv = p.acc_result_var or f"$a{p.index}"
                agg = p.agg
                if agg.arg is not None:
                    avar = f"$s{p.index}"
                    body2 = body + [f"{avar} : {agg.arg.name}"]
                    inner = f"{p.type_name}({', '.join(body2)})"
                    call = f"{agg.func}({avar})"
                else:
                    call = f"{agg.func}()"
                win = getattr(p, "window", None)
                over = f" {win.render()}" if win is not None else ""
                lhs_lines.append(f"    accumulate( {inner}{over}; {rv} : {call} )")
            elif p.ce == "groupby":
                rv = p.acc_result_var or f"$a{p.index}"
                agg = p.agg
                kvar = f"$k{p.index}"
                body2 = list(body) + [f"{kvar} : {p.group_key.name}"]
                if agg.arg is not None:
                    avar = f"$s{p.index}"
                    body2.append(f"{avar} : {agg.arg.name}")
                    call = f"{agg.func}({avar})"
                else:
                    call = f"{agg.func}()"
                inner = f"{p.type_name}({', '.join(body2)})"
                lhs_lines.append(f"    groupby( {inner}; {kvar}; {rv} : {call} )")
            elif p.ce == "collect":
                lhs_lines.append(f"    $l{p.index} : ArrayList() from collect( {inner} )")
            else:
                head = f"{p.fact_var} : " if p.fact_var else ""
                lhs_lines.append(f"    {head}{inner}")

        for group in self.or_groups:
            branches = []
            for cls, cs in group:
                inner = ", ".join(c.render(self) for c in cs)
                branches.append(f"{cls.__name__}({inner})")
            lhs_lines.append("    ( " + " or ".join(branches) + " )")

        if not lhs_lines:
            raise CompileError(f"rule {self.name}: no patterns")
        nl = "no-loop\n" if self.no_loop else ""
        ag = f'agenda-group "{self.agenda_group}"\n' if self.agenda_group else ""
        return (
            f'rule "{self.name}"\n{sal_attr}{nl}{ag}when\n'
            + "\n".join(lhs_lines)
            + "\nthen\n"
            + ("\n".join(rhs_lines) + "\n" if rhs_lines else "")
            + "end\n"
        )


def _lint_unstratified_negation(rules: "list[Rule]") -> None:
    """Negation-as-failure over a type the SAME STRATUM is still
    producing: `when_not(T)` asks "has nothing inserted T *yet*", and
    when another rule at the same agenda_group and salience inserts T,
    "yet" depends on declaration order — the negating rule's outcome
    silently flips with the order rules were appended to a list.

    Local and static by design (the altitude that kept the temporal
    lint out of STP territory): per-rule, type-level, no chain or
    fixpoint reasoning, and only the user's own declarations are read.
    A rule whose consequences are ALL insertLogical is exempt — truth
    maintenance retracts its products when the negation is falsified
    later, so its finals are order-invariant (the firing trace may
    still vary). The message leads with that as the first remedy,
    because the underlying question is a modeling one: a conclusion a
    later fact should invalidate is a derived view (insertLogical); a
    conclusion that must survive is a record, and records need strata
    or a second pass. Dynamic (bound-field) salience is statically
    unknowable, so those pairs stay silent. Self-negation (a rule
    negating a type it itself inserts) is the fire-once idiom, not a
    race. The engine stays Drools-faithful — raw DRL keeps the
    footgun; this only stops the authoring layer from silently
    accepting rule sets whose answer depends on list order.

    CAVEAT — this lint is a SAMPLER of the underlying modeling
    error, not a detector of it. The error is asserting a
    defeasible conclusion with a stated (monotonic) insert: any
    rule whose LHS quantifies over the absence or aggregate of
    working memory (not, exists, accumulate, forall) concludes
    from the WHOLE current state, and a state that can still
    change can defeat it. Negation-as-failure is merely where that
    error leaks first, because ordering makes it observable — an
    accumulate-derived stated fact with no `not` anywhere in the
    rule set is just as wrong and stays perfectly silent here.
    (The oracle's own stale-min/max defect, fixed upstream via the
    D-093 report, was this class in production: an extremum is an
    aggregate, the stale value a permanence claim that failed to
    retract when its premise moved.) Passing this lint means the
    modeling error did not leak through a same-stratum negation
    this time; it does not mean the model is sound."""
    def stratum(r: Rule):
        s = r.salience if r.salience is not None else 0
        return (r.agenda_group, s) if isinstance(s, int) else None
    inserters: dict = {}  # (stratum, fact class) -> [rule names]
    for r in rules:
        st = stratum(r)
        if st is None:
            continue
        for a in r.actions:
            if a.kind in ("insert", "insert_logical"):
                inserters.setdefault((st, a.kw["cls"]), []).append(r.name)
    for r in rules:
        st = stratum(r)
        if st is None:
            continue
        if r.actions and all(a.kind == "insert_logical" for a in r.actions):
            continue  # TMS self-corrects this rule's finals
        for p in r.patterns:
            if p.ce != "not":
                continue
            offenders = [n for n in inserters.get((st, p.cls), []) if n != r.name]
            if offenders:
                names = ", ".join(f'"{n}"' for n in sorted(set(offenders)))
                where = (
                    f"agenda_group {st[0]!r}" if st[0] is not None else "the default agenda group"
                ) + f" at salience {st[1]}"
                raise CompileError(
                    f'rule "{r.name}" negates {p.cls.__name__}, but rule {names} '
                    f"inserts {p.cls.__name__} in {where} — the negation may be "
                    f"evaluated before that insert, so this rule's outcome depends "
                    f"on the order rules were declared. First decide what this "
                    f"rule's conclusion is: if it is a derived view that a "
                    f"later {p.cls.__name__} should invalidate, use "
                    f"then_insert_logical — truth maintenance retracts it when "
                    f"the negation is falsified, so finals are order-invariant "
                    f"(and the firings table still records that it was "
                    f"considered). If it is a durable record, separate the "
                    f"strata: give the inserting rule higher salience or its "
                    f"own agenda_group, or compute {p.cls.__name__} in a "
                    f"separate session pass and feed it back as input facts."
                )


def _eval_constraint_on(c, values: dict, pattern) -> Optional[bool]:
    """Three-valued static evaluation of one pattern constraint against
    a literal insert record: True = the inserted record satisfies it,
    False = it cannot, None = statically unknown. Used by the logical-
    cycle lint's self-loop boundary — only PROVEN-True results ever
    reject, so every unknown errs toward silence."""
    if isinstance(c, _Group):
        vals = [_eval_constraint_on(ch, values, pattern) for ch in c.children]
        if c.op == "!":
            return None if vals[0] is None else (not vals[0])
        if c.op == "&&":
            if any(v is False for v in vals):
                return False
            return True if all(v is True for v in vals) else None
        if any(v is True for v in vals):  # "||"
            return True
        return False if all(v is False for v in vals) else None
    if not isinstance(c, _Constraint):
        return None
    if not isinstance(c.field, FieldRef) or c.field.owner is not pattern.cls:
        return None
    v = values.get(c.field.name)
    if isinstance(v, BoundField):
        # copied verbatim from the matched fact's SAME field: satisfies
        # whatever that field satisfied at match time, by construction
        if getattr(v, "pattern", None) is pattern and v.name == c.field.name:
            return True
        return None
    if isinstance(v, (FieldRef, SalExpr)):
        return None
    if isinstance(c.rhs, _Null):
        return (v is None) if c.op == "==" else (v is not None)
    if v is None:
        return None  # NULL vs literal: 3VL UNKNOWN never admits, but
        # the fact could re-enter via another rule — stay silent
    rhs = c.rhs
    if c.op in ("in", "not in"):
        if not all(isinstance(x, (int, float, str, bool)) for x in rhs):
            return None
        hit = any(type(x) is type(v) and x == v for x in rhs)
        return hit if c.op == "in" else not hit
    if c.op == "contains":
        return rhs in v if isinstance(v, str) else None
    if c.op == "matches":
        return None  # Java regex semantics: stay out of the guessing game
    if isinstance(rhs, (BoundField, FieldRef, SalExpr)):
        return None
    # scalar comparison; bool is compared only against bool
    if isinstance(v, bool) or isinstance(rhs, bool):
        if not (isinstance(v, bool) and isinstance(rhs, bool)):
            return None
        return (v == rhs) if c.op == "==" else (v != rhs) if c.op == "!=" else None
    if isinstance(v, (int, float, _pydecimal.Decimal)) and isinstance(
        rhs, (int, float, _pydecimal.Decimal)
    ):
        pass  # numeric cross-compare is well-defined
    elif type(v) is not type(rhs):
        return None
    try:
        return {
            "==": v == rhs, "!=": v != rhs,
            "<": v < rhs, "<=": v <= rhs,
            ">": v > rhs, ">=": v >= rhs,
        }.get(c.op)
    except TypeError:
        return None


def _lint_logical_cycles(rules) -> None:
    """The logical-derivation cycle lint: a cycle of insertLogical
    justifications across DISTINCT types (M1 -> M2 -> ... -> M1) is
    self-supporting — truth maintenance is justification-set based (it
    asks only whether a key's justifier set is empty, never whether the
    support is grounded), so once the grounded root is deleted the
    members keep each other alive PERMANENTLY, and an all-logical loop
    has no external handle left to break it. Certified against the
    oracle: Drools orphans such cycles identically.

    Same altitude as the unstratified-negation lint: type-level, no
    constraint or fixpoint reasoning. Edges come only from patterns
    whose match genuinely supports the firing — plain `when` and
    `when_exists`; negations support nothing, and accumulate/collect
    fire even over an empty source. The load-bearing exemption is the
    SELF-LOOP (a rule matching T that logically inserts T): constraint-
    guarded bounded escalation over one type is a real, valid pattern,
    and it is exactly where type-level over-approximation would bite.
    The exemption is SATISFIABILITY-BOUNDED, not blanket (external
    review found the blanket form leaking): a self-loop whose inserted
    record PROVABLY re-satisfies the rule's own guard is a one-node
    cycle — it self-justifies at the plateau and orphans exactly like
    the multi-type shapes — so proven-live self-loops reject. Strict
    progress (the insert falls outside the guard) and anything
    statically undecidable stay silent. Like every static lint here,
    this is a sampler, not a detector: a green result means no PROVEN
    logical cycle, not that the derivation is sound."""
    # type-level edges: matched class -> logically-inserted class
    edges: dict = {}  # cls -> list[(dst cls, rule name)]
    for r in rules:
        outs = [a.kw["cls"] for a in r.actions if a.kind == "insert_logical"]
        if not outs:
            continue
        for p in r.patterns:
            if p.ce not in ("", "exists"):
                continue
            for o in outs:
                if o is p.cls:
                    # the self-loop boundary: reject ONLY when the
                    # inserted record provably re-satisfies every
                    # constraint on this pattern (no guard counts as
                    # trivially satisfied) — then it self-justifies
                    # and orphans; otherwise silent
                    vals = next(
                        a.kw["values"] for a in r.actions
                        if a.kind == "insert_logical" and a.kw["cls"] is o
                    )
                    verdicts = [
                        _eval_constraint_on(c, vals, p)
                        for c in p.constraints
                        if not isinstance(c, _Temporal)
                    ]
                    if all(v is True for v in verdicts):
                        raise CompileError(
                            f'rule "{r.name}": self-justifying logical '
                            f"loop on {p.cls.__name__} — the record it "
                            f"insertLogical's satisfies the rule's own "
                            f"matching constraints"
                            + (" (there are none)" if not verdicts else "")
                            + ", so the fact re-justifies itself: once "
                            f"the grounded support is deleted, truth "
                            f"maintenance never retracts it (justifier "
                            f"sets are counted, not grounded) and it "
                            f"survives as permanently unreclaimable "
                            f"working memory. Make the escalation "
                            f"strict — insert a value the guard "
                            f"excludes — or derive from the grounded "
                            f"tier, or compute the closure outside the "
                            f"session and insert stated facts."
                        )
                    continue  # strict-progress or undecidable: exempt
                edges.setdefault(p.cls, []).append((o, r.name))

    # iterative DFS, deterministic order; report the first cycle found
    state: dict = {}  # cls -> 1 visiting / 2 done
    def walk(start):
        stack = [(start, iter(edges.get(start, ())))]
        path = [(start, None)]  # (cls, rule that led here)
        state[start] = 1
        while stack:
            cls, it = stack[-1]
            step = next(it, None)
            if step is None:
                state[cls] = 2
                stack.pop()
                path.pop()
                continue
            nxt, rname = step
            if state.get(nxt) == 1:
                cyc = path[[c for c, _ in path].index(nxt):] + [(nxt, rname)]
                arcs = " -> ".join(
                    f"{c.__name__}"
                    + (f' (rule "{rn}")' if rn and i > 0 else "")
                    for i, (c, rn) in enumerate(cyc)
                )
                raise CompileError(
                    f"logical derivation cycle: {arcs} — every link is an "
                    f"insertLogical justification, so the cycle supports "
                    f"itself: delete the grounded facts and truth "
                    f"maintenance never retracts it (justification sets are "
                    f"counted, not grounded), leaving the members alive as "
                    f"permanently unreclaimable working memory with no "
                    f"handle to break the loop. Keep logical derivation "
                    f"DAG-shaped: derive each fact from the grounded tier "
                    f"directly, or compute the mutually-recursive closure "
                    f"outside the session and insert the results as stated "
                    f"facts."
                )
            if state.get(nxt) is None:
                state[nxt] = 1
                path.append((nxt, rname))
                stack.append((nxt, iter(edges.get(nxt, ()))))
    for node in list(edges):
        if state.get(node) is None:
            walk(node)


def _lint_self_feeding_modify(rules) -> None:
    """The self-feeding modify check: a modify() re-evaluates every
    pattern on the modified class that LISTENS to a written field —
    listened means constrained OR bound (a field is bound the moment
    its BoundField is used anywhere in the rule). If the written
    values provably leave the rule's own match intact, the rule
    re-triggers itself and loops to the fire limit — oracle and engine
    identically, because the shape is certified-runnable; that is
    exactly why the authoring layer refuses to write it silently, and
    the check fires the same on literal and computed values (the
    hazard is the mask, not the arithmetic).

    Same altitude as the neighbors above: per-rule, per-action, local.
    Only the modify's own target pattern is judged — no cross-rule or
    cross-instance reasoning (cross-rule ping-pong is fire-limit-
    governed on both engines by design; a cycle check here would gate
    what the certified subset runs). Exemptions, each a certified
    idiom:
    - the FALSIFYING WRITE — the guard-flip that terminates the match
      (write true under `g == False`, write 90.0 under `total < 90.0`):
      proven-False post-write means the rule exits its own match;
    - no_loop=True — the engine suppresses the rule's own
      re-activation, so an unguarded self-write fires once per match;
    - anything statically undecidable stays silent (three-valued eval;
      only PROVEN outcomes act). Temporal constraints are skipped
      (under-listening errs toward silence).
    Like the neighbors, a sampler, not a detector: green means no
    PROVEN self-feed, not that firing is bounded."""
    def cfields(c, cls, out):
        if isinstance(c, _Group):
            for ch in c.children:
                cfields(ch, cls, out)
        elif isinstance(c, _Constraint) and isinstance(c.field, FieldRef) \
                and c.field.owner is cls:
            out.add(c.field.name)

    for r in rules:
        if r.no_loop:
            continue  # own re-activation suppressed: fires once per match
        # fields of each positive pattern bound BY USE anywhere in the rule
        def note(v, p, out):
            if isinstance(v, BoundField) and getattr(v, "pattern", None) is p:
                out.add(v.name)
            elif isinstance(v, SalExpr):
                note(v.a, p, out)
                note(v.b, p, out)
        def rhs_walk(c, p, out):
            if isinstance(c, _Group):
                for ch in c.children:
                    rhs_walk(ch, p, out)
            elif isinstance(c, _Constraint):
                note(c.rhs, p, out)
        for a in r.actions:
            if a.kind != "modify":
                continue
            p = a.kw["pattern"]
            vals = a.kw["values"]
            if any(isinstance(v, SalExpr) for v in vals.values()):
                # Not renderable today — to_drl's arithmetic wall already
                # rejected this rule with the steering message. WHEN
                # AUTHORING COMPUTED ARGS LAND (the boundary-redraw
                # roadmap's authoring-sugar item), remove this skip so
                # computed self-feeds are judged right here.
                continue
            written = set(vals)
            listened = set()
            for c in p.constraints:
                cfields(c, p.cls, listened)
            for act in r.actions:
                for v in act.kw.get("values", {}).values():
                    note(v, p, listened)
            note(r.salience, p, listened)
            for q in r.patterns:
                for c in q.constraints:
                    rhs_walk(c, p, listened)
            if not (written & listened):
                continue  # unlistened write: the modify cannot re-stage its own match
            # still-matching, three-valued: unwritten fields keep their
            # matched values (the same-field BoundField reads as
            # "satisfies what it satisfied"), written fields carry the
            # new values
            probe_vals = {
                f: getattr(p, f)
                for f in p.cls.__seine_fields__ if f not in written
            }
            probe_vals.update(vals)
            verdicts = [
                _eval_constraint_on(c, probe_vals, p)
                for c in p.constraints
                if not isinstance(c, _Temporal)
            ]
            if any(v is False for v in verdicts):
                continue  # falsifying write: the rule exits its own match
            if any(v is None for v in verdicts):
                continue  # undecidable: stay silent
            hot = ", ".join(sorted(written & listened))
            raise CompileError(
                f'rule "{r.name}": self-feeding modify on {p.cls.__name__} '
                f"— it writes {hot}, which its own when() listens to (a "
                f"constrained or bound field), and the written values "
                f"provably leave the rule still matching"
                + (" (there is no constraint to exit through)" if not verdicts else "")
                + ", so it re-triggers itself and loops to the fire limit. "
                f"First decide what one firing means: if the write should "
                f"END the match, write a value the rule's own constraint "
                f"excludes (flip a guard field — the terminating idiom); "
                f"if one firing per match is the intent, set no_loop=True; "
                f"if you are accumulating a running value, that is LHS "
                f"work — acc_sum/acc_count and insert the result, or "
                f"update() from Python between session passes. Raw DRL "
                f"keeps Drools-faithful behavior (both engines run this "
                f"shape to the fire limit)."
            )


def compile_rules(rules) -> str:
    """Render a list of Rule objects into one DRL source string."""
    out = []
    seen = set()
    rules = list(rules)
    for r in rules:
        if not isinstance(r, Rule):
            raise CompileError(f"expected seine_rs.Rule, got {type(r).__name__}")
        if r.name in seen:
            raise CompileError(f"duplicate rule name {r.name!r}")
        seen.add(r.name)
        out.append(r.to_drl())
    _lint_unstratified_negation(rules)
    _lint_logical_cycles(rules)
    _lint_self_feeding_modify(rules)
    return "\n".join(out)
