//! seine Python bindings — Layer 1 (D-044).
//!
//! Design contract (keeps the differential certification true across
//! the FFI boundary):
//! - Facts cross as Arrow columnar batches (PyCapsule C-stream
//!   interface), never per-fact Python objects; Python holds integer
//!   HANDLES into the Rust arenas.
//! - The binding adds ZERO semantics: type widening (i8/16/32 -> i64,
//!   f32 -> f64) is exact and done in Rust; NULLS ARE REJECTED loudly —
//!   the certified subset has no null semantics.
//! - Sessions are ONE-SHOT (build -> insert -> fire -> read): the
//!   certified envelope is insert-all-then-fire-once. A second fire()
//!   raises.
//! - Callbacks are OBSERVERS only: `on_fire` receives plain data after
//!   the (GIL-free) run completes, in firing order — observationally
//!   identical to streaming for an immutable result, and working memory
//!   is unreachable from the callback by construction.
//! - Rules are authored as DRL strings; everything outside the certified
//!   grammar stays a parse/compile error, exactly as in the engine.

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, Float64Builder, Int64Builder, StringBuilder,
};
use arrow_array::{Array, ArrayRef, RecordBatch, RecordBatchIterator, RecordBatchReader};
use arrow_schema::{DataType, Field, Schema};
use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyCapsule, PyDict, PyList};

use seine_engine::{Engine, FactView, FieldType, TypeSchema, Value};

// ---------------------------------------------------------------------
// Arrow ingestion: PyCapsule C-stream -> typed column pulls
// ---------------------------------------------------------------------

/// Import an Arrow stream from any object implementing
/// `__arrow_c_stream__` (polars, pyarrow, pandas>=2.2, arro3, ...).
fn import_stream(obj: &Bound<'_, PyAny>) -> PyResult<arrow::ffi_stream::ArrowArrayStreamReader> {
    if !obj.hasattr("__arrow_c_stream__")? {
        return Err(PyTypeError::new_err(
            "expected an Arrow-compatible table (anything implementing __arrow_c_stream__: \
             polars.DataFrame, pyarrow.Table, ...) or a dict of column lists",
        ));
    }
    let capsule_obj = obj.call_method0("__arrow_c_stream__")?;
    let capsule: &Bound<'_, PyCapsule> = capsule_obj.downcast()?;
    let name = capsule.name()?;
    let expected = CString::new("arrow_array_stream").unwrap();
    if name != Some(expected.as_c_str()) {
        return Err(PyValueError::new_err("capsule is not an arrow_array_stream"));
    }
    let ptr = capsule.pointer() as *mut arrow::ffi_stream::FFI_ArrowArrayStream;
    // Take ownership of the stream out of the capsule (the standard
    // consumption protocol: leave an empty struct behind).
    let stream = unsafe { std::ptr::replace(ptr, arrow::ffi_stream::FFI_ArrowArrayStream::empty()) };
    arrow::ffi_stream::ArrowArrayStreamReader::try_new(stream)
        .map_err(|e| PyValueError::new_err(format!("arrow stream import failed: {e}")))
}

/// The engine field type an Arrow column maps to, or an error for
/// anything outside the certified subset.
fn map_dtype(type_name: &str, field: &Field) -> PyResult<FieldType> {
    use DataType::*;
    let ft = match field.data_type() {
        Int8 | Int16 | Int32 | Int64 | UInt8 | UInt16 | UInt32 => FieldType::I64,
        Float32 | Float64 => FieldType::F64,
        Boolean => FieldType::Bool,
        Utf8 | LargeUtf8 | Utf8View => FieldType::Str,
        other => {
            return Err(PyTypeError::new_err(format!(
                "{type_name}.{}: Arrow type {other} is outside the certified subset \
                 (supported: int8..int64/uint8..uint32 -> i64, float32/64 -> f64, \
                 bool, utf8; cast or drop the column first)",
                field.name()
            )))
        }
    };
    Ok(ft)
}

