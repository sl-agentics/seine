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

use crate::drl::{self, AccFunc, Action, CeKind, CmpOp, CmpRhs, Constraint, Literal, RhsArg, RuleDef};
use crate::store::{FactId, FactStore, FactView, FieldType, TypeId, TypeSchema, Value};

/// Reserved type name backing first-position not/exists CEs (D-031):
/// Drools matches those rules on InitialFactImpl. The engine keeps one
/// synthetic fact of this hidden zero-field type; it renders in match
/// lists but never in the final fact set.
pub(crate) const INITIAL_FACT: &str = "InitialFact";
/// Hidden types backing accumulate results (D-038): each accumulate
/// context owns one synthetic fact of the matching type, updated in
/// place as the set mutates; collect results carry their element list
/// in an engine side table.
pub(crate) const ACC_LONG: &str = "Long";
pub(crate) const ACC_DOUBLE: &str = "Double";
pub(crate) const ACC_COLLECTION: &str = "Collection";
const RESERVED_TYPES: [&str; 4] = [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION];

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
    /// Original variable name for Var-rhs constraints — part of node
    /// identity (D-037/ne_t13: `f1 != $x` and `f1 != $y` do NOT share
    /// even when $x/$y bind the same field; unreferenced declarations
    /// stay name-irrelevant per ne_t2).
    rhs_var: Option<String>,
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
    /// SET of field-bound indices (bitmask): part of node-sharing
    /// identity (D-036/ne_t1..t10 — binding names, order, duplicates and
    /// fact-level bindings are irrelevant; the bound-field SET is not).
    bind_fields: u64,
    /// Accumulate/collect spec (D-038): the pattern describes the SOURCE;
    /// the node emits one synthetic result per left context.
    acc: Option<CompiledAcc>,
}

#[derive(Clone)]
struct CompiledAcc {
    func: AccFunc,
    /// Source field accumulated over (None for count/collect).
    arg_field: Option<usize>,
    arg_ft: FieldType,
    /// Hidden result type (ACC_LONG / ACC_DOUBLE / ACC_COLLECTION).
    result_tid: TypeId,
    /// Original arg variable name — identity-significant like any
    /// referenced variable (D-037 spirit; conservative).
    arg_name: Option<String>,
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

use crate::phreak::{self, Origin, Staged, Tup};

/// Level-0 network node: one per distinct pattern-0 identity (the LIA +
/// its alpha chain). Shared across every rule whose first pattern has the
/// same structural key (D-036).
struct Lia {
    /// Canonical (rule, pattern position 0) for alpha evaluation.
    env: (usize, usize),
    active: HashSet<FactId>,
    /// Level-1 trie nodes fed by this LIA. LIA propagation is EAGER (at
    /// WM-action time, D-014) and per-child copies carry no flip — every
    /// child sees identical staging (pinned by the whole multi-rule
    /// corpus).
    children: Vec<usize>,
    /// Single-pattern rules on this LIA (terminal-only paths keep their
    /// own pos0 staging with the pr04/pr08 oldest-first consumption).
    k1_rules: Vec<usize>,
}

/// A sink of a shared beta node, in BUILD (rule declaration) order.
#[derive(Clone, Copy)]
enum Sink {
    Node(usize),
    Term(usize),
}

/// One SHARED beta node in the prefix trie (D-036/D-037): rules whose
/// pattern prefixes are structurally equal share the node instance, its
/// memories and its staging — the node evaluates ONCE per window (at the
/// first sharer's agenda turn to reach it) and each batch propagates to
/// every sink: the FIRST-built sink receives the staged lists appended
/// as-is (TupleSetsImpl.addAll — lagging sinks accumulate batches FIFO),
/// every later sink a REVERSED per-batch copy (SegmentPropagator peer
/// prepends — lagging peers stack batches LIFO).
struct TrieNode {
    node: phreak::Node,
    /// Canonical (rule, pattern position) for constraints/keys.
    env: (usize, usize),
    /// Alpha membership of this node's right input.
    active: HashSet<FactId>,
    /// Transient link pulse for UNCONSTRAINED not nodes (D-031).
    pulse: bool,
    /// Level-1 only: pattern-0 fact staging from the owning LIA.
    s0_in: Staged<FactId>,
    /// Child sinks in build order (first = preserved propagation).
    sinks: Vec<Sink>,
    /// Accumulate contexts per left tuple (D-038).
    acc: HashMap<Tup, AccCtx>,
    /// Lefts holding a match on each right source fact, in match order.
    acc_by_right: HashMap<FactId, Vec<Tup>>,
    /// LEVEL-1 COLLECT only (D-040): pattern-0 fields referenced anywhere
    /// downstream (collect beta constraints, later patterns, RHS args).
    /// The LIA drops a pattern-0 property MODIFY into this child unless
    /// the modification mask intersects — CollectAccumulator is known to
    /// read nothing from the left, so its left inferred mask is just the
    /// inherited interest (unlike inline accumulates, whose opaque
    /// lambdas force ALL-SET and always re-propagate).
    collect_left_gate: Option<u64>,
}

/// One accumulate context: the function state, the stored per-match
/// contributions (reverse operates on these, never on live fields), and
/// the reused synthetic result fact (D-038).
struct AccCtx {
    result: Option<FactId>,
    propagated: bool,
    /// (right fact, stored contribution) in match order.
    matches: Vec<(FactId, Value)>,
    sum_i: i64,
    sum_f: f64,
    count: i64,
    minmax: Option<Value>,
    list: Vec<FactId>,
}

impl AccCtx {
    fn new() -> AccCtx {
        AccCtx {
            result: None,
            propagated: false,
            matches: Vec::new(),
            sum_i: 0,
            sum_f: 0.0,
            count: 0,
            minmax: None,
            list: Vec::new(),
        }
    }

