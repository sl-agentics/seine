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

use crate::drl::{self, Action, CeKind, CmpOp, CmpRhs, Constraint, Literal, RhsArg, RuleDef};
use crate::store::{FactId, FactStore, FactView, FieldType, TypeId, TypeSchema, Value};

/// Reserved type name backing first-position not/exists CEs (D-031):
/// Drools matches those rules on InitialFactImpl. The engine keeps one
/// synthetic fact of this hidden zero-field type; it renders in match
/// lists but never in the final fact set.
pub(crate) const INITIAL_FACT: &str = "InitialFact";

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
    /// Field of the fact bound at tuple position `.0`, field index `.1`,
    /// read LIVE from the store (getter calls, LHS join constraints).
    Field(usize, usize),
    /// LHS binding used on the RHS: Drools extracts declarations when the
    /// consequence starts, so the value is a SNAPSHOT taken at RHS start —
    /// setters earlier in the same RHS must not affect it (fz_7_2525).
    SnapField(usize, usize),
}

/// One compiled LHS constraint test on a single field.
enum Test {
    Cmp { op: CmpOp, rhs: Src },
    /// `matches` — full-string regex acceptance (D-030).
    Matches(crate::rx::Regex),
    /// `contains` — String substring (D-030).
    Contains(String),
    /// `in` / `not in` — OR of `==`-with-promotion branches; never
    /// participates in eq-node sharing/hashing (D-030, op_i4/op_i6).
    InList { items: Vec<Value>, negated: bool },
}

struct CompiledCmp {
    field_idx: usize,
    test: Test,
}

struct CompiledPattern {
    type_id: TypeId,
    cmps: Vec<CompiledCmp>,
    /// Bit i set = this pattern's constraints reference field i (listen mask
    /// for property reactivity, D-013).
    listen_mask: u64,
    ce: CeKind,
    /// This pattern's index into rule tuples (None for not/exists — CE
    /// patterns contribute no tuple element, D-031).
    tpos: Option<usize>,
    /// Whether any constraint references an earlier pattern's binding
    /// (beta constraint) — drives not-node linking (D-031).
    beta: bool,
    /// Beta-memory index kind (D-032): equality hash wins; not/exists
    /// nodes fall back to a COMPARISON (range) index on the first
    /// relational var constraint with compatible operands.
    pindex: phreak::Index,
    /// cmps position of the range-indexed constraint (Cmp index only).
    index_ci: Option<usize>,
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
    /// Per-node segment-boundary flips (D-033/ne_s*): node j's staged
    /// output is REVERSED before the next node / terminal when this
    /// rule's continuation is not the FIRST-built sink of a shared node.
    flips: Vec<bool>,
}

use crate::phreak::{self, Origin, Staged, Tup};

/// Per-rule network state: pos0 staged input, one phreak::Node per join,
/// and the terminal activation queue (only UNFIRED activations live here —
/// fired tuples leave the queue on firing, per RuleExecutor semantics).
struct RuleNet {
    s0: Staged<FactId>,
    nodes: Vec<phreak::Node>,
    queue: Vec<Tup>,
    /// Eager mirror of alpha membership per position.
    active: Vec<HashSet<FactId>>,
    /// Agenda-item lifecycle: set when the rule links (or re-dirties
    /// while linked), cleared when its evaluation leaves the queue empty.
    /// An unlinked-but-queued rule still evaluates when reached
    /// (fz_42_1464); an unqueued rule accumulates staged input
    /// (fz_42_124, fz_7_145).
    queued: bool,
    /// Per-position transient link pulse for UNCONSTRAINED not nodes: the
    /// first right insert force-links the node so its blocking batch
    /// evaluates, after which it unlinks again (D-031,
    /// unlinkNotNodeOnRightInsert). Cleared when the rule evaluates.
    pulse: Vec<bool>,
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// Rule indices sorted by (salience desc, declaration order).
    rule_order: Vec<usize>,
    /// Per-rule join networks (phreak behavioral port).
    nets: Vec<RuleNet>,
    lists_built: bool,
    /// The synthetic InitialFact (inserted before scenario facts once a
    /// CE-first rule compiles — Drools asserts it at session init).
    init_fact: Option<FactId>,
}

