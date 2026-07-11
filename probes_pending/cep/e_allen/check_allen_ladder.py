#!/usr/bin/env python3
"""D-164: run the Allen reach-value ladder through the ORACLE and compare
every scenario's final-facts presence against the manifest prediction."""
import json, os, subprocess, sys

ROOT = "/home/bryan/rust-rules"
TMP = os.environ.get("ALLEN_TMP", "/home/bryan/.claude/jobs/577ad61a/tmp/allen_ladder")
manifest = json.load(open(f"{TMP}/manifest.json"))
files = [f"{TMP}/{n}.json" for n in sorted(manifest)]
out = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--",
                      "oracle"] + files, capture_output=True, text=True, cwd=ROOT)
facts_by_name = {}
for ln in out.stdout.splitlines():
    ln = ln.strip()
    if ln.startswith("{"):
        j = json.loads(ln)
        res = j.get("result")
        if res is not None:
            facts_by_name[j["scenario"]] = res.get("facts", [])
bad = miss = 0
for name, exp in sorted(manifest.items()):
    if name not in facts_by_name:
        print(f"  MISSING oracle result: {name}")
        miss += 1
        continue
    present = any(f.get("type") == exp["type"] for f in facts_by_name[name])
    if present != exp["present"]:
        bad += 1
        print(f"  MISPREDICT {name}: predicted present={exp['present']}, "
              f"oracle present={present}")
print(f"{len(manifest)} ladder cells: "
      f"{'ALL PREDICTED ✓' if bad == 0 and miss == 0 else f'{bad} mispredicted, {miss} missing'}")
sys.exit(1 if (bad or miss) else 0)
