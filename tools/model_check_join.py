#!/usr/bin/env python3
"""model_check_join.py — D-082 ordering-model elimination for the join
multi-window / re-entry family.

Doctrine (memory, Phase Q1): validate a full ordering model in a
throwaway Python replica against ALL probe outputs BEFORE writing Rust
— wrong sub-models die in seconds and the survivor doubles as the spec.

Ground truth: oracle firing sequences for probes jw3/jw4/jw5 (the
certified multi-window join matrix, pr_hw_joinwin*) and jr1..jr10 (the
re-entry ladder, hw_jr*). Each scenario is a list of FIRES; each fire
is (events-before-the-join-evaluates, expected firing order). Events:
    ('L+', id)   left insert
    ('RF', id)   right enters via fresh INSERT (jw's produced rights)
    ('RU', id)   right enters via UPDATE alpha-transition (jr's re-adds)
    ('R-', id)   right leaves alpha
The fresh-vs-update split is forced by the data: jw3 and jr10 are
structurally identical event sequences with OPPOSITE oracle orders —
the only difference is how the right entered.
Producers (jw's R1) outrank the join rule, so their mid-fire inserts
are ordinary events in sequence. The join rule fires each (left,right)
pair once per creation; a FIRED child that is killed and re-created
counts as a REFIRE (fires again).

Model dimensions:
  window_mode   merged      one batch per fire (engine today)
                per_event   each WM action is its own full phase pass
  phase_order   which of Rdel/Rins/Lins processes first (dels lead —
                jr1's refire demands the old child dies before the
                re-add recreates it)
  rins_walk     Rins walks left MEMORY in arrival or reversed order
  lins_walk     Lins walks right MEMORY in arrival or reversed order
  emit          created children stage LIFO (per-entry prepend, head
                consume — the engine) or FIFO
  rdel_kill     Rdel kills all children of the fact, or only children
                born before the current batch (re-entry protection)
  refire_chan   a re-created FIRED child fires via the same ins queue,
                or via the UPD channel that the terminal processes
                before ins (dels -> upds -> ins)
"""

import itertools, sys

# ---------------------------------------------------------------- data
# (left ids are T0.f0 / jw uses T0-vs-T1 ids; right id constant 5 for jr)
SCENARIOS = [
    ("jw3", [
        ([("L+", 1), ("RF", 2)], [(1, 2)]),
        ([("L+", 3), ("RF", 4)], [(3, 4), (3, 2), (1, 4)]),
    ]),
    ("jw4", [
        ([("RF", 1), ("L+", 2)], [(2, 1)]),
        ([("RF", 3), ("L+", 4)], [(4, 3), (4, 1), (2, 3)]),
    ]),
    ("jw5", [
        ([("L+", 1), ("RF", 2)], [(1, 2)]),
        ([("L+", 3), ("RF", 4)], [(3, 4), (3, 2), (1, 4)]),
        ([("L+", 5), ("RF", 6)], [(5, 6), (5, 4), (5, 2), (3, 6), (1, 6)]),
    ]),
    ("jr1", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("R-", 5), ("RU", 5)], [(1, 5)]),
    ]),
    ("jr3", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("R-", 5), ("L+", 2), ("RU", 5)], [(1, 5), (2, 5)]),
    ]),
    ("jr5", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("L+", 2), ("R-", 5), ("RU", 5)], [(1, 5), (2, 5)]),
    ]),
    ("jr6", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("R-", 5), ("RU", 5), ("L+", 2)], [(1, 5), (2, 5)]),
    ]),
    ("jr7", [
        ([("L+", 1), ("L+", 2), ("RF", 5)], [(1, 5), (2, 5)]),
        ([("R-", 5), ("RU", 5)], [(1, 5), (2, 5)]),
    ]),
    ("jr8", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("R-", 5), ("L+", 2), ("L+", 3), ("RU", 5)], [(1, 5), (2, 5), (3, 5)]),
    ]),
    ("jr9", [
        ([("L+", 1), ("RF", 5)], [(1, 5)]),
        ([("R-", 5), ("L+", 2)], []),
    ]),
    ("jr10", [
        ([("L+", 1)], []),
        ([("L+", 2), ("RU", 5)], [(1, 5), (2, 5)]),
    ]),
]

import itertools as _it
PHASE_ORDERS = [("Rdel",) + p for p in _it.permutations(("RinsF", "RinsU", "Lins"))]
DIMS = dict(
    window_mode=["merged", "per_event"],
    phase_order=PHASE_ORDERS,
    rinsf_walk=["arrival", "reversed"],
    rinsu_walk=["arrival", "reversed"],
    lins_walk=["arrival", "reversed"],
    lstaged_walk=["arrival", "reversed"],
    emit=["lifo", "fifo"],
    rdel_kill=["all", "pre_batch"],
    refire_chan=["ins", "upd_first"],
)

