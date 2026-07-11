#!/usr/bin/env python3
"""D-167 recon model v4 — self-join modify + watch-mask re-add + alpha entry.
Spec under test (s/r ladders + the D-166 v3 base):
 * 2-pattern rule: $a : E(tag=="z") $b : E(this before[lo,hi] $a) — SELF-join,
   or $a-type != $b-type (plain). Match ts frozen at insert (ts0).
 * modify(f): phase A = $a-side alpha transition (entry -> left-INSERT firing
   against the PRE-MOVE $b memory; exit -> left-DELETE, silent); phase B =
   $b-side child_upd refires (ALL live children of f-as-$b, FIFO) — heard
   regardless of the watch mask; phase C = tail RE-ADD of f in the $b memory
   iff the update changed ts (the watched field; tag-only never moves).
   Grid knobs: MOD_ORDER (A-before-B per s4; keep switchable), REFIRE_UNWATCHED.
 * left-INSERT of an anchor: scan $b memory FORWARD, addInsert PREPEND
   (one reversal); the anchor pairs with itself when it matches (ts0).
 * per-action batches FIFO; a tuple staged twice in one epoch fires once
   at its first position (D-166 u5).
Usage: model_tjupd_v4.py fuzz <n> <seed>   (live gate oracle, like v3)
"""
import json, os, sys, random, subprocess

CFG = {"BREF": "prepend",       # B-refire emission: prepend | forward
       "QPLACE": "fifo"}        # epoch queue: fifo | ins_block_first
ROOT = "/home/bryan/rust-rules"

def win(lo, hi, a_ts, b_ts):
    d = a_ts - b_ts          # before[lo,hi]: a.ts - b.ts in [lo,hi]
    return lo <= d <= hi

class M:
    """One temporal join node, $a alpha tag=='z'. Certified disciplines:
    arrival = left_insert (iff anchor) then right_insert (D-156 phase
    membership on the D-125 scan: forward scan + addInsert prepend, one
    reversal per batch); modify = phase A alpha transition (entry ->
    left_insert on the PRE-move memory; exit -> left_delete, silent),
    phase B $b child_upd refires (heard regardless of the watch mask),
    then a tail RE-ADD iff ts changed (the watched field). Rendering at
    epoch end; a tuple staged twice in one epoch fires once (first pos)."""
    def __init__(self, lo, hi):
        self.lo, self.hi = lo, hi
        self.rtm = []            # $b memory (fact refs, insertion order)
        self.ltm = []            # $a anchors present
        self.children = {}       # (a_id, b_id) alive
        self.childlist = {}      # a_id -> [b_id] append-created, refire moves to END
        self.buffer = []         # (a,b) refs awaiting epoch-end render
        self.fired = []
        self.pending = None

    def _emit(self, a, b, kind="ins"):
        key = (a["id"], b["id"])
        if self.pending is not None:
            if key in self.pending:
                return
            self.pending.add(key)
        self.buffer.append((a, b, kind))

    def render(self):
        buf = self.buffer
        if CFG["QPLACE"] == "ins_block_first":
            buf = [e for e in buf if e[2] == "ins"] + [e for e in buf if e[2] == "upd"]
        for a, b, _ in buf:
            self.fired.append(f"{a['ts']}{a['tag']}|{b['ts']}{b['tag']}")
        self.buffer = []

    def _match(self, a, b):
        d = a["ts0"] - b["ts0"]
        return self.lo <= d <= self.hi

    def left_insert(self, a):
        self.ltm.append(a)
        batch = []
        for b in self.rtm:
            if self._match(a, b):
                self.children[(a["id"], b["id"])] = True
                self.childlist.setdefault(a["id"], []).append(b["id"])
                batch.insert(0, b)           # addInsert prepend
        for b in batch:
            self._emit(a, b)

    def left_delete(self, a):
        self.ltm = [x for x in self.ltm if x["id"] != a["id"]]
        killed = [k for k in self.children if k[0] == a["id"]]
        for k in killed:
            del self.children[k]
        self.childlist.pop(a["id"], None)
        # matchCancelled: killed children drop their same-epoch pending fires
        kset = set(killed)
        self.buffer = [e for e in self.buffer if (e[0]["id"], e[1]["id"]) not in kset]
        if self.pending is not None:
            self.pending -= kset

    def right_insert(self, f):
        self.rtm.append(f)
        batch = []
        for a in self.ltm:
            if self._match(a, f):
                self.children[(a["id"], f["id"])] = True
                self.childlist.setdefault(a["id"], []).append(f["id"])
                batch.insert(0, a)
        for a in batch:
            self._emit(a, f)

    def right_refire(self, f):
        ups = []
        for a in self.ltm:
            if (a["id"], f["id"]) in self.children:
                if CFG["BREF"] == "prepend":
                    ups.insert(0, a)         # staged prepend (one reversal)
                else:
                    ups.append(a)            # forward ltm order
        for a in ups:
            cl = self.childlist.get(a["id"], [])
            if f["id"] in cl:
                cl.remove(f["id"]); cl.append(f["id"])
            self._emit(a, f, "upd")

    def left_refire(self, a):
        """In-place anchor update: refire the anchor's own children,
        REVERSED creation order (staged prepend), value-blind."""
        by_id = {f["id"]: f for f in self.rtm}
        for bid in reversed(self.childlist.get(a["id"], [])):
            b = by_id.get(bid)
            if b is not None and (a["id"], bid) in self.children:
                self._emit(a, b, "upd")

    def right_readd(self, f):
        if f in self.rtm:
            self.rtm.remove(f)
            self.rtm.append(f)

    def left_readd(self, f):
        for i, a in enumerate(self.ltm):
            if a["id"] == f["id"]:
                self.ltm.append(self.ltm.pop(i))
                return


