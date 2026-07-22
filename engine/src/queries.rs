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

/// A nested query call threads its caller through `caller: Env`, whose
/// own `root` may be another `Root::Nested` — so a deep recursion builds
/// an Rc-linked chain as long as the recursion depth. The default
/// destructor descends that chain recursively and overflows the native
/// stack (SIGSEGV, no `STEP_LIMIT` on the drop path) on cyclic data or a
/// very deep acyclic recursion. Flatten the drop into a loop: steal each
/// node's caller-root and null it before letting the node fall, so every
/// nested node we free already has a `Top` root and its own drop returns
/// immediately. A still-shared link (refcount > 1) stops the walk — its
/// surviving owner flattens from there when it drops.
impl Drop for NestedRoot {
    fn drop(&mut self) {
        let mut node = std::mem::replace(&mut self.caller.root, Root::Top);
        while let Root::Nested(rc) = node {
            match Rc::try_unwrap(rc) {
                Ok(mut inner) => node = std::mem::replace(&mut inner.caller.root, Root::Top),
                Err(_) => break,
            }
        }
    }
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
        Value::Null => 0, // unreachable: nullable types are walled from queries (D-097)
        Value::Dec { .. } => 0, // unreachable: Dec walled from queries (D-098)
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
        // queries are walled from nullable types (D-097); a null literal
        // in a query body is rejected at compile before this point
        Literal::Null => Value::Null,
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
    // D-097 wall: queries over types with nullable fields are out of
    // subset for the data-types arc (3VL x query-stack-machine
    // unprobed; liftable with its own ladder).
    for b in &def.branches {
        for pat in b {
            if let Some(tid) = store.type_id(&pat.name) {
                let sch = store.schema(tid);
                if sch.nullable != 0 {
                    return err(format!(
                        "{} has nullable fields — queries over nullable types are walled (D-097)",
                        pat.name
                    ));
                }
                if sch.fields.iter().any(|(_, ft)| matches!(ft, FieldType::Dec { .. })) {
                    return err(format!(
                        "{} has decimal fields — queries over decimal types are walled (D-098)",
                        pat.name
                    ));
                }
            }
        }
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
                    Constraint::Temporal { .. } => {
                        return err(
                            "temporal constraints in query bodies are out of subset (D-101)"
                                .into(),
                        )
                    }
                    Constraint::Group(_) => {
                        return err(
                            "inline constraint groups in query bodies are out of subset (D-073)"
                                .into(),
                        )
                    }
                    Constraint::ArithCmp { .. } => {
                        return err(
                            "arithmetic constraints in query bodies are out of subset (D-291)"
                                .into(),
                        )
                    }
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
                                            "param {v} used with a non-== operator (unification is == only, \
                                             D-051). For threshold/range filtering, put the value in a FACT \
                                             and join: Threshold($t : value) then Account(balance > $t) — \
                                             certified, and the threshold updates like any fact. Or return \
                                             the rows and filter in Python"
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
    cap: u32,
}

const TABLE_LEN: u32 = 128;
/// Backstop for cyclic recursion data (Drools HANGS there, D-055): total
/// stack pushes across one top-level call.
const STEP_LIMIT: usize = 1_000_000;

impl Table {
    /// Models drools-core's TupleIndexHashTable population for one
    /// evaluation (the D-253 recon; the old >96-key fence is LIFTED):
    ///
    /// 1. LIFO FLUSH — staged inserts prepend and the flush walks
    ///    insertFirst (TupleSetsImpl/PhreakJoinNode), so keys enter in
    ///    REVERSE arrival order.
    /// 2. BULK PRE-SIZE — a batch of >32 tuples calls
    ///    ensureCapacity(N) first: capacity doubles from 128 while
    ///    size+N exceeds 0.75*capacity. The table is EMPTY at build
    ///    (fresh-call model), so this moves no chains.
    /// 3. Each new key's list is HEAD-inserted into its bucket
    ///    (getOrCreate).
    /// 4. INCREMENTAL RESIZE — post-add, when pre-add distinct count
    ///    reaches 0.75*capacity (`size++ >= threshold`): capacity
    ///    doubles; the transfer walks each old chain head->tail and
    ///    head-inserts (AbstractHashTable.resize), so same-new-bucket
    ///    runs REVERSE; buckets split (hash & (2len-1)), never merge.
    ///
    /// `arrival` is the pattern-memory drain — ALREADY newest-first
    /// (reverse-insertion, this module's header) — so iterating it
    /// forward IS the LIFO flush; head-inserting it yields exactly the
    /// physical chains the D-253 dumps show (oldest key at bucket
    /// head), and the stack machine's prepend/append plumbing supplies
    /// the reversed emission. The no-resize path is byte-identical to
    /// the certified <=96 build (unchanged code path). Facts WITHIN a
    /// key keep memory order: Drools transfers whole TupleLists, so
    /// resize never reorders them.
    fn build(
        store: &FactStore,
        arrival: &[FactId],
        fields: &[usize],
        seed: u32,
    ) -> Result<Table, EngineError> {
        let mut cap: u32 = TABLE_LEN;
        let mut slots: Vec<Vec<KeyList>> = (0..cap).map(|_| Vec::new()).collect();
        let transfer = |slots: &mut Vec<Vec<KeyList>>, cap: &mut u32, newcap: u32| {
            let mut ns: Vec<Vec<KeyList>> = (0..newcap).map(|_| Vec::new()).collect();
            for chain in slots.drain(..) {
                for kl in chain {
                    // head->tail walk + head-insert: the reversing transfer
                    ns[(kl.hash & (newcap - 1)) as usize].insert(0, kl);
                }
            }
            *slots = ns;
            *cap = newcap;
        };
        // bulk pre-size (empty here, so nothing reverses)
        let n = arrival.len();
        if n > 32 && n > (cap as usize * 3) / 4 {
            let mut newcap = cap * 2;
            while (newcap as usize) < n {
                newcap *= 2;
            }
            transfer(&mut slots, &mut cap, newcap);
        }
        let mut distinct = 0usize;
        for &f in arrival {
            let key: Vec<Value> = fields.iter().map(|&fi| store.value(f, fi)).collect();
            let h = key_hash(seed, &key);
            let slot = (h & (cap - 1)) as usize;
            if let Some(kl) = slots[slot]
                .iter_mut()
                .find(|kl| kl.hash == h && key_eq(&kl.key, &key))
            {
                kl.facts.push(f);
            } else {
                slots[slot].insert(0, KeyList { hash: h, key, facts: vec![f] });
                let presize = distinct;
                distinct += 1;
                if presize >= (cap as usize * 3) / 4 {
                    let newcap = cap * 2;
                    transfer(&mut slots, &mut cap, newcap);
                }
            }
        }
        Ok(Table { slots, cap })
    }

