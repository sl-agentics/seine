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
//! - Sessions support MULTI-FIRE (D-046): insert -> fire -> insert more
//!   -> fire again, each fire returning ITS OWN delta. The incremental
//!   envelope is differentially certified (epoch scenarios + campaign).
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

mod derive;

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
        Decimal128(p, s) => {
            if *p == 0 || *p > 38 || *s < 0 || *s > *p as i8 {
                return Err(PyTypeError::new_err(format!(
                    "{type_name}.{}: decimal128({p},{s}) is outside Decimal128 limits",
                    field.name()
                )));
            }
            FieldType::Dec { p: *p, s: *s as u8 }
        }
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

/// Subset type strings from the authoring layer (D-098): base types,
/// `decimal(p,s)`, and a trailing `?` for nullable (Optional[X]).
fn parse_subset_type(
    type_name: &str,
    fname: &str,
    spec: &str,
) -> PyResult<(FieldType, bool)> {
    let (base, nullable) = match spec.strip_suffix('?') {
        Some(b) => (b, true),
        None => (spec, false),
    };
    let ft = match base {
        "i64" => FieldType::I64,
        "f64" => FieldType::F64,
        "bool" => FieldType::Bool,
        "String" => FieldType::Str,
        d if d.starts_with("decimal(") && d.ends_with(')') => {
            let inner = &d["decimal(".len()..d.len() - 1];
            let (p, s) = inner.split_once(',').ok_or_else(|| {
                PyValueError::new_err(format!("{type_name}.{fname}: bad decimal spec {spec:?}"))
            })?;
            let p: u8 = p.trim().parse().map_err(|_| {
                PyValueError::new_err(format!("{type_name}.{fname}: bad decimal spec {spec:?}"))
            })?;
            let s: u8 = s.trim().parse().map_err(|_| {
                PyValueError::new_err(format!("{type_name}.{fname}: bad decimal spec {spec:?}"))
            })?;
            if p == 0 || p > 38 || s > p {
                return Err(PyValueError::new_err(format!(
                    "{type_name}.{fname}: decimal(p,s) needs 1<=p<=38, 0<=s<=p"
                )));
            }
            FieldType::Dec { p, s }
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "{type_name}.{fname}: unknown subset type {other:?}"
            )))
        }
    };
    Ok((ft, nullable))
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