def simulate(scn):
    node = M(scn["lo"], scn["hi"])
    facts = {}
    nid = 0
    def insert(ts, tag, is_b, is_a_type):
        nonlocal nid
        f = {"id": nid, "ts": ts, "ts0": ts, "tag": tag,
             "a_type": is_a_type, "b_type": is_b}
        facts[nid] = f; nid += 1
        if is_a_type and tag == "z":
            node.left_insert(f)              # D-156: left before self-right
        if is_b:
            node.right_insert(f)
        return f
    node.pending = set()
    for (ts, tag, kind) in scn["facts"]:
        insert(ts, tag, kind in ("b", "both"), kind in ("a", "both"))
    node.pending = None
    node.render()
    for ep in scn["epochs"]:
        node.pending = set()
        for act in ep["actions"]:
            f = facts[act["target"]]
            old_tag = f["tag"]
            new_tag = act.get("tag", old_tag)
            ts_changed = "ts" in act and act["ts"] != f["ts"]
            if "ts" in act:
                f["ts"] = act["ts"]          # printed only; ts0 frozen
            f["tag"] = new_tag
            was_anchor = f["a_type"] and old_tag == "z"
            now_anchor = f["a_type"] and new_tag == "z"
            # phase A: $a alpha transition (scan sees PRE-move memory);
            # in-place anchor update refires its own children (A')
            if not was_anchor and now_anchor:
                node.left_insert(f)
            elif was_anchor and not now_anchor:
                node.left_delete(f)
            elif was_anchor and now_anchor and "tag" in act:
                node.left_refire(f)          # $a watches tag only
            # phase B: $b-side child_upd refires (always heard)
            if f["b_type"]:
                node.right_refire(f)
            # phase C: EVERY update moves the $b tuple to the tail
            # (listen-all beta side; the alpha-entry scan above already
            # ran against the PRE-move memory); a surviving ANCHOR moves
            # to the tail of the left memory too (symmetric re-add)
            if f["b_type"]:
                node.right_readd(f)
            if f["a_type"] and f["tag"] == "z" and "tag" in act:
                node.left_readd(f)           # gated by the $a watch mask
        for (ts, tag, kind) in ep["facts"]:
            insert(ts, tag, kind in ("b", "both"), kind in ("a", "both"))
        node.pending = None
        node.render()
    return node.fired

# ---- scenario JSON (harness format) + oracle ----
ET = lambda n: {"name": n, "fields": [{"name": "ts", "type": "i64"}, {"name": "tag", "type": "String"}],
                "event": {"timestamp": "ts", "expires_ms": 100000}}

