//! Runs a scenario JSON file through seine-engine and produces the canonical
//! result JSON (same schema the Java oracle emits).

use seine_engine::{Engine, FactView, FieldType, QueryVal, TypeSchema, Value};
use serde_json::{json, Map, Value as J};



pub fn run_scenario_file(path: &str) -> Result<(String, J), (String, String)> {
    run_scenario_file_parts(path)
        .map(|(name, parts)| { let v = parts.to_value(); (name, v) })
}

pub fn run_scenario_file_parts(path: &str) -> Result<(String, RunParts), (String, String)> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| (path.to_string(), format!("cannot read {path}: {e}")))?;
    // D-270 (the memory diet, slab 1): parse ONCE into owned typed
    // structs — no serde_json Value tree retained across the run. The
    // tree's per-object BTreeMap nodes were 72% of peak live bytes at
    // 1M facts (the D-269 workload); `facts` is the only unbounded
    // surface, so fact rows get a compact shape and everything small
    // stays a Value. The raw text drops here too.
    let sc: Scenario = serde_json::from_str(&text)
        .map_err(|e| (path.to_string(), format!("bad scenario JSON: {e}")))?;
    drop(text);
    let name = sc.name.clone().unwrap_or_else(|| path.to_string());
    run_scenario(sc).map(|r| (name.clone(), r)).map_err(|e| (name, e))
}

/// The scenario document, typed. Every field the runner reads is here;
/// unknown keys are skipped (the old `.get()` walk ignored them too).
/// Missing-key errors stay at the USE sites so their strings and
/// relative order match the tree-walking code exactly.
#[derive(Default)]
struct Scenario {
    name: Option<String>,
    drl: Option<String>,
    types: Option<J>,
    facts: Option<Vec<FactSpec>>,
    epochs: Option<Vec<Epoch>>,
    queries: Option<Vec<J>>,
    /// D-332: per-scenario fire-limit override. The pr_rw_ error-parity
    /// cells relay FOREVER (no stable model) — the parity claim is "both
    /// engines diverge to the SAME wall", not "100k specifically"; a low
    /// wall buys back ~2.5s/cell engine-side and the oracle's grind. The
    /// oracle runner reads the same field; both sides format the same
    /// number into the error, so the D-013/j21 substring parity holds.
    fire_limit: Option<usize>,
}

#[derive(Default)]
struct FactSpec {
    type_name: Option<String>,
    fields: Option<Fields>,
    entry_point: Option<String>,
}

#[derive(Default)]
struct Epoch {
    actions: Option<Vec<J>>,
    facts: Option<Vec<FactSpec>>,
    queries: Option<Vec<J>>,
}

/// A fact's fields, flattened to a sorted key/value Vec. Built through
/// a TRANSIENT BTreeMap so duplicate-key (last wins) and iteration
/// (ascending key) semantics are the old tree's by construction; only
/// the flat Vec survives — the ~632-byte tree node does not.
struct Fields(Vec<(String, J)>);

impl<'de> serde::Deserialize<'de> for Fields {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = Fields;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a fields object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut m: A) -> Result<Fields, A::Error> {
                let mut map = std::collections::BTreeMap::<String, J>::new();
                while let Some((k, v)) = m.next_entry()? {
                    map.insert(k, v);
                }
                Ok(Fields(map.into_iter().collect()))
            }
        }
        d.deserialize_map(V)
    }
}

/// Hand-rolled object visitors (the ser.rs D-267 style — no derive
/// dep): match known keys, last duplicate wins (BTreeMap semantics),
/// skip unknown values without building them.
macro_rules! de_object {
    ($ty:ident { $($json_key:literal => $field:ident),+ $(,)? }) => {
        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = $ty;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " object"))
                    }
                    fn visit_map<A: serde::de::MapAccess<'de>>(
                        self,
                        mut m: A,
                    ) -> Result<$ty, A::Error> {
                        let mut out = $ty::default();
                        while let Some(k) = m.next_key::<String>()? {
                            match k.as_str() {
                                $($json_key => out.$field = Some(m.next_value()?),)+
                                _ => {
                                    m.next_value::<serde::de::IgnoredAny>()?;
                                }
                            }
                        }
                        Ok(out)
                    }
                }
                d.deserialize_map(V)
            }
        }
    };
}

