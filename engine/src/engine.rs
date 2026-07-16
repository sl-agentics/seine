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

use crate::drl::{
    self, AccFunc, Action, AllenOp, CeKind, CmpOp, CmpRhs, Constraint, Literal, RhsArg, RuleDef,
};
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
/// D-108: collectSet results — canonicalized SORTED on both sides
/// (Drools iterates raw HashSet internals; order is unspecified).
pub(crate) const ACC_SETCOLLECTION: &str = "SetCollection";
const ACC_DECIMAL: &str = "Decimal";
const RESERVED_TYPES: [&str; 6] =
    [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION, ACC_SETCOLLECTION, ACC_DECIMAL];
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
    /// `field == null` / `field != null` — the D-097 surface mapping to
    /// IS [NOT] NULL: a DEFINITE two-valued test.
    IsNull { negated: bool },
    /// Constant UNKNOWN (D-097): synthesized for null members of
    /// `not in` lowerings — never admits, and stays UNKNOWN under
    /// negation inside groups.
    Unknown,
    /// CEP E1/E2 (D-101/D-118/D-119) interval temporal join: the Allen
    /// relation `op` holds between the SELF event `[Bs,Be]` (Bs=own.ts read
    /// at `field_idx`, Be=Bs+self_dur) and the ANCHOR event `[As,Ae]`
    /// (As=anchor ts, Ae=As+anchor_dur). `params` = 0-4 bounds (after/before
    /// carry `[lo,hi]`). `anchor` = (anchor tuple pos, anchor ts field-idx);
    /// `self_dur_fi`/`anchor_dur_fi` = the `@duration` field indices (None ⇒
    /// point ⇒ dur 0). Beta-only (D-101). Evaluated by `eval_allen`.
    Temporal {
        op: AllenOp,
        params: Vec<i64>,
        anchor: (usize, usize),
        self_dur_fi: Option<usize>,
        anchor_dur_fi: Option<usize>,
    },
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
    IsNull { field_idx: usize, negated: bool },
    /// Constant UNKNOWN (null in-list member inside a composite).
    Unknown,
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
/// Inline-group evaluation is TRI-STATE (D-097/SQL 3VL): None =
/// UNKNOWN. A null operand makes cmp/matches/contains/in UNKNOWN;
/// AND/OR/NOT compose per the pinned 3VL tables
/// (docs/duckdb-datatype-pins.md B) — the load-bearing case is
/// `!(…)` over UNKNOWN staying UNKNOWN (never admitting). Callers
/// admit only Some(true). Certified null-free scenarios are
/// bit-identical: without Null every leaf is Some(_) and the
/// combinators degenerate to two-valued logic.
/// D-108: the set-canonicalization key — mirrors the oracle's
/// sort-by-rendered-JSON ({"type":"Long","fields":{"value":N}} compares
/// lexicographically by type name then the JSON-rendered value).
fn scalar_canon_key(v: &Value) -> String {
    match v {
        Value::Bool(b) => format!("{{\"type\":\"Boolean\",\"fields\":{{\"value\":{b}}}}}"),
        Value::F64(x) => {
            format!("{{\"type\":\"Double\",\"fields\":{{\"value\":{x:?}}}}}")
        }
        Value::I64(n) => format!("{{\"type\":\"Long\",\"fields\":{{\"value\":{n}}}}}"),
        Value::Str(s) => format!("{{\"type\":\"String\",\"fields\":{{\"value\":{s:?}}}}}"),
        other => format!("{other:?}"),
    }
}

fn dbg_eval(ctx: &str, ri: usize) {
    if std::env::var("SEINE_EVAL_DEBUG").is_ok() {
        eprintln!("EVAL[{ctx}] rule {ri}");
    }
}

fn eval_gexpr(
    g: &GExpr,
    store: &FactStore,
    f: FactId,
    l: Option<&Tup>,
    tpos: Option<usize>,
) -> Option<bool> {
    match g {
        GExpr::Cmp { field_idx, op, rhs } => {
            let lhs = store.value(f, *field_idx);
            if lhs.is_null() {
                return None;
            }
            match rhs {
                Src::Lit(Value::Null) => unreachable!("surface == null compiles to IsNull; list nulls to Unknown"),
                Src::Lit(v) => Some(eval_cmp(&lhs, *op, v)),
                Src::Field(ti, fi) => {
                    let other = if Some(*ti) == tpos {
                        f
                    } else {
                        l.expect("cross_var group evaluated without a left tuple")[*ti]
                    };
                    let rv = store.value(other, *fi);
                    if rv.is_null() {
                        return None;
                    }
                    Some(eval_cmp_join(&lhs, *op, &rv))
                }
                Src::SnapField(..) => unreachable!("SnapField in LHS group"),
            }
        }
        GExpr::IsNull { field_idx, negated } => {
            Some(store.value(f, *field_idx).is_null() != *negated)
        }
        GExpr::Unknown => None,
        GExpr::Matches { field_idx, rx } => match store.value(f, *field_idx) {
            Value::Null => None,
            Value::Str(s) => Some(rx.accepts(&s)),
            _ => Some(false),
        },
        GExpr::Contains { field_idx, needle } => match store.value(f, *field_idx) {
            Value::Null => None,
            Value::Str(s) => Some(s.contains(needle.as_str())),
            _ => Some(false),
        },
        GExpr::InList { field_idx, items, negated } => {
            let lhs = store.value(f, *field_idx);
            if lhs.is_null() {
                return None;
            }
            let has_null_item = items.iter().any(|v| v.is_null());
            let hit = items.iter().any(|v| !v.is_null() && eval_cmp(&lhs, CmpOp::Eq, v));
            match (hit, has_null_item) {
                (true, _) => Some(!*negated),
                (false, true) => None, // pin C: the not-in null trap
                (false, false) => Some(*negated),
            }
        }
        GExpr::And(xs) => {
            let mut unknown = false;
            for x in xs {
                match eval_gexpr(x, store, f, l, tpos) {
                    Some(false) => return Some(false),
                    None => unknown = true,
                    Some(true) => {}
                }
            }
            if unknown { None } else { Some(true) }
        }
        GExpr::Or(xs) => {
            let mut unknown = false;
            for x in xs {
                match eval_gexpr(x, store, f, l, tpos) {
                    Some(true) => return Some(true),
                    None => unknown = true,
                    Some(false) => {}
                }
            }
            if unknown { None } else { Some(false) }
        }
        GExpr::Not(x) => eval_gexpr(x, store, f, l, tpos).map(|b| !b),
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
    /// Position in the per-rule temporal-distance matrix (D-132). Positive
    /// patterns reuse `tpos`; a temporal `not` gets a PHANTOM position (it
    /// records after/before @expires-inference edges without claiming a tuple
    /// slot); exists stays None (inference kept out, D-127). Drives the
    /// bare-pattern NEVER check so a temporally-constrained `not` is NOT forced
    /// to NEVER.
    temporal_pos: Option<usize>,
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
    /// P1c/D-089 subnetwork role: Inner patterns build the subnet branch
    /// (ordinary joins off the fork, tuple slots BEYOND the main prefix);
    /// the Outer pseudo-pattern is the counting CE node fed by the
    /// branch tip through the RIA hop.
    sub: SubRole,
    /// CEP E2 item D: interned entry-point id (0 = DEFAULT). A fact only
    /// enters this pattern when its entry-point matches (alpha_passes).
    entry_point: u32,
}

/// Subnetwork role of a compiled pattern (P1c/D-089).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SubRole {
    None,
    /// Member of a group CE's subnetwork branch (a positive join or a
    /// bare not/exists per its own `ce` kind — sn_g5 shapes).
    Inner,
    /// The group's outer counting node: `len` inner patterns precede it;
    /// `plen` = main-prefix tuple length (start-tuple truncation).
    Outer { len: usize, plen: usize },
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
    /// Hidden result type (ACC_LONG / ACC_DOUBLE / ACC_COLLECTION,
    /// or the per-rule groupby row type).
    result_tid: TypeId,
    /// Original arg variable name — identity-significant like any
    /// referenced variable (D-037 spirit; conservative).
    arg_name: Option<String>,
    /// D-108 groupby: the source field the group key reads.
    key_field: Option<usize>,
    /// CEP E2 item B (D-110): `over window:time(N)` — a source event
    /// contributes while `clock − ts < N`, evicted at `ts+N` (per-subtree
    /// unmatch: the fact survives WM). None = no window.
    window_time: Option<i64>,
    /// D-185: `over window:length(N)` — a SLOT-RETENTION ring of the last
    /// N admissions (TrieNode.win_ring); eviction pops the oldest SLOT via
    /// `stage_acc_removal` (detach — revival stays mask-gated, like time).
    window_len: Option<i64>,
}

/// Compiled RHS insert-arg expression (D-283 Tier 1): Java semantics —
/// i64 wraps, `/` truncates, `%` keeps the dividend's sign, div by
/// zero errors at fire time; any f64 operand promotes both to IEEE
/// doubles. Atoms reuse Src (snapshot semantics per fz_7_2525).
#[derive(Clone)]
enum CExpr {
    Atom(Src),
    Neg(Box<CExpr>),
    Bin(char, Box<CExpr>, Box<CExpr>),
}

enum CompiledAction {
    Insert { type_id: TypeId, args: Vec<CExpr> },
    /// D-076: TMS-justified insert. Args stay ATOMS — computed logical
    /// args are the stratified tier (D-282), walled at compile.
    InsertLogical { type_id: TypeId, args: Vec<Src> },
    Set { pos: usize, field_idx: usize, arg: Src },
    Update { pos: usize },
    Delete { pos: usize },
    /// D-106: push the group on the focus stack (relocate if present).
    SetFocus { group: String },
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
    /// D-140 (item #2): `Some(tuple_pos)` iff this rule is the modeled
    /// non-temporal `not <EVENT>() P()` shape — then the static-agenda pick
    /// reorders firings to the not-unblock BATCH-STAGING order
    /// (`not_order_key`), keying on the fact at tuple position `tuple_pos`
    /// (the P join slot). `None` for every other rule ⇒ plain FIFO, untouched.
    not_order_pos: Option<usize>,
    /// D-144 (item 1b Family B exists): this gated rule is `exists <EVENT>() P()`
    /// (not `not`). The witness-toggle RE-FIRE order is the D-140 EPOCH model
    /// (`not_order_key`) unconditionally — NOT the P-first SEGMENT branch (which
    /// is a `not`-only phenomenon, D-151: now the mechanical BfShadow).
    order_exists: bool,
    /// D-158: `Some(tuple_pos)` iff this rule is the modeled non-temporal
    /// `not <PLAIN>() P()` shape INSIDE A STREAM SESSION (the cf313x4
    /// family) — the gated pick then follows the PnShadow's emitted order.
    /// `None` everywhere else — in particular in NON-stream sessions, where
    /// the plain-not order is main-axis-certified and stays untouched.
    pn_pos: Option<usize>,
    /// D-162: `Some(tuple_pos)` iff this rule is the modeled non-temporal
    /// `exists <PLAIN>() P()` shape in a STREAM session — the plain-witness
    /// satisfy-order family. The gated pick follows the PxShadow's emitted
    /// order; `None` ⇒ plain FIFO, untouched (non-stream plain-exists is
    /// main-axis-certified).
    px_pos: Option<usize>,
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

/// D-151: the MECHANICAL per-arrival flush shadow for one gated
/// `not <EVENT>() P()` rule with BARE patterns — the engine port of the
/// D-150 spec (`tools/model_check_notorder_b.py MODEL=flush`, derived from
/// the BfDump graft + its PropagationList proxy). It replays, per external
/// op, the five pieces of Drools machinery that GENERATE the event-not
/// firing order, and at each fire boundary emits the predicted order that
/// ranks the gated agenda picks — retiring the D-140/D-143/D-146
/// phenomenological keys (which were per-regime shadows of this machinery):
///   1. the hidden state is the join right-memory LIST ORDER (`rtm`) plus
///      the staged-right-insert backlog; firing at an unblock =
///      reverse(rtm) (children prepend into trg, the terminal appends);
///   2. every op is a FIFO propagation entry; an EVENT insert force-flushes
///      an eval at its queue position, draining the backlog staged-LIFO
///      into rtm;
///   3. expiry entries only REGISTER — all retracts run at QUIESCENCE
///      (flushExpirations), after every queued entry (post-advance updates
///      included), in deadline order, each with its own force-eval; the
///      last retract relinks the not and unblocks. Explicit deletes retract
///      AT their queue position instead (D-138 delete-time semantics);
///   4. a bare-P update is an IMMEDIATE rtm move-to-tail at its position
///      (empty inferred mask ⇒ BetaNode.modifyObject reorder-only branch);
///      an update of a still-staged P is a total no-op; updates never
///      re-fire;
///   5. P staging queues the fire-loop eval ONLY while the segment is
///      linked: an E0 right-insert processed while linked UNLINKS the
///      unconstrained NotNode (unlinkNotNodeOnRightInsert; relink at the
///      last E0 retract), and the JOIN unlinks when its right counter hits
///      0 (last P deleted) and relinks on the next P insert — the linking
///      history decides which drain window each insert lands in.
/// Validated 0-div on 9,693 oracle scenarios (all event-blocker regimes +
/// the explicit-delete population + probes). Rules whose RHS touches a
/// gated type (or windowed/event-P shapes) never get a shadow — STATIC
/// exclusions at build keep it inside the validated external-op regime.
struct BfShadow {
    e0_tid: TypeId,
    e0_ep: u32,
    p_tid: TypeId,
    p_ep: u32,
    rtm: Vec<FactId>,
    staged: Vec<FactId>,
    e0_alive: Vec<FactId>,
    pending_exp: Vec<FactId>,
    not_linked: bool,
    join_count: u32,
    join_linked: bool,
    exec_queued: bool,
    if_blocked: bool,
    if_propagated: bool,
    /// Predicted emission order for the current fire cycle.
    q: Vec<FactId>,
    /// FactId -> rank in `q` (consumed by the gated pick; cleared at the
    /// fire boundary).
    emit_rank: HashMap<FactId, usize>,
}

impl BfShadow {
    fn new(e0_tid: TypeId, e0_ep: u32, p_tid: TypeId, p_ep: u32) -> Self {
        BfShadow {
            e0_tid,
            e0_ep,
            p_tid,
            p_ep,
            rtm: Vec::new(),
            staged: Vec::new(),
            e0_alive: Vec::new(),
            pending_exp: Vec::new(),
            not_linked: true,
            join_count: 0,
            join_linked: false,
            exec_queued: false,
            if_blocked: false,
            if_propagated: false,
            q: Vec::new(),
            emit_rank: HashMap::new(),
        }
    }

    fn segment_linked(&self) -> bool {
        self.not_linked && self.join_linked
    }

    /// doRightInserts: iterate the staged list LIFO, appending to rtm (the
    /// batch lands reversed); with lefts present each child prepends into
    /// trg so the terminal sees arrival order back (double reversal).
    fn drain(&mut self, emit_children: bool) {
        self.rtm.extend(self.staged.iter().rev().copied());
        if emit_children {
            self.q.extend(self.staged.iter().copied());
        }
        self.staged.clear();
    }

    /// One network evaluation: the not consumes its staged E0 op (upstream),
    /// then the join drains staged right-inserts, then an unblock emits
    /// reverse(rtm). `e0_op` = (is_insert, id). In a FIRE-LOOP eval only,
    /// the IF's own staged left-ins processes last (D-153, the exists-arc
    /// discovery ported back: a force-flush skips staged lefts, and
    /// one-sided windows never queue — so the IF can sit staged across
    /// epochs and its first emission then covers the whole rtm at that
    /// point; nb884x248/nb886x21).
    fn eval_window(&mut self, e0_op: Option<(bool, FactId)>, fire_loop: bool) {
        let mut unblock = false;
        if let Some((is_ins, id)) = e0_op {
            if is_ins {
                if self.segment_linked() {
                    self.not_linked = false; // unlinkNotNodeOnRightInsert
                }
                if self.if_propagated && !self.if_blocked {
                    self.if_blocked = true; // block: children die
                    self.q.clear(); // matchCancelled for queued
                }
            } else {
                self.e0_alive.retain(|&e| e != id);
                if self.if_propagated && self.if_blocked && self.e0_alive.is_empty() {
                    unblock = true;
                }
            }
        }
        let emit = self.if_propagated && !self.if_blocked;
        self.drain(emit);
        if unblock {
            self.if_blocked = false;
            let rev: Vec<FactId> = self.rtm.iter().rev().copied().collect();
            self.q.extend(rev);
        }
        if fire_loop && !self.if_propagated {
            self.if_propagated = true;
            self.if_blocked = !self.e0_alive.is_empty();
            if !self.if_blocked {
                let rev: Vec<FactId> = self.rtm.iter().rev().copied().collect();
                self.q.extend(rev);
            }
        }
    }

    /// External E0 insert at its queue position: staging notify (queue iff
    /// the segment is linked, read PRE-unlink), then the STREAM force-flush
    /// eval. `due_now` = the deadline is nonneg-past/at the insertion clock
    /// (Drools enqueues the expire action in the same flush; a NEGATIVE
    /// deadline is the DROOLS-455 leak — alive forever, never registered —
    /// D-133 boundary corrected by D-152).
    fn on_e0_insert(&mut self, id: FactId, due_now: bool) {
        if self.segment_linked() {
            self.exec_queued = true;
        }
        self.e0_alive.push(id);
        self.eval_window(Some((true, id)), false);
        if due_now {
            self.pending_exp.push(id);
        }
    }

    /// External P insert: stage (a prepend list modeled as an arrival-order
    /// Vec drained LIFO); counter 0->1 links the join (+ rule notify iff the
    /// segment completes), else first-staged notifies iff linked.
    fn on_p_insert(&mut self, id: FactId) {
        let staged_was_empty = self.staged.is_empty();
        self.staged.push(id);
        self.join_count += 1;
        if self.join_count == 1 {
            self.join_linked = true;
            if self.segment_linked() {
                self.exec_queued = true;
            }
        } else if staged_was_empty && self.segment_linked() {
            self.exec_queued = true;
        }
    }

    /// External bare-P update: immediate rtm move-to-tail at this queue
    /// position (reorder-only branch); still-staged ⇒ total no-op.
    fn on_p_update(&mut self, id: FactId) {
        if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
            self.rtm.push(id);
        }
        // D-166 (update-recency, the cf933x385 cell): if the emission queue
        // already formed (a window deadline forced an early eval inside the
        // same advance — the unblock q froze before this update's queue
        // position), the rtm move must reflect there too: q = reverse(rtm),
        // so move-to-tail ≡ hoist-to-front (tjt_933_min/tjt_933_upd_before_adv;
        // controls tjt_933_{noupd,upd_p2,split_epoch} pin the scope).
        if let Some(pos) = self.q.iter().position(|&p| p == id) {
            self.q.remove(pos);
            self.q.insert(0, id);
            // the rank map is a snapshot of q taken at the eval step —
            // rebuild it so the pick sees the hoist
            self.emit_rank = self.q.iter().enumerate().map(|(i, &f)| (f, i)).collect();
        }
    }

    /// External explicit delete (never the expiration drain): an E0 retracts
    /// AT ITS QUEUE POSITION (same eval as an expiry retract — relink on
    /// counter 1->0, unblock if last); a P annihilates its staged insert or
    /// leaves rtm, unlinking the join at counter 0, and cancels its queued
    /// activation.
    fn on_delete(&mut self, id: FactId) {
        if self.e0_alive.contains(&id) {
            self.pending_exp.retain(|&e| e != id);
            if self.e0_alive.len() == 1 {
                self.not_linked = true; // NotNode counter 1->0: relink
                self.exec_queued = true;
            } else if self.segment_linked() {
                self.exec_queued = true;
            }
            self.eval_window(Some((false, id)), false);
            return;
        }
        let in_staged = self.staged.contains(&id);
        let in_rtm = self.rtm.contains(&id);
        if in_staged || in_rtm {
            self.join_count -= 1;
            if self.join_count == 0 {
                self.join_linked = false; // join counter 1->0: unlink
            } else if self.segment_linked() {
                self.exec_queued = true;
            }
        }
        if in_staged {
            self.staged.retain(|&p| p != id); // addDelete annihilates
        } else if in_rtm {
            self.rtm.retain(|&p| p != id);
        }
        self.q.retain(|&p| p != id); // matchCancelled
    }

    /// A clock advance made this live E0's deadline due: register it (the
    /// retract itself runs at the next fire's quiescence). Engine deadline
    /// order = registration order.
    fn on_advance_due(&mut self, id: FactId) {
        if self.e0_alive.contains(&id) && !self.pending_exp.contains(&id) {
            self.pending_exp.push(id);
        }
    }

    /// fireAllRules analog: the fire-loop eval if the executor was queued
    /// (which also processes the IF's staged left-ins — D-153: the IF is
    /// NOT unconditionally propagated at fire 1; one-sided windows leave it
    /// staged); then QUIESCENCE — every registered expiration retracts in
    /// deadline order, each with its own force-eval; the last relinks the
    /// not and unblocks (a still-staged IF then processes in the continuing
    /// fire loop). Ranks the emitted order for the gated picks.
    fn pre_fire(&mut self) {
        if self.exec_queued {
            self.eval_window(None, true); // the fire-loop eval — runs iff
            self.exec_queued = false;     // QUEUED; processes the staged IF
        }
        for id in std::mem::take(&mut self.pending_exp) {
            if !self.e0_alive.contains(&id) {
                continue; // explicitly deleted after registration
            }
            if self.e0_alive.len() == 1 {
                self.not_linked = true; // NotNode counter 1->0: relink
                self.exec_queued = true;
            } else if self.segment_linked() {
                self.exec_queued = true;
            }
            self.eval_window(Some((false, id)), false);
        }
        if self.exec_queued && !self.if_propagated {
            self.eval_window(None, true); // a relink queued the executor with
                                          // the IF still staged: it processes
                                          // in the continuing fire loop
        }
        self.exec_queued = false;
        self.emit_rank = self.q.iter().enumerate().map(|(i, &f)| (f, i)).collect();
    }

    /// Fire boundary: the cycle's prediction is consumed.
    fn post_fire(&mut self) {
        self.q.clear();
        self.emit_rank.clear();
    }

}

/// D-152: the mechanical flush shadow for one gated `exists <EVENT>() P()`
/// rule with BARE patterns — the exists-side sibling of `BfShadow`, ported
/// from the graft-validated spec (`tools/model_check_exists.py EMODEL=flush`;
/// BfDump on ex501x14/ex990x20/ex990x32; 0-div on 5,274 oracle scenarios
/// across the banked D-144/D-147 populations and the full-axis
/// SEINE_EXPOP_FULL soup — P deletes, partial witness deletes, delayed first
/// satisfaction, staggered multi-witness expiry, due-on-arrival witnesses,
/// witness updates). Retires the D-144/D-147 key models (`not_order_key`
/// re-fire gating + `satisfy_seg`/`ins_seg` regime-2 split), which were
/// per-regime shadows of this machinery. Same rtm/staged carrier as the not
/// family; the EXISTS-side mechanics:
///   1. the IF (InitialFact) left is itself STAGED at the exists until the
///      first FIRE-LOOP eval — an E1 force-flush processes RIGHTS along the
///      path but staged LEFTS wait. A FIRST satisfaction therefore emits
///      reverse(rtm) at the fire-loop eval, AFTER that eval's own drain of
///      post-witness P's (ex990x20 fires [3,1,2]); a RE-satisfy (the IF
///      resident in the exists memory) emits at the witness's exec, after
///      that exec's drain — the D-144 "epoch reversal" and the D-147
///      before/after-witness split both fall out of this seam;
///   2. the fire-loop eval runs iff the RuleAgendaItem got QUEUED this
///      window: the satisfy-link COMPLETING the segment (witness count 0->1
///      with the join populated), P staging while the exists side is
///      populated, or a terminal-reaching delete. One-sided windows queue
///      nothing — the IF and the P backlog sit staged across epochs
///      (ex990x32 cycle 0: witnesses alone never link the rule);
///   3. an unsatisfy edge (count 1->0) is the IF left-DELETE: children die
///      and QUEUED activations cancel. An explicit E1 delete retracts at
///      its queue position; EXPIRY retracts run at QUIESCENCE (registered
///      by advance / due-on-arrival in the same flush, deadline order)
///      AFTER the agenda drained — pre-quiescence emissions fire (the
///      transient fires, probes xm1-xm4) and marked-expired witnesses keep
///      counting/blocking until their retract (`q_floor` protects the
///      already-drained prefix from a quiescence unsatisfy);
///   4. a drain emits children in arrival order iff the IF is THROUGH (the
///      regime-2 fresh stream fires); a bare-P update is an rtm
///      move-to-tail (staged: no-op) and never re-fires; witness updates
///      are inert; a P delete annihilates its staged insert or leaves rtm
///      and cancels its queued activation.
struct ExShadow {
    e1_tid: TypeId,
    e1_ep: u32,
    p_tid: TypeId,
    p_ep: u32,
    rtm: Vec<FactId>,
    staged: Vec<FactId>,
    e1_alive: Vec<FactId>,
    pending_exp: Vec<FactId>,
    join_count: u32,
    if_staged: bool,
    if_through: bool,
    exec_queued: bool,
    /// Predicted emission order for the current fire cycle; `q_floor` marks
    /// the agenda-drained prefix a quiescence unsatisfy cannot cancel.
    q: Vec<FactId>,
    q_floor: usize,
    emit_rank: HashMap<FactId, usize>,
}

impl ExShadow {
    fn new(e1_tid: TypeId, e1_ep: u32, p_tid: TypeId, p_ep: u32) -> Self {
        ExShadow {
            e1_tid,
            e1_ep,
            p_tid,
            p_ep,
            rtm: Vec::new(),
            staged: Vec::new(),
            e1_alive: Vec::new(),
            pending_exp: Vec::new(),
            join_count: 0,
            if_staged: true,
            if_through: false,
            exec_queued: false,
            q: Vec::new(),
            q_floor: 0,
            emit_rank: HashMap::new(),
        }
    }

    /// doRightInserts: iterate the staged prepend-list LIFO, appending to
    /// rtm; with the IF through, each child prepends into trg so the
    /// terminal sees arrival order back (double reversal).
    fn drain(&mut self, emit_children: bool) {
        self.rtm.extend(self.staged.iter().rev().copied());
        if emit_children {
            self.q.extend(self.staged.iter().copied());
        }
        self.staged.clear();
    }

    /// One network evaluation: the exists consumes its staged E1 op
    /// (`(is_insert, id)`), the join drains staged right-inserts, then the
    /// join's staged LEFTS process — the re-satisfy child in the same exec,
    /// the IF's own left-ins only in a FIRE-LOOP eval.
    fn eval_window(&mut self, e1_op: Option<(bool, FactId)>, fire_loop: bool) {
        let mut satisfy = false;
        if let Some((is_ins, id)) = e1_op {
            if is_ins {
                if !self.if_staged && !self.if_through && self.e1_alive.len() == 1 {
                    satisfy = true; // resident IF re-blocks: emits THIS exec
                }
            } else {
                self.e1_alive.retain(|&e| e != id);
                if self.if_through && self.e1_alive.is_empty() {
                    self.if_through = false; // left-delete: children die
                    self.q.truncate(self.q_floor); // matchCancelled for queued
                }
            }
        }
        self.drain(self.if_through);
        if satisfy {
            self.if_through = true;
            let rev: Vec<FactId> = self.rtm.iter().rev().copied().collect();
            self.q.extend(rev);
        }
        if fire_loop && self.if_staged {
            self.if_staged = false;
            if !self.e1_alive.is_empty() {
                self.if_through = true;
                let rev: Vec<FactId> = self.rtm.iter().rev().copied().collect();
                self.q.extend(rev);
            }
        }
    }

    /// External E1 insert at its queue position: the satisfy-link queues the
    /// rule iff it COMPLETES the segment (join populated); then the STREAM
    /// force-flush eval. `due_now` = deadline == insertion clock (registers
    /// in the same flush; deadline < clock is the D-132 leak — alive
    /// forever, never registered).
    fn on_e1_insert(&mut self, id: FactId, due_now: bool) {
        let was_empty = self.e1_alive.is_empty();
        self.e1_alive.push(id);
        if was_empty && self.join_count > 0 {
            self.exec_queued = true;
        }
        self.eval_window(Some((true, id)), false);
        if due_now {
            self.pending_exp.push(id);
        }
    }

    /// External P insert: stage; staging notifies iff the exists side is
    /// populated (marked-expired witnesses count until their retract).
    fn on_p_insert(&mut self, id: FactId) {
        self.staged.push(id);
        self.join_count += 1;
        if !self.e1_alive.is_empty() {
            self.exec_queued = true;
        }
    }

    /// External bare-P update: immediate rtm move-to-tail at this queue
    /// position (reorder-only branch); still-staged ⇒ total no-op.
    fn on_p_update(&mut self, id: FactId) {
        if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
            self.rtm.push(id);
        }
    }

    /// External explicit delete (never the expiration drain): an E1
    /// retracts AT ITS QUEUE POSITION (unsatisfy cancels queued
    /// activations); a P annihilates its staged insert or leaves rtm and
    /// cancels its queued activation. Terminal-reaching deletes queue the
    /// fire-loop eval.
    fn on_delete(&mut self, id: FactId) {
        if self.e1_alive.contains(&id) {
            self.pending_exp.retain(|&e| e != id);
            if self.if_through {
                self.exec_queued = true;
            }
            self.eval_window(Some((false, id)), false);
            return;
        }
        let in_staged = self.staged.contains(&id);
        let in_rtm = self.rtm.contains(&id);
        if in_staged || in_rtm {
            self.join_count -= 1;
            if self.if_through {
                self.exec_queued = true;
            }
        }
        if in_staged {
            self.staged.retain(|&p| p != id); // addDelete annihilates
        } else if in_rtm {
            self.rtm.retain(|&p| p != id);
        }
        self.q.retain(|&p| p != id); // matchCancelled
    }

    /// A clock advance made this live E1's deadline due: register it (the
    /// retract itself runs at the next fire's quiescence). Engine deadline
    /// order = registration order.
    fn on_advance_due(&mut self, id: FactId) {
        if self.e1_alive.contains(&id) && !self.pending_exp.contains(&id) {
            self.pending_exp.push(id);
        }
    }

    /// fireAllRules analog: the fire-loop eval if the executor was queued;
    /// then QUIESCENCE — the agenda has drained (`q_floor` fences the fired
    /// prefix), and every registered expiration retracts in deadline order,
    /// each with its own force-eval. Ranks the emitted order for the gated
    /// picks.
    fn pre_fire(&mut self) {
        if self.exec_queued {
            self.eval_window(None, true);
        }
        self.q_floor = self.q.len();
        for id in std::mem::take(&mut self.pending_exp) {
            if !self.e1_alive.contains(&id) {
                continue; // explicitly deleted after registration
            }
            self.eval_window(Some((false, id)), false);
        }
        self.q_floor = 0;
        self.exec_queued = false;
        self.emit_rank = self.q.iter().enumerate().map(|(i, &f)| (f, i)).collect();
    }

    /// Fire boundary: the cycle's prediction is consumed.
    fn post_fire(&mut self) {
        self.q.clear();
        self.emit_rank.clear();
    }
}

/// D-158: the mechanical flush shadow for one gated `not <PLAIN>() P()`
/// rule in a STREAM session — the plain-blocker sibling of `BfShadow`, the
/// engine port of `tools/model_check_notorder_b.py MODEL=pflush` (derived
/// from the BfDump pnb_* graft battery; 0-div on 1,667 oracle scenarios,
/// 781 of them frozen-model out-of-sample). Same rtm/staged carrier and
/// emit_rank pick plumbing; SIX deltas vs the event machine:
///   1. plain ops (P and D alike) STAGE until a network eval — no
///      per-arrival force-flush (that is EVENT machinery);
///   2. the executor evaluates iff QUEUED: a D-DELETE staging queues (any
///      provenance); a D-INSERT or P-INSERT queues only while the left is
///      present (linked); a pure D-ins while blocked queues NOTHING ⇒
///      multi-epoch staged backlogs;
///   3. lazy smem init: the FIRST-ever eval drains staged rights into rtm
///      even while blocked;
///   4. the not consumes its staged D ops SEQUENTIALLY in arrival order
///      with TRANSIENT releases: count 1->0 releases the left INTO the
///      join (drain + emit staged-children ++ reversed(pre-rtm)); a later
///      ins RE-BLOCKS — unfired emissions cancel, the DRAIN persists;
///      2->1 is ABSORBED, the join untouched;
///   5. staged-ins ANNIHILATION: a D delete reaching a still-unprocessed
///      staged ins removes it — the not never sees either;
///   6. D events reach the shadow at the WM level regardless of provenance
///      (external ops, RHS/logical inserts, TMS retracts, expiry
///      cascades) — the shadow needs NO deadline model of its own; the
///      engine's real expiry/TMS machinery already delivers deletes in
///      deadline order. In-fire D events run their eval immediately when
///      queued (churn / quiescence-retract semantics); external-phase ones
///      wait for the pre_fire eval.
struct PnShadow {
    d_tid: TypeId,
    d_ep: u32,
    p_tid: TypeId,
    p_ep: u32,
    rtm: Vec<FactId>,
    staged: Vec<FactId>,
    /// Pending not-side ops in ARRIVAL order: (is_insert, id, rhs stamp).
    /// The stamp exists for churn canonicalization only — a same-RHS
    /// epilogue retract hops before that RHS's own staged inses.
    d_staged: Vec<(bool, FactId, u64)>,
    d_alive: Vec<FactId>,
    if_propagated: bool,
    if_blocked: bool,
    smem_init: bool,
    exec_queued: bool,
    /// Predicted emission order for the current fire cycle.
    q: Vec<FactId>,
    /// FactId -> rank in `q` (consumed by the gated pick; cleared at the
    /// fire boundary). Rebuilt after every q mutation — in-fire D events
    /// extend the prediction mid-cycle.
    emit_rank: HashMap<FactId, usize>,
}

impl PnShadow {
    fn new(d_tid: TypeId, d_ep: u32, p_tid: TypeId, p_ep: u32) -> Self {
        PnShadow {
            d_tid,
            d_ep,
            p_tid,
            p_ep,
            rtm: Vec::new(),
            staged: Vec::new(),
            d_staged: Vec::new(),
            d_alive: Vec::new(),
            if_propagated: false,
            if_blocked: false,
            smem_init: false,
            exec_queued: false,
            q: Vec::new(),
            emit_rank: HashMap::new(),
        }
    }

    fn rerank(&mut self) {
        self.emit_rank = self.q.iter().enumerate().map(|(i, &f)| (f, i)).collect();
    }

    /// The (re-)released left propagates into the join: drain staged
    /// right-ins (reversed-append into rtm) and emit — staged children in
    /// arrival order, then the pre-drain rtm reversed (algebraically ==
    /// reversed(post-drain rtm)).
    fn join_left_ins(&mut self, pending: &mut Vec<FactId>) {
        let pre: Vec<FactId> = self.rtm.iter().rev().copied().collect();
        self.rtm.extend(self.staged.iter().rev().copied());
        pending.extend(self.staged.iter().copied());
        self.staged.clear();
        pending.extend(pre);
    }

    /// One network eval over the staged not-side ops (see the struct doc).
    fn eval(&mut self, fire_loop: bool) {
        let mut pending: Vec<FactId> = Vec::new();
        let left_at_start = self.if_propagated && !self.if_blocked;
        if !self.smem_init {
            self.smem_init = true;
            self.rtm.extend(self.staged.iter().rev().copied());
            self.staged.clear();
        }
        for (is_ins, d, _) in std::mem::take(&mut self.d_staged) {
            if is_ins {
                self.d_alive.push(d);
                if self.if_propagated && !self.if_blocked {
                    self.if_blocked = true; // (re-)block: children die
                    pending.clear();
                    self.q.clear(); // matchCancelled for queued
                }
            } else {
                self.d_alive.retain(|&x| x != d);
                if self.d_alive.is_empty() && self.if_propagated && self.if_blocked {
                    self.if_blocked = false; // release (maybe transient)
                    self.join_left_ins(&mut pending);
                }
            }
        }
        // the join is visited by this segment eval iff its LEFT was
        // populated when the eval began — it drains its right staging even
        // when the not blocked mid-eval (child-less then); a
        // blocked-at-start eval leaves the backlog staged
        if left_at_start && !self.staged.is_empty() {
            self.rtm.extend(self.staged.iter().rev().copied());
            if !self.if_blocked {
                pending.extend(self.staged.iter().copied());
            }
            self.staged.clear();
        }
        // the IF's own staged left-ins — first fire-loop eval only
        if fire_loop && !self.if_propagated {
            self.if_propagated = true;
            self.if_blocked = !self.d_alive.is_empty();
            if !self.if_blocked {
                self.join_left_ins(&mut pending);
            }
        }
        self.q.extend(pending);
        self.rerank();
    }

    fn on_p_insert(&mut self, id: FactId) {
        self.staged.push(id);
        if self.if_propagated && !self.if_blocked {
            self.exec_queued = true;
        }
    }

    /// Bare-P external update: immediate rtm move-to-tail at its queue
    /// position when flushed; still-staged ⇒ total no-op.
    fn on_p_update(&mut self, id: FactId) {
        if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
            self.rtm.push(id);
        }
    }

    fn on_p_delete(&mut self, id: FactId) {
        if let Some(pos) = self.staged.iter().position(|&p| p == id) {
            self.staged.remove(pos); // addDelete annihilates staged ins
        } else if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
        }
        self.q.retain(|&p| p != id); // matchCancelled
        self.rerank();
    }

    /// A D reaching the WM (any provenance). `in_fire` runs the eval
    /// immediately when queued (mid-fire staging is processed by the hot
    /// executor); external-phase events wait for pre_fire.
    fn on_d_insert(&mut self, id: FactId, in_fire: bool, seq: u64) {
        self.d_staged.push((true, id, seq));
        if self.if_propagated && !self.if_blocked {
            self.exec_queued = true;
        }
        if in_fire && self.exec_queued {
            self.eval(false);
            self.exec_queued = false;
        }
    }

    /// A D leaving the WM (explicit delete, TMS retract, expiry cascade).
    /// A still-staged ins ANNIHILATES; else the del stages and queues —
    /// and evaluates immediately in-fire (churn / quiescence retract).
    /// `churn` = the execute_rhs stale-key epilogue: the spec arrival order
    /// there is del-BEFORE-this-RHS's-inses (Drools' TMS WM-DELETE is
    /// synchronous at the fire while insertLogical is queued — bf_full
    /// [38]->[43]), so the del hops before the trailing same-`seq` staged
    /// inses; every other provenance appends in arrival order.
    fn on_d_delete(&mut self, id: FactId, in_fire: bool, churn: bool, seq: u64) {
        if let Some(pos) = self.d_staged.iter().position(|e| e.0 && e.1 == id) {
            self.d_staged.remove(pos);
            self.d_alive.retain(|&x| x != id);
            return;
        }
        let mut at = self.d_staged.len();
        if churn {
            while at > 0 && self.d_staged[at - 1].0 && self.d_staged[at - 1].2 == seq {
                at -= 1;
            }
        }
        self.d_staged.insert(at, (false, id, seq));
        self.exec_queued = true;
        if in_fire {
            self.eval(false);
            self.exec_queued = false;
        }
    }

    /// fireAllRules analog: the fire-loop eval if the executor was queued
    /// (a D-del staging, a linked P/D-ins, pending agenda items, or the
    /// first-ever fire). Quiescence needs no simulation here — expiry
    /// cascades arrive as in-fire WM deletes.
    fn pre_fire(&mut self, has_items: bool) {
        if self.exec_queued || !self.if_propagated || has_items {
            self.eval(true);
        }
        self.exec_queued = false;
    }

    /// Fire boundary: the cycle's prediction is consumed.
    fn post_fire(&mut self) {
        self.q.clear();
        self.emit_rank.clear();
    }
}

/// D-162: the mechanical flush shadow for the gated PLAIN-witness exists
/// (`exists <PLAIN>() P()` / `exists <PLAIN>(alpha) P()` in a STREAM
/// session — the fourth shadow). A Rust replay of the validated spec
/// `predict_pexists` (tools/model_check_exists.py EMODEL=pexists, 0-div
/// on 1,800 oracle scenarios, seeds 5001-5006): the pflush join skeleton
/// with the EXISTS polarity, the D-161 NET witness semantics, and a
/// link-counter queue economy. The gated pick follows `emit_rank`.
///
///  - P machinery is PnShadow verbatim (same downstream join): inserts
///    stage arrival-order; a bare-P update is an immediate rtm
///    move-to-tail when in rtm (no-op staged); a delete annihilates its
///    staged insert or leaves rtm and cancels its queued activation.
///  - Witness (D) ops stage and apply as ONE NET batch per eval — only
///    the net 0->1 / 1->0 transition satisfies (join_left_ins =
///    staged-arrival ++ reversed(rtm); NO refraction — a re-satisfy
///    re-emits the whole memory) or unsatisfies (children die, queued
///    activations cancel). Every eval drains staged P's into rtm
///    (reversed-append), emitting them iff THROUGH after the net step.
///  - Deletes are SYNC at entry (explicit + TMS cascade move the link
///    counter, annihilating still-staged inses); alpha-exit UPDATES of a
///    processed D are DEFERRED (staged del, NO counter move — the D-155
///    principle), of a still-staged ins a sync annihilation; alpha-admit
///    stages a fresh ins. The shadow tracks per-fact alpha state itself.
///  - Queue signals: LINK (a D-ins taking the counter 0->1 with the join
///    populated; a P-ins while the counter > 0) — DEQUEUED by any sync
///    counter 1->0 (segment delink); WM (explicit D deletes only, even
///    annihilating ones) — never dequeued. TMS cascade dels are silent.
///  - Evals run iff queued: pre-fire, mid-drain (a D event in-fire with
///    the executor queued or NE items pending), and the QUIESCENCE eval
///    (staged witness ops left at the agenda-empty point — where a
///    cross-boundary unsatisfy is observed; a same-window del+ins pair
///    has already coalesced net-wise). The IF left stays staged until
///    the first fire-loop eval.
struct PxShadow {
    d_tid: TypeId,
    d_ep: u32,
    p_tid: TypeId,
    p_ep: u32,
    rtm: Vec<FactId>,
    staged: Vec<FactId>,
    /// Pending witness ops in ARRIVAL order: (is_insert, id, rhs stamp).
    /// Applied as a NET batch at the eval; the stamp orders churn
    /// canonicalization only (a same-RHS epilogue retract hops before
    /// that RHS's own staged inses).
    d_staged: Vec<(bool, FactId, u64)>,
    /// PROCESSED live witnesses (the exists right memory, net).
    d_alive: Vec<FactId>,
    /// The exists link counter: |d_alive| + staged inses - sync dels.
    /// A sync 1->0 dequeues the LINK-class signal (segment delink).
    counter: i32,
    /// Per-fact alpha state (last seen) for classifying witness updates
    /// (exit-of-staged = annihilation; exit-of-processed = deferred del;
    /// admit = fresh ins).
    alpha_state: HashMap<FactId, bool>,
    if_propagated: bool,
    if_blocked: bool,
    exec_link: bool,
    exec_wm: bool,
    /// Predicted emission order for the current fire cycle.
    q: Vec<FactId>,
    emit_rank: HashMap<FactId, usize>,
}

impl PxShadow {
    fn new(d_tid: TypeId, d_ep: u32, p_tid: TypeId, p_ep: u32) -> Self {
        PxShadow {
            d_tid,
            d_ep,
            p_tid,
            p_ep,
            rtm: Vec::new(),
            staged: Vec::new(),
            d_staged: Vec::new(),
            d_alive: Vec::new(),
            counter: 0,
            alpha_state: HashMap::new(),
            if_propagated: false,
            if_blocked: true, // exists polarity: blocked while NO witness
            exec_link: false,
            exec_wm: false,
            q: Vec::new(),
            emit_rank: HashMap::new(),
        }
    }

    fn rerank(&mut self) {
        self.emit_rank = self.q.iter().enumerate().map(|(i, &f)| (f, i)).collect();
    }

    fn queued(&self) -> bool {
        self.exec_link || self.exec_wm
    }

    fn consume(&mut self) {
        self.exec_link = false;
        self.exec_wm = false;
    }

    /// A sync counter decrement; 1->0 dequeues the link-class signal.
    fn counter_dec(&mut self) {
        self.counter -= 1;
        if self.counter == 0 {
            self.exec_link = false;
        }
    }

    /// The satisfy left-ins into the join: emit staged-arrival ++
    /// reversed(pre-rtm) (algebraically reversed(post-drain rtm)).
    fn join_left_ins(&mut self, pending: &mut Vec<FactId>) {
        let pre: Vec<FactId> = self.rtm.iter().rev().copied().collect();
        self.rtm.extend(self.staged.iter().rev().copied());
        pending.extend(self.staged.iter().copied());
        self.staged.clear();
        pending.extend(pre);
    }

    /// One network eval: the NET witness step, then the join right-drain,
    /// then the IF's own staged left-ins (first fire-loop eval only).
    fn eval(&mut self, fire_loop: bool) {
        let mut pending: Vec<FactId> = Vec::new();
        let was = !self.d_alive.is_empty();
        for (is_ins, d, _) in std::mem::take(&mut self.d_staged) {
            if is_ins {
                self.d_alive.push(d);
            } else {
                self.d_alive.retain(|&x| x != d);
            }
        }
        self.counter = self.d_alive.len() as i32;
        let now = !self.d_alive.is_empty();
        if self.if_propagated {
            if was && !now && !self.if_blocked {
                self.if_blocked = true; // unsatisfy: children die
                self.q.clear(); // matchCancelled for queued
            } else if !was && now && self.if_blocked {
                self.if_blocked = false; // satisfy: left-ins into the join
                self.join_left_ins(&mut pending);
            }
        }
        // the join drains its staged rights at EVERY eval (a blocked eval
        // still moves the backlog into rtm); children emit iff THROUGH
        // after the net step
        if !self.staged.is_empty() {
            self.rtm.extend(self.staged.iter().rev().copied());
            if self.if_propagated && !self.if_blocked {
                pending.extend(self.staged.iter().copied());
            }
            self.staged.clear();
        }
        if fire_loop && !self.if_propagated {
            self.if_propagated = true;
            self.if_blocked = !now;
            if !self.if_blocked {
                self.join_left_ins(&mut pending);
            }
        }
        self.q.extend(pending);
        self.rerank();
    }

    fn on_p_insert(&mut self, id: FactId) {
        self.staged.push(id);
        if self.counter > 0 {
            self.exec_link = true; // staging notifies while linked
        }
    }

    fn on_p_update(&mut self, id: FactId) {
        if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
            self.rtm.push(id);
        }
    }

    fn on_p_delete(&mut self, id: FactId) {
        if let Some(pos) = self.staged.iter().position(|&p| p == id) {
            self.staged.remove(pos); // addDelete annihilates staged ins
        } else if let Some(pos) = self.rtm.iter().position(|&p| p == id) {
            self.rtm.remove(pos);
        }
        self.q.retain(|&p| p != id); // matchCancelled
        self.rerank();
    }

    /// Stage a witness ins (alpha already passed by the caller); a
    /// counter 0->1 with the join populated is the satisfy-link.
    fn stage_d_ins(&mut self, id: FactId, seq: u64) {
        self.d_staged.push((true, id, seq));
        self.counter += 1;
        if self.counter == 1 && (!self.staged.is_empty() || !self.rtm.is_empty()) {
            self.exec_link = true;
        }
    }

    /// A witness reaching the WM (any provenance). `has_ne` = the rule
    /// has queued unfired activations (the executor evaluates before its
    /// next item when witness ops are staged).
    fn on_d_insert(&mut self, id: FactId, alpha: bool, in_fire: bool, has_ne: bool, seq: u64) {
        self.alpha_state.insert(id, alpha);
        if !alpha {
            return;
        }
        self.stage_d_ins(id, seq);
        if in_fire && !self.d_staged.is_empty() && (self.queued() || has_ne) {
            self.eval(true);
            self.consume();
        }
    }

    /// A witness leaving the WM. SYNC: annihilates a still-staged ins or
    /// stages a del, moving the counter at entry (1->0 dequeues LINK
    /// signals). `explicit` (a direct session.delete of this handle)
    /// additionally carries the WM signal; TMS cascades are silent.
    /// `churn` hops the del before the same-RHS trailing staged inses.
    fn on_d_delete(
        &mut self,
        id: FactId,
        explicit: bool,
        in_fire: bool,
        has_ne: bool,
        churn: bool,
        seq: u64,
    ) {
        let was_alpha = self.alpha_state.remove(&id).unwrap_or(false);
        if !was_alpha {
            return;
        }
        if let Some(pos) = self.d_staged.iter().position(|e| e.0 && e.1 == id) {
            self.d_staged.remove(pos);
            self.counter_dec();
        } else {
            let mut at = self.d_staged.len();
            if churn {
                while at > 0 && self.d_staged[at - 1].0 && self.d_staged[at - 1].2 == seq {
                    at -= 1;
                }
            }
            self.d_staged.insert(at, (false, id, seq));
            self.counter_dec();
        }
        if explicit {
            self.exec_wm = true;
        }
        if in_fire && !self.d_staged.is_empty() && (self.queued() || has_ne) {
            self.eval(true);
            self.consume();
        }
    }

    /// An external witness update, classified against the tracked alpha
    /// state: exit-of-staged = staging-level annihilation (counter moves,
    /// no signal); exit-of-processed = DEFERRED del (no counter move);
    /// admit = a fresh staged ins; no-change = inert (bare witness mask).
    fn on_d_update(&mut self, id: FactId, alpha_now: bool, seq: u64) {
        let was = self.alpha_state.insert(id, alpha_now).unwrap_or(false);
        if was && !alpha_now {
            if let Some(pos) = self.d_staged.iter().position(|e| e.0 && e.1 == id) {
                self.d_staged.remove(pos);
                self.counter_dec();
            } else if self.d_alive.contains(&id) {
                self.d_staged.push((false, id, seq));
            }
        } else if !was && alpha_now {
            self.stage_d_ins(id, seq);
        }
    }

    /// fireAllRules analog: the fire-loop eval iff QUEUED.
    fn pre_fire(&mut self) {
        if self.queued() {
            self.eval(true);
        }
        self.consume();
    }

    /// The agenda-quiescence eval: staged witness ops left unprocessed at
    /// the window's end evaluate now even with nothing queued — where a
    /// cross-boundary unsatisfy is observed.
    fn quiescence(&mut self) {
        if !self.d_staged.is_empty() {
            self.eval(true);
            self.consume();
        }
    }

    /// Fire boundary: the cycle's prediction is consumed.
    fn post_fire(&mut self) {
        self.q.clear();
        self.emit_rank.clear();
    }
}

use crate::phreak::{self, Origin, Staged, Tup};
use smallvec::smallvec;

/// TMS equality-key value: Value with Java-equals semantics for doubles
/// (Double.equals = bit comparison: NaN==NaN, +0.0 != -0.0 — tms_u6).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum KeyVal {
    I(i64),
    F(u64),
    S(String),
    B(bool),
    /// D-097: nulls COLLAPSE in TMS value-equality keys (pin H:
    /// GROUP BY/DISTINCT treat NULLs as one group).
    Null,
    /// D-098: decimal keys are VALUE-identical across scales —
    /// normalized (trailing zeros stripped) so 1.10 == 1.1 (pin J).
    D(i128, u8),
}

fn key_vals(vals: &[Value]) -> Vec<KeyVal> {
    vals.iter()
        .map(|v| match v {
            Value::I64(n) => KeyVal::I(*n),
            Value::F64(x) => KeyVal::F(x.to_bits()),
            Value::Str(s) => KeyVal::S(s.clone()),
            Value::Bool(b) => KeyVal::B(*b),
            Value::Null => KeyVal::Null,
            Value::Dec { u, s } => {
                let (nu, ns) = crate::store::dec_normalize(*u, *s);
                KeyVal::D(nu, ns)
            }
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
    /// ⚖ D-208 ACTIVATION-BACKFILL (the port, D-211): keys form at TMS
    /// ACTIVATION = the session's first insertLogical. Stated facts
    /// noted BEFORE activation wait here (keyless — observationally a
    /// per-handle singleton key); the backfill maps the LAST per value.
    activated: bool,
    pre_stated: Vec<FactId>,
    /// ⚖ ORPHANS (x1/r1/L6 events): handles dropped from key
    /// bookkeeping when their key died around them — WM-alive and
    /// UNDELETABLE (route-delete no-ops).
    orphans: HashSet<FactId>,
    /// ⚖ UNSTAGE-BORN handles (dump7 materializations): TMS-dropped;
    /// their deletes must NOT cancel queued acts (the dynamic law).
    unstage_born: HashSet<FactId>,
    /// Freshly materialized facts awaiting the boundary force-eval
    /// (D-211/F2: the act must exist before any later delete).
    force_eval: Vec<FactId>,
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
    /// The u8 = CAUSE flags: bit0 = LEFT (an own-origin left/property
    /// hit — the update-break lane; flush-drains only MID-RUN, the
    /// last one rides to the item's pop — D-196 ip_c1/gt13); bit1 =
    /// RIGHT (an own-origin CE-side op — the self-defeat lane;
    /// flush-drains unconditionally at the run end — D-196 ip_a3).
    deferred: Vec<(usize, Tup, u8)>,
    /// D-102 (q1/q4/cf5x33): EXPIRATION-routed lazy teardowns — drain
    /// at the rule's post-firing block or at agenda QUIESCENCE; never
    /// make an item reachable, never drain at cloud fire-end
    exp_deferred: Vec<(usize, Tup)>,
    /// Facts touched by the CURRENT evaluation's left-side staging,
    /// with the staging origin (eager-flush drains are OWN-origin only,
    /// min3783 vs tms_t20_b_s).
    left_touched: Vec<(FactId, Origin)>,
    /// CE-side (right) staged ops consumed by the CURRENT evaluation,
    /// with origins — the SELF-DEFEAT signature (D-196 port, ip_a3 +
    /// the L-SD eager row): a no-loop justifier whose own insertLogical
    /// broke its own not has its terminal-del deferred with a
    /// right-side op of its OWN origin; that entry is flush-drainable
    /// (run-end landing) like an own-origin left hit. Foreign-origin
    /// right breaks stay lazy (the t20/min3783 discipline unchanged).
    right_touched: Vec<(FactId, Origin)>,
    /// Acts whose insertLogical ran AFTER their own tuple-break (the
    /// MUTFIRST signature — the dep attached late): their LEFT-lane
    /// last-firing teardown rides to the item's pop instead of the
    /// flush (D-195/D-196 race; gt13/ip_c1 vs pr_tms_t20d).
    late_acts: Vec<(usize, Tup)>,
    /// Own-origin ops reaching a JOIN's right (a positive non-LIA
    /// pattern — the LEAD topology's P side, ip_c1): flush-drainable
    /// MID-RUN only; the last firing's entry rides to the pop even for
    /// ilfirst (pr_tms_t20a/b/c + selfbreak_lazy certified pop).
    joinr_touched: Vec<(FactId, Origin)>,
    /// Ambient flag: inside the post-firing force evaluation.
    defer_mode: bool,
    /// Facts currently dying BY EXPIRATION (advance()): steers the
    /// k=1 eager-break scan onto the LAZY deferred path (D-101).
    expiring: std::collections::HashSet<FactId>,
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
    /// RIA hop (P1c/D-089): this node is a subnetwork TIP whose child
    /// tuples stage into the outer counting node's rights — per-entry
    /// prepend, i.e. the batch REVERSES (doRiaNode2 walk + addInsert;
    /// pinned by the sn replica against sn_a3/b3/b4/x*).
    Ria(usize),
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
    /// CEP E2 item C class 2 (D-137): events removed from `active` by a
    /// CLOCK job (window eviction / expiration-eager acc-removal via
    /// `stage_acc_removal`) — still `is_alive` until the deferred drain.
    /// A later external UPDATE must NOT revive such an event into the
    /// accumulate (Drools keeps it removed); the `on_update` re-entry
    /// branch consults this to suppress the revival. Populated only for
    /// event types (empty on the plain corpus ⇒ byte-identical).
    clock_removed: HashSet<FactId>,
    /// D-185 `window:length(N)`: the SLOT ring — FactIds in admission
    /// order. Slots are RETAINED by corpses (delete/exit/expiration leave
    /// the entry; it still evicts in FIFO order); only overflow pops.
    /// Empty unless this node's acc has `window_len`.
    win_ring: Vec<FactId>,
    /// D-185 (the LANDING LAW in the acc machinery): when deferred acc
    /// entries are PENDING (acc_pending, D-160), a walk-time admission's
    /// RING ops defer HERE and land at the drain AFTER the entries — in
    /// true action order (entries first at their FIFO positions, then the
    /// walk admissions in arrival order). Immediate ring ops would invert
    /// the order: a pre-eviction update becomes a spurious revival
    /// (wl603x23: 42 vs 3) and an entry-admission slots LAST instead of
    /// FIRST, surviving evictions it should take (wl603x54: 104 vs 15).
    win_admit_pending: Vec<FactId>,
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
    /// Subnetwork CE state (P1c/D-089, SubnetNot/SubnetExists only) —
    /// the counting machine per PhreakSubnetworkNotExistsNode. Rights
    /// are subnetwork TUPLES staged through the RIA hop (per-entry
    /// prepend = the pinned reversal); matches key by the tuple's START
    /// left (truncation to sn_plen). One child per left, a left copy.
    sn_right: Staged<Tup>,
    sn_matches: HashMap<Tup, Vec<Tup>>,
    sn_lefts: Vec<Tup>,
    sn_has_child: HashSet<Tup>,
    /// Main-prefix tuple length (start-tuple truncation, D-089).
    sn_plen: usize,
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
    /// D-098: exact decimal sum (unscaled, scale) — scales align UP as
    /// contributions arrive; overflow past i128/DECIMAL(38) panics
    /// loudly (DuckDB errors there too, pin J).
    sum_d: (i128, u8),
    count: i64,
    minmax: Option<Value>,
    list: Vec<FactId>,
    /// D-108 collectList: (fact, value) per match — ordered, one
    /// instance leaves per reverse.
    vlist: Vec<(FactId, Value)>,
    /// D-108 collectSet: COUNTED values in first-arrival order.
    vset: Vec<(Value, usize)>,
}

/// D-108: one live groupby group.
struct GbGroup {
    key: Value,
    ctx: AccCtx,
    row: Option<FactId>,
    propagated: bool,
}

impl AccCtx {
    fn new() -> AccCtx {
        AccCtx {
            result: None,
            propagated: false,
            matches: Vec::new(),
            sum_i: 0,
            sum_f: 0.0,
            sum_d: (0, 0),
            count: 0,
            minmax: None,
            list: Vec::new(),
            vlist: Vec::new(),
            vset: Vec::new(),
        }
    }

    /// reinit (fresh function state; the result fact and propagation
    /// flag survive — the handle is reused).
    fn reset_state(&mut self) {
        self.sum_i = 0;
        self.sum_f = 0.0;
        self.sum_d = (0, 0);
        self.count = 0;
        self.minmax = None;
        self.list.clear();
        self.vlist.clear();
        self.vset.clear();
    }

    /// accumulate(): the exact op sequence (D-038).
    fn apply(&mut self, func: AccFunc, f: FactId, v: &Value) {
        // D-097/pin G: null CONTRIBUTIONS are skipped by the value
        // aggregates (sum/average/min/max — average skips BOTH the sum
        // and the count). count() is count(*)-like (counts matches) and
        // collect gathers facts — neither looks at v.
        if v.is_null() && !matches!(func, AccFunc::Count | AccFunc::Collect) {
            return;
        }
        let _ = ();
        match func {
            AccFunc::Sum => match v {
                Value::I64(x) => self.sum_i += x,
                Value::F64(x) => self.sum_f += x,
                Value::Dec { u, s } => {
                    let t = (self.sum_d.1).max(*s);
                    let (a, _) = crate::store::dec_rescale(self.sum_d.0, self.sum_d.1, t)
                        .expect("decimal sum overflow (DECIMAL(38) exceeded)");
                    let (b, _) = crate::store::dec_rescale(*u, *s, t)
                        .expect("decimal sum overflow (DECIMAL(38) exceeded)");
                    self.sum_d = (
                        a.checked_add(b).expect("decimal sum overflow (DECIMAL(38) exceeded)"),
                        t,
                    );
                }
                _ => {}
            },
            AccFunc::Count => self.count += 1,
            AccFunc::Average => {
                let x = match v {
                    Value::I64(n) => *n as f64,
                    Value::F64(n) => *n,
                    // pin J: AVG(decimal) is DOUBLE — the one place a
                    // decimal deliberately becomes a float.
                    Value::Dec { u, s } => *u as f64 / 10f64.powi(*s as i32),
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
                            (Value::Dec { u: a, s: x }, Value::Dec { u: b, s: y }) => {
                                crate::store::dec_cmp(*a, *x, *b, *y)
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
            AccFunc::CollectList => self.vlist.push((f, v.clone())),
            AccFunc::CollectSet => {
                if let Some(e) = self.vset.iter_mut().find(|(x, _)| x == v) {
                    e.1 += 1;
                } else {
                    self.vset.push((v.clone(), 1));
                }
            }
        }
    }

    /// tryReverse(): true when reversed in place; min/max cannot reverse
    /// and require a reinit + refold over the remaining matches (D-038).
    fn try_reverse(&mut self, func: AccFunc, f: FactId, v: &Value) -> bool {
        // D-097: a skipped-null contribution has nothing to undo — and
        // must NOT trigger the min/max refold path.
        if v.is_null() && !matches!(func, AccFunc::Count | AccFunc::Collect) {
            return true;
        }
        match func {
            AccFunc::Sum => {
                match v {
                    Value::I64(x) => self.sum_i -= x,
                    Value::F64(x) => self.sum_f -= x,
                    Value::Dec { u, s } => {
                        let t = (self.sum_d.1).max(*s);
                        let (a, _) = crate::store::dec_rescale(self.sum_d.0, self.sum_d.1, t)
                            .expect("decimal sum overflow (DECIMAL(38) exceeded)");
                        let (b, _) = crate::store::dec_rescale(*u, *s, t)
                            .expect("decimal sum overflow (DECIMAL(38) exceeded)");
                        self.sum_d = (
                            a.checked_sub(b).expect("decimal sum overflow (DECIMAL(38) exceeded)"),
                            t,
                        );
                    }
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
                    Value::Dec { u, s } => *u as f64 / 10f64.powi(*s as i32),
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
            AccFunc::CollectList => {
                if let Some(i) = self.vlist.iter().position(|(x, _)| *x == f) {
                    self.vlist.remove(i);
                }
                true
            }
            AccFunc::CollectSet => {
                if let Some(i) = self.vset.iter().position(|(x, _)| x == v) {
                    self.vset[i].1 -= 1;
                    if self.vset[i].1 == 0 {
                        self.vset.remove(i);
                    }
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
                // D-098 ruling 2 composition: an empty/all-null decimal
                // sum is 0 AT THE FIELD'S SCALE and still fires.
                FieldType::Dec { s, .. } => {
                    let (u, us) = self.sum_d;
                    if us == 0 && u == 0 {
                        Value::Dec { u: 0, s }
                    } else {
                        Value::Dec { u, s: us }
                    }
                }
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
            AccFunc::CollectList | AccFunc::CollectSet => Some(Value::I64(0)),
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
    /// D-170 (T6 movability): queued activations staged by a TAG-class
    /// update of the recorded fact this epoch — a later alpha-entry of
    /// the SAME fact relocates them behind its fresh inserts. Cleared
    /// at the fire boundary.
    act_movable: HashMap<Tup, FactId>,
    queue: std::collections::VecDeque<Act>,
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
    /// D-084: the rule's item has EXISTED (the path linked at least
    /// once). A dirty was-linked rule re-queues even while unlinked —
    /// its evaluation before quiescence drains staged inputs into the
    /// node memories (one accumulated-LIFO batch), so a later fire
    /// call's fresh stagings land AFTER them (fz_min_455 pairing,
    /// pr_rl2/rl3/rl5). Never-linked rules have no item and keep
    /// holding (fz_7_145, pr_hw_jr10, pr_rl4).
    ever_linked: bool,
    /// D-091: RuleExecutor.dirty — the network-needs-evaluation flag,
    /// SEPARATE from `queued` (item on the agenda). Set by every
    /// staging notify while LINKED (queueRuleAgendaItem.setDirty) and
    /// by link/unlink transitions; cleared when the network evaluates.
    /// Gates evaluation (evaluateNetworkIfDirty) — staging that arrives
    /// while UNLINKED does not set it, so a queued-but-clean item pops
    /// without draining (the faithful hold). Item removal requires
    /// !dirty && queue-empty (removeRuleAgendaItemWhenEmpty).
    dirty: bool,
    /// D-106: stage_seq at the most recent dirty-marking — the
    /// executor's halt-peek ignores dirt born of the CURRENT firing.
    dirty_stamp: u64,
    /// D-151: the mechanical flush shadow for a gated `not <EVENT>() P()`
    /// rule with bare patterns (None otherwise — constrained/exotic shapes
    /// fall to FIFO). Replaces the retired D-140/D-143/D-146 key models.
    bf: Option<BfShadow>,
    /// D-152: the mechanical flush shadow for a gated `exists <EVENT>() P()`
    /// rule with bare patterns, under the same static exclusions. Replaces
    /// the retired D-144/D-147 key models (re-fire epoch key + regime-2
    /// segment split — per-regime shadows of this machinery).
    ex: Option<ExShadow>,
    /// D-158: the mechanical flush shadow for a gated `not <PLAIN>() P()`
    /// rule in a STREAM session with bare patterns (the cf313x4 family).
    pn: Option<PnShadow>,
    /// D-162: the mechanical flush shadow for a gated `exists <PLAIN>() P()`
    /// rule in a STREAM session (the plain-exists satisfy-order family).
    /// The witness pattern may carry ALPHA-only constraints (the cons
    /// drive); the P pattern must be bare.
    px: Option<PxShadow>,
}

impl RuleNet {
    /// Window-aware k=1 staging (D-047): TupleSets folds span windows.
    fn s0_add_ins(&mut self, f: FactId, o: Origin) {
        // D-267: a seen-miss on every window PROVES f is unstaged (the
        // cross-window walk was O(staged) per insert = O(N²) per flush,
        // the 78% flamegraph box); a stale-positive hit falls back to
        // the exact scan.
        if self.s0.iter().any(|w| w.maybe_contains(&f))
            && self.s0.iter().any(|w| {
                w.ins.iter().any(|(x, _, _)| *x == f) || w.upd.iter().any(|(x, _, _)| *x == f)
            })
        {
            return;
        }
        self.s0.last_mut().unwrap().add_ins(f, o);
    }

    fn s0_add_upd(&mut self, f: FactId, o: Origin) {
        // D-267: same seen-miss fast path as s0_add_ins.
        if self.s0.iter().any(|w| w.maybe_contains(&f))
            && self.s0.iter().any(|w| {
                w.ins.iter().any(|(x, _, _)| *x == f)
                    || w.upd.iter().any(|(x, _, _)| *x == f)
                    || w.del.iter().any(|(x, _, _)| *x == f)
            })
        {
            return;
        }
        self.s0.last_mut().unwrap().add_upd(f, o);
    }

    fn s0_add_del(&mut self, f: FactId, o: Origin) {
        // D-267: same seen-miss fast path — a miss everywhere skips all
        // three cancel/dedup walks (the lists provably hold no f).
        if self.s0.iter().any(|w| w.maybe_contains(&f)) {
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
                pending.seen_add(t);
                pending.upd.push_front((t.clone(), *o, *ph));
            }
        }
        for (t, o, ph) in &fresh.ins {
            if let Some(i) = pending.ins.iter().position(|(x, _, _)| x == t) {
                if let Some(e) = pending.ins.remove(i) {
                    pending.ins.push_front(e);
                }
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| x == t) {
                pending.upd.remove(i);
                pending.upd.push_front((t.clone(), *o, *ph));
                continue;
            }
            if self.peer_live.contains(t) {
                pending.seen_add(t);
                pending.upd.push_front((t.clone(), *o, *ph));
                continue;
            }
            self.peer_live.insert(t.clone());
            pending.seen_add(t);
            pending.ins.push_front((t.clone(), *o, *ph));
        }
        self.term_pending = pending;
    }
}

/// Per-event-type metadata (CEP E1/E2). `ts_fi` = timestamp field index
/// (i64 epoch-ms, read at insert). `expires` = `Some(ms)` → auto-retract at
/// `ts+dur+ms(+1, D-102)`; `None` = NEVER (explicit `@expires` kept verbatim,
/// else D-109-inferred). `dur_fi` = OPTIONAL `@duration` field index (CEP E2
/// item E, D-118): the event occupies the interval `[ts, ts+dur]`; `None` ⇒
/// point event ⇒ dur=0 everywhere ⇒ BYTE-IDENTICAL to pre-item-E (the Q3
/// corpus-preservation gate). `Copy` so the ~8 read sites stay a one-line
/// `let EventSpec { .. } = *self.event_specs.get(..)`.
#[derive(Clone, Copy)]
struct EventSpec {
    ts_fi: usize,
    expires: Option<i64>,
    dur_fi: Option<usize>,
}

/// D-160: one queued external op on an EVENT-TYPED accumulate source —
/// Drools executes these per-entry (FIFO) at the fire drain against the
/// epoch-final bean, each entry dirtying the accumulate result.
#[derive(Clone, Copy)]
enum AccEntry {
    /// External update carrying ITS OWN written-mask (no merging).
    Upd(u64),
    /// Explicit external delete (expiry stays on its own certified path).
    Del,
}

pub struct Engine {
    /// CEP E1 (D-100/D-101): pseudo-clock in ms. Advances only via
    /// advance(); starts at 0 like the oracle's PseudoClockScheduler.
    clock_ms: i64,
    /// Event metadata per type (see `EventSpec`): timestamp field index,
    /// expiry, and the optional `@duration` field index (item E). Explicit
    /// `@expires` is kept verbatim (Some); un-annotated event types are
    /// filled by `infer_event_expiry` after rule compile (CEP E2 item A,
    /// D-109) — offset = MAX over temporal constraints of {+hi if earlier,
    /// -lo if later}; a MAX < 0 (the lo>0 later-event leak) or no
    /// constraint → None. Allen-op constraints (item E) contribute NO
    /// inference edge (D-120 fence): an Allen-only event type infers NEVER.
    event_specs: std::collections::HashMap<TypeId, EventSpec>,
    /// Event types with an EXPLICIT `@expires` — inference skips these
    /// (a8: explicit hard expiry overrides the inferred reach; Drools
    /// `PatternBuilder` `if(hard) use it`, no max-merge).
    explicit_expiry: std::collections::HashSet<TypeId>,
    /// D-109 inference accumulator: per event type, the running MAX of
    /// its temporal-constraint upperBound contributions (row-max of the
    /// TemporalDependencyMatrix). Collected during `compile_rule`;
    /// consumed by `infer_event_expiry`. A→B SEAM: window:time(N) folds
    /// its size in here (max) when item B lands.
    temporal_ub: std::collections::HashMap<TypeId, i64>,
    /// D-109: event types whose inferred expiry is forced to NEVER. Two
    /// sources, both = Drools `getExpirationOffset` returning NEVER and
    /// OVERWRITING the OTN offset (order-independent; nb/char/iso probes):
    /// (1) a BARE pattern — positive/not/exists where the type has no
    /// temporal constraint; (2) a purely-BACKWARD pattern — its per-rule
    /// row-max upperBound is < 0 (the LATER event of `after[lo>0]`, or a
    /// self-join's probe side). Explicit `@expires` (hard) is immune.
    never_inferred: std::collections::HashSet<TypeId>,
    /// Deadline-ordered expiration queue; Vec preserves insertion
    /// order within a deadline (the a2-pinned stable tie order).
    deadlines: std::collections::BTreeMap<i64, Vec<FactId>>,
    pending_expirations: Vec<FactId>,
    /// CEP E2 item B (D-110): scheduled per-subtree window evictions,
    /// keyed by the wall-clock deadline `ts+N` (NO +1 — win_t_b pins the
    /// boundary at exactly ts+N, distinct from expiration's ts+off+1).
    /// Each entry is (windowed accumulate trie-node idx, event id).
    /// D-112: drained EAGERLY in `advance()` — a SCOPED right-delete at
    /// the node drops the count (Phase B→G re-fire) WITHOUT retracting the
    /// fact (the fact survives WM-wide: win_t_b/win_x_bare keep E while
    /// count→0). Independent of expiration — fires even under a huge or
    /// explicit @expires.
    window_deadlines: std::collections::BTreeMap<i64, Vec<(usize, FactId)>>,
    /// D-154: FIFO of EXTERNAL update ENTRIES for facts whose type feeds a
    /// windowed accumulate. Drools queues each update as its own
    /// propagation entry (its own written-mask) and executes it at the
    /// next flush point against the LIVE bean — by then the epoch-FINAL
    /// field state (BfDump proxy: a same-epoch tag z→x pair never shows
    /// the network the z state; a fire boundary between them does).
    /// Drained FIFO at fire_all pre-fire. Single-update epochs are
    /// equivalent to the pre-D-154 immediate processing: staging lands
    /// before the first agenda pick either way, and the pick orders by
    /// (salience, decl_pos), not queue time.
    /// D-160 generalizes the queue to ALL accumulate nodes over
    /// EVENT-TYPED sources and adds explicit external DELETES as entries:
    /// per-entry FIFO execution against epoch-final fields, with
    /// aliveness decided by ENTRY ORDER (an update entry followed by a
    /// delete entry executes while "alive" — Drools fires the net value,
    /// even net-zero). Plain-typed sources keep immediate processing
    /// (oracle-certified: ap1/ap1b probes — plain ops batch-annihilate).
    acc_pending: Vec<(FactId, AccEntry)>,
    /// D-134 (§3B): scheduled temporal-`not` firing DEFERRALS, keyed by the
    /// window-close `fire_time` (mirror of `deadlines`, but a firing not a
    /// reap). Each entry is (not-node trie idx, held left tuple, origin).
    /// Populated from `Node.new_deferrals` after a not node evaluates;
    /// drained at fire quiescence (`drain_pending_fires`) BEFORE the
    /// expiration reap, so a not fires while its anchor is still alive.
    fire_deadlines: std::collections::BTreeMap<i64, Vec<(usize, Tup, Origin, u64)>>,
    /// D-134 (§3B): monotonic CREATION sequence stamped on each deferral (the
    /// D-125 join-creation order the not node sees). The release drain uses it
    /// as the tie-break so a batch fires in the model's order: creation order
    /// at the initial fire (agenda FIFO), (−fire_time, creation) at an advance.
    fire_seq: u64,
    /// D-112 (accumulate-eager deferral): recomputed by each
    /// `build_network` — (accumulate trie-node idx, source event type,
    /// window size N or None for a plain accumulate). Windowed nodes
    /// schedule an eviction at `ts+N`; ALL accumulate nodes take an EAGER
    /// right-delete at advance-time when a feeding event expires, so the
    /// count-drop lands before the fire's inserts and fires by salience
    /// (df_* pins; model_check_accdefer survivor: acc EAGER, not-CE LAZY).
    acc_nodes: Vec<(usize, TypeId, Option<i64>, Option<i64>)>,
    in_expiration_drain: bool,
    in_stream_flush: bool,
    /// D-158: inside fire_all's activation loop — PnShadow D events arriving
    /// here (TMS churns, expiry cascades) evaluate immediately when queued;
    /// external-phase ones wait for the boundary eval.
    in_fire_loop: bool,
    /// D-158: RHS-execution stamp for PnShadow churn canonicalization —
    /// bumped at each execute_rhs (and each external-phase D event, so a
    /// stale backlog ins can never alias a live RHS).
    pn_seq: u64,
    /// D-162: the handle currently being session.delete'd (explicit
    /// provenance for the px shadow's WM signal) — cascaded TMS retracts
    /// inside the same delete are NOT it.
    px_explicit_victim: Option<FactId>,
    /// D-158: set around execute_rhs's stale-key TMS retract epilogue — the
    /// ONE site whose WM deletes are churn-class: the spec order there is
    /// del-BEFORE-this-RHS's-inses (Drools' synchronous WM-DELETE vs queued
    /// insertLogical), while the engine retracts in the epilogue (after).
    pn_churn_ctx: bool,
    fire_no: u64,
    flush_trigger_tid: Option<TypeId>,
    /// D-170 (T6): the in-flight EXTERNAL event-update trigger —
    /// (fact, written mask, type). Drives emission movability and the
    /// relocation gate at the terminal consume; None outside the
    /// per-action update flush.
    tj_trigger: Option<(FactId, u64, TypeId)>,
    /// D-170: rules whose pattern-0 alpha the trigger fact ENTERED
    /// (stage 1) during this update — the relocation gate.
    tj_entered: Vec<usize>,
    ever_linked: Vec<bool>,
    stage_seq: u64,
    /// D-106: the agenda focus stack. Empty = MAIN focused. Rules
    /// fire only while their group is on TOP; an emptied top pops.
    focus_stack: Vec<String>,
    /// Set by a firing's setFocus; the post-firing executor-continue
    /// applies only then (the rescan is pool-equivalent otherwise).
    focus_changed: bool,
    /// stage_seq at the start of the CURRENT firing — the halt-peek
    /// ignores dirt born after it (fz_9003_879 vs fz_9005_2842).
    firing_stage_floor: u64,
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// CEP E2 item D: entry-point interning. `entry_points[0]` = "DEFAULT";
    /// a pattern/fact's entry-point is a u32 index. `ep_ids` maps name→index,
    /// built at COMPILE from rule `from entry-point` references — an insert
    /// into an unreferenced name errors (Drools' getEntryPoint = null).
    /// Survives reset (compile-time, like `rules`).
    entry_points: Vec<String>,
    ep_ids: HashMap<String, u32>,
    /// Per-fact entry-point id (sparse; anything not tagged — DEFAULT
    /// inserts, RHS inserts, synthetics — reads 0). Indexed by FactId.0;
    /// cleared on reset with the store.
    fact_eps: Vec<u32>,
    /// E1-hardening NON-TERMINATION backstop: a per-`fire_all` agenda-step
    /// counter. `next_activation`'s re-evaluation loops (TMS deferred drains,
    /// agenda scan) can cycle forever on a rare pre-existing temporal/TMS
    /// re-add shape the fire limit can't catch (`scenarios/hang-backlog/`);
    /// `spin_tick` trips past `AGENDA_SPIN_LIMIT` so the engine ERRORS instead
    /// of hanging. Never approached by legitimate sessions.
    spin_guard: u64,
    /// `AGENDA_SPIN_LIMIT`, overridable via `SEINE_SPIN_GUARD` (recon lens:
    /// a genuine cycle's verdict is limit-independent, so a low limit turns
    /// an ~18s guard trip into milliseconds; the default is the backstop).
    spin_limit: u64,
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
    /// D-108: collectList/collectSet result VALUES (scalar elements;
    /// set results stored pre-sorted by canonical order).
    collect_scalar_vals: HashMap<FactId, Vec<Value>>,
    /// Global activation sequence (D-043 tie order).
    act_seq: u64,
    /// Compiled DRL queries, evaluated on demand (Phase Q0, D-050).
    queries: Vec<crate::queries::CompiledQuery>,
    /// Hidden per-query row types for ?query CEs (D-056), aligned with
    /// `queries`.
    qrow_tids: Vec<TypeId>,
    /// D-108: hidden per-pattern groupby row types ([res, key]).
    gbrow_tids: Vec<TypeId>,
    /// D-108: per-node groupby groups (leading-position only), keyed by
    /// the canonicalized key value.
    gb_state: HashMap<usize, HashMap<String, GbGroup>>,
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
    /// D-107: per-site ?query-CE child rows, keyed by the calling left
    /// tuple — the leftDel/leftUpd arms retract them (caller-side
    /// churn = fresh re-pull, qm8/qm9/qm10 pins).
    qce_children: HashMap<(usize, usize), HashMap<Tup, Vec<FactId>>>,
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
        schemas.push(TypeSchema { name: INITIAL_FACT.into(), fields: Vec::new(), nullable: 0 });
        schemas.push(TypeSchema {
            name: ACC_LONG.into(),
            fields: vec![("value".into(), FieldType::I64)],
            nullable: 0,
        });
        schemas.push(TypeSchema {
            name: ACC_DOUBLE.into(),
            fields: vec![("value".into(), FieldType::F64)],
            nullable: 0,
        });
        schemas.push(TypeSchema { name: ACC_COLLECTION.into(), fields: Vec::new(), nullable: 0 });
        schemas.push(TypeSchema {
            name: ACC_SETCOLLECTION.into(),
            fields: Vec::new(),
            nullable: 0,
        });
        // D-098: decimal accumulate results. The column stores per-row
        // (unscaled, scale) exactly as computed; the declared (p, s)
        // here is nominal (this insert path bypasses coerce).
        schemas.push(TypeSchema {
            name: ACC_DECIMAL.into(),
            fields: vec![("value".into(), FieldType::Dec { p: 38, s: 0 })],
            nullable: 0,
        });
        Ok(Engine {
            clock_ms: 0,
            event_specs: std::collections::HashMap::new(),
            explicit_expiry: std::collections::HashSet::new(),
            temporal_ub: std::collections::HashMap::new(),
            never_inferred: std::collections::HashSet::new(),
            deadlines: std::collections::BTreeMap::new(),
            pending_expirations: Vec::new(),
            window_deadlines: std::collections::BTreeMap::new(),
            acc_pending: Vec::new(),
            fire_deadlines: std::collections::BTreeMap::new(),
            fire_seq: 0,
            acc_nodes: Vec::new(),
            in_expiration_drain: false,
            in_stream_flush: false,
            in_fire_loop: false,
            pn_seq: 0,
            px_explicit_victim: None,
            pn_churn_ctx: false,
            fire_no: 0,
            flush_trigger_tid: None,
            tj_trigger: None,
            tj_entered: Vec::new(),
            ever_linked: Vec::new(),
            stage_seq: 0,
            focus_stack: Vec::new(),
            focus_changed: false,
            firing_stage_floor: 0,
            store: FactStore::new(schemas),
            rules: Vec::new(),
            entry_points: vec!["DEFAULT".to_string()],
            ep_ids: HashMap::new(),
            fact_eps: Vec::new(),
            spin_guard: 0,
            spin_limit: std::env::var("SEINE_SPIN_GUARD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50_000_000),
            rule_parents: Vec::new(),
            rule_order: Vec::new(),
            lias: Vec::new(),
            trie: Vec::new(),
            nets: Vec::new(),
            lists_built: false,
            init_fact: None,
            collect_vals: HashMap::new(),
            collect_scalar_vals: HashMap::new(),
            act_seq: 0,
            queries: Vec::new(),
            qrow_tids: Vec::new(),
            gbrow_tids: Vec::new(),
            gb_state: HashMap::new(),
            query_mem: crate::queries::QueryMem::default(),
            query_pending: Vec::new(),
            query_armed: Vec::new(),
            pending_err: None,
            qce_children: HashMap::new(),
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
        self.qce_children.clear();
        self.gb_state.clear();
            self.query_armed = vec![false; self.queries.len()];
            // Hidden row types for ?query CEs (D-056): fields = params.
            for q in &self.queries {
                let tid = self.store.add_schema(TypeSchema {
                    name: format!("{QROW_PREFIX}{}", q.name),
                    fields: q.params_view().to_vec(),
                    nullable: 0,
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
        let qce_rules: Vec<&str> = self
            .rules
            .iter()
            .filter(|r| r.patterns.iter().any(|p| p.qce.is_some()))
            .map(|r| r.def.name.as_str())
            .collect();
        let has_qce = !qce_rules.is_empty();
        let mutating: Vec<&str> = self
            .rules
            .iter()
            .filter(|r| {
                r.actions.iter().any(|a| {
                    matches!(
                        a,
                        CompiledAction::Set { .. }
                            | CompiledAction::Update { .. }
                            | CompiledAction::Delete { .. }
                    )
                })
            })
            .map(|r| r.def.name.as_str())
            .collect();
        // D-106: setFocus to a group NO rule declares is a Drools
        // runtime NPE (murky ConsequenceException) — walled at compile
        // per the D-076 pattern (fail fast, name the fix).
        {
            let declared: HashSet<&str> = self
                .rules
                .iter()
                .filter_map(|r| r.def.agenda_group.as_deref())
                .collect();
            for r in &self.rules {
                for a in &r.actions {
                    if let CompiledAction::SetFocus { group } = a {
                        if !declared.contains(group.as_str()) {
                            return Err(EngineError(format!(
                                "rule {}: setFocus({group:?}) targets a group no rule declares —                                  Drools NPEs at runtime on this; declare `agenda-group {group:?}`                                  on at least one rule (D-106)",
                                r.def.name
                            )));
                        }
                    }
                }
            }
        }
        // D-107: the D-057 qce x mutation wall LIFTED (pull-at-
        // activation, qm4 pin: RHS updates do not re-pull).
        let _ = (&qce_rules, &mutating);
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
            // D-107: the D-076/D-057 qce x insertLogical wall LIFTED
            // (qm5 pin: TMS retraction composes with the pull).
            let _ = has_qce;
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
                        .find(|p| p.tpos == Some(pos) && p.sub != SubRole::Inner)
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
        // CEP E2 item A (D-109): now that every rule's temporal reach is
        // known, fill inferred @expires for un-annotated event types.
        self.infer_event_expiry();
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
                // P1c/D-089: the parent BEFORE the open group's subnet
                // branch — the outer counting node's LEFT input forks
                // there. The subnetwork attaches first (Drools build
                // order; sn_c3 R1 pins the resulting sink order).
                let mut fork_parent: Option<Option<usize>> = None;
                for j in 1..k {
                    prefix.push_str("||");
                    prefix.push_str(&keys[ri][j]);
                    let role = self.rules[ri].patterns[j].sub;
                    if role == SubRole::Inner && fork_parent.is_none() {
                        fork_parent = Some(parent);
                    }
                    let nid = match trie_index.get(&prefix) {
                        Some(&nid) => nid,
                        None => {
                            let pat = &self.rules[ri].patterns[j];
                            let kind = if pat.qce.is_some() {
                                phreak::Kind::Query
                            } else if pat.acc.is_some() {
                                phreak::Kind::Acc
                            } else if matches!(pat.sub, SubRole::Outer { .. }) {
                                match pat.ce {
                                    CeKind::Not => phreak::Kind::SubnetNot,
                                    _ => phreak::Kind::SubnetExists,
                                }
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
                                node: phreak::Node::new_ex(
                                    pat.pindex,
                                    kind,
                                    pat.cmps.iter().any(|c| matches!(c.test, Test::Temporal { .. })),
                                ),
                                env: (ri, j),
                                active: HashSet::new(),
                                clock_removed: HashSet::new(),
                                win_ring: Vec::new(),
                                win_admit_pending: Vec::new(),
                                pulse: false,
                                s0_in,
                                sinks: Vec::new(),
                                acc: HashMap::new(),
                                acc_by_right: HashMap::new(),
                                collect_left_gate: None,
                                sn_right: Staged::default(),
                                sn_matches: HashMap::new(),
                                sn_lefts: Vec::new(),
                                sn_has_child: HashSet::new(),
                                sn_plen: match pat.sub {
                                    SubRole::Outer { plen, .. } => plen,
                                    _ => 0,
                                },
                            });
                            let nid = self.trie.len() - 1;
                            trie_index.insert(prefix.clone(), nid);
                            if let SubRole::Outer { .. } = role {
                                // left input = the fork; rights = the
                                // subnet tip through the RIA hop
                                match fork_parent.unwrap_or(None) {
                                    None => self.lias[lia].children.push(nid),
                                    Some(p) => self.trie[p].sinks.push(Sink::Node(nid)),
                                }
                                self.trie[parent.unwrap()].sinks.push(Sink::Ria(nid));
                            } else {
                                match parent {
                                    None => self.lias[lia].children.push(nid),
                                    Some(p) => self.trie[p].sinks.push(Sink::Node(nid)),
                                }
                            }
                            nid
                        }
                    };
                    if matches!(role, SubRole::Outer { .. }) {
                        fork_parent = None;
                    }
                    path.push(nid);
                    parent = Some(nid);
                }
                self.trie[parent.unwrap()].sinks.push(Sink::Term(ri));
            }
            // D-151/D-152: build the mechanical flush shadow for a gated
            // existential rule whose CE and P patterns are BARE (no
            // constraints, no bindings — the validated surface; the inferred
            // update mask is empty exactly then): `not` ⇒ BfShadow (D-151),
            // `exists` ⇒ ExShadow (D-152). STATIC exclusions keep each
            // shadow inside its validated regime (each ⇒ no shadow ⇒ the
            // pick is plain FIFO):
            //   - the P type must be a NON-event (an event P would expire
            //     outside the shadow's op stream);
            //   - distinct blocker/P classification (type or entry point);
            //   - no rule's RHS inserts or mutates either gated type (RHS
            //     ops are mid-fire propagation entries the spec never
            //     validated — the D-140 in_cycle guard covers the not
            //     cycle, the exclusion covers the residue);
            //   - no windowed accumulate over either gated type (window
            //     evictions delete outside the external-op stream).
            let (bf, ex) = {
                let r = &self.rules[ri];
                match r.not_order_pos {
                    Some(_) => {
                        let n = &r.patterns[1];
                        let p = &r.patterns[2];
                        let bare = n.cmps.is_empty()
                            && n.bind_fields == 0
                            && p.cmps.is_empty()
                            && p.bind_fields == 0;
                        let distinct =
                            n.type_id != p.type_id || n.entry_point != p.entry_point;
                        let p_plain = !self.event_specs.contains_key(&p.type_id);
                        let gated = [n.type_id, p.type_id];
                        let rhs_touches = self.rules.iter().any(|rr| {
                            rr.actions.iter().any(|a| match a {
                                CompiledAction::Insert { type_id, .. }
                                | CompiledAction::InsertLogical { type_id, .. } => {
                                    gated.contains(type_id)
                                }
                                CompiledAction::Set { pos, .. }
                                | CompiledAction::Update { pos }
                                | CompiledAction::Delete { pos } => rr
                                    .patterns
                                    .iter()
                                    .find(|q| q.tpos == Some(*pos))
                                    .is_some_and(|q| gated.contains(&q.type_id)),
                                CompiledAction::SetFocus { .. } => false,
                            })
                        });
                        let windowed = self.rules.iter().any(|rr| {
                            rr.patterns.iter().any(|q| {
                                q.acc
                                    .as_ref()
                                    .is_some_and(|a| a.window_time.is_some() || a.window_len.is_some())
                                    && gated.contains(&q.type_id)
                            })
                        });
                        // D-166: the `windowed` exclusion (a D-150 scope cut)
                        // is LIFTED for the event-not shadow — cf933x385's cell
                        // needs the shadow (and its update hoist) under a
                        // window:time accumulate over the blocker type. The
                        // event-not populations + notpop-FULL gate the lift.
                        let _ = windowed;
                        if bare && distinct && p_plain && !rhs_touches {
                            if r.order_exists {
                                (
                                    None,
                                    Some(ExShadow::new(
                                        n.type_id,
                                        n.entry_point,
                                        p.type_id,
                                        p.entry_point,
                                    )),
                                )
                            } else {
                                (
                                    Some(BfShadow::new(
                                        n.type_id,
                                        n.entry_point,
                                        p.type_id,
                                        p.entry_point,
                                    )),
                                    None,
                                )
                            }
                        } else {
                            (None, None)
                        }
                    }
                    _ => (None, None),
                }
            };
            // D-158: the PLAIN-blocker shadow. Static exclusions mirror the
            // BfShadow set with ONE deliberate difference: RHS Insert /
            // InsertLogical of the BLOCKER type is ALLOWED — logical D's
            // justified by expiring events ARE the validated mechanism (the
            // shadow sees them as WM events, TMS retracts included; shared
            // justifications are WM-invisible on both sides). Excluded ⇒ no
            // shadow ⇒ the pick stays plain FIFO:
            //   - non-bare patterns (constraints or bindings on N/P);
            //   - same blocker/P classification;
            //   - an event-typed P (expires outside the shadow's stream);
            //   - any RHS Insert/InsertLogical of the P type, or any RHS
            //     Set/Update/Delete touching EITHER gated type (mid-fire
            //     mutations the spec never validated);
            //   - a windowed accumulate over either gated type.
            let pn = {
                let r = &self.rules[ri];
                match r.pn_pos {
                    Some(_) => {
                        let n = &r.patterns[1];
                        let p = &r.patterns[2];
                        let bare = n.cmps.is_empty()
                            && n.bind_fields == 0
                            && p.cmps.is_empty()
                            && p.bind_fields == 0;
                        let distinct =
                            n.type_id != p.type_id || n.entry_point != p.entry_point;
                        let p_plain = !self.event_specs.contains_key(&p.type_id);
                        let gated = [n.type_id, p.type_id];
                        let rhs_bad = self.rules.iter().any(|rr| {
                            rr.actions.iter().any(|a| match a {
                                CompiledAction::Insert { type_id, .. }
                                | CompiledAction::InsertLogical { type_id, .. } => {
                                    *type_id == p.type_id
                                }
                                CompiledAction::Set { pos, .. }
                                | CompiledAction::Update { pos }
                                | CompiledAction::Delete { pos } => rr
                                    .patterns
                                    .iter()
                                    .find(|q| q.tpos == Some(*pos))
                                    .is_some_and(|q| gated.contains(&q.type_id)),
                                CompiledAction::SetFocus { .. } => false,
                            })
                        });
                        let windowed = self.rules.iter().any(|rr| {
                            rr.patterns.iter().any(|q| {
                                q.acc
                                    .as_ref()
                                    .is_some_and(|a| a.window_time.is_some() || a.window_len.is_some())
                                    && gated.contains(&q.type_id)
                            })
                        });
                        if bare && distinct && p_plain && !rhs_bad && !windowed {
                            Some(PnShadow::new(
                                n.type_id,
                                n.entry_point,
                                p.type_id,
                                p.entry_point,
                            ))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            };
            // D-162: the plain-EXISTS shadow's static exclusions — the pn
            // set, except the WITNESS may carry ALPHA-only constraints (the
            // cons drive `D(tag=="x")`: the shadow re-evaluates the alpha
            // per witness op via alpha_passes_fields) and RHS
            // Insert/InsertLogical of the WITNESS type is allowed (the
            // logical J-drive; shared justifications are WM-visible at the
            // hook). The P pattern must stay bare (mask-empty reorder
            // semantics).
            let px = {
                let r = &self.rules[ri];
                match r.px_pos {
                    Some(_) => {
                        let d = &r.patterns[1];
                        let p = &r.patterns[2];
                        let d_alpha_only = d.bind_fields == 0
                            && d.cmps.iter().all(|c| match &c.test {
                                Test::IsNull { .. } | Test::Unknown => true,
                                Test::Matches(_) | Test::Contains(_) => true,
                                Test::Cmp { rhs: Src::Lit(_), .. } => true,
                                Test::Cmp { rhs: Src::Field(ti, _), .. } => {
                                    Some(*ti) == d.tpos
                                }
                                Test::Group { cross_var, .. } => !*cross_var,
                                _ => false,
                            });
                        let p_bare = p.cmps.is_empty() && p.bind_fields == 0;
                        let distinct =
                            d.type_id != p.type_id || d.entry_point != p.entry_point;
                        let p_plain = !self.event_specs.contains_key(&p.type_id);
                        let gated = [d.type_id, p.type_id];
                        let rhs_bad = self.rules.iter().any(|rr| {
                            rr.actions.iter().any(|a| match a {
                                CompiledAction::Insert { type_id, .. }
                                | CompiledAction::InsertLogical { type_id, .. } => {
                                    *type_id == p.type_id
                                }
                                CompiledAction::Set { pos, .. }
                                | CompiledAction::Update { pos }
                                | CompiledAction::Delete { pos } => rr
                                    .patterns
                                    .iter()
                                    .find(|q| q.tpos == Some(*pos))
                                    .is_some_and(|q| gated.contains(&q.type_id)),
                                CompiledAction::SetFocus { .. } => false,
                            })
                        });
                        let windowed = self.rules.iter().any(|rr| {
                            rr.patterns.iter().any(|q| {
                                q.acc
                                    .as_ref()
                                    .is_some_and(|a| a.window_time.is_some() || a.window_len.is_some())
                                    && gated.contains(&q.type_id)
                            })
                        });
                        if d_alpha_only && p_bare && distinct && p_plain && !rhs_bad && !windowed
                        {
                            Some(PxShadow::new(
                                d.type_id,
                                d.entry_point,
                                p.type_id,
                                p.entry_point,
                            ))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            };
            self.nets.push(RuleNet {
                s0: vec![Staged::default()],
                lia,
                path,
                term_pending: Staged::default(),
                peer_live: HashSet::new(),
                act_movable: HashMap::new(),
                queue: std::collections::VecDeque::new(),
                item_sal: 0,
                act_num: HashMap::new(),
                queued: false,
                ever_linked: false,
                dirty: false,
                dirty_stamp: 0,
                bf,
                ex,
                pn,
                px,
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
        // D-110/D-112: index EVERY accumulate node (trie indices are only
        // valid after this build, and change on reset) — windowed ones
        // schedule evictions, all of them take eager expiration removals.
        self.precompute_acc_nodes();
    }

    /// D-110/D-112: collect `(trie-node idx, source event type, window
    /// size N | None)` for every accumulate node. The node's rights are
    /// the source events. Windowed nodes schedule an eviction at each
    /// event's `ts+N`; every node takes a scoped EAGER right-delete when a
    /// feeding event expires (so the count-drop precedes the fire's
    /// inserts and fires by salience — the accumulate-eager mechanism).
    fn precompute_acc_nodes(&mut self) {
        self.acc_nodes.clear();
        for ni in 0..self.trie.len() {
            if self.trie[ni].node.kind != phreak::Kind::Acc {
                continue;
            }
            let (ri, pos) = self.trie[ni].env;
            if let Some(acc) = self.rules[ri].patterns[pos].acc.as_ref() {
                let tid = self.rules[ri].patterns[pos].type_id;
                self.acc_nodes.push((ni, tid, acc.window_time, acc.window_len));
            }
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
                    CmpRhs::Lit(Literal::Null) => {
                        let negated = match op {
                            CmpOp::Eq => false,
                            CmpOp::Ne => true,
                            other => {
                                return Err(err(format!(
                                    "only ==/!= accept null (IS [NOT] NULL semantics, D-097); got {other:?}"
                                )))
                            }
                        };
                        if self.store.schema(type_id).nullable >> fi & 1 != 1 {
                            return Err(err(format!(
                                "{tname} field is not nullable — null tests need a nullable field (D-097)"
                            )));
                        }
                        let _ = write!(key, "g{fi}isnull{negated}");
                        Ok(GExpr::IsNull { field_idx: fi, negated })
                    }
                    CmpRhs::Lit(l) => {
                        let mut v = lit_value(l);
                        let mut ft = lit_type(l);
                        if matches!(lhs_ft, FieldType::Dec { .. }) {
                            v = lit_for_dec(&v).ok_or_else(|| {
                                err(format!("group literal {l:?} is not an exact decimal (D-098)"))
                            })?;
                            ft = v.type_of();
                        }
                        check_cmp_types(rname, lhs_ft, *op, ft)?;
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
        if let SubRole::Outer { len, plen } = p.sub {
            // the inner chain is already part of the trie prefix string;
            // the outer key adds the CE kind + shape (D-089)
            return format!("SN|{:?}|{len}|{plen}", p.ce);
        }
        // CEP E2 item D: entry_point is identity-significant — two patterns
        // share a node only if same type AND same entry point (all-DEFAULT
        // corpus gets a uniform `e0`, so the grouping is unchanged).
        let mut s = format!("{}|{:?}|b{}|e{}", p.type_id.0, p.ce, p.bind_fields, p.entry_point);
        for c in &p.cmps {
            let _ = write!(s, ";{}", c.field_idx);
            match &c.test {
                Test::Temporal { op, params, anchor, self_dur_fi, anchor_dur_fi } => {
                    match op {
                        // after/before keep the EXACT E1 key string so
                        // node-sharing identity is byte-identical
                        // (params[0]=lo, params[1]=hi).
                        AllenOp::After | AllenOp::Before => {
                            let after = *op == AllenOp::After;
                            let _ = write!(
                                s, "tmp{after}{}:{}@{}.{}",
                                params[0], params[1], anchor.0, anchor.1
                            );
                        }
                        // Allen ops fold op + params + BOTH @duration field
                        // indices into the node identity — two different
                        // relations/tolerances (or interval shapes) over the
                        // same binding must NOT share a node (D-113 lesson: a
                        // missing key field silently mis-shares).
                        _ => {
                            let _ = write!(
                                s, "aln{op:?}{params:?}@{}.{}d{self_dur_fi:?}/{anchor_dur_fi:?}",
                                anchor.0, anchor.1
                            );
                        }
                    }
                }
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
                Test::IsNull { negated } => {
                    let _ = write!(s, "isnull{negated}");
                }
                Test::Unknown => {
                    let _ = write!(s, "unk3");
                }
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
            // D-112: the WINDOW spec and the groupby key are part of the
            // accumulate's node identity — two accumulates over the same
            // source binding but a different `over window:time(N)` (or a
            // different groupby) must NOT share the node (share_same:
            // a windowed W2 and a plain W3 over `E1($t:ts)` shared and both
            // reported the windowed value). Absent before, since D-111
            // added `window_time` to the spec but not to this key.
            let _ = write!(
                s,
                "|acc{:?}:{}:{:?}:w{:?}:g{:?}",
                acc.func,
                acc.arg_name.as_deref().unwrap_or(""),
                acc.arg_field,
                acc.window_time,
                acc.key_field,
            );
            let _ = write!(
                s,
                ":wl{:?}",
                acc.window_len,
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
                        Test::Temporal { .. } => {
                            // beta-only; never an alpha chain member
                        }
                        Test::IsNull { negated } => {
                            let _ = (negated,);
                            // chain member only (like InList, op_i7)
                        }
                        Test::Unknown => {}
                        Test::Cmp { op, rhs: Src::Lit(v) } => {
                            if *op == CmpOp::Eq
                                && !v.is_null()
                                && !matches!(v, Value::Dec { .. })
                            {
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

    fn compile_rule(&mut self, def: RuleDef) -> Result<CompiledRule, EngineError> {
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
        // D-109: per-rule temporal STP edges (u,v,ub) = upperBound of
        // time_v−time_u, plus a position→type map; Floyd-Warshall-closed
        // at compile end to fold each event type's inferred @expires reach
        // into `temporal_ub` (consumed by infer_event_expiry). Lower bounds
        // travel as reverse edges, so one bound per edge suffices (STP).
        let mut temporal_edges: Vec<(usize, usize, i64)> = Vec::new();
        let mut temporal_pos_type: HashMap<usize, TypeId> = HashMap::new();

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
                temporal_pos: Some(0),
                beta: false,
                pindex: phreak::Index::None,
                index_ci: None,
                bind_fields: 0,
                acc: None,
                qce: None,
                sub: SubRole::None,
                entry_point: 0,
            });
            tuple_len = 1;
        }

        // P1c/D-089 flattening: a group CE lowers to its inner patterns
        // (the subnetwork branch — tuple slots BEYOND the main prefix,
        // bindings scoped to the group) followed by the Outer counting
        // pseudo-pattern. `sub_off` counts positive inners of the OPEN
        // group; `group_binds` collects its scoped binding names.
        let flat: Vec<(&drl::Pattern, SubRole)> = def
            .patterns
            .iter()
            .flat_map(|p| -> Vec<(&drl::Pattern, SubRole)> {
                match &p.group {
                    Some(inner) => inner
                        .iter()
                        .map(|ip| (ip, SubRole::Inner))
                        .chain(std::iter::once((p, SubRole::Outer { len: inner.len(), plen: 0 })))
                        .collect(),
                    None => vec![(p, SubRole::None)],
                }
            })
            .collect();
        let mut sub_off = 0usize;
        // Phantom temporal-matrix positions for `not` (D-132) and `exists`
        // (D-135) patterns: a high base keeps them clear of real tuple positions
        // (0..tuple_len + subnet slots), which stay small.
        let mut phantom_pos = 1usize << 20;
        let mut group_binds: Vec<String> = Vec::new();
        for (p, role) in flat {
            if let SubRole::Outer { len, .. } = role {
                // The outer counting node: no alpha input (rights arrive
                // through the RIA hop), no constraints, no tuple slot.
                // Scoped bindings leave scope here — later references
                // fail with "unknown binding", the faithful Drools error
                // (sn_g1).
                for b in group_binds.drain(..) {
                    fact_binds.remove(&b);
                    field_binds.remove(&b);
                }
                sub_off = 0;
                let tid = self
                    .store
                    .type_id(INITIAL_FACT)
                    .ok_or_else(|| err("internal: InitialFact type missing".into()))?;
                patterns.push(CompiledPattern {
                    type_id: tid,
                    cmps: Vec::new(),
                    listen_mask: 0,
                    ce: p.ce,
                    tpos: None,
                    temporal_pos: None,
                    beta: false,
                    pindex: phreak::Index::None,
                    index_ci: None,
                    bind_fields: 0,
                    acc: None,
                    qce: None,
                    sub: SubRole::Outer { len, plen: tuple_len },
                    entry_point: 0,
                });
                continue;
            }
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
                    temporal_pos: Some(t),
                    beta: false,
                    pindex: phreak::Index::None,
                    index_ci: None,
                    bind_fields: 0,
                    acc: None,
                    qce: Some(CompiledQce { qi, args, row_tid, bound_mask }),
                    sub: SubRole::None,
                    entry_point: 0,
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
                if role == SubRole::Inner {
                    // subnet-branch slot: extends the main prefix without
                    // claiming a rule-tuple position (the outer node
                    // truncates back to the prefix, D-089)
                    let t = tuple_len + sub_off;
                    sub_off += 1;
                    Some(t)
                } else {
                    let t = tuple_len;
                    tuple_len += 1;
                    Some(t)
                }
            } else {
                None
            };
            // Temporal-matrix position (D-132/D-135): a `not` OR `exists` gets a
            // phantom so its after/before @expires-inference edges record (they
            // contribute no tuple element, so they can't reuse a tuple slot);
            // positive patterns reuse their tuple slot. D-135: exists inference
            // is FACTS-only (reaping) — INVISIBLE to firings (fires when a
            // partner is present, retraction unobservable) — so this is the
            // whole exists slab; no firing scheduler.
            let self_temporal_pos: Option<usize> = if matches!(p.ce, CeKind::Not | CeKind::Exists) {
                let pp = phantom_pos;
                phantom_pos += 1;
                Some(pp)
            } else {
                tpos
            };
            if let Some(b) = &p.binding {
                let t = tpos.ok_or_else(|| err("binding on a CE pattern".into()))?;
                if fact_binds.insert(b.clone(), (t, type_id)).is_some() {
                    return Err(err(format!("duplicate binding {b}")));
                }
                if role == SubRole::Inner {
                    group_binds.push(b.clone());
                }
            }
            let mut cmps = Vec::new();
            let mut listen_mask = 0u64;
            let mut bind_fields = 0u64;
            for c in &p.constraints {
                match c {
                    Constraint::Temporal { op, params, var } => {
                        // FENCE LIFTED (D-134): temporal `not` is now PORTED.
                        // §3A (arc-B REAPING) records the not's own @expires
                        // inference via the phantom `self_temporal_pos` edges
                        // below (D-132/D-133); §3B (arc-A FIRING DEFERRAL) is
                        // the `fire_deadlines` window-close scheduler in
                        // do_existential_node / drain_pending_fires. This was
                        // the last CEP-E2 fence. See
                        // docs/not-temporal-port-mechanism.md.
                        // CEP E1/E2 (D-101/D-118): both sides must be DECLARED
                        // events; the test reads each side's ts and, for
                        // intervals, its @duration end (item E).
                        let own_spec = *self.event_specs.get(&type_id).ok_or_else(|| {
                            err(format!(
                                "{}: temporal constraints need a declared event type",
                                p.type_name
                            ))
                        })?;
                        let own_fi = own_spec.ts_fi;
                        let (apos, atid) = *fact_binds.get(var).ok_or_else(|| {
                            err(format!("unknown fact binding {var} (temporal anchor)"))
                        })?;
                        let anchor_spec = *self.event_specs.get(&atid).ok_or_else(|| {
                            err(format!("temporal anchor {var} is not a declared event type"))
                        })?;
                        let anchor_fi = anchor_spec.ts_fi;
                        // D-109 @expires INFERENCE — after/before ONLY (D-120
                        // FENCE). Record directed STP edges for the per-rule
                        // TemporalDependencyMatrix (closed by Floyd-Warshall at
                        // compile end so multi-hop chains compose — trans_e1:
                        // E1→E2→E3 gives E1 the SUMMED reach 150, not the
                        // pairwise 100). Edge (u,v,ub) means ub(time_v − time_u).
                        // `after`: t_self−t_anchor ∈ [lo,hi] ⇒ ub(anchor→self)
                        // =hi, ub(self→anchor)=−lo; `before` swaps self/anchor.
                        // D-164: the 11 Allen ops emit their constant interval
                        // edges too (the D-120 slab-1 fence is LIFTED — see the
                        // else-branch below).
                        if matches!(op, AllenOp::After | AllenOp::Before) {
                            // `self_temporal_pos` = the tuple slot for a positive
                            // pattern, a PHANTOM slot for a `not` (D-132) or an
                            // `exists` (D-135): both record inference edges — E0
                            // anchor gets +hi, the CE's type gets −lo→NEVER/0
                            // (after; before mirrors). The offset is invisible to
                            // firings; it only fixes the reaped `facts`.
                            if let Some(self_pos) = self_temporal_pos {
                                let (lo_ms, hi_ms) = (params[0], params[1]);
                                let (earlier, later) = if *op == AllenOp::After {
                                    (apos, self_pos)
                                } else {
                                    (self_pos, apos)
                                };
                                temporal_edges.push((earlier, later, hi_ms));
                                temporal_edges.push((later, earlier, -lo_ms));
                                temporal_pos_type.insert(self_pos, type_id);
                                temporal_pos_type.insert(apos, atid);
                            }
                        } else if let Some(self_pos) = self_temporal_pos {
                            // D-164 (the D-120 fence LIFTED): the 11 Allen ops
                            // emit their PARAM-BLIND constant interval edges —
                            // Drools' mvel EvaluatorDefinitions return fixed
                            // getInterval() bounds regardless of dev/min/max
                            // params (oracle-verified, 124-cell reach ladder,
                            // probes_pending/cep/e_allen/gen_allen_ladder.py):
                            //   coincides/starts/startedby      [0, 0]
                            //   meets/overlappedby/finishes     [0, MAX]
                            //   metby/overlaps/includes/
                            //     finishedby                    [MIN, 0]
                            //   during                          [1, MAX]
                            // Edge (anchor→self)=H iff H<MAX; (self→anchor)=−L
                            // iff L>MIN; the existing closure derives the
                            // classification (only `during` leaks both sides —
                            // its −1 backward row-max is the after[lo>0]-style
                            // NEVER) and the deadline stays endTS + reach + 1.
                            let (lo, hi): (Option<i64>, Option<i64>) = match op {
                                AllenOp::Coincides
                                | AllenOp::Starts
                                | AllenOp::StartedBy => (Some(0), Some(0)),
                                AllenOp::Meets
                                | AllenOp::OverlappedBy
                                | AllenOp::Finishes => (Some(0), None),
                                AllenOp::MetBy
                                | AllenOp::Overlaps
                                | AllenOp::Includes
                                | AllenOp::FinishedBy => (None, Some(0)),
                                AllenOp::During => (Some(1), None),
                                AllenOp::After | AllenOp::Before => unreachable!(),
                            };
                            if let Some(h) = hi {
                                temporal_edges.push((apos, self_pos, h));
                            }
                            if let Some(l) = lo {
                                temporal_edges.push((self_pos, apos, -l));
                            }
                            temporal_pos_type.insert(self_pos, type_id);
                            temporal_pos_type.insert(apos, atid);
                        }
                        listen_mask |= 1 << own_fi;
                        cmps.push(CompiledCmp {
                            field_idx: own_fi,
                            test: Test::Temporal {
                                op: *op,
                                params: params.clone(),
                                anchor: (apos, anchor_fi),
                                self_dur_fi: own_spec.dur_fi,
                                anchor_dur_fi: anchor_spec.dur_fi,
                            },
                            rhs_var: Some(var.clone()),
                        });
                    }
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
                        if role == SubRole::Inner {
                            group_binds.push(var.clone());
                        }
                    }
                    Constraint::Cmp { field, op, rhs } => {
                        let fi = self
                            .store
                            .field_index(type_id, field)
                            .ok_or_else(|| err(format!("{} has no field {field}", p.type_name)))?;
                        listen_mask |= 1 << fi;
                        let lhs_ft = self.store.field_type(type_id, fi);
                        // D-097 surface mapping: `field == null` /
                        // `field != null` are IS [NOT] NULL — definite
                        // tests; any other op with null is an error.
                        if matches!(rhs, CmpRhs::Lit(Literal::Null)) {
                            let negated = match op {
                                CmpOp::Eq => false,
                                CmpOp::Ne => true,
                                other => {
                                    return Err(err(format!(
                                        "only ==/!= accept null (IS [NOT] NULL semantics, D-097); got {other:?}"
                                    )))
                                }
                            };
                            if self.store.schema(type_id).nullable >> fi & 1 != 1 {
                                return Err(err(format!(
                                    "{}.{field} is not nullable — null tests need a nullable field (D-097)",
                                    p.type_name
                                )));
                            }
                            cmps.push(CompiledCmp {
                                field_idx: fi,
                                test: Test::IsNull { negated },
                                rhs_var: None,
                            });
                            continue;
                        }
                        let (src, rhs_ft, rhs_var) = match rhs {
                            CmpRhs::Lit(l) => {
                                let mut v = lit_value(l);
                                let mut ft = lit_type(l);
                                if matches!(lhs_ft, FieldType::Dec { .. }) {
                                    v = lit_for_dec(&v).ok_or_else(|| {
                                        err(format!(
                                            "{}.{field}: literal {l:?} is not an exact decimal (D-098)",
                                            p.type_name
                                        ))
                                    })?;
                                    ft = v.type_of();
                                }
                                (Src::Lit(v), ft, None)
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
                            if matches!(l, Literal::Null) {
                                vals.push(Value::Null); // 3VL member (pin C)
                                continue;
                            }
                            if matches!(lhs_ft, FieldType::Dec { .. }) {
                                let v = lit_for_dec(&lit_value(l)).ok_or_else(|| {
                                    err(format!(
                                        "in-list literal {l:?} is not an exact decimal (D-098)"
                                    ))
                                })?;
                                vals.push(v);
                                continue;
                            }
                            check_cmp_types(&rname, lhs_ft, CmpOp::Eq, lit_type(l))?;
                            vals.push(lit_value(l));
                        }
                        if *negated {
                            for v in vals {
                                if v.is_null() {
                                    // `x != <null member>` is UNKNOWN for
                                    // every x — the not-in trap (D-097)
                                    cmps.push(CompiledCmp {
                                        field_idx: fi,
                                        test: Test::Unknown,
                                        rhs_var: None,
                                    });
                                    continue;
                                }
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
                                    if v.is_null() {
                                        return GExpr::Unknown;
                                    }
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
                    let numeric =
                        matches!(arg_ft, FieldType::I64 | FieldType::F64 | FieldType::Dec { .. });
                    if spec.arg.is_some()
                        && !numeric
                        && !matches!(
                            spec.func,
                            AccFunc::Count | AccFunc::CollectList | AccFunc::CollectSet
                        )
                    {
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
                        // pin J: AVG over decimal is DOUBLE
                        AccFunc::Average => (ACC_DOUBLE, FieldType::F64),
                        AccFunc::Sum | AccFunc::Min | AccFunc::Max => match arg_ft {
                            FieldType::I64 => (ACC_LONG, FieldType::I64),
                            // sum widens to DECIMAL(38,s); min/max preserve (pin J)
                            FieldType::Dec { s, .. } => {
                                (ACC_DECIMAL, FieldType::Dec { p: 38, s })
                            }
                            _ => (ACC_DOUBLE, FieldType::F64),
                        },
                        AccFunc::Collect => (ACC_COLLECTION, FieldType::I64),
                        AccFunc::CollectList => (ACC_COLLECTION, FieldType::I64),
                        AccFunc::CollectSet => (ACC_SETCOLLECTION, FieldType::I64),
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
                    let (result_tid, key_field) = if let Some(kv) = &spec.group_key {
                        let init_tid = self.store.type_id(INITIAL_FACT).unwrap();
                        if !patterns.iter().all(|q: &CompiledPattern| q.type_id == init_tid) {
                            return Err(err(
                                "groupby after other patterns is out of subset (leading position only, D-108)"
                                    .into(),
                            ));
                        }
                        // D-108 groupby: resolve the key binding to its
                        // source field; the result rides a PER-PATTERN
                        // hidden row type [result, key] so both bind
                        // downstream (ga10) and render as the composite
                        // (ga3 raw: QueryArgs [result, key]).
                        let kf = p
                            .constraints
                            .iter()
                            .find_map(|c| match c {
                                Constraint::Bind { var, field } if var == kv => {
                                    self.store.field_index(type_id, field)
                                }
                                _ => None,
                            })
                            .ok_or_else(|| {
                                err(format!("groupby key {kv} is not a source binding"))
                            })?;
                        let key_ft = self.store.field_type(type_id, kf);
                        let row_tid = self.store.add_schema(TypeSchema {
                            name: format!("__gbrow${}${}", rname, t),
                            fields: vec![
                                ("res".into(), result_ft),
                                ("key".into(), key_ft),
                            ],
                            nullable: 0,
                        });
                        self.gbrow_tids.push(row_tid);
                        if field_binds
                            .insert(kv.clone(), (t, 1, key_ft))
                            .is_some()
                        {
                            return Err(err(format!("duplicate binding {kv}")));
                        }
                        (row_tid, Some(kf))
                    } else {
                        (result_tid, None)
                    };
                    // D-110: `over window:time(N)` runtime — per-subtree
                    // eviction (scheduled at ts+N, drops the count while
                    // the fact survives WM-wide) + the A→B seam (window
                    // size folds N−1 into the inferred @expires). Carried
                    // through as `window_time` below.
                    Some(CompiledAcc {
                        func: spec.func,
                        arg_field,
                        arg_ft,
                        result_tid,
                        arg_name: spec.arg.clone(),
                        key_field,
                        window_time: spec.window.and_then(|w| match w {
                            drl::Window::Time(n) => Some(n),
                            drl::Window::Length(_) => None,
                        }),
                        window_len: spec.window.and_then(|w| match w {
                            drl::Window::Length(n) => Some(n),
                            drl::Window::Time(_) => None,
                        }),
                    })
                }
            };
            let beta = cmps.iter().any(|c| {
                matches!(c.test, Test::Cmp { rhs: Src::Field(..), .. })
                    || matches!(c.test, Test::Group { cross_var: true, .. })
                    || matches!(c.test, Test::Temporal { .. })
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
                            .rev()
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
                temporal_pos: self_temporal_pos,
                beta,
                pindex,
                index_ci,
                bind_fields,
                acc,
                qce: None,
                sub: role,
                entry_point: self.intern_ep(&p.entry_point),
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
                    let nullable_mask = schema.nullable;
                    let mut srcs: Vec<CExpr> = Vec::new();
                    for (i, (aexpr, (fname, ftype))) in
                        args.iter().zip(schema.fields.clone()).enumerate()
                    {
                        // The atom path is the pre-arithmetic contract,
                        // byte-unchanged (null-lit D-097, decimal-lit
                        // D-098, assignability).
                        if let crate::drl::RhsExpr::Atom(arg) = aexpr {
                            // D-097: a null literal arg needs a nullable
                            // target field (checked here); a null VALUE
                            // flowing through a binding into a non-nullable
                            // field errors loudly at runtime (store push).
                            if matches!(arg, RhsArg::Lit(Literal::Null)) {
                                if nullable_mask >> i & 1 != 1 {
                                    return Err(err(format!(
                                        "insert new {type_name}: null arg for non-nullable field {fname} (D-097)"
                                    )));
                                }
                                srcs.push(CExpr::Atom(Src::Lit(Value::Null)));
                                continue;
                            }
                            if let (RhsArg::Lit(l), FieldType::Dec { .. }) = (arg, ftype) {
                                if !matches!(l, Literal::Str(_)) {
                                    let v = lit_for_dec(&lit_value(l)).ok_or_else(|| {
                                        err(format!(
                                            "insert new {type_name}: arg for {fname} is not an exact decimal (D-098)"
                                        ))
                                    })?;
                                    srcs.push(CExpr::Atom(Src::Lit(v)));
                                    continue;
                                }
                            }
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
                            srcs.push(CExpr::Atom(src));
                            continue;
                        }
                        // D-283 Tier 1: a COMPUTED arg. Plain insert only —
                        // computed insertLogical is the stratified tier
                        // (D-282); modify-with-computation stays WONT
                        // (D-231).
                        if logical {
                            return Err(err(format!(
                                "insert new {type_name}: computed insertLogical args are \
                                 outside the certified subset (the stratified tier, D-282) \
                                 — compute via a plain insert"
                            )));
                        }
                        let (ce, expr_ft) = self.compile_cexpr(
                            &rname,
                            aexpr,
                            &fact_binds,
                            &field_binds,
                            &acc_opaque,
                            &def,
                            &patterns,
                        )?;
                        if !assignable(expr_ft, ftype) {
                            return Err(err(format!(
                                "insert new {type_name}: computed arg for {fname} has type \
                                 {} but the field needs {} (a double expression cannot \
                                 narrow into a long field)",
                                ft_name(expr_ft),
                                ft_name(ftype)
                            )));
                        }
                        srcs.push(ce);
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
                        // D-089 extension (Bryan's ruling): group-CE
                        // justifiers are walled — revalidation over
                        // subnetworks is unprobed.
                        if patterns.iter().any(|p| p.sub != SubRole::None) {
                            return Err(err(
                                "insertLogical from rules with not/exists GROUP CEs is out of subset (D-089/D-076)".into(),
                            ));
                        }
                        // all atoms by construction: the computed-arg wall above
                        let atoms = srcs
                            .into_iter()
                            .map(|c| match c {
                                CExpr::Atom(s) => s,
                                _ => unreachable!("computed insertLogical walled at compile"),
                            })
                            .collect();
                        actions.push(CompiledAction::InsertLogical { type_id: tid, args: atoms });
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
                    if matches!(arg, RhsArg::Lit(Literal::Null)) {
                        if self.store.schema(tid).nullable >> fi & 1 != 1 {
                            return Err(err(format!(
                                "setter {var}.{field}: null for a non-nullable field (D-097)"
                            )));
                        }
                        actions.push(CompiledAction::Set {
                            pos,
                            field_idx: fi,
                            arg: Src::Lit(Value::Null),
                        });
                        continue;
                    }
                    if let (RhsArg::Lit(l), FieldType::Dec { .. }) = (arg, ftype) {
                        if !matches!(l, Literal::Str(_)) {
                            let v = lit_for_dec(&lit_value(l)).ok_or_else(|| {
                                err(format!(
                                    "setter {var}.{field}: not an exact decimal literal (D-098)"
                                ))
                            })?;
                            actions.push(CompiledAction::Set { pos, field_idx: fi, arg: Src::Lit(v) });
                            continue;
                        }
                    }
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
                Action::SetFocus { group } => {
                    actions.push(CompiledAction::SetFocus { group: group.clone() });
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
                            let src_pat = patterns.iter().find(|p| p.tpos == Some(ti));
                            if let Some(sp) = src_pat {
                                if self.store.schema(sp.type_id).nullable >> fi & 1 == 1 {
                                    return Err(err(format!(
                                        "salience: {v} reads a nullable field — no agenda semantics for UNKNOWN (walled, D-097)"
                                    )));
                                }
                            }
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
        // D-109: any event pattern in this rule that does NOT participate
        // in a temporal constraint (a bare positive ref, or a not/exists —
        // temporal on not/exists is walled, so those are always bare)
        // forces its type to NEVER expire (Drools overwrites the OTN
        // offset to NEVER; order-independent — nb/char probes). Explicit
        // @expires is immune (infer_event_expiry skips explicit types).
        for cp in &patterns {
            if !self.event_specs.contains_key(&cp.type_id) {
                continue;
            }
            // A→B SEAM (D-110): a windowed accumulate source contributes a
            // FINITE reach (Drools `SlidingTimeWindow.getExpirationOffset`
            // = size N). Fold it as N−1 so the D-102 +1 scheduler yields
            // the pinned `ts+N` deadline (win2_seam: E gone at exactly
            // ts+N). The windowed pattern itself never adds to
            // never_inferred — but a SEPARATE bare/backward pattern on the
            // same type still can, and that NEVER overwrite dominates the
            // window (win_x_bare/win_x_back: E persists, only the subtree
            // count drops). A larger temporal reach wins via the max
            // (win3_seam_tmax=200); a smaller one loses to the window
            // (win_x_fwd50: 99 beats 50).
            if let Some(n) = cp.acc.as_ref().and_then(|a| a.window_time) {
                if n >= 1 {
                    self.temporal_ub
                        .entry(cp.type_id)
                        .and_modify(|m| *m = (*m).max(n - 1))
                        .or_insert(n - 1);
                }
                continue;
            }
            // `temporal_pos` (not `tpos`) so a temporally-constrained `not`
            // counts as participating (D-132): its phantom position is in
            // `temporal_pos_type` iff it recorded an after/before edge. A bare
            // `not`/exists (no temporal edge) still forces its type to NEVER.
            if !cp.temporal_pos.is_some_and(|tp| temporal_pos_type.contains_key(&tp)) {
                self.never_inferred.insert(cp.type_id);
            }
        }
        // D-109: close this rule's temporal graph and fold the per-type
        // inferred reach into `temporal_ub` (MAX across rules).
        self.accumulate_temporal_closure(&temporal_edges, &temporal_pos_type);
        // D-140 (item #2): detect the modeled `not <EVENT>() P()` shape —
        // compiled patterns [InitialFact, non-temporal NOT over an event type,
        // one positive P]. @role(event) membership is set at type declaration
        // (before compile), so this reads it here. A PLAIN blocker or a
        // temporal not falls through to None (plain firing order already
        // matches the oracle; temporal not-order is the fenced D-134 §6 tie).
        // D-143 (item 1b Family B): `not`. D-144 (item 1b Family B exists): also
        // `exists <EVENT>() P()` — the witness-toggle RE-FIRE order (P's fire when
        // the witness EXISTS; each satisfy transition re-fires the whole memory).
        // Both gate on a non-temporal existential over an event + a positive P.
        let is_not = patterns.len() == 3
            && patterns[1].ce == CeKind::Not
            && !patterns[1].cmps.iter().any(|c| matches!(c.test, Test::Temporal { .. }))
            && self.event_specs.contains_key(&patterns[1].type_id)
            && patterns[2].ce == CeKind::Positive;
        let is_exists = patterns.len() == 3
            && patterns[1].ce == CeKind::Exists
            && !patterns[1].cmps.iter().any(|c| matches!(c.test, Test::Temporal { .. }))
            && self.event_specs.contains_key(&patterns[1].type_id)
            && patterns[2].ce == CeKind::Positive;
        let not_order_pos = (is_not || is_exists).then(|| patterns[2].tpos).flatten();
        let order_exists = is_exists;
        // D-158: the PLAIN-blocker sibling — `not <PLAIN>() P()` in a STREAM
        // session (event types declared ⇒ the runners build STREAM). The
        // non-stream plain-not order is main-axis-certified: `pn_pos` stays
        // None there and the pick is byte-identical FIFO.
        let is_pn = patterns.len() == 3
            && patterns[1].ce == CeKind::Not
            && !patterns[1].cmps.iter().any(|c| matches!(c.test, Test::Temporal { .. }))
            && !self.event_specs.contains_key(&patterns[1].type_id)
            && patterns[2].ce == CeKind::Positive
            && !self.event_specs.is_empty();
        let pn_pos = is_pn.then(|| patterns[2].tpos).flatten();
        // D-162: the PLAIN-witness exists sibling — `exists <PLAIN>() P()` in
        // a STREAM session (the satisfy EMISSION-ORDER family, seeds
        // 5001-5006). Non-stream plain-exists order is main-axis-certified:
        // `px_pos` stays None there and the pick is byte-identical FIFO.
        let is_px = patterns.len() == 3
            && patterns[1].ce == CeKind::Exists
            && !patterns[1].cmps.iter().any(|c| matches!(c.test, Test::Temporal { .. }))
            && !self.event_specs.contains_key(&patterns[1].type_id)
            && patterns[2].ce == CeKind::Positive
            && !self.event_specs.is_empty();
        let px_pos = is_px.then(|| patterns[2].tpos).flatten();
        Ok(CompiledRule { def, patterns, actions, salience, dep_queries, not_order_pos, order_exists, pn_pos, px_pos })
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

    /// D-283 Tier 1: compile an RHS insert-arg arithmetic expression.
    /// Numeric only (i64/f64), non-nullable operands only; the result
    /// type follows Java promotion (any f64 operand -> f64). The oracle
    /// semantics are pinned in probes_pending/arith_grammar/PINS.md §A.
    #[allow(clippy::too_many_arguments)]
    fn compile_cexpr(
        &self,
        rname: &str,
        e: &crate::drl::RhsExpr,
        fact_binds: &HashMap<String, (usize, TypeId)>,
        field_binds: &HashMap<String, (usize, usize, FieldType)>,
        acc_opaque: &HashSet<String>,
        def: &RuleDef,
        patterns: &[CompiledPattern],
    ) -> Result<(CExpr, FieldType), EngineError> {
        use crate::drl::RhsExpr;
        let numeric = |ft: FieldType, what: &str| -> Result<(), EngineError> {
            if matches!(ft, FieldType::I64 | FieldType::F64) {
                Ok(())
            } else {
                Err(EngineError(format!(
                    "rule {rname}: RHS arithmetic over a {} operand ({what}) is outside \
                     the subset — arithmetic is i64/f64 only",
                    ft_name(ft)
                )))
            }
        };
        match e {
            RhsExpr::Atom(arg) => {
                if matches!(arg, RhsArg::Lit(Literal::Null)) {
                    return Err(EngineError(format!(
                        "rule {rname}: null inside RHS arithmetic — a computed value \
                         cannot be null (D-097)"
                    )));
                }
                let (src, ft) =
                    self.compile_arg(rname, arg, fact_binds, field_binds, acc_opaque, def, patterns)?;
                numeric(ft, &format!("{arg:?}"))?;
                // Nullable field operands are walled: Java would NPE
                // unboxing; fill or guard upstream instead.
                let nullable_bit = match arg {
                    RhsArg::Lit(_) => false,
                    RhsArg::Var(v) => {
                        let (pi, fi, _) = field_binds[v];
                        patterns
                            .iter()
                            .find(|p| p.tpos == Some(pi))
                            .map(|p| self.store.schema(p.type_id).nullable >> fi & 1 == 1)
                            .unwrap_or(false)
                    }
                    RhsArg::Getter { var, field } => {
                        let (_, tid) = fact_binds[var];
                        let fi = self.store.field_index(tid, field).unwrap();
                        self.store.schema(tid).nullable >> fi & 1 == 1
                    }
                };
                if nullable_bit {
                    return Err(EngineError(format!(
                        "rule {rname}: RHS arithmetic over a NULLABLE field operand \
                         ({arg:?}) is outside the subset — guard or fill the null \
                         upstream (D-097)"
                    )));
                }
                Ok((CExpr::Atom(src), ft))
            }
            RhsExpr::Neg(a) => {
                let (ca, ft) =
                    self.compile_cexpr(rname, a, fact_binds, field_binds, acc_opaque, def, patterns)?;
                Ok((CExpr::Neg(Box::new(ca)), ft))
            }
            RhsExpr::Bin(op, a, b) => {
                let (ca, fa) =
                    self.compile_cexpr(rname, a, fact_binds, field_binds, acc_opaque, def, patterns)?;
                let (cb, fb) =
                    self.compile_cexpr(rname, b, fact_binds, field_binds, acc_opaque, def, patterns)?;
                let ft = if fa == FieldType::F64 || fb == FieldType::F64 {
                    FieldType::F64
                } else {
                    FieldType::I64
                };
                Ok((CExpr::Bin(*op, Box::new(ca), Box::new(cb)), ft))
            }
        }
    }

    /// CEP E2 item D: intern an entry-point name at COMPILE (registers it as
    /// rule-referenced). None / "DEFAULT" → 0.
    fn intern_ep(&mut self, name: &Option<String>) -> u32 {
        match name {
            None => 0,
            Some(n) if n == "DEFAULT" => 0,
            Some(n) => {
                if let Some(&id) = self.ep_ids.get(n) {
                    return id;
                }
                let id = self.entry_points.len() as u32;
                self.entry_points.push(n.clone());
                self.ep_ids.insert(n.clone(), id);
                id
            }
        }
    }

    /// A fact's interned entry-point id (DEFAULT = 0 for anything untagged —
    /// DEFAULT/RHS/synthetic inserts).
    fn fact_ep(&self, f: FactId) -> u32 {
        self.fact_eps.get(f.0 as usize).copied().unwrap_or(0)
    }

    pub fn insert(
        &mut self,
        type_name: &str,
        mut fields: Vec<(String, Value)>,
    ) -> Result<FactId, EngineError> {
        self.insert_into(type_name, fields, None)
    }

    /// CEP E2 item D: external insert into a NAMED entry point (`from
    /// entry-point`). `entry_point = None`/"DEFAULT" → the default WM. A name
    /// no rule references is rejected (Drools' getEntryPoint(unref) = null).
    pub fn insert_into(
        &mut self,
        type_name: &str,
        mut fields: Vec<(String, Value)>,
        entry_point: Option<&str>,
    ) -> Result<FactId, EngineError> {
        let ep_id = match entry_point {
            None => 0,
            Some(n) if n.is_empty() || n == "DEFAULT" => 0,
            Some(n) => *self.ep_ids.get(n).ok_or_else(|| {
                EngineError(format!(
                    "no rule references entry-point {n:?} (Drools: getEntryPoint returns null)"
                ))
            })?,
        };
        let id = self.insert_default(type_name, &mut fields)?;
        if ep_id != 0 {
            if self.fact_eps.len() <= id.0 as usize {
                self.fact_eps.resize(id.0 as usize + 1, 0);
            }
            self.fact_eps[id.0 as usize] = ep_id;
        }
        self.after_insert(id);
        Ok(id)
    }

    /// Store-level insert (field coercion/order) shared by insert_into; does
    /// NOT propagate — callers set fact_ep then call after_insert.
    fn insert_default(
        &mut self,
        type_name: &str,
        fields: &mut Vec<(String, Value)>,
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
        Ok(id)
    }

    /// Post-insert propagation (D-046/D-047): schedule clock jobs, then once
    /// the network is live, route + STREAM-flush. Runs AFTER the fact's
    /// entry-point tag is set (CEP E2 item D) so routing (alpha_passes) sees
    /// the right partition.
    fn after_insert(&mut self, id: FactId) {
        // D-141 (item 1b): snapshot the event's FIXED temporal position (its ts
        // at insert). A later ts-field UPDATE mutates the field but not this —
        // temporal joins / index keys keep the insert position (Drools CEP).
        let tid = self.store.fact_type(id);
        if let Some(&EventSpec { ts_fi, .. }) = self.event_specs.get(&tid) {
            if let Value::I64(ts) = self.store.value(id, ts_fi) {
                self.store.set_event_ts(id, ts);
            }
            // D-154: pending windowed-acc update entries drain at the FIRE
            // boundary only. Drools force-flushes them at an event insert's
            // queue position (D-150), but an accumulate's result emission
            // happens at rule evaluation either way, so fold values and
            // per-rule sequences are position-independent — and staging
            // into an acc node mid-epoch trips the per-arrival stream
            // flush's segment scoping (the port_insdrain probe lost the
            // revival to it).
        }
        let exp_ord = self.schedule_expiration(id);
        // D-151: feed the mechanical flush shadows (external inserts only —
        // the RHS insert path poisons instead).
        self.bf_on_external_insert(id, exp_ord);
        self.schedule_window_evictions(id);
        self.tms_note_stated(id);
        // Multi-fire (D-046): before the first fire_all the initial
        // batch propagates in its prologue; afterwards each insert
        // stages immediately (session.insert semantics — agenda
        // evaluation still waits for the next fire) and closes a k=1
        // staging window (external actions compose action-ordered at
        // terminals, D-047).
        if self.lists_built {
            let pre = self.stage_snapshot();
            self.on_insert(id, None);
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
            self.flush_trigger_tid = Some(self.store.fact_type(id));
            self.stream_flush(&pre);
            self.flush_trigger_tid = None;
        }
    }

    /// Type name of a live fact (bindings support, D-098 boundary).
    pub fn fact_type_name(&self, id: FactId) -> Option<String> {
        if !self.store.is_alive(id) {
            return None;
        }
        Some(self.store.schema(self.store.fact_type(id)).name.clone())
    }

    /// Nth VISIBLE inserted fact (D-047): the global insertion sequence
    /// excluding synthetics (InitialFact, accumulate results) — the same
    /// sequence Drools' objectInserted listener observes, so scenario
    /// action targets mean the same fact in both engines.
    /// D-104 (Arc 2): in-place session reset — the paged-batch
    /// lifecycle. Mirrors StatefulKnowledgeSessionImpl.reset():
    /// clears WM, agenda, TMS, clock/deadlines, handle numbering and
    /// all staging; KEEPS the compiled rules/queries/event specs.
    /// The network rebuilds to its just-compiled state and the next
    /// fire re-runs the prologue (re-creating the InitialFact — the
    /// oracle re-fires not-CE rules, probe rs_r7).
    pub fn reset(&mut self) -> Result<(), EngineError> {
        self.clock_ms = 0;
        self.deadlines.clear();
        self.pending_expirations.clear();
        // D-110/D-112: window deadline queue clears; acc_nodes is
        // recomputed by build_network below (trie reindexes on rebuild).
        self.window_deadlines.clear();
        self.acc_pending.clear();
        self.fire_deadlines.clear(); // D-134 (§3B)
        self.fire_seq = 0;
        self.in_expiration_drain = false;
        self.in_stream_flush = false;
        self.in_fire_loop = false;
        self.pn_seq = 0;
        self.px_explicit_victim = None;
        self.pn_churn_ctx = false;
        self.flush_trigger_tid = None;
        self.ever_linked.clear();
        self.focus_stack.clear();
        self.store.reset();
        // CEP E2 item D: per-fact EP tags die with the store; the compiled
        // entry_points/ep_ids table survives (compile-time, like rules).
        self.fact_eps.clear();
        self.lias.clear();
        self.trie.clear();
        self.nets.clear();
        self.lists_built = false;
        self.init_fact = None;
        self.collect_vals.clear();
        self.collect_scalar_vals.clear();
        self.act_seq = 0;
        self.query_mem = Default::default();
        self.query_pending = vec![false; self.queries.len()];
        self.qce_children.clear();
        self.gb_state.clear();
        self.query_armed = vec![false; self.queries.len()];
        self.pending_err = None;
        self.tms = Tms::default();
        // rebuild the network from the compiled rules (pattern keys are
        // pure; the alpha-sharing rewrites already live in the cmps)
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
        self.build_network(&keys);
        let init_tid = self.store.type_id(INITIAL_FACT).unwrap();
        if self
            .rules
            .iter()
            .any(|r| r.patterns.iter().any(|p| p.type_id == init_tid))
        {
            self.init_fact = Some(self.store.insert(init_tid, Vec::new()).map_err(EngineError)?);
        }
        Ok(())
    }

    pub fn nth_inserted(&self, n: usize) -> Option<FactId> {
        let hidden: Vec<TypeId> =
            [INITIAL_FACT, ACC_LONG, ACC_DOUBLE, ACC_COLLECTION, ACC_SETCOLLECTION, ACC_DECIMAL]
            .iter()
            .filter_map(|t| self.store.type_id(t))
            .collect();
        self.store
            .all_facts_in_insertion_order()
            .filter(|f| !hidden.contains(&self.store.fact_type(*f)))
            .filter(|f| !self.gbrow_tids.contains(&self.store.fact_type(*f)))
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
        // D-151/D-152: an external bare-P update is an immediate rtm
        // move-to-tail in the mechanical shadows at this op's queue position.
        self.bf_on_external_update(id);
        if self.lists_built {
            // D-166 (update-recency, FIFO refires): an EVENT-type external
            // update gets the same per-arrival trigger-scoped stream flush
            // as an insert (D-125 pattern) — each update action is its own
            // propagation batch, so multi-update epochs refire FIFO in
            // action order (model_join_flush v3 u5/m6; the tj-tail family).
            // Plain-type updates keep the certified batch path (the
            // mechanical shadows pin it).
            if self.event_specs.contains_key(&tid) {
                let pre = self.stage_snapshot();
                // D-170 (T6): expose the trigger to the flush's terminal
                // consumes — movability marking + the relocation gate.
                self.tj_trigger = Some((id, mask, tid));
                self.tj_entered.clear();
                self.on_update(id, mask, None);
                for net in self.nets.iter_mut() {
                    net.s0_close_window();
                }
                self.flush_trigger_tid = Some(tid);
                self.stream_flush(&pre);
                self.flush_trigger_tid = None;
                self.tj_trigger = None;
                self.tj_entered.clear();
            } else {
                self.on_update(id, mask, None);
                for net in self.nets.iter_mut() {
                    net.s0_close_window();
                }
            }
        }
        Ok(())
    }

    /// EXTERNAL working-memory delete by handle (D-047). Routes through
    /// the TMS quirk model (D-076): on a justified key the JUSTIFIED
    /// handle dies whichever handle was named; a stated sibling of a
    /// once-justified key no-ops (dump3).
    /// CEP E1/E2: declare a type as a point event — timestamps read
    /// from `ts_field` (i64, epoch-ms) at insert. `expires_ms`:
    /// `Some(n)` = explicit `@expires(n ms)`, auto-retract at
    /// ts+n(+1, D-102); `None` = INFER the reach from the temporal
    /// constraints after rule compile (CEP E2 item A, D-109). Explicit
    /// expiry is authoritative and suppresses inference (a8).
    pub fn declare_event(
        &mut self,
        type_name: &str,
        ts_field: &str,
        expires_ms: Option<i64>,
        duration: Option<&str>,
    ) -> Result<(), EngineError> {
        let tid = self
            .store
            .type_id(type_name)
            .ok_or_else(|| EngineError(format!("unknown type {type_name}")))?;
        let fi = self
            .store
            .field_index(tid, ts_field)
            .ok_or_else(|| EngineError(format!("{type_name} has no field {ts_field}")))?;
        if self.store.field_type(tid, fi) != FieldType::I64 {
            return Err(EngineError(format!(
                "{type_name}.{ts_field}: event timestamps are i64 epoch-ms (E1 point events)"
            )));
        }
        // CEP E2 item E (D-118): `@duration(f)` makes T an INTERVAL event
        // occupying `[ts, ts+f]`. `f` is an i64 field (ms), read at insert
        // like the timestamp. Absent ⇒ point event (dur=0, byte-identical).
        let dur_fi = match duration {
            Some(df) => {
                let dfi = self.store.field_index(tid, df).ok_or_else(|| {
                    EngineError(format!("{type_name} has no field {df}"))
                })?;
                if self.store.field_type(tid, dfi) != FieldType::I64 {
                    return Err(EngineError(format!(
                        "{type_name}.{df}: @duration fields are i64 ms (E2 item E)"
                    )));
                }
                Some(dfi)
            }
            None => None,
        };
        match expires_ms {
            Some(n) if n < 0 => {
                return Err(EngineError("expires_ms must be >= 0".into()));
            }
            Some(n) => {
                self.event_specs
                    .insert(tid, EventSpec { ts_fi: fi, expires: Some(n), dur_fi });
                self.explicit_expiry.insert(tid);
            }
            None => {
                // un-annotated: register as an event (ts field known);
                // expiry is filled by infer_event_expiry after all
                // rules compile (its temporal reach is not yet known).
                self.event_specs
                    .insert(tid, EventSpec { ts_fi: fi, expires: None, dur_fi });
            }
        }
        Ok(())
    }

    /// D-109: Floyd-Warshall closure of ONE rule's temporal STP, folding
    /// each event position's inferred forward reach into `temporal_ub`
    /// (MAX across rules). Mirrors Drools `BuildUtils`/`TimeUtils.
    /// calculateTemporalDistance` + `TemporalDependencyMatrix.
    /// getExpirationOffset` (per-subrule). `edges` are directed upper
    /// bounds `(u,v,ub)` = max(time_v − time_u); the closure lets a
    /// chain's earliest event inherit the SUMMED reach (trans_e1: E1 →
    /// E2 → E3 gives E1 = 150, not the pairwise 100). A type's reach =
    /// MAX over other positions of the closed upperBound; no finite
    /// forward reach ⇒ no contribution (never expires — the lo>0 leak).
    fn accumulate_temporal_closure(
        &mut self,
        edges: &[(usize, usize, i64)],
        pos_type: &HashMap<usize, TypeId>,
    ) {
        if edges.is_empty() {
            return;
        }
        const INF: i64 = 1 << 60;
        let mut positions: Vec<usize> = pos_type.keys().copied().collect();
        positions.sort_unstable();
        let n = positions.len();
        let idx: HashMap<usize, usize> =
            positions.iter().enumerate().map(|(i, &p)| (p, i)).collect();
        // d[i][j] = upperBound(time_j − time_i); diagonal 0, else INF.
        let mut d = vec![vec![INF; n]; n];
        for (i, row) in d.iter_mut().enumerate() {
            row[i] = 0;
        }
        for &(u, v, ub) in edges {
            let (i, j) = (idx[&u], idx[&v]);
            if ub < d[i][j] {
                d[i][j] = ub; // tightest bound wins
            }
        }
        // Floyd-Warshall STP tightening (finite compositions only).
        for k in 0..n {
            for i in 0..n {
                if d[i][k] >= INF {
                    continue;
                }
                for j in 0..n {
                    if d[k][j] < INF {
                        let via = d[i][k] + d[k][j];
                        if via < d[i][j] {
                            d[i][j] = via;
                        }
                    }
                }
            }
        }
        // reach(pos i) = MAX finite upperBound to any other position.
        for (i, p) in positions.iter().enumerate() {
            let tid = pos_type[p];
            let mut best: Option<i64> = None;
            for j in 0..n {
                if i != j && d[i][j] < INF {
                    best = Some(best.map_or(d[i][j], |b| b.max(d[i][j])));
                }
            }
            match best {
                // forward reach ≥ 0 → a finite inferred expiry (MAX-merged)
                Some(b) if b >= 0 => {
                    self.temporal_ub
                        .entry(tid)
                        .and_modify(|m| *m = (*m).max(b))
                        .or_insert(b);
                }
                // purely-backward (row-max < 0) or isolated → Drools'
                // matrix returns NEVER for this pattern, OVERWRITING the
                // type's OTN offset to NEVER: the LATER event of
                // after[lo>0] (the leak), and a self-join's probe side.
                _ => {
                    self.never_inferred.insert(tid);
                }
            }
        }
    }

    /// CEP E2 item A (D-109): infer `@expires` for event types with no
    /// explicit annotation. `temporal_ub` already holds each type's
    /// reach — the row-max of the transitively closed, per-rule Temporal
    /// DependencyMatrix (see `accumulate_temporal_closure`; `+hi` when a
    /// type is the EARLIER event, plus multi-hop sums). NEVER (`None`) if
    /// the type is in `never_inferred` (a bare or purely-backward pattern
    /// — see that field / `accumulate_temporal_closure`) or in no temporal
    /// constraint at all. Explicit `@expires` is
    /// authoritative (a8) and skipped. Feeds the existing D-102
    /// ts+offset+1 scheduler: Seine stores the raw upperBound and the
    /// +1 is applied at schedule time for rule-referenced types (so the
    /// inferred boundary is byte-identical to an explicit @expires of
    /// the same value — the infctl differential proof).
    ///
    /// A→B SEAM: the real Drools offset is `max(matrix_ub, window_ub)`
    /// (PatternBuilder:356-376; `SlidingTimeWindow.getExpirationOffset`
    /// = size, `SlidingLengthWindow` = -1). Windows (item B) are not in
    /// the subset yet; when they land, fold each `window:time(N)` size
    /// into `temporal_ub` (max) at compile so this stays the single
    /// inference point, and re-pin the a4-style inference-with-window
    /// probes. `window:length` contributes nothing (count-based).
    fn infer_event_expiry(&mut self) {
        let tids: Vec<TypeId> = self.event_specs.keys().copied().collect();
        for tid in tids {
            if self.explicit_expiry.contains(&tid) {
                continue; // explicit @expires wins, no max-merge (a8)
            }
            let inferred = if self.never_inferred.contains(&tid) {
                None // bare or purely-backward pattern forces NEVER (D-109)
            } else {
                // temporal_ub holds only finite forward reaches (≥ 0);
                // absent → type in no temporal constraint → NEVER.
                self.temporal_ub.get(&tid).copied()
            };
            if let Some(spec) = self.event_specs.get_mut(&tid) {
                spec.expires = inferred;
            }
        }
    }

    /// Schedule expiration for a freshly inserted fact of an event
    /// type. Timestamps are FIXED at insert (DefaultEventHandle
    /// semantics); deadline = ts + expires_ms.
    /// D-151/D-152: classify an EXTERNAL insert for every rule's mechanical
    /// flush shadow. `exp_ord` = the boundary class from
    /// `schedule_expiration` (Equal ⇒ due-on-arrival registers in the same
    /// flush; Less = a NEGATIVE deadline, the DROOLS-455 leak ⇒ alive
    /// forever, never registered).
    fn bf_on_external_insert(&mut self, id: FactId, exp_ord: Option<std::cmp::Ordering>) {
        if self.nets.is_empty() {
            return;
        }
        let tid = self.store.fact_type(id);
        let ep = self.fact_ep(id);
        let due_now = exp_ord == Some(std::cmp::Ordering::Equal);
        for net in self.nets.iter_mut() {
            if let Some(bf) = net.bf.as_mut() {
                if bf.e0_tid == tid && bf.e0_ep == ep {
                    bf.on_e0_insert(id, due_now);
                } else if bf.p_tid == tid && bf.p_ep == ep {
                    bf.on_p_insert(id);
                }
            }
            if let Some(ex) = net.ex.as_mut() {
                if ex.e1_tid == tid && ex.e1_ep == ep {
                    ex.on_e1_insert(id, due_now);
                } else if ex.p_tid == tid && ex.p_ep == ep {
                    ex.on_p_insert(id);
                }
            }
            // D-158: the plain-blocker shadow's P side (external only — the
            // gate excludes RHS-touched P types). D events reach it at the
            // WM level (on_insert/on_delete) instead.
            if let Some(pn) = net.pn.as_mut() {
                if pn.p_tid == tid && pn.p_ep == ep {
                    pn.on_p_insert(id);
                }
            }
            // D-162: the plain-exists shadow's P side (same discipline).
            if let Some(px) = net.px.as_mut() {
                if px.p_tid == tid && px.p_ep == ep {
                    px.on_p_insert(id);
                }
            }
        }
    }

    /// D-151: an external update steps the shadows (bare-P move-to-tail;
    /// E0 updates are inert — the temporal position and deadline are
    /// insert-fixed, D-141, and a bare not-pattern has no mask to hit).
    fn bf_on_external_update(&mut self, id: FactId) {
        if self.nets.is_empty() {
            return;
        }
        let tid = self.store.fact_type(id);
        let ep = self.fact_ep(id);
        for net in self.nets.iter_mut() {
            if let Some(bf) = net.bf.as_mut() {
                if bf.p_tid == tid && bf.p_ep == ep {
                    bf.on_p_update(id);
                }
            }
            if let Some(ex) = net.ex.as_mut() {
                if ex.p_tid == tid && ex.p_ep == ep {
                    ex.on_p_update(id);
                }
            }
            if let Some(pn) = net.pn.as_mut() {
                if pn.p_tid == tid && pn.p_ep == ep {
                    pn.on_p_update(id);
                }
            }
        }
        // D-162: the plain-exists shadow — bare-P move-to-tail, and the
        // WITNESS update classified against the shadow's tracked alpha
        // state (exit-of-staged annihilates; exit-of-processed defers;
        // admit stages a fresh ins). Index loop: the alpha re-evaluation
        // borrows self.
        if self.nets.iter().any(|n| n.px.is_some()) {
            for ri in 0..self.nets.len() {
                let Some(px) = self.nets[ri].px.as_ref() else { continue };
                if px.p_tid == tid && px.p_ep == ep {
                    self.nets[ri].px.as_mut().unwrap().on_p_update(id);
                } else if px.d_tid == tid && px.d_ep == ep {
                    let alpha_now = self.alpha_passes_fields(ri, 1, id);
                    let seq = self.pn_seq;
                    self.nets[ri].px.as_mut().unwrap().on_d_update(id, alpha_now, seq);
                }
            }
        }
    }

    /// D-151: an external explicit delete steps the shadows at its queue
    /// position (classification by membership inside `on_delete`).
    fn bf_on_external_delete(&mut self, id: FactId) {
        if self.nets.is_empty() {
            return;
        }
        let tid = self.store.fact_type(id);
        let ep = self.fact_ep(id);
        for net in self.nets.iter_mut() {
            if let Some(bf) = net.bf.as_mut() {
                if (bf.e0_tid == tid && bf.e0_ep == ep) || (bf.p_tid == tid && bf.p_ep == ep) {
                    bf.on_delete(id);
                }
            }
            if let Some(ex) = net.ex.as_mut() {
                if (ex.e1_tid == tid && ex.e1_ep == ep) || (ex.p_tid == tid && ex.p_ep == ep) {
                    ex.on_delete(id);
                }
            }
            // D-158: P only — an external D delete reaches the pn shadow via
            // the WM-level on_delete hook (single site for all provenances).
            if let Some(pn) = net.pn.as_mut() {
                if pn.p_tid == tid && pn.p_ep == ep {
                    pn.on_p_delete(id);
                }
            }
            // D-162: P only — witness deletes reach the px shadow at the WM
            // level (px_on_wm_delete), where explicit provenance is flagged.
            if let Some(px) = net.px.as_mut() {
                if px.p_tid == tid && px.p_ep == ep {
                    px.on_p_delete(id);
                }
            }
        }
    }

    /// D-158: a fact reaching the WM (any provenance — external op, RHS
    /// insert, insertLogical, pending-belief unstage) steps every plain-not
    /// shadow whose BLOCKER type matches. External-phase events take a
    /// fresh stamp so a stale backlog ins can never alias a live RHS.
    fn pn_on_wm_insert(&mut self, f: FactId) {
        if self.nets.iter().all(|n| n.pn.is_none()) {
            return;
        }
        let tid = self.store.fact_type(f);
        let ep = self.fact_ep(f);
        let in_fire = self.in_fire_loop;
        if !in_fire {
            self.pn_seq += 1;
        }
        let seq = self.pn_seq;
        for net in self.nets.iter_mut() {
            if let Some(pn) = net.pn.as_mut() {
                if pn.d_tid == tid && pn.d_ep == ep {
                    pn.on_d_insert(f, in_fire, seq);
                }
            }
        }
    }

    /// D-158: a fact leaving the WM (explicit delete, TMS retract, expiry
    /// cascade) steps every plain-not shadow whose BLOCKER type matches.
    /// `pn_churn_ctx` marks the execute_rhs stale-key epilogue — the one
    /// churn-class site (del hops before this RHS's staged inses).
    fn pn_on_wm_delete(&mut self, f: FactId) {
        if self.nets.iter().all(|n| n.pn.is_none()) {
            return;
        }
        let tid = self.store.fact_type(f);
        let ep = self.fact_ep(f);
        let in_fire = self.in_fire_loop;
        let churn = self.pn_churn_ctx;
        if !in_fire {
            self.pn_seq += 1;
        }
        let seq = self.pn_seq;
        for net in self.nets.iter_mut() {
            if let Some(pn) = net.pn.as_mut() {
                if pn.d_tid == tid && pn.d_ep == ep {
                    pn.on_d_delete(f, in_fire, churn, seq);
                }
            }
        }
    }

    /// D-162: a witness-typed fact reaching the WM (any provenance —
    /// external op, RHS insertLogical, pending-belief unstage) steps every
    /// plain-exists shadow whose witness type matches, with the alpha
    /// re-evaluated against the LIVE fields (the cons drive). `has_ne`
    /// (queued unfired activations) feeds the mid-drain eval condition.
    fn px_on_wm_insert(&mut self, f: FactId) {
        if self.nets.iter().all(|n| n.px.is_none()) {
            return;
        }
        let tid = self.store.fact_type(f);
        let ep = self.fact_ep(f);
        let in_fire = self.in_fire_loop;
        if !in_fire {
            self.pn_seq += 1;
        }
        let seq = self.pn_seq;
        for ri in 0..self.nets.len() {
            let Some(px) = self.nets[ri].px.as_ref() else { continue };
            if px.d_tid == tid && px.d_ep == ep {
                let alpha = self.alpha_passes_fields(ri, 1, f);
                let has_ne = !self.nets[ri].queue.is_empty();
                self.nets[ri]
                    .px
                    .as_mut()
                    .unwrap()
                    .on_d_insert(f, alpha, in_fire, has_ne, seq);
            }
        }
    }

    /// D-162: a witness-typed fact leaving the WM. `explicit` iff this
    /// handle is the direct session.delete victim (the WM signal); TMS
    /// cascades and expiry stay silent — an unobserved unsatisfy coalesces
    /// until the quiescence eval. `pn_churn_ctx` hops an epilogue retract
    /// before its own RHS's staged inses (shared with the pn shadow).
    fn px_on_wm_delete(&mut self, f: FactId) {
        if self.nets.iter().all(|n| n.px.is_none()) {
            return;
        }
        let tid = self.store.fact_type(f);
        let ep = self.fact_ep(f);
        let in_fire = self.in_fire_loop;
        let churn = self.pn_churn_ctx;
        let explicit = self.px_explicit_victim == Some(f);
        if !in_fire {
            self.pn_seq += 1;
        }
        let seq = self.pn_seq;
        for ri in 0..self.nets.len() {
            let Some(px) = self.nets[ri].px.as_ref() else { continue };
            if px.d_tid == tid && px.d_ep == ep {
                let has_ne = !self.nets[ri].queue.is_empty();
                self.nets[ri]
                    .px
                    .as_mut()
                    .unwrap()
                    .on_d_delete(f, explicit, in_fire, has_ne, churn, seq);
            }
        }
    }


    /// Returns the deadline-vs-clock ordering when an expiration exists
    /// (None = never expires / not an event) — read by the D-151 shadow
    /// (`Equal` = due-on-arrival ⇒ registers in the same flush; `Less` =
    /// the D-132 leak ⇒ alive forever).
    fn schedule_expiration(&mut self, id: FactId) -> Option<std::cmp::Ordering> {
        let tid = self.store.fact_type(id);
        if let Some(&EventSpec { ts_fi: fi, expires, dur_fi }) = self.event_specs.get(&tid) {
            // expires == None → NEVER expires (D-109): the lo>0 later-event
            // leak, an Allen-only type (D-120 fence), or a type in no
            // temporal constraint. No deadline is scheduled; the fact lives
            // until retracted.
            if let Some(exp) = expires {
                if let Value::I64(ts) = self.store.value(id, fi) {
                    // CEP E2 item E (D-118): an interval event expires from
                    // its END `ts+dur`, not its start — explicit and
                    // inferred offsets both apply from the end (i2_int seam;
                    // dur=0 for point events ⇒ byte-identical).
                    let dur = dur_fi.map_or(0, |dfi| match self.store.value(id, dfi) {
                        Value::I64(d) => d,
                        _ => 0,
                    });
                    // D-102 (b1/b2 + f_only30 pins): Drools expiration is
                    // STRICTLY AFTER the window for RULE-REFERENCED types
                    // (the ObjectTypeNode schedules at offset + 1); an
                    // event type NO rule references has no OTN and expires
                    // at exactly ts + dur + expires.
                    let referenced = self.rules.iter().any(|r| {
                        r.patterns.iter().any(|p| p.type_id == tid)
                    });
                    let plus = if referenced { 1 } else { 0 };
                    // Java long overflow WRAPS (two's-complement), and the
                    // wrapped-negative deadline feeds the same DROOLS-455
                    // guard below — wrapping here keeps debug==release==Java
                    // (pr_cep_expoverflow pins the composite; same convention
                    // as the RHS expr evaluator).
                    let deadline =
                        ts.wrapping_add(dur).wrapping_add(exp).wrapping_add(plus);
                    // D-133 boundary, CORRECTED by D-152: the D-133 probes ran
                    // at insert-clock 0, where "past deadline" ⇔ NEGATIVE
                    // deadline — two cases they conflated (Drools
                    // PropagationEntry.Insert.scheduleExpiration read for
                    // names; oracle-verified xq1-xq3 + the exists full-axis
                    // population):
                    //   deadline < 0 ⇒ KEPT forever (the DROOLS-455 guard maps
                    //     a negative effectiveEnd to Long.MAX_VALUE = never —
                    //     the leak, pos_far/xq1);
                    //   0 <= deadline <= clock ⇒ due on arrival: the expire
                    //     action enqueues in THIS flush — the event still
                    //     MATCHES and FIRES this cycle, then drops at the
                    //     quiescence drain (pos_ins/xq2/xq3) — push the LAZY
                    //     delete (NOT mark_expired, which would suppress the
                    //     firing);
                    //   deadline > clock ⇒ scheduled normally on the reaper
                    //     queue.
                    // The returned Ordering is the boundary CLASS (Equal =
                    // due-on-arrival), consumed by the mechanical shadows.
                    let ord = if deadline < 0 {
                        std::cmp::Ordering::Less
                    } else if deadline <= self.clock_ms {
                        self.pending_expirations.push(id);
                        std::cmp::Ordering::Equal
                    } else {
                        self.deadlines.entry(deadline).or_default().push(id);
                        std::cmp::Ordering::Greater
                    };
                    return Some(ord);
                }
            }
        }
        None
    }

    /// CEP E2 item B (D-110): schedule this event's window evictions.
    /// For every windowed accumulate node whose source is this event's
    /// type, queue a scoped subtree eviction at EXACTLY `ts+N` (no +1 —
    /// win_t_b/win_t_slide boundary). The event's `@timestamp` is fixed
    /// at insert (DefaultEventHandle), so the deadline is known now.
    /// Over-scheduling is harmless: an event filtered out by the source
    /// constraint (win4_constr) or already gone is a no-op at drain
    /// (the node's `active` set gates it).
    fn schedule_window_evictions(&mut self, id: FactId) {
        if self.acc_nodes.is_empty() {
            return;
        }
        let tid = self.store.fact_type(id);
        let Some(&EventSpec { ts_fi: fi, .. }) = self.event_specs.get(&tid) else {
            return; // windowed sources are events; non-events have no ts
        };
        let Value::I64(ts) = self.store.value(id, fi) else {
            return;
        };
        for i in 0..self.acc_nodes.len() {
            let (ni, wtid, win, _) = self.acc_nodes[i];
            if wtid == tid {
                if let Some(n) = win {
                    // D-154: an already-due deadline never schedules — the
                    // event is REJECTED at admission (winacc_admits uses the
                    // same immutable snapshot ts, so it can never be admitted
                    // later either), and a stale past-key entry would
                    // otherwise pop at the next advance and evict a REVIVED
                    // member that Drools' queue never contained
                    // (wf902x184: the zombie must survive).
                    let due = ts.wrapping_add(n); // Java long wrap (see deadline above)
                    if due > self.clock_ms {
                        self.window_deadlines.entry(due).or_default().push((ni, id));
                    }
                }
            }
        }
    }

    /// D-154: sliding-window ADMISSION for one event at one windowed
    /// accumulate node — Drools' `SlidingTimeWindow.assertFact`:
    /// `startTimestamp + N <= now` REJECTS (wa_fresh_bnd_at pins the exact
    /// boundary), where startTimestamp is the INSERT-FIXED snapshot
    /// (D-141; a live ts write never re-positions — wa_fresh_reject_snap).
    /// Runs on the assertObject paths only: the insert walk and the
    /// no-RightTuple update transition. Members admitted here evict at the
    /// pre-scheduled `window_deadlines` entry (snapshot ts+N), which
    /// always still pends when admission succeeds.
    fn winacc_admits(&self, ni: usize, f: FactId) -> bool {
        let (ri, pos) = self.trie[ni].env;
        let Some(n) =
            self.rules[ri].patterns[pos].acc.as_ref().and_then(|a| a.window_time)
        else {
            return true;
        };
        let tid = self.store.fact_type(f);
        let Some(&EventSpec { ts_fi, .. }) = self.event_specs.get(&tid) else {
            return true; // windowed sources are events; non-events never gate
        };
        let Value::I64(ts) = self.store.temporal_ts(f, ts_fi) else {
            return true;
        };
        ts.wrapping_add(n) > self.clock_ms // Java long wrap
    }

    /// D-185 `window:length(N)` admission: append a SLOT for a fresh
    /// admission (insert-walk or never-admitted alpha entry — NOT a
    /// revival) and, on overflow, evict the OLDEST SLOT's occupant via
    /// `stage_acc_removal` (a no-op if that occupant is already a corpse —
    /// slot retention, D-184 sr1/sr2). The staged del folds with the same
    /// epoch's staging: one net re-fire (t1/t2/b1). No-op on non-length
    /// nodes and on a retained slot (never re-append).
    fn winlen_admit(&mut self, ni: usize, f: FactId) {
        let (ri, pos) = self.trie[ni].env;
        let Some(n) =
            self.rules[ri].patterns[pos].acc.as_ref().and_then(|a| a.window_len)
        else {
            return;
        };
        if self.trie[ni].win_ring.contains(&f)
            || self.trie[ni].win_admit_pending.contains(&f)
        {
            return;
        }
        if !self.acc_pending.is_empty() {
            // deferred entries pend: this walk admission's ring ops land
            // at the drain, AFTER the entries (true action order)
            self.trie[ni].win_admit_pending.push(f);
            return;
        }
        self.winlen_ring_op(ni, f, n);
    }

    /// The ring op proper: append the slot; on overflow evict the OLDEST
    /// SLOT's occupant (a no-op if it is already a corpse/detached — slot
    /// retention, D-184 sr1/sr2).
    fn winlen_ring_op(&mut self, ni: usize, f: FactId, n: i64) {
        self.trie[ni].win_ring.push(f);
        if self.trie[ni].win_ring.len() as i64 > n {
            let old = self.trie[ni].win_ring.remove(0);
            self.stage_acc_removal(ni, old);
        }
    }

    /// D-154: ONE windowed-accumulate node processes ONE update of a
    /// source event — either a drained EXTERNAL entry (deferred; live
    /// fields = epoch-final) or an RHS modify (immediate; fuzz-unreachable
    /// for windowed sources, kept for coherence). The RightTuple machine:
    /// `clock_removed` = DETACHED (RT present, fold absent) — marked by
    /// eviction/expiry-eager (stage_acc_removal), by admission REJECTION
    /// (RT plants with no transient — wa_stale_ins_reject/revive), and by
    /// an alpha-fail exit (WindowNode keeps the RT; wa_toggle_*).
    /// Transitions:
    ///   in-fold, alpha-pass  -> D-139 re-fold, mask = source BINDINGS
    ///                           only (constraints live on the WindowNode);
    ///   in-fold, alpha-fail  -> fold-out, UN-mask-gated (the WindowNode
    ///                           re-checks constraints on every modify) +
    ///                           detach;
    ///   detached, alpha-pass -> mask-HIT re-asserts at LIVE fields =
    ///                           REVIVAL (BetaNode.modifyObject: absent
    ///                           RightTuple + mask intersect -> assert),
    ///                           BYPASSING the window queue — a
    ///                           revived-after-eviction member is a zombie
    ///                           (wa_zombie; only delete/expiry reap it), a
    ///                           revived-before-eviction member still
    ///                           evicts at ts0+N (wa_toggle_reevict).
    ///                           Mask-MISS does NOTHING (the
    ///                           pr_cep_c_upd_evict_revive cell — D-137's
    ///                           guard was this cell over-generalized);
    ///   no RT, alpha-pass    -> fresh admission (winacc_admits) — the 242
    ///                           class: REJECT folds nothing (no
    ///                           transient) but plants the RT.
    fn winacc_step(&mut self, ni: usize, f: FactId, mask: u64, origin: Origin, was: &mut Vec<bool>) {
        let (ri, pos) = self.trie[ni].env;
        let was_in = self.trie[ni].active.contains(&f);
        // fields-only alpha (D-160): a drained entry may execute for a
        // fact a LATER Del entry retracts — entry-order aliveness.
        let now = self.alpha_passes_fields(ri, pos, f);
        let hit = mask == u64::MAX || self.rules[ri].patterns[pos].bind_fields & mask != 0;
        match (was_in, now) {
            (true, true) => {
                if hit {
                    self.trie[ni].node.s_right.add_upd(f, origin);
                } else {
                    // mask miss: immediate right-memory reAdd, no staging
                    // (the existing D-139 miss path, fz_42_4359)
                    let env = JoinEnvImpl {
                        store: &self.store,
                        rule: &self.rules[ri],
                        flush: self.in_stream_flush,
                        fire_no: self.fire_no,
                        not_releasing: false,
                    };
                    let key = phreak::JoinEnv::key_of_right(&env, pos - 1, f);
                    self.trie[ni].node.re_add_right_fact(f, key);
                }
            }
            (true, false) => {
                self.trie[ni].active.remove(&f);
                self.trie[ni].clock_removed.insert(f);
                self.trie[ni].node.s_right.add_del(f, origin);
            }
            (false, true) => {
                let detached = self.trie[ni].clock_removed.contains(&f);
                let admit = if detached { hit } else { self.winacc_admits(ni, f) };
                if admit {
                    self.trie[ni].clock_removed.remove(&f);
                    self.trie[ni].active.insert(f);
                    if !detached {
                        // D-185: a FRESH admission (x1 never-admitted entry)
                        // takes a slot; a revival (detached mask-hit) rides
                        // its existing state — zombie or retained slot.
                        self.winlen_admit(ni, f);
                    }
                    // re-entry staging exactly as the plain (false,true)
                    // update branch (D-082/D-083 ph classification)
                    let reentry =
                        self.trie[ni].node.s_right.del.iter().any(|(x, _, _)| *x == f);
                    let ph = if reentry { 1 } else { 0 };
                    self.trie[ni].node.s_right.add_ins_ph(f, origin, ph);
                } else if !detached {
                    self.trie[ni].clock_removed.insert(f); // rejected: RT plants
                }
            }
            (false, false) => {}
        }
        self.note_link_effects_ex(was, Some(f));
    }

    /// D-160: the plain (non-windowed) accumulate analog of `winacc_step` —
    /// the D-137/D-139 immediate arms extracted verbatim, executed at the
    /// entry drain against the live (epoch-final) fields. One entry, one
    /// node, D-094 two-pass (pass 0 = entries/updates, pass 1 = exits).
    fn plainacc_step(
        &mut self,
        ni: usize,
        f: FactId,
        mask: u64,
        pass: u8,
        was: &mut Vec<bool>,
    ) {
        let (ri, pos) = self.trie[ni].env;
        let was_in = self.trie[ni].active.contains(&f);
        // fields-only alpha (D-160): entry-order aliveness (see winacc_step)
        let now = self.alpha_passes_fields(ri, pos, f);
        if (pass == 0) == (was_in && !now) {
            return; // pass A: entries/updates; pass B: exits (D-094)
        }
        match (was_in, now) {
            (false, true) if self.trie[ni].clock_removed.contains(&f) => {
                // D-137 class 2: expiry-eager acc-removed events stay removed.
            }
            (false, true) => {
                self.trie[ni].active.insert(f);
                self.maybe_pulse(ni);
                let reentry =
                    self.trie[ni].node.s_right.del.iter().any(|(x, _, _)| *x == f);
                let ph = if reentry { 1 } else { 0 };
                self.trie[ni].node.s_right.add_ins_ph(f, None, ph);
            }
            (true, false) => {
                self.trie[ni].active.remove(&f);
                self.trie[ni].node.s_right.add_del(f, None);
            }
            (true, true) => {
                // plain accumulate: full listen mask (constraints ∪ bindings,
                // D-139); acc nodes are never temporal positives.
                if mask == u64::MAX || self.rules[ri].patterns[pos].listen_mask & mask != 0 {
                    self.trie[ni].node.s_right.add_upd(f, None);
                } else {
                    let env = JoinEnvImpl {
                        store: &self.store,
                        rule: &self.rules[ri],
                        flush: self.in_stream_flush,
                        fire_no: self.fire_no,
                        not_releasing: false,
                    };
                    let key = phreak::JoinEnv::key_of_right(&env, pos - 1, f);
                    self.trie[ni].node.re_add_right_fact(f, key);
                }
            }
            (false, false) => {}
        }
        self.note_link_effects_ex(was, Some(f));
    }

    /// D-154/D-160: execute the queued external acc-source entries FIFO
    /// against the live (now epoch-final) fields, at fire_all pre-fire.
    /// Aliveness is decided by ENTRY ORDER (D-160): an update entry
    /// followed by a Del entry executes while "alive" — its fold-in and
    /// the Del's fold-out EACH dirty the accumulate, so the terminal
    /// re-fires the net value (the oracle's per-entry incremental flush;
    /// xf_cep_acc_updel_flush_{plain,win}). A fact dead WITHOUT a later
    /// Del entry (expiry / prior epochs) still drops — its fold effects
    /// were compensated on the certified expiry path (D-155).
    fn drain_acc_pending(&mut self) {
        if self.acc_pending.is_empty() {
            return;
        }
        let entries = std::mem::take(&mut self.acc_pending);
        let del_pos: std::collections::HashMap<FactId, usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, (_, e))| matches!(e, AccEntry::Del))
            .map(|(i, (f, _))| (*f, i))
            .fold(std::collections::HashMap::new(), |mut m, (f, i)| {
                m.entry(f).or_insert(i);
                m
            });
        let mut drain_dead: std::collections::HashSet<FactId> =
            std::collections::HashSet::new();
        let mut was: Vec<bool> =
            (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        for (i, (f, entry)) in entries.into_iter().enumerate() {
            let ftype = self.store.fact_type(f);
            match entry {
                AccEntry::Upd(mask) => {
                    if drain_dead.contains(&f) {
                        continue; // an earlier Del entry retracted it
                    }
                    if !self.store.is_alive(f)
                        && del_pos.get(&f).is_none_or(|&d| d < i)
                    {
                        continue; // dead via expiry/other: certified drop
                    }
                    for pass in 0..2u8 {
                        for ni in 0..self.trie.len() {
                            let (ri, pos) = self.trie[ni].env;
                            let pat = &self.rules[ri].patterns[pos];
                            if pat.type_id != ftype
                                || matches!(pat.sub, SubRole::Outer { .. })
                                || pat.acc.is_none()
                            {
                                continue;
                            }
                            if pat.acc.as_ref().is_some_and(|a| a.window_time.is_some() || a.window_len.is_some())
                            {
                                if pass == 0 {
                                    self.winacc_step(ni, f, mask, None, &mut was);
                                }
                                continue;
                            }
                            self.plainacc_step(ni, f, mask, pass, &mut was);
                        }
                    }
                }
                AccEntry::Del => {
                    drain_dead.insert(f);
                    for ni in 0..self.trie.len() {
                        let (ri, pos) = self.trie[ni].env;
                        let pat = &self.rules[ri].patterns[pos];
                        if pat.type_id != ftype
                            || matches!(pat.sub, SubRole::Outer { .. })
                            || pat.acc.is_none()
                        {
                            continue;
                        }
                        if self.trie[ni].active.remove(&f) {
                            let had_pending_ins = self.trie[ni]
                                .node
                                .s_right
                                .ins
                                .iter()
                                .any(|(x, _, _)| *x == f);
                            self.trie[ni].node.s_right.add_del(f, None);
                            if had_pending_ins {
                                // The del ANNIHILATED an in-drain staged ins:
                                // Drools' two entries each dirtied the result —
                                // force the net-value re-emission through every
                                // left (Phase D re-derives + re-propagates).
                                for l in self.trie[ni].node.lefts_snapshot() {
                                    self.trie[ni].node.s_left.add_upd(l, None);
                                }
                                for ri2 in 0..self.rules.len() {
                                    if self.nets[ri2].path.contains(&ni)
                                        && self.rule_linked(ri2)
                                    {
                                        self.nets[ri2].queued = true;
                                        self.nets[ri2].dirty = true;
                                        self.nets[ri2].dirty_stamp = self.stage_seq;
                                    }
                                }
                            }
                        }
                        self.note_link_effects_ex(&mut was, Some(f));
                    }
                }
            }
        }
        // D-185: deferred walk admissions land after the entry FIFO, in
        // arrival order (pending admissions exist only when entries were
        // pending, so this fn was guaranteed to run past its empty-check).
        for ni in 0..self.trie.len() {
            let pending = std::mem::take(&mut self.trie[ni].win_admit_pending);
            if pending.is_empty() {
                continue;
            }
            let (ri, pos) = self.trie[ni].env;
            let Some(n) =
                self.rules[ri].patterns[pos].acc.as_ref().and_then(|a| a.window_len)
            else {
                continue;
            };
            for f in pending {
                if !self.trie[ni].win_ring.contains(&f) {
                    self.winlen_ring_op(ni, f, n);
                }
            }
        }
    }

    /// D-102: per-node staged-length snapshot for trigger-scoped
    /// flushes. Captured BEFORE an insert's on_insert; the insert's
    /// own propagation is the HEAD segment beyond these lengths
    /// (staging prepends).
    fn stage_snapshot(
        &self,
    ) -> (Vec<(usize, usize, usize, usize, usize, usize, usize, usize, usize)>, Vec<usize>, Vec<bool>, Vec<bool>, Vec<Vec<usize>>) {
        // Per-node (s0_in, s_left, s_right) ins lengths for TOUCH
        // detection + per-rule k=1 s0 sizes. The STASH is rights-only
        // (D-102: the flush is LEFT-flushing — forceFlushLeftTuple;
        // held rights stay until a normal evaluation); the wider
        // lengths scope the flush to the trigger's own paths (a7c:
        // untouched paths must not process staged deletes early).
        (
            self.trie
                .iter()
                .map(|t| {
                    (
                        t.s0_in.ins.len(),
                        t.node.s_left.ins.len(),
                        t.node.s_right.ins.len(),
                        t.s0_in.del.len(),
                        t.node.s_left.del.len(),
                        t.node.s_right.del.len(),
                        t.s0_in.upd.len(),
                        t.node.s_left.upd.len(),
                        t.node.s_right.upd.len(),
                    )
                })
                .collect(),
            self.nets
                .iter()
                .map(|n| n.s0.iter().map(|w| w.ins.len() + w.del.len() + w.upd.len()).sum())
                .collect(),
            (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect(),
            self.ever_linked.clone(),
            // D-166: per-net per-window UPD counts — the k=1 stash scopes
            // to PRE-EXISTING upds so an update-triggered flush sees its
            // own effect (insert triggers add no upds: byte-identical).
            self.nets.iter().map(|n| n.s0.iter().map(|w| w.upd.len()).collect()).collect(),
        )
    }

    /// D-102 (model-check survivor drain_t/nonflush): the STREAM-mode
    /// per-insert flush, TRIGGER-SCOPED — Drools' forceFlushLeftTuple
    /// carries EMPTY tuple sets, so only the triggering insert's own
    /// propagation flushes; pre-existing staged backlogs stay held
    /// (v2), and TMS deferred state is untouched (a7c). Mechanics:
    /// stash the pre-insert staged tails, evaluate QUEUED rules over
    /// the delta (activation queueing only — each flush is its own
    /// D-047-style window), SELF-DRAIN unlinked temporal deltas to
    /// memory (t6/t14 — this replaced drain-at-link), restore stashes.
    fn stream_flush(
        &mut self,
        pre: &(Vec<(usize, usize, usize, usize, usize, usize, usize, usize, usize)>, Vec<usize>, Vec<bool>, Vec<bool>, Vec<Vec<usize>>),
    ) {
        self.stream_flush_ex(pre, true)
    }

    fn stream_flush_ex(
        &mut self,
        pre: &(Vec<(usize, usize, usize, usize, usize, usize, usize, usize, usize)>, Vec<usize>, Vec<bool>, Vec<bool>, Vec<Vec<usize>>),
        close_windows: bool,
    ) {
        if self.event_specs.is_empty() || !self.lists_built {
            return;
        }
        if self.ever_linked.len() != self.rules.len() {
            self.ever_linked = vec![false; self.rules.len()];
        }
        if close_windows {
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
        }
        // touched nodes/rules = staging grew during this insert
        let touched_node: Vec<bool> = self
            .trie
            .iter()
            .enumerate()
            .map(|(ni, t)| {
                let p = pre.0[ni];
                t.s0_in.ins.len() > p.0
                    || t.node.s_left.ins.len() > p.1
                    || t.node.s_right.ins.len() > p.2
                    // D-166: an update-triggered flush touches via upds
                    || t.s0_in.upd.len() > p.6
                    || t.node.s_left.upd.len() > p.7
                    || t.node.s_right.upd.len() > p.8
            })
            .collect();
        // stash pre-insert HELD RIGHT tails (delta = head segments) —
        // JOIN nodes only (D-102/a3-measured): a held right at a
        // not/exists node is a BLOCKER whose absence flips admission;
        // the flush walk must see it (E1 blocks E2 at the fire-1
        // flush), while held JOIN rights stay unpaired (v2's P1).
        // D-102 survivor (stay, hidden): PLAIN joins hide ALL staged
        // rights (delta included — rights never flush-pair) and held
        // (pre-tail) lefts; DELTA lefts still flush (v2's IF).
        // TEMPORAL joins and existential nodes stash nothing.
        let mut stash: Vec<(std::collections::VecDeque<(FactId, Origin, u8)>, std::collections::VecDeque<(FactId, Origin, u8)>, std::collections::VecDeque<(Tup, Origin, u8)>)> =
            Vec::with_capacity(self.trie.len());
        let mut dstash: Vec<(std::collections::VecDeque<(FactId, Origin, u8)>, std::collections::VecDeque<(Tup, Origin, u8)>, std::collections::VecDeque<(FactId, Origin, u8)>)> =
            Vec::with_capacity(self.trie.len());
        // per-rule PRE-LINK staging check BEFORE the stash empties it
        // (if_flush = pair_unless_held, u1c/987)
        let rule_held_pre: Vec<bool> = (0..self.rules.len())
            .map(|ri| {
                // D-102 (u1c/987, NARROW — the broad per-rule gate
                // blast-radiused 18% of stream scenarios): defer the
                // flush eval only when the TRIGGER toggled this rule's
                // not/exists CE (enabler-type match) AND pre-link
                // (ph=4) rights are held on the path.
                // D-102 (w1-w5 ladder): NO eval gate — the enabler-
                // toggle's flush eval runs; faithfulness comes from the
                // toggle-aware stash exemption below (held ph=4 rights
                // stay VISIBLE to the toggling rule's eval, composing
                // rightIns-then-IF in certified phase order).
                let _ = ri;
                false
            })
            .collect();
        // linked-ness per node BEFORE mutation (temporal stash gate)
        let node_linked: Vec<bool> =
            (0..self.trie.len()).map(|ni| self.rule_linked(self.trie[ni].env.0)).collect();
        // D-102 (cf101x551 vs t14): a node SHARED by >1 rule never
        // flush-pairs — force-flushing a shared segment would push
        // tuples into multiple rule paths out of agenda order; the
        // certified pop path composes them instead (lazy creation-
        // order for every sharing terminal).
        let node_shared: Vec<bool> = (0..self.trie.len())
            .map(|ni| {
                self.nets.iter().filter(|n| n.path.contains(&ni)).count() > 1
            })
            .collect();
        for ni in 0..self.trie.len() {
            self.trie[ni].node.shared = node_shared[ni];
        }
        // D-136: shared temporal-join nodes whose shape is the VALIDATED one
        // (clean insert-only delta, all-Term sinks) route their per-arrival
        // D-125 flush to EACH sharing sink instead of the legacy pop-time
        // bail. Populated in the stash loop below (kept OUT of the eval walk
        // so `do_node` never re-orders them), consumed in the D-125 flush
        // loop. `None` = keep the certified pop-time path.
        let mut shared_tj_stash: Vec<
            Option<(
                std::collections::VecDeque<(FactId, Origin, u8)>,
                std::collections::VecDeque<(FactId, Origin, u8)>,
                std::collections::VecDeque<(Tup, Origin, u8)>,
            )>,
        > = (0..self.trie.len()).map(|_| None).collect();
        for (ni, p) in pre.0.iter().enumerate() {
            let t = &mut self.trie[ni];
            // pre-tail DELS stash at ALL nodes: staged deletes from
            // earlier actions (expirations, D-101 a3) batch to the
            // FIRE; only the trigger's own del effects flush
            let dd0 = t.s0_in.del.len() - p.3.min(t.s0_in.del.len());
            let mut s0_dtail = t.s0_in.del.split_off(dd0);
            // D-171 (relink out-and-back): a pre-existing s0-del whose
            // fact RE-ENTERS in THIS flush (a fresh same-fact s0-ins)
            // stays VISIBLE — the eval then processes del-then-ins in
            // stage order (Drools' relink drain kills the OLD tuple
            // objects before the fresh pairs derive). Stashed, the del
            // drains at the next fire AFTER the re-entry eval, and —
            // the engine kills by VALUE where Drools kills by tuple
            // OBJECT identity — destroys the re-created children
            // (tu51x80/tu51x187: the relink SET losses).
            let fresh_ins = t.s0_in.ins.len() - p.0.min(t.s0_in.ins.len());
            // D-181 (tu81x60 lingering-del, identity-law instance #3): at a
            // TEMPORAL node the pre-tail same-fact ins is VISIBLE to this
            // eval (temporal joins stash no lefts), so a LINGERING del —
            // staged by an earlier action or epoch on the then-unlinked
            // path, never drained — must stay visible with it, or it
            // drains at the pop AFTER the re-entry pair derives and kills
            // it by value. Plain joins stash pre-tail lefts too, so their
            // fresh-only scan keeps the certified del-batching.
            let scan_n = if t.node.temporal { t.s0_in.ins.len() } else { fresh_ins };
            if scan_n > 0 && !s0_dtail.is_empty() {
                let fresh: Vec<FactId> =
                    t.s0_in.ins.iter().take(scan_n).map(|(f, _, _)| *f).collect();
                let mut keep: Vec<(FactId, Origin, u8)> = Vec::new();
                s0_dtail.retain(|e| {
                    if fresh.contains(&e.0) {
                        keep.push(e.clone());
                        false
                    } else {
                        true
                    }
                });
                t.s0_in.del.extend(keep);
            }
            let ddl = t.node.s_left.del.len() - p.4.min(t.node.s_left.del.len());
            let sl_dtail = t.node.s_left.del.split_off(ddl);
            let ddr = t.node.s_right.del.len() - p.5.min(t.node.s_right.del.len());
            let sr_dtail = t.node.s_right.del.split_off(ddr);
            dstash.push((s0_dtail, sl_dtail, sr_dtail));
            if !matches!(t.node.kind, phreak::Kind::Join) {
                // D-159: a PLAIN (non-event) blocker at a not node stages
                // lazily — Drools force-flushes only EVENT inserts, so a
                // plain-D right-ins must not block at this arrival's flush:
                // a same-window del ANNIHILATES it in staging (the eager
                // block re-fired the whole join memory at the release).
                // EVENT blockers keep the certified D-102 visibility ("E1
                // blocks E2 at the fire-1 flush").
                // D-161 widens the gate to Kind::Exists (the follow-on the
                // D-159 scope note anticipated; own population, seeds
                // 5001-5003 = 205/900 base divergences): a plain witness
                // ins left visible to a mid-epoch flush SPLITS from its
                // dstash-hidden ph=1 del twin — the update-churn WEDGE
                // (the exists child dies permanently, ex2/ex2b) — and a
                // flush-time first-satisfy drains the P backlog
                // newest-first where the fire-eval drain is arrival-FIFO
                // (ex8). Churn stays COALESCING (c_exists_churn_plain).
                if matches!(t.node.kind, phreak::Kind::Not | phreak::Kind::Exists)
                    && !self
                        .event_specs
                        .contains_key(&self.rules[t.env.0].patterns[t.env.1].type_id)
                {
                    let sr_all = std::mem::take(&mut t.node.s_right.ins);
                    stash.push((sr_all, std::collections::VecDeque::new(), std::collections::VecDeque::new()));
                } else {
                    stash.push((std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new()));
                }
                continue;
            }
            if t.node.temporal {
                // D-102 (cf101x616/134/551/853 — ALL shared shapes;
                // the unscoped version blast-radiused 18% of ordinary
                // single-rule scenarios, caught by fresh-seed campaign
                // seeds 7/13/29): the stay-at-flush semantics apply to
                // SHARED temporal nodes ONLY. Unshared temporal nodes
                // keep the certified pre-0dc2a4e flush behavior
                // (delta rights walk; transition fills+pairs).
                // Unlinked deltas stay staged for the self-drain.
                // D-136: the stay-at-flush pop-time path orders shared
                // firings wrong (14%, order-only). The fix routes the
                // per-arrival D-125 flush to EACH sharing sink (below in the
                // flush loop) — but ONLY for the validated shape: a clean
                // insert-only delta feeding all-Term sinks. A naive un-bail
                // (letting the eval walk's `do_node` flush per-arrival) was
                // 61% WORSE + 2 corpus regressions, because `do_node` emits
                // the REVERSED base order; `flush_ins_delta` (D-125) emits
                // the right one. So keep the staging OUT of the eval walk
                // (stash it) and hand the clean shape to the flush loop;
                // everything else keeps the certified pop-time bail.
                let clean_shared = node_shared[ni]
                    && t.node.s_right.upd.is_empty()
                    && t.node.s_right.del.is_empty()
                    && t.node.s_left.upd.is_empty()
                    && t.node.s_left.del.is_empty()
                    && t.s0_in.upd.is_empty()
                    && t.s0_in.del.is_empty()
                    && dstash
                        .last()
                        .map_or(true, |(a, b, c)| a.is_empty() && b.is_empty() && c.is_empty())
                    && t.node.s_right.ins.iter().all(|(_, _, ph)| *ph != 1)
                    && !t.sinks.is_empty()
                    && t.sinks.iter().all(|s| matches!(s, Sink::Term(_)));
                if clean_shared {
                    // D-136: divert to the per-arrival D-125 flush (below) —
                    // it drains THIS arrival's single-side delta to memory in
                    // ARRIVAL order and accumulates emissions. Do it on EVERY
                    // arrival, linked or not: an UNLINKED left left in staging
                    // batch-flushes (and self-drains) REVERSED, flipping the
                    // base order the peers then reverse again. The unshared
                    // D-125 path already drains unlinked deltas the same way.
                    let sr_all = std::mem::take(&mut t.node.s_right.ins);
                    let s0_all = std::mem::take(&mut t.s0_in.ins);
                    let sl_all = std::mem::take(&mut t.node.s_left.ins);
                    shared_tj_stash[ni] = Some((sr_all, s0_all, sl_all));
                    stash.push((std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new()));
                } else if node_linked[ni] && node_shared[ni] {
                    // legacy pop-time bail (non-clean shared temporal node)
                    let sr_all = std::mem::take(&mut t.node.s_right.ins);
                    let (s0_all, sl_all) = (
                        std::mem::take(&mut t.s0_in.ins),
                        std::mem::take(&mut t.node.s_left.ins),
                    );
                    stash.push((sr_all, s0_all, sl_all));
                } else {
                    stash.push((std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new()));
                }
                continue;
            }
            // D-102 (w5): when the TRIGGER toggles this node's rule's
            // not/exists CE and the node holds PRE-LINK (ph=4) rights,
            // the stash is EXEMPT — the toggle eval composes the held
            // rights with the IF in certified phase order (rightIns
            // pre-LIFO, then the IF's leftIns x full memory).
            let toggled = self.flush_trigger_tid.is_some_and(|tid| {
                self.rules[t.env.0].patterns.iter().any(|pt| {
                    matches!(pt.ce, CeKind::Not | CeKind::Exists) && pt.type_id == tid
                })
            });
            let has_ph4 = t.node.s_right.ins.iter().any(|(_, _, ph)| *ph == 4);
            // The IF actually TOGGLES iff the rule LINKS at this insert
            // (pre-unlinked -> now-linked; 358's second enabler is a
            // type-match but NOT a toggle). RE-materialization only
            // (w6-vs-v2): the FIRST-ever link (pmem creation) defers; a
            // relink of an existing path exempt-evaluates with the held
            // rights visible.
            let toggles_now = !pre.2[t.env.0] && node_linked[ni];
            let relink = pre.3.get(t.env.0).copied().unwrap_or(false);
            if toggled && toggles_now && has_ph4 && relink {
                stash.push((std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new()));
                continue;
            }
            let sr_all = std::mem::take(&mut t.node.s_right.ins);
            let d0 = t.s0_in.ins.len() - p.0.min(t.s0_in.ins.len());
            let s0_tail = t.s0_in.ins.split_off(d0);
            let dl = t.node.s_left.ins.len() - p.1.min(t.node.s_left.ins.len());
            let sl_tail = t.node.s_left.ins.split_off(dl);
            stash.push((sr_all, s0_tail, sl_tail));
        }
        // k=1 window DELS/UPDS are never the insert-trigger's own effect
        // — stash them ALL for the fire (expiration deletes batch to the
        // fire; cf5x17's k=1 justifier teardown must not run at a flush)
        let mut k1_stash: Vec<Vec<(usize, std::collections::VecDeque<(FactId, Origin, u8)>, std::collections::VecDeque<(FactId, Origin, u8)>)>> =
            Vec::with_capacity(self.nets.len());
        for (ri, net) in self.nets.iter_mut().enumerate() {
            let mut per: Vec<(usize, std::collections::VecDeque<(FactId, Origin, u8)>, std::collections::VecDeque<(FactId, Origin, u8)>)> =
                Vec::new();
            for (wi, w) in net.s0.iter_mut().enumerate() {
                // D-166: stash only PRE-EXISTING upds (the tail — staging
                // prepends); an update-triggered flush keeps its own fresh
                // upds visible. Insert triggers add none: byte-identical.
                let pre_upd =
                    pre.4.get(ri).and_then(|v| v.get(wi)).copied().unwrap_or(0).min(w.upd.len());
                let fresh_len = w.upd.len() - pre_upd;
                let upd_tail = w.upd.split_off(fresh_len);
                if !w.del.is_empty() || !upd_tail.is_empty() {
                    per.push((wi, std::mem::take(&mut w.del), upd_tail));
                }
            }
            k1_stash.push(per);
        }
        for ri in 0..self.rules.len() {
            let s0_now: usize =
                self.nets[ri].s0.iter().map(|w| w.ins.len() + w.del.len() + w.upd.len()).sum();
            let is_touched = s0_now > pre.1[ri]
                || self.nets[ri].path.iter().any(|&ni| touched_node[ni]);
            if std::env::var("SEINE_FLUSH_DEBUG").is_ok() && (self.nets[ri].queued || is_touched) {
                eprintln!("flush: r{ri} queued={} touched={is_touched}", self.nets[ri].queued);
            }
            // D-102 cycle-4 (u1c/987, if_flush=pair_unless_held): a
            // path holding PRE-LINK (ph=4) rights does not evaluate at
            // the flush — its IF-toggle cascade would pair against
            // right memory early; the whole composition waits for the
            // pop (where pre-LIFO rights precede the IF re-pairs).
            if self.nets[ri].queued && is_touched && !rule_held_pre[ri] {
                dbg_eval("flush", ri);
                self.in_stream_flush = true;
                self.evaluate_rule(ri, false, false);
                self.in_stream_flush = false;
            }
        }
        for (ri, per) in k1_stash.into_iter().enumerate() {
            for (wi, dels, upds) in per {
                if wi < self.nets[ri].s0.len() {
                    self.nets[ri].s0[wi].del.extend(dels);
                    self.nets[ri].s0[wi].upd.extend(upds);
                } else if let Some(w) = self.nets[ri].s0.last_mut() {
                    // D-266: CROSS-Staged restore (window gone) — register
                    // with the target's seen set.
                    for (f, _, _) in &dels {
                        w.seen_add(f);
                    }
                    for (f, _, _) in &upds {
                        w.seen_add(f);
                    }
                    w.del.extend(dels);
                    w.upd.extend(upds);
                }
            }
        }
        // D-125 (v2 flush model port): deltas left staged at temporal
        // nodes the eval didn't consume process PER-ARRIVAL, ascending
        // ni = parents before children so same-flush emissions cascade.
        // An eligible UNSHARED temporal join eager-joins a partner whose
        // anchor side is populated (individually, memory order) and
        // holds it in memory otherwise; the anchor's own arrival drains
        // the held batch through the staged prepend (one reversal —
        // `model_join_flush.py`, 0-div vs the gate oracle). Everything
        // else (shared nodes, mixed/upd/del staging, AB self-join
        // shapes, ph=1 rights, RIA sinks) keeps the certified legacy
        // self-drain: memory in arrival order, no children.
        for ni in 0..self.trie.len() {
            if !self.trie[ni].node.temporal {
                continue;
            }
            // D-136: a shared temporal join computes its per-arrival D-125
            // batch here (`flush_ins_delta` — the base order the pop-time
            // `do_join_node` gets REVERSED) but ACCUMULATES it into the
            // node's `tj_epoch` in forward order rather than routing to the
            // sinks. It can't route per-arrival: the peer copy reverses each
            // single-tuple batch (a no-op) and term_pending drains between
            // arrivals, so the whole-epoch reversal never forms (61% naive
            // un-bail). The fire boundary drains `tj_epoch` ONCE — first sink
            // forward, peers reversed (`fire_all`; model_shared_tjo.py 0-div).
            if let Some((sr, s0, sl)) = shared_tj_stash[ni].take() {
                let nidx = self.trie[ni].env.1 - 1;
                let (ri, _) = self.trie[ni].env;
                self.trie[ni].node.s_right.ins = sr;
                self.trie[ni].node.s_left.ins = sl;
                let s0_folds: Vec<(Tup, Origin, u8)> =
                    s0.into_iter().map(|(f, o, p)| (smallvec![f], o, p)).collect();
                let mut node = std::mem::replace(
                    &mut self.trie[ni].node,
                    phreak::Node::new_ex(phreak::Index::None, phreak::Kind::Join, true),
                );
                let mut trg: Staged<Tup> = Staged::default();
                node.flush_ins_delta(
                    &JoinEnvImpl { store: &self.store, rule: &self.rules[ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false },
                    nidx,
                    s0_folds,
                    &mut trg,
                );
                node.tj_epoch.extend(trg.ins);
                self.trie[ni].node = node;
                continue;
            }
            let nidx = self.trie[ni].env.1 - 1;
            let (ri, _) = self.trie[ni].env;
            let has_r = !self.trie[ni].node.s_right.ins.is_empty();
            let has_l = !self.trie[ni].node.s_left.ins.is_empty()
                || !self.trie[ni].s0_in.ins.is_empty();
            if !has_r && !has_l {
                continue;
            }
            // D-127: a temporal EXISTS node is admitted PER-ARRIVAL at eval
            // time (do_existential_node → exists_flush_admit reconstructs the
            // arrival order from the whole staged batch). It must NOT
            // self-drain here — self_drain_delta moves lefts to memory in
            // REVERSED order and without checking blockers, which corrupts
            // both the admission order and (blocker-before-left) the set.
            // Leaving the staging in place is the lazy-PHREAK behavior; the
            // eval consumes it. (`not` stays fenced; joins still drain.)
            if self.trie[ni].node.kind == phreak::Kind::Exists {
                continue;
            }
            let n = &self.trie[ni].node;
            let t = &self.trie[ni];
            let eligible = n.kind == phreak::Kind::Join
                && !node_shared[ni]
                && n.s_right.upd.is_empty()
                && n.s_right.del.is_empty()
                && n.s_left.upd.is_empty()
                && n.s_left.del.is_empty()
                && t.s0_in.upd.is_empty()
                && t.s0_in.del.is_empty()
                && !(has_r && has_l)
                && n.s_right.ins.iter().all(|(_, _, ph)| *ph != 1)
                && t.sinks.len() == 1
                && match t.sinks[0] {
                    Sink::Node(_) => true,
                    // a Term-sinked emission only happens at a LINKED
                    // eval (which already consumed the staging) — allow
                    // the cascade only when no emission is possible
                    Sink::Term(_) => {
                        (has_r && n.lefts_is_empty()) || (has_l && n.rights_is_empty())
                    }
                    Sink::Ria(_) => false,
                };
            if eligible {
                let s0_folds: Vec<(Tup, Origin, u8)> =
                    std::mem::take(&mut self.trie[ni].s0_in.ins)
                        .into_iter()
                        .map(|(f, o, p)| (smallvec![f], o, p))
                        .collect();
                let mut node = std::mem::replace(
                    &mut self.trie[ni].node,
                    phreak::Node::new_ex(phreak::Index::None, phreak::Kind::Join, true),
                );
                let mut trg: Staged<Tup> = Staged::default();
                node.flush_ins_delta(
                    &JoinEnvImpl { store: &self.store, rule: &self.rules[ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false },
                    nidx,
                    s0_folds,
                    &mut trg,
                );
                self.trie[ni].node = node;
                if !trg.is_empty() {
                    match self.trie[ni].sinks[0] {
                        Sink::Node(c) => {
                            let pending = self.trie[c].node.s_left.take();
                            self.trie[c].node.s_left =
                                Staged::append_into_pending(pending, trg);
                        }
                        Sink::Term(rb) => {
                            let pending = self.nets[rb].term_pending.take();
                            self.nets[rb].term_pending =
                                Staged::append_into_pending(pending, trg);
                        }
                        Sink::Ria(_) => unreachable!("cascade never targets a RIA sink"),
                    }
                }
            // D-167: only CLEAN insert-only staging may self-drain. A stale
            // staged upd means the arrival failed the D-125 eligibility gate
            // on an UNLINKED join; a childless self_drain_delta would lose
            // the pair PERMANENTLY across the unlink/relink (the relink eval
            // never re-derives from memories) — mixed staging stays for the
            // eval instead (cf6001x384).
            } else if (!self.trie[ni].node.s_right.ins.is_empty()
                || !self.trie[ni].node.s_left.ins.is_empty())
                && self.trie[ni].node.s_right.upd.is_empty()
                && self.trie[ni].node.s_left.upd.is_empty()
                && self.trie[ni].s0_in.upd.is_empty()
            {
                let mut node = std::mem::replace(
                    &mut self.trie[ni].node,
                    phreak::Node::new_ex(phreak::Index::None, phreak::Kind::Join, true),
                );
                node.self_drain_delta(
                    &JoinEnvImpl { store: &self.store, rule: &self.rules[ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false },
                    nidx,
                );
                self.trie[ni].node = node;
            }
        }
        // restore the held right tails
        let mut touched: Vec<usize> = Vec::new();
        for (ni, (sr_all, s0_tail, sl_tail)) in stash.into_iter().enumerate() {
            if !sr_all.is_empty() || !s0_tail.is_empty() || !sl_tail.is_empty() {
                touched.push(ni);
            }
            self.trie[ni].node.s_right.ins.extend(sr_all);
            self.trie[ni].s0_in.ins.extend(s0_tail);
            self.trie[ni].node.s_left.ins.extend(sl_tail);
        }
        for (ni, (s0_dtail, sl_dtail, sr_dtail)) in dstash.into_iter().enumerate() {
            if !s0_dtail.is_empty() || !sl_dtail.is_empty() || !sr_dtail.is_empty() {
                touched.push(ni);
            }
            self.trie[ni].s0_in.del.extend(s0_dtail);
            self.trie[ni].node.s_left.del.extend(sl_dtail);
            self.trie[ni].node.s_right.del.extend(sr_dtail);
        }
        // a linked rule whose path holds restored staging stays
        // queued+dirty (the flush's empty-queue dequeue must not
        // orphan the held work)
        if !touched.is_empty() {
            for ri in 0..self.rules.len() {
                if self.nets[ri].path.iter().any(|ni| touched.contains(ni))
                    && self.rule_linked(ri)
                {
                    if std::env::var("SEINE_FLUSH_DEBUG").is_ok() {
                        eprintln!("flush: REQUEUE r{ri}");
                    }
                    self.nets[ri].queued = true;
                    self.nets[ri].dirty = true;
            self.nets[ri].dirty_stamp = self.stage_seq;
                self.nets[ri].dirty_stamp = self.stage_seq;
                    self.nets[ri].dirty_stamp = self.stage_seq;
                }
            }
        }
        for ri in 0..self.rules.len() {
            if self.rule_linked(ri) {
                self.ever_linked[ri] = true;
            }
        }
    }

    /// CEP E1: advance the pseudo-clock. Due expirations apply as a
    /// deadline-ordered batch of EXTERNAL deletes at this action's
    /// position (a3: Drools batches all due retractions into the next
    /// evaluation with no intermediate agenda pass; a7 trio: the TMS
    /// cascade composition then follows the certified defer machinery).
    /// Already-dead facts (user-deleted, or retracted by an earlier
    /// expiration's TMS cascade) skip silently.
    pub fn advance(&mut self, ms: i64) -> Result<(), EngineError> {
        if ms < 0 {
            return Err(EngineError("advance must be >= 0".into()));
        }
        // Java long wrap: PseudoClockScheduler's timer += ms wraps the same
        self.clock_ms = self.clock_ms.wrapping_add(ms);
        let due: Vec<FactId> = {
            let mut keys: Vec<i64> = self
                .deadlines
                .range(..=self.clock_ms)
                .map(|(k, _)| *k)
                .collect();
            keys.sort_unstable();
            let mut out = Vec::new();
            for k in keys {
                if let Some(v) = self.deadlines.remove(&k) {
                    out.extend(v);
                }
            }
            out
        };
        // D-151/D-152: register due events with the mechanical shadows in
        // engine deadline order (the retracts run at the shadows' quiescence).
        for &id in &due {
            for net in self.nets.iter_mut() {
                if let Some(bf) = net.bf.as_mut() {
                    bf.on_advance_due(id);
                }
                if let Some(ex) = net.ex.as_mut() {
                    ex.on_advance_due(id);
                }
            }
        }
        // D-102 (cf5x33): expiration deletes to NOT-CE / temporal / join
        // propagate at agenda QUIESCENCE — a not-CE over an expired event
        // stays BLOCKED through all higher pops of the next fire. Those
        // still queue (pending_expirations, drained lazily). D-112: the
        // ACCUMULATE effect is the EXCEPTION — Drools retracts the event
        // from the accumulate at advance-time, so the count-drop precedes
        // the fire's inserts and fires by SALIENCE (df_* pins;
        // model_check_accdefer survivor: acc EAGER, not-CE LAZY).
        for id in due {
            if self.store.is_alive(id) {
                self.tms.expiring.insert(id);
                self.store.mark_expired(id);
                self.pending_expirations.push(id);
                self.eager_acc_removals(id); // eager accumulate right-delete
            }
        }
        // CEP E2 item B (D-110/D-112): a window eviction is EAGER — a due
        // scoped right-delete now (count drops, fact survives WM-wide;
        // df_win_evict_ctl / df_evict_reins). The EXACT eager/lazy timing
        // of eviction vs a coincident/earlier windowed EXPIRATION under a
        // later insert + salience is a deeper composition that flip-flops
        // (cf1x65 vs cf1x233; df_win_evict_ctl vs cf1x249) — DEFERRED to a
        // model_check_stream + WindowNode sub-recon (D-112 handoff); do not
        // hand-tune it (D-083). This keeps the pin-correct eager eviction.
        if !self.window_deadlines.is_empty() {
            let wkeys: Vec<i64> = self
                .window_deadlines
                .range(..=self.clock_ms)
                .map(|(k, _)| *k)
                .collect();
            for k in wkeys {
                if let Some(v) = self.window_deadlines.remove(&k) {
                    for (ni, id) in v {
                        self.stage_acc_removal(ni, id);
                    }
                }
            }
        }
        Ok(())
    }

    /// D-112: stage the EAGER scoped right-delete of an expiring event at
    /// every PLAIN accumulate node its type feeds — the count/sum drops now
    /// (before the fire's inserts, and firing by salience), while the fact
    /// is retracted later by the deferred `pending_expirations` delete.
    /// WINDOWED accumulates are SKIPPED: an expiration reaching the
    /// accumulate THROUGH a window node defers like other expiration
    /// propagation (df_win_expire_reins: with expiry < N the removal fires
    /// AFTER a later insert — the transient IS the oracle), so it rides the
    /// lazy `delete_fact` at quiescence. Window EVICTION is separately eager
    /// (window_deadlines). No-op per node if the event never entered it
    /// (alpha-filtered) or already left — the `active` guard.
    fn eager_acc_removals(&mut self, id: FactId) {
        if self.acc_nodes.is_empty() {
            return;
        }
        let tid = self.store.fact_type(id);
        for i in 0..self.acc_nodes.len() {
            let (ni, atid, win, wlen) = self.acc_nodes[i];
            // D-185: length-windowed accs skip the eager expiration removal
            // like time-windowed ones — the drop rides the lazy quiescence
            // delete THROUGH the window (e1/e3; the trickle corners stay
            // fenced with the time family, wl_f1/wl_f2).
            if atid == tid && win.is_none() && wlen.is_none() {
                self.stage_acc_removal(ni, id);
            }
        }
    }

    /// Quiescence step: propagate the pending expiration batch through
    /// the certified delete path. Returns true if anything processed.
    fn drain_pending_expirations(&mut self) -> bool {
        if self.pending_expirations.is_empty() {
            return false;
        }
        let pending = std::mem::take(&mut self.pending_expirations);
        self.in_expiration_drain = true;
        for id in pending {
            if self.store.is_alive(id) {
                let _ = self.delete_fact(id);
            }
        }
        self.in_expiration_drain = false;
        self.tms.expiring.clear();
        true
    }

    /// D-134 (§3B): release temporal-`not` firing deferrals whose window has
    /// closed (fire_time <= clock). Re-injects each due held left into its
    /// not node's `pending_release` and re-queues the rule, so the next
    /// evaluation fires it (if still UNBLOCKED — a blocked/blocked-then-
    /// expired left is no longer in node.lefts and stays silent). A due
    /// advance batch releases in DESCENDING fire_time (arc-A reverse-close-
    /// time); within one fire_time, arrival/creation order. An anchor reaped
    /// before its close (element no longer alive) is dropped. Returns whether
    /// anything was staged (⇒ the fire loop must rescan). Drained at
    /// quiescence BEFORE `drain_pending_expirations` so a not fires while its
    /// anchor is still alive (the fire at ts+hi precedes the reap at ts+hi+1).
    fn drain_pending_fires(&mut self) -> bool {
        let keys: Vec<i64> =
            self.fire_deadlines.range(..=self.clock_ms).map(|(k, _)| *k).collect();
        if keys.is_empty() {
            return false;
        }
        // Gather every due deferral as (fire_time, creation_seq, ni, left, o).
        let mut due: Vec<(i64, u64, usize, Tup, Origin)> = Vec::new();
        for k in keys {
            if let Some(batch) = self.fire_deadlines.remove(&k) {
                for (ni, l, o, seq) in batch {
                    due.push((k, seq, ni, l, o));
                }
            }
        }
        // Target agenda order (model_not_infer): the INITIAL fire (clock 0)
        // fires in CREATION order — the agenda is FIFO, no timer involved;
        // an ADVANCE batch fires DESCENDING close-time then creation — the
        // PseudoClockScheduler PriorityQueue drains by fire-time.
        if self.clock_ms == 0 {
            due.sort_by_key(|(_ft, seq, _, _, _)| *seq);
        } else {
            due.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        // Each released child PREPENDS at the not node (addInsert), so the
        // agenda is the REVERSE of the push order — push reverse(target).
        let mut any = false;
        for (_ft, _seq, ni, l, o) in due.into_iter().rev() {
            if !l.iter().all(|&f| self.store.is_alive(f)) {
                continue; // anchor reaped before its window closed
            }
            let ri = self.trie[ni].env.0;
            self.trie[ni].node.pending_release.push((l, o));
            self.nets[ri].dirty = true;
            self.nets[ri].queued = true;
            any = true;
        }
        any
    }

    /// D-110/D-112: remove event `id` from ONE accumulate node's right
    /// memory — a scoped right-delete (eval_acc_node Phase B→G re-fires: the
    /// count/sum drops) that leaves the fact in WM and every OTHER node
    /// untouched (win3_seam_tmax: E stays a live temporal anchor for the
    /// sibling rule while its window count drops). Staged EAGERLY at
    /// advance-time for both window eviction and expiration, so the drop
    /// precedes the fire's inserts (no transient) and fires by salience.
    /// No-op if the event is no longer active at the node (already expired,
    /// window-evicted, or alpha-filtered) — the `active` guard mirrors
    /// on_delete and gives win4_expl50 its double-removal NO-OP. Returns
    /// whether a delete was staged (⇒ the rule needs a re-eval).
    fn stage_acc_removal(&mut self, ni: usize, id: FactId) -> bool {
        if !self.trie[ni].active.remove(&id) {
            return false;
        }
        // CEP E2 item C class 2 (D-137): remember that the CLOCK removed this
        // event here, so a later external UPDATE (which re-propagates on
        // `is_alive`+alpha-pass, ignorant of the eviction/expiration) does not
        // revive it into the accumulate. Persists past the staged del's
        // processing — Drools keeps a clock-removed event removed.
        self.trie[ni].clock_removed.insert(id);
        self.mark_queries_pending();
        let mut was: Vec<bool> =
            (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        self.trie[ni].node.s_right.add_del(id, None);
        self.note_link_effects_ex(&mut was, Some(id));
        true
    }

    pub fn delete_fact(&mut self, id: FactId) -> Result<(), EngineError> {
        self.reject_mutation_with_qce("delete")?;
        if !self.store.is_alive(id) {
            // CEP E2 item C (D-115): Drools' session.delete is LENIENT on an
            // already-retracted handle — a double-delete (c_double_del) or a
            // delete of a fully-drained event is a graceful no-op, not an
            // error. (delete-of-EXPIRED already no-ops because expired events
            // stay is_alive until the deferred drain — c_del_after_exp.)
            return Ok(());
        }
        let Some(victim) = self.tms_route_delete(id) else {
            return Ok(()); // pinned no-op quirk
        };
        self.store.kill(victim);
        // D-151: an EXTERNAL explicit delete steps the mechanical shadows at
        // its queue position (E0 = retract-now force-eval; P = staged
        // annihilation / rtm removal). The expiration drain is excluded —
        // expiry retracts run at the shadows' own quiescence (pre_fire).
        if !self.in_expiration_drain {
            self.bf_on_external_delete(victim);
        }
        if self.lists_built {
            // D-160: an explicit external delete of an EVENT-TYPED fact
            // feeding an accumulate node queues a Del ENTRY — its acc
            // fold-out executes at the drain in FIFO order with the
            // deferred update entries (Drools' per-entry incremental
            // flush: an update entry before it still executes "alive").
            // Expiry (in_expiration_drain) and internal cascades keep
            // the immediate path.
            let victim_tid = self.store.fact_type(victim);
            let defer_acc = !self.in_expiration_drain
                && self.event_specs.contains_key(&victim_tid)
                && (0..self.trie.len()).any(|ni| {
                    let (ri, pos) = self.trie[ni].env;
                    let pat = &self.rules[ri].patterns[pos];
                    pat.type_id == victim_tid
                        && !matches!(pat.sub, SubRole::Outer { .. })
                        && pat.acc.is_some()
                });
            // D-162: mark the explicit victim for the px shadow's WM signal
            // (cascaded TMS retracts inside this delete are NOT explicit).
            if !self.in_expiration_drain {
                self.px_explicit_victim = Some(victim);
            }
            self.on_delete_ex(victim, None, defer_acc);
            self.px_explicit_victim = None;
            if defer_acc {
                self.acc_pending.push((victim, AccEntry::Del));
            }
            for net in self.nets.iter_mut() {
                net.s0_close_window();
            }
            // CEP E2 item C class 3 (D-138): an EXPLICIT delete of an EVENT
            // witness at an exists/not node is evaluated at DELETE-TIME (in
            // arrival order), NOT deferred to fire_all — so a same-epoch
            // REINSERT (whose own per-arrival stream-flush follows) re-blocks
            // and RE-FIRES the churn. STREAM mode stream-flushes inserts
            // per-arrival but defers deletes to the fire; that ins-before-del
            // order otherwise coalesces a delete-first event-exists churn
            // (spec: model_check_exists_churn.py `event_explicit_arrival`).
            // Expiration deletes (`in_expiration_drain`) stay deferred (D-102).
            // Scoped to rules with an exists/not CE over the victim's TYPE ⇒ the
            // plain corpus and every non-event / non-existential delete are
            // untouched (the deferred fire_all drain still processes them).
            if !self.in_expiration_drain
                && self.event_specs.contains_key(&self.store.fact_type(victim))
            {
                let tid = self.store.fact_type(victim);
                let affected: Vec<usize> = (0..self.rules.len())
                    .filter(|&ri| {
                        self.rules[ri].patterns.iter().any(|p| {
                            matches!(p.ce, CeKind::Not | CeKind::Exists) && p.type_id == tid
                        })
                    })
                    .collect();
                if !affected.is_empty() {
                    let saved = self.in_stream_flush;
                    self.in_stream_flush = true;
                    for ri in affected {
                        self.evaluate_rule(ri, true, false);
                    }
                    self.in_stream_flush = saved;
                }
            }
        }
        Ok(())
    }

    /// D-057: external update/delete with ?query CEs compiled is out of
    /// subset (left churn at query nodes is unprobed).
    fn reject_mutation_with_qce(&self, _what: &str) -> Result<(), EngineError> {
        // D-107 (Arc 5): the D-057 wall LIFTED — the qmut ladder pinned
        // ?query CEs as PULL-AT-ACTIVATION (queried-side churn never
        // re-evaluates existing matches), which the D-056 drain-window
        // machinery already implements. Mutation composes.
        Ok(())
    }

    pub fn fire_all(&mut self, limit: usize) -> Result<Vec<Firing>, EngineError> {
        self.in_fire_loop = false; // D-158: defensive (an errored prior fire)
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
                // D-102: STREAM sessions flush per pre-fire insert too
                // (session.insert before fireAllRules force-flushes) —
                // WITHOUT window closes: the initial batch composes as
                // ONE window (a3's batch pin holds pre- and post-flush).
                let pre = self.stage_snapshot();
                self.on_insert(f, None);
                self.flush_trigger_tid = Some(self.store.fact_type(f));
                self.stream_flush_ex(&pre, false);
                self.flush_trigger_tid = None;
            }
        }
        // D-154: execute the queued external-update entries for windowed
        // accumulates BEFORE any evaluation — FIFO, against the live
        // (epoch-final) fields. The agenda pick orders by (salience,
        // decl_pos), so staging here instead of at the update call is
        // byte-identical for single-update epochs.
        self.drain_acc_pending();
        // D-136: drain each shared temporal join's accumulated epoch batch
        // to its sinks ONCE, at the fire boundary — the FIRST sink FORWARD
        // (addAll ⇒ the D-125 order the flush computed), every PEER REVERSED
        // (peer_merge_term prepends the whole batch ⇒ the SegmentPropagator
        // reversal over the WHOLE epoch, which the per-arrival flush can't
        // form). Term sinks only (the validated shape); model_shared_tjo.py
        // 0-div. Empty `tj_epoch` everywhere else ⇒ byte-identical.
        for ni in 0..self.trie.len() {
            if self.trie[ni].node.tj_epoch.is_empty() {
                continue;
            }
            let mut trg: Staged<Tup> = Staged::default();
            let epoch = std::mem::take(&mut self.trie[ni].node.tj_epoch);
            for (t, _, _) in &epoch {
                trg.seen_add(t); // D-266: direct ins assignment below
            }
            trg.ins = epoch.into();
            let sinks = self.trie[ni].sinks.clone();
            for (si, sink) in sinks.into_iter().enumerate() {
                if let Sink::Term(rb) = sink {
                    if si == 0 {
                        let pending = self.nets[rb].term_pending.take();
                        self.nets[rb].term_pending =
                            Staged::append_into_pending(pending, trg.clone());
                    } else {
                        self.nets[rb].peer_merge_term(&trg);
                    }
                    self.nets[rb].dirty = true;
                    self.nets[rb].queued = true;
                }
            }
        }
        // D-151/D-152: step every mechanical flush shadow to its fire
        // boundary (fire-loop eval if queued, then quiescence expirations)
        // and rank the predicted emission for the gated picks below.
        // D-158: the plain-not shadow's boundary eval also runs when the
        // rule has queued items (the executor evaluates before firing).
        for ri in 0..self.nets.len() {
            let has_items = !self.nets[ri].queue.is_empty();
            let net = &mut self.nets[ri];
            if let Some(bf) = net.bf.as_mut() {
                bf.pre_fire();
            }
            if let Some(ex) = net.ex.as_mut() {
                ex.pre_fire();
            }
            if let Some(pn) = net.pn.as_mut() {
                pn.pre_fire(has_items);
            }
            // D-162: the plain-exists shadow's fire-loop eval iff QUEUED
            // (the spec's fire_all pre-drain step — no has_items, no
            // unconditional first eval: a pure-P backlog stays staged).
            if let Some(px) = net.px.as_mut() {
                px.pre_fire();
            }
        }
        self.in_fire_loop = true; // D-158: mid-loop D events eval immediately
        let mut firings = Vec::new();
        let mut last_fired: Option<usize> = None;
        while let Some(ri) = self.next_activation(last_fired) {
            if let Some(e) = self.pending_err.take() {
                return Err(EngineError(e));
            }
            last_fired = Some(ri);
            self.firing_stage_floor = self.stage_seq;
            if firings.len() >= limit {
                return Err(EngineError(format!(
                    "fire limit {limit} reached (non-terminating?)"
                )));
            }
            // RuleExecutor.getNextTuple: static rules removeFirst (FIFO);
            // dynamic-salience rules pop the queue MAX — ties NEWEST
            // first (MatchConflictResolver, D-043).
            let idx = match self.rules[ri].salience {
                // D-151/D-152/D-153: a gated existential rule (`not/exists
                // <EVENT>() P()`, `not_order_pos`) follows its MECHANICAL
                // flush shadow's emitted order — the retired D-140/143/146
                // not keys, D-144/147 exists keys, and D-140 in_cycle guard
                // were per-regime approximations of that machinery. A no-op
                // for a singleton queue; every non-gated (incl.
                // PLAIN-blocker) rule stays byte-identical FIFO.
                EngineSalience::Static(_) => match self.rules[ri].not_order_pos {
                    // D-158: the plain-blocker gated pick — rank by the
                    // PnShadow's emitted order (extended mid-cycle by
                    // in-fire churn/quiescence evals). No shadow (static
                    // exclusions) or unranked facts ⇒ plain FIFO.
                    None => match (self.rules[ri].pn_pos, self.rules[ri].px_pos) {
                        (None, None) => 0,
                        (Some(pos), _) => {
                            let rank = self.nets[ri].pn.as_ref().map(|b| &b.emit_rank);
                            self.nets[ri]
                                .queue
                                .iter()
                                .enumerate()
                                .min_by_key(|(i, a)| {
                                    let r = a
                                        .t
                                        .get(pos)
                                        .and_then(|f| rank.and_then(|m| m.get(f)))
                                        .copied()
                                        .unwrap_or(usize::MAX);
                                    (r, *i)
                                })
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                        (None, Some(pos)) => {
                            // D-162: the plain-exists gated pick — rank by
                            // the PxShadow's emitted order (extended
                            // mid-cycle by in-fire witness evals and the
                            // quiescence eval). No shadow (static
                            // exclusions) or unranked facts ⇒ plain FIFO.
                            let rank = self.nets[ri].px.as_ref().map(|b| &b.emit_rank);
                            self.nets[ri]
                                .queue
                                .iter()
                                .enumerate()
                                .min_by_key(|(i, a)| {
                                    let r = a
                                        .t
                                        .get(pos)
                                        .and_then(|f| rank.and_then(|m| m.get(f)))
                                        .copied()
                                        .unwrap_or(usize::MAX);
                                    (r, *i)
                                })
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                    },
                    Some(pos) => {
                        let exists = self.rules[ri].order_exists;
                        let q = &self.nets[ri].queue;
                        if exists {
                            // D-152: rank by the exists shadow's emission
                            // order — first-satisfaction FIFO, re-fire
                            // reversal, and the regime-2 fresh split all
                            // fall out of the machinery (no in_cycle guard:
                            // the shadow covers in-cycle stream inserts
                            // natively; RHS-touching shapes never build a
                            // shadow ⇒ every fact unranked ⇒ plain FIFO).
                            let rank = self.nets[ri].ex.as_ref().map(|e| &e.emit_rank);
                            q.iter()
                                .enumerate()
                                .min_by_key(|(i, a)| {
                                    let r = a
                                        .t
                                        .get(pos)
                                        .and_then(|f| rank.and_then(|m| m.get(f)))
                                        .copied()
                                        .unwrap_or(usize::MAX);
                                    (r, *i)
                                })
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        } else {
                            // D-151/D-153: rank by the mechanical shadow's
                            // emission order, UNGUARDED — the D-140 in_cycle
                            // guard retired with its last regime (the shadow
                            // covers same-cycle insert + delete-unblock
                            // windows natively, full-axis-soup-validated;
                            // pr_cep_c_del_not/_u3/_v3/_v5 reproduce
                            // mechanically). Unranked facts (no shadow /
                            // unknown) sort FIFO after ranked ones — with no
                            // shadow every fact is unranked and the pick IS
                            // FIFO (RHS-touching shapes never build one).
                            let rank = self.nets[ri].bf.as_ref().map(|b| &b.emit_rank);
                            q.iter()
                                .enumerate()
                                .min_by_key(|(i, a)| {
                                    let r = a
                                        .t
                                        .get(pos)
                                        .and_then(|f| rank.and_then(|m| m.get(f)))
                                        .copied()
                                        .unwrap_or(usize::MAX);
                                    (r, *i)
                                })
                                .map(|(i, _)| i)
                                .unwrap_or(0)
                        }
                    }
                },
                EngineSalience::Dyn { .. } => self.nets[ri]
                    .queue
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, a)| (a.sal, a.seq))
                    .map(|(i, _)| i)
                    .unwrap_or(0),
            };
            let tuple = self.nets[ri].queue.remove(idx).expect("pick idx in queue").t;
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
                    // D-108: scalar collections (collectList/collectSet)
                    if let Some(vals) = self.collect_scalar_vals.get(&f) {
                        fv.elems =
                            Some(vals.iter().map(|v| Some(scalar_view(v.clone()))).collect());
                    }
                    // D-108 groupby rows render as the [res, key]
                    // composite (ga3 raw: QueryArgs)
                    if self.gbrow_tids.contains(&self.store.fact_type(f)) {
                        fv = FactView {
                            type_name: "QueryArgs".into(),
                            fields: Vec::new(),
                            handle: u32::MAX,
                            elems: Some(vec![
                                Some(scalar_view(self.store.value(f, 0))),
                                Some(scalar_view(self.store.value(f, 1))),
                            ]),
                        };
                    }
                    fv
                })
                .collect();
            firings.push(Firing { rule: self.rules[ri].def.name.clone(), matches });
        }
        self.in_fire_loop = false; // D-158
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

        // D-151/D-152: the shadows' cycle predictions are consumed.
        for net in self.nets.iter_mut() {
            if let Some(bf) = net.bf.as_mut() {
                bf.post_fire();
            }
            if let Some(ex) = net.ex.as_mut() {
                ex.post_fire();
            }
            if let Some(pn) = net.pn.as_mut() {
                pn.post_fire();
            }
            if let Some(px) = net.px.as_mut() {
                px.post_fire();
            }
        }
        // D-170 (T6): the fire boundary closes the movability/self-slot
        // epoch — clear queued-activation movability and re-snapshot
        // every temporal join's right-memory order.
        for net in self.nets.iter_mut() {
            net.act_movable.clear();
        }
        for ni in 0..self.trie.len() {
            if self.trie[ni].node.temporal {
                let floor = self.stage_seq;
                self.trie[ni].node.epoch_reset(floor);
            }
        }
        self.fire_no += 1; // D-102 cycle-4: fire boundary — between-fire inserts stamp the NEXT fire
        Ok(firings)
    }

    /// D-058: WM events queue an ARMED query's agenda item; a pending
    /// item's evaluation drains its pattern memories (one window).
    /// D-086 (fz_min_3959): the item queues only while the query's path
    /// is LINKED — some or-branch with every positive pattern's alpha
    /// populated. An armed query whose branches all miss a pattern
    /// accumulates staged facts and drains them as ONE window at the
    /// linking event (the blanket pending=armed over-approximation
    /// split that window and reordered the memory: batches [10][100]
    /// [-1e9,-5] vs Drools' [10][-1e9,-5,100]). Pull evaluations
    /// (?query CE / getQueryResults) drain regardless of linking.
    fn mark_queries_pending(&mut self) {
        for qi in 0..self.queries.len() {
            self.query_pending[qi] =
                self.query_armed[qi] && crate::queries::query_linked(&self.store, &self.queries, qi);
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
    /// E1-hardening backstop: count one agenda step; return true once the
    /// per-`fire_all` budget is blown (a re-add cycle the fire limit can't
    /// catch). The caller must then bail out of `next_activation` with `None`
    /// — `fire_all` surfaces `pending_err` as an error, so the engine
    /// terminates instead of spinning. Never reached by legitimate sessions.
    fn spin_tick(&mut self) -> bool {
        // Per-`next_activation` CALL budget: one call's real work is bounded by
        // the agenda size (rules × queued tuples) + deferred size — at most a
        // few million even for large scenarios — so this is a huge margin that
        // only a genuine re-add cycle blows. `SEINE_SPIN_GUARD` overrides the
        // limit (recon only — a cycle's verdict is limit-independent).
        let limit = self.spin_limit;
        self.spin_guard += 1;
        if self.spin_guard > limit {
            if self.pending_err.is_none() {
                self.pending_err = Some(format!(
                    "agenda non-termination guard tripped at {limit} steps \
                     (E1-hardening backstop — a pre-existing temporal/TMS re-add cycle)"
                ));
            }
            return true;
        }
        false
    }

    /// The between-firings flush (Drools: evaluateEagerList inside
    /// fireNextItem/haltRuleFiring, plus the D-211/F2 unstage bridge).
    /// Runs before every agenda pick AND (D-261) at the same-rule
    /// sibling-continue — every firing boundary, so eager receivers
    /// stage per-delta exactly as often as Drools evaluates them.
    fn eager_flush(&mut self) {
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
                // D-196 (ip_a3 vs ip_c1/gt13): RIGHT-cause entries
                // (self-defeat) flush-drain at the run end; LEFT-cause
                // entries (update-break lane) drain only MID-RUN — the
                // run's LAST break rides to the item's pop (the
                // eval-consumption landing's (b) row: strictly-higher
                // observers glimpse the zombie). D-201 (sdp7004x51,
                // model land_eager mutfirst): the bit2 LATE-DEP ride
                // exempts the bit1 lane too — a MUTFIRST self-defeat's
                // last key rides drops[] to the pop (ip_a3's k0 entry
                // has no bit2; unaffected).
                self.tms_flush_drain(ri, dyn_sal, "flush-pre");
                dbg_eval("eager", ri);
                self.evaluate_rule(ri, false, true);
                // D-201 (sdp7001x97, the eager decl-law): an entry
                // pushed DURING ri's evaluation drains at ri's OWN
                // eager-list slot — decl-AFTER deleters then receive
                // the blocker ins+del FOLDED (gt6/x11 net-out) while
                // decl-BEFORE ones, already evaluated, churn (x70).
                if self.tms_flush_drain(ri, dyn_sal, "flush-mid") {
                    self.evaluate_rule(ri, false, true);
                }
            }
        }
        // Pass 2 (D-198, sd_b4): drains produced BY the pass-1 flush
        // evaluations run AFTER every eager rule evaluated — an
        // or-twin's sibling branch consumes the in-firing block (its
        // queue prunes) BEFORE the self-defeat drop un-breaks the not
        // (Drools' one-item or semantics; evaluateEagerList inside
        // haltRuleFiring — t20 pins pr_tms_selfbreak_flush /
        // pr_tms_t20d unchanged).
        for i in 0..self.rule_order.len() {
            let ri = self.rule_order[i];
            if self.rules[ri].def.no_loop
                || matches!(self.rules[ri].salience, EngineSalience::Dyn { .. })
            {
                let dyn_sal = matches!(self.rules[ri].salience, EngineSalience::Dyn { .. });
                if self.tms_flush_drain(ri, dyn_sal, "flush-post") {
                    self.evaluate_rule(ri, false, true);
                }
                // removeRuleAgendaItemWhenEmpty applies to EAGER
                // evaluations too (fz_42_8775): an emptied item leaves
                // the agenda and stops claiming shared-node windows.
                // D-091: removal requires !dirty as well.
                if self.nets[ri].queued
                    && self.nets[ri].queue.is_empty()
                    && !self.nets[ri].dirty
                {
                    self.nets[ri].queued = false;
                }
            }
        }
        // ⚖ D-211/F2 (the dynamic law's timing bridge, the D-201
        // churn force-evaluate precedent): a freshly UNSTAGED fact's
        // insert must reach every matching terminal AT THE FIRING
        // BOUNDARY (Drools creates the act at the flush, while the
        // fact is alive); a later delete then leaves the queued act
        // to fire with the dead handle's values (the exempted queue
        // cancels).
        if !self.tms.force_eval.is_empty() {
            let pending: Vec<FactId> = std::mem::take(&mut self.tms.force_eval);
            for f in pending {
                if !self.store.is_alive(f) {
                    continue;
                }
                for ri in 0..self.rules.len() {
                    let matches = self.rules[ri].patterns.iter().enumerate().any(
                        |(pos, pat)| {
                            pat.sub != SubRole::Inner
                                && self.alpha_passes(ri, pos, f)
                        },
                    );
                    if matches && self.nets[ri].dirty {
                        dbg_eval("unstage-force", ri);
                        self.evaluate_rule(ri, true, false);
                    }
                }
            }
        }
    }

    fn next_activation(&mut self, last: Option<usize>) -> Option<usize> {
        self.spin_guard = 0; // E1-hardening: per-call non-termination budget
        if let Some(l) = last {
            // D-091 (RuleExecutor.fire / haltRuleFiring): the just-fired
            // rule re-evaluates its network ONLY on the fire-loop's
            // CONTINUE path. When a STRICTLY-higher-salience item waits,
            // it HALTS without the self re-evaluation — the item stays
            // queued and its DIRTY flag keeps it alive to the deferred
            // pop, which then drains everything staged since (incl.
            // input that arrived while the path was unlinked —
            // fz_min_455's held right). The same strictly-higher gate
            // already governed the TMS defer drain (D-076, min608 vs
            // t11) — Drools' halt structure is WHY that pin exists; one
            // mechanism, now unified.
            let l_sal = self.item_salience(l);
            let l_group = self.rules[l].def.agenda_group.as_deref().unwrap_or("MAIN");
            let top_g: &str = self.focus_stack.last().map(|s| s.as_str()).unwrap_or("MAIN");
            let higher = (0..self.rules.len()).any(|rj| {
                rj != l
                    && self.nets[rj].queued
                    && self.rules[rj].def.agenda_group.as_deref().unwrap_or("MAIN") == top_g
                    && self.item_salience(rj) > l_sal
            }) || (top_g == "MAIN"
                && (0..self.queries.len())
                    .any(|qi| self.query_pending[qi] && 0 > l_sal));
            let _ = l_group;
            // ⚖ P3 pop-precedence (D-199, model head(); min812's
            // glimpse): a LAZY rule's run-end drops land only if l
            // would be the NEXT SELECTION anyway — an EQUAL-salience
            // DECL-PRECEDING queued same-group item pops first and
            // glimpses the transient; the entry lingers to l's own
            // pop (drain[pop]). The halt keeps the certified
            // strictly-higher gate (the min608 over-generalization
            // drained equal salience wholesale). EAGER rules' drops
            // land before the next selection commits (land_eager) —
            // exempt.
            let lazy_l = !(self.rules[l].def.no_loop
                || matches!(self.rules[l].salience, EngineSalience::Dyn { .. }));
            let eq_decl_preempt = lazy_l
                && (0..self.rules.len()).any(|rj| {
                    rj != l
                        && self.nets[rj].queued
                        && self.rules[rj].def.agenda_group.as_deref().unwrap_or("MAIN") == top_g
                        && self.item_salience(rj) == l_sal
                        && rj < l
                });
            let pre_force_qlen = self.nets[l].queue.len();
            if !higher {
                dbg_eval("post-fire-force", l);
                self.evaluate_rule(l, true, false);
                self.tms.defer_mode = false;
                if self.tms.deferred.iter().any(|(ri, _, _)| *ri == l)
                    || self.tms.exp_deferred.iter().any(|(ri, _)| *ri == l)
                {
                    while let Some(i) =
                        self.tms.exp_deferred.iter().position(|(ri, _)| *ri == l)
                    {
                        let (_, tuple) = self.tms.exp_deferred.remove(i);
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS drain[post-fire-exp] r{l} {tuple:?}");
                        }
                        self.tms_on_terminal_del(l, &tuple);
                        if self.spin_tick() {
                            return None;
                        }
                    }
                    // (eq_decl_preempt computed beside `higher` above —
                    // the ⚖ P3 pop-precedence gate). Per-ENTRY: only the
                    // NOT-side self-defeat lane (bit1) defers to the pop
                    // — the LIA/t20 lane (bit0, no not: self-update/
                    // delete breaks) keeps its CERTIFIED continue-drain
                    // discipline (pr_tms_t20*; 14 regression cells
                    // pinned it when the whole-drain gate over-deferred).
                    while let Some(i) = self.tms.deferred.iter().position(|(ri, _, fl)| {
                        *ri == l
                            && (*fl & 16) == 0
                            && !(eq_decl_preempt && (*fl & 2) != 0)
                    }) {
                        let (_, tuple, fl) = self.tms.deferred.remove(i);
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS drain[post-fire-continue] r{l} {tuple:?}");
                        }
                    // D-198 (sd_b4): an or-twin's SIBLING branch must
                    // consume the in-firing block (materializing its
                    // blocked list + queue prune) BEFORE the self-defeat
                    // drop un-breaks the not — one Drools item covers
                    // both branches.
                    {
                        let par = self.rule_parents[l];
                        let sibs: Vec<usize> = (0..self.rules.len())
                            .filter(|&rj| rj != l && self.rule_parents[rj] == par)
                            .collect();
                        for rj in sibs {
                            self.evaluate_rule(rj, false, false);
                        }
                    }
                        // ⚖ k0 churn (D-201): del-group rules consume
                        // the staged blocker-ins before the retract.
                        self.tms_churn_del_group(l, &tuple);
                        // ⚖ the mutfirst teardown (D-201): t0 release
                        // order for bit1+bit2 composite keys — AFTER
                        // the churn materializes the blocks. EAGER
                        // lane only (model: "Lazy routing untouched").
                        if fl & 6 == 6
                            && (self.rules[l].def.no_loop
                                || matches!(self.rules[l].salience, EngineSalience::Dyn { .. }))
                        {
                            self.tms_mf_teardown_reverse(l, &tuple);
                        }
                        self.tms_on_terminal_del(l, &tuple);
                        // ⚖ land_eager lead-k1 (D-199): the eager
                        // landing's unbreak re-propagates — unpark so
                        // the re-derived tuples re-fire. SELF-KILLED
                        // premises only (tms_left_death): the no-amut
                        // shape is a Drools runaway the engine fences.
                        if self.rules[l].def.no_loop
                            && self.tms_lead_k1(l)
                            && self.tms_left_death(l, &tuple)
                        {
                            self.tms.parked.retain(|(pri, _)| *pri != l);
                        }
                        if self.spin_tick() {
                            return None;
                        }
                    }
                    dbg_eval("post-fire-deferred", l);
                    self.evaluate_rule(l, false, false);
                }
            } else {
                self.tms.defer_mode = false;
            }
            if self.nets[l].queue.is_empty()
                && !self.nets[l].dirty
                && !self.tms.deferred.iter().any(|(ri, _, _)| *ri == l)
            {
                // removeRuleAgendaItemWhenEmpty: !dirty && empty (D-091)
                self.nets[l].queued = false;
            }
            // D-106 (fz_9001_1795 vs fz_9001_6127): a non-halted
            // executor KEEPS CONTROL — observable exactly when THIS
            // firing ran setFocus (the rescan would let the group-pop
            // change the candidate pool). All other paths keep the
            // corpus-certified rescan, which is pool-equivalent.
            let _ = std::mem::take(&mut self.focus_changed);
            // D-106 halt fine structure (fz_9003_879 closing the
            // 1795/9004_9 family): the executor's halt-check PEEKS the
            // focus-stack top WITHOUT popping. An EMPTY top peeks null
            // -> the executor keeps control regardless of what waits
            // in groups below (879: continue at salience -8 past
            // queued 0s in MAIN). A non-empty foreign top halts. When
            // the top IS the executor's own group, the certified
            // strictly-higher rule already decided (`higher` above,
            // scoped to top_g) — the cloud path is byte-identical.
            if !self.nets[l].queue.is_empty() {
                let l_grp = self.rules[l].def.agenda_group.as_deref().unwrap_or("MAIN");
                let top_now: &str =
                    self.focus_stack.last().map(|s| s.as_str()).unwrap_or("MAIN");
                if std::env::var("SEINE_AG_DEBUG").is_ok() {
                    let members: Vec<String> = (0..self.rules.len())
                        .filter(|&rj| {
                            self.rules[rj].def.agenda_group.as_deref().unwrap_or("MAIN")
                                == top_now
                        })
                        .map(|rj| {
                            format!(
                                "r{rj}(q={},len={},d={})",
                                self.nets[rj].queued,
                                self.nets[rj].queue.len(),
                                self.nets[rj].dirty
                            )
                        })
                        .collect();
                    eprintln!(
                        "halt-check: l=r{l} grp={l_grp} top={top_now} {:?} higher={higher} preq={pre_force_qlen}",
                        members
                    );
                }
                if top_now != l_grp {
                    // D-106 (fz_9003_879 + the halt matrix, 10 configs
                    // x 88 witnesses): the peek EVALUATES dirty items
                    // before comparing; a transparent top with a
                    // pre-force drain list continues. Every blocker-
                    // pool variant {any, stack+MAIN, MAIN, stack,
                    // MAIN-dyn} measured WORSE (77-81 vs 83) — the
                    // continue consults no other groups' queues.
                    let top_owned = top_now.to_string();
                    let mut members: Vec<usize> = (0..self.rules.len())
                        .filter(|&rj| {
                            self.rules[rj].def.agenda_group.as_deref().unwrap_or("MAIN")
                                == top_owned
                        })
                        .collect();
                    // D-262 (fz_4242_286, bd_a3/bd_g4): the peek walks the
                    // group's items in PICK order (item salience DESC, decl
                    // ASC — the agenda's own order) and STOPS at the first
                    // live one. top_empty is an EXISTENCE question; members
                    // below the stop point stay dirty and materialize at
                    // their own pop — Drools' lazy timing, so a sibling's
                    // later inserts land inside their single at-pop
                    // evaluation (certified emission machinery) instead of
                    // appending to a peek-pinned batch. NO ordering is
                    // encoded here: the fix only narrows WHICH members the
                    // peek evaluates; queue construction is untouched. The
                    // all-empty (continue) case still evaluates every
                    // member — the 88-witness halt matrix outcomes and the
                    // D-258 late-continue below are decided by the same
                    // boolean as before.
                    members.sort_by_key(|&rj| {
                        (std::cmp::Reverse(self.item_salience(rj)), self.rules[rj].def.decl_pos)
                    });
                    let mut top_nonempty = false;
                    for &rj in &members {
                        if self.nets[rj].queued && !self.nets[rj].queue.is_empty() {
                            top_nonempty = true;
                            break;
                        }
                        if self.nets[rj].queued
                            && self.nets[rj].queue.is_empty()
                            && self.nets[rj].dirty
                        {
                            self.evaluate_rule(rj, false, false);
                            if self.nets[rj].queue.is_empty() && !self.nets[rj].dirty {
                                self.nets[rj].queued = false;
                            }
                            if !self.nets[rj].queue.is_empty() {
                                top_nonempty = true;
                                break;
                            }
                        }
                    }
                    let top_empty = !top_nonempty;
                    if top_empty && pre_force_qlen > 0 {
                        // D-258 (fz_9901_1221 + fz_9104_5192/fz_9202_2058):
                        // the late-continue is a CONTINUE path, so the D-091
                        // continue-path self re-evaluation applies — `higher`
                        // skipped the post-fire force above, and without it a
                        // delete/update staged by THIS firing's RHS never
                        // prunes l's own queue (one stale activation fires;
                        // Drools' evaluateNetworkIfDirty at the item pop
                        // cancels the siblings first). Return Some(l) only if
                        // the queue survives the re-eval; else fall through
                        // with the D-091 removeRuleAgendaItemWhenEmpty
                        // unqueue. The !higher sibling path below is safe —
                        // it is only reached after the post-fire force.
                        if higher && self.nets[l].dirty {
                            dbg_eval("late-continue-force", l);
                            self.evaluate_rule(l, true, false);
                        }
                        if !self.nets[l].queue.is_empty() {
                            return Some(l);
                        }
                        if !self.nets[l].dirty {
                            self.nets[l].queued = false;
                        }
                    }
                } else if !higher && !self.focus_stack.is_empty() {
                    // D-261 (fz_5150_1857, bd_d4): the same-rule
                    // sibling-continue is a FIRING BOUNDARY — Drools'
                    // fireNextItem runs evaluateEagerList between every
                    // firing, continue or not. Without the flush here, a
                    // rule firing twice consecutively under focus
                    // coalesces an eager receiver's per-delta staging
                    // into one batch (a self-join then emits
                    // left-delta-major over the FINAL memory — bd_d4's
                    // (N-2,N5) row before N5 existed). The pick is
                    // unchanged: the executor keeps control (D-106).
                    self.eager_flush();
                    return Some(l);
                }
            }
        }
        self.eager_flush();
        // Agenda pop (D-008/D-043): items order by (item salience DESC,
        // decl index ASC). Static items carry their constant; DYNAMIC
        // items track their queue top (0 while empty/unevaluated) and
        // re-sort after their network evaluates
        // (RuleExecutor.updateSalience / haltRuleFiring). Evaluation
        // stays lazy: only the popped item's network runs, so window
        // claiming keeps its pinned order.
        loop {
            if self.spin_tick() {
                return None; // E1-hardening non-termination backstop
            }
            // (salience DESC, decl_pos ASC) over rule items AND pending
            // query items (D-058: queries are agenda items at salience 0;
            // PathMemory.queueRuleAgendaItem adds them to the group).
            let top_group: &str = self.focus_stack.last().map(|s| s.as_str()).unwrap_or("MAIN");
            let mut best: Option<(i32, usize, bool, usize)> = None; // (sal, decl, is_query, idx)
            for i in 0..self.rule_order.len() {
                let ri = self.rule_order[i];
                if self.rules[ri].def.agenda_group.as_deref().unwrap_or("MAIN") != top_group {
                    continue; // D-106: only the focused group competes
                }
                // D-076 (certified): any deferred entry makes the item
                // reachable. Expiration-routed teardowns live in
                // exp_deferred (D-102 q1/q4) and never resurrect items.
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
                if top_group != "MAIN" {
                    break; // queries live in MAIN (no agenda-group)
                }
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
            let Some((_, _, is_query, ri)) = best else {
                // D-106: an emptied focused group pops; the scan
                // resumes with the next group down (ag2/ag5/ag7)
                if !self.focus_stack.is_empty() {
                    self.focus_stack.pop();
                    continue;
                }
                // D-134 (§3B): release temporal-not firing deferrals whose
                // window has closed, BEFORE the expiration reap — a not fires
                // at ts+hi while its anchor is still alive (reap is ts+hi+1).
                // The re-injected lefts fire on the rescan.
                if self.drain_pending_fires() {
                    continue;
                }
                // AGENDA QUIESCENCE (q1/q4): lazy deferred teardowns
                // drain once the agenda empties; their retractions may
                // activate rules (not-D observers), so rescan.
                if self.drain_pending_expirations() {
                    // one round: the deletes just routed their TMS
                    // teardowns onto exp_deferred — run them NOW so
                    // every quiescence effect (not-unblocks AND
                    // belief retractions) materializes before the
                    // rescan; salience then orders the observers
                    // (cf11x24: ND4@2 before NE5@0).
                    let pending = std::mem::take(&mut self.tms.exp_deferred);
                    for (dri, tuple) in pending {
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS drain[quiescence-exp] r{dri} {tuple:?}");
                        }
                        self.tms_on_terminal_del(dri, &tuple);
                    }
                    continue;
                }
                // D-112: window evictions + accumulate expiration removals
                // are applied EAGERLY in advance() now, not here (no
                // pending_window_evictions queue) — the count-drop precedes
                // the fire's inserts and fires by salience.
                if !self.tms.exp_deferred.is_empty() {
                    let pending = std::mem::take(&mut self.tms.exp_deferred);
                    for (dri, tuple) in pending {
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS drain[quiescence-bare] r{dri} {tuple:?}");
                        }
                        self.tms_on_terminal_del(dri, &tuple);
                    }
                    continue;
                }
                // D-162 QUIESCENCE (the flushExpirations slot): staged
                // witness ops at a PLAIN-witnessed non-temporal EXISTS node
                // in a STREAM session evaluate at the window's end even with
                // the rule's item clean — a cross-boundary unsatisfy is
                // OBSERVED here (the exists child dies; the next window's
                // re-satisfy then re-fires the whole join memory — spec
                // predict_pexists, ex5001x75/x129), while a same-window
                // del+ins pair has already coalesced in one eval (witness
                // handover, no re-fire — x170/x79). Without this the leaked
                // del batches with a LATER window's ins: the D-161 stash
                // hides it from mid-epoch flushes whose evals clear the
                // dirty flag, so the boundary pop held it (the x75 6-vs-8
                // under-fire). The px shadows run their own quiescence eval
                // first so the rescan's picks are ranked.
                if !self.event_specs.is_empty() {
                    for net in self.nets.iter_mut() {
                        if let Some(px) = net.px.as_mut() {
                            px.quiescence();
                        }
                    }
                    let mut requeued = false;
                    for ri in 0..self.nets.len() {
                        let needs = self.nets[ri].path.iter().any(|&ni| {
                            let t = &self.trie[ni];
                            t.node.kind == phreak::Kind::Exists
                                && !t.node.temporal
                                && !t.node.s_right.is_empty()
                                && !self.event_specs.contains_key(
                                    &self.rules[t.env.0].patterns[t.env.1].type_id,
                                )
                        });
                        if needs && !(self.nets[ri].queued && self.nets[ri].dirty) {
                            self.nets[ri].queued = true;
                            self.nets[ri].dirty = true;
                            self.nets[ri].dirty_stamp = self.stage_seq;
                            requeued = true;
                        }
                    }
                    if requeued {
                        continue;
                    }
                }
                return None;
            };
            if is_query {
                self.drain_query_item(ri);
                continue;
            }
            // D-076: process deferred terminal-del unmatches when the
            // item is REACHED (Drools evaluateNetworkIfDirty position).
            // Expiration-routed entries (exp_deferred) instead wait for
            // the post-firing drain or quiescence (D-102 q1/q4).
            while let Some(i) =
                self.tms.deferred.iter().position(|(dri, _, _)| *dri == ri)
            {
                let (_, tuple, _) = self.tms.deferred.remove(i);
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS drain[pop] r{ri} {tuple:?}");
                }
                    // D-198 (sd_b4): an or-twin's SIBLING branch must
                    // consume the in-firing block (materializing its
                    // blocked list + queue prune) BEFORE the self-defeat
                    // drop un-breaks the not — one Drools item covers
                    // both branches.
                    {
                        let par = self.rule_parents[ri];
                        let sibs: Vec<usize> = (0..self.rules.len())
                            .filter(|&rj| rj != ri && self.rule_parents[rj] == par)
                            .collect();
                        for rj in sibs {
                            self.evaluate_rule(rj, false, false);
                        }
                    }
                // ⚖ k0 churn (D-201): del-group rules consume the
                // staged blocker-ins before the retract.
                self.tms_churn_del_group(ri, &tuple);
                self.tms_on_terminal_del(ri, &tuple);
            }
            dbg_eval("pop", ri);
            self.evaluate_rule(ri, false, false);
            if self.nets[ri].queue.is_empty() {
                if !self.nets[ri].dirty {
                    // removeRuleAgendaItemWhenEmpty (D-091) — lazy
                    // deferred entries LINGER (quiescence will drain)
                    self.nets[ri].queued = false;
                }
                continue;
            }
            // dynamic salience may have moved this item; re-check —
            // DYN-SALIENCE ITEMS ONLY (D-101/cf5x17): a static item that
            // activated OTHER rules during its pop (deferred terminal-del
            // retracting a belief) still fires its own head first; the
            // strictly-higher check happens BETWEEN firings (Drools'
            // executor keeps control through the current fire).
            if !matches!(self.rules[ri].salience, EngineSalience::Dyn { .. }) {
                return Some(ri);
            }
            let now = self.item_salience(ri);
            let cur_top: &str = self.focus_stack.last().map(|s| s.as_str()).unwrap_or("MAIN");
            let preempted = (0..self.rules.len()).any(|rj| {
                rj != ri
                    && self.nets[rj].queued
                    && self.rules[rj].def.agenda_group.as_deref().unwrap_or("MAIN") == cur_top
                    && {
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
        self.pn_on_wm_insert(f); // D-158: plain-not blocker shadows
        self.px_on_wm_insert(f); // D-162: plain-exists witness shadows
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
                    self.stage_seq += 1;
                    self.trie[c].node.left_sseq.insert(smallvec![f], self.stage_seq);
                    self.trie[c].s0_in.add_ins(f, origin);
                }
                self.note_link_effects_ex(&mut was, Some(f));
            }
        }
        for ni in 0..self.trie.len() {
            let (ri, pos) = self.trie[ni].env;
            if matches!(self.rules[ri].patterns[pos].sub, SubRole::Outer { .. }) {
                continue; // subnet CE nodes take rights via the RIA hop
            }
            if self.alpha_passes(ri, pos, f) {
                // D-154: sliding-window admission at INSERT — an event whose
                // snapshot ts+N is already due folds NOTHING (no transient,
                // wa_stale_ins_reject) but plants the RightTuple bit, so a
                // later mask-hit update revives it (wa_stale_ins_revive).
                if self.rules[ri].patterns[pos]
                    .acc
                    .as_ref()
                    .is_some_and(|a| a.window_time.is_some() || a.window_len.is_some())
                    && !self.winacc_admits(ni, f)
                {
                    self.trie[ni].clock_removed.insert(f);
                    continue;
                }
                self.trie[ni].active.insert(f);
                self.winlen_admit(ni, f); // D-185: length-window slot + eviction
                self.maybe_pulse(ni);
                // D-102 (survivor pre_lifo_then_post_arr): in EVENT
                // sessions, plain-join rights record their LINK-RELATIVE
                // generation — ph=4 when the path was unlinked at
                // staging (pre-link, incl. the link trigger itself);
                // plain ph=0 = post-link. The fire walk orders
                // pre-LIFO then post-ARRIVAL.
                self.stage_seq += 1;
                self.trie[ni].node.right_sseq.insert(f, self.stage_seq);
                if !self.event_specs.is_empty()
                    && matches!(self.trie[ni].node.kind, phreak::Kind::Join)
                    && !self.trie[ni].node.temporal
                    && !self.rule_linked(ri)
                {
                    self.trie[ni].node.s_right.add_ins_ph(f, origin, 4);
                } else {
                    self.trie[ni].node.s_right.add_ins(f, origin);
                }
                self.note_link_effects_ex(&mut was, Some(f));
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
        let mut acc_defer = false; // D-154/D-160: external update of an acc source (evented)
        let mut was: Vec<bool> = (0..self.rules.len()).map(|ri| self.rule_linked(ri)).collect();
        // D-094: within ONE fact-update, alpha ENTRIES and in-place
        // (mask-hit) updates process BEFORE alpha EXITS — Drools asserts
        // new entries during the OTN sink walk and defers exits
        // (unmatched previous tuples) to the end-of-modify drain
        // (ModifyPreviousTuples). The entry-before-exit window can
        // TRANSIENTLY all-link a path, creating+queueing its agenda
        // item (fz_7_2122 refined to within-update), whose D-091 pop
        // then drains held staging into memories mid-fire
        // (fz_min_2256's held right reaching memory in fire 1).
        // Pass A = entries + updates; pass B = exits; link bookkeeping
        // after every node event in both passes.
        for pass in 0..2u8 {
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
            if stage == 0 || (pass == 0) == (stage == 2) {
                continue; // pass A: entries/updates; pass B: exits
            }
            if stage == 1 {
                self.lias[li].active.insert(f);
                // D-170 (T6): the trigger fact alpha-ENTERED a pattern-0
                // alpha — the rules fed by this LIA become relocation-
                // eligible for this trigger (entry-of-f).
                if self.tj_trigger.is_some_and(|(tf, _, _)| tf == f) && pos == 0 {
                    for c in 0..self.lias[li].children.len() {
                        let ri2 = self.trie[self.lias[li].children[c]].env.0;
                        if !self.tj_entered.contains(&ri2) {
                            self.tj_entered.push(ri2);
                        }
                    }
                }
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
                    1 => {
                        self.stage_seq += 1;
                        self.trie[c].node.left_sseq.insert(smallvec![f], self.stage_seq);
                        self.trie[c].s0_in.add_ins(f, origin)
                    }
                    2 => self.trie[c].s0_in.add_del(f, origin),
                    _ => {
                        if self.trie[c].node.temporal {
                            self.stage_seq += 1;
                            let s = self.stage_seq;
                            if !self.trie[c].s0_in.upd.iter().any(|(x, _, _)| *x == f) {
                                self.trie[c].node.upd_lsseq.insert(smallvec![f], s);
                            }
                            if self.rules[self.trie[c].env.0].patterns.len() == 2 {
                                self.trie[c].node.pending_lmoves.push((smallvec![f], s));
                            }
                        }
                        self.trie[c].s0_in.add_upd(f, origin)
                    }
                }
            }
            self.note_link_effects_ex(&mut was, Some(f));
        }
        for ni in 0..self.trie.len() {
            let (ri, pos) = self.trie[ni].env;
            let pat = &self.rules[ri].patterns[pos];
            if pat.type_id != ftype || matches!(pat.sub, SubRole::Outer { .. }) {
                continue;
            }
            // D-154: WINDOWED accumulate nodes take updates via the
            // RightTuple entry machine (winacc_step) — external updates
            // DEFER to the flush points as queued entries evaluating the
            // epoch-final fields (the m1-m15 matrix); RHS modifies of a
            // windowed source (fuzz-unreachable) step immediately. The
            // D-137/D-139 arms below stay for every other node — for
            // windowed ones their machinery (incl. the clock_removed
            // guard and the bind_fields eff_mask) is subsumed by the
            // step's mask-gated revival/admission semantics.
            if pat.acc.as_ref().is_some_and(|a| a.window_time.is_some() || a.window_len.is_some()) {
                if pass == 0 {
                    if origin.is_none() {
                        acc_defer = true;
                    } else {
                        self.winacc_step(ni, f, mask, origin, &mut was);
                    }
                }
                continue;
            }
            // D-160: PLAIN accumulate nodes over an EVENT-TYPED source
            // defer external updates to the same per-entry drain (the
            // oracle executes each queued entry incrementally against the
            // epoch-final bean — updel/multiupd witnesses). Plain-typed
            // sources keep the immediate arms below (oracle-certified:
            // plain ops batch-annihilate, ap1/ap1b). RHS modifies step
            // immediately, mirroring the windowed arm.
            if pat.acc.is_some()
                && origin.is_none()
                && self.event_specs.contains_key(&ftype)
            {
                if pass == 0 {
                    acc_defer = true;
                }
                continue;
            }
            let was_in = self.trie[ni].active.contains(&f);
            let now = self.alpha_passes(ri, pos, f);
            if (pass == 0) == (was_in && !now) {
                continue; // pass A: entries/updates; pass B: exits (D-094)
            }
            match (was_in, now) {
                (false, true) if self.trie[ni].clock_removed.contains(&f) => {
                    // CEP E2 item C class 2 (D-137): a CLOCK-removed event
                    // (expiration-eager acc-removed — staged out of `active`
                    // by stage_acc_removal, still is_alive until the deferred
                    // drain) is NOT revived by an external update. Drools
                    // keeps it removed (xf_cep_c_upd_after_exp expiration).
                    // Without this, the re-entry below re-adds it to the
                    // accumulate (the count springs back). Leave it removed —
                    // a no-op entry. (The WINDOW-evicted variant now lives in
                    // winacc_step: eviction-detached events DO revive on a
                    // mask-HIT — D-154; this arm serves plain accumulates.)
                }
                (false, true) => {
                    self.trie[ni].active.insert(f);
                    self.maybe_pulse(ni);
                    // D-082/D-083: ph=1 marks a RE-ENTRY — the fact has
                    // a staged DEL at this node (left the alpha earlier
                    // in the same batch, out-and-back). Joins process
                    // those in a late pass (after left inserts, lefts
                    // walked newest-arrival-first; jr1..jr8/jr17 pins).
                    // PURE entries — any provenance — are ordinary
                    // right inserts (rightInserts slot, post-reorder
                    // memory walk): u12/u13/u16 + 4 fz counterexamples,
                    // jr10/jr11/jr16/jr18, fz_42_440. The D-083
                    // model-check survivor (tools/model_check_join2.py,
                    // 32 machines x 22 oracle timelines, unique); the
                    // same staged-del+staged-ins signature D-081 pinned
                    // for existential re-entries.
                    let reentry =
                        self.trie[ni].node.s_right.del.iter().any(|(x, _, _)| *x == f);
                    let ph = if reentry { 1 } else { 0 };
                    self.trie[ni].node.s_right.add_ins_ph(f, origin, ph);
                }
                (true, false) => {
                    self.trie[ni].active.remove(&f);
                    self.trie[ni].node.s_right.add_del(f, origin);
                }
                (true, true) => {
                    // ALL-SET mask (bare update) is class-reactive
                    // (fz_42_3311); property masks need intersection.
                    // CEP E2 item C class 1 (D-137): a POSITIVE temporal-join
                    // Behavior node is NOT property-reactive — Drools re-fires
                    // an after/before match on ANY external update of the event
                    // on the temporal (constraint-bearing) side, even a no-op
                    // value or an irrelevant field (xf_cep_c_upd_temporal; probe
                    // p_prober: prober update re-fires, p_anchor: the plain
                    // anchor/left input does NOT). Force the re-propagate for a
                    // temporal positive node regardless of the listen mask.
                    let temporal_refire =
                        pat.ce == CeKind::Positive && self.trie[ni].node.temporal;
                    // CEP E2 item C §1a (D-139): a WINDOWED accumulate is
                    // property-reactive on its source pattern's BINDINGS ONLY —
                    // the alpha-CONSTRAINT fields are dropped from the watch mask
                    // (with an intervening WindowNode, Drools gates the source
                    // modify on what the accumulate reads = the bound vars, not
                    // the alpha constraints). A PLAIN accumulate (and every
                    // join/not/exists) keeps the full listen mask (constraints ∪
                    // bindings). Probed over 28 oracle cells: windowed count()
                    // (no binding) never re-folds on ANY field update; windowed
                    // sum($v)/max($v) re-fold only when $v's field changes;
                    // constraint-field updates (tag=="y", val>5) do NOT re-fold
                    // even though listen_mask includes them (xf_cep_c_upd_win_
                    // {live,noop}). bind_fields is bindings-only, listen_mask is
                    // constraints∪bindings, so this drops exactly the constraints.
                    let eff_mask = if pat.acc.as_ref().is_some_and(|a| a.window_time.is_some() || a.window_len.is_some()) {
                        pat.bind_fields
                    } else {
                        pat.listen_mask
                    };
                    if mask == u64::MAX || eff_mask & mask != 0 || temporal_refire {
                        // D-170 (T6): a TAG-CLASS update — the trigger fact's
                        // type is the anchor (pattern-0) type and the written
                        // mask intersects the anchor's watch mask — stages
                        // ph=6: its child refires are MOVABLE (relocated by a
                        // later same-fact alpha-entry) and its memory move is
                        // invisible to the fact's own entry scan (self-slot).
                        // ts-only updates stage the anchored default.
                        let pat0 = &self.rules[ri].patterns[0];
                        let tagc = temporal_refire
                            && pat0.type_id == ftype
                            && (mask == u64::MAX || pat0.listen_mask & mask != 0);
                        // stamp keeps the FIRST staging's arrival (TupleSets
                        // keep-first: a deduped re-touch replays at the
                        // original action's slot — en3)
                        if self.trie[ni].node.temporal
                            && !self.trie[ni]
                                .node
                                .s_right
                                .upd
                                .iter()
                                .any(|(x, _, _)| *x == f)
                        {
                            self.stage_seq += 1;
                            let s = self.stage_seq;
                            self.trie[ni].node.upd_rsseq.insert(f, s);
                        }
                        // D-170 (T6, 2-pattern temporal): record ONE pending
                        // memory-move per update ACTION (dedup-proof — a
                        // re-touch of a staged upd appends another move,
                        // tu11x95); the replay applies them at their stamps
                        // so they interleave with staged inserts (tu11x92).
                        if self.trie[ni].node.temporal
                            && self.rules[ri].patterns.len() == 2
                        {
                            self.stage_seq += 1;
                            let s = self.stage_seq;
                            self.trie[ni].node.pending_rmoves.push((f, tagc, s));
                        }
                        if tagc {
                            self.trie[ni].node.s_right.add_upd_ph(f, origin, 6);
                        } else {
                            self.trie[ni].node.s_right.add_upd(f, origin);
                        }
                    } else {
                        // mask miss: immediate right-memory reAdd, no
                        // staging (fz_42_4359). Not nodes use the
                        // existential variant: blocked lefts re-search
                        // and unmatched ones stay DETACHED (D-031,
                        // NotNode.reorderRightTuple's null sink).
                        let env = JoinEnvImpl { store: &self.store, rule: &self.rules[ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
            self.note_link_effects_ex(&mut was, Some(f));
        }
        }
        if acc_defer {
            // ONE queue entry per external update call, carrying ITS OWN
            // written-mask (BfDump: no merging across a batch's entries);
            // the drain walks every accumulate node of the type.
            self.acc_pending.push((f, AccEntry::Upd(mask)));
        }
        self.tms_eager_break(f, false);
    }

    fn on_delete(&mut self, f: FactId, origin: Origin) {
        self.on_delete_ex(f, origin, false)
    }

    fn on_delete_ex(&mut self, f: FactId, origin: Origin, defer_acc: bool) {
        self.pn_on_wm_delete(f); // D-158: plain-not blocker shadows
        self.px_on_wm_delete(f); // D-162: plain-exists witness shadows
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
                self.note_link_effects_ex(&mut was, Some(f));
            }
        }
        for ni in 0..self.trie.len() {
            // D-160: an explicit external delete of an EVENT-TYPED acc
            // source executes its fold-out at the entry drain (its queue
            // position relative to deferred update entries), not here.
            if defer_acc
                && self.rules[self.trie[ni].env.0].patterns[self.trie[ni].env.1]
                    .acc
                    .is_some()
            {
                continue;
            }
            if self.trie[ni].active.remove(&f) {
                self.trie[ni].node.s_right.add_del(f, origin);
                self.note_link_effects_ex(&mut was, Some(f));
            }
        }
        self.tms_p_death_sweep(f, origin);
        self.tms_eager_break(f, true);
    }

    /// ⚖ t15 foreign-death sweep, LEAD lane (D-199, model t15_revive):
    /// the model's revive keys on the P DEATH ITSELF — a lead-k1
    /// justifier's dying child can ANNIHILATE in staging (ins+del fold)
    /// and never reach the terminal, so the parked-del lane misses the
    /// trigger (sdp7007x86: the first foreign delete revives in the
    /// oracle; by the second, the candidates are gone). On a foreign
    /// fact death, every LAZY plain non-ortwin LEAD-k1 justifier whose
    /// positive pattern ADMITS the dead fact's stale values (the
    /// value-level pmut gate — an alpha'd-out P's death never touches
    /// the node, ⚖ the starvation law) clears its parks; RECORDED
    /// full-width siblings re-activate in INSERTION order, bare
    /// prefixes simply stop suppressing (staged re-derivations queue
    /// at their consumption). TRAIL rules revive REVERSED-chain
    /// (sd_c1's certified order; the same staging annihilation starves
    /// their parked-del lane too — sdp7002x121, D-201); the parked-del
    /// lane remains for deaths that DO reach the terminal first.
    fn tms_p_death_sweep(&mut self, f: FactId, origin: Option<usize>) {
        if self.tms.parked.is_empty() {
            return;
        }
        let ftid = self.store.fact_type(f);
        for rj in 0..self.rules.len() {
            if !self.tms.parked.iter().any(|(pri, _)| *pri == rj) {
                continue;
            }
            if origin.is_some_and(|oi| self.rule_parents[oi] == self.rule_parents[rj]) {
                continue; // self-inflicted: the actor never revives its own
            }
            let eager = self.rules[rj].def.no_loop
                || matches!(self.rules[rj].salience, EngineSalience::Dyn { .. });
            let ortwin = (0..self.rules.len())
                .any(|rk| rk != rj && self.rule_parents[rk] == self.rule_parents[rj]);
            if eager || ortwin {
                continue;
            }
            let Some(pos) = self.rules[rj]
                .patterns
                .iter()
                .position(|p| p.ce == CeKind::Positive && p.type_id == ftid && p.tpos.is_some())
            else {
                continue;
            };
            // stale-value admit (the fact is already killed here — the
            // aliveness gate would always fail; D-160 fields variant)
            if !self.alpha_passes_fields(rj, pos, f) {
                continue;
            }
            let full_w = self.rules[rj]
                .patterns
                .iter()
                .filter_map(|p| p.tpos)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
            let entries: Vec<Tup> = self
                .tms
                .parked
                .iter()
                .filter(|(pri, _)| *pri == rj)
                .map(|(_, pt)| pt.clone())
                .collect();
            self.tms.parked.retain(|(pri, _)| *pri != rj);
            // re-add order by notpos: LEAD = insertion (the model's
            // land-lane law), TRAIL = reversed chain (sd_c1/gt16,
            // fz_42_5213). ⚠ the x63/x77/x33 lazy-trail tails want
            // the UNREVERSED list — three shapes, two orders, one
            // flat list = the ⚖ epicycle stop: the park list is too
            // flat for the model's phys history; the next move on
            // that corner is an SdDump of the per-round phys, not a
            // toggle (left open, D-202).
            let entries: Vec<Tup> = if self.tms_lead_k1(rj) {
                entries
            } else {
                entries.into_iter().rev().collect()
            };
            for pt in entries {
                if pt.len() < full_w
                    || pt.contains(&f)
                    || pt.iter().any(|x| !self.store.is_alive(*x))
                {
                    continue;
                }
                let k = self.rules[rj].patterns.len();
                let alphas = (0..k).all(|p2| {
                    let pat = &self.rules[rj].patterns[p2];
                    pat.sub == SubRole::Inner
                        || pat
                            .tpos
                            .map(|tp| tp < pt.len() && self.alpha_passes(rj, p2, pt[tp]))
                            .unwrap_or(true)
                });
                if !alphas {
                    continue;
                }
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS sweep-revive r{rj} {pt:?} (death f{f:?})");
                }
                self.push_activation(rj, pt);
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
        self.note_link_effects_ex(was, None);
    }

    fn note_link_effects_ex(&mut self, was: &mut [bool], cur: Option<FactId>) {
        for ri in 0..self.rules.len() {
            let now = self.rule_linked(ri);
            if was[ri] && !now {
                // doUnlinkRule: setDirty(true) + enqueue (D-032/D-091)
                self.nets[ri].queued = true;
                self.nets[ri].dirty = true;
            self.nets[ri].dirty_stamp = self.stage_seq;
                self.nets[ri].dirty_stamp = self.stage_seq;
            }
            if !was[ri] && now && self.in_expiration_drain {
                // D-102 (model-check survivor, ldrain_plain=nonflush):
                // an ADVANCE-triggered link (we're inside an expiration
                // batch — tms.expiring is non-empty exactly then) drains
                // held staged rights at PLAIN nodes into memory in
                // ARRIVAL order, no children (u3/v5 pins). Temporal
                // nodes never drain here — their unlinked inserts
                // self-drain at flush time (drain_t; t6/t14).
                for &ni in &self.nets[ri].path.clone() {
                    if !self.trie[ni].node.temporal {
                        let nidx = self.trie[ni].env.1 - 1;
                        let mut node = std::mem::replace(
                            &mut self.trie[ni].node,
                            phreak::Node::new_ex(phreak::Index::None, phreak::Kind::Join, false),
                        );
                        node.drain_staged_rights_to_memory_if(
                            &JoinEnvImpl { store: &self.store, rule: &self.rules[ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false },
                            nidx,
                            None,
                            &|f| self.store.is_alive(f),
                        );
                        self.trie[ni].node = node;
                    }
                }
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
        // P1c/D-089 subnetwork linking (staticDoLink/UnlinkRiaNode):
        // inner positions never gate the MAIN path directly; the outer
        // node gates per its kind — a subnet NOT links when the branch
        // cannot produce (sn_c7: fires with an inner alpha empty), a
        // subnet EXISTS waits for a producible branch (all inner alphas
        // populated) or live matches whose retracts still need a window.
        if pat.sub == SubRole::Inner {
            return true;
        }
        if let SubRole::Outer { len, .. } = pat.sub {
            if pat.ce == CeKind::Not {
                return true;
            }
            let node = &self.trie[self.nets[ri].path[pos - 1]];
            if !node.sn_matches.is_empty() || !node.sn_right.is_empty() {
                return true;
            }
            return (pos - len..pos).all(|ip| {
                let n = &self.trie[self.nets[ri].path[ip - 1]];
                let ipat = &self.rules[ri].patterns[ip];
                // inner bare nots do not require data (bare-CE analog)
                ipat.ce == CeKind::Not || !n.active.is_empty()
            });
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
    /// is currently linked. (An unlinked-but-was-linked rule does NOT
    /// re-queue mid-fire — its staged input accumulates and drains once
    /// at the fire boundary, D-084/pr_rl3.)
    fn refresh_linked(&mut self, ri: usize) {
        if self.rule_linked(ri) {
            self.nets[ri].ever_linked = true;
        }
        if self.rule_linked(ri) && self.rule_dirty(ri) {
            // SegmentMemory.notifyRuleLinkSegment -> queueRuleAgendaItem:
            // setDirty(true) on EVERY staging notify while linked, enqueue
            // if not queued (D-091).
            self.nets[ri].dirty = true;
            self.nets[ri].dirty_stamp = self.stage_seq;
            if !self.nets[ri].queued {
                self.nets[ri].queued = true;
            }
        }
    }

    fn rule_dirty(&self, ri: usize) -> bool {
        let net = &self.nets[ri];
        net.s0_dirty()
            || !net.term_pending.is_empty()
            || net.path.iter().any(|&ni| {
                let n = &self.trie[ni];
                !n.node.s_right.is_empty()
                    || !n.node.s_left.is_empty()
                    || !n.sn_right.is_empty()
                    // LIA children can sit at any path step of a group
                    // rule (the outer counting node is one, D-089)
                    || !n.s0_in.is_empty()
                    // D-134 (§3B): a released temporal-not deferral re-fires
                    || !n.node.pending_release.is_empty()
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
            // nothing staged anywhere: the evaluation is a no-op walk —
            // it still clears the executor flag (evaluateNetwork ->
            // setDirty(false), D-091)
            self.nets[ri].dirty = false;
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
            // Subnet-INNER positives are excluded too (D-089): their
            // tpos are subnet-branch slots that do not exist in rule
            // tuples.
            let positives: Vec<(usize, usize)> = self.rules[ri]
                .patterns
                .iter()
                .enumerate()
                .filter(|(_, p)| p.qce.is_none() && p.sub != SubRole::Inner)
                .filter_map(|(pos, p)| p.tpos.map(|t| (pos, t)))
                .collect();
            let pre = self.queue_top_sal(ri).unwrap_or(0);
            let n0 = self.nets[ri].queue.len();
            // ⚖ D-211/F2 (the dynamic law): an UNSTAGE-BORN fact's act
            // survives the j05 deactivation prune — the queued act
            // fires with the dead handle's values (b1/b2/4048).
            let ub = self.tms.unstage_born.clone();
            self.nets[ri].queue.retain(|a| {
                positives.iter().all(|(pos, ti)| {
                    alive[*pos].contains(&a.t[*ti]) || ub.contains(&a.t[*ti])
                })
            });
            if self.nets[ri].queue.len() != n0 {
                self.update_item_salience(ri, pre);
            }
            self.nets[ri].act_num.retain(|t, _| {
                positives.iter().all(|(pos, ti)| {
                    alive[*pos].contains(&t[*ti]) || ub.contains(&t[*ti])
                })
            });
        }
        // Agenda-item gate: only a queued item evaluates (the just-fired
        // rule is force-evaluated, fz_42_5243).
        if !force && !self.nets[ri].queued {
            return;
        }
        // evaluateNetworkIfDirty (D-091): the FLAG gates every
        // evaluation, force included — staging that arrived while the
        // path was UNLINKED never set it, so a queued-but-clean item
        // pops without draining (the faithful hold; fz_min_455's T1
        // drains at the deferred pop because the unlink transition
        // itself set the flag).
        if !self.nets[ri].dirty {
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
            self.tms.right_touched.clear(); // no CE side on LIA->terminal
            self.tms.joinr_touched.clear();
            for s0 in windows {
                for (f, o, _) in s0.del.iter().rev() {
                    if self.tms.unstage_born.contains(f) {
                        // ⚖ D-211/F2 THE DYNAMIC LAW (b1/b2/4048/7219/
                        // 6368): the delete of an UNSTAGE-BORN handle
                        // never cancels queued acts — they fire later
                        // with the dead handle's values. The k=1
                        // terminal consume skips whole (no deps/parks
                        // exist on a TMS-dropped handle). k>=2
                        // observers of unstage-born facts are outside
                        // the pinned envelope (ird-port-plan.md).
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS key[del-survive] f{f:?} r{ri}");
                        }
                        continue;
                    }
                    self.nets[ri].act_num.retain(|t, _| t[0] != *f);
                    let pre = self.queue_top_sal(ri).unwrap_or(0);
                    let n0 = self.nets[ri].queue.len();
                    self.nets[ri].queue.retain(|a| a.t[0] != *f);
                    if self.nets[ri].queue.len() != n0 {
                        self.update_item_salience(ri, pre);
                    }
                    self.tms_on_terminal_del(ri, &smallvec![*f]);
                    self.tms_parked_del(ri, &smallvec![*f], *o);
                }
                for (f, o, _) in s0.upd.iter().rev() {
                    let queued = self.nets[ri].queue.iter().any(|a| a.t[0] == *f);
                    if queued {
                        continue; // pending: keep position AND salience (se3)
                    }
                    if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                        continue; // own update does not re-activate (j04)
                    }
                    self.tms_unpark_upd(ri, &smallvec![*f]);
                    self.push_activation(ri, smallvec![*f]);
                }
                for (f, o, _) in s0.ins.iter().rev() {
                    if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                        continue;
                    }
                    if self.tms_parked_ins(ri, &smallvec![*f]) {
                        continue;
                    }
                    self.push_activation(ri, smallvec![*f]);
                }
            }
            self.nets[ri].dirty = false; // evaluateNetwork -> setDirty(false)
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
        // D-134 (§3B): this walk is a temporal-`not` deferral RELEASE iff some
        // path node holds a pending release. Computed once here (before the not
        // node consumes it) so a DOWNSTREAM join (not_mid's E2) skips the
        // is_expired partner filter for the whole released-left propagation.
        let releasing = self
            .nets[ri]
            .path
            .iter()
            .any(|&ni| !self.trie[ni].node.pending_release.is_empty());
        for step in 0..self.nets[ri].path.len() {
            let ni = self.nets[ri].path[step];
            let (env_ri, env_pos) = self.trie[ni].env;
            // Staging held across fire boundaries drains here as one
            // LIFO-merged batch — the hold/deferred-drain semantics are
            // carried by the D-091 dirty-flag lifecycle (the D-084
            // boundary-window plumbing is deleted, obsolete post-port).
            let (s0w, slw, srw) = (
                self.trie[ni].s0_in.take(),
                self.trie[ni].node.s_left.take(),
                self.trie[ni].node.s_right.take(),
            );
            let mut fresh: Staged<Tup> = Staged::default();
            if step == 0 {
                self.tms.left_touched.clear();
                self.tms.right_touched.clear();
                self.tms.joinr_touched.clear();
            }
            if !s0w.is_empty() {
                // pattern-0 fact staging: any LIA child on the path
                // drains its own copy (a group rule's outer counting
                // node is a level-1 child too, D-089)
                let s0 = s0w;
                self.tms.left_touched.extend(
                    s0.upd.iter().chain(s0.del.iter()).map(|(f, o, _)| (*f, *o)),
                );
                fresh.ins = s0.ins.into_iter().map(|(f, o, p)| (smallvec![f], o, p)).collect();
                fresh.upd = s0.upd.into_iter().map(|(f, o, p)| (smallvec![f], o, p)).collect();
                fresh.del = s0.del.into_iter().map(|(f, o, p)| (smallvec![f], o, p)).collect();
            }
            let pending = slw;
            let src = Staged::merge_into_pending(pending, fresh);
            let sr = srw;
            // Lane split (D-196): a JOIN's right is a POSITIVE pattern —
            // its own-origin ops are the LEFT lane (the update-break
            // class; in the LEAD topology P rides the join right, so
            // left_touched alone misses it — ip_c1). NOT-side ops are
            // the RIGHT lane (the self-defeat class, ip_a3).
            if matches!(
                self.trie[ni].node.kind,
                phreak::Kind::Not | phreak::Kind::SubnetNot
            ) {
                self.tms.right_touched.extend(
                    sr.ins
                        .iter()
                        .chain(sr.upd.iter())
                        .chain(sr.del.iter())
                        .map(|(f, o, _)| (*f, *o)),
                );
            } else {
                self.tms.joinr_touched.extend(
                    sr.upd
                        .iter()
                        .chain(sr.del.iter())
                        .map(|(f, o, _)| (*f, *o)),
                );
            }
            let sn_dirty = matches!(
                self.trie[ni].node.kind,
                phreak::Kind::SubnetNot | phreak::Kind::SubnetExists
            ) && !self.trie[ni].sn_right.is_empty();
            // D-134 (§3B): a pending temporal-not release makes the node
            // dirty even with no staged input — evaluate so it fires.
            if src.is_empty()
                && sr.is_empty()
                && !sn_dirty
                && self.trie[ni].node.pending_release.is_empty()
            {
                continue;
            }
            // Cross-window child clashes resolve against the FIRST
            // sink's pending at touch time (D-041) — take it out for
            // the node evaluation, then blind-append the batch (addAll).
            let first_sink = self.trie[ni].sinks.first().copied();
            let mut first_pending = match first_sink {
                Some(Sink::Node(c)) => self.trie[c].node.s_left.take(),
                Some(Sink::Term(rb)) => self.nets[rb].term_pending.take(),
                // RIA hop: clash folds happen at the sn_right staging
                // itself (doRiaNode2 removeInsert semantics)
                Some(Sink::Ria(_)) => Staged::default(),
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
                let is_gb = self.rules[env_ri].patterns[env_pos]
                    .acc
                    .as_ref()
                    .is_some_and(|a| a.key_field.is_some());
                trg = if is_gb {
                    self.eval_groupby_node(ni, env_ri, env_pos, src, sr, &mut first_pending)
                } else {
                    self.eval_acc_node(ni, env_ri, env_pos, src, sr, &mut first_pending)
                };
            } else if matches!(
                self.trie[ni].node.kind,
                phreak::Kind::SubnetNot | phreak::Kind::SubnetExists
            ) {
                trg = self.eval_subnet_node(ni, src, &mut first_pending);
            } else {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: releasing };
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
            // D-134 (§3B): a temporal not deferred these fresh lefts —
            // schedule each at its window-close fire_time; the quiescence
            // `drain_pending_fires` releases them (descending fire_time).
            if !self.trie[ni].node.new_deferrals.is_empty() {
                for (l, o, ft) in std::mem::take(&mut self.trie[ni].node.new_deferrals) {
                    self.fire_deadlines.entry(ft).or_default().push((ni, l, o, self.fire_seq));
                    self.fire_seq += 1;
                }
            }
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
                    Sink::Ria(c) => {
                        // RIA hop (D-089): stage the subnetwork batch
                        // into the outer counting node's rights with the
                        // pinned per-entry-prepend REVERSAL + TupleSets
                        // folds (doRiaNode2: a delete of a still-staged
                        // insert cancels outright — sn_c5b). All RIA
                        // sinks receive the same treatment (peer copies
                        // carry no extra flip).
                        for (t, o, _) in trg.ins.iter() {
                            self.trie[c].sn_right.add_ins(t.clone(), *o);
                        }
                        for (t, o, _) in trg.del.iter().chain(trg.norm_del.iter()) {
                            self.trie[c].sn_right.add_del(t.clone(), *o);
                        }
                        for (t, o, _) in trg.upd.iter() {
                            self.trie[c].sn_right.add_upd(t.clone(), *o);
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
        for (t, o, _) in src.del.iter() {
            if t.iter().any(|x| self.tms.unstage_born.contains(x) && !self.store.is_alive(*x)) {
                // ⚖ D-211/F2 THE DYNAMIC LAW (the general terminal
                // consume twin of the k=1 site): a retraction caused
                // by a dead UNSTAGE-BORN member leaves the queued act
                // to fire with the dead handle's values.
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS key[del-survive/term] r{ri} {t:?}");
                }
                continue;
            }
            self.nets[ri].act_num.remove(t);
            let pre = self.queue_top_sal(ri).unwrap_or(0);
            let n0 = self.nets[ri].queue.len();
            self.nets[ri].queue.retain(|a| a.t != *t);
            if self.nets[ri].queue.len() != n0 {
                self.update_item_salience(ri, pre);
            }
            self.tms_on_terminal_del(ri, t);
            self.tms_parked_del(ri, t, *o);
        }
        // D-170 (T6): an eval whose external trigger alpha-ENTERED this
        // rule's anchor consumes INSERTS before UPDATES (the model's
        // phase A-then-B: fresh refires land BEHIND the entry's ins
        // batch — tv1/vi3/dt3/ex10); every other eval keeps the
        // certified updates-then-inserts consume.
        let entry_eval = self.tj_trigger.is_some() && self.tj_entered.contains(&ri);
        if entry_eval {
            self.consume_term_ins(ri, &src, no_loop, parent);
            self.consume_term_upds(ri, &src, no_loop, parent);
        } else {
            self.consume_term_upds(ri, &src, no_loop, parent);
            self.consume_term_ins(ri, &src, no_loop, parent);
        }
        self.nets[ri].dirty = false; // evaluateNetwork -> setDirty(false)
    }

    /// PhreakSubnetworkNotExistsNode.doSubNetworkNode (P1c/D-089): the
    /// COUNTING machine — no blocker model. Each left keeps a matches
    /// list of live subnetwork tuples (correlated by START-tuple
    /// truncation to sn_plen); only count edges act: not fires at
    /// 0 matches (leftIns) and on ->0 (rightDel), exists on 0->1
    /// (rightIns); children die on the inverse edges. Phase order:
    /// leftDel, rightIns, leftIns, rightUpd (NO-OP — "here before,
    /// here now": in-place inner updates never refire, sn_b7/sn_e1),
    /// rightDel (deliberately late), leftUpd (child UPDATE -> refire,
    /// mask-gated upstream; sn_b10). Counting subsumes handover:
    /// support 2->1 = no refire, no cancel (sn_b6).
    fn eval_subnet_node(
        &mut self,
        ni: usize,
        src: Staged<Tup>,
        first_pending: &mut Staged<Tup>,
    ) -> Staged<Tup> {
        let sr = self.trie[ni].sn_right.take();
        let plen = self.trie[ni].sn_plen;
        let is_not = self.trie[ni].node.kind == phreak::Kind::SubnetNot;
        let mut trg: Staged<Tup> = Staged::default();
        let node = &mut self.trie[ni];
        let mut out = phreak::Out { trg: &mut trg, pending: first_pending };
        // --- leftDel ---
        for (l, o, _) in src.del.iter().chain(src.norm_del.iter()) {
            if let Some(i) = node.sn_lefts.iter().position(|x| x == l) {
                node.sn_lefts.remove(i);
            }
            if node.sn_has_child.remove(l) {
                out.child_del(l.clone(), *o);
            }
            node.sn_matches.remove(l);
        }
        // --- rightIns (before leftIns, "so 'not' knows if there are
        // matches before creating the child") ---
        for (s, o, _) in sr.ins.iter() {
            let p: Tup = Tup::from_slice(&s[..plen.min(s.len())]);
            let m = node.sn_matches.entry(p.clone()).or_default();
            if m.contains(s) {
                continue; // value-identity idempotency (re-delivered peer)
            }
            m.push(s.clone());
            if m.len() == 1 {
                if is_not {
                    if node.sn_has_child.remove(&p) {
                        out.child_del(p.clone(), *o);
                    }
                } else {
                    // exists 0->1: the child is created HERE, in the
                    // right walk, even for same-batch lefts (sn_a3 R3 /
                    // sn_b4 reverse-arrival pins)
                    if node.sn_has_child.insert(p.clone()) {
                        out.child_ins(p.clone(), *o, 1);
                    }
                }
            }
        }
        // --- leftIns ---
        for (l, o, _) in src.ins.iter() {
            node.sn_lefts.push(l.clone());
            let has_matches = node.sn_matches.get(l).is_some_and(|m| !m.is_empty());
            if is_not && !has_matches {
                if node.sn_has_child.insert(l.clone()) {
                    out.child_ins(l.clone(), *o, 0);
                }
            }
        }
        // --- rightUpd: NO-OP ("does nothing; here before, here now") ---
        // --- rightDel (late, so nothing staged here is then unstaged) ---
        for (s, o, _) in sr.del.iter() {
            let p: Tup = Tup::from_slice(&s[..plen.min(s.len())]);
            let Some(m) = node.sn_matches.get_mut(&p) else { continue };
            if let Some(i) = m.iter().position(|x| x == s) {
                m.remove(i);
            }
            if m.is_empty() {
                node.sn_matches.remove(&p);
                if !node.sn_lefts.contains(&p) {
                    continue; // left died too (deleteLeft nulled matches)
                }
                if is_not {
                    if node.sn_has_child.insert(p.clone()) {
                        out.child_ins(p.clone(), *o, 2);
                    }
                } else if node.sn_has_child.remove(&p) {
                    out.child_del(p.clone(), *o);
                }
            }
        }
        // --- leftUpd (very last) ---
        for (l, o, _) in src.upd.iter() {
            if node.sn_has_child.contains(l) {
                out.child_upd(l.clone(), *o, 2);
            }
        }
        trg
    }

    /// PhreakAccumulateNode.doNode (D-038): leftDel, rightDel, rightUpd,
    /// leftUpd, rightIns, leftIns; touched lefts collect into a temp set
    /// and results evaluate at the END (temp inserts head-first, then
    /// updates). Deletes REVERSE the stored per-match contribution;
    /// updates are reverse(stored)+accumulate(new); min/max reinit and
    /// refold when reverse is unsupported. The single result child per
    /// left reuses its synthetic fact, updating the value in place.
    /// D-108 groupby node (LEADING position — the left is the
    /// InitialFact tuple; joined groupby is fenced): per-key AccCtx,
    /// one child [l, rowfact] per live group; empty groups retract
    /// silently (ga9); any contributing change re-fires (ga8/ga15);
    /// re-keys migrate (ga8).
    fn eval_groupby_node(
        &mut self,
        ni: usize,
        env_ri: usize,
        env_pos: usize,
        src: Staged<Tup>,
        sr: Staged<FactId>,
        first_pending: &mut Staged<Tup>,
    ) -> Staged<Tup> {
        let spec = self.rules[env_ri].patterns[env_pos].acc.clone().unwrap();
        let kf = spec.key_field.unwrap();
        let mut trg: Staged<Tup> = Staged::default();
        let mut touched: Vec<String> = Vec::new();
        let mut touch = |k: String, touched: &mut Vec<String>| {
            if !touched.contains(&k) {
                touched.push(k);
            }
        };

        // left delete: the whole node clears
        for (l, o, _) in src.del.iter() {
            self.trie[ni].node.remove_left(l);
            if let Some(groups) = self.gb_state.remove(&ni) {
                for (_, g) in groups {
                    if g.propagated {
                        if let Some(r) = g.row {
                            let mut child = l.clone();
                            child.push(r);
                            if first_pending.remove_ins(&child) {
                                trg.norm_del.push((child, *o, 0));
                            } else {
                                first_pending.remove_upd(&child);
                                trg.add_del(child, *o);
                            }
                            self.store.kill(r);
                        }
                    }
                }
            }
        }

        // right deletes: reverse out of the fact's group
        for (f, _o, _) in sr.del.iter() {
            self.trie[ni].node.remove_right(*f);
            let groups = self.gb_state.entry(ni).or_default();
            let hit = groups
                .iter()
                .find(|(_, g)| g.ctx.matches.iter().any(|(rf, _)| rf == f))
                .map(|(k, _)| k.clone());
            if let Some(k) = hit {
                let g = groups.get_mut(&k).unwrap();
                let Some(i) = g.ctx.matches.iter().position(|(rf, _)| rf == f) else { continue };
                let (_, stored) = g.ctx.matches.remove(i);
                if !g.ctx.try_reverse(spec.func, *f, &stored) {
                    g.ctx.reset_state();
                    let remaining = g.ctx.matches.clone();
                    for (rf, vv) in &remaining {
                        g.ctx.apply(spec.func, *rf, vv);
                    }
                }
                touch(k, &mut touched);
            }
        }

        // right updates: possibly re-keyed — remove from the OLD group,
        // fold into the CURRENT-key group
        for (f, _o, _) in sr.upd.iter() {
            let groups = self.gb_state.entry(ni).or_default();
            let hit = groups
                .iter()
                .find(|(_, g)| g.ctx.matches.iter().any(|(rf, _)| rf == f))
                .map(|(k, _)| k.clone());
            if let Some(k) = hit {
                let g = groups.get_mut(&k).unwrap();
                if let Some(i) = g.ctx.matches.iter().position(|(rf, _)| rf == f) {
                    let (_, stored) = g.ctx.matches.remove(i);
                    if !g.ctx.try_reverse(spec.func, *f, &stored) {
                        g.ctx.reset_state();
                        let remaining = g.ctx.matches.clone();
                        for (rf, vv) in &remaining {
                            g.ctx.apply(spec.func, *rf, vv);
                        }
                    }
                }
                touch(k, &mut touched);
            }
            let key_v = self.store.value(*f, kf);
            let kk = scalar_canon_key(&key_v);
            let v = self.acc_contribution(&spec, *f);
            let groups = self.gb_state.entry(ni).or_default();
            let g = groups.entry(kk.clone()).or_insert_with(|| GbGroup {
                key: key_v,
                ctx: AccCtx::new(),
                row: None,
                propagated: false,
            });
            g.ctx.apply(spec.func, *f, &v);
            g.ctx.matches.push((*f, v));
            touch(kk, &mut touched);
        }

        // right inserts: fold into the key's group
        for (f, _o, _) in sr.ins.iter() {
            let key = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
                phreak::JoinEnv::key_of_right(&env, env_pos - 1, *f)
            };
            self.trie[ni].node.push_right(*f, key);
            let key_v = self.store.value(*f, kf);
            let kk = scalar_canon_key(&key_v);
            let v = self.acc_contribution(&spec, *f);
            let groups = self.gb_state.entry(ni).or_default();
            let g = groups.entry(kk.clone()).or_insert_with(|| GbGroup {
                key: key_v,
                ctx: AccCtx::new(),
                row: None,
                propagated: false,
            });
            g.ctx.apply(spec.func, *f, &v);
            g.ctx.matches.push((*f, v));
            touch(kk, &mut touched);
        }

        // left insert: register the left (groups may already exist from
        // this same batch's rights — they emit below)
        for (l, o, _) in src.ins.iter() {
            let lkey = {
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
                phreak::JoinEnv::key_of_left(&env, env_pos - 1, l)
            };
            self.trie[ni].node.push_left(l.clone(), lkey);
            let _ = o;
        }

        // result phase: per touched group against the (single) left
        let lefts = self.trie[ni].node.lefts_snapshot();
        let Some(l) = lefts.first().cloned() else {
            return trg;
        };
        let origin: Origin = None;
        for kk in touched.into_iter().rev() {
            let (empty, row, propagated, result) = {
                let groups = self.gb_state.entry(ni).or_default();
                let Some(g) = groups.get(&kk) else { continue };
                (
                    g.ctx.matches.is_empty(),
                    g.row,
                    g.propagated,
                    g.ctx.result_value(spec.func, spec.arg_ft),
                )
            };
            if empty {
                if propagated {
                    if let Some(r) = row {
                        let mut child = l.clone();
                        child.push(r);
                        if first_pending.remove_ins(&child) {
                            trg.norm_del.push((child, origin, 0));
                        } else {
                            first_pending.remove_upd(&child);
                            trg.add_del(child, origin);
                        }
                        self.store.kill(r);
                    }
                }
                self.gb_state.entry(ni).or_default().remove(&kk);
                continue;
            }
            let Some(res_v) = result else { continue };
            let key_v = self.gb_state.entry(ni).or_default()[&kk].key.clone();
            let r = match row {
                Some(r) => {
                    self.store.set_value(r, 0, res_v).expect("gb res set");
                    r
                }
                None => {
                    let r = self
                        .store
                        .insert(spec.result_tid, vec![res_v, key_v])
                        .expect("gb row insert");
                    self.gb_state.entry(ni).or_default().get_mut(&kk).unwrap().row = Some(r);
                    r
                }
            };
            let mut child = l.clone();
            child.push(r);
            if propagated {
                if first_pending.remove_ins(&child) {
                    trg.add_ins(child, origin);
                } else {
                    first_pending.remove_upd(&child);
                    trg.add_upd(child, origin);
                }
            } else {
                trg.add_ins(child, origin);
                self.gb_state.entry(ni).or_default().get_mut(&kk).unwrap().propagated = true;
            }
        }
        trg
    }

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
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
                phreak::JoinEnv::key_of_right(&env, node_idx, *f)
            };
            self.trie[ni].node.push_right(*f, key.clone());
            for l in self.trie[ni].node.lefts_bucket_pub(key.as_ref()) {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
                phreak::JoinEnv::key_of_left(&env, node_idx, l)
            };
            self.trie[ni].node.push_left(l.clone(), lkey.clone());
            self.trie[ni].acc.insert(l.clone(), AccCtx::new());
            for f in self.trie[ni].node.rights_bucket_pub(lkey.as_ref()) {
                let allowed = {
                    let env = JoinEnvImpl { store: &self.store, rule: &self.rules[env_ri], flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
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
                            if !matches!(
                                spec.func,
                                AccFunc::Collect | AccFunc::CollectList | AccFunc::CollectSet
                            ) {
                                self.store.set_value(r, 0, v).expect("acc result set");
                            }
                            r
                        }
                        None => {
                            let vals = if matches!(
                                spec.func,
                                AccFunc::Collect | AccFunc::CollectList | AccFunc::CollectSet
                            ) {
                                vec![]
                            } else {
                                vec![v]
                            };
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
                    if spec.func == AccFunc::CollectList {
                        let vals: Vec<Value> = self.trie[ni].acc[&l]
                            .vlist
                            .iter()
                            .map(|(_, v)| v.clone())
                            .collect();
                        self.collect_scalar_vals.insert(res, vals);
                    }
                    if spec.func == AccFunc::CollectSet {
                        let mut vals: Vec<Value> = self.trie[ni].acc[&l]
                            .vset
                            .iter()
                            .map(|(v, _)| v.clone())
                            .collect();
                        vals.sort_by_key(|v| scalar_canon_key(v));
                        self.collect_scalar_vals.insert(res, vals);
                    }
                    let mut child = l.clone();
                    child.push(res);
                    if propagated {
                        // propagateResult: normalizeStagedTuples against
                        // the first sink's pending, THEN addUpdate — a
                        // pending insert re-stages as an UPDATE here,
                        // unlike updateChildLeftTuple (D-041). The
                        // resolved insert keeps its kind for the FIRST
                        // sink only — every other sink resolves its OWN
                        // child and an already-consumed peer stages an
                        // UPDATE (D-071 kept-kind; D-085/xf_min_9976:
                        // dropping the marker ate the refire when the
                        // first sink was never-linked with the insert
                        // still pending).
                        if first_pending.remove_ins(&child) {
                            trg.peer_upd.push(child.clone());
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
        let qce = self.rules[env_ri].patterns[env_pos]
            .qce
            .clone()
            .expect("query node pattern has a qce spec");
        // D-107 (qm8/qm9/qm10): caller-side churn — leftDel retracts
        // the left's pulled rows; leftUpd = retract + FRESH re-pull
        // (the oracle re-fires with the new values as new activations).
        let site = (env_ri, env_pos);
        let mut pre: Staged<Tup> = Staged::default();
        for (t, o, ph) in src.del.iter().chain(src.norm_del.iter()) {
            if let Some(rows) = self
                .qce_children
                .get_mut(&site)
                .and_then(|m| m.remove(t))
            {
                for fid in rows {
                    let mut child = t.clone();
                    child.push(fid);
                    pre.seen_add(&child);
                    pre.del.push_back((child, *o, *ph));
                    self.store.kill(fid);
                }
            }
        }
        let mut upd_ins: Vec<(Tup, Origin, u8)> = Vec::new();
        for (t, o, ph) in src.upd.iter() {
            if let Some(rows) = self
                .qce_children
                .get_mut(&site)
                .and_then(|m| m.remove(t))
            {
                for fid in rows {
                    let mut child = t.clone();
                    child.push(fid);
                    pre.seen_add(&child);
                    pre.del.push_back((child, *o, *ph));
                    self.store.kill(fid);
                }
            }
            upd_ins.push((t.clone(), *o, *ph));
        }
        let mut src = src;
        for (t, _, _) in &upd_ins {
            src.seen_add(t); // D-266: fresh child tuples entering src.ins
        }
        src.ins.extend(upd_ins);
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
            self.qce_children
                .entry(site)
                .or_default()
                .entry(left.clone())
                .or_default()
                .push(fid);
            let mut child = left.clone();
            child.push(fid);
            children.push((child, *o, *ph));
        }
        if sink_count > 1 {
            children.reverse(); // QueryTupleSets.addTo re-reversal (D-056)
        }
        let mut trg: Staged<Tup> = pre;
        for (t, _, _) in &children {
            trg.seen_add(t); // D-266: direct ins assignment below
        }
        trg.ins = children.into();
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
    /// Terminal consume, UPDATE list (head-first). D-170 adds the T6
    /// relocation: during an entry-of-f eval, a queued activation staged
    /// MOVABLE by the same fact re-queues at the tail (behind the ins
    /// batch this eval already pushed) in re-emission order; everything
    /// else keeps position AND salience (se3; the u5 keep-first).
    fn consume_term_upds(
        &mut self,
        ri: usize,
        src: &phreak::Staged<Tup>,
        no_loop: bool,
        parent: usize,
    ) {
        let mut reloc: Vec<Tup> = Vec::new();
        for (t, o, _) in src.upd.iter() {
            if self.nets[ri].queue.iter().any(|a| a.t == *t) {
                if let Some((tf, _, _)) = self.tj_trigger {
                    if self.tj_entered.contains(&ri)
                        && self.nets[ri].act_movable.get(t) == Some(&tf)
                    {
                        reloc.push(t.clone());
                    }
                }
                continue;
            }
            if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                continue;
            }
            self.tms_unpark_upd(ri, t);
            // fired activation re-added: salience RE-EVALUATED (D-043)
            self.push_activation(ri, t.clone());
            self.tj_mark_movable(ri, t);
        }
        for t in reloc {
            let pre = self.queue_top_sal(ri).unwrap_or(0);
            self.nets[ri].queue.retain(|a| a.t != t);
            self.update_item_salience(ri, pre);
            self.push_activation(ri, t.clone());
            self.tj_mark_movable(ri, &t);
        }
    }

    /// Terminal consume, INSERT list (head-first).
    fn consume_term_ins(
        &mut self,
        ri: usize,
        src: &phreak::Staged<Tup>,
        no_loop: bool,
        parent: usize,
    ) {
        for (t, o, _) in src.ins.iter() {
            if no_loop && o.is_some_and(|oi| self.rule_parents[oi] == parent) {
                continue;
            }
            if self.tms_parked_suppress(ri, t) {
                continue;
            }
            self.push_activation(ri, t.clone());
        }
    }

    /// D-170 (T6): mark a just-queued activation MOVABLE when the
    /// in-flight external trigger is a TAG-CLASS update of the tuple's
    /// temporal-side fact on a 2-pattern temporal-positive join — a
    /// later alpha-entry of the same fact relocates it. ts-only
    /// triggers (and everything outside the tj shape) stay anchored.
    fn tj_mark_movable(&mut self, ri: usize, t: &Tup) {
        let Some((tf, mask, _)) = self.tj_trigger else { return };
        if t.last() != Some(&tf) {
            return;
        }
        let pats = &self.rules[ri].patterns;
        if pats.len() != 2 || pats[1].ce != CeKind::Positive {
            return;
        }
        let node_temporal = self.nets[ri]
            .path
            .first()
            .is_some_and(|&ni| self.trie[ni].node.temporal);
        if !node_temporal {
            return;
        }
        let pat0 = &pats[0];
        let tagc = pat0.type_id == self.store.fact_type(tf)
            && (mask == u64::MAX || pat0.listen_mask & mask != 0);
        if tagc {
            self.nets[ri].act_movable.insert(t.clone(), tf);
        }
    }

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
        // D-267: the pre-change top feeds update_item_salience, which
        // only reads it for DYN-salience rules (RuleExecutor.updateSalience,
        // D-043) — for static rules the full-queue scan was pure waste,
        // O(queue) per push = O(N²) per flush (the 60% flamegraph box).
        let pre = if matches!(self.rules[ri].salience, EngineSalience::Dyn { .. }) {
            self.queue_top_sal(ri).unwrap_or(0)
        } else {
            0
        };
        self.nets[ri].queue.push_back(Act { t, sal, seq });
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
        self.store.is_alive(f) && self.alpha_passes_fields(ri, pos, f)
    }

    /// D-160: the constraint/type/entry-point test WITHOUT the liveness
    /// gate — the acc entry drain evaluates a queued update entry whose
    /// fact a LATER Del entry retracts (entry-order aliveness; retracted
    /// facts' fields stay readable in the arena, matching the live Java
    /// bean Drools' queued entry executes against). Every other caller
    /// goes through `alpha_passes` (identical on alive facts).
    fn alpha_passes_fields(&self, ri: usize, pos: usize, f: FactId) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        // CEP E2 item D: a fact only feeds a pattern in the SAME entry point
        // (DEFAULT=0 for both untagged facts and plain patterns → no change
        // to the certified corpus). The single choke point for alpha/source
        // membership, so all routing (insert/update/delete/accumulate) and
        // node-sharing partition by entry point.
        if self.store.fact_type(f) != pat.type_id || self.fact_ep(f) != pat.entry_point {
            return false;
        }
        pat.cmps.iter().all(|c| {
            if let Test::Group { g, cross_var, .. } = &c.test {
                // cross-pattern groups evaluate at join time (D-073);
                // same-pattern/literal groups are alpha tests.
                return *cross_var
                    || eval_gexpr(g, &self.store, f, None, pat.tpos) == Some(true);
            }
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::IsNull { negated } => lhs.is_null() != *negated,
                Test::Unknown => false,
                Test::Cmp { op, rhs: Src::Lit(v) } => !lhs.is_null() && eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } if Some(*ti) == pat.tpos => {
                    eval_cmp_join(&lhs, *op, &self.store.value(f, *fi))
                }
                // join constraint, checked with prefix; SnapField never
                // occurs in LHS constraints
                Test::Cmp { .. } => true,
                Test::Temporal { .. } => true, // beta-only (D-101)
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    fn execute_rhs(&mut self, ri: usize, tuple: &[FactId]) -> Result<(), EngineError> {
        self.pn_seq += 1; // D-158: one stamp per RHS execution
        // D-076 refire-supersede prologue: snapshot this activation's
        // prior support keys; deps not re-established by THIS firing are
        // removed in the epilogue (fz_7777_112/74, dump-c).
        let tms_act = (ri, Tup::from_slice(tuple));
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
                        let rname = self.rules[ri].def.name.clone();
                        args.clone()
                            .iter()
                            .zip(schema.fields.iter())
                            .map(|(a, (_, ft))| {
                                let v = self.eval_cexpr(&rname, a, tuple, &snapshot)?;
                                coerce(v, *ft).ok_or_else(|| {
                                    EngineError("RHS insert: arg type mismatch".into())
                                })
                            })
                            .collect::<Result<_, _>>()?
                    };
                    let fid = self.store.insert(tid, values).map_err(EngineError)?;
                    self.schedule_expiration(fid);
                    self.schedule_window_evictions(fid);
                    self.tms_note_stated(fid);
                    let pre = self.stage_snapshot();
                    self.on_insert(fid, Some(ri));
                    self.flush_trigger_tid = Some(self.store.fact_type(fid));
                    self.stream_flush(&pre);
                    self.flush_trigger_tid = None;
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
                    self.tms_insert_logical(ri, &Tup::from_slice(tuple), tid, values)?;
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
                CompiledAction::SetFocus { group } => {
                    // D-106 (ag9): relocate-or-push — the group moves
                    // to the TOP of the focus stack
                    let g = group.clone();
                    self.focus_stack.retain(|x| x != &g);
                    self.focus_stack.push(g);
                    self.focus_changed = true;
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
                            // ⚖ D-211 L6-EVENT at the refire-supersede
                            // epilogue too (fz_7_9902: the per-epoch
                            // refire drops the stale dep HERE): the key
                            // dies whole, stated siblings orphan, a
                            // later re-justification re-keys fresh.
                            let sibs: Vec<FactId> = e.stated.drain(..).collect();
                            for sb in sibs {
                                self.tms.by_fact.remove(&sb);
                                self.tms.orphans.insert(sb);
                            }
                            self.tms.keys.remove(key);
                        } else if e.pending_vals.is_some() {
                            // ⚖ D-211 pending-clear (c2/c3/c4 law).
                            e.pending_vals = None;
                            if e.stated.is_empty() {
                                self.tms.keys.remove(key);
                            }
                        }
                    }
                }
            }
            // D-158: these WM deletes are the CHURN class — a re-fire's
            // stale-key retract, which Drools stages synchronously BEFORE
            // the re-fire's queued insertLogical reaches the not.
            self.pn_churn_ctx = true;
            for jf in to_retract {
                if self.store.is_alive(jf) {
                    self.store.kill(jf);
                    self.on_delete(jf, None);
                }
            }
            self.pn_churn_ctx = false;
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

    /// D-283 Tier 1: evaluate a compiled RHS arithmetic expression with
    /// JAVA semantics (PINS.md §A): i64 wraps on overflow (MIN/-1 wraps
    /// to MIN, MIN%-1 is 0 — Long semantics), `/` truncates, `%` keeps
    /// the dividend's sign, division by zero errors ("/ by zero", the
    /// oracle's ArithmeticException text — the diff judge treats
    /// both-sides-"/ by zero" as agreement); any f64 operand promotes
    /// both sides to IEEE doubles (x/0.0 -> ±inf, 0.0/0.0 -> NaN).
    fn eval_cexpr(
        &self,
        rname: &str,
        e: &CExpr,
        tuple: &[FactId],
        snapshot: &[Vec<Value>],
    ) -> Result<Value, EngineError> {
        let num = |v: Value| -> (Option<i64>, f64) {
            match v {
                Value::I64(n) => (Some(n), n as f64),
                Value::F64(x) => (None, x),
                // unreachable: compile_cexpr admits numeric non-nullable
                // operands only
                other => unreachable!("non-numeric in RHS arithmetic: {other:?}"),
            }
        };
        match e {
            CExpr::Atom(s) => Ok(self.eval_src(s, tuple, snapshot)),
            CExpr::Neg(a) => {
                let v = self.eval_cexpr(rname, a, tuple, snapshot)?;
                Ok(match num(v) {
                    (Some(n), _) => Value::I64(n.wrapping_neg()),
                    (None, x) => Value::F64(-x),
                })
            }
            CExpr::Bin(op, a, b) => {
                let va = self.eval_cexpr(rname, a, tuple, snapshot)?;
                let vb = self.eval_cexpr(rname, b, tuple, snapshot)?;
                match (num(va), num(vb)) {
                    ((Some(x), _), (Some(y), _)) => {
                        if matches!(op, '/' | '%') && y == 0 {
                            return Err(EngineError(format!(
                                "rule {rname:?}: java.lang.ArithmeticException: / by zero"
                            )));
                        }
                        Ok(Value::I64(match op {
                            '+' => x.wrapping_add(y),
                            '-' => x.wrapping_sub(y),
                            '*' => x.wrapping_mul(y),
                            '/' => x.wrapping_div(y),
                            '%' => x.wrapping_rem(y),
                            _ => unreachable!("parser admits + - * / %"),
                        }))
                    }
                    ((_, x), (_, y)) => Ok(Value::F64(match op {
                        '+' => x + y,
                        '-' => x - y,
                        '*' => x * y,
                        '/' => x / y,
                        '%' => x % y,
                        _ => unreachable!("parser admits + - * / %"),
                    })),
                }
            }
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
        if !self.tms.activated {
            // ⚖ D-211 activation-backfill: pre-activation stateds are
            // keyless until the first insertLogical (d1/d2 dumps: two
            // keys, one value — only the LAST backfills into the map).
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS key[stated-note/pre] f{f:?}");
            }
            self.tms.pre_stated.push(f);
            return;
        }
        let key = self.tms_key_of(tid, f);
        self.tms.keys.entry(key.clone()).or_default().stated.push(f);
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            eprintln!("TMS key[stated-note] f{f:?} key={key:?}");
        }
        self.tms.by_fact.insert(f, key);
    }

    /// ⚖ D-211: TMS activation — the first insertLogical backfills the
    /// pre-activation stated facts; the LAST one per value becomes the
    /// mapped entry's member (earlier ones stay keyless: their deletes
    /// are ordinary, matching the oracle's unmapped singleton keys).
    fn tms_activate(&mut self) {
        if self.tms.activated {
            return;
        }
        self.tms.activated = true;
        let pre = std::mem::take(&mut self.tms.pre_stated);
        for f in pre {
            if !self.store.is_alive(f) {
                continue;
            }
            let tid = self.store.fact_type(f);
            let key = self.tms_key_of(tid, f);
            let e = self.tms.keys.entry(key.clone()).or_default();
            for prev in e.stated.drain(..) {
                self.tms.by_fact.remove(&prev);
            }
            e.stated.push(f);
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS key[backfill] f{f:?} key={key:?}");
            }
            self.tms.by_fact.insert(f, key);
        }
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
        self.tms_activate();
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
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            let e = self.tms.keys.get(&key).expect("key");
            eprintln!(
                "TMS key[logical] key={:?} need_insert={} pending={} beliefs={} stated={:?} justified={:?}",
                key, need_insert, e.pending_vals.is_some(), e.beliefs.len(),
                e.stated, e.justified
            );
        }
        self.tms.firing_keys.push(key.clone());
        // ⚖ D-195/D-196 (the RHS-order race, engine translation): a
        // MUTFIRST consequence mutated its own tuple BEFORE this
        // insertLogical — a tuple member's alpha already fails on the
        // LIVE fields — so the dep attaches LATE: its teardown rides to
        // the item's pop (the zombie window gt13/ip_c1 observers see).
        // An ILFIRST dep attaches while the tuple is whole and dies at
        // the flush (pr_tms_t20d / pr_tms_selfbreak_flush certified;
        // x147's oracle twin). D-201 (sdp7007x98, the del-lane window):
        // a tuple member DELETED before the attach is the same race,
        // del flavor — the last generation's LK rides to the pop and
        // strictly-higher observers glimpse it once (the model's
        // x88/x0 windows).
        let late = act.1.iter().any(|f2| {
            if !self.store.is_alive(*f2) {
                return true;
            }
            self.rules[ri].patterns.iter().enumerate().any(|(pos, pat)| {
                pat.sub != SubRole::Inner
                    && pat.tpos.map(|t| act.1.get(t) == Some(f2)).unwrap_or(false)
                    && !self.alpha_passes(ri, pos, *f2)
            })
        });
        if late && !self.tms.late_acts.iter().any(|a| *a == act) {
            self.tms.late_acts.push(act.clone());
        }
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
        if self.tms.orphans.contains(&f) {
            // ⚖ D-211 (x1/r1/c5 events): an orphaned handle is
            // UNDELETABLE — the delete silently no-ops.
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS key[route-del/orphan-noop] f{f:?}");
            }
            return (None, None);
        }
        let Some(key) = self.tms.by_fact.get(&f).cloned() else {
            return (Some(f), None); // not a logical-type fact: normal delete
        };
        let Some(e) = self.tms.keys.get_mut(&key) else {
            return (Some(f), None);
        };
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            eprintln!(
                "TMS key[route-del] f{:?} rhs={} stated={:?} beliefs={} pending={} justified={:?} had_justified={}",
                f, rhs, e.stated, e.beliefs.len(), e.pending_vals.is_some(),
                e.justified, e.had_justified
            );
        }
        if let Some(jf) = e.justified {
            e.justified = None;
            e.beliefs.clear();
            self.tms.by_fact.remove(&jf);
            // ⚖ D-211 (c5 + the irdp6003x128 residual): deleting the
            // WM belief kills the key WHOLE — coexisting stated
            // siblings ORPHAN (undeletable, preserving the dump3
            // no-op observable) and a LATER stated insert of the
            // value re-keys FRESH and DELETABLE (the oracle's
            // key-death; the old key-survives+had_justified
            // approximation leaked undeletability onto post-death
            // inserts). Zero-sibling case = fz_42_1395 unchanged.
            let sibs: Vec<FactId> = e.stated.drain(..).collect();
            for sb in sibs {
                self.tms.by_fact.remove(&sb);
                self.tms.orphans.insert(sb);
            }
            for (_, keys) in self.tms.by_act.iter_mut() {
                keys.retain(|k| *k != key);
            }
            self.tms.by_act.retain(|(_, keys)| !keys.is_empty());
            self.tms.keys.remove(&key);
            return (Some(jf), None);
        }
        if e.had_justified {
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS key[route-del/dump3-noop] f{f:?}");
            }
            return (None, None); // dump3: undeletable stated sibling
        }
        // ⚖ D-211 THE R1-EVENT (r1/d1/d2/8757; b1 = the 0-sibling
        // case): an RHS stated-delete of a pending-mixed key kills the
        // named handle, ORPHANS the remaining stateds, UNSTAGES the
        // pending belief, and the key dies WHOLE. Externals keep the
        // old path (dump8: no materialization). The beliefs gate keeps
        // a cleared-pending zombie from unstaging (c2/c3).
        if rhs && e.stated.contains(&f) && e.pending_vals.is_some() && !e.beliefs.is_empty() {
            let vals = e.pending_vals.take().expect("gated Some");
            let sibs: Vec<FactId> = e.stated.iter().copied().filter(|x| *x != f).collect();
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!(
                    "TMS key[route-del/r1-event] f{f:?} orphans={sibs:?} vals={vals:?}"
                );
            }
            self.tms.by_fact.remove(&f);
            for sb in &sibs {
                self.tms.by_fact.remove(sb);
                self.tms.orphans.insert(*sb);
            }
            for (_, keys) in self.tms.by_act.iter_mut() {
                keys.retain(|k| *k != key);
            }
            self.tms.by_act.retain(|(_, keys)| !keys.is_empty());
            self.tms.keys.remove(&key);
            return (Some(f), Some((key.0, vals)));
        }
        e.stated.retain(|x| *x != f);
        self.tms.by_fact.remove(&f);
        if rhs && e.stated.is_empty() && !e.beliefs.is_empty() {
            if let Some(vals) = e.pending_vals.take() {
                // unstage (dump7): the pending justified belief becomes
                // a live fact after the stated handle dies; its deps
                // are already in place.
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS key[route-del/unstage] f{f:?} vals={vals:?}");
                }
                return (Some(f), Some((key.0, vals)));
            }
        }
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            eprintln!(
                "TMS key[route-del/plain] f{:?} beliefs-cleared={} stated-remainder={:?} pending-still={}",
                f, e.beliefs.len(), e.stated, e.pending_vals.is_some()
            );
        }
        // ⚖ D-211: the tms_e6 clear is scoped to the key actually
        // dying — the D-210-pinned mis-scope wiped beliefs on NON-LAST
        // stated deletes and starved the unstage gate.
        if e.stated.is_empty() {
            e.beliefs.clear(); // stated-only key dies with its handles (tms_e6)
            for (_, keys) in self.tms.by_act.iter_mut() {
                keys.retain(|k| *k != key);
            }
            self.tms.by_act.retain(|(_, keys)| !keys.is_empty());
            if self.tms.keys.get(&key).map(|e| e.justified.is_none() && e.stated.is_empty()).unwrap_or(false) {
                self.tms.keys.remove(&key);
            }
        }
        (Some(f), None)
    }

    /// Materialize an unstaged justified belief (dump7): insert the
    /// pending values as the key's justified handle.
    fn tms_materialize(&mut self, tid: TypeId, vals: Vec<Value>) -> Result<(), EngineError> {
        let key = (tid, key_vals(&vals));
        let f = self.store.insert(tid, vals).map_err(EngineError)?;
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            eprintln!("TMS key[materialize] key={key:?} f{f:?}");
        }
        // ⚖ D-211: the unstage-born handle is fully TMS-DROPPED (the
        // oracle dumps show @5 leaving the map — 4048/d1); no entry
        // update, no by_fact. Its later delete is ordinary WM removal
        // and (the dynamic law, F2) must not cancel queued acts.
        self.tms.unstage_born.insert(f);
        self.tms.force_eval.push(f);
        if let Some(e) = self.tms.keys.get_mut(&key) {
            e.justified = Some(f);
            e.had_justified = true;
            self.tms.by_fact.insert(f, key);
        }
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
    fn tms_eager_break(&mut self, f: FactId, from_delete: bool) {
        if self.tms.by_act.is_empty() {
            return;
        }
        // D-177 (the LANDING LAW): delete-sourced teardowns land by
        // mode x cause. In a STREAM session an EXPLICIT delete's
        // teardown lands at the delete's PROPAGATION for k>=2 acts too
        // (external: at the action — hm1/hm1b + tju_spin_deps_
        // {extdel,delpartner}; RHS: at the firing — hm2b), so the k=1
        // scope below LIFTS. Expiration keeps its lazy row
        // (in_expiration_drain; q1/q4/a7c), and update-sourced breaks
        // (from_delete=false) keep the certified lazy path (unprobed
        // on the staircase instrument).
        let stream_del_land = from_delete
            && !self.in_expiration_drain
            && self.event_specs.contains_key(&self.store.fact_type(f));
        let broken: Vec<(usize, Tup)> = self
            .tms
            .by_act
            .iter()
            .filter(|((ri, tuple), _)| {
                // ⚖ D-211/F3 (the rule-shape law, D-208 s2): a
                // justifier breaking its OWN tuple mid-firing lands
                // LAZY only when its LHS is a SELF-JOIN on the broken
                // fact's type (>=2 tuple facts of that type — m3/m6/m7/
                // s2, fz_42_2442's shape); a SINGLE-BINDING same-batch
                // self-break lands EAGERLY like a foreign one (m1/m2/
                // m5, fz_777_2956 + fz_7_1591 + fz_7_5988: no act on
                // the belief ever fires).
                if self.tms.current_act.as_ref() == Some(&(*ri, tuple.clone())) {
                    let ftid = self.store.fact_type(f);
                    let same_type = tuple
                        .iter()
                        .filter(|x| self.store.fact_type(**x) == ftid)
                        .count();
                    if same_type >= 2 {
                        return false;
                    }
                }
                // In CLOUD, eager teardown reaches the terminal DIRECTLY
                // only for k=1 justifiers (LIA->terminal); k>=2 tuples
                // die via staged network propagation = the LAZY path
                // (min3783: a witness fires on the transient between a
                // join-justifier's tuple-fact delete and its item's
                // evaluation, exactly like t11/t12). Stream explicit
                // deletes take the D-177 eager landing instead.
                if !self.nets[*ri].path.is_empty() && !stream_del_land {
                    return false;
                }
                tuple.contains(&f) && {
                    let dead = !self.store.is_alive(f);
                    dead || {
                        // alpha re-check of f's own slots only
                        self.rules[*ri].patterns.iter().enumerate().any(|(pos, pat)| {
                            pat.sub != SubRole::Inner
                                && pat.tpos.map(|t| tuple[t] == f).unwrap_or(false)
                                && !self.alpha_passes(*ri, pos, f)
                        })
                    }
                }
            })
            .map(|(a, _)| a.clone())
            .collect();
        for act in broken {
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS eager-break act r{} {:?} expiring={}", act.0, act.1, act.1.iter().any(|x| self.tms.expiring.contains(x)));
            }
            // D-101 (a7c/a7d/cf5x0): an EXPIRING justifier's teardown is
            // LAZY — it rides the certified tms.deferred list and drains
            // at the justifier's ITEM POP (salience/decl agenda order),
            // exactly like k>=2 walk-path teardowns. External deletes
            // keep the certified EAGER path (the a7d delete twin).
            // D-175/D-176: same cause split as tms_on_terminal_del —
            // lazy is the drain's own prunes or an all-alive scheduled
            // act; a flag-false break with a DEAD member is an external
            // delete inside the mark window and stays on the a7d eager
            // path (spin_deps_k1 pin — the k=1 D-117 family flavor).
            if self.in_expiration_drain
                || (act.1.iter().any(|x| self.tms.expiring.contains(x))
                    && act.1.iter().all(|x| self.store.is_alive(*x)))
            {
                if !self.tms.exp_deferred.iter().any(|(r, t)| (*r, t) == (act.0, &act.1)) {
                    self.tms.exp_deferred.push((act.0, act.1));
                }
                continue;
            }
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
        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
            eprintln!(
                "TMS terminal-del r{ri} {tuple:?} expiring={:?} exp_deferred={:?} deferred={}",
                self.tms.expiring,
                self.tms.exp_deferred,
                self.tms.deferred.len()
            );
        }
        // D-101/D-102 (cf5x17 second bite): an EXPIRING justifier's
        // teardown is LAZY — reroute to the certified tms.deferred
        // item-pop path. This covers the DIRECT prune callers (queue
        // pruning during advance()'s deletes), which bypass the
        // eager-break scan's routing.
        // D-175/D-176 (tju_359_spin_min / spin_deps_{extdel,delpartner}
        // pins): lazy is the EXPIRATION cause only — the drain's own
        // prunes (in_expiration_drain) and mid-fire consumes of a
        // scheduled fact that is STILL ALIVE (q1). A flag-false report
        // with a DEAD member means an external delete killed the tuple
        // inside the mark window: Drools tears its beliefs down eagerly
        // at the propagation (the a7d cause split, oracle-pinned 3x at
        // both corners), and the pending expiration later no-ops on the
        // dead handle. The old mark-only check re-added the entry the
        // post-fire drain had just handed it — the D-117 re-add cycle.
        if self.in_expiration_drain
            || (tuple.iter().any(|f| self.tms.expiring.contains(f))
                && tuple.iter().all(|f| self.store.is_alive(*f)))
        {
            if !self.tms.exp_deferred.iter().any(|(r, t)| (*r, t) == (ri, tuple)) {
                self.tms.exp_deferred.push((ri, tuple.clone()));
            }
            return;
        }
        // an act already PENDING as a lazy entry stays lazy — the fire
        // walk's window consume re-reports the same terminal-del after
        // the expiring marks are cleared (q1: the teardown must wait
        // for a firing pop or quiescence, not run at the re-report)
        if self.tms.exp_deferred.iter().any(|(r, t)| (*r, t) == (ri, tuple)) {
            return;
        }
        if self.tms.by_act.is_empty() {
            return;
        }
        let act = (ri, tuple.clone());
        if !self.tms.by_act.iter().any(|(a, _)| *a == act) {
            return;
        }
        if self.tms.defer_mode {
            if !self.tms.deferred.iter().any(|(r, t, _)| (*r, t) == (act.0, &act.1)) {
                let left = act.1.iter().any(|f| {
                    self.tms
                        .left_touched
                        .iter()
                        .any(|(lf, lo)| lf == f && *lo == Some(act.0))
                });
                let right = self
                    .tms
                    .right_touched
                    .iter()
                    .any(|(_, ro)| *ro == Some(act.0));
                let late = self.tms.late_acts.iter().any(|a| (*a).0 == act.0 && a.1 == act.1);
                let joinr = act.1.iter().any(|f| {
                    self.tms
                        .joinr_touched
                        .iter()
                        .any(|(jf, jo)| jf == f && *jo == Some(act.0))
                });
                let mut flags = (left as u8)
                    | ((right as u8) << 1)
                    | ((late as u8) << 2)
                    | ((joinr as u8) << 3);
                // ⚖ D-201 (sdp7004x51, model composite re-route): an
                // EAGER MUTFIRST composite's (bit1+bit2) key with NO
                // SURVIVORS — no alive fact still passing the positive
                // pattern's alpha (all pmut'd/deleted) — is the run's
                // LAST key: it rides to the POP (bit4). Mid-run keys
                // land at flush/selection boundaries as certified.
                if flags & 6 == 6
                    && (self.rules[act.0].def.no_loop
                        || matches!(self.rules[act.0].salience, EngineSalience::Dyn { .. }))
                {
                    let init_tid = self.store.type_id(INITIAL_FACT);
                    let pos = self.rules[act.0].patterns.iter().position(|p| {
                        p.ce == CeKind::Positive
                            && Some(p.type_id) != init_tid
                            && p.tpos.is_some()
                    });
                    let survivors = pos.is_some_and(|pp| {
                        self.store
                            .all_facts_in_insertion_order()
                            .any(|f| self.alpha_passes(act.0, pp, f))
                    });
                    if !survivors {
                        flags |= 16;
                    }
                }
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS defer-push r{} {:?} flags={}", act.0, act.1, flags);
                }
                self.tms.deferred.push((act.0, act.1, flags));
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
                    let env = JoinEnvImpl { store: &self.store, rule, flush: self.in_stream_flush, fire_no: self.fire_no, not_releasing: false };
                    pat.cmps.iter().all(|c| {
                        if let Test::Group { g, cross_var, .. } = &c.test {
                            return *cross_var
                                || eval_gexpr(g, &self.store, f, None, pat.tpos)
                                    == Some(true);
                        }
                        let lhs = self.store.value(f, c.field_idx);
                        match &c.test {
                            Test::IsNull { negated } => lhs.is_null() != *negated,
                            Test::Unknown => false,
                            Test::Cmp { op, rhs: Src::Lit(v) } => !lhs.is_null() && eval_cmp(&lhs, *op, v),
                            Test::Cmp { .. } => true,
                            Test::Temporal { .. } => true, // beta: JoinEnv::allowed re-checks
                            other => eval_alpha_test(&lhs, other),
                        }
                    }) && phreak::JoinEnv::allowed(&env, pos - 1, tuple, f)
                }
            });
            if self_blocker {
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS park-own r{ri} {tuple:?}");
                }
                self.tms.parked.push((ri, tuple.clone()));
                // Drools leaks the WHOLE blocked list of the dying
                // blocker (tms_t21: sibling tuples blocked by the same
                // self-defeat fact stay parked too, firing once not
                // per-tuple). OR-SIBLINGS share the one Drools item —
                // their blocked lefts park too (D-198, sd_b4: the twin
                // branch fires once, not per-branch).
                let par = self.rule_parents[ri];
                let group: Vec<usize> = (0..self.rules.len())
                    .filter(|&rj| self.rule_parents[rj] == par)
                    .collect();
                for gri in group {
                let ri = gri;
                for pos in 0..self.rules[ri].patterns.len() {
                    let pat = &self.rules[ri].patterns[pos];
                    if pat.ce != CeKind::Not || pat.type_id != ftid {
                        continue;
                    }
                    // D-198 (sd_b2): find the not's node by env — a
                    // LEAD not's blocked left is the short prefix tuple
                    // (e.g. the InitialFact) and parks as a PREFIX
                    // (tms_parked_ins matches by starts_with). Match by
                    // DEPTH, not creator (D-199, sd_b4): a shared node
                    // (or-twins, equal-prefix rules) carries its FIRST
                    // owner's env, but sharing preserves depth — each
                    // sharer's pattern `pos` is this path's node with
                    // env.1 == pos.
                    let Some(&ni) = self
                        .nets[ri]
                        .path
                        .iter()
                        .find(|&&ni| self.trie[ni].env.1 == pos)
                    else {
                        continue;
                    };
                    if let Some(lefts) = self.trie[ni].node.blocked_of(f) {
                        for lt in lefts {
                            // the in-firing block CANCELS queued
                            // activations extending this left (sd_b4:
                            // the twin's original activation dies with
                            // the block, before any un-break)
                            self.nets[ri].queue.retain(|a| !a.t.starts_with(&lt));
                            self.nets[ri].act_num.retain(|t, _| !t.starts_with(&lt));
                            if !self.tms.parked.iter().any(|(r, t)| *r == ri && *t == lt) {
                                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                                    eprintln!("TMS park-leak r{ri} {lt:?} (blocker f{f:?})");
                                }
                                self.tms.parked.push((ri, lt));
                            }
                        }
                    }
                }
                }
            }
        }
    }

    /// ⚖ land_eager lead-k1 (D-199, model_sd land_eager / sd_d3-d5 law):
    /// an EAGER (no-loop) justifier with exactly one plain NOT strictly
    /// upstream of exactly one positive join — the flush-time unbreak of
    /// the upstream not RE-PROPAGATES: parked tuples re-derive as new
    /// objects and re-fire (sdp7002x4-class: one firing per P). Lazy
    /// landings never self-revive (sd_b2's park holds), and the mutfirst
    /// last key rides to the POP and lands lazy (no rederive), so the
    /// unpark runs at the eager drain sites only.
    fn tms_lead_k1(&self, ri: usize) -> bool {
        let init_tid = self.store.type_id(INITIAL_FACT);
        let (mut not_pos, mut pos_pos) = (None, None);
        let (mut positives, mut nots) = (0usize, 0usize);
        for (i, p) in self.rules[ri].patterns.iter().enumerate() {
            if p.acc.is_some() || p.qce.is_some() || !matches!(p.sub, SubRole::None) {
                return false;
            }
            if Some(p.type_id) == init_tid && p.ce == CeKind::Positive {
                continue;
            }
            match p.ce {
                CeKind::Positive => {
                    positives += 1;
                    pos_pos = Some(i);
                }
                CeKind::Not => {
                    nots += 1;
                    not_pos = Some(i);
                }
                CeKind::Exists => return false,
            }
        }
        positives == 1 && nots == 1 && not_pos < pos_pos
    }

    /// A tuple member died or exited its pattern alpha — the LEFT-side
    /// death signature shared by the parked-del revive and the
    /// land_eager unpark (D-199): the eager rederive applies only when
    /// the firing SELF-KILLED its premise (amut del/set_break); a
    /// no-amut self-defeat keeps the park — that shape is a Drools
    /// RUNAWAY (sd_d3/d5, model=True) and the engine must terminate,
    /// so its divergence stays Family-II fenced (the sdp7002x40 spin:
    /// the ungated unpark re-derived + re-fired forever).
    fn tms_left_death(&self, ri: usize, t: &Tup) -> bool {
        t.iter().any(|f| !self.store.is_alive(*f)) || {
            let k = self.rules[ri].patterns.len();
            (0..k).any(|pos| {
                let pat = &self.rules[ri].patterns[pos];
                pat.sub != SubRole::Inner
                    && pat
                        .tpos
                        .map(|tp| tp < t.len() && !self.alpha_passes(ri, pos, t[tp]))
                        .unwrap_or(false)
            })
        }
    }

    /// Park bookkeeping at terminal events (D-076 self-defeat quirk).
    /// INS while parked: skipped (right-side churn can re-add children;
    /// Drools' parked tuple never sees them). UPD: left-side event —
    /// unpark and activate. DEL: unpark only when a tuple fact died or
    /// fails its alpha (left-side death); blocking churn keeps the park.
    fn tms_parked_ins(&self, ri: usize, t: &Tup) -> bool {
        // prefix semantics (D-198, sd_b2): a parked LEFT tuple (a lead
        // not's blocked prefix) suppresses every terminal ins that
        // EXTENDS it; trail parks are full-width so prefix == exact.
        self.tms.parked.iter().any(|(pri, pt)| *pri == ri && t.starts_with(pt))
    }

    /// tms_parked_ins + full-width RECORDING (D-199): a PREFIX park (a
    /// lead not's blocked left) suppresses the re-derived child at the
    /// terminal, but the child stays MATERIALIZED in the join — record
    /// it as a full-width park so a later foreign left-death finds it
    /// and the ⚖ t15 revive runs (the lazy-LEAD alternation,
    /// sdp7002x29-class; the model's t15_revive rederive sweeps lead
    /// rules too). Trail parks are full-width already — no-op there.
    fn tms_parked_suppress(&mut self, ri: usize, t: &Tup) -> bool {
        let Some(pt_len) = self
            .tms
            .parked
            .iter()
            .find(|(pri, pt)| *pri == ri && t.starts_with(pt))
            .map(|(_, pt)| pt.len())
        else {
            return false;
        };
        if pt_len < t.len() && !self.tms.parked.iter().any(|(pri, pt)| *pri == ri && pt == t) {
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS park-record r{ri} {t:?} (prefix-suppressed ins)");
            }
            self.tms.parked.push((ri, t.clone()));
        }
        true
    }

    fn tms_unpark_upd(&mut self, ri: usize, t: &Tup) -> bool {
        if let Some(i) = self.tms.parked.iter().position(|(pri, pt)| *pri == ri && pt == t) {
            self.tms.parked.remove(i);
            true
        } else {
            false
        }
    }

    fn tms_parked_del(&mut self, ri: usize, t: &Tup, origin: Option<usize>) {
        let Some(i) = self.tms.parked.iter().position(|(pri, pt)| *pri == ri && pt == t) else {
            return;
        };
        let left_death = self.tms_left_death(ri, t);
        if left_death {
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS park-del r{ri} {t:?} (left-death, origin {origin:?})");
            }
            self.tms.parked.remove(i);
            // ⚖ t15/d4 (D-197 round 2, sd_c1 / fz_42_5213 clause C): a
            // LEFT-side death revives the rule's OTHER parked tuples —
            // they re-derive as new objects and re-queue (strictly-
            // higher re-queue then preempts the deleter after one
            // firing = the certified alternation). Without a left
            // event siblings stay parked (t21 unchanged). LAZY plain
            // rules only — the t15 law excludes eager (no-loop/dyn)
            // and or-twins (fz_777_6816; the model's t15 scope).
            // ⚖ ACTOR EXCLUSION (D-199, model t15_revive actor / kin of
            // fz_42_2442): a SELF-INFLICTED left-death — the rule's own
            // RHS deleted/updated its P — never revives the actor's own
            // tuples (sdp7002x31-class: the trail mutfirst park holds).
            let eager = self.rules[ri].def.no_loop
                || matches!(self.rules[ri].salience, EngineSalience::Dyn { .. });
            let ortwin = (0..self.rules.len())
                .any(|rj| rj != ri && self.rule_parents[rj] == self.rule_parents[ri]);
            let self_inflicted =
                origin.is_some_and(|oi| self.rule_parents[oi] == self.rule_parents[ri]);
            if eager || ortwin || self_inflicted {
                return;
            }
            let revived: Vec<Tup> = self
                .tms
                .parked
                .iter()
                .filter(|(pri, pt)| {
                    // full-width parks only: a PREFIX park (a lead
                    // not's blocked left) re-derives via the network,
                    // never by direct re-activation (its tuple is
                    // shorter than the terminal width)
                    *pri == ri
                        && pt.len() == t.len()
                        && pt.iter().all(|f| self.store.is_alive(*f))
                        && {
                            let k = self.rules[ri].patterns.len();
                            (0..k).all(|pos| {
                                let pat = &self.rules[ri].patterns[pos];
                                pat.sub == SubRole::Inner
                                    || pat
                                        .tpos
                                        .map(|tp| {
                                            tp < pt.len()
                                                && self.alpha_passes(ri, pos, pt[tp])
                                        })
                                        .unwrap_or(true)
                            })
                        }
                })
                .map(|(_, pt)| pt.clone())
                .collect();
            // re-add order = the reversed blocked-chain scan (gt16's
            // pre-fold phys law; sd_c1 fires P3 before P2) — TRAIL only;
            // a LEAD justifier re-derives in INSERTION order (the model's
            // land-lane comment, banked x108; sdp7002x29's alternation
            // fires P1 first).
            let revived: Vec<Tup> = if self.tms_lead_k1(ri) {
                revived
            } else {
                revived.into_iter().rev().collect()
            };
            for rt in revived {
                if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                    eprintln!("TMS park-revive r{ri} {rt:?}");
                }
                self.tms.parked.retain(|(pri, pt)| !(*pri == ri && *pt == rt));
                self.push_activation(ri, rt);
            }
        }
    }

    /// Remove an activation's deps; retract facts whose belief sets
    /// emptied (nested WM deletes — cascades recurse through
    /// tms_eager_break/terminal processing). Returns the retracted facts.
    /// Read-only peek: the justified facts this act's drop WOULD
    /// retract (tms_drop_act_deps' emptiness pre-check, no mutation).
    fn tms_act_drop_victims(&self, act: &(usize, Tup)) -> Vec<FactId> {
        let Some((_, keys)) = self.tms.by_act.iter().find(|(a, _)| a == act) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for key in keys {
            let Some(e) = self.tms.keys.get(key) else { continue };
            let survives = e
                .beliefs
                .iter()
                .any(|j| !(j.ri == act.0 && j.tuple == act.1));
            if !survives {
                if let Some(jf) = e.justified {
                    if self.store.is_alive(jf) {
                        out.push(jf);
                    }
                }
            }
        }
        out
    }

    /// ⚖ the k0 fold/churn law (D-201, model fold_on_drop; the gt3/d4
    /// + gt6/x11 dump truths): a justifier's belief-drop CHURNS the
    /// del-group — rules with a positive join and a NOT matching the
    /// dying belief's type consume the staged blocker-ins BEFORE the
    /// retract (block + queued-act cancel), so the un-break re-adds
    /// their lefts in the blocked list's PREPEND order = the firing
    /// order REVERSES (sdp7001x54: the oracle deletes P4..P1, the
    /// engine's cross-batch ins+del annihilation kept t0 order).
    /// LAZY justifier: every del-group rule churns; EAGER (no-loop/
    /// dyn): SINK ORDER — only rules DECLARED BEFORE the justifier
    /// (gt6/x11 net-out vs the x70-class churn). Or-siblings ride the
    /// D-198 sibling-eval lane, not this one.
    fn tms_churn_del_group(&mut self, l: usize, tuple: &Tup) {
        let victims = self.tms_act_drop_victims(&(l, tuple.clone()));
        if victims.is_empty() {
            return;
        }
        let vtypes: Vec<TypeId> =
            victims.iter().map(|f| self.store.fact_type(*f)).collect();
        let init_tid = self.store.type_id(INITIAL_FACT);
        let eager_l = self.rules[l].def.no_loop
            || matches!(self.rules[l].salience, EngineSalience::Dyn { .. });
        let par = self.rule_parents[l];
        for rj in 0..self.rules.len() {
            if rj == l || self.rule_parents[rj] == par {
                continue;
            }
            if eager_l && rj > l {
                continue;
            }
            let pats = &self.rules[rj].patterns;
            let has_not = pats
                .iter()
                .any(|p| p.ce == CeKind::Not && vtypes.contains(&p.type_id));
            let has_pos = pats.iter().any(|p| {
                p.ce == CeKind::Positive && Some(p.type_id) != init_tid
            });
            if has_not && has_pos {
                self.evaluate_rule(rj, false, false);
            }
        }
    }

    /// One flush-drain sweep for ri's eligible deferred entries — the
    /// full landing body (or-sibling consumption [D-198], the k0
    /// churn [D-201], the retract, the land_eager unpark [D-199]).
    /// Called BEFORE the pass-1 evaluation (entries from earlier
    /// phases), AFTER it (D-201: an entry pushed DURING ri's eval
    /// drains at ri's OWN eager-list slot — the decl-law), and in
    /// pass 2 (the residue net). Returns whether anything drained.
    fn tms_flush_drain(&mut self, ri: usize, dyn_sal: bool, site: &str) -> bool {
        let mut drained = false;
        loop {
            let run_live = !self.nets[ri].queue.is_empty();
            let Some(di) = self.tms.deferred.iter().position(|(dri, _, fl)| {
                *dri == ri
                    && (dyn_sal
                        || ((*fl & 2) != 0 && (*fl & 16) == 0)
                        || ((*fl & 1) != 0 && (run_live || (*fl & 4) == 0))
                        || ((*fl & 8) != 0 && run_live))
            }) else {
                break;
            };
            drained = true;
            let (_, tuple, fl) = self.tms.deferred.remove(di);
            if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                eprintln!("TMS drain[{site}] r{ri} {tuple:?}");
            }
            // D-198 (sd_b4): the or-twin's SIBLING branch consumes the
            // in-firing block BEFORE the self-defeat drop un-breaks.
            {
                let par = self.rule_parents[ri];
                let sibs: Vec<usize> = (0..self.rules.len())
                    .filter(|&rj| rj != ri && self.rule_parents[rj] == par)
                    .collect();
                for rj in sibs {
                    self.evaluate_rule(rj, false, false);
                }
            }
            self.tms_churn_del_group(ri, &tuple);
            if fl & 6 == 6 {
                self.tms_mf_teardown_reverse(ri, &tuple);
            }
            self.tms_on_terminal_del(ri, &tuple);
            if self.rules[ri].def.no_loop
                && self.tms_lead_k1(ri)
                && self.tms_left_death(ri, &tuple)
            {
                self.tms.parked.retain(|(pri, _)| *pri != ri);
            }
        }
        drained
    }

    /// ⚖ the mutfirst teardown (D-201, model x119/x30): a MUTFIRST
    /// composite key (bit1+bit2) "never propagated" — the deleter
    /// consumes t0 order EVEN when declared first. Reverse the
    /// victims' blocked lists pre-retract so the right-del release
    /// emits INSERTION order instead of the prepend chain.
    fn tms_mf_teardown_reverse(&mut self, ri: usize, tuple: &Tup) {
        let victims = self.tms_act_drop_victims(&(ri, tuple.clone()));
        for f in victims {
            let ftid = self.store.fact_type(f);
            for rj in 0..self.rules.len() {
                for pos in 0..self.rules[rj].patterns.len() {
                    let pat = &self.rules[rj].patterns[pos];
                    if pat.ce != CeKind::Not || pat.type_id != ftid {
                        continue;
                    }
                    let Some(&ni) = self
                        .nets[rj]
                        .path
                        .iter()
                        .find(|&&ni| self.trie[ni].env.1 == pos)
                    else {
                        continue;
                    };
                    self.trie[ni].node.blocked_reverse_of(f);
                }
            }
        }
    }

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
                    // ⚖ D-211 THE L6-EVENT (l5/l6/x1/9902): the last
                    // dep's break kills the key WHOLE — stated siblings
                    // ORPHAN (undeletable, WM-alive); a later
                    // re-justification re-keys FRESH with a WM-visible
                    // handle (the l6 rebirth).
                    let sibs: Vec<FactId> = e.stated.drain(..).collect();
                    for sb in sibs {
                        self.tms.by_fact.remove(&sb);
                        self.tms.orphans.insert(sb);
                        if std::env::var("SEINE_TMS_DEBUG").is_ok() {
                            eprintln!("TMS key[l6-orphan] f{sb:?} key={key:?}");
                        }
                    }
                    self.tms.keys.remove(&key);
                } else if e.pending_vals.is_some() {
                    // ⚖ D-211 PENDING-CLEAR (c2/c3/c4): deps-empty on a
                    // NON-WM pending belief clears the bookkeeping; the
                    // key survives as pure stated.
                    e.pending_vals = None;
                    if e.stated.is_empty() {
                        self.tms.keys.remove(&key);
                    }
                } else if e.stated.is_empty() {
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
                    tuple: j.tuple.to_vec(),
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
        self.facts_iter().collect()
    }

    /// D-272 (the memory diet): the same WM dump as `facts()`, lazily —
    /// callers that serialize can stream one FactView at a time instead
    /// of materializing the whole Vec (identical items, identical order,
    /// by construction: `facts()` is this iterator collected).
    pub fn facts_iter(&self) -> impl Iterator<Item = FactView> + '_ {
        let mut hidden: Vec<TypeId> = RESERVED_TYPES
            .iter()
            .filter_map(|n| self.store.type_id(n))
            .collect();
        hidden.extend(self.qrow_tids.iter().copied());
        hidden.extend(self.gbrow_tids.iter().copied());
        self.store
            .live_facts()
            .filter(move |f| !hidden.contains(&self.store.fact_type(*f)))
            // CEP E2 item D: `session.getObjects()` returns only DEFAULT-EP
            // objects; named-EP facts live in separate partitions and are
            // not part of the default WM dump.
            .filter(|f| self.fact_ep(*f) == 0)
            .map(|f| self.store.render(f))
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
        Value::Null => "Null",
        Value::Dec { .. } => "Decimal", // unreachable: Dec walled from queries (D-098)
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
        | GExpr::IsNull { field_idx, .. }
        | GExpr::Matches { field_idx, .. }
        | GExpr::Contains { field_idx, .. }
        | GExpr::InList { field_idx, .. } => *field_idx,
        GExpr::Unknown => 0,
        GExpr::And(xs) | GExpr::Or(xs) => first_group_field(&xs[0]),
        GExpr::Not(x) => first_group_field(x),
    }
}

/// D-098: a written decimal literal (lexed as f64) recovered EXACTLY
/// via the shortest round-trip representation — exact for every
/// literal with <= 15 significant digits; longer literals fail
/// dec_parse (exponent forms) or round (documented wall).
fn f64_lit_to_dec(x: f64) -> Option<Value> {
    let (u, s) = crate::store::dec_parse(&format!("{x}"))?;
    Some(Value::Dec { u, s })
}

/// Convert a compile-time literal for a decimal-typed field: written
/// integers scale exactly; written decimals via shortest repr; the
/// f64 WALL applies only to f64-typed BINDINGS, not written literals.
fn lit_for_dec(v: &Value) -> Option<Value> {
    match v {
        Value::I64(n) => Some(Value::Dec { u: *n as i128, s: 0 }),
        Value::F64(x) => f64_lit_to_dec(*x),
        Value::Dec { .. } => Some(v.clone()),
        _ => None,
    }
}

fn lit_value(l: &Literal) -> Value {
    match l {
        Literal::I64(n) => Value::I64(*n),
        Literal::F64(n) => Value::F64(*n),
        Literal::Str(s) => Value::Str(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Null => Value::Null,
    }
}

fn lit_type(l: &Literal) -> FieldType {
    match l {
        Literal::I64(_) => FieldType::I64,
        Literal::F64(_) => FieldType::F64,
        Literal::Str(_) => FieldType::Str,
        Literal::Bool(_) => FieldType::Bool,
        // callers gate null literals before type-directed dispatch
        Literal::Null => FieldType::I64,
    }
}

/// Java-style: exact match, or i64 widening into f64.
fn ft_name(ft: FieldType) -> &'static str {
    match ft {
        FieldType::I64 => "i64",
        FieldType::F64 => "f64",
        FieldType::Str => "String",
        FieldType::Bool => "bool",
        FieldType::Dec { .. } => "decimal",
    }
}

fn assignable(src: FieldType, dst: FieldType) -> bool {
    if let (FieldType::Dec { .. }, FieldType::Dec { .. }) = (src, dst) {
        return true; // runtime rescale + precision check in coerce (D-098)
    }
    if let (FieldType::I64, FieldType::Dec { .. }) = (src, dst) {
        return true;
    }
    src == dst || (src == FieldType::I64 && dst == FieldType::F64)
}

/// Java-style widening: i64 -> f64 is allowed, nothing else converts.
fn coerce(v: Value, target: FieldType) -> Option<Value> {
    match (v, target) {
        // Null passes typing; the STORE's validity gate is the single
        // nullability authority (loud error for non-nullable, D-097).
        (Value::Null, _) => Some(Value::Null),
        (Value::I64(n), FieldType::F64) => Some(Value::F64(n as f64)),
        // D-098 decimal ingestion: exact strings and integers only —
        // JSON/IEEE floats are REJECTED (no lossy round-trips for
        // money). Rescale to the field's declared scale (exact when
        // widening, HALF-UP when narrowing, pin J) and enforce the
        // declared precision (loud error on overflow).
        (Value::Str(txt), FieldType::Dec { p, s }) => {
            let (u0, s0) = crate::store::dec_parse(&txt)?;
            let (u, s) = crate::store::dec_rescale(u0, s0, s)?;
            crate::store::dec_fits(u, p).then_some(Value::Dec { u, s })
        }
        (Value::I64(n), FieldType::Dec { p, s }) => {
            let (u, s) = crate::store::dec_rescale(n as i128, 0, s)?;
            crate::store::dec_fits(u, p).then_some(Value::Dec { u, s })
        }
        (Value::Dec { u: u0, s: s0 }, FieldType::Dec { p, s }) => {
            let (u, s) = crate::store::dec_rescale(u0, s0, s)?;
            crate::store::dec_fits(u, p).then_some(Value::Dec { u, s })
        }
        (Value::F64(_), FieldType::Dec { .. }) => None, // the wall
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
    let dec = |t| matches!(t, FieldType::Dec { .. });
    // D-097 ruling 4: decimal never meets f64 — a COMPILE error,
    // stricter than DuckDB's cast-to-double (pin J documents the
    // un-walled semantics). decimal-vs-i64/decimal stays (exact).
    if (dec(lhs) && rhs == FieldType::F64) || (lhs == FieldType::F64 && dec(rhs)) {
        return Err(EngineError(format!(
            "rule {rule}: decimal-vs-double comparison is WALLED (money never meets floats, D-097 ruling 4)"
        )));
    }
    let ok = (numeric(lhs) && numeric(rhs))
        || (dec(lhs) && (dec(rhs) || rhs == FieldType::I64))
        || (lhs == FieldType::I64 && dec(rhs))
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
    flush: bool,
    fire_no: u64,
    /// D-134 (§3B): TRUE while a temporal-`not` deferral RELEASE propagates.
    /// A released not-left logically fires at its window-close fire_time, when
    /// its downstream join partners were still alive; but a collapsed
    /// advance() has already expiration-FLAGGED partners that reap after that
    /// fire_time (they are still `is_alive`, deleted only at the later drain).
    /// So the release join must NOT skip them (the model ignores partner
    /// expiration for the not_mid join) — is_expired reads false here.
    not_releasing: bool,
}

impl phreak::JoinEnv for JoinEnvImpl<'_> {
    fn fire_no(&self) -> u64 {
        self.fire_no
    }
    fn in_flush(&self) -> bool {
        self.flush
    }
    fn two_pattern(&self) -> bool {
        self.rule.patterns.len() == 2
    }
    fn is_expired(&self, f: FactId) -> bool {
        // D-134 (§3B): a not-release join sees partners that were alive at the
        // window-close fire_time, even if a collapsed advance flagged them.
        !self.not_releasing && self.store.is_expired(f)
    }
    fn allowed(&self, node: usize, l: &Tup, f: FactId) -> bool {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        pat.cmps.iter().all(|c| {
            if let Test::Group { g, .. } = &c.test {
                return eval_gexpr(g, self.store, f, Some(l), pat.tpos) == Some(true);
            }
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::IsNull { negated } => lhs.is_null() != *negated,
                Test::Unknown => false,
                Test::Cmp { op, rhs: Src::Lit(v) } => !lhs.is_null() && eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } => {
                    let other = if Some(*ti) == pat.tpos { f } else { l[*ti] };
                    eval_cmp_join(&lhs, *op, &self.store.value(other, *fi))
                }
                Test::Temporal { op, params, anchor, self_dur_fi, anchor_dur_fi } => {
                    // CEP E1/E2 interval join: [Bs,Be] (self) vs [As,Ae]
                    // (anchor); each end = start + @duration (0 if point, so
                    // after/before reduce to the E1 point delta).
                    // D-141 (item 1b): the interval STARTS read the INSERT-fixed
                    // temporal position (`temporal_ts`), NOT the mutable ts field
                    // (`lhs`) — a ts-update mutates the field but never moves the
                    // event in the stream (`tj_ts_update{,_under}` repros).
                    let Value::I64(bs) = self.store.temporal_ts(f, c.field_idx) else { return false };
                    let Value::I64(as_) = self.store.temporal_ts(l[anchor.0], anchor.1) else {
                        return false;
                    };
                    let be = bs.wrapping_add(dur_of(self.store, f, *self_dur_fi));
                    let ae = as_.wrapping_add(dur_of(self.store, l[anchor.0], *anchor_dur_fi));
                    eval_allen(*op, params, bs, be, as_, ae)
                }
                Test::Cmp { .. } => true,
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    fn key_of_left(&self, node: usize, l: &Tup) -> Option<Vec<Value>> {
        let pos = node + 1;
        let pat = &self.rule.patterns[pos];
        // D-101: temporal nodes key the LEFT by the ANCHOR fact's ts
        // (D-141 item 1b: the INSERT-fixed position, so a ts-update never
        // re-buckets the event — consistent with the eval above).
        if let Some(Test::Temporal { anchor, .. }) =
            pat.cmps.iter().find_map(|c| matches!(&c.test, Test::Temporal { .. }).then(|| &c.test))
        {
            return Some(vec![self.store.temporal_ts(l[anchor.0], anchor.1)]);
        }
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
        // D-101: temporal nodes key the RIGHT by its own ts field
        // (D-141 item 1b: the INSERT-fixed position, so a ts-update never
        // re-buckets the event — consistent with the eval above).
        if let Some(fi) = pat.cmps.iter().find_map(|c| {
            matches!(&c.test, Test::Temporal { .. }).then(|| c.field_idx)
        }) {
            return Some(vec![self.store.temporal_ts(f, fi)]);
        }
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
                return eval_gexpr(g, self.store, f, Some(l), pat.tpos) == Some(true);
            }
            let lhs = self.store.value(f, c.field_idx);
            match &c.test {
                Test::IsNull { negated } => lhs.is_null() != *negated,
                Test::Unknown => false,
                Test::Cmp { op, rhs: Src::Lit(v) } => !lhs.is_null() && eval_cmp(&lhs, *op, v),
                Test::Cmp { op, rhs: Src::Field(ti, fi) } => {
                    let other = if Some(*ti) == pat.tpos { f } else { l[*ti] };
                    eval_cmp_join(&lhs, *op, &self.store.value(other, *fi))
                }
                Test::Temporal { op, params, anchor, self_dur_fi, anchor_dur_fi } => {
                    // CEP E1/E2 interval join: [Bs,Be] (self) vs [As,Ae]
                    // (anchor); each end = start + @duration (0 if point, so
                    // after/before reduce to the E1 point delta).
                    // D-141 (item 1b): the interval STARTS read the INSERT-fixed
                    // temporal position (`temporal_ts`), NOT the mutable ts field
                    // (`lhs`) — a ts-update mutates the field but never moves the
                    // event in the stream (`tj_ts_update{,_under}` repros).
                    let Value::I64(bs) = self.store.temporal_ts(f, c.field_idx) else { return false };
                    let Value::I64(as_) = self.store.temporal_ts(l[anchor.0], anchor.1) else {
                        return false;
                    };
                    let be = bs.wrapping_add(dur_of(self.store, f, *self_dur_fi));
                    let ae = as_.wrapping_add(dur_of(self.store, l[anchor.0], *anchor_dur_fi));
                    eval_allen(*op, params, bs, be, as_, ae)
                }
                Test::Cmp { .. } => true,
                other => eval_alpha_test(&lhs, other),
            }
        })
    }

    /// D-134 (§3B): fire_time for a temporal `not` left. The window closes
    /// (no blocker can still arrive) at anchor.ts+hi for `after` and at
    /// anchor.ts−lo for `before`; the DEFERRED regime is where that time is
    /// >= anchor.ts (after any hi; before with lo==0, the whole fuzz
    /// population). before[lo>0] is the IMMEDIATE regime and other Allen ops
    /// are unmodelled — both return None (fire at insert). Point-event
    /// formula (anchor.ts, no @duration end): the modelled/fuzzed shapes.
    fn not_fire_time(&self, node: usize, l: &Tup) -> Option<i64> {
        let pat = &self.rule.patterns[node + 1];
        let (op, params, anchor) = pat.cmps.iter().find_map(|c| match &c.test {
            Test::Temporal { op, params, anchor, .. } => Some((*op, params, *anchor)),
            _ => None,
        })?;
        let Value::I64(a_ts) = self.store.value(l[anchor.0], anchor.1) else {
            return None;
        };
        let (lo, hi) = (params[0], params[1]);
        match op {
            AllenOp::After => Some(a_ts.wrapping_add(hi)),
            AllenOp::Before if lo == 0 => Some(a_ts),
            _ => None,
        }
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
/// Read an `@duration` field (ms) from fact `f`; 0 when the type has no
/// `@duration` (point event) or the value is null/non-i64 (CEP E2 item E).
fn dur_of(store: &FactStore, f: FactId, dur_fi: Option<usize>) -> i64 {
    match dur_fi {
        Some(fi) => match store.value(f, fi) {
            Value::I64(d) => d,
            _ => 0,
        },
        None => 0,
    }
}

/// Overlap-distance bounds `[min,max]` for `overlaps`/`overlappedby`
/// (D-119). The structural predicate already forces the overlap ≥ 1, so
/// bare/`[max]` default the lower bound to 1; `[min,max]` sets both.
fn overlap_bounds(params: &[i64]) -> (i64, i64) {
    match params {
        [max] => (1, *max),
        [min, max] => (*min, *max),
        _ => (1, i64::MAX),
    }
}

/// `during`/`includes` endpoint windows — (start-min, start-max, end-min,
/// end-max) applied to (dS, dE). Bare = strict inside (min 1, the pinned
/// `dist>0` default); `[v]` = `[1,v]` on both; `[lo,hi]` = `[lo,hi]` on
/// both; `[lo1,hi1,lo2,hi2]` = split (D-119; Drools default minDev=1).
fn during_bounds(params: &[i64]) -> (i64, i64, i64, i64) {
    match params {
        [v] => (1, *v, 1, *v),
        [lo, hi] => (*lo, *hi, *lo, *hi),
        [lo1, hi1, lo2, hi2] => (*lo1, *hi1, *lo2, *hi2),
        _ => (1, i64::MAX, 1, i64::MAX),
    }
}

/// CEP E2 item E (D-118/D-119): evaluate the Allen relation `op` (with
/// optional tolerance `params`) between the SELF interval `[bs, be]` and
/// the ANCHOR interval `[as_, ae]`. "B op A" convention — bs/be = subject
/// (`this`=B), as_/ae = object (anchor `$a`=A). Bounds are the oracle-
/// pinned endpoint comparisons; parameterized forms bound a specific
/// endpoint distance (inclusive). Point events feed `be==bs` / `ae==as_`,
/// so a dur=0 anchor/self reduces each op to its point behavior.
fn eval_allen(op: AllenOp, params: &[i64], bs: i64, be: i64, as_: i64, ae: i64) -> bool {
    use AllenOp::*;
    // Deltas in Java long arithmetic: overflow WRAPS (and Math.abs(MIN)
    // stays MIN), so extreme-timestamp pairs mis-compare identically on
    // both sides — pr_cep_tjoverflow pins the After composite; same
    // convention as the RHS expr evaluator.
    let sub = i64::wrapping_sub;
    match op {
        // after: d = Bs − Ae ∈ [lo,hi] (B later; only the anchor's dur
        // enters via Ae). before: d = As − Be ∈ [lo,hi] (B earlier; only
        // self's dur enters via Be). Bounds inclusive, exact (no ±1).
        After => {
            let d = sub(bs, ae);
            d >= params[0] && d <= params[1]
        }
        Before => {
            let d = sub(as_, be);
            d >= params[0] && d <= params[1]
        }
        // bare: Bs==As ∧ Be==Ae. [dev]: |Bs−As|≤dev ∧ |Be−Ae|≤dev.
        // [sDev,eDev]: split start/end tolerances.
        Coincides => {
            let (sdev, edev) = match params {
                [dev] => (*dev, *dev),
                [sd, ed] => (*sd, *ed),
                _ => (0, 0),
            };
            sub(bs, as_).wrapping_abs() <= sdev && sub(be, ae).wrapping_abs() <= edev
        }
        // bare: Be==As. [dev]: |Be−As|≤dev.
        Meets => sub(be, as_).wrapping_abs() <= params.first().copied().unwrap_or(0),
        // bare: Bs==Ae. [dev]: |Bs−Ae|≤dev.
        MetBy => sub(bs, ae).wrapping_abs() <= params.first().copied().unwrap_or(0),
        // structural Bs<As<Be<Ae; overlap = Be−As within [min,max].
        Overlaps => {
            let (min, max) = overlap_bounds(params);
            let ov = sub(be, as_);
            bs < as_ && as_ < be && be < ae && ov >= min && ov <= max
        }
        // structural As<Bs<Ae<Be; overlap = Ae−Bs within [min,max].
        OverlappedBy => {
            let (min, max) = overlap_bounds(params);
            let ov = sub(ae, bs);
            as_ < bs && bs < ae && ae < be && ov >= min && ov <= max
        }
        // B strictly inside A: dS=Bs−As, dE=Ae−Be, each in its window.
        During => {
            let (smin, smax, emin, emax) = during_bounds(params);
            let (ds, de) = (sub(bs, as_), sub(ae, be));
            ds >= smin && ds <= smax && de >= emin && de <= emax
        }
        // A strictly inside B (during with A,B swapped): dS=As−Bs, dE=Be−Ae.
        Includes => {
            let (smin, smax, emin, emax) = during_bounds(params);
            let (ds, de) = (sub(as_, bs), sub(be, ae));
            ds >= smin && ds <= smax && de >= emin && de <= emax
        }
        // Bs==As (±dev) ∧ Be<Ae (the end side stays strict).
        Starts => sub(bs, as_).wrapping_abs() <= params.first().copied().unwrap_or(0) && be < ae,
        // Bs==As (±dev) ∧ Be>Ae.
        StartedBy => {
            sub(bs, as_).wrapping_abs() <= params.first().copied().unwrap_or(0) && be > ae
        }
        // Be==Ae (±dev) ∧ Bs>As (the start side stays strict).
        Finishes => sub(be, ae).wrapping_abs() <= params.first().copied().unwrap_or(0) && bs > as_,
        // Be==Ae (±dev) ∧ Bs<As.
        FinishedBy => {
            sub(be, ae).wrapping_abs() <= params.first().copied().unwrap_or(0) && bs < as_
        }
    }
}

fn eval_alpha_test(lhs: &Value, test: &Test) -> bool {
    match test {
        Test::Matches(r) => matches!(lhs, Value::Str(s) if r.accepts(s)),
        Test::Contains(needle) => matches!(lhs, Value::Str(s) if s.contains(needle.as_str())),
        Test::IsNull { negated } => lhs.is_null() != *negated,
        Test::Unknown => false,
        Test::Cmp { .. } => unreachable!("Cmp handled by callers"),
        Test::Group { .. } => unreachable!("Group handled by callers"),
        Test::Temporal { .. } => unreachable!("Temporal is beta-only (D-101)"),
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
        (Value::Dec { u: a, s: x }, Value::Dec { u: b, s: y }) => {
            Some(crate::store::dec_cmp(*a, *x, *b, *y))
        }
        (Value::Dec { u: a, s: x }, Value::I64(b)) => {
            Some(crate::store::dec_cmp(*a, *x, *b as i128, 0))
        }
        (Value::I64(a), Value::Dec { u: b, s: y }) => {
            Some(crate::store::dec_cmp(*a as i128, 0, *b, *y))
        }
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

#[cfg(test)]
mod allen_eval_tests {
    //! CEP E2 item E (D-118/D-119): certify the `eval_allen` predicate table
    //! independently of the oracle differential — the pinned bare matrix,
    //! parameterized forms, directionality, and the dur=0 point reduction.
    //! Configs mirror the oracle-verified probes (B op A; Bs=B.ts,
    //! Be=B.ts+B.dur, As=A.ts, Ae=A.ts+A.dur).
    use super::AllenOp::*;
    use super::{eval_allen, AllenOp};

    /// `eval_allen(op, params, Bs, Be, As, Ae)`.
    fn f(op: AllenOp, p: &[i64], bs: i64, be: i64, as_: i64, ae: i64) -> bool {
        eval_allen(op, p, bs, be, as_, ae)
    }

    #[test]
    fn bare_matrix_fires_on_the_relation() {
        assert!(f(Meets, &[], 0, 50, 50, 90)); // Be==As
        assert!(f(MetBy, &[], 50, 90, 0, 50)); // Bs==Ae
        assert!(f(Coincides, &[], 10, 60, 10, 60)); // Bs==As & Be==Ae
        assert!(f(Overlaps, &[], 0, 50, 30, 100)); // Bs<As<Be<Ae
        assert!(f(OverlappedBy, &[], 30, 100, 0, 50)); // As<Bs<Ae<Be
        assert!(f(During, &[], 20, 80, 0, 100)); // As<Bs & Be<Ae
        assert!(f(Includes, &[], 0, 100, 20, 80)); // Bs<As & Ae<Be
        assert!(f(Starts, &[], 10, 50, 10, 90)); // Bs==As & Be<Ae
        assert!(f(StartedBy, &[], 10, 90, 10, 50)); // Bs==As & Be>Ae
        assert!(f(Finishes, &[], 50, 90, 10, 90)); // Be==Ae & Bs>As
        assert!(f(FinishedBy, &[], 10, 90, 50, 90)); // Be==Ae & Bs<As
    }

    #[test]
    fn bare_matrix_inert_on_near_miss() {
        assert!(!f(Meets, &[], 0, 49, 50, 90)); // Be=49 != As=50
        assert!(!f(Meets, &[], 0, 51, 50, 90)); // Be=51 != As=50
        assert!(!f(During, &[], 0, 80, 0, 100)); // eqstart: dS=0 not strict
        assert!(!f(During, &[], 0, 100, 0, 100)); // equal bounds: not inside
        assert!(!f(Starts, &[], 10, 90, 10, 50)); // endgt: Be>Ae is startedby
        assert!(!f(Starts, &[], 11, 50, 10, 90)); // Bs!=As
        assert!(!f(Coincides, &[], 11, 60, 10, 60)); // Bs off by 1, no dev
    }

    #[test]
    fn directional_not_symmetric() {
        // a during-config (A big, B small inside) under `includes` is inert
        // and vice-versa (xdir_* pins).
        assert!(!f(Includes, &[], 20, 80, 0, 100));
        assert!(!f(During, &[], 0, 100, 20, 80));
    }

    #[test]
    fn parameterized_forms() {
        // during[min,max]: dS,dE both in [lo,hi], inclusive.
        assert!(f(During, &[20, 25], 20, 80, 0, 100)); // dS=dE=20
        assert!(!f(During, &[21, 25], 20, 80, 0, 100)); // 20 < min 21
        // during[lo1,hi1,lo2,hi2]: split start/end windows.
        assert!(f(During, &[15, 25, 25, 35], 20, 70, 0, 100)); // dS=20, dE=30
        // overlaps[max]/[min,max] bound the overlap Be-As (structural still holds).
        assert!(!f(Overlaps, &[15], 0, 50, 30, 100)); // overlap 20 > 15
        assert!(f(Overlaps, &[10, 25], 0, 50, 30, 100)); // 10<=20<=25
        assert!(!f(Overlaps, &[21, 25], 0, 50, 30, 100)); // 20 < min 21
        // meets[dev]/coincides[sDev,eDev] tolerances, inclusive.
        assert!(f(Meets, &[1], 0, 49, 50, 90)); // |49-50|=1<=1
        assert!(!f(Coincides, &[0, 1], 11, 60, 10, 60)); // |Bs-As|=1 > sDev 0
        assert!(f(Coincides, &[1, 1], 11, 61, 10, 60)); // both diffs 1 <= 1
    }

    #[test]
    fn after_before_distance_and_point_reduction() {
        // after: d = Bs - Ae in [lo,hi]. Interval anchor A(ts=100,dur=30)
        // ⇒ Ae=130; B(ts=200) ⇒ d=70 ∈ [60,80] fires.
        assert!(f(After, &[60, 80], 200, 230, 100, 130));
        // point anchor (dur=0 ⇒ Ae=As=100) ⇒ d=100 ∉ [60,80] inert — the
        // endTS=ts+dur feature: only the earlier event's dur enters.
        assert!(!f(After, &[60, 80], 200, 200, 100, 100));
        // before: d = As - Be in [lo,hi].
        assert!(f(Before, &[60, 80], 100, 130, 200, 230)); // 200-130=70
        // dur=0 both sides: every op reduces to its point behavior —
        // coincides on identical points fires, during is inert.
        assert!(f(Coincides, &[], 10, 10, 10, 10));
        assert!(!f(During, &[], 10, 10, 10, 10));
    }
}
