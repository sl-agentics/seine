#!/usr/bin/env python3
"""Compact firing-log dumper for oracle NDJSON output."""
import json, sys

def render(m):
    if not isinstance(m, dict):
        return str(m)
    t = m.get("type", "?")
    f = m.get("fields", {})
    if not f:
        return t
    vals = ",".join(f"{k}={v}" for k, v in sorted(f.items()))
    return f"{t}({vals})"

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        d = json.loads(line)
    except Exception:
        print("RAW:", line[:300])
        continue
    n = d.get("scenario")
    if "error" in d:
        print(f"== {n}: ERROR: {d['error'][:500]}")
        continue
    r = d["result"]
    firs = r.get("firings", [])
    print(f"== {n}: {len(firs)} firings")
    for f in firs:
        ms = " | ".join(render(m) for m in f.get("matches", []))
        print(f"   {f['rule']:28s} [{ms}]")
    facts = r.get("facts", [])
    if facts:
        fs = ", ".join(render(m) for m in facts)
        print(f"   -- facts: {fs}")
