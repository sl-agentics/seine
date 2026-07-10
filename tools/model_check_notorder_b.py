#!/usr/bin/env python3
"""CEP item-1b Family B — MODEL for the EVENT-not EXPIRY firing order (P-FIRST
regime). Validates a PREDICT against the oracle population from fuzz_notorder_b.py
(0-div is the port spec). CRACKED 2026-07-09 (D-143): the SEGMENT model.

THE RULE (`not E0() P()`, blocked P's fire at the expiry-unblock advance):
  Process ops chronologically. A SEGMENT counter increments on each E0 INSERT
  (the initial blocker AND every mid-run arrival). Each P records:
    - ins_seg: the segment current at its insert;
    - upd_seg / upd_app: the segment current at its LAST update + a global apply
      sequence (only if updated).
  FINAL PLACEMENT: a P updated into a LATER segment than its insert MOVES to that
  segment's UPDATES sublist; a same-segment update leaves it an INSERT (no move).
  FIRE ORDER = segments in REVERSE index order (newest first); within a segment,
  INSERTS first (insertion order = gidx asc) then UPDATES (apply order DESC =
  last-updated first).

Why segments (not epochs, the D-140 model): D-140's `fuzz_notorder` was BLOCKER-
FIRST, where every epoch flush is blocked ⇒ each epoch is its own segment (epoch
reversal). In the P-FIRST regime (a P inserted before the blocker — the real
witness cf401x362, and this population) epoch boundaries do NOT segment; only E0
inserts do. So D-140's epoch key is the blocker-first special case; this segment
model is the general P-first rule. (blocker-first is NOT reproduced by this model
— it is a separate regime; the port branches on it. Verified 0-div on 2671+
P-first scenarios; the D-140 pins are blocker-first and unaffected.)

Usage:  model_check_notorder_b.py <notpop_b_*.json> [MODEL]
  MODEL=seg (default, the cracked spec) | d140 (the blocker-first special case)
"""
import json, os, sys


def predict_seg(scn):
    seg = [0]
    gidx = {}; ins_seg = {}; upd_seg = {}; upd_app = {}; vof = {}
    idx = [0]; app = [0]

    def do_insert(f):
        if f["type"] == "E0":
            seg[0] += 1
        else:  # a positive-pattern (P) fact
            v = f["fields"]["v"]; vof[idx[0]] = v
            gidx[v] = idx[0]; ins_seg[v] = seg[0]
        idx[0] += 1

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg[0]; upd_app[v] = app[0]; app[0] += 1

    for f in scn["facts"]:
        do_insert(f)
    for ep in scn["epochs"]:
        for a in ep["actions"]:
            if a["op"] == "update":
                do_update(a)
            elif a["op"] == "insert":
                do_insert(a)
            # advance / delete: no segmentation effect
        for f in ep["facts"]:
            do_insert(f)

    ps = list(gidx)
    seg_of = {}; is_upd = {}
    for v in ps:
        if v in upd_seg and upd_seg[v] != ins_seg[v]:
            seg_of[v], is_upd[v] = upd_seg[v], True
        else:
            seg_of[v], is_upd[v] = ins_seg[v], False
    out = []
    for s in sorted({seg_of[v] for v in ps}, reverse=True):
        mem = [v for v in ps if seg_of[v] == s]
        inserts = sorted((v for v in mem if not is_upd[v]), key=lambda v: gidx[v])
        updates = sorted((v for v in mem if is_upd[v]), key=lambda v: -upd_app[v])
        out.extend(inserts + updates)
    return out