impl Engine {
    pub fn new(mut schemas: Vec<TypeSchema>) -> Result<Engine, EngineError> {
        let mut seen = HashSet::new();
        for s in &schemas {
            if !seen.insert(s.name.clone()) {
                return Err(EngineError(format!("duplicate type {}", s.name)));
            }
            if s.name == INITIAL_FACT {
                return Err(EngineError(format!("type name {INITIAL_FACT} is reserved")));
            }
            if s.fields.len() > 64 {
                return Err(EngineError(format!("type {}: more than 64 fields", s.name)));
            }
        }
        // Hidden zero-field type backing first-position CEs (D-031).
        schemas.push(TypeSchema { name: INITIAL_FACT.into(), fields: Vec::new() });
        Ok(Engine {
            store: FactStore::new(schemas),
            rules: Vec::new(),
            rule_order: Vec::new(),
            nets: Vec::new(),
            lists_built: false,
            init_fact: None,
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
        self.compute_segment_flips();
        self.share_and_hash_alphas();
        // CE-first rules match on InitialFact: assert the synthetic fact
        // ahead of scenario facts, as Drools does at session init.
        let init_tid = self.store.type_id(INITIAL_FACT).unwrap();
        if self.init_fact.is_none()
            && self
                .rules
                .iter()
                .any(|r| r.patterns.iter().any(|p| p.type_id == init_tid))
        {
            self.init_fact = Some(self.store.insert(init_tid, Vec::new()).map_err(EngineError)?);
        }
        Ok(())
    }

    /// Node-sharing segment boundaries (D-033, probes ne_s1..ne_s10):
    /// rules with structurally equal pattern PREFIXES share beta nodes
    /// (binding names are irrelevant; literals compare by their D-029
    /// alpha-node identity). Where sharers diverge, the shared node has
    /// multiple sinks and a segment boundary forms: the FIRST-declared
    /// sink's continuation receives the staged list as-is, every later
    /// sink gets a REVERSED copy (identical-LHS twins fire in opposite
    /// orders, ne_s7; declaration order picks the preserved one, ne_s8;
    /// boundaries stack per depth, ne_s10).
    fn compute_segment_flips(&mut self) {
        let keys: Vec<Vec<String>> = self
            .rules
            .iter()
            .map(|r| r.patterns.iter().map(|p| self.pattern_key(p)).collect())
            .collect();
        for ri in 0..self.rules.len() {
            let k = self.rules[ri].patterns.len();
            let mut flips = vec![false; k.saturating_sub(1)];
            for j in 1..k {
                let sharers: Vec<usize> = (0..self.rules.len())
                    .filter(|&rb| keys[rb].len() > j && keys[rb][..=j] == keys[ri][..=j])
                    .collect();
                if sharers.len() < 2 {
                    continue;
                }
                // extension identity at j+1; a rule ENDING here is its own
                // sink (each rule has its own terminal node)
                let ext = |rb: usize| {
                    keys[rb].get(j + 1).cloned().unwrap_or_else(|| format!("__end_{rb}"))
                };
                let my_ext = ext(ri);
                if sharers.iter().all(|&rb| ext(rb) == my_ext) {
                    continue; // single sink: the boundary is deeper
                }
                let first = sharers[0]; // min rule index builds sink 1
                if ext(first) != my_ext {
                    flips[j - 1] = true;
                }
            }
            self.rules[ri].flips = flips;
        }
    }

    /// Structural identity of a pattern for node sharing: type, CE kind,
    /// and the ordered non-binding constraints — var references by
    /// (tuple pos, field), eq literals coerced to the field type (the
    /// D-029 alpha-node key), other literals as written.
    fn pattern_key(&self, p: &CompiledPattern) -> String {
        use std::fmt::Write as _;
        let mut s = format!("{}|{:?}", p.type_id.0, p.ce);
        for c in &p.cmps {
            let _ = write!(s, ";{}", c.field_idx);
            match &c.test {
                Test::Cmp { op, rhs: Src::Lit(v) } => {
                    let ft = self.store.field_type(p.type_id, c.field_idx);
                    let vv = if *op == CmpOp::Eq {
                        match (v, ft) {
                            (Value::F64(x), FieldType::I64) => Value::I64(*x as i64),
                            (Value::I64(n), FieldType::F64) => Value::F64(*n as f64),
                            (v, _) => v.clone(),
                        }
                    } else {
                        v.clone()
                    };
                    let _ = write!(s, "{op:?}{vv:?}");
                }
                Test::Cmp { op, rhs: Src::Field(ti, fi) } => {
                    let _ = write!(s, "{op:?}v{ti}.{fi}");
                }
                Test::Cmp { .. } => {}
                Test::Matches(r) => {
                    let _ = write!(s, "m{}", r.source());
                }
                Test::Contains(n) => {
                    let _ = write!(s, "c{n}");
                }
                Test::InList { items, negated } => {
                    let _ = write!(s, "in{negated}{items:?}");
                }
            }
        }
        s
    }

    /// Alpha-network build semantics for `field == literal` constraints
    /// (probe series w1-w18 / pr_lit / u15, D-029):
    /// - node identity is (type, preceding-literal-chain, field, literal
    ///   COERCED to the field's type): a later rule whose coerced literal
    ///   collides SHARES the first-built node and inherits its ORIGINAL
    ///   literal (w10: `n == 1.5` after `n == 1` matches n=1; w16
    ///   reversed: `n == 1` after `n == 1.5` matches nothing);
    /// - with >= 3 sibling eq-nodes (post-sharing) on one field, the sink
    ///   adapter hashes: membership uses the COERCED key, i.e. a double
    ///   literal on a long field truncates (w5/w8/w12, fz_777_4504);
    /// - below the threshold each node compares its first-built literal
    ///   with double promotion (w4/w6/u15).
    fn share_and_hash_alphas(&mut self) {
        use std::collections::HashMap as Map;
        // group key -> members (rule, pattern, cmp index, coerced key,
        // original literal), in build order
        let mut groups: Map<(TypeId, String, usize), Vec<(usize, usize, usize, String, Value)>> =
            Map::new();
        for ri in 0..self.rules.len() {
            for pi in 0..self.rules[ri].patterns.len() {
                let pat = &self.rules[ri].patterns[pi];
                let mut prefix = String::new();
                for ci in 0..pat.cmps.len() {
                    let c = &pat.cmps[ci];
                    // Every literal alpha constraint contributes to the
                    // node-chain prefix that scopes downstream eq groups
                    // (op_i7); only Eq-vs-literal constraints are members.
                    match &c.test {
                        Test::Cmp { op, rhs: Src::Lit(v) } => {
                            if *op == CmpOp::Eq {
                                let ft = self.store.field_type(pat.type_id, c.field_idx);
                                let coerced = match (v, ft) {
                                    (Value::F64(x), FieldType::I64) => Value::I64(*x as i64),
                                    (Value::I64(n), FieldType::F64) => Value::F64(*n as f64),
                                    (v, _) => v.clone(),
                                };
                                groups
                                    .entry((pat.type_id, prefix.clone(), c.field_idx))
                                    .or_default()
                                    .push((ri, pi, ci, format!("{coerced:?}"), v.clone()));
                            }
                            prefix.push_str(&format!("{}|{:?}|{:?};", c.field_idx, op, v));
                        }
                        Test::Cmp { .. } => {} // join constraint: beta, not alpha
                        Test::Matches(r) => {
                            prefix.push_str(&format!("{}|m|{};", c.field_idx, r.source()));
                        }
                        Test::Contains(n) => {
                            prefix.push_str(&format!("{}|c|{n};", c.field_idx));
                        }
                        Test::InList { items, negated } => {
                            prefix.push_str(&format!("{}|in{negated}|{items:?};", c.field_idx));
                        }
                    }
                }
            }
        }
        for (_, members) in groups {
            // first-built literal per coerced node key
            let mut node_lit: Map<String, Value> = Map::new();
            for (_, _, _, key, lit) in &members {
                node_lit.entry(key.clone()).or_insert_with(|| lit.clone());
            }
            let hashed = node_lit.len() >= 3;
            for (ri, pi, ci, key, _) in members {
                let pat = &mut self.rules[ri].patterns[pi];
                let ft = self.store.field_type(pat.type_id, pat.cmps[ci].field_idx);
                let new_lit = if hashed {
                    // hashed membership: coerced key comparison
                    match (&node_lit[&key], ft) {
                        (Value::F64(x), FieldType::I64) => Value::I64(*x as i64),
                        (v, _) => v.clone(),
                    }
                } else {
                    node_lit[&key].clone() // shared node's original literal
                };
                pat.cmps[ci].test = Test::Cmp { op: CmpOp::Eq, rhs: Src::Lit(new_lit) };
            }
        }
    }

    fn compile_rule(&self, def: RuleDef) -> Result<CompiledRule, EngineError> {
        let rname = def.name.clone();
        let err = |m: String| EngineError(format!("rule {rname}: {m}"));
        if def.patterns.is_empty() {
            return Err(err("empty LHS not in subset".into()));
        }
        // Bindings visible so far: fact bindings ($p -> tuple index) and
        // field bindings ($a -> (tuple index, field, type)), declaration
        // order. CE patterns own no tuple slot (D-031).
        let mut fact_binds: HashMap<String, (usize, TypeId)> = HashMap::new();
        let mut field_binds: HashMap<String, (usize, usize, FieldType)> = HashMap::new();
        let mut patterns = Vec::new();
        let mut tuple_len = 0usize;

        // A rule whose first pattern is a CE matches on InitialFact
        // (ne_f1): inject the synthetic positive position 0.
        if def.patterns[0].ce != CeKind::Positive {
            let tid = self
                .store
                .type_id(INITIAL_FACT)
                .ok_or_else(|| err("internal: InitialFact type missing".into()))?;
            patterns.push(CompiledPattern {
                type_id: tid,
                cmps: Vec::new(),
                listen_mask: 0,
                ce: CeKind::Positive,
                tpos: Some(0),
                beta: false,
                pindex: phreak::Index::None,
                index_ci: None,
            });
            tuple_len = 1;
        }

        for p in def.patterns.iter() {
            if p.type_name == INITIAL_FACT {
                return Err(err(format!("type name {INITIAL_FACT} is reserved")));
            }
            let type_id = self
                .store
                .type_id(&p.type_name)
                .ok_or_else(|| err(format!("unknown type {}", p.type_name)))?;
            let tpos = if p.ce == CeKind::Positive {
                let t = tuple_len;
                tuple_len += 1;
                Some(t)
            } else {
                None
            };
            if let Some(b) = &p.binding {
                let t = tpos.ok_or_else(|| err("binding on a CE pattern".into()))?;
                if fact_binds.insert(b.clone(), (t, type_id)).is_some() {
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
                        let t = tpos.ok_or_else(|| err("binding in a CE pattern".into()))?;
                        if field_binds.insert(var.clone(), (t, fi, ft)).is_some() {
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
                        cmps.push(CompiledCmp {
                            field_idx: fi,
                            test: Test::Cmp { op: *op, rhs: src },
                        });
                    }
                    Constraint::Matches { field, regex } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        if self.store.field_type(type_id, fi) != FieldType::Str {
                            return Err(err(format!(
                                "matches requires a String field (subset wall), {field} is not"
                            )));
                        }
                        listen_mask |= 1 << fi;
                        let r = crate::rx::Regex::parse(regex).map_err(|e| err(e))?;
                        cmps.push(CompiledCmp { field_idx: fi, test: Test::Matches(r) });
                    }
                    Constraint::Contains { field, needle } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        if self.store.field_type(type_id, fi) != FieldType::Str {
                            return Err(err(format!(
                                "contains requires a String field (subset wall), {field} is not"
                            )));
                        }
                        listen_mask |= 1 << fi;
                        cmps.push(CompiledCmp {
                            field_idx: fi,
                            test: Test::Contains(needle.clone()),
                        });
                    }
                    Constraint::InList { field, items, negated } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        listen_mask |= 1 << fi;
                        let lhs_ft = self.store.field_type(type_id, fi);
                        let mut vals = Vec::new();
                        for l in items {
                            check_cmp_types(&rname, lhs_ft, CmpOp::Eq, lit_type(l))?;
                            vals.push(lit_value(l));
                        }
                        cmps.push(CompiledCmp {
                            field_idx: fi,
                            test: Test::InList { items: vals, negated: *negated },
                        });
                    }
                }
            }
            let beta = cmps
                .iter()
                .any(|c| matches!(c.test, Test::Cmp { rhs: Src::Field(..), .. }));
            let (pindex, index_ci) = {
                let var_cmps: Vec<(usize, CmpOp, usize, usize)> = cmps
                    .iter()
                    .enumerate()
                    .filter_map(|(ci, c)| match &c.test {
                        Test::Cmp { op, rhs: Src::Field(ti, fi) } if Some(*ti) != tpos => {
                            Some((ci, *op, *ti, *fi))
                        }
                        _ => None,
                    })
                    .collect();
                if var_cmps.iter().any(|(_, op, _, _)| *op == CmpOp::Eq) {
                    (phreak::Index::Eq, None)
                } else if p.ce != CeKind::Positive {
                    // range index (not/exists only): first relational var
                    // constraint with Number/Number or same-type operands
                    let mut found = (phreak::Index::None, None);
                    for (ci, op, ti, fi) in &var_cmps {
                        if !matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge) {
                            continue;
                        }
                        let lhs_ft = self.store.field_type(type_id, cmps[*ci].field_idx);
                        let src_pat = patterns
                            .iter()
                            .find(|q: &&CompiledPattern| q.tpos == Some(*ti))
                            .expect("binding source pattern");
                        let rhs_ft = self.store.field_type(src_pat.type_id, *fi);
                        let numeric =
                            |t: FieldType| matches!(t, FieldType::I64 | FieldType::F64);
                        if (numeric(lhs_ft) && numeric(rhs_ft)) || lhs_ft == rhs_ft {
                            found = (phreak::Index::Cmp(*op), Some(*ci));
                            break;
                        }
                    }
                    found
                } else {
                    (phreak::Index::None, None)
                }
            };
            patterns.push(CompiledPattern {
                type_id,
                cmps,
                listen_mask,
                ce: p.ce,
                tpos,
                beta,
                pindex,
                index_ci,
            });
        }

        let mut actions = Vec::new();
        for a in &def.actions {
            match a {
                Action::Insert { type_name, args } => {
                    if type_name == INITIAL_FACT {
                        return Err(err(format!("type name {INITIAL_FACT} is reserved")));
                    }
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
                    let (pos, tid) = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
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
                    let (pos, _) = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
                    actions.push(CompiledAction::Update { pos });
                }
                Action::Delete { var } => {
                    let (pos, _) = *fact_binds
                        .get(var)
                        .ok_or_else(|| err(format!("unknown fact binding {var}")))?;
                    actions.push(CompiledAction::Delete { pos });
                }
            }
        }
        Ok(CompiledRule { def, patterns, actions, flips: Vec::new() })
    }

    fn compile_arg(
        &self,
        rname: &str,
        arg: &RhsArg,
        fact_binds: &HashMap<String, (usize, TypeId)>,
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
                Ok((Src::SnapField(pi, fi), ft))
            }
            RhsArg::Getter { var, field } => {
                let (pos, tid) = *fact_binds.get(var).ok_or_else(|| {
                    EngineError(format!("rule {rname}: unknown fact binding {var}"))
                })?;
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
        if type_name == INITIAL_FACT {
            return Err(EngineError(format!("type name {INITIAL_FACT} is reserved")));
        }
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
        if !self.lists_built {
            self.nets = self
                .rules
                .iter()
                .map(|r| {
                    let k = r.patterns.len();
                    RuleNet {
                        s0: Staged::default(),
                        nodes: (1..k)
                            .map(|j| {
                                let pat = &r.patterns[j];
                                let kind = match pat.ce {
                                    CeKind::Positive => phreak::Kind::Join,
                                    CeKind::Not => phreak::Kind::Not,
                                    CeKind::Exists => phreak::Kind::Exists,
                                };
                                phreak::Node::new(pat.pindex, j == 1, kind)
                            })
                            .collect(),
                        queue: Vec::new(),
                        active: vec![HashSet::new(); k],
                        queued: false,
                        pulse: vec![false; k],
                    }
                })
                .collect();
            self.lists_built = true;
            let initial: Vec<FactId> = self.store.live_facts().collect();
            for f in initial {
                self.on_insert(f, None);
            }
        }
        let mut firings = Vec::new();
        let mut last_fired: Option<usize> = None;
        while let Some(ri) = self.next_activation(last_fired) {
            last_fired = Some(ri);
            if firings.len() >= limit {
                return Err(EngineError(format!(
                    "fire limit {limit} reached (non-terminating?)"
                )));
            }
            // RuleExecutor.getNextTuple: removeFirst + setQueued(false).
            let tuple = self.nets[ri].queue.remove(0);
            self.execute_rhs(ri, &tuple)?;
            // Post-RHS rendering (D-013 / j03).
            let matches: Vec<FactView> = tuple.iter().map(|&f| self.store.render(f)).collect();
            firings.push(Firing { rule: self.rules[ri].def.name.clone(), matches });
        }
        Ok(firings)
    }

    /// Agenda (D-018/D-027): eager (no-loop) rules evaluate per flush with
    /// reverse-creation terminal appends; the just-fired rule re-evaluates
    /// even if self-unlinked (fz_42_5243); lazy rules evaluate on reach
    /// with creation-order terminal appends.
    fn next_activation(&mut self, last: Option<usize>) -> Option<usize> {
        if let Some(l) = last {
            self.evaluate_rule(l, true, false);
            if self.nets[l].queue.is_empty() {
                self.nets[l].queued = false; // emptied item leaves agenda
            }
        }
        for i in 0..self.rule_order.len() {
            let ri = self.rule_order[i];
            if self.rules[ri].def.no_loop {
                self.evaluate_rule(ri, false, true);
            }
        }
        for i in 0..self.rule_order.len() {
            let ri = self.rule_order[i];
            self.evaluate_rule(ri, false, false);
            if !self.nets[ri].queue.is_empty() {
                return Some(ri);
            }
            if self.nets[ri].queued {
                self.nets[ri].queued = false; // evaluated empty: removed
            }
        }
        None
    }

    fn on_insert(&mut self, f: FactId, origin: Origin) {
        for ri in 0..self.rules.len() {
            for pos in 0..self.rules[ri].patterns.len() {
                if self.alpha_passes(ri, pos, f) {
                    self.nets[ri].active[pos].insert(f);
                    self.maybe_pulse(ri, pos);
                    if pos == 0 {
                        self.nets[ri].s0.add_ins(f, origin);
                    } else {
                        self.nets[ri].nodes[pos - 1].s_right.add_ins(f, origin);
                    }
                }
            }
            self.refresh_linked(ri);
        }
    }

    /// The first right insert into an UNCONSTRAINED not node force-links
    /// it for one evaluation so the blocking batch processes, after which
    /// it unlinks again (D-031, NotNode.assertObject).
    fn maybe_pulse(&mut self, ri: usize, pos: usize) {
        let pat = &self.rules[ri].patterns[pos];
        if pos > 0
            && pat.ce == CeKind::Not
            && !pat.beta
            && self.nets[ri].active[pos].len() == 1
        {
            self.nets[ri].pulse[pos] = true;
        }
    }

    fn on_update(&mut self, f: FactId, mask: u64, src_ri: usize) {
        let ftype = self.store.fact_type(f);
        for ri in 0..self.rules.len() {
            let was_linked = self.rule_linked(ri);
            for pos in 0..self.rules[ri].patterns.len() {
                let pat = &self.rules[ri].patterns[pos];
                if pat.type_id != ftype {
                    continue;
                }
                let was = self.nets[ri].active[pos].contains(&f);
                let now = self.alpha_passes(ri, pos, f);
                let origin = Some(src_ri);
                match (was, now) {
                    (false, true) => {
                        self.nets[ri].active[pos].insert(f);
                        self.maybe_pulse(ri, pos);
                        if pos == 0 {
                            self.nets[ri].s0.add_ins(f, origin);
                        } else {
                            self.nets[ri].nodes[pos - 1].s_right.add_ins(f, origin);
                        }
                    }
                    (true, false) => {
                        self.nets[ri].active[pos].remove(&f);
                        if pos == 0 {
                            self.nets[ri].s0.add_del(f, origin);
                        } else {
                            self.nets[ri].nodes[pos - 1].s_right.add_del(f, origin);
                        }
                    }
                    (true, true) => {
                        // ALL-SET mask (bare update) is class-reactive
                        // (fz_42_3311); property masks need intersection.
                        if mask == u64::MAX || pat.listen_mask & mask != 0 {
                            if pos == 0 {
                                self.nets[ri].s0.add_upd(f, origin);
                            } else {
                                self.nets[ri].nodes[pos - 1].s_right.add_upd(f, origin);
                            }
                        } else if pos > 0 {
                            // mask miss: immediate right-memory reAdd, no
                            // staging (fz_42_4359). Not nodes use the
                            // existential variant: blocked lefts re-search
                            // and unmatched ones stay DETACHED (D-031,
                            // NotNode.reorderRightTuple's null sink).
                            let env =
                                JoinEnvImpl { store: &self.store, rule: &self.rules[ri] };
                            if self.rules[ri].patterns[pos].ce == CeKind::Not {
                                self.nets[ri].nodes[pos - 1]
                                    .not_mask_miss_re_add(&env, pos - 1, f);
                            } else {
                                let key = phreak::JoinEnv::key_of_right(&env, pos - 1, f);
                                self.nets[ri].nodes[pos - 1].re_add_right_fact(f, key);
                            }
                        }
                    }
                    (false, false) => {}
                }
            }
            // PathMemory.doUnlinkRule (D-031/ne_x2): a LINKED->UNLINKED
            // transition queues the agenda item so cancellations and
            // unblocks evaluate in their own window.
            if was_linked && !self.rule_linked(ri) {
                self.nets[ri].queued = true;
            }
            self.refresh_linked(ri);
        }
    }

    fn on_delete(&mut self, f: FactId, origin: Origin) {
        for ri in 0..self.rules.len() {
            let was_linked = self.rule_linked(ri);
            for pos in 0..self.rules[ri].patterns.len() {
                if self.nets[ri].active[pos].remove(&f) {
                    if pos == 0 {
                        self.nets[ri].s0.add_del(f, origin);
                    } else {
                        self.nets[ri].nodes[pos - 1].s_right.add_del(f, origin);
                    }
                }
            }
            // PathMemory.doUnlinkRule (D-031/ne_x2): a LINKED->UNLINKED
            // transition queues the agenda item so the delete window
            // evaluates before later re-inserts (exists refire-after-gap).
            if was_linked && !self.rule_linked(ri) {
                self.nets[ri].queued = true;
            }
            // A delete can also LINK a rule: an unconstrained not node
            // re-links when its right input empties (NotNode.doDeleteRightTuple).
            self.refresh_linked(ri);
        }
    }

    /// Per-position segment-linking requirement (D-031): positive and
    /// exists positions need alpha data; a constrained not is always
    /// linked; an unconstrained not is linked while its right input is
    /// EMPTY (or transiently via the insert pulse).
    fn pos_linked(&self, ri: usize, pos: usize) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        match pat.ce {
            CeKind::Positive | CeKind::Exists => !self.nets[ri].active[pos].is_empty(),
            CeKind::Not => {
                pat.beta
                    || self.nets[ri].active[pos].is_empty()
                    || self.nets[ri].pulse[pos]
            }
        }
    }

    fn rule_linked(&self, ri: usize) -> bool {
        (0..self.rules[ri].patterns.len()).all(|p| self.pos_linked(ri, p))
    }

    /// Dirty-notification: (re)queue the rule's agenda item if the rule
    /// is currently linked.
    fn refresh_linked(&mut self, ri: usize) {
        if !self.nets[ri].queued && self.rule_linked(ri) && self.rule_dirty(ri) {
            self.nets[ri].queued = true;
        }
    }

    fn rule_dirty(&self, ri: usize) -> bool {
        !self.nets[ri].s0.is_empty()
            || self.nets[ri].nodes.iter().any(|n| !n.s_right.is_empty() || !n.s_left.is_empty())
    }

    fn evaluate_rule(&mut self, ri: usize, force: bool, eager: bool) {
        if !self.rule_dirty(ri) {
            return;
        }
        let k = self.rules[ri].patterns.len();
        // Segment linking: an unlinked rule still evaluates its network
        // when reached (memories advance, fz_42_1464) unless it was NEVER
        // linked (fz_7_145: staged input accumulates until first link).
        // The queue is pruned of deactivated facts (j05); tuples hold
        // POSITIVE positions only, so map pattern -> tuple index.
        if !self.rule_linked(ri) {
            let active = self.nets[ri].active.clone();
            let positives: Vec<(usize, usize)> = self.rules[ri]
                .patterns
                .iter()
                .enumerate()
                .filter_map(|(pos, p)| p.tpos.map(|t| (pos, t)))
                .collect();
            self.nets[ri]
                .queue
                .retain(|t| positives.iter().all(|(pos, ti)| active[*pos].contains(&t[*ti])));
        }
        // Agenda-item gate: only a queued item evaluates (the just-fired
        // rule is force-evaluated, fz_42_5243).
        if !force && !self.nets[ri].queued {
            return;
        }

        let s0 = self.nets[ri].s0.take();
        let no_loop = self.rules[ri].def.no_loop;

        if k == 1 {
            // LIA -> terminal directly; working-memory staging consumed
            // OLDEST-first (pr08/pr04 pin).
            let net = &mut self.nets[ri];
            for (f, _, _) in s0.del.iter().rev() {
                net.queue.retain(|t| t[0] != *f);
            }
            for (f, o, _) in s0.upd.iter().rev() {
                let queued = net.queue.iter().any(|t| t[0] == *f);
                if queued {
                    continue; // pending: keep position
                }
                if no_loop && *o == Some(ri) {
                    continue; // own update does not re-activate (j04)
                }
                net.queue.push(vec![*f]);
            }
            for (f, o, _) in s0.ins.iter().rev() {
                if no_loop && *o == Some(ri) {
                    continue;
                }
                net.queue.push(vec![*f]);
            }
            return;
        }

        // Multi-node chain. Split borrows: env reads store+rules, nodes
        // mutate nets.
        let env = JoinEnvImpl { store: &self.store, rule: &self.rules[ri] };
        let net = &mut self.nets[ri];
        let mut src: Staged<Tup> = Staged::default();
        // pos0 staged facts become 1-tuples (consume order preserved).
        src.ins = s0.ins.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();
        src.upd = s0.upd.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();
        src.del = s0.del.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();

        for j in 0..net.nodes.len() {
            // merge this batch into any left-staged remainder (from
            // unlinked accumulation), with clash-move semantics
            let pending = net.nodes[j].s_left.take();
            src = Staged::merge_into_pending(pending, src);
            let sr = net.nodes[j].s_right.take();
            let mut trg: Staged<Tup> = Staged::default();
            phreak::do_node(&env, j, &mut net.nodes[j], src, sr, &mut trg);
            // Segment-boundary flip (D-033): a non-first sink of a shared
            // node receives the staged propagation REVERSED.
            if self.rules[ri].flips[j] {
                trg.ins.reverse();
                trg.upd.reverse();
                trg.del.reverse();
            }
            src = trg;
        }
        // Consuming the batch spends any not-node link pulses
        // (unlinkNotNodeOnRightInsert, D-031).
        for p in net.pulse.iter_mut() {
            *p = false;
        }

        // Terminal (PhreakRuleTerminalNode + RuleExecutor):
        // deletes, then updates, then inserts. Lazy evaluations append in
        // CREATION order (oldest staged first); eager-list evaluations in
        // reverse-creation order (D-027 calibration).
        for (t, _, _) in src.del.iter() {
            net.queue.retain(|x| x != t);
        }
        // Terminal (PhreakRuleTerminalNode): updates then inserts, each
        // consumed staged-list head-first, appending to the executor's
        // tuple list. A queued activation keeps its position; an unqueued
        // (fired) one is effectively recreated.
        for (t, o, _) in src.upd.iter() {
            if net.queue.iter().any(|x| x == t) {
                continue;
            }
            if no_loop && *o == Some(ri) {
                continue;
            }
            net.queue.push(t.clone());
        }
        for (t, o, _) in src.ins.iter() {
            if no_loop && *o == Some(ri) {
                continue;
            }
            net.queue.push(t.clone());
        }
    }

    fn alpha_passes(&self, ri: usize, pos: usize, f: FactId) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        if !self.store.is_alive(f) || self.store.fact_type(f) != pat.type_id {
            return false;
        }
        pat.cmps.iter().all(|c| {
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::Cmp { op, rhs: Src::Lit(v) } => eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } if Some(*ti) == pat.tpos => {
                    eval_cmp_join(&lhs, *op, &self.store.value(f, *fi))
                }
                // join constraint, checked with prefix; SnapField never
                // occurs in LHS constraints
                Test::Cmp { .. } => true,
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    fn execute_rhs(&mut self, ri: usize, tuple: &[FactId]) -> Result<(), EngineError> {
        // Declaration snapshot: binding values are extracted once when the
        // consequence starts (fz_7_2525).
        let snapshot: Vec<Vec<Value>> = tuple
            .iter()
            .map(|&f| {
                let tid = self.store.fact_type(f);
                (0..self.store.schema(tid).fields.len())
                    .map(|fi| self.store.value(f, fi))
                    .collect()
            })
            .collect();
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
                                coerce(self.eval_src(a, tuple, &snapshot), *ft).ok_or_else(|| {
                                    EngineError("RHS insert: arg type mismatch".into())
                                })
                            })
                            .collect::<Result<_, _>>()?
                    };
                    let fid = self.store.insert(tid, values).map_err(EngineError)?;
                    self.on_insert(fid, Some(ri));
                }
                CompiledAction::Set { pos, field_idx, arg } => {
                    let f = tuple[*pos];
                    let fi = *field_idx;
                    let tid = self.store.fact_type(f);
                    let ft = self.store.field_type(tid, fi);
                    let v = coerce(self.eval_src(&arg.clone(), tuple, &snapshot), ft)
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
                    self.on_update(f, mask, ri);
                }
                CompiledAction::Delete { pos } => {
                    self.store.kill(tuple[*pos]);
                    self.on_delete(tuple[*pos], Some(ri));
                }
            }
        }
        Ok(())
    }

    fn eval_src(&self, src: &Src, tuple: &[FactId], snapshot: &[Vec<Value>]) -> Value {
        match src {
            Src::Lit(v) => v.clone(),
            Src::Field(pi, fi) => self.store.value(tuple[*pi], *fi),
            Src::SnapField(pi, fi) => snapshot[*pi][*fi].clone(),
        }
    }

    /// All live facts, in insertion order, rendered. The synthetic
    /// InitialFact never appears here (matches session.getObjects()).
    pub fn facts(&self) -> Vec<FactView> {
        self.store
            .live_facts()
            .filter(|f| Some(*f) != self.init_fact)
            .map(|f| self.store.render(f))
            .collect()
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

/// Join-environment bridge for phreak::do_node: constraint tests use LIVE
/// values; index keys are the eq-join constraint values normalized to the
/// FIELD's type (u14: a double binding indexed against a long field
/// truncates toward zero).
struct JoinEnvImpl<'a> {
    store: &'a FactStore,
    rule: &'a CompiledRule,
}

