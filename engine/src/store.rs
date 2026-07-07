//! Working-memory fact store.
//!
//! Layout constraint (non-negotiable, see brief §2 Phase 0): facts live as
//! packed values in per-type, per-field columnar arenas. All references
//! between structures are integer handles (`FactId`, `TypeId`), never
//! pointers. This keeps mmap-backed arenas, hot/cold tiering, and key-sharded
//! partitioning reachable later without a rewrite.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TypeId(pub u32);

/// Global fact handle. Handles are allocated sequentially in insertion order
/// and are never reused; recency/ordering comparisons on handles are
/// meaningful (Drools fact handle ids behave the same way).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct FactId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldType {
    I64,
    F64,
    Str,
    Bool,
    /// Exact decimal, Arrow Decimal128-compatible (D-095/D-098):
    /// unscaled i128 at the declared scale; 1 <= p <= 38, 0 <= s <= p.
    Dec { p: u8, s: u8 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    I64(i64),
    F64(f64),
    Str(String),
    Bool(bool),
    /// SQL NULL (D-095/D-097): unknown value. Only representable in
    /// fields declared nullable; comparisons involving it are UNKNOWN
    /// (3VL) except the IS [NOT] NULL surface tests. PartialEq derives
    /// Null == Null (identity/staging semantics); JOIN-KEY matching
    /// must NOT use that — bucket lookups skip null keys (pin F).
    Null,
    /// Exact decimal (D-098): self-carrying unscaled value + scale.
    /// Comparisons are VALUE-based across scales (pin J) via dec_cmp;
    /// PartialEq derives representation equality — value-sensitive
    /// paths (keys, TMS) go through dec_cmp/normalization instead.
    Dec { u: i128, s: u8 },
}

