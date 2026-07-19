//! Canonicalization + comparison of result JSONs (DECISIONS.md D-003).
//!
//! Comparison is semantic, not textual: both sides are parsed, scalars are
//! encoded canonically (i64 exact, f64 by IEEE-754 bit pattern, so the two
//! runners' number formatting can differ), facts are compared as a multiset,
//! and the firing log is compared in order with per-firing match multisets.

use serde_json::Value as J;
use std::collections::BTreeMap;

pub fn compare(engine: &J, oracle: &J) -> Result<(), Vec<String>> {
    let e = canon_result(engine).map_err(|e| vec![format!("engine result malformed: {e}")])?;
    let o = canon_result(oracle).map_err(|e| vec![format!("oracle result malformed: {e}")])?;
    let mut msgs = Vec::new();

    // Final facts: multiset diff.
    let mut counts: BTreeMap<&String, i64> = BTreeMap::new();
    for f in &e.facts {
        *counts.entry(f).or_insert(0) += 1;
    }
    for f in &o.facts {
        *counts.entry(f).or_insert(0) -= 1;
    }
    for (fact, n) in counts {
        if n > 0 {
            msgs.push(format!("fact only in engine (x{n}): {fact}"));
        } else if n < 0 {
            msgs.push(format!("fact only in oracle (x{}): {fact}", -n));
        }
    }

    // Firing log: order-significant.
    if e.firings.len() != o.firings.len() {
        msgs.push(format!(
            "firing count differs: engine {} vs oracle {}",
            e.firings.len(),
            o.firings.len()
        ));
    }
    for (i, (ef, of)) in e.firings.iter().zip(o.firings.iter()).enumerate() {
        if ef != of {
            msgs.push(format!(
                "firing[{i}] differs:\n       engine: {}\n       oracle: {}",
                render_firing(ef),
                render_firing(of)
            ));
            break; // first divergence is the informative one
        }
    }

    // Query results (D-049): positional per call; identifiers as a SET
    // (Drools order is a HashMap artifact); rows ORDER-SIGNIFICANT.
    if e.queries.len() != o.queries.len() {
        msgs.push(format!(
            "query call count differs: engine {} vs oracle {}",
            e.queries.len(),
            o.queries.len()
        ));
    }
    for (i, (eq, oq)) in e.queries.iter().zip(o.queries.iter()).enumerate() {
        if eq.call != oq.call || eq.args != oq.args {
            msgs.push(format!(
                "queries[{i}] call/args differ: engine {}{:?} vs oracle {}{:?}",
                eq.call, eq.args, oq.call, oq.args
            ));
            break;
        }
        if eq.identifiers != oq.identifiers {
            msgs.push(format!(
                "queries[{i}] ({}) identifier sets differ:\n       engine: {:?}\n       oracle: {:?}",
                eq.call, eq.identifiers, oq.identifiers
            ));
            break;
        }
        if eq.rows.len() != oq.rows.len() {
            msgs.push(format!(
                "queries[{i}] ({}) row count differs: engine {} vs oracle {}",
                eq.call,
                eq.rows.len(),
                oq.rows.len()
            ));
            break;
        }
        if let Some((ri, (er, or))) =
            eq.rows.iter().zip(oq.rows.iter()).enumerate().find(|(_, (a, b))| a != b)
        {
            msgs.push(format!(
                "queries[{i}] ({}) row[{ri}] differs:\n       engine: {er:?}\n       oracle: {or:?}",
                eq.call
            ));
            break;
        }
    }

    if msgs.is_empty() {
        Ok(())
    } else {
        Err(msgs)
    }
}

struct CanonQuery {
    call: String,
    /// canonical scalars; "null" marks an unbound arg
    args: Vec<String>,
    /// sorted (set semantics)
    identifiers: Vec<String>,
    /// each row: identifier -> canonical rendering, rows in order
    rows: Vec<BTreeMap<String, String>>,
}

struct CanonResult {
    facts: Vec<String>,
    firings: Vec<(String, Vec<String>)>,
    queries: Vec<CanonQuery>,
}

