//! Match/fire loop.
//!
//! Conflict-resolution policy is *pinned by oracle probes*, never assumed.
//! Current pinned facts (see DECISIONS.md):
//!   - D-006 (preliminary): with all facts inserted before fire_all, same-rule
//!     activations fire in fact insertion (handle) order.
//! Everything else (multi-rule tie-break, mid-fire insertion ordering,
//! salience interaction) is provisional until its Phase 1 probe exists.

use std::collections::HashSet;

use crate::drl::{self, Action, CmpOp, Constraint, Literal, RhsArg, RuleDef};
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

/// A compiled single-pattern test: field index + op + literal, resolved
/// against the schema at rule-add time so firing does no name lookups.
struct CompiledCmp {
    field_idx: usize,
    op: CmpOp,
    rhs: Literal,
}

struct CompiledPattern {
    type_id: TypeId,
    cmps: Vec<CompiledCmp>,
}

struct CompiledRule {
    def: RuleDef,
    patterns: Vec<CompiledPattern>,
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
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
        }
        Ok(Engine { store: FactStore::new(schemas), rules: Vec::new(), fired: HashSet::new() })
    }

    pub fn add_rules_drl(&mut self, src: &str) -> Result<(), EngineError> {
        for def in drl::parse_rules(src)? {
            let compiled = self.compile_rule(def)?;
            self.rules.push(compiled);
        }
        Ok(())
    }

    fn compile_rule(&self, def: RuleDef) -> Result<CompiledRule, EngineError> {
        if def.patterns.is_empty() {
            return Err(EngineError(format!("rule {}: empty LHS not in subset", def.name)));
        }
        if def.patterns.len() > 1 {
            return Err(EngineError(format!(
                "rule {}: multi-pattern rules not implemented yet (Phase 2)",
                def.name
            )));
        }
        let mut patterns = Vec::new();
        for p in &def.patterns {
            let type_id = self.store.type_id(&p.type_name).ok_or_else(|| {
                EngineError(format!("rule {}: unknown type {}", def.name, p.type_name))
            })?;
            let mut cmps = Vec::new();
            for c in &p.constraints {
                match c {
                    Constraint::Bind { field, .. } => {
                        // Field bindings are resolved lazily on the RHS; just
                        // validate the field exists.
                        self.store.field_index(type_id, field).ok_or_else(|| {
                            EngineError(format!(
                                "rule {}: type {} has no field {}",
                                def.name, p.type_name, field
                            ))
                        })?;
                    }
                    Constraint::Cmp { field, op, rhs } => {
                        let field_idx =
                            self.store.field_index(type_id, field).ok_or_else(|| {
                                EngineError(format!(
                                    "rule {}: type {} has no field {}",
                                    def.name, p.type_name, field
                                ))
                            })?;
                        check_cmp_types(
                            &def.name,
                            self.store.field_type(type_id, field_idx),
                            *op,
                            rhs,
                        )?;
                        cmps.push(CompiledCmp { field_idx, op: *op, rhs: rhs.clone() });
                    }
                }
            }
            patterns.push(CompiledPattern { type_id, cmps });
        }
        Ok(CompiledRule { def, patterns })
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
            let matches: Vec<FactView> = tuple.iter().map(|&f| self.store.render(f)).collect();
            let rule_name = self.rules[ri].def.name.clone();
            self.execute_rhs(ri, &tuple)?;
            firings.push(Firing { rule: rule_name, matches });
        }
        Ok(firings)
    }

    /// Conflict resolution, pinned by oracle probes pr01–pr08 (DECISIONS.md
    /// D-008): after EVERY firing the next activation is the minimum of
    /// (salience descending, rule declaration index ascending, fact
    /// insertion/handle order ascending), re-evaluated globally — a firing
    /// that activates an earlier-declared rule is preempted by it (pr06).
    fn next_activation(&self) -> Option<(usize, Vec<FactId>)> {
        let mut best: Option<((i64, usize, u32), (usize, Vec<FactId>))> = None;
        for (ri, rule) in self.rules.iter().enumerate() {
            let pat = &rule.patterns[0];
            for fact in self.store.live_facts_of(pat.type_id) {
                if !self.matches_pattern(pat, fact) {
                    continue;
                }
                let tuple = vec![fact];
                if self.fired.contains(&(ri, tuple.clone())) {
                    continue;
                }
                // Lexicographic: highest salience, then rule declaration
                // order, then lowest fact handle (pr01–pr08).
                let key = (-rule.def.salience, ri, fact.0);
                if best.as_ref().map_or(true, |(bk, _)| key < *bk) {
                    best = Some((key, (ri, tuple)));
                }
            }
        }
        best.map(|(_, act)| act)
    }

    fn matches_pattern(&self, pat: &CompiledPattern, fact: FactId) -> bool {
        pat.cmps.iter().all(|c| {
            let lhs = self.store.value(fact, c.field_idx);
            eval_cmp(&lhs, c.op, &c.rhs)
        })
    }

    fn execute_rhs(&mut self, ri: usize, tuple: &[FactId]) -> Result<(), EngineError> {
        let actions = self.rules[ri].def.actions.clone();
        for action in &actions {
            match action {
                Action::Insert { type_name, args } => {
                    let tid = self.store.type_id(type_name).ok_or_else(|| {
                        EngineError(format!("RHS insert: unknown type {type_name}"))
                    })?;
                    let schema = self.store.schema(tid).clone();
                    if args.len() != schema.fields.len() {
                        return Err(EngineError(format!(
                            "RHS insert new {type_name}: expected {} args, got {}",
                            schema.fields.len(),
                            args.len()
                        )));
                    }
                    let mut values = Vec::with_capacity(args.len());
                    for (arg, (fname, ftype)) in args.iter().zip(&schema.fields) {
                        let v = self.eval_rhs_arg(ri, tuple, arg)?;
                        let v = coerce(v, *ftype).ok_or_else(|| {
                            EngineError(format!(
                                "RHS insert new {type_name}: arg for {fname} has wrong type"
                            ))
                        })?;
                        values.push(v);
                    }
                    self.store.insert(tid, values).map_err(EngineError)?;
                }
            }
        }
        Ok(())
    }

    fn eval_rhs_arg(
        &self,
        ri: usize,
        tuple: &[FactId],
        arg: &RhsArg,
    ) -> Result<Value, EngineError> {
        let rule = &self.rules[ri];
        match arg {
            RhsArg::Lit(l) => Ok(lit_value(l)),
            RhsArg::Getter { var, field } => {
                let (pi, _) = self.resolve_fact_binding(rule, var)?;
                let fact = tuple[pi];
                let tid = self.store.fact_type(fact);
                let idx = self.store.field_index(tid, field).ok_or_else(|| {
                    EngineError(format!(
                        "RHS: type {} has no field {field}",
                        self.store.schema(tid).name
                    ))
                })?;
                Ok(self.store.value(fact, idx))
            }
            RhsArg::Var(var) => {
                // A field binding `$a : age` from some pattern.
                for (pi, p) in rule.def.patterns.iter().enumerate() {
                    for c in &p.constraints {
                        if let Constraint::Bind { var: v, field } = c {
                            if v == var {
                                let fact = tuple[pi];
                                let tid = self.store.fact_type(fact);
                                let idx =
                                    self.store.field_index(tid, field).ok_or_else(|| {
                                        EngineError(format!("RHS: no field {field}"))
                                    })?;
                                return Ok(self.store.value(fact, idx));
                            }
                        }
                    }
                }
                Err(EngineError(format!("RHS: unknown binding {var}")))
            }
        }
    }

    fn resolve_fact_binding(
        &self,
        rule: &CompiledRule,
        var: &str,
    ) -> Result<(usize, TypeId), EngineError> {
        for (pi, p) in rule.def.patterns.iter().enumerate() {
            if p.binding.as_deref() == Some(var) {
                return Ok((pi, rule.patterns[pi].type_id));
            }
        }
        Err(EngineError(format!("RHS: unknown fact binding {var}")))
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
    field: FieldType,
    op: CmpOp,
    rhs: &Literal,
) -> Result<(), EngineError> {
    let ok = match (field, rhs) {
        (FieldType::I64 | FieldType::F64, Literal::I64(_) | Literal::F64(_)) => true,
        (FieldType::Str, Literal::Str(_)) => true,
        (FieldType::Bool, Literal::Bool(_)) => matches!(op, CmpOp::Eq | CmpOp::Ne),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(EngineError(format!(
            "rule {rule}: constraint type mismatch ({field:?} {op:?} {rhs:?})"
        )))
    }
}

fn eval_cmp(lhs: &Value, op: CmpOp, rhs: &Literal) -> bool {
    use std::cmp::Ordering;
    let ord: Option<Ordering> = match (lhs, rhs) {
        (Value::I64(a), Literal::I64(b)) => Some(a.cmp(b)),
        (Value::I64(a), Literal::F64(b)) => (*a as f64).partial_cmp(b),
        (Value::F64(a), Literal::I64(b)) => a.partial_cmp(&(*b as f64)),
        (Value::F64(a), Literal::F64(b)) => a.partial_cmp(b),
        // String comparison order = Java String.compareTo (UTF-16 code units).
        // For ASCII corpus data this equals Rust byte order; non-ASCII is
        // restricted at the generator until probed.
        (Value::Str(a), Literal::Str(b)) => Some(a.as_str().cmp(b.as_str())),
        (Value::Bool(a), Literal::Bool(b)) => Some(a.cmp(b)),
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
