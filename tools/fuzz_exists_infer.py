#!/usr/bin/env python3
"""@expires INFERENCE through an exists — recon + FACTS gate (D-135).

Sibling of `fuzz_exists_temporal.py` (the D-127 FIRING gate, explicit @expires
everywhere) but with @expires ABSENT half the time + a coin-flip `advance`, to
exercise the INFERENCE arc. Splits engine-vs-oracle divergence into FIRING-set
vs FACTS-only (reaping). Recon (seeds 5001-5003, 300 each): FIRING 0-div (the
inference is INVISIBLE to firings — exists fires when a partner is present,
retraction unobservable, D-127), FACTS-only ~25% (absent-@expires + advance:
Drools infers a finite expiry and reaps; the engine forces NEVER for exists so
it KEEPS the events). Mechanism = the not §3A analog (D-130/D-132): the exists
pattern needs a phantom `temporal_pos` so its after/before STP edges record
(offsets after ⇒ E0=hi, E1=lo?0:NEVER; before mirror). NO §3B (no firing
deferral). Port target: this tool 0 FIRING + 0 FACTS divergence.

Usage: fuzz_exists_infer.py <n> <seed>
"""
import json, os, random, subprocess, sys, collections
REPO = "/home/bryan/rust-rules"

def etype(n, expires):
    ev = {"timestamp": "ts"}
    if expires: ev["expires_ms"] = 100000
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}], "event": ev}

def gen(rng, name):
    shape = rng.choice(["ex_partner","ex_partner","chain_ex","ex_mid"])
    op1 = rng.choice(["after","before"]); h1 = rng.choice([50,100,150])
    expires = rng.random() < 0.5          # inference axis (absent half)
    advance = rng.random() < 0.5          # reaping axis
    if shape == "ex_partner":
        drl = f"rule EX when $a : E0() exists E1(this {op1}[0ms,{h1}ms] $a) then end\n"; ntypes=2
    elif shape == "chain_ex":
        op2 = rng.choice(["after","before"]); h2 = rng.choice([50,100])
        drl = (f"rule EX when $a : E0() $b : E1(this {op1}[0ms,{h1}ms] $a) "
               f"exists E2(this {op2}[0ms,{h2}ms] $b) then end\n"); ntypes=3
    else:
        op2 = rng.choice(["after","before"]); h2 = rng.choice([50,100])
        drl = (f"rule EX when $a : E0() exists E1(this {op1}[0ms,{h1}ms] $a) "
               f"$c : E2(this {op2}[0ms,{h2}ms] $a) then end\n"); ntypes=3
    facts=[]; a_ts = rng.sample(range(0,6), rng.choice([1,1,2]))
    facts += [("E0",t) for t in a_ts]; base=a_ts[0]
    for lvl in range(1,ntypes):
        op,hi = (op1,h1) if lvl==1 else (op2,h2)
        tss,used=[],set()
        for _ in range(rng.randint(1,3)):
            for _ in range(20):
                d=rng.randint(0,hi+10); ts=base+d if op=="after" else base-d
                if ts not in used: used.add(ts); tss.append(ts); break
        facts += [(f"E{lvl}",t) for t in tss]; base=tss[0]
    rng.shuffle(facts)
    epochs=[{"actions":[{"op":"advance","ms":1000}],"facts":[]}] if advance else []
    return {"name":name,"types":[etype(f"E{i}",expires) for i in range(ntypes)],
            "drl":drl,"facts":[{"type":t,"fields":{"ts":v}} for t,v in facts],"epochs":epochs}

def run(cmd, paths):
    env=dict(os.environ); env["PATH"]=os.path.expanduser("~/.cargo/bin")+":"+env.get("PATH","")
    r=subprocess.run(["cargo","run","-q","-p","seine-harness","--",cmd]+paths,cwd=REPO,capture_output=True,text=True,env=env)
    out={}
    for line in r.stdout.splitlines():
        try:o=json.loads(line)
        except:continue
        res=o.get("result") or {}
        fir=[tuple(sorted((m["type"],m["fields"]["ts"]) for m in fr["matches"])) for fr in res.get("firings",[])]
        fac=collections.Counter((f["type"],f["fields"]["ts"]) for f in res.get("facts",[]))
        out[o["scenario"]]=(fir,fac)
    return out

def main():
    n,seed=int(sys.argv[1]),int(sys.argv[2])
    OUT=f"/tmp/exinf{seed}"; os.makedirs(OUT,exist_ok=True)
    rng=random.Random(seed); paths,scns=[],{}
    for i in range(n):
        s=gen(rng,f"exi{seed}x{i}"); p=os.path.join(OUT,f"exi{seed}x{i}.json"); json.dump(s,open(p,"w")); paths.append(p); scns[s["name"]]=s
    eng=run("run",paths); ora=run("oracle",paths)
    fir_div=fac_only=both_ok=0; ex_fac=[]
    for nm,s in scns.items():
        e,o=eng.get(nm),ora.get(nm)
        if e is None or o is None: continue
        ef,efc=e; of,ofc=o
        firdiff = ef!=of; facdiff = efc!=ofc
        if firdiff: fir_div+=1
        elif facdiff:
            fac_only+=1
            if len(ex_fac)<8: ex_fac.append((nm, sorted((efc-ofc).elements()), sorted((ofc-efc).elements()), s))
        else: both_ok+=1
    print(f"=== exists-inference RECON: {n} cases seed {seed} ===")
    print(f"  FIRING-set divergences: {fir_div}  ({100*fir_div//n}%)")
    print(f"  FACTS-only  divergences: {fac_only}  ({100*fac_only//n}%)")
    print(f"  clean: {both_ok}")
    for nm,eonly,oonly,s in ex_fac:
        exp=s['types'][0]['event'].get('expires_ms','-'); adv=bool(s['epochs'])
        print(f"  FACTS-only {nm}: engine_extra={eonly} oracle_extra={oonly}")
        print(f"     drl={s['drl'].strip()}  exp={exp} adv={adv}  facts={[(f['type'],f['fields']['ts']) for f in s['facts']]}")

if __name__=="__main__": main()
