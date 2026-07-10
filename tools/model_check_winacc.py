#!/usr/bin/env python3
"""A2 windowed-accumulate MECHANICAL MODEL (D-154 candidate) — the spec.

Replays winaccpop_* populations (fuzz_winacc.py) through a per-node fold
simulator and compares each accumulate rule's fired-VALUE sequence against
the banked oracle firings. 0-div on the population = the model is the
mechanism; the engine port then reproduces the model.

The mechanism (oracle-validated on the 17-probe wa_* battery, source-shaped
by WindowNode/SlidingTimeWindow/BetaNode.modifyObject — names only):

Per (windowed-accumulate node, event) THREE bits:
  rt        — "RightTuple at the window node": set on the FIRST alpha-pass
              (insert or update; the entry-point clause is part of alpha),
              NEVER cleared by eviction or alpha-fail modifies; cleared only
              by delete/expiry.
  in_queue  — window membership: set iff admission succeeded; admission runs
              ONLY on the rt 0->1 transition and checks the INSERT-SNAPSHOT
              ts:  ts0 + N > now  (isExpired uses <=; live ts is IRRELEVANT
              — wa_fresh_reject_snap). Eviction pops at exactly ts0+N.
  fold      — accumulate membership (folds LIVE field values at fire time).

Transitions:
  insert, alpha-pass:   rt=1; ts0+N>now ? (queue=1, fold-in) : REJECTED
                        (no transient — wa_stale_ins_reject; rt persists!).
  update,  rt=0:        alpha-pass -> same as insert admission (live fields
                        fold, snapshot admission — wa_fresh_accept/reject).
  update,  rt=1:        alpha-fail -> fold-out if in (UN-mask-gated;
                        WindowNode re-checks constraints on every modify).
                        alpha-pass -> mask-gated (windowed mask = source
                        BINDINGS ONLY, constraints dropped — D-139):

DEFERRED-EXECUTION UPDATES (the m1-m15 matrix + wf901x261, settled by the
BfDump PropagationList proxy): each external update ENQUEUES its own
propagation entry carrying ITS OWN written-mask; entries execute FIFO at
the batch drain (the epoch's fire), and each evaluates against the LIVE
bean — which by drain time holds the epoch-FINAL field state (the bean is
shared; intermediate states never reach the network). No mask merging.
So: an alpha-toggling intermediate write never evaluates (m14: in-fold
x->z->x with no mask write = complete no-op; m3: the z-state vanishes and
the FIRST entry's {tag,ts} mask carries the revival), a fire boundary
makes intermediate states real (m2: the z-state evaluates in its own
epoch, killing the RightTuple-side... no — constraint-fail evaluates,
fold-absent, nothing; the next epoch's {tag} entry then mask-misses), and
two entries can evaluate to DIFFERENT outcomes in one epoch as node state
evolves between them (wf901x261: entry 1 = fresh-admission REJECT sets rt,
entry 2 = mask-hit REVIVAL folds it in).
                          miss -> NOTHING (pr_cep_c_upd_evict_revive,
                                  wa_toggle_stuck: even an alpha fail->pass
                                  toggle stays out of the fold on a miss);
                          hit & in-fold  -> re-fold live (D-139 cell);
                          hit & out-of-fold -> fold-in live = REVIVAL
                                  (BetaNode.modifyObject: absent RightTuple
                                  + mask intersect -> assert). The queue is
                                  UNTOUCHED: revived-after-eviction = ZOMBIE
                                  (never evicted again — wa_zombie; only
                                  delete/expiry reap it); revived-before-
                                  eviction stays queued and re-evicts at
                                  ts0+N (wa_toggle_reevict).
  eviction (queue, ts0+N <= clock, EAGER at the advance — df_win_evict_ctl):
                        queue=0; fold-out if in. rt persists.
  delete:               all bits cleared, fold-out (immediate).
  expiry:               all bits cleared; the windowed fold-out is DEFERRED
                        to post-fire quiescence (trailing fire —
                        df_win_expire_reins transient); PLAIN accumulate
                        fold-outs are EAGER at the advance (D-112
                        eager_acc_removals; df_plain_expire_reins).

Plain (unwindowed) accumulate rules ride the same simulator with no
queue/admission and mask = constraints UNION bindings (listen_mask).

Expiry model (D-152/D-150, +1 boundary — wf905x127/x16 pinned it): deadline
D = ts0 + @expires + 1; D < 0 leaks (DROOLS-455, immortal — so ts+ex <= -2);
insert with 0 <= D <= clock = due-on-arrival (folds this cycle, dropped at
the same epoch's quiescence — ts+ex = -1 gives D = 0, due-on-arrival, NOT a
leak); an in-WM event expires when the clock reaches D (clock >= D; an
advance TO exactly ts+ex survives). Soup v1 generates explicit @expires
only (inferred expiry rides the fuzz_cep sweep).

Usage: model_check_winacc.py <popdir ...>   (dirs from fuzz_winacc.py)
"""
import json, os, sys, re

