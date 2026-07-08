#!/usr/bin/env python3
"""Accumulate-deferral checker (D-112): enumerate the CLOCK-DRIVEN-REMOVAL
timing dimension for each node kind against the df_* oracle pins, and find
the unique survivor before porting.

model_check_stream models the beta-JOIN flush micro-order; it has no
accumulate node and no cross-rule agenda, so it cannot decide this. The
question here is purely AGENDA-level: when a clock job (expiration OR window
eviction) removes an event, WHEN is the resulting activation created —

  eager  : at advance-time -> the activation is on the agenda at fire-start,
           competes by SALIENCE, and the removal is applied to the node
           BEFORE the fire's inserts (no transient).
  lazy   : at agenda QUIESCENCE -> the activation fires AFTER every
           salience-ordered item, and the removal is applied to the node
           AFTER the fire's inserts (transient possible).

Dimensions enumerated (independently per node kind):
  acc_timing in {eager, lazy}   -- accumulate count/sum re-eval on removal
  not_timing in {eager, lazy}   -- not-CE unblock on removal

Simulator (states, not encoded answers): each fire builds a list of
activations (rule, value, phase, salience); phase=start activations fire
SALIENCE-desc (decl-order tie-break), then phase=quiescence activations fire
in creation order. The accumulate VALUE derives from which events are in the
bag at eval time, which depends on acc_timing (removal before vs after the
epoch's inserts). Pins are the LIVE-oracle df_* firing sequences.
"""
import itertools

# --- scenario model -----------------------------------------------------
# A scenario is (rules, init_bag, epoch), where:
#   rules  = [(name, kind, salience)]  kind in {"acc_count","acc_sum","not","plain"}
#   init_bag = list of event ts values initially in the accumulate
#   epoch  = {"remove":[ts...],       events removed by the clock job
#             "acc_ins":[ts...],      events inserted INTO the accumulate
#             "plain_ins": bool}      whether the concurrent plain rule's
#                                     trigger is inserted this epoch
# The not-CE is blocked while any event of the accumulate's type is alive,
# and unblocks when ALL are removed (init_bag drained, none re-inserted).

def simulate(scn, acc_timing, not_timing):
    _name, rules, init_bag, epoch = scn
    fires = []  # each = list of (rule, value)

    # ---- fire 0 (initial batch) ----
    acts0 = []
    for (name, kind, sal) in rules:
        if kind == "acc_count":
            acts0.append((name, len(init_bag), "start", sal))
        elif kind == "acc_sum":
            acts0.append((name, sum(init_bag), "start", sal))
        # not: blocked initially iff any event alive -> no fire if init_bag
        elif kind == "not" and not init_bag:
            acts0.append((name, None, "start", sal))
        # plain: its trigger not present at fire 0
    fires.append(order(acts0))

    # ---- epoch fire ----
    removed = epoch.get("remove", [])
    acc_ins = epoch.get("acc_ins", [])
    plain_ins = epoch.get("plain_ins", False)

    bag_after_ins_only = [e for e in init_bag] + acc_ins          # removal deferred
    bag_net = [e for e in init_bag if e not in removed] + acc_ins  # removal applied
    all_gone = len(bag_net) == 0

    acts = []
    for (name, kind, sal) in rules:
        if kind in ("acc_count", "acc_sum"):
            val = (lambda b: len(b) if kind == "acc_count" else sum(b))
            base = val(init_bag)  # value carried from fire 0
            if acc_timing == "eager":
                # removal applied before the insert -> single net re-eval
                v = val(bag_net)
                # Drools modifies the result even if the numeric value is
                # unchanged (underlying bag changed) -> it re-fires.
                if bag_net != init_bag:
                    acts.append((name, v, "start", sal))
            else:  # lazy
                # fire-start sees inserts only (removal deferred)
                if bag_after_ins_only != init_bag:
                    acts.append((name, val(bag_after_ins_only), "start", sal))
                # quiescence applies the removal -> net re-eval
                if bag_net != bag_after_ins_only:
                    acts.append((name, val(bag_net), "quiescence", sal))
        elif kind == "not":
            if all_gone and init_bag:  # became unblocked this epoch
                phase = "start" if not_timing == "eager" else "quiescence"
                acts.append((name, None, phase, sal))
        elif kind == "plain" and plain_ins:
            acts.append((name, None, "start", sal))
    fires.append(order(acts))
    return fires


