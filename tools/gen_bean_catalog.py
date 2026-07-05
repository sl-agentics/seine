#!/usr/bin/env python3
"""Generate bean_catalog.json entries by parsing the simple POJO model beans
in the Drools test suite (read-only). Conservative: anything not a scalar
subset type maps to OUT:<javatype>; constructors that do more than
`this.f = param` / super() are dropped.

Usage: gen_bean_catalog.py OUT.json DIR [DIR...]
"""
import json
import os
import re
import sys

SCALAR = {
    'int': ('i64', 0), 'long': ('i64', 0), 'short': ('i64', 0), 'byte': ('i64', 0),
    'Integer': ('i64', None), 'Long': ('i64', None),
    'double': ('f64', 0.0), 'float': ('f64', 0.0),
    'Double': ('f64', None), 'Float': ('f64', None),
    'boolean': ('bool', False), 'Boolean': ('bool', None),
    'String': ('String', None),
}

FIELD_RE = re.compile(
    r'^\s*(?:private|protected|public)\s+(?:final\s+)?([\w.<>\[\],\s]+?)\s+(\w+)\s*(?:=\s*([^;]+))?;\s*$',
    re.M)


def strip_comments(src):
    src = re.sub(r'/\*.*?\*/', '', src, flags=re.S)
    src = re.sub(r'//[^\n]*', '', src)
    return src


def parse_literal_default(text, kind):
    text = text.strip()
    if kind == 'String' and text.startswith('"') and text.endswith('"'):
        return text[1:-1]
    if kind == 'bool' and text in ('true', 'false'):
        return text == 'true'
    if kind == 'i64':
        try:
            return int(text.rstrip('lL'))
        except ValueError:
            return None
    if kind == 'f64':
        try:
            return float(text.rstrip('dDfF'))
        except ValueError:
            return None
    return None


def find_matching(src, open_idx, open_c='{', close_c='}'):
    depth = 0
    for i in range(open_idx, len(src)):
        if src[i] == open_c:
            depth += 1
        elif src[i] == close_c:
            depth -= 1
            if depth == 0:
                return i
    return -1


def parse_bean(path, pkg):
    src = strip_comments(open(path).read())
    name = os.path.basename(path)[:-5]
    m = re.search(r'\bclass\s+' + name + r'\b', src)
    if not m or re.search(r'\b(abstract|interface|enum)\s+(class\s+)?' + name + r'\b', src):
        if not m:
            return None
    fields = {}
    order = []
    for fm in FIELD_RE.finditer(src):
        jtype, fname, init = fm.group(1).strip(), fm.group(2), fm.group(3)
        if 'static' in jtype or jtype.startswith('class'):
            continue
        if fname in fields:
            continue
        if jtype in SCALAR:
            kind, default = SCALAR[jtype]
            spec = {'type': kind}
            if init is not None:
                lit = parse_literal_default(init, kind)
                if lit is not None:
                    spec['default'] = lit
                # non-literal initializer on a scalar: leave nullable
            elif default is not None:
                spec['default'] = default
        else:
            spec = {'type': 'OUT:' + re.sub(r'\s+', '', jtype)}
        fields[fname] = spec
        order.append(fname)
    if not order:
        return None

    raw_ctors = []      # (pnames, body)
    for cm in re.finditer(r'public\s+' + name + r'\s*\(([^)]*)\)\s*\{', src):
        params = [p.strip() for p in re.sub(r'\s+', ' ', cm.group(1)).split(',') if p.strip()]
        pnames = []
        ok = True
        for p in params:
            pm = re.match(r'(?:final\s+)?([\w.<>\[\]]+)\s+(\w+)$', p)
            if not pm:
                ok = False
                break
            pnames.append(pm.group(2))
        if not ok:
            continue
        body_end = find_matching(src, src.index('{', cm.end() - 1))
        raw_ctors.append((pnames, src[cm.end():body_end]))

    def solve(pnames, body, depth=0):
        """Return {'params': [field per param], 'presets': {field: literal}}
        or None."""
        if depth > 3:
            return None
        assigns = {}
        presets = {}
        for stmt in body.split(';'):
            stmt = stmt.strip()
            if not stmt or stmt == 'super()':
                continue
            dm = re.match(r'this\s*\(([^)]*)\)$', stmt)
            if dm:
                dargs = [a.strip() for a in dm.group(1).split(',')] if dm.group(1).strip() else []
                solved = None
                for rc in raw_ctors:
                    if len(rc[0]) != len(dargs) or rc[0] == pnames:
                        continue
                    sub = solve(rc[0], rc[1], depth + 1)
                    if sub is None:
                        continue
                    t_assigns = {}
                    t_presets = dict(sub['presets'])
                    ok2 = True
                    for f, arg in zip(sub['params'], dargs):
                        if arg in pnames:
                            t_assigns[arg] = f
                        elif arg == 'null':
                            pass
                        else:
                            kind = fields.get(f, {}).get('type')
                            lit = parse_literal_default(arg, kind) if kind else None
                            if lit is None:
                                ok2 = False
                                break
                            t_presets[f] = lit
                    if ok2:
                        solved = (t_assigns, t_presets)
                        break
                if solved is None:
                    return None
                assigns.update(solved[0])
                presets.update(solved[1])
                continue
            am = re.match(r'this\s*\.\s*(\w+)\s*=\s*(\w+)$', stmt)
            if am and am.group(2) in pnames:
                assigns[am.group(2)] = am.group(1)
                continue
            am = re.match(r'(\w+)\s*=\s*(\w+)$', stmt)
            if am and am.group(2) in pnames and am.group(1) in fields:
                assigns[am.group(2)] = am.group(1)
                continue
            am = re.match(r'(?:this\s*\.\s*)?set(\w+)\s*\(\s*(\w+)\s*\)$', stmt)
            if am and am.group(2) in pnames:
                f = am.group(1)[0].lower() + am.group(1)[1:]
                assigns[am.group(2)] = f
                continue
            return None
        if len(assigns) != len(pnames):
            return None
        params_fields = [assigns[p] for p in pnames]
        if not all(f in fields for f in params_fields):
            return None
        if not all(f in fields for f in presets):
            return None
        return {'params': params_fields, 'presets': presets}

    ctors = []
    for pnames, body in raw_ctors:
        r = solve(pnames, body)
        if r is not None:
            ctors.append(r['params'] if not r['presets'] else r)
    if not raw_ctors:
        ctors.append([])    # implicit default constructor

    return pkg + '.' + name, {'fields': {f: fields[f] for f in order}, 'ctors': ctors}


def pkg_of(path):
    src = open(path).read()
    m = re.search(r'^package\s+([\w.]+);', src, re.M)
    return m.group(1) if m else ''


def main():
    out_path = sys.argv[1]
    catalog = {}
    for d in sys.argv[2:]:
        for fn in sorted(os.listdir(d)):
            if not fn.endswith('.java') or fn.endswith('Test.java'):
                continue
            path = os.path.join(d, fn)
            try:
                r = parse_bean(path, pkg_of(path))
            except Exception as e:
                print(f'  ! {fn}: {e}', file=sys.stderr)
                continue
            if r:
                catalog[r[0]] = r[1]
    with open(out_path, 'w') as f:
        json.dump(catalog, f, indent=2)
        f.write('\n')
    print(f'{len(catalog)} beans -> {out_path}')


if __name__ == '__main__':
    main()