fn render_firing((rule, matches): &(String, Vec<String>)) -> String {
    format!("{rule} {matches:?}")
}

fn canon_result(v: &J) -> Result<CanonResult, String> {
    let facts_json = v
        .get("facts")
        .and_then(J::as_array)
        .ok_or("missing 'facts' array")?;
    let mut facts: Vec<String> = facts_json.iter().map(canon_fact).collect::<Result<_, _>>()?;
    facts.sort();

    let firings_json = v
        .get("firings")
        .and_then(J::as_array)
        .ok_or("missing 'firings' array")?;
    let mut firings = Vec::with_capacity(firings_json.len());
    for f in firings_json {
        let rule = f
            .get("rule")
            .and_then(J::as_str)
            .ok_or("firing missing 'rule'")?
            .to_string();
        let mut matches: Vec<String> = f
            .get("matches")
            .and_then(J::as_array)
            .ok_or("firing missing 'matches'")?
            .iter()
            .map(canon_fact)
            .collect::<Result<_, _>>()?;
        matches.sort();
        firings.push((rule, matches));
    }

    // "queries" is optional: pre-query scenarios/oracles omit it.
    let mut queries = Vec::new();
    for q in v.get("queries").and_then(J::as_array).unwrap_or(&Vec::new()) {
        let call = q
            .get("call")
            .and_then(J::as_str)
            .ok_or("query entry missing 'call'")?
            .to_string();
        let args = q
            .get("args")
            .and_then(J::as_array)
            .map(|a| {
                a.iter()
                    .map(|v| {
                        if v.is_null() {
                            Ok("null".to_string())
                        } else {
                            canon_scalar(v)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let mut identifiers: Vec<String> = q
            .get("identifiers")
            .and_then(J::as_array)
            .ok_or("query entry missing 'identifiers'")?
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or("identifier not a string"))
            .collect::<Result<_, _>>()?;
        identifiers.sort();
        let mut rows = Vec::new();
        for row in q
            .get("rows")
            .and_then(J::as_array)
            .ok_or("query entry missing 'rows'")?
        {
            let obj = row.as_object().ok_or("query row not an object")?;
            let mut m = BTreeMap::new();
            for (k, v) in obj {
                // identifiers local to another or-branch are null (D-054)
                let cv = if v.is_null() { "null".to_string() } else { canon_fact(v)? };
                m.insert(k.clone(), cv);
            }
            rows.push(m);
        }
        queries.push(CanonQuery { call, args, identifiers, rows });
    }
    Ok(CanonResult { facts, firings, queries })
}

fn canon_fact(v: &J) -> Result<String, String> {
    let t = v
        .get("type")
        .and_then(J::as_str)
        .ok_or("fact missing 'type'")?;
    let fields = v
        .get("fields")
        .and_then(J::as_object)
        .ok_or("fact missing 'fields'")?;
    // D-328: SetCollection is UNORDERED (Drools: counted-HashMap keySet,
    // "order not guaranteed"; D-108 pinned canonicalize-SORTED). The two
    // runners sort by DIFFERENT render keys (the oracle by Jackson JSON
    // toString, where Java's scientific Double.toString reorders vs the
    // engine's plain-decimal key — the fz_662607_47 swap) — complete the
    // canonicalization HERE, where both sides share one rendering.
    // Collection (collectList) stays ORDER-SIGNIFICANT (D-323).
    if t == "SetCollection" {
        if let Some(items) = fields.get("value").and_then(J::as_array) {
            let mut parts: Vec<String> =
                items.iter().map(canon_fact).collect::<Result<_, _>>()?;
            parts.sort();
            return Ok(format!("SetCollection(value=[{}])", parts.join(",")));
        }
    }
    let mut parts: Vec<(String, String)> = fields
        .iter()
        .map(|(k, fv)| Ok((k.clone(), canon_scalar(fv)?)))
        .collect::<Result<_, String>>()?;
    parts.sort();
    let body: Vec<String> = parts.into_iter().map(|(k, s)| format!("{k}={s}")).collect();
    Ok(format!("{t}({})", body.join(",")))
}

fn canon_scalar(v: &J) -> Result<String, String> {
    match v {
        J::Bool(b) => Ok(format!("b:{b}")),
        J::String(s) => Ok(format!("s:{s:?}")),
        J::Number(n) => {
            if n.is_f64() {
                Ok(format!("f:{:016x}", n.as_f64().unwrap().to_bits()))
            } else if let Some(i) = n.as_i64() {
                Ok(format!("i:{i}"))
            } else {
                Err(format!("integer out of i64 range: {n}"))
            }
        }
        // collect results (D-038) and ?query-CE args arrays (D-056): an
        // ORDER-significant array of fact renderings; null elements are
        // BOUND arg positions.
        J::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|e| {
                    if e.is_null() {
                        Ok("null".to_string())
                    } else {
                        canon_fact(e)
                    }
                })
                .collect::<Result<_, String>>()?;
            Ok(format!("[{}]", parts.join(",")))
        }
        other => Err(format!("unsupported scalar {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fact_order_is_irrelevant_but_firing_order_is_not() {
        let a = json!({
            "facts": [
                {"type": "P", "fields": {"x": 1}},
                {"type": "P", "fields": {"x": 2}}
            ],
            "firings": [
                {"rule": "r1", "matches": [{"type": "P", "fields": {"x": 1}}]},
                {"rule": "r2", "matches": []}
            ]
        });
        let b = json!({
            "facts": [
                {"type": "P", "fields": {"x": 2}},
                {"type": "P", "fields": {"x": 1}}
            ],
            "firings": [
                {"rule": "r1", "matches": [{"type": "P", "fields": {"x": 1}}]},
                {"rule": "r2", "matches": []}
            ]
        });
        assert!(compare(&a, &b).is_ok());

        let c = json!({
            "facts": a["facts"],
            "firings": [
                {"rule": "r2", "matches": []},
                {"rule": "r1", "matches": [{"type": "P", "fields": {"x": 1}}]}
            ]
        });
        assert!(compare(&a, &c).is_err());
    }

    #[test]
    fn float_int_distinction_and_bit_equality() {
        // 1 (i64) and 1.0 (f64) are DIFFERENT canonical scalars — field types
        // must match, not just numeric values.
        let a = json!({"facts": [{"type": "P", "fields": {"x": 1}}], "firings": []});
        let b = json!({"facts": [{"type": "P", "fields": {"x": 1.0}}], "firings": []});
        assert!(compare(&a, &b).is_err());

        let c = json!({"facts": [{"type": "P", "fields": {"x": 2.5}}], "firings": []});
        let d = json!({"facts": [{"type": "P", "fields": {"x": 2.5}}], "firings": []});
        assert!(compare(&c, &d).is_ok());
    }

    #[test]
    fn set_collection_order_insensitive_list_order_significant() {
        // D-328: the runners sort sets by DIFFERENT keys (the oracle's
        // Java-scientific "-1.000000007E9" sorts before "-1.0"; the
        // engine's plain-decimal key after) — canon completes the D-108
        // canonicalize-SORTED intent, so element ORDER never diverges a
        // SetCollection while content still must match exactly.
        let el = |v: f64| json!({"type": "Double", "fields": {"value": v}});
        let set = |vals: &[f64]| {
            json!({"facts": [], "firings": [{"rule": "R", "matches": [
                {"type": "SetCollection", "fields": {"value": vals.iter().map(|&v| el(v)).collect::<Vec<_>>()}}
            ]}]})
        };
        assert!(compare(&set(&[-1.0, -1000000007.0]), &set(&[-1000000007.0, -1.0])).is_ok());
        assert!(compare(&set(&[-1.0, -1000000007.0]), &set(&[-1.0, -1000000007.5])).is_err());

        let list = |vals: &[f64]| {
            json!({"facts": [], "firings": [{"rule": "R", "matches": [
                {"type": "Collection", "fields": {"value": vals.iter().map(|&v| el(v)).collect::<Vec<_>>()}}
            ]}]})
        };
        assert!(compare(&list(&[1.0, 2.0]), &list(&[2.0, 1.0])).is_err());
    }
}
