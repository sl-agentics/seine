"""Tire 5 — WM lifecycle (update/delete by handle), salience, grammar wall."""
import seine_rs
from seine_rs import Rule, fact, CompileError, sum_, min_, count

# ================= 5a: insert -> fire -> update -> delete =================
@fact
class Account:
    id: str
    balance: float
@fact
class Overdrawn:            # derived view, kept in sync by TMS
    id: str
    balance: float

od = Rule("Overdraft")
a = od.when(Account, Account.balance < 0)
od.then_insert_logical(Overdrawn, id=a.id, balance=a.balance)

sess = seine_rs.Session([od], {Account: [], Overdrawn: []})
h_alice = sess.insert_row(Account, {"id": "alice", "balance": 100.0})
h_bob   = sess.insert_row(Account, {"id": "bob",   "balance": -50.0})

def overdrawn(res):
    t = res.facts.get("Overdrawn")
    return sorted(x["id"] for x in t.to_pylist()) if t else []

r = sess.fire();                         print("5a init  (alice=100, bob=-50)   overdrawn:", overdrawn(r))
sess.update(h_bob, balance=20.0)         # bob deposits -> no longer overdrawn
r = sess.fire();                         print("5a bob deposits to 20          overdrawn:", overdrawn(r))
sess.update(h_alice, balance=-30.0)      # alice goes negative
r = sess.fire();                         print("5a alice drops to -30          overdrawn:", overdrawn(r))
sess.delete(h_alice)                     # close alice's account
r = sess.fire();                         print("5a alice account deleted       overdrawn:", overdrawn(r))

# ================= 5b: salience / agenda ordering =================
@fact
class Ticket:
    id: int
    sev: str
@fact
class Routed:
    id: int
    lane: str

# Same Ticket matched by two rules; higher salience wins the agenda.
hi = Rule("CritFirst", salience=100)
t1 = hi.when(Ticket, Ticket.sev == "crit")
hi.then_insert(Routed, id=t1.id, lane="pager")

lo = Rule("LogAll", salience=1)
t2 = lo.when(Ticket)
lo.then_insert(Routed, id=t2.id, lane="log")

order = []
seine_rs.Session([hi, lo], {Ticket: [Ticket(1, "crit")], Routed: []}).fire(
    on_fire=lambda rule, tup: order.append(rule))
print("\n5b fire order (salience 100 then 1):", order)

# ================= 5c: the certified-grammar wall =================
print("\n5c CompileError guardrails (definition-time rejections):")
def wall(label, thunk):
    try:
        thunk()
        print(f"   [NO ERROR!] {label}")
    except CompileError as e:
        print(f"   ✓ {label}\n       -> {str(e).splitlines()[0][:96]}")

@fact
class Src:
    v: float
    k: int
@fact
class Out:
    v: float

def callable_in_rhs():
    r = Rule("Bad"); s = r.when(Src); r.then_insert(Out, v=lambda x: x)
wall("python callable as an insert value", callable_in_rhs)

def minmax_float_downstream():
    r = Rule("Bad"); m = r.accumulate(Src, agg=min_(Src.v)); r.then_insert(Out, v=m)
wall("min() over a float used downstream (opaque Number)", minmax_float_downstream)

def acc_in_salience():
    r = Rule("Bad"); c = r.accumulate(Src, agg=count())
    r.set_salience(c)
wall("accumulate result inside a salience expression", acc_in_salience)

def collect_cross_pattern():
    r = Rule("Bad"); base = r.when(Out); r.collect(Src, Src.v == base.v)
wall("collect() source referencing another pattern (RIA subnet)", collect_cross_pattern)

def event_window_on_nonevent():
    r = Rule("Bad")
    r.accumulate(Src, agg=count(), window=seine_rs.window_time(1000))
wall("time window over a non-event type", event_window_on_nonevent)
