import json, subprocess, re, sys, copy, os

BASE = sys.argv[1] if len(sys.argv) > 1 else 'xfail/fz_42_4373.json'
TMP = '/tmp/claude-1000/-home-bryan-rust-rules/b62fbc2b-c6af-47e8-b5cf-b8d205378766/scratchpad/min_case.json'

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

changed = True
while changed:
    changed = False
    rules = split_rules(d['drl'])
    for i in range(len(rules)):
        cand = copy.deepcopy(d)
        cand['drl'] = ''.join(rules[:i] + rules[i+1:])
        if diverges(cand):
            d = cand; changed = True
            print(f'removed rule {i}, {len(rules)-1} left', flush=True)
            break
    if changed: continue
    for i in range(len(d['facts'])):
        cand = copy.deepcopy(d)
        del cand['facts'][i]
        if diverges(cand):
            d = cand; changed = True
            print(f'removed fact {i}, {len(d["facts"])} left', flush=True)
            break

json.dump(d, open(TMP,'w'), indent=1)
print('=== MINIMIZED ===')
print(d['drl'])
for i,f in enumerate(d['facts']): print(i, f['type'], f['fields'])
