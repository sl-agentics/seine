//! `seine_rs.derive` expression layer — the derivation plane's row-wise
//! column math (docs/derivation-plane.md; semantics pinned by
//! measurement in docs/derive-expr-pins.md).
//!
//! Python builds a CLOSED expression tree by operator overloading
//! (`col("price") * col("qty")`); this module walks the tree (plain
//! nested dicts — native literals, no serialization), typechecks it
//! upfront, and evaluates it over an Arrow RecordBatch with the pinned
//! arrow-rs compute kernels. Two entry points: `derive_with_columns`
//! (append named computed columns; expressions see the INPUT columns
//! only — polars semantics) and `derive_filter` (SQL WHERE: TRUE rows
//! pass, NULL rows drop).
//!
//! Semantics contract (the pins doc is the authority; highlights):
//! - NULLS PROPAGATE, SQL-style. This is a deliberate divergence from
//!   the match plane's ingestion (which rejects nulls loudly because
//!   the certified subset has no null semantics): the expression plane
//!   HAS null semantics — Kleene 3VL booleans, null-in→null-out
//!   elementwise — so nulls flow here and are only rejected later if
//!   inserted into a non-nullable engine field.
//! - Integer overflow, division by zero, domain errors (sqrt of a
//!   negative) and failed casts are LOUD ERRORS, never silent nulls.
//! - `/` is true division (always f64); `//` is integer-only truncating
//!   division (DuckDB/C; Python's floored `//` differs — documented);
//!   `%` follows the sign of the dividend (DuckDB/C; Python differs).
//! - f64→i64 casts round HALF-TO-EVEN with a range check (DuckDB).
//! - round(x, n) rounds the SHORTEST-DECIMAL representation of x,
//!   half away from zero (measured; agrees with f64::round at n=0).
//! - IEEE floats: NaN/inf are values and flow through arithmetic and
//!   comparisons (NaN != NaN).

use std::sync::Arc;

use arrow_array::builder::{BooleanBuilder, Float64Builder, Int64Builder, StringBuilder};
use arrow_array::types::{Float64Type, Int64Type};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Datum, Float64Array, Int64Array, RecordBatch,
    RecordBatchReader, Scalar, StringArray, UInt32Array,
};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};

use crate::{import_stream, PyTable};

const MAX_DEPTH: usize = 256;

// ---------------------------------------------------------------------
// The closed tree
// ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ty {
    I64,
    F64,
    Bool,
    Utf8,
}

impl Ty {
    fn name(self) -> &'static str {
        match self {
            Ty::I64 => "i64",
            Ty::F64 => "f64",
            Ty::Bool => "bool",
            Ty::Utf8 => "utf8",
        }
    }
    fn dtype(self) -> DataType {
        match self {
            Ty::I64 => DataType::Int64,
            Ty::F64 => DataType::Float64,
            Ty::Bool => DataType::Boolean,
            Ty::Utf8 => DataType::Utf8,
        }
    }
    fn numeric(self) -> bool {
        matches!(self, Ty::I64 | Ty::F64)
    }
}

#[derive(Clone, Debug)]
enum Lit {
    I64(i64),
    F64(f64),
    Bool(bool),
    Str(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Un {
    Neg,
    Not,
    IsNull,
    IsNotNull,
    Abs,
    Floor,
    Ceil,
    Sqrt,
    StrLen,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Bin {
    Add,
    Sub,
    Mul,
    TrueDiv,
    FloorDiv,
    Rem,
    Pow,
    Eq,
    Neq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Contains,
    StartsWith,
    EndsWith,
    Concat,
    FillNull,
}

#[derive(Clone, Debug)]
enum Ex {
    Col(String),
    Lit(Lit),
    Un(Un, Box<Ex>),
    Bin(Bin, Box<Ex>, Box<Ex>),
    IfElse(Box<Ex>, Box<Ex>, Box<Ex>),
    Cast(Box<Ex>, Ty),
    Round(Box<Ex>, i32),
}

const SUPPORTED_OPS: &str = "col, lit, add, sub, mul, div, floordiv, rem, pow, neg, \
     eq, neq, lt, lt_eq, gt, gt_eq, and, or, not, is_null, is_not_null, fill_null, \
     if_else, abs, floor, ceil, round, sqrt, cast, str_contains, str_starts_with, \
     str_ends_with, str_len, concat";

// ---------------------------------------------------------------------
// Parse: nested Python dicts -> Ex
// ---------------------------------------------------------------------

fn parse(label: &str, obj: &Bound<'_, PyAny>, depth: usize) -> PyResult<Ex> {
    if depth > MAX_DEPTH {
        return Err(PyValueError::new_err(format!(
            "{label}: expression nesting exceeds {MAX_DEPTH} — flatten the tree"
        )));
    }
    let d: &Bound<'_, PyDict> = obj.downcast().map_err(|_| {
        PyTypeError::new_err(format!(
            "{label}: expected an expression node (build expressions with \
             seine_rs.derive.col()/lit()), got {}",
            obj.get_type().name().map(|n| n.to_string()).unwrap_or_default()
        ))
    })?;
    let op: String = d
        .get_item("op")?
        .ok_or_else(|| PyValueError::new_err(format!("{label}: expression node missing 'op'")))?
        .extract()?;

    let args = |n: usize| -> PyResult<Vec<Bound<'_, PyAny>>> {
        let raw = d
            .get_item("args")?
            .ok_or_else(|| PyValueError::new_err(format!("{label}: op {op:?} missing 'args'")))?;
        let list: &Bound<'_, PyList> = raw.downcast().map_err(|_| {
            PyTypeError::new_err(format!("{label}: op {op:?} 'args' must be a list"))
        })?;
        if list.len() != n {
            return Err(PyValueError::new_err(format!(
                "{label}: op {op:?} takes {n} argument(s), got {}",
                list.len()
            )));
        }
        list.iter().map(Ok).collect()
    };
    let sub = |a: &Bound<'_, PyAny>| parse(label, a, depth + 1);

    let unary = |u: Un| -> PyResult<Ex> {
        let a = args(1)?;
        Ok(Ex::Un(u, Box::new(sub(&a[0])?)))
    };
    let binary = |b: Bin| -> PyResult<Ex> {
        let a = args(2)?;
        Ok(Ex::Bin(b, Box::new(sub(&a[0])?), Box::new(sub(&a[1])?)))
    };

    match op.as_str() {
        "col" => {
            let name: String = d
                .get_item("name")?
                .ok_or_else(|| PyValueError::new_err(format!("{label}: col node missing 'name'")))?
                .extract()?;
            Ok(Ex::Col(name))
        }
        "lit" => {
            let v = d
                .get_item("value")?
                .ok_or_else(|| PyValueError::new_err(format!("{label}: lit node missing 'value'")))?;
            Ok(Ex::Lit(parse_lit(label, &v)?))
        }
        "add" => binary(Bin::Add),
        "sub" => binary(Bin::Sub),
        "mul" => binary(Bin::Mul),
        "div" => binary(Bin::TrueDiv),
        "floordiv" => binary(Bin::FloorDiv),
        "rem" => binary(Bin::Rem),
        "pow" => binary(Bin::Pow),
        "eq" => binary(Bin::Eq),
        "neq" => binary(Bin::Neq),
        "lt" => binary(Bin::Lt),
        "lt_eq" => binary(Bin::LtEq),
        "gt" => binary(Bin::Gt),
        "gt_eq" => binary(Bin::GtEq),
        "and" => binary(Bin::And),
        "or" => binary(Bin::Or),
        "str_contains" => binary(Bin::Contains),
        "str_starts_with" => binary(Bin::StartsWith),
        "str_ends_with" => binary(Bin::EndsWith),
        "concat" => binary(Bin::Concat),
        "fill_null" => binary(Bin::FillNull),
        "neg" => unary(Un::Neg),
        "not" => unary(Un::Not),
        "is_null" => unary(Un::IsNull),
        "is_not_null" => unary(Un::IsNotNull),
        "abs" => unary(Un::Abs),
        "floor" => unary(Un::Floor),
        "ceil" => unary(Un::Ceil),
        "sqrt" => unary(Un::Sqrt),
        "str_len" => unary(Un::StrLen),
        "if_else" => {
            let a = args(3)?;
            Ok(Ex::IfElse(Box::new(sub(&a[0])?), Box::new(sub(&a[1])?), Box::new(sub(&a[2])?)))
        }
        "cast" => {
            let a = args(1)?;
            let to: String = d
                .get_item("to")?
                .ok_or_else(|| PyValueError::new_err(format!("{label}: cast node missing 'to'")))?
                .extract()?;
            let ty = match to.as_str() {
                "i64" => Ty::I64,
                "f64" => Ty::F64,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "{label}: cast target {other:?} is outside the v1 subset \
                         (supported: \"i64\", \"f64\")"
                    )))
                }
            };
            Ok(Ex::Cast(Box::new(sub(&a[0])?), ty))
        }
        "round" => {
            let a = args(1)?;
            let nd: i32 = match d.get_item("ndigits")? {
                Some(v) => v.extract().map_err(|_| {
                    PyTypeError::new_err(format!("{label}: round 'ndigits' must be an int"))
                })?,
                None => 0,
            };
            Ok(Ex::Round(Box::new(sub(&a[0])?), nd))
        }
        other => Err(PyValueError::new_err(format!(
            "{label}: unknown expression op {other:?} (supported: {SUPPORTED_OPS})"
        ))),
    }
}