def predict_d140(scn):
    """The D-140 blocker-first model (last-touch EPOCH batch, reverse, initial
    last; within batch inserts then updates). Diverges on the P-first population;
    kept for the bisect."""
    facts = scn["facts"]
    gidx = {}; vof = {}; batch = {}; kind = {}; useq = {}
    idx = 0; app = [0]
    for f in facts:
        if f["type"] == "P":
            v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = 0; kind[v] = "ins"
        idx += 1
    for e_i, ep in enumerate(scn["epochs"]):
        for a in ep["actions"]:
            if a["op"] == "update":
                v = vof.get(a["target"])
                if v is not None:
                    batch[v] = e_i + 1; kind[v] = "upd"; useq[v] = app[0]; app[0] += 1
        for f in ep["facts"]:
            if f["type"] == "P":
                v = f["fields"]["v"]; gidx[v] = idx; vof[idx] = v; batch[v] = e_i + 1; kind[v] = "ins"
            idx += 1
    ps = list(gidx)
    ebs = sorted({batch[v] for v in ps if batch[v] >= 1})
    out = []
    for b in list(reversed(ebs)) + [0]:
        members = [v for v in ps if batch[v] == b]
        members.sort(key=lambda v: (0, gidx[v]) if kind[v] == "ins" else (1, -useq[v]))
        out.extend(members)
    return out


def predict_seg2(scn):
    """D-146 UNIFIED rule (mixed initial positions, `xf_cep_not_order_mixed_initial`).
    BLOCKER-FIRST (no initial P before the first E0) -> the D-140 epoch model.
    Else (P-first / MIXED): segments (E0-insert count) NEWEST-first as D-143; WITHIN
    a segment three classes: [epoch>=1 inserts, gidx asc] ++ [updates, apply-seq
    DESC] ++ [EPOCH-0 INITIALS, gidx asc — the last-class tail]. Class moves: a P
    updated into a LATER segment moves there as an update (D-143); an EPOCH-0
    initial updated AT ALL (even same-segment) promotes into the updates slot
    (D-145 m_updP2mid); an epoch>=1 insert updated same-segment stays an insert
    (D-143 nb801x0)."""
    seg = 0
    gidx = {}; ins_seg = {}; ins_epoch = {}; upd_seg = {}; upd_app = {}; is_upd = {}
    vof = {}
    idx = 0; app = [0]
    first_e0 = first_p = None

    def do_insert(f, epoch_no):
        nonlocal seg, idx, first_e0, first_p
        if f["type"] == "E0":
            if first_e0 is None:
                first_e0 = idx
            seg += 1
        else:
            v = f["fields"]["v"]; vof[idx] = v
            if first_p is None:
                first_p = idx
            gidx[v] = idx; ins_seg[v] = seg; ins_epoch[v] = epoch_no; is_upd[v] = False
        idx += 1

    def do_update(a):
        v = vof.get(a["target"])
        if v is not None:
            upd_seg[v] = seg; upd_app[v] = app[0]; is_upd[v] = True; app[0] += 1

    for f in scn["facts"]:
        do_insert(f, 0)
    for e_i, ep in enumerate(scn["epochs"]):
        for a in ep["actions"]:
            if a["op"] == "update":
                do_update(a)
            elif a["op"] == "insert":
                do_insert(a, e_i + 1)
        for f in ep["facts"]:
            do_insert(f, e_i + 1)

    blocker_first = first_e0 is not None and (first_p is None or first_e0 < first_p)
    if blocker_first:
        return predict_d140(scn)

    ps = list(gidx)
    seg_of = {}; cls = {}
    for v in ps:
        moved = is_upd[v] and (upd_seg[v] > ins_seg[v] or ins_epoch[v] == 0)
        if moved:
            seg_of[v] = upd_seg[v]; cls[v] = 1
        elif ins_epoch[v] == 0:
            seg_of[v] = ins_seg[v]; cls[v] = 2
        else:
            seg_of[v] = ins_seg[v]; cls[v] = 0
    out = []
    for s in sorted({seg_of[v] for v in ps}, reverse=True):
        mem = [v for v in ps if seg_of[v] == s]
        mem.sort(key=lambda v: (cls[v], -upd_app[v] if cls[v] == 1 else gidx[v]))
        out.extend(mem)
    return out


def predict(scn, model="seg"):
    if model == "d140":
        return predict_d140(scn)
    if model == "seg2":
        return predict_seg2(scn)
    return predict_seg(scn)


def main():
    pop = json.load(open(sys.argv[1]))
    model = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("MODEL", "seg")
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