ENGINE_MODEL = None  # engine reproduction dropped: the oracle model is the target


class State:
    def __init__(self):
        self.lefts = []            # arrival order
        self.rights = []
        self.children = {}         # (l, r) -> {"fired": bool, "born": batch_no}
        self.batch_no = 0


def run_fire(model, st, events, log):
    batches = [events] if model["window_mode"] == "merged" else [[e] for e in events]
    for batch in batches:
        st.batch_no += 1
        lins = [i for k, i in batch if k == "L+"]
        rinsf = [i for k, i in batch if k == "RF"]
        rinsu = [i for k, i in batch if k == "RU"]
        rdel = [i for k, i in batch if k == "R-"]
        # same-fact out-and-back inside one batch: both stay staged
        # (del-then-ins does not fold) — matches the engine's Staged.
        out_ins, out_upd = [], []

        def emit(child, refire):
            q = out_upd if (refire and model["refire_chan"] == "upd_first") else out_ins
            if model["emit"] == "lifo":
                q.insert(0, child)
            else:
                q.append(child)

        def do_rdel():
            for f in rdel:
                if f in st.rights:
                    st.rights.remove(f)
                for (l, r), c in list(st.children.items()):
                    if r == f and (model["rdel_kill"] == "all" or c["born"] < st.batch_no):
                        del st.children[(l, r)]


        def do_lins():
            # memory fills in ARRIVAL order regardless of the staged
            # processing walk — coupling them corrupts every later
            # fire's memory walks (jr7's second fire).
            st.lefts.extend(lins)
            lorder = list(lins)
            if model["lstaged_walk"] == "reversed":
                lorder.reverse()
            for l in lorder:
                rights = list(st.rights)
                if model["lins_walk"] == "reversed":
                    rights.reverse()
                for r in rights:
                    if (l, r) in st.children:
                        continue
                    st.children[(l, r)] = {"fired": False, "born": st.batch_no}
                    emit((l, r), False)

        # refire detection needs the fired-flag of the OLD child at kill
        # time: capture before Rdel destroys it.
        fired_before = {k: c["fired"] for k, c in st.children.items()}

        def do_rins(rins, walk):
            for f in rins:
                lefts = list(st.lefts)
                if walk == "reversed":
                    lefts.reverse()
                st.rights.append(f)
                for l in lefts:
                    if (l, f) in st.children:
                        continue
                    refire = fired_before.get((l, f), False)
                    st.children[(l, f)] = {"fired": False, "born": st.batch_no}
                    emit((l, f), refire)

        for ph in model["phase_order"]:
            if ph == "Rdel":
                do_rdel()
            elif ph == "RinsF":
                do_rins(rinsf, model["rinsf_walk"])
            elif ph == "RinsU":
                do_rins(rinsu, model["rinsu_walk"])
            elif ph == "Lins":
                do_lins()

        # terminal: dels (cancellations, no firing), then upds, then ins
        for child in out_upd + out_ins:
            if child in st.children and not st.children[child]["fired"]:
                st.children[child]["fired"] = True
                log.append(child)


def simulate(model, fires):
    st, out = State(), []
    for events, _ in fires:
        log = []
        run_fire(model, st, events, log)
        out.append(log)
    return out


def check(model):
    fails = []
    for name, fires in SCENARIOS:
        got = simulate(model, fires)
        exp = [e for _, e in fires]
        if got != exp:
            fails.append((name, exp, got))
    return fails


def main():
    keys = list(DIMS)
    survivors = []
    total = 0
    for combo in itertools.product(*(DIMS[k] for k in keys)):
        model = dict(zip(keys, combo))
        total += 1
        fails = check(model)
        if not fails:
            survivors.append(model)
    print(f"{total} models, {len(survivors)} survivors")
    for m in survivors:
        print("  SURVIVOR:", {k: (v if not isinstance(v, tuple) else "/".join(v)) for k, v in m.items()})

    if not survivors:
        # nearest misses: fewest failing scenarios
        best = []
        for combo in itertools.product(*(DIMS[k] for k in keys)):
            model = dict(zip(keys, combo))
            fails = check(model)
            best.append((len(fails), [f[0] for f in fails], model))
        best.sort(key=lambda x: x[0])
        for n, names, m in best[:5]:
            print(f"  near-miss ({n} fail: {names}):",
                  {k: (v if not isinstance(v, tuple) else "/".join(v)) for k, v in m.items()})
    return 0 if survivors else 1


if __name__ == "__main__":
    sys.exit(main())
