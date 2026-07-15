//! `seine_rs.derive` — derivation-plane kernels (docs/derivation-plane.md,
//! D-249 design, D-250 geometry).
//!
//! Pure columnar functions over Arrow data, upstream of the certified
//! match: the match grammar never grows arithmetic; these kernels
//! produce honest FIELDS that rules then constrain on. Their oracle is
//! an independent pure-python reference + property battery
//! (bindings/tests/test_derive.py) — the Drools oracle has no opinion
//! about column math, and nothing here touches engine/.
//!
//! Contract points carried over from the polars prototype
//! (demo/adsb_convergence.py) bit-compatibly, so the scenario twin
//! scenarios/demo/adsb_convergence.json survives:
//! - haversine: EARTH_R = 6_371_000.0, same operation order, meters
//!   rounded half-away-from-zero to Int64.
//! - pair_candidates: METRIC-space prune (round-27 findings): the lon
//!   delta WRAPS across the antimeridian, the lon threshold scales by
//!   cos(mean lat) clipped to 1e-6 and saturates to latitude-only at
//!   the poles (threshold capped at 180 degrees); `a < b` dedup.
//! - closing: state is the CALLER's dict (replay re-derives; nothing
//!   hides in module globals), entries carry the epoch timestamp and
//!   are swept by TTL at the top of every call — eviction is a pure
//!   function of the raw epoch sequence, so WAL-replay determinism is
//!   preserved.

use std::sync::Arc;

use arrow_array::builder::{BooleanBuilder, Float64Builder, Int64Builder, StringBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};

use seine_engine::{FieldType, Value};

use crate::{ingest_any, PyTable};

/// Demo-identical sphere radius (the twin's derived values depend on it).
const EARTH_R: f64 = 6_371_000.0; // meters

