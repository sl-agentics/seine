"""D-370 arity boundary, both cells on ONE 0.4.47 .so (commit 074b605).
Marker-keyed kick throughout (kick shape is NOT the discriminator).
Only the DERIVE arity changes: 2-pattern join vs 1-pattern LIA-terminal."""
import seine_rs
from seine_rs import Rule, fact

@fact
class Marker:
    tag: int
@fact
class Premise:
    id: int
@fact
class Belief:
    id: int

def beliefs(res):
    return sorted(x["id"] for x in res.facts.get("Belief").to_pylist()) if "Belief" in res.facts else []

def run_case(join_derive, label):
    believe = Rule("Believe", agenda_group="deferred")   # starved + static salience
    if join_derive:
        believe.when(Marker)          # <-- beta segment: staged delete parks here
    pr = believe.when(Premise)        # single pattern alone = LIA-terminal (no segment)
    believe.then_insert_logical(Belief, id=pr.id)

    kick = Rule("Kick")
    kick.when(Marker)                 # fires once; cannot re-activate on the premise delete
    kick.then_set_focus("deferred")

    sess = seine_rs.Session([kick, believe], {Marker: [], Premise: [], Belief: []})
    sess.insert_row(Marker, {"tag": 1})
    hp = sess.insert_row(Premise, {"id": 7})
    setup = beliefs(sess.fire())
    cascade = sess.delete(hp)                    # frozen -> [] ; eager -> [belief handle]
    after = beliefs(sess.fire())
    verdict = "SURVIVES (frozen in beta segment)" if after == [7] else "retracts (eager, LIA-terminal)"
    print(f"  {label}")
    print(f"      derive arity           : {'2-pattern JOIN (Marker⋈Premise)' if join_derive else '1-pattern (Premise only)'}")
    print(f"      belief after setup     : {setup}")
    print(f"      sess.delete() cascade  : {cascade!r}")
    print(f"      belief after next fire : {after}   -> {verdict}")

print("D-370 arity boundary on 0.4.47 @ 074b605 (one .so, both cells):\n")
run_case(True,  "pr_nl_g12_extdel_starved  (join, frozen)")
print()
run_case(False, "pr_nl_g13_extdel_1pat     (LIA, eager)")
