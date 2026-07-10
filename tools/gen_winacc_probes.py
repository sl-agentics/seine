#!/usr/bin/env python3
"""A2 windowed-accumulate probe battery (D-154 recon) — the wa_*/m* cells.

Deterministic generator for the battery that validated the mechanism (see
~/.claude/plans/a2-winacc-mechanism-report.md and the docstring of
tools/model_check_winacc.py). At PORT time these graduate to
scenarios/probes/pr_cep_winacc_* — each currently FAILS engine-side by
design (they pin the oracle semantics the port must reproduce).

  usage: gen_winacc_probes.py [outdir]
  then:  cargo run -q -p seine-harness -- oracle <outdir>/*.json \
             > <outdir>/oracle.jsonl
         python3 tools/model_check_winacc.py <outdir>   # the spec gate

The wa_* set = the RT/queue/mask state machine (17 cells, all first-shot
oracle-confirmed). The m* set = the deferred-execution matrix (same-epoch
multi-updates evaluate FIFO against the epoch-final bean with per-entry
masks — settled by the BfDump PropagationList proxy).
"""
import json, os, sys

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    os.environ.get("WINACC_TMP", "/tmp/seine_winacc"), "probes")
os.makedirs(OUT, exist_ok=True)

# E: ts,val,tag. val-bound sum keeps ts OUT of the mask (mask={val}).
TYPES_BIG_EXP = [
    {"name": "E", "fields": [{"name": "ts", "type": "i64"}, {"name": "val", "type": "i64"},
                             {"name": "tag", "type": "String"}],
     "event": {"timestamp": "ts", "expires_ms": 100000}},
]
W_SUM = 'rule W when accumulate( E($v : val) over window:time(100ms); $c : sum($v) ) then end\n'
W_SUM_TAGX = 'rule W when accumulate( E(tag == "x", $v : val) over window:time(100ms); $c : sum($v) ) then end\n'
W_CNT_TAGX = 'rule W when accumulate( E(tag == "x") over window:time(100ms); $c : count() ) then end\n'

def sc(name, drl, facts, epochs, types=None, pred=""):
    return {"name": name, "types": types or TYPES_BIG_EXP, "drl": drl,
            "facts": facts, "epochs": epochs, "_pred": pred}

E = lambda ts, val, tag="x", ep=None: {**{"type": "E", "fields": {"ts": ts, "val": val, "tag": tag}},
                                       **({"entry_point": ep} if ep else {})}
adv = lambda ms: {"op": "advance", "ms": ms}
upd = lambda t, **f: {"op": "update", "target": t, "fields": f}
dele = lambda t: {"op": "delete", "target": t}
ep_ = lambda actions, facts=None: {"actions": actions, "facts": facts or []}