fn parse_lit(label: &str, v: &Bound<'_, PyAny>) -> PyResult<Lit> {
    // bool before int: Python bool is an int subclass (house rule)
    if let Ok(b) = v.downcast::<pyo3::types::PyBool>() {
        return Ok(Lit::Bool(b.is_true()));
    }
    if v.downcast::<pyo3::types::PyInt>().is_ok() {
        let n: i64 = v.extract().map_err(|_| {
            PyValueError::new_err(format!(
                "{label}: int literal {v} does not fit i64 — the expression subset \
                 is 64-bit"
            ))
        })?;
        return Ok(Lit::I64(n));
    }
    if v.downcast::<pyo3::types::PyFloat>().is_ok() {
        let x: f64 = v.extract()?;
        if !x.is_finite() {
            return Err(PyValueError::new_err(format!(
                "{label}: non-finite float literal ({x}) — NaN/inf enter only as \
                 column DATA, never as literals; fill or filter the column instead"
            )));
        }
        return Ok(Lit::F64(x));
    }
    if let Ok(s) = v.extract::<String>() {
        return Ok(Lit::Str(s));
    }
    if v.is_none() {
        return Err(PyValueError::new_err(format!(
            "{label}: lit(None) has no type — use .is_null()/.is_not_null() to test \
             for nulls and .fill_null(x) to replace them"
        )));
    }
    Err(PyTypeError::new_err(format!(
        "{label}: unsupported literal type {} (supported: bool, int, float, str)",
        v.get_type().name().map(|n| n.to_string()).unwrap_or_default()
    )))
}

// ---------------------------------------------------------------------
// Pretty-printer for error messages
// ---------------------------------------------------------------------

fn render(ex: &Ex) -> String {
    fn bsym(b: Bin) -> &'static str {
        match b {
            Bin::Add => "+",
            Bin::Sub => "-",
            Bin::Mul => "*",
            Bin::TrueDiv => "/",
            Bin::FloorDiv => "//",
            Bin::Rem => "%",
            Bin::Pow => "**",
            Bin::Eq => "==",
            Bin::Neq => "!=",
            Bin::Lt => "<",
            Bin::LtEq => "<=",
            Bin::Gt => ">",
            Bin::GtEq => ">=",
            Bin::And => "&",
            Bin::Or => "|",
            Bin::Contains => ".str_contains",
            Bin::StartsWith => ".str_starts_with",
            Bin::EndsWith => ".str_ends_with",
            Bin::Concat => ".concat",
            Bin::FillNull => ".fill_null",
        }
    }
    match ex {
        Ex::Col(n) => format!("col({n:?})"),
        Ex::Lit(Lit::I64(v)) => v.to_string(),
        Ex::Lit(Lit::F64(v)) => format!("{v:?}"),
        Ex::Lit(Lit::Bool(v)) => if *v { "True" } else { "False" }.into(),
        Ex::Lit(Lit::Str(v)) => format!("{v:?}"),
        Ex::Un(u, a) => match u {
            Un::Neg => format!("(-{})", render(a)),
            Un::Not => format!("(~{})", render(a)),
            Un::IsNull => format!("{}.is_null()", render(a)),
            Un::IsNotNull => format!("{}.is_not_null()", render(a)),
            Un::Abs => format!("{}.abs()", render(a)),
            Un::Floor => format!("{}.floor()", render(a)),
            Un::Ceil => format!("{}.ceil()", render(a)),
            Un::Sqrt => format!("{}.sqrt()", render(a)),
            Un::StrLen => format!("{}.str_len()", render(a)),
        },
        Ex::Bin(b, l, r) => match b {
            Bin::Contains | Bin::StartsWith | Bin::EndsWith | Bin::Concat | Bin::FillNull => {
                format!("{}{}({})", render(l), bsym(*b), render(r))
            }
            _ => format!("({} {} {})", render(l), bsym(*b), render(r)),
        },
        Ex::IfElse(c, t, f) => {
            format!("if_else({}, {}, {})", render(c), render(t), render(f))
        }
        Ex::Cast(a, t) => format!("{}.cast({:?})", render(a), t.name()),
        Ex::Round(a, n) => format!("{}.round({n})", render(a)),
    }
}

// ---------------------------------------------------------------------
// Typecheck (upfront — errors before any kernel runs)
// ---------------------------------------------------------------------

struct Ctx {
    names: Vec<String>,
    types: Vec<Ty>,
    arrays: Vec<ArrayRef>,
    nrows: usize,
}

impl Ctx {
    fn lookup(&self, label: &str, name: &str) -> PyResult<usize> {
        self.names.iter().position(|n| n == name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "{label}: missing column {name:?} (columns: {})",
                self.names.join(", ")
            ))
        })
    }
}

/// Numeric promotion: i64⊕i64 → i64; any f64 → f64. None = not numeric.
fn promote(a: Ty, b: Ty) -> Option<Ty> {
    match (a, b) {
        (Ty::I64, Ty::I64) => Some(Ty::I64),
        (Ty::I64, Ty::F64) | (Ty::F64, Ty::I64) | (Ty::F64, Ty::F64) => Some(Ty::F64),
        _ => None,
    }
}

fn type_err(label: &str, ex: &Ex, detail: String) -> PyErr {
    PyTypeError::new_err(format!("{label}: in {} — {detail}", render(ex)))
}

