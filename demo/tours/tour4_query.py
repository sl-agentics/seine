"""Tire 4 — DRL queries: bound + unbound params against working memory."""
import seine_rs
from seine_rs import fact

@fact
class Person:
    name: str
    age: int
    city: str

DRL = '''
query "adultsInCity" (String $city)
    Person($name : name, $age : age, city == $city, age >= 18)
end

query "everyone" ()
    Person($name : name, $age : age, $city : city)
end
'''

people = [
    Person("ada",   36, "portland"),
    Person("kurt",  17, "portland"),
    Person("grace", 45, "austin"),
    Person("alan",  29, "austin"),
    Person("edsger",16, "austin"),
]

sess = seine_rs.Session(DRL, {Person: people})
sess.fire()

print("== query 'adultsInCity' BOUND to 'austin' ==")
for row in sess.query("adultsInCity", "austin"):
    print("  ", row)

print("\n== query 'adultsInCity' with UNBOUND $city (pass None) ==")
print("   ($city binds per-row; returns every adult with their city)")
for row in sess.query("adultsInCity", None):
    print("  ", row)

print("\n== query 'everyone' (no params) ==")
for row in sess.query("everyone"):
    print("  ", row)
