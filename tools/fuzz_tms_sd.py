#!/usr/bin/env python3
"""fuzz_tms_sd — dedicated L-SD envelope population fuzzer (D-189).

Draws scenarios INSIDE the (cloud x self-defeat) fence — the gen.rs
D-078/D-080 walls STAY UP; this is arc-local recon tooling — and
checks the executable spec (probes_pending/tms_envelope/model_sd.py)
against the live ORACLE on fresh out-of-sample seeds. Also records the
ENGINE-vs-oracle census (the port A/B baseline), which is diagnostic,
not the 0-div target.

Grammar per scenario:
  2-4 distinct P facts; exactly 1 justifier (k 0/1, lead/trail not,
  plain/no-loop, or-twin only with no-loop [the lazy or-twin is the
  banked b3 runaway — drawing it wastes oracle time], breaks 90%);
  0-2 observers (LK / LK-join-P / P-only); 0-1 deleter (not-guarded or
  LK-joined, no-loop); salience from a small menu; decl order shuffled.

MODEL-vs-ORACLE mismatches are re-run 3x oracle-side (flake filter,
fz_42_84 doctrine) before being reported as real. Usage:
  python3 tools/fuzz_tms_sd.py <n_cases> <seed> [--keep]
Exit 0 = population clean; 1 = real model divergences (listed).
"""
import json, os, random, shutil, subprocess, sys, tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO, "probes_pending", "tms_envelope"))
from model_sd import simulate  # noqa: E402

HARNESS = ["cargo", "run", "-q", "-p", "seine-harness", "--"]
P_T = {"name": "P", "fields": [{"name": "f0", "type": "i64"}]}
LK_T = {"name": "LK2", "fields": [{"name": "f0", "type": "i64"},
                                  {"name": "f1", "type": "bool"}]}
SALS = [-10, -5, -1, 0, 0, 0, 1, 5, 7, 10]


def draw(rng):
    nfacts = rng.randint(2, 4)
    facts = list(range(1, nfacts + 1))
    rules = []
    eager = rng.random() < 0.4
    ortwin = eager and rng.random() < 0.2
    k = 0 if ortwin else rng.choice([0, 1, 1, 1])
    rules.append({"kind": "justifier", "sal": rng.choice(SALS), "k": k,
                  "notpos": rng.choice(["lead", "trail"]) if k else "trail",
                  "eager": eager, "ortwin": ortwin,
                  "breaks": rng.random() < 0.9})
    for _ in range(rng.randint(0, 2)):
        rules.append({"kind": rng.choice(["obs_lk", "obs_join", "obs_p"]),
                      "sal": rng.choice(SALS)})
    if rng.random() < 0.6:
        rules.append({"kind": rng.choice(["del_not", "del_join"]),
                      "sal": rng.choice(SALS)})
    rng.shuffle(rules)
    for i, r in enumerate(rules):
        r["name"] = f"R{i}"
    return facts, rules


def drl_of(rules):
    out = []
    for r in rules:
        sal = f"salience {r['sal']}\n" if r["sal"] else ""
        nl = "no-loop\n" if (r["kind"] == "justifier" and r["eager"]) \
             or r["kind"] in ("del_not", "del_join") else ""
        head = f'rule "{r["name"]}"\n{nl}{sal}when\n'
        if r["kind"] == "justifier":
            ins_f1 = "false" if r["breaks"] else "true"
            if r.get("ortwin"):
                body = ("    ( not LK2(f1 != true) )\n    or\n"
                        "    ( not LK2(f1 != true) )\n")
                rhs = f"    insertLogical(new LK2(7, {ins_f1}));\n"
            elif r["k"] == 0:
                body = "    not LK2(f1 != true)\n"
                rhs = f"    insertLogical(new LK2(7, {ins_f1}));\n"
            else:
                pat = "    $p : P($x : f0)\n"
                np = "    not LK2(f1 != true)\n"
                body = np + pat if r["notpos"] == "lead" else pat + np
                rhs = f"    insertLogical(new LK2($x, {ins_f1}));\n"
        elif r["kind"] == "obs_lk":
            body, rhs = "    LK2($v : f0)\n", ""
        elif r["kind"] == "obs_join":
            body, rhs = "    LK2($v : f0)\n    $p : P($x : f0)\n", ""
        elif r["kind"] == "obs_p":
            body, rhs = "    $p : P($x : f0)\n", ""
        elif r["kind"] == "del_not":
            body = "    $p : P($x : f0)\n    not LK2(f1 != true)\n"
            rhs = "    delete($p);\n"
        elif r["kind"] == "del_join":
            body = "    $p : P($x : f0)\n    LK2(f1 == false)\n"
            rhs = "    delete($p);\n"
        out.append(head + body + "then\n" + rhs + "end\n")
    return "".join(out)


