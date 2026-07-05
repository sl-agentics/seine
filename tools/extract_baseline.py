#!/usr/bin/env python3
"""Extract DRL-behavior tests from the Drools 9.44.0.Final regression suite
into Seine differential scenarios (the BASELINE corpus tier).

Reads Java test sources (read-only; Apache-2.0 — see NOTICE), recognizes the
stereotyped shape

    String drl = "..." + X.class.getCanonicalName() + "...";
    KieSession ksession = ...;
    ksession.insert(new Bean(...));
    int fired = ksession.fireAllRules();
    assertThat(fired).isEqualTo(N);

and emits scenario JSON files (types/facts/drl[/epochs][/queries]) with
provenance. The Seine parser is the subset arbiter: extraction does NOT
decide in/out of subset — every extracted candidate is routed later by
`seine-harness run` (parse error => out-of-subset with reason).

Faithfulness rules:
  * The DRL is munged only by (a) dropping package/import/global/dialect
    lines, (b) stripping WM-INERT consequence statements (println, local
    var decls, calls on global collections, static counters). Anything
    else is left verbatim for the engine parser to accept/reject.
  * Facts translate through a hand-curated bean catalog (constructor
    signatures, field defaults). A DRL-referenced field left at Java
    default null routes the method OUT (subset has no nulls).
  * The JUnit-expected fireAllRules() count is recorded in provenance;
    the harness cross-checks it against the ORACLE's firing count later —
    a mismatch means translation drift, and the case is quarantined.

Usage:
  extract_baseline.py --catalog tools/bean_catalog.json \
      --out scenarios/baseline --routes /tmp/routes.tsv FILE.java...
"""

import argparse
import json
import os
import re
import sys

# ---------------------------------------------------------------- Java lexing

def strip_comments(src: str) -> str:
    out = []
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == '"':
            j = i + 1
            while j < n:
                if src[j] == '\\':
                    j += 2
                    continue
                if src[j] == '"':
                    break
                j += 1
            out.append(src[i:j + 1])
            i = j + 1
        elif src.startswith('//', i):
            j = src.find('\n', i)
            i = n if j < 0 else j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            i = n if j < 0 else j + 2
        else:
            out.append(c)
            i += 1
    return ''.join(out)


def java_unescape(lit: str) -> str:
    """Unescape the CONTENTS of a Java string literal."""
    out = []
    i, n = 0, len(lit)
    while i < n:
        c = lit[i]
        if c == '\\' and i + 1 < n:
            nxt = lit[i + 1]
            mapping = {'n': '\n', 't': '\t', 'r': '\r', '"': '"', "'": "'",
                       '\\': '\\', 'b': '\b', 'f': '\f', '0': '\0'}
            if nxt in mapping:
                out.append(mapping[nxt])
                i += 2
                continue
            if nxt == 'u' and i + 6 <= n:
                out.append(chr(int(lit[i + 2:i + 6], 16)))
                i += 6
                continue
        out.append(c)
        i += 1
    return ''.join(out)


def split_statements(body: str):
    """Split a method body into top-level statements (string-aware,
    depth-aware for parens/braces). Keeps `for`/`try`/`if` blocks as single
    statements so we can recognize and reject/ignore them wholesale."""
    stmts = []
    cur = []
    depth_par = depth_brace = 0
    i, n = 0, len(body)
    while i < n:
        c = body[i]
        if c == '"':
            j = i + 1
            while j < n:
                if body[j] == '\\':
                    j += 2
                    continue
                if body[j] == '"':
                    break
                j += 1
            cur.append(body[i:j + 1])
            i = j + 1
            continue
        if c == '(':
            depth_par += 1
        elif c == ')':
            depth_par -= 1
        elif c == '{':
            depth_brace += 1
        elif c == '}':
            depth_brace -= 1
            if depth_brace == 0 and depth_par == 0:
                cur.append(c)
                s = ''.join(cur).strip()
                if s:
                    stmts.append(s)
                cur = []
                i += 1
                continue
        elif c == ';' and depth_par == 0 and depth_brace == 0:
            s = ''.join(cur).strip()
            if s:
                stmts.append(s + ';')
            cur = []
            i += 1
            continue
        cur.append(c)
        i += 1
    tail = ''.join(cur).strip()
    if tail:
        stmts.append(tail)
    return stmts


