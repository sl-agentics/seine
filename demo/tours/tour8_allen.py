"""Tire 8 — the full Allen interval algebra on interval events.
Anchor A = [100,200]. 13 probes, one per Allen relation. For each operator,
check it fires for exactly its matching probe (the diagonal)."""
import seine_rs
from seine_rs import Rule, fact, Event
from seine_rs import (this_before, this_after, this_meets, this_metby,
    this_overlaps, this_overlappedby, this_during, this_includes,
    this_starts, this_startedby, this_finishes, this_finishedby, this_coincides)

EV = Event(timestamp="ts", duration="dur", expires_ms=10**9)

@fact(event=EV)
class Anchor:
    ts: int
    dur: int
@fact(event=EV)
class Probe:
    ts: int
    dur: int
    label: str
@fact
class Hit:
    label: str

anchor = Anchor(ts=100, dur=100)                     # interval [100, 200]
# each probe interval placed in exactly one Allen relation to [100,200]:
PROBES = [
    ("before",       0,  50),   # [0,50]
    ("meets",       50,  50),   # [50,100]  end==A.start
    ("overlaps",    50, 100),   # [50,150]
    ("starts",     100,  50),   # [100,150] same start, ends first
    ("during",     120,  60),   # [120,180] strictly inside
    ("finishes",   150,  50),   # [150,200] same end, starts later
    ("coincides",  100, 100),   # [100,200] equal
    ("finishedby",  50, 150),   # [50,200]  same end, starts earlier
    ("includes",    50, 200),   # [50,250]  strictly contains A
    ("startedby",  100, 150),   # [100,250] same start, ends later
    ("overlappedby",150,100),   # [150,250]
    ("metby",      200,  50),   # [200,250] start==A.end
    ("after",      250,  50),   # [250,300]
]
probes = [Probe(ts=ts, dur=dur, label=lab) for (lab, ts, dur) in PROBES]

# operator -> constraint factory (bare Allen form; before/after need a gap range
# tight enough to exclude the gap-0 neighbours meets/metby)
OPS = [
    ("before",       lambda a: this_before(a, 10, 10**9)),
    ("meets",        lambda a: this_meets(a)),
    ("overlaps",     lambda a: this_overlaps(a)),
    ("starts",       lambda a: this_starts(a)),
    ("during",       lambda a: this_during(a)),
    ("finishes",     lambda a: this_finishes(a)),
    ("coincides",    lambda a: this_coincides(a)),
    ("finishedby",   lambda a: this_finishedby(a)),
    ("includes",     lambda a: this_includes(a)),
    ("startedby",    lambda a: this_startedby(a)),
    ("overlappedby", lambda a: this_overlappedby(a)),
    ("metby",        lambda a: this_metby(a)),
    ("after",        lambda a: this_after(a, 10, 10**9)),
]

print(f"{'operator':14} fired-for-probes            diagonal?")
print("-" * 52)
all_ok = True
for name, mk in OPS:
    r = Rule("T_" + name)
    a = r.when(Anchor)
    b = r.when(Probe, mk(a))
    r.then_insert(Hit, label=b.label)
    res = seine_rs.run([r], {Anchor: [anchor], Probe: probes, Hit: []})
    fired = sorted(x["label"] for x in res.derived["Hit"].to_pylist())
    ok = (fired == [name])
    all_ok &= ok
    print(f"{name:14} {str(fired):27} {'✓' if ok else '✗ EXPECTED ['+name+']'}")

print("-" * 52)
print("FULL DIAGONAL — every operator matched exactly its own relation:", all_ok)
