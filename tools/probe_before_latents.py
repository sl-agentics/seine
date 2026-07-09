import json, os, subprocess, collections
REPO="/home/bryan/rust-rules"; OUT="/tmp/seine_before_latents"; os.makedirs(OUT,exist_ok=True)
def et(n): return {"name":n,"fields":[{"name":"ts","type":"i64"}],"event":{"timestamp":"ts"}}
def runcmd(cmd,C):
    env=dict(os.environ); env["PATH"]=os.path.expanduser("~/.cargo/bin")+":"+env.get("PATH","")
    paths=[]
    for nm,s in C.items():
        p=os.path.join(OUT,nm+".json"); json.dump(s,open(p,"w")); paths.append(p)
    r=subprocess.run(["cargo","run","-q","-p","seine-harness","--",cmd]+paths,cwd=REPO,capture_output=True,text=True,env=env)
    R={}
    for ln in r.stdout.splitlines():
        try:o=json.loads(ln)
        except:continue
        rr=o.get("result"); R[o["scenario"]]=None if not rr else sorted((f["type"],f["fields"]["ts"]) for f in rr.get("facts",[]))
    return R
def scn(nm,drl,nt,facts,adv=True):
    return {"name":nm,"types":[et(f"E{i}") for i in range(nt)],"drl":drl,
            "facts":[{"type":t,"fields":{"ts":v}} for t,v in facts],
            "epochs":([{"actions":[{"op":"advance","ms":1000}],"facts":[]}] if adv else [])}
C={}
# PURE POSITIVE, no not — before[0,100] earlier operand E2@-129
C["pos_far"]=scn("pos_far","rule R when $a : E0() $c : E2(this before[0ms,100ms] $a) then end\n",3,[("E0",3),("E2",-129)])
# PURE POSITIVE at-insert expiry: E2@-51, before[0,50], NO advance
C["pos_ins"]=scn("pos_ins","rule R when $a : E0() $c : E2(this before[0ms,50ms] $a) then end\n",3,[("E0",1),("E2",-51)],adv=False)
eng=runcmd("run",C); ora=runcmd("oracle",C)
for nm in C:
    tag = "DIVERGE" if eng.get(nm)!=ora.get(nm) else "match"
    print(f"{nm:9} [{tag}] engine={eng.get(nm)}  oracle={ora.get(nm)}")