S = []
# 1. revival, mask-hit (34-analog via val, no ts write)
S.append(sc("wa_revive_maskhit", W_SUM, [E(0, 10)], [
    ep_([adv(150)]),                       # evict @100 -> W:0
    ep_([upd(0, val=20)]),                 # mask-hit revive -> W:20
], pred="W:10 | W:0 | W:20"))
# 2. mask-miss update leaves it out; later mask-hit revives
S.append(sc("wa_revive_maskmiss", W_SUM, [E(0, 10)], [
    ep_([adv(150)]),
    ep_([upd(0, tag="y")]),                # miss -> nothing
    ep_([upd(0, val=20)]),                 # hit -> W:20
], pred="W:10 | W:0 | (none) | W:20"))
# 3. zombie: revived member survives all deadlines; observed via later insert
S.append(sc("wa_zombie", W_SUM, [E(0, 10)], [
    ep_([adv(150)]),
    ep_([upd(0, val=20)]),
    ep_([adv(150)], [E(290, 5)]),          # clock 300; zombie(20)+5 -> W:25
], pred="W:10 | W:0 | W:20 | W:25"))
# 4. delete reaps the zombie
S.append(sc("wa_zombie_del", W_SUM, [E(0, 10)], [
    ep_([adv(150)]),
    ep_([upd(0, val=20)]),
    ep_([adv(150)], [E(290, 5)]),
    ep_([dele(0)]),                        # -> W:5
], pred="... | W:5"))
# 5. fresh admission rejected by SNAPSHOT ts even when live ts is in-window
S.append(sc("wa_fresh_reject_snap", W_SUM_TAGX, [E(0, 10, "z")], [
    ep_([adv(150)]),
    ep_([upd(0, tag="x", ts=140, val=20)]),  # snapshot 0+100<=150 -> REJECT
], pred="W:0 | (none) | (none)  [live-ts model would fire W:20]"))
# 6. fresh admission accepted inside window; evicted at snapshot ts+N
S.append(sc("wa_fresh_accept", W_SUM_TAGX, [E(0, 10, "z")], [
    ep_([adv(50)]),
    ep_([upd(0, tag="x")]),                # 0+100>50 -> admit -> W:10
    ep_([adv(60)]),                        # evict @100 -> W:0
], pred="W:0 | (none) | W:10 | W:0"))
# 7+8. stale-on-arrival insert rejected (no transient); later update REVIVES it
S.append(sc("wa_stale_ins_reject", W_SUM, [], [
    ep_([adv(150)], [E(0, 10)]),           # 0+100<=150 -> reject, no fire
], pred="W:0 | (none)"))
S.append(sc("wa_stale_ins_revive", W_SUM, [], [
    ep_([adv(150)], [E(0, 10)]),
    ep_([upd(0, val=20)]),                 # RT was created at insert -> revive
], pred="W:0 | (none) | W:20"))
# 9. alpha toggle chain: out un-gated; back-in mask-gated; late revive
S.append(sc("wa_toggle_stuck", W_SUM_TAGX, [E(0, 10, "x")], [
    ep_([adv(30)]),
    ep_([upd(0, tag="z")]),                # alpha-fail -> fold-out -> W:0
    ep_([upd(0, tag="x")]),                # alpha-pass, mask-miss -> stays OUT
    ep_([adv(120)]),                       # evict pops queue silently
    ep_([upd(0, val=20)]),                 # mask-hit -> revive -> W:20
], pred="W:10 | (none) | W:0 | (none) | (none) | W:20"))
# 10. toggle back BEFORE eviction: re-evicted at ts+N (still in queue)
S.append(sc("wa_toggle_reevict", W_SUM_TAGX, [E(0, 10, "x")], [
    ep_([adv(30)]),
    ep_([upd(0, tag="z", val=11)]),        # fold-out -> W:0
    ep_([upd(0, tag="x", val=12)]),        # mask-hit assert -> W:12
    ep_([adv(80)]),                        # clock 110: evict @100 -> W:0
], pred="W:10 | (none) | W:0 | W:12 | W:0"))
# 11. count() can never revive (mask always empty)
S.append(sc("wa_count_norevive", W_CNT_TAGX, [E(0, 10, "x")], [
    ep_([adv(150)]),                       # evict -> W:0
    ep_([upd(0, val=20)]),                 # miss
    ep_([upd(0, tag="x")]),                # miss (constraint not in mask)
], pred="W:1 | W:0 | (none) | (none)"))
# 12. two windows, one source: per-node split (revive in W1, re-fold in W2)
S.append(sc("wa_two_windows",
            'rule W1 when accumulate( E($v : val) over window:time(100ms); $c : sum($v) ) then end\n'
            'rule W2 when accumulate( E($v : val) over window:time(200ms); $c : sum($v) ) then end\n',
            [E(0, 10)], [
    ep_([adv(150)]),                       # W1 evicts -> W1:0 ; W2 keeps 10
    ep_([upd(0, val=20)]),                 # W1 revive:20 ; W2 re-fold:20
    ep_([adv(100)]),                       # clock 250: W2 evicts @200 -> W2:0; W1 zombie stays
], pred="W1:10 W2:10 | W1:0 | W1:20 W2:20 | W2:0"))
# 13+14. admission boundary: now == ts+N rejects; now == ts+N-1 admits
S.append(sc("wa_fresh_bnd_at", W_SUM_TAGX, [E(0, 10, "z")], [
    ep_([adv(100)]),
    ep_([upd(0, tag="x")]),                # 100<=100 -> REJECT
], pred="W:0 | (none) | (none)"))
S.append(sc("wa_fresh_bnd_in", W_SUM_TAGX, [E(0, 10, "z")], [
    ep_([adv(99)]),
    ep_([upd(0, tag="x")]),                # 100>99 -> admit -> W:10
    ep_([adv(1)]),                         # evict @100 -> W:0
], pred="W:0 | (none) | W:10 | W:0"))
# 15. entry-point routed fresh admission (routing sanity)
S.append(sc("wa_ep_fresh_accept",
            'rule W when accumulate( E(tag == "x", $v : val) over window:time(100ms) from entry-point "S1"; $c : sum($v) ) then end\n',
            [E(0, 10, "z", ep="S1")], [
    ep_([adv(50)]),
    ep_([upd(0, tag="x")]),
    ep_([adv(60)]),
], pred="W:0 | (none) | W:10 | W:0"))
# 16. @expires reaps a zombie
S.append(sc("wa_expire_zombie", W_SUM, [E(0, 10)], [
    ep_([adv(150)]),
    ep_([upd(0, val=20)]),                 # revive
    ep_([adv(250)]),                       # clock 400 > 0+300: expiry retract -> W:0
], types=[{"name": "E", "fields": [{"name": "ts", "type": "i64"}, {"name": "val", "type": "i64"},
                                   {"name": "tag", "type": "String"}],
           "event": {"timestamp": "ts", "expires_ms": 300}}],
    pred="W:10 | W:0 | W:20 | W:0"))
