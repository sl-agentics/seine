//! Runs a scenario JSON file through seine-engine and produces the canonical
//! result JSON (same schema the Java oracle emits).

use seine_engine::{Engine, FactView, FieldType, QueryVal, TypeSchema, Value};
use serde_json::{json, Map, Value as J};

const FIRE_LIMIT: usize = 100_000;

pub fn run_scenario_file(path: &str) -> Result<(String, J), (String, String)> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| (path.to_string(), format!("cannot read {path}: {e}")))?;
    let sc: J = serde_json::from_str(&text)
        .map_err(|e| (path.to_string(), format!("bad scenario JSON: {e}")))?;
    let name = sc
        .get("name")
        .and_then(J::as_str)
        .unwrap_or(path)
        .to_string();
    run_scenario(&sc).map(|r| (name.clone(), r)).map_err(|e| (name, e))
}

fn run_scenario(sc: &J) -> Result<J, String> {
    let schemas = parse_types(sc.get("types").ok_or("scenario missing 'types'")?)?;
    let mut engine = Engine::new(schemas).map_err(|e| e.to_string())?;
    // CEP E1/E2: type-level event metadata. `expires_ms` is OPTIONAL
    // (D-109) — absent ⇒ infer the expiration reach from the temporal
    // constraints after rule compile (CEP E2 item A).
    for t in sc.get("types").and_then(J::as_array).into_iter().flatten() {
        if let Some(ev) = t.get("event") {
            let tname = t.get("name").and_then(J::as_str).unwrap_or_default();
            let ts = ev
                .get("timestamp")
                .and_then(J::as_str)
                .ok_or_else(|| format!("{tname}: event needs a 'timestamp' field name"))?;
            let exp = ev.get("expires_ms").and_then(J::as_i64);
            // CEP E2 item E (D-118): optional `@duration(field)` — the event
            // occupies `[ts, ts+field]`; absent ⇒ point event.
            let dur = ev.get("duration").and_then(J::as_str);
            engine.declare_event(tname, ts, exp, dur).map_err(|e| e.to_string())?;
        }
    }
    let drl = sc
        .get("drl")
        .and_then(J::as_str)
        .ok_or("scenario missing 'drl'")?;
    engine.add_rules_drl(drl).map_err(|e| e.to_string())?;

    for fact in sc
        .get("facts")
        .and_then(J::as_array)
        .ok_or("scenario missing 'facts'")?
    {
        let type_name = fact
            .get("type")
            .and_then(J::as_str)
            .ok_or("fact missing 'type'")?;
        let fields_obj = fact
            .get("fields")
            .and_then(J::as_object)
            .ok_or("fact missing 'fields'")?;
        let fields = json_fields_to_values(fields_obj)?;
        let ep = fact.get("entry_point").and_then(J::as_str);
        engine.insert_into(type_name, fields, ep).map_err(|e| e.to_string())?;
    }

    let mut firings = engine.fire_all(FIRE_LIMIT).map_err(|e| e.to_string())?;
    // Multi-fire epochs (D-046) + external WM actions (D-047): each
    // epoch runs its ordered actions (insert / update / delete-by-
    // global-insertion-index), then legacy "facts" inserts, then fires
    // again; the firing log continues.
    let mut queries_out = Vec::new();
    if let Some(epochs) = sc.get("epochs").and_then(J::as_array) {
        for epoch in epochs {
            for action in epoch.get("actions").and_then(J::as_array).unwrap_or(&Vec::new()) {
                let op = action.get("op").and_then(J::as_str).ok_or("action missing 'op'")?;
                match op {
                    "insert" => {
                        let type_name = action
                            .get("type")
                            .and_then(J::as_str)
                            .ok_or("insert action missing 'type'")?;
                        let fields_obj = action
                            .get("fields")
                            .and_then(J::as_object)
                            .ok_or("insert action missing 'fields'")?;
                        let fields = json_fields_to_values(fields_obj)?;
                        let ep = action.get("entry_point").and_then(J::as_str);
                        engine.insert_into(type_name, fields, ep).map_err(|e| e.to_string())?;
                    }
                    "update" => {
                        let target = action
                            .get("target")
                            .and_then(J::as_u64)
                            .ok_or("update action missing 'target'")?;
                        let fields_obj = action
                            .get("fields")
                            .and_then(J::as_object)
                            .ok_or("update action missing 'fields'")?;
                        let fields = json_fields_to_values(fields_obj)?;
                        let id = engine
                            .nth_inserted(target as usize)
                            .ok_or(format!("update target {target} out of range"))?;
                        engine.update_fact(id, fields).map_err(|e| e.to_string())?;
                    }
                    "advance" => {
                        let ms = action
                            .get("ms")
                            .and_then(J::as_i64)
                            .ok_or("advance action missing 'ms'")?;
                        engine.advance(ms).map_err(|e| e.to_string())?;
                    }
                    "delete" => {
                        let target = action
                            .get("target")
                            .and_then(J::as_u64)
                            .ok_or("delete action missing 'target'")?;
                        let id = engine
                            .nth_inserted(target as usize)
                            .ok_or(format!("delete target {target} out of range"))?;
                        engine.delete_fact(id).map_err(|e| e.to_string())?;
                    }
                    "reset" => {
                        engine.reset().map_err(|e| e.to_string())?;
                    }
                    other => return Err(format!("unknown epoch action op {other:?}")),
                }
            }
            for fact in epoch.get("facts").and_then(J::as_array).unwrap_or(&Vec::new()) {
                let type_name = fact
                    .get("type")
                    .and_then(J::as_str)
                    .ok_or("epoch fact missing 'type'")?;
                let fields_obj = fact
                    .get("fields")
                    .and_then(J::as_object)
                    .ok_or("epoch fact missing 'fields'")?;
                let fields = json_fields_to_values(fields_obj)?;
                let ep = fact.get("entry_point").and_then(J::as_str);
                engine.insert_into(type_name, fields, ep).map_err(|e| e.to_string())?;
            }
            firings.extend(engine.fire_all(FIRE_LIMIT).map_err(|e| e.to_string())?);
            // Arc 5 (D-107): per-epoch query invocation — queries run
            // against the WM as of THIS epoch's quiescence
            if let Some(eq) = epoch.get("queries").and_then(J::as_array) {
                run_query_calls(&mut engine, eq, &mut queries_out)?;
            }
        }
    }
    // Query phase (D-049): ordered calls against the final WM. JSON null
    // arg = unbound (Variable.v on the oracle side).

    if let Some(queries) = sc.get("queries").and_then(J::as_array) {
        run_query_calls(&mut engine, queries, &mut queries_out)?;
    }

    Ok(json!({
        "facts": engine.facts().iter().map(fact_view_to_json).collect::<Vec<J>>(),
        "firings": firings
            .iter()
            .map(|f| json!({
                "rule": f.rule,
                "matches": f.matches.iter().map(fact_view_to_json).collect::<Vec<J>>(),
            }))
            .collect::<Vec<J>>(),
        "queries": queries_out,
    }))
}

