"""seine_rs.derive — the derivation plane's Rust kernels.

The two-plane contract (docs/derivation-plane.md): Drools
semantics in the match, dataframe semantics in the data. These are
pure columnar functions over Arrow data — anything ``run()`` accepts
(``__arrow_c_stream__`` tables or dicts of column lists) in, a
``seine_rs.Table`` out — producing honest FIELDS upstream of
assertion; the certified match grammar never grows arithmetic. Their
oracle is an independent pure-python reference + the property battery
in bindings/tests/test_derive.py; the Drools oracle has no opinion
about column math.

Epoch contract: derivation runs inside the epoch, upstream of
assertion, as a deterministic function of (raw batch, caller-owned
state) — the WAL stores RAW epochs and replay re-derives identically.

Two surfaces:

**The expression layer** — general row-wise column math, declared by
operator overloading and evaluated in Rust (no Python in the loop):

    from seine_rs.derive import col, with_columns
    orders = with_columns(orders, total=col("price") * col("qty"))

- ``with_columns(data, **named_exprs)`` — append computed columns.
- ``filter(data, pred)`` — keep rows where the predicate is TRUE.
- ``col`` / ``lit`` / ``if_else`` / :class:`Expr` — the closed
  expression grammar (arithmetic, comparisons, SQL three-valued
  boolean logic, conditionals, casts, core string ops). Nulls
  propagate SQL-style; semantics are measured against the data-plane
  oracle in docs/derive-expr-pins.md.

**The bespoke kernel set** (ADS-B-driven; geometry per the design
doc's round-27 hardening):

- ``haversine`` — great-circle distance columns -> Int64 meters.
- ``pair_candidates`` — cross-join + metric-space candidate prune
  over one position table (wrapped lon delta, cos(lat)-scaled
  threshold saturating to latitude-only at the poles).
- ``closing`` — TTL'd decreasing-distance flag; state is the CALLER's
  dict, swept by epoch timestamp on every call.
"""
from seine_rs._native import (
    derive_closing as _closing,
    derive_filter as _filter,
    derive_haversine as _haversine,
    derive_pair_candidates as _pair_candidates,
    derive_with_columns as _with_columns,
)

__all__ = [
    "EARTH_R",
    "Expr",
    "closing",
    "col",
    "filter",
    "haversine",
    "if_else",
    "lit",
    "pair_candidates",
    "with_columns",
]

EARTH_R = 6_371_000.0  # meters — the kernels' sphere radius


def haversine(data, lat1="lat1", lon1="lon1", lat2="lat2", lon2="lon2",
              out="dist_m"):
    """Columnar great-circle distance on the EARTH_R sphere.

    ``data`` needs four numeric columns (named by ``lat1``/``lon1``/
    ``lat2``/``lon2``; degrees; ints widen exactly; NaN/inf raise —
    a NaN would otherwise cast to 0 meters, the strongest possible
    false convergence signal). Returns the input columns plus
    ``out``: Int64 meters, rounded half away from zero —
    bit-compatible with the retired polars stage.
    """
    return _haversine(data, lat1=lat1, lon1=lon1, lat2=lat2, lon2=lon2,
                      out=out)


def pair_candidates(data, id="id", lat="lat", lon="lon", radius_m=25_000.0):
    """Candidate pairs from one position table (``id``/``lat``/``lon``).

    ``a < b`` dedup over the cross join, then a SOUND metric-space
    prune whose contract is COMPLETENESS: no pair whose true haversine
    distance is <= ``radius_m`` is ever dropped. A prune, not the
    exact test — false positives are expected; run :func:`haversine`
    on the output for true distances. With theta = radius_m/EARTH_R:
    |dlat| <= theta; pairs reachable across a pole (colatitude sum <=
    theta) admit regardless of longitude; otherwise the wrapped lon
    delta is bounded by the spherical-cap limit
    asin(sin theta / cos(max |lat|)), skipped entirely when the cap
    reaches a pole. Comparisons are inclusive (a coincident pair
    admits even at radius_m=0).

    Preconditions: ids must be UNIQUE (the ``a < b`` dedup means a
    duplicated id never pairs with itself — such pairs are silently
    absent) and coordinates FINITE (NaN/inf raise; they would
    otherwise decay into false convergence signals downstream).

    Output columns: ``{id}_a, {lat}_a, {lon}_a, {id}_b, {lat}_b,
    {lon}_b, key`` (``"{a}|{b}"``), in cross-join order (a-major).
    """
    return _pair_candidates(data, id=id, lat=lat, lon=lon, radius_m=radius_m)


