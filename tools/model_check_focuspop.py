#!/usr/bin/env python3
"""D-346: the focus-pop halt-law model (the D-333 model_check pattern).

Adjudicates the executor halt/continue law around setFocus pushes —
the D-345 open question (fz_342002_1206 / mz3 vs the D-106 88-witness
keep-control calibration) — over a fuzz population of ALPHA-ONLY
agenda scenarios (single-pattern rules on one fact type, so the match
sets are computable exactly here; no beta, no acc, no TMS, no
no-loop, no dynamic salience).

Source facts (drools-core/kiesession 9.44.0.Final, read verbatim):
- KnowledgeHelper.setFocus -> AgendaGroupQueueImpl.setFocus ->
  addPropagation(SetFocusAction): DEFERRED to the post-firing flush;
  internalExecute: if agenda.setFocus(name) actually pushed (the
  focusStack.getLast() != group guard -> already-top setFocus is a
  NO-OP) -> haltGroupEvaluation().
- MzProbe (probes_pending/focuspop): after FIRE B(zz): PUSH g,
  MATCH+ A, POP g, FIRE A — a REAL push yields the executor even
  when the pushed group is EMPTY; the loop pops empty tops and
  re-selects fresh by (salience, decl).
- RuleExecutor.haltRuleFiring between same-rule firings: peek the
  FOCUS-TOP group only (empty top peeks null -> continue); halt on
  any foreign-group item at top, or an own-group item strictly
  preceding l per RuleAgendaConflictResolver (salience desc, decl
  asc).
- fireLoop/getNextFocus: pops empty auto-deactivate tops (plain
  agenda-groups pop when empty — the probe's POP g); MAIN never pops.

CANDIDATE LAWS (the machine axis):
  keep   — the engine today: a real push does NOT by itself yield;
           the halt-check peeks the top (empty -> keep control).
  yield  — a REAL push yields after the current firing (no-op
           setFocus never does); everything else = the peek law.
  yieldall — ANY setFocus yields (tests the no-op guard).
  naive  — no yield; but an equal-salience decl-preceding QUEUED
           own-group member blocks the empty-top continue (the
           reverted D-345 gate).
"""
import json
import os
import random
import subprocess
import sys

REPO = "/home/bryan/rust-rules"

# ── scenario generation ────────────────────────────────────────────

VALS = ["zz", "azz", "x", "y", ""]

CONSTRAINTS = [
    ("bare", lambda f: True, {"f0"}),            # T1($x : f0) binds f0
    ("czz", lambda f: "zz" in f["f0"], {"f0"}),
    ("cx", lambda f: f["f0"] == "x", {"f0"}),
    ("f2f", lambda f: f["f2"] is False, {"f2"}),
    ("f2t", lambda f: f["f2"] is True, {"f2"}),
    ("czz_bf2", lambda f: "zz" in f["f0"], {"f0", "f2"}),  # + $b : f2
]

RHS = ["none", "focus", "modify", "focus_modify", "insert"]


def gen(rng, name):
    nrules = rng.choice([2, 3, 3, 4])
    grp_rule = rng.randrange(nrules) if rng.random() < 0.8 else None
    rules = []
    for i in range(nrules):
        ckey, _, _ = CONSTRAINTS[rng.randrange(len(CONSTRAINTS))]
        sal = rng.choice([0, 0, 0, 2, -2])
        rhs = RHS[rng.randrange(len(RHS))] if grp_rule is not None else \
            rng.choice(["none", "modify", "insert"])
        if i == grp_rule:
            # a self-setFocus from the focused group's own rule is the
            # ALREADY-TOP no-op case (the getLast() != group guard) —
            # the yield-vs-yieldall discriminator
            rhs = "focus" if rng.random() < 0.4 else "none"
        rules.append({"ckey": ckey, "sal": sal, "rhs": rhs,
                      "grp": "g" if i == grp_rule else "MAIN"})
    if grp_rule is not None and not any(
            r["rhs"] in ("focus", "focus_modify") for r in rules):
        k = rng.choice([i for i in range(nrules) if i != grp_rule])
        rules[k]["rhs"] = rng.choice(["focus", "focus_modify"])
    nfacts = rng.choice([2, 3, 3, 4])
    facts = [{"f0": rng.choice(VALS), "f2": rng.random() < 0.4}
             for _ in range(nfacts)]
    return {"name": name, "rules": rules, "facts": facts}


def to_drl(spec):
    lines = []
    for i, r in enumerate(spec["rules"]):
        cons = {
            "bare": "$x : f0", "czz": 'f0 contains "zz"', "cx": 'f0 == "x"',
            "f2f": "f2 == false", "f2t": "f2 == true",
            "czz_bf2": 'f0 contains "zz", $b : f2',
        }[r["ckey"]]
        acts = {
            "none": "", "focus": 'drools.setFocus("g"); ',
            "modify": "modify($p) { setF2(true) } ",
            "focus_modify": 'drools.setFocus("g"); modify($p) { setF2(true) } ',
            "insert": 'insert(new T1("nv", false)); ',
        }[r["rhs"]]
        attrs = ""
        if r["sal"]:
            attrs += f"salience {r['sal']} "
        if r["grp"] != "MAIN":
            attrs += f'agenda-group "{r["grp"]}" '
        lines.append(
            f'rule R{i} {attrs}when $p : T1({cons}) then {acts}end')
    return "\n".join(lines) + "\n"


def to_scenario(spec):
    return {
        "name": spec["name"],
        "types": [{"name": "T1", "fields": [
            {"name": "f0", "type": "String"},
            {"name": "f2", "type": "bool"}]}],
        "drl": to_drl(spec),
        "facts": [{"type": "T1", "fields": dict(f)} for f in spec["facts"]],
    }