# ---------------------------------------------------------------- parsing

RULE_RE = re.compile(
    r'rule\s+(\w+)\s+when\s+accumulate\(\s*(\w+)\(([^)]*)\)'
    r'(?:\s+over window:time\((\d+)ms\))?'
    r'(?:\s+from entry-point "(\w+)")?'
    r'\s*;\s*\$c\s*:\s*(sum|count)\(([^)]*)\)\s*\)\s*then end', re.S)


def parse_rules(drl):
    rules = []
    for m in RULE_RE.finditer(drl):
        name, typ, inner, win, ep, fn, _arg = m.groups()
        cons = None
        bind = None
        for part in [p.strip() for p in inner.split(",") if p.strip()]:
            cm = re.match(r'tag\s*==\s*"(\w)"', part)
            bm = re.match(r'\$\w+\s*:\s*(\w+)', part)
            if cm:
                cons = cm.group(1)
            elif bm:
                bind = bm.group(1)
        rules.append({
            "name": name, "type": typ, "cons": cons, "bind": bind,
            "win": int(win) if win else None, "ep": ep, "fn": fn,
        })
    return rules


class Node:
    def __init__(self, rule):
        self.r = rule
        self.rt = set()
        self.queue = {}     # fid -> eviction deadline (ts0 + N)
        self.fold = set()
        self.changed = False
        self.fired = []
        if rule["win"] is not None:
            self.mask = {rule["bind"]} if rule["bind"] else set()
        else:
            self.mask = set()
            if rule["bind"]:
                self.mask.add(rule["bind"])
            if rule["cons"]:
                self.mask.add("tag")

    def alpha(self, f):
        r = self.r
        if f["type"] != r["type"]:
            return False
        if f.get("ep") != r["ep"]:
            return False
        return r["cons"] is None or f["fields"].get("tag") == r["cons"]

    def value(self, facts):
        if self.r["fn"] == "count":
            return len(self.fold)
        b = self.r["bind"]
        return sum(facts[fid]["fields"][b] for fid in self.fold)

    def on_insert(self, fid, f, clock):
        if not self.alpha(f):
            return
        self.rt.add(fid)
        if self.r["win"] is not None:
            dl = f["ts0"] + self.r["win"]
            if dl <= clock:
                return                      # rejected on arrival, no transient
            self.queue[fid] = dl
        self.fold.add(fid)
        self.changed = True

    def on_update(self, fid, f, written, clock):
        if f["type"] != self.r["type"] or f.get("ep") != self.r["ep"]:
            return
        a = self.alpha(f)
        if self.r["win"] is None:
            if fid in self.fold:
                if not a:
                    self.fold.discard(fid)
                    self.changed = True
                elif written & self.mask:
                    self.changed = True     # re-fold live
            elif a and (written & self.mask):
                self.fold.add(fid)
                self.changed = True
            return
        if fid not in self.rt:
            if not a:
                return
            self.on_insert(fid, f, clock)   # fresh admission (snapshot check)
            return
        if not a:
            if fid in self.fold:
                self.fold.discard(fid)      # constraint re-check, un-mask-gated
                self.changed = True
            return
        if written & self.mask:
            if fid not in self.fold:
                self.fold.add(fid)          # revival (queue untouched)
            self.changed = True

    def on_remove(self, fid, windowed_defer):
        """delete (immediate) or expiry (windowed_defer=True -> caller defers)."""
        self.rt.discard(fid)
        self.queue.pop(fid, None)
        if fid in self.fold:
            self.fold.discard(fid)
            self.changed = True

    def on_advance(self, clock):
        due = [fid for fid, dl in self.queue.items() if dl <= clock]
        for fid in due:
            del self.queue[fid]
            if fid in self.fold:
                self.fold.discard(fid)
                self.changed = True


