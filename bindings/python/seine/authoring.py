"""Pythonic rule authoring that COMPILES TO DRL TEXT (D-045).

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
from typing import Any, Optional, Union


class CompileError(Exception):
    """The construct falls outside the certified rule grammar."""


_PY_TO_SUBSET = {int: "i64", float: "f64", str: "String", bool: "bool"}
_SUBSET_ARROW = {"i64": "int64", "f64": "float64", "String": "string", "bool": "bool"}
_CMP = {"eq": "==", "ne": "!=", "lt": "<", "le": "<=", "gt": ">", "ge": ">="}


def _lit(v: Any) -> str:
    """Render a Python literal into DRL constraint syntax."""
    if isinstance(v, bool):
        return "true" if v else "false"
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


# ---------------------------------------------------------------------
# Fact classes and field expressions
# ---------------------------------------------------------------------

class FieldRef:
    """A typed field of a fact CLASS; operator overloads build
    constraint AST nodes, never evaluate."""

    def __init__(self, owner: type, name: str, subset_type: str):
        self.owner = owner
        self.name = name
        self.subset_type = subset_type

    # -- comparisons -> Constraint
    def _cmp(self, op: str, other: Any):
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
        if not isinstance(regex, str):
            raise CompileError(".matches() takes a literal regex string")
        if self.subset_type != "String":
            raise CompileError(f".matches() requires a str field, {self.name} is {self.subset_type}")
        return _Constraint(self, "matches", regex)

    def contains(self, needle: str):
        if not isinstance(needle, str):
            raise CompileError(".contains() takes a literal string")
        if self.subset_type != "String":
            raise CompileError(f".contains() requires a str field, {self.name} is {self.subset_type}")
        return _Constraint(self, "contains", needle)

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


def fact(cls: type) -> type:
    """Declare a fact type from an annotated class:

        @seine.fact
        class Person:
            name: str
            age: int

    Annotations map onto the certified subset (int -> i64, float -> f64,
    str -> String, bool -> bool). The class becomes a dataclass (usable
    for row construction) and its class attributes become FieldRefs for
    rule expressions.
    """
    ann = getattr(cls, "__annotations__", {})
    if not ann:
        raise CompileError(f"@fact {cls.__name__}: no annotated fields")
    fields = {}
    for name, py_t in ann.items():
        if py_t not in _PY_TO_SUBSET:
            raise CompileError(
                f"@fact {cls.__name__}.{name}: type {getattr(py_t, '__name__', py_t)!r} "
                "is outside the certified subset (int, float, str, bool)"
            )
        fields[name] = _PY_TO_SUBSET[py_t]
    dc = dataclasses.dataclass(cls)
    dc.__seine_fields__ = fields  # ordered: annotation order = constructor order
    for name, st in fields.items():
        setattr(dc, name, FieldRef(dc, name, st))
    return dc


class BoundField:
    """A field of a MATCHED pattern (`p.age` where `p = r.when(Person)`),
    usable in later constraints, RHS args, accumulate args and salience
    expressions. Compiles to a `$binding : field` declaration."""

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
    is the accumulate CE. D-039 typing walls apply."""

    def __init__(self, pattern: "_Pattern", func: str, arg: Optional[BoundField]):
        st = {
            "count": "i64",
            "average": "f64",
            "sum": arg.subset_type if arg else "i64",
            "min": arg.subset_type if arg else "i64",
            "max": arg.subset_type if arg else "i64",
        }[func]
        super().__init__(pattern, f"__acc_{func}", st)
        self.func = func
        self.arg = arg
        if func in ("min", "max") and st == "f64":
            self.opaque = True  # D-039: compiles nowhere downstream
        else:
            self.opaque = False

    def _guard_opaque(self, use: str):
        if self.opaque:
            raise CompileError(
                f"{self.func}() over a float field yields an opaque Number in Drools "
                f"(D-039): it cannot be used in {use}. Aggregate an int field, or "
                "keep the result unused."
            )

    def _arith(self, op, other, reflected=False):
        raise CompileError(
            "accumulate results in salience expressions are not certified "
            "against the oracle (D-043); compute the aggregate into a fact "
            "and reference that instead"
        )


# aggregate constructors -------------------------------------------------

class _Agg:
    def __init__(self, func: str, arg: Optional[FieldRef]):
        self.func, self.arg = func, arg


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

class _Constraint:
    def __init__(self, field: FieldRef, op: str, rhs: Any):
        self.field, self.op, self.rhs = field, op, rhs

    def render(self, rule: "Rule") -> str:
        f = self.field.name
        if self.op in ("in", "not in"):
            items = ", ".join(_lit(v) for v in self.rhs)
            return f"{f} {self.op} ({items})"
        if self.op in ("matches", "contains"):
            return f"{f} {self.op} {_lit(self.rhs)}"
        if isinstance(self.rhs, BoundField):
            var = rule._binding_for(self.rhs, use="a join constraint")
            return f"{f} {self.op} {var}"
        return f"{f} {self.op} {_lit(self.rhs)}"


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
            if self.ce in ("accumulate", "collect"):
                raise CompileError(
                    "fields of an accumulate/collect SOURCE are scoped inside the "
                    "aggregate; use the aggregate's result instead"
                )
            return BoundField(self, name, fields[name])
        raise AttributeError(name)


