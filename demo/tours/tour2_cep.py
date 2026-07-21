"""Tire 2 — Temporal CEP: Allen `after`, sliding time window, expiration."""
import seine_rs
from seine_rs import Rule, fact, Event, count

# ================= 2a: Allen `after` sequence detection =================
@fact(event=Event(timestamp="ts", expires_ms=10_000))
class Login:
    user: str
    ts: int

@fact(event=Event(timestamp="ts", expires_ms=10_000))
class Failure:
    user: str
    ts: int

@fact
class Alert:
    user: str
    kind: str

seq = Rule("LoginThenFailure")
lg = seq.when(Login)
seq.when(Failure, Failure.user == lg.user, seine_rs.this_after(lg, 0, 2_000))
seq.then_insert(Alert, user=lg.user, kind="login_then_fail")

print("========== 2a DRL ==========")
print(seq.to_drl())

res = seine_rs.run([seq], {
    Login:   [Login("alice", 1_000), Login("bob", 1_000)],
    Failure: [Failure("alice", 2_500),   # +1500ms after alice login -> HIT
              Failure("bob",   9_000),    # +8000ms after bob login   -> miss (>2s)
              Failure("alice",   100)],   # BEFORE alice login          -> miss
    Alert: [],
})
alerts = res.derived["Alert"].to_pylist()
print("2a alerts:", alerts)
assert [(a["user"], a["kind"]) for a in alerts] == [("alice", "login_then_fail")], "unexpected"

# ============ 2b: sliding time window + event expiration ============
@fact
class WindowCount:
    n: int

@fact
class Critical:
    n: int

# Rule 1: count Failure events currently inside a 3s sliding window.
win = Rule("CountInWindow")
c = win.accumulate(Failure, agg=count(), window=seine_rs.window_time(3_000))
win.then_insert(WindowCount, n=c)

# Rule 2 (certified threshold idiom): downstream match on the aggregate fact.
crit = Rule("EscalateBurst")
wc = crit.when(WindowCount, WindowCount.n >= 3)
crit.then_insert(Critical, n=wc.n)

print("\n========== 2b DRL (windowed count + threshold chain) ==========")
print(win.to_drl())
print(crit.to_drl())

def counts(res):
    f = res.facts
    wcs = f["WindowCount"].to_pylist() if "WindowCount" in f else []
    crs = f["Critical"].to_pylist() if "Critical" in f else []
    return [w["n"] for w in wcs], [x["n"] for x in crs]

sess = seine_rs.Session([win, crit], {Failure: [], WindowCount: [], Critical: []})

# three failures clustered at t=1000,1200,1500 (all within any 3s window)
for ts in (1_000, 1_200, 1_500):
    sess.insert_row(Failure, {"user": "carol", "ts": ts})
r1 = sess.fire()
wc1, cr1 = counts(r1)
print("\nafter 3 clustered failures  -> WindowCount seen:", wc1, "| Critical:", cr1)

# advance the pseudo-clock well past the 3s window so the cluster expires,
# then drop in a single lone failure.
sess.reset()  # clean slate to isolate the expiration observation
for ts in (1_000, 1_200, 1_500):
    sess.insert_row(Failure, {"user": "carol", "ts": ts})
sess.fire()
print("advancing clock by 5000ms (past the 3s window)...")
sess.advance(5_000)
sess.insert_row(Failure, {"user": "carol", "ts": 7_000})  # lone, in-window now
r2 = sess.fire()
wc2, cr2 = counts(r2)
print("after expiry + 1 lone failure -> WindowCount seen:", wc2, "| Critical:", cr2)