fn check(label: &str, ex: &Ex, ctx: &Ctx) -> PyResult<Ty> {
    match ex {
        Ex::Col(name) => Ok(ctx.types[ctx.lookup(label, name)?]),
        Ex::Lit(Lit::I64(_)) => Ok(Ty::I64),
        Ex::Lit(Lit::F64(_)) => Ok(Ty::F64),
        Ex::Lit(Lit::Bool(_)) => Ok(Ty::Bool),
        Ex::Lit(Lit::Str(_)) => Ok(Ty::Utf8),
        Ex::Un(u, a) => {
            let t = check(label, a, ctx)?;
            match u {
                Un::Neg | Un::Abs => {
                    if !t.numeric() {
                        return Err(type_err(label, ex, format!("needs a numeric operand, got {}", t.name())));
                    }
                    Ok(t)
                }
                Un::Floor | Un::Ceil => {
                    if !t.numeric() {
                        return Err(type_err(label, ex, format!("needs a numeric operand, got {}", t.name())));
                    }
                    Ok(t) // floor/ceil of i64 is the identity
                }
                Un::Sqrt => {
                    if !t.numeric() {
                        return Err(type_err(label, ex, format!("needs a numeric operand, got {}", t.name())));
                    }
                    Ok(Ty::F64)
                }
                Un::Not => {
                    if t != Ty::Bool {
                        return Err(type_err(label, ex, format!("~ needs a boolean operand, got {}", t.name())));
                    }
                    Ok(Ty::Bool)
                }
                Un::IsNull | Un::IsNotNull => Ok(Ty::Bool),
                Un::StrLen => {
                    if t != Ty::Utf8 {
                        return Err(type_err(label, ex, format!(".str_len() needs a utf8 operand, got {}", t.name())));
                    }
                    Ok(Ty::I64)
                }
            }
        }
        Ex::Bin(b, l, r) => {
            let lt = check(label, l, ctx)?;
            let rt = check(label, r, ctx)?;
            match b {
                Bin::Add | Bin::Sub | Bin::Mul | Bin::Rem => promote(lt, rt).ok_or_else(|| {
                    let hint = if *b == Bin::Add && (lt == Ty::Utf8 || rt == Ty::Utf8) {
                        " (use .concat() for strings)"
                    } else {
                        ""
                    };
                    type_err(label, ex, format!(
                        "cannot apply to {} and {}{hint}", lt.name(), rt.name()
                    ))
                }),
                Bin::TrueDiv | Bin::Pow => {
                    promote(lt, rt).ok_or_else(|| {
                        type_err(label, ex, format!("cannot apply to {} and {}", lt.name(), rt.name()))
                    })?;
                    Ok(Ty::F64)
                }
                Bin::FloorDiv => {
                    if lt == Ty::I64 && rt == Ty::I64 {
                        Ok(Ty::I64)
                    } else {
                        Err(type_err(label, ex, format!(
                            "// is integer-only (got {} and {}) — use / and .floor() \
                             for float floor division",
                            lt.name(), rt.name()
                        )))
                    }
                }
                Bin::Eq | Bin::Neq => {
                    if promote(lt, rt).is_some() || lt == rt {
                        Ok(Ty::Bool)
                    } else {
                        Err(type_err(label, ex, format!("cannot compare {} and {}", lt.name(), rt.name())))
                    }
                }
                Bin::Lt | Bin::LtEq | Bin::Gt | Bin::GtEq => {
                    if promote(lt, rt).is_some() || (lt == Ty::Utf8 && rt == Ty::Utf8) {
                        Ok(Ty::Bool)
                    } else if lt == Ty::Bool && rt == Ty::Bool {
                        Err(type_err(label, ex, "booleans have no order — use == or !=".into()))
                    } else {
                        Err(type_err(label, ex, format!("cannot compare {} and {}", lt.name(), rt.name())))
                    }
                }
                Bin::And | Bin::Or => {
                    if lt == Ty::Bool && rt == Ty::Bool {
                        Ok(Ty::Bool)
                    } else {
                        Err(type_err(label, ex, format!(
                            "& and | need boolean operands (got {} and {}) — did a \
                             comparison lose a parenthesis?",
                            lt.name(), rt.name()
                        )))
                    }
                }
                Bin::Contains | Bin::StartsWith | Bin::EndsWith => {
                    if lt == Ty::Utf8 && rt == Ty::Utf8 {
                        Ok(Ty::Bool)
                    } else {
                        Err(type_err(label, ex, format!("needs utf8 operands, got {} and {}", lt.name(), rt.name())))
                    }
                }
                Bin::Concat => {
                    if lt == Ty::Utf8 && rt == Ty::Utf8 {
                        Ok(Ty::Utf8)
                    } else {
                        Err(type_err(label, ex, format!(".concat() needs utf8 operands, got {} and {}", lt.name(), rt.name())))
                    }
                }
                Bin::FillNull => {
                    if let Some(t) = promote(lt, rt) {
                        Ok(t)
                    } else if lt == rt {
                        Ok(lt)
                    } else {
                        Err(type_err(label, ex, format!(
                            ".fill_null() replacement type {} does not match {}", rt.name(), lt.name()
                        )))
                    }
                }
            }
        }
        Ex::IfElse(c, t, f) => {
            let ct = check(label, c, ctx)?;
            if ct != Ty::Bool {
                return Err(type_err(label, ex, format!("if_else condition must be boolean, got {}", ct.name())));
            }
            let tt = check(label, t, ctx)?;
            let ft = check(label, f, ctx)?;
            if let Some(p) = promote(tt, ft) {
                Ok(p)
            } else if tt == ft {
                Ok(tt)
            } else {
                Err(type_err(label, ex, format!(
                    "if_else branches must match, got {} and {}", tt.name(), ft.name()
                )))
            }
        }
        Ex::Cast(a, to) => {
            let t = check(label, a, ctx)?;
            if !t.numeric() {
                return Err(type_err(label, ex, format!(
                    ".cast() from {} is outside the v1 subset (numeric only)", t.name()
                )));
            }
            Ok(*to)
        }
        Ex::Round(a, nd) => {
            let t = check(label, a, ctx)?;
            if !t.numeric() {
                return Err(type_err(label, ex, format!("needs a numeric operand, got {}", t.name())));
            }
            if t == Ty::I64 && *nd != 0 {
                return Err(type_err(label, ex, format!(
                    ".round(ndigits={nd}) on i64 is outside v1 — .cast(\"f64\") first"
                )));
            }
            Ok(t)
        }
    }
}

// ---------------------------------------------------------------------
// Eval
// ---------------------------------------------------------------------

/// An evaluated node: a full column or a scalar (len-1 array used as an
/// arrow Datum — `col * lit(2)` never materializes an N-row literal).
enum Ev {
    Arr(ArrayRef),
    Scl(ArrayRef),
}

impl Ev {
    fn array(&self) -> &ArrayRef {
        match self {
            Ev::Arr(a) | Ev::Scl(a) => a,
        }
    }
    fn is_scalar(&self) -> bool {
        matches!(self, Ev::Scl(_))
    }
    /// Materialize to a full column of the batch's length.
    fn broadcast(self, label: &str, nrows: usize) -> PyResult<ArrayRef> {
        match self {
            Ev::Arr(a) => Ok(a),
            Ev::Scl(a) => {
                let idx = UInt32Array::from(vec![0u32; nrows]);
                arrow_select::take::take(a.as_ref(), &idx, None)
                    .map_err(|e| PyValueError::new_err(format!("{label}: broadcast failed: {e}")))
            }
        }
    }
}