def find_matching_brace(src: str, open_idx: int) -> int:
    depth = 0
    i, n = open_idx, len(src)
    while i < n:
        c = src[i]
        if c == '"':
            i += 1
            while i < n:
                if src[i] == '\\':
                    i += 2
                    continue
                if src[i] == '"':
                    break
                i += 1
        elif c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1

# ------------------------------------------------------------- test scanning

TEST_RE = re.compile(
    r'@Test(?:\s*\(\s*\))?\s+(?:public\s+)?void\s+(\w+)\s*\(\s*\)'
    r'(?:\s+throws\s+[\w.,\s]+)?\s*\{')
IMPORT_RE = re.compile(r'^import\s+(?:static\s+)?([\w.]+);', re.M)


def parse_imports(src: str):
    imports = {}
    for m in IMPORT_RE.finditer(src):
        fq = m.group(1)
        imports[fq.rsplit('.', 1)[-1]] = fq
    return imports

# --------------------------------------------------- DRL expression resolver

STR_LIT = re.compile(r'"((?:[^"\\]|\\.)*)"')


def resolve_string_expr(expr: str, imports, str_vars):
    """Resolve a Java string-concatenation expression to text.
    Handles literals, X.class.getCanonicalName()/getName(), and references
    to previously resolved String variables. Returns None if any fragment
    is not statically resolvable."""
    parts = []
    i, n = 0, len(expr)
    while i < n:
        c = expr[i]
        if c in ' \t\n\r+':
            i += 1
            continue
        if c == '"':
            m = STR_LIT.match(expr, i)
            if not m:
                return None
            parts.append(java_unescape(m.group(1)))
            i = m.end()
            continue
        m = re.match(r'([A-Za-z_][\w.]*)\s*\.class\s*\.\s*get(?:Canonical)?Name\s*\(\s*\)', expr[i:])
        if m:
            cls = m.group(1)
            parts.append(imports.get(cls, cls))
            i += m.end()
            continue
        m = re.match(r'[A-Za-z_]\w*', expr[i:])
        if m and m.group(0) in str_vars:
            parts.append(str_vars[m.group(0)])
            i += m.end()
            continue
        return None
    return ''.join(parts)

# ----------------------------------------------------------------- bean model

class Catalog:
    """Bean catalog keyed by FQCN (Person exists in several test packages
    with different shapes). A per-file VIEW resolves simple names through
    that file's imports."""

    def __init__(self, path):
        with open(path) as f:
            self.by_fqcn = json.load(f)

    def view(self, imports):
        v = {}
        for simple, fq in imports.items():
            if fq in self.by_fqcn:
                info = dict(self.by_fqcn[fq])
                info['name'] = simple
                v[simple] = info
        return v


NUM_RE = re.compile(r'^-?\d+$')
DEC_RE = re.compile(r'^-?\d*\.\d+[dDfF]?$|^-?\d+[dDfF]$')


def parse_java_literal(tok: str):
    """Return (kind, value) or None. kind in i64/f64/String/bool."""
    tok = tok.strip()
    if tok.startswith('"') and tok.endswith('"'):
        return ('String', java_unescape(tok[1:-1]))
    if tok in ('true', 'false'):
        return ('bool', tok == 'true')
    if NUM_RE.match(tok.rstrip('lL')):
        return ('i64', int(tok.rstrip('lL')))
    if DEC_RE.match(tok):
        return ('f64', float(tok.rstrip('dDfF')))
    return None


