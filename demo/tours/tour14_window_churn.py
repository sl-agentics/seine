"""Tire 14 — windowed aggregate under insert/update/delete churn.
sum over window:length(3); watch the window track, then churn it and observe
the shrink/re-admission ordering (the flagged pinned-subtlety lane)."""
import seine_rs
from seine_rs import Rule, fact, sum_, count, window_length

@fact
class Reading:
    v: int
@fact
class WinSum:
    t: int
@fact
class WinCnt:
    n: int

rs = Rule("WindowSum")
c = rs.accumulate(Reading, agg=sum_(Reading.v), window=window_length(3))
rs.then_insert_logical(WinSum, t=c)
rc = Rule("WindowCnt")
n = rc.accumulate(Reading, agg=count(), window=window_length(3))
rc.then_insert_logical(WinCnt, n=n)

sess = seine_rs.Session([rs, rc], {Reading: [], WinSum: [], WinCnt: []})
def state(res):
    ws = res.facts.get("WinSum"); wc = res.facts.get("WinCnt")
    s = ws.to_pylist()[0]["t"] if ws and ws.to_pylist() else None
    k = wc.to_pylist()[0]["n"] if wc and wc.to_pylist() else None
    return f"sum={s} count={k}"

print("== fill the length-3 window ==")
H = {}
for v in (10, 20, 30, 40, 50):
    H[v] = sess.insert_row(Reading, {"v": v})
    r = sess.fire()
    print(f"  insert {v:3} -> {state(r)}   (naive last-3 window)")

print("\n== churn ==")
sess.update(H[40], v=400)                       # mutate a reading INSIDE the window
print(f"  update 40->400 -> {state(sess.fire())}   (expect window {{30,400,50}} = 480)")

sess.delete(H[50])                              # delete the newest -> window SHRINKS
print(f"  delete 50      -> {state(sess.fire())}   (<-- shrink: does an older reading slide back in?)")

sess.delete(H[30])                              # delete a mid reading
print(f"  delete 30      -> {state(sess.fire())}")

h60 = sess.insert_row(Reading, {"v": 60})       # re-grow after churn
print(f"  insert 60      -> {state(sess.fire())}   (admission order after deletes?)")