def closing(state, ts, data, key="key", dist="dist_m", ttl_ms=60_000,
            out="closing"):
    """Stateful decreasing-distance flag keyed by ``key``.

    ``state`` is the CALLER's dict (``key -> (dist, epoch_ts)``) —
    hold it in your driver and rebuild it on WAL replay; nothing hides
    in module globals. Entries older than ``ttl_ms`` are swept FIRST
    on every call (pass the raw epoch timestamp as ``ts``), so call
    once per epoch even when the batch is empty: eviction stays a pure
    function of the raw epoch sequence. Epoch ``ts`` must be
    MONOTONIC: a ``ts`` earlier than any timestamp held in ``state``
    raises (silently computing flags against future-stamped state is
    the alternative; the error replays deterministically). Appends
    ``out`` (bool): true iff the key was seen within the TTL horizon
    at a strictly greater distance. Rows update the state in row
    order. An entry aged exactly ``ttl_ms`` still counts; one epoch
    older is swept.
    """
    return _closing(state, ts, data, key=key, dist=dist, ttl_ms=ttl_ms,
                    out=out)


# ---------------------------------------------------------------------
# The expression layer: closed trees built by operator overloading,
# evaluated in Rust over Arrow columns (docs/derivation-plane.md; the
# measured semantics live in docs/derive-expr-pins.md).
# ---------------------------------------------------------------------

_I64_MIN = -(2**63)
_I64_MAX = 2**63 - 1


def _lift(value, ctx):
    """A raw Python scalar (or Expr) -> tree node. bool checks BEFORE
    int (Python bool is an int subclass)."""
    if isinstance(value, Expr):
        return value._tree, repr(value)
    if value is None:
        raise ValueError(
            f"{ctx}: None has no expression type — use .is_null()/"
            ".is_not_null() to test for nulls and .fill_null(x) to "
            "replace them"
        )
    if isinstance(value, bool):
        return {"op": "lit", "value": value}, repr(value)
    if isinstance(value, int):
        if not _I64_MIN <= value <= _I64_MAX:
            raise ValueError(
                f"{ctx}: int literal {value} does not fit i64 — the "
                "expression subset is 64-bit"
            )
        return {"op": "lit", "value": value}, repr(value)
    if isinstance(value, float):
        if value != value or value in (float("inf"), float("-inf")):
            raise ValueError(
                f"{ctx}: non-finite float literal ({value!r}) — NaN/inf "
                "enter only as column DATA, never as literals; fill or "
                "filter the column instead"
            )
        return {"op": "lit", "value": value}, repr(value)
    if isinstance(value, str):
        return {"op": "lit", "value": value}, repr(value)
    raise TypeError(
        f"{ctx}: unsupported literal type {type(value).__name__} "
        "(supported: bool, int, float, str, Expr)"
    )


