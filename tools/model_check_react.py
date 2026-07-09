#!/usr/bin/env python3
"""CEP E2 item C §1a checker (D-139): the ACCUMULATE-SOURCE external-update
property-reactivity rule, validated engine == oracle == PREDICATE over a
systematic cell matrix.

Unlike the model_check_stream family (which enumerates a flip-flopping staging
ORDER for a unique survivor), this gap is a clean boolean PREDICATE — no
simulation needed — so the "model" is the predicate itself and the checker
asserts BOTH engines reproduce it.

THE RULE (probed, 2026-07-09; overturns the D-137 "plain re-folds on ANY
modify" finding):
  An external in-place UPDATE of an accumulate SOURCE event that keeps it
  matching (alpha still passes, still in window) RE-FIRES the rule iff the
  updated field set intersects the node's WATCH MASK:
    watch(plain accumulate)    = source CONSTRAINT fields ∪ source BINDING fields
    watch(windowed accumulate) = source BINDING fields ONLY   (constraints dropped)
  count() (no binding) windowed ⇒ empty mask ⇒ never re-fires on any field.
  The timestamp field follows the same rule (watched iff BOUND), independent
  of the fact that it drives window membership.

Engine device: on_update (true,true) gates the source re-fold on `listen_mask`
(= constraints∪bindings) for plain and `bind_fields` (= bindings) for windowed.
"""
import json, os, subprocess, sys

TMP = os.environ.get("REACT_TMP", "/home/bryan/.claude/jobs/577ad61a/tmp/mc_react")
os.makedirs(TMP, exist_ok=True)
ROOT = "/home/bryan/rust-rules"

# 4 fields so constraint / binding / unread / timestamp are all distinguishable.
TYPE = {"name": "E0", "fields": [
    {"name": "ts", "type": "i64"}, {"name": "tag", "type": "String"},
    {"name": "val", "type": "i64"}, {"name": "oth", "type": "i64"}],
    "event": {"timestamp": "ts", "expires_ms": 100000}}

def fact(ts=38): return {"type": "E0", "fields": {"ts": ts, "tag": "y", "val": 10, "oth": 0}}

# A cell = (windowed, constraint_fields, binding_specs, update_field)
#   binding_specs: list of (var, field) bound in the source; the function reads
#   the FIRST binding (or count() if none).
# Predicted re-fire iff update_field ∈ watch mask.
FIELDS = {"tag", "val", "oth", "ts"}

def build_drl(windowed, cons, binds):
    parts = []
    for f in sorted(cons):
        parts.append(f'tag == "y"' if f == "tag" else f'{f} > -999')
    for var, fld in binds:
        parts.append(f"${var} : {fld}")
    src = "E0(" + ", ".join(parts) + ")"
    win = " over window:time(200ms)" if windowed else ""
    fn = f"$s : sum(${binds[0][0]})" if binds else "$c : count()"
    return f"rule W2 when accumulate( {src}{win}; {fn} ) then end\n"

def predict(windowed, cons, binds, upd):
    bindf = {fld for _, fld in binds}
    watch = bindf if windowed else (set(cons) | bindf)
    return 2 if upd in watch else 1

# systematic matrix
CELLS = []
def cell(name, windowed, cons, binds, upd):
    CELLS.append((name, windowed, frozenset(cons), tuple(binds), upd))

for w in (False, True):
    p = "w" if w else "p"
    # count(), constraint on tag: update each field
    for u in ("tag", "oth", "val", "ts"):
        cell(f"{p}_cnt_tagc_{u}", w, {"tag"}, [], u)
    # count(), no constraint
    cell(f"{p}_cnt_bare_tag", w, set(), [], "tag")
    # count(), constraint on val (inequality)
    cell(f"{p}_cnt_valc_val", w, {"val"}, [], "val")
    # sum($v:val), no constraint: update each
    for u in ("val", "oth", "tag", "ts"):
        cell(f"{p}_sum_v_{u}", w, set(), [("v", "val")], u)
    # sum($v:val), constraint on tag: update tag(cons) / val(bind) / oth(unread)
    for u in ("tag", "val", "oth"):
        cell(f"{p}_sum_v_tagc_{u}", w, {"tag"}, [("v", "val")], u)
    # sum($v:val) with a 2nd binding $w:oth NOT used by the fn: update oth
    cell(f"{p}_sum_v_bindoth", w, set(), [("v", "val"), ("w", "oth")], "oth")
    # sum($t:ts) — the timestamp AS the bound fn arg (the fuzz shape): update ts / tag
    cell(f"{p}_sum_ts_ts", w, {"tag"}, [("t", "ts")], "ts")
    cell(f"{p}_sum_ts_tag", w, {"tag"}, [("t", "ts")], "tag")

def emit():
    files = []
    for name, w, cons, binds, upd in CELLS:
        drl = build_drl(w, cons, list(binds))
        uval = {"tag": "y", "val": 10, "oth": 0, "ts": 40}[upd]  # membership-preserving
        scn = {"name": name, "types": [TYPE], "drl": drl, "facts": [fact()],
               "epochs": [{"actions": [{"op": "update", "target": 0, "fields": {upd: uval}}], "facts": []}]}
        p = os.path.join(TMP, name + ".json")
        json.dump(scn, open(p, "w"), indent=1)
        files.append(p)
    return files

def run(cmd, files):
    out = subprocess.run(["cargo", "run", "-q", "-p", "seine-harness", "--", cmd] + files,
                         capture_output=True, text=True, cwd=ROOT)
    res = {}
    for ln in out.stdout.splitlines():
        ln = ln.strip()
        if ln.startswith("{"):
            j = json.loads(ln); r = j.get("result")
            res[j["scenario"]] = len(r["firings"]) if r else None
    return res

def main():
    files = emit()
    eng = run("run", files)
    ora = run("oracle", files)
    bad = 0
    for name, w, cons, binds, upd in CELLS:
        exp = predict(w, cons, list(binds), upd)
        o, e = ora.get(name), eng.get(name)
        ok = (o == exp and e == exp)
        if not ok:
            bad += 1
            print(f"  MISMATCH {name:22} predict={exp} oracle={o} engine={e}")
    print(f"{len(CELLS)} cells: {'ALL MATCH predicate ✓ (engine==oracle==rule)' if bad==0 else f'{bad} MISMATCH'}")
    return 1 if bad else 0

if __name__ == "__main__":
    sys.exit(main())