def split_args(argstr: str):
    args = []
    cur = []
    depth = 0
    i, n = 0, len(argstr)
    while i < n:
        c = argstr[i]
        if c == '"':
            j = i + 1
            while j < n:
                if argstr[j] == '\\':
                    j += 2
                    continue
                if argstr[j] == '"':
                    break
                j += 1
            cur.append(argstr[i:j + 1])
            i = j + 1
            continue
        if c in '(<[':
            depth += 1
        elif c in ')>]':
            depth -= 1
        elif c == ',' and depth == 0:
            args.append(''.join(cur).strip())
            cur = []
            i += 1
            continue
        cur.append(c)
        i += 1
    tail = ''.join(cur).strip()
    if tail:
        args.append(tail)
    return args

# -------------------------------------------------------------- DRL munging

GLOBAL_LINE = re.compile(r'^\s*global\s+([\w.<>\[\]]+)\s+(\w+)\s*;?\s*$')
DECLARE_BLOCK = re.compile(r'\bdeclare\s+(?:enum\s+)?([\w.]+)(.*?)\bend\b', re.S)
DECL_FIELD = re.compile(r'^\s*(\w+)\s*:\s*([\w.<>\[\]]+)\s*(@[^\n]*)?$')
DECL_TYPE_MAP = {
    'int': 'i64', 'Integer': 'i64', 'long': 'i64', 'Long': 'i64',
    'short': 'i64', 'byte': 'i64',
    'double': 'f64', 'Double': 'f64', 'float': 'f64', 'Float': 'f64',
    'String': 'String', 'boolean': 'bool', 'Boolean': 'bool',
}


def lift_declares(drl):
    """Extract inline scalar `declare` blocks into scenario type schemas.
    Returns (drl_without_declares, [type dicts], notes, disqualify|None)."""
    types = []
    notes = []
    out = drl
    for m in DECLARE_BLOCK.finditer(drl):
        tname, body = m.group(1), m.group(2)
        if 'enum' in drl[m.start():m.start() + 20]:
            return None, None, None, 'declare-enum'
        if '.' in tname:
            return None, None, None, 'declare-fqcn-redeclare'
        if re.search(r'\bextends\b', body) or '@' in body:
            return None, None, None, 'declare-annotations-or-extends'
        fields = []
        for ln in body.strip().split('\n'):
            ln = ln.strip()
            if not ln:
                continue
            fm = DECL_FIELD.match(ln)
            if not fm or fm.group(3):
                return None, None, None, 'declare-field-unparsed'
            jt = fm.group(2)
            if jt not in DECL_TYPE_MAP:
                return None, None, None, f'declare-type-out:{jt}'
            if DECL_TYPE_MAP[jt] != jt and jt not in ('boolean', 'String'):
                notes.append(f'declare {tname}.{fm.group(1)}: {jt} -> {DECL_TYPE_MAP[jt]}')
            fields.append({'name': fm.group(1), 'type': DECL_TYPE_MAP[jt]})
        types.append({'name': tname, 'fields': fields})
        out = out.replace(m.group(0), '')
    return out, types, notes, None
DROP_LINE = re.compile(r'^\s*(package|import)\b')
DIALECT_RE = re.compile(r'dialect\s+"(mvel|java)"')


def munge_drl(drl: str):
    """Drop package/import/global lines; strip inert RHS statements.
    Returns (munged_drl, globals_map, notes, disqualify_reason|None)."""
    globals_map = {}
    notes = []
    if re.search(r'dialect\s+"mvel"', drl):
        return None, None, None, 'dialect-mvel'
    # token-based removal: package/import/global statements may share a
    # line with rule text (space-joined DRL strings are common upstream)
    munged = re.sub(r'\bpackage\s+[\w.]+\s*;?', '', drl)
    munged = re.sub(r'\bimport\s+(?:static\s+)?[\w.*]+\s*;?', '', munged)

    def _global(m):
        globals_map[m.group(2)] = m.group(1)
        notes.append(f'dropped global {m.group(1)} {m.group(2)}')
        return ''

    munged = re.sub(r'\bglobal\s+([\w.<>\[\]]+)\s+(\w+)\s*;?', _global, munged)

    # Consequence munging: for each rule body between 'then' and 'end',
    # strip whitelisted-inert statements.
    out = []
    pos = 0
    pat = re.compile(r'\bthen\b(.*?)\bend\b', re.S)
    for m in pat.finditer(munged):
        rhs = m.group(1)
        new_rhs, rhs_notes, disq = munge_rhs(rhs, globals_map)
        if disq:
            return None, None, None, disq
        notes.extend(rhs_notes)
        out.append(munged[pos:m.start(1)])
        out.append(new_rhs)
        pos = m.end(1)
    out.append(munged[pos:])
    munged = ''.join(out)

    if globals_map:
        # a surviving reference to a dropped global => not translatable
        for g in globals_map:
            if re.search(r'\b' + re.escape(g) + r'\b', munged):
                return None, None, None, 'global-in-lhs-or-complex-rhs'
    return munged, globals_map, notes, None