class _RhsAction:
    def __init__(self, kind: str, **kw):
        self.kind = kind
        self.kw = kw


class Rule:
    """Builder for one rule. Patterns declare in order; `when` returns a
    pattern object whose attributes are usable in later constraints and
    in the RHS. `to_drl()` shows exactly what the engine will run."""

    def __init__(self, name: str, salience: Union[int, BoundField, SalExpr, None] = None,
                 no_loop: bool = False):
        if not name or any(c in name for c in '"\n'):
            raise CompileError(f"bad rule name {name!r}")
        _reject_callable(salience, "salience")
        if isinstance(salience, AccResult):
            raise CompileError(
                "accumulate results in salience expressions are not certified "
                "against the oracle (D-043)"
            )
        if salience is not None and not isinstance(salience, (int, BoundField, SalExpr)):
            raise CompileError(
                "salience must be an int, a numeric bound field, or a single "
                "`term op term` expression over bindings — Python callables "
                "cannot run in the match loop"
            )
        self.name = name
        self.salience = salience
        self.no_loop = no_loop
        self.patterns: list[_Pattern] = []
        self.actions: list[_RhsAction] = []
        self._bind_seq = 0

    def set_salience(self, salience: Union[int, BoundField, SalExpr]) -> "Rule":
        """Set salience after patterns exist (needed when the expression
        references a matched field)."""
        _reject_callable(salience, "salience")
        if isinstance(salience, AccResult):
            raise CompileError(
                "accumulate results in salience expressions are not certified "
                "against the oracle (D-043)"
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
            raise CompileError(f"{cls!r} is not a @seine.fact class")
        for c in constraints:
            _reject_callable(c, f"{cls.__name__} constraint")
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

    def when_not(self, cls: type, *constraints) -> None:
        self._add_pattern(cls, constraints, "not")

    def when_exists(self, cls: type, *constraints) -> None:
        self._add_pattern(cls, constraints, "exists")

    def accumulate(self, cls: type, *constraints, agg: _Agg) -> AccResult:
        """Inline accumulate over a source pattern. Join constraints
        against earlier patterns are allowed (no subnetwork is built for
        inline accumulates)."""
        if not isinstance(agg, _Agg):
            raise CompileError(
                "agg must be one of seine.sum_/count/average/min_/max_ — custom "
                "accumulate functions are outside the certified subset"
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
        p = self._add_pattern(cls, constraints, "accumulate", agg)
        arg_bf = BoundField(p, agg.arg.name, agg.arg.subset_type) if agg.arg else None
        return AccResult(p, agg.func, arg_bf)

    def collect(self, cls: type, *constraints) -> None:
        """`List() from collect(...)`. The source must be ALPHA-only:
        a collect source referencing other patterns builds an RIA
        subnetwork, which is outside the certified subset (D-041)."""
        for c in constraints:
            if isinstance(c, _Constraint) and isinstance(c.rhs, BoundField):
                raise CompileError(
                    "collect sources cannot reference other patterns (that builds "
                    "an RIA subnetwork, outside the certified subset — D-041); "
                    "use accumulate() for joined aggregation"
                )
        self._add_pattern(cls, constraints, "collect")

    # -- RHS ------------------------------------------------------------
    def then_insert(self, cls: type, **field_values) -> "Rule":
        if not hasattr(cls, "__seine_fields__"):
            raise CompileError(f"{cls!r} is not a @seine.fact class")
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
                if isinstance(c.rhs, BoundField):
                    self._binding_for(c.rhs, "a join constraint")

        lhs_lines: list[str] = []
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
                lhs_lines.append(f"    accumulate( {inner}; {rv} : {call} )")
            elif p.ce == "collect":
                lhs_lines.append(f"    $l{p.index} : ArrayList() from collect( {inner} )")
            else:
                head = f"{p.fact_var} : " if p.fact_var else ""
                lhs_lines.append(f"    {head}{inner}")

        if not lhs_lines:
            raise CompileError(f"rule {self.name}: no patterns")
        nl = "no-loop\n" if self.no_loop else ""
        return (
            f'rule "{self.name}"\n{sal_attr}{nl}when\n'
            + "\n".join(lhs_lines)
            + "\nthen\n"
            + ("\n".join(rhs_lines) + "\n" if rhs_lines else "")
            + "end\n"
        )


def compile_rules(rules) -> str:
    """Render a list of Rule objects into one DRL source string."""
    out = []
    seen = set()
    for r in rules:
        if not isinstance(r, Rule):
            raise CompileError(f"expected seine.Rule, got {type(r).__name__}")
        if r.name in seen:
            raise CompileError(f"duplicate rule name {r.name!r}")
        seen.add(r.name)
        out.append(r.to_drl())
    return "\n".join(out)