class Expr:
    """A row-wise column expression — a closed tree, built by operator
    overloading and evaluated in Rust; it never computes in Python.

    Build leaves with :func:`col` and :func:`lit`; combine with
    ``+ - * / // % **``, comparisons, ``& | ~`` (SQL three-valued
    logic), :func:`if_else`, and the methods below. Feed the result to
    :func:`with_columns` or :func:`filter`.

    Semantics notes (measured against the data-plane oracle; see
    docs/derive-expr-pins.md):

    - NULL propagates: null in -> null out per row; ``&``/``|`` follow
      SQL three-valued logic.
    - ``/`` is true division and always yields f64.
    - ``//`` is INTEGER-ONLY and TRUNCATES toward zero (`-7 // 2 == -3`)
      — this differs from Python's floored ``//``; together with ``%``
      it preserves ``a == (a // b) * b + a % b``.
    - ``%`` takes the SIGN OF THE DIVIDEND (`-7 % 3 == -1`) — this
      differs from Python's floored ``%``.
    - ``.round()`` rounds half AWAY from zero on the value's shortest
      decimal representation — not Python's banker's rounding.
    - Integer overflow, division by zero, sqrt of a negative, and
      failed casts raise; they never produce silent nulls.
    - Floats are IEEE-754: NaN/inf flow as values and ``NaN != NaN``.
    """

    __slots__ = ("_tree", "_repr")

    def __init__(self, tree, repr_):
        self._tree = tree
        self._repr = repr_

    def to_tree(self):
        """The expression as a plain nested dict (the exact structure
        handed to the Rust evaluator) — for tooling and tests."""
        import copy

        return copy.deepcopy(self._tree)

    def __repr__(self):
        return self._repr

    # -- guards ---------------------------------------------------------
    __hash__ = None  # __eq__ builds trees; hashing would be a silent trap

    def __bool__(self):
        raise TypeError(
            "the truth value of an Expr is ambiguous: Python's `and`/`or` "
            "and chained comparisons (10 < x < 100) short-circuit through "
            "bool() and would silently drop an operand. Combine with "
            "`&` / `|` / `~` and write a range as two comparisons "
            "((x > 10) & (x < 100))."
        )

    def __contains__(self, other):
        raise TypeError(
            "`in` cannot build an expression (Python coerces it through "
            "bool()) — use .str_contains(needle) for substring tests"
        )

    def __xor__(self, other):
        raise TypeError("`^` is outside the expression subset — use != for boolean xor")

    __rxor__ = __xor__

    # -- internals ------------------------------------------------------
    def _bin(self, op, sym, other, reflected=False):
        tree, rep = _lift(other, f"Expr {sym}")
        a, b = (tree, self._tree) if reflected else (self._tree, tree)
        ra, rb = (rep, self._repr) if reflected else (self._repr, rep)
        return Expr({"op": op, "args": [a, b]}, f"({ra} {sym} {rb})")

    def _method(self, op, name, other):
        tree, rep = _lift(other, f"Expr.{name}")
        return Expr(
            {"op": op, "args": [self._tree, tree]}, f"{self._repr}.{name}({rep})"
        )

    def _unary(self, op, repr_):
        return Expr({"op": op, "args": [self._tree]}, repr_)

    def _cmp(self, op, sym, other):
        if other is None:
            good = "is_null()" if sym == "==" else "is_not_null()"
            raise TypeError(
                f"Expr {sym} None: null tests are EXPLICIT three-valued "
                f"logic — use .{good}"
            )
        return self._bin(op, sym, other)

    def _kleene(self, op, sym, other, reflected=False):
        if not isinstance(other, (Expr, bool)):
            raise TypeError(
                f"`{sym}` reached a bare {type(other).__name__}: Python's "
                f"`&` and `|` bind TIGHTER than comparisons, so "
                f"`a > 10 {sym} b` parses as `a > (10 {sym} b)`. "
                "Parenthesize each comparison — (col(\"a\") > 10) "
                f"{sym} (col(\"b\") < 100)."
            )
        return self._bin(op, sym, other, reflected)

    # -- arithmetic -----------------------------------------------------
    def __add__(self, other):
        return self._bin("add", "+", other)

    def __radd__(self, other):
        return self._bin("add", "+", other, reflected=True)

    def __sub__(self, other):
        return self._bin("sub", "-", other)

    def __rsub__(self, other):
        return self._bin("sub", "-", other, reflected=True)

    def __mul__(self, other):
        return self._bin("mul", "*", other)

    def __rmul__(self, other):
        return self._bin("mul", "*", other, reflected=True)

    def __truediv__(self, other):
        return self._bin("div", "/", other)

    def __rtruediv__(self, other):
        return self._bin("div", "/", other, reflected=True)

    def __floordiv__(self, other):
        return self._bin("floordiv", "//", other)

    def __rfloordiv__(self, other):
        return self._bin("floordiv", "//", other, reflected=True)

    def __mod__(self, other):
        return self._bin("rem", "%", other)

    def __rmod__(self, other):
        return self._bin("rem", "%", other, reflected=True)

    def __pow__(self, other):
        return self._bin("pow", "**", other)

    def __rpow__(self, other):
        return self._bin("pow", "**", other, reflected=True)

    def __neg__(self):
        return self._unary("neg", f"(-{self._repr})")

    def __pos__(self):
        return self

    def __abs__(self):
        return self.abs()

    def __round__(self, ndigits=None):
        return self.round(0 if ndigits is None else ndigits)

    # -- comparisons ----------------------------------------------------
    def __eq__(self, other):
        return self._cmp("eq", "==", other)

    def __ne__(self, other):
        return self._cmp("neq", "!=", other)

    def __lt__(self, other):
        return self._cmp("lt", "<", other)

    def __le__(self, other):
        return self._cmp("lt_eq", "<=", other)

    def __gt__(self, other):
        return self._cmp("gt", ">", other)

    def __ge__(self, other):
        return self._cmp("gt_eq", ">=", other)

    # -- boolean (Kleene) -----------------------------------------------
    def __and__(self, other):
        return self._kleene("and", "&", other)

    def __rand__(self, other):
        return self._kleene("and", "&", other, reflected=True)

    def __or__(self, other):
        return self._kleene("or", "|", other)

    def __ror__(self, other):
        return self._kleene("or", "|", other, reflected=True)

    def __invert__(self):
        return self._unary("not", f"(~{self._repr})")

    # -- methods ---------------------------------------------------------
    def abs(self):
        """Absolute value (integer ``abs(i64::MIN)`` raises)."""
        return self._unary("abs", f"{self._repr}.abs()")

    def floor(self):
        """Round toward negative infinity (identity on integers)."""
        return self._unary("floor", f"{self._repr}.floor()")

    def ceil(self):
        """Round toward positive infinity (identity on integers)."""
        return self._unary("ceil", f"{self._repr}.ceil()")

    def sqrt(self):
        """Square root (always f64). A NEGATIVE operand raises — a
        silent NaN would flow into every downstream comparison."""
        return self._unary("sqrt", f"{self._repr}.sqrt()")

    def round(self, ndigits=0):
        """Round half AWAY from zero on the shortest decimal
        representation (the data-plane oracle's behavior; differs from
        Python's banker's rounding). ``ndigits`` may be negative
        (``round(-2)`` -> hundreds). On i64 only ``ndigits=0`` (the
        identity) is supported — cast to f64 first for digit rounding.
        """
        if not isinstance(ndigits, int) or isinstance(ndigits, bool):
            raise TypeError("Expr.round: ndigits must be an int")
        return Expr(
            {"op": "round", "args": [self._tree], "ndigits": ndigits},
            f"{self._repr}.round({ndigits})",
        )

    def cast(self, to):
        """Cast between numeric types: ``"i64"`` or ``"f64"``.
        f64 -> i64 rounds half TO EVEN with a range check (NaN, inf and
        out-of-range values raise; nothing becomes a silent null)."""
        if to not in ("i64", "f64"):
            raise ValueError(
                f'Expr.cast: target {to!r} is outside the v1 subset '
                '(supported: "i64", "f64")'
            )
        return Expr(
            {"op": "cast", "args": [self._tree], "to": to},
            f"{self._repr}.cast({to!r})",
        )

    def is_null(self):
        """True where the value is null (never null itself)."""
        return self._unary("is_null", f"{self._repr}.is_null()")

    def is_not_null(self):
        """True where the value is present (never null itself)."""
        return self._unary("is_not_null", f"{self._repr}.is_not_null()")

    def fill_null(self, value):
        """Replace nulls with ``value`` (a scalar or another Expr of a
        matching type)."""
        return self._method("fill_null", "fill_null", value)

    def str_contains(self, needle):
        """Substring test (null in -> null out)."""
        return self._method("str_contains", "str_contains", needle)

    def str_starts_with(self, prefix):
        """Prefix test (null in -> null out)."""
        return self._method("str_starts_with", "str_starts_with", prefix)

    def str_ends_with(self, suffix):
        """Suffix test (null in -> null out)."""
        return self._method("str_ends_with", "str_ends_with", suffix)

    def str_len(self):
        """String length in BYTES (utf8), as i64."""
        return self._unary("str_len", f"{self._repr}.str_len()")

    def concat(self, other):
        """String concatenation (null in -> null out; use
        ``.fill_null("")`` first to treat nulls as empty)."""
        return self._method("concat", "concat", other)