/// Reject nulls loudly: the certified subset has no null semantics, and
/// silently zeroing them would void the differential guarantees on the
/// user's actual data.
fn reject_nulls(type_name: &str, field: &Field, arr: &dyn Array) -> PyResult<()> {
    if arr.null_count() > 0 {
        return Err(PyValueError::new_err(format!(
            "{type_name}.{}: {} null(s) present — nulls are outside the certified \
             subset; drop or fill them before insertion",
            field.name(),
            arr.null_count()
        )));
    }
    Ok(())
}

/// Pull one column as engine Values (exact widening only).
fn column_values(type_name: &str, field: &Field, arr: &ArrayRef) -> PyResult<Vec<Value>> {
    use arrow_array::cast::AsArray;
    use arrow_array::types::*;
    reject_nulls(type_name, field, arr.as_ref())?;
    let n = arr.len();
    let mut out = Vec::with_capacity(n);
    match arr.data_type() {
        DataType::Int64 => {
            let a = arr.as_primitive::<Int64Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v)));
        }
        DataType::Int32 => {
            let a = arr.as_primitive::<Int32Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::Int16 => {
            let a = arr.as_primitive::<Int16Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::Int8 => {
            let a = arr.as_primitive::<Int8Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::UInt8 => {
            let a = arr.as_primitive::<UInt8Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::UInt16 => {
            let a = arr.as_primitive::<UInt16Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::UInt32 => {
            let a = arr.as_primitive::<UInt32Type>();
            out.extend(a.values().iter().map(|&v| Value::I64(v as i64)));
        }
        DataType::Float64 => {
            let a = arr.as_primitive::<Float64Type>();
            out.extend(a.values().iter().map(|&v| Value::F64(v)));
        }
        DataType::Float32 => {
            let a = arr.as_primitive::<Float32Type>();
            out.extend(a.values().iter().map(|&v| Value::F64(v as f64)));
        }
        DataType::Boolean => {
            let a = arr.as_boolean();
            for i in 0..n {
                out.push(Value::Bool(a.value(i)));
            }
        }
        DataType::Utf8 => {
            let a = arr.as_string::<i32>();
            for i in 0..n {
                out.push(Value::Str(a.value(i).to_string()));
            }
        }
        DataType::LargeUtf8 => {
            let a = arr.as_string::<i64>();
            for i in 0..n {
                out.push(Value::Str(a.value(i).to_string()));
            }
        }
        DataType::Utf8View => {
            let a = arr.as_string_view();
            for i in 0..n {
                out.push(Value::Str(a.value(i).to_string()));
            }
        }
        other => {
            return Err(PyTypeError::new_err(format!(
                "{type_name}.{}: unsupported Arrow type {other}",
                field.name()
            )))
        }
    }
    Ok(out)
}

/// A dict of equal-length Python lists -> (schema fields, row values).
/// Convenience path for REPL-scale data; the same engine insert path.
fn columns_from_dict(
    type_name: &str,
    d: &Bound<'_, PyDict>,
) -> PyResult<(Vec<(String, FieldType)>, Vec<Vec<Value>>)> {
    let mut names: Vec<String> = Vec::new();
    let mut cols: Vec<Vec<Value>> = Vec::new();
    let mut fields: Vec<(String, FieldType)> = Vec::new();
    let mut nrows: Option<usize> = None;
    for (k, v) in d.iter() {
        let name: String = k.extract()?;
        let list: &Bound<'_, PyList> = v.downcast().map_err(|_| {
            PyTypeError::new_err(format!("{type_name}.{name}: expected a list of values"))
        })?;
        let mut col = Vec::with_capacity(list.len());
        let mut ft: Option<FieldType> = None;
        for item in list.iter() {
            let v = py_scalar(type_name, &name, &item)?;
            let t = v.type_of();
            match ft {
                None => ft = Some(t),
                Some(prev) if prev == t => {}
                // int -> float promotion inside one column
                Some(FieldType::F64) if t == FieldType::I64 => {}
                Some(FieldType::I64) if t == FieldType::F64 => {
                    ft = Some(FieldType::F64);
                }
                Some(prev) => {
                    return Err(PyTypeError::new_err(format!(
                        "{type_name}.{name}: mixed types {prev:?} vs {t:?}"
                    )))
                }
            }
            col.push(v);
        }
        let ft = ft.ok_or_else(|| {
            PyTypeError::new_err(format!(
                "{type_name}.{name}: empty column — schema cannot be inferred; \
                 pass an Arrow table with an explicit schema instead"
            ))
        })?;
        if let Some(r) = nrows {
            if r != col.len() {
                return Err(PyValueError::new_err(format!(
                    "{type_name}: ragged columns ({r} vs {} rows)",
                    col.len()
                )));
            }
        } else {
            nrows = Some(col.len());
        }
        // normalize promoted int literals in f64 columns
        if ft == FieldType::F64 {
            for v in col.iter_mut() {
                if let Value::I64(n) = v {
                    *v = Value::F64(*n as f64);
                }
            }
        }
        names.push(name.clone());
        fields.push((name, ft));
        cols.push(col);
    }
    Ok((fields, cols))
}

fn py_scalar(type_name: &str, field: &str, v: &Bound<'_, PyAny>) -> PyResult<Value> {
    if v.is_none() {
        return Err(PyValueError::new_err(format!(
            "{type_name}.{field}: None is outside the certified subset (no null semantics)"
        )));
    }
    // bool before int: Python bool is an int subclass
    if let Ok(b) = v.downcast::<pyo3::types::PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }
    if let Ok(n) = v.extract::<i64>() {
        return Ok(Value::I64(n));
    }
    if let Ok(f) = v.extract::<f64>() {
        return Ok(Value::F64(f));
    }
    if let Ok(s) = v.extract::<String>() {
        return Ok(Value::Str(s));
    }
    Err(PyTypeError::new_err(format!(
        "{type_name}.{field}: unsupported scalar {}",
        v.get_type().name()?
    )))
}

