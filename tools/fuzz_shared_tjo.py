#!/usr/bin/env python3
"""SHARED temporal-join node ORDER — recon + gate (D-136).

RECON (seeds 7101+, ~300 cases): 14% ORDER-only divergence, 0 SET-miss. The
oracle's PER-RULE tuple order is EXACTLY the single-rule D-125 flush order
(`model_join_flush`, 100% match — proven by a single-rule-variant diff), so the
target is a KNOWN spec, not a heap tie. ~76% of divergences are grouped-by-rule
(each sharing rule fires its D-125 batch contiguously); ~24% interleave across
rules (mostly multi-epoch = the agenda-pop composition). The engine BAILS the
D-125 per-arrival flush for a SHARED temporal node (`engine.rs stream_flush_ex`
~3760: `node_linked && node_shared` => stash-all => legacy pop-time), which
orders wrong. Port target: flush a shared node per-arrival routed to each
sharing rule path in D-125 order + model the cross-rule agenda interleaving.
NOTE D-102: the naive unscoped force-flush blast-radiused 18% of single-rule
scenarios — the plumbing needs care.

RECON: SHARED temporal-join node ordering. Two rules with the SAME temporal
join LHS ($a:E0() $b:E1(op[0,hi] $a)) => the join node is SHARED, so the engine
BAILS the D-125 per-arrival flush to legacy pop-time composition. Positive-only
(no TMS/not/salience) to isolate the shared-node order. Split engine-vs-oracle
into firing-SET vs firing-ORDER (same multiset, different sequence)."""
import json, os, random, subprocess, sys, collections
REPO="/home/bryan/rust-rules"
def etype(n): return {"name":n,"fields":[{"name":"ts","type":"i64"}],"event":{"timestamp":"ts","expires_ms":100000}}
def gen(rng,name):
    op=rng.choice(["after","before"]); hi=rng.choice([50,100,150])
    nrules=rng.choice([2,2,3])   # 2-3 rules sharing the SAME temporal pattern
    pat=f"$a : E0() $b : E1(this {op}[0ms,{hi}ms] $a)"
    drl="".join(f"rule TJ{i} when {pat} then end\n" for i in range(nrules))
    facts=[]; a_ts=rng.sample(range(0,6),rng.choice([1,2,2]))
    facts+=[("E0",t) for t in a_ts]; base=a_ts[0]
    tss,used=[],set()
    for _ in range(rng.randint(1,3)):
        for _ in range(20):
            d=rng.randint(0,hi+10); ts=base+d if op=="after" else base-d
            if ts not in used: used.add(ts); tss.append(ts); break
    facts+=[("E1",t) for t in tss]
    rng.shuffle(facts)
    # optional: split some facts into a 2nd epoch (pop-time boundary)
    ep=[]
    if rng.random()<0.5 and len(facts)>2:
        k=rng.randint(1,len(facts)-1); head,tail=facts[:k],facts[k:]
        facts=head; ep=[{"actions":[],"facts":[{"type":t,"fields":{"ts":v}} for t,v in tail]}]
    return {"name":name,"types":[etype("E0"),etype("E1")],"drl":drl,
            "facts":[{"type":t,"fields":{"ts":v}} for t,v in facts],"epochs":ep}
def run(cmd,paths):
    env=dict(os.environ); env["PATH"]=os.path.expanduser("~/.cargo/bin")+":"+env.get("PATH","")
    r=subprocess.run(["cargo","run","-q","-p","seine-harness","--",cmd]+paths,cwd=REPO,capture_output=True,text=True,env=env)
    out={}
    for line in r.stdout.splitlines():
        try:o=json.loads(line)
        except:continue
        res=o.get("result") or {}
        fir=[(fr["rule"],tuple(sorted((m["type"],m["fields"]["ts"]) for m in fr["matches"]))) for fr in res.get("firings",[])]
        out[o["scenario"]]=fir
    return out
def main():
    n,seed=int(sys.argv[1]),int(sys.argv[2])
    OUT=f"/tmp/shtjo{seed}"; os.makedirs(OUT,exist_ok=True)
    rng=random.Random(seed); paths,scns=[],{}
    for i in range(n):
        s=gen(rng,f"sh{seed}x{i}"); p=os.path.join(OUT,f"sh{seed}x{i}.json"); json.dump(s,open(p,"w")); paths.append(p); scns[s["name"]]=s
    eng=run("run",paths); ora=run("oracle",paths)
    setdiff=orderdiff=clean=0; ex=[]
    for nm,s in scns.items():
        e,o=eng.get(nm),ora.get(nm)
        if e is None or o is None: continue
        if e==o: clean+=1
        elif collections.Counter(e)==collections.Counter(o):
            orderdiff+=1
            if len(ex)<6: ex.append(("ORDER",nm,e,o,s))
        else:
            setdiff+=1
            if len(ex)<6: ex.append(("SET",nm,e,o,s))
    print(f"=== shared-tjo RECON: {n} seed {seed} ===  ORDER={orderdiff} ({100*orderdiff//n}%) SET={setdiff} clean={clean}")
    for k,nm,e,o,s in ex:
        print(f"  {k} {nm}: drl_rules={s['drl'].count('rule')} eng={[(r,list(t)) for r,t in e]}")
        print(f"        ora={[(r,list(t)) for r,t in o]}  epochs={bool(s['epochs'])}")
if __name__=="__main__": main()