INERT_STMT = [
    re.compile(r'^System\s*\.\s*(out|err)\s*\.\s*print', re.S),
    # local declaration with a pure-literal/binding initializer
    re.compile(r'^(final\s+)?(boolean|int|long|double|float|String|Object)\s+\w+\s*=[^;]*$', re.S),
]


def munge_rhs(rhs: str, globals_map):
    """Strip inert statements from a consequence. Returns
    (new_rhs, notes, disqualify_reason|None)."""
    notes = []
    stmts = split_statements(rhs)
    kept = []
    for s in stmts:
        body = s.rstrip(';').strip()
        if not body:
            continue
        # calls on a dropped global collection: list.add(x), results.clear()
        gm = re.match(r'^(\w+)\s*\.\s*\w+\s*\(', body)
        if gm and gm.group(1) in globals_map:
            notes.append(f'stripped global stmt: {body[:40]}')
            continue
        if any(p.match(body) for p in INERT_STMT):
            notes.append(f'stripped inert stmt: {body[:40]}')
            continue
        kept.append(body + ';')
    return ' ' + ' '.join(kept) + ' ', notes, None

# ------------------------------------------------------------- method model

class MethodExtract:
    def __init__(self, cls, method):
        self.cls = cls
        self.method = method
        self.drl = None
        self.inserts = []          # list of (bean, {field: (kind, value)})
        self.fire_calls = []       # list of expected-count-or-None
        self.queries = []          # list of {"call":..,"args":[..]}
        self.skip = None           # reason tag => routed out
        self.notes = []

# The statement scanner: recognize or reject.
NEW_BEAN = re.compile(r'^(?:final\s+)?([A-Z]\w*)\s+(\w+)\s*=\s*new\s+([A-Z]\w*)\s*\((.*)\)\s*;?$', re.S)
SETTER = re.compile(r'^(\w+)\s*\.\s*set([A-Z]\w*)\s*\(\s*(.+?)\s*\)\s*;?$', re.S)
INSERT_VAR = re.compile(r'^(?:\w+\s*=\s*)?(\w+)\s*\.\s*insert\s*\(\s*(\w+)\s*\)\s*;?$')
INSERT_NEW = re.compile(r'^(?:\w+\s*=\s*)?(\w+)\s*\.\s*insert\s*\(\s*new\s+([A-Z]\w*)\s*\((.*)\)\s*\)\s*;?$', re.S)
FIRE = re.compile(r'\bfireAllRules\s*\(\s*(\d*)\s*\)')
ASSERT_FIRE_THAT = re.compile(
    r'^assertThat\s*\(\s*(?:\w+\s*\.\s*)?fireAllRules\s*\(\s*\)\s*\)\s*\.\s*isEqualTo\s*\(\s*(\d+)\s*\)\s*;?$')
ASSERT_FIRE_EQ = re.compile(
    r'^assertEquals\s*\(\s*(\d+)\s*,\s*(?:\w+\s*\.\s*)?fireAllRules\s*\(\s*\)\s*\)\s*;?$')