fn wrap(l: &Ev, r: &Ev, out: ArrayRef) -> Ev {
    if l.is_scalar() && r.is_scalar() {
        Ev::Scl(out)
    } else {
        Ev::Arr(out)
    }
}

fn map_un(ev: Ev, out: ArrayRef) -> Ev {
    match ev {
        Ev::Arr(_) => Ev::Arr(out),
        Ev::Scl(_) => Ev::Scl(out),
    }
}

fn arrow_err(label: &str, ex: &Ex, e: ArrowError) -> PyErr {
    match &e {
        ArrowError::DivideByZero => PyValueError::new_err(format!(
            "{label}: division by zero in {} — guard with a comparison, or use \
             f64 operands for IEEE inf/NaN",
            render(ex)
        )),
        ArrowError::ArithmeticOverflow(_) => PyValueError::new_err(format!(
            "{label}: integer overflow in {} — .cast(\"f64\") an operand for IEEE \
             arithmetic",
            render(ex)
        )),
        _ => PyValueError::new_err(format!("{label}: {} failed: {e}", render(ex))),
    }
}

/// Datum-based binary kernel dispatch over the Ev pair.
fn bin_kernel(
    label: &str,
    ex: &Ex,
    l: &Ev,
    r: &Ev,
    f: impl Fn(&dyn Datum, &dyn Datum) -> Result<ArrayRef, ArrowError>,
) -> PyResult<Ev> {
    let run = |ld: &dyn Datum, rd: &dyn Datum| f(ld, rd).map_err(|e| arrow_err(label, ex, e));
    let out = match (l, r) {
        (Ev::Arr(a), Ev::Arr(b)) => run(a, b)?,
        (Ev::Arr(a), Ev::Scl(b)) => run(a, &Scalar::new(b.clone()))?,
        (Ev::Scl(a), Ev::Arr(b)) => run(&Scalar::new(a.clone()), b)?,
        (Ev::Scl(a), Ev::Scl(b)) => run(&Scalar::new(a.clone()), &Scalar::new(b.clone()))?,
    };
    Ok(wrap(l, r, out))
}

/// Comparison kernels return BooleanArray, not ArrayRef.
fn cmp_kernel(
    label: &str,
    ex: &Ex,
    l: &Ev,
    r: &Ev,
    f: impl Fn(&dyn Datum, &dyn Datum) -> Result<BooleanArray, ArrowError>,
) -> PyResult<Ev> {
    bin_kernel(label, ex, l, r, |a, b| f(a, b).map(|o| Arc::new(o) as ArrayRef))
}

fn f64_arr(ev: &Ev) -> &Float64Array {
    ev.array().as_any().downcast_ref::<Float64Array>().expect("typechecked f64")
}
fn i64_arr(ev: &Ev) -> &Int64Array {
    ev.array().as_any().downcast_ref::<Int64Array>().expect("typechecked i64")
}
fn bool_arr(ev: &Ev) -> &BooleanArray {
    ev.array().as_any().downcast_ref::<BooleanArray>().expect("typechecked bool")
}
fn str_arr(ev: &Ev) -> &StringArray {
    ev.array().as_any().downcast_ref::<StringArray>().expect("typechecked utf8")
}

/// Promote an evaluated numeric node to f64 if it is i64. Hand-rolled
/// (`x as f64` = round-to-nearest, identical to arrow's cast) — going
/// through arrow_cast::cast_with_options links its ENTIRE type-pair
/// matrix (~3MiB of .text; the wheel-size finding), for four trivial
/// conversions this module actually needs.
fn to_f64(_label: &str, _ex: &Ex, ev: Ev) -> PyResult<Ev> {
    if ev.array().data_type() == &DataType::Float64 {
        return Ok(ev);
    }
    let out = arrow_arith::arity::unary::<Int64Type, _, Float64Type>(
        i64_arr(&ev),
        |x| x as f64,
    );
    Ok(map_un(ev, Arc::new(out)))
}

/// Elementwise binary string op with null union and scalar broadcast
/// (replaces the arrow_string kernels, whose `like` module links the
/// whole regex family for predicates that never use it).
fn str_bin<T, B>(
    l: &Ev,
    r: &Ev,
    nrows: usize,
    mut push: impl FnMut(&mut B, &str, &str),
    mut builder: B,
    finish: impl FnOnce(B) -> T,
    mut push_null: impl FnMut(&mut B),
) -> Ev
where
    T: Array + 'static,
{
    let (la, ra) = (str_arr(l), str_arr(r));
    let both_scalar = l.is_scalar() && r.is_scalar();
    let n = if both_scalar { 1 } else { nrows };
    let idx = |ev: &Ev, i: usize| if ev.is_scalar() { 0 } else { i };
    for i in 0..n {
        let (li, ri) = (idx(l, i), idx(r, i));
        if la.is_null(li) || ra.is_null(ri) {
            push_null(&mut builder);
        } else {
            push(&mut builder, la.value(li), ra.value(ri));
        }
    }
    let out = Arc::new(finish(builder)) as ArrayRef;
    if both_scalar {
        Ev::Scl(out)
    } else {
        Ev::Arr(out)
    }
}

fn str_pred(l: &Ev, r: &Ev, nrows: usize, f: impl Fn(&str, &str) -> bool) -> Ev {
    str_bin(
        l,
        r,
        nrows,
        |b: &mut BooleanBuilder, x, y| b.append_value(f(x, y)),
        BooleanBuilder::new(),
        |mut b| b.finish(),
        |b| b.append_null(),
    )
}

/// round(x, n): round the SHORTEST-DECIMAL representation of x half away
/// from zero (docs/derive-expr-pins.md section E + ledger row 8 — the
/// measured DuckDB behavior; 2.675 -> 2.68 although the binary value is
/// 2.67499...). Agrees with f64::round at n=0. NaN/inf pass through.
fn round_shortest_decimal(x: f64, nd: i32) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let neg = x.is_sign_negative();
    let mag = x.abs();
    let s = format!("{mag:e}"); // shortest mantissa, "d.ddde<exp>"
    let (mant, exp) = s.split_once('e').expect("std {:e} format");
    let exp: i32 = exp.parse().expect("std {:e} exponent");
    let mut digits: Vec<u8> =
        mant.bytes().filter(|b| *b != b'.').map(|b| b - b'0').collect();
    // digits[i] sits at the 10^(exp - i) place; keep places >= 10^(-nd),
    // i.e. indices 0..=exp+nd.
    let keep = exp as i64 + nd as i64;
    let result_mag = if keep < -1 {
        0.0
    } else if keep == -1 {
        if digits[0] >= 5 {
            10f64.powi(-nd)
        } else {
            0.0
        }
    } else if keep as usize >= digits.len() - 1 {
        mag
    } else {
        let k = keep as usize;
        let up = digits[k + 1] >= 5;
        digits.truncate(k + 1);
        let mut exp = exp;
        if up {
            let mut i = k as i64;
            loop {
                if i < 0 {
                    digits.insert(0, 1);
                    exp += 1;
                    break;
                }
                if digits[i as usize] == 9 {
                    digits[i as usize] = 0;
                    i -= 1;
                } else {
                    digits[i as usize] += 1;
                    break;
                }
            }
        }
        let body: String = digits.iter().map(|d| (d + b'0') as char).collect();
        format!("{}.{}e{}", &body[..1], if body.len() > 1 { &body[1..] } else { "0" }, exp)
            .parse::<f64>()
            .expect("rebuilt decimal parses")
    };
    if neg {
        -result_mag
    } else {
        result_mag
    }
}