fn run_query_calls(
    engine: &mut seine_engine::Engine,
    queries: &[J],
    queries_out: &mut Vec<J>,
) -> Result<(), String> {
    {
        for call in queries {
            let name = call
                .get("call")
                .and_then(J::as_str)
                .ok_or("query call missing 'call'")?;
            let args_json = call.get("args").and_then(J::as_array).cloned().unwrap_or_default();
            let mut args = Vec::with_capacity(args_json.len());
            for a in &args_json {
                args.push(match a {
                    J::Null => None,
                    J::Bool(b) => Some(Value::Bool(*b)),
                    J::String(s) => Some(Value::Str(s.clone())),
                    J::Number(n) => Some(if n.is_f64() {
                        Value::F64(n.as_f64().unwrap())
                    } else {
                        Value::I64(n.as_i64().ok_or("query arg out of i64 range")?)
                    }),
                    other => return Err(format!("unsupported query arg {other}")),
                });
            }
            let out = engine.run_query(name, &args).map_err(|e| e.to_string())?;
            let rows: Vec<J> = out
                .rows
                .iter()
                .map(|row| {
                    let mut o = Map::new();
                    for (ident, v) in out.identifiers.iter().zip(row) {
                        o.insert(ident.clone(), query_val_to_json(v));
                    }
                    J::Object(o)
                })
                .collect();
            queries_out.push(json!({
                "call": name,
                "args": args_json,
                "identifiers": out.identifiers,
                "rows": rows,
            }));
        }
    }
    Ok(())
}

/// Query row values render like the oracle's: facts as full renderings,
/// scalars boxed as {"type": Long/Double/String/Boolean, "fields": {"value"}}.
fn query_val_to_json(v: &QueryVal) -> J {
    match v {
        QueryVal::Fact(fv) => fact_view_to_json(fv),
        // identifier unbound in this row's or-branch (D-054)
        QueryVal::Null => J::Null,
        QueryVal::Scalar(Value::I64(n)) => json!({"type": "Long", "fields": {"value": n}}),
        QueryVal::Scalar(Value::F64(n)) => json!({"type": "Double", "fields": {"value": n}}),
        QueryVal::Scalar(Value::Str(s)) => json!({"type": "String", "fields": {"value": s}}),
        QueryVal::Scalar(Value::Bool(b)) => json!({"type": "Boolean", "fields": {"value": b}}),
        // unreachable: nullable types are walled from queries (D-097)
        QueryVal::Scalar(Value::Null) => J::Null,
        // unreachable: decimal types are walled from queries (D-098)
        QueryVal::Scalar(Value::Dec { u, s }) => {
            json!({"type": "Decimal", "fields": {"value": seine_engine::dec_render(*u, *s)}})
        }
    }
}