// ---------------------------------------------------------------------
// Arrow export: engine FactViews -> RecordBatch -> PyCapsule stream
// ---------------------------------------------------------------------

/// Build an Arrow batch for one fact type: `_handle` + schema columns.
fn batch_for_type(schema: &TypeSchema, rows: &[&FactView]) -> PyResult<RecordBatch> {
    let mut fields: Vec<Field> = vec![Field::new("_handle", DataType::Int64, false)];
    for (name, ft) in &schema.fields {
        let dt = match ft {
            FieldType::I64 => DataType::Int64,
            FieldType::F64 => DataType::Float64,
            FieldType::Bool => DataType::Boolean,
            FieldType::Str => DataType::Utf8,
        };
        fields.push(Field::new(name, dt, false));
    }
    let arrow_schema = Arc::new(Schema::new(fields));
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(schema.fields.len() + 1);
    let mut handles = Int64Builder::with_capacity(rows.len());
    for r in rows {
        handles.append_value(r.handle as i64);
    }
    arrays.push(Arc::new(handles.finish()));
    for (ci, (fname, ft)) in schema.fields.iter().enumerate() {
        let col: PyResult<ArrayRef> = match ft {
            FieldType::I64 => {
                let mut b = Int64Builder::with_capacity(rows.len());
                for r in rows {
                    match &r.fields[ci].1 {
                        Value::I64(n) => b.append_value(*n),
                        _ => return Err(PyRuntimeError::new_err(format!("{fname}: column type drift"))),
                    }
                }
                Ok(Arc::new(b.finish()))
            }
            FieldType::F64 => {
                let mut b = Float64Builder::with_capacity(rows.len());
                for r in rows {
                    match &r.fields[ci].1 {
                        Value::F64(x) => b.append_value(*x),
                        _ => return Err(PyRuntimeError::new_err(format!("{fname}: column type drift"))),
                    }
                }
                Ok(Arc::new(b.finish()))
            }
            FieldType::Bool => {
                let mut b = BooleanBuilder::with_capacity(rows.len());
                for r in rows {
                    match &r.fields[ci].1 {
                        Value::Bool(x) => b.append_value(*x),
                        _ => return Err(PyRuntimeError::new_err(format!("{fname}: column type drift"))),
                    }
                }
                Ok(Arc::new(b.finish()))
            }
            FieldType::Str => {
                let mut b = StringBuilder::new();
                for r in rows {
                    match &r.fields[ci].1 {
                        Value::Str(s) => b.append_value(s),
                        _ => return Err(PyRuntimeError::new_err(format!("{fname}: column type drift"))),
                    }
                }
                Ok(Arc::new(b.finish()))
            }
        };
        arrays.push(col?);
    }
    RecordBatch::try_new(arrow_schema, arrays)
        .map_err(|e| PyRuntimeError::new_err(format!("arrow batch build failed: {e}")))
}