def simulate(sc):
    rules = parse_rules(sc["drl"])
    nodes = [Node(r) for r in rules]
    facts = {}
    expiry = {t["name"]: t.get("event", {}).get("expires_ms")
              for t in sc["types"]}
    clock = 0
    fid = 0

    def deadline(f):
        ex = expiry.get(f["type"])
        return None if ex is None else f["ts0"] + ex + 1

    def do_insert(fact, due_arrival_bin):
        nonlocal fid
        f = {"type": fact["type"], "fields": dict(fact["fields"]),
             "ep": fact.get("entry_point"), "alive": True}
        f["ts0"] = f["fields"].get("ts", 0)
        facts[fid] = f
        for n in nodes:
            n.on_insert(fid, f, clock)
        d = deadline(f)
        if d is not None and 0 <= d <= clock:
            due_arrival_bin.append(fid)     # due-on-arrival: drop at quiescence
        fid += 1

    def fire(pending_exp):
        out = []
        for n in nodes:
            if n.changed:
                out.append((n.r["name"], n.value(facts)))
                n.changed = False
        # quiescence: deferred windowed-expiry fold-outs -> trailing fire
        if pending_exp:
            for x in pending_exp:
                facts[x]["alive"] = False
                for n in nodes:
                    n.on_remove(x, True)
            for n in nodes:
                if n.changed:
                    out.append((n.r["name"], n.value(facts)))
                    n.changed = False
        return out

    firings = []
    pending = []
    for fact in sc.get("facts", []):
        do_insert(fact, pending)
    for n in nodes:
        n.changed = True    # cycle 0: every accumulate fires its initial value
    firings += fire(pending)

    for ep in sc.get("epochs", []):
        pending = []
        entries = []
        for a in ep.get("actions", []):
            if a["op"] == "advance":
                clock += a["ms"]
                for n in nodes:
                    n.on_advance(clock)     # eviction EAGER
                for x, f in facts.items():
                    if not f["alive"]:
                        continue
                    d = deadline(f)
                    if d is not None and d >= 0 and clock >= d:
                        # expiry crossed: PLAIN nodes drop eagerly; windowed
                        # nodes defer to quiescence (trailing fire)
                        windowed_hit = False
                        for n in nodes:
                            if n.r["win"] is None:
                                n.on_remove(x, False)
                            elif x in n.fold:
                                windowed_hit = True
                        if windowed_hit or any(n.r["win"] is not None for n in nodes):
                            if x not in pending:
                                pending.append(x)
                        else:
                            facts[x]["alive"] = False
            elif a["op"] == "update":
                # fields apply to the shared bean NOW (only the epoch-final
                # state ever evaluates); the entry itself executes at drain
                t = a["target"]
                facts[t]["fields"].update(a["fields"])
                entries.append(("upd", t, frozenset(a["fields"].keys())))
            elif a["op"] == "delete":
                entries.append(("del", a["target"], None))
        # drain: entries execute FIFO against the final bean state, each
        # update with its OWN written-mask (BfDump: no merging)
        for kind, t, written in entries:
            if kind == "upd":
                if facts[t]["alive"]:
                    for n in nodes:
                        n.on_update(t, facts[t], written, clock)
            elif facts[t]["alive"]:
                facts[t]["alive"] = False
                for n in nodes:
                    n.on_remove(t, False)
        for fact in ep.get("facts", []):
            do_insert(fact, pending)
        firings += fire(pending)
    return firings


# ---------------------------------------------------------------- checking

def oracle_seq(result, rulenames):
    seqs = {r: [] for r in rulenames}
    for f in result["firings"]:
        if f["rule"] in seqs:
            vals = [m["fields"]["value"] for m in f["matches"] if m["type"] == "Long"]
            if len(vals) == 1:
                seqs[f["rule"]].append(vals[0])
    return seqs


def check_dir(d):
    ok = div = 0
    bank = os.path.join(d, "oracle.jsonl")
    results = {}
    nerr = 0
    for line in open(bank):
        r = json.loads(line)
        if "result" in r:
            results[r["scenario"]] = r["result"]
        else:
            nerr += 1
            print(f"ORACLE-ERR {r.get('scenario')}: {str(r.get('error'))[:120]}")
    if nerr:
        print(f"WARNING: {nerr} oracle errors in {bank} (excluded from check)")
    for name, result in sorted(results.items()):
        sc = json.load(open(os.path.join(d, name + ".json")))
        rules = [r["name"] for r in parse_rules(sc["drl"])]
        want = oracle_seq(result, rules)
        got = {r: [] for r in rules}
        try:
            for rn, v in simulate(sc):
                got[rn].append(v)
        except Exception as e:
            print(f"DIV {name}: simulator error {e!r}")
            div += 1
            continue
        if got != want:
            div += 1
            print(f"DIV {name}:")
            for rn in rules:
                if got[rn] != want[rn]:
                    print(f"   {rn}: model {got[rn]}  oracle {want[rn]}")
        else:
            ok += 1
    return ok, div


if __name__ == "__main__":
    tot_ok = tot_div = 0
    for d in sys.argv[1:]:
        ok, div = check_dir(d)
        print(f"{d}: {ok} ok, {div} div")
        tot_ok += ok
        tot_div += div
    print(f"TOTAL: {tot_ok} ok, {tot_div} div")
    sys.exit(1 if tot_div else 0)
