//! Runs a scenario JSON file through seine-engine and produces the canonical
//! result JSON (same schema the Java oracle emits).

use seine_engine::{Engine, FactView, FieldType, TypeSchema, Value};
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
        engine.insert(type_name, fields).map_err(|e| e.to_string())?;
    }

    let firings = engine.fire_all(FIRE_LIMIT).map_err(|e| e.to_string())?;
    Ok(json!({
        "facts": engine.facts().iter().map(fact_view_to_json).collect::<Vec<J>>(),
        "firings": firings
            .iter()
            .map(|f| json!({
                "rule": f.rule,
                "matches": f.matches.iter().map(fact_view_to_json).collect::<Vec<J>>(),
            }))
            .collect::<Vec<J>>(),
    }))
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
                other => return Err(format!("unknown field type {other:?}")),
            };
            fields.push((fname, ftype));
        }
        out.push(TypeSchema { name, fields });
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
            other => return Err(format!("field {k}: unsupported JSON value {other}")),
        };
        out.push((k.clone(), val));
    }
    Ok(out)
}

fn fact_view_to_json(fv: &FactView) -> J {
    let mut fields = Map::new();
    for (name, v) in &fv.fields {
        let jv = match v {
            Value::I64(n) => json!(n),
            Value::F64(n) => json!(n),
            Value::Str(s) => json!(s),
            Value::Bool(b) => json!(b),
        };
        fields.insert(name.clone(), jv);
    }
    json!({"type": fv.type_name, "fields": fields})
}
