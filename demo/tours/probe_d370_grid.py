"""Probe the D-370 g-grid from the wheel side: reproduce SURVIVAL (starved +
static + external delete + no refocus) vs my original RETRACTION (Premise-keyed
kick re-grants focus). The sess.delete() cascade is the frozen-vs-drained tell."""
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

def run_case(kick_on, label):
    """kick_on='premise' -> kick LHS is Premise (can it re-focus?); 'marker' -> keyed on a
    separate Marker so it fires once and cannot re-activate after the premise delete."""
    believe = Rule("Believe", agenda_group="deferred")   # static salience, starved unless focused
    pr = believe.when(Premise)
    believe.then_insert_logical(Belief, id=pr.id)

    kick = Rule("Kick")
    kick.when(Premise if kick_on == "premise" else Marker)
    kick.then_set_focus("deferred")

    sess = seine_rs.Session([kick, believe], {Marker: [], Premise: [], Belief: []})
    sess.insert_row(Marker, {"tag": 1})
    hp = sess.insert_row(Premise, {"id": 7})
    b_setup = beliefs(sess.fire())
    cascade = sess.delete(hp)                 # <-- staged-delete cascade at delete time
    b_after = beliefs(sess.fire())            # <-- does a later fire drain the staged delete?
    verdict = "SURVIVES (frozen)" if b_after == [7] else "retracts"
    print(f"  kick_on={kick_on:8} [{label}]")
    print(f"      belief after setup     : {b_setup}")
    print(f"      sess.delete() cascade  : {cascade!r}")
    print(f"      belief after next fire : {b_after}   -> {verdict}")

print("D-370 g-grid, wheel-side reproduction:")
run_case("marker",  "starved+static, kick can't re-activate")
run_case("premise", "kick LHS = Premise (refocus arm?)")
