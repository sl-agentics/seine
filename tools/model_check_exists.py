#!/usr/bin/env python3
"""CEP item-1b Family B (exists) — `exists E1() P()` RE-FIRE order model (D-144).
Validates a PREDICT against the oracle firing SEQUENCE from fuzz_existsorder.py.
0-div on the CLEAN regime (delete + single toggle, EMODEL=epoch); the full
multi-toggle/expiry populations are validated engine-vs-oracle by `diff` (the
simplified batch-structure sim here does not replicate expiry transient fires).

THE RULE (cracked 2026-07-09): P's fire when the witness EXISTS; each satisfy
transition (live 0->1) re-fires the whole held memory.
  - the FIRST satisfaction fires the accumulated P's FIFO (insertion order);
  - every RE-FIRE (after the witness toggles) uses the D-140 EPOCH model: batch
    by last-touch epoch, REVERSE (newest first), the INITIAL epoch LAST; within a
    batch INSERTS (insertion order) then UPDATES (newest apply first). A P updated
    in a later epoch re-stages into that (newest) batch.
So exists re-fire == the D-140 blocker-first `not` order (`not_order_key`), NOT
the D-143 P-first SEGMENT model. FENCED tail (regime 2): a P inserted in the
SATISFYING epoch (before/after the re-arrival witness) — cf407x121's NE6 residual.
EMODEL=epoch (default) | seg (the rejected mirror-of-not variant, kept for record).
Usage: model_check_exists.py <existspop_*.json>
"""
import json, os, sys


def predict(scn, model=None):
    import os as _os
    model = model or _os.environ.get("EMODEL", "epoch")
    seg = [0]
    epoch = [0]       # fire-boundary (scenario epoch) index
    gidx = {}; ins_seg = {}; upd_seg = {}; upd_app = {}; is_upd = {}
    ins_epoch = {}; upd_epoch = {}
    vof = {}; idx = [0]; app = [0]
    ts_of = {}        # E1 fact idx -> deadline (ts + 100)
    live = set()      # live E1 fact indices
    clock = [0]
    firings = []
    fired = [False]  # has the exists been satisfied (fired) before?

    def order_now():
        ps = list(gidx)
        if not fired[0]:
            # FIRST satisfaction: fire the accumulated P's FIFO (insertion order).
            # Only after the witness TOGGLES do the epoch batches reverse.
            return sorted(ps, key=lambda v: gidx[v])
        if model == "epoch":
            # D-140-style EPOCH batches: batch = last-touch epoch; reverse
            # (newest first), the INITIAL epoch (0) LAST; within a batch INSERTS
            # (gidx) then UPDATES (newest apply first).
            batch = {v: (upd_epoch[v] if is_upd.get(v) else ins_epoch[v]) for v in ps}
            ebs = sorted({batch[v] for v in ps if batch[v] >= 1}, reverse=True)
            out = []
            for b in ebs + [0]:
                mem = [v for v in ps if batch[v] == b]
                inss = sorted((v for v in mem if not is_upd.get(v)), key=lambda v: gidx[v])
                upds = sorted((v for v in mem if is_upd.get(v)), key=lambda v: -upd_app[v])
                out.extend(inss + upds)
            return out
        # seg model (mirror of not, within-segment flipped)
        seg_of = {}; role = {}
        for v in ps:
            if is_upd.get(v):
                seg_of[v] = upd_seg[v]; role[v] = 1
            else:
                seg_of[v] = ins_seg[v]; role[v] = 0
        out = []
        for s in sorted({seg_of[v] for v in ps}, reverse=True):
            mem = [v for v in ps if seg_of[v] == s]
            upds = sorted((v for v in mem if role[v] == 1), key=lambda v: -upd_app[v])
            inss = sorted((v for v in mem if role[v] == 0), key=lambda v: gidx[v])
            out.extend(upds + inss)
        return out

    def do_insert(f, fi):
        if f["type"] == "E1":
            # expiry check happens at advance; here just track liveness
            was = len(live)
            live.add(fi)
            ts_of[fi] = f["fields"]["ts"] + 100
            if was == 0:            # unsatisfied -> satisfied: RE-FIRE (or cycle 1)
                firings.extend(order_now())
                fired[0] = True
            seg[0] += 1             # segment boundary (like not's E0 insert)
        else:  # P
            v = f["fields"]["v"]; vof[fi] = v
            gidx[v] = idx[0]; ins_seg[v] = seg[0]; upd_seg[v] = seg[0]; is_upd[v] = False
            ins_epoch[v] = epoch[0]; upd_epoch[v] = epoch[0]

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg[0]; upd_app[v] = app[0]; is_upd[v] = True; app[0] += 1
            upd_epoch[v] = epoch[0]

    def do_delete(a):
        live.discard(a["target"])

    def do_advance(ms):
        clock[0] += ms
        for fi in list(live):
            if clock[0] >= ts_of[fi]:
                live.discard(fi)

    fi = 0
    for f in scn["facts"]:
        do_insert(f, fi); fi += 1
    for ep in scn["epochs"]:
        epoch[0] += 1
        for a in ep["actions"]:
            if a["op"] == "update": do_update(a)
            elif a["op"] == "delete": do_delete(a)
            elif a["op"] == "advance": do_advance(a["ms"])
            elif a["op"] == "insert": do_insert(a, fi); fi += 1
        for f in ep["facts"]:
            do_insert(f, fi); fi += 1
    return firings


def main():
    pop = json.load(open(sys.argv[1]))
    bad = 0
    for e in pop:
        got = predict(e["scenario"])
        want = [v for v in e["firings"] if v is not None]
        if got != want:
            bad += 1
            if bad <= 14:
                print(f"  MISMATCH {e['scenario']['name']:12} predict={got} oracle={want}")
    print(f"{len(pop)} scenarios: {'ALL MATCH ✓' if bad == 0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