def col(name):
    """A column reference: ``col("price")``. The column must exist in
    the table handed to :func:`with_columns` / :func:`filter` —
    existence and types are checked there, before anything computes."""
    if not isinstance(name, str) or not name:
        raise TypeError("col() takes a non-empty column name string")
    return Expr({"op": "col", "name": name}, f'col("{name}")')


def lit(value):
    """An explicit literal: ``lit(2)``, ``lit("far")``. Raw Python
    scalars auto-lift in operator positions (``col("price") * 2``), so
    lit() is only needed as a standalone expression or for clarity.
    bool is checked before int (a Python bool never becomes an i64);
    ints must fit i64; non-finite floats and None raise."""
    tree, rep = _lift(value, "lit")
    if isinstance(value, Expr):
        raise TypeError("lit() takes a raw scalar, not an Expr")
    return Expr(tree, rep)


def if_else(cond, then, otherwise):
    """SQL CASE: rows where ``cond`` is true take ``then``, rows where
    it is false OR NULL take ``otherwise``. Branch types must match
    (numeric branches promote together)."""
    if not isinstance(cond, Expr):
        raise TypeError(
            "if_else: the condition must be an Expr (comparisons and "
            "boolean ops build one)"
        )
    tt, tr = _lift(then, "if_else")
    ft, fr = _lift(otherwise, "if_else")
    return Expr(
        {"op": "if_else", "args": [cond._tree, tt, ft]},
        f"if_else({cond!r}, {tr}, {fr})",
    )