fn eval(label: &str, ex: &Ex, ctx: &Ctx) -> PyResult<Ev> {
    use arrow_arith::arity;
    match ex {
        Ex::Col(name) => Ok(Ev::Arr(ctx.arrays[ctx.lookup(label, name)?].clone())),
        Ex::Lit(l) => Ok(Ev::Scl(match l {
            Lit::I64(v) => Arc::new(Int64Array::from(vec![*v])) as ArrayRef,
            Lit::F64(v) => Arc::new(Float64Array::from(vec![*v])),
            Lit::Bool(v) => Arc::new(BooleanArray::from(vec![*v])),
            Lit::Str(v) => Arc::new(StringArray::from(vec![v.clone()])),
        })),
        Ex::Un(u, a) => {
            let at = check(label, a, ctx)?;
            let av = eval(label, a, ctx)?;
            match u {
                Un::Neg => {
                    let out = arrow_arith::numeric::neg(av.array().as_ref())
                        .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, out))
                }
                Un::Not => {
                    let out = arrow_arith::boolean::not(bool_arr(&av))
                        .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, Arc::new(out)))
                }
                Un::IsNull => {
                    let out = arrow_arith::boolean::is_null(av.array().as_ref())
                        .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, Arc::new(out)))
                }
                Un::IsNotNull => {
                    let out = arrow_arith::boolean::is_not_null(av.array().as_ref())
                        .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, Arc::new(out)))
                }
                Un::Abs => {
                    if at == Ty::I64 {
                        let out = arity::try_unary::<Int64Type, _, Int64Type>(
                            i64_arr(&av),
                            |x| {
                                x.checked_abs().ok_or_else(|| {
                                    ArrowError::ArithmeticOverflow(format!("abs({x})"))
                                })
                            },
                        )
                        .map_err(|e| arrow_err(label, ex, e))?;
                        Ok(map_un(av, Arc::new(out)))
                    } else {
                        let out =
                            arity::unary::<Float64Type, _, Float64Type>(f64_arr(&av), f64::abs);
                        Ok(map_un(av, Arc::new(out)))
                    }
                }
                Un::Floor | Un::Ceil => {
                    if at == Ty::I64 {
                        return Ok(av); // identity on integers
                    }
                    let f = if *u == Un::Floor { f64::floor } else { f64::ceil };
                    let out = arity::unary::<Float64Type, _, Float64Type>(f64_arr(&av), f);
                    Ok(map_un(av, Arc::new(out)))
                }
                Un::Sqrt => {
                    let av = to_f64(label, ex, av)?;
                    // sqrt of a negative is a loud DOMAIN error (pins §F:
                    // the oracle errors; a silent NaN would flow into
                    // comparisons as poison). NaN input propagates (IEEE).
                    let out = arity::try_unary::<Float64Type, _, Float64Type>(
                        f64_arr(&av),
                        |x| {
                            if x < 0.0 {
                                Err(ArrowError::ComputeError(format!(
                                    "square root of a negative ({x})"
                                )))
                            } else {
                                Ok(x.sqrt())
                            }
                        },
                    )
                    .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, Arc::new(out)))
                }
                Un::StrLen => {
                    // BYTE length (utf8), i64 out — matches the oracle's
                    // strlen(); hand-rolled to avoid arrow_string
                    let s = str_arr(&av);
                    let mut b = Int64Builder::with_capacity(s.len());
                    for i in 0..s.len() {
                        if s.is_null(i) {
                            b.append_null();
                        } else {
                            b.append_value(s.value(i).len() as i64);
                        }
                    }
                    Ok(map_un(av, Arc::new(b.finish())))
                }
            }
        }
        Ex::Bin(b, l, r) => {
            let lt = check(label, l, ctx)?;
            let rt = check(label, r, ctx)?;
            let mut lv = eval(label, l, ctx)?;
            let mut rv = eval(label, r, ctx)?;
            // numeric promotion (mirrors check/promote exactly)
            let promoted = promote(lt, rt);
            let want_f64 = matches!(b, Bin::TrueDiv | Bin::Pow)
                || (promoted == Some(Ty::F64)
                    && matches!(
                        b,
                        Bin::Add
                            | Bin::Sub
                            | Bin::Mul
                            | Bin::Rem
                            | Bin::Eq
                            | Bin::Neq
                            | Bin::Lt
                            | Bin::LtEq
                            | Bin::Gt
                            | Bin::GtEq
                            | Bin::FillNull
                    ));
            if want_f64 && promoted.is_some() {
                lv = to_f64(label, ex, lv)?;
                rv = to_f64(label, ex, rv)?;
            }
            match b {
                Bin::Add => bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::add(a, c)),
                Bin::Sub => bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::sub(a, c)),
                Bin::Mul => bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::mul(a, c)),
                // / promoted to f64 above: IEEE, never the int div0 arm
                Bin::TrueDiv => {
                    bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::div(a, c))
                }
                // // is i64-only (typecheck): arrow int div TRUNCATES,
                // matching the oracle (pins §A); div0/overflow error
                Bin::FloorDiv => {
                    bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::div(a, c))
                }
                Bin::Rem => bin_kernel(label, ex, &lv, &rv, |a, c| arrow_arith::numeric::rem(a, c)),
                Bin::Pow => {
                    // both f64 by promotion above
                    let out = match (&lv, &rv) {
                        (a, Ev::Scl(_)) => {
                            let p = pow_scalar(&rv);
                            match p {
                                Some(p) => Arc::new(arity::unary::<Float64Type, _, Float64Type>(
                                    f64_arr(a),
                                    move |x| x.powf(p),
                                )) as ArrayRef,
                                None => {
                                    // null scalar exponent -> all-null result
                                    Arc::new(Float64Array::new_null(a.array().len()))
                                }
                            }
                        }
                        (Ev::Scl(_), b) => {
                            let p = pow_scalar(&lv);
                            match p {
                                Some(p) => Arc::new(arity::unary::<Float64Type, _, Float64Type>(
                                    f64_arr(b),
                                    move |x| p.powf(x),
                                )) as ArrayRef,
                                None => Arc::new(Float64Array::new_null(b.array().len())),
                            }
                        }
                        (a, b) => Arc::new(
                            arity::binary::<Float64Type, Float64Type, _, Float64Type>(
                                f64_arr(a),
                                f64_arr(b),
                                f64::powf,
                            )
                            .map_err(|e| arrow_err(label, ex, e))?,
                        ),
                    };
                    Ok(wrap(&lv, &rv, out))
                }
                // Floats compare with the NATIVE IEEE operators, not
                // arrow_ord::cmp — the arrow kernels use totalOrder
                // (-0.0 != 0.0, NaN == NaN), which contradicts both the
                // oracle's value semantics at ±0 and the published IEEE
                // contract at NaN (pins ledger row 1).
                Bin::Eq | Bin::Neq | Bin::Lt | Bin::LtEq | Bin::Gt | Bin::GtEq
                    if lv.array().data_type() == &DataType::Float64 =>
                {
                    Ok(cmp_f64_ieee(*b, &lv, &rv, ctx.nrows))
                }
                Bin::Eq => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::eq(a, c)),
                Bin::Neq => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::neq(a, c)),
                Bin::Lt => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::lt(a, c)),
                Bin::LtEq => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::lt_eq(a, c)),
                Bin::Gt => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::gt(a, c)),
                Bin::GtEq => cmp_kernel(label, ex, &lv, &rv, |a, c| arrow_ord::cmp::gt_eq(a, c)),
                // Kleene kernels are BooleanArray-typed, not Datum:
                // broadcast scalar sides
                Bin::And | Bin::Or => {
                    let n = if lv.is_scalar() && rv.is_scalar() { 1 } else { ctx.nrows };
                    let scalar_out = lv.is_scalar() && rv.is_scalar();
                    let la = lv.broadcast(label, n)?;
                    let ra = rv.broadcast(label, n)?;
                    let (lb, rb) = (
                        la.as_any().downcast_ref::<BooleanArray>().expect("typechecked bool"),
                        ra.as_any().downcast_ref::<BooleanArray>().expect("typechecked bool"),
                    );
                    let out = if *b == Bin::And {
                        arrow_arith::boolean::and_kleene(lb, rb)
                    } else {
                        arrow_arith::boolean::or_kleene(lb, rb)
                    }
                    .map_err(|e| arrow_err(label, ex, e))?;
                    let out = Arc::new(out) as ArrayRef;
                    Ok(if scalar_out { Ev::Scl(out) } else { Ev::Arr(out) })
                }
                Bin::Contains => Ok(str_pred(&lv, &rv, ctx.nrows, |a, b| a.contains(b))),
                Bin::StartsWith => Ok(str_pred(&lv, &rv, ctx.nrows, |a, b| a.starts_with(b))),
                Bin::EndsWith => Ok(str_pred(&lv, &rv, ctx.nrows, |a, b| a.ends_with(b))),
                Bin::Concat => Ok(str_bin(
                    &lv,
                    &rv,
                    ctx.nrows,
                    |b: &mut StringBuilder, x, y| {
                        b.append_value(format!("{x}{y}"));
                    },
                    StringBuilder::new(),
                    |mut b| b.finish(),
                    |b| b.append_null(),
                )),
                Bin::FillNull => {
                    // zip(a.is_not_null(), a, b)
                    let n = if lv.is_scalar() && rv.is_scalar() { 1 } else { ctx.nrows };
                    let scalar_out = lv.is_scalar() && rv.is_scalar();
                    let la = lv.broadcast(label, n)?;
                    let mask = arrow_arith::boolean::is_not_null(la.as_ref())
                        .map_err(|e| arrow_err(label, ex, e))?;
                    let out = match &rv {
                        Ev::Scl(s) => arrow_select::zip::zip(&mask, &la, &Scalar::new(s.clone())),
                        Ev::Arr(a) => arrow_select::zip::zip(&mask, &la, a),
                    }
                    .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(if scalar_out { Ev::Scl(out) } else { Ev::Arr(out) })
                }
            }
        }
        Ex::IfElse(c, t, f) => {
            let tt = check(label, t, ctx)?;
            let ft = check(label, f, ctx)?;
            let cv = eval(label, c, ctx)?;
            let mut tv = eval(label, t, ctx)?;
            let mut fv = eval(label, f, ctx)?;
            if promote(tt, ft) == Some(Ty::F64) {
                tv = to_f64(label, ex, tv)?;
                fv = to_f64(label, ex, fv)?;
            }
            let all_scalar = cv.is_scalar() && tv.is_scalar() && fv.is_scalar();
            let n = if all_scalar { 1 } else { ctx.nrows };
            // A null condition takes the falsy side = SQL CASE (pins §G).
            // zip DOCUMENTS null->falsy but actually walks the mask's raw
            // VALUES buffer (SlicesIterator ignores validity) — a null
            // slot in a KERNEL-COMPUTED mask (~, ==, ...) can carry a
            // garbage bit and silently pick the then-branch. Rewrite
            // nulls to literal false first, honoring validity.
            let ca = cv.broadcast(label, n)?;
            let mask = ca.as_any().downcast_ref::<BooleanArray>().expect("typechecked bool");
            let prepped;
            let mask = if mask.null_count() > 0 {
                prepped = arrow_select::filter::prep_null_mask_filter(mask);
                &prepped
            } else {
                mask
            };
            let run = |t: &dyn Datum, f: &dyn Datum| {
                arrow_select::zip::zip(mask, t, f).map_err(|e| arrow_err(label, ex, e))
            };
            let out = match (&tv, &fv) {
                (Ev::Arr(a), Ev::Arr(b)) => run(a, b)?,
                (Ev::Arr(a), Ev::Scl(b)) => run(a, &Scalar::new(b.clone()))?,
                (Ev::Scl(a), Ev::Arr(b)) => run(&Scalar::new(a.clone()), b)?,
                (Ev::Scl(a), Ev::Scl(b)) => {
                    run(&Scalar::new(a.clone()), &Scalar::new(b.clone()))?
                }
            };
            Ok(if all_scalar { Ev::Scl(out) } else { Ev::Arr(out) })
        }
        Ex::Cast(a, to) => {
            let at = check(label, a, ctx)?;
            let av = eval(label, a, ctx)?;
            match (at, to) {
                (t, to) if t == *to => Ok(av),
                (Ty::I64, Ty::F64) => to_f64(label, ex, av),
                (Ty::F64, Ty::I64) => {
                    // The oracle casts DOUBLE->BIGINT by rounding HALF TO
                    // EVEN with a range check (pins §D: 2.5 -> 2, 3.5 -> 4;
                    // NaN/inf/out-of-range error) — arrow's cast truncates,
                    // so this one is hand-rolled.
                    let out = arity::try_unary::<Float64Type, _, Int64Type>(
                        f64_arr(&av),
                        |x| {
                            if !x.is_finite() {
                                return Err(ArrowError::CastError(format!(
                                    "{x} has no i64 value"
                                )));
                            }
                            let r = x.round_ties_even();
                            if r >= 9223372036854775808.0 || r < -9223372036854775808.0 {
                                return Err(ArrowError::CastError(format!(
                                    "{x} is out of range for i64"
                                )));
                            }
                            Ok(r as i64)
                        },
                    )
                    .map_err(|e| arrow_err(label, ex, e))?;
                    Ok(map_un(av, Arc::new(out)))
                }
                _ => unreachable!("typechecked cast"),
            }
        }
        Ex::Round(a, nd) => {
            let at = check(label, a, ctx)?;
            let av = eval(label, a, ctx)?;
            if at == Ty::I64 {
                return Ok(av); // ndigits==0 identity (typecheck rejects others)
            }
            let nd = *nd;
            let out = arity::unary::<Float64Type, _, Float64Type>(f64_arr(&av), move |x| {
                round_shortest_decimal(x, nd)
            });
            Ok(map_un(av, Arc::new(out)))
        }
    }
}

