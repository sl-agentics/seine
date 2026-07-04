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
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    I64(i64),
    F64(f64),
    Str(String),
    Bool(bool),
}

impl Value {
    pub fn type_of(&self) -> FieldType {
        match self {
            Value::I64(_) => FieldType::I64,
            Value::F64(_) => FieldType::F64,
            Value::Str(_) => FieldType::Str,
            Value::Bool(_) => FieldType::Bool,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeSchema {
    pub name: String,
    pub fields: Vec<(String, FieldType)>,
}

/// One column of packed values for a single field of a single type.
enum Column {
    I64(Vec<i64>),
    F64(Vec<f64>),
    Str(Vec<String>),
    Bool(Vec<bool>),
}

impl Column {
    fn new(ft: FieldType) -> Column {
        match ft {
            FieldType::I64 => Column::I64(Vec::new()),
            FieldType::F64 => Column::F64(Vec::new()),
            FieldType::Str => Column::Str(Vec::new()),
            FieldType::Bool => Column::Bool(Vec::new()),
        }
    }

    fn push(&mut self, v: Value) -> Result<(), String> {
        match (self, v) {
            (Column::I64(c), Value::I64(x)) => c.push(x),
            (Column::F64(c), Value::F64(x)) => c.push(x),
            (Column::Str(c), Value::Str(x)) => c.push(x),
            (Column::Bool(c), Value::Bool(x)) => c.push(x),
            (_, v) => return Err(format!("column type mismatch for value {v:?}")),
        }
        Ok(())
    }

    fn set(&mut self, row: usize, v: Value) -> Result<(), String> {
        match (self, v) {
            (Column::I64(c), Value::I64(x)) => c[row] = x,
            (Column::F64(c), Value::F64(x)) => c[row] = x,
            (Column::Str(c), Value::Str(x)) => c[row] = x,
            (Column::Bool(c), Value::Bool(x)) => c[row] = x,
            (_, v) => return Err(format!("column type mismatch for value {v:?}")),
        }
        Ok(())
    }

    fn get(&self, row: usize) -> Value {
        match self {
            Column::I64(c) => Value::I64(c[row]),
            Column::F64(c) => Value::F64(c[row]),
            Column::Str(c) => Value::Str(c[row].clone()),
            Column::Bool(c) => Value::Bool(c[row]),
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
}

impl FactStore {
    pub fn new(schemas: Vec<TypeSchema>) -> FactStore {
        let data = schemas
            .iter()
            .map(|s| TypeData {
                columns: s.fields.iter().map(|(_, ft)| Column::new(*ft)).collect(),
                rows: 0,
            })
            .collect();
        FactStore { schemas, data, handles: Vec::new() }
    }

    pub fn schemas(&self) -> &[TypeSchema] {
        &self.schemas
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
        }
    }
}
