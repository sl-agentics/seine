"""Tire 6 — exact Decimal arithmetic vs IEEE float drift; average_exact
rounding; the average-over-decimal wall (D-341)."""
import decimal, functools, operator, math
from typing import Annotated
import seine_rs
from seine_rs import Rule, fact, sum_, average, average_exact, CompileError

D = decimal.Decimal

@fact
class Payment:
    amt_f: float                                          # IEEE double
    amt_d: Annotated[decimal.Decimal, seine_rs.Decimal(18, 2)]  # exact money

@fact
class FloatTotal:
    t: float
@fact
class DecTotal:
    t: Annotated[decimal.Decimal, seine_rs.Decimal(38, 2)]      # sum widens to (38,s)

# same numbers, two lanes: f64 sum vs exact-decimal sum
rf = Rule("SumFloat"); f = rf.accumulate(Payment, agg=sum_(Payment.amt_f)); rf.then_insert(FloatTotal, t=f)
rd = Rule("SumDec");   d = rd.accumulate(Payment, agg=sum_(Payment.amt_d)); rd.then_insert(DecTotal,  t=d)

vals = [("0.10", 0.10)] * 10          # ten dimes -> exactly $1.00
pays = [Payment(amt_f=flt, amt_d=D(s)) for (s, flt) in vals]

res = seine_rs.run([rf, rd], {Payment: pays, FloatTotal: [], DecTotal: []})
eng_f = res.derived["FloatTotal"].to_pylist()[0]["t"]
eng_d = res.derived["DecTotal"].to_pylist()[0]["t"]
floats = [flt for _, flt in vals]
naive_fold = functools.reduce(operator.add, floats, 0.0)  # Java/Drools-style
compensated = sum(floats)                                 # CPython 3.12+ Neumaier
exact_float = math.fsum(floats)                           # exact float sum

print("== sum of ten $0.10 payments (should be exactly $1.00) ==")
print(f"  engine f64 sum         : {eng_f!r}")
print(f"  naive left-fold (Java) : {naive_fold!r}   <- engine MATCHES this")
print(f"  python sum() 3.12+     : {compensated!r}   (compensated — hides drift)")
print(f"  math.fsum()            : {exact_float!r}   (exact float)")
print(f"  engine decimal sum     : {eng_d!r}   (type {type(eng_d).__name__}) <- exact money")
print(f"  --> engine f64 == naive Java double sum? {eng_f == naive_fold} ; "
      f"decimal exact? {D(str(eng_d)) == D('1.00')}")

# ---- average_exact rounding modes ----
@fact
class AvgOut:
    a: Annotated[decimal.Decimal, seine_rs.Decimal(38, 2)]

def avg_exact(payments, scale, mode):
    r = Rule("Avg")
    a = r.accumulate(Payment, agg=average_exact(Payment.amt_d, scale=scale, rounding=mode))
    r.then_insert(AvgOut, a=a)
    return seine_rs.run([r], {Payment: payments, AvgOut: []}).derived["AvgOut"].to_pylist()[0]["a"]

print("\n== average_exact of [10.00, 10.00, 5.00] = 25/3 = 8.333... @ scale 2 ==")
non_tie = [Payment(0.0, D("10.00")), Payment(0.0, D("10.00")), Payment(0.0, D("5.00"))]
for mode in ("half_up", "half_down", "floor", "ceiling", "half_even"):
    print(f"  rounding={mode:10} -> {avg_exact(non_tie, 2, mode)}")

print("\n== average_exact of [8.12, 8.13] = 8.125 — a TRUE .5 tie @ scale 2 ==")
tie = [Payment(0.0, D("8.12")), Payment(0.0, D("8.13"))]
for mode in ("half_up", "half_down", "half_even", "ceiling", "floor"):
    print(f"  rounding={mode:10} -> {avg_exact(tie, 2, mode)}   "
          f"{'<- banker rounds to even (2)' if mode=='half_even' else ''}")

# ---- the wall: IEEE average over a decimal field is out of subset ----
print("\n== D-341 wall ==")
try:
    average(Payment.amt_d)
    print("  [NO ERROR!]")
except CompileError as e:
    print("  ✓", str(e).splitlines()[0][:100])
