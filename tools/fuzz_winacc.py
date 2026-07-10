#!/usr/bin/env python3
"""A2 windowed-accumulate SOUP generator (D-154 arc) — oracle population bank.

Pure-accumulate scenarios (1-3 sum/count rules over one event type; windowed
and plain siblings, optional tag constraint / entry-point routing / a second
window size) under a free op soup: inserts with ts around every boundary
(stale-on-arrival, due-on-arrival expiry, deep-negative leak ts), external
UPDATES over every field subset (boundary-crossing ts both directions,
same-value no-op writes, mask hit/miss), explicit deletes of in-window /
evicted / revived members, advances landing exactly ON eviction (ts0+N) and
expiry (ts0+ex, ts0+ex+1) deadlines. Never updates/deletes a dead handle
(deadline-gated liveness, D-151 note: the oracle NPEs).

Banks scenario files + oracle firings for the spec gate:
  fuzz_winacc.py <n> <seed>
    -> $WINACC_TMP/winaccpop_<seed>/{wf<seed>x<i>.json, oracle.jsonl}
  then: python3 tools/model_check_winacc.py $WINACC_TMP/winaccpop_<seed>

The banked populations double as the engine's post-port gate
(`seine-harness -- diff` over the same files).
"""
import json, os, random, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TMP = os.environ.get("WINACC_TMP", "/tmp/seine_winacc")

WINDOWS = [50, 100, 100, 150, 200]
EXPIRES = [40, 80, 150, 400, 100000]


def gen(r, name):
    ex = r.choice(EXPIRES)
    use_ep = r.random() < 0.15
    types = [{"name": "E", "fields": [
        {"name": "ts", "type": "i64"}, {"name": "val", "type": "i64"},
        {"name": "tag", "type": "String"}],
        "event": {"timestamp": "ts", "expires_ms": ex}}]

    rules = []
    specs = []
    for i in range(r.randint(1, 3)):
        win = r.choice(WINDOWS) if r.random() < 0.75 else None
        fn = r.choice(["sumts", "sumval", "count"])
        cons = f'tag == "{r.choice("xyz")}"' if r.random() < 0.4 else ""
        ep = ' from entry-point "S1"' if use_ep else ""
        if fn == "count":
            src = f"E({cons})"
            agg = "$c : count()"
        else:
            fld = "ts" if fn == "sumts" else "val"
            inner = (cons + ", " if cons else "") + f"$t : {fld}"
            src = f"E({inner})"
            agg = "$c : sum($t)"
        overw = f" over window:time({win}ms)" if win else ""
        rules.append(f"rule W{i} when accumulate( {src}{overw}{ep}; {agg} ) then end")
        specs.append(win)

    clock = 0
    gidx = 0
    live = []          # {idx, deadline}  (deadline None = leak/immortal)
    all_deadlines = set()

    def mk_event():
        nonlocal gidx
        if r.random() < 0.04:
            ts = r.randint(-500, -450)      # deep-negative: expiry leak
        else:
            ts = clock + r.choice([-260, -160, -60, -1, 0, 1, 5, 15, 30])
        f = {"type": "E",
             "fields": {"ts": ts, "val": r.randint(1, 9), "tag": r.choice("xyz")}}
        if use_ep:
            f["entry_point"] = "S1"
        # expiry deadline D = ts+ex+1 (D-150 boundary): D < 0 leaks
        # (immortal), else the handle dies at clock >= D — the liveness
        # gate is clock < D (an update AT clock == ts+ex is still safe)
        d = ts + ex + 1
        # track a COPY — the update op mutates the tracker's view; sharing
        # the serialized fact's dict would rewrite the insert retroactively
        live.append({"idx": gidx, "deadline": None if d < 0 else d,
                     "fields": dict(f["fields"])})
        for w in specs:
            if w is not None and ts + w > clock:
                all_deadlines.add(ts + w)
        if d >= 0:
            all_deadlines.update((d - 1, d))
        gidx += 1
        return f

    facts = [mk_event() for _ in range(r.randint(0, 3))]

    epochs = []
    for _ in range(r.randint(2, 5)):
        actions = []
        if r.random() < 0.8:
            future = sorted(dl for dl in all_deadlines if dl > clock)
            if future and r.random() < 0.5:
                tgt = r.choice(future[:4])
                ms = tgt - clock + r.choice([-1, 0, 0, 1])
                ms = max(1, ms)
            else:
                ms = r.randint(10, 180)
            actions.append({"op": "advance", "ms": ms})
            clock += ms
        alive = [t for t in live
                 if t["deadline"] is None or clock < t["deadline"]]
        for _ in range(r.randint(0, 3)):
            alive = [t for t in alive
                     if t["deadline"] is None or clock < t["deadline"]]
            if not alive or r.random() < 0.3:
                break
            tg = r.choice(alive)
            if r.random() < 0.25:
                actions.append({"op": "delete", "target": tg["idx"]})
                live.remove(tg)
                alive.remove(tg)
            else:
                fields = {}
                if r.random() < 0.55:
                    fields["tag"] = (tg["fields"]["tag"] if r.random() < 0.25
                                     else r.choice("xyz"))
                if r.random() < 0.45:
                    n = r.choice([w for w in specs if w] or [100])
                    fields["ts"] = clock + r.choice(
                        [-n - 20, -n, -n + 1, -5, 0, 10, 25])
                if r.random() < 0.4:
                    fields["val"] = (tg["fields"]["val"] if r.random() < 0.25
                                     else r.randint(1, 9))
                if not fields:
                    fields["val"] = tg["fields"]["val"]     # no-op write
                # ts updates do NOT move the expiry/eviction deadlines
                # (insert-fixed, D-141) — the liveness gate keeps using the
                # insert-time deadline.
                tg["fields"].update(fields)
                actions.append({"op": "update", "target": tg["idx"],
                                "fields": dict(fields)})
        efacts = [mk_event() for _ in range(r.randint(0, 2))]
        epochs.append({"actions": actions, "facts": efacts})

    return {"name": name, "types": types, "drl": "\n".join(rules) + "\n",
            "facts": facts, "epochs": epochs}


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    outdir = os.path.join(TMP, f"winaccpop_{seed}")
    os.makedirs(outdir, exist_ok=True)
    r = random.Random(seed)
    paths = []
    for i in range(n):
        sc = gen(r, f"wf{seed}x{i}")
        p = os.path.join(outdir, sc["name"] + ".json")
        json.dump(sc, open(p, "w"))
        paths.append(p)
    bank = os.path.join(outdir, "oracle.jsonl")
    with open(bank, "w") as out:
        for i in range(0, len(paths), 100):
            batch = paths[i:i + 100]
            res = subprocess.run(
                ["cargo", "run", "-q", "-p", "seine-harness", "--", "oracle", *batch],
                cwd=ROOT, capture_output=True, text=True, timeout=1200)
            out.write(res.stdout)
            print(f"  oracle batch@{i}: {len(batch)} done", flush=True)
    print(f"banked {len(paths)} -> {outdir}")


if __name__ == "__main__":
    main()
