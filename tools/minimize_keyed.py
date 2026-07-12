"""Delta-minimize a divergent scenario while a KEYED divergence persists.

The D-165/D-167 recon workhorse, previously rebuilt per session (the
tjupd-ledger handoff's trap list) — now committed. Like minimize.py,
but the divergence predicate additionally pins the FAILURE SIGNATURE:
each KEY literal must appear in the diff output (or must NOT appear,
when prefixed with '!'), so minimization can't drift to a different
divergence class:
  - an ORDER witness pins its rule:      KEY = the rule name
  - a SET witness pins the count class:  KEY = "firing count differs"
  - an ORDER-class witness excludes SET: KEY = '!firing count differs'
Rule splitting handles the fuzz generators' UNQUOTED rule names.

Usage: python3 tools/minimize_keyed.py [--errored] <scenario.json> <KEY[,KEY...]> [out.json]

--errored (the HANG/guard-trip variant, D-117 class): the predicate
becomes "engine errored but oracle succeeded" INSTEAD of excluding
errored runs — without it the default predicate rejects the baseline
(loud assert), and hand-flipping the wrong clause would silently
reduce a spin witness to nothing. Run under a low SEINE_SPIN_GUARD so
each diverging variant costs ms, not ~18s; re-verify the final
artifact at the DEFAULT guard before trusting it.
"""
import json, subprocess, re, sys, copy, os, tempfile

argv = [a for a in sys.argv[1:] if a != '--errored']
ERRORED = len(argv) != len(sys.argv) - 1
BASE = argv[0]
KEYS = argv[1].split(',')
OUT = argv[2] if len(argv) > 2 else 'target/min_case.json'
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
    if ERRORED:
        if 'engine errored but oracle succeeded' not in out:
            return False
    elif 'FAIL' not in out or 'errored' in out:
        return False
    for k in KEYS:
        if k.startswith('!'):
            if k[1:] in out:
                return False
        elif k not in out:
            return False
    return True

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
