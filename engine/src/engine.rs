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
/// Hidden row types for ?query CEs (D-056): one per query, fields = the
/// query's params. Rows render as QueryArgs match elements and never
/// appear in the final fact set.
pub(crate) const QROW_PREFIX: &str = "__qrow$";

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
    /// Inline boolean group (D-073): composite over possibly-multiple
    /// fields. Leaf `==` uses double promotion like `in` (ib23) and
    /// never joins eq-hash groups (ib21/ib22). `cross_var` = references
    /// an EARLIER pattern's binding: evaluated at join time only.
    /// `key` is the D-037 identity text (fields, ops, coerced-for-eq
    /// literals NOT applied — composites keep written literals — and
    /// referenced var names with positions).
    Group { g: GExpr, cross_var: bool, key: String },
}

/// Compiled inline-group expression tree (D-073).
enum GExpr {
    Cmp { field_idx: usize, op: CmpOp, rhs: Src },
    Matches { field_idx: usize, rx: crate::rx::Regex },
    Contains { field_idx: usize, needle: String },
    InList { field_idx: usize, items: Vec<Value>, negated: bool },
    And(Vec<GExpr>),
    Or(Vec<GExpr>),
    Not(Box<GExpr>),
}

/// Evaluate a compiled group against fact `f`; `l` is the left tuple
/// for cross-pattern references (None in alpha contexts, where
/// cross_var groups are skipped by the caller); `tpos` resolves
/// same-pattern references to `f`.
fn eval_gexpr(
    g: &GExpr,
    store: &FactStore,
    f: FactId,
    l: Option<&Tup>,
    tpos: Option<usize>,
) -> bool {
    match g {
        GExpr::Cmp { field_idx, op, rhs } => {
            let lhs = store.value(f, *field_idx);
            match rhs {
                Src::Lit(v) => eval_cmp(&lhs, *op, v),
                Src::Field(ti, fi) => {
                    let other = if Some(*ti) == tpos {
                        f
                    } else {
                        l.expect("cross_var group evaluated without a left tuple")[*ti]
                    };
                    eval_cmp_join(&lhs, *op, &store.value(other, *fi))
                }
                Src::SnapField(..) => unreachable!("SnapField in LHS group"),
            }
        }
        GExpr::Matches { field_idx, rx } => {
            matches!(store.value(f, *field_idx), Value::Str(s) if rx.accepts(&s))
        }
        GExpr::Contains { field_idx, needle } => match store.value(f, *field_idx) {
            Value::Str(s) => s.contains(needle.as_str()),
            _ => false,
        },
        GExpr::InList { field_idx, items, negated } => {
            let lhs = store.value(f, *field_idx);
            let hit = items.iter().any(|v| eval_cmp(&lhs, CmpOp::Eq, v));
            hit != *negated
        }
        GExpr::And(xs) => xs.iter().all(|x| eval_gexpr(x, store, f, l, tpos)),
        GExpr::Or(xs) => xs.iter().any(|x| eval_gexpr(x, store, f, l, tpos)),
        GExpr::Not(x) => !eval_gexpr(x, store, f, l, tpos),
    }
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
    /// `?query` pull CE spec (D-056): the pattern's tuple element is a
    /// synthetic row fact of the query's hidden row type.
    qce: Option<CompiledQce>,
}

/// Compiled `?query(args;)` CE (D-056).
#[derive(Clone)]
struct CompiledQce {
    /// Index into Engine.queries.
    qi: usize,
    /// Per-param-position argument sources.
    args: Vec<CeArg>,
    /// Hidden row type holding one emitted row per firing (fields = the
    /// query's params in order).
    row_tid: TypeId,
    /// Bit i set = position i is BOUND (literal or earlier binding) —
    /// renders as null in the QueryArgs match element.
    bound_mask: u64,
}

#[derive(Clone)]
enum CeArg {
    Lit(Value),
    /// (tuple position, field index) of an earlier scalar binding.
    /// Bound args make the CE node PER-RULE (D-058) — no identity beyond
    /// the private key.
    Bound { pos: usize, field: usize },
    /// Fresh output variable (name irrelevant for sharing,
    /// qx5_share_name).
    Unbound,
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
    /// D-076: TMS-justified insert.
    InsertLogical { type_id: TypeId, args: Vec<Src> },
    Set { pos: usize, field_idx: usize, arg: Src },
    Update { pos: usize },
    Delete { pos: usize },
}

struct CompiledRule {
    def: RuleDef,
    patterns: Vec<CompiledPattern>,
    actions: Vec<CompiledAction>,
    salience: EngineSalience,
    /// Transitive call closure of the rule's ?query CEs (D-058):
    /// evaluateQueriesForRule drains these BEFORE the rule's network
    /// evaluates.
    dep_queries: Vec<usize>,
}

/// Compiled rule salience (D-043). Dynamic expressions evaluate per
/// activation over the tuple; results pass through Java
/// Number.intValue(): i64 -> low 32 bits, f64 -> trunc toward zero with
/// i32 saturation, NaN -> 0.
#[derive(Clone)]
enum EngineSalience {
    Static(i32),
    Dyn { a: SalSrc, op: Option<(char, SalSrc)> },
}

#[derive(Clone, Copy)]
enum SalSrc {
    Lit(i64),
    /// (tuple index, field index, is_f64)
    Field(usize, usize, bool),
}

/// One agenda activation: the tuple, its salience (computed at CREATION
/// or fired-re-add; kept through queued restages — D-043), and a global
/// creation sequence for tie order (static rules: FIFO oldest-first via
/// position; dynamic ties: NEWEST first).
#[derive(Clone)]
struct Act {
    t: Tup,
    sal: i32,
    seq: u64,
}

use crate::phreak::{self, Origin, Staged, Tup};

/// TMS equality-key value: Value with Java-equals semantics for doubles
/// (Double.equals = bit comparison: NaN==NaN, +0.0 != -0.0 — tms_u6).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum KeyVal {
    I(i64),
    F(u64),
    S(String),
    B(bool),
}

fn key_vals(vals: &[Value]) -> Vec<KeyVal> {
    vals.iter()
        .map(|v| match v {
            Value::I64(n) => KeyVal::I(*n),
            Value::F64(x) => KeyVal::F(x.to_bits()),
            Value::Str(s) => KeyVal::S(s.clone()),
            Value::Bool(b) => KeyVal::B(*b),
        })
        .collect()
}

/// One justification: the ACTIVATION (subrule + fired tuple) whose
/// insertLogical supports the key. `seq` = global creation order (the
/// queryable graph's stable ordering; also the deterministic retract
/// order within a revalidation wave).
#[derive(Clone, Debug)]
struct Justif {
    ri: usize,
    tuple: Tup,
    seq: u64,
}

/// One TMS equality key (D-076): at most one JUSTIFIED handle with its
/// belief set, plus any number of STATED handles (identity-mode inserts
/// coexist — tms_w1/w5). `had_justified` drives the pinned Drools
/// delete quirk (dump3: once a key has hosted a justified handle,
/// deleting a stated sibling is a silent no-op).
#[derive(Default, Debug)]
struct EqKeyEntry {
    justified: Option<FactId>,
    beliefs: Vec<Justif>,
    stated: Vec<FactId>,
    had_justified: bool,
    /// Values of a PENDING logical belief (dep recorded on a
    /// stated-only key, dump-b): an RHS delete of the stated handle
    /// UNSTAGES it into a live justified fact (dump7/fz_42_2659);
    /// an external delete nets materialize-then-die = nothing (dump8).
    pending_vals: Option<Vec<Value>>,
}

/// The truth-maintenance state — kept FIRST-CLASS and queryable (D-076
/// design constraint): the justification graph IS the why-engine's
/// substrate; retraction is derived from it, not the other way around.
#[derive(Default)]
struct Tms {
    /// Value-equality keys over ALL declared fields (D-066).
    keys: HashMap<(TypeId, Vec<KeyVal>), EqKeyEntry>,
    /// Every live fact of a logical type -> its key.
    by_fact: HashMap<FactId, (TypeId, Vec<KeyVal>)>,
    /// Activation -> keys it currently supports, in support order.
    by_act: Vec<((usize, Tup), Vec<(TypeId, Vec<KeyVal>)>)>,
    /// Types that appear in any insertLogical (the mutation wall's and
    /// the bookkeeping's scope).
    logical_tids: HashSet<TypeId>,
    seq: u64,
    /// Keys the CURRENT firing's insertLogicals touched — drives the
    /// refire-supersede pass (fz_7777_112/74: Drools removes an
    /// activation's previous-firing deps that the new firing did not
    /// re-establish; dump-c's stable belief count is replace-not-keep).
    firing_keys: Vec<(TypeId, Vec<KeyVal>)>,
    /// PARKED tuples (the self-defeat quirk, t10/t11/t15): Drools
    /// leaks the dead blocker, so the tuple ignores right-side churn
    /// entirely; only LEFT-side events (tuple-fact update / death /
    /// alpha-break) unpark it. Terminal INS arrivals are skipped while
    /// parked; terminal UPD unparks and re-activates.
    parked: Vec<(usize, Tup)>,
    /// Terminal-del unmatches observed during the POST-FIRING force
    /// evaluation (D-076/t11): Drools checks salience preemption BEFORE
    /// re-evaluating the fired rule's network, so the dep removal lands
    /// when the item is next REACHED in the pop loop — the engine keeps
    /// its certified force-evaluation (window claiming, D-037) and
    /// defers only the TMS side-effect. The bool = the breaking action
    /// property-hit the tuple's LEFT side (LIA staging), which makes an
    /// eager (no-loop/dyn) justifier's entry drain at the FLUSH instead
    /// (tms_t20_b_s vs nb_ns event dumps).
    deferred: Vec<(usize, Tup, bool)>,
    /// Facts touched by the CURRENT evaluation's left-side staging,
    /// with the staging origin (eager-flush drains are OWN-origin only,
    /// min3783 vs tms_t20_b_s).
    left_touched: Vec<(FactId, Origin)>,
    /// Ambient flag: inside the post-firing force evaluation.
    defer_mode: bool,
    /// The activation currently executing its RHS (self-break laziness,
    /// fz_42_2442).
    current_act: Option<(usize, Tup)>,
}

/// Queryable justification view (D-076): what supports this fact, and
/// what stated siblings share its equality key.
#[derive(Clone, Debug)]
pub struct SupportView {
    pub rule: String,
    pub tuple: Vec<FactId>,
    pub seq: u64,
}

#[derive(Clone, Debug)]
pub struct JustificationView {
    pub fact: FactId,
    pub rendering: FactView,
    pub supports: Vec<SupportView>,
    pub stated_siblings: Vec<FactId>,
}

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
    /// k=1 rules: pos0 staging as a WINDOW QUEUE (D-047): external
    /// session actions each close a window and compose ACTION-ORDERED
    /// at the terminal (xv2/xv3), while events within one window (the
    /// initial batch, one RHS flush) stay phase-grouped and are consumed
    /// OLDEST-first (pr08/pr04). Folds apply across windows (one staged
    /// entry per fact, TupleSets semantics).
    s0: Vec<Staged<FactId>>,
    /// This rule's LIA and trie path (one node per pattern 1..k-1).
    lia: usize,
    path: Vec<usize>,
    /// Terminal staging propagated from the last path node (per-sink
    /// copy); consumed into `queue` at this rule's evaluation.
    term_pending: Staged<Tup>,
    /// PEER-sink terminals only (D-041/fz_7_5773): tuples with a live
    /// peer object at this terminal. processPeerInserts on an existing
    /// unstaged peer stages an UPDATE (hasNodeMemory(RTN) is false), so
    /// a kind-preserved re-insert must not re-activate.
    peer_live: HashSet<Tup>,
    queue: Vec<Act>,
    /// STICKY RuleAgendaItem salience (D-043/fz_27182_862): dynamic
    /// items keep their last value across empty->removed->relinked
    /// cycles; updateSalience only rewrites it when the queue top
    /// CHANGES or the item is re-added unqueued. A 0-salience arrival
    /// into an empty queue therefore keeps the stale value.
    item_sal: i32,
    /// Activation numbers per live terminal child (D-043/fz_7_6534):
    /// a fired activation RE-ADDED after a restage keeps its ORIGINAL
    /// number (the RuleTerminalNodeLeftTuple object persists), so
    /// dynamic ties order by FIRST creation. Cleared when the child
    /// dies (delete/prune); a recreated child gets a fresh number.
    act_num: HashMap<Tup, u64>,
    /// Agenda-item lifecycle: set when the rule links (or re-dirties
    /// while linked), cleared when its evaluation leaves the queue empty.
    /// An unlinked-but-queued rule still evaluates when reached
    /// (fz_42_1464); an unqueued rule accumulates staged input
    /// (fz_42_124, fz_7_145).
    queued: bool,
}

