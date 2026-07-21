"""Tire 12 — agenda groups + focus stack; plus a targeted check of the
D-370 'starved group's logical beliefs survive' behavior."""
import seine_rs
from seine_rs import Rule, fact

@fact
class Trigger:
    id: int
@fact
class Ran:
    id: int

# ---------- 1: a non-focused agenda group is STARVED ----------
deferred_only = Rule("OnlyOnFocus", agenda_group="deferred")
tr = deferred_only.when(Trigger)
deferred_only.then_insert(Ran, id=tr.id)

sess = seine_rs.Session([deferred_only], {Trigger: [], Ran: []})
sess.insert_row(Trigger, {"id": 1})
r = sess.fire()
ran = r.facts.get("Ran").to_pylist() if "Ran" in r.facts else []
print("1) deferred group, never focused -> Ran:", ran, "(starved: rule never fires)")

# ---------- 2: setFocus grants the group its turn ----------
kick = Rule("Kick")                        # MAIN group (has focus by default)
tk = kick.when(Trigger)
kick.then_set_focus("deferred")            # push 'deferred' onto the focus stack

sess2 = seine_rs.Session([kick, deferred_only], {Trigger: [], Ran: []})
sess2.insert_row(Trigger, {"id": 1})
order = []
r = sess2.fire(on_fire=lambda rule, tup: order.append(rule))
print("2) with Kick -> setFocus('deferred'):  fire order", order,
      "| Ran:", [x["id"] for x in (r.facts.get("Ran").to_pylist() if "Ran" in r.facts else [])])

# ---------- 3: D-370 boundary — a 1-pattern derive retracts EAGERLY ----------
# (Historical note: this construction originally expected D-370 survival and
# saw retraction — that honest-negative became the g12/g13 round. The law:
# a SINGLE-pattern derive is LIA-terminal — no beta segment for the staged
# delete to park in — so the external premise delete unjustifies at the
# action, group-independent (pr_nl_g13_extdel_1pat). The starved-group
# SURVIVAL cell needs a 2-pattern JOIN derive: see probe_d370_arity.py.)
@fact
class Premise:
    id: int
@fact
class Belief:
    id: int

believe = Rule("Believe", agenda_group="deferred")
pr = believe.when(Premise)
believe.then_insert_logical(Belief, id=pr.id)

kick2 = Rule("KickBelieve")
kp = kick2.when(Premise)
kick2.then_set_focus("deferred")

def beliefs(res): return sorted(x["id"] for x in res.facts.get("Belief").to_pylist()) if "Belief" in res.facts else []

sess3 = seine_rs.Session([kick2, believe], {Premise: [], Belief: []})
hp = sess3.insert_row(Premise, {"id": 7})
r = sess3.fire()
print("\n3) Premise in, deferred focused -> Belief:", beliefs(r))
sess3.delete(hp)                            # withdraw the justifying premise
r = sess3.fire()                            # MAIN has focus now; deferred starved
survived = beliefs(r)
print("   delete Premise, fire (deferred now starved) -> Belief:", survived)
print("   g13 expectation (1-pattern derive = eager unjustify at the delete):",
      "CONFIRMED ✓" if survived == [] else f"UNEXPECTED (got {survived})")
print("   (for the D-370 starved-group SURVIVAL cell, see probe_d370_arity.py)")