de_object!(Scenario {
    "name" => name,
    "drl" => drl,
    "types" => types,
    "facts" => facts,
    "epochs" => epochs,
    "queries" => queries,
    "fire_limit" => fire_limit,
});
de_object!(FactSpec {
    "type" => type_name,
    "fields" => fields,
    "entry_point" => entry_point,
});
de_object!(Epoch {
    "actions" => actions,
    "facts" => facts,
    "queries" => queries,
});

/// The engine-shaped result pieces, before any JSON assembly. One
/// producer, two consumers: cmd_run serializes DIRECTLY (D-267, no
/// Value tree); diff/fuzz build the comparison Value via to_value().
/// D-272: the final WM dump is not materialized here — the ENGINE
/// rides along and consumers pull `facts_iter()` (cmd_run streams it;
/// to_value collects it), so 2M-fact dumps never coexist with their
/// serialized bytes.
pub struct RunParts {
    pub engine: Engine,
    pub firings: Vec<seine_engine::Firing>,
    pub queries: Vec<J>,
}

impl RunParts {
    /// The pre-D-267 Value assembly, byte/structure-identical — the
    /// judge's comparison shape.
    pub fn to_value(&self) -> J {
        json!({
            "facts": self.engine.facts_iter().map(|fv| fact_view_to_json(&fv)).collect::<Vec<J>>(),
            "firings": self.firings
                .iter()
                .map(|f| json!({
                    "rule": f.rule,
                    "matches": f.matches.iter().map(fact_view_to_json).collect::<Vec<J>>(),
                }))
                .collect::<Vec<J>>(),
            "queries": self.queries,
        })
    }
}

