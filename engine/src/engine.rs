//! Match/fire loop.
//!
//! Every semantic here is pinned by oracle probes (DECISIONS.md D-008,
//! D-011, D-013), never assumed:
//! - Agenda key: (salience desc, rule declaration index asc, tuple position
//!   in PHREAK candidate order asc), re-picked globally after every firing.
//! - Candidate (join) order: prefix list for pattern 1 = pattern 0's facts
//!   ascending; before joining pattern i (i >= 2) the accumulated prefix
//!   list is REVERSED; right-side facts iterate ascending. Self-join tuples
//!   may repeat a fact across positions.
//! - Property reactivity: a pattern listens to the fields its constraints
//!   (incl. bindings) reference; update() carries the mask of setters run
//!   since the last update of that fact (no setters => all fields). Fired
//!   activations whose tuple contains the updated fact at a listening
//!   position are re-created (refraction entry cleared) — except the firing
//!   rule's own current tuple when it has no-loop.
//! - Matches are rendered AFTER the RHS runs (post-mutation values).

use std::collections::{HashMap, HashSet};

use crate::drl::{self, Action, CmpOp, CmpRhs, Constraint, Literal, RhsArg, RuleDef};
use crate::store::{FactId, FactStore, FactView, FieldType, TypeId, TypeSchema, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "engine error: {}", self.0)
    }
}