def order(acts):
    """Fire order within one fire: phase=start by SALIENCE desc (decl-order
    tie-break, preserved by stable sort on -salience), then phase=quiescence
    in creation order."""
    start = [a for a in acts if a[2] == "start"]
    quies = [a for a in acts if a[2] == "quiescence"]
    start_sorted = sorted(start, key=lambda a: -a[3])  # stable => decl tie
    return [(a[0], a[1]) for a in (start_sorted + quies)]


def flat(fires):
    return [f for fire in fires for f in fire]


# --- pins (LIVE ORACLE, probes_pending/cep/df_*) ------------------------
# rules: (name, kind, salience); init_bag; epoch; want (flat firing seq)
PINS = [
    # df_ord_acc: acc-count sal5 + plain sal0; E expires, Q inserted
    (("df_ord_acc",
      [("Rx", "acc_count", 5), ("Rp", "plain", 0)], [0],
      {"remove": [0], "plain_ins": True}),
     [("Rx", 1), ("Rx", 0), ("Rp", None)]),
    # df_ord_acc_lo: acc-count sal0 + plain sal5
    (("df_ord_acc_lo",
      [("Rx", "acc_count", 0), ("Rp", "plain", 5)], [0],
      {"remove": [0], "plain_ins": True}),
     [("Rx", 1), ("Rp", None), ("Rx", 0)]),
    # df_ord_not: not sal5 + plain sal0
    (("df_ord_not",
      [("Rx", "not", 5), ("Rp", "plain", 0)], [0],
      {"remove": [0], "plain_ins": True}),
     [("Rp", None), ("Rx", None)]),
    # df_ord_not_lo: not sal0 + plain sal5
    (("df_ord_not_lo",
      [("Rx", "not", 0), ("Rp", "plain", 5)], [0],
      {"remove": [0], "plain_ins": True}),
     [("Rp", None), ("Rx", None)]),
    # df_ord_inter: acc sal5 + not sal3 + plain sal0, same event E
    (("df_ord_inter",
      [("Racc", "acc_count", 5), ("Rnot", "not", 3), ("Rp", "plain", 0)], [0],
      {"remove": [0], "plain_ins": True}),
     [("Racc", 1), ("Racc", 0), ("Rp", None), ("Rnot", None)]),
    # df_reins_sum: acc-sum; E@10 removed, E@100 inserted into the acc
    (("df_reins_sum",
      [("Rx", "acc_sum", 0)], [10],
      {"remove": [10], "acc_ins": [100]}),
     [("Rx", 10), ("Rx", 100)]),
    # df_reins_count: acc-count; E@10 removed, E@100 inserted (net count 1)
    (("df_reins_count",
      [("Rx", "acc_count", 0)], [10],
      {"remove": [10], "acc_ins": [100]}),
     [("Rx", 1), ("Rx", 1)]),
    # df_evict_reins: eviction is the same removal dimension (sum)
    (("df_evict_reins",
      [("Rx", "acc_sum", 0)], [10],
      {"remove": [10], "acc_ins": [100]}),
     [("Rx", 10), ("Rx", 100)]),
]


def main():
    survivors = []
    for acc_t, not_t in itertools.product(["eager", "lazy"], repeat=2):
        ok = True
        fails = []
        for scn, want in PINS:
            got = flat(simulate(scn, acc_t, not_t))
            if got != want:
                ok = False
                fails.append((scn[0], got, want))
        tag = f"acc={acc_t:<5} not={not_t:<5}"
        if ok:
            survivors.append((acc_t, not_t))
            print(f"SURVIVOR  {tag}")
        else:
            print(f"reject    {tag}  ({fails[0][0]}: {fails[0][1]} != {fails[0][2]})")
    print(f"\n{len(survivors)} survivor(s): {survivors}")
    if len(survivors) == 1:
        print(f"UNIQUE mechanism -> accumulate removals {survivors[0][0].upper()}, "
              f"not-CE {survivors[0][1].upper()}")


if __name__ == "__main__":
    main()
