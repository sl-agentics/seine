"""Delta-minimize a divergent scenario while the divergence persists.

Usage: python3 tools/minimize.py <scenario.json> [out.json]
Drops whole rules, facts, single constraints, and single RHS statements,
iterating to a fixpoint. Writes the minimized case to out.json (default
target/min_case.json) and prints it.
"""
import json, subprocess, re, sys, copy, os, tempfile

BASE = sys.argv[1] if len(sys.argv) > 1 else 'scenarios/failures/case.json'
OUT = sys.argv[2] if len(sys.argv) > 2 else 'target/min_case.json'
TMP = os.path.join(tempfile.gettempdir(), f'seine_min_{os.getpid()}.json')

def diverges(d):
    d = copy.deepcopy(d); d['name'] = 'min_case'
    json.dump(d, open(TMP, 'w'))
    r = subprocess.run(['cargo','run','-q','-p','seine-harness','--','diff',TMP],
                       capture_output=True, text=True)
    out = r.stdout
    return 'FAIL' in out and 'errored' not in out

d = json.load(open(BASE))
assert diverges(d), 'baseline must diverge'

def split_rules(drl):
    parts = re.split(r'(?=rule ")', drl)
    return [p for p in parts if p.strip()]

def try_variant(cand):
    global d
    if diverges(cand):
        d = cand
        return True
    return False

changed = True
while changed:
    changed = False
    rules = split_rules(d['drl'])
    # drop a whole rule
    for i in range(len(rules)):
        cand = copy.deepcopy(d)
        cand['drl'] = ''.join(rules[:i] + rules[i+1:])
        if try_variant(cand):
            changed = True
            print(f'removed rule {i}, {len(rules)-1} left', flush=True)
            break
    if changed:
        continue
    # drop a fact
    for i in range(len(d['facts'])):
        cand = copy.deepcopy(d)
        del cand['facts'][i]
        if try_variant(cand):
            changed = True
            print(f'removed fact {i}, {len(d["facts"])} left', flush=True)
            break
    if changed:
        continue
    # drop a single constraint inside some pattern line
    lines = d['drl'].split('\n')
    done = False
    for li, line in enumerate(lines):
        m = re.match(r'^(\s*(?:not |exists )?(?:\$\w+ : )?\w+\()(.*)(\).*)$', line)
        if not m or not m.group(2).strip():
            continue
        cs = [c.strip() for c in m.group(2).split(',')]
        if len(cs) < 1:
            continue
        for ci in range(len(cs)):
            rest = cs[:ci] + cs[ci+1:]
            cand = copy.deepcopy(d)
            newline = m.group(1) + ', '.join(rest) + m.group(3)
            cand['drl'] = '\n'.join(lines[:li] + [newline] + lines[li+1:])
            if try_variant(cand):
                changed = done = True
                print(f'dropped constraint {ci} on line {li}', flush=True)
                break
        if done:
            break
    if changed:
        continue
    # drop a single RHS statement line
    for li, line in enumerate(lines):
        s = line.strip()
        if not (s.endswith(';') or (s.startswith('modify(') and s.endswith('}'))):
            continue
        cand = copy.deepcopy(d)
        cand['drl'] = '\n'.join(lines[:li] + lines[li+1:])
        if try_variant(cand):
            changed = True
            print(f'dropped RHS statement on line {li}: {s}', flush=True)
            break

json.dump(d, open(OUT, 'w'), indent=1)
print('=== MINIMIZED ===')
print(d['drl'])
for i, f in enumerate(d['facts']):
    print(i, f['type'], f['fields'])
print('written to', OUT)