/// A one-batch Arrow table exposed via the PyCapsule stream interface —
/// `polars.DataFrame(t)` / `pyarrow.table(t)` import it zero-copy.
#[pyclass(name = "Table")]
struct PyTable {
    batch: RecordBatch,
}

#[pymethods]
impl PyTable {
    fn __arrow_c_stream__<'py>(
        &self,
        py: Python<'py>,
        _requested_schema: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyCapsule>> {
        let schema = self.batch.schema();
        let reader: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
            vec![Ok(self.batch.clone())].into_iter(),
            schema,
        ));
        let stream = arrow::ffi_stream::FFI_ArrowArrayStream::new(reader);
        let name = CString::new("arrow_array_stream").unwrap();
        PyCapsule::new(py, stream, Some(name))
    }

    fn __len__(&self) -> usize {
        self.batch.num_rows()
    }

    fn __repr__(&self) -> String {
        format!(
            "seine.Table({} rows x {} cols)",
            self.batch.num_rows(),
            self.batch.num_columns() - 1
        )
    }
}

// ---------------------------------------------------------------------
// Session / Result
// ---------------------------------------------------------------------

/// Firing audit row storage (long format).
struct AuditRows {
    seq: Vec<i64>,
    rule: Vec<String>,
    pos: Vec<i64>,
    ftype: Vec<String>,
    handle: Vec<i64>,
    values_json: Vec<String>,
}

fn fact_json(fv: &FactView) -> String {
    let mut m = serde_json::Map::new();
    for (k, v) in &fv.fields {
        let jv = match v {
            Value::I64(n) => serde_json::json!(n),
            Value::F64(x) => serde_json::json!(x),
            Value::Bool(b) => serde_json::json!(b),
            Value::Str(s) => serde_json::json!(s),
        };
        m.insert(k.clone(), jv);
    }
    if let Some(elems) = &fv.elems {
        m.insert(
            "value".into(),
            serde_json::Value::Array(
                elems.iter().map(|e| serde_json::Value::String(fact_json(e))).collect(),
            ),
        );
    }
    serde_json::Value::Object(m).to_string()
}

#[pyclass(name = "Result")]
struct PyResult_ {
    /// type name -> final live facts batch
    facts: HashMap<String, RecordBatch>,
    /// type name -> facts inserted BY RULES during the run
    derived: HashMap<String, RecordBatch>,
    /// handles the run deleted (of facts Python inserted)
    deleted: Vec<i64>,
    firings: RecordBatch,
    fired: usize,
}