fn col_index(label: &str, fields: &[(String, FieldType)], name: &str) -> PyResult<usize> {
    fields.iter().position(|(n, _)| n == name).ok_or_else(|| {
        PyValueError::new_err(format!(
            "{label}: missing column {name:?} (columns: {})",
            fields
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

fn f64_col(label: &str, name: &str, col: &[Value]) -> PyResult<Vec<f64>> {
    col.iter()
        .map(|v| match v {
            Value::F64(x) => Ok(*x),
            Value::I64(n) => Ok(*n as f64),
            other => Err(PyTypeError::new_err(format!(
                "{label}.{name}: expected a numeric column, got {other:?}"
            ))),
        })
        .collect()
}

/// Coordinate columns reject non-finite values LOUDLY (round 28, D3):
/// a NaN latitude would slide through the candidate prune (comparison
/// polarity admits NaN) and the Int64 cast would turn the NaN distance
/// into 0 meters — garbage upstream data becoming the strongest
/// possible convergence signal. Same doctrine as the D-044 null
/// rejection: clean the feed first.
fn coord_col(label: &str, name: &str, col: &[Value]) -> PyResult<Vec<f64>> {
    let xs = f64_col(label, name, col)?;
    if let Some(row) = xs.iter().position(|x| !x.is_finite()) {
        return Err(PyValueError::new_err(format!(
            "{label}.{name}: non-finite coordinate (NaN/inf) at row {row} — \
             coordinates are outside the kernel contract; drop or fill them first"
        )));
    }
    Ok(xs)
}

fn reject_out_collision(
    label: &str,
    fields: &[(String, FieldType)],
    out: &str,
) -> PyResult<()> {
    if fields.iter().any(|(n, _)| n == out) {
        return Err(PyValueError::new_err(format!(
            "{label}: output column {out:?} already exists in the input — pass out=..."
        )));
    }
    Ok(())
}

/// One ingested Value column as an Arrow array (derive v1 subset:
/// i64/f64/bool/utf8 — no nulls reach here; ingestion already rejected
/// them loudly, D-044).
fn array_of(label: &str, name: &str, ft: FieldType, col: &[Value]) -> PyResult<(DataType, ArrayRef)> {
    let drift = |v: &Value| {
        PyTypeError::new_err(format!("{label}.{name}: column type drift ({v:?} in a {ft:?} column)"))
    };
    Ok(match ft {
        FieldType::I64 => {
            let mut b = Int64Builder::with_capacity(col.len());
            for v in col {
                match v {
                    Value::I64(n) => b.append_value(*n),
                    other => return Err(drift(other)),
                }
            }
            (DataType::Int64, Arc::new(b.finish()) as ArrayRef)
        }
        FieldType::F64 => {
            let mut b = Float64Builder::with_capacity(col.len());
            for v in col {
                match v {
                    Value::F64(x) => b.append_value(*x),
                    Value::I64(n) => b.append_value(*n as f64),
                    other => return Err(drift(other)),
                }
            }
            (DataType::Float64, Arc::new(b.finish()) as ArrayRef)
        }
        FieldType::Bool => {
            let mut b = BooleanBuilder::with_capacity(col.len());
            for v in col {
                match v {
                    Value::Bool(x) => b.append_value(*x),
                    other => return Err(drift(other)),
                }
            }
            (DataType::Boolean, Arc::new(b.finish()) as ArrayRef)
        }
        FieldType::Str => {
            let mut b = StringBuilder::new();
            for v in col {
                match v {
                    Value::Str(s) => b.append_value(s),
                    other => return Err(drift(other)),
                }
            }
            (DataType::Utf8, Arc::new(b.finish()) as ArrayRef)
        }
        FieldType::Dec { .. } => {
            return Err(PyTypeError::new_err(format!(
                "{label}.{name}: decimal columns are outside the derive v1 subset — cast first"
            )))
        }
    })
}

/// Input columns + appended computed columns -> one-batch Table.
fn batch_with(
    label: &str,
    fields: &[(String, FieldType)],
    cols: &[Vec<Value>],
    extra: Vec<(String, DataType, ArrayRef)>,
) -> PyResult<PyTable> {
    let mut schema_fields: Vec<Field> = Vec::with_capacity(fields.len() + extra.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(fields.len() + extra.len());
    for (ci, (name, ft)) in fields.iter().enumerate() {
        let (dt, arr) = array_of(label, name, *ft, &cols[ci])?;
        schema_fields.push(Field::new(name, dt, false));
        arrays.push(arr);
    }
    for (name, dt, arr) in extra {
        schema_fields.push(Field::new(&name, dt, false));
        arrays.push(arr);
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(schema_fields)), arrays)
        .map_err(|e| PyValueError::new_err(format!("{label}: arrow batch build failed: {e}")))?;
    Ok(PyTable { batch })
}

/// The demo kernel's exact operation order (bit-compatibility with the
/// polars stage keeps the two implementations in lockstep to the meter).
fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> i64 {
    let p1 = lat1.to_radians();
    let p2 = lat2.to_radians();
    let dp = (lat2 - lat1).to_radians() / 2.0;
    let dl = (lon2 - lon1).to_radians() / 2.0;
    let sp = dp.sin();
    let sl = dl.sin();
    let h = sp * sp + p1.cos() * p2.cos() * sl * sl;
    let d = 2.0 * EARTH_R * h.sqrt().asin();
    d.round() as i64 // round half away from zero, like the demo's .round(0)
}

/// Columnar haversine: (lat1, lon1, lat2, lon2) columns (Float64; ints
/// widen exactly) -> the input columns plus `out` (Int64 meters,
/// rounded half away from zero). Great-circle distance on the
/// EARTH_R = 6_371_000 m sphere.
#[pyfunction]
#[pyo3(signature = (data, lat1="lat1", lon1="lon1", lat2="lat2", lon2="lon2", out="dist_m"))]
pub(crate) fn derive_haversine(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    lat1: &str,
    lon1: &str,
    lat2: &str,
    lon2: &str,
    out: &str,
) -> PyResult<PyTable> {
    const LABEL: &str = "derive.haversine";
    let (fields, cols) = ingest_any(py, LABEL, data, None)?;
    reject_out_collision(LABEL, &fields, out)?;
    let la1 = coord_col(LABEL, lat1, &cols[col_index(LABEL, &fields, lat1)?])?;
    let lo1 = coord_col(LABEL, lon1, &cols[col_index(LABEL, &fields, lon1)?])?;
    let la2 = coord_col(LABEL, lat2, &cols[col_index(LABEL, &fields, lat2)?])?;
    let lo2 = coord_col(LABEL, lon2, &cols[col_index(LABEL, &fields, lon2)?])?;
    let mut b = Int64Builder::with_capacity(la1.len());
    for i in 0..la1.len() {
        b.append_value(haversine_m(la1[i], lo1[i], la2[i], lo2[i]));
    }
    batch_with(
        LABEL,
        &fields,
        &cols,
        vec![(out.to_string(), DataType::Int64, Arc::new(b.finish()) as ArrayRef)],
    )
}

/// Candidate pairs from one position table (id/lat/lon columns):
/// `a < b` dedup over the cross join, then a SOUND metric-space prune
/// (round 28, D1/D2 — supersedes the D-250 geometry): the contract is
/// completeness — NO pair whose true haversine distance is <= radius_m
/// is ever dropped (a prune, not the exact test; false positives are
/// fine and filtered by `haversine`). With theta = radius_m/EARTH_R
/// (the radius as a central angle):
/// - latitude: |dlat| <= theta (exact meridian rate — the old 111320
///   constant was ~0.11% tight and falsely pruned a thin
///   within-radius shell at every latitude);
/// - over-the-pole admission: a pair whose colatitude sum (toward
///   either pole) is <= theta is reachable across the pole and admits
///   regardless of longitude (the old cos(lat)-scaled test pruned
///   same-latitude/opposite-meridian convergence geometry outright);
/// - longitude: skipped when the radius cap can reach a pole
///   (max|lat| + theta >= 90deg — no lon bound is sound there);
///   otherwise the spherical-cap bound wrapped_dlon <=
///   asin(sin theta / cos(max|lat|)), which is exact for a cap and
///   strictly wider than the old parallel-arc scaling (whose
///   great-circle-undercuts-the-parallel error grows as the square of
///   the threshold angle — a band below the over-the-pole zone was
///   falsely pruned too).
/// Comparisons are inclusive with an fp-slack epsilon; NaN/inf
/// coordinates are rejected loudly (D3). NOTE: ids must be unique —
/// the a<b dedup means a duplicated id never pairs with itself.
/// Output columns: {id}_a, {lat}_a, {lon}_a, {id}_b, {lat}_b,
/// {lon}_b, key ("{a}|{b}"). Row order is the cross-join order
/// (a-major), matching the retired polars prototype.
#[pyfunction]
#[pyo3(signature = (data, id="id", lat="lat", lon="lon", radius_m=25_000.0))]
pub(crate) fn derive_pair_candidates(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    id: &str,
    lat: &str,
    lon: &str,
    radius_m: f64,
) -> PyResult<PyTable> {
    const LABEL: &str = "derive.pair_candidates";
    const EPS: f64 = 1e-12;
    let (fields, cols) = ingest_any(py, LABEL, data, None)?;
    let idi = col_index(LABEL, &fields, id)?;
    let id_ft = fields[idi].1;
    if !matches!(id_ft, FieldType::Str | FieldType::I64) {
        return Err(PyTypeError::new_err(format!(
            "{LABEL}.{id}: id column must be utf8 or int64, got {id_ft:?}"
        )));
    }
    let ids = &cols[idi];
    let lats = coord_col(LABEL, lat, &cols[col_index(LABEL, &fields, lat)?])?;
    let lons = coord_col(LABEL, lon, &cols[col_index(LABEL, &fields, lon)?])?;
    let n = ids.len();

    let theta = radius_m / EARTH_R; // central angle, radians
    let half_pi = std::f64::consts::FRAC_PI_2;
    let mut sel: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let a_lt_b = match (&ids[i], &ids[j]) {
                (Value::Str(a), Value::Str(b)) => a < b,
                (Value::I64(a), Value::I64(b)) => a < b,
                _ => false, // uniform column type guaranteed by ingestion
            };
            if !a_lt_b {
                continue;
            }
            let la = lats[i].to_radians();
            let lb = lats[j].to_radians();
            if (la - lb).abs() > theta + EPS {
                continue;
            }
            // over-the-pole reachability: sum of colatitudes toward
            // the nearer pole
            let colat_sum = ((half_pi - la) + (half_pi - lb))
                .min((half_pi + la) + (half_pi + lb));
            if colat_sum > theta + EPS {
                let phi_m = la.abs().max(lb.abs());
                // lon prune is only sound while the radius cap stays
                // clear of the pole
                if phi_m + theta < half_pi {
                    let raw_dlon = (lons[i] - lons[j]).abs() % 360.0;
                    let wrapped = raw_dlon.min(360.0 - raw_dlon).to_radians();
                    let dmax = (theta.sin() / phi_m.cos()).min(1.0).asin();
                    if wrapped > dmax + EPS {
                        continue;
                    }
                }
            }
            sel.push((i, j));
        }
    }

    let id_str = |v: &Value| match v {
        Value::Str(s) => s.clone(),
        Value::I64(x) => x.to_string(),
        _ => unreachable!(),
    };
    let id_array = |pick: &dyn Fn(&(usize, usize)) -> usize| -> (DataType, ArrayRef) {
        match id_ft {
            FieldType::Str => {
                let mut b = StringBuilder::new();
                for p in &sel {
                    b.append_value(id_str(&ids[pick(p)]));
                }
                (DataType::Utf8, Arc::new(b.finish()) as ArrayRef)
            }
            _ => {
                let mut b = Int64Builder::with_capacity(sel.len());
                for p in &sel {
                    match &ids[pick(p)] {
                        Value::I64(x) => b.append_value(*x),
                        _ => unreachable!(),
                    }
                }
                (DataType::Int64, Arc::new(b.finish()) as ArrayRef)
            }
        }
    };
    let f64_array = |src: &[f64], pick: &dyn Fn(&(usize, usize)) -> usize| -> ArrayRef {
        let mut b = Float64Builder::with_capacity(sel.len());
        for p in &sel {
            b.append_value(src[pick(p)]);
        }
        Arc::new(b.finish())
    };
    let mut key_b = StringBuilder::new();
    for (i, j) in &sel {
        key_b.append_value(format!("{}|{}", id_str(&ids[*i]), id_str(&ids[*j])));
    }
    let (id_dt, ida) = id_array(&|p: &(usize, usize)| p.0);
    let (_, idb) = id_array(&|p: &(usize, usize)| p.1);
    let out_fields = vec![
        Field::new(format!("{id}_a"), id_dt.clone(), false),
        Field::new(format!("{lat}_a"), DataType::Float64, false),
        Field::new(format!("{lon}_a"), DataType::Float64, false),
        Field::new(format!("{id}_b"), id_dt, false),
        Field::new(format!("{lat}_b"), DataType::Float64, false),
        Field::new(format!("{lon}_b"), DataType::Float64, false),
        Field::new("key", DataType::Utf8, false),
    ];
    let arrays: Vec<ArrayRef> = vec![
        ida,
        f64_array(&lats, &|p: &(usize, usize)| p.0),
        f64_array(&lons, &|p: &(usize, usize)| p.0),
        idb,
        f64_array(&lats, &|p: &(usize, usize)| p.1),
        f64_array(&lons, &|p: &(usize, usize)| p.1),
        Arc::new(key_b.finish()),
    ];
    let batch = RecordBatch::try_new(Arc::new(Schema::new(out_fields)), arrays)
        .map_err(|e| PyValueError::new_err(format!("{LABEL}: arrow batch build failed: {e}")))?;
    Ok(PyTable { batch })
}

/// Stateful decreasing-distance flag keyed by `key`, with TTL'd state.
/// `state` is the CALLER's dict (key -> (dist, epoch_ts)) — replay
/// re-derives by rebuilding it from the raw epoch sequence; nothing is
/// hidden in module globals. Every call sweeps entries older than
/// ttl_ms FIRST (state hygiene is part of the epoch function, D-250),
/// so call it once per epoch even when the batch is empty. Within a
/// batch, rows update the state in row order (a key seen twice
/// compares against its earlier row). Appends `out` (bool): true iff
/// the key was seen within the TTL horizon at a strictly greater
/// distance.
#[pyfunction]
#[pyo3(signature = (state, ts, data, key="key", dist="dist_m", ttl_ms=60_000, out="closing"))]
pub(crate) fn derive_closing(
    py: Python<'_>,
    state: &Bound<'_, PyDict>,
    ts: i64,
    data: &Bound<'_, PyAny>,
    key: &str,
    dist: &str,
    ttl_ms: i64,
    out: &str,
) -> PyResult<PyTable> {
    const LABEL: &str = "derive.closing";
    // TTL sweep first, unconditionally — eviction is a pure function of
    // the raw epoch sequence (WAL-replay determinism). Epochs must be
    // MONOTONIC (round 28, Q1): a backwards ts would silently compute
    // closing flags against future-stamped state, so it errors loudly
    // instead (deterministic on replay — same sequence, same error).
    let cutoff = ts - ttl_ms;
    let mut stale: Vec<PyObject> = Vec::new();
    for (k, v) in state.iter() {
        let (_, t): (f64, i64) = v.extract().map_err(|_| {
            PyValueError::new_err(format!(
                "{LABEL}: state values must be (dist, epoch_ts) tuples"
            ))
        })?;
        if t > ts {
            return Err(PyValueError::new_err(format!(
                "{LABEL}: epoch ts went backwards (state holds t={t} > ts={ts}) — \
                 epochs must be monotonic; rebuild the state for out-of-order replay"
            )));
        }
        if t < cutoff {
            stale.push(k.unbind());
        }
    }
    for k in stale {
        state.del_item(k)?;
    }

    let (fields, cols) = ingest_any(py, LABEL, data, None)?;
    reject_out_collision(LABEL, &fields, out)?;
    let ki = col_index(LABEL, &fields, key)?;
    if fields[ki].1 != FieldType::Str {
        return Err(PyTypeError::new_err(format!(
            "{LABEL}.{key}: key column must be utf8, got {:?}",
            fields[ki].1
        )));
    }
    let di = col_index(LABEL, &fields, dist)?;
    let dists = f64_col(LABEL, dist, &cols[di])?;
    let mut b = BooleanBuilder::with_capacity(dists.len());
    for (r, d) in dists.iter().enumerate() {
        let Value::Str(k) = &cols[ki][r] else { unreachable!() };
        let closing = match state.get_item(k.as_str())? {
            Some(prev) => {
                let (pd, _): (f64, i64) = prev.extract().map_err(|_| {
                    PyValueError::new_err(format!(
                        "{LABEL}: state values must be (dist, epoch_ts) tuples"
                    ))
                })?;
                *d < pd
            }
            None => false,
        };
        // store the column's native value back (ints stay ints)
        match &cols[di][r] {
            Value::I64(n) => state.set_item(k.as_str(), (*n, ts))?,
            _ => state.set_item(k.as_str(), (*d, ts))?,
        }
        b.append_value(closing);
    }
    batch_with(
        LABEL,
        &fields,
        &cols,
        vec![(out.to_string(), DataType::Boolean, Arc::new(b.finish()) as ArrayRef)],
    )
}
