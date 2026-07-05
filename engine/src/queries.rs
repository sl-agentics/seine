//! DRL query support: Phase Q0 (unification, D-049..D-053) + Phase Q1
//! (or-branches, positional patterns, query calls, self-recursion,
//! D-054/D-055). Queries evaluate on demand against the current WM.
//!
//! One evaluator serves both phases: the oracle-pinned STACK MACHINE of
//! D-054 (a Q0 query is a single-branch, call-free run):
//!   - the root env is staged into EVERY branch's shared pool; branches
//!     evaluate in declaration order; top-level rows APPEND. A pool may
//!     be swept early by a nested takeAll — rows still route by tuple
//!     parentage.
//!   - fact levels batch per Q0: consume src head→tail, children
//!     PREPEND into the next stage (memories in reverse-insertion
//!     arrival order; D-053 index; full-slot iteration only for
//!     single-field unification indexes; D-052 per-site unification
//!     binding at pattern exit).
//!   - call levels push a RESUME frame, stage one nested env per src
//!     tuple (PREPEND into every callee-branch pool), then push one
//!     BranchEval per callee branch (declaration order; LIFO pop).
//!   - terminals route by root: top-level rows append; nested rows
//!     build the child env (caller env + FIRST-WINS threaded bindings)
//!     and PREPEND it into the call-site's result staging.
//!   - a RESUME pop splices the site's staged results after its
//!     captured trg and continues after the call node.
//! The D-055 wall (base-first 2-branch self-recursion only, no left
//! recursion, no mutual recursion, acyclic data backstopped by a step
//! limit) keeps evaluation inside the pinned subset — Drools' late-
//! result re-push (checkAndTriggerQueryReevaluation) is unreachable
//! there and deliberately not modeled.

use std::collections::HashMap;
use std::rc::Rc;

use crate::drl::{self, CmpOp, Constraint, Literal, QArg, QElemBody, QueryDef};
use crate::engine::{eval_cmp_join_pub, eval_cmp_pub, EngineError};
use crate::store::{FactStore, FactView, FactId, FieldType, TypeId, Value};

/// One literal (alpha) test: same-type literals only (D-051).
enum AlphaTest {
    Cmp { op: CmpOp, rhs: Value },
    Matches(crate::rx::Regex),
    Contains(String),
    InList { items: Vec<Value>, negated: bool },
}

/// Operand of a beta constraint: a query parameter (unification) or a
/// scalar binding declared in an earlier element.
#[derive(Clone, Copy)]
enum Operand {
    Param(usize),
    Binding(usize),
}

struct BetaCon {
    field_idx: usize,
    op: CmpOp,
    operand: Operand,
}

struct QPattern {
    tid: TypeId,
    fact_slot: Option<usize>,
    /// (env slot, field index) for bindings, textual order.
    field_binds: Vec<(usize, usize)>,
    alpha: Vec<(usize, AlphaTest)>,
    beta: Vec<BetaCon>,
    /// positions into `beta` forming the index (D-053).
    index: Vec<usize>,
    unification_join: bool,
    seed: u32,
}

/// One call argument: an env slot or a literal value.
#[derive(Clone)]
enum CArg {
    Slot(usize),
    Lit(Value),
}

enum CNode {
    Fact(QPattern),
    Call { callee: usize, args: Vec<CArg> },
}

pub struct CompiledQuery {
    pub name: String,
    /// Declaration position in the DRL unit (rules+queries interleaved) —
    /// the query's agenda item sits at (salience 0, this) (D-058).
    pub decl_pos: usize,
    params: Vec<(String, FieldType)>,
    /// output identifiers: params + FIRST branch declarations (D-054).
    idents: Vec<String>,
    /// env size (slots span all branches; cross-branch name reuse is
    /// compile-rejected, D-055).
    slot_count: usize,
    branches: Vec<Vec<CNode>>,
}

pub enum QueryVal {
    Fact(FactView),
    Scalar(Value),
    /// identifier not bound in this row's branch (D-054).
    Null,
}

pub struct QueryOutput {
    pub identifiers: Vec<String>,
    pub rows: Vec<Vec<QueryVal>>,
}

#[derive(Clone)]
enum EnvVal {
    Fact(FactId),
    Val(Value),
}

/// Root of an evaluation tuple: the top-level call, a nested dquery
/// remembering its call site and full caller env for result routing, or
/// a rule-side ?query CE call (D-056) remembering which left it serves.
#[derive(Clone)]
enum Root {
    Top,
    Nested(Rc<NestedRoot>),
    Site(usize),
}

struct NestedRoot {
    site: (usize, usize, usize),
    caller: Env,
}

#[derive(Clone)]
struct Env {
    slots: Vec<Option<EnvVal>>,
    root: Root,
}

// ---------------------------------------------------------------------
// Java hash reproduction (D-050) — verified bit-exact against live
// Drools TupleIndexHashTable dumps.

fn java_hash(v: &Value) -> u32 {
    match v {
        Value::I64(n) => {
            let u = *n as u64;
            (u ^ (u >> 32)) as u32
        }
        Value::F64(f) => {
            let bits = f.to_bits();
            (bits ^ (bits >> 32)) as u32
        }
        Value::Bool(b) => {
            if *b {
                1231
            } else {
                1237
            }
        }
        Value::Str(s) => s
            .encode_utf16()
            .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32)),
    }
}

/// JDK6 supplemental hash (AbstractHashTable.rehash).
fn rehash(mut h: u32) -> u32 {
    h ^= (h >> 20) ^ (h >> 12);
    h ^ (h >> 7) ^ (h >> 4)
}

fn key_hash(seed: u32, key: &[Value]) -> u32 {
    let mut h = seed;
    for v in key {
        h = h.wrapping_mul(31).wrapping_add(java_hash(v));
    }
    rehash(h)
}

/// Extractor index (D-050): 1 + rank of the field's accessor method name
/// among the bean's no-arg public methods sorted by name.
fn extractor_index(store: &FactStore, tid: TypeId, field_idx: usize) -> u32 {
    let schema = store.schema(tid);
    let accessor = |name: &str, ft: FieldType| -> String {
        let mut cs = name.chars();
        let head = cs.next().unwrap().to_ascii_uppercase();
        let cap = format!("{head}{}", cs.as_str());
        match ft {
            FieldType::Bool => format!("is{cap}"),
            _ => format!("get{cap}"),
        }
    };
    let mut methods: Vec<String> = schema
        .fields
        .iter()
        .map(|(n, ft)| accessor(n, *ft))
        .collect();
    methods.push("getClass".into());
    methods.push("hashCode".into());
    methods.push("toString".into());
    methods.sort();
    let target = accessor(&schema.fields[field_idx].0, schema.fields[field_idx].1);
    1 + methods.iter().position(|m| *m == target).unwrap() as u32
}

