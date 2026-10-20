"""Decision tables: Drools' CSV RuleTable and RuleMatrix formats, compiled to DRL text.

A decision table is an AUTHORING frontend. Drools expands a spreadsheet into DRL with its SpreadsheetCompiler and the
engine never sees a table; Seine does the same. This module is a line-by-line port of drools-decisiontables 9.44.0
(DefaultRuleSheetListener, RuleMatrixSheetListener, LhsBuilder, RhsBuilder, the CSV parsers) and the drools-templates
model that renders the DRL. The certification is text parity: for every table in scenarios/tables/, `drl` is
byte-identical to what SpreadsheetCompiler prints (oracle-tables/, tools/table_oracle.py), parse errors included. The
generated rules are then ordinary DRL, certified by the engine corpus like any other.

    t = seine_rs.decision_table("pricing.csv")          # RuleTable: one rule per data row
    t = seine_rs.decision_table("grid.csv", matrix=True)  # RuleMatrix: one rule per filled grid cell
    t.drl          # Drools' text, byte for byte
    t.rules        # the same rules as engine DRL (no package/import header)
    t.warnings     # Drools-compatible input that is probably a mistake (a header read by its first letter, ...)
    seine_rs.run(t, facts)

    python -m seine_rs table [--matrix] [--rules] TABLE.csv     # the DRL on stdout; errors on stderr, exit 1

Table format (Drools' own; see its documentation for the full reference):

    RuleSet,my_rules                     <- optional properties, one per row: key cell, value cell
    Import,org.acme.Person               (Import / Variables / Functions / Queries / Declare / Sequential /
                                          EscapeQuotes / NumericDisabled / SequentialMin|MaxPriority / Unit /
                                          Dialect / any rule attribute as a ruleset-wide default)
    RuleTable Discounts                  <- starts a table; the text after it prefixes the rule names
    CONDITION,CONDITION,ACTION           <- column kinds: CONDITION, ACTION, NAME, DESCRIPTION, PRIORITY, NO-LOOP,
                                            AGENDA-GROUP, ACTIVATION-GROUP, METADATA, ... (or their one-letter codes)
    Person,Person,                       <- object type the CONDITION columns constrain (blank: free-standing)
    age > $param,tier == "$param",insert(new Discount($param));   <- code snippets; $param / $1 $2 / forall(op){..}
    Age over,Tier,Discount               <- labels (comments; not rendered)
    30,"gold",10                         <- one rule per row; a blank cell leaves that column out of the rule
    ,"silver",5

    AgendaGroup,pricing                  <- a RuleMatrix (matrix=True) reads these properties instead
    HorizontalCondition,Tier(name == "$param")
    VerticalCondition,Age(band == "$param")
    Action,insert(new Discount($param));
    RuleTable Grid
    ,gold,silver                         <- column headers fill HorizontalCondition
    young,10,5                           <- row header fills VerticalCondition; each filled cell is one rule
"""
from __future__ import annotations

import os
import re
from typing import Optional

from .authoring import CompileError

__all__ = ["DecisionTable", "DecisionTableError", "decision_table"]


class DecisionTableError(CompileError):
    """A table that cannot be compiled.

    Where Drools reports the problem (DecisionTableParseException) the message is Drools' own, word for word. Where
    Drools would CRASH instead (a NullPointerException on a RuleMatrix without an AgendaGroup, an index error on a grid
    cell without a column header, ...), Seine detects the cause and reports it, with `drools_crashes` set. No input
    makes `decision_table()` raise anything but this error (or OSError-free TypeError for a misused call).

    Attributes: `cell` ("C7") when the problem is at one cell, `source` (the file path) when compiled from a file."""

    def __init__(self, message: str, *, cell: Optional[str] = None, drools_crashes: bool = False):
        super().__init__(message)
        self.message = message
        self.cell = cell
        self.drools_crashes = drools_crashes
        self.source: Optional[str] = None

    def __str__(self):
        return self.message if self.source is None else f"{self.source}: {self.message}"


def _guard(message: str, cell: Optional[str] = None) -> DecisionTableError:
    """An input Drools would crash on: reported, not crashed."""
    return DecisionTableError(message, cell=cell, drools_crashes=True)


NON_MERGED = -1


# ---------------------------------------------------------------- Java semantics the output depends on

def _jtrim(s: str) -> str:
    """java.lang.String.trim(): strips code points <= U+0020 only (not Unicode whitespace)."""
    i, j = 0, len(s)
    while i < j and ord(s[i]) <= 32:
        i += 1
    while j > i and ord(s[j - 1]) <= 32:
        j -= 1
    return s[i:j]


def _jstr_hash(s: str) -> int:
    """java.lang.String.hashCode over UTF-16 code units, as an unsigned 32-bit value."""
    h = 0
    for unit in _utf16_units(s):
        h = (31 * h + unit) & 0xFFFFFFFF
    return h


def _utf16_units(s: str):
    b = s.encode("utf-16-le")
    return [b[i] | (b[i + 1] << 8) for i in range(0, len(b), 2)]


def _spread(h: int) -> int:
    return (h ^ (h >> 16)) & 0xFFFFFFFF


class _JavaHashMap:
    """java.util.HashMap with Java's ITERATION ORDER: bucket index (spread hash & (capacity - 1)), insertion order
    within a bucket, capacity doubling past 0.75 load. Drools renders rule attributes and orders table properties by
    iterating HashMaps, so the DRL text depends on it."""

    def __init__(self, key_hash):
        self._d: dict = {}
        self._order: list = []
        self._max = 0
        self._hash = key_hash

    def put(self, k, v):
        if k not in self._d:
            self._order.append(k)
            self._max = max(self._max, len(self._order))
        self._d[k] = v

    def get(self, k, default=None):
        return self._d.get(k, default)

    def __contains__(self, k):
        return k in self._d

    def __len__(self):
        return len(self._d)

    def clear(self):
        self._d.clear()
        self._order.clear()     # Java's clear() keeps the table: capacity stays at what it reached
        # (self._max is kept on purpose)

    def _capacity(self) -> int:
        cap = 16
        while self._max > cap * 0.75:
            cap *= 2
        return cap

    def keys(self):
        cap = self._capacity()
        return sorted(self._order, key=lambda k: _spread(self._hash(k)) & (cap - 1))   # sorted() is stable

    def items(self):
        return [(k, self._d[k]) for k in self.keys()]

    def values(self):
        return [self._d[k] for k in self.keys()]