    fn full_order(&self) -> Vec<FactId> {
        self.slots
            .iter()
            .flat_map(|chain| chain.iter().flat_map(|kl| kl.facts.iter().copied()))
            .collect()
    }

    fn bucket(&self, hash: u32, key: &[Value]) -> Vec<FactId> {
        let slot = (hash & (self.cap - 1)) as usize;
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
pub struct QueryMem(
    HashMap<(usize, usize, usize), Vec<FactId>>,
    /// D-363: per-site drain-window fact lists (one entry per drain
    /// that appended fresh facts, in drain order). The top-level
    /// full-walk reorder needs the window partition; fact-id lists
    /// survive deletions (membership lookups filter to the live walk).
    HashMap<(usize, usize, usize), Vec<Vec<FactId>>>,
    /// D-367: per-site (type_gen, type_mut_gen, by_type high-water mark)
    /// at the last drain. Equal type_gen: the drain is a NO-OP by
    /// construction — the retain removes nothing (liveness and field
    /// values unchanged), the fresh scan finds nothing (same live set,
    /// all seen), no window record — skipped outright. Equal mut_gen
    /// only (inserts since, no kill/set_value): the retain is still a
    /// no-op and every pre-hwm fact is already seen-or-alpha-failed
    /// with unchanged values, so only the post-hwm handles are tested —
    /// the identical fresh list the full walk would produce.
    HashMap<(usize, usize, usize), (u64, u64, u32)>,
);

/// One drain window for one pattern (qx8_statemem/3): staged deletes
/// leave; staged alpha-passing inserts append NEWEST-FIRST after the
/// existing batches. The memory order IS the arrival order.
fn drain_pattern(
    mem: &mut QueryMem,
    store: &FactStore,
    site: (usize, usize, usize),
    pat: &QPattern,
) -> Vec<FactId> {
    drain_pattern_update(mem, store, site, pat);
    mem.0.get(&site).cloned().unwrap_or_default()
}

/// The stateful half of `drain_pattern` (D-367 split): performs the
/// drain without cloning the memory out, so state-only callers
/// (`drain_query`) skip the O(|memory|) copy — and skips entirely when
/// the pattern type's generation is unchanged since the last drain.
fn drain_pattern_update(
    mem: &mut QueryMem,
    store: &FactStore,
    site: (usize, usize, usize),
    pat: &QPattern,
) {
    let gen = store.type_gen(pat.tid);
    let mgen = store.type_mut_gen(pat.tid);
    let hwm = store.by_type_len(pat.tid);
    let alpha_ok = |f: FactId| {
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
    };
    match mem.2.get(&site) {
        // clean: the drain would be a no-op (see QueryMem.2)
        Some(&(g, _, _)) if g == gen => return,
        // insert-only delta: the retain is a no-op and pre-hwm facts
        // are frozen — test only the new handles (see QueryMem.2)
        Some(&(_, mg, from)) if mg == mgen => {
            let mut fresh: Vec<FactId> =
                store.facts_of_since(pat.tid, from).filter(|&f| alpha_ok(f)).collect();
            fresh.reverse();
            if !fresh.is_empty() {
                mem.1.entry(site).or_default().push(fresh.clone()); // D-363 window record
            }
            mem.0.entry(site).or_default().extend(fresh);
            mem.2.insert(site, (gen, mgen, hwm));
            return;
        }
        _ => {}
    }
    let m = mem.0.entry(site).or_default();
    // D-107 (qm1): an external UPDATE can flip an accumulated fact out
    // of the pattern — the window re-tests alpha at every drain (still-
    // passing facts keep their qx8-pinned accumulation).
    m.retain(|f| store.is_alive(*f) && alpha_ok(*f));
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
    if !fresh.is_empty() {
        mem.1.entry(site).or_default().push(fresh.clone()); // D-363 window record
    }
    m.extend(fresh);
    mem.2.insert(site, (gen, mgen, hwm));
}

/// D-367: the summed member-type generation of a query's own fact
/// patterns — the `query_linked` memo stamp. Generations are monotone,
/// so any member-type mutation strictly raises the sum; an equal stamp
/// proves the linkedness inputs are untouched.
pub fn linked_stamp(store: &FactStore, queries: &[CompiledQuery], qi: usize) -> u64 {
    queries[qi]
        .branches
        .iter()
        .flat_map(|b| b.iter())
        .filter_map(|n| match n {
            CNode::Fact(p) => Some(store.type_gen(p.tid)),
            _ => None,
        })
        .sum()
}

/// D-086: a query's path is LINKED when some or-branch has every
/// positive fact pattern's alpha populated by at least one live fact.
/// An armed query's agenda item queues on WM events only while linked
/// (fz_min_3959: an unlinked branch accumulates staged facts into ONE
/// drain window that opens at the linking event).
pub fn query_linked(store: &FactStore, queries: &[CompiledQuery], qi: usize) -> bool {
    queries[qi].branches.iter().any(|branch| {
        branch.iter().all(|node| match node {
            CNode::Fact(pat) => store.live_facts_of(pat.tid).any(|f| {
                pat.alpha.iter().all(|(fi, t)| {
                    let v = store.value(f, *fi);
                    match t {
                        AlphaTest::Cmp { op, rhs } => eval_cmp_pub(&v, *op, rhs),
                        AlphaTest::Matches(r) => {
                            matches!(&v, Value::Str(s) if r.accepts(s))
                        }
                        AlphaTest::Contains(n) => {
                            matches!(&v, Value::Str(s) if s.contains(n.as_str()))
                        }
                        AlphaTest::InList { items, negated } => {
                            let hit = items.iter().any(|i| eval_cmp_pub(&v, CmpOp::Eq, i));
                            hit != *negated
                        }
                    }
                })
            }),
            _ => true,
        })
    })
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
                drain_pattern_update(mem, store, (qi, bi, ni), pat);
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
    /// Per-run memo of each fact pattern's drained arrival + built index
    /// (keyed by site). Working memory is frozen for the duration of one
    /// Machine run and `drain_pattern` is idempotent within it, so a
    /// self-recursive query that re-enters the same site at every depth
    /// would otherwise re-drain and rebuild the identical O(N) table at
    /// each level — quadratic. Computing it once collapses that to O(N)
    /// total; the memoized table is byte-identical to the per-level one,
    /// so emission order and multiplicity are unchanged.
    level_cache: HashMap<(usize, usize, usize), (Vec<FactId>, Option<Table>, Option<Vec<FactId>>)>,
    /// D-367 RESULT memo (intra-run; the same WM freeze): per (callee,
    /// uniform bound-arg vector), the callee's qmem emission parsed
    /// into flush SEGMENTS — per segment one contiguous BLOCK per
    /// caller, every block identical to the single-caller row
    /// sequence, block order forward or reverse (the flat-list
    /// machinery is env-by-env expansion + whole-list reversals +
    /// front-splices, all caller-block-lockstep-preserving). Captured
    /// once by a TWO-probe evaluation and VALIDATED by the parse: a
    /// capture that does not decompose into equal-block segments
    /// stores `None` and the call evaluates for real — the theory is
    /// runtime-checked per (callee, argvec), never assumed. Value keys
    /// compare with derived equality (a NaN argvec never hits — it
    /// probes again, correct and merely slower). Linear scan: distinct
    /// argvecs per run are few.
    memo: Vec<((usize, Vec<Option<Value>>), Option<MemoEntry>)>,
    /// Non-zero while a probe evaluation runs. Probes nest native
    /// frames (probe -> drain -> walk), so memoization is fenced to
    /// depth 0 — below one probe the machine stays fully iterative
    /// (the D-055 deep-recursion guarantee).
    probe_depth: u32,
}

/// D-367: one memoized callee evaluation (see `Machine::memo`).
/// Fills are keyed by CALLEE ARG POSITION, never by caller slot — the
/// same (callee, argvec) is reachable from call sites with different
/// arg-slot structures (qc3_sibling's direct($x,$y) vs direct($z,$y)
/// pinned this at the byte gate: slot-keyed fills wrote the wrong
/// slots at the second site and manufactured an open-recursion loop).
/// The replay maps positions onto the REPLAYING site's slots with the
/// terminal handler's own rule (unbound slots, first position per
/// slot).
struct MemoEntry {
    /// The unbound arg positions, in position order.
    fill_pos: Vec<usize>,
    /// Flush segments: (callers forward?, block rows). Each block row
    /// holds the callee's param values at `fill_pos` positions.
    segments: Vec<(bool, Vec<Vec<Option<EnvVal>>>)>,
}

/// Strict equality for fill/argvec comparison (derived Value equality:
/// F64 by value semantics — NaN never equal, which FENCES rather than
/// corrupts: mismatches disable the memo for that call).
fn envval_opt_eq(a: &Option<EnvVal>, b: &Option<EnvVal>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(EnvVal::Fact(x)), Some(EnvVal::Fact(y))) => x == y,
        (Some(EnvVal::Val(x)), Some(EnvVal::Val(y))) => x == y,
        _ => false,
    }
}

/// D-363 (xf_fz_296002_1494/f5/p363a): at a TOP-LEVEL call on a
/// multi-branch query, when another branch is alpha-populated, a
/// single-fact-pattern branch's rows re-partition by drain window:
/// post-pull windows FIFO (within-window row order kept), the first
/// (pull-time) window LAST. Every no-conjunction shape matches the
/// engine's composition as-is (pr_qe_e1..e10, p363b, p363c), and
/// indexed/join branches (bound-arg walks) are untouched — only the
/// full-walk enumeration branch reorders. The qce pull path
/// (run_site) never calls this.
fn d363_reorder(
    store: &FactStore,
    queries: &[CompiledQuery],
    mem: &QueryMem,
    qi: usize,
    args: &[Option<Value>],
    single_pull_site: bool,
    out: &mut [Env],
) {
    let q = &queries[qi];
    // The law was extracted from single-pull-site histories; a query
    // pulled from several rule sites accumulates windows the
    // first-window-last reading mis-partitions (fz_9101_7133's
    // queries[1] pinned this at the byte gate) — fenced.
    if !single_pull_site || q.branches.len() < 2 {
        return;
    }
    let alpha_live = |pat: &QPattern| -> bool {
        store.live_facts_of(pat.tid).any(|f| {
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
    };
    for b in 0..q.branches.len() {
        let facts: Vec<(usize, &QPattern)> = q.branches[b]
            .iter()
            .enumerate()
            .filter_map(|(ni, n)| match n {
                CNode::Fact(p) => Some((ni, p)),
                _ => None,
            })
            .collect();
        let [(ni, pat)] = facts[..] else { continue }; // single-fact-pattern branches only
        let Some(slot) = pat.fact_slot else { continue };
        let Some(windows) = mem.1.get(&(qi, b, ni)) else { continue };
        if windows.len() < 2 {
            continue;
        }
        let populated = (0..q.branches.len()).any(|b2| {
            b2 != b
                && q.branches[b2].iter().any(|n| matches!(n, CNode::Fact(_)))
                && q.branches[b2].iter().all(|n| match n {
                    CNode::Fact(p) => alpha_live(p),
                    _ => true,
                })
        });
        if !populated {
            continue;
        }
        // The walk is index-BUCKETED by the first indexed key (false
        // bucket before true — p363a AND the witness both put the
        // false facts at the head); within a bucket the post-pull
        // windows go FIFO and the pull-window members last. Non-bool
        // first keys are unprobed — fence.
        let Some(&bi) = pat.index.first() else { continue };
        let key_field = pat.beta[bi].field_idx;
        // The law is a FULL-WALK law: a call binding the key argument
        // is an indexed lookup with its own certified order
        // (fz_9101_7133's Q0(true,..) call pinned this at the byte
        // gate) — reorder only when the key's param is UNBOUND.
        match pat.beta[bi].operand {
            Operand::Param(s) => {
                if args.get(s).map_or(true, |a| a.is_some()) {
                    continue;
                }
            }
            Operand::Binding(_) => continue,
        }
        // D-367: fact -> first-containing-window index, built once —
        // entry-or-insert keeps the FIRST hit, the exact value the old
        // per-row windows.position(contains) scan produced.
        let mut wmap: HashMap<FactId, usize> = HashMap::new();
        for (wi, w) in windows.iter().enumerate() {
            for &f in w {
                wmap.entry(f).or_insert(wi);
            }
        }
        let rank = |f: FactId| -> Option<(u8, usize)> {
            let bucket = match store.value(f, key_field) {
                Value::Bool(b) => b as u8,
                _ => return None, // non-bool key: fence
            };
            wmap.get(&f)
                .map(|&wi| (bucket, if wi == 0 { usize::MAX } else { wi }))
        };
        let idx: Vec<usize> = (0..out.len())
            .filter(|&i| matches!(out[i].slots.get(slot), Some(Some(EnvVal::Fact(_)))))
            .collect();
        let mut keyed: Vec<((u8, usize), Env)> = Vec::with_capacity(idx.len());
        for &i in &idx {
            let Some(Some(EnvVal::Fact(f))) = out[i].slots.get(slot) else { return };
            let Some(r) = rank(*f) else { return }; // unknown fact/key: fence
            keyed.push((r, out[i].clone()));
        }
        keyed.sort_by_key(|(r, _)| *r); // stable: within-window order kept
        for (pos, (_, env)) in idx.iter().zip(keyed) {
            out[*pos] = env;
        }
    }
}

pub fn run_query(
    store: &FactStore,
    queries: &[CompiledQuery],
    mem: &mut QueryMem,
    name: &str,
    args: &[Option<Value>],
    single_pull_site: bool,
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
        level_cache: HashMap::new(),
        memo: Vec::new(),
        probe_depth: 0,
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
    let mut out_envs = std::mem::take(&mut m.out);
    drop(m);
    d363_reorder(store, queries, mem, qi, args, single_pull_site, &mut out_envs);
    let idents = q.idents.clone();
    let rows = out_envs
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
        level_cache: HashMap::new(),
        memo: Vec::new(),
        probe_depth: 0,
    };
    let mut envs: Vec<Env> = Vec::with_capacity(calls.len());
    for (idx, args) in calls.iter().enumerate() {
        let mut env = Env { slots: vec![None; q.slot_count], root: Root::Site(idx) };
        for (i, a) in args.iter().enumerate() {
            env.slots[i] = a.clone().map(EnvVal::Val);
        }
        envs.push(env);
    }
    // pool = reverse of src (see the doc comment): the per-call insert(0)
    // built [env_k..env_1]; one reversed extend is identical, O(k) (D-300)
    for b in 0..q.branches.len() {
        m.pool.entry((qi, b)).or_default().extend(envs.iter().rev().cloned());
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
            // D-367: no 'cyclic?' guess — a finite DAG workload can trip
            // this too (fz_9201_1660 proved it before the result memo).
            return Err(EngineError(
                "query evaluation step limit exceeded (D-055 backstop: cyclic data or an expansion past the step budget)".into(),
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
                    // D-367: a uniform-argvec batch replays the callee's
                    // memoized emission instead of re-deriving it per
                    // caller (see `Machine::memo`). Fenced (None) paths
                    // fall through to the real frame evaluation.
                    if let Some(rows) = self.try_memo_call(qi, bi, ni, *callee, &src)? {
                        let mut cont = trg;
                        cont.extend(rows);
                        src = cont;
                        ni += 1;
                        continue;
                    }
                    self.stack.push(Frame::Resume { q: qi, b: bi, node: ni, trg });
                    let cq = &self.queries[*callee];
                    let mut cenvs: Vec<Env> = Vec::with_capacity(src.len());
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
                        cenvs.push(cenv);
                    }
                    // per-env insert(0) made each pool [cenv_k..cenv_1] ++
                    // pre-existing; one front-splice of the reversed block
                    // is the identical order, O(k) (D-300)
                    for b2 in 0..cq.branches.len() {
                        self.pool
                            .entry((*callee, b2))
                            .or_default()
                            .splice(0..0, cenvs.iter().rev().cloned());
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
        // terminal: route src head→tail by root. The row-scale PREPEND
        // targets (site_out, per-site qmem pending) collect in loop order
        // and front-splice REVERSED after the loop — the identical order
        // the per-row insert(0) produced, without the O(rows²) shift
        // (D-300; on an error return the partial prepends are dropped with
        // the run — every query EngineError is scenario-fatal).
        let mut site_rows: Vec<(usize, Vec<Value>)> = Vec::new();
        let mut qmem_rows: Vec<((usize, usize, usize), Env)> = Vec::new();
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
                    site_rows.push((idx, vals));
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
                    qmem_rows.push((root.site, child));
                }
            }
        }
        self.site_out.splice(0..0, site_rows.into_iter().rev());
        // group by site in loop order (site count is program-bounded),
        // then front-splice each site's reversed block
        let mut by_site: Vec<((usize, usize, usize), Vec<Env>)> = Vec::new();
        for (site, child) in qmem_rows {
            match by_site.iter_mut().find(|(s, _)| *s == site) {
                Some((_, v)) => v.push(child),
                None => by_site.push((site, vec![child])),
            }
        }
        for (site, rows) in by_site {
            self.qmem
                .entry(site)
                .or_default()
                .splice(0..0, rows.into_iter().rev());
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
        // Compute this site's drained arrival + index once per run (see
        // `level_cache`): a self-recursive query re-enters the same site at
        // every depth, and WM is frozen here, so the drain+build is
        // identical each level. Take the memo out to work with owned
        // locals (keeps `self.bump()` borrow-free in the loop), then
        // restore it.
        if !self.level_cache.contains_key(&site) {
            let arrival: Vec<FactId> = drain_pattern(self.mem, store, site, pat);
            let index_fields: Vec<usize> =
                pat.index.iter().map(|&i| pat.beta[i].field_idx).collect();
            let table = if pat.index.is_empty() {
                None
            } else {
                Some(Table::build(store, &arrival, &index_fields, pat.seed)?)
            };
            let full_order = match (&table, pat.unification_join) {
                (Some(t), true) => Some(t.full_order()),
                _ => None,
            };
            self.level_cache.insert(site, (arrival, table, full_order));
        }
        let (arrival, table, full_order) = self.level_cache.remove(&site).unwrap();

        // D-367: AUX equality bucket for the unbound-index descent. A
        // recursive call threads a concrete value into an eq beta that
        // is NOT the D-053 index (the index is the FIRST unification
        // alone), and the old path full-order-scanned per env. When the
        // bound value's type EXACTLY matches the field type and is not
        // F64 (I64/Str/Bool: join-eq is exact equality — no D-020
        // coercion, no bit-pattern/NaN split), a bucket keyed on that
        // field over the full_order sequence returns precisely the
        // full-order facts passing that beta, in full-order sequence —
        // the identical survivor walk, without the O(arrival) scan per
        // env. F64 and cross-type keys keep the scan (join-eq and
        // bucket equality diverge there).
        let aux_bi: Option<usize> = pat.beta.iter().enumerate().position(|(i, b)| {
            b.op == CmpOp::Eq
                && !pat.index.contains(&i)
                && matches!(
                    store.field_type(pat.tid, b.field_idx),
                    FieldType::I64 | FieldType::Str | FieldType::Bool
                )
        });
        let mut aux: Option<HashMap<u32, Vec<(Value, Vec<FactId>)>>> = None;

        let mut trg: Vec<Env> = Vec::new();
        for env in &src {
            self.bump()?;
            let owned: Vec<FactId>;
            let candidates: &[FactId] = match (&table, pat.unification_join) {
                (None, _) => &arrival,
                (Some(t), true) => {
                    // A single-field unification join (index on a query
                    // param) full-scans when the param is UNBOUND (top-level
                    // open call binds it from each fact). When it is BOUND
                    // — every self-recursive call threads a concrete value —
                    // hash-bucket on it instead: all facts sharing the key
                    // sit in one contiguous KeyList, so the bucket yields
                    // exactly the full-order survivors in identical order,
                    // but in O(1) rather than O(N). This is what turns the
                    // recursive descent from O(N^2) into O(N).
                    let vals: Vec<Option<Value>> = pat
                        .index
                        .iter()
                        .map(|&i| match &env.slots[operand_slot(&pat.beta[i].operand)] {
                            Some(EnvVal::Val(v)) => Some(v.clone()),
                            _ => None,
                        })
                        .collect();
                    if vals.iter().all(|v| v.is_some()) {
                        let k: Vec<Value> = vals.into_iter().flatten().collect();
                        owned = t.bucket(key_hash(pat.seed, &k), &k);
                        &owned
                    } else {
                        // Index operand unbound: the full-order walk —
                        // served from the aux equality bucket when one
                        // applies to this env (identical survivors and
                        // order, see the aux_bi comment).
                        let mut cand: Option<&[FactId]> = None;
                        if let Some(bi) = aux_bi {
                            let b = &pat.beta[bi];
                            if let Some(EnvVal::Val(bound)) =
                                &env.slots[operand_slot(&b.operand)]
                            {
                                if bound.type_of() == store.field_type(pat.tid, b.field_idx)
                                {
                                    if aux.is_none() {
                                        let mut mm: HashMap<u32, Vec<(Value, Vec<FactId>)>> =
                                            HashMap::new();
                                        for &f in full_order.as_ref().unwrap() {
                                            let fv = store.value(f, b.field_idx);
                                            let h = java_hash(&fv);
                                            let chain = mm.entry(h).or_default();
                                            match chain.iter_mut().find(|(k, _)| *k == fv) {
                                                Some((_, fs)) => fs.push(f),
                                                None => chain.push((fv, vec![f])),
                                            }
                                        }
                                        aux = Some(mm);
                                    }
                                    cand = Some(
                                        aux.as_ref()
                                            .unwrap()
                                            .get(&java_hash(bound))
                                            .and_then(|chain| {
                                                chain.iter().find(|(k, _)| k == bound)
                                            })
                                            .map(|(_, fs)| &fs[..])
                                            .unwrap_or(&[]),
                                    );
                                }
                            }
                        }
                        match cand {
                            Some(c) => c,
                            None => full_order.as_ref().unwrap(),
                        }
                    }
                }
                (Some(t), false) => {
                    let key: Vec<Value> = pat
                        .index
                        .iter()
                        .map(|&i| match &env.slots[operand_slot(&pat.beta[i].operand)] {
                            Some(EnvVal::Val(v)) => v.clone(),
                            _ => unreachable!("bucket key operands are bound by construction"),
                        })
                        .collect();
                    owned = t.bucket(key_hash(pat.seed, &key), &key);
                    &owned
                }
            };
            'cand: for &f in candidates {
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
                trg.push(env2);
            }
        }
        self.level_cache.insert(site, (arrival, table, full_order));
        // PREPEND (staged-set LIFO): one reversal replaces the per-emission
        // insert(0) — same final order, O(k) instead of O(k²) (D-300; the
        // D-254-filed prepend-stage quadratic, 1.26M-env hang class)
        trg.reverse();
        Ok(trg)
    }

    /// D-367: attempt the memoized-call path (see `Machine::memo`).
    /// Ok(None) = fenced — evaluate for real. Ok(Some(rows)) = the
    /// children the Resume would have taken from qmem, in emission
    /// order.
    fn try_memo_call(
        &mut self,
        qi: usize,
        bi: usize,
        ni: usize,
        callee: usize,
        src: &[Env],
    ) -> Result<Option<Vec<Env>>, EngineError> {
        if self.probe_depth > 0 {
            return Ok(None); // stay iterative below one probe (D-055)
        }
        // Fence: a non-empty callee pool means this call would sweep
        // pre-staged envs into its batch (run_query's top-level
        // self-recursion, "pools may be swept early") — out of class.
        for b2 in 0..self.queries[callee].branches.len() {
            if self.pool.get(&(callee, b2)).is_some_and(|p| !p.is_empty()) {
                return Ok(None);
            }
        }
        let args: Vec<CArg> = match &self.queries[qi].branches[bi][ni] {
            CNode::Call { args, .. } => args.clone(),
            _ => unreachable!("memo call on a non-call node"),
        };
        // Uniform argvec fence: every caller must present the same
        // bound-arg vector (fact-valued args are out of class).
        let mut argvec: Vec<Option<Value>> = Vec::with_capacity(args.len());
        for a in &args {
            match a {
                CArg::Lit(v) => argvec.push(Some(v.clone())),
                CArg::Slot(s) => match &src[0].slots[*s] {
                    Some(EnvVal::Val(v)) => argvec.push(Some(v.clone())),
                    None => argvec.push(None),
                    Some(EnvVal::Fact(_)) => return Ok(None),
                },
            }
        }
        // Fence: a caller slot repeated across UNBOUND arg positions
        // makes per-position fills unrecoverable from a probe child
        // (the shared slot holds only the first position's value).
        {
            let mut unbound_slots: Vec<usize> = Vec::new();
            for (pos, a) in args.iter().enumerate() {
                if let CArg::Slot(s) = a {
                    if argvec[pos].is_none() {
                        if unbound_slots.contains(s) {
                            return Ok(None);
                        }
                        unbound_slots.push(*s);
                    }
                }
            }
        }
        for env in &src[1..] {
            for (pos, a) in args.iter().enumerate() {
                if let CArg::Slot(s) = a {
                    let same = match (&env.slots[*s], &argvec[pos]) {
                        (None, None) => true,
                        (Some(EnvVal::Val(v)), Some(w)) => v == w,
                        _ => false,
                    };
                    if !same {
                        return Ok(None);
                    }
                }
            }
        }
        let idx = match self
            .memo
            .iter()
            .position(|(k, _)| k.0 == callee && k.1 == argvec)
        {
            Some(i) => i,
            None => {
                let entry = self.probe_call(qi, (qi, bi, ni), callee, &args, &argvec)?;
                self.memo.push(((callee, argvec), entry));
                self.memo.len() - 1
            }
        };
        if self.memo[idx].1.is_none() {
            return Ok(None); // capture did not parse — permanent fallback
        }
        // Map fill POSITIONS onto THIS site's slots with the terminal
        // handler's own rule (unbound arg slots, first position per
        // slot — the seen dedup).
        let fill_map: Vec<(usize, usize)> = {
            let entry = self.memo[idx].1.as_ref().unwrap();
            let mut m: Vec<(usize, usize)> = Vec::new(); // (row col k, slot)
            let mut seen: Vec<usize> = Vec::new();
            for (k, &pos) in entry.fill_pos.iter().enumerate() {
                if let CArg::Slot(s) = args[pos] {
                    if !seen.contains(&s) {
                        seen.push(s);
                        m.push((k, s));
                    }
                }
            }
            m
        };
        // Replay: per segment, one block per caller in the segment's
        // direction. Steps: one bump per caller per segment — the
        // replayed rows are counted again at whatever level consumes
        // them, so per-row double-bumping here would only shrink the
        // runaway backstop's headroom.
        let mut out: Vec<Env> = Vec::new();
        let seg_count = self.memo[idx].1.as_ref().unwrap().segments.len();
        for si in 0..seg_count {
            let forward = self.memo[idx].1.as_ref().unwrap().segments[si].0;
            let order: Vec<usize> = if forward {
                (0..src.len()).collect()
            } else {
                (0..src.len()).rev().collect()
            };
            for ci in order {
                self.bump()?;
                let entry = self.memo[idx].1.as_ref().unwrap();
                let block = &entry.segments[si].1;
                for row in block {
                    let mut child = src[ci].clone();
                    for &(k, slot) in &fill_map {
                        child.slots[slot] = row[k].clone();
                    }
                    out.push(child);
                }
            }
        }
        Ok(Some(out))
    }

    /// D-367: evaluate the callee ONCE with two probe callers on a
    /// swapped-out machine state, capture its qmem emission, and parse
    /// it into lockstep segments. Shares level_cache, the pattern
    /// memories, and the step counter with the real run — the drains a
    /// probe performs are exactly the drains the real evaluation would
    /// perform first (level_cache first-touch), and cyclic data trips
    /// the step limit inside the probe with the identical error.
    fn probe_call(
        &mut self,
        caller_qi: usize,
        site: (usize, usize, usize),
        callee: usize,
        args: &[CArg],
        argvec: &[Option<Value>],
    ) -> Result<Option<MemoEntry>, EngineError> {
        let saved_pool = std::mem::take(&mut self.pool);
        let saved_qmem = std::mem::take(&mut self.qmem);
        let saved_stack = std::mem::take(&mut self.stack);
        let saved_out = std::mem::take(&mut self.out);
        let saved_site_out = std::mem::take(&mut self.site_out);
        self.probe_depth += 1;
        let result = self.probe_call_inner(caller_qi, site, callee, args, argvec);
        self.probe_depth -= 1;
        self.pool = saved_pool;
        self.qmem = saved_qmem;
        self.stack = saved_stack;
        self.out = saved_out;
        self.site_out = saved_site_out;
        result
    }

    fn probe_call_inner(
        &mut self,
        caller_qi: usize,
        site: (usize, usize, usize),
        callee: usize,
        args: &[CArg],
        argvec: &[Option<Value>],
    ) -> Result<Option<MemoEntry>, EngineError> {
        // Two probe callers standing for callers 1 and 2 (Root::Site
        // carries the probe id — probes never reach a terminal
        // themselves, only their Nested children do).
        let caller_slots = self.queries[caller_qi].slot_count;
        let mut pslots: Vec<Option<EnvVal>> = vec![None; caller_slots];
        for (pos, a) in args.iter().enumerate() {
            if let CArg::Slot(s) = a {
                pslots[*s] = argvec[pos].clone().map(EnvVal::Val);
            }
        }
        let probes: Vec<Env> = (0..2)
            .map(|i| Env { slots: pslots.clone(), root: Root::Site(i) })
            .collect();
        // Mimic the Call arm verbatim (pools are empty — fenced).
        let cq = &self.queries[callee];
        let mut cenvs: Vec<Env> = Vec::with_capacity(2);
        for env in &probes {
            let mut cenv = Env {
                slots: vec![None; cq.slot_count],
                root: Root::Nested(Rc::new(NestedRoot { site, caller: env.clone() })),
            };
            for (p, a) in cenv.slots.iter_mut().zip(args) {
                *p = match a {
                    CArg::Lit(v) => Some(EnvVal::Val(v.clone())),
                    CArg::Slot(s) => env.slots[*s].clone(),
                };
            }
            cenvs.push(cenv);
        }
        let nb = self.queries[callee].branches.len();
        for b2 in 0..nb {
            self.pool
                .entry((callee, b2))
                .or_default()
                .splice(0..0, cenvs.iter().rev().cloned());
        }
        for b2 in 0..nb {
            let batch = self
                .pool
                .get_mut(&(callee, b2))
                .map(std::mem::take)
                .unwrap_or_default();
            self.stack.push(Frame::Branch { q: callee, b: b2, batch });
        }
        self.drain()?;
        let captured = self.qmem.remove(&site).unwrap_or_default();
        // The unbound positions, with this site's slot per position
        // (unique across unbound positions — fenced upstream).
        let mut fill_pos: Vec<usize> = Vec::new();
        let mut pos_slot: Vec<usize> = Vec::new();
        for (pos, a) in args.iter().enumerate() {
            if let CArg::Slot(s) = a {
                if argvec[pos].is_none() {
                    fill_pos.push(pos);
                    pos_slot.push(*s);
                }
            }
        }
        let mut rows: Vec<(usize, Vec<Option<EnvVal>>)> = Vec::with_capacity(captured.len());
        for child in &captured {
            let pid = match child.root {
                Root::Site(i) => i,
                _ => return Ok(None),
            };
            rows.push((
                pid,
                pos_slot.iter().map(|&s| child.slots[s].clone()).collect(),
            ));
        }
        // Greedy lockstep parse: a maximal same-probe run opens a
        // segment; the partner probe's equal block must follow. Same-
        // direction consecutive segments cannot merge (the partner
        // block intervenes), so the parse is unambiguous; any capture
        // outside the lockstep form bails to real evaluation.
        let mut segments: Vec<(bool, Vec<Vec<Option<EnvVal>>>)> = Vec::new();
        let mut i = 0usize;
        while i < rows.len() {
            let first = rows[i].0;
            let mut j = i;
            while j < rows.len() && rows[j].0 == first {
                j += 1;
            }
            let blk = j - i;
            if j + blk > rows.len() {
                return Ok(None);
            }
            for k in 0..blk {
                let (pid, fills) = &rows[j + k];
                if *pid != 1 - first {
                    return Ok(None);
                }
                if fills.len() != rows[i + k].1.len()
                    || !fills.iter().zip(&rows[i + k].1).all(|(a, b)| envval_opt_eq(a, b))
                {
                    return Ok(None);
                }
            }
            segments.push((first == 0, rows[i..j].iter().map(|(_, f)| f.clone()).collect()));
            i = j + blk;
        }
        Ok(Some(MemoEntry { fill_pos, segments }))
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
            nullable: 0,
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
            nullable: 0,
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
        let out = run_query(&store, &qs, &mut mem, "ByAge", &[None], false).unwrap();
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
        let out = run_query(&store, &qs, &mut mem, "ByAge", &[Some(Value::I64(30))], false).unwrap();
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
            nullable: 0,
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
            false,
        )
        .unwrap();
        assert_eq!(out.rows.len(), 1);
        let out = run_query(&store, &qs, &mut mem, "contained", &[None, None], false).unwrap();
        assert_eq!(out.rows.len(), 15);
        // branch-2 local $z is not an identifier (params + first branch)
        assert_eq!(out.identifiers, vec!["$x", "$y"]);
    }

