"""Tire 3 — Aggregation provenance: why() through the logical layer,
then acc_sources() through the summation down to line-item leaves."""
import seine_rs
from seine_rs import Rule, fact, sum_
import json

@fact
class LineItem:
    invoice: str
    sku: str
    amount: float

@fact
class InvoiceTotal:       # derived view: total per invoice
    total: float

# group_by invoice, sum amount, insertLogical the total (a derived VIEW,
# kept in sync by truth maintenance).
r = Rule("TotalPerInvoice")
g = r.group_by(LineItem, key=LineItem.invoice, agg=sum_(LineItem.amount))
r.then_insert_logical(InvoiceTotal, total=g)

items = [
    LineItem("INV-1", "widget",  40.0),
    LineItem("INV-1", "gadget",  10.0),
    LineItem("INV-1", "gizmo",    2.5),
    LineItem("INV-2", "widget",  40.0),
    LineItem("INV-2", "sprocket", 7.5),
]
sess = seine_rs.Session([r], {LineItem: items, InvoiceTotal: []})
sess.fire()

print("== line items (handle : invoice/sku/amount) ==")
for h, it in enumerate(items, start=1):   # handles 1..5 (0 = InitialFact)
    print(f"   h{h}: {it.invoice:6} {it.sku:9} {it.amount}")

print("\n== derived InvoiceTotal facts ==")
for j in sess.justifications():
    print("  ", j["type"], "handle", j["fact"], "->", j["fields"])

print("\n== FULL AUDIT CHAIN per derived total ==")
for j in sess.justifications():
    th = j["fact"]
    total = j["fields"]["total"]
    why = sess.why(th)
    print(f"\nInvoiceTotal(total={total})  [fact {th}]")
    for sup in why["supports"]:
        print(f"  justified by rule {sup['rule']!r}, matched tuple {sup['tuple']}, seq {sup['seq']}")
        # walk each handle in the support tuple; the group-result handle
        # answers acc_sources with the contributing line items.
        for src_handle in sup["tuple"]:
            srcs = sess.acc_sources(src_handle)
            if srcs is not None:
                print(f"    acc_sources({src_handle}) = summation leaves:")
                recovered = 0.0
                for leaf_h, contrib in srcs:
                    li = items[leaf_h - 1]
                    recovered += contrib
                    print(f"        line h{leaf_h}: {li.invoice} {li.sku:9} contributed {contrib}")
                print(f"        -> leaves sum to {recovered}  (matches total? {recovered == total})")