ASSERT_EQ = re.compile(r'^assertThat\s*\(\s*(\w+)\s*\)\s*\.\s*(?:isEqualTo\s*\(\s*(\d+)\s*\)|isZero\s*\(\s*\))\s*;?$')
ASSERT_EQ_JUNIT = re.compile(r'^assertEquals\s*\(\s*(\d+)\s*,\s*(\w+)\s*\)\s*;?$')
INSERT_LITERAL = re.compile(r'^(\w+)\s*\.\s*insert\s*\(\s*("|new\s+(Integer|Long|Double|Float|Boolean|String|BigDecimal)|-?\d)')
QUERY_CALL = re.compile(r'\bgetQueryResults\s*\(\s*(.+)\)\s*;?$', re.S)

# statements that force a skip: behavior we can't (yet) translate
DISQUALIFIERS = [
    (re.compile(r'\bEntryPoint\b|\.getEntryPoint\s*\('), 'entry-points'),
    (re.compile(r'SessionPseudoClock|\.advanceTime\s*\('), 'clock'),
    (re.compile(r'\.update\s*\(|\.delete\s*\(|\.retract\s*\('), 'external-wm-api'),
    (re.compile(r'addEventListener|AgendaFilter|\.halt\s*\('), 'listener-api'),
    (re.compile(r'fireUntilHalt'), 'fire-until-halt'),
    (re.compile(r'Thread\b|CountDownLatch|Executor'), 'threads'),
    (re.compile(r'\bkContainer\b|KieContainer|KieFileSystem|ReleaseId'), 'kie-container-api'),
    (re.compile(r'InternalWorkingMemory|ReteDumper|\.getRete\b|InternalFactHandle|ObjectTypeNode'), 'engine-internals'),
    (re.compile(r'kieModuleConfigurationProperties|KieBaseConfiguration|KieSessionConfiguration|System\s*\.\s*setProperty'), 'kiebase-config'),
    (re.compile(r'SerializationHelper|getSerialisedStateful'), 'serialization'),
    (re.compile(r'\.getKieBase\s*\(\s*\)\s*\.\s*getFactType|\bFactType\b'), 'facttype-api'),
    (re.compile(r'\.getObjects\s*\(|\.getFactHandle'), 'wm-introspection'),
    (re.compile(r'for\s*\(|while\s*\('), 'loop-in-test'),
]

IGNORABLE = [
    re.compile(r'^(final\s+)?(KieBase|KieSession|KieServices|KieHelper|StatelessKieSession)\b'),
    re.compile(r'^(final\s+)?(KieBuilder|KieModule)\b'),
    re.compile(r'^\w+\s*\.\s*setGlobal\s*\('),
    re.compile(r'^(final\s+)?(List|ArrayList|Collection|Set|HashSet|Map|HashMap)\b[^=]*=\s*new\s+'),
    re.compile(r'^(final\s+)?(List|Collection)\s*<[^>]*>\s+\w+\s*=\s*\(?\s*(List|Collection)'),
    re.compile(r'^assert(That|Equals|True|False|Null|NotNull)\b'),
    re.compile(r'^fail\s*\('),
    re.compile(r'^(final\s+)?AgendaEventListener\s+\w+\s*=\s*mock\s*\('),
    re.compile(r'^\w+\s*\.\s*addEventListener\s*\(\s*\w+\s*\)'),
    re.compile(r'^verify\s*\(|^(final\s+)?ArgumentCaptor\b|^(final\s+)?InOrder\b'),
    re.compile(r'^Collections\s*\.'),
    re.compile(r'^(final\s+)?(int|long)\s+\w+\s*=\s*\w+\s*\.\s*fireAllRules'),
    re.compile(r'^assert\w*'),
    re.compile(r'^ksession\s*\.\s*dispose'),
    re.compile(r'^\w+\s*\.\s*dispose\s*\(\s*\)\s*;?$'),
    re.compile(r'^System\s*\.'),
    re.compile(r'^(final\s+)?String\s+\w+\s*='),   # drl strings handled separately
    re.compile(r'^logger\b|^LOG\b'),
]


