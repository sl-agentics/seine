//! D-267: direct serialization of run results — the same bytes the old
//! `json!` Value-tree assembly produced, without building the tree. The
//! old path used serde_json's default (BTreeMap) maps, so every object's
//! keys came out ALPHABETICALLY sorted with last-insert-wins on duplicate
//! keys; the impls below reproduce that ordering explicitly. All leaf
//! formatting (string escaping, i64, ryu f64, NaN/inf -> null) goes
//! through serde_json's own Serializer, so it is identical by
//! construction. Verified by the all-scenarios byte gate (D-266
//! protocol).

use crate::runner::RunParts;
use seine_engine::{FactView, Firing, Value};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::Value as J;

/// One NDJSON line: {"result": ..., "scenario": name}
pub struct LineOk<'a> {
    pub name: &'a str,
    pub parts: &'a RunParts,
}

impl Serialize for LineOk<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("result", &ResultOut { parts: self.parts })?;
        m.serialize_entry("scenario", self.name)?;
        m.end()
    }
}

struct ResultOut<'a> {
    parts: &'a RunParts,
}

impl Serialize for ResultOut<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(3))?;
        m.serialize_entry("facts", &FvSeq(&self.parts.facts))?;
        m.serialize_entry("firings", &FiringSeq(&self.parts.firings))?;
        m.serialize_entry("queries", &self.parts.queries)?;
        m.end()
    }
}

struct FvSeq<'a>(&'a [FactView]);

impl Serialize for FvSeq<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut q = s.serialize_seq(Some(self.0.len()))?;
        for fv in self.0 {
            q.serialize_element(&FvJson(fv))?;
        }
        q.end()
    }
}

struct FiringSeq<'a>(&'a [Firing]);

impl Serialize for FiringSeq<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut q = s.serialize_seq(Some(self.0.len()))?;
        for f in self.0 {
            q.serialize_element(&FiringJson(f))?;
        }
        q.end()
    }
}

struct FiringJson<'a>(&'a Firing);

impl Serialize for FiringJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("matches", &FvSeq(&self.0.matches))?;
        m.serialize_entry("rule", &self.0.rule)?;
        m.end()
    }
}

fn handles_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SEINE_HANDLES").is_ok())
}

struct FvJson<'a>(&'a FactView);

impl Serialize for FvJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("fields", &FieldsJson(self.0))?;
        m.serialize_entry("type", &self.0.type_name)?;
        m.end()
    }
}

/// A leaf value with json!'s exact semantics (non-finite f64 -> null).
struct Leaf<'a>(&'a Value);

impl Serialize for Leaf<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::I64(n) => s.serialize_i64(*n),
            Value::F64(n) => {
                if n.is_finite() {
                    s.serialize_f64(*n)
                } else {
                    s.serialize_unit() // json!(non-finite f64) == null
                }
            }
            Value::Str(v) => s.serialize_str(v),
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Null => s.serialize_unit(),
            Value::Dec { u, s: sc } => s.serialize_str(&seine_engine::dec_render(*u, *sc)),
        }
    }
}

enum FieldVal<'a> {
    Handle(u32),
    Plain(&'a Value),
    Elems(&'a [Option<FactView>]),
}

impl Serialize for FieldVal<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            FieldVal::Handle(h) => s.serialize_u32(*h),
            FieldVal::Plain(v) => Leaf(v).serialize(s),
            FieldVal::Elems(es) => {
                let mut q = s.serialize_seq(Some(es.len()))?;
                for e in es.iter() {
                    match e {
                        Some(fv) => q.serialize_element(&FvJson(fv))?,
                        None => q.serialize_element(&J::Null)?,
                    }
                }
                q.end()
            }
        }
    }
}

struct FieldsJson<'a>(&'a FactView);

impl Serialize for FieldsJson<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Reproduce the BTreeMap: gather insertion-ordered pairs
        // (__h, then declared fields, then "value"), keep the LAST
        // write per key, emit sorted by key.
        let fv = self.0;
        let mut pairs: Vec<(&str, FieldVal)> = Vec::with_capacity(fv.fields.len() + 2);
        if handles_on() && fv.handle != u32::MAX {
            pairs.push(("__h", FieldVal::Handle(fv.handle)));
        }
        for (name, v) in &fv.fields {
            pairs.push((name.as_str(), FieldVal::Plain(v)));
        }
        if let Some(elems) = &fv.elems {
            pairs.push(("value", FieldVal::Elems(elems)));
        }
        // last-insert-wins dedup (BTreeMap::insert overwrite semantics)
        let mut keep: Vec<usize> = Vec::with_capacity(pairs.len());
        for i in 0..pairs.len() {
            if pairs[i + 1..].iter().all(|(k, _)| *k != pairs[i].0) {
                keep.push(i);
            }
        }
        keep.sort_by(|&a, &b| pairs[a].0.cmp(pairs[b].0));
        let mut m = s.serialize_map(Some(keep.len()))?;
        for &i in &keep {
            m.serialize_entry(pairs[i].0, &pairs[i].1)?;
        }
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::RunParts;

    /// The direct serializer and the old json! Value assembly must emit
    /// identical bytes — including the corners the corpus byte gate may
    /// not exercise under every env: __h handles, elems arrays, a user
    /// field literally named "value" (BTreeMap last-write-wins), NaN
    /// (json! -> null), decimals.
    #[test]
    fn direct_matches_value_tree() {
        let fv_plain = FactView {
            type_name: "T".into(),
            fields: vec![
                ("b".into(), Value::I64(-3)),
                ("a".into(), Value::F64(1.5)),
                ("z".into(), Value::Str("q\"uo\\te\n".into())),
                ("value".into(), Value::Bool(true)),
                ("n".into(), Value::Null),
                ("nan".into(), Value::F64(f64::NAN)),
                ("d".into(), Value::Dec { u: 12345, s: 2 }),
            ],
            handle: 7,
            elems: Some(vec![
                None,
                Some(FactView {
                    type_name: "E".into(),
                    fields: vec![("x".into(), Value::I64(1))],
                    handle: u32::MAX,
                    elems: None,
                }),
            ]),
        };
        let parts = RunParts {
            facts: vec![fv_plain.clone()],
            firings: vec![Firing { rule: "R".into(), matches: vec![fv_plain] }],
            queries: vec![serde_json::json!({"rows": [1, 2]})],
        };
        let old = serde_json::json!({"scenario": "t", "result": parts.to_value()});
        let direct =
            serde_json::to_string(&LineOk { name: "t", parts: &parts }).unwrap();
        assert_eq!(direct, serde_json::to_string(&old).unwrap());
    }
}
