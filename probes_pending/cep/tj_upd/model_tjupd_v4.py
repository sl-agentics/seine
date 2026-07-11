#!/usr/bin/env python3
"""D-167 recon model v4 / D-169 v5 — self-join modify composition, now with
the DOUBLE-TOUCH sub-rules (the last D-167 §2 residual), pinned by the
5-round ladder `ladder_dt*.py` (31 oracle-verified cells, all STABLE 2x).

Base (D-167, unchanged): 2-pattern rule $a : E(tag=="z") $b : E(this
before[lo,hi] $a) — SELF-join or plain 2-type. Match ts frozen at insert
(ts0). modify(f): phase A = $a alpha transition (entry -> left-INSERT scan,
exit -> left-DELETE + matchCancelled purge, in-place tag-touch -> A' refire
of f's children in REVERSED child-list order); phase B = $b child refires
(listen-all); phase C = tail RE-ADD in the $b memory on EVERY update (+ the
surviving anchor's left-memory re-add on tag-touch). Entry scans FORWARD +
addInsert PREPEND (one reversal per batch); per-action batches FIFO.

D-169 double-touch sub-rules (T6):
 1. EMISSION CLASS: an upd emission staged by a TAG-WRITING action
    (noop y->y, both-fields, in-place z->z, exit z->y) is MOVABLE-by-f;
    ts-only actions stage ANCHORED emissions.
 2. RELOCATION: re-emitting a movable emission during a LATER alpha-ENTRY
    OF THE SAME fact (the entry's phase-B) MOVES it to the current
    emission point (dt1/dt2/tb1/dr1/tv3). Anchored, different-fact,
    same-action, or non-entry re-emissions keep their first position
    (dup1/int1/int2/ip1/ex10/x84 — the u5 discipline); ins-staged
    emissions absorb re-touches (x126 self-pair).
 3. MOVES ARE IMMEDIATE (post-scan) for every update class — exits included
    (ex7 insert-scan, ex4/ex6/en1/en3/tv1/tv2, ex1 durability). ⚠ ORACLE
    FLAKE: the exit-move's visibility to a LATER same-epoch DIFFERENT-fact
    ENTRY scan is JVM-nondeterministic (ex9: 16 moved / 2 unmoved across
    JVM instances, each internally consistent — the fz_42_84 class). The
    model encodes the moved majority; ex9's exact shape is not pinnable.
 4. SELF-SLOT: an ENTRY's scan sees the entering fact ITSELF at its
    pre-epoch slot when its same-epoch moves were tag-class — exits
    included (x56/x227/dt2b/x145/tv3) — but at its moved slot after
    ts-only moves (en3); other facts always at their current positions.
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
    """One temporal join node, $a alpha tag=='z'. Certified base disciplines
    (D-167) + the D-169 T6 double-touch sub-rules — see the module docstring.
    Per-action context (cur_fact/cur_tagwrite) drives emission movability and
    relocation; exit $b-moves defer to insert-time/epoch-end; entry scans use
    scan_rtm() for the self-slot rule."""
    def __init__(self, lo, hi):
        self.lo, self.hi = lo, hi
        self.rtm = []            # $b memory (fact refs, insertion order)
        self.ltm = []            # $a anchors present
        self.children = {}       # (a_id, b_id) alive
        self.childlist = {}      # a_id -> [b_id] append-created, refire moves to END
        self.buffer = []         # emission dicts awaiting epoch-end render
        self.entries = {}        # key -> live buffer entry (dedup/relocation)
        self.fired = []
        self.pending = None
        self.cur_fact = None     # current action's target fact id
        self.cur_tagwrite = False
        self.cur_entry = False   # current action alpha-ENTERS its fact
        self.act_no = 0          # action counter (relocation needs LATER)
        self.epoch_start = []    # rtm ids at epoch start (self-slot replay)
        self.epoch_log = []      # ("move", id, tagclass) | ("ins", id)
        self.self_dirty = set()  # facts tag-class-moved this epoch

    def _emit(self, a, b, kind="ins"):
        key = (a["id"], b["id"])
        movable = self.cur_fact if (kind == "upd" and self.cur_tagwrite) else None
        if self.pending is not None:
            if key in self.pending:
                e = self.entries.get(key)
                # T6-2: relocation — a MOVABLE emission re-emitted by a LATER
                # alpha-ENTRY of the SAME fact moves to the current position;
                # anchored / different-fact / same-action / non-entry /
                # ins-staged stay (u5, absorb; dt4/ip1/x84: A'-and-in-place
                # re-touches keep first).
                if (e is not None and e["kind"] == "upd"
                        and e["movable"] is not None
                        and e["movable"] == self.cur_fact
                        and e["act"] != self.act_no
                        and self.cur_entry):
                    self.buffer.remove(e)
                    e2 = {"a": a, "b": b, "kind": "upd", "movable": movable,
                          "act": self.act_no}
                    self.buffer.append(e2)
                    self.entries[key] = e2
                return
            self.pending.add(key)
        e = {"a": a, "b": b, "kind": kind, "movable": movable,
             "act": self.act_no}
        self.buffer.append(e)
        self.entries[key] = e

    def render(self):
        buf = self.buffer
        if CFG["QPLACE"] == "ins_block_first":
            buf = ([e for e in buf if e["kind"] == "ins"]
                   + [e for e in buf if e["kind"] == "upd"])
        for e in buf:
            a, b = e["a"], e["b"]
            self.fired.append(f"{a['ts']}{a['tag']}|{b['ts']}{b['tag']}")
        self.buffer = []
        self.entries = {}

    def _match(self, a, b):
        d = a["ts0"] - b["ts0"]
        return self.lo <= d <= self.hi

    def scan_rtm(self, f):
        """T6-4: the entry scan's view. Others at their CURRENT slots
        (immediate moves applied; exit moves absent from rtm anyway); the
        entering fact ITSELF at its pre-epoch slot when its same-epoch
        moves were tag-class — replay the epoch log skipping f's tag moves."""
        if f["id"] not in self.self_dirty:
            return self.rtm
        by_id = {x["id"]: x for x in self.rtm}
        order = list(self.epoch_start)
        for op in self.epoch_log:
            if op[0] == "move":
                _, fid, tagclass = op
                if fid == f["id"] and tagclass:
                    continue
                if fid in order:
                    order.remove(fid)
                    order.append(fid)
            else:                            # ("ins", id)
                order.append(op[1])
        return [by_id[i] for i in order if i in by_id]

    def left_insert(self, a):
        self.ltm.append(a)
        batch = []
        for b in self.scan_rtm(a):
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
        self.buffer = [e for e in self.buffer
                       if (e["a"]["id"], e["b"]["id"]) not in kset]
        for k in kset:
            self.entries.pop(k, None)
        if self.pending is not None:
            self.pending -= kset

    def right_insert(self, f):
        self.rtm.append(f)
        self.epoch_log.append(("ins", f["id"]))
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
            self.epoch_log.append(("move", f["id"], self.cur_tagwrite))
            if self.cur_tagwrite:
                self.self_dirty.add(f["id"])

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
        node.epoch_start = [x["id"] for x in node.rtm]
        node.epoch_log = []
        node.self_dirty = set()
        for act in ep["actions"]:
            f = facts[act["target"]]
            old_tag = f["tag"]
            new_tag = act.get("tag", old_tag)
            if "ts" in act:
                f["ts"] = act["ts"]          # printed only; ts0 frozen
            f["tag"] = new_tag
            was_anchor = f["a_type"] and old_tag == "z"
            now_anchor = f["a_type"] and new_tag == "z"
            is_exit = was_anchor and not now_anchor
            node.act_no += 1
            node.cur_fact = f["id"]
            node.cur_tagwrite = "tag" in act
            node.cur_entry = not was_anchor and now_anchor
            # phase A: $a alpha transition (entry scan = scan_rtm view);
            # in-place anchor update refires its own children (A')
            if not was_anchor and now_anchor:
                node.left_insert(f)
            elif is_exit:
                node.left_delete(f)
            elif was_anchor and now_anchor and "tag" in act:
                node.left_refire(f)          # $a watches tag only
            # phase B: $b-side child_upd refires (always heard; movable
            # when this action writes tag — T6-1/2)
            if f["b_type"]:
                node.right_refire(f)
            # phase C: the $b tail re-add — immediate for every class
            # (T6-3; the self-slot rule alone hides tag-class moves from
            # the fact's own entry scan); a surviving ANCHOR moves to the
            # tail of the left memory on tag-touch
            if f["b_type"]:
                node.right_readd(f)
            if f["a_type"] and f["tag"] == "z" and "tag" in act:
                node.left_readd(f)           # gated by the $a watch mask
            node.cur_fact = None
            node.cur_tagwrite = False
            node.cur_entry = False
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
