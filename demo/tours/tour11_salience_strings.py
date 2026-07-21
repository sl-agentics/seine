"""Tire 11 — data-driven (dynamic) salience ordering, and string/null ops."""
import json
from typing import Optional
import seine_rs
from seine_rs import Rule, fact

# ---------- dynamic salience: agenda order follows a bound field ----------
@fact
class Task:
    name: str
    priority: int
@fact
class Done:
    name: str

proc = Rule("Process")
t = proc.when(Task)
proc.set_salience(t.priority)          # salience($priority) — data drives order
proc.then_insert(Done, name=t.name)
print("dynamic-salience DRL header:")
print("\n".join(proc.to_drl().splitlines()[:4]))

tasks = [Task("low", 1), Task("urgent", 10), Task("med", 5), Task("crit", 99)]
res = seine_rs.run([proc], {Task: tasks, Done: []})
fire_order = [json.loads(r["values_json"])["name"]
              for r in sorted(res.firings.to_pylist(), key=lambda r: r["seq"])
              if r["type"] == "Task"]
print("fire order (should be priority-desc):", fire_order)
assert fire_order == ["crit", "urgent", "med", "low"], fire_order
print("  ✓ agenda fired highest-salience first, by data\n")

# ---------- string / null operators ----------
@fact
class Person:
    name: str
    email: Optional[str]     # nullable String
    tag: str
@fact
class Match:
    name: str
    why: str

people = [
    Person("Ada",   "ada@x.io", "vip"),
    Person("Alan",  None,       "gold"),
    Person("bob",   "bob@x.io", "std"),
    Person("Eve",   None,       "vip"),
]

def rule(name, why, *cons):
    r = Rule(name); p = r.when(Person, *cons); r.then_insert(Match, name=p.name, why=why); return r

rules = [
    rule("StartsA", "name ~ A.* (full-match)", Person.name.matches("A.*")),  # Java matches() is full-string
    rule("HasAt",   "email contains @", Person.email.contains("@")),
    rule("NoEmail", "email is null",    Person.email.is_null()),
    rule("VipGold", "tag in {vip,gold}",Person.tag.in_("vip", "gold")),
]
res = seine_rs.run(rules, {Person: people, Match: []})
by_why = {}
for m in res.derived["Match"].to_pylist():
    by_why.setdefault(m["why"], []).append(m["name"])
for why, names in by_why.items():
    print(f"  {why:22} -> {sorted(names)}")
