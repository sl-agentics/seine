#!/usr/bin/env python3
"""CEP item-1b Family B — model for the EVENT-not EXPIRY-with-UPDATE firing order.
Validates a PREDICT against the oracle population from fuzz_notorder_b.py (0-div
is the port spec). Starts from the D-140 predict (MODEL=d140); refined variants
are selected by the MODEL env var so the divergence can be bisected.

Usage:  model_check_notorder_b.py <notpop_b_*.json> [MODEL]   (or MODEL=... env)
"""
import json, os, sys


def predict(scn, model):
    facts = scn["facts"]
    gidx = {}          # v -> global insertion index
    vof = {}           # global insertion index -> v
    ins_epoch = {}     # v -> INSERT epoch (0 = initial)
    batch = {}         # v -> last-touch epoch
    kind = {}          # v -> "ins" | "upd" (last touch)
    useq = {}          # v -> global apply-seq of the P's LAST update
    idx = 0
    app = [0]
    for f in facts:
        if f["type"] == "P":
            v = f["fields"]["v"]
            gidx[v] = idx; vof[idx] = v; ins_epoch[v] = 0; batch[v] = 0; kind[v] = "ins"
        idx += 1
    n_ep = len(scn["epochs"])
    for e_i, ep in enumerate(scn["epochs"]):
        epoch_no = e_i + 1
        is_final = (e_i == n_ep - 1)
        for a in ep["actions"]:
            if a["op"] == "update":
                v = vof.get(a["target"])
                if v is not None:
                    batch[v] = epoch_no; kind[v] = "upd"; useq[v] = app[0]; app[0] += 1
                    if model in ("ignore_final_upd", "b1") and is_final:
                        # a same-(unblock)-epoch update does NOT re-stage: revert
                        # the batch to the P's prior touch (insert or prior update)
                        pass  # handled below via ins_epoch fallback
        for f in ep["facts"]:
            if f["type"] == "P":
                v = f["fields"]["v"]
                gidx[v] = idx; vof[idx] = v; ins_epoch[v] = epoch_no; batch[v] = epoch_no; kind[v] = "ins"
            idx += 1

    ps = list(gidx)
    final_ep = n_ep  # the unblock epoch number

    def batch_of(v):
        if model in ("ignore_final_upd", "b1"):
            # updates in the UNBLOCK epoch do not promote — use the insert epoch
            # (the probe result: oracle ignores an unblock-epoch update)
            if kind.get(v) == "upd" and batch[v] == final_ep:
                return ins_epoch[v]
            return batch[v]
        return batch[v]  # d140: last-touch epoch

    def is_upd(v):
        if model in ("ignore_final_upd", "b1"):
            return kind.get(v) == "upd" and batch[v] != final_ep
        return kind.get(v) == "upd"

    ebatches = sorted({batch_of(v) for v in ps if batch_of(v) >= 1})
    order_batches = list(reversed(ebatches)) + [0]  # event blocker
    out = []
    for b in order_batches:
        members = [v for v in ps if batch_of(v) == b]
        members.sort(key=lambda v: (1, -useq[v]) if is_upd(v) else (0, gidx[v]))
        out.extend(members)
    return out


def main():
    pop = json.load(open(sys.argv[1]))
    model = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("MODEL", "d140")
    bad = 0
    for e in pop:
        got = predict(e["scenario"], model)
        want = [v for v in e["order"] if v is not None]
        if got != want:
            bad += 1
            if bad <= 14:
                print(f"  MISMATCH {e['scenario']['name']:12} predict={got} oracle={want}")
    print(f"{len(pop)} scenarios (MODEL={model}): {'ALL MATCH ✓' if bad == 0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
