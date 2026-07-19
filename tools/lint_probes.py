#!/usr/bin/env python3
"""D-102 harness hardening: probe LIVENESS + ENGINE-VALIDITY lint.

Two failure classes this guards (the u2 lesson):
1. GHOST PROBES — scenarios whose DRL the engine cannot parse/compile
   sit unnoticed in probes_pending/ until drafted into a differential
   (u2's getter syntax). Every probe must RUN engine-side.
2. INERT GREENS — a differential where both sides execute but zero
   activations ever fire passes trivially ("green because inert").
   Every probe must produce at least one firing across its lifetime,
   unless it declares {"expect_inert": true} (a deliberate
   nothing-must-fire pin).

Per-fire emptiness is fine (many pins assert empty fires); the lint is
scenario-TOTAL liveness. Exit 1 on any violation.

Usage: .venv/bin/python tools/lint_probes.py <files-or-dirs...>
       (default: scenarios/probes scenarios/regressions scenarios/duckdb
        probes_pending)
"""
import json
import os
import subprocess
import sys

BIN = "target/debug/seine-harness"


def scan(paths):
    files = []
    for p in paths:
        if os.path.isdir(p):
            for root, _, names in os.walk(p):
                files.extend(os.path.join(root, n) for n in sorted(names) if n.endswith(".json"))
        elif p.endswith(".json"):
            files.append(p)
    return files


def main():
    paths = sys.argv[1:] or [
        "scenarios/probes", "scenarios/regressions", "scenarios/duckdb", "probes_pending",
    ]
    subprocess.run(["cargo", "build", "-q", "-p", "seine-harness"], check=True)
    files = scan(paths)
    ghosts, inert, ok = [], [], 0
    for f in files:
        try:
            scn = json.load(open(f))
        except Exception as ex:
            ghosts.append((f, f"unparseable JSON: {ex}"))
            continue
        if scn.get("open_divergence"):
            # a filed OPEN divergence witness (engine may error or
            # diverge; the pending dir holds it until its class closes)
            ok += 1
            continue
        if scn.get("expect_error"):
            # D-330: a certified ERROR-PARITY cell (make diff certifies
            # error-vs-error against the oracle — the D-013/j21 lane);
            # the engine must still ERROR here, else the parity is gone
            r = subprocess.run([BIN, "run", f], capture_output=True, text=True)
            out = None
            try:
                out = json.loads(r.stdout)
            except Exception:
                pass
            if r.returncode == 0 and out and "error" not in out:
                ghosts.append((f, "expect_error probe ran CLEAN — the parity error is gone?"))
            else:
                ok += 1
            continue
        if scn.get("engine_fenced"):
            # deliberate oracle-recon probe of a WALLED shape: the
            # engine must reject it LOUDLY (a fence regression check)
            r = subprocess.run([BIN, "run", f], capture_output=True, text=True)
            out = None
            try:
                out = json.loads(r.stdout)
            except Exception:
                pass
            if r.returncode == 0 and out and "error" not in out:
                ghosts.append((f, "engine_fenced probe RAN — the wall is gone?"))
            else:
                ok += 1
            continue
        r = subprocess.run([BIN, "run", f], capture_output=True, text=True)
        out = None
        try:
            out = json.loads(r.stdout)
        except Exception:
            pass
        err = (out or {}).get("error") if out else (r.stderr[-200:] or "no output")
        if r.returncode != 0 or (out and "error" in out):
            ghosts.append((f, str(err)[:140]))
            continue
        res = out.get("result", {})
        firings = res.get("firings", [])
        queries = res.get("queries", [])
        has_query_rows = any(q.get("rows") or q.get("identifiers") for q in queries)
        if not firings and not has_query_rows and not scn.get("expect_inert"):
            inert.append(f)
            continue
        ok += 1
    for f, e in ghosts:
        print(f"GHOST  {f}: {e}")
    for f in inert:
        print(f"INERT  {f} (no firings ever; add \"expect_inert\": true if deliberate)")
    print(f"--- {ok} live, {len(ghosts)} ghosts, {len(inert)} inert, {len(files)} total")
    sys.exit(1 if (ghosts or inert) else 0)


if __name__ == "__main__":
    main()