impl RuleNet {
    /// Window-aware k=1 staging (D-047): TupleSets folds span windows.
    fn s0_add_ins(&mut self, f: FactId, o: Origin) {
        if self.s0.iter().any(|w| {
            w.ins.iter().any(|(x, _, _)| *x == f) || w.upd.iter().any(|(x, _, _)| *x == f)
        }) {
            return;
        }
        self.s0.last_mut().unwrap().add_ins(f, o);
    }

    fn s0_add_upd(&mut self, f: FactId, o: Origin) {
        if self.s0.iter().any(|w| {
            w.ins.iter().any(|(x, _, _)| *x == f)
                || w.upd.iter().any(|(x, _, _)| *x == f)
                || w.del.iter().any(|(x, _, _)| *x == f)
        }) {
            return;
        }
        self.s0.last_mut().unwrap().add_upd(f, o);
    }

    fn s0_add_del(&mut self, f: FactId, o: Origin) {
        for w in self.s0.iter_mut() {
            if let Some(i) = w.ins.iter().position(|(x, _, _)| *x == f) {
                w.ins.remove(i); // never materialized: cancel
                return;
            }
        }
        for w in self.s0.iter_mut() {
            if let Some(i) = w.upd.iter().position(|(x, _, _)| *x == f) {
                w.upd.remove(i);
            }
        }
        if self.s0.iter().any(|w| w.del.iter().any(|(x, _, _)| *x == f)) {
            return;
        }
        self.s0.last_mut().unwrap().add_del(f, o);
    }

    fn s0_dirty(&self) -> bool {
        self.s0.iter().any(|w| !w.is_empty())
    }

    /// External action boundary: close the current window (D-047).
    fn s0_close_window(&mut self) {
        if !self.s0.last().unwrap().is_empty() {
            self.s0.push(Staged::default());
        }
    }

    /// Peer-copy of a batch into this TERMINAL's staging (D-041,
    /// SegmentPropagator.processPeer* for an RTN sink): per-entry
    /// prepends (batch reversal, LIFO stacking); update clashes SKIP;
    /// insert clashes move to the head; an insert whose peer object is
    /// LIVE at this terminal arrives as an UPDATE instead
    /// (updateChildLeftTupleDuringInsert with hasNodeMemory == false,
    /// fz_7_5773); deletes (incl. normalized) end the peer lifetime.
    fn peer_merge_term(&mut self, fresh: &Staged<Tup>) {
        let mut pending = self.term_pending.take();
        for (t, o, _) in fresh.del.iter().chain(fresh.norm_del.iter()) {
            self.peer_live.remove(t);
            pending.add_del(t.clone(), *o);
        }
        for (t, o, ph) in &fresh.upd {
            let staged = pending.ins.iter().any(|(x, _, _)| x == t)
                || pending.upd.iter().any(|(x, _, _)| x == t)
                || pending.del.iter().any(|(x, _, _)| x == t);
            if !staged {
                pending.upd.insert(0, (t.clone(), *o, *ph));
            }
        }
        for (t, o, ph) in &fresh.ins {
            if let Some(i) = pending.ins.iter().position(|(x, _, _)| x == t) {
                let e = pending.ins.remove(i);
                pending.ins.insert(0, e);
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| x == t) {
                pending.upd.remove(i);
                pending.upd.insert(0, (t.clone(), *o, *ph));
                continue;
            }
            if self.peer_live.contains(t) {
                pending.upd.insert(0, (t.clone(), *o, *ph));
                continue;
            }
            self.peer_live.insert(t.clone());
            pending.ins.insert(0, (t.clone(), *o, *ph));
        }
        self.term_pending = pending;
    }
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// rules[i].def.parent, copied out for borrow-friendly no-loop
    /// checks (D-070): subrules of one `or` rule share a parent.
    rule_parents: Vec<usize>,
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
    /// Global activation sequence (D-043 tie order).
    act_seq: u64,
    /// Compiled DRL queries, evaluated on demand (Phase Q0, D-050).
    queries: Vec<crate::queries::CompiledQuery>,
    /// Hidden per-query row types for ?query CEs (D-056), aligned with
    /// `queries`.
    qrow_tids: Vec<TypeId>,
    /// Persistent query-network pattern memories (D-056, qx8_statemem):
    /// drain windows accumulate across evaluations.
    query_mem: crate::queries::QueryMem,
    /// Pending query agenda items (D-058): set on every WM event for
    /// ARMED queries, cleared when the item's network evaluates (drains)
    /// at its agenda position (salience 0, decl order) or before a
    /// depending rule's evaluation.
    query_pending: Vec<bool>,
    /// A query arms when a ?query CE first pulls it (the resident dquery
    /// links its network paths; WM events then queue its item). A
    /// standalone call retracts its dquery and never arms (pre-Q2
    /// scenarios keep their one-batch drains — fz_7_546/fz_777_145).
    query_armed: Vec<bool>,
    /// Deferred evaluation error (?query CE runtime backstops surface
    /// here because evaluate_rule has no error channel).
    pending_err: Option<String>,
    /// Truth maintenance (D-076) — queryable justification graph.
    tms: Tms,
}

impl Engine {
    pub fn new(mut schemas: Vec<TypeSchema>) -> Result<Engine, EngineError> {
        let mut seen = HashSet::new();
        for s in &schemas {
            if !seen.insert(s.name.clone()) {
                return Err(EngineError(format!("duplicate type {}", s.name)));
            }
            if RESERVED_TYPES.contains(&s.name.as_str()) || s.name.starts_with(QROW_PREFIX) {
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
            rule_parents: Vec::new(),
            rule_order: Vec::new(),
            lias: Vec::new(),
            trie: Vec::new(),
            nets: Vec::new(),
            lists_built: false,
            init_fact: None,
            collect_vals: HashMap::new(),
            act_seq: 0,
            queries: Vec::new(),
            qrow_tids: Vec::new(),
            query_mem: crate::queries::QueryMem::default(),
            query_pending: Vec::new(),
            query_armed: Vec::new(),
            pending_err: None,
            tms: Tms::default(),
        })
    }

