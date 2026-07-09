#!/usr/bin/env python3
"""exists×temporal admission-ORDER reference model + population validator
(D-126 → the unwalling slab; sibling of model_join_flush.py). `simulate()` is
the candidate spec for the `do_existential_node` port; the harness differs it
against the gate oracle over the fuzz_exists_temporal shuffled population.

  model_exists_flush.py fuzz <n> <seed>     # shuffled exists×temporal population
  model_exists_flush.py cases               # the two D-126 golden witnesses

--- the model (per-arrival flush; extends model_join_flush v2 to exists) ---
The network is a linear chain of nodes off the E0 root; each pattern after E0
is a node whose right-input is that event type. A node is a JOIN (positive
pattern, extends the tuple) or an EXISTS (existential pattern, propagates the
left UNCHANGED while it has a blocker). Same phreak disciplines as the
validated join model (drools-core 9.44; ARBITER = oracle):

* node memory (TupleList.add) APPENDS; scan the OPPOSITE memory FORWARD.
* emissions -> child's staged-left set via addInsert = PREPEND; the child's
  doLeftInserts reads it getInsertFirst (= prepend order) and APPENDS.
* NET: one emit is identity; a BATCH of N emits is reversed EXACTLY ONCE.

Exists specifics (PhreakExistsNode):
* a left is BLOCKED by the first matching right in memory order; a blocked
  left LEAVES the left memory and lives on the blocker's blocked list
  (RightTuple.addBlocked PREPENDS). Exists propagates a child while BLOCKED.
* RIGHT-insert: scan the (unblocked) left memory FORWARD, block each match,
  emit it (PREPEND into child-staged) -> a right blocking N lefts reverses
  them once. This is the D-126 multi-anchor family the join port left out.
* LEFT-insert: find the FIRST blocker (memory order); one emit = identity.
  An unblocked left waits in memory; a later right admits it.

Faithfulness bar (this file): ZERO model-vs-oracle divergences on the
shuffled population. Curated cases alone are disqualifying evidence (v1
lesson). Only after 0-div does the port into do_existential_node begin.
"""
import json
import os
import re
import sys


# --- DRL -> ordered pattern list -------------------------------------------
# Each pattern: (kind, eidx, anchor_pos, op, lo, hi). E0 is the positive root
# (no constraint). Positive patterns bind a tuple position (in order); exists
# patterns bind none. `anchor_pos` is the tuple position of the referenced
# variable, resolved against the positions bound by EARLIER patterns.
PAT = re.compile(r'(exists\s+)?(?:\$(\w+)\s*:\s*)?E(\d+)\s*\(([^)]*)\)')
CONSTR = re.compile(r'this\s+(after|before)\[(\d+)ms,\s*(\d+)ms\]\s*\$(\w+)')


def parse(drl):
    body = drl.split("when", 1)[1].split("then", 1)[0]
    var_pos, next_pos, pats = {}, 0, []
    for m in PAT.finditer(body):
        is_ex = m.group(1) is not None
        var, eidx, inner = m.group(2), int(m.group(3)), m.group(4)
        c = CONSTR.search(inner)
        if c is None:
            # E0 root: positive, no constraint
            pats.append(("join", eidx, None, None, None, None))
        else:
            op, lo, hi, avar = c.group(1), int(c.group(2)), int(c.group(3)), c.group(4)
            anchor = var_pos[avar]
            pats.append(("exists" if is_ex else "join", eidx, anchor, op, lo, hi))
        if not is_ex:
            var_pos[var] = next_pos
            next_pos += 1
    return pats


