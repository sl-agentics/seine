#!/usr/bin/env python3
"""Stress the D-132 reaper fix: random POSITIVE temporal rules (no not/exists),
absent-or-large @expires, far-past + boundary timestamps, 0-2 advances. Compare
engine vs oracle FACTS (and firings) to catch any missed born-expired / at-insert
cases."""
import json, os, subprocess, sys, random, collections
REPO="/home/bryan/rust-rules"; OUT="/tmp/seine_posfz"; os.makedirs(OUT,exist_ok=True)
def et(n,exp):
    ev={"timestamp":"ts"}
    if exp: ev["expires_ms"]=exp
    return {"name":n,"fields":[{"name":"ts","type":"i64"}],"event":ev}
def res_of(rr):
    if not rr: return None
    fr=["-".join(str(d[f"E{i}"]) for i in sorted(int(k[1:]) for k in d)) for d in ({m["type"]:m["fields"]["ts"] for m in f["matches"]} for f in rr["firings"])]
    fac=sorted((f["type"],f["fields"]["ts"]) for f in rr.get("facts",[]))
    return (fr,fac)
def runcmd(cmd,paths):
    env=dict(os.environ); env["PATH"]=os.path.expanduser("~/.cargo/bin")+":"+env.get("PATH","")
    r=subprocess.run(["cargo","run","-q","-p","seine-harness","--",cmd]+paths,cwd=REPO,capture_output=True,text=True,env=env)
    R={}
    for ln in r.stdout.splitlines():
        try:o=json.loads(ln)
        except:continue
        R[o["scenario"]]=res_of(o.get("result"))
    return R
def gen(rng,nm):
    op=rng.choice(["after","before"]); hi=rng.choice([50,100,150])
    exp=rng.choice([None,None,100000])   # absent (inferred) or large
    drl=f"rule R when $a : E0() $b : E1(this {op}[0ms,{hi}ms] $a) then end\n"
    facts=[]
    for _ in range(rng.choice([1,1,2])):
        facts.append(("E0", rng.randint(-5,5)))
    base=facts[0][1]
    for _ in range(rng.randint(1,3)):
        d=rng.randint(0,hi+10)
        facts.append(("E1", base+d if op=="after" else base-d))
    # sometimes a deliberately far-past / boundary E1
    if rng.random()<0.5:
        facts.append(("E1", -(hi+1)+rng.choice([-30,-1,0,1])))
    rng.shuffle(facts)
    epochs=[]
    for _ in range(rng.randint(0,2)):
        epochs.append({"actions":[{"op":"advance","ms":rng.choice([1,50,1000])}],"facts":[]})
    return {"name":nm,"types":[et("E0",exp),et("E1",exp)],"drl":drl,
            "facts":[{"type":t,"fields":{"ts":v}} for t,v in facts],"epochs":epochs}
n,seed=int(sys.argv[1]),int(sys.argv[2]); rng=random.Random(seed)
paths,scns=[],{}
for i in range(n):
    s=gen(rng,f"pf{seed}x{i}"); p=os.path.join(OUT,f"pf{seed}x{i}.json"); json.dump(s,open(p,"w")); paths.append(p); scns[s["name"]]=s
eng=runcmd("run",paths); ora=runcmd("oracle",paths)
fd=ffr=0
for nm in scns:
    e,o=eng.get(nm),ora.get(nm)
    if e==o: continue
    ediff = (e[1] if e else None)!=(o[1] if o else None)
    frdiff = (e[0] if e else None)!=(o[0] if o else None)
    if ediff: fd+=1
    if frdiff: ffr+=1
    if fd+ffr<=10:
        print(f"  DIV {nm}: eng={e} ora={o}")
        print(f"     drl={scns[nm]['drl'].strip()} facts={[(f['type'],f['fields']['ts']) for f in scns[nm]['facts']]} epochs={len(scns[nm]['epochs'])}")
print(f"positive reaper fuzz {n} seed {seed}: {fd} FACTS-div, {ffr} FIRING-div")
