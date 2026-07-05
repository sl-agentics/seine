//! DRL query support (Phase Q0): non-recursive queries with unification,
//! evaluated on demand against the current working memory.
//!
//! Every ordering rule below is oracle-pinned (D-049..D-053) — see
//! DECISIONS.md. The observable model:
//!   - each pattern iterates the type's alpha-passing live facts in
//!     REVERSE insertion ("arrival") order — Drools stages right inserts
//!     LIFO and drains them at the query's first evaluation;
//!   - a pattern with beta equalities owns a 128-slot hash table; Drools
//!     sorts beta constraints regular-equalities-first (D-053), so the
//!     index is the regular (non-unification) equalities in textual
//!     order — duplicates included, cap 3 — and only a pattern with NO
//!     regular equality indexes its FIRST unification instead. New
//!     key-lists PREPEND into their slot chain, facts APPEND within a
//!     list;
//!   - a unification index means FULL-table iteration (slots ascending,
//!     chain order, list order) filtering all beta constraints — bound
//!     params filter, unbound params pass per-site and bind at pattern
//!     exit (first site wins, D-052); a regular index does a bucket
//!     lookup on the bound key; no index scans arrival order;
//!   - stage lists: each join consumes its stage head→tail and PREPENDS
//!     every emitted child into the next stage; the terminal appends
//!     rows head→tail.

use crate::drl::{self, CmpOp, Constraint, Literal, QueryDef};
use crate::engine::{eval_cmp_join_pub, eval_cmp_pub, EngineError};
use crate::store::{FactStore, FactView, FactId, FieldType, TypeId, Value};

/// One literal (alpha) test: same-type literals only — cross-type literal
/// coercion inside queries stays out of subset (D-051).
enum AlphaTest {
    Cmp { op: CmpOp, rhs: Value },
    Matches(crate::rx::Regex),
    Contains(String),
    InList { items: Vec<Value>, negated: bool },
}

/// Operand of a beta constraint: a query parameter (unification when
/// combined with `==`) or a scalar field binding declared earlier.
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
    /// env slot for the pattern's fact binding, if bound.
    fact_slot: Option<usize>,
    /// (env slot, field index) for `$x : field` bindings, textual order.
    field_binds: Vec<(usize, usize)>,
    alpha: Vec<(usize, AlphaTest)>,
    /// var-operand constraints in textual order.
    beta: Vec<BetaCon>,
    /// positions into `beta` forming the index (equalities, cap 3).
    index: Vec<usize>,
    /// true if any indexed equality unifies against a query parameter.
    unification_join: bool,
    /// startResult seed folded over the indexed fields' extractor indexes.
    seed: u32,
}

pub struct CompiledQuery {
    pub name: String,
    params: Vec<(String, FieldType)>,
    /// identifier per env slot: params first, then bindings in
    /// declaration order.
    idents: Vec<String>,
    patterns: Vec<QPattern>,
}

/// One row value: a matched fact or a bound scalar.
pub enum QueryVal {
    Fact(FactView),
    Scalar(Value),
}

pub struct QueryOutput {
    pub identifiers: Vec<String>,
    /// rows in oracle-pinned order; each aligned with `identifiers`.
    pub rows: Vec<Vec<QueryVal>>,
}