def truth_of(entry):
    if "error" in entry:
        return {"runaway": "fire limit" in str(entry["error"]),
                "error": str(entry["error"])[:60], "firings": None, "finals": None}
    seq = []
    for f in entry["result"]["firings"]:
        pv = None
        for m in f.get("matches", []):
            if m["type"] == "P":
                pv = m["fields"].get("f0")
                break
        seq.append((f["rule"], pv))
    finals = sorted(("LK" if fa["type"] == "LK2" else fa["type"],
                     fa["fields"].get("f0")) for fa in entry["result"]["facts"])
    return {"runaway": False, "firings": seq, "finals": finals}


def model_of(facts, rules):
    got = simulate(facts, rules)
    if got["runaway"]:
        return {"runaway": True, "firings": None, "finals": None}
    return {"runaway": False, "firings": [tuple(x) for x in got["firings"]],
            "finals": got["finals"]}


def run_batch(cmd, files, out_path):
    with open(out_path, "w") as fh:
        r = subprocess.run(HARNESS + [cmd] + files, stdout=fh,
                           stderr=subprocess.DEVNULL, cwd=REPO)
    if r.returncode != 0:
        sys.exit(f"harness {cmd} batch failed")
    return {json.loads(l)["scenario"]: json.loads(l) for l in open(out_path)}


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    keep = "--keep" in sys.argv
    rng = random.Random(seed)
    work = tempfile.mkdtemp(prefix=f"sdpop_{seed}_")
    cases, files = {}, []
    for i in range(n):
        facts, rules = draw(rng)
        name = f"sdp{seed}x{i}"
        path = os.path.join(work, name + ".json")
        json.dump({"name": name, "drl": drl_of(rules),
                   "facts": [{"type": "P", "fields": {"f0": v}} for v in facts],
                   "types": [P_T, LK_T]}, open(path, "w"))
        cases[name] = (facts, rules)
        files.append(path)
    BATCH = 40
    orc, eng = {}, {}
    for j in range(0, len(files), BATCH):
        chunk = files[j:j + BATCH]
        orc.update(run_batch("oracle", chunk, os.path.join(work, f"o{j}.ndjson")))
        eng.update(run_batch("run", chunk, os.path.join(work, f"e{j}.ndjson")))
    mism, eng_div = [], 0
    for name, (facts, rules) in cases.items():
        t, m = truth_of(orc[name]), model_of(facts, rules)
        if truth_of(eng[name]) != t:
            eng_div += 1
        if (m["runaway"], m["firings"], m["finals"]) != \
           (t["runaway"], t["firings"], t["finals"]):
            mism.append((name, m, t))
    real = []
    for name, m, t in mism:                     # flake filter: 3x re-run
        stable = 0
        for rep in range(3):
            o2 = run_batch("oracle", [os.path.join(work, name + ".json")],
                           os.path.join(work, f"re_{name}_{rep}.ndjson"))
            if truth_of(o2[name]) == (t if isinstance(t, dict) else t):
                stable += 1
        if stable == 3:
            real.append((name, m, t))
        else:
            print(f"FLAKY {name} (oracle unstable across launches) — quarantine-class")
    print(f"population: {n} cases, seed {seed}: model-vs-oracle "
          f"{n - len(mism)} clean, {len(mism)} raw mismatches, "
          f"{len(real)} REAL after 3x; engine-vs-oracle divergent: {eng_div}")
    for name, m, t in real:
        print(f"== {name}  ({os.path.join(work, name + '.json')})")
        print(f"   model  {m}")
        print(f"   oracle {t}")
    if keep or real:
        print(f"work dir kept: {work}")
    else:
        shutil.rmtree(work, ignore_errors=True)
    sys.exit(1 if real else 0)


if __name__ == "__main__":
    main()
