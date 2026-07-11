"""Delta-minimize a divergent scenario while a KEYED divergence persists.

The D-165/D-167 recon workhorse, previously rebuilt per session (the
tjupd-ledger handoff's trap list) — now committed. Like minimize.py,
but the divergence predicate additionally requires a KEY literal in the
diff output, so minimization can't drift to a different divergence:
  - an ORDER witness pins its rule:      KEY = the rule name
  - a SET witness pins the count class:  KEY = "firing count differs"
Rule splitting handles the fuzz generators' UNQUOTED rule names.

Usage: python3 tools/minimize_keyed.py <scenario.json> <KEY> [out.json]
"""
import json, subprocess, re, sys, copy, os, tempfile

BASE = sys.argv[1]
KEY = sys.argv[2]
OUT = sys.argv[3] if len(sys.argv) > 3 else 'target/min_case.json'
TMP = os.path.join(tempfile.gettempdir(), f'seine_min_{os.getpid()}.json')

def diverges(d):
    d = copy.deepcopy(d); d['name'] = 'min_case'
    json.dump(d, open(TMP, 'w'))
    try:
        r = subprocess.run(['cargo','run','-q','-p','seine-harness','--','diff',TMP],
                           capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return False
    out = r.stdout
    return 'FAIL' in out and 'errored' not in out and KEY in out

d = json.load(open(BASE))
assert diverges(d), 'baseline must diverge with the key present'

def split_rules(drl):
    parts = re.split(r'(?=rule )', drl)
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
    # drop whole rules
    rules = split_rules(d['drl'])
    if len(rules) > 1:
        for i in range(len(rules)):
            cand = copy.deepcopy(d)
            cand['drl'] = ''.join(rules[:i] + rules[i+1:])
            if try_variant(cand):
                changed = True
                break
        if changed:
            continue
    # drop initial facts (re-target epoch actions by index shift is NOT
    # attempted — only trailing/unreferenced facts drop safely, so drop
    # a fact only when no action targets at-or-beyond it)
    for i in range(len(d.get('facts', [])) - 1, -1, -1):
        refd = any(a.get('target', -1) >= i
                   for ep in d.get('epochs', []) for a in ep.get('actions', []))
        if refd:
            continue
        cand = copy.deepcopy(d)
        del cand['facts'][i]
        if try_variant(cand):
            changed = True
            break
    if changed:
        continue
    # drop whole epochs (from the end first)
    for i in range(len(d.get('epochs', [])) - 1, -1, -1):
        cand = copy.deepcopy(d)
        del cand['epochs'][i]
        if try_variant(cand):
            changed = True
            break
    if changed:
        continue
    # drop single epoch actions / epoch facts
    for ei in range(len(d.get('epochs', []))):
        for ai in range(len(d['epochs'][ei].get('actions', [])) - 1, -1, -1):
            cand = copy.deepcopy(d)
            del cand['epochs'][ei]['actions'][ai]
            if try_variant(cand):
                changed = True
                break
        if changed:
            break
        for fi in range(len(d['epochs'][ei].get('facts', [])) - 1, -1, -1):
            cand = copy.deepcopy(d)
            del cand['epochs'][ei]['facts'][fi]
            if try_variant(cand):
                changed = True
                break
        if changed:
            break

json.dump(d, open(OUT, 'w'), indent=1)
print(json.dumps(d, indent=1))
print(f'-> {OUT}', file=sys.stderr)