class Node:
    def __init__(self, kind, anchor, op, lo, hi):
        self.kind, self.anchor, self.op, self.lo, self.hi = kind, anchor, op, lo, hi
        self.ltm, self.rtm, self.child, self.firings = [], [], None, []

    def _win(self, anchor_ts, this_ts):
        d = this_ts - anchor_ts if self.op == "after" else anchor_ts - this_ts
        return self.lo <= d <= self.hi

    def _propagate(self, trg):
        if self.child is None:
            self.firings.extend(trg)
        else:
            self.child.emit_set(trg)

    def emit_set(self, staged_left):
        """doLeftInserts over `staged_left` (getInsertFirst order): append to
        this node's ltm, scan the opposite memory, PREPEND matches into trg."""
        trg = []
        for lt in staged_left:
            if self.kind == "join":
                self.ltm.append(lt)
                for rt in self.rtm:                          # forward scan
                    if self._win(lt[self.anchor][1], rt[1]):
                        trg.insert(0, lt + [rt])             # addInsert PREPEND
            else:                                            # exists left-insert
                blk = next((rt for rt in self.rtm
                            if self._win(lt[self.anchor][1], rt[1])), None)
                if blk is not None:
                    trg.insert(0, lt)                        # single emit (identity)
                else:
                    self.ltm.append(lt)                      # wait, unblocked
        self._propagate(trg)

    def right_insert(self, rt):
        """doRightInserts: append rt to rtm; join emits every left match,
        exists blocks every unblocked left match (PREPEND -> reversed batch)."""
        self.rtm.append(rt)
        trg = []
        if self.kind == "join":
            for lt in self.ltm:                              # forward scan
                if self._win(lt[self.anchor][1], rt[1]):
                    trg.insert(0, lt + [rt])                 # addInsert PREPEND
        else:                                                # exists right-insert
            newly = []
            for lt in self.ltm:                              # forward scan
                if self._win(lt[self.anchor][1], rt[1]):
                    trg.insert(0, lt)                        # addBlocked/addInsert PREPEND
                    newly.append(lt)
            for lt in newly:
                self.ltm.remove(lt)                          # blocked lefts leave memory
        self._propagate(trg)


def simulate(scenario):
    pats = parse(scenario["drl"])
    nodes = [Node(k, a, o, lo, hi) for (k, _e, a, o, lo, hi) in pats[1:]]
    for i in range(len(nodes) - 1):
        nodes[i].child = nodes[i + 1]
    eidx_node = {pats[i + 1][1]: nodes[i] for i in range(len(nodes))}  # Ei -> its node
    for f in scenario["facts"]:
        eidx, ts = int(f["type"][1:]), f["fields"]["ts"]
        if eidx == 0:
            nodes[0].emit_set([[(0, ts)]])                   # E0 root left-insert
        else:
            eidx_node[eidx].right_insert((eidx, ts))
    out = []
    for t in nodes[-1].firings:
        out.append("-".join(str(ts) for _e, ts in sorted(t)))
    return out


# --- oracle bridge (identical rendering to model_join_flush._gate_oracle) ---
def _gate_oracle(paths):
    import subprocess
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(
        ["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
        cwd="/home/bryan/rust-rules", capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try:
            o = json.loads(line)
        except Exception:
            continue
        res = o.get("result")
        if not res:
            out[o["scenario"]] = None
            continue
        seq = []
        for fr in res["firings"]:
            d = {m["type"]: m["fields"]["ts"] for m in fr["matches"]}
            seq.append("-".join(str(d[f"E{i}"])
                                for i in sorted(int(k[1:]) for k in d)))
        out[o["scenario"]] = seq
    return out


def fuzz(n, seed):
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import fuzz_exists_temporal as fx
    import random
    rng = random.Random(seed)
    OUT = "/tmp/model_exists_fuzz"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 150, n)):
            scn = fx.gen(rng, f"mxf{seed}x{i}")
            p = os.path.join(OUT, f"mxf{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = _gate_oracle(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            pred = simulate(scns[nm])
            if pred != ora.get(nm):
                ndiff += 1
                diffs.append(nm)
                if len(diffs) <= 12:
                    print(f"  DIV {nm}\n    model : {pred}\n    oracle: {ora.get(nm)}\n"
                          f"    drl   : {scns[nm]['drl'].strip()}\n"
                          f"    facts : {[(f['type'], f['fields']['ts']) for f in scns[nm]['facts']]}")
        done += len(paths)
    print(f"exists model-vs-oracle: {n} cases seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")


def cases():
    base = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        "..", "probes_pending", "cep", "e_recon")
    paths = [os.path.join(base, f"cp_ex_multi_anchor_{s}.json") for s in ("before", "after")]
    ora = _gate_oracle(paths)
    for p in paths:
        nm = os.path.basename(p)[:-5]
        pred = simulate(json.load(open(p)))
        tag = "ok " if pred == ora.get(nm) else "MISMATCH"
        print(f"  {tag} {nm}\n    model : {pred}\n    oracle: {ora.get(nm)}")


if __name__ == "__main__":
    if sys.argv[1] == "fuzz":
        fuzz(int(sys.argv[2]), int(sys.argv[3]))
    elif sys.argv[1] == "cases":
        cases()
