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

    if msgs.is_empty() {
        Ok(())
    } else {
        Err(msgs)
    }
}

struct CanonResult {
    facts: Vec<String>,
    firings: Vec<(String, Vec<String>)>,
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
    Ok(CanonResult { facts, firings })
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
}
