#!/usr/bin/env python3
"""I-RD population fuzzer (D-207) — arc-local recon; gen.rs walls STAY UP.

Draws random compositions from the model_ird vocabulary (ONE draw
emits both the DRL scenario and the model spec), then scores:
  MODEL-vs-ORACLE  — the 0-div gate ("do the three I-RD laws
                     generalize?"); raw mismatches are re-run 3x
                     oracle-side (flake filter) before counting REAL.
  ENGINE-vs-ORACLE — the census ride-along = the I-RD port baseline.
Cases that reach a model assert-unreachable corner are counted per
corner (the next cell round's worklist), banked one witness per
corner, and excluded from the 0-div comparison — they are not
failures. Grammar constraints (confound controls, pre-registered in
ird-population-predictions.md): distinct saliences per scenario; no
T0(f0==false) alphas; arms/triggers disjoint from payloads.

Usage: python3 tools/fuzz_tms_ird.py <n> <seed> [--keep]
"""
import json
import os
import random
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO, "probes_pending", "tms_envelope"))
import model_ird as M  # noqa: E402

HARNESS = ["cargo", "run", "-q", "-p", "seine-harness", "--"]
TYPES = [
    {"fields": [{"name": "f0", "type": "bool"}], "name": "T0"},
    {"fields": [{"name": "f0", "type": "String"},
                {"name": "f1", "type": "bool"}], "name": "T1"},
]
PAYLOADS = ["v", "w"]


# ---------- DRL templates (1:1 with the model kinds) ----------

def drl_rule(name, sal, spec):
    k = spec["kind"]
    if k == "jl":
        if spec["selfjoin"]:
            lhs = "    $a : T0()\n    $b : T0(f0 == true)"
            tgt = "$b"
        else:
            lhs = "    $t : T0(f0 == true)"
            tgt = "$t"
        rhs = f'    insertLogical(new T1("{spec["val"]}", true));\n'
        if spec["brk"] == "update":
            rhs += f"    {tgt}.setF0(false);\n    update({tgt});\n"
        elif spec["brk"] == "modify":
            rhs += f"    modify({tgt}) {{ setF0(false) }}\n"
        elif spec["brk"] == "delete":
            rhs += f"    delete({tgt});\n"
    elif k == "st":
        lhs = "    T0(f0 == true)"
        rhs = f'    insert(new T1("{spec["val"]}", true));\n'
    elif k == "del":
        lhs = f'    $p : T1(f0 == "{spec["val"]}", f1 == true)'
        rhs = "    delete($p);\n"
    elif k == "obs":
        lhs = "    T1($x : f0, $y : f1)"
        rhs = ""
    elif k == "midt":
        lhs = f'    T1(f0 == "{spec["trig"]}", f1 == false)'
        bf1 = "true" if spec["bf1"] else "false"
        rhs = f'    insertLogical(new T1("{spec["val"]}", {bf1}));\n'
    elif k == "killt0":
        lhs = ('    $t : T0(f0 == true)\n'
               f'    $k : T1(f0 == "{spec["arm"]}", f1 == false)')
        rhs = "    delete($t);\n    delete($k);\n"
    elif k == "ru":
        lhs = ('    $t : T0(f0 == true)\n'
               f'    $a : T1(f0 == "{spec["arm"]}", f1 == false)')
        rhs = "    $t.setF0(false);\n    update($t);\n    delete($a);\n"
    else:
        raise AssertionError(k)
    return f'rule "{name}"\nsalience {sal}\nwhen\n{lhs}\nthen\n{rhs}end\n'