impl Value {
    pub fn type_of(&self) -> FieldType {
        match self {
            Value::I64(_) => FieldType::I64,
            Value::F64(_) => FieldType::F64,
            Value::Str(_) => FieldType::Str,
            Value::Bool(_) => FieldType::Bool,
            // Null is typeless; callers gate on is_null() first. Any
            // type-directed dispatch reaching a Null is a bug upstream
            // (nullable-walled surface) — I64 is a harmless sentinel
            // for the walled paths (queries reject nullable types).
            Value::Null => FieldType::I64,
            // p is not recoverable from a value; 38 is the storage max.
            Value::Dec { s, .. } => FieldType::Dec { p: 38, s: *s },
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

/// Exact cross-scale decimal comparison (pin J: value equality is
/// scale-independent). Aligns the smaller-scale side up with checked
/// multiplication; on overflow the widened side strictly exceeds any
/// i128 the other side holds, so its SIGN decides — exact and total
/// without 256-bit arithmetic.
pub fn dec_cmp(au: i128, as_: u8, bu: i128, bs: u8) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if as_ == bs {
        return au.cmp(&bu);
    }
    let (lo_u, lo_s, hi_u, hi_s, flip) =
        if as_ < bs { (au, as_, bu, bs, false) } else { (bu, bs, au, as_, true) };
    let pow = 10i128.checked_pow((hi_s - lo_s) as u32);
    let scaled = pow.and_then(|p| lo_u.checked_mul(p));
    let ord = match scaled {
        Some(v) => v.cmp(&hi_u),
        // |lo_u * 10^d| > i128::MAX >= |hi_u| -> sign of lo_u decides
        None => {
            if lo_u > 0 {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    };
    if flip { ord.reverse() } else { ord }
}

/// Canonical (unscaled, scale): trailing decimal zeros stripped — the
/// value-identity form for TMS keys and hashes (1.10 == 1.1, pin H/J).
pub fn dec_normalize(mut u: i128, mut s: u8) -> (i128, u8) {
    while s > 0 && u % 10 == 0 {
        u /= 10;
        s -= 1;
    }
    (u, s)
}

/// Render at the value's own scale: "1.25", "-3.50", "7".
pub fn dec_render(u: i128, s: u8) -> String {
    if s == 0 {
        return u.to_string();
    }
    let neg = u < 0;
    let abs = u.unsigned_abs().to_string();
    let s = s as usize;
    let (int, frac) = if abs.len() > s {
        (abs[..abs.len() - s].to_string(), abs[abs.len() - s..].to_string())
    } else {
        ("0".to_string(), format!("{:0>width$}", abs, width = s))
    };
    format!("{}{}.{}", if neg { "-" } else { "" }, int, frac)
}

/// Parse an exact decimal string ("1.25", "-3.5", "7") into
/// (unscaled, scale). No exponents, no floats — exactness only.
pub fn dec_parse(txt: &str) -> Option<(i128, u8)> {
    let t = txt.trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (int_part, frac_part) = match t.split_once('.') {
        Some((a, b)) => (a, b),
        None => (t, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
        || frac_part.len() > 38
    {
        return None;
    }
    let digits = format!("{int_part}{frac_part}");
    let mut u: i128 = 0;
    for c in digits.chars() {
        u = u.checked_mul(10)?.checked_add((c as u8 - b'0') as i128)?;
    }
    Some((if neg { -u } else { u }, frac_part.len() as u8))
}

/// Rescale to a target scale: exact when widening; HALF-UP (away from
/// zero) when narrowing (pin J); None on i128 overflow.
pub fn dec_rescale(u: i128, s: u8, target: u8) -> Option<(i128, u8)> {
    use std::cmp::Ordering::*;
    match s.cmp(&target) {
        Equal => Some((u, target)),
        Less => {
            let p = 10i128.checked_pow((target - s) as u32)?;
            Some((u.checked_mul(p)?, target))
        }
        Greater => {
            let p = 10i128.checked_pow((s - target) as u32)?;
            let q = u / p;
            let r = u % p;
            let half = p / 2;
            let adj = if r.abs() >= half { u.signum() } else { 0 };
            Some((q + adj, target))
        }
    }
}

/// Enforce a declared precision: |u| must fit in p digits.
pub fn dec_fits(u: i128, p: u8) -> bool {
    let max = 10i128.checked_pow(p as u32).map(|x| x - 1).unwrap_or(i128::MAX);
    u.abs() <= max
}

#[derive(Clone, Debug)]
pub struct TypeSchema {
    pub name: String,
    pub fields: Vec<(String, FieldType)>,
    /// Bit i set = field i is NULLABLE (opt-in, D-097): its column
    /// carries a validity bitmap and accepts Value::Null. Default 0
    /// keeps every certified scenario byte-identical.
    pub nullable: u64,
}

/// One column of packed values for a single field of a single type.
/// `valid` exists only for NULLABLE fields (Arrow validity model,
/// D-097): false rows hold a default in the packed vec and read back
/// as Value::Null.
struct Column {
    data: ColData,
    valid: Option<Vec<bool>>,
}

enum ColData {
    I64(Vec<i64>),
    F64(Vec<f64>),
    Str(Vec<String>),
    Bool(Vec<bool>),
    /// Per-row (unscaled, scale): user fields arrive pre-normalized to
    /// the field's declared scale (Engine::coerce); accumulate result
    /// rows store their exact computed scale (D-098).
    Dec(Vec<(i128, u8)>),
}

impl Column {
    fn new(ft: FieldType, nullable: bool) -> Column {
        let data = match ft {
            FieldType::I64 => ColData::I64(Vec::new()),
            FieldType::F64 => ColData::F64(Vec::new()),
            FieldType::Str => ColData::Str(Vec::new()),
            FieldType::Bool => ColData::Bool(Vec::new()),
            FieldType::Dec { .. } => ColData::Dec(Vec::new()),
        };
        Column { data, valid: if nullable { Some(Vec::new()) } else { None } }
    }

    fn push(&mut self, v: Value) -> Result<(), String> {
        if let Value::Null = v {
            let Some(valid) = &mut self.valid else {
                return Err("null value for a non-nullable field".into());
            };
            valid.push(false);
            match &mut self.data {
                ColData::I64(c) => c.push(0),
                ColData::F64(c) => c.push(0.0),
                ColData::Str(c) => c.push(String::new()),
                ColData::Bool(c) => c.push(false),
                ColData::Dec(c) => c.push((0, 0)),
            }
            return Ok(());
        }
        match (&mut self.data, v) {
            (ColData::I64(c), Value::I64(x)) => c.push(x),
            (ColData::F64(c), Value::F64(x)) => c.push(x),
            (ColData::Str(c), Value::Str(x)) => c.push(x),
            (ColData::Bool(c), Value::Bool(x)) => c.push(x),
            (ColData::Dec(c), Value::Dec { u, s }) => c.push((u, s)),
            (_, v) => return Err(format!("column type mismatch for value {v:?}")),
        }
        if let Some(valid) = &mut self.valid {
            valid.push(true);
        }
        Ok(())
    }

    fn set(&mut self, row: usize, v: Value) -> Result<(), String> {
        if let Value::Null = v {
            let Some(valid) = &mut self.valid else {
                return Err("null value for a non-nullable field".into());
            };
            valid[row] = false;
            return Ok(());
        }
        match (&mut self.data, v) {
            (ColData::I64(c), Value::I64(x)) => c[row] = x,
            (ColData::F64(c), Value::F64(x)) => c[row] = x,
            (ColData::Str(c), Value::Str(x)) => c[row] = x,
            (ColData::Bool(c), Value::Bool(x)) => c[row] = x,
            (ColData::Dec(c), Value::Dec { u, s }) => c[row] = (u, s),
            (_, v) => return Err(format!("column type mismatch for value {v:?}")),
        }
        if let Some(valid) = &mut self.valid {
            valid[row] = true;
        }
        Ok(())
    }

    fn get(&self, row: usize) -> Value {
        if let Some(valid) = &self.valid {
            if !valid[row] {
                return Value::Null;
            }
        }
        match &self.data {
            ColData::I64(c) => Value::I64(c[row]),
            ColData::F64(c) => Value::F64(c[row]),
            ColData::Str(c) => Value::Str(c[row].clone()),
            ColData::Bool(c) => Value::Bool(c[row]),
            ColData::Dec(c) => {
                let (u, s) = c[row];
                Value::Dec { u, s }
            }
        }
    }
}

struct TypeData {
    columns: Vec<Column>,
    rows: u32,
}

#[derive(Clone, Copy)]
struct HandleEntry {
    type_id: u32,
    row: u32,
    alive: bool,
}

pub struct FactStore {
    schemas: Vec<TypeSchema>,
    data: Vec<TypeData>,
    handles: Vec<HandleEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactView {
    pub type_name: String,
    pub fields: Vec<(String, Value)>,
    /// The fact's global handle (insertion sequence) — diagnostic only.
    pub handle: u32,
    /// Collect results (D-038) and ?query-CE args arrays (D-056) render
    /// as an ORDER-significant element array; None entries are JSON null
    /// (bound arg positions). None for ordinary facts.
    pub elems: Option<Vec<Option<FactView>>>,
}

impl FactStore {
    pub fn new(schemas: Vec<TypeSchema>) -> FactStore {
        let data = schemas
            .iter()
            .map(|s| TypeData {
                columns: s.fields.iter().enumerate().map(|(i, (_, ft))| Column::new(*ft, s.nullable >> i & 1 == 1)).collect(),
                rows: 0,
            })
            .collect();
        FactStore { schemas, data, handles: Vec::new() }
    }

    pub fn schemas(&self) -> &[TypeSchema] {
        &self.schemas
    }

    /// Register a hidden type after construction (?query-CE row types,
    /// D-056). Existing TypeIds are unaffected.
    pub fn add_schema(&mut self, schema: TypeSchema) -> TypeId {
        self.data.push(TypeData {
            columns: schema.fields.iter().enumerate().map(|(i, (_, ft))| Column::new(*ft, schema.nullable >> i & 1 == 1)).collect(),
            rows: 0,
        });
        self.schemas.push(schema);
        TypeId((self.schemas.len() - 1) as u32)
    }

    pub fn type_id(&self, name: &str) -> Option<TypeId> {
        self.schemas
            .iter()
            .position(|s| s.name == name)
            .map(|i| TypeId(i as u32))
    }

    pub fn schema(&self, tid: TypeId) -> &TypeSchema {
        &self.schemas[tid.0 as usize]
    }

    pub fn field_index(&self, tid: TypeId, field: &str) -> Option<usize> {
        self.schemas[tid.0 as usize]
            .fields
            .iter()
            .position(|(n, _)| n == field)
    }

    pub fn field_type(&self, tid: TypeId, field_idx: usize) -> FieldType {
        self.schemas[tid.0 as usize].fields[field_idx].1
    }

    /// Insert a fact; `values` must be in schema field order.
    pub fn insert(&mut self, tid: TypeId, values: Vec<Value>) -> Result<FactId, String> {
        let td = &mut self.data[tid.0 as usize];
        let schema = &self.schemas[tid.0 as usize];
        if values.len() != schema.fields.len() {
            return Err(format!(
                "type {} expects {} fields, got {}",
                schema.name,
                schema.fields.len(),
                values.len()
            ));
        }
        for (col, v) in td.columns.iter_mut().zip(values) {
            col.push(v)?;
        }
        let row = td.rows;
        td.rows += 1;
        let id = FactId(self.handles.len() as u32);
        self.handles.push(HandleEntry { type_id: tid.0, row, alive: true });
        Ok(id)
    }

    pub fn fact_type(&self, id: FactId) -> TypeId {
        TypeId(self.handles[id.0 as usize].type_id)
    }

    pub fn is_alive(&self, id: FactId) -> bool {
        self.handles[id.0 as usize].alive
    }

    /// In-place field mutation (RHS setter). Values of retracted facts stay
    /// readable in the arena, matching Drools where a Java object outlives
    /// its retraction for rendering purposes.
    pub fn set_value(&mut self, id: FactId, field_idx: usize, v: Value) -> Result<(), String> {
        let h = self.handles[id.0 as usize];
        self.data[h.type_id as usize].columns[field_idx].set(h.row as usize, v)
    }

    /// Retract: mark dead. Idempotent; the row's values remain readable.
    pub fn kill(&mut self, id: FactId) {
        self.handles[id.0 as usize].alive = false;
    }

    pub fn value(&self, id: FactId, field_idx: usize) -> Value {
        let h = self.handles[id.0 as usize];
        self.data[h.type_id as usize].columns[field_idx].get(h.row as usize)
    }

    /// EVERY fact ever inserted, live or dead, in handle order (D-047:
    /// external-action targets index the visible insertion sequence).
    pub fn all_facts_in_insertion_order(&self) -> impl Iterator<Item = FactId> + '_ {
        (0..self.handles.len()).map(|i| FactId(i as u32))
    }

    /// All live facts in handle (insertion) order.
    pub fn live_facts(&self) -> impl Iterator<Item = FactId> + '_ {
        self.handles
            .iter()
            .enumerate()
            .filter(|(_, h)| h.alive)
            .map(|(i, _)| FactId(i as u32))
    }

    /// All live facts of one type in handle (insertion) order.
    pub fn live_facts_of(&self, tid: TypeId) -> impl Iterator<Item = FactId> + '_ {
        self.handles
            .iter()
            .enumerate()
            .filter(move |(_, h)| h.alive && h.type_id == tid.0)
            .map(|(i, _)| FactId(i as u32))
    }

    pub fn render(&self, id: FactId) -> FactView {
        let h = self.handles[id.0 as usize];
        let schema = &self.schemas[h.type_id as usize];
        let td = &self.data[h.type_id as usize];
        FactView {
            handle: id.0,
            type_name: schema.name.clone(),
            fields: schema
                .fields
                .iter()
                .enumerate()
                .map(|(i, (n, _))| (n.clone(), td.columns[i].get(h.row as usize)))
                .collect(),
            elems: None,
        }
    }
}