impl phreak::JoinEnv for JoinEnvImpl<'_> {
    fn allowed(&self, node: usize, l: &Tup, f: FactId) -> bool {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        pat.cmps.iter().all(|c| {
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::Cmp { op, rhs: Src::Lit(v) } => eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } => {
                    let other = if Some(*ti) == pat.tpos { f } else { l[*ti] };
                    eval_cmp_join(&lhs, *op, &self.store.value(other, *fi))
                }
                Test::Cmp { .. } => true,
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    fn key_of_left(&self, node: usize, l: &Tup) -> Option<Vec<Value>> {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        if let Some(ci) = pat.index_ci {
            // range index: the single relational constraint's binding value
            if let Test::Cmp { rhs: Src::Field(ti, fi), .. } = &pat.cmps[ci].test {
                return Some(vec![self.store.value(l[*ti], *fi)]);
            }
        }
        let mut out = Vec::new();
        for c in &pat.cmps {
            if let Test::Cmp { op: CmpOp::Eq, rhs: Src::Field(ti, fi) } = &c.test {
                if Some(*ti) != pat.tpos {
                    // stored in the binding's natural type; coercion
                    // happens at probe time (u14 / fz_123_3057)
                    out.push(self.store.value(l[*ti], *fi));
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    fn key_of_right(&self, node: usize, f: FactId) -> Option<Vec<Value>> {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        if let Some(ci) = pat.index_ci {
            // range index: the constraint's own field value
            return Some(vec![self.store.value(f, pat.cmps[ci].field_idx)]);
        }
        let mut out = Vec::new();
        for c in &pat.cmps {
            if let Test::Cmp { op: CmpOp::Eq, rhs: Src::Field(ti, _) } = &c.test {
                if Some(*ti) != pat.tpos {
                    out.push(self.store.value(f, c.field_idx));
                }
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Existential test: skip the range-indexed constraint — the index
    /// probe decided it with stored-type coercion (ne_r3/ne_r5, D-035).
    fn allowed_ce(&self, node: usize, l: &Tup, f: FactId) -> bool {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        pat.cmps.iter().enumerate().all(|(ci, c)| {
            if pat.index_ci == Some(ci) {
                return true;
            }
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::Cmp { op, rhs: Src::Lit(v) } => eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } => {
                    let other = if Some(*ti) == pat.tpos { f } else { l[*ti] };
                    eval_cmp_join(&lhs, *op, &self.store.value(other, *fi))
                }
                Test::Cmp { .. } => true,
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    /// <=1 beta constraint always allows the optimization (Single/Empty
    /// BetaConstraints); with more, every one must be an equality (D-031).
    fn left_update_optimization(&self, node: usize) -> bool {
        let pat = &self.rule.patterns[node + 1];
        let betas: Vec<&CompiledCmp> = pat
            .cmps
            .iter()
            .filter(|c| {
                matches!(&c.test, Test::Cmp { rhs: Src::Field(ti, _), .. } if Some(*ti) != pat.tpos)
            })
            .collect();
        betas.len() <= 1
            || betas
                .iter()
                .all(|c| matches!(&c.test, Test::Cmp { op: CmpOp::Eq, .. }))
    }
}

/// Non-Cmp alpha tests (D-030): matches = full-string regex on Strings;
/// contains = substring; in = OR of ==-with-promotion branches (a double
/// literal never truncates against a long field here — op_i3).
fn eval_alpha_test(lhs: &Value, test: &Test) -> bool {
    match test {
        Test::Matches(r) => matches!(lhs, Value::Str(s) if r.accepts(s)),
        Test::Contains(needle) => matches!(lhs, Value::Str(s) if s.contains(needle.as_str())),
        Test::InList { items, negated } => {
            let hit = items.iter().any(|v| eval_cmp(lhs, CmpOp::Eq, v));
            hit != *negated
        }
        Test::Cmp { .. } => unreachable!("Cmp handled by callers"),
    }
}

/// Variable (join) constraint evaluation: Drools' indexed `==` coerces the
/// bound value to the LEFT field's type — a double binding compared to a
/// long field is CAST (truncated toward zero), so `n == $x` with n=0,
/// $x=-0.5 MATCHES (u14/fz_7_4974). `!=` and relational joins promote to
/// double like literals do (u14/u15).
fn eval_cmp_join(lhs: &Value, op: CmpOp, rhs: &Value) -> bool {
    if op == CmpOp::Eq {
        if let (Value::I64(a), Value::F64(b)) = (lhs, rhs) {
            return *a == (*b as i64); // Java (long) cast: truncate toward zero
        }
    }
    eval_cmp(lhs, op, rhs)
}

/// Same-type / promoted comparison, exported for the phreak range scans.
pub(crate) fn eval_cmp_pub(lhs: &Value, op: CmpOp, rhs: &Value) -> bool {
    eval_cmp(lhs, op, rhs)
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
