"""Tire 1 — Truth maintenance: insertLogical, why(), auto-retraction."""
import seine_rs
from seine_rs import Rule, fact

@fact
class Txn:
    id: int
    account: str
    amount: float

@fact
class Suspicious:          # a DERIVED view — justified, never stated
    account: str
    amount: float

# Any txn >= 10k logically implies a Suspicious marker for that account.
flag = Rule("FlagLarge")
t = flag.when(Txn, Txn.amount >= 10_000)
flag.then_insert_logical(Suspicious, account=t.account, amount=t.amount)

sess = seine_rs.Session([flag], {Txn: [], Suspicious: []})

h1 = sess.insert_row(Txn, {"id": 1, "account": "alice", "amount": 25_000.0})
h2 = sess.insert_row(Txn, {"id": 2, "account": "bob",   "amount":  9_999.0})
h3 = sess.insert_row(Txn, {"id": 3, "account": "carol", "amount": 40_000.0})
print("inserted txn handles:", h1, h2, h3)

fired = sess.fire()
print("fired:", fired)

print("\n-- justification graph after first fire --")
for j in sess.justifications():
    print(" ", j)

# Grab a derived Suspicious handle from the graph and ask WHY.
derived = sess.justifications()
alice_view = next(j for j in derived if j["fields"]["account"] == "alice")
print("\n-- why(Suspicious alice, fact %s) --" % alice_view["fact"])
import json
print(json.dumps(sess.why(alice_view["fact"]), indent=2, default=str))

# Now WITHDRAW the support: delete alice's big txn. The derived
# Suspicious should retract on its own at the next quiescence.
print("\n-- delete supporting txn (alice, handle %s) then fire --" % h1)
sess.delete(h1)
sess.fire()

print("\n-- justification graph after support withdrawn --")
after = sess.justifications()
for j in after:
    print(" ", j)
print("alice still derived?", any(j["fields"]["account"] == "alice" for j in after))
print("carol still derived?", any(j["fields"]["account"] == "carol" for j in after))
