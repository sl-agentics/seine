#!/usr/bin/env python3
"""§3A validation: compare ENGINE vs ORACLE on FACTS (working-memory multiset)
only, over the not-temporal population. Firings are expected to diverge until
§3B (deferral) lands; this isolates the arc-B reaping port."""
import json, os, subprocess, sys, collections, importlib.util
REPO = "/home/bryan/rust-rules"
spec = importlib.util.spec_from_file_location("m", REPO + "/tools/model_not_infer.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

def facts_ms(res):
    if not res: return None
    return collections.Counter((f["type"], f["fields"]["ts"]) for f in res.get("facts", []))

def run(cmd, paths):
    env = dict(os.environ); env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo","run","-q","-p","seine-harness","--",cmd]+paths,
                       cwd=REPO, capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try: o = json.loads(line)
        except Exception: continue
        out[o["scenario"]] = facts_ms(o.get("result"))
    if not out: sys.stderr.write((r.stderr or "")[-1500:] + "\n")
    return out

def main():
    import random
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    OUT = "/tmp/seine_facts_check"; os.makedirs(OUT, exist_ok=True)
    rng = random.Random(seed)
    paths, scns = [], {}
    for i in range(n):
        s = m.gen(rng, f"fc{seed}x{i}")
        p = os.path.join(OUT, f"fc{seed}x{i}.json"); json.dump(s, open(p, "w"))
        paths.append(p); scns[s["name"]] = s
    eng = run("run", paths); ora = run("oracle", paths)
    ndiff = 0; err = 0
    for nm in scns:
        e, o = eng.get(nm), ora.get(nm)
        if e is None and o is None:  # both errored (shouldn't now)
            err += 1; continue
        if e != o:
            ndiff += 1
            if ndiff <= 12:
                exp = scns[nm]['types'][0]['event'].get('expires_ms','-')
                adv = bool(scns[nm]['epochs'])
                print(f"  FACTSDIV {nm}: engine={dict(e) if e else e} oracle={dict(o) if o else o}")
                print(f"      drl={scns[nm]['drl'].strip()}  exp={exp} adv={adv}")
                print(f"      facts_in={[(f['type'],f['fields']['ts']) for f in scns[nm]['facts']]}")
    print(f"FACTS engine-vs-oracle: {n} seed {seed}: {ndiff} divergences, {err} both-error ({100*ndiff//max(1,n)}%)")

if __name__ == "__main__":
    main()