# 17. revival folds LIVE values of ALL fields incl. unwritten ones (ts bound case = 34 exact)
S.append(sc("wa_revive_ts_bound",
            'rule W when accumulate( E($t : ts) over window:time(100ms); $c : sum($t) ) then end\n',
            [E(10, 0)], [
    ep_([adv(150)]),                       # evict @110 -> W:0
    ep_([upd(0, ts=200)]),                 # mask-hit (ts bound) -> revive at live ts=200
], pred="W:10 | W:0 | W:200"))

# ---- the deferred-execution matrix (m*): same-epoch multi-updates.
# One rejected-at-insert event; W = sum($t:ts) win 150 cons tag=="x";
# advance 157 first. Updates queue as entries with their OWN masks and
# execute FIFO at the drain against the epoch-FINAL bean, so the z-state
# of a same-epoch z->x pair never evaluates (m3 fires 7 off the FIRST
# entry's {tag,ts} mask) while a fire boundary makes it real (m2: dead).
MWIN = ('rule W when accumulate( E(tag == "x", $t : ts) over '
        'window:time(150ms); $c : sum($t) ) then end\n')
MU = lambda **f: {"op": "update", "target": 0, "fields": f}
mfacts = [E(-453, 1, "x")]
mtwo = [E(-453, 1, "x"), E(-400, 2, "y")]

def msc(name, epochs, facts=None, pred=""):
    return {"name": name, "types": TYPES_BIG_EXP, "drl": MWIN,
            "facts": facts or [dict(f) for f in mfacts],
            "epochs": [ep_([adv(157)])] + epochs, "_pred": pred}

S.append(msc("m1_failts_only", [ep_([MU(tag="z", ts=7)])],
             pred="W:0 (final alpha-fail: nothing)"))
S.append(msc("m2_failts_then_pass",
             [ep_([MU(tag="z", ts=7)]), ep_([MU(tag="x")])],
             pred="W:0 (fire boundary between: z-state real, then {tag} miss)"))
S.append(msc("m3_pair_same_epoch", [ep_([MU(tag="z", ts=7), MU(tag="x")])],
             pred="W:0 | W:7 (entry1 evaluates final bean, {tag,ts} hits)"))
S.append(msc("m4_fail_then_passts", [ep_([MU(tag="z"), MU(tag="x", ts=7)])],
             pred="W:0 | W:7"))
S.append(msc("m5_ts_only", [ep_([MU(ts=7)])],
             pred="W:0 | W:7 (plain revival of a rejected insert)"))
S.append(msc("m6_failval_then_pass", [ep_([MU(tag="z", val=9), MU(tag="x")])],
             pred="W:0 (no entry carries ts: all miss)"))
S.append(msc("m8_failts_passval", [ep_([MU(tag="z", ts=7), MU(tag="x", val=9)])],
             pred="W:0 | W:7"))
S.append(msc("m10_del_between",
             [{"actions": [MU(tag="z", ts=7), {"op": "delete", "target": 1},
                           MU(tag="x")], "facts": []}],
             facts=[dict(f) for f in mtwo],
             pred="W:0 | W:7 (interleaved delete of another fact: no flush)"))
S.append(msc("m11_upd_between",
             [{"actions": [MU(tag="z", ts=7),
                           {"op": "update", "target": 1, "fields": {"val": 3}},
                           MU(tag="x")], "facts": []}],
             facts=[dict(f) for f in mtwo],
             pred="W:0 | W:7"))
S.append(msc("m12_adv_between",
             [{"actions": [MU(tag="z", ts=7), adv(1), MU(tag="x")],
               "facts": []}],
             pred="W:0 | W:7 (advance between: no flush either)"))
S.append(msc("m13_triple", [ep_([MU(tag="z", ts=7), MU(tag="y"), MU(tag="x")])],
             pred="W:0 | W:7"))
S.append({"name": "m14_infold_toggle", "types": TYPES_BIG_EXP, "drl": MWIN,
          "facts": [E(10, 1, "x")],
          "epochs": [ep_([adv(50)]), ep_([MU(tag="z"), MU(tag="x")])],
          "_pred": "W:10 only (out-and-back with no mask write = no-op)"})
S.append({"name": "m15_infold_toggle_ts", "types": TYPES_BIG_EXP, "drl": MWIN,
          "facts": [E(10, 1, "x")],
          "epochs": [ep_([adv(50)]), ep_([MU(tag="z", ts=25), MU(tag="x")])],
          "_pred": "W:10 | W:25 (single re-fold at final state)"})

for s in S:
    p = os.path.join(OUT, s["name"] + ".json")
    json.dump(s, open(p, "w"), indent=1)
print(f"{len(S)} probes -> {OUT}")
print("\n".join(s["name"] + "  PRED: " + s["_pred"] for s in S))