// D-271: takes the scenario BY VALUE so fact rows can be dropped the
// moment they are inserted — top-level facts after the initial insert
// loop, each epoch's facts as its epoch completes. At 1M facts/side
// the retained FactSpecs were ~250MB of live peak for data the engine
// already owns.
fn run_scenario(mut sc: Scenario) -> Result<RunParts, String> {
    let types = sc.types.as_ref().ok_or("scenario missing 'types'")?;
    let schemas = parse_types(types)?;
    let mut engine = Engine::new(schemas).map_err(|e| e.to_string())?;
    // CEP E1/E2: type-level event metadata. `expires_ms` is OPTIONAL
    // (D-109) — absent ⇒ infer the expiration reach from the temporal
    // constraints after rule compile (CEP E2 item A).
    for t in types.as_array().into_iter().flatten() {
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
    // D-332: env override (diagnostic) > scenario field > the certified
    // default. Captured before `sc` is consumed.
    let limit = std::env::var("SEINE_FIRE_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .or(sc.fire_limit)
        .unwrap_or(100_000);
    let drl = sc.drl.as_deref().ok_or("scenario missing 'drl'")?;
    engine.add_rules_drl(drl).map_err(|e| e.to_string())?;

    let facts = sc.facts.take().ok_or("scenario missing 'facts'")?;
    for fact in &facts {
        insert_fact_spec(&mut engine, fact, "fact")?;
    }
    drop(facts);

    let mut firings = engine.fire_all(limit).map_err(|e| e.to_string())?;
    // Multi-fire epochs (D-046) + external WM actions (D-047): each
    // epoch runs its ordered actions (insert / update / delete-by-
    // global-insertion-index), then legacy "facts" inserts, then fires
    // again; the firing log continues.
    let mut queries_out = Vec::new();
    if let Some(epochs) = sc.epochs.take() {
        // By-value iteration: each epoch's rows drop as their epoch
        // completes, not at end of run.
        for epoch in epochs {
            for action in epoch.actions.as_deref().unwrap_or_default() {
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
            for fact in epoch.facts.as_deref().unwrap_or_default() {
                insert_fact_spec(&mut engine, fact, "epoch fact")?;
            }
            firings.extend(engine.fire_all(limit).map_err(|e| e.to_string())?);
            // Arc 5 (D-107): per-epoch query invocation — queries run
            // against the WM as of THIS epoch's quiescence
            if let Some(eq) = &epoch.queries {
                run_query_calls(&mut engine, eq, &mut queries_out)?;
            }
        }
    }
    // Query phase (D-049): ordered calls against the final WM. JSON null
    // arg = unbound (Variable.v on the oracle side).

    if let Some(queries) = &sc.queries {
        run_query_calls(&mut engine, queries, &mut queries_out)?;
    }

    Ok(RunParts { engine, firings, queries: queries_out })
}

/// Insert one typed fact row. `what` = "fact" | "epoch fact" so the
/// missing-key error strings match the old tree walk byte for byte.
fn insert_fact_spec(engine: &mut Engine, fact: &FactSpec, what: &str) -> Result<(), String> {
    let type_name = fact.type_name.as_deref().ok_or(format!("{what} missing 'type'"))?;
    let flat = fact.fields.as_ref().ok_or(format!("{what} missing 'fields'"))?;
    let fields = json_fields_to_values(flat.0.iter().map(|(k, v)| (k, v)))?;
    let ep = fact.entry_point.as_deref();
    engine.insert_into(type_name, fields, ep).map_err(|e| e.to_string())?;
    Ok(())
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
        QueryVal::Scalar(Value::F64(n)) => {
            json!({"type": "Double", "fields": {"value": f64_to_json(*n)}})
        }
        QueryVal::Scalar(Value::Str(s)) => json!({"type": "String", "fields": {"value": s}}),
        QueryVal::Scalar(Value::Bool(b)) => json!({"type": "Boolean", "fields": {"value": b}}),
        // unreachable: nullable types are walled from queries (D-097)
        QueryVal::Scalar(Value::Null) => J::Null,
        // walled from queries (D-098) but reachable as an accumulate-result
        // match element; the oracle boxes it as its Java class
        QueryVal::Scalar(Value::Dec { u, s }) => {
            json!({"type": "BigDecimal", "fields": {"value": seine_engine::dec_render(*u, *s)}})
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

fn json_fields_to_values<'a>(
    obj: impl IntoIterator<Item = (&'a String, &'a J)>,
) -> Result<Vec<(String, Value)>, String> {
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

/// Non-finite doubles render as Java's Double.toString strings —
/// "Infinity"/"-Infinity"/"NaN" — matching the oracle's Jackson output
/// (D-283; computed RHS args make non-finite REACHABLE: 1.0/0.0). The
/// old json!(NaN) -> null path was a latent divergence no corpus
/// scenario could reach (JSON has no non-finite literals).
pub(crate) fn f64_to_json(n: f64) -> J {
    if n.is_finite() {
        json!(n)
    } else if n.is_nan() {
        json!("NaN")
    } else if n > 0.0 {
        json!("Infinity")
    } else {
        json!("-Infinity")
    }
}

pub(crate) fn fact_view_to_json(fv: &FactView) -> J {
    let mut fields = Map::new();
    // u32::MAX marks synthetic views (QueryArgs arrays, boxed scalars) —
    // the oracle emits no __h for those either (D-056).
    if std::env::var("SEINE_HANDLES").is_ok() && fv.handle != u32::MAX {
        fields.insert("__h".into(), json!(fv.handle));
    }
    for (name, v) in &fv.fields {
        let jv = match v {
            Value::I64(n) => json!(n),
            Value::F64(n) => f64_to_json(*n),
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
