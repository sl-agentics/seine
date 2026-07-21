"""Tire 9 — forward-chaining cascade: RHS then_modify/then_insert/then_delete
drive a state machine to completion in a single fire()."""
import seine_rs
from seine_rs import Rule, fact

@fact
class Order:
    id: int
    state: str
@fact
class Archived:
    id: int

# new -> validated -> shipped -> (archived + order deleted), all in one fire
validate = Rule("Validate", no_loop=True)
o1 = validate.when(Order, Order.state == "new")
validate.then_modify(o1, state="validated")

ship = Rule("Ship", no_loop=True)
o2 = ship.when(Order, Order.state == "validated")
ship.then_modify(o2, state="shipped")

archive = Rule("Archive")
o3 = archive.when(Order, Order.state == "shipped")
archive.then_insert(Archived, id=o3.id)
archive.then_delete(o3)

sess = seine_rs.Session([validate, ship, archive], {Order: [], Archived: []})
sess.insert_row(Order, {"id": 1, "state": "new"})

trail = []
res = sess.fire(on_fire=lambda rule, tup: trail.append(rule))
print("firing cascade (single fire):", trail)
print("final Orders  :", res.facts.get("Order").to_pylist() if "Order" in res.facts else [])
print("final Archived:", res.facts.get("Archived").to_pylist() if "Archived" in res.facts else [])
print("audit trail (rule @ seq):")
for row in res.firings.to_pylist():
    if row["type"] == "Order":
        print(f"   seq {row['seq']}: {row['rule']:9} matched Order {row['values_json']}")