    pub fn add_rules_drl(&mut self, src: &str) -> Result<(), EngineError> {
        let file = drl::parse_file(src)?;
        // Queries compile to on-demand evaluators; they add NOTHING to the
        // rule network and cannot perturb rule semantics (q8, D-050).
        // Call indexes are unit-relative, so all queries must arrive in
        // one DRL unit.
        if !file.queries.is_empty() {
            if !self.queries.is_empty() {
                return Err(EngineError(
                    "queries must be defined in a single DRL unit".into(),
                ));
            }
            self.queries =
                crate::queries::compile_queries(&self.store, file.queries, &RESERVED_TYPES)?;
            crate::queries::validate_calls(&self.queries)?;
            self.query_pending = vec![false; self.queries.len()];
            self.query_armed = vec![false; self.queries.len()];
            // Hidden row types for ?query CEs (D-056): fields = params.
            for q in &self.queries {
                let tid = self.store.add_schema(TypeSchema {
                    name: format!("{QROW_PREFIX}{}", q.name),
                    fields: q.params_view().to_vec(),
                });
                self.qrow_tids.push(tid);
            }
        }
        // Parent ids are unit-local (D-070): offset by units already
        // added so no-loop scoping never crosses DRL units.
        let pbase = self.rules.iter().map(|r| r.def.parent + 1).max().unwrap_or(0);
        for mut def in file.rules {
            def.parent += pbase;
            let compiled = self.compile_rule(def)?;
            self.rule_parents.push(compiled.def.parent);
            self.rules.push(compiled);
        }
        // D-057: query+mutation stays walled (D-051) — a unit with ?query
        // CEs must be insert-only.
        let has_qce = self
            .rules
            .iter()
            .any(|r| r.patterns.iter().any(|p| p.qce.is_some()));
        if has_qce
            && self.rules.iter().any(|r| {
                r.actions.iter().any(|a| {
                    matches!(
                        a,
                        CompiledAction::Set { .. }
                            | CompiledAction::Update { .. }
                            | CompiledAction::Delete { .. }
                    )
                })
            })
        {
            return Err(EngineError(
                "?query CEs cannot coexist with update/modify/delete actions (D-057)".into(),
            ));
        }
        // D-076 walls. Logical types = every type any insertLogical
        // targets (unit-wide).
        let logical_tids: HashSet<TypeId> = self
            .rules
            .iter()
            .flat_map(|r| r.actions.iter())
            .filter_map(|a| match a {
                CompiledAction::InsertLogical { type_id, .. } => Some(*type_id),
                _ => None,
            })
            .collect();
        if !logical_tids.is_empty() {
            // (1) TMS retracts are WM deletes the query drain windows
            // would see — same reasoning as the D-057 mutation wall.
            if has_qce {
                return Err(EngineError(
                    "?query CEs cannot coexist with insertLogical (D-076/D-057)".into(),
                ));
            }
            // (2) Mutating a fact of a logically-inserted type is a
            // Drools RUNTIME error with murky triggers (tms_u1/tms_u4);
            // the subset walls it at compile time (Bryan's ruling).
            for r in &self.rules {
                for a in &r.actions {
                    let pos = match a {
                        CompiledAction::Set { pos, .. } => *pos,
                        CompiledAction::Update { pos } => *pos,
                        _ => continue,
                    };
                    let tid = r
                        .patterns
                        .iter()
                        .find(|p| p.tpos == Some(pos))
                        .map(|p| p.type_id);
                    if let Some(tid) = tid {
                        if logical_tids.contains(&tid) {
                            return Err(EngineError(format!(
                                "rule {}: setters/update/modify on a logically-inserted type are out of subset (D-076; Drools runtime-errors on this — tms_u1)",
                                r.def.name
                            )));
                        }
                    }
                }
            }
            self.tms.logical_tids = logical_tids;
            // (3) Facts of logical types inserted BEFORE this unit
            // compiled (multi-unit sessions) would predate key
            // bookkeeping; single-unit sessions insert facts after
            // add_rules_drl, so retrofit is unnecessary — enforce that.
            if self.store.live_facts().next().is_some() {
                return Err(EngineError(
                    "insertLogical requires rules to be added before any facts (D-076)".into(),
                ));
            }
        }
        self.rule_order = (0..self.rules.len()).collect();
        self.rule_order
            .sort_by_key(|&ri| {
                let base = match self.rules[ri].salience {
                    EngineSalience::Static(n) => n,
                    // dynamic items enter the agenda at DEFAULT salience 0
                    // until their queue tops re-sort them (D-043)
                    EngineSalience::Dyn { .. } => 0,
                };
                (-(base as i64), ri)
            });
        // Structural node identity uses the PRE-rewrite constraint values
        // (the D-029 literal rewrite itself groups by these same keys).
        let keys: Vec<Vec<String>> = self
            .rules
            .iter()
            .enumerate()
            .map(|(ri, r)| {
                r.patterns
                    .iter()
                    .enumerate()
                    .map(|(pos, p)| self.pattern_key(p, ri, pos))
                    .collect()
            })
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
                            let kind = if pat.qce.is_some() {
                                phreak::Kind::Query
                            } else if pat.acc.is_some() {
                                phreak::Kind::Acc
                            } else {
                                match pat.ce {
                                    CeKind::Positive => phreak::Kind::Join,
                                    CeKind::Not => phreak::Kind::Not,
                                    CeKind::Exists => phreak::Kind::Exists,
                                }
                            };
                            let mut s0_in: Staged<FactId> = Staged::default();
                            s0_in.slot_memory = true; // D-047/fz_7_5801
                            self.trie.push(TrieNode {
                                node: phreak::Node::new(pat.pindex, kind),
                                env: (ri, j),
                                active: HashSet::new(),
                                pulse: false,
                                s0_in,
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
                s0: vec![Staged::default()],
                lia,
                path,
                term_pending: Staged::default(),
                peer_live: HashSet::new(),
                queue: Vec::new(),
                item_sal: 0,
                act_num: HashMap::new(),
                queued: false,
            });
        }
        // LEVEL-1 COLLECT gates (D-040, corrected by the mg1..mg8
        // matrix): the mask a pattern-0 MODIFY must intersect =
        // pattern-0's CONSTRAINT fields (its listened properties;
        // bare bindings do NOT count) + the collect's own beta
        // references into pattern 0. Later patterns' and the
        // consequence's usage do NOT inherit through the collect
        // (mg8, mg2). Unioned across every rule sharing the node.
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
            for c in &self.rules[ri].patterns[0].cmps {
                gate |= 1 << c.field_idx;
            }
            for c in &self.rules[ri].patterns[1].cmps {
                if let Test::Cmp { rhs: Src::Field(0, fi), .. } = &c.test {
                    gate |= 1 << fi;
                }
            }
            *self.trie[first].collect_left_gate.get_or_insert(0) |= gate;
        }
    }

    /// Structural identity of a pattern for node sharing: type, CE kind,
    /// and the ordered non-binding constraints — var references by
    /// (tuple pos, field), eq literals coerced to the field type (the
    /// D-029 alpha-node key), other literals as written.
    /// Compile one inline-group node (D-073): resolves fields against
    /// `type_id`, bindings against `field_binds` (same-pattern refs via
    /// tpos are legal, mirroring top-level Cmp), accumulates the listen
    /// mask, cross-pattern flag and the identity text (var names are
    /// identity-significant, D-037).
    #[allow(clippy::too_many_arguments)]
    fn compile_gexpr(
        &self,
        gx: &drl::CExpr,
        type_id: TypeId,
        tname: &str,
        rname: &str,
        tpos: Option<usize>,
        field_binds: &HashMap<String, (usize, usize, FieldType)>,
        acc_opaque: &HashSet<String>,
        listen_mask: &mut u64,
        cross: &mut bool,
        key: &mut String,
    ) -> Result<GExpr, EngineError> {
        use std::fmt::Write as _;
        let err = |m: String| EngineError(format!("rule {rname}: {m}"));
        let fidx = |field: &str| -> Result<usize, EngineError> {
            self.store
                .field_index(type_id, field)
                .ok_or_else(|| err(format!("{tname} has no field {field}")))
        };
        match gx {
            drl::CExpr::Cmp { field, op, rhs } => {
                let fi = fidx(field)?;
                *listen_mask |= 1 << fi;
                let lhs_ft = self.store.field_type(type_id, fi);
                match rhs {
                    CmpRhs::Lit(l) => {
                        check_cmp_types(rname, lhs_ft, *op, lit_type(l))?;
                        let v = lit_value(l);
                        let _ = write!(key, "g{fi}{op:?}{v:?}");
                        Ok(GExpr::Cmp { field_idx: fi, op: *op, rhs: Src::Lit(v) })
                    }
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
                        check_cmp_types(rname, lhs_ft, *op, bft)?;
                        if Some(bpi) != tpos {
                            *cross = true;
                        }
                        let _ = write!(key, "g{fi}{op:?}{v}@{bpi}.{bfi}");
                        Ok(GExpr::Cmp { field_idx: fi, op: *op, rhs: Src::Field(bpi, bfi) })
                    }
                }
            }
            drl::CExpr::Matches { field, regex } => {
                let fi = fidx(field)?;
                if self.store.field_type(type_id, fi) != FieldType::Str {
                    return Err(err(format!(
                        "matches requires a String field (subset wall), {field} is not"
                    )));
                }
                *listen_mask |= 1 << fi;
                let rx = crate::rx::Regex::parse(regex).map_err(err)?;
                let _ = write!(key, "g{fi}m{}", rx.source());
                Ok(GExpr::Matches { field_idx: fi, rx })
            }
            drl::CExpr::Contains { field, needle } => {
                let fi = fidx(field)?;
                if self.store.field_type(type_id, fi) != FieldType::Str {
                    return Err(err(format!(
                        "contains requires a String field (subset wall), {field} is not"
                    )));
                }
                *listen_mask |= 1 << fi;
                let _ = write!(key, "g{fi}c{needle}");
                Ok(GExpr::Contains { field_idx: fi, needle: needle.clone() })
            }
            drl::CExpr::InList { field, items, negated } => {
                let fi = fidx(field)?;
                *listen_mask |= 1 << fi;
                let lhs_ft = self.store.field_type(type_id, fi);
                let mut vals = Vec::new();
                for l in items {
                    check_cmp_types(rname, lhs_ft, CmpOp::Eq, lit_type(l))?;
                    vals.push(lit_value(l));
                }
                let _ = write!(key, "g{fi}in{negated}{vals:?}");
                Ok(GExpr::InList { field_idx: fi, items: vals, negated: *negated })
            }
            drl::CExpr::And(xs) => {
                key.push_str("gAnd(");
                let mut out = Vec::new();
                for x in xs {
                    out.push(self.compile_gexpr(
                        x, type_id, tname, rname, tpos, field_binds, acc_opaque,
                        listen_mask, cross, key,
                    )?);
                }
                key.push(')');
                Ok(GExpr::And(out))
            }
            drl::CExpr::Or(xs) => {
                key.push_str("gOr(");
                let mut out = Vec::new();
                for x in xs {
                    out.push(self.compile_gexpr(
                        x, type_id, tname, rname, tpos, field_binds, acc_opaque,
                        listen_mask, cross, key,
                    )?);
                }
                key.push(')');
                Ok(GExpr::Or(out))
            }
            drl::CExpr::Not(x) => {
                key.push_str("gNot(");
                let g = self.compile_gexpr(
                    x, type_id, tname, rname, tpos, field_binds, acc_opaque,
                    listen_mask, cross, key,
                )?;
                key.push(')');
                Ok(GExpr::Not(Box::new(g)))
            }
        }
    }

    fn pattern_key(&self, p: &CompiledPattern, ri: usize, pos: usize) -> String {
        use std::fmt::Write as _;
        // ?query CE identity (D-056/D-058): nodes share ONLY when every
        // arg is UNBOUND — QueryElement.equals compares args templates
        // whose unbound positions hold the Variable.v SINGLETON, while
        // literal and bound-declaration args are per-rule objects
        // (min_6795 / qx9_share_bound_late: identical literal or bound
        // args pull independently at each rule's own window;
        // qx3_two_rules / qx5_share_name / qx6_share_first: all-unbound
        // templates share).
        if let Some(qce) = &p.qce {
            if qce.args.iter().all(|a| matches!(a, CeArg::Unbound)) {
                return format!("QCE|{}|U{}", qce.qi, qce.args.len());
            }
            return format!("QCE|{}|priv{ri}.{pos}", qce.qi);
        }
        let _ = (ri, pos);
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
                Test::Group { key, .. } => {
                    let _ = write!(s, "{key}");
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
                        Test::Group { cross_var, key, .. } => {
                            // composite groups are alpha-chain members
                            // (like InList) but never eq-group members
                            // (D-073/ib21-ib22); cross-var groups are
                            // beta and stay out of the alpha prefix.
                            if !cross_var {
                                prefix.push_str(&format!("{key};"));
                            }
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
        // Vars bound by ?query CEs: usable downstream except in salience
        // expressions (D-057).
        let mut qce_binds: HashSet<String> = HashSet::new();
        let mut patterns = Vec::new();
        let mut tuple_len = 0usize;

        // A rule whose first pattern is a CE (or an accumulate, which is
        // a beta node needing a left input, or a ?query CE) matches on
        // InitialFact (ne_f1/acc1/qx0_first): inject the synthetic
        // positive position 0.
        if def.patterns[0].ce != CeKind::Positive
            || def.patterns[0].acc.is_some()
            || def.patterns[0].q_args.is_some()
        {
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
                qce: None,
            });
            tuple_len = 1;
        }

        for p in def.patterns.iter() {
            if p.type_name == INITIAL_FACT {
                return Err(err(format!("type name {INITIAL_FACT} is reserved")));
            }
            if p.type_name.starts_with(QROW_PREFIX) {
                return Err(err(format!("type name {} is reserved", p.type_name)));
            }
            // ---- `?query(args;)` pull CE (D-056) ----
            if let Some(qargs) = &p.q_args {
                let qi = self
                    .queries
                    .iter()
                    .position(|q| q.name == p.type_name)
                    .ok_or_else(|| err(format!("?{}: no such query", p.type_name)))?;
                let params: Vec<(String, FieldType)> =
                    self.queries[qi].params_view().to_vec();
                if qargs.len() != params.len() {
                    return Err(err(format!(
                        "?{}: expected {} args, got {}",
                        p.type_name,
                        params.len(),
                        qargs.len()
                    )));
                }
                let t = tuple_len;
                tuple_len += 1;
                let row_tid = self.qrow_tids[qi];
                let mut args = Vec::new();
                let mut bound_mask = 0u64;
                let mut fresh_here: HashSet<String> = HashSet::new();
                for (i, (a, (pname, pt))) in qargs.iter().zip(&params).enumerate() {
                    match a {
                        drl::QArg::Lit(l) => {
                            if lit_type(l) != *pt {
                                return Err(err(format!(
                                    "?{}: literal arg for {pname} must match the param type exactly (D-057)",
                                    p.type_name
                                )));
                            }
                            bound_mask |= 1 << i;
                            args.push(CeArg::Lit(lit_value(l)));
                        }
                        drl::QArg::Var(v) => {
                            if fact_binds.contains_key(v) {
                                return Err(err(format!(
                                    "?{}: {v} is a fact binding; call args must be scalars (D-055)",
                                    p.type_name
                                )));
                            }
                            if let Some((bpi, bfi, bft)) = field_binds.get(v).copied() {
                                if fresh_here.contains(v) {
                                    // repeated FRESH var in one call: every
                                    // occurrence is UNBOUND; the var binds
                                    // its LAST position (qx4_dupvar_out)
                                    if !crate::queries::param_bound_all_branches(
                                        &self.queries,
                                        qi,
                                        i,
                                    ) {
                                        return Err(err(format!(
                                            "?{}: unbound arg for {pname}, which is not bound in every branch of the callee (D-057)",
                                            p.type_name
                                        )));
                                    }
                                    if *pt != bft {
                                        return Err(err(format!(
                                            "?{}: repeated var {v} spans differently-typed params",
                                            p.type_name
                                        )));
                                    }
                                    field_binds.insert(v.clone(), (t, i, *pt));
                                    args.push(CeArg::Unbound);
                                    continue;
                                }
                                if bft != *pt {
                                    return Err(err(format!(
                                        "?{}: arg {v} must match the type of param {pname} exactly (D-057)",
                                        p.type_name
                                    )));
                                }
                                bound_mask |= 1 << i;
                                args.push(CeArg::Bound { pos: bpi, field: bfi });
                            } else {
                                // fresh output var: binds per row; a
                                // repeated fresh var takes its LAST
                                // position downstream (qx4_dupvar_out)
                                if !crate::queries::param_bound_all_branches(&self.queries, qi, i)
                                {
                                    return Err(err(format!(
                                        "?{}: unbound arg for {pname}, which is not bound in every branch of the callee (D-057)",
                                        p.type_name
                                    )));
                                }
                                field_binds.insert(v.clone(), (t, i, *pt));
                                qce_binds.insert(v.clone());
                                fresh_here.insert(v.clone());
                                args.push(CeArg::Unbound);
                            }
                        }
                    }
                }
                patterns.push(CompiledPattern {
                    type_id: row_tid,
                    cmps: Vec::new(),
                    listen_mask: 0,
                    ce: CeKind::Positive,
                    tpos: Some(t),
                    beta: false,
                    pindex: phreak::Index::None,
                    index_ci: None,
                    bind_fields: 0,
                    acc: None,
                    qce: Some(CompiledQce { qi, args, row_tid, bound_mask }),
                });
                continue;
            }
            let type_id = self.store.type_id(&p.type_name).ok_or_else(|| {
                if self.queries.iter().any(|q| q.name == p.type_name) {
                    err(format!(
                        "{}: reactive (push) query CEs are out of subset — use ?{}(...) (D-057)",
                        p.type_name, p.type_name
                    ))
                } else {
                    err(format!("unknown type {}", p.type_name))
                }
            })?;
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
                        // D-074 normalization: Drools compiles `not in
                        // (a, b)` to an AND of `!=` constraints that
                        // SPLITS like `&&` — each conjunct is a plain
                        // alpha node sharing with written `!=` (q2/q4);
                        // `in (a, b)` compiles to an OR composite that
                        // shares with the equivalent written `||` group
                        // (q3/q5b) and never joins eq-hash groups (q6).
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
                        if *negated {
                            for v in vals {
                                cmps.push(CompiledCmp {
                                    field_idx: fi,
                                    test: Test::Cmp { op: CmpOp::Ne, rhs: Src::Lit(v) },
                                    rhs_var: None,
                                });
                            }
                        } else {
                            use std::fmt::Write as _;
                            let mut key = String::from("gOr(");
                            let leaves: Vec<GExpr> = vals
                                .iter()
                                .map(|v| {
                                    let _ = write!(key, "g{fi}Eq{v:?}");
                                    GExpr::Cmp {
                                        field_idx: fi,
                                        op: CmpOp::Eq,
                                        rhs: Src::Lit(v.clone()),
                                    }
                                })
                                .collect();
                            key.push(')');
                            cmps.push(CompiledCmp {
                                field_idx: fi,
                                test: Test::Group {
                                    g: GExpr::Or(leaves),
                                    cross_var: false,
                                    key,
                                },
                                rhs_var: None,
                            });
                        }
                    }
                    Constraint::Group(gx) => {
                        // Inline boolean group (D-073): compile the tree,
                        // collecting listened fields, cross-pattern
                        // references and the D-037 identity text.
                        let mut cross = false;
                        let mut key = String::new();
                        let g = self.compile_gexpr(
                            gx,
                            type_id,
                            &p.type_name,
                            &rname,
                            tpos,
                            &field_binds,
                            &acc_opaque,
                            &mut listen_mask,
                            &mut cross,
                            &mut key,
                        )?;
                        if cross && tpos == Some(0) {
                            return Err(err(
                                "constraint groups referencing bindings need an earlier pattern (D-073)".into(),
                            ));
                        }
                        let field_idx = first_group_field(&g);
                        cmps.push(CompiledCmp {
                            field_idx,
                            test: Test::Group { g, cross_var: cross, key },
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
                    if spec.arg.is_some() && !numeric && spec.func != AccFunc::Count {
                        // count ignores its argument; the value-bearing
                        // functions stay numeric-only (subset wall)
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
            let beta = cmps.iter().any(|c| {
                matches!(c.test, Test::Cmp { rhs: Src::Field(..), .. })
                    || matches!(c.test, Test::Group { cross_var: true, .. })
            });
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
                qce: None,
            });
        }

        let mut actions = Vec::new();
        for a in &def.actions {
            match a {
                Action::Insert { type_name, args } | Action::InsertLogical { type_name, args } => {
                    let logical = matches!(a, Action::InsertLogical { .. });
                    if type_name == INITIAL_FACT || type_name.starts_with(QROW_PREFIX) {
                        return Err(err(format!("type name {type_name} is reserved")));
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
                    if logical {
                        // D-076 walls: justifying-tuple revalidation
                        // re-runs the LHS match — acc/collect/?query
                        // conditions are not revalidatable.
                        if patterns.iter().any(|p| p.acc.is_some() || p.qce.is_some()) {
                            return Err(err(
                                "insertLogical from accumulate/collect/?query rules is out of subset (D-076)".into(),
                            ));
                        }
                        actions.push(CompiledAction::InsertLogical { type_id: tid, args: srcs });
                    } else {
                        actions.push(CompiledAction::Insert { type_id: tid, args: srcs });
                    }
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
        // Salience (D-043): static, or a per-activation expression over
        // numeric LHS bindings.
        let salience = match &def.salience {
            drl::SalienceSpec::Static(n) => EngineSalience::Static(*n as i32),
            drl::SalienceSpec::Expr { a, op } => {
                let resolve = |t: &drl::SalTerm| -> Result<SalSrc, EngineError> {
                    match t {
                        drl::SalTerm::Lit(n) => Ok(SalSrc::Lit(*n)),
                        drl::SalTerm::Var(v) => {
                            if qce_binds.contains(v) {
                                return Err(err(format!(
                                    "salience: {v} is bound by a ?query CE (out of subset, D-057)"
                                )));
                            }
                            let (ti, fi, ft) = field_binds
                                .get(v)
                                .copied()
                                .ok_or_else(|| err(format!("salience: unknown binding {v}")))?;
                            match ft {
                                FieldType::I64 => Ok(SalSrc::Field(ti, fi, false)),
                                FieldType::F64 => Ok(SalSrc::Field(ti, fi, true)),
                                _ => Err(err(format!(
                                    "salience: {v} must be numeric (subset wall)"
                                ))),
                            }
                        }
                    }
                };
                let a = resolve(a)?;
                let op = match op {
                    None => None,
                    Some((c, b)) => Some((*c, resolve(b)?)),
                };
                EngineSalience::Dyn { a, op }
            }
        };
        // D-057: ?query CEs compose with plain positive patterns only —
        // not/exists/accumulate in the same rule are unprobed.
        if patterns.iter().any(|p| p.qce.is_some())
            && patterns
                .iter()
                .any(|p| p.ce != CeKind::Positive || p.acc.is_some())
        {
            return Err(err(
                "?query CEs cannot mix with not/exists/accumulate in one rule (D-057)".into(),
            ));
        }
        let roots: Vec<usize> = patterns.iter().filter_map(|p| p.qce.as_ref().map(|q| q.qi)).collect();
        let dep_queries = if roots.is_empty() {
            Vec::new()
        } else {
            crate::queries::dependencies(&self.queries, &roots)
        };
        Ok(CompiledRule { def, patterns, actions, salience, dep_queries })
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
        let id = self.store.insert(tid, ordered).map_err(EngineError)?;
        self.tms_note_stated(id);
        // Multi-fire (D-046): before the first fire_all the initial
        // batch propagates in its prologue; afterwards each insert
        // stages immediately (session.insert semantics — agenda
        // evaluation still waits for the next fire) and closes a k=1
        // staging window (external actions compose action-ordered at
        // terminals, D-047).
        if self.lists_built {
            self.on_insert(id, None);
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
        }
        Ok(id)
    }

    /// Nth VISIBLE inserted fact (D-047): the global insertion sequence
    /// excluding synthetics (InitialFact, accumulate results) — the same
    /// sequence Drools' objectInserted listener observes, so scenario
    /// action targets mean the same fact in both engines.
    pub fn nth_inserted(&self, n: usize) -> Option<FactId> {
        let hidden: Vec<TypeId> = [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION]
            .iter()
            .filter_map(|t| self.store.type_id(t))
            .collect();
        self.store
            .all_facts_in_insertion_order()
            .filter(|f| !hidden.contains(&self.store.fact_type(*f)))
            .nth(n)
    }

    /// EXTERNAL working-memory update by handle (D-047): set the given
    /// fields and propagate with the CHANGED-FIELDS property mask (the
    /// oracle mirror is session.update(fh, obj, modifiedProperties...)).
    /// No rule origin: no-loop never suppresses external events.
    pub fn update_fact(
        &mut self,
        id: FactId,
        fields: Vec<(String, Value)>,
    ) -> Result<(), EngineError> {
        self.reject_mutation_with_qce("update")?;
        if !self.store.is_alive(id) {
            return Err(EngineError(format!("update of dead handle {}", id.0)));
        }
        let tid = self.store.fact_type(id);
        if self.tms.logical_tids.contains(&tid) {
            return Err(EngineError(
                "external update of a logically-inserted type is out of subset (D-076; Drools runtime-errors — tms_u1/u4)".into(),
            ));
        }
        let schema = self.store.schema(tid).clone();
        let mut mask = 0u64;
        for (name, v) in fields {
            let fi = self
                .store
                .field_index(tid, &name)
                .ok_or_else(|| EngineError(format!("{}: no field {name}", schema.name)))?;
            let ft = self.store.field_type(tid, fi);
            let v = coerce(v, ft)
                .ok_or_else(|| EngineError(format!("{}.{name}: type mismatch", schema.name)))?;
            self.store.set_value(id, fi, v).map_err(EngineError)?;
            mask |= 1 << fi;
        }
        if self.lists_built {
            self.on_update(id, mask, None);
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
        }
        Ok(())
    }

    /// EXTERNAL working-memory delete by handle (D-047). Routes through
    /// the TMS quirk model (D-076): on a justified key the JUSTIFIED
    /// handle dies whichever handle was named; a stated sibling of a
    /// once-justified key no-ops (dump3).
    pub fn delete_fact(&mut self, id: FactId) -> Result<(), EngineError> {
        self.reject_mutation_with_qce("delete")?;
        if !self.store.is_alive(id) {
            return Err(EngineError(format!("delete of dead handle {}", id.0)));
        }
        let Some(victim) = self.tms_route_delete(id) else {
            return Ok(()); // pinned no-op quirk
        };
        self.store.kill(victim);
        if self.lists_built {
            self.on_delete(victim, None);
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
        }
        Ok(())
    }

    /// D-057: external update/delete with ?query CEs compiled is out of
    /// subset (left churn at query nodes is unprobed).
    fn reject_mutation_with_qce(&self, what: &str) -> Result<(), EngineError> {
        if self
            .rules
            .iter()
            .any(|r| r.patterns.iter().any(|p| p.qce.is_some()))
        {
            return Err(EngineError(format!(
                "external {what} with ?query CEs is out of subset (D-057)"
            )));
        }
        Ok(())
    }

    pub fn fire_all(&mut self, limit: usize) -> Result<Vec<Firing>, EngineError> {
        // D-081: slot memory does not survive a fire boundary —
        // same-window out-and-back restores the original slot
        // (fz_7_5801), but re-entries after an intervening
        // fireAllRules place at the head like any fresh add
        // (hw_hb4/hb5, fz_min_1144).
        for ni in 0..self.trie.len() {
            self.trie[ni].s0_in.clear_slots();
        }
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
            if let Some(e) = self.pending_err.take() {
                return Err(EngineError(e));
            }
            last_fired = Some(ri);
            if firings.len() >= limit {
                return Err(EngineError(format!(
                    "fire limit {limit} reached (non-terminating?)"
                )));
            }
            // RuleExecutor.getNextTuple: static rules removeFirst (FIFO);
            // dynamic-salience rules pop the queue MAX — ties NEWEST
            // first (MatchConflictResolver, D-043).
            let idx = match self.rules[ri].salience {
                EngineSalience::Static(_) => 0,
                EngineSalience::Dyn { .. } => self.nets[ri]
                    .queue
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, a)| (a.sal, a.seq))
                    .map(|(i, _)| i)
                    .unwrap_or(0),
            };
            let tuple = self.nets[ri].queue.remove(idx).t;
            self.execute_rhs(ri, &tuple)?;
            // Mid-firing item resort (RuleExecutor.fire, D-043): after
            // the flush, a dynamic item whose queue top no longer
            // matches its salience is dequeued and re-added at the top.
            if matches!(self.rules[ri].salience, EngineSalience::Dyn { .. }) {
                if let Some(top) = self.queue_top_sal(ri) {
                    if top != self.nets[ri].item_sal {
                        self.nets[ri].item_sal = top;
                    }
                }
            }
            // Post-RHS rendering (D-013 / j03); collect results carry
            // their CURRENT element list (D-038); ?query-CE rows render
            // as the QueryArgs array (D-056: null at bound positions).
            let matches: Vec<FactView> = tuple
                .iter()
                .enumerate()
                .map(|(pos, &f)| {
                    if let Some(qv) = self.render_qargs(ri, pos, f) {
                        return qv;
                    }
                    let mut fv = self.store.render(f);
                    if let Some(elems) = self.collect_vals.get(&f) {
                        fv.elems = Some(
                            elems.iter().map(|&e| Some(self.store.render(e))).collect(),
                        );
                    }
                    fv
                })
                .collect();
            firings.push(Firing { rule: self.rules[ri].def.name.clone(), matches });
        }
        if let Some(e) = self.pending_err.take() {
            return Err(EngineError(e));
        }
        // D-081: slot memory does not survive the fire boundary —
        // same-fire-window out-and-back restores the original slot
        // (fz_7_5801: cancel + re-add within one epoch's actions), but
        // re-entries after this fire returns place at the head like any
        // fresh add (hb4/hb5, fz_min_1144: exits mid-fire or in a prior
        // epoch never slot-restore later).
        for ni in 0..self.trie.len() {
            self.trie[ni].s0_in.clear_slots();
        }
        Ok(firings)
    }

    /// D-058: WM events queue every query's agenda item; a pending item's
    /// evaluation just drains its pattern memories (one window). Marking
    /// is a safe over-approximation — a drain that appends nothing leaves
    /// the memory (and so every observable) unchanged.
    fn mark_queries_pending(&mut self) {
        for (p, armed) in self.query_pending.iter_mut().zip(&self.query_armed) {
            *p = *armed;
        }
    }

    fn drain_query_item(&mut self, qi: usize) {
        self.query_pending[qi] = false;
        crate::queries::drain_query(&self.store, &self.queries, &mut self.query_mem, qi);
    }

    /// Agenda (D-018/D-027): eager (no-loop) rules evaluate per flush with
    /// reverse-creation terminal appends; the just-fired rule re-evaluates
    /// even if self-unlinked (fz_42_5243); lazy rules evaluate on reach
    /// with creation-order terminal appends.
    fn next_activation(&mut self, last: Option<usize>) -> Option<usize> {
        if let Some(l) = last {
            self.evaluate_rule(l, true, false);
            self.tms.defer_mode = false;
            // D-076 (min608 vs t11): Drools' RuleExecutor re-evaluates
            // the fired rule's network — including the TMS dep removal —
            // unless a STRICTLY-higher-salience item waits (equal
            // salience / earlier decl does NOT preempt it). Drain the
            // deferred unmatches now unless someone strictly outranks l.
            if self.tms.deferred.iter().any(|(ri, _, _)| *ri == l) {
                let l_sal = self.item_salience(l);
                let higher = (0..self.rules.len()).any(|rj| {
                    rj != l && self.nets[rj].queued && self.item_salience(rj) > l_sal
                }) || (0..self.queries.len())
                    .any(|qi| self.query_pending[qi] && 0 > l_sal);
                if !higher {
                    while let Some(i) =
                        self.tms.deferred.iter().position(|(ri, _, _)| *ri == l)
                    {
                        let (_, tuple, _) = self.tms.deferred.remove(i);
                        self.tms_on_terminal_del(l, &tuple);
                    }
                    self.evaluate_rule(l, false, false);
                }
            }
            if self.nets[l].queue.is_empty()
                && !self.tms.deferred.iter().any(|(ri, _, _)| *ri == l)
            {
                self.nets[l].queued = false; // emptied item leaves agenda
            }
        }
        for i in 0..self.rule_order.len() {
            let ri = self.rule_order[i];
            // The eager list: no-loop rules AND dynamic-salience rules
            // (RuleImpl.setSalience -> setEager(true)) — their networks
            // evaluate per flush so item saliences are current before
            // the agenda pop (D-043/se1).
            if self.rules[ri].def.no_loop
                || matches!(self.rules[ri].salience, EngineSalience::Dyn { .. })
            {
                // D-076 flush-drain rules: NO-LOOP eager items drain
                // only own-origin left hits (t20 dumps vs min3783);
                // DYN-SALIENCE items drain UNCONDITIONALLY — their
                // flush evaluation is the D-043 salience-currency
                // machinery and Drools' dep removal rides it
                // (fz_999_3020: the justifier's foreign-origin break
                // lands before the witness pops).
                let dyn_sal = matches!(self.rules[ri].salience, EngineSalience::Dyn { .. });
                while let Some(di) = self
                    .tms
                    .deferred
                    .iter()
                    .position(|(dri, _, eok)| *dri == ri && (*eok || dyn_sal))
                {
                    // (dyn entries here only from FORCE evals; flush
                    // evals process them inline per the wrapper)
                    let (_, tuple, _) = self.tms.deferred.remove(di);
                    self.tms_on_terminal_del(ri, &tuple);
                }
                self.evaluate_rule(ri, false, true);
                // removeRuleAgendaItemWhenEmpty applies to EAGER
                // evaluations too (fz_42_8775): an emptied item leaves
                // the agenda and stops claiming shared-node windows.
                if self.nets[ri].queued && self.nets[ri].queue.is_empty() {
                    self.nets[ri].queued = false;
                }
            }
        }
        // Agenda pop (D-008/D-043): items order by (item salience DESC,
        // decl index ASC). Static items carry their constant; DYNAMIC
        // items track their queue top (0 while empty/unevaluated) and
        // re-sort after their network evaluates
        // (RuleExecutor.updateSalience / haltRuleFiring). Evaluation
        // stays lazy: only the popped item's network runs, so window
        // claiming keeps its pinned order.
        loop {
            // (salience DESC, decl_pos ASC) over rule items AND pending
            // query items (D-058: queries are agenda items at salience 0;
            // PathMemory.queueRuleAgendaItem adds them to the group).
            let mut best: Option<(i32, usize, bool, usize)> = None; // (sal, decl, is_query, idx)
            for i in 0..self.rule_order.len() {
                let ri = self.rule_order[i];
                let has_deferred = self.tms.deferred.iter().any(|(dri, _, _)| *dri == ri);
                if !self.nets[ri].queued && !has_deferred {
                    continue;
                }
                let sal = self.item_salience(ri);
                let decl = self.rules[ri].def.decl_pos;
                let better = match best {
                    None => true,
                    Some((bs, bd, _, _)) => sal > bs || (sal == bs && decl < bd),
                };
                if better {
                    best = Some((sal, decl, false, ri));
                }
            }
            for qi in 0..self.queries.len() {
                if !self.query_pending[qi] {
                    continue;
                }
                let decl = self.queries[qi].decl_pos;
                let better = match best {
                    None => true,
                    Some((bs, bd, _, _)) => 0 > bs || (0 == bs && decl < bd),
                };
                if better {
                    best = Some((0, decl, true, qi));
                }
            }
            let Some((_, _, is_query, ri)) = best else { return None };
            if is_query {
                self.drain_query_item(ri);
                continue;
            }
            // D-076: process deferred terminal-del unmatches when the
            // item is REACHED (Drools evaluateNetworkIfDirty position).
            while let Some(i) = self.tms.deferred.iter().position(|(dri, _, _)| *dri == ri) {
                let (_, tuple, _) = self.tms.deferred.remove(i);
                self.tms_on_terminal_del(ri, &tuple);
            }
            self.evaluate_rule(ri, false, false);
            if self.nets[ri].queue.is_empty() {
                self.nets[ri].queued = false; // evaluated empty: removed
                continue;
            }
            // dynamic salience may have moved this item; re-check.
            let now = self.item_salience(ri);
            let preempted = (0..self.rules.len()).any(|rj| {
                rj != ri && self.nets[rj].queued && {
                    let sj = self.item_salience(rj);
                    sj > now || (sj == now && rj < ri)
                }
            });
            if !preempted {
                return Some(ri);
            }
        }
    }

    /// WM insert: stage into the SHARED network once per LIA / trie node.
    /// Link effects run after EVERY node event — Drools propagates a WM
    /// action through the alpha sinks sequentially, and an intermediate
    /// link (e.g. a not node re-linking before a later join unlinks)
    /// transiently links the path and QUEUES its items (D-037/fz_7_2122).
    fn on_insert(&mut self, f: FactId, origin: Origin) {
        self.mark_queries_pending();
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for li in 0..self.lias.len() {
            let (ri, pos) = self.lias[li].env;
            if self.alpha_passes(ri, pos, f) {
                self.lias[li].active.insert(f);
                for i in 0..self.lias[li].k1_rules.len() {
                    let rb = self.lias[li].k1_rules[i];
                    self.nets[rb].s0_add_ins(f, origin);
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

    fn on_update(&mut self, f: FactId, mask: u64, origin: Origin) {
        self.mark_queries_pending();
        let ftype = self.store.fact_type(f);
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
                    1 => self.nets[rb].s0_add_ins(f, origin),
                    2 => self.nets[rb].s0_add_del(f, origin),
                    _ => self.nets[rb].s0_add_upd(f, origin),
                }
            }
            // Shared-LIA modify gate (D-072/fz_999_7082): the stage-vs-drop
            // decision for a pattern-0 MODIFY is made ONCE against the
            // FIRST-BUILT trie child's effective left mask — a collect
            // child contributes its D-040 gate (bindings do NOT count),
            // a join child the full listen mask — and the decision
            // applies to EVERY trie child (m7082_vis_jf: join-first
            // stages the collect sibling; m7082_vis_cf2: collect-first
            // drops the join sibling). k=1 rules gate independently on
            // the canonical listen mask (m7082_r3k1).
            let child_stage = if stage == 3 && mask != u64::MAX {
                let eff = self.lias[li]
                    .children
                    .first()
                    .and_then(|&c| self.trie[c].collect_left_gate)
                    .unwrap_or(listen);
                if eff & mask != 0 { 3 } else { 0 }
            } else {
                stage
            };
            for i in 0..self.lias[li].children.len() {
                let c = self.lias[li].children[i];
                match child_stage {
                    0 => {}
                    1 => self.trie[c].s0_in.add_ins(f, origin),
                    2 => self.trie[c].s0_in.add_del(f, origin),
                    _ => self.trie[c].s0_in.add_upd(f, origin),
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
        self.tms_eager_break(f);
    }

    fn on_delete(&mut self, f: FactId, origin: Origin) {
        self.mark_queries_pending();
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for li in 0..self.lias.len() {
            if self.lias[li].active.remove(&f) {
                for i in 0..self.lias[li].k1_rules.len() {
                    let rb = self.lias[li].k1_rules[i];
                    self.nets[rb].s0_add_del(f, origin);
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
        self.tms_eager_break(f);
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
        if pat.qce.is_some() {
            // QueryElementNode has no right input: never gates the path
            // (a rule with an empty-rowed ?query CE still evaluates and
            // simply fires nothing — qx6_empty).
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
        net.s0_dirty()
            || !net.term_pending.is_empty()
            || net.path.iter().enumerate().any(|(step, &ni)| {
                let n = &self.trie[ni];
                !n.node.s_right.is_empty()
                    || !n.node.s_left.is_empty()
                    || (step == 0 && !n.s0_in.is_empty())
            })
    }

    /// D-076: TMS terminal-del side-effects defer out of BOTH the
    /// post-firing force evaluation (t11/min608) and the eager-flush
    /// evaluation (min3783) — they drain at the flush only for
    /// own-origin left hits (tms_t20_b_s), else at the item's pop.
    /// The flag is scoped HERE, around the whole body: any early exit
    /// of the inner fn (e.g. the unlinked-rule path) leaking it turns
    /// the drain loops into non-firing infinite spins that no
    /// fire-limit can catch (the seed-42 and seed-123 gate hangs).
    fn evaluate_rule(&mut self, ri: usize, force: bool, eager: bool) {
        // DYN-SALIENCE flush evaluations process TMS dels INLINE — the
        // flush IS Drools' salience-currency evaluation and dep removal
        // rides it (fz_999_3020); no-loop flushes defer (min3783/t20).
        let dyn_sal = matches!(self.rules[ri].salience, EngineSalience::Dyn { .. });
        self.tms.defer_mode = force || (eager && !dyn_sal);
        self.evaluate_rule_inner(ri, force, eager);
        self.tms.defer_mode = false;
    }

    fn evaluate_rule_inner(&mut self, ri: usize, force: bool, eager: bool) {
        let _ = eager;
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
            // ?query-CE positions hold synthetic row facts that never
            // retract (pull semantics, D-056) — excluded from pruning.
            let positives: Vec<(usize, usize)> = self.rules[ri]
                .patterns
                .iter()
                .enumerate()
                .filter(|(_, p)| p.qce.is_none())
                .filter_map(|(pos, p)| p.tpos.map(|t| (pos, t)))
                .collect();
            let pre = self.queue_top_sal(ri).unwrap_or(0);
            let n0 = self.nets[ri].queue.len();
            self.nets[ri]
                .queue
                .retain(|a| positives.iter().all(|(pos, ti)| alive[*pos].contains(&a.t[*ti])));
            if self.nets[ri].queue.len() != n0 {
                self.update_item_salience(ri, pre);
            }
            self.nets[ri]
                .act_num
                .retain(|t, _| positives.iter().all(|(pos, ti)| alive[*pos].contains(&t[*ti])));
        }
        // Agenda-item gate: only a queued item evaluates (the just-fired
        // rule is force-evaluated, fz_42_5243).
        if !force && !self.nets[ri].queued {
            return;
        }

        // evaluateQueriesForRule (D-058): a CE-bearing rule's evaluation
        // first evaluates its depending queries' PENDING networks — their
        // pattern memories drain one window each.
        if !self.rules[ri].dep_queries.is_empty() {
            let deps = self.rules[ri].dep_queries.clone();
            for qi in deps {
                if self.query_pending[qi] {
                    self.drain_query_item(qi);
                }
            }
        }

        // no-loop blocks per PARENT rule (D-070/or_a20): an update from
        // any subrule's RHS suppresses re-activation of every sibling
        // branch — Drools compares the shared Rule object.
        let no_loop = self.rules[ri].def.no_loop;
        let parent = self.rules[ri].def.parent;

        if k == 1 {
            // LIA -> terminal directly. WINDOWS compose in order (D-047:
            // external actions are one window each); within a window
            // phases apply and staging is consumed OLDEST-first
            // (pr08/pr04 pin).
            let windows = std::mem::replace(&mut self.nets[ri].s0, vec![Staged::default()]);
            self.tms.left_touched = windows
                .iter()
                .flat_map(|w| w.upd.iter().chain(w.del.iter()))
                .map(|(f, o, _)| (*f, *o))
                .collect();
            for s0 in windows {
                for (f, _, _) in s0.del.iter().rev() {
                    self.nets[ri].act_num.retain(|t, _| t[0] != *f);
                    let pre = self.queue_top_sal(ri).unwrap_or(0);
                    let n0 = self.nets[ri].queue.len();
                    self.nets[ri].queue.retain(|a| a.t[0] != *f);
                    if self.nets[ri].queue.len() != n0 {
                        self.update_item_salience(ri, pre);
                    }
                    self.tms_on_terminal_del(ri, &vec![*f]);
                    self.tms_parked_del(ri, &vec![*f]);
                }
                for (f, o, _) in s0.upd.iter().rev() {
                    let queued = self.nets[ri].queue.iter().any(|a| a.t[0] == *f);
                    if queued {
                        continue; // pending: keep position AND salience (se3)
                    }
                    if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                        continue; // own update does not re-activate (j04)
                    }
                    self.tms_unpark_upd(ri, &vec![*f]);
                    self.push_activation(ri, vec![*f]);
                }
                for (f, o, _) in s0.ins.iter().rev() {
                    if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                        continue;
                    }
                    if self.tms_parked_ins(ri, &vec![*f]) {
                        continue;
                    }
                    self.push_activation(ri, vec![*f]);
                }
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
                self.tms.left_touched = s0
                    .upd
                    .iter()
                    .chain(s0.del.iter())
                    .map(|(f, o, _)| (*f, *o))
                    .collect();
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
            // Cross-window child clashes resolve against the FIRST
            // sink's pending at touch time (D-041) — take it out for
            // the node evaluation, then blind-append the batch (addAll).
            let first_sink = self.trie[ni].sinks.first().copied();
            let mut first_pending = match first_sink {
                Some(Sink::Node(c)) => self.trie[c].node.s_left.take(),
                Some(Sink::Term(rb)) => self.nets[rb].term_pending.take(),
                None => Staged::default(),
            };
            let mut trg: Staged<Tup> = Staged::default();
            if self.trie[ni].node.kind == phreak::Kind::Query {
                let sink_count = self.trie[ni].sinks.len();
                match self.eval_query_ce_node(env_ri, env_pos, src, sink_count) {
                    Ok(t) => trg = t,
                    Err(e) => {
                        self.pending_err = Some(e.0);
                        return;
                    }
                }
            } else if self.trie[ni].node.kind == phreak::Kind::Acc {
                trg = self.eval_acc_node(ni, env_ri, env_pos, src, sr, &mut first_pending);
            } else {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                phreak::do_node(
                    &env,
                    env_pos - 1,
                    &mut self.trie[ni].node,
                    src,
                    sr,
                    &mut trg,
                    &mut first_pending,
                );
            }
            // Consuming the batch spends the not-node link pulse
            // (unlinkNotNodeOnRightInsert, D-031).
            self.trie[ni].pulse = false;
            for si in 0..self.trie[ni].sinks.len() {
                let sink = self.trie[ni].sinks[si];
                match sink {
                    Sink::Node(c) => {
                        if si == 0 {
                            self.trie[c].node.s_left =
                                Staged::append_into_pending(first_pending.take(), trg.clone());
                        } else if !trg.is_empty() {
                            self.trie[c].node.peer_merge_left(&trg);
                        }
                    }
                    Sink::Term(rb) => {
                        if si == 0 {
                            self.nets[rb].term_pending =
                                Staged::append_into_pending(first_pending.take(), trg.clone());
                        } else if !trg.is_empty() {
                            self.nets[rb].peer_merge_term(&trg);
                        }
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
        if std::env::var("SEINE_TRACE").is_ok() && !src.is_empty() {
            eprintln!("term[{ri}] consume ins={:?} upd={:?} del={:?}", src.ins, src.upd, src.del);
        }
        for (t, _, _) in src.del.iter() {
            self.nets[ri].act_num.remove(t);
            let pre = self.queue_top_sal(ri).unwrap_or(0);
            let n0 = self.nets[ri].queue.len();
            self.nets[ri].queue.retain(|a| a.t != *t);
            if self.nets[ri].queue.len() != n0 {
                self.update_item_salience(ri, pre);
            }
            self.tms_on_terminal_del(ri, t);
            self.tms_parked_del(ri, t);
        }
        for (t, o, _) in src.upd.iter() {
            if self.nets[ri].queue.iter().any(|a| a.t == *t) {
                continue; // queued: keep position AND salience (se3)
            }
            if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                continue;
            }
            self.tms_unpark_upd(ri, t);
            // fired activation re-added: salience RE-EVALUATED (D-043)
            self.push_activation(ri, t.clone());
        }
        for (t, o, _) in src.ins.iter() {
            if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                continue;
            }
            if self.tms_parked_ins(ri, t) {
                continue;
            }
            self.push_activation(ri, t.clone());
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
        first_pending: &mut Staged<Tup>,
    ) -> Staged<Tup> {
        let spec = self.rules[env_ri].patterns[env_pos].acc.clone().unwrap();
        let node_idx = env_pos - 1;
        let indexed = self.rules[env_ri].patterns[env_pos].pindex != phreak::Index::None;
        let mut trg: Staged<Tup> = Staged::default();
        let mut temp: Staged<Tup> = Staged::default();
        // Rights-phase temp staging is GATED on the left NOT being
        // staged in the incoming left sets (getStagedType()==NONE in
        // doRight*: "will get processed via left iteration") — a left
        // touched on both sides enters temp in the LEFT phase, i.e.
        // LAST (fz_7_5893).
        let sl_staged = |l: &Tup, sl: &Staged<Tup>| {
            sl.upd.iter().any(|(x, _, _)| x == l) || sl.ins.iter().any(|(x, _, _)| x == l)
        };

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
                        if first_pending.remove_ins(&child) {
                            trg.norm_del.push((child, *o, 0));
                        } else {
                            first_pending.remove_upd(&child);
                            trg.add_del(child, *o);
                        }
                    }
                }
            }
        }

        // Phase B: right deletes — reverse each stored contribution.
        // Visit order = match arrival order (fz_123_449 originally
        // looked like a chain-direction issue; the real mechanism was
        // the staged-left gate below — 25 round-2 regressions pinned
        // arrival order, scenarios/regressions/fz_{42,7,123,777,999}_*).
        for (f, o, _) in sr.del.iter() {
            self.trie[ni].node.remove_right(*f);
            for l in self.trie[ni].acc_by_right.remove(f).unwrap_or_default() {
                self.acc_remove_match(ni, spec.func, &l, *f);
                if !sl_staged(&l, &src) {
                    temp.add_upd(l, *o);
                }
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
                // index moved: remove all previous matches (arrival order)
                for l in self.trie[ni].acc_by_right.remove(f).unwrap_or_default() {
                    self.acc_remove_match(ni, spec.func, &l, *f);
                    if !sl_staged(&l, &src) {
                        temp.add_upd(l, *o);
                    }
                }
            }
            for l in bucket {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri] };
                    phreak::JoinEnv::allowed(&env, node_idx, &l, *f)
                };
                let had = self.trie[ni].acc_by_right.get(f).is_some_and(|v| v.contains(&l));
                if allowed {
                    if !sl_staged(&l, &src) {
                        temp.add_upd(l.clone(), *o);
                    }
                    if had {
                        self.acc_remove_match(ni, spec.func, &l, *f);
                    }
                    let v = self.acc_contribution(&spec, *f);
                    self.acc_add_match(ni, spec.func, &l, *f, v);
                } else if had {
                    self.acc_remove_match(ni, spec.func, &l, *f);
                    if !sl_staged(&l, &src) {
                        temp.add_upd(l.clone(), *o);
                    }
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
                    if !sl_staged(&l, &src) {
                        temp.add_upd(l, *o);
                    }
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
                        // propagateDelete: normalize against the first
                        // sink's pending, then stage the retract (D-041);
                        // a cancelled pending insert still reaches peers.
                        if first_pending.remove_ins(&child) {
                            trg.norm_del.push((child, o, 0));
                        } else {
                            first_pending.remove_upd(&child);
                            trg.add_del(child, o);
                        }
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
                        // propagateResult: normalizeStagedTuples against
                        // the first sink's pending, THEN addUpdate — a
                        // pending insert re-stages as an UPDATE here,
                        // unlike updateChildLeftTuple (D-041).
                        if first_pending.remove_ins(&child) {
                            trg.add_ins_ph(child, o, 0);
                        } else {
                            first_pending.remove_upd(&child);
                            trg.add_upd_ph(child, o, 2);
                        }
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
    /// Evaluate a ?query CE node (D-056): each staged left pulls the
    /// query against the CURRENT WM through the Q1 stack machine (one
    /// batched run — dquery envs prepend into the callee pools in src
    /// order, so evaluation interleaves exactly like Drools'). One child
    /// tuple per result row, carrying a synthetic row fact. The site
    /// staging (rows prepended at arrival) drains ORDER-PRESERVED to a
    /// single sink (TupleSetsImpl.addTo = addAll); a SHARED node
    /// re-reverses first (QueryTupleSets.addTo re-addInserts) so the
    /// D-037 propagation gives the first-built sink arrival order and
    /// later sinks the flipped copies (qx3_two_rules/qx5_three_rules).
    fn eval_query_ce_node(
        &mut self,
        env_ri: usize,
        env_pos: usize,
        src: Staged<Tup>,
        sink_count: usize,
    ) -> Result<Staged<Tup>, EngineError> {
        if !src.upd.is_empty() || !src.del.is_empty() || !src.norm_del.is_empty() {
            return Err(EngineError(
                "?query CE under left update/delete is out of subset (D-057)".into(),
            ));
        }
        let qce = self.rules[env_ri].patterns[env_pos]
            .qce
            .clone()
            .expect("query node pattern has a qce spec");
        // src head→tail = real staged order (full LIFO across windows,
        // qx6_windows); bound args read the left tuple.
        let calls: Vec<Vec<Option<Value>>> = src
            .ins
            .iter()
            .map(|(t, _, _)| {
                qce.args
                    .iter()
                    .map(|a| match a {
                        CeArg::Lit(v) => Some(v.clone()),
                        CeArg::Bound { pos, field } => {
                            Some(self.store.value(t[*pos], *field))
                        }
                        CeArg::Unbound => None,
                    })
                    .collect()
            })
            .collect();
        // Arming (D-058): the pull leaves resident dqueries in the
        // callee networks (transitively) — from now on WM events queue
        // their agenda items.
        for qi in crate::queries::dependencies(&self.queries, &[qce.qi]) {
            self.query_armed[qi] = true;
        }
        let staged = crate::queries::run_site(
            &self.store,
            &self.queries,
            &mut self.query_mem,
            qce.qi,
            &calls,
        )?;
        let mut children = Vec::with_capacity(staged.len());
        for (call_idx, values) in staged {
            let fid = self.store.insert(qce.row_tid, values).map_err(EngineError)?;
            let (left, o, ph) = &src.ins[call_idx];
            let mut child = left.clone();
            child.push(fid);
            children.push((child, *o, *ph));
        }
        if sink_count > 1 {
            children.reverse(); // QueryTupleSets.addTo re-reversal (D-056)
        }
        let mut trg: Staged<Tup> = Staged::default();
        trg.ins = children;
        Ok(trg)
    }

    /// Render a ?query-CE tuple element as its QueryArgs array (D-056):
    /// null at bound positions, the row's value at unbound positions.
    fn render_qargs(&self, ri: usize, pos: usize, f: FactId) -> Option<FactView> {
        let pat = self.rules[ri]
            .patterns
            .iter()
            .find(|p| p.tpos == Some(pos))?;
        let qce = pat.qce.as_ref()?;
        let elems = (0..qce.args.len())
            .map(|i| {
                if qce.bound_mask >> i & 1 == 1 {
                    None
                } else {
                    Some(scalar_view(self.store.value(f, i)))
                }
            })
            .collect();
        Some(FactView {
            type_name: "QueryArgs".into(),
            fields: Vec::new(),
            handle: u32::MAX,
            elems: Some(elems),
        })
    }

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
        // @key-all fallout (D-078/fz_42_2019): Drools' collect reverse is
        // Collection.remove(Object) = remove the FIRST equals() element
        // (value equality under @key-all declares), not the identical
        // instance. Pick the list victim by value before the removal.
        let list_victim = if func == AccFunc::Collect {
            let tidf = self.store.fact_type(f);
            let nfld = self.store.schema(tidf).fields.len();
            let fvals = key_vals(&(0..nfld).map(|i| self.store.value(f, i)).collect::<Vec<_>>());
            self.trie[ni].acc[l]
                .list
                .iter()
                .copied()
                .find(|x| {
                    *x == f || {
                        self.store.fact_type(*x) == tidf
                            && key_vals(
                                &(0..nfld).map(|i| self.store.value(*x, i)).collect::<Vec<_>>(),
                            ) == fvals
                    }
                })
                .unwrap_or(f)
        } else {
            f
        };
        let ctx = self.trie[ni].acc.get_mut(l).expect("acc ctx");
        let Some(i) = ctx.matches.iter().position(|(rf, _)| *rf == f) else { return };
        let (_, stored) = ctx.matches.remove(i);
        if !ctx.try_reverse(func, list_victim, &stored) {
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

    /// Enqueue an activation with salience computed NOW (activation
    /// creation / fired-re-add; queued restages never reach here — se3).
    fn push_activation(&mut self, ri: usize, t: Tup) {
        let sal = self.eval_salience(ri, &t);
        let seq = match self.nets[ri].act_num.get(&t) {
            Some(&n) => n, // re-added fired activation: original number
            None => {
                self.act_seq += 1;
                self.nets[ri].act_num.insert(t.clone(), self.act_seq);
                self.act_seq
            }
        };
        let pre = self.queue_top_sal(ri).unwrap_or(0);
        self.nets[ri].queue.push(Act { t, sal, seq });
        self.update_item_salience(ri, pre);
    }

    /// Per-activation salience (D-043): i64 math WRAPS to the low 32
    /// bits (Number.intValue of a Long); any f64 operand switches to
    /// double math with trunc-toward-zero, i32 saturation, NaN -> 0.
    fn eval_salience(&self, ri: usize, t: &Tup) -> i32 {
        let (a, op) = match &self.rules[ri].salience {
            EngineSalience::Static(n) => return *n,
            EngineSalience::Dyn { a, op } => (a, op),
        };
        let read = |src: &SalSrc| -> (f64, i64, bool) {
            match src {
                SalSrc::Lit(n) => (*n as f64, *n, false),
                SalSrc::Field(ti, fi, is_f) => match self.store.value(t[*ti], *fi) {
                    Value::I64(n) => (n as f64, n, *is_f),
                    Value::F64(x) => (x, x as i64, true),
                    _ => (0.0, 0, *is_f),
                },
            }
        };
        let (af, ai, a_is_f) = read(a);
        let (result_f, result_i, any_f) = match op {
            None => (af, ai, a_is_f),
            Some((c, b)) => {
                let (bf, bi, b_is_f) = read(b);
                let rf = match c {
                    '+' => af + bf,
                    '-' => af - bf,
                    _ => af * bf,
                };
                let ri64 = match c {
                    '+' => ai.wrapping_add(bi),
                    '-' => ai.wrapping_sub(bi),
                    _ => ai.wrapping_mul(bi),
                };
                (rf, ri64, a_is_f || b_is_f)
            }
        };
        if any_f {
            if result_f.is_nan() {
                0
            } else {
                result_f.trunc().clamp(i32::MIN as f64, i32::MAX as f64) as i32
            }
        } else {
            result_i as i32 // low 32 bits (se14)
        }
    }

    /// A rule item's CURRENT agenda salience (D-043): static rules use
    /// their constant; dynamic rules use the STICKY item value (see
    /// RuleNet::item_sal — it may lag the queue top by design).
    fn item_salience(&self, ri: usize) -> i32 {
        match self.rules[ri].salience {
            EngineSalience::Static(n) => n,
            EngineSalience::Dyn { .. } => self.nets[ri].item_sal,
        }
    }

    /// Queue top by the dynamic pop order (salience DESC, seq DESC).
    fn queue_top_sal(&self, ri: usize) -> Option<i32> {
        self.nets[ri].queue.iter().map(|a| (a.sal, a.seq)).max().map(|(s, _)| s)
    }

    /// RuleExecutor.updateSalience (D-043): called after an activation
    /// add/remove with the PRE-change top. If the top changed, the item
    /// is dequeued; an unqueued item is re-added at the new top (0 when
    /// empty). A relinked item that skips this keeps its stale value.
    fn update_item_salience(&mut self, ri: usize, pre_top: i32) {
        if !matches!(self.rules[ri].salience, EngineSalience::Dyn { .. }) {
            return;
        }
        let new_top = self.queue_top_sal(ri).unwrap_or(0);
        if pre_top != new_top {
            self.nets[ri].queued = false; // ruleAgendaItem.remove()
        }
        if !self.nets[ri].queued {
            self.nets[ri].item_sal = new_top;
            self.nets[ri].queued = true;
        }
    }

    fn alpha_passes(&self, ri: usize, pos: usize, f: FactId) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        if !self.store.is_alive(f) || self.store.fact_type(f) != pat.type_id {
            return false;
        }
        pat.cmps.iter().all(|c| {
            if let Test::Group { g, cross_var, .. } = &c.test {
                // cross-pattern groups evaluate at join time (D-073);
                // same-pattern/literal groups are alpha tests.
                return *cross_var || eval_gexpr(g, &self.store, f, None, pat.tpos);
            }
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
        // D-076 refire-supersede prologue: snapshot this activation's
        // prior support keys; deps not re-established by THIS firing are
        // removed in the epilogue (fz_7777_112/74, dump-c).
        let tms_act = (ri, tuple.to_vec());
        let tms_prev: Vec<(TypeId, Vec<KeyVal>)> = self
            .tms
            .by_act
            .iter()
            .find(|(a, _)| *a == tms_act)
            .map(|(_, ks)| ks.clone())
            .unwrap_or_default();
        self.tms.firing_keys.clear();
        self.tms.current_act = Some(tms_act.clone());
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
                    self.tms_note_stated(fid);
                    self.on_insert(fid, Some(ri));
                }
                CompiledAction::InsertLogical { type_id, args } => {
                    let tid = *type_id;
                    let values: Vec<Value> = {
                        let schema = self.store.schema(tid).clone();
                        args.clone()
                            .iter()
                            .zip(schema.fields.iter())
                            .map(|(a, (_, ft))| {
                                coerce(self.eval_src(a, tuple, &snapshot), *ft).ok_or_else(|| {
                                    EngineError("RHS insertLogical: arg type mismatch".into())
                                })
                            })
                            .collect::<Result<_, _>>()?
                    };
                    self.tms_insert_logical(ri, &tuple.to_vec(), tid, values)?;
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
                    self.on_update(f, mask, Some(ri));
                }
                CompiledAction::Delete { pos } => {
                    // D-076: delete routes through the TMS quirk model —
                    // a justified-key delete kills the JUSTIFIED handle
                    // whichever handle was named; a stated sibling of a
                    // once-justified key is a silent no-op (dump3); a
                    // stated handle with a pending logical belief
                    // UNSTAGES it (dump7).
                    let (victim, materialize) = self.tms_route_delete_ex(tuple[*pos], true);
                    if let Some(victim) = victim {
                        self.store.kill(victim);
                        self.on_delete(victim, Some(ri));
                    }
                    if let Some((tid, vals)) = materialize {
                        self.tms_materialize(tid, vals)?;
                    }
                }
            }
        }
        // D-076 refire-supersede epilogue: previous-firing deps this
        // firing did not re-establish are removed; emptied belief sets
        // retract their justified facts (nested actions + fixpoint).
        let stale: Vec<(TypeId, Vec<KeyVal>)> = tms_prev
            .into_iter()
            .filter(|k| !self.tms.firing_keys.contains(k))
            .collect();
        if !stale.is_empty() {
            let mut to_retract: Vec<FactId> = Vec::new();
            if let Some(i) = self.tms.by_act.iter().position(|(a, _)| *a == tms_act) {
                let (_, keys) = &mut self.tms.by_act[i];
                keys.retain(|k| !stale.contains(k));
                if keys.is_empty() {
                    self.tms.by_act.remove(i);
                }
            }
            for key in &stale {
                if let Some(e) = self.tms.keys.get_mut(key) {
                    e.beliefs.retain(|j| !(j.ri == tms_act.0 && j.tuple == tms_act.1));
                    if e.beliefs.is_empty() {
                        if let Some(jf) = e.justified.take() {
                            self.tms.by_fact.remove(&jf);
                            to_retract.push(jf);
                        }
                    }
                }
            }
            for jf in to_retract {
                if self.store.is_alive(jf) {
                    self.store.kill(jf);
                    self.on_delete(jf, None);
                }
            }
        }
        self.tms.firing_keys.clear();
        self.tms.current_act = None;
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
    // ------------------------------------------------------------------
    // Truth maintenance (D-076). The justification graph is FIRST-CLASS
    // and queryable — the why-engine's substrate. Retraction is derived
    // from the graph (belief set empties -> justified handle retracts).
    // ------------------------------------------------------------------

    fn tms_key_of(&self, tid: TypeId, f: FactId) -> (TypeId, Vec<KeyVal>) {
        let n = self.store.schema(tid).fields.len();
        let vals: Vec<Value> = (0..n).map(|i| self.store.value(f, i)).collect();
        (tid, key_vals(&vals))
    }

    /// STATED insert bookkeeping for logical types (identity mode:
    /// stated equals coexist — tms_w1/w5; the key just tracks them).
    fn tms_note_stated(&mut self, f: FactId) {
        let tid = self.store.fact_type(f);
        if !self.tms.logical_tids.contains(&tid) {
            return;
        }
        let key = self.tms_key_of(tid, f);
        self.tms.keys.entry(key.clone()).or_default().stated.push(f);
        self.tms.by_fact.insert(f, key);
    }

    /// insertLogical (D-076): merge onto the key's justified handle,
    /// no-op-with-dep onto stated-only keys (dump-b), else create the
    /// justified fact. Same-activation deps are idempotent (dump-c).
    fn tms_insert_logical(
        &mut self,
        ri: usize,
        tuple: &Tup,
        tid: TypeId,
        values: Vec<Value>,
    ) -> Result<(), EngineError> {
        let key = (tid, key_vals(&values));
        let act = (ri, tuple.clone());
        let seq = self.tms.seq;
        self.tms.seq += 1;
        let entry = self.tms.keys.entry(key.clone()).or_default();
        let need_insert = entry.justified.is_none() && entry.stated.is_empty();
        if entry.justified.is_some() || !entry.stated.is_empty() {
            if !entry.beliefs.iter().any(|j| j.ri == ri && j.tuple == *tuple) {
                entry.beliefs.push(Justif { ri, tuple: tuple.clone(), seq });
            }
            if entry.justified.is_none() && entry.pending_vals.is_none() {
                entry.pending_vals = Some(values.clone());
            }
        }
        let mut inserted = None;
        if need_insert {
            let f = self.store.insert(tid, values).map_err(EngineError)?;
            let e = self.tms.keys.get_mut(&key).expect("key just created");
            e.justified = Some(f);
            e.had_justified = true;
            e.beliefs.push(Justif { ri, tuple: tuple.clone(), seq });
            self.tms.by_fact.insert(f, key.clone());
            inserted = Some(f);
        }
        self.tms.firing_keys.push(key.clone());
        match self.tms.by_act.iter_mut().find(|(a, _)| *a == act) {
            Some((_, keys)) => {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            None => self.tms.by_act.push((act, vec![key])),
        }
        if let Some(f) = inserted {
            self.on_insert(f, Some(ri));
        }
        Ok(())
    }

    /// Delete routing with the pinned Drools quirk (dumps 1/2a/3):
    /// on a key with a live justified handle, delete() kills the
    /// JUSTIFIED fact whichever handle was named; once a key has hosted
    /// a justified handle, deleting a stated sibling silently no-ops.
    fn tms_route_delete(&mut self, f: FactId) -> Option<FactId> {
        self.tms_route_delete_ex(f, false).0
    }

    /// `rhs` = the delete comes from a rule consequence: a stated-handle
    /// delete then UNSTAGES a pending logical belief into a live
    /// justified fact (dump7). External deletes net materialize-then-die
    /// (dump8) — nothing survives, so no materialization is produced.
    fn tms_route_delete_ex(&mut self, f: FactId, rhs: bool) -> (Option<FactId>, Option<(TypeId, Vec<Value>)>) {
        let Some(key) = self.tms.by_fact.get(&f).cloned() else {
            return (Some(f), None); // not a logical-type fact: normal delete
        };
        let Some(e) = self.tms.keys.get_mut(&key) else {
            return (Some(f), None);
        };
        if let Some(jf) = e.justified {
            e.justified = None;
            e.beliefs.clear();
            let empty = e.stated.is_empty();
            self.tms.by_fact.remove(&jf);
            for (_, keys) in self.tms.by_act.iter_mut() {
                keys.retain(|k| *k != key);
            }
            self.tms.by_act.retain(|(_, keys)| !keys.is_empty());
            if empty {
                // no surviving handles: the key vanishes — a later
                // stated insert starts FRESH (fz_42_1395); the no-op
                // delete quirk only protects SIBLINGS that coexisted
                // with the justified handle (dump3).
                self.tms.keys.remove(&key);
            }
            return (Some(jf), None);
        }
        if e.had_justified {
            return (None, None); // dump3: undeletable stated sibling
        }
        e.stated.retain(|x| *x != f);
        self.tms.by_fact.remove(&f);
        if rhs && e.stated.is_empty() && !e.beliefs.is_empty() {
            if let Some(vals) = e.pending_vals.take() {
                // unstage (dump7): the pending justified belief becomes
                // a live fact after the stated handle dies; its deps
                // are already in place.
                return (Some(f), Some((key.0, vals)));
            }
        }
        e.beliefs.clear(); // stated-only key dies with its handles (tms_e6)
        for (_, keys) in self.tms.by_act.iter_mut() {
            keys.retain(|k| *k != key);
        }
        self.tms.by_act.retain(|(_, keys)| !keys.is_empty());
        if self.tms.keys.get(&key).map(|e| e.justified.is_none() && e.stated.is_empty()).unwrap_or(false) {
            self.tms.keys.remove(&key);
        }
        (Some(f), None)
    }

    /// Materialize an unstaged justified belief (dump7): insert the
    /// pending values as the key's justified handle.
    fn tms_materialize(&mut self, tid: TypeId, vals: Vec<Value>) -> Result<(), EngineError> {
        let key = (tid, key_vals(&vals));
        let f = self.store.insert(tid, vals).map_err(EngineError)?;
        if let Some(e) = self.tms.keys.get_mut(&key) {
            e.justified = Some(f);
            e.had_justified = true;
        }
        self.tms.by_fact.insert(f, key);
        self.on_insert(f, None);
        Ok(())
    }

    /// EAGER unmatch path (D-076, t1/t5/t8; Drools ModifyPreviousTuples
    /// analog): a DELETE or alpha-breaking UPDATE of a fact cancels the
    /// justifying activations whose tuples CONTAIN it, inside the same
    /// WM action — deps drop, emptied belief sets retract (nested
    /// actions cascade recursively, e7). CE- and beta-mediated breaks
    /// take the LAZY path at the justifier's terminal instead
    /// (tms_on_terminal_del — t11/t12/min_1310).
    fn tms_eager_break(&mut self, f: FactId) {
        if self.tms.by_act.is_empty() {
            return;
        }
        let broken: Vec<(usize, Tup)> = self
            .tms
            .by_act
            .iter()
            .filter(|((ri, tuple), _)| {
                // a justifier breaking its OWN tuple mid-firing lands
                // LAZY at its terminal instead (fz_42_2442: R3's higher-
                // salience activation fires before the retract).
                if self.tms.current_act.as_ref() == Some(&(*ri, tuple.clone())) {
                    return false;
                }
                // eager teardown reaches the terminal DIRECTLY only for
                // k=1 justifiers (LIA->terminal); k>=2 tuples die via
                // staged network propagation = the LAZY path
                // (min3783: a witness fires on the transient between a
                // join-justifier's tuple-fact delete and its item's
                // evaluation, exactly like t11/t12).
                if !self.nets[*ri].path.is_empty() {
                    return false;
                }
                tuple.contains(&f) && {
                    let dead = !self.store.is_alive(f);
                    dead || {
                        // alpha re-check of f's own slots only
                        self.rules[*ri].patterns.iter().enumerate().any(|(pos, pat)| {
                            pat.tpos.map(|t| tuple[t] == f).unwrap_or(false)
                                && !self.alpha_passes(*ri, pos, f)
                        })
                    }
                }
            })
            .map(|(a, _)| a.clone())
            .collect();
        for act in broken {
            self.tms_drop_act_deps(&act);
        }
    }

    /// LAZY unmatch path (D-076, t11/t12/t15/min_1310): dep removal
    /// rides the TERMINAL tuple delete at the justifier's own agenda
    /// evaluation (Drools cancelActivation ->
    /// removeLogicalDependencies). Self-defeat quirk: when a retracted
    /// fact could have been the not-CE blocker of the act's own tuple,
    /// the unblock's terminal re-add is suppressed ONE-SHOT — the tuple
    /// stays parked until a left-side event re-propagates it (t15's
    /// revival by property-relevant update; t10/t11 fire once).
    fn tms_on_terminal_del(&mut self, ri: usize, tuple: &Tup) {
        if self.tms.by_act.is_empty() {
            return;
        }
        let act = (ri, tuple.clone());
        if !self.tms.by_act.iter().any(|(a, _)| *a == act) {
            return;
        }
        if self.tms.defer_mode {
            if !self.tms.deferred.iter().any(|(r, t, _)| (*r, t) == (act.0, &act.1)) {
                let eager_ok = act.1.iter().any(|f| {
                    self.tms
                        .left_touched
                        .iter()
                        .any(|(lf, lo)| lf == f && *lo == Some(act.0))
                });
                self.tms.deferred.push((act.0, act.1, eager_ok));
            }
            return;
        }
        let retracted = self.tms_drop_act_deps(&act);
        for f in retracted.clone() {
            let ftid = self.store.fact_type(f);
            // alpha check on the (now-dead) fact's stale values — the
            // aliveness gate would always fail post-retract.
            let self_blocker = self.rules[ri].patterns.iter().enumerate().any(|(pos, pat)| {
                pat.ce == CeKind::Not && pat.type_id == ftid && {
                    let rule = &self.rules[ri];
                    let env = JoinEnvImpl { store: &self.store, rule };
                    pat.cmps.iter().all(|c| {
                        if let Test::Group { g, cross_var, .. } = &c.test {
                            return *cross_var
                                || eval_gexpr(g, &self.store, f, None, pat.tpos);
                        }
                        let lhs = self.store.value(f, c.field_idx);
                        match &c.test {
                            Test::Cmp { op, rhs: Src::Lit(v) } => eval_cmp(&lhs, *op, v),
                            Test::Cmp { .. } => true,
                            other => eval_alpha_test(&lhs, other),
                        }
                    }) && phreak::JoinEnv::allowed(&env, pos - 1, tuple, f)
                }
            });
            if self_blocker {
                self.tms.parked.push((ri, tuple.clone()));
                // Drools leaks the WHOLE blocked list of the dying
                // blocker (tms_t21: sibling tuples blocked by the same
                // self-defeat fact stay parked too, firing once not
                // per-tuple).
                for pos in 0..self.rules[ri].patterns.len() {
                    let pat = &self.rules[ri].patterns[pos];
                    if pat.ce != CeKind::Not || pat.type_id != ftid || pos == 0 {
                        continue;
                    }
                    let ni = self.nets[ri].path[pos - 1];
                    if let Some(lefts) = self.trie[ni].node.blocked_of(f) {
                        for lt in lefts {
                            if !self.tms.parked.iter().any(|(r, t)| *r == ri && *t == lt) {
                                self.tms.parked.push((ri, lt));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Park bookkeeping at terminal events (D-076 self-defeat quirk).
    /// INS while parked: skipped (right-side churn can re-add children;
    /// Drools' parked tuple never sees them). UPD: left-side event —
    /// unpark and activate. DEL: unpark only when a tuple fact died or
    /// fails its alpha (left-side death); blocking churn keeps the park.
    fn tms_parked_ins(&self, ri: usize, t: &Tup) -> bool {
        self.tms.parked.iter().any(|(pri, pt)| *pri == ri && pt == t)
    }

    fn tms_unpark_upd(&mut self, ri: usize, t: &Tup) -> bool {
        if let Some(i) = self.tms.parked.iter().position(|(pri, pt)| *pri == ri && pt == t) {
            self.tms.parked.remove(i);
            true
        } else {
            false
        }
    }

    fn tms_parked_del(&mut self, ri: usize, t: &Tup) {
        let Some(i) = self.tms.parked.iter().position(|(pri, pt)| *pri == ri && pt == t) else {
            return;
        };
        let left_death = t.iter().any(|f| !self.store.is_alive(*f)) || {
            let k = self.rules[ri].patterns.len();
            (0..k).any(|pos| {
                let pat = &self.rules[ri].patterns[pos];
                pat.tpos
                    .map(|tp| !self.alpha_passes(ri, pos, t[tp]))
                    .unwrap_or(false)
            })
        };
        if left_death {
            self.tms.parked.remove(i);
        }
    }

    /// Remove an activation's deps; retract facts whose belief sets
    /// emptied (nested WM deletes — cascades recurse through
    /// tms_eager_break/terminal processing). Returns the retracted facts.
    fn tms_drop_act_deps(&mut self, act: &(usize, Tup)) -> Vec<FactId> {
        let keys = match self.tms.by_act.iter().position(|(a, _)| a == act) {
            Some(i) => self.tms.by_act.remove(i).1,
            None => return Vec::new(),
        };
        let mut to_retract: Vec<(u64, FactId)> = Vec::new();
        for key in keys {
            let Some(e) = self.tms.keys.get_mut(&key) else { continue };
            e.beliefs.retain(|j| !(j.ri == act.0 && j.tuple == act.1));
            if e.beliefs.is_empty() {
                if let Some(jf) = e.justified.take() {
                    self.tms.by_fact.remove(&jf);
                    to_retract.push((self.tms.seq, jf));
                }
                if e.stated.is_empty() {
                    self.tms.keys.remove(&key); // fz_42_1395: fresh start
                }
            }
        }
        to_retract.sort_by_key(|(s, f)| (*s, f.0));
        let mut out = Vec::new();
        for (_, jf) in to_retract {
            if self.store.is_alive(jf) {
                self.store.kill(jf);
                self.on_delete(jf, None);
                out.push(jf);
            }
        }
        out
    }

    /// The queryable justification graph (D-076): every justified fact
    /// with its supports and stated siblings — the why-engine substrate.
    pub fn justifications(&self) -> Vec<JustificationView> {
        let mut out: Vec<JustificationView> = Vec::new();
        for ((_, _), e) in self.tms.keys.iter().map(|(k, e)| (k, e)) {
            let Some(jf) = e.justified else { continue };
            let mut supports: Vec<SupportView> = e
                .beliefs
                .iter()
                .map(|j| SupportView {
                    rule: self.rules[j.ri].def.name.clone(),
                    tuple: j.tuple.clone(),
                    seq: j.seq,
                })
                .collect();
            supports.sort_by_key(|s| s.seq);
            out.push(JustificationView {
                fact: jf,
                rendering: self.store.render(jf),
                supports,
                stated_siblings: e.stated.iter().copied().filter(|f| self.store.is_alive(*f)).collect(),
            });
        }
        out.sort_by_key(|v| v.fact.0);
        out
    }

    /// Why does this fact hold? None for unkeyed or purely stated facts.
    pub fn why(&self, f: FactId) -> Option<JustificationView> {
        self.justifications().into_iter().find(|v| v.fact == f)
    }

    pub fn facts(&self) -> Vec<FactView> {
        let mut hidden: Vec<TypeId> = [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION]
            .iter()
            .filter_map(|n| self.store.type_id(n))
            .collect();
        hidden.extend(self.qrow_tids.iter().copied());
        self.store
            .live_facts()
            .filter(|f| !hidden.contains(&self.store.fact_type(*f)))
            .map(|f| self.store.render(f))
            .collect()
    }

    /// Run a DRL query against the current WM (Phase Q0). `None` args are
    /// unbound (Drools `Variable.v`); rows come back in the oracle-pinned
    /// order (D-050).
    pub fn run_query(
        &mut self,
        name: &str,
        args: &[Option<Value>],
    ) -> Result<crate::queries::QueryOutput, EngineError> {
        crate::queries::run_query(
            &self.store,
            &self.queries,
            &mut self.query_mem,
            name,
            args,
        )
    }
}

/// Boxed-scalar rendering for QueryArgs elements (D-056) — same shape as
/// query row scalars (D-049).
fn scalar_view(v: Value) -> FactView {
    let type_name = match v {
        Value::I64(_) => "Long",
        Value::F64(_) => "Double",
        Value::Str(_) => "String",
        Value::Bool(_) => "Boolean",
    };
    FactView {
        type_name: type_name.into(),
        fields: vec![("value".into(), v)],
        handle: u32::MAX,
        elems: None,
    }
}

/// First leaf field of a group — a stable anchor for CompiledCmp's
/// field_idx (never used semantically for groups; alpha/join eval
/// special-cases Test::Group before the shared lhs fetch).
fn first_group_field(g: &GExpr) -> usize {
    match g {
        GExpr::Cmp { field_idx, .. }
        | GExpr::Matches { field_idx, .. }
        | GExpr::Contains { field_idx, .. }
        | GExpr::InList { field_idx, .. } => *field_idx,
        GExpr::And(xs) | GExpr::Or(xs) => first_group_field(&xs[0]),
        GExpr::Not(x) => first_group_field(x),
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
            if let Test::Group { g, .. } = &c.test {
                return eval_gexpr(g, self.store, f, Some(l), pat.tpos);
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
            if let Test::Group { g, .. } = &c.test {
                return eval_gexpr(g, self.store, f, Some(l), pat.tpos);
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
                    || matches!(&c.test, Test::Group { cross_var: true, .. })
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
        Test::Cmp { .. } => unreachable!("Cmp handled by callers"),
        Test::Group { .. } => unreachable!("Group handled by callers"),
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

/// Join-constraint comparison (D-020 coercion), exported for queries.
pub(crate) fn eval_cmp_join_pub(lhs: &Value, op: CmpOp, rhs: &Value) -> bool {
    eval_cmp_join(lhs, op, rhs)
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