fn parse_types(types: &J) -> Result<Vec<TypeSchema>, String> {
    let mut out = Vec::new();
    for t in types.as_array().ok_or("'types' must be an array")? {
        let name = t
            .get("name")
            .and_then(J::as_str)
            .ok_or("type missing 'name'")?
            .to_string();
        let mut fields = Vec::new();
        let mut nmask = 0u64;
        for f in t
            .get("fields")
            .and_then(J::as_array)
            .ok_or("type missing 'fields' array")?
        {
            let fname = f
                .get("name")
                .and_then(J::as_str)
                .ok_or("field missing 'name'")?
                .to_string();
            let ftype = match f.get("type").and_then(J::as_str) {
                Some("i64") => FieldType::I64,
                Some("f64") => FieldType::F64,
                Some("String") => FieldType::Str,
                Some("bool") => FieldType::Bool,
                Some(t) if t.starts_with("decimal(") && t.ends_with(')') => {
                    let inner = &t["decimal(".len()..t.len() - 1];
                    let (p, s) = inner
                        .split_once(',')
                        .ok_or_else(|| format!("bad decimal type {t:?}"))?;
                    let p: u8 = p.trim().parse().map_err(|_| format!("bad decimal type {t:?}"))?;
                    let s: u8 = s.trim().parse().map_err(|_| format!("bad decimal type {t:?}"))?;
                    if p == 0 || p > 38 || s > p {
                        return Err(format!("decimal(p,s) needs 1<=p<=38, 0<=s<=p, got {t:?}"));
                    }
                    FieldType::Dec { p, s }
                }
                other => return Err(format!("unknown field type {other:?}")),
            };
            let nullable = f.get("nullable").and_then(J::as_bool).unwrap_or(false);
            if nullable {
                nmask |= 1u64 << fields.len();
            }
            fields.push((fname, ftype));
        }
        out.push(TypeSchema { name, fields, nullable: nmask });
    }
    Ok(out)
}

fn json_fields_to_values(obj: &Map<String, J>) -> Result<Vec<(String, Value)>, String> {
    let mut out = Vec::new();
    for (k, v) in obj {
        let val = match v {
            J::Bool(b) => Value::Bool(*b),
            J::String(s) => Value::Str(s.clone()),
            J::Number(n) => {
                if n.is_f64() {
                    Value::F64(n.as_f64().unwrap())
                } else {
                    Value::I64(
                        n.as_i64()
                            .ok_or_else(|| format!("field {k}: integer out of i64 range"))?,
                    )
                }
            }
            J::Null => Value::Null, // nullable fields only — the store rejects otherwise (D-097)
            other => return Err(format!("field {k}: unsupported JSON value {other}")),
        };
        out.push((k.clone(), val));
    }
    Ok(out)
}

fn fact_view_to_json(fv: &FactView) -> J {
    let mut fields = Map::new();
    // u32::MAX marks synthetic views (QueryArgs arrays, boxed scalars) —
    // the oracle emits no __h for those either (D-056).
    if std::env::var("SEINE_HANDLES").is_ok() && fv.handle != u32::MAX {
        fields.insert("__h".into(), json!(fv.handle));
    }
    for (name, v) in &fv.fields {
        let jv = match v {
            Value::I64(n) => json!(n),
            Value::F64(n) => json!(n),
            Value::Str(s) => json!(s),
            Value::Bool(b) => json!(b),
            Value::Null => J::Null,
            Value::Dec { u, s } => json!(seine_engine::dec_render(*u, *s)),
        };
        fields.insert(name.clone(), jv);
    }
    if let Some(elems) = &fv.elems {
        // collect results / QueryArgs: ORDER-significant element array
        // (D-038/D-056); None elements are JSON null (bound positions)
        fields.insert(
            "value".into(),
            J::Array(
                elems
                    .iter()
                    .map(|e| e.as_ref().map(fact_view_to_json).unwrap_or(J::Null))
                    .collect(),
            ),
        );
    }
    json!({"type": fv.type_name, "fields": fields})
}
