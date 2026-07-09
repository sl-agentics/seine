#!/usr/bin/env python3
"""CEP item #2 — non-temporal not-unblock STAGING-order model. Validates the
PREDICTED firing order against the ORACLE population captured by
fuzz_notorder.py (0-div is the port spec, D-125 methodology).

THE RULE (probed 2026-07-09):
  On unblock of `not <BLOCKER>() P()`, the blocked P's fire grouped by BATCH =
  the epoch in which each P was LAST TOUCHED (initial insert = epoch 0; an epoch
  insert or update = that epoch). Batch ordering:
    - the INITIAL batch (epoch 0) is ALWAYS last;
    - the epoch batches (>=1) go FORWARD for a PLAIN blocker, REVERSE for an
      EVENT blocker (@role event); trigger (delete vs expiry) does NOT matter.
  WITHIN a batch: inserts first (by insertion index), then updates (by the P's
  ORIGINAL insertion index).

Engine today: full-LIFO on event-expiry (reverses WITHIN batches too), full-FIFO
on event-delete; plain-delete already matches. Usage:
  model_check_notorder.py <notpop_*.json> [event|plain]
"""
import json, sys

def predict(scn, event_blocker):
    facts = scn["facts"]
    gidx = {}          # v -> global insertion index (insertion order key)
    vof = {}           # global insertion index -> v (resolves update targets)
    batch = {}         # v -> last-touch epoch (0 = initial)
    kind = {}          # v -> "ins" | "upd" (last touch)
    useq = {}          # v -> global apply-sequence of the P's LAST update
    idx = 0
    app = [0]
    for f in facts:
        if f["type"] == "P":
            v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = 0; kind[v] = "ins"
        idx += 1
    for e_i, ep in enumerate(scn["epochs"]):
        epoch_no = e_i + 1
        for a in ep["actions"]:
            if a["op"] == "update":
                v = vof.get(a["target"])
                if v is not None:
                    batch[v] = epoch_no; kind[v] = "upd"; useq[v] = app[0]; app[0] += 1
        for f in ep["facts"]:
            if f["type"] == "P":
                v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = epoch_no; kind[v] = "ins"
            idx += 1
    ps = list(gidx)
    epoch_batches = sorted({batch[v] for v in ps if batch[v] >= 1})
    order_batches = (list(reversed(epoch_batches)) if event_blocker else epoch_batches) + [0]
    out = []
    for b in order_batches:
        members = [v for v in ps if batch[v] == b]
        # within a batch: inserts (insertion order), then updates (REVERSE apply order)
        members.sort(key=lambda v: (0, gidx[v]) if kind[v] == "ins" else (1, -useq[v]))
        out.extend(members)
    return out

def main():
    pop = json.load(open(sys.argv[1]))
    event = (sys.argv[2] if len(sys.argv) > 2 else "event") == "event"
    bad = 0
    for e in pop:
        got = predict(e["scenario"], event)
        want = e["order"]
        if got != want:
            bad += 1
            if bad <= 12:
                print(f"  MISMATCH {e['scenario']['name']:12} predict={got} oracle={want}")
    print(f"{len(pop)} scenarios ({'event' if event else 'plain'} blocker): "
          f"{'ALL MATCH ✓' if bad == 0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main())