def to_harness(scn, name):
    if scn["self_join"]:
        types = [ET("E0")]
        drl = f'rule SJ when $a : E0(tag == "z") $b : E0(this before[{scn["lo"]}ms,{scn["hi"]}ms] $a) then end\n'
        tof = lambda kind: "E0"
    else:
        types = [ET("E0"), ET("E1")]
        drl = f'rule RJ when $a : E1(tag == "z") $b : E0(this before[{scn["lo"]}ms,{scn["hi"]}ms] $a) then end\n'
        tof = lambda kind: "E1" if kind == "a" else "E0"
    facts = [{"type": tof(k), "fields": {"ts": ts, "tag": tag}} for (ts, tag, k) in scn["facts"]]
    epochs = []
    for ep in scn["epochs"]:
        acts = []
        for a in ep["actions"]:
            fields = {}
            if "tag" in a: fields["tag"] = a["tag"]
            if "ts" in a: fields["ts"] = a["ts"]
            acts.append({"op": "update", "target": a["target"], "fields": fields})
        efacts = [{"type": tof(k), "fields": {"ts": ts, "tag": tag}} for (ts, tag, k) in ep["facts"]]
        epochs.append({"actions": acts, "facts": efacts})
    return {"name": name, "types": types, "drl": drl, "facts": facts, "epochs": epochs}

def oracle_seq(paths):
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
                       cwd=ROOT, capture_output=True, text=True, env=env)
    out = {}
    for ln in r.stdout.splitlines():
        try: j = json.loads(ln)
        except Exception: continue
        res = j.get("result")
        if res is None:
            out[j["scenario"]] = None; continue
        seq = []
        for fr in res["firings"]:
            ms = fr["matches"]
            seq.append(f"{ms[0]['fields']['ts']}{ms[0]['fields']['tag']}|{ms[1]['fields']['ts']}{ms[1]['fields']['tag']}")
        out[j["scenario"]] = seq
    return out

def gen(rng, name):
    self_join = rng.random() < 0.6
    lo, hi = 0, rng.choice([50, 100, 200])
    used = set()
    def ts():
        while True:
            t = rng.randint(0, 120)
            if t not in used:
                used.add(t); return t
    facts = []
    for _ in range(rng.randint(2, 5)):
        kind = "both" if self_join else rng.choice(["a", "b", "b"])
        tag = rng.choice(["y", "y", "z"])
        facts.append((ts(), tag, kind))
    epochs = []
    n_upd_targets = len(facts)
    for _ in range(rng.randint(1, 3)):
        acts = []
        for _ in range(rng.randint(1, 2)):
            tgt = rng.randrange(n_upd_targets)
            a = {"target": tgt}
            mode = rng.choice(["tag", "ts", "both", "tag"])
            if mode in ("tag", "both"):
                a["tag"] = rng.choice(["y", "z"])
            if mode in ("ts", "both"):
                a["ts"] = ts()
            acts.append(a)
        efacts = []
        for _ in range(rng.randint(0, 2)):
            kind = "both" if self_join else rng.choice(["a", "b"])
            efacts.append((ts(), rng.choice(["y", "z"]), kind))
        epochs.append({"actions": acts, "facts": efacts})
    return {"self_join": self_join, "lo": lo, "hi": hi, "facts": facts, "epochs": epochs}

def fuzz(n, seed):
    rng = random.Random(seed)
    OUT = "/tmp/model_tjupd_v4"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    while done < n:
        scns, paths = {}, []
        for i in range(done, min(done + 200, n)):
            s = gen(rng, f"tu{seed}x{i}")
            h = to_harness(s, f"tu{seed}x{i}")
            p = f"{OUT}/tu{seed}x{i}.json"
            json.dump(h, open(p, "w"))
            scns[h["name"]] = s
            paths.append(p)
        ora = oracle_seq(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            pred = simulate(scns[nm])
            if pred != ora.get(nm):
                ndiff += 1; diffs.append(nm)
        done += len(paths)
    print(f"v4 model-vs-oracle: {n} seed {seed}: {ndiff} div ({100*ndiff//max(1,n)}%)")
    if diffs: print("  ", " ".join(diffs[:20]))

if __name__ == "__main__":
    fuzz(int(sys.argv[2]), int(sys.argv[3])) if sys.argv[1] == "fuzz" else None