def draw(rng):
    """One draw -> (facts, rules, drl). Redraws until a justifier exists."""
    while True:
        n = rng.randint(3, 6)
        sals = rng.sample(range(-10, 26), n)
        kinds = rng.choices(
            ["jl", "st", "del", "obs", "midt", "killt0", "ru"],
            weights=[30, 20, 20, 10, 10, 5, 5], k=n)
        if not any(k in ("jl", "midt") for k in kinds):
            continue
        facts = [("T0", {"f0": True})]
        if rng.random() < 0.2:
            facts.append(("T0", {"f0": True}))
        for p in PAYLOADS:
            if rng.random() < 0.4:
                facts.append(("T1", {"f0": p, "f1": True}))
        rules = {}
        armn = 0
        for i, (k, s) in enumerate(zip(kinds, sals)):
            name = f"R{i}"
            if k == "jl":
                brk = rng.choices([None, "update", "modify", "delete"],
                                  weights=[50, 20, 15, 15])[0]
                sj = rng.random() < 0.15
                rules[name] = M.JL(s, rng.choice(PAYLOADS), brk=brk,
                                   selfjoin=sj)
            elif k == "st":
                rules[name] = M.ST(s, rng.choice(PAYLOADS))
            elif k == "del":
                rules[name] = M.DEL(s, rng.choice(PAYLOADS))
            elif k == "obs":
                rules[name] = M.OBS(s)
            elif k == "midt":
                trig = f"g{armn}"; armn += 1
                facts.append(("T1", {"f0": trig, "f1": False}))
                rules[name] = M.MIDT(s, trig, False, rng.choice(PAYLOADS),
                                     bf1=rng.random() < 0.7)
            elif k == "killt0":
                arm = f"k{armn}"; armn += 1
                facts.append(("T1", {"f0": arm, "f1": False}))
                rules[name] = M.KILLT0(s, arm)
            elif k == "ru":
                arm = f"u{armn}"; armn += 1
                facts.append(("T1", {"f0": arm, "f1": False}))
                rules[name] = M.RUK(s, arm)
        drl = "".join(drl_rule(nm, rules[nm]["sal"], rules[nm])
                      for nm in rules)
        return facts, rules, drl


def run_batch(cmd, files, out_path):
    with open(out_path, "w") as fh:
        r = subprocess.run(HARNESS + [cmd] + files, stdout=fh,
                           stderr=subprocess.DEVNULL, cwd=REPO, timeout=600)
    if r.returncode != 0:
        sys.exit(f"harness {cmd} batch failed")
    return {json.loads(l)["scenario"]: json.loads(l) for l in open(out_path)}


def canon(entry):
    if "error" in entry or "result" not in entry:
        return ("ERROR", str(entry.get("error"))[:120])
    return M.truth_of(entry)


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    keep = "--keep" in sys.argv
    rng = random.Random(seed)
    work = tempfile.mkdtemp(prefix=f"irdpop_{seed}_")
    cases, files = {}, []
    for i in range(n):
        facts, rules, drl = draw(rng)
        name = f"irdp{seed}x{i}"
        path = os.path.join(work, name + ".json")
        json.dump({"name": name, "drl": drl,
                   "facts": [{"type": t, "fields": f} for t, f in facts],
                   "types": TYPES}, open(path, "w"))
        cases[name] = (facts, rules)
        files.append(path)

    BATCH = 40
    orc, eng = {}, {}
    for j in range(0, len(files), BATCH):
        chunk = files[j:j + BATCH]
        orc.update(run_batch("oracle", chunk,
                             os.path.join(work, f"o{j}.ndjson")))
        eng.update(run_batch("run", chunk,
                             os.path.join(work, f"e{j}.ndjson")))

    corners, corner_witness = {}, {}
    raw_mism, clean, errors = [], 0, 0
    eng_div = 0
    for name, (facts, rules) in cases.items():
        otruth = canon(orc[name])
        if otruth[0] == "ERROR":
            errors += 1
            continue
        if canon(eng[name]) != otruth:
            eng_div += 1
        try:
            got = M.Sim(facts, rules).run()
        except AssertionError as e:
            key = str(e)
            corners[key] = corners.get(key, 0) + 1
            corner_witness.setdefault(key, name)
            continue
        if got == otruth:
            clean += 1
        else:
            raw_mism.append((name, got))

    real = []
    for name, got in raw_mism:
        f = os.path.join(work, name + ".json")
        o2 = canon(run_batch("oracle", [f],
                             os.path.join(work, name + "_o2.ndjson"))[name])
        o3 = canon(run_batch("oracle", [f],
                             os.path.join(work, name + "_o3.ndjson"))[name])
        base = canon(orc[name])
        if not (base == o2 == o3):
            print(f"FLAKY {name} (oracle unstable across launches) — "
                  f"quarantine-class")
            continue
        real.append((name, got, base))

    cstr = (", ".join(f"{k.split(':')[-1].strip()}={v} "
                      f"[{corner_witness[k]}]" for k, v in sorted(corners.items()))
            or "none")
    print(f"population: {n} cases, seed {seed}: model-vs-oracle {clean} "
          f"clean, {len(raw_mism)} raw mismatches, {len(real)} REAL after "
          f"3x; corners: {cstr}; engine-vs-oracle divergent: {eng_div}"
          + (f"; oracle-errors: {errors}" if errors else ""))
    for name, got, want in real:
        print(f"REAL {name}")
        print(f"   oracle: {want}")
        print(f"   model:  {got}")
    if keep or real or corners:
        print(f"workdir: {work}")
    return 1 if real else 0


if __name__ == "__main__":
    raise SystemExit(main())