/// HashEntry.equals mirror: exact per type, doubles by bit pattern.
fn key_eq(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| match (x, y) {
            (Value::F64(p), Value::F64(q)) => p.to_bits() == q.to_bits(),
            _ => x == y,
        })
}

fn lit_value(l: &Literal) -> Value {
    match l {
        Literal::I64(n) => Value::I64(*n),
        Literal::F64(f) => Value::F64(*f),
        Literal::Str(s) => Value::Str(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
    }
}

// ---------------------------------------------------------------------
// Compilation

/// Compile all queries of a DRL file together (calls may reference
/// queries defined later; the call graph is validated per D-055).
pub fn compile_queries(
    store: &FactStore,
    defs: Vec<QueryDef>,
    reserved: &[&str],
) -> Result<Vec<CompiledQuery>, EngineError> {
    for def in &defs {
        if defs.iter().filter(|d| d.name == def.name).count() > 1 {
            return Err(EngineError(format!("duplicate query {}", def.name)));
        }
    }
    let names: Vec<String> = defs.iter().map(|d| d.name.clone()).collect();
    let mut out = Vec::new();
    for def in &defs {
        out.push(compile_query(store, def, &names, reserved)?);
    }
    // D-055 call-graph walls: only DIRECT self-recursion; the shape checks
    // (2 branches, base first, call not first, single self-call) are
    // enforced in compile_query; mutual recursion is rejected here.
    let graph: Vec<Vec<usize>> = out
        .iter()
        .map(|q| {
            let mut cs = Vec::new();
            for br in &q.branches {
                for n in br {
                    if let CNode::Call { callee, .. } = n {
                        cs.push(*callee);
                    }
                }
            }
            cs
        })
        .collect();
    for start in 0..graph.len() {
        // DFS for a cycle of length >= 2 through `start`
        let mut stack = vec![(start, 0usize)];
        let mut path = vec![start];
        let mut visited = vec![false; graph.len()];
        while let Some((node, ci)) = stack.pop() {
            if ci < graph[node].len() {
                stack.push((node, ci + 1));
                let next = graph[node][ci];
                if next == node {
                    continue; // direct self-recursion is allowed
                }
                if next == start && path.len() >= 2 {
                    return Err(EngineError(format!(
                        "query {}: mutual recursion is out of subset (D-055)",
                        out[start].name
                    )));
                }
                if !visited[next] && !path.contains(&next) {
                    visited[next] = true;
                    path.push(next);
                    stack.push((next, 0));
                }
            } else {
                path.pop();
            }
        }
    }
    Ok(out)
}

fn compile_query(
    store: &FactStore,
    def: &QueryDef,
    query_names: &[String],
    reserved: &[&str],
) -> Result<CompiledQuery, EngineError> {
    let err = |m: String| Err(EngineError(format!("query {}: {m}", def.name)));
    if def.branches.is_empty() || def.branches.iter().any(|b| b.is_empty()) {
        return err("empty query body not in subset".into());
    }
    let mut params = Vec::new();
    for (ty, name) in &def.params {
        let ft = match ty.as_str() {
            "long" => FieldType::I64,
            "double" => FieldType::F64,
            "String" => FieldType::Str,
            "boolean" => FieldType::Bool,
            other => {
                return err(format!(
                    "param type {other} not in subset (long/double/String/boolean)"
                ))
            }
        };
        params.push((name.clone(), ft));
    }

    // slot table: params first, then declarations walking branches in
    // order; a name may be declared in exactly ONE branch (D-055).
    let mut slots: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
    let mut slot_types: Vec<Option<FieldType>> = params.iter().map(|(_, t)| Some(*t)).collect();
    let mut slot_is_fact: Vec<bool> = params.iter().map(|_| false).collect();
    let mut slot_branch: Vec<Option<usize>> = params.iter().map(|_| None).collect();

    let is_recursive = def.branches.iter().flatten().any(|e| e.name == def.name);
    if is_recursive {
        // D-055 fenced shape: exactly 2 branches, base first (no self
        // call), recursive branch second with exactly one self-call that
        // is not its first element.
        if def.branches.len() != 2 {
            return err("recursive queries must have exactly 2 or-branches (D-055)".into());
        }
        if def.branches[0].iter().any(|e| e.name == def.name) {
            return err(
                "the recursive branch must be SECOND (base branch first, D-055)".into(),
            );
        }
        let selfs: Vec<usize> = def.branches[1]
            .iter()
            .enumerate()
            .filter(|(_, e)| e.name == def.name)
            .map(|(i, _)| i)
            .collect();
        if selfs.len() != 1 {
            return err("recursive queries take exactly ONE self-call (D-055)".into());
        }
        if selfs[0] == 0 {
            return err("left recursion: the self-call cannot be the first element of its branch (qb7, D-055)".into());
        }
    } else if def.branches.len() > 3 {
        return err("more than 3 or-branches not in subset (D-055)".into());
    }

    let mut branches = Vec::new();
    let mut used_params = vec![false; params.len()];
    for (bi, branch) in def.branches.iter().enumerate() {
        let mut nodes = Vec::new();
        // slots declared by earlier ELEMENTS of earlier branches or this
        // branch (same-element operands are out of subset, D-053)
        let mut prior_elem_slots = slots.len();
        for elem in branch {
            if let Some(qi) = query_names.iter().position(|n| *n == elem.name) {
                // ---- query call ----
                if elem.binding.is_some() {
                    return err(format!("call to {} cannot take a pattern binding", elem.name));
                }
                let args_raw = match &elem.body {
                    QElemBody::Positional(a) => a,
                    QElemBody::Named(_) => {
                        return err(format!(
                            "call to {} must use positional args (`{}(...;)`)",
                            elem.name, elem.name
                        ))
                    }
                };
                let callee_def = &def; // arity/type check against defs below
                let _ = callee_def;
                let mut args = Vec::new();
                for a in args_raw {
                    match a {
                        QArg::Lit(l) => args.push(CArg::Lit(lit_value(l))),
                        QArg::Var(v) => {
                            if let Some(slot) = slots.iter().position(|s| s == v) {
                                if slot_is_fact[slot] {
                                    return err(format!("{v} is a fact binding; call args must be scalars"));
                                }
                                if slot >= prior_elem_slots {
                                    return err(format!(
                                        "{v} is bound in the same element group (out of subset, D-053)"
                                    ));
                                }
                                if slot >= params.len() {
                                    if let Some(db) = slot_branch[slot] {
                                        if db != bi {
                                            return err(format!(
                                                "{v} is declared in another or-branch (D-055)"
                                            ));
                                        }
                                    }
                                }
                                if slot < params.len() {
                                    used_params[slot] = true;
                                }
                                args.push(CArg::Slot(slot));
                            } else {
                                // fresh variable: declared by this call
                                slots.push(v.clone());
                                slot_types.push(None); // typed below via callee
                                slot_is_fact.push(false);
                                slot_branch.push(Some(bi));
                                args.push(CArg::Slot(slots.len() - 1));
                            }
                        }
                    }
                }
                nodes.push(CNode::Call { callee: qi, args });
                prior_elem_slots = slots.len();
                continue;
            }
            // ---- fact pattern ----
            if reserved.contains(&elem.name.as_str()) {
                return err(format!("type {} is reserved", elem.name));
            }
            let tid = store
                .type_id(&elem.name)
                .ok_or_else(|| EngineError(format!("query {}: unknown type or query {}", def.name, elem.name)))?;
            let fact_slot = match &elem.binding {
                Some(b) => {
                    if slots.iter().any(|s| s == b) {
                        return err(format!("duplicate identifier {b}"));
                    }
                    slots.push(b.clone());
                    slot_types.push(None);
                    slot_is_fact.push(true);
                    slot_branch.push(Some(bi));
                    Some(slots.len() - 1)
                }
                None => None,
            };
            // positional form desugars to unification/bind/alpha constraints
            let constraints: Vec<Constraint> = match &elem.body {
                QElemBody::Named(cs) => cs.clone(),
                QElemBody::Positional(args) => {
                    let schema = store.schema(tid);
                    if args.len() != schema.fields.len() {
                        return err(format!(
                            "{}: positional pattern expects {} args, got {}",
                            elem.name,
                            schema.fields.len(),
                            args.len()
                        ));
                    }
                    let mut cs = Vec::new();
                    for ((fname, _), a) in schema.fields.iter().zip(args) {
                        match a {
                            QArg::Lit(l) => cs.push(Constraint::Cmp {
                                field: fname.clone(),
                                op: CmpOp::Eq,
                                rhs: drl::CmpRhs::Lit(l.clone()),
                            }),
                            QArg::Var(v) => {
                                if slots.iter().any(|s| s == v) {
                                    cs.push(Constraint::Cmp {
                                        field: fname.clone(),
                                        op: CmpOp::Eq,
                                        rhs: drl::CmpRhs::Var(v.clone()),
                                    });
                                } else {
                                    cs.push(Constraint::Bind {
                                        var: v.clone(),
                                        field: fname.clone(),
                                    });
                                }
                            }
                        }
                    }
                    cs
                }
            };
            let mut alpha = Vec::new();
            let mut beta: Vec<BetaCon> = Vec::new();
            let mut field_binds = Vec::new();
            for c in &constraints {
                match c {
                    Constraint::Bind { var, field } => {
                        let fi = store.field_index(tid, field).ok_or_else(|| {
                            EngineError(format!("query {}: {} has no field {field}", def.name, elem.name))
                        })?;
                        let ft = store.field_type(tid, fi);
                        if slots.iter().any(|s| s == var) {
                            return err(format!("duplicate identifier {var}"));
                        }
                        slots.push(var.clone());
                        slot_types.push(Some(ft));
                        slot_is_fact.push(false);
                        slot_branch.push(Some(bi));
                        field_binds.push((slots.len() - 1, fi));
                    }
                    Constraint::Cmp { field, op, rhs } => {
                        let fi = store.field_index(tid, field).ok_or_else(|| {
                            EngineError(format!("query {}: {} has no field {field}", def.name, elem.name))
                        })?;
                        let ft = store.field_type(tid, fi);
                        match rhs {
                            drl::CmpRhs::Lit(l) => {
                                let v = lit_value(l);
                                if v.type_of() != ft {
                                    return err(format!(
                                        "literal constraint on {field} must match the field type exactly (D-051)"
                                    ));
                                }
                                alpha.push((fi, AlphaTest::Cmp { op: *op, rhs: v }));
                            }
                            drl::CmpRhs::Var(v) => {
                                let slot = slots.iter().position(|s| s == v).ok_or_else(|| {
                                    EngineError(format!("query {}: unknown binding {v}", def.name))
                                })?;
                                if slot_is_fact[slot] {
                                    return err(format!("{v} is a fact binding; comparing fields to fact bindings is out of subset"));
                                }
                                if slot >= prior_elem_slots {
                                    return err(format!(
                                        "{v} is bound in the same pattern (same-pattern operands are out of subset, D-053)"
                                    ));
                                }
                                if slot >= params.len() {
                                    if let Some(db) = slot_branch[slot] {
                                        if db != bi {
                                            return err(format!(
                                                "{v} is declared in another or-branch (D-055)"
                                            ));
                                        }
                                    }
                                }
                                let operand = if slot < params.len() {
                                    if *op != CmpOp::Eq {
                                        return err(format!(
                                            "param {v} used with a non-== operator (unification is == only, D-051)"
                                        ));
                                    }
                                    if params[slot].1 != ft {
                                        return err(format!(
                                            "param {v} type does not match field {field} exactly"
                                        ));
                                    }
                                    used_params[slot] = true;
                                    Operand::Param(slot)
                                } else {
                                    if slot_types[slot].map(|t| t != ft).unwrap_or(false) {
                                        // cross-type joins in queries are
                                        // generator-excluded; allow with
                                        // D-020 coercion at eval time
                                    }
                                    Operand::Binding(slot)
                                };
                                beta.push(BetaCon { field_idx: fi, op: *op, operand });
                            }
                        }
                    }
                    Constraint::Matches { field, regex } => {
                        let fi = require_str_field(store, tid, field, &def.name)?;
                        let r = crate::rx::Regex::parse(regex)
                            .map_err(|e| EngineError(format!("query {}: bad regex: {e}", def.name)))?;
                        alpha.push((fi, AlphaTest::Matches(r)));
                    }
                    Constraint::Contains { field, needle } => {
                        let fi = require_str_field(store, tid, field, &def.name)?;
                        alpha.push((fi, AlphaTest::Contains(needle.clone())));
                    }
                    Constraint::InList { field, items, negated } => {
                        let fi = store.field_index(tid, field).ok_or_else(|| {
                            EngineError(format!("query {}: {} has no field {field}", def.name, elem.name))
                        })?;
                        let ft = store.field_type(tid, fi);
                        let vals: Vec<Value> = items.iter().map(lit_value).collect();
                        if vals.iter().any(|v| v.type_of() != ft) {
                            return err(format!(
                                "in-list items on {field} must match the field type exactly (D-051)"
                            ));
                        }
                        alpha.push((fi, AlphaTest::InList { items: vals, negated: *negated }));
                    }
                }
            }
            // D-053 index: regular equalities (textual order, dups, cap 3)
            // else the first unification alone.
            let regular_eqs: Vec<usize> = beta
                .iter()
                .enumerate()
                .filter(|(_, b)| b.op == CmpOp::Eq && matches!(b.operand, Operand::Binding(_)))
                .map(|(i, _)| i)
                .collect();
            let (index, unification_join) = if !regular_eqs.is_empty() {
                (regular_eqs.into_iter().take(3).collect::<Vec<_>>(), false)
            } else if let Some(first_unif) = beta
                .iter()
                .position(|b| b.op == CmpOp::Eq && matches!(b.operand, Operand::Param(_)))
            {
                (vec![first_unif], true)
            } else {
                (Vec::new(), false)
            };
            let mut seed: u32 = 31;
            for &i in &index {
                let ext = extractor_index(store, tid, beta[i].field_idx);
                seed = seed.wrapping_add(seed.wrapping_mul(31)).wrapping_add(ext);
            }
            nodes.push(CNode::Fact(QPattern {
                tid,
                fact_slot,
                field_binds,
                alpha,
                beta,
                index,
                unification_join,
                seed,
            }));
            prior_elem_slots = slots.len();
        }
        branches.push(nodes);
    }
    if let Some(i) = used_params.iter().position(|u| !u) {
        // params may be threaded via calls too — check call args
        let threaded = branches.iter().any(|br| {
            br.iter().any(|n| match n {
                CNode::Call { args, .. } => args
                    .iter()
                    .any(|a| matches!(a, CArg::Slot(s) if *s == i)),
                _ => false,
            })
        });
        if !threaded {
            return err(format!(
                "param {} is never unified or threaded (unused params are out of subset, D-051)",
                params[i].0
            ));
        }
    }
    // identifiers: params + FIRST branch declarations, slot order (D-054)
    let mut idents: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
    for (i, name) in slots.iter().enumerate().skip(params.len()) {
        if slot_branch[i] == Some(0) {
            idents.push(name.clone());
        }
    }
    Ok(CompiledQuery {
        name: def.name.clone(),
        decl_pos: def.decl_pos,
        params,
        idents,
        slot_count: slots.len(),
        branches,
    })
}

/// Arity/param-type validation of calls needs all queries compiled; run
/// as a second pass.
pub fn validate_calls(queries: &[CompiledQuery]) -> Result<(), EngineError> {
    for q in queries {
        for br in &q.branches {
            for n in br {
                if let CNode::Call { callee, args } = n {
                    let c = &queries[*callee];
                    if args.len() != c.params.len() {
                        return Err(EngineError(format!(
                            "query {}: call to {} expects {} args, got {}",
                            q.name,
                            c.name,
                            c.params.len(),
                            args.len()
                        )));
                    }
                    for (a, (pname, pt)) in args.iter().zip(&c.params) {
                        if let CArg::Lit(v) = a {
                            if v.type_of() != *pt {
                                return Err(EngineError(format!(
                                    "query {}: literal arg for {pname} of {} has the wrong type",
                                    q.name, c.name
                                )));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn require_str_field(
    store: &FactStore,
    tid: TypeId,
    field: &str,
    qname: &str,
) -> Result<usize, EngineError> {
    let fi = store
        .field_index(tid, field)
        .ok_or_else(|| EngineError(format!("query {qname}: no field {field}")))?;
    if store.field_type(tid, fi) != FieldType::Str {
        return Err(EngineError(format!(
            "query {qname}: {field} must be a String field"
        )));
    }
    Ok(fi)
}

// ---------------------------------------------------------------------
// Evaluation: the D-054 stack machine.

struct KeyList {
    hash: u32,
    key: Vec<Value>,
    facts: Vec<FactId>,
}

struct Table {
    slots: Vec<Vec<KeyList>>,
}

const TABLE_LEN: u32 = 128;
const RESIZE_THRESHOLD: usize = 96;
/// Backstop for cyclic recursion data (Drools HANGS there, D-055): total
/// stack pushes across one top-level call.
const STEP_LIMIT: usize = 1_000_000;

impl Table {
    fn build(
        store: &FactStore,
        arrival: &[FactId],
        fields: &[usize],
        seed: u32,
    ) -> Result<Table, EngineError> {
        let mut slots: Vec<Vec<KeyList>> = (0..TABLE_LEN).map(|_| Vec::new()).collect();
        let mut distinct = 0usize;
        for &f in arrival {
            let key: Vec<Value> = fields.iter().map(|&fi| store.value(f, fi)).collect();
            let h = key_hash(seed, &key);
            let slot = (h & (TABLE_LEN - 1)) as usize;
            if let Some(kl) = slots[slot]
                .iter_mut()
                .find(|kl| kl.hash == h && key_eq(&kl.key, &key))
            {
                kl.facts.push(f);
            } else {
                distinct += 1;
                if distinct > RESIZE_THRESHOLD {
                    return Err(EngineError(
                        "query index exceeds 96 distinct keys (hash-table resize is out of subset, D-051)"
                            .into(),
                    ));
                }
                slots[slot].insert(0, KeyList { hash: h, key, facts: vec![f] });
            }
        }
        Ok(Table { slots })
    }

    fn full_order(&self) -> Vec<FactId> {
        self.slots
            .iter()
            .flat_map(|chain| chain.iter().flat_map(|kl| kl.facts.iter().copied()))
            .collect()
    }

    fn bucket(&self, hash: u32, key: &[Value]) -> Vec<FactId> {
        let slot = (hash & (TABLE_LEN - 1)) as usize;
        self.slots[slot]
            .iter()
            .find(|kl| kl.hash == hash && key_eq(&kl.key, key))
            .map(|kl| kl.facts.clone())
            .unwrap_or_default()
    }
}

enum Frame {
    Branch { q: usize, b: usize, batch: Vec<Env> },
    Resume { q: usize, b: usize, node: usize, trg: Vec<Env> },
}

/// PERSISTENT per-pattern right memories of the query networks (D-056,
/// probes qx8_statemem/qx8_statemem3): staged alpha-passing facts drain
/// into a pattern's memory AT EACH EVALUATION of its query network —
/// newest-first within the batch, batches APPENDED. A ?query CE
/// evaluating mid-firing therefore splits the memory into drain windows;
/// facts inserted later land in LATER batches, unlike a fresh
/// reverse-insertion rebuild. With every evaluation post-quiescence (the
/// pre-Q2 envelope) there is a single batch and the two models coincide.
/// Keyed by (query, branch, node); deletes leave at the next drain.
#[derive(Default)]
pub struct QueryMem(HashMap<(usize, usize, usize), Vec<FactId>>);

/// One drain window for one pattern (qx8_statemem/3): staged deletes
/// leave; staged alpha-passing inserts append NEWEST-FIRST after the
/// existing batches. The memory order IS the arrival order.
fn drain_pattern(
    mem: &mut QueryMem,
    store: &FactStore,
    site: (usize, usize, usize),
    pat: &QPattern,
) -> Vec<FactId> {
    let m = mem.0.entry(site).or_default();
    m.retain(|f| store.is_alive(*f));
    let seen: std::collections::HashSet<FactId> = m.iter().copied().collect();
    let mut fresh: Vec<FactId> = store
        .live_facts_of(pat.tid)
        .filter(|f| !seen.contains(f))
        .filter(|&f| {
            pat.alpha.iter().all(|(fi, t)| {
                let v = store.value(f, *fi);
                match t {
                    AlphaTest::Cmp { op, rhs } => eval_cmp_pub(&v, *op, rhs),
                    AlphaTest::Matches(r) => matches!(&v, Value::Str(s) if r.accepts(s)),
                    AlphaTest::Contains(n) => {
                        matches!(&v, Value::Str(s) if s.contains(n.as_str()))
                    }
                    AlphaTest::InList { items, negated } => {
                        let hit = items.iter().any(|i| eval_cmp_pub(&v, CmpOp::Eq, i));
                        hit != *negated
                    }
                }
            })
        })
        .collect();
    fresh.reverse();
    m.extend(fresh);
    m.clone()
}

/// Evaluate a query's own network with no driving tuples — the agenda-
/// item evaluation of a PENDING query (D-058): every fact pattern of the
/// query drains one window. Called queries have their OWN items and are
/// not touched.
pub fn drain_query(
    store: &FactStore,
    queries: &[CompiledQuery],
    mem: &mut QueryMem,
    qi: usize,
) {
    for (bi, branch) in queries[qi].branches.iter().enumerate() {
        for (ni, node) in branch.iter().enumerate() {
            if let CNode::Fact(pat) = node {
                drain_pattern(mem, store, (qi, bi, ni), pat);
            }
        }
    }
}

/// Transitive call closure of a set of root queries (rule
/// getDependingQueries mirror, D-058).
pub fn dependencies(queries: &[CompiledQuery], roots: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    let mut work: Vec<usize> = roots.to_vec();
    while let Some(qi) = work.pop() {
        if out.contains(&qi) {
            continue;
        }
        out.push(qi);
        for br in &queries[qi].branches {
            for n in br {
                if let CNode::Call { callee, .. } = n {
                    work.push(*callee);
                }
            }
        }
    }
    out.sort();
    out
}

struct Machine<'a> {
    store: &'a FactStore,
    queries: &'a [CompiledQuery],
    mem: &'a mut QueryMem,
    pool: HashMap<(usize, usize), Vec<Env>>,
    qmem: HashMap<(usize, usize, usize), Vec<Env>>,
    stack: Vec<Frame>,
    out: Vec<Env>,
    /// Rule-site result staging (D-056): rows PREPEND at arrival
    /// (rowAdded/addInsert), so index 0 = newest.
    site_out: Vec<(usize, Vec<Value>)>,
    steps: usize,
}

pub fn run_query(
    store: &FactStore,
    queries: &[CompiledQuery],
    mem: &mut QueryMem,
    name: &str,
    args: &[Option<Value>],
) -> Result<QueryOutput, EngineError> {
    let qi = queries
        .iter()
        .position(|q| q.name == name)
        .ok_or_else(|| EngineError(format!("query {name} does not exist")))?;
    let q = &queries[qi];
    if args.len() != q.params.len() {
        return Err(EngineError(format!(
            "query {name} expects {} arguments, got {}",
            q.params.len(),
            args.len()
        )));
    }
    for (a, (pname, pt)) in args.iter().zip(&q.params) {
        if let Some(v) = a {
            if v.type_of() != *pt {
                return Err(EngineError(format!(
                    "query {name}: argument {pname} type mismatch"
                )));
            }
        }
    }
    let mut m = Machine {
        store,
        queries,
        mem,
        pool: HashMap::new(),
        qmem: HashMap::new(),
        stack: Vec::new(),
        out: Vec::new(),
        site_out: Vec::new(),
        steps: 0,
    };
    let mut env0 = Env { slots: vec![None; q.slot_count], root: Root::Top };
    for (i, a) in args.iter().enumerate() {
        env0.slots[i] = a.clone().map(EnvVal::Val);
    }
    // stage the root into every branch pool, then evaluate paths in
    // declaration order (D-054); pools may be swept early.
    for b in 0..q.branches.len() {
        m.pool.entry((qi, b)).or_default().insert(0, env0.clone());
    }
    for b in 0..q.branches.len() {
        let batch = m.pool.get_mut(&(qi, b)).map(std::mem::take).unwrap_or_default();
        m.stack.push(Frame::Branch { q: qi, b, batch });
        m.drain()?;
    }
    let idents = q.idents.clone();
    let rows = m
        .out
        .iter()
        .map(|env| {
            idents
                .iter()
                .map(|ident| {
                    let slot = slot_of(q, ident);
                    match &env.slots[slot] {
                        Some(EnvVal::Fact(id)) => QueryVal::Fact(store.render(*id)),
                        Some(EnvVal::Val(v)) => QueryVal::Scalar(v.clone()),
                        None => QueryVal::Null,
                    }
                })
                .collect()
        })
        .collect();
    Ok(QueryOutput { identifiers: idents, rows })
}

/// Rule-side ?query CE evaluation (D-056): one BATCHED machine run for a
/// window's staged lefts. `calls` holds each left's args in REAL staged
/// order (head first, full LIFO); each is PREPENDED as a dquery env into
/// every callee-branch pool (pool = reverse of src — evaluation
/// interleaves per left exactly like PhreakQueryNode.doLeftInserts), then
/// ALL branch frames push in declaration order and pop LIFO — unlike the
/// standalone entry point, which drives paths sequentially in declaration
/// order (both pinned; evalQueryNode vs getQueryResults).
/// Returns the site staging head-first: (call index, full row values per
/// param). The caller drains it order-preserved for a single sink and
/// re-reversed for shared sinks (QueryTupleSets.addTo).
pub fn run_site(
    store: &FactStore,
    queries: &[CompiledQuery],
    mem: &mut QueryMem,
    qi: usize,
    calls: &[Vec<Option<Value>>],
) -> Result<Vec<(usize, Vec<Value>)>, EngineError> {
    let q = &queries[qi];
    let mut m = Machine {
        store,
        queries,
        mem,
        pool: HashMap::new(),
        qmem: HashMap::new(),
        stack: Vec::new(),
        out: Vec::new(),
        site_out: Vec::new(),
        steps: 0,
    };
    for (idx, args) in calls.iter().enumerate() {
        let mut env = Env { slots: vec![None; q.slot_count], root: Root::Site(idx) };
        for (i, a) in args.iter().enumerate() {
            env.slots[i] = a.clone().map(EnvVal::Val);
        }
        for b in 0..q.branches.len() {
            m.pool.entry((qi, b)).or_default().insert(0, env.clone());
        }
    }
    for b in 0..q.branches.len() {
        let batch = m.pool.get_mut(&(qi, b)).map(std::mem::take).unwrap_or_default();
        m.stack.push(Frame::Branch { q: qi, b, batch });
    }
    m.drain()?;
    debug_assert!(m.qmem.values().all(|v| v.is_empty()), "leftover nested results");
    Ok(std::mem::take(&mut m.site_out))
}

/// True when param `i` of query `qi` is bound in EVERY branch — directly
/// by a fact-pattern unification, or by threading into a call whose
/// corresponding param is (recursively) all-branches-bound. ?query CEs
/// require this of every UNBOUND arg (D-057): the emitted row must carry
/// a value at each param position.
pub fn param_bound_all_branches(queries: &[CompiledQuery], qi: usize, i: usize) -> bool {
    fn go(queries: &[CompiledQuery], qi: usize, i: usize, visiting: &mut Vec<(usize, usize)>) -> bool {
        if visiting.contains(&(qi, i)) {
            return true; // optimistic on cycles (self-recursion bottoms out at the base branch)
        }
        visiting.push((qi, i));
        let q = &queries[qi];
        let ok = q.branches.iter().all(|br| {
            br.iter().any(|n| match n {
                CNode::Fact(p) => p
                    .beta
                    .iter()
                    .any(|b| matches!(b.operand, Operand::Param(s) if s == i)),
                CNode::Call { callee, args } => args.iter().enumerate().any(|(j, a)| {
                    matches!(a, CArg::Slot(s) if *s == i)
                        && go(queries, *callee, j, visiting)
                }),
            })
        });
        visiting.pop();
        ok
    }
    go(queries, qi, i, &mut Vec::new())
}

fn slot_of(q: &CompiledQuery, ident: &str) -> usize {
    // idents are a subset of slots in slot order; params occupy the
    // prefix. Recompute by name (idents are few).
    q.params
        .iter()
        .position(|(n, _)| n == ident)
        .unwrap_or_else(|| {
            // non-param identifiers: locate via the ident list offset —
            // idents beyond params map to first-branch slots in order.
            // Slots are not stored by name at runtime, so recover from
            // the compile-time invariant: idents was built from the slot
            // table in order.
            q.ident_slots()[q.idents.iter().position(|i| i == ident).unwrap()]
        })
}

impl CompiledQuery {
    /// Param (name, type) view for ?query-CE compilation (D-056).
    pub fn params_view(&self) -> &[(String, FieldType)] {
        &self.params
    }

    /// slot index per identifier (params prefix + first-branch slots).
    fn ident_slots(&self) -> Vec<usize> {
        // params are slots 0..P; first-branch declarations follow in
        // allocation order. Rebuild by walking branch 0 the same way the
        // compiler allocated slots.
        let mut v: Vec<usize> = (0..self.params.len()).collect();
        let mut next = self.params.len();
        for (bi, br) in self.branches.iter().enumerate() {
            for n in br {
                match n {
                    CNode::Fact(p) => {
                        if let Some(s) = p.fact_slot {
                            if bi == 0 {
                                v.push(s);
                            }
                            next = next.max(s + 1);
                        }
                        for (s, _) in &p.field_binds {
                            if bi == 0 {
                                v.push(*s);
                            }
                            next = next.max(s + 1);
                        }
                    }
                    CNode::Call { args, .. } => {
                        for a in args {
                            if let CArg::Slot(s) = a {
                                if *s >= self.params.len() && *s >= next {
                                    if bi == 0 {
                                        v.push(*s);
                                    }
                                    next = *s + 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        v.sort();
        v.dedup();
        v
    }
}

impl Machine<'_> {
    fn drain(&mut self) -> Result<(), EngineError> {
        while let Some(frame) = self.stack.pop() {
            match frame {
                Frame::Branch { q, b, batch } => self.walk(q, b, 0, batch)?,
                Frame::Resume { q, b, node, mut trg } => {
                    let pending = self
                        .qmem
                        .get_mut(&(q, b, node))
                        .map(std::mem::take)
                        .unwrap_or_default();
                    trg.extend(pending);
                    self.walk(q, b, node + 1, trg)?;
                }
            }
        }
        Ok(())
    }

    fn bump(&mut self) -> Result<(), EngineError> {
        self.steps += 1;
        if self.steps > STEP_LIMIT {
            return Err(EngineError(
                "query evaluation step limit exceeded (cyclic recursion data? D-055)".into(),
            ));
        }
        Ok(())
    }

    fn walk(&mut self, qi: usize, bi: usize, mut ni: usize, mut src: Vec<Env>) -> Result<(), EngineError> {
        let branch = &self.queries[qi].branches[bi];
        while ni < branch.len() {
            self.bump()?;
            match &branch[ni] {
                CNode::Call { callee, args } => {
                    let site = (qi, bi, ni);
                    let trg = self
                        .qmem
                        .get_mut(&site)
                        .map(std::mem::take)
                        .unwrap_or_default();
                    if src.is_empty() {
                        // evalQueryNode with an empty src skips the call
                        // setup entirely (no frames, no pool takeAll) and
                        // evaluation CONTINUES at the next node — later
                        // fact levels still evaluate, so their memories
                        // drain this window (D-056 statefulness).
                        src = trg;
                        ni += 1;
                        continue;
                    }
                    self.stack.push(Frame::Resume { q: qi, b: bi, node: ni, trg });
                    let cq = &self.queries[*callee];
                    for env in &src {
                        let mut cenv = Env {
                            slots: vec![None; cq.slot_count],
                            root: Root::Nested(Rc::new(NestedRoot {
                                site,
                                caller: env.clone(),
                            })),
                        };
                        for (p, a) in cenv.slots.iter_mut().zip(args) {
                            *p = match a {
                                CArg::Lit(v) => Some(EnvVal::Val(v.clone())),
                                CArg::Slot(s) => env.slots[*s].clone(),
                            };
                        }
                        for b2 in 0..cq.branches.len() {
                            self.pool
                                .entry((*callee, b2))
                                .or_default()
                                .insert(0, cenv.clone());
                        }
                    }
                    for b2 in 0..cq.branches.len() {
                        let batch = self
                            .pool
                            .get_mut(&(*callee, b2))
                            .map(std::mem::take)
                            .unwrap_or_default();
                        self.stack.push(Frame::Branch { q: *callee, b: b2, batch });
                    }
                    return Ok(());
                }
                CNode::Fact(pat) => {
                    src = self.eval_fact_level((qi, bi, ni), pat, src)?;
                    ni += 1;
                }
            }
        }
        // terminal: route src head→tail by root
        for env in src {
            self.bump()?;
            match env.root.clone() {
                Root::Top => self.out.push(env),
                // rule-site row (D-056): PREPEND the full param row into
                // the site staging (rowAdded → addInsert)
                Root::Site(idx) => {
                    let q = &self.queries[qi];
                    let mut vals = Vec::with_capacity(q.params.len());
                    for s in 0..q.params.len() {
                        match &env.slots[s] {
                            Some(EnvVal::Val(v)) => vals.push(v.clone()),
                            _ => {
                                return Err(EngineError(
                                    "?query CE row left a param position unbound (D-057)"
                                        .into(),
                                ))
                            }
                        }
                    }
                    self.site_out.insert(0, (idx, vals));
                }
                Root::Nested(root) => {
                    let (cq_idx, args) = {
                        let (q, b, n) = root.site;
                        match &self.queries[q].branches[b][n] {
                            CNode::Call { callee, args } => (*callee, args.clone()),
                            _ => unreachable!("site is a call node"),
                        }
                    };
                    let _ = cq_idx;
                    let mut child = root.caller.clone();
                    let mut seen: Vec<usize> = Vec::new();
                    for (pos, a) in args.iter().enumerate() {
                        if let CArg::Slot(s) = a {
                            if root.caller.slots[*s].is_none() && !seen.contains(s) {
                                seen.push(*s);
                                child.slots[*s] = env.slots[pos].clone();
                            }
                        }
                    }
                    self.qmem.entry(root.site).or_default().insert(0, child);
                }
            }
        }
        Ok(())
    }

    fn eval_fact_level(
        &mut self,
        site: (usize, usize, usize),
        pat: &QPattern,
        src: Vec<Env>,
    ) -> Result<Vec<Env>, EngineError> {
        let store = self.store;
        let arrival: Vec<FactId> = drain_pattern(self.mem, store, site, pat);

        let index_fields: Vec<usize> = pat.index.iter().map(|&i| pat.beta[i].field_idx).collect();
        let table = if pat.index.is_empty() {
            None
        } else {
            Some(Table::build(store, &arrival, &index_fields, pat.seed)?)
        };
        let full_order = match (&table, pat.unification_join) {
            (Some(t), true) => Some(t.full_order()),
            _ => None,
        };

        let mut trg: Vec<Env> = Vec::new();
        for env in &src {
            self.bump()?;
            let candidates: Vec<FactId> = match (&table, pat.unification_join) {
                (None, _) => arrival.clone(),
                (Some(_), true) => full_order.clone().unwrap(),
                (Some(t), false) => {
                    let key: Vec<Value> = pat
                        .index
                        .iter()
                        .map(|&i| match &env.slots[operand_slot(&pat.beta[i].operand)] {
                            Some(EnvVal::Val(v)) => v.clone(),
                            _ => unreachable!("bucket key operands are bound by construction"),
                        })
                        .collect();
                    t.bucket(key_hash(pat.seed, &key), &key)
                }
            };
            'cand: for f in candidates {
                // D-052: constraints read the pattern-ENTRY env; the first
                // unbound site per param records the exit binding.
                let mut pending: Vec<(usize, Value)> = Vec::new();
                for b in &pat.beta {
                    let fv = store.value(f, b.field_idx);
                    let slot = operand_slot(&b.operand);
                    match &env.slots[slot] {
                        Some(EnvVal::Val(bound)) => {
                            if !eval_cmp_join_pub(&fv, b.op, bound) {
                                continue 'cand;
                            }
                        }
                        Some(EnvVal::Fact(_)) => unreachable!("fact operands rejected at compile"),
                        None => {
                            if !pending.iter().any(|(s, _)| *s == slot) {
                                pending.push((slot, fv));
                            }
                        }
                    }
                }
                let mut env2 = env.clone();
                for (slot, v) in pending {
                    env2.slots[slot] = Some(EnvVal::Val(v));
                }
                if let Some(slot) = pat.fact_slot {
                    env2.slots[slot] = Some(EnvVal::Fact(f));
                }
                for (slot, fi) in &pat.field_binds {
                    env2.slots[*slot] = Some(EnvVal::Val(store.value(f, *fi)));
                }
                trg.insert(0, env2); // PREPEND (staged-set LIFO)
            }
        }
        Ok(trg)
    }
}

fn operand_slot(o: &Operand) -> usize {
    match o {
        Operand::Param(s) | Operand::Binding(s) => *s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TypeSchema;

    fn person_store() -> FactStore {
        FactStore::new(vec![TypeSchema {
            name: "Person".into(),
            fields: vec![
                ("name".into(), FieldType::Str),
                ("age".into(), FieldType::I64),
            ],
        }])
    }

    /// Ground truth from live Drools table dumps (D-050, MemDump).
    #[test]
    fn hash_pipeline_matches_live_drools_dump() {
        let store = person_store();
        let tid = store.type_id("Person").unwrap();
        assert_eq!(extractor_index(&store, tid, 1), 1); // getAge
        assert_eq!(extractor_index(&store, tid, 0), 3); // getName (getClass=2)
        let mut seed: u32 = 31;
        seed = seed.wrapping_add(seed.wrapping_mul(31)).wrapping_add(1);
        assert_eq!(seed, 993);
        assert_eq!(key_hash(993, &[Value::I64(3)]), 32561);
        assert_eq!(key_hash(993, &[Value::I64(30)]), 32559);
        assert_eq!(key_hash(993, &[Value::I64(10)]), 32570);
        assert_eq!(key_hash(993, &[Value::I64(45)]), 32541);
    }

    /// MemDump3 shape matrix: accessor-sort extractor rule (D-050).
    #[test]
    fn extractor_index_accessor_sort_rule() {
        let store = FactStore::new(vec![TypeSchema {
            name: "Person4".into(),
            fields: vec![
                ("name".into(), FieldType::Str),
                ("city".into(), FieldType::Str),
                ("married".into(), FieldType::Bool),
                ("score".into(), FieldType::F64),
            ],
        }]);
        let tid = store.type_id("Person4").unwrap();
        assert_eq!(extractor_index(&store, tid, 1), 1); // city
        assert_eq!(extractor_index(&store, tid, 0), 3); // name
        assert_eq!(extractor_index(&store, tid, 3), 4); // score
        assert_eq!(extractor_index(&store, tid, 2), 6); // married (isMarried)
    }

    fn compile_all(store: &FactStore, drl_src: &str) -> Vec<CompiledQuery> {
        let file = drl::parse_file(drl_src).unwrap();
        let qs = compile_queries(store, file.queries, &[]).unwrap();
        validate_calls(&qs).unwrap();
        qs
    }

    /// q2_param_unify oracle-pinned row order (D-050).
    #[test]
    fn unbound_unification_row_order() {
        let mut store = person_store();
        let tid = store.type_id("Person").unwrap();
        for (n, a) in [("alice", 30), ("bob", 10), ("carol", 45), ("dave", 30)] {
            store
                .insert(tid, vec![Value::Str(n.into()), Value::I64(a)])
                .unwrap();
        }
        let qs = compile_all(&store, "query ByAge(long $a)\n    $p : Person(age == $a)\nend\n");
        let mut mem = QueryMem::default();
        let out = run_query(&store, &qs, &mut mem, "ByAge", &[None]).unwrap();
        let pi = out.identifiers.iter().position(|i| i == "$p").unwrap();
        let names: Vec<String> = out
            .rows
            .iter()
            .map(|r| match &r[pi] {
                QueryVal::Fact(fv) => match &fv.fields[0].1 {
                    Value::Str(s) => s.clone(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(names, ["bob", "alice", "dave", "carol"]);
        let out = run_query(&store, &qs, &mut mem, "ByAge", &[Some(Value::I64(30))]).unwrap();
        assert_eq!(out.rows.len(), 2);
    }

    /// qa4 transitive closure: exact closure content (order certified by
    /// the differential corpus).
    #[test]
    fn transitive_closure_counts() {
        let mut store = FactStore::new(vec![TypeSchema {
            name: "Location".into(),
            fields: vec![
                ("thing".into(), FieldType::Str),
                ("location".into(), FieldType::Str),
            ],
        }]);
        let tid = store.type_id("Location").unwrap();
        for (t, l) in [
            ("desk", "office"),
            ("chair", "office"),
            ("office", "house"),
            ("key", "drawer"),
            ("drawer", "desk"),
            ("pen", "desk"),
        ] {
            store
                .insert(tid, vec![Value::Str(t.into()), Value::Str(l.into())])
                .unwrap();
        }
        let qs = compile_all(
            &store,
            "query contained(String $x, String $y)\n    Location($x, $y;)\n    or\n    ( Location($z, $y;) and contained($x, $z;) )\nend\n",
        );
        let mut mem = QueryMem::default();
        let out = run_query(
            &store,
            &qs,
            &mut mem,
            "contained",
            &[Some(Value::Str("key".into())), Some(Value::Str("house".into()))],
        )
        .unwrap();
        assert_eq!(out.rows.len(), 1);
        let out = run_query(&store, &qs, &mut mem, "contained", &[None, None]).unwrap();
        assert_eq!(out.rows.len(), 15);
        // branch-2 local $z is not an identifier (params + first branch)
        assert_eq!(out.identifiers, vec!["$x", "$y"]);
    }

    /// D-055 walls reject out-of-shape recursion at compile time.
    #[test]
    fn recursion_walls() {
        let store = FactStore::new(vec![TypeSchema {
            name: "L".into(),
            fields: vec![("a".into(), FieldType::Str), ("b".into(), FieldType::Str)],
        }]);
        // left recursion
        let f = drl::parse_file(
            "query q(String $x, String $y)\n    L($x, $y;)\n    or\n    ( q($x, $z;) and L($z, $y;) )\nend\n",
        )
        .unwrap();
        assert!(compile_queries(&store, f.queries, &[]).is_err());
        // recursive branch first
        let f = drl::parse_file(
            "query q(String $x, String $y)\n    ( L($z, $y;) and q($x, $z;) )\n    or\n    L($x, $y;)\nend\n",
        )
        .unwrap();
        assert!(compile_queries(&store, f.queries, &[]).is_err());
        // 3-branch recursive
        let f = drl::parse_file(
            "query q(String $x, String $y)\n    L($x, $y;)\n    or\n    ( L($z, $y;) and q($x, $z;) )\n    or\n    L($y, $x;)\nend\n",
        )
        .unwrap();
        assert!(compile_queries(&store, f.queries, &[]).is_err());
        // mutual recursion
        let f = drl::parse_file(
            "query a(String $x)\n    L($x, $z;)\n    b($z;)\nend\nquery b(String $x)\n    L($x, $z;)\n    a($z;)\nend\n",
        )
        .unwrap();
        assert!(compile_queries(&store, f.queries, &[]).is_err());
    }
}