#[pymethods]
impl PyResult_ {
    /// Final working memory, one Arrow table per fact type.
    fn facts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, b) in &self.facts {
            d.set_item(k, PyTable { batch: b.clone() }.into_pyobject(py)?)?;
        }
        Ok(d)
    }

    /// Facts derived (inserted) by rule firings, per type — the WM delta.
    fn derived<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, b) in &self.derived {
            d.set_item(k, PyTable { batch: b.clone() }.into_pyobject(py)?)?;
        }
        Ok(d)
    }

    /// Handles of Python-inserted facts the run deleted.
    fn deleted_handles(&self) -> Vec<i64> {
        self.deleted.clone()
    }

    /// Long-format firing audit: (seq, rule, pos, type, handle,
    /// values_json) — values as rendered at fire time (post-RHS).
    fn firings(&self) -> PyTable {
        PyTable { batch: self.firings.clone() }
    }

    #[getter]
    fn fired(&self) -> usize {
        self.fired
    }

    fn __repr__(&self) -> String {
        format!(
            "seine.Result(fired={}, derived_types={}, deleted={})",
            self.fired,
            self.derived.len(),
            self.deleted.len()
        )
    }
}

#[pyclass(name = "Session")]
struct PySession {
    engine: Option<Engine>,
    schemas: Vec<TypeSchema>,
    drl: String,
    /// handles Python inserted (fact provenance for the delta)
    py_handles: Vec<u32>,
    built: bool,
}

impl PySession {
    fn ensure_built(&mut self) -> PyResult<()> {
        if self.built {
            return Ok(());
        }
        let mut engine = Engine::new(self.schemas.clone())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        engine
            .add_rules_drl(&self.drl)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.engine = Some(engine);
        self.built = true;
        Ok(())
    }

    fn insert_columns(
        &mut self,
        type_name: &str,
        fields: &[(String, FieldType)],
        cols: Vec<Vec<Value>>,
    ) -> PyResult<()> {
        let nrows = cols.first().map(|c| c.len()).unwrap_or(0);
        let engine = self.engine.as_mut().expect("built");
        for r in 0..nrows {
            let row: Vec<(String, Value)> = fields
                .iter()
                .enumerate()
                .map(|(ci, (n, _))| (n.clone(), cols[ci][r].clone()))
                .collect();
            let id = engine
                .insert(type_name, row)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            self.py_handles.push(id.0);
        }
        Ok(())
    }
}

