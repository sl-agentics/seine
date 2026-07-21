"""Tire 13 — the collect CE: List() from collect(...). Alpha-only source;
fires ONCE gathering all matches (vs a plain pattern per-match); the collected
list is observable in the firings audit but not downstream-usable."""
import json
import seine_rs
from seine_rs import Rule, fact, CompileError

@fact
class Sensor:
    id: int
    status: str
@fact
class Report:
    tag: int

# collect: one firing, all faulty sensors gathered into an ArrayList
gather = Rule("GatherFaulty")
gather.collect(Sensor, Sensor.status == "fault")
gather.then_insert(Report, tag=1)
print("collect DRL:")
print(gather.to_drl())

# plain pattern: one firing PER faulty sensor
each = Rule("EachFaulty")
s = each.when(Sensor, Sensor.status == "fault")
each.then_insert(Report, tag=2)

sensors = [Sensor(1, "fault"), Sensor(2, "ok"), Sensor(3, "fault"), Sensor(4, "fault")]

res_c = seine_rs.run([gather], {Sensor: sensors, Report: []})
res_e = seine_rs.run([each],   {Sensor: sensors, Report: []})
print("collect -> # Report firings:", len(res_c.derived["Report"].to_pylist()), "(ONE, all gathered)")
print("plain   -> # Report firings:", len(res_e.derived["Report"].to_pylist()), "(one per faulty sensor)")

print("\ncollect firing audit (the ArrayList of faulty sensors):")
for row in res_c.firings.to_pylist():
    print(f"   pos {row['pos']}: type={row['type']:10} handle={row['handle']} values={row['values_json']}")

# empty collection: does collect still fire once (with an empty list)?
res_empty = seine_rs.run([gather], {Sensor: [Sensor(9, "ok")], Report: []})
print("\nempty match set -> collect firings:", len(res_empty.derived["Report"].to_pylist()),
      "(collect fires once even over an empty collection)")

# the alpha-only wall (collect source cannot reference another pattern)
print("\nalpha-only wall:")
try:
    r = Rule("Bad"); base = r.when(Report); r.collect(Sensor, Sensor.id == base.tag)
    print("  [NO ERROR!]")
except CompileError as e:
    print("  ✓", str(e).splitlines()[0][:90])
