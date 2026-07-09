#!/usr/bin/env python3
"""not×temporal firing-DEFERRAL reference model (D-129, cold-starting the not
slab). Single not off a positive anchor (`not_partner`), no @expires (isolates
the deferral arc from the inference arc). `simulate()` is the candidate spec;
the harness differs it vs the gate oracle over a shuffled population WITH random
clock advances.

--- the deferral semantic (D-129, oracle-measured) ---
A `not B(this OP[lo,hi] $a)` does NOT fire when the anchor A is merely
satisfied-so-far (that is Seine's bug); Drools DEFERS until the pseudo-clock
proves no blocker can still arrive. Per-anchor firing clock:
  * after[lo,hi]:  fire_time = A.ts + hi     (blocker window [A+lo,A+hi] is future)
  * before[lo,hi]: fire_time = A.ts - lo     (blocker window [A-hi,A-lo] is past)
  IMMEDIATE (fire at the initial fire, clock 0) iff fire_time < A.ts — i.e. the
  whole blocker window is strictly before A (before with lo>0). Otherwise DEFER:
  fire when clock >= fire_time (after: always; before with lo==0: at A.ts).
A blocker B with B.ts in the window suppresses the firing (no delete/expire in
this population, so a present B blocks permanently). Deferred firings that come
due in ONE advanceTime fire in REVERSE close-time order (descending fire_time —
the addInsert-PREPEND discipline again); across separate advances, in advance
order. Immediate firings fire at the initial fire.

Faithfulness bar: ZERO model-vs-oracle divergences on the shuffled population.
"""
import json
import os
import random
import re
import subprocess
import sys

REPO = "/home/bryan/rust-rules"


def etype(n):
    # LARGE explicit @expires: nothing expires within the test horizon, so
    # blockers block permanently — this ISOLATES the deferral arc. (Absent
    # @expires, Drools INFERS a blocker reach from the temporal constraint and
    # an advance expires the blocker — that is the coupled inference arc, D-130.)
    return {"name": n, "fields": [{"name": "ts", "type": "i64"}],
            "event": {"timestamp": "ts", "expires_ms": 1000000}}


def gen(rng, name):
    op = rng.choice(["after", "before"])
    lo = rng.choice([0, 0, 20, 60])
    hi = lo + rng.choice([20, 40, 80])
    drl = f"rule NT when $a : E0() not E1(this {op}[{lo}ms,{hi}ms] $a) then end\n"
    a_ts = rng.sample(range(0, 12), rng.choice([1, 2, 2, 3]))
    facts = [("E0", t) for t in a_ts]
    # blockers: some in-window, some out
    for _ in range(rng.randint(0, 3)):
        anchor = rng.choice(a_ts)
        if op == "after":
            b = anchor + rng.randint(0, hi + 20)        # [anchor, anchor+hi+20]
        else:
            b = anchor - rng.randint(0, hi + 20)
        facts.append(("E1", b))
    rng.shuffle(facts)
    # a chain of 0-2 clock advances (probes the deferral schedule)
    epochs = []
    for _ in range(rng.randint(0, 2)):
        epochs.append({"actions": [{"op": "advance", "ms": rng.choice([20, 50, 100, 200])}],
                       "facts": []})
    return {"name": name, "types": [etype("E0"), etype("E1")], "drl": drl,
            "facts": [{"type": t, "fields": {"ts": v}} for t, v in facts],
            "epochs": epochs}


def simulate(scn):
    m = re.search(r'not E1\(this (after|before)\[(\d+)ms,(\d+)ms\] \$a\)', scn["drl"])
    op, lo, hi = m.group(1), int(m.group(2)), int(m.group(3))
    anchors = [f["fields"]["ts"] for f in scn["facts"] if f["type"] == "E0"]
    blockers = [f["fields"]["ts"] for f in scn["facts"] if f["type"] == "E1"]

    def in_window(b, a):
        return (a + lo <= b <= a + hi) if op == "after" else (a - hi <= b <= a - lo)

    # Per unblocked anchor: IMMEDIATE iff the blocker window is strictly before
    # A (before with lo>0) — fires at the initial fire regardless of clock.
    # Otherwise DEFERRED with fire_time = A+hi (after) or A-lo=A (before lo=0);
    # fires at the first fire-point whose clock >= fire_time (so fire_time<=0
    # fires at the initial fire too).
    entries = []   # (a_ts, immediate, fire_time)
    for a in anchors:
        if any(in_window(b, a) for b in blockers):
            continue                                    # blocked -> never fires
        if op == "before" and lo > 0:
            entries.append((a, True, a - lo))
        else:
            entries.append((a, False, a + hi if op == "after" else a))

    # fire points: the initial fire (clock 0) then each epoch's cumulative clock
    clocks, c = [0], 0
    for ep in scn.get("epochs", []):
        for act in ep.get("actions", []):
            if act.get("op") == "advance":
                c += act["ms"]
        clocks.append(c)

    immediate_regime = op == "before" and lo > 0
    fired, firings = set(), []
    for ci, clock in enumerate(clocks):
        batch = [(a, ft) for (a, imm, ft) in entries
                 if a not in fired and ((imm and ci == 0) or (not imm and ft <= clock))]
        if not immediate_regime:
            # a deferred advanceTime batch fires REVERSE close-time (PREPEND);
            # the immediate regime keeps normal agenda (insertion) order
            batch.sort(key=lambda p: p[1], reverse=True)
        for a, _ in batch:
            fired.add(a)
            firings.append(a)
    return firings


def _oracle(paths):
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle"] + paths,
                       cwd=REPO, capture_output=True, text=True, env=env)
    out = {}
    for line in r.stdout.splitlines():
        try:
            o = json.loads(line)
        except Exception:
            continue
        res = o.get("result")
        out[o["scenario"]] = None if not res else [
            fr["matches"][0]["fields"]["ts"] for fr in res["firings"]]
    return out


def fuzz(n, seed):
    rng = random.Random(seed)
    OUT = "/tmp/model_not_defer"
    os.makedirs(OUT, exist_ok=True)
    done = ndiff = 0
    diffs = []
    while done < n:
        paths, scns = [], {}
        for i in range(done, min(done + 150, n)):
            scn = gen(rng, f"ndf{seed}x{i}")
            p = os.path.join(OUT, f"ndf{seed}x{i}.json")
            json.dump(scn, open(p, "w"))
            paths.append(p)
            scns[scn["name"]] = scn
        ora = _oracle(paths)
        for p in paths:
            nm = os.path.basename(p)[:-5]
            pred = simulate(scns[nm])
            if pred != ora.get(nm):
                ndiff += 1
                diffs.append(nm)
                if len(diffs) <= 12:
                    print(f"  DIV {nm}: model={pred} oracle={ora.get(nm)}")
                    print(f"      drl={scns[nm]['drl'].strip()}")
                    print(f"      facts={[(f['type'], f['fields']['ts']) for f in scns[nm]['facts']]}"
                          f" epochs={scns[nm]['epochs']}")
        done += len(paths)
    print(f"not-defer model-vs-oracle: {n} seed {seed}: {ndiff} divergences "
          f"({100*ndiff//max(1,n)}%)")


if __name__ == "__main__":
    fuzz(int(sys.argv[2]), int(sys.argv[3])) if sys.argv[1] == "fuzz" else None