#[pymethods]
impl PySession {
    /// Session(drl, facts=None): declared types come from the ingested
    /// tables' schemas (constructor argument order in DRL = column
    /// order). One-shot: insert(s) then a single fire().
    #[new]
    #[pyo3(signature = (drl, facts=None))]
    fn new(py: Python<'_>, drl: String, facts: Option<Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut sess = PySession {
            engine: None,
            schemas: Vec::new(),
            drl,
            py_handles: Vec::new(),
            built: false,
        };
        if let Some(f) = facts {
            // Pass 1: schemas from every table, so cross-type rules
            // compile regardless of dict order.
            let mut pending: Vec<(String, Vec<(String, FieldType)>, Vec<Vec<Value>>)> = Vec::new();
            for (k, v) in f.iter() {
                let type_name: String = k.extract()?;
                let (fields, cols) = ingest_any(py, &type_name, &v)?;
                sess.schemas.push(TypeSchema {
                    name: type_name.clone(),
                    fields: fields.clone(),
                });
                pending.push((type_name, fields, cols));
            }
            sess.ensure_built()?;
            for (type_name, fields, cols) in pending {
                sess.insert_columns(&type_name, &fields, cols)?;
            }
        }
        Ok(sess)
    }

    /// Insert more rows before fire(): an Arrow table or a dict of
    /// column lists. The type must already be known to the session.
    fn insert(&mut self, py: Python<'_>, type_name: String, data: Bound<'_, PyAny>) -> PyResult<()> {
        if self.engine.is_none() {
            return Err(PyRuntimeError::new_err(
                "session has no declared types: construct with facts= to establish schemas",
            ));
        }
        if !self.schemas.iter().any(|s| s.name == type_name) {
            return Err(PyValueError::new_err(format!(
                "unknown fact type {type_name} (types are declared by the constructor's tables)"
            )));
        }
        let (fields, cols) = ingest_any(py, &type_name, &data)?;
        let declared = &self.schemas.iter().find(|s| s.name == type_name).unwrap().fields;
        if &fields != declared {
            return Err(PyValueError::new_err(format!(
                "{type_name}: schema mismatch with the declaring table"
            )));
        }
        self.insert_columns(&type_name, &fields, cols)
    }

    /// Insert a single fact from keyword-style dict (REPL convenience;
    /// same bulk path, batch of one).
    fn insert_row(&mut self, type_name: String, row: Bound<'_, PyDict>) -> PyResult<()> {
        if self.engine.is_none() {
            return Err(PyRuntimeError::new_err(
                "session has no declared types: construct with facts= to establish schemas",
            ));
        }
        let declared = self
            .schemas
            .iter()
            .find(|s| s.name == type_name)
            .ok_or_else(|| PyValueError::new_err(format!("unknown fact type {type_name}")))?
            .fields
            .clone();
        let mut vals: Vec<(String, Value)> = Vec::new();
        for (fname, ft) in &declared {
            let item = row
                .get_item(fname)?
                .ok_or_else(|| PyValueError::new_err(format!("{type_name}: missing field {fname}")))?;
            let mut v = py_scalar(&type_name, fname, &item)?;
            if *ft == FieldType::F64 {
                if let Value::I64(n) = v {
                    v = Value::F64(n as f64);
                }
            }
            vals.push((fname.clone(), v));
        }
        let engine = self.engine.as_mut().unwrap();
        let id = engine
            .insert(&type_name, vals)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.py_handles.push(id.0);
        Ok(())
    }

    /// Run the rules to quiescence. ONE-SHOT: a second call raises.
    /// `on_fire(rule, matches)` is an OBSERVER invoked per firing after
    /// the run completes, in firing order; matches is a list of
    /// (type, handle) pairs. The run itself releases the GIL.
    #[pyo3(signature = (fire_limit=100_000, on_fire=None))]
    fn fire(
        &mut self,
        py: Python<'_>,
        fire_limit: usize,
        on_fire: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyResult_> {
        self.ensure_built()?;
        let mut engine = self
            .engine
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("one-shot session: fire() already called"))?;
        let firings = py
            .allow_threads(|| engine.fire_all(fire_limit))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        // observer callbacks (plain data; WM unreachable)
        if let Some(cb) = on_fire {
            for f in &firings {
                let matches: Vec<(String, i64)> = f
                    .matches
                    .iter()
                    .map(|m| (m.type_name.clone(), m.handle as i64))
                    .collect();
                cb.call1((f.rule.clone(), matches))?;
            }
        }

        // WM delta + final view
        let live = engine.facts();
        let py_set: std::collections::HashSet<u32> = self.py_handles.iter().copied().collect();
        let live_set: std::collections::HashSet<u32> = live.iter().map(|f| f.handle).collect();
        let mut facts: HashMap<String, RecordBatch> = HashMap::new();
        let mut derived: HashMap<String, RecordBatch> = HashMap::new();
        for schema in &self.schemas {
            let all: Vec<&FactView> =
                live.iter().filter(|f| f.type_name == schema.name).collect();
            let new: Vec<&FactView> = all
                .iter()
                .copied()
                .filter(|f| !py_set.contains(&f.handle))
                .collect();
            facts.insert(schema.name.clone(), batch_for_type(schema, &all)?);
            derived.insert(schema.name.clone(), batch_for_type(schema, &new)?);
        }
        let deleted: Vec<i64> = self
            .py_handles
            .iter()
            .filter(|h| !live_set.contains(h))
            .map(|&h| h as i64)
            .collect();

        // firing audit (long format)
        let mut a = AuditRows {
            seq: Vec::new(),
            rule: Vec::new(),
            pos: Vec::new(),
            ftype: Vec::new(),
            handle: Vec::new(),
            values_json: Vec::new(),
        };
        for (i, f) in firings.iter().enumerate() {
            for (pos, m) in f.matches.iter().enumerate() {
                a.seq.push(i as i64);
                a.rule.push(f.rule.clone());
                a.pos.push(pos as i64);
                a.ftype.push(m.type_name.clone());
                a.handle.push(m.handle as i64);
                a.values_json.push(fact_json(m));
            }
        }
        let audit_schema = Arc::new(Schema::new(vec![
            Field::new("seq", DataType::Int64, false),
            Field::new("rule", DataType::Utf8, false),
            Field::new("pos", DataType::Int64, false),
            Field::new("type", DataType::Utf8, false),
            Field::new("handle", DataType::Int64, false),
            Field::new("values_json", DataType::Utf8, false),
        ]));
        let mut rule_b = StringBuilder::new();
        let mut type_b = StringBuilder::new();
        let mut json_b = StringBuilder::new();
        for v in &a.rule {
            rule_b.append_value(v);
        }
        for v in &a.ftype {
            type_b.append_value(v);
        }
        for v in &a.values_json {
            json_b.append_value(v);
        }
        let audit = RecordBatch::try_new(
            audit_schema,
            vec![
                Arc::new(arrow_array::Int64Array::from(a.seq)),
                Arc::new(rule_b.finish()),
                Arc::new(arrow_array::Int64Array::from(a.pos)),
                Arc::new(type_b.finish()),
                Arc::new(arrow_array::Int64Array::from(a.handle)),
                Arc::new(json_b.finish()),
            ],
        )
        .map_err(|e| PyRuntimeError::new_err(format!("audit batch build failed: {e}")))?;

        Ok(PyResult_ {
            facts,
            derived,
            deleted,
            firings: audit,
            fired: firings.len(),
        })
    }
}