# ── the agenda model ───────────────────────────────────────────────

def cinfo(ckey):
    for k, fn, listen in CONSTRAINTS:
        if k == ckey:
            return fn, listen
    raise KeyError(ckey)


def simulate(spec, law):
    rules = spec["rules"]
    facts = [dict(f) for f in spec["facts"]]      # fact id = index
    # per-rule FIFO activation queues of fact ids
    queues = [[] for _ in rules]
    fstack = ["MAIN"]
    firings = []

    def matches(ri, fi):
        fn, _ = cinfo(rules[ri]["ckey"])
        return fn(facts[fi])

    for fi in range(len(facts)):
        for ri in range(len(rules)):
            if matches(ri, fi):
                queues[ri].append(fi)

    def grp_items(g):
        """queued rule items of group g in (salience desc, decl asc)."""
        idx = [ri for ri in range(len(rules))
               if rules[ri]["grp"] == g and queues[ri]]
        return sorted(idx, key=lambda ri: (-rules[ri]["sal"], ri))

    def apply_rhs(ri, fi):
        """returns pushed: whether a REAL focus push happened."""
        rhs = rules[ri]["rhs"]
        pushed = False
        if rhs in ("focus", "focus_modify"):
            if fstack[-1] != "g":
                if "g" in fstack:
                    fstack.remove("g")
                fstack.append("g")
                pushed = True
            elif law == "yieldall":
                pushed = True
        if rhs in ("modify", "focus_modify"):
            facts[fi]["f2"] = True
            # Drools modify propagates on the SETTER MASK — no value
            # diff: setF2(true) on an already-true fact still updates
            # (x49/x162: the oracle refires listeners each time).
            changed = {"f2"}
            for rj in range(len(rules)):
                _, listen = cinfo(rules[rj]["ckey"])
                if not (listen & changed):
                    continue
                ok = matches(rj, fi)
                if ok and fi not in queues[rj]:
                    queues[rj].append(fi)
                elif not ok and fi in queues[rj]:
                    queues[rj].remove(fi)
        if rhs == "insert":
            facts.append({"f0": "nv", "f2": False})
            nfi = len(facts) - 1
            for rj in range(len(rules)):
                if matches(rj, nfi):
                    queues[rj].append(nfi)
        return pushed

    def pop_empty_tops():
        while len(fstack) > 1 and not grp_items(fstack[-1]):
            fstack.pop()

    guard = 0
    while True:
        guard += 1
        if guard > 500:
            return ["NONTERM"]
        pop_empty_tops()
        items = grp_items(fstack[-1])
        if not items:
            if len(fstack) > 1:
                continue
            break
        l = items[0]
        # the executor: fire l's activations FIFO with the halt law
        while queues[l]:
            fi = queues[l].pop(0)
            firings.append((f"R{l}", facts[fi]["f0"]))
            pushed = apply_rhs(l, fi)
            if pushed and law in ("yield", "yieldall"):
                break  # haltGroupEvaluation: yield to re-selection
            # between-firings halt check (haltRuleFiring):
            top = fstack[-1]
            titems = grp_items(top)
            nxt = next((r for r in titems if r != l), None)
            if law == "naive" and nxt is not None \
                    and rules[nxt]["grp"] == rules[l]["grp"] \
                    and rules[nxt]["sal"] == rules[l]["sal"] and nxt < l:
                break
            if nxt is None:
                continue  # empty/self top peeks null -> keep control
            if rules[nxt]["grp"] != rules[l]["grp"]:
                break  # foreign focused item halts
            if (-rules[nxt]["sal"], nxt) < (-rules[l]["sal"], l):
                break  # strictly-preceding own-group item halts
        if not queues[l]:
            pass  # removeRuleAgendaItemWhenEmpty
    return firings


# ── oracle ─────────────────────────────────────────────────────────

def oracle(paths):
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    r = subprocess.run(
        ["./target/release/seine-harness", "oracle"] + paths,
        cwd=REPO, capture_output=True, text=True, env=env)
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
            m = fr["matches"][0]
            seq.append((fr["rule"], m["fields"].get("f0")))
        out[o["scenario"]] = seq
    return out


LAWS = ["keep", "yield", "yieldall", "naive"]


def main(n, seed):
    rng = random.Random(seed)
    outdir = os.environ.get("FOCUSPOP_OUT", "/tmp/focuspop_pop")
    os.makedirs(outdir, exist_ok=True)
    score = {law: 0 for law in LAWS}
    diverg = {law: [] for law in LAWS}
    done = 0
    while done < n:
        batch = []
        for i in range(done, min(done + 100, n)):
            spec = gen(rng, f"fp{seed}x{i}")
            p = os.path.join(outdir, f"fp{seed}x{i}.json")
            json.dump(to_scenario(spec), open(p, "w"))
            batch.append((spec, p))
        ora = oracle([p for _, p in batch])
        for spec, p in batch:
            og = ora.get(spec["name"])
            if og is None:
                continue
            for law in LAWS:
                pred = simulate(spec, law)
                if pred != og:
                    score[law] += 1
                    if len(diverg[law]) < 6:
                        diverg[law].append(spec["name"])
        done += len(batch)
    print(f"focuspop model-vs-oracle: {n} cases seed {seed}")
    for law in LAWS:
        print(f"  {law:9s} divergences: {score[law]:4d}   e.g. {diverg[law][:4]}")


if __name__ == "__main__":
    main(int(sys.argv[1]), int(sys.argv[2]))