fn pow_scalar(ev: &Ev) -> Option<f64> {
    let a = f64_arr(ev);
    if a.is_null(0) {
        None
    } else {
        Some(a.value(0))
    }
}

/// Standard IEEE-754 f64 comparisons (Rust's native operators):
/// -0.0 == 0.0, NaN != anything including itself. Null propagates.
fn cmp_f64_ieee(op: Bin, l: &Ev, r: &Ev, nrows: usize) -> Ev {
    let f = |a: f64, b: f64| -> bool {
        match op {
            Bin::Eq => a == b,
            Bin::Neq => a != b,
            Bin::Lt => a < b,
            Bin::LtEq => a <= b,
            Bin::Gt => a > b,
            Bin::GtEq => a >= b,
            _ => unreachable!(),
        }
    };
    let (la, ra) = (f64_arr(l), f64_arr(r));
    let both_scalar = l.is_scalar() && r.is_scalar();
    let n = if both_scalar { 1 } else { nrows };
    let idx = |ev: &Ev, i: usize| if ev.is_scalar() { 0 } else { i };
    let out: BooleanArray = (0..n)
        .map(|i| {
            let (li, ri) = (idx(l, i), idx(r, i));
            if la.is_null(li) || ra.is_null(ri) {
                None
            } else {
                Some(f(la.value(li), ra.value(ri)))
            }
        })
        .collect();
    let out = Arc::new(out) as ArrayRef;
    if both_scalar {
        Ev::Scl(out)
    } else {
        Ev::Arr(out)
    }
}