    /// reinit (fresh function state; the result fact and propagation
    /// flag survive — the handle is reused).
    fn reset_state(&mut self) {
        self.sum_i = 0;
        self.sum_f = 0.0;
        self.count = 0;
        self.minmax = None;
        self.list.clear();
    }

    /// accumulate(): the exact op sequence (D-038).
    fn apply(&mut self, func: AccFunc, f: FactId, v: &Value) {
        match func {
            AccFunc::Sum => match v {
                Value::I64(x) => self.sum_i += x,
                Value::F64(x) => self.sum_f += x,
                _ => {}
            },
            AccFunc::Count => self.count += 1,
            AccFunc::Average => {
                let x = match v {
                    Value::I64(n) => *n as f64,
                    Value::F64(n) => *n,
                    _ => 0.0,
                };
                self.sum_f += x;
                self.count += 1;
            }
            AccFunc::Min | AccFunc::Max => {
                let better = match &self.minmax {
                    None => true,
                    Some(cur) => {
                        let ord = match (v, cur) {
                            (Value::I64(a), Value::I64(b)) => a.cmp(b),
                            (Value::F64(a), Value::F64(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            _ => std::cmp::Ordering::Equal,
                        };
                        if func == AccFunc::Min {
                            ord == std::cmp::Ordering::Less
                        } else {
                            ord == std::cmp::Ordering::Greater
                        }
                    }
                };
                if better {
                    self.minmax = Some(v.clone());
                }
            }
            AccFunc::Collect => self.list.push(f),
        }
    }

    /// tryReverse(): true when reversed in place; min/max cannot reverse
    /// and require a reinit + refold over the remaining matches (D-038).
    fn try_reverse(&mut self, func: AccFunc, f: FactId, v: &Value) -> bool {
        match func {
            AccFunc::Sum => {
                match v {
                    Value::I64(x) => self.sum_i -= x,
                    Value::F64(x) => self.sum_f -= x,
                    _ => {}
                }
                true
            }
            AccFunc::Count => {
                self.count -= 1;
                true
            }
            AccFunc::Average => {
                let x = match v {
                    Value::I64(n) => *n as f64,
                    Value::F64(n) => *n,
                    _ => 0.0,
                };
                self.sum_f -= x;
                self.count -= 1;
                true
            }
            AccFunc::Min | AccFunc::Max => false,
            AccFunc::Collect => {
                if let Some(i) = self.list.iter().position(|x| *x == f) {
                    self.list.remove(i);
                }
                true
            }
        }
    }

    /// getResult(): None (average/min/max of an empty set) blocks
    /// propagation and retracts an existing child (D-038).
    fn result_value(&self, func: AccFunc, arg_ft: FieldType) -> Option<Value> {
        match func {
            AccFunc::Sum => Some(match arg_ft {
                FieldType::I64 => Value::I64(self.sum_i),
                _ => Value::F64(self.sum_f),
            }),
            AccFunc::Count => Some(Value::I64(self.count)),
            AccFunc::Average => {
                if self.count == 0 {
                    None
                } else {
                    Some(Value::F64(self.sum_f / self.count as f64))
                }
            }
            AccFunc::Min | AccFunc::Max => self.minmax.clone(),
            AccFunc::Collect => Some(Value::I64(0)), // list lives in collect_vals
        }
    }
}

/// Per-rule agenda/terminal state (the beta network itself is shared).
struct RuleNet {
    /// k=1 rules: pos0 staging, consumed OLDEST-first (pr08/pr04 pin).
    s0: Staged<FactId>,
    /// This rule's LIA and trie path (one node per pattern 1..k-1).
    lia: usize,
    path: Vec<usize>,
    /// Terminal staging propagated from the last path node (per-sink
    /// copy); consumed into `queue` at this rule's evaluation.
    term_pending: Staged<Tup>,
    queue: Vec<Tup>,
    /// Agenda-item lifecycle: set when the rule links (or re-dirties
    /// while linked), cleared when its evaluation leaves the queue empty.
    /// An unlinked-but-queued rule still evaluates when reached
    /// (fz_42_1464); an unqueued rule accumulates staged input
    /// (fz_42_124, fz_7_145).
    queued: bool,
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// Rule indices sorted by (salience desc, declaration order).
    rule_order: Vec<usize>,
    /// Shared network: level-0 LIAs and the beta-node prefix trie.
    lias: Vec<Lia>,
    trie: Vec<TrieNode>,
    /// Per-rule agenda/terminal state.
    nets: Vec<RuleNet>,
    lists_built: bool,
    /// The synthetic InitialFact (inserted before scenario facts once a
    /// CE-first rule compiles — Drools asserts it at session init).
    init_fact: Option<FactId>,
    /// Collect results: synthetic Collection fact -> current element
    /// list, updated at each result evaluation (D-038).
    collect_vals: HashMap<FactId, Vec<FactId>>,
}

impl Engine {
    pub fn new(mut schemas: Vec<TypeSchema>) -> Result<Engine, EngineError> {
        let mut seen = HashSet::new();
        for s in &schemas {
            if !seen.insert(s.name.clone()) {
                return Err(EngineError(format!("duplicate type {}", s.name)));
            }
            if RESERVED_TYPES.contains(&s.name.as_str()) {
                return Err(EngineError(format!("type name {} is reserved", s.name)));
            }
            if s.fields.len() > 64 {
                return Err(EngineError(format!("type {}: more than 64 fields", s.name)));
            }
        }
        // Hidden types: first-position CEs (D-031) + accumulate results
        // (D-038).
        schemas.push(TypeSchema { name: INITIAL_FACT.into(), fields: Vec::new() });
        schemas.push(TypeSchema {
            name: ACC_LONG.into(),
            fields: vec![("value".into(), FieldType::I64)],
        });
        schemas.push(TypeSchema {
            name: ACC_DOUBLE.into(),
            fields: vec![("value".into(), FieldType::F64)],
        });
        schemas.push(TypeSchema { name: ACC_COLLECTION.into(), fields: Vec::new() });
        Ok(Engine {
            store: FactStore::new(schemas),
            rules: Vec::new(),
            rule_order: Vec::new(),
            lias: Vec::new(),
            trie: Vec::new(),
            nets: Vec::new(),
            lists_built: false,
            init_fact: None,
            collect_vals: HashMap::new(),
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
        // Structural node identity uses the PRE-rewrite constraint values
        // (the D-029 literal rewrite itself groups by these same keys).
        let keys: Vec<Vec<String>> = self
            .rules
            .iter()
            .map(|r| r.patterns.iter().map(|p| self.pattern_key(p)).collect())
            .collect();
        self.share_and_hash_alphas();
        self.build_network(&keys);
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

    /// Build the shared network (D-033/D-036/D-037, probes ne_s*/ne_t*):
    /// rules with structurally equal pattern PREFIXES share LIAs and beta
    /// nodes. Rules are added in declaration order, so sinks attach to
    /// each shared node in build order — the propagation contract (first
    /// sink preserved, later sinks flipped per batch) follows from that
    /// ordering (identical-LHS twins fire in opposite orders, ne_s7;
    /// declaration order picks the preserved one, ne_s8; boundaries stack
    /// per depth, ne_s10; a never-linked first sink still holds the
    /// preserved copy, ne_t5).
    fn build_network(&mut self, keys: &[Vec<String>]) {
        let mut lia_index: HashMap<String, usize> = HashMap::new();
        let mut trie_index: HashMap<String, usize> = HashMap::new();
        for ri in 0..self.rules.len() {
            let k = self.rules[ri].patterns.len();
            let lia = *lia_index.entry(keys[ri][0].clone()).or_insert_with(|| {
                self.lias.push(Lia {
                    env: (ri, 0),
                    active: HashSet::new(),
                    children: Vec::new(),
                    k1_rules: Vec::new(),
                });
                self.lias.len() - 1
            });
            let mut path = Vec::new();
            if k == 1 {
                self.lias[lia].k1_rules.push(ri);
            } else {
                let mut prefix = keys[ri][0].clone();
                let mut parent: Option<usize> = None;
                for j in 1..k {
                    prefix.push_str("||");
                    prefix.push_str(&keys[ri][j]);
                    let nid = match trie_index.get(&prefix) {
                        Some(&nid) => nid,
                        None => {
                            let pat = &self.rules[ri].patterns[j];
                            let kind = if pat.acc.is_some() {
                                phreak::Kind::Acc
                            } else {
                                match pat.ce {
                                    CeKind::Positive => phreak::Kind::Join,
                                    CeKind::Not => phreak::Kind::Not,
                                    CeKind::Exists => phreak::Kind::Exists,
                                }
                            };
                            self.trie.push(TrieNode {
                                node: phreak::Node::new(pat.pindex, kind),
                                env: (ri, j),
                                active: HashSet::new(),
                                pulse: false,
                                s0_in: Staged::default(),
                                sinks: Vec::new(),
                                acc: HashMap::new(),
                                acc_by_right: HashMap::new(),
                                collect_left_gate: None,
                            });
                            let nid = self.trie.len() - 1;
                            trie_index.insert(prefix.clone(), nid);
                            match parent {
                                None => self.lias[lia].children.push(nid),
                                Some(p) => self.trie[p].sinks.push(Sink::Node(nid)),
                            }
                            nid
                        }
                    };
                    path.push(nid);
                    parent = Some(nid);
                }
                self.trie[parent.unwrap()].sinks.push(Sink::Term(ri));
            }
            self.nets.push(RuleNet {
                s0: Staged::default(),
                lia,
                path,
                term_pending: Staged::default(),
                queue: Vec::new(),
                queued: false,
            });
        }
        // LEVEL-1 COLLECT gates (D-040): pattern-0 fields referenced
        // downstream, unioned across every rule sharing the node.
        for ri in 0..self.rules.len() {
            let Some(&first) = self.nets[ri].path.first() else { continue };
            let (eri, epos) = self.trie[first].env;
            let is_collect = self.rules[eri].patterns[epos]
                .acc
                .as_ref()
                .is_some_and(|a| a.func == AccFunc::Collect);
            if !is_collect {
                continue;
            }
            let mut gate = 0u64;
            for pat in &self.rules[ri].patterns[1..] {
                for c in &pat.cmps {
                    if let Test::Cmp { rhs: Src::Field(0, fi), .. } = &c.test {
                        gate |= 1 << fi;
                    }
                }
            }
            for a in &self.rules[ri].actions {
                let mut note = |src: &Src| {
                    if let Src::Field(0, fi) | Src::SnapField(0, fi) = src {
                        gate |= 1 << fi;
                    }
                };
                match a {
                    CompiledAction::Insert { args, .. } => args.iter().for_each(&mut note),
                    CompiledAction::Set { arg, .. } => note(arg),
                    _ => {}
                }
            }
            *self.trie[first].collect_left_gate.get_or_insert(0) |= gate;
        }
    }

    /// Structural identity of a pattern for node sharing: type, CE kind,
    /// and the ordered non-binding constraints — var references by
    /// (tuple pos, field), eq literals coerced to the field type (the
    /// D-029 alpha-node key), other literals as written.
    fn pattern_key(&self, p: &CompiledPattern) -> String {
        use std::fmt::Write as _;
        let mut s = format!("{}|{:?}|b{}", p.type_id.0, p.ce, p.bind_fields);
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
                    // the variable NAME is identity-significant when it
                    // appears in a constraint (ne_t13 vs ne_t14)
                    let name = c.rhs_var.as_deref().unwrap_or("");
                    let _ = write!(s, "{op:?}{name}@{ti}.{fi}");
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
        if let Some(acc) = &p.acc {
            let _ = write!(
                s,
                "|acc{:?}:{}:{:?}",
                acc.func,
                acc.arg_name.as_deref().unwrap_or(""),
                acc.arg_field
            );
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
        // Accumulate results carry their natural compile-time type
        // (Double/Long) EXCEPT min/max over double args, which Drools
        // types opaquely (Comparable/Number) — those results compile
        // nowhere: not in comparisons, not as RHS args (D-039).
        let mut acc_opaque: HashSet<String> = HashSet::new();
        let mut patterns = Vec::new();
        let mut tuple_len = 0usize;

        // A rule whose first pattern is a CE (or an accumulate, which is
        // a beta node needing a left input) matches on InitialFact
        // (ne_f1/acc1): inject the synthetic positive position 0.
        if def.patterns[0].ce != CeKind::Positive || def.patterns[0].acc.is_some() {
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
                bind_fields: 0,
                acc: None,
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
            let mut bind_fields = 0u64;
            for c in &p.constraints {
                match c {
                    Constraint::Bind { var, field } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        listen_mask |= 1 << fi;
                        bind_fields |= 1 << fi;
                        let ft = self.store.field_type(type_id, fi);
                        if p.acc.is_some() {
                            // the accumulate arg binding is scoped INSIDE
                            // the source (D-038); nothing registers outward
                            continue;
                        }
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
                        let (src, rhs_ft, rhs_var) = match rhs {
                            CmpRhs::Lit(l) => (Src::Lit(lit_value(l)), lit_type(l), None),
                            CmpRhs::Var(v) => {
                                if acc_opaque.contains(v) {
                                    return Err(err(format!(
                                        "{v}: min/max over double is not comparable downstream (Drools Number typing, D-039)"
                                    )));
                                }
                                let (bpi, bfi, bft) = field_binds
                                    .get(v)
                                    .copied()
                                    .ok_or_else(|| err(format!("unknown binding {v} (must be declared before use)")))?;
                                (Src::Field(bpi, bfi), bft, Some(v.clone()))
                            }
                        };
                        check_cmp_types(&rname, lhs_ft, *op, rhs_ft)?;
                        cmps.push(CompiledCmp {
                            field_idx: fi,
                            test: Test::Cmp { op: *op, rhs: src },
                            rhs_var,
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
                        cmps.push(CompiledCmp {
                            field_idx: fi,
                            test: Test::Matches(r),
                            rhs_var: None,
                        });
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
                            rhs_var: None,
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
                            rhs_var: None,
                        });
                    }
                }
            }
            let acc = match &p.acc {
                None => None,
                Some(spec) => {
                    let (arg_field, arg_ft) = match &spec.arg {
                        None => (None, FieldType::I64),
                        Some(a) => {
                            let fi = p
                                .constraints
                                .iter()
                                .find_map(|c| match c {
                                    Constraint::Bind { var, field } if var == a => {
                                        self.store.field_index(type_id, field)
                                    }
                                    _ => None,
                                })
                                .ok_or_else(|| err(format!("unknown accumulate arg {a}")))?;
                            (Some(fi), self.store.field_type(type_id, fi))
                        }
                    };
                    let numeric = matches!(arg_ft, FieldType::I64 | FieldType::F64);
                    if spec.arg.is_some() && !numeric {
                        return Err(err(format!(
                            "{:?} requires a numeric argument (subset wall)",
                            spec.func
                        )));
                    }
                    // result type per D-038 pins
                    let (result_name, result_ft) = match spec.func {
                        AccFunc::Count => (ACC_LONG, FieldType::I64),
                        AccFunc::Average => (ACC_DOUBLE, FieldType::F64),
                        AccFunc::Sum | AccFunc::Min | AccFunc::Max => match arg_ft {
                            FieldType::I64 => (ACC_LONG, FieldType::I64),
                            _ => (ACC_DOUBLE, FieldType::F64),
                        },
                        AccFunc::Collect => (ACC_COLLECTION, FieldType::I64),
                    };
                    let result_tid = self.store.type_id(result_name).unwrap();
                    let t = tpos.ok_or_else(|| err("accumulate cannot be a CE".into()))?;
                    if spec.func != AccFunc::Collect {
                        if field_binds
                            .insert(spec.result_var.clone(), (t, 0, result_ft))
                            .is_some()
                        {
                            return Err(err(format!("duplicate binding {}", spec.result_var)));
                        }
                        if matches!(spec.func, AccFunc::Min | AccFunc::Max)
                            && arg_ft == FieldType::F64
                        {
                            acc_opaque.insert(spec.result_var.clone());
                        }
                    }
                    Some(CompiledAcc {
                        func: spec.func,
                        arg_field,
                        arg_ft,
                        result_tid,
                        arg_name: spec.arg.clone(),
                    })
                }
            };
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
                bind_fields,
                acc,
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
                            &acc_opaque,
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
                        self.compile_arg(&rname, arg, &fact_binds, &field_binds, &acc_opaque, &def, &patterns)?;
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
        Ok(CompiledRule { def, patterns, actions })
    }

    fn compile_arg(
        &self,
        rname: &str,
        arg: &RhsArg,
        fact_binds: &HashMap<String, (usize, TypeId)>,
        field_binds: &HashMap<String, (usize, usize, FieldType)>,
        acc_opaque: &HashSet<String>,
        _def: &RuleDef,
        _patterns: &[CompiledPattern],
    ) -> Result<(Src, FieldType), EngineError> {
        match arg {
            RhsArg::Lit(l) => Ok((Src::Lit(lit_value(l)), lit_type(l))),
            RhsArg::Var(v) => {
                if acc_opaque.contains(v) {
                    return Err(EngineError(format!(
                        "rule {rname}: {v}: min/max over double compiles nowhere (Drools Number typing, D-039)"
                    )));
                }
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
            // Post-RHS rendering (D-013 / j03); collect results carry
            // their CURRENT element list (D-038).
            let matches: Vec<FactView> = tuple
                .iter()
                .map(|&f| {
                    let mut fv = self.store.render(f);
                    if let Some(elems) = self.collect_vals.get(&f) {
                        fv.elems =
                            Some(elems.iter().map(|&e| self.store.render(e)).collect());
                    }
                    fv
                })
                .collect();
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

    /// WM insert: stage into the SHARED network once per LIA / trie node.
    /// Link effects run after EVERY node event — Drools propagates a WM
    /// action through the alpha sinks sequentially, and an intermediate
    /// link (e.g. a not node re-linking before a later join unlinks)
    /// transiently links the path and QUEUES its items (D-037/fz_7_2122).
    fn on_insert(&mut self, f: FactId, origin: Origin) {
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for li in 0..self.lias.len() {
            let (ri, pos) = self.lias[li].env;
            if self.alpha_passes(ri, pos, f) {
                self.lias[li].active.insert(f);
                for i in 0..self.lias[li].k1_rules.len() {
                    let rb = self.lias[li].k1_rules[i];
                    self.nets[rb].s0.add_ins(f, origin);
                }
                for i in 0..self.lias[li].children.len() {
                    let c = self.lias[li].children[i];
                    self.trie[c].s0_in.add_ins(f, origin);
                }
                self.note_link_effects(&mut was);
            }
        }
        for ni in 0..self.trie.len() {
            let (ri, pos) = self.trie[ni].env;
            if self.alpha_passes(ri, pos, f) {
                self.trie[ni].active.insert(f);
                self.maybe_pulse(ni);
                self.trie[ni].node.s_right.add_ins(f, origin);
                self.note_link_effects(&mut was);
            }
        }
    }

    /// The first right insert into an UNCONSTRAINED not node force-links
    /// it for one evaluation so the blocking batch processes, after which
    /// it unlinks again (D-031, NotNode.assertObject).
    fn maybe_pulse(&mut self, ni: usize) {
        let (ri, pos) = self.trie[ni].env;
        let pat = &self.rules[ri].patterns[pos];
        if pat.ce == CeKind::Not && !pat.beta && self.trie[ni].active.len() == 1 {
            self.trie[ni].pulse = true;
        }
    }

    fn on_update(&mut self, f: FactId, mask: u64, src_ri: usize) {
        let ftype = self.store.fact_type(f);
        let origin = Some(src_ri);
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for li in 0..self.lias.len() {
            let (ri, pos) = self.lias[li].env;
            if self.rules[ri].patterns[pos].type_id != ftype {
                continue;
            }
            let was_in = self.lias[li].active.contains(&f);
            let now = self.alpha_passes(ri, pos, f);
            let listen = self.rules[ri].patterns[pos].listen_mask;
            let stage: u8 = match (was_in, now) {
                (false, true) => 1,
                (true, false) => 2,
                (true, true) if mask == u64::MAX || listen & mask != 0 => 3,
                _ => 0,
            };
            if stage == 0 {
                continue;
            }
            if stage == 1 {
                self.lias[li].active.insert(f);
            } else if stage == 2 {
                self.lias[li].active.remove(&f);
            }
            for i in 0..self.lias[li].k1_rules.len() {
                let rb = self.lias[li].k1_rules[i];
                match stage {
                    1 => self.nets[rb].s0.add_ins(f, origin),
                    2 => self.nets[rb].s0.add_del(f, origin),
                    _ => self.nets[rb].s0.add_upd(f, origin),
                }
            }
            for i in 0..self.lias[li].children.len() {
                let c = self.lias[li].children[i];
                match stage {
                    1 => self.trie[c].s0_in.add_ins(f, origin),
                    2 => self.trie[c].s0_in.add_del(f, origin),
                    _ => {
                        // LIA sink masking for level-1 COLLECT children
                        // (D-040): a pattern-0 MODIFY is dropped unless
                        // the mask intersects the downstream interest.
                        if let Some(gate) = self.trie[c].collect_left_gate {
                            if mask != u64::MAX && mask & gate == 0 {
                                continue;
                            }
                        }
                        self.trie[c].s0_in.add_upd(f, origin)
                    }
                }
            }
            self.note_link_effects(&mut was);
        }
        for ni in 0..self.trie.len() {
            let (ri, pos) = self.trie[ni].env;
            let pat = &self.rules[ri].patterns[pos];
            if pat.type_id != ftype {
                continue;
            }
            let was_in = self.trie[ni].active.contains(&f);
            let now = self.alpha_passes(ri, pos, f);
            match (was_in, now) {
                (false, true) => {
                    self.trie[ni].active.insert(f);
                    self.maybe_pulse(ni);
                    self.trie[ni].node.s_right.add_ins(f, origin);
                }
                (true, false) => {
                    self.trie[ni].active.remove(&f);
                    self.trie[ni].node.s_right.add_del(f, origin);
                }
                (true, true) => {
                    // ALL-SET mask (bare update) is class-reactive
                    // (fz_42_3311); property masks need intersection.
                    if mask == u64::MAX || pat.listen_mask & mask != 0 {
                        self.trie[ni].node.s_right.add_upd(f, origin);
                    } else {
                        // mask miss: immediate right-memory reAdd, no
                        // staging (fz_42_4359). Not nodes use the
                        // existential variant: blocked lefts re-search
                        // and unmatched ones stay DETACHED (D-031,
                        // NotNode.reorderRightTuple's null sink).
                        let env = JoinEnvImpl { store: &self.store, rule: &self.rules[ri] };
                        if pat.ce == CeKind::Not {
                            self.trie[ni].node.not_mask_miss_re_add(&env, pos - 1, f);
                        } else {
                            let key = phreak::JoinEnv::key_of_right(&env, pos - 1, f);
                            self.trie[ni].node.re_add_right_fact(f, key);
                        }
                    }
                }
                (false, false) => {}
            }
            self.note_link_effects(&mut was);
        }
    }

    fn on_delete(&mut self, f: FactId, origin: Origin) {
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for li in 0..self.lias.len() {
            if self.lias[li].active.remove(&f) {
                for i in 0..self.lias[li].k1_rules.len() {
                    let rb = self.lias[li].k1_rules[i];
                    self.nets[rb].s0.add_del(f, origin);
                }
                for i in 0..self.lias[li].children.len() {
                    let c = self.lias[li].children[i];
                    self.trie[c].s0_in.add_del(f, origin);
                }
                self.note_link_effects(&mut was);
            }
        }
        for ni in 0..self.trie.len() {
            if self.trie[ni].active.remove(&f) {
                self.trie[ni].node.s_right.add_del(f, origin);
                self.note_link_effects(&mut was);
            }
        }
    }

    /// Per-rule agenda effects after ONE node event:
    /// PathMemory.doUnlinkRule (D-031/ne_x2) — a LINKED->UNLINKED
    /// transition queues the agenda item so cancellations and unblocks
    /// evaluate in their own window; the usual dirty-while-linked
    /// (re)queue covers LINK transitions (incl. a not node re-linking on
    /// its last right's delete, NotNode.doDeleteRightTuple). Tracking is
    /// incremental per node event: an INTERMEDIATE link inside one WM
    /// action queues the item even if a later node unlinks the path
    /// again (D-037/fz_7_2122).
    fn note_link_effects(&mut self, was: &mut [bool]) {
        for ri in 0..self.rules.len() {
            let now = self.rule_linked(ri);
            if was[ri] && !now {
                self.nets[ri].queued = true;
            }
            self.refresh_linked(ri);
            was[ri] = now;
        }
    }

    /// Per-position segment-linking requirement (D-031): positive and
    /// exists positions need alpha data; a constrained not is always
    /// linked; an unconstrained not is linked while its right input is
    /// EMPTY (or transiently via the insert pulse).
    fn pos_linked(&self, ri: usize, pos: usize) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        if pos == 0 {
            return !self.lias[self.nets[ri].lia].active.is_empty();
        }
        if pat.acc.is_some() {
            // AccumulateNode canBeDisabled == false: never gates the path.
            return true;
        }
        let node = &self.trie[self.nets[ri].path[pos - 1]];
        match pat.ce {
            CeKind::Positive | CeKind::Exists => !node.active.is_empty(),
            CeKind::Not => pat.beta || node.active.is_empty() || node.pulse,
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
        let net = &self.nets[ri];
        !net.s0.is_empty()
            || !net.term_pending.is_empty()
            || net.path.iter().enumerate().any(|(step, &ni)| {
                let n = &self.trie[ni];
                !n.node.s_right.is_empty()
                    || !n.node.s_left.is_empty()
                    || (step == 0 && !n.s0_in.is_empty())
            })
    }

    fn evaluate_rule(&mut self, ri: usize, force: bool, _eager: bool) {
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
            let alive: Vec<HashSet<FactId>> = (0..k)
                .map(|pos| {
                    if pos == 0 {
                        self.lias[self.nets[ri].lia].active.clone()
                    } else {
                        self.trie[self.nets[ri].path[pos - 1]].active.clone()
                    }
                })
                .collect();
            let positives: Vec<(usize, usize)> = self.rules[ri]
                .patterns
                .iter()
                .enumerate()
                .filter_map(|(pos, p)| p.tpos.map(|t| (pos, t)))
                .collect();
            self.nets[ri]
                .queue
                .retain(|t| positives.iter().all(|(pos, ti)| alive[*pos].contains(&t[*ti])));
        }
        // Agenda-item gate: only a queued item evaluates (the just-fired
        // rule is force-evaluated, fz_42_5243).
        if !force && !self.nets[ri].queued {
            return;
        }

        let no_loop = self.rules[ri].def.no_loop;

        if k == 1 {
            // LIA -> terminal directly; working-memory staging consumed
            // OLDEST-first (pr08/pr04 pin).
            let s0 = self.nets[ri].s0.take();
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

        // Walk this rule's path through the SHARED trie (D-037). Each
        // node consumes its own staged inputs — whichever sharer's item
        // is reached first claims the batch — and every batch propagates
        // to ALL sinks: the first-built sink via addAll-append
        // (preserved, FIFO across batches), later sinks via reversed
        // peer copies (SegmentPropagator prepends, LIFO across batches).
        // This rule's continuation is just one of the sinks; the walk
        // then consumes the next node's (freshly topped-up) pending.
        for step in 0..self.nets[ri].path.len() {
            let ni = self.nets[ri].path[step];
            let (env_ri, env_pos) = self.trie[ni].env;
            let mut fresh: Staged<Tup> = Staged::default();
            if step == 0 {
                let s0 = self.trie[ni].s0_in.take();
                fresh.ins = s0.ins.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();
                fresh.upd = s0.upd.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();
                fresh.del = s0.del.into_iter().map(|(f, o, p)| (vec![f], o, p)).collect();
            }
            let pending = self.trie[ni].node.s_left.take();
            let src = Staged::merge_into_pending(pending, fresh);
            let sr = self.trie[ni].node.s_right.take();
            if src.is_empty() && sr.is_empty() {
                continue;
            }
            let mut trg: Staged<Tup> = Staged::default();
            if self.trie[ni].node.kind == phreak::Kind::Acc {
                trg = self.eval_acc_node(ni, env_ri, env_pos, src, sr);
            } else {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::do_node(&env, env_pos - 1, &mut self.trie[ni].node, src, sr, &mut trg);
            }
            // Consuming the batch spends the not-node link pulse
            // (unlinkNotNodeOnRightInsert, D-031).
            self.trie[ni].pulse = false;
            if trg.is_empty() {
                continue;
            }
            for si in 0..self.trie[ni].sinks.len() {
                let sink = self.trie[ni].sinks[si];
                let batch = trg.clone();
                match sink {
                    Sink::Node(c) => {
                        let pending = self.trie[c].node.s_left.take();
                        self.trie[c].node.s_left = if si == 0 {
                            Staged::append_into_pending(pending, batch)
                        } else {
                            Staged::peer_merge_into_pending(pending, batch)
                        };
                    }
                    Sink::Term(rb) => {
                        let pending = self.nets[rb].term_pending.take();
                        self.nets[rb].term_pending = if si == 0 {
                            Staged::append_into_pending(pending, batch)
                        } else {
                            Staged::peer_merge_into_pending(pending, batch)
                        };
                    }
                }
            }
        }

        // Terminal (PhreakRuleTerminalNode + RuleExecutor): consume THIS
        // rule's staged terminal input — deletes, then updates, then
        // inserts, head-first, appending to the executor's tuple list. A
        // queued activation keeps its position; an unqueued (fired) one
        // is effectively recreated.
        let src = self.nets[ri].term_pending.take();
        let net = &mut self.nets[ri];
        for (t, _, _) in src.del.iter() {
            net.queue.retain(|x| x != t);
        }
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

    /// PhreakAccumulateNode.doNode (D-038): leftDel, rightDel, rightUpd,
    /// leftUpd, rightIns, leftIns; touched lefts collect into a temp set
    /// and results evaluate at the END (temp inserts head-first, then
    /// updates). Deletes REVERSE the stored per-match contribution;
    /// updates are reverse(stored)+accumulate(new); min/max reinit and
    /// refold when reverse is unsupported. The single result child per
    /// left reuses its synthetic fact, updating the value in place.
    fn eval_acc_node(
        &mut self,
        ni: usize,
        env_ri: usize,
        env_pos: usize,
        src: Staged<Tup>,
        sr: Staged<FactId>,
    ) -> Staged<Tup> {
        let spec = self.rules[env_ri].patterns[env_pos].acc.clone().unwrap();
        let node_idx = env_pos - 1;
        let indexed = self.rules[env_ri].patterns[env_pos].pindex != phreak::Index::None;
        let mut trg: Staged<Tup> = Staged::default();
        let mut temp: Staged<Tup> = Staged::default();

        // Phase A: left deletes — discard the context, retract the child.
        for (l, o, _) in src.del.iter() {
            self.trie[ni].node.remove_left(l);
            if let Some(ctx) = self.trie[ni].acc.remove(l) {
                for (rf, _) in &ctx.matches {
                    if let Some(v) = self.trie[ni].acc_by_right.get_mut(rf) {
                        v.retain(|x| x != l);
                    }
                }
                if ctx.propagated {
                    if let Some(res) = ctx.result {
                        let mut child = l.clone();
                        child.push(res);
                        trg.add_del(child, *o);
                    }
                }
            }
        }

        // Phase B: right deletes — reverse each stored contribution.
        for (f, o, _) in sr.del.iter() {
            self.trie[ni].node.remove_right(*f);
            for l in self.trie[ni].acc_by_right.remove(f).unwrap_or_default() {
                self.acc_remove_match(ni, spec.func, &l, *f);
                temp.add_upd(l, *o);
            }
        }

        // Phase C: right updates — reorder (re-key, move to END), then
        // reverse(stored)+accumulate(new) per still/newly allowed left.
        for (f, _, _) in sr.upd.iter() {
            let key = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::JoinEnv::key_of_right(&env, node_idx, *f)
            };
            self.trie[ni].node.re_add_right_tuple(*f, key);
        }
        for (f, o, _) in sr.upd.iter() {
            let fkey = self.trie[ni].node.right_key_pub(*f);
            let bucket = self.trie[ni].node.lefts_bucket_pub(fkey.as_ref());
            let matched = self.trie[ni].acc_by_right.get(f).cloned().unwrap_or_default();
            if indexed && !matched.is_empty() && !bucket.contains(&matched[0]) {
                // index moved: remove all previous matches
                for l in self.trie[ni].acc_by_right.remove(f).unwrap_or_default() {
                    self.acc_remove_match(ni, spec.func, &l, *f);
                    temp.add_upd(l, *o);
                }
            }
            for l in bucket {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                    phreak::JoinEnv::allowed(&env, node_idx, &l, *f)
                };
                let had = self.trie[ni].acc_by_right.get(f).is_some_and(|v| v.contains(&l));
                if allowed {
                    temp.add_upd(l.clone(), *o);
                    if had {
                        self.acc_remove_match(ni, spec.func, &l, *f);
                    }
                    let v = self.acc_contribution(&spec, *f);
                    self.acc_add_match(ni, spec.func, &l, *f, v);
                } else if had {
                    self.acc_remove_match(ni, spec.func, &l, *f);
                    temp.add_upd(l.clone(), *o);
                }
            }
        }

        // Phase D: left updates — reorder, re-derive this left's matches.
        // Still-matching matches KEEP their stored contributions (our
        // functions take no left declarations, D-038/acc11).
        for (l, _, _) in src.upd.iter() {
            let key = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::JoinEnv::key_of_left(&env, node_idx, l)
            };
            self.trie[ni].node.re_add_left_tuple(l, key);
        }
        for (l, o, _) in src.upd.iter() {
            let lkey = self.trie[ni].node.left_key_pub(l);
            let bucket = self.trie[ni].node.rights_bucket_pub(lkey.as_ref());
            let matched: Vec<FactId> = self.trie[ni]
                .acc
                .get(l)
                .map(|c| c.matches.iter().map(|(f, _)| *f).collect())
                .unwrap_or_default();
            if indexed && !matched.is_empty() && !bucket.contains(&matched[0]) {
                // index moved: unlink all previous matches + reinit
                for rf in &matched {
                    if let Some(v) = self.trie[ni].acc_by_right.get_mut(rf) {
                        v.retain(|x| x != l);
                    }
                }
                if let Some(ctx) = self.trie[ni].acc.get_mut(l) {
                    ctx.matches.clear();
                    ctx.reset_state();
                }
            }
            for f in bucket {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                    phreak::JoinEnv::allowed(&env, node_idx, l, f)
                };
                let had = self.trie[ni]
                    .acc
                    .get(l)
                    .is_some_and(|c| c.matches.iter().any(|(rf, _)| *rf == f));
                if allowed && !had {
                    let v = self.acc_contribution(&spec, f);
                    self.acc_add_match(ni, spec.func, l, f, v);
                } else if !allowed && had {
                    self.acc_remove_match(ni, spec.func, l, f);
                }
            }
            temp.add_upd(l.clone(), *o);
        }

        // Phase E: right inserts (before left inserts).
        for (f, o, _) in sr.ins.iter() {
            let key = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::JoinEnv::key_of_right(&env, node_idx, *f)
            };
            self.trie[ni].node.push_right(*f, key.clone());
            for l in self.trie[ni].node.lefts_bucket_pub(key.as_ref()) {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                    phreak::JoinEnv::allowed(&env, node_idx, &l, *f)
                };
                if allowed {
                    let v = self.acc_contribution(&spec, *f);
                    self.acc_add_match(ni, spec.func, &l, *f, v);
                    temp.add_upd(l, *o);
                }
            }
        }

        // Phase F: left inserts — init context, fold the matching bucket.
        for (l, o, _) in src.ins.iter() {
            let lkey = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::JoinEnv::key_of_left(&env, node_idx, l)
            };
            self.trie[ni].node.push_left(l.clone(), lkey.clone());
            self.trie[ni].acc.insert(l.clone(), AccCtx::new());
            for f in self.trie[ni].node.rights_bucket_pub(lkey.as_ref()) {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                    phreak::JoinEnv::allowed(&env, node_idx, l, f)
                };
                if allowed {
                    let v = self.acc_contribution(&spec, f);
                    self.acc_add_match(ni, spec.func, l, f, v);
                }
            }
            temp.add_ins(l.clone(), *o);
        }

        // Phase G: result evaluation — temp inserts head-first, then
        // updates; null results retract, others insert/update the child.
        let ins: Vec<(Tup, Origin)> = temp.ins.iter().map(|(l, o, _)| (l.clone(), *o)).collect();
        let upd: Vec<(Tup, Origin)> = temp.upd.iter().map(|(l, o, _)| (l.clone(), *o)).collect();
        for (l, o) in ins.into_iter().chain(upd) {
            let Some(ctx) = self.trie[ni].acc.get(&l) else { continue };
            let rv = ctx.result_value(spec.func, spec.arg_ft);
            let (existing, propagated) = (ctx.result, ctx.propagated);
            match rv {
                None => {
                    if propagated {
                        let res = existing.unwrap();
                        let mut child = l.clone();
                        child.push(res);
                        trg.add_del(child, o);
                        self.trie[ni].acc.get_mut(&l).unwrap().propagated = false;
                    }
                }
                Some(v) => {
                    let res = match existing {
                        Some(r) => {
                            if spec.func != AccFunc::Collect {
                                self.store.set_value(r, 0, v).expect("acc result set");
                            }
                            r
                        }
                        None => {
                            let vals =
                                if spec.func == AccFunc::Collect { vec![] } else { vec![v] };
                            let r = self
                                .store
                                .insert(spec.result_tid, vals)
                                .expect("acc result insert");
                            self.trie[ni].acc.get_mut(&l).unwrap().result = Some(r);
                            r
                        }
                    };
                    if spec.func == AccFunc::Collect {
                        let list = self.trie[ni].acc[&l].list.clone();
                        self.collect_vals.insert(res, list);
                    }
                    let mut child = l.clone();
                    child.push(res);
                    if propagated {
                        trg.add_upd_ph(child, o, 2);
                    } else {
                        trg.add_ins_ph(child, o, 0);
                        self.trie[ni].acc.get_mut(&l).unwrap().propagated = true;
                    }
                }
            }
        }
        trg
    }

    /// accumulate(): apply the contribution and record the match.
    fn acc_add_match(&mut self, ni: usize, func: AccFunc, l: &Tup, f: FactId, v: Value) {
        let ctx = self.trie[ni].acc.get_mut(l).expect("acc ctx");
        ctx.apply(func, f, &v);
        ctx.matches.push((f, v));
        self.trie[ni].acc_by_right.entry(f).or_default().push(l.clone());
    }

    /// removeMatch(): reverse the STORED contribution; when the function
    /// cannot reverse (min/max), reinit and refold the remaining matches.
    fn acc_remove_match(&mut self, ni: usize, func: AccFunc, l: &Tup, f: FactId) {
        if let Some(v) = self.trie[ni].acc_by_right.get_mut(&f) {
            if let Some(i) = v.iter().position(|x| x == l) {
                v.remove(i);
            }
        }
        let ctx = self.trie[ni].acc.get_mut(l).expect("acc ctx");
        let Some(i) = ctx.matches.iter().position(|(rf, _)| *rf == f) else { return };
        let (_, stored) = ctx.matches.remove(i);
        if !ctx.try_reverse(func, f, &stored) {
            ctx.reset_state();
            let remaining = ctx.matches.clone();
            for (rf, vv) in &remaining {
                ctx.apply(func, *rf, vv);
            }
        }
    }

    /// The value a source fact contributes (live field read at
    /// accumulate time; reverses use the stored copy).
    fn acc_contribution(&self, spec: &CompiledAcc, f: FactId) -> Value {
        match spec.arg_field {
            Some(fi) => self.store.value(f, fi),
            None => Value::I64(0),
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
    /// InitialFact and accumulate-result facts never appear here
    /// (matches session.getObjects(): result Numbers/Collections are not
    /// working-memory objects).
    pub fn facts(&self) -> Vec<FactView> {
        let hidden: Vec<TypeId> = [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION]
            .iter()
            .filter_map(|n| self.store.type_id(n))
            .collect();
        self.store
            .live_facts()
            .filter(|f| !hidden.contains(&self.store.fact_type(*f)))
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