def scan_method(cls, mname, body, imports, beans):
    """`beans` is the per-file catalog view: simple name -> bean info."""
    ex = MethodExtract(cls, mname)
    str_vars = {}
    objs = {}       # var -> (bean, fields dict)
    stmts = split_statements(body)

    # unwrap try/finally/catch/bare-block scaffolding into inner statements
    def unwrap(sts, depth=0):
        out = []
        for st in sts:
            m = re.match(r'^(?:try|finally|(?:catch\s*\([^)]*\)))\s*\{(.*)\}$', st, re.S) \
                or re.match(r'^\{(.*)\}$', st, re.S)
            if m and depth < 4:
                out.extend(unwrap(split_statements(m.group(1)), depth + 1))
            else:
                out.append(st)
        return out
    stmts = unwrap(stmts)

    full_text = body
    for rx, tag in DISQUALIFIERS:
        if rx.search(full_text):
            ex.skip = tag
            return ex

    pending_fire_var = None
    for s in stmts:
        body_s = s.rstrip(';').strip()
        if not body_s:
            continue

        # String var assignment (drl fragments)
        m = re.match(r'^(?:final\s+)?String\s+(\w+)\s*=\s*(.+)$', body_s, re.S)
        if m:
            val = resolve_string_expr(m.group(2), imports, str_vars)
            if val is None:
                ex.skip = 'drl-dynamic'
                return ex
            str_vars[m.group(1)] = val
            continue

        m = re.match(r'^(\w+)\s*\+=\s*(.+)$', body_s, re.S)
        if m and m.group(1) in str_vars:
            val = resolve_string_expr(m.group(2), imports, str_vars)
            if val is None:
                ex.skip = 'drl-dynamic'
                return ex
            str_vars[m.group(1)] += val
            continue

        m = NEW_BEAN.match(body_s)
        if m and m.group(1) == m.group(3):
            bean = m.group(3)
            info = beans.get(bean)
            if info is None:
                ex.skip = f'bean-unknown:{bean}'
                return ex
            fields, why = apply_ctor(info, split_args(m.group(4)))
            if fields is None:
                ex.skip = why
                return ex
            objs[m.group(2)] = (bean, fields)
            continue

        m = SETTER.match(body_s)
        if m and m.group(1) in objs:
            var, fname_uc, argstr = m.group(1), m.group(2), m.group(3)
            bean, fields = objs[var]
            info = beans[bean]
            fname = fname_uc[0].lower() + fname_uc[1:]
            if fname not in info['fields']:
                ex.skip = f'setter-unknown:{bean}.{fname}'
                return ex
            lit = parse_java_literal(argstr)
            if lit is None:
                ex.skip = f'setter-nonliteral:{bean}.{fname}'
                return ex
            fields[fname] = lit
            continue

        m = INSERT_VAR.match(body_s)
        if m and m.group(2) in objs:
            ex.inserts.append(objs[m.group(2)])
            continue

        m = INSERT_NEW.match(body_s)
        if m:
            bean = m.group(2)
            info = beans.get(bean)
            if info is None:
                ex.skip = f'bean-unknown:{bean}'
                return ex
            fields, why = apply_ctor(info, split_args(m.group(3)))
            if fields is None:
                ex.skip = why
                return ex
            ex.inserts.append((bean, fields))
            continue

        m = ASSERT_FIRE_THAT.match(body_s) or ASSERT_FIRE_EQ.match(body_s)
        if m:
            ex.fire_calls.append(int(m.group(1)))
            continue

        fm = FIRE.search(body_s)
        if fm:
            if fm.group(1):
                ex.skip = 'fire-limit-arg'
                return ex
            m2 = re.match(r'^(?:final\s+)?(?:int|long)\s+(\w+)\s*=', body_s)
            pending_fire_var = m2.group(1) if m2 else None
            ex.fire_calls.append(None)
            continue

        m = ASSERT_EQ.match(body_s)
        if m and pending_fire_var and m.group(1) == pending_fire_var and ex.fire_calls:
            ex.fire_calls[-1] = int(m.group(2)) if m.group(2) is not None else 0
            pending_fire_var = None
            continue

        m = ASSERT_EQ_JUNIT.match(body_s)
        if m and pending_fire_var and m.group(2) == pending_fire_var and ex.fire_calls:
            ex.fire_calls[-1] = int(m.group(1))
            pending_fire_var = None
            continue

        if INSERT_LITERAL.match(body_s):
            ex.skip = 'fact-nonbean'
            return ex

        if QUERY_CALL.search(body_s):
            ex.skip = 'query-api'   # v2: translate into scenario "queries"
            return ex

        if any(rx.match(body_s) for rx in IGNORABLE):
            continue

        ex.skip = f'stmt-unrecognized:{body_s[:60]}'
        return ex

    # locate the DRL among resolved strings: the one containing "rule" or "query"
    drls = [v for v in str_vars.values() if re.search(r'\b(rule|query)\b', v)]
    if not drls:
        ex.skip = 'no-drl-string'
        return ex
    if len(drls) > 1:
        ex.skip = 'multiple-drl-strings'
        return ex
    ex.drl = drls[0]
    if not ex.fire_calls:
        ex.skip = 'no-fire-call'
        return ex
    if len(ex.fire_calls) > 1:
        ex.skip = 'multi-fire'      # v2: epochs
        return ex
    return ex


