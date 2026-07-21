"""Tire 10 — TMS edges: multi-support survival, and stated_siblings
(a fact that is BOTH stated and logically derived)."""
import seine_rs
from seine_rs import Rule, fact
import json

@fact
class Signal:
    kind: str
@fact
class Risk:
    account: str

# two INDEPENDENT rules both logically derive the SAME Risk(acme)
fraud = Rule("FraudImpliesRisk")
sf = fraud.when(Signal, Signal.kind == "fraud")
fraud.then_insert_logical(Risk, account="acme")

aml = Rule("AmlImpliesRisk")
sa = aml.when(Signal, Signal.kind == "aml")
aml.then_insert_logical(Risk, account="acme")

def risks(res): return sorted(x["account"] for x in res.facts.get("Risk").to_pylist()) if "Risk" in res.facts else []
def supports_of_acme(sess):
    j = [x for x in sess.justifications() if x["fields"]["account"] == "acme"]
    return [s["rule"] for x in j for s in x["supports"]] if j else None

print("===== multi-support survival =====")
sess = seine_rs.Session([fraud, aml], {Signal: [], Risk: []})
hf = sess.insert_row(Signal, {"kind": "fraud"})
ha = sess.insert_row(Signal, {"kind": "aml"})
r = sess.fire()
print("both signals in -> Risk:", risks(r), "| supports:", supports_of_acme(sess))
sess.delete(hf); r = sess.fire()
print("drop fraud signal -> Risk:", risks(r), "| supports:", supports_of_acme(sess), "(survives on aml)")
sess.delete(ha); r = sess.fire()
print("drop aml signal   -> Risk:", risks(r), "(all support gone -> retracts)")

print("\n===== stated_siblings (both stated AND derived) =====")
sess2 = seine_rs.Session([fraud], {Signal: [], Risk: []})
hstate = sess2.insert_row(Risk, {"account": "acme"})     # STATED directly
hsig = sess2.insert_row(Signal, {"kind": "fraud"})        # will ALSO derive it
sess2.fire()
j = [x for x in sess2.justifications() if x["fields"]["account"] == "acme"]
print("derived Risk(acme) why():")
print(json.dumps(sess2.why(j[0]["fact"]), indent=2, default=str) if j else "  (no derived view)")
r = sess2.delete(hsig); r = sess2.fire()   # withdraw the logical support
print("after withdrawing logical support -> Risk still present (stated survives)?:",
      "acme" in risks(sess2.fire()))