/// Pull one column as engine Values (exact widening only). `target` =
/// the DECLARED (type, nullable) when a schema was declared: nullable
/// columns turn validity-nulls into Value::Null and normalize float
/// NaN -> NULL (D-095/D-098 §6 point 5 — the type declaration IS the
/// NaN-vs-NULL choice); non-nullable keeps the loud D-044 rejection
/// and bit-exact NaN.
fn column_values(
    type_name: &str,
    field: &Field,
    arr: &ArrayRef,
    target: Option<(FieldType, bool)>,
) -> PyResult<Vec<Value>> {
    use arrow_array::cast::AsArray;
    use arrow_array::types::*;
    let nullable = target.map(|(_, n)| n).unwrap_or(false);
    if !nullable {
        reject_nulls(type_name, field, arr.as_ref())?;
    }
    let n = arr.len();
    let mut out = Vec::with_capacity(n);
    // decimal128 columns (exact) — rescaled to the declared (p,s)
    if let DataType::Decimal128(_, arr_s) = arr.data_type() {
        let a = arr.as_primitive::<Decimal128Type>();
        let (tp, ts) = match target {
            Some((FieldType::Dec { p, s }, _)) => (p, s),
            None => match map_dtype(type_name, field)? {
                FieldType::Dec { p, s } => (p, s),
                _ => unreachable!(),
            },
            Some((other, _)) => {
                return Err(PyTypeError::new_err(format!(
                    "{type_name}.{}: decimal column for a {other:?} field",
                    field.name()
                )))
            }
        };
        for i in 0..n {
            if a.is_null(i) {
                out.push(Value::Null);
                continue;
            }
            let (u, s) = seine_engine::dec_rescale_pub(a.value(i), *arr_s as u8, ts)
                .filter(|(u2, _)| seine_engine::dec_fits_pub(*u2, tp))
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "{type_name}.{}: decimal value overflows decimal({tp},{ts})",
                        field.name()
                    ))
                })?;
            out.push(Value::Dec { u, s });
        }
        return Ok(out);
    }
    if nullable && arr.null_count() > 0 || nullable && matches!(arr.data_type(), DataType::Float64 | DataType::Float32) {
        // per-element path: validity -> Null; NaN -> Null for floats
        for i in 0..n {
            if arr.is_null(i) {
                out.push(Value::Null);
                continue;
            }
            let v = match arr.data_type() {
                DataType::Int64 => Value::I64(arr.as_primitive::<Int64Type>().value(i)),
                DataType::Int32 => Value::I64(arr.as_primitive::<Int32Type>().value(i) as i64),
                DataType::Int16 => Value::I64(arr.as_primitive::<Int16Type>().value(i) as i64),
                DataType::Int8 => Value::I64(arr.as_primitive::<Int8Type>().value(i) as i64),
                DataType::UInt8 => Value::I64(arr.as_primitive::<UInt8Type>().value(i) as i64),
                DataType::UInt16 => Value::I64(arr.as_primitive::<UInt16Type>().value(i) as i64),
                DataType::UInt32 => Value::I64(arr.as_primitive::<UInt32Type>().value(i) as i64),
                DataType::Float64 => {
                    let x = arr.as_primitive::<Float64Type>().value(i);
                    if x.is_nan() { Value::Null } else { Value::F64(x) }
                }
                DataType::Float32 => {
                    let x = arr.as_primitive::<Float32Type>().value(i) as f64;
                    if x.is_nan() { Value::Null } else { Value::F64(x) }
                }
                DataType::Boolean => Value::Bool(arr.as_boolean().value(i)),
                DataType::Utf8 => Value::Str(arr.as_string::<i32>().value(i).to_string()),
                DataType::LargeUtf8 => Value::Str(arr.as_string::<i64>().value(i).to_string()),
                DataType::Utf8View => Value::Str(arr.as_string_view().value(i).to_string()),
                other => {
                    return Err(PyTypeError::new_err(format!(
                        "{type_name}.{}: Arrow type {other} is outside the certified subset",
                        field.name()
                    )))
                }
            };
            out.push(v);
        }
        return Ok(out);
    }
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
    target_of: &dyn Fn(&str) -> Option<(FieldType, bool)>,
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
        let target = target_of(&name);
        let mut ft: Option<FieldType> = target.map(|(t, _)| t);
        for item in list.iter() {
            let v = py_scalar(type_name, &name, &item, target)?;
            if matches!(v, Value::Null | Value::Dec { .. }) {
                col.push(v);
                continue; // typed by the declared target; skip inference
            }
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

fn py_scalar(
    type_name: &str,
    field: &str,
    v: &Bound<'_, PyAny>,
    target: Option<(FieldType, bool)>,
) -> PyResult<Value> {
    let nullable = target.map(|(_, n)| n).unwrap_or(false);
    if v.is_none() {
        if nullable {
            return Ok(Value::Null);
        }
        return Err(PyValueError::new_err(format!(
            "{type_name}.{field}: None is outside the certified subset for a \
             non-nullable field — declare it Optional[...] to opt in (D-097)"
        )));
    }
    // python decimal.Decimal -> exact engine decimal (D-098): via str,
    // never through a float
    let py = v.py();
    let dec_cls = py.import("decimal")?.getattr("Decimal")?;
    if v.is_instance(&dec_cls)? {
        let Some((FieldType::Dec { p, s }, _)) = target else {
            return Err(PyTypeError::new_err(format!(
                "{type_name}.{field}: decimal.Decimal for a non-decimal field — declare \
                 Annotated[Decimal, seine.Decimal(p, s)] (D-098)"
            )));
        };
        let txt: String = v.call_method0("__str__")?.extract()?;
        let plain = if txt.contains(['e', 'E']) {
            v.call_method1("__format__", ("f",))?.extract::<String>()?
        } else {
            txt
        };
        let parsed = seine_engine::dec_parse(&plain)
            .and_then(|(u0, s0)| seine_engine::dec_rescale_pub(u0, s0, s))
            .filter(|(u, _)| seine_engine::dec_fits_pub(*u, p));
        return match parsed {
            Some((u, s)) => Ok(Value::Dec { u, s }),
            None => Err(PyValueError::new_err(format!(
                "{type_name}.{field}: {plain} does not fit decimal({p},{s})"
            ))),
        };
    }
    // bool before int: Python bool is an int subclass
    if let Ok(b) = v.downcast::<pyo3::types::PyBool>() {
        return Ok(Value::Bool(b.is_true()));
    }
    if let Ok(n) = v.extract::<i64>() {
        if let Some((FieldType::Dec { p, s }, _)) = target {
            let parsed = seine_engine::dec_rescale_pub(n as i128, 0, s)
                .filter(|(u, _)| seine_engine::dec_fits_pub(*u, p));
            return match parsed {
                Some((u, s)) => Ok(Value::Dec { u, s }),
                None => Err(PyValueError::new_err(format!(
                    "{type_name}.{field}: {n} does not fit decimal({p},{s})"
                ))),
            };
        }
        return Ok(Value::I64(n));
    }
    // a genuine Python int too large for i64 must fail HERE, loudly —
    // otherwise it decays into the f64 fallback below and surfaces
    // later as a baffling schema mismatch (or a silently-float column)
    if v.downcast::<pyo3::types::PyInt>().is_ok() {
        if matches!(target, Some((FieldType::F64, _))) {
            // declared float field: same promotion ints already get
            if let Ok(f) = v.extract::<f64>() {
                return Ok(Value::F64(f));
            }
        }
        return Err(PyValueError::new_err(format!(
            "{type_name}.{field}: {v} does not fit a 64-bit signed integer \
             (i64 / Java long: -9223372036854775808..9223372036854775807)"
        )));
    }
    if let Ok(f) = v.extract::<f64>() {
        if matches!(target, Some((FieldType::Dec { .. }, _))) {
            return Err(PyTypeError::new_err(format!(
                "{type_name}.{field}: floats never ingest into decimals — pass \
                 decimal.Decimal or str (D-098)"
            )));
        }
        if f.is_nan() && nullable {
            // D-098 §6 point 5: Optional[float] normalizes NaN -> NULL
            return Ok(Value::Null);
        }
        return Ok(Value::F64(f));
    }
    if let Ok(s) = v.extract::<String>() {
        if let Some((ft @ FieldType::Dec { .. }, _)) = target {
            return py_dec_from_str(type_name, field, &s, ft);
        }
        return Ok(Value::Str(s));
    }
    Err(PyTypeError::new_err(format!(
        "{type_name}.{field}: unsupported scalar {}",
        v.get_type().name()?
    )))
}

fn py_dec_from_str(
    type_name: &str,
    field: &str,
    txt: &str,
    ft: FieldType,
) -> PyResult<Value> {
    let FieldType::Dec { p, s } = ft else { unreachable!() };
    seine_engine::dec_parse(txt)
        .and_then(|(u0, s0)| seine_engine::dec_rescale_pub(u0, s0, s))
        .filter(|(u, _)| seine_engine::dec_fits_pub(*u, p))
        .map(|(u, s)| Value::Dec { u, s })
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "{type_name}.{field}: {txt:?} does not fit decimal({p},{s})"
            ))
        })
}