/// Ingest either an Arrow-stream-capable object or a dict of lists.
fn ingest_any(
    _py: Python<'_>,
    type_name: &str,
    obj: &Bound<'_, PyAny>,
) -> PyResult<(Vec<(String, FieldType)>, Vec<Vec<Value>>)> {
    if let Ok(d) = obj.downcast::<PyDict>() {
        return columns_from_dict(type_name, d);
    }
    let mut reader = import_stream(obj)?;
    let schema = reader.schema();
    let mut fields: Vec<(String, FieldType)> = Vec::new();
    for f in schema.fields() {
        fields.push((f.name().clone(), map_dtype(type_name, f)?));
    }
    let mut cols: Vec<Vec<Value>> = vec![Vec::new(); fields.len()];
    while let Some(batch) = reader.next() {
        let batch = batch.map_err(|e| PyValueError::new_err(format!("arrow read: {e}")))?;
        for (ci, f) in schema.fields().iter().enumerate() {
            let vals = column_values(type_name, f, batch.column(ci))?;
            cols[ci].extend(vals);
        }
    }
    Ok((fields, cols))
}

/// One-call convenience: build a session from tables, fire, return the
/// result. `seine.run(drl, {"P": df})`.
#[pyfunction]
#[pyo3(signature = (drl, facts, fire_limit=100_000, on_fire=None))]
fn run(
    py: Python<'_>,
    drl: String,
    facts: Bound<'_, PyDict>,
    fire_limit: usize,
    on_fire: Option<Bound<'_, PyAny>>,
) -> PyResult<PyResult_> {
    let mut sess = PySession::new(py, drl, Some(facts))?;
    sess.fire(py, fire_limit, on_fire)
}

#[pymodule]
fn seine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyResult_>()?;
    m.add_class::<PyTable>()?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}