#[derive(Clone)]
enum EnvVal {
    Fact(FactId),
    Val(Value),
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
        // Java String.hashCode folds UTF-16 code units.
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

/// Extractor index of a declared-type field: 1 + rank of its accessor
/// method name (getX, or isX for bool) among the generated bean's no-arg
/// public methods sorted by name (getters + getClass/hashCode/toString);
/// slot 0 is the `this` accessor. Pinned across 18 shapes (D-050).
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

/// Key equality mirrors HashEntry.equals: exact per type, doubles by bit
/// pattern (Double.equals) — equivalent to `==` in the NaN/±0-free subset.
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

pub fn compile_query(store: &FactStore, def: QueryDef) -> Result<CompiledQuery, EngineError> {
    let err = |m: String| Err(EngineError(format!("query {}: {m}", def.name)));
    if def.patterns.is_empty() {
        return err("empty query body not in subset".into());
    }
    let mut params = Vec::new();
    for (ty, name) in &def.params {
        let ft = match ty.as_str() {
            "long" => FieldType::I64,
            "double" => FieldType::F64,
            "String" => FieldType::Str,
            "boolean" => FieldType::Bool,
            other => return err(format!("param type {other} not in subset (long/double/String/boolean)")),
        };
        params.push((name.clone(), ft));
    }

    // env slots: params, then bindings in declaration order
    let mut idents: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
    let mut slot_types: Vec<Option<FieldType>> = params.iter().map(|(_, t)| Some(*t)).collect();
    let mut fact_slots: Vec<bool> = params.iter().map(|_| false).collect();
    let add_slot = |idents: &mut Vec<String>,
                        slot_types: &mut Vec<Option<FieldType>>,
                        fact_slots: &mut Vec<bool>,
                        name: &str,
                        ft: Option<FieldType>,
                        is_fact: bool|
     -> Result<usize, EngineError> {
        if idents.iter().any(|i| i == name) {
            return Err(EngineError(format!(
                "query {}: duplicate identifier {name}",
                def.name
            )));
        }
        idents.push(name.to_string());
        slot_types.push(ft);
        fact_slots.push(is_fact);
        Ok(idents.len() - 1)
    };

    let mut used_params = vec![false; params.len()];
    // slots declared by earlier PATTERNS (same-pattern operands compile to
    // alpha predicates in Drools — out of subset, D-053)
    let mut prior_pattern_slots = params.len();
    let mut patterns = Vec::new();
    for pat in &def.patterns {
        if pat.ce != drl::CeKind::Positive || pat.acc.is_some() {
            return err("only plain positive patterns are in the query subset (D-051)".into());
        }
        let tid = store
            .type_id(&pat.type_name)
            .ok_or_else(|| EngineError(format!("query {}: unknown type {}", def.name, pat.type_name)))?;
        let fact_slot = match &pat.binding {
            Some(b) => Some(add_slot(&mut idents, &mut slot_types, &mut fact_slots, b, None, true)?),
            None => None,
        };
        let mut alpha = Vec::new();
        let mut beta: Vec<BetaCon> = Vec::new();
        let mut field_binds = Vec::new();
        for c in &pat.constraints {
            match c {
                Constraint::Bind { var, field } => {
                    let fi = store.field_index(tid, field).ok_or_else(|| {
                        EngineError(format!("query {}: {} has no field {field}", def.name, pat.type_name))
                    })?;
                    let ft = store.field_type(tid, fi);
                    let slot =
                        add_slot(&mut idents, &mut slot_types, &mut fact_slots, var, Some(ft), false)?;
                    field_binds.push((slot, fi));
                }
                Constraint::Cmp { field, op, rhs } => {
                    let fi = store.field_index(tid, field).ok_or_else(|| {
                        EngineError(format!("query {}: {} has no field {field}", def.name, pat.type_name))
                    })?;
                    let ft = store.field_type(tid, fi);
                    match rhs {
                        drl::CmpRhs::Lit(l) => {
                            let v = lit_value(l);
                            if v.type_of() != ft {
                                return err(format!(
                                    "literal constraint on {field} must match the field type exactly (cross-type literal coercion in queries is out of subset, D-051)"
                                ));
                            }
                            alpha.push((fi, AlphaTest::Cmp { op: *op, rhs: v }));
                        }
                        drl::CmpRhs::Var(v) => {
                            let slot = idents.iter().position(|i| i == v).ok_or_else(|| {
                                EngineError(format!("query {}: unknown binding {v}", def.name))
                            })?;
                            if fact_slots[slot] {
                                return err(format!("{v} is a fact binding; comparing fields to fact bindings is out of subset"));
                            }
                            if slot >= prior_pattern_slots {
                                return err(format!(
                                    "{v} is bound in the same pattern (same-pattern operands are out of subset, D-053)"
                                ));
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
                        EngineError(format!("query {}: {} has no field {field}", def.name, pat.type_name))
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

        // Index composition (D-050 corrected by D-053): Drools SORTS beta
        // constraints regular-equalities-first, so the index is the
        // regular (non-unification) equalities in textual order — dups
        // included, cap 3 — and unifications never join it. Only when NO
        // regular equality exists does the FIRST unification become a
        // single-field index with full-table (slot-order) iteration.
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
        // startResult: sr = 31; per index field: sr += 31*sr + extIdx
        let mut seed: u32 = 31;
        for &i in &index {
            let ext = extractor_index(store, tid, beta[i].field_idx);
            seed = seed
                .wrapping_add(seed.wrapping_mul(31))
                .wrapping_add(ext);
        }
        patterns.push(QPattern {
            tid,
            fact_slot,
            field_binds,
            alpha,
            beta,
            index,
            unification_join,
            seed,
        });
        prior_pattern_slots = idents.len();
    }
    if let Some(i) = used_params.iter().position(|u| !u) {
        return err(format!(
            "param {} is never unified against a field (unused params are out of subset, D-051)",
            params[i].0
        ));
    }
    Ok(CompiledQuery { name: def.name, params, idents, patterns })
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
// Evaluation

struct KeyList {
    hash: u32,
    key: Vec<Value>,
    facts: Vec<FactId>,
}

/// 128-slot table mirror: chains of key-lists per slot, keys prepended
/// on creation, facts appended within a list. Resize (>96 distinct keys)
/// is out of subset (D-051) and reported as an engine error.
struct Table {
    slots: Vec<Vec<KeyList>>,
}

const TABLE_LEN: u32 = 128;
const RESIZE_THRESHOLD: usize = 96;

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

    /// Full iteration: slots ascending, chain order, list order.
    fn full_order(&self) -> Vec<FactId> {
        self.slots
            .iter()
            .flat_map(|chain| chain.iter().flat_map(|kl| kl.facts.iter().copied()))
            .collect()
    }

    fn bucket(&self, seed_hash: u32, key: &[Value]) -> Vec<FactId> {
        let slot = (seed_hash & (TABLE_LEN - 1)) as usize;
        self.slots[slot]
            .iter()
            .find(|kl| kl.hash == seed_hash && key_eq(&kl.key, key))
            .map(|kl| kl.facts.clone())
            .unwrap_or_default()
    }
}

pub fn run_query(
    store: &FactStore,
    queries: &[CompiledQuery],
    name: &str,
    args: &[Option<Value>],
) -> Result<QueryOutput, EngineError> {
    let q = queries
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| EngineError(format!("query {name} does not exist")))?;
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

    let mut env0: Vec<Option<EnvVal>> = vec![None; q.idents.len()];
    for (i, a) in args.iter().enumerate() {
        env0[i] = a.clone().map(EnvVal::Val);
    }
    let mut stage: Vec<Vec<Option<EnvVal>>> = vec![env0];

    for pat in &q.patterns {
        // arrival = alpha-passing live facts in REVERSE insertion order
        let mut arrival: Vec<FactId> = store
            .live_facts_of(pat.tid)
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
        arrival.reverse();

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

        let mut next_stage: Vec<Vec<Option<EnvVal>>> = Vec::new();
        for env in &stage {
            let candidates: Vec<FactId> = match (&table, pat.unification_join) {
                (None, _) => arrival.clone(),
                (Some(_), true) => full_order.clone().unwrap(),
                (Some(t), false) => {
                    // bucket lookup by the (always bound) key
                    let key: Vec<Value> = pat
                        .index
                        .iter()
                        .map(|&i| match &env[operand_slot(&pat.beta[i].operand)] {
                            Some(EnvVal::Val(v)) => v.clone(),
                            _ => unreachable!("bucket key operands are bound by construction"),
                        })
                        .collect();
                    t.bucket(key_hash(pat.seed, &key), &key)
                }
            };
            'cand: for f in candidates {
                // All constraints evaluate against the pattern-ENTRY env:
                // multiple unification sites of one param inside a single
                // pattern are INDEPENDENT when the param is unbound (no
                // cross-site consistency), and the FIRST textual site's
                // value becomes the exit binding (q11_multisite, D-052).
                let mut pending: Vec<(usize, Value)> = Vec::new();
                for b in &pat.beta {
                    let fv = store.value(f, b.field_idx);
                    let slot = operand_slot(&b.operand);
                    match &env[slot] {
                        Some(EnvVal::Val(bound)) => {
                            // pinned rule-join comparison semantics (D-020)
                            if !eval_cmp_join_pub(&fv, b.op, bound) {
                                continue 'cand;
                            }
                        }
                        Some(EnvVal::Fact(_)) => unreachable!("fact operands rejected at compile"),
                        None => {
                            // unbound param site: matches anything; the
                            // first site records the exit binding
                            if !pending.iter().any(|(s, _)| *s == slot) {
                                pending.push((slot, fv));
                            }
                        }
                    }
                }
                let mut env2 = env.clone();
                for (slot, v) in pending {
                    env2[slot] = Some(EnvVal::Val(v));
                }
                if let Some(slot) = pat.fact_slot {
                    env2[slot] = Some(EnvVal::Fact(f));
                }
                for (slot, fi) in &pat.field_binds {
                    env2[*slot] = Some(EnvVal::Val(store.value(f, *fi)));
                }
                next_stage.insert(0, env2); // PREPEND (staged-set LIFO)
            }
        }
        stage = next_stage;
    }

    let rows = stage
        .iter()
        .map(|env| {
            env.iter()
                .map(|v| match v {
                    Some(EnvVal::Fact(id)) => QueryVal::Fact(store.render(*id)),
                    Some(EnvVal::Val(v)) => QueryVal::Scalar(v.clone()),
                    None => unreachable!("all identifiers bound in emitted rows"),
                })
                .collect()
        })
        .collect();
    Ok(QueryOutput { identifiers: q.idents.clone(), rows })
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

    /// Ground truth from live Drools 9.44.0.Final table dumps (D-050,
    /// MemDump): Person(name String, age long), seed 993, key-list hashes.
    #[test]
    fn hash_pipeline_matches_live_drools_dump() {
        // startResult for a single index on Person.age: extractor idx 1
        let store = FactStore::new(vec![TypeSchema {
            name: "Person".into(),
            fields: vec![
                ("name".into(), FieldType::Str),
                ("age".into(), FieldType::I64),
            ],
        }]);
        let tid = store.type_id("Person").unwrap();
        assert_eq!(extractor_index(&store, tid, 1), 1); // getAge
        assert_eq!(extractor_index(&store, tid, 0), 3); // getName (getClass=2)
        let mut seed: u32 = 31;
        seed = seed.wrapping_add(seed.wrapping_mul(31)).wrapping_add(1);
        assert_eq!(seed, 993);
        // observed IndexTupleList hashCodes: age 3 -> 32561 (slot 49),
        // 30 -> 32559 (slot 47), 10 -> 32570 (slot 58), 45 -> 32541 (slot 29)
        assert_eq!(key_hash(993, &[Value::I64(3)]), 32561);
        assert_eq!(key_hash(993, &[Value::I64(30)]), 32559);
        assert_eq!(key_hash(993, &[Value::I64(10)]), 32570);
        assert_eq!(key_hash(993, &[Value::I64(45)]), 32541);
    }

    /// MemDump3 shape matrix: the accessor-sort rule (getX/isX +
    /// getClass/hashCode/toString, slot 0 = this).
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
        // sorted: getCity(1) getClass(2) getName(3) getScore(4) hashCode(5) isMarried(6)
        assert_eq!(extractor_index(&store, tid, 1), 1); // city
        assert_eq!(extractor_index(&store, tid, 0), 3); // name
        assert_eq!(extractor_index(&store, tid, 3), 4); // score
        assert_eq!(extractor_index(&store, tid, 2), 6); // married (isMarried)
    }

    /// End-to-end: q2_param_unify oracle-pinned row order (bob, alice,
    /// dave, carol for the unbound call — slot-descending via terminal
    /// staging reversal; within-bucket forward insertion).
    #[test]
    fn unbound_unification_row_order() {
        let mut store = FactStore::new(vec![TypeSchema {
            name: "Person".into(),
            fields: vec![
                ("name".into(), FieldType::Str),
                ("age".into(), FieldType::I64),
            ],
        }]);
        let tid = store.type_id("Person").unwrap();
        for (n, a) in [("alice", 30), ("bob", 10), ("carol", 45), ("dave", 30)] {
            store
                .insert(tid, vec![Value::Str(n.into()), Value::I64(a)])
                .unwrap();
        }
        let file = drl::parse_file("query ByAge(long $a)\n    $p : Person(age == $a)\nend\n").unwrap();
        let q = compile_query(&store, file.queries[0].clone()).unwrap();
        let out = run_query(&store, &[q], "ByAge", &[None]).unwrap();
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
        // bound call filters to the 30-bucket, forward insertion order
        let out = run_query(&store, &[compile_query(&store, file.queries[0].clone()).unwrap()],
                            "ByAge", &[Some(Value::I64(30))]).unwrap();
        assert_eq!(out.rows.len(), 2);
    }
}