/// One engine Value as a native Python object (query rows). Decimals
/// come back as decimal.Decimal via their exact string rendering.
fn value_to_py(py: Python<'_>, v: &Value) -> PyResult<PyObject> {
    Ok(match v {
        Value::I64(n) => n.into_pyobject(py)?.into_any().unbind(),
        Value::F64(x) => x.into_pyobject(py)?.into_any().unbind(),
        Value::Bool(b) => pyo3::types::PyBool::new(py, *b).to_owned().into_any().unbind(),
        Value::Str(t) => t.into_pyobject(py)?.into_any().unbind(),
        Value::Null => py.None(),
        Value::Dec { u, s } => {
            let txt = seine_engine::dec_render(*u, *s);
            py.import("decimal")?
                .getattr("Decimal")?
                .call1((txt,))?
                .unbind()
        }
    })
}

// ---------------------------------------------------------------------
// Arrow export: engine FactViews -> RecordBatch -> PyCapsule stream
// ---------------------------------------------------------------------

/// Build an Arrow batch for one fact type: `handle` + schema columns.
fn batch_for_type(schema: &TypeSchema, rows: &[&FactView]) -> PyResult<RecordBatch> {
    let mut fields: Vec<Field> = vec![Field::new("handle", DataType::Int64, false)];
    for (fi, (name, ft)) in schema.fields.iter().enumerate() {
        let dt = match ft {
            FieldType::I64 => DataType::Int64,
            FieldType::F64 => DataType::Float64,
            FieldType::Bool => DataType::Boolean,
            FieldType::Str => DataType::Utf8,
            FieldType::Dec { p, s } => DataType::Decimal128(*p, *s as i8),
        };
        fields.push(Field::new(name, dt, schema.nullable >> fi & 1 == 1));
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
                        Value::Null => b.append_null(),
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
                        Value::Null => b.append_null(),
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
                        Value::Null => b.append_null(),
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
                        Value::Null => b.append_null(),
                        _ => return Err(PyRuntimeError::new_err(format!("{fname}: column type drift"))),
                    }
                }
                Ok(Arc::new(b.finish()))
            }
            FieldType::Dec { p, s } => {
                let mut b = arrow_array::builder::Decimal128Builder::with_capacity(rows.len())
                    .with_precision_and_scale(*p, *s as i8)
                    .map_err(|e| PyRuntimeError::new_err(format!("{fname}: {e}")))?;
                for r in rows {
                    match &r.fields[ci].1 {
                        Value::Dec { u, s: vs } => {
                            let (ru, _) = seine_engine::dec_rescale_pub(*u, *vs, *s)
                                .ok_or_else(|| {
                                    PyRuntimeError::new_err(format!("{fname}: decimal rescale overflow"))
                                })?;
                            b.append_value(ru);
                        }
                        Value::Null => b.append_null(),
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

/// A one-batch Arrow table. Consume it zero-copy via the PyCapsule
/// C-stream interface: `t.to_arrow()` (pyarrow.Table), `t.to_polars()`
/// (polars.DataFrame), `t.to_pylist()` (list of dicts), or hand it
/// directly to anything accepting `__arrow_c_stream__`
/// (`pyarrow.table(t)`, `polars.DataFrame(t)`, pandas>=2.2, arro3, ...).
/// Fact tables carry a `handle` column: the engine's fact handle,
/// correlating rows with `Result.deleted_handles`, the firings audit,
/// and `Session.update`/`Session.delete`.
#[pyclass(name = "Table")]
pub(crate) struct PyTable {
    pub(crate) batch: RecordBatch,
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

    /// Materialize as a `pyarrow.Table` (zero-copy C-stream import).
    fn to_arrow<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let pa = py
            .import("pyarrow")
            .map_err(|e| optional_dep_err(py, e, "to_arrow()", "pyarrow", "arrow"))?;
        pa.call_method1("table", (slf,))
    }

    /// Materialize as a `polars.DataFrame` (zero-copy C-stream import;
    /// needs polars only — not pyarrow).
    fn to_polars<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let pl = py
            .import("polars")
            .map_err(|e| optional_dep_err(py, e, "to_polars()", "polars", "polars"))?;
        pl.call_method1("DataFrame", (slf,))
    }

    /// Materialize as a list of row dicts — natively, so the
    /// dependency-free wheel can read its own results without any
    /// optional install. Nulls land as None; Decimal128 lands as
    /// `decimal.Decimal` (pyarrow's to_pylist parity).
    fn to_pylist<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyList>> {
        use arrow_array::cast::AsArray;
        use arrow_array::types::{Decimal128Type, Float64Type, Int64Type};
        let py = slf.py();
        let t = slf.borrow();
        let schema = t.batch.schema();
        let names: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
        let dec_cls = if t
            .batch
            .columns()
            .iter()
            .any(|c| matches!(c.data_type(), DataType::Decimal128(_, _)))
        {
            Some(py.import("decimal")?.getattr("Decimal")?)
        } else {
            None
        };
        let out = PyList::empty(py);
        for i in 0..t.batch.num_rows() {
            let row = PyDict::new(py);
            for (ci, col) in t.batch.columns().iter().enumerate() {
                let name = names[ci].as_str();
                if col.is_null(i) {
                    row.set_item(name, py.None())?;
                    continue;
                }
                match col.data_type() {
                    DataType::Int64 => {
                        row.set_item(name, col.as_primitive::<Int64Type>().value(i))?
                    }
                    DataType::Float64 => {
                        row.set_item(name, col.as_primitive::<Float64Type>().value(i))?
                    }
                    DataType::Boolean => row.set_item(name, col.as_boolean().value(i))?,
                    DataType::Utf8 => row.set_item(name, col.as_string::<i32>().value(i))?,
                    DataType::Decimal128(_, _) => {
                        let s = col.as_primitive::<Decimal128Type>().value_as_string(i);
                        row.set_item(name, dec_cls.as_ref().unwrap().call1((s,))?)?;
                    }
                    other => {
                        return Err(PyRuntimeError::new_err(format!(
                            "to_pylist: unsupported column type {other:?} in {name}"
                        )))
                    }
                }
            }
            out.append(row)?;
        }
        Ok(out)
    }

    fn __repr__(&self) -> String {
        format!(
            "seine_rs.Table({} rows x {} cols)",
            self.batch.num_rows(),
            self.batch.num_columns()
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
            Value::Null => serde_json::Value::Null,
            Value::Dec { u, s } => serde_json::json!(seine_engine::dec_render(*u, *s)),
        };
        m.insert(k.clone(), jv);
    }
    if let Some(elems) = &fv.elems {
        m.insert(
            "value".into(),
            serde_json::Value::Array(
                elems
                    .iter()
                    .map(|e| match e {
                        Some(e) => serde_json::Value::String(fact_json(e)),
                        None => serde_json::Value::Null,
                    })
                    .collect(),
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
    #[getter]
    fn facts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, b) in &self.facts {
            d.set_item(k, PyTable { batch: b.clone() }.into_pyobject(py)?)?;
        }
        Ok(d)
    }

    /// Facts derived (inserted) by rule firings, per type — the WM delta.
    #[getter]
    fn derived<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        for (k, b) in &self.derived {
            d.set_item(k, PyTable { batch: b.clone() }.into_pyobject(py)?)?;
        }
        Ok(d)
    }

    /// Every handle that LEFT working memory without Python asking for
    /// it by name: RHS delete()s during this fire, plus truth-maintenance
    /// retractions — including those a between-fire Session.delete() or
    /// update() triggered synchronously. Handles Python deleted itself
    /// are not echoed back (mirror of Python inserts staying out of
    /// `derived`); Session.delete() returns its own TMS cascade directly.
    #[getter]
    fn deleted_handles(&self) -> Vec<i64> {
        self.deleted.clone()
    }

    /// Long-format firing audit: (seq, rule, pos, type, handle,
    /// values_json) — values as rendered at fire time (post-RHS).
    /// `values_json` is a JSON string column by design: one audit table
    /// spans every fact type a rule can match, and heterogeneous fact
    /// schemas cannot share Arrow columns — long format with per-row
    /// JSON is the faithful columnar encoding of a mixed-type log.
    #[getter]
    fn firings(&self) -> PyTable {
        PyTable { batch: self.firings.clone() }
    }

    #[getter]
    fn fired(&self) -> usize {
        self.fired
    }

    fn __repr__(&self) -> String {
        format!(
            "seine_rs.Result(fired={}, derived_types={}, deleted={})",
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
    built: bool,
    /// Set by the first fire(); external actions before it would run
    /// against the engine's pre-build staging batch — a shape the
    /// certified epoch sequence (initial facts, fire, then act+fire)
    /// structurally never produces.
    fired_once: bool,
    /// Event declarations (type -> (ts_field, expires_ms, duration
    /// field)), applied before rule compilation at build time. expires
    /// None = the certified inference (D-109); duration Some = interval
    /// event.
    events: Vec<(String, String, Option<i64>, Option<String>)>,
    /// Handles retracted by the ENGINE between fires — the TMS cascade
    /// of an external delete()/update() (a justified fact losing its
    /// premise dies synchronously, before the next fire's before-set is
    /// snapshotted). Merged into the next fire's deleted_handles so the
    /// WM-delta stays complete; the explicitly-acted-on handle itself is
    /// NOT included (Python already knows its own actions, same as
    /// between-fire inserts staying out of `derived`).
    pending_retracted: Vec<i64>,
}

impl PySession {
    fn ensure_built(&mut self) -> PyResult<()> {
        if self.built {
            return Ok(());
        }
        let mut engine = Engine::new(self.schemas.clone())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        for (tname, ts_field, expires_ms, duration) in &self.events {
            engine
                .declare_event(tname, ts_field, *expires_ms, duration.as_deref())
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }
        engine
            .add_rules_drl(&self.drl)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.engine = Some(engine);
        self.built = true;
        Ok(())
    }

    /// External actions compose action-ordered at the certified epoch
    /// boundary — which only exists once the initial state has been
    /// drained. Before the first fire() they would land in the engine's
    /// staging batch instead, where clock movement and event arrival
    /// compose differently than every certified scenario.
    fn require_fired(&self, what: &str) -> PyResult<()> {
        if self.fired_once {
            return Ok(());
        }
        Err(PyRuntimeError::new_err(format!(
            "{what} before the session's first fire() is outside the certified \
            epoch shape — the initial facts are still a staging batch, and clock \
            movement or mutation against that batch composes differently from \
            every certified sequence. Call fire() to drain the initial state \
            first (the certified shape is: construct with facts, fire, then per \
            epoch: act, fire). Inserting more facts before the first fire is \
            fine — that IS the initial batch."
        )))
    }

    fn insert_columns(
        &mut self,
        type_name: &str,
        fields: &[(String, FieldType)],
        cols: Vec<Vec<Value>>,
    ) -> PyResult<Vec<i64>> {
        let nrows = cols.first().map(|c| c.len()).unwrap_or(0);
        let engine = self.engine.as_mut().expect("built");
        let mut handles = Vec::with_capacity(nrows);
        for r in 0..nrows {
            let row: Vec<(String, Value)> = fields
                .iter()
                .enumerate()
                .map(|(ci, (n, _))| (n.clone(), cols[ci][r].clone()))
                .collect();
            let id = engine
                .insert(type_name, row)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            handles.push(id.0 as i64);
        }
        Ok(handles)
    }
}

#[pymethods]
impl PySession {
    /// Session(drl, facts=None, schemas=None): declared types come from
    /// the ingested tables' schemas plus any EXPLICIT schemas
    /// ({type: {field: "i64"|"f64"|"bool"|"String"}} — lets
    /// @fact-class keys declare types with zero rows). Constructor
    /// argument order in DRL = field order.
    #[new]
    #[pyo3(signature = (drl, facts=None, schemas=None, events=None))]
    fn new(
        py: Python<'_>,
        drl: String,
        facts: Option<Bound<'_, PyDict>>,
        schemas: Option<Bound<'_, PyDict>>,
        events: Option<Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let mut sess = PySession {
            engine: None,
            schemas: Vec::new(),
            drl,
            built: false,
            fired_once: false,
            events: Vec::new(),
            pending_retracted: Vec::new(),
        };
        if let Some(ev) = events {
            for (k, v) in ev.iter() {
                let tname: String = k.extract()?;
                let (ts_field, expires_ms, duration): (String, Option<i64>, Option<String>) =
                    v.extract()?;
                sess.events.push((tname, ts_field, expires_ms, duration));
            }
        }
        if let Some(sd) = schemas {
            for (k, v) in sd.iter() {
                let type_name: String = k.extract()?;
                let fd: &Bound<'_, PyDict> = v.downcast()?;
                let mut fields = Vec::new();
                let mut nullable = 0u64;
                for (fk, fv) in fd.iter() {
                    let fname: String = fk.extract()?;
                    let spec = fv.extract::<String>()?;
                    let (ft, is_nullable) = parse_subset_type(&type_name, &fname, &spec)?;
                    if is_nullable {
                        nullable |= 1u64 << fields.len();
                    }
                    fields.push((fname, ft));
                }
                reject_reserved_fields(&type_name, &fields)?;
                sess.schemas.push(TypeSchema { name: type_name, fields, nullable });
            }
        }
        if let Some(f) = facts {
            // Pass 1: schemas from every table, so cross-type rules
            // compile regardless of dict order.
            let mut pending: Vec<(String, Vec<(String, FieldType)>, Vec<Vec<Value>>)> = Vec::new();
            for (k, v) in f.iter() {
                let type_name: String = k.extract()?;
                let declared = sess.schemas.iter().find(|s| s.name == type_name).cloned();
                let (fields, cols) = ingest_any(py, &type_name, &v, declared.as_ref())?;
                match sess.schemas.iter().find(|s| s.name == type_name) {
                    Some(declared) if declared.fields != fields => {
                        return Err(PyValueError::new_err(format!(
                            "{type_name}: table schema differs from the declared schema \
                             (declared {:?}, table {:?})",
                            declared.fields, fields
                        )))
                    }
                    Some(_) => {}
                    None => sess.schemas.push(TypeSchema {
                        name: type_name.clone(),
                        fields: fields.clone(),
                        // inferred schemas stay non-nullable (D-044
                        // loud rejection); nullable/decimal need a
                        // declared schema (@seine.fact / schemas=)
                        nullable: 0,
                    }),
                }
                pending.push((type_name, fields, cols));
            }
            sess.ensure_built()?;
            for (type_name, fields, cols) in pending {
                sess.insert_columns(&type_name, &fields, cols)?;
            }
            // constructor handles are discarded; use insert()/insert_row()
            // return values for provenance
        } else if !sess.schemas.is_empty() {
            sess.ensure_built()?;
        }
        Ok(sess)
    }

    /// Insert more rows: an Arrow table or a dict of column lists.
    /// Returns the new facts' HANDLES (insertion order). The type must
    /// already be known to the session.
    fn insert(&mut self, py: Python<'_>, type_name: String, data: Bound<'_, PyAny>) -> PyResult<Vec<i64>> {
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
        let declared_schema = self.schemas.iter().find(|s| s.name == type_name).unwrap().clone();
        let (fields, cols) = ingest_any(py, &type_name, &data, Some(&declared_schema))?;
        let declared = &declared_schema.fields;
        if &fields != declared {
            return Err(PyValueError::new_err(format!(
                "{type_name}: schema mismatch with the declaring table"
            )));
        }
        self.insert_columns(&type_name, &fields, cols)
    }

    /// Insert a single fact from keyword-style dict (REPL convenience;
    /// same bulk path, batch of one). Returns the new fact's HANDLE.
    fn insert_row(&mut self, type_name: String, row: Bound<'_, PyDict>) -> PyResult<i64> {
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
            .clone();
        let nullable_mask = declared.nullable;
        let declared = declared.fields;
        // unknown keys were the one schema violation this path accepted
        // silently — a typo'd field name must not vanish into a
        // defaulted-looking insert
        for (k, _) in row.iter() {
            let k: String = k.extract()?;
            if !declared.iter().any(|(f, _)| f == &k) {
                return Err(PyValueError::new_err(format!(
                    "{type_name}: unknown field {k} (declared fields: {})",
                    declared
                        .iter()
                        .map(|(f, _)| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
        let mut vals: Vec<(String, Value)> = Vec::new();
        for (fi, (fname, ft)) in declared.iter().enumerate() {
            let item = row
                .get_item(fname)?
                .ok_or_else(|| PyValueError::new_err(format!("{type_name}: missing field {fname}")))?;
            let mut v =
                py_scalar(&type_name, fname, &item, Some((*ft, nullable_mask >> fi & 1 == 1)))?;
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
        Ok(id.0 as i64)
    }

    /// EXTERNAL update by handle: set the given fields and propagate
    /// with the changed-fields property mask. Handles come from result
    /// tables' `handle` column. Composes with other external actions
    /// in session-action order (certified).
    #[pyo3(signature = (handle, **fields))]
    fn update(
        &mut self,
        handle: i64,
        fields: Option<Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        self.require_fired("update()")?;
        let fields = fields
            .filter(|d| !d.is_empty())
            .ok_or_else(|| PyValueError::new_err("update: no fields given"))?;
        let tname = engine.fact_type_name(seine_engine::FactId(handle as u32));
        let schema = self.schemas.iter().find(|s| Some(s.name.as_str()) == tname.as_deref()).cloned();
        let mut vals: Vec<(String, Value)> = Vec::new();
        for (k, v) in fields.iter() {
            let name: String = k.extract()?;
            let target = schema.as_ref().and_then(|sch| {
                sch.fields
                    .iter()
                    .position(|(n, _)| n == &name)
                    .map(|i| (sch.fields[i].1, sch.nullable >> i & 1 == 1))
            });
            vals.push((name.clone(), py_scalar("update", &name, &v, target)?));
        }
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        // same TMS-cascade capture as delete(): an update that breaks a
        // justification can retract facts synchronously
        let pre: std::collections::HashSet<u32> =
            engine.facts().iter().map(|f| f.handle).collect();
        engine
            .update_fact(seine_engine::FactId(handle as u32), vals)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let post: std::collections::HashSet<u32> =
            engine.facts().iter().map(|f| f.handle).collect();
        let mut cascade: Vec<i64> = pre
            .iter()
            .filter(|h| !post.contains(h) && **h as i64 != handle)
            .map(|&h| h as i64)
            .collect();
        cascade.sort_unstable();
        self.pending_retracted.extend(cascade.iter().copied());
        Ok(())
    }

    /// EXTERNAL delete by handle.
    /// CEP: advance the pseudo-clock by ms.
    fn advance(&mut self, ms: i64) -> PyResult<()> {
        self.ensure_built()?;
        self.require_fired("advance()")?;
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        engine
            .advance(ms)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Run a DRL query against current working memory (direct
    /// invocation — the `session.getQueryResults` surface). Positional
    /// args follow the query's parameter list; pass None for an UNBOUND
    /// parameter (Drools `Variable.v`) — its bindings come back in the
    /// rows. Rows return in the certified order, as dicts keyed by the
    /// query's identifiers: fact values as {"type", "handle", fields...},
    /// scalars as plain Python values, or-branch-unbound as None.
    #[pyo3(signature = (name, *args))]
    fn query(
        &mut self,
        py: Python<'_>,
        name: String,
        args: Bound<'_, pyo3::types::PyTuple>,
    ) -> PyResult<Vec<Py<PyDict>>> {
        self.ensure_built()?;
        self.require_fired("query()")?;
        let mut qargs: Vec<Option<Value>> = Vec::with_capacity(args.len());
        for a in args.iter() {
            if a.is_none() {
                qargs.push(None);
                continue;
            }
            if let Ok(b) = a.downcast::<pyo3::types::PyBool>() {
                qargs.push(Some(Value::Bool(b.is_true())));
                continue;
            }
            if let Ok(n) = a.extract::<i64>() {
                qargs.push(Some(Value::I64(n)));
                continue;
            }
            if a.downcast::<pyo3::types::PyInt>().is_ok() {
                return Err(PyValueError::new_err(format!(
                    "query arg {a} does not fit a 64-bit signed integer"
                )));
            }
            if let Ok(f) = a.extract::<f64>() {
                qargs.push(Some(Value::F64(f)));
                continue;
            }
            if let Ok(t) = a.extract::<String>() {
                qargs.push(Some(Value::Str(t)));
                continue;
            }
            return Err(PyTypeError::new_err(format!(
                "query args are int/float/str/bool or None (unbound), got {}",
                a.get_type().name()?
            )));
        }
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        let out = engine
            .run_query(&name, &qargs)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mut rows: Vec<Py<PyDict>> = Vec::with_capacity(out.rows.len());
        for row in &out.rows {
            let d = PyDict::new(py);
            for (ident, v) in out.identifiers.iter().zip(row) {
                let obj: PyObject = match v {
                    seine_engine::QueryVal::Null => py.None(),
                    seine_engine::QueryVal::Scalar(sv) => value_to_py(py, sv)?,
                    seine_engine::QueryVal::Fact(fv) => {
                        let fd = PyDict::new(py);
                        fd.set_item("type", &fv.type_name)?;
                        if fv.handle != u32::MAX {
                            fd.set_item("handle", fv.handle as i64)?;
                        }
                        for (fname, fval) in &fv.fields {
                            fd.set_item(fname, value_to_py(py, fval)?)?;
                        }
                        fd.into()
                    }
                };
                d.set_item(ident, obj)?;
            }
            rows.push(d.into());
        }
        Ok(rows)
    }

    /// In-place session reset for paged batches — clears WM,
    /// agenda, TMS, clock and handle numbering; keeps rules/queries.
    fn reset(&mut self) -> PyResult<()> {
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        self.pending_retracted.clear();
        self.fired_once = false;
        engine.reset().map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn delete(&mut self, handle: i64) -> PyResult<Vec<i64>> {
        self.require_fired("delete()")?;
        let engine = self
            .engine
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("session has no declared types"))?;
        // TMS: deleting a premise synchronously retracts the facts it
        // justified. Capture that cascade so it reaches the WM-delta —
        // returned here AND merged into the next fire's deleted_handles.
        let pre: std::collections::HashSet<u32> =
            engine.facts().iter().map(|f| f.handle).collect();
        engine
            .delete_fact(seine_engine::FactId(handle as u32))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let post: std::collections::HashSet<u32> =
            engine.facts().iter().map(|f| f.handle).collect();
        let mut cascade: Vec<i64> = pre
            .iter()
            .filter(|h| !post.contains(h) && **h as i64 != handle)
            .map(|&h| h as i64)
            .collect();
        cascade.sort_unstable();
        self.pending_retracted.extend(cascade.iter().copied());
        Ok(cascade)
    }

    /// Run the rules to quiescence and return THIS fire's delta.
    /// Sessions are multi-fire: insert more facts afterwards and
    /// fire again. `on_fire(rule, matches)` is an OBSERVER invoked per
    /// firing after the run completes, in firing order; matches is a
    /// list of (type, handle) pairs. The run itself releases the GIL.
    #[pyo3(signature = (fire_limit=100_000, on_fire=None))]
    fn fire(
        &mut self,
        py: Python<'_>,
        fire_limit: usize,
        on_fire: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyResult_> {
        self.ensure_built()?;
        self.fired_once = true;
        let engine = self.engine.as_mut().expect("built");
        let pre_live: std::collections::HashSet<u32> =
            engine.facts().iter().map(|f| f.handle).collect();
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

        // Per-fire WM delta (D-046): derived = live-after minus
        // live-before (rule-inserted this fire; Python inserts between
        // fires are in the before-set); deleted = before minus after.
        let engine = self.engine.as_ref().expect("built");
        let live = engine.facts();
        let live_set: std::collections::HashSet<u32> = live.iter().map(|f| f.handle).collect();
        let mut facts: HashMap<String, RecordBatch> = HashMap::new();
        let mut derived: HashMap<String, RecordBatch> = HashMap::new();
        for schema in &self.schemas {
            let all: Vec<&FactView> =
                live.iter().filter(|f| f.type_name == schema.name).collect();
            let new: Vec<&FactView> = all
                .iter()
                .copied()
                .filter(|f| !pre_live.contains(&f.handle))
                .collect();
            facts.insert(schema.name.clone(), batch_for_type(schema, &all)?);
            derived.insert(schema.name.clone(), batch_for_type(schema, &new)?);
        }
        let mut deleted: Vec<i64> = pre_live
            .iter()
            .filter(|h| !live_set.contains(h))
            .map(|&h| h as i64)
            .collect();
        // engine-initiated retractions between fires (TMS cascades of
        // external delete/update) happen BEFORE this fire's before-set
        // was snapshotted — merge them so the WM-delta is complete
        deleted.extend(self.pending_retracted.drain(..));
        deleted.sort_unstable();
        deleted.dedup();

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

/// An optional-interop import failed: re-raise as ModuleNotFoundError
/// with the actionable install line. The wheel itself is
/// dependency-free — to_pylist() is the built-in read path; pyarrow
/// and polars are conveniences, and a raw ModuleNotFoundError at the
/// result-reading step reads as a packaging bug rather than a choice.
fn optional_dep_err(py: Python<'_>, e: PyErr, method: &str, package: &str, extra: &str) -> PyErr {
    if !e.is_instance_of::<pyo3::exceptions::PyImportError>(py) {
        return e;
    }
    let new = pyo3::exceptions::PyModuleNotFoundError::new_err(format!(
        "{method} requires {package}, which is not installed — \
         `pip install {package}` (or `pip install 'seine-rs[{extra}]'`). \
         to_pylist() works with no extra install."
    ));
    new.set_cause(py, Some(e));
    new
}

/// Result tables prepend the engine's fact handle under this column
/// name (see batch_for_type); a user field with the same name would
/// produce a duplicate column that collapses silently downstream.
fn reject_reserved_fields(type_name: &str, fields: &[(String, FieldType)]) -> PyResult<()> {
    if fields.iter().any(|(n, _)| n == "handle") {
        return Err(PyValueError::new_err(format!(
            "{type_name}.handle: the field name \"handle\" is reserved — result \
             tables carry the engine's fact handle in a column of that name; \
             rename the field"
        )));
    }
    Ok(())
}

/// Ingest either an Arrow-stream-capable object or a dict of lists.
pub(crate) fn ingest_any(
    _py: Python<'_>,
    type_name: &str,
    obj: &Bound<'_, PyAny>,
    declared: Option<&TypeSchema>,
) -> PyResult<(Vec<(String, FieldType)>, Vec<Vec<Value>>)> {
    let target_of = |fname: &str| -> Option<(FieldType, bool)> {
        declared.and_then(|d| {
            d.fields
                .iter()
                .position(|(n, _)| n == fname)
                .map(|i| (d.fields[i].1, d.nullable >> i & 1 == 1))
        })
    };
    if let Ok(d) = obj.downcast::<PyDict>() {
        let (fields, cols) = columns_from_dict(type_name, d, &target_of)?;
        reject_reserved_fields(type_name, &fields)?;
        return Ok((fields, cols));
    }
    let mut reader = import_stream(obj)?;
    let schema = reader.schema();
    let mut fields: Vec<(String, FieldType)> = Vec::new();
    for f in schema.fields() {
        // the DECLARED type wins (nullable/decimal round-trip through
        // the schema-equality check); inference covers the rest
        match target_of(f.name()) {
            Some((ft, _)) => fields.push((f.name().clone(), ft)),
            None => fields.push((f.name().clone(), map_dtype(type_name, f)?)),
        }
    }
    reject_reserved_fields(type_name, &fields)?;
    let mut cols: Vec<Vec<Value>> = vec![Vec::new(); fields.len()];
    while let Some(batch) = reader.next() {
        let batch = batch.map_err(|e| PyValueError::new_err(format!("arrow read: {e}")))?;
        for (ci, f) in schema.fields().iter().enumerate() {
            let vals = column_values(type_name, f, batch.column(ci), target_of(f.name()))?;
            cols[ci].extend(vals);
        }
    }
    Ok((fields, cols))
}

/// One-call convenience: build a session from tables, fire, return the
/// result. `seine.run(drl, {"P": df})`.
#[pyfunction]
#[pyo3(signature = (drl, facts, fire_limit=100_000, on_fire=None, schemas=None, events=None))]
fn run(
    py: Python<'_>,
    drl: String,
    facts: Bound<'_, PyDict>,
    fire_limit: usize,
    on_fire: Option<Bound<'_, PyAny>>,
    schemas: Option<Bound<'_, PyDict>>,
    events: Option<Bound<'_, PyDict>>,
) -> PyResult<PyResult_> {
    let mut sess = PySession::new(py, drl, Some(facts), schemas, events)?;
    sess.fire(py, fire_limit, on_fire)
}

/// The certification claim, interrogable at runtime: the Drools
/// oracle this build is differentially certified against, the
/// differential corpus it was built beside (the same directory globs
/// as the repo's `make diff` gate), the quarantined open-divergence
/// count, and the source commit. Numbers are stamped at build time;
/// wheels built outside the source tree stamp zeros/"unknown".
#[pyfunction]
fn certification(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("oracle", "Drools 9.44.0.Final (+ vendored upstream fix apache/incubator-kie-drools#6796)")?;
    d.set_item("engine_version", env!("CARGO_PKG_VERSION"))?;
    d.set_item("corpus_baseline", env!("SEINE_CORPUS_BASELINE").parse::<i64>().unwrap_or(0))?;
    d.set_item("corpus_probes", env!("SEINE_CORPUS_PROBES").parse::<i64>().unwrap_or(0))?;
    d.set_item("corpus_regressions", env!("SEINE_CORPUS_REGRESSIONS").parse::<i64>().unwrap_or(0))?;
    d.set_item("quarantine_xfail", env!("SEINE_CORPUS_XFAIL").parse::<i64>().unwrap_or(0))?;
    d.set_item("commit", env!("SEINE_GIT_COMMIT"))?;
    d.set_item(
        "scope",
        "certified = corpus_baseline + corpus_probes + corpus_regressions, \
byte-checked engine-vs-oracle by the repo's differential gate (make diff); \
quarantine_xfail = documented-open divergences (drift-tracked, NOT certified); \
excludes WIP recon instruments (probes_pending/)",
    )?;
    Ok(d)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyResult_>()?;
    m.add_class::<PyTable>()?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(certification, m)?)?;
    m.add_function(wrap_pyfunction!(derive::derive_haversine, m)?)?;
    m.add_function(wrap_pyfunction!(derive::derive_pair_candidates, m)?)?;
    m.add_function(wrap_pyfunction!(derive::derive_closing, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