def apply_ctor(info, args):
    """Match constructor by arity; map literal args onto fields.
    Returns (explicit fields dict, None) or (None, reason). Only fields the
    constructor actually SETS are recorded — defaults resolve at emission."""
    if args == ['']:
        args = []
    lits = [parse_java_literal(a) for a in args]
    if any(l is None for l in lits):
        return None, f'ctor-nonliteral:{info["name"]}'

    def norm(c):
        return (c['params'], c.get('presets', {})) if isinstance(c, dict) else (c, {})

    candidates = [norm(c) for c in info.get('ctors', [])]
    candidates = [c for c in candidates if len(c[0]) == len(args)]
    if not candidates:
        return None, f'ctor-arity:{info["name"]}/{len(args)}'

    def compatible(params):
        for fname, (kind, _) in zip(params, lits):
            spec = info['fields'].get(fname)
            if spec is None:
                return False
            ft = spec['type']
            if kind == ft or (kind == 'i64' and ft == 'f64'):
                continue
            return False
        return True

    matching = [c for c in candidates if compatible(c[0])]
    if not matching:
        return None, f'ctor-type:{info["name"]}/{len(args)}'
    params, presets = matching[0]
    fields = {}
    for fname, preset in presets.items():
        ft = info['fields'][fname]['type']
        fields[fname] = (ft, preset)
    for fname, lit in zip(params, lits):
        fields[fname] = lit
    return fields, None

# ------------------------------------------------------------ scenario emit

TYPE_MAP = {'i64': 'i64', 'f64': 'f64', 'String': 'String', 'bool': 'bool'}


def referenced_fields(drl: str, info):
    """Fields of this bean textually referenced in the DRL (field name or
    accessor form)."""
    refs = set()
    for f in info['fields']:
        acc = ('is' if info['fields'][f]['type'] == 'bool' else 'get') + f[0].upper() + f[1:]
        if re.search(r'\b' + re.escape(f) + r'\b', drl) or re.search(r'\b' + acc + r'\b', drl):
            refs.add(f)
    return refs