// ---------------------------------------------------------------------
// Ingestion: anything -> a canonical RecordBatch (i64/f64/bool/utf8)
// ---------------------------------------------------------------------

/// Batch-native ingestion for the expression layer. Unlike the match
/// plane's `ingest_any` (which decomposes to engine Values and REJECTS
/// nulls — the certified subset has no null semantics), this keeps the
/// Arrow buffers intact and ACCEPTS nulls: the expression plane has SQL
/// null semantics. A `handle` column is legal INPUT here (the
/// result-table → filter → handle-aligned delete/update pipeline);
/// `handle` stays reserved as an OUTPUT name.
pub(crate) fn ingest_batch(
    py: Python<'_>,
    label: &str,
    data: &Bound<'_, PyAny>,
) -> PyResult<RecordBatch> {
    let _ = py;
    let batch = if let Ok(d) = data.downcast::<PyDict>() {
        dict_to_batch(label, d)?
    } else {
        let mut reader = import_stream(data)?;
        let schema = reader.schema();
        let mut batches = Vec::new();
        for b in reader.by_ref() {
            batches
                .push(b.map_err(|e| PyValueError::new_err(format!("{label}: arrow stream: {e}")))?);
        }
        match batches.len() {
            0 => RecordBatch::new_empty(schema),
            1 => batches.pop().unwrap(),
            _ => arrow_select::concat::concat_batches(&schema, &batches)
                .map_err(|e| PyValueError::new_err(format!("{label}: concat failed: {e}")))?,
        }
    };
    canonicalize(label, batch)
}

/// Widen every column to the expression subset's canonical types
/// (exactly the widenings the match-plane ingestion performs).
fn canonicalize(label: &str, batch: RecordBatch) -> PyResult<RecordBatch> {
    use DataType::*;
    let mut fields = Vec::with_capacity(batch.num_columns());
    let mut arrays = Vec::with_capacity(batch.num_columns());
    for (i, field) in batch.schema().fields().iter().enumerate() {
        let arr = batch.column(i);
        let target = match field.data_type() {
            Int64 | Float64 | Boolean | Utf8 => None,
            Int8 | Int16 | Int32 | UInt8 | UInt16 | UInt32 => Some(Int64),
            Float32 => Some(Float64),
            LargeUtf8 | Utf8View => Some(Utf8),
            Decimal128(_, _) => {
                return Err(PyTypeError::new_err(format!(
                    "{label}.{}: decimal columns are outside the derive v1 subset — \
                     cast first",
                    field.name()
                )))
            }
            other => {
                return Err(PyTypeError::new_err(format!(
                    "{label}.{}: Arrow type {other} is outside the expression subset \
                     (supported: int8..int64/uint8..uint32 -> i64, float32/64 -> f64, \
                     bool, utf8; cast or drop the column first)",
                    field.name()
                )))
            }
        };
        let (dt, arr) = match target {
            None => (field.data_type().clone(), arr.clone()),
            Some(t) => (t, widen(label, field.name(), arr)?),
        };
        fields.push(Field::new(field.name(), dt, field.is_nullable()));
        arrays.push(arr);
    }
    RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|e| PyValueError::new_err(format!("{label}: batch build failed: {e}")))
}