    /// Round 30: an unbounded recursive query over CYCLIC data must not
    /// take down the process. It once built a deep `Root::Nested` Rc chain
    /// whose recursive destructor overflowed the native stack (SIGSEGV /
    /// stack-abort) with no `STEP_LIMIT` on the drop path. The iterative
    /// `Drop for NestedRoot` flattens that, so divergence now surfaces as
    /// the catchable step-limit error instead. Exhaustive-enumeration bug:
    /// crashed even when the base case proved (goal on/at the cycle), so
    /// this exercises the follow-the-cycle direction that never bottoms.
    #[test]
    fn cyclic_data_no_crash() {
        let mut store = FactStore::new(vec![TypeSchema {
            name: "Location".into(),
            fields: vec![
                ("thing".into(), FieldType::Str),
                ("location".into(), FieldType::Str),
            ],
            nullable: 0,
        }]);
        let tid = store.type_id("Location").unwrap();
        // 3-cycle: a->b->c->a, goal "z" unreachable
        for (t, l) in [("a", "b"), ("b", "c"), ("c", "a")] {
            store
                .insert(tid, vec![Value::Str(t.into()), Value::Str(l.into())])
                .unwrap();
        }
        let qs = compile_all(
            &store,
            "query contained(String $x, String $y)\n    Location($x, $y;)\n    or\n    ( Location($z, $y;) and contained($x, $z;) )\nend\n",
        );
        let mut mem = QueryMem::default();
        let res = run_query(
            &store,
            &qs,
            &mut mem,
            "contained",
            &[None, Some(Value::Str("a".into()))],
            false,
        );
        match res {
            Err(EngineError(msg)) => {
                assert!(msg.contains("step limit"), "unexpected error: {msg}")
            }
            Ok(_) => panic!("cyclic query must error, not crash"),
        }
    }

    /// D-055 walls reject out-of-shape recursion at compile time.
    #[test]
    fn recursion_walls() {
        let store = FactStore::new(vec![TypeSchema {
            name: "L".into(),
            fields: vec![("a".into(), FieldType::Str), ("b".into(), FieldType::Str)],
            nullable: 0,
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