def build_scenario(ex, beans, name, provenance):
    munged, globals_map, notes, disq = munge_drl(ex.drl)
    if disq:
        return None, disq
    ex.notes.extend(notes)

    if not re.search(r'\b(rule|query)\b', munged):
        return None, 'munge-emptied-drl'

    declared_types = []
    if re.search(r'\bdeclare\b', munged):
        munged, declared_types, dnotes, disq = lift_declares(munged)
        if disq:
            return None, disq
        ex.notes.extend(dnotes)

    used_beans = []
    for bean, _ in ex.inserts:
        if bean not in used_beans:
            used_beans.append(bean)
    # types mentioned in DRL but never inserted still need declaring
    for bean in beans:
        if bean in used_beans:
            continue
        if re.search(r'\b' + re.escape(bean) + r'\s*\(', munged):
            used_beans.append(bean)

    types = []
    per_bean_fields = {}
    for bean in used_beans:
        info = beans.get(bean)
        if info is None:
            return None, f'bean-unknown:{bean}'
        refs = referenced_fields(munged, info)
        set_fields = set()
        for b, fields in ex.inserts:
            if b == bean:
                set_fields |= set(fields)
        chosen = [f for f in info['fields'] if f in (refs | set_fields)]
        for f in chosen:
            t = info['fields'][f]['type']
            if t not in TYPE_MAP:
                return None, f'field-type-out:{bean}.{f}:{t}'
        per_bean_fields[bean] = chosen
        types.append({'name': bean,
                      'fields': [{'name': f, 'type': info['fields'][f]['type']}
                                 for f in chosen]})

    facts = []
    for bean, fields in ex.inserts:
        info = beans[bean]
        out = {}
        for f in per_bean_fields[bean]:
            v = fields.get(f)
            if v is None:
                spec = info['fields'][f]
                if 'default' not in spec or spec['default'] is None:
                    return None, f'null-field:{bean}.{f}'
                v = (spec['type'], spec['default'])
            kind, val = v
            ft = info['fields'][f]['type']
            if kind == 'i64' and ft == 'f64':
                val = float(val)
            elif kind != ft:
                return None, f'field-type-mismatch:{bean}.{f}:{kind}->{ft}'
            out[f] = val
        facts.append({'type': bean, 'fields': out})

    all_types = declared_types + types
    seen = set()
    for t in all_types:
        if t['name'] in seen:
            return None, f'type-name-clash:{t["name"]}'
        seen.add(t['name'])
    scenario = {
        'name': name,
        'provenance': provenance,
        'types': all_types,
        'facts': facts,
        'drl': munged,
    }
    return scenario, None

# --------------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--catalog', required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--routes', required=True)
    ap.add_argument('--prefix', default='bl')
    ap.add_argument('files', nargs='+')
    args = ap.parse_args()

    catalog = Catalog(args.catalog)
    os.makedirs(args.out, exist_ok=True)
    routes = []
    n_extracted = 0

    for path in args.files:
        with open(path) as f:
            src = strip_comments(f.read())
        cls = os.path.basename(path)[:-5]
        imports = parse_imports(src)
        beans = catalog.view(imports)
        for m in TEST_RE.finditer(src):
            mname = m.group(1)
            open_brace = src.index('{', m.end() - 1)
            close = find_matching_brace(src, open_brace)
            if close < 0:
                routes.append((cls, mname, 'skip:body-parse'))
                continue
            body = src[open_brace + 1:close]
            ex = scan_method(cls, mname, body, imports, beans)
            if ex.skip:
                routes.append((cls, mname, f'skip:{ex.skip}'))
                continue
            name = f'{args.prefix}_{cls}_{mname}'
            provenance = {
                'source': f'drools-9.44.0.Final:{os.path.relpath(path, "/home/bryan/drools-9.44-src")}',
                'method': f'{cls}#{mname}',
                'expect_fire_count': ex.fire_calls[0],
                'adaptation': ex.notes,
            }
            scenario, disq = build_scenario(ex, beans, name, provenance)
            if disq:
                routes.append((cls, mname, f'skip:{disq}'))
                continue
            out_path = os.path.join(args.out, name + '.json')
            with open(out_path, 'w') as f:
                json.dump(scenario, f, indent=2)
                f.write('\n')
            routes.append((cls, mname, f'extracted:{out_path}'))
            n_extracted += 1

    with open(args.routes, 'w') as f:
        for cls, mname, route in routes:
            f.write(f'{cls}\t{mname}\t{route}\n')
    counts = {}
    for _, _, route in routes:
        key = route.split(':')[0] + (':' + route.split(':')[1] if route.startswith('skip') else '')
        counts[key] = counts.get(key, 0) + 1
    for k in sorted(counts, key=counts.get, reverse=True):
        print(f'{counts[k]:5d}  {k}')
    print(f'total methods: {len(routes)}, extracted: {n_extracted}')


if __name__ == '__main__':
    main()