def _str_map() -> _JavaHashMap:
    return _JavaHashMap(_jstr_hash)


def _int_map() -> _JavaHashMap:
    return _JavaHashMap(lambda i: i & 0xFFFFFFFF)


def _rc2name(row: int, col: int) -> str:
    """RuleSheetParserUtil.rc2name: (0-based row, col) -> spreadsheet cell name, Drools' arithmetic verbatim."""
    sb = []
    b, p = 26, 1
    if col >= b:
        col -= b
        p *= b
    if col >= b * b:
        col -= b * b
        p *= b
    while p > 0:
        sb.append(chr(col // p + ord("A")))
        col %= p
        p //= b
    sb.append(str(row + 1))
    return "".join(sb)


def _is_true(prop: Optional[str]) -> bool:
    """RuleSheetParserUtil.isStringMeaningTrue."""
    if prop is None:
        return False
    prop = _jtrim(prop)
    return prop.lower() == "true" or prop.startswith("Y") or prop.startswith("y") or prop.lower() == "on"


def _java_int(value: str) -> int:
    """Integer.valueOf(String): optional sign, ASCII digits, 32-bit range; anything else is a NumberFormatException."""
    if not re.fullmatch(r"[+-]?[0-9]+", value):
        raise ValueError(value)
    v = int(value)
    if not -2**31 <= v < 2**31:
        raise ValueError(value)
    return v


def _java_long(value: str) -> int:
    if not re.fullmatch(r"[+-]?[0-9]+", value):
        raise ValueError(value)
    v = int(value)
    if not -2**63 <= v < 2**63:
        raise ValueError(value)
    return v


def _count_quotes(s: str, start: int, end: int) -> int:
    return sum(1 for i in range(start, end) if s[i] == '"' and (i == 0 or s[i - 1] != "\\"))


def _replace_out_of_quotes(s: str, old: str, new: str) -> str:
    """org.drools.util.StringUtils.replaceOutOfQuotes, including its scan step of len(new) after every hit."""
    i = s.find(old)
    while i >= 0:
        if _count_quotes(s, 0, i) % 2 != 1:
            s = s[:i] + new + s[i + len(old):]
        i = s.find(old, i + len(new))
    return s


def _split_constraints(string: str) -> list:
    """StringUtils.splitConstraints(string, false): split on commas outside quotes and brackets."""
    args, last, nested = [], 0, 0
    sq = dq = False
    i, n = 0, len(string)
    while i < n:
        c = string[i]
        if c == ",":
            if not sq and not dq and nested == 0:
                args.append(string[last:i])
                last = i + 1
        elif c in "([{":
            if not sq and not dq:
                nested += 1
        elif c in ")]}":
            if not sq and not dq:
                nested -= 1
        elif c == '"':
            if not sq and (i == 0 or string[i - 1] != "\\"):
                dq = not dq
        elif c == "'":
            if not dq and (i == 0 or string[i - 1] != "\\"):
                sq = not sq
        elif c == "\\":
            if i + 1 < n and string[i + 1] == '"':
                i += 1
        i += 1
    tail = string[last:]
    if len(tail) > 0:
        args.append(tail)
    return args


# ---------------------------------------------------------------- drools-templates: SnippetBuilder

_PARAM = "$param"
_FORALL = re.compile(r"forall\(([^{}]*)\)\{([^{}]+)\}")


def _snippet_type(template: str) -> str:
    if _FORALL.search(template):
        return "FORALL"
    if "$1" in template:
        return "INDEXED"
    if _PARAM in template:
        return "PARAM"
    return "SINGLE"


def _replace(text: str, repl: str, with_: str, max_: int = 256) -> str:
    """SnippetBuilder.replace: at most 256 substitutions; newlines in the value become a literal backslash-n."""
    if text is None or not repl or with_ is None or max_ == 0:
        return text
    with_ = with_.replace("\n", "\\n")
    out, start = [], 0
    while True:
        end = text.find(repl, start)
        if end == -1:
            break
        out.append(text[start:end])
        out.append(with_)
        start = end + len(repl)
        max_ -= 1
        if max_ == 0:
            break
    out.append(text[start:])
    return "".join(out)


class _Snippet:
    def __init__(self, template: Optional[str], trim: bool = True):
        if template is None:
            raise _guard("a code snippet is missing (Drools: \"Script template is null\")")
        self.t, self.trim, self.type = template, trim, _snippet_type(template)

    def build(self, cell: str) -> str:
        if self.type == "FORALL":
            return self._forall(cell)
        if self.type == "INDEXED":
            vals = _split_constraints(cell)
            r = self.t
            for n in range(len(vals) - 1, -1, -1):
                r = _replace(r, "$" + str(n + 1), _jtrim(vals[n]) if self.trim else vals[n])
            return r
        return _replace(self.t, _PARAM, cell)

    def _forall(self, cell: str) -> str:
        vals = _split_constraints(cell)
        reps = _str_map()
        for m in _FORALL.finditer(self.t):
            g = m.group(0)
            reps.put(g, "")
            for n, v in enumerate(vals):
                reps.put(g, reps.get(g) + ("" if n == 0 else " " + m.group(1) + " ")
                         + _replace(m.group(2), "$", _jtrim(v)))
        r = self.t
        for k in reps.keys():
            r = _replace(r, k, reps.get(k))
        return self.t if r == "" else r


# ---------------------------------------------------------------- drools-templates: the DRL model

def _as_string_literal(value: Optional[str]) -> str:
    if value is None:
        raise _guard("a rule attribute has no value")
    if value.startswith('"') and value.endswith('"') and len(value) >= 2:
        value = value[1:-1]
    return '"' + value.replace('"', '\\"') + '"'


def _as_timer_literal(value: str) -> str:
    if value.startswith("(") and value.endswith(")") and len(value) >= 2:
        value = value[1:-1]
    return "(" + value + ")"


class _Attributed:
    def __init__(self, salience: Optional[int] = None):
        self.attr = _str_map()
        self.comment: Optional[str] = None
        if salience is not None:
            self.attr.put("salience", str(salience))

    def render_attrs(self, out: list):
        for k, v in self.attr.items():
            out.append(f"\t{k} {v}")

    def set_salience(self, v):
        self.attr.put("salience", str(v))

    def set(self, code: str, value: str):
        a = self.attr
        if code == "DURATION":
            a.put("duration", str(value))
        elif code == "TIMER":
            a.put("timer", _as_timer_literal(value))
        elif code == "ENABLED":
            a.put("enabled", "true" if _is_true(value) else "false")
        elif code == "CALENDARS":
            a.put("calendars", _as_string_literal(value))
        elif code == "NO-LOOP":
            a.put("no-loop", "true" if _is_true(value) else "false")
        elif code == "LOCK-ON-ACTIVE":
            a.put("lock-on-active", "true" if _is_true(value) else "false")
        elif code == "AUTO-FOCUS":
            a.put("auto-focus", "true" if _is_true(value) else "false")
        elif code == "ACTIVATION-GROUP":
            a.put("activation-group", _as_string_literal(value))
        elif code == "AGENDA-GROUP":
            a.put("agenda-group", _as_string_literal(value))
        elif code == "RULEFLOW-GROUP":
            a.put("ruleflow-group", _as_string_literal(value))
        elif code == "DATE-EFFECTIVE":
            a.put("date-effective", _as_string_literal(value))
        elif code == "DATE-EXPIRES":
            a.put("date-expires", _as_string_literal(value))


class _Rule(_Attributed):
    def __init__(self, name: str, salience: Optional[int], spreadsheet_row: int):
        super().__init__(salience)
        self.name = _as_string_literal(name)
        self.description: Optional[str] = None
        self.metadata: list = []
        self.lhs: list = []
        self.rhs: list = []

    def render(self, out: list):
        if self.comment:
            out.append("//" + self.comment)
        out.append("rule " + self.name)
        if self.description is not None:
            out.append("/* " + self.description + " */")
        for m in self.metadata:
            out.append("\t@" + m)
        self.render_attrs(out)
        out.append("\twhen")
        for c in self.lhs:
            if c is None:
                raise _guard(f"rule {self.name} has an empty condition")
            out.append("\t\t" + c)
        out.append("\tthen")
        for c in self.rhs:
            out.append("\t\t" + c)
        out.append("end\n")


class _Package(_Attributed):
    def __init__(self, name: Optional[str]):
        super().__init__()
        self.name = name
        self.imports: list = []
        self.globals: list = []
        self.rules: list = []
        self.functions = self.queries = self.declares = None
        self.unit = self.dialect = None

    def render(self, out: list):
        if self.name is not None:
            out.append("package " + self.name.replace(" ", "_") + ";")
        if self.unit is not None:
            out.append("unit " + self.unit + ";")
        if self.dialect is not None:
            out.append('dialect "' + self.dialect + '"')
        out.append("//generated from Decision Table")
        for i in self.imports:
            out.append("import " + i + ";")
        for cls, ident in self.globals:
            out.append("global " + cls + " " + ident + ";")
        for listing in (self.functions, self.queries, self.declares):
            if listing is not None:
                out.append(listing)
        self.render_attrs(out)
        for r in self.rules:
            r.render(out)


# ---------------------------------------------------------------- drools-decisiontables: column kinds

# (header, one-letter code, max columns per table); PRIORITY..DATE-EXPIRES are the attribute codes
_CODES = [("CONDITION", "C", None), ("ACTION", "A", None), ("NAME", "N", 1), ("DESCRIPTION", "I", None),
          ("PRIORITY", "P", 1), ("DURATION", "D", 1), ("TIMER", "T", 1), ("ENABLED", "B", 1), ("CALENDARS", "E", 1),
          ("NO-LOOP", "U", 1), ("LOCK-ON-ACTIVE", "L", 1), ("AUTO-FOCUS", "F", 1), ("ACTIVATION-GROUP", "X", 1),
          ("AGENDA-GROUP", "G", 1), ("RULEFLOW-GROUP", "R", 1), ("DATE-EFFECTIVE", "V", 1), ("DATE-EXPIRES", "Z", 1),
          ("METADATA", "@", None)]
_TAG2CODE = {}
for _h, _s, _ in _CODES:
    _TAG2CODE[_h] = _h
    _TAG2CODE[_s] = _h
_MAX = {h: m for h, _, m in _CODES}
_SHORT = {h: s for h, s, _ in _CODES}
_ATTRIBUTE_CODES = [h for h, _, _ in _CODES[4:17]]   # EnumSet.range(SALIENCE, DATEEXPIRES)
_MAX_INT = 2**31 - 1


class _ActionType:
    def __init__(self, code: str):
        self.code = code
        self.src = None

    def add_template(self, row, col, content):
        if self.src is None:
            raise DecisionTableError(f'Unexpected content "{content}" in cell {_rc2name(row, col)}, '
                                     "leave this cell blank")
        self.src.add_template(row, col, content)

    def add_cell_value(self, row, col, content, escape_quotes, trim):
        if escape_quotes:
            idx = content.find('"')
            if idx > 0 and content.find('"', idx) > -1:
                content = content.replace('"', '\\"')
        self.src.add_cell_value(row, col, content, trim)


def _new_action_type(actions: dict, value: str, column: int, row: int, warnings: Optional[list] = None):
    uc = value.upper()
    code = _TAG2CODE.get(uc) or _TAG2CODE.get(uc[:1])
    if code is not None and uc not in _TAG2CODE and warnings is not None:
        warnings.append(f"column header \"{value}\" at {_rc2name(row, column)} is not a column kind; Drools reads its "
                        f"first letter and makes it {code}")
    if code is None:
        raise DecisionTableError(f"Invalid column header: {value}, should be CONDITION, ACTION or attribute, "
                                 f"in cell {_rc2name(row, column)}")
    mx = _MAX[code]
    if mx is not None and sum(1 for a in actions.values() if a.code == code) >= mx:
        raise DecisionTableError(f"Maximum number of {code}/{_SHORT[code]} columns is {mx}, "
                                 f"in cell {_rc2name(row, column)}")
    actions[column] = _ActionType(code)


# ---------------------------------------------------------------- LhsBuilder / RhsBuilder

_OPERATORS = ("==", "=", "!=", "<", ">", "<=", ">=", "contains", "matches", "memberOf", "str[startsWith]",
              "str[endsWith]", "str[length]")
_PAT_PAR_FRM = re.compile(r"\(\s*\)\s*from\b")
_PAT_FRM = re.compile(r"\s+from\s+")
_PAT_PAR = re.compile(r"\(\s*\)\s*(?=\n?\Z)")
_PAT_EVAL = re.compile(r"\beval\s*(?:\(\s*\)\s*)?(?=\n?\Z)")
_PAT_OOPATH = re.compile(r".*\:\s*/.*")
_QUOTES = ('"', "“", "”")


class _Lhs:
    code = "CONDITION"

    def __init__(self, row: int, column: int, col_def: Optional[str]):
        self.header_row, self.column = row, column
        self.constraints: dict = {}
        self.field_types: dict = {}
        self.values: list = []
        self.for_all = False
        self.has_values = False
        cd = "" if col_def is None else col_def
        ann = ""
        pos = cd.find("@watch")
        if pos > 0:
            ann = " " + cd[pos:]
            cd = _jtrim(cd[:pos])
        if cd == "":
            self.prefix = self.suffix = self.andop = ""
            self.multiple = False
            return
        self.multiple = True
        m = _PAT_EVAL.search(cd)
        if m:
            self.prefix, self.suffix, self.andop = cd[:m.start()] + "eval(", ")", " && "
            return
        self.andop = ", "
        m = _PAT_PAR_FRM.search(cd)
        if m:
            self.prefix, self.suffix = cd[:m.start()] + "(", ") from" + cd[m.end():] + ann
            return
        m = _PAT_FRM.search(cd)
        if m:
            self.prefix, self.suffix = cd[:m.start()] + "(", ") from " + cd[m.end():] + ann
            return
        m = _PAT_PAR.search(cd)
        if m:
            self.prefix, self.suffix = cd[:m.start()] + "(", ")" + cd[m.end():] + ann
            return
        if _PAT_OOPATH.fullmatch(cd):
            self.prefix, self.suffix = cd + "[", "]" + ann
            return
        if cd.endswith(")"):
            self.prefix, self.suffix = cd, ann
        else:
            self.prefix, self.suffix = cd + "(", ")" + ann

    @staticmethod
    def _field_type(content: str) -> str:
        st = _snippet_type(content)
        if st == "FORALL":
            return "FORALL"
        if st != "SINGLE":
            return "NORMAL"
        if any(content.endswith(op) for op in _OPERATORS):
            return "OPERATOR"
        return "QUESTION" if content.endswith("?") else "SINGLE"

    def add_template(self, row, column, content):
        content = _jtrim(content)
        if column in self.constraints:
            raise _guard("Can't have a constraint added twice to one spreadsheet col.", _rc2name(row, column))
        ft = self._field_type(content)
        if not self.multiple:
            self.constraints[column] = content
        elif ft in ("FORALL", "NORMAL"):
            self.for_all = self.for_all or ft == "FORALL"
            self.constraints[column] = content
        elif ft == "SINGLE":
            self.constraints[column] = content + ' == "' + _PARAM + '"'
        elif ft == "OPERATOR":
            self.constraints[column] = content + ' "' + _PARAM + '"'
        else:
            self.constraints[column] = content[:-1]
        self.field_types[column] = ft

    def add_cell_value(self, row, column, value, trim=True):
        self.has_values = True
        if not self.constraints:
            return
        content = self.constraints.get(column)
        if content is None:
            raise DecisionTableError("No code snippet for CONDITION in cell "
                                     + _rc2name(self.header_row + 2, self.column))
        self.values.append(_Snippet(content, trim).build(self._fix(column, value)))

    def _fix(self, column, value):
        if self.field_types.get(column) == "NORMAL" or not self.multiple or self.for_all:
            return value
        for o, c in (('"', '"'), ("“", "”"), ("“", "“"), ("”", "”")):
            if value.find(o) == 0 and value.find(c, 1) == len(value) - 1:
                return value[1:-1]
        return value

    def clear(self):
        self.has_values = False
        self.values.clear()

    def result(self) -> str:
        if not self.multiple:
            return "\n".join(self.values)
        return self.prefix + self.andop.join(self.values) + self.suffix


class _Rhs:
    def __init__(self, code: str, row: int, column: int, bound: Optional[str]):
        self.code, self.header_row, self.column = code, row, column
        self.var = "" if bound is None else _jtrim(bound)
        self.templates: dict = {}
        self.values: list = []
        self.has_values = False

    def add_template(self, row, column, content):
        content = _jtrim(content)
        if self.var != "":
            content = self.var + "." + content + ";"
        self.templates[column] = content

    def add_cell_value(self, row, column, value, trim=True):
        self.has_values = True
        t = self.templates.get(column)
        if t is None:
            raise DecisionTableError(f"No code snippet for {self.code}, above cell "
                                     + _rc2name(self.header_row + 2, self.column))
        self.values.append(_Snippet(t).build(value))

    def clear(self):
        self.has_values = False
        self.values.clear()

    def result(self) -> str:
        return "\n".join(self.values)


# ---------------------------------------------------------------- PropertiesSheetListener

class _Properties:
    def __init__(self):
        self.rows = _int_map()           # row -> [key, value, cell]: flushed in Java HashMap order
        self.props: dict = {}            # lower-cased key -> [[value, cell], ...]

    def flush(self):
        for kv in self.rows.values():
            self.props.setdefault(kv[0].lower(), []).append([kv[1], kv[2]])
        self.rows.clear()

    def new_cell(self, row, column, value):
        if value is None or _jtrim(value) == "":
            return
        if row in self.rows:
            kv = self.rows.get(row)
            if kv[1] == "":
                kv[1], kv[2] = value, _rc2name(row, column)
        else:
            self.rows.put(row, [_jtrim(value), "", _rc2name(row, column + 1)])

    def get(self, key):
        self.flush()
        r = self.props.get(key.lower())
        return None if r is None else [v for v, _ in r]

    def cells(self, key):
        self.flush()
        r = self.props.get(key.lower())
        return None if r is None else [c for _, c in r]

    def single(self, key, default=None):
        self.flush()
        r = self.props.get(key.lower())
        v = r[0][0] if r else None
        if default is not None and (v is None or v == ""):
            return default
        return v

    def single_cell(self, key):
        self.flush()
        r = self.props.get(key.lower())
        return r[0][1] if r else None


# ---------------------------------------------------------------- DefaultRuleSheetListener

class _RuleSheet:
    ACTION_ROW, OBJECT_TYPE_ROW, CODE_ROW, LABEL_ROW = 1, 2, 3, 4

    def __init__(self, show_package: bool = True, trim_cell: bool = True):
        self.show_package, self.trim_cell = show_package, trim_cell
        self.in_table = False
        self.rule_row = self.start_col = self.start_row = 0
        self.rule: Optional[_Rule] = None
        self.prefix = ""
        self.sequential = False
        self.escape_quotes = True
        self.salience = 65535            # template Rule.MAX_ROWS
        self.min_salience = 0
        self.actions: dict = {}
        self.rules: list = []
        self.builders: Optional[dict] = None     # column -> builder (TreeSet by column: first one per column wins)
        self.props = _Properties()
        self.warnings: list = []                 # Drools-compatible but surprising input; never changes the DRL

    # -- the sheet
    def finish_sheet(self):
        self.props.flush()
        self._finish_table()
        self._flush_rule()

    def new_row(self, row, columns):
        if self.rule is not None:
            self._flush_rule()

    def _flush_rule(self):
        if self.builders is None:
            return
        for col in sorted(self.builders):
            src = self.builders[col]
            if src.has_values:
                if src.code == "CONDITION":
                    self.rule.lhs.append(_replace_out_of_quotes(src.result(), "\\n", " "))
                elif src.code == "ACTION":
                    self.rule.rhs.append(_replace_out_of_quotes(src.result(), "\\n", " "))
                elif src.code == "METADATA":
                    self.rule.metadata.append(src.result())
                src.clear()

    def _add_builder(self, src):
        if src.column not in self.builders:
            self.builders[src.column] = src

    def new_cell(self, row, column, value, merged_start):
        if value is None or _jtrim(value) == "":
            return
        if self.in_table and row == self.start_row:
            return
        if self.in_table:
            self._rule_cell(row, column, value, merged_start)
        else:
            self._non_rule_cell(row, column, value)

    @staticmethod
    def _is_rule_table(test: str) -> bool:
        return test == "ruletable" or test.startswith("ruletable ")

    def _non_rule_cell(self, row, column, value):
        if self._is_rule_table(_jtrim(value).lower()):
            self._init_table(row, column, _jtrim(value), True)
        else:
            self.props.new_cell(row, column, value)

    # -- hooks the matrix listener overrides
    def pre_init(self, row, column, value):
        pass

    def post_init(self, row, column, value):
        pass

    def _flag(self, name, default: bool) -> bool:
        return _is_true(self.props.single(name, "true" if default else "false"))

    def _number(self, name, default: int) -> int:
        try:
            return _java_int(self.props.single(name, str(default)))
        except ValueError:
            raise DecisionTableError("Invalid numeric value for option: " + name) from None

    def _init_table(self, row, column, value, first):
        self.pre_init(row, column, value)
        self.in_table = True
        self.actions = {}
        self.builders = {}
        self.start_col, self.start_row = column, row
        self.rule_row = row + self.LABEL_ROW + 1
        low = value.lower()
        self.prefix = _jtrim(value[low.find("ruletable") + len("ruletable"):])
        self.sequential = self._flag("Sequential", False)
        self.escape_quotes = self._flag("EscapeQuotes", True)
        if first:
            self.salience = self._number("SequentialMaxPriority", self.salience)
            self.min_salience = self._number("SequentialMinPriority", self.min_salience)
        self.rule = self._new_rule(self.rule_row, _rc2name(self.start_row, self.start_col),
                                   _rc2name(self.rule_row, self.start_col))
        self.rules.append(self.rule)
        self.post_init(row, column, value)

    def _finish_table(self):
        if self.in_table:
            self.sequential = False
            self.in_table = False

    def _rule_cell(self, row, column, value, merged_start):
        val = _jtrim(value) if self.trim_cell else value
        if self._is_rule_table(val.lower()):
            self._finish_table()
            self._init_table(row, column, val, False)
            return
        if column < self.start_col or row == self.start_row:
            return
        d = row - self.start_row
        if d == self.ACTION_ROW:
            _new_action_type(self.actions, val, column, row, self.warnings)
        elif d == self.OBJECT_TYPE_ROW:
            self._object_type_row(row, column, val, merged_start)
        elif d == self.CODE_ROW:
            self._code_row(row, column, val)
        elif d == self.LABEL_ROW:
            self._action_for(row, column)      # label cells are comments; a label without a column kind still fails
        else:
            self._data_cell(row, column, val)

    def _action_for(self, row, column) -> _ActionType:
        a = self.actions.get(column)
        if a is None:
            raise DecisionTableError(f"Code description in cell {_rc2name(row, column)} "
                                     "does not have an 'ACTION' or 'CONDITION' column header.")
        return a

    def _object_type_row(self, row, column, value, merged_start):
        if "$param" in value or "$1" in value:
            raise DecisionTableError("It looks like you have snippets in the row that is meant for object "
                                     "declarations. Please insert an additional row before the snippets, at cell "
                                     + _rc2name(row, column))
        a = self._action_for(row, column)
        if merged_start == NON_MERGED or column == merged_start:
            if a.code == "CONDITION":
                a.src = _Lhs(row - 1, column, value)
                self._add_builder(a.src)
            elif a.code == "ACTION":
                a.src = _Rhs("ACTION", row - 1, column, value)
                self._add_builder(a.src)
        else:
            a.src = self._action_for(row, merged_start).src

    def _code_row(self, row, column, value):
        a = self._action_for(row, column)
        if a.src is None:
            if a.code in ("CONDITION", "PRIORITY"):
                a.src = _Lhs(row - 2, column, None)
                self._add_builder(a.src)
            elif a.code in ("ACTION", "METADATA"):
                a.src = _Rhs(a.code, row - 2, column, None)
                self._add_builder(a.src)
        if _jtrim(value) == "" and a.code in ("ACTION", "CONDITION", "METADATA"):
            raise DecisionTableError(f"Code description in cell {_rc2name(row, column)} does not contain any "
                                     "code specification. It should!")
        a.add_template(row, column, value)

    def _new_rule(self, row, head_cell, rule_cell) -> _Rule:
        salience = None
        if self.sequential:
            salience = self.salience
            self.salience -= 1
            if salience < self.min_salience:
                raise DecisionTableError(f"Salience less than the minimum specified on row: {row}")
        r = _Rule(f"{self.prefix}_{row + 1}", salience, row + 1)
        r.comment = f" rule values at {rule_cell}, header at {head_cell}"
        return r

    def _data_cell(self, row, column, value):
        a = self._action_for(row, column)
        if row - self.rule_row > 1:
            self._finish_table()
            self._non_rule_cell(row, column, value)
            return
        if row > self.rule_row:
            self.rule = self._new_rule(row, _rc2name(self.start_row, self.start_col),
                                       _rc2name(row, self.start_col))
            self.rules.append(self.rule)
            self.rule_row += 1
        c, r = a.code, self.rule
        if c in ("CONDITION", "ACTION", "METADATA"):
            if a.src is None:
                raise DecisionTableError(f"Data cell {_rc2name(row, column)} has an empty column header.")
            a.add_cell_value(row, column, value, self.escape_quotes, self.trim_cell)
        elif c == "PRIORITY":
            if not self.sequential:
                if value.startswith("(") and value.endswith(")"):
                    r.set_salience(value)
                else:
                    try:
                        r.set_salience(_java_int(value))
                    except ValueError:
                        raise DecisionTableError("Priority is not an integer literal, in cell "
                                                 + _rc2name(row, column)) from None
        elif c == "NAME":
            r.name = _as_string_literal(value)
        elif c == "DESCRIPTION":
            r.description = value
        elif c == "DURATION":
            try:
                r.set("DURATION", str(_java_long(value)))
            except ValueError:
                raise DecisionTableError("Duration is not an integer literal, in cell "
                                         + _rc2name(row, column)) from None
        else:
            r.set(c, value)

    # -- the package
    def rule_set(self) -> _Package:
        if not self.rules:
            raise DecisionTableError("No RuleTable cells in spreadsheet.")
        p = self.props
        pkg = _Package(p.single("RuleSet", "rule_table") if self.show_package else None)
        pkg.rules = list(self.rules)
        units = p.get("Unit")
        if units:
            pkg.unit = units[0]
        dialects = p.get("Dialect")
        if dialects:
            pkg.dialect = dialects[0]
        for cell in p.get("Import") or []:
            pkg.imports += [_jtrim(t) for t in _tokens(cell, ",")]
        for cell in p.get("Variables") or []:
            for tok in _tokens(cell, ","):
                parts = _tokens(tok, " ")
                if len(parts) < 2:
                    raise DecisionTableError("The format for global variables is incorrect. It should be: "
                                             f"[Class name, Class otherName]. But it was: [{cell}]")
                pkg.globals.append((parts[0], parts[1]))
        for key, attr in (("Functions", "functions"), ("Queries", "queries"), ("Declare", "declares")):
            for listing in p.get(key) or []:
                setattr(pkg, attr, listing)       # Drools keeps only the last listing of each
        for code in _ATTRIBUTE_CODES:
            values = p.get(code)
            if values is None:
                continue
            if len(values) > 1:
                raise DecisionTableError(f"Multiple values for {code} in cells [{', '.join(p.cells(code))}]")
            v = values[0]
            if code == "PRIORITY":
                try:
                    pkg.set_salience(_java_int(v))
                except ValueError:
                    raise DecisionTableError("Priority is not an integer literal, in cell "
                                             + p.single_cell(code)) from None
            elif code == "DURATION":
                try:
                    pkg.set("DURATION", str(_java_long(v)))
                except ValueError:
                    raise DecisionTableError("Duration is not an integer literal, in cell "
                                             + p.single_cell(code)) from None
            else:
                pkg.set(code, v)
        return pkg


def _tokens(s: str, delims: str) -> list:
    """java.util.StringTokenizer: split on any delimiter char, dropping empty tokens."""
    return [t for t in re.split("[" + re.escape(delims) + "]", s) if t != ""]


# ---------------------------------------------------------------- RuleMatrixSheetListener

class _MatrixSheet(_RuleSheet):
    """RuleMatrixSheetListener. Drools' text is reproduced for every grid Drools compiles CORRECTLY; where it would
    crash or silently build wrong rules, the grid is refused with a DecisionTableError (drools_crashes=True):
      - no AgendaGroup / Precondition / Action / HorizontalCondition / VerticalCondition property (Drools: NPE or
        "Script template is null");
      - a blank cell between column headers (Drools shifts every later column onto the wrong condition, or crashes
        with an index error past the end);
      - a value in a row whose row header is blank (Drools silently reuses the previous row's condition, or crashes
        on the first row)."""

    _PROPS = ("AgendaGroup", "Precondition", "Action", "HorizontalCondition", "VerticalCondition")

    def __init__(self):
        super().__init__()
        self.m_row = self.m_col = 0
        self.m_in = False
        self.agenda = self.action = self.hcond = self.vcond = None
        self.precondition = None
        self.hconds: list = []
        self.hcols: list = []          # the column of each header, in order
        self.vcurrent = None
        self.vrow = None               # the row the current row header belongs to
        self.first: Optional[_Rule] = None

    def _need(self, prop, value, cell):
        if value is None:
            raise _guard(f"RuleMatrix: no {prop} property, needed for cell {cell}; add a row \"{prop},...\" "
                         "above the RuleTable cell", cell)

    def new_cell(self, row, column, value, merged_start):
        if not self.m_in:
            super().new_cell(row, column, value, merged_start)
            return
        if value is None or value == "":           # StringUtils.isEmpty: no trim here
            return
        cell = _rc2name(row, column)
        if row == self.m_row and column > self.m_col:
            self._need("HorizontalCondition", self.hcond, cell)
            if column != self.m_col + 1 + len(self.hconds):
                gap = _rc2name(row, self.m_col + 1 + len(self.hconds))
                raise _guard(f"RuleMatrix: column header cell {gap} is blank but {cell} after it is not; every "
                             "column needs a header (Drools would shift the later columns onto the wrong condition)",
                             gap)
            self.hconds.append(_Snippet(self.hcond).build(value))
            self.hcols.append(column)
        elif row > self.m_row and column == self.m_col:
            self._need("VerticalCondition", self.vcond, cell)
            self.vcurrent = _Snippet(self.vcond).build(value)
            self.vrow = row
        elif row > self.m_row and column > self.m_col:
            self._create(row, column, value)

    def _create(self, row, column, value):
        cell = _rc2name(row, column)
        for prop, v in (("Action", self.action), ("AgendaGroup", self.agenda), ("Precondition", self.precondition)):
            self._need(prop, v, cell)
        if self.vrow != row:
            raise _guard(f"RuleMatrix: cell {cell} has a value but its row header {_rc2name(row, self.m_col)} is "
                         "blank (Drools would reuse the previous row's condition)", cell)
        k = column - (self.m_col + 1)
        if k >= len(self.hconds):
            raise _guard(f"RuleMatrix: cell {cell} has a value but its column header {_rc2name(self.m_row, column)} "
                         "is blank", cell)
        cons = _Snippet(self.action).build(value)
        rule = self.first
        if rule is None:
            rule = _Rule(f"rule_{row}_{column}", None, row)
            self.rules.append(rule)
        else:
            self.first = None
            rule.name = _as_string_literal(f"rule_{row}_{column}")
        rule.set("AGENDA-GROUP", self.agenda)
        rule.lhs.append(self.precondition)
        rule.lhs.append(self.vcurrent)
        rule.lhs.append(self.hconds[k])
        rule.rhs.append(cons)

    def new_row(self, row, columns):
        pass

    def finish_sheet(self):
        pass

    def post_init(self, row, column, value):
        self.first = self.rule

    def pre_init(self, row, column, value):
        self.m_col, self.m_row, self.m_in = column, row, True
        p = self.props
        self.agenda = p.single("AgendaGroup")
        self.action = p.single("Action")
        self.hcond = p.single("HorizontalCondition")
        self.vcond = p.single("VerticalCondition")
        pre = p.single("Precondition")
        if pre is not None:
            self.precondition = pre


# ---------------------------------------------------------------- the CSV reader

def _csv_line(line: str) -> list:
    """CsvLineParser: quoted fields with "" escapes; a trailing separator adds no empty field."""
    if len(line) == 0:
        return [line]
    out, i, n = [], 0, len(line)
    while True:
        sb = []
        if i < n and line[i] == '"':
            i += 1
            j = i
            while j < n:
                if line[j] == '"' and j + 1 < n:
                    if line[j + 1] == '"':
                        j += 1
                    elif line[j + 1] == ",":
                        j += 1
                        break
                elif line[j] == '"' and j + 1 == n:
                    break
                sb.append(line[j])
                j += 1
            i = j
        else:
            j = line.find(",", i)
            if j == -1:
                sb.append(line[i:])
                i = n
            else:
                sb.append(line[i:j])
                i = j
        out.append("".join(sb))
        i += 1
        if not i < n:
            return out


def _lines(text: str) -> list:
    """BufferedReader.readLine over the whole text: \\n, \\r or \\r\\n end a line; no empty line after the last."""
    if text == "":
        return []
    parts = re.split(r"\r\n|\r|\n", text)
    if re.search(r"(\r\n|\r|\n)\Z", text):
        parts.pop()
    return parts


def _parse(text: str, listener: _RuleSheet, where: list):
    """Feed the CSV to the listener. `where` holds the cell being read, for the internal-error report."""
    for row, line in enumerate(_lines(text)):
        cells = _csv_line(line)
        where[:] = [row, None]
        listener.new_row(row, len(cells))
        start = NON_MERGED
        for col, cell in enumerate(cells):
            if cell.endswith("...") and start == NON_MERGED:
                start = col
            elif not cell.endswith("..."):
                start = NON_MERGED
            if start != NON_MERGED:
                cell = cell[:-3]
                if cell == "" and col != start:
                    listener.warnings.append(f"cell {_rc2name(row, col)} is a bare \"...\": Drools reads it as blank, "
                                             "so it merges nothing; repeat the text in every cell of the range "
                                             "(\"Person...,Person...\")")
            where[:] = [row, col]
            listener.new_cell(row, col, cell, start)
    where[:] = [None, None]
    listener.finish_sheet()


# ---------------------------------------------------------------- the public surface

class DecisionTable:
    """A compiled decision table: `drl` is Drools' text byte for byte; `rules` is what the engine takes."""

    def __init__(self, drl: str, package: _Package, source: Optional[str] = None, warnings: Optional[list] = None):
        self.drl = drl
        self._pkg = package
        self._source = source
        self.warnings: list = list(warnings or [])
        """Input Drools compiles but probably not as meant (a header read by its first letter, a bare "..." merge).
        The DRL is unaffected; the command line prints these to stderr."""

    @property
    def rules(self) -> str:
        """Engine DRL: the rules alone. Seine DRL is rules-only (fact types live in Python), so the package line and
        imports are dropped; globals, functions, queries, declares, a rule unit, the mvel dialect and ruleset-wide
        attributes have no Seine counterpart and are a CompileError rather than a silent drop."""
        try:
            return self._rules()
        except DecisionTableError as e:
            e.source = self._source
            raise

    def _rules(self) -> str:
        p = self._pkg
        for what, present in (("Variables (globals)", p.globals), ("Functions", p.functions),
                              ("Queries", p.queries), ("Declare", p.declares), ("Unit", p.unit)):
            if present:
                raise DecisionTableError(f"{what} has no Seine counterpart (Seine DRL is rules-only; fact types "
                                         "live in Python)")
        if p.dialect is not None and p.dialect.lower() != "java":
            raise DecisionTableError(f'dialect "{p.dialect}" is outside the subset (java only)')
        if len(p.attr):
            raise DecisionTableError("ruleset-wide attributes (" + ", ".join(p.attr.keys())
                                     + ") have no rules-only DRL form; put them in a column instead")
        out: list = []
        for r in p.rules:
            r.render(out)
        return "".join(line + "\n" for line in out)

    def __str__(self):
        return self.drl

    def __repr__(self):
        return f"<DecisionTable: {len(self._pkg.rules)} rules>"


def decision_table(path: "str | os.PathLike | None" = None, *, text: "str | bytes | None" = None,
                   matrix: bool = False, trim_cells: bool = True) -> DecisionTable:
    """Compile a CSV decision table (a file `path`, or the CSV `text`) the way Drools' SpreadsheetCompiler does.

    matrix=True reads a RuleMatrix (RuleMatrixSheetListener) instead of a RuleTable. trim_cells mirrors
    SpreadsheetCompiler(trimCell). Any table that cannot be compiled, and any file that cannot be read, raises
    DecisionTableError and nothing else: Drools' own message where Drools rejects the table, a Seine message naming
    the cell where Drools would crash or silently build wrong rules."""
    if (path is None) == (text is None):
        raise TypeError("decision_table: give exactly one of path or text=")
    if matrix and not trim_cells:
        raise TypeError("decision_table: trim_cells applies to RuleTables (RuleMatrixSheetListener always trims)")
    source = None
    if path is not None:
        source = os.fspath(path)
        try:
            with open(source, "rb") as f:
                text = f.read()
        except OSError as e:
            err = DecisionTableError(f"cannot read the table: {e.strerror or e}")
            err.source = source
            raise err from None
    if isinstance(text, (bytes, bytearray)):
        text = bytes(text).decode("utf-8", errors="replace")   # InputStreamReader(UTF-8): a BOM stays a character
    if not isinstance(text, str):
        raise TypeError(f"decision_table: text= must be str or bytes, not {type(text).__name__}")
    where: list = [None, None]
    try:
        listener = _MatrixSheet() if matrix else _RuleSheet(True, trim_cells)
        _parse(text, listener, where)
        pkg = listener.rule_set()
        out: list = []
        pkg.render(out)
        return DecisionTable("".join(line + "\n" for line in out), pkg, source, listener.warnings)
    except DecisionTableError as e:
        e.source = source
        raise
    except Exception as e:   # a defect in the port: still a clean, located error, never a stray exception
        row, col = where
        at = ("" if row is None else f" at {_rc2name(row, col)}" if col is not None else f" at row {row + 1}")
        err = DecisionTableError(f"internal error{at} ({type(e).__name__}: {e}); please report this table to the "
                                 "Seine maintainers", cell=None if row is None or col is None else _rc2name(row, col))
        err.source = source
        raise err from e


def main(argv: Optional[list] = None) -> int:
    """`python -m seine_rs table [--matrix] [--rules] TABLE.csv`: print the DRL (Drools' text, or with --rules the
    engine DRL). On an error, print `TABLE.csv: message` to stderr and exit 1; on bad usage, exit 2."""
    import argparse
    import sys
    ap = argparse.ArgumentParser(prog="python -m seine_rs table",
                                 description="Compile a Drools CSV decision table to DRL.")
    ap.add_argument("table", help="the CSV file")
    ap.add_argument("--matrix", action="store_true", help="read a RuleMatrix instead of a RuleTable")
    ap.add_argument("--rules", action="store_true", help="print the engine DRL (no package/import header)")
    ap.add_argument("--no-trim", action="store_true", help="keep cell padding (SpreadsheetCompiler(false))")
    a = ap.parse_args(argv)
    try:
        t = decision_table(a.table, matrix=a.matrix, trim_cells=not a.no_trim)
        out = t.rules if a.rules else t.drl
        for w in t.warnings:
            print(f"warning: {a.table}: {w}", file=sys.stderr)
        sys.stdout.write(out)
        return 0
    except DecisionTableError as e:
        if e.source is None:
            e.source = a.table
        print(f"error: {e}", file=sys.stderr)
        return 1
    except TypeError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2