/// Exact widening to the canonical types, hand-rolled per source type
/// (arrow_cast's general cast would link its whole type-pair matrix).
fn widen(label: &str, name: &str, arr: &ArrayRef) -> PyResult<ArrayRef> {
    use arrow_arith::arity::unary;
    use arrow_array::types::{
        Float32Type, Int16Type, Int32Type, Int8Type, UInt16Type, UInt32Type, UInt8Type,
    };
    fn prim<T: arrow_array::ArrowPrimitiveType>(a: &ArrayRef) -> &arrow_array::PrimitiveArray<T> {
        a.as_any().downcast_ref().expect("dtype-matched")
    }
    let out: ArrayRef = match arr.data_type() {
        DataType::Int8 => Arc::new(unary::<Int8Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::Int16 => Arc::new(unary::<Int16Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::Int32 => Arc::new(unary::<Int32Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::UInt8 => Arc::new(unary::<UInt8Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::UInt16 => Arc::new(unary::<UInt16Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::UInt32 => Arc::new(unary::<UInt32Type, _, Int64Type>(prim(arr), |x| x as i64)),
        DataType::Float32 => {
            Arc::new(unary::<Float32Type, _, Float64Type>(prim(arr), |x| x as f64))
        }
        DataType::LargeUtf8 => {
            let s: &arrow_array::LargeStringArray =
                arr.as_any().downcast_ref().expect("dtype-matched");
            let mut b = StringBuilder::new();
            for i in 0..s.len() {
                if s.is_null(i) {
                    b.append_null();
                } else {
                    b.append_value(s.value(i));
                }
            }
            Arc::new(b.finish())
        }
        DataType::Utf8View => {
            let s: &arrow_array::StringViewArray =
                arr.as_any().downcast_ref().expect("dtype-matched");
            let mut b = StringBuilder::new();
            for i in 0..s.len() {
                if s.is_null(i) {
                    b.append_null();
                } else {
                    b.append_value(s.value(i));
                }
            }
            Arc::new(b.finish())
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "{label}.{name}: widening from {other} failed"
            )))
        }
    };
    Ok(out)
}

/// Dict-of-lists -> RecordBatch, building arrow arrays directly (None ->
/// null slot; no engine-Value intermediate).
fn dict_to_batch(label: &str, d: &Bound<'_, PyDict>) -> PyResult<RecordBatch> {
    #[derive(Clone, Copy, PartialEq)]
    enum K {
        I64,
        F64,
        Bool,
        Str,
    }
    let mut fields: Vec<Field> = Vec::new();
    let mut arrays: Vec<ArrayRef> = Vec::new();
    let mut nrows: Option<usize> = None;
    for (k, v) in d.iter() {
        let name: String = k.extract().map_err(|_| {
            PyTypeError::new_err(format!("{label}: column names must be strings"))
        })?;
        let col: &Bound<'_, PyList> = v.downcast().map_err(|_| {
            PyTypeError::new_err(format!("{label}.{name}: expected a list of values"))
        })?;
        match nrows {
            None => nrows = Some(col.len()),
            Some(n) if n != col.len() => {
                return Err(PyValueError::new_err(format!(
                    "{label}: ragged columns ({name} has {} values, expected {n})",
                    col.len()
                )))
            }
            _ => {}
        }
        // pass 1: infer (bool before int; int->float promotion; None ok)
        let mut kind: Option<K> = None;
        let mut saw_null = false;
        for item in col.iter() {
            if item.is_none() {
                saw_null = true;
                continue;
            }
            let this = if item.downcast::<pyo3::types::PyBool>().is_ok() {
                K::Bool
            } else if item.downcast::<pyo3::types::PyInt>().is_ok() {
                K::I64
            } else if item.downcast::<pyo3::types::PyFloat>().is_ok() {
                K::F64
            } else if item.downcast::<pyo3::types::PyString>().is_ok() {
                K::Str
            } else {
                return Err(PyTypeError::new_err(format!(
                    "{label}.{name}: unsupported value type {} (supported: bool, int, \
                     float, str, None)",
                    item.get_type().name().map(|n| n.to_string()).unwrap_or_default()
                )));
            };
            kind = Some(match (kind, this) {
                (None, t) => t,
                (Some(a), b) if a == b => a,
                (Some(K::I64), K::F64) | (Some(K::F64), K::I64) => K::F64,
                (Some(_), _) => {
                    return Err(PyTypeError::new_err(format!(
                        "{label}.{name}: mixed value types in one column"
                    )))
                }
            });
        }
        let Some(kind) = kind else {
            return Err(PyValueError::new_err(format!(
                "{label}.{name}: cannot infer a type (no non-null values) — pass an \
                 Arrow table with a typed schema instead"
            )));
        };
        // pass 2: build
        let (dt, arr): (DataType, ArrayRef) = match kind {
            K::I64 => {
                let mut b = Int64Builder::with_capacity(col.len());
                for item in col.iter() {
                    if item.is_none() {
                        b.append_null();
                    } else {
                        b.append_value(item.extract::<i64>().map_err(|_| {
                            PyValueError::new_err(format!(
                                "{label}.{name}: int {item} does not fit i64"
                            ))
                        })?);
                    }
                }
                (DataType::Int64, Arc::new(b.finish()))
            }
            K::F64 => {
                let mut b = Float64Builder::with_capacity(col.len());
                for item in col.iter() {
                    if item.is_none() {
                        b.append_null();
                    } else {
                        b.append_value(item.extract::<f64>()?);
                    }
                }
                (DataType::Float64, Arc::new(b.finish()))
            }
            K::Bool => {
                let mut b = BooleanBuilder::with_capacity(col.len());
                for item in col.iter() {
                    if item.is_none() {
                        b.append_null();
                    } else {
                        let pb = item.downcast::<pyo3::types::PyBool>().map_err(|_| {
                            PyTypeError::new_err(format!(
                                "{label}.{name}: mixed value types in one column"
                            ))
                        })?;
                        b.append_value(pb.is_true());
                    }
                }
                (DataType::Boolean, Arc::new(b.finish()))
            }
            K::Str => {
                let mut b = StringBuilder::new();
                for item in col.iter() {
                    if item.is_none() {
                        b.append_null();
                    } else {
                        b.append_value(item.extract::<String>().map_err(|_| {
                            PyTypeError::new_err(format!(
                                "{label}.{name}: mixed value types in one column"
                            ))
                        })?);
                    }
                }
                (DataType::Utf8, Arc::new(b.finish()))
            }
        };
        fields.push(Field::new(&name, dt, saw_null));
        arrays.push(arr);
    }
    if fields.is_empty() {
        return Err(PyValueError::new_err(format!(
            "{label}: empty dict — at least one column is required"
        )));
    }
    RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|e| PyValueError::new_err(format!("{label}: batch build failed: {e}")))
}

// ---------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------

fn ctx_of(batch: &RecordBatch) -> Ctx {
    let names = batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect::<Vec<_>>();
    let types = batch
        .schema()
        .fields()
        .iter()
        .map(|f| match f.data_type() {
            DataType::Int64 => Ty::I64,
            DataType::Float64 => Ty::F64,
            DataType::Boolean => Ty::Bool,
            DataType::Utf8 => Ty::Utf8,
            other => unreachable!("canonicalized batch has {other}"),
        })
        .collect();
    Ctx { names, types, arrays: batch.columns().to_vec(), nrows: batch.num_rows() }
}

/// `derive.with_columns(data, **named_exprs)` — append computed columns.
/// Expressions see the INPUT columns only (polars semantics); output =
/// input columns, then computed columns in keyword order.
#[pyfunction]
pub(crate) fn derive_with_columns(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    exprs: &Bound<'_, PyDict>,
) -> PyResult<PyTable> {
    const LABEL: &str = "derive.with_columns";
    let batch = ingest_batch(py, LABEL, data)?;
    let ctx = ctx_of(&batch);
    let mut fields: Vec<Field> =
        batch.schema().fields().iter().map(|f| f.as_ref().clone()).collect();
    let mut arrays: Vec<ArrayRef> = batch.columns().to_vec();
    let mut names: Vec<String> = ctx.names.clone();
    for (k, v) in exprs.iter() {
        let name: String = k.extract().map_err(|_| {
            PyTypeError::new_err(format!("{LABEL}: output column names must be strings"))
        })?;
        let label = format!("{LABEL}.{name}");
        if name == "handle" {
            return Err(PyValueError::new_err(format!(
                "{label}: the column name \"handle\" is reserved — result tables \
                 carry the engine's fact handle under it; pick a new name"
            )));
        }
        if names.iter().any(|n| *n == name) {
            return Err(PyValueError::new_err(format!(
                "{label}: output column {name:?} already exists in the input — \
                 pick a new name"
            )));
        }
        let ex = parse(&label, &v, 0)?;
        let ty = check(&label, &ex, &ctx)?;
        let ev = eval(&label, &ex, &ctx)?;
        let arr = ev.broadcast(&label, ctx.nrows)?;
        fields.push(Field::new(&name, ty.dtype(), true));
        arrays.push(arr);
        names.push(name);
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|e| PyValueError::new_err(format!("{LABEL}: batch build failed: {e}")))?;
    Ok(PyTable { batch })
}

/// `derive.filter(data, pred)` — SQL WHERE: rows where the predicate is
/// TRUE pass; FALSE and NULL rows drop. Output schema is the input's.
#[pyfunction]
pub(crate) fn derive_filter(
    py: Python<'_>,
    data: &Bound<'_, PyAny>,
    pred: &Bound<'_, PyAny>,
) -> PyResult<PyTable> {
    const LABEL: &str = "derive.filter";
    let batch = ingest_batch(py, LABEL, data)?;
    let ctx = ctx_of(&batch);
    let ex = parse(LABEL, pred, 0)?;
    let ty = check(LABEL, &ex, &ctx)?;
    if ty != Ty::Bool {
        return Err(PyTypeError::new_err(format!(
            "{LABEL}: predicate resolves to {} — comparisons and boolean ops \
             produce the boolean filter() needs",
            ty.name()
        )));
    }
    let ev = eval(LABEL, &ex, &ctx)?;
    let mask_arr = ev.broadcast(LABEL, ctx.nrows)?;
    let mask = mask_arr.as_any().downcast_ref::<BooleanArray>().expect("typechecked bool");
    // SQL WHERE: null predicate rows drop (pins §L)
    let mask = if mask.null_count() > 0 {
        arrow_select::filter::prep_null_mask_filter(mask)
    } else {
        mask.clone()
    };
    let out = arrow_select::filter::filter_record_batch(&batch, &mask)
        .map_err(|e| PyValueError::new_err(format!("{LABEL}: filter failed: {e}")))?;
    Ok(PyTable { batch: out })
}