def with_columns(data, **named_exprs):
    """Append computed columns: one obvious line per derived field.

        total = derive.with_columns(orders, total=col("price") * col("qty"))

    ``data`` is anything the session accepts (an Arrow-compatible table
    or a dict of column lists). Every expression sees the INPUT columns
    only — a name computed in the same call is not visible to its
    siblings (chain a second call for dependent columns). Output: the
    input columns, then the computed columns in keyword order. A name
    colliding with an existing column raises; ``handle`` is reserved.

    Values may be Exprs or raw scalars (a scalar broadcasts to every
    row). Rows compute independently and deterministically — same batch
    in, same batch out, which is what keeps WAL replay re-derivable.
    """
    trees = {}
    for name, e in named_exprs.items():
        if isinstance(e, Expr):
            trees[name] = e._tree
        else:
            tree, _ = _lift(e, f"with_columns({name}=...)")
            trees[name] = tree
    return _with_columns(data, trees)


def filter(data, pred):  # noqa: A001 - mirrors the SQL/polars verb
    """Keep rows where ``pred`` is TRUE — SQL WHERE semantics: rows
    where the predicate is false or NULL drop.

        far = derive.filter(pairs, col("dist_m") > 5000)

    The predicate must be a boolean Expr (a bare Python bool is almost
    certainly a bug — a constant filter keeps everything or nothing).
    The output schema is the input's, unchanged; a ``handle`` column
    rides through untouched for handle-aligned ``Session.update`` /
    ``Session.delete`` pipelines."""
    if not isinstance(pred, Expr):
        raise TypeError(
            "filter: the predicate must be an Expr (comparisons and "
            "boolean ops build one)"
        )
    return _filter(data, pred._tree)