impl From<drl::DrlError> for EngineError {
    fn from(e: drl::DrlError) -> Self {
        EngineError(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Firing {
    pub rule: String,
    pub matches: Vec<FactView>,
}

/// Where an RHS argument / constraint RHS value comes from.
#[derive(Clone)]
enum Src {
    Lit(Value),
    /// Field of the fact bound at tuple position `.0`, field index `.1`.
    Field(usize, usize),
}

struct CompiledCmp {
    field_idx: usize,
    op: CmpOp,
    rhs: Src,
}

struct CompiledPattern {
    type_id: TypeId,
    cmps: Vec<CompiledCmp>,
    /// Bit i set = this pattern's constraints reference field i (listen mask
    /// for property reactivity, D-013).
    listen_mask: u64,
}

enum CompiledAction {
    Insert { type_id: TypeId, args: Vec<Src> },
    Set { pos: usize, field_idx: usize, arg: Src },
    Update { pos: usize },
    Delete { pos: usize },
}

struct CompiledRule {
    def: RuleDef,
    patterns: Vec<CompiledPattern>,
    actions: Vec<CompiledAction>,
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// Rule indices sorted by (salience desc, declaration order).
    rule_order: Vec<usize>,
    /// Refraction memory: (rule index, matched fact tuple) that already fired.
    fired: HashSet<(usize, Vec<FactId>)>,
}

impl Engine {
    pub fn new(schemas: Vec<TypeSchema>) -> Result<Engine, EngineError> {
        let mut seen = HashSet::new();
        for s in &schemas {
            if !seen.insert(s.name.clone()) {
                return Err(EngineError(format!("duplicate type {}", s.name)));
            }
            if s.fields.len() > 64 {
                return Err(EngineError(format!("type {}: more than 64 fields", s.name)));
            }
        }
        Ok(Engine {
            store: FactStore::new(schemas),
            rules: Vec::new(),
            rule_order: Vec::new(),
            fired: HashSet::new(),
        })
    }

    pub fn add_rules_drl(&mut self, src: &str) -> Result<(), EngineError> {
        for def in drl::parse_rules(src)? {
            let compiled = self.compile_rule(def)?;
            self.rules.push(compiled);
        }
        self.rule_order = (0..self.rules.len()).collect();
        self.rule_order
            .sort_by_key(|&ri| (-self.rules[ri].def.salience, ri));
        Ok(())
    }

    fn compile_rule(&self, def: RuleDef) -> Result<CompiledRule, EngineError> {
        let rname = def.name.clone();
        let err = |m: String| EngineError(format!("rule {rname}: {m}"));
        if def.patterns.is_empty() {
            return Err(err("empty LHS not in subset".into()));
        }
        // Bindings visible so far: fact bindings ($p -> position) and field
        // bindings ($a -> (position, field, type)), declaration order.
        let mut fact_binds: HashMap<String, usize> = HashMap::new();
        let mut field_binds: HashMap<String, (usize, usize, FieldType)> = HashMap::new();
        let mut patterns = Vec::new();

        for (pi, p) in def.patterns.iter().enumerate() {
            let type_id = self
                .store
                .type_id(&p.type_name)
                .ok_or_else(|| err(format!("unknown type {}", p.type_name)))?;
            if let Some(b) = &p.binding {
                if fact_binds.insert(b.clone(), pi).is_some() {
                    return Err(err(format!("duplicate binding {b}")));
                }
            }
            let mut cmps = Vec::new();
            let mut listen_mask = 0u64;
            for c in &p.constraints {
                match c {
                    Constraint::Bind { var, field } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        listen_mask |= 1 << fi;
                        let ft = self.store.field_type(type_id, fi);
                        if field_binds.insert(var.clone(), (pi, fi, ft)).is_some() {
                            return Err(err(format!("duplicate binding {var}")));
                        }
                    }
                    Constraint::Cmp { field, op, rhs } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        listen_mask |= 1 << fi;
                        let lhs_ft = self.store.field_type(type_id, fi);
                        let (src, rhs_ft) = match rhs {
                            CmpRhs::Lit(l) => (Src::Lit(lit_value(l)), lit_type(l)),
                            CmpRhs::Var(v) => {
                                let (bpi, bfi, bft) = field_binds
                                    .get(v)
                                    .copied()
                                    .ok_or_else(|| err(format!("unknown binding {v} (must be declared before use)")))?;
                                (Src::Field(bpi, bfi), bft)
                            }
                        };
                        check_cmp_types(&rname, lhs_ft, *op, rhs_ft)?;
                        cmps.push(CompiledCmp { field_idx: fi, op: *op, rhs: src });
                    }
                }
            }
            patterns.push(CompiledPattern { type_id, cmps, listen_mask });
        }

        let mut actions = Vec::new();
        for a in &def.actions {
            match a {
                Action::Insert { type_name, args } => {
                    let tid = self
                        .store
                        .type_id(type_name)
                        .ok_or_else(|| err(format!("RHS insert: unknown type {type_name}")))?;
                    let schema = self.store.schema(tid);
                    if args.len() != schema.fields.len() {
                        return Err(err(format!(
                            "insert new {type_name}: expected {} args, got {}",
                            schema.fields.len(),
                            args.len()
                        )));
                    }
                    let mut srcs = Vec::new();
                    for (arg, (fname, ftype)) in args.iter().zip(schema.fields.clone()) {
                        let (src, src_ft) = self.compile_arg(
                            &rname,
                            arg,
                            &fact_binds,
                            &field_binds,
                            &def,
                            &patterns,
                        )?;
                        if !assignable(src_ft, ftype) {
                            return Err(err(format!(
                                "insert new {type_name}: arg for {fname} has wrong type"
                            )));
                        }
                        srcs.push(src);
                    }
                    actions.push(CompiledAction::Insert { type_id: tid, args: srcs });
                }
                Action::Set { var, field, arg } => {
                    let pos = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
                    let tid = patterns[pos].type_id;
                    let fi = self
                        .store
                        .field_index(tid, field)
                        .ok_or_else(|| err(format!("no field {field} for setter on {var}")))?;
                    let ftype = self.store.field_type(tid, fi);
                    let (src, src_ft) =
                        self.compile_arg(&rname, arg, &fact_binds, &field_binds, &def, &patterns)?;
                    if !assignable(src_ft, ftype) {
                        return Err(err(format!("setter {var}.{field}: wrong arg type")));
                    }
                    actions.push(CompiledAction::Set { pos, field_idx: fi, arg: src });
                }
                Action::Update { var } => {
                    let pos = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
                    actions.push(CompiledAction::Update { pos });
                }
                Action::Delete { var } => {
                    let pos = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
                    actions.push(CompiledAction::Delete { pos });
                }
            }
        }
        Ok(CompiledRule { def, patterns, actions })
    }

    fn compile_arg(
        &self,
        rname: &str,
        arg: &RhsArg,
        fact_binds: &HashMap<String, usize>,
        field_binds: &HashMap<String, (usize, usize, FieldType)>,
        _def: &RuleDef,
        patterns: &[CompiledPattern],
    ) -> Result<(Src, FieldType), EngineError> {
        match arg {
            RhsArg::Lit(l) => Ok((Src::Lit(lit_value(l)), lit_type(l))),
            RhsArg::Var(v) => {
                let (pi, fi, ft) = field_binds
                    .get(v)
                    .copied()
                    .ok_or_else(|| EngineError(format!("rule {rname}: unknown binding {v}")))?;
                Ok((Src::Field(pi, fi), ft))
            }
            RhsArg::Getter { var, field } => {
                let pos = *fact_binds.get(var).ok_or_else(|| {
                    EngineError(format!("rule {rname}: unknown fact binding {var}"))
                })?;
                let tid = patterns[pos].type_id;
                let fi = self.store.field_index(tid, field).ok_or_else(|| {
                    EngineError(format!("rule {rname}: no field {field} behind getter on {var}"))
                })?;
                Ok((Src::Field(pos, fi), self.store.field_type(tid, fi)))
            }
        }
    }

    pub fn insert(
        &mut self,
        type_name: &str,
        mut fields: Vec<(String, Value)>,
    ) -> Result<FactId, EngineError> {
        let tid = self
            .store
            .type_id(type_name)
            .ok_or_else(|| EngineError(format!("unknown type {type_name}")))?;
        let schema = self.store.schema(tid).clone();
        let mut ordered = Vec::with_capacity(schema.fields.len());
        for (fname, ftype) in &schema.fields {
            let pos = fields
                .iter()
                .position(|(n, _)| n == fname)
                .ok_or_else(|| EngineError(format!("{type_name}: missing field {fname}")))?;
            let (_, v) = fields.swap_remove(pos);
            let v = coerce(v, *ftype)
                .ok_or_else(|| EngineError(format!("{type_name}.{fname}: type mismatch")))?;
            ordered.push(v);
        }
        if let Some((extra, _)) = fields.first() {
            return Err(EngineError(format!("{type_name}: unknown field {extra}")));
        }
        self.store.insert(tid, ordered).map_err(EngineError)
    }

    pub fn fire_all(&mut self, limit: usize) -> Result<Vec<Firing>, EngineError> {
        let mut firings = Vec::new();
        while let Some((ri, tuple)) = self.next_activation() {
            if firings.len() >= limit {
                return Err(EngineError(format!(
                    "fire limit {limit} reached (non-terminating?)"
                )));
            }
            self.fired.insert((ri, tuple.clone()));
            self.execute_rhs(ri, &tuple)?;
            // Post-RHS rendering (D-013 / j03): values reflect the mutations
            // this firing just performed.
            let matches: Vec<FactView> = tuple.iter().map(|&f| self.store.render(f)).collect();
            firings.push(Firing { rule: self.rules[ri].def.name.clone(), matches });
        }
        Ok(firings)
    }

    fn next_activation(&self) -> Option<(usize, Vec<FactId>)> {
        for &ri in &self.rule_order {
            for tuple in self.candidates(ri) {
                if !self.fired.contains(&(ri, tuple.clone())) {
                    return Some((ri, tuple));
                }
            }
        }
        None
    }

    /// All currently-matching tuples of a rule, in pinned PHREAK firing
    /// order (D-013): p0 ascending; reverse the prefix list before joining
    /// pattern i for i >= 2; right side ascending.
    fn candidates(&self, ri: usize) -> Vec<Vec<FactId>> {
        let rule = &self.rules[ri];
        let mut tuples: Vec<Vec<FactId>> = Vec::new();
        for (pi, pat) in rule.patterns.iter().enumerate() {
            if pi == 0 {
                for f in self.store.live_facts_of(pat.type_id) {
                    if self.pattern_matches(pat, f, &[]) {
                        tuples.push(vec![f]);
                    }
                }
                continue;
            }
            if pi >= 2 {
                tuples.reverse();
            }
            let mut next = Vec::new();
            for prefix in &tuples {
                for f in self.store.live_facts_of(pat.type_id) {
                    if self.pattern_matches(pat, f, prefix) {
                        let mut t = prefix.clone();
                        t.push(f);
                        next.push(t);
                    }
                }
            }
            tuples = next;
        }
        tuples
    }

    fn pattern_matches(&self, pat: &CompiledPattern, fact: FactId, prefix: &[FactId]) -> bool {
        pat.cmps.iter().all(|c| {
            let lhs = self.store.value(fact, c.field_idx);
            let rhs = match &c.rhs {
                Src::Lit(v) => v.clone(),
                Src::Field(pi, fi) => self.store.value(prefix[*pi], *fi),
            };
            eval_cmp(&lhs, c.op, &rhs)
        })
    }

    fn execute_rhs(&mut self, ri: usize, tuple: &[FactId]) -> Result<(), EngineError> {
        // Pending modification masks: setters accumulate, update() consumes.
        let mut pending: HashMap<FactId, u64> = HashMap::new();
        let n_actions = self.rules[ri].actions.len();
        for ai in 0..n_actions {
            // (indices instead of iterating borrows: actions may mutate self)
            match &self.rules[ri].actions[ai] {
                CompiledAction::Insert { type_id, args } => {
                    let tid = *type_id;
                    let values: Vec<Value> = {
                        let schema = self.store.schema(tid).clone();
                        args.clone()
                            .iter()
                            .zip(schema.fields.iter())
                            .map(|(a, (_, ft))| {
                                coerce(self.eval_src(a, tuple), *ft).ok_or_else(|| {
                                    EngineError("RHS insert: arg type mismatch".into())
                                })
                            })
                            .collect::<Result<_, _>>()?
                    };
                    self.store.insert(tid, values).map_err(EngineError)?;
                }
                CompiledAction::Set { pos, field_idx, arg } => {
                    let f = tuple[*pos];
                    let fi = *field_idx;
                    let tid = self.store.fact_type(f);
                    let ft = self.store.field_type(tid, fi);
                    let v = coerce(self.eval_src(&arg.clone(), tuple), ft)
                        .ok_or_else(|| EngineError("RHS setter: arg type mismatch".into()))?;
                    self.store.set_value(f, fi, v).map_err(EngineError)?;
                    *pending.entry(f).or_insert(0) |= 1 << fi;
                }
                CompiledAction::Update { pos } => {
                    let f = tuple[*pos];
                    if !self.store.is_alive(f) {
                        continue;
                    }
                    // No setters before update => all-fields mask (D-013/j21).
                    let mask = pending.remove(&f).unwrap_or(u64::MAX);
                    self.apply_update(f, mask, ri, tuple);
                }
                CompiledAction::Delete { pos } => {
                    self.store.kill(tuple[*pos]);
                }
            }
        }
        Ok(())
    }

    /// Property-reactivity bookkeeping (D-013): clear refraction entries for
    /// every activation whose tuple holds `f` at a position whose listen
    /// mask overlaps `mask` — except the currently-firing tuple when its
    /// rule is no-loop.
    fn apply_update(&mut self, f: FactId, mask: u64, cur_ri: usize, cur_tuple: &[FactId]) {
        let ftype = self.store.fact_type(f);
        // (rule idx -> positions that listen to this update)
        let mut hot: Vec<(usize, Vec<usize>)> = Vec::new();
        for (rj, rule) in self.rules.iter().enumerate() {
            let positions: Vec<usize> = rule
                .patterns
                .iter()
                .enumerate()
                .filter(|(_, p)| p.type_id == ftype && p.listen_mask & mask != 0)
                .map(|(i, _)| i)
                .collect();
            if !positions.is_empty() {
                hot.push((rj, positions));
            }
        }
        let no_loop_guard = self.rules[cur_ri].def.no_loop;
        self.fired.retain(|(rj, t)| {
            if no_loop_guard && *rj == cur_ri && t.as_slice() == cur_tuple {
                return true; // no-loop: own tuple's refraction survives own update
            }
            match hot.iter().find(|(r, _)| r == rj) {
                None => true,
                Some((_, positions)) => !positions.iter().any(|&p| t[p] == f),
            }
        });
    }

    fn eval_src(&self, src: &Src, tuple: &[FactId]) -> Value {
        match src {
            Src::Lit(v) => v.clone(),
            Src::Field(pi, fi) => self.store.value(tuple[*pi], *fi),
        }
    }

    /// All live facts, in insertion order, rendered.
    pub fn facts(&self) -> Vec<FactView> {
        self.store.live_facts().map(|f| self.store.render(f)).collect()
    }
}

fn lit_value(l: &Literal) -> Value {
    match l {
        Literal::I64(n) => Value::I64(*n),
        Literal::F64(n) => Value::F64(*n),
        Literal::Str(s) => Value::Str(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
    }
}

fn lit_type(l: &Literal) -> FieldType {
    match l {
        Literal::I64(_) => FieldType::I64,
        Literal::F64(_) => FieldType::F64,
        Literal::Str(_) => FieldType::Str,
        Literal::Bool(_) => FieldType::Bool,
    }
}

/// Java-style: exact match, or i64 widening into f64.
fn assignable(src: FieldType, dst: FieldType) -> bool {
    src == dst || (src == FieldType::I64 && dst == FieldType::F64)
}

/// Java-style widening: i64 -> f64 is allowed, nothing else converts.
fn coerce(v: Value, target: FieldType) -> Option<Value> {
    match (v, target) {
        (Value::I64(n), FieldType::F64) => Some(Value::F64(n as f64)),
        (v, t) if v.type_of() == t => Some(v),
        _ => None,
    }
}

fn check_cmp_types(
    rule: &str,
    lhs: FieldType,
    op: CmpOp,
    rhs: FieldType,
) -> Result<(), EngineError> {
    let numeric = |t| matches!(t, FieldType::I64 | FieldType::F64);
    let ok = (numeric(lhs) && numeric(rhs))
        || (lhs == FieldType::Str && rhs == FieldType::Str)
        || (lhs == FieldType::Bool
            && rhs == FieldType::Bool
            && matches!(op, CmpOp::Eq | CmpOp::Ne));
    if ok {
        Ok(())
    } else {
        Err(EngineError(format!(
            "rule {rule}: constraint type mismatch ({lhs:?} {op:?} {rhs:?})"
        )))
    }
}

fn eval_cmp(lhs: &Value, op: CmpOp, rhs: &Value) -> bool {
    use std::cmp::Ordering;
    let ord: Option<Ordering> = match (lhs, rhs) {
        (Value::I64(a), Value::I64(b)) => Some(a.cmp(b)),
        (Value::I64(a), Value::F64(b)) => (*a as f64).partial_cmp(b),
        (Value::F64(a), Value::I64(b)) => a.partial_cmp(&(*b as f64)),
        (Value::F64(a), Value::F64(b)) => a.partial_cmp(b),
        // String comparison order = Java String.compareTo (UTF-16 code
        // units); equals Rust byte order for the ASCII-only corpus.
        (Value::Str(a), Value::Str(b)) => Some(a.as_str().cmp(b.as_str())),
        (Value::Bool(a), Value::Bool(b)) => Some(a.cmp(b)),
        _ => None,
    };
    match ord {
        None => false, // NaN comparisons are all false in Java too
        Some(o) => match op {
            CmpOp::Eq => o == Ordering::Equal,
            CmpOp::Ne => o != Ordering::Equal,
            CmpOp::Lt => o == Ordering::Less,
            CmpOp::Le => o != Ordering::Greater,
            CmpOp::Gt => o == Ordering::Greater,
            CmpOp::Ge => o != Ordering::Less,
        },
    }
}
