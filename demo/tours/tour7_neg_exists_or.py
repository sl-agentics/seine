"""Tire 7 — negation (when_not), existential (when_exists, no multiplication),
and OR across classes (when_any)."""
import seine_rs
from seine_rs import Rule, fact

# ---------- 7a: negation with a join (orders with no shipment) ----------
@fact
class Order:
    id: int
@fact
class Shipment:
    order_id: int
@fact
class Unshipped:
    id: int

r7a = Rule("FindUnshipped")
o = r7a.when(Order)
r7a.when_not(Shipment, Shipment.order_id == o.id)
r7a.then_insert(Unshipped, id=o.id)

res = seine_rs.run([r7a], {
    Order:    [Order(1), Order(2), Order(3)],
    Shipment: [Shipment(1), Shipment(3)],   # 2 has no shipment
    Unshipped: [],
})
print("7a when_not — unshipped orders:", sorted(x["id"] for x in res.derived["Unshipped"].to_pylist()))

# ---------- 7b: existential vs join multiplication ----------
@fact
class Customer:
    id: int
@fact
class Purchase:
    cust: int
    amount: float
@fact
class BigSpenderExists:
    id: int
@fact
class BigSpenderJoin:
    id: int

# EXISTS: fires ONCE per customer no matter how many big purchases.
r_ex = Rule("BigExists")
c = r_ex.when(Customer)
r_ex.when_exists(Purchase, Purchase.cust == c.id, Purchase.amount >= 1000.0)
r_ex.then_insert(BigSpenderExists, id=c.id)

# JOIN: fires once PER matching big purchase (row multiplication).
r_jn = Rule("BigJoin")
c2 = r_jn.when(Customer)
r_jn.when(Purchase, Purchase.cust == c2.id, Purchase.amount >= 1000.0)
r_jn.then_insert(BigSpenderJoin, id=c2.id)

facts = {
    Customer: [Customer(1), Customer(2)],
    Purchase: [Purchase(1, 5000.0), Purchase(1, 2000.0),  # alice: TWO big
               Purchase(2, 50.0)],                         # bob: none big
    BigSpenderExists: [], BigSpenderJoin: [],
}
res = seine_rs.run([r_ex, r_jn], facts)
print("7b when_exists  -> BigSpenderExists rows:", res.derived["BigSpenderExists"].to_pylist(),
      "(alice ONCE)")
print("7b plain join   -> BigSpenderJoin  rows:", res.derived["BigSpenderJoin"].to_pylist(),
      "(alice TWICE — one per big purchase)")

# ---------- 7c: OR across two classes (when_any) ----------
@fact
class SmokeDetected:
    zone: int
@fact
class HeatSpike:
    zone: int
@fact
class Alarm:
    n: int

r7c = Rule("FireAlarm")
r7c.when_any((SmokeDetected,), (HeatSpike,))   # alpha-only OR, no bind-out
r7c.then_insert(Alarm, n=1)
print("\n7c when_any DRL:")
print(r7c.to_drl())
res = seine_rs.run([r7c], {SmokeDetected: [SmokeDetected(3)], HeatSpike: [HeatSpike(7), HeatSpike(9)], Alarm: []})
print("7c alarms fired (1 smoke + 2 heat -> 3 firings):", len(res.derived["Alarm"].to_pylist()))
