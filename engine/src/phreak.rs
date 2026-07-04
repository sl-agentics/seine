//! Join-network evaluation: a behavioral port of the PHREAK node algorithm
//! (PhreakJoinNode / PhreakRuleTerminalNode / RuleExecutor semantics),
//! validated exclusively against the Drools 9.44.0.Final oracle — no
//! upstream code is copied; every ordering below is pinned by a probe or
//! regression scenario (see DECISIONS.md D-026/D-027).
//!
//! Structures per rule:
//! - staged TupleSets per node input, PREPEND on staging (LIFO), consumed
//!   head-first; working-memory staging into the pos0 input of a
//!   single-pattern rule is consumed OLDEST-first (pr08/pr04 pin);
//! - left/right memories: ordered lists with STORED index keys; updated
//!   entries re-key and move to the END (removeAdd / remove-all-re-add);
//! - child tuples per join, linked into BOTH parents' ordered child lists
//!   (creation PREPENDS; a re-add moves a child to the END);
//! - terminal: `queue` holds only unfired activations (fired ones leave on
//!   firing); a terminal UPDATE keeps a queued activation in place and
//!   re-appends an unqueued (fired) one; no-loop blocks re-activation when
//!   the propagation origin is the rule itself.

use std::collections::HashMap;

use crate::store::{FactId, Value};

pub type Tup = Vec<FactId>;
pub type Origin = Option<usize>;

/// TupleSets: three LIFO lists (index 0 = most recently staged) with the
/// upstream fold rules.
#[derive(Clone)]
pub struct Staged<T: Clone + PartialEq> {
    pub ins: Vec<(T, Origin, u8)>,
    pub upd: Vec<(T, Origin, u8)>,
    pub del: Vec<(T, Origin, u8)>,
}

impl<T: Clone + PartialEq> Default for Staged<T> {
    fn default() -> Self {
        Staged { ins: Vec::new(), upd: Vec::new(), del: Vec::new() }
    }
}

impl<T: Clone + PartialEq> Staged<T> {
    pub fn is_empty(&self) -> bool {
        self.ins.is_empty() && self.upd.is_empty() && self.del.is_empty()
    }

    pub fn take(&mut self) -> Staged<T> {
        std::mem::take(self)
    }

    pub fn add_ins(&mut self, t: T, origin: Origin) {
        self.add_ins_ph(t, origin, 0)
    }

    /// NOTE: no del+ins fold — Drools folds by tuple OBJECT identity, and a
    /// re-created child is a NEW object (c13). `phase` records which processing
    /// phase created the entry: 0 = left-insert, 1 = right-insert,
    /// 2 = update-derived (terminal block ordering, D-027).
    pub fn add_ins_ph(&mut self, t: T, origin: Origin, phase: u8) {
        if self.upd.iter().any(|(x, _, _)| *x == t) || self.ins.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        self.ins.insert(0, (t, origin, phase));
    }

    pub fn add_upd(&mut self, t: T, origin: Origin) {
        self.add_upd_ph(t, origin, 2)
    }

    pub fn add_upd_ph(&mut self, t: T, origin: Origin, phase: u8) {
        // TupleSetsImpl.addUpdate: already staged (any list) -> no-op.
        if self.ins.iter().any(|(x, _, _)| *x == t)
            || self.upd.iter().any(|(x, _, _)| *x == t)
            || self.del.iter().any(|(x, _, _)| *x == t)
        {
            return;
        }
        self.upd.insert(0, (t, origin, phase));
    }

    /// Merge a downstream node's PENDING staging with a fresh trg batch.
    /// updateChildLeftTuple semantics: an event re-touching a tuple that
    /// is staged in PENDING removes it there and re-stages it fresh
    /// (moving it to the head); same-kind staging keeps its kind
    /// (a pending INSERT touched by an update stays an INSERT).
    pub fn merge_into_pending(mut pending: Staged<T>, trg: Staged<T>) -> Staged<T> {
        for (t, o, ph) in trg.del.into_iter().rev() {
            pending.add_del(t, o);
            let _ = ph;
        }
        for (t, o, ph) in trg.upd.into_iter().rev() {
            if let Some(i) = pending.ins.iter().position(|(x, _, _)| *x == t) {
                let e = pending.ins.remove(i);
                pending.ins.insert(0, e); // stays an insert, moves to head
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| *x == t) {
                pending.upd.remove(i);
            }
            if pending.del.iter().any(|(x, _, _)| *x == t) {
                continue;
            }
            pending.upd.insert(0, (t, o, ph));
        }
        for (t, o, ph) in trg.ins.into_iter().rev() {
            if pending.ins.iter().any(|(x, _, _)| *x == t) {
                continue;
            }
            pending.ins.insert(0, (t, o, ph));
        }
        pending
    }

    pub fn add_del(&mut self, t: T, origin: Origin) {
        if let Some(i) = self.ins.iter().position(|(x, _, _)| *x == t) {
            self.ins.remove(i); // never materialized: cancel
            return;
        }
        if let Some(i) = self.upd.iter().position(|(x, _, _)| *x == t) {
            self.upd.remove(i);
        }
        if self.del.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        self.del.insert(0, (t, origin, 0));
    }

    /// Segment propagation to the FIRST-built sink (D-036/D-037):
    /// TupleSetsImpl.addAll APPENDS the incoming lists at the tail, so a
    /// lagging first sink accumulates batches FIFO (fz_42_580's oracle:
    /// batch 1 fires before batch 2), with the same same-tuple clash
    /// folds as merge_into_pending.
    pub fn append_into_pending(mut pending: Staged<T>, fresh: Staged<T>) -> Staged<T> {
        for (t, o, _) in fresh.del.into_iter().rev() {
            pending.add_del(t, o);
        }
        for (t, o, ph) in fresh.upd {
            if let Some(i) = pending.ins.iter().position(|(x, _, _)| *x == t) {
                let e = pending.ins.remove(i);
                pending.ins.push(e); // stays an insert, moves to the tail
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| *x == t) {
                pending.upd.remove(i);
            }
            if pending.del.iter().any(|(x, _, _)| *x == t) {
                continue;
            }
            pending.upd.push((t, o, ph));
        }
        for (t, o, ph) in fresh.ins {
            if pending.ins.iter().any(|(x, _, _)| *x == t) {
                continue;
            }
            pending.ins.push((t, o, ph));
        }
        pending
    }

    /// Peer-copy of a staged batch into a LATER sink's pending
    /// (SegmentPropagator.processPeer*): each source list is walked
    /// head-first and PREPENDED into the peer staging, so a batch arrives
    /// REVERSED and batches stack LIFO. Clash rules differ from the
    /// intra-chain merge (D-037/fz_999_3298): an UPDATE touching an
    /// already-staged peer is SKIPPED — it keeps its existing position
    /// and kind (processPeerUpdates' staged-type check) — while an INSERT
    /// clash moves the entry to the head (updateChildLeftTupleDuringInsert).
    pub fn peer_merge_into_pending(mut pending: Staged<T>, fresh: Staged<T>) -> Staged<T> {
        for (t, o, _) in fresh.del {
            pending.add_del(t, o); // prepends; folds staged ins/upd
        }
        for (t, o, ph) in fresh.upd {
            let staged = pending.ins.iter().any(|(x, _, _)| *x == t)
                || pending.upd.iter().any(|(x, _, _)| *x == t)
                || pending.del.iter().any(|(x, _, _)| *x == t);
            if staged {
                continue; // already scheduled: keep position and kind
            }
            pending.upd.insert(0, (t, o, ph));
        }
        for (t, o, ph) in fresh.ins {
            if let Some(i) = pending.ins.iter().position(|(x, _, _)| *x == t) {
                let e = pending.ins.remove(i);
                pending.ins.insert(0, e); // stays an insert, moves to head
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| *x == t) {
                pending.upd.remove(i);
                pending.upd.insert(0, (t, o, ph)); // re-staged as update at head
                continue;
            }
            pending.ins.insert(0, (t, o, ph));
        }
        pending
    }
}

/// Node behavior kind. Join extends tuples by the matched right fact;
/// Not/Exists (D-031) propagate the LEFT tuple unchanged, gated on blocker
/// absence/presence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Join,
    Not,
    Exists,
}

/// Beta-memory index kind (D-032). Equality hash indexes apply to every
/// node kind; COMPARISON (range) indexes apply to not/exists nodes only —
/// IndexUtil.canHaveRangeIndexForNodeType — on the first relational join
/// constraint with Number/Number or same-class Comparable operands. The
/// op is the constraint's op: right_field OP left_binding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Index {
    None,
    Eq,
    Cmp(crate::drl::CmpOp),
}

/// One child tuple of a node (join: left extended by the right fact;
/// not/exists: a copy of the left, `right` = None).
struct Child {
    tuple: Tup,
    left: Tup,
    right: Option<FactId>,
    dead: bool,
}

/// A join node's beta memory. Index keys are STORED: they are recomputed
/// only when the owning tuple is staged as an update (stale-index
/// semantics — constraints always evaluate live values, the bucket lookup
/// uses the stored key).
pub struct Node {
    pub kind: Kind,
    /// (left prefix, stored key). List order is memory order. For
    /// not/exists this holds UNBLOCKED lefts only (blocked lefts live on
    /// their blocker's blocked list — PhreakNot/ExistsNode semantics).
    lefts: Vec<(Tup, Option<Vec<Value>>)>,
    rights: Vec<(FactId, Option<Vec<Value>>)>,
    pub s_left: Staged<Tup>,
    pub s_right: Staged<FactId>,
    children: Vec<Child>,
    child_ix: HashMap<Tup, usize>,
    by_left: HashMap<Tup, Vec<usize>>,
    by_right: HashMap<FactId, Vec<usize>>,
    /// Existential blocker state: per-right blocked lefts (index 0 = most
    /// recently blocked; RightTuple.addBlocked PREPENDS), plus the reverse
    /// pointer. Empty for join nodes.
    blocked: HashMap<FactId, Vec<Tup>>,
    blocker_of: HashMap<Tup, FactId>,
    /// Existential-reorder capture for staged right updates
    /// (tempBlocked / tempNextRightTuple).
    temp_blocked: HashMap<FactId, Vec<Tup>>,
    temp_next: HashMap<FactId, Option<FactId>>,
    /// Beta-memory index kind (equality hash / comparison range / none).
    pub index: Index,
}

impl Node {
    /// BetaNode.modifyObject on a mask MISS: the right tuple is re-added
    /// (removeAdd to the END, re-keyed) immediately, without staging and
    /// without child updates (fz_42_4359/3433 vs fz_42_1057 pins).
    pub fn re_add_right_fact(&mut self, f: FactId, key: Option<Vec<Value>>) {
        if let Some(i) = self.rights.iter().position(|(x, _)| *x == f) {
            self.rights.remove(i);
            self.rights.push((f, key));
        }
        // RightTuple.reAdd also re-appends its children in their LEFT
        // parents' child lists, preserving sync-walk alignment
        // (fz_123_1438 vs fz_42_4359/fz_42_1057 pins).
        if let Some(ids) = self.by_right.get(&f).cloned() {
            for c in ids {
                if !self.children[c].dead {
                    self.re_add_left(c);
                }
            }
        }
    }

    pub fn new(index: Index, kind: Kind) -> Node {
        Node {
            kind,
            lefts: Vec::new(),
            rights: Vec::new(),
            s_left: Staged::default(),
            s_right: Staged::default(),
            children: Vec::new(),
            child_ix: HashMap::new(),
            by_left: HashMap::new(),
            by_right: HashMap::new(),
            blocked: HashMap::new(),
            blocker_of: HashMap::new(),
            temp_blocked: HashMap::new(),
            temp_next: HashMap::new(),
            index,
        }
    }

    fn eq_indexed(&self) -> bool {
        self.index == Index::Eq
    }

    fn alive_children<'a>(&'a self, ids: &'a [usize]) -> impl Iterator<Item = usize> + 'a {
        ids.iter().copied().filter(|&i| !self.children[i].dead)
    }

    fn first_child_of_right(&self, f: FactId) -> Option<usize> {
        self.by_right.get(&f).and_then(|v| self.alive_children(v).next())
    }

    fn first_child_of_left(&self, l: &Tup) -> Option<usize> {
        self.by_left.get(l).and_then(|v| self.alive_children(v).next())
    }

    /// Create a child, linking it at the END of both parents' child lists
    /// (LeftTuple constructor semantics). `before_left`/`before_right`
    /// insert BEFORE that child instead (the sync-walk cursor threading of
    /// insertChildLeftTuple, which keeps child lists aligned with memory
    /// iteration order).
    fn create_child(
        &mut self,
        l: &Tup,
        f: FactId,
        before_left: Option<usize>,
        before_right: Option<usize>,
    ) -> Tup {
        let mut t = l.clone();
        t.push(f);
        let idx = self.children.len();
        self.children
            .push(Child { tuple: t.clone(), left: l.clone(), right: Some(f), dead: false });
        self.child_ix.insert(t.clone(), idx);
        let lv = self.by_left.entry(l.clone()).or_default();
        match before_left.and_then(|c| lv.iter().position(|&x| x == c)) {
            Some(p) => lv.insert(p, idx),
            None => lv.push(idx),
        }
        let rv = self.by_right.entry(f).or_default();
        match before_right.and_then(|c| rv.iter().position(|&x| x == c)) {
            Some(p) => rv.insert(p, idx),
            None => rv.push(idx),
        }
        t
    }

    /// Not/exists child: the LEFT tuple propagates unchanged (CE patterns
    /// contribute no tuple element, D-031); no right parent.
    fn create_ce_child(&mut self, l: &Tup) -> Tup {
        let t = l.clone();
        let idx = self.children.len();
        self.children.push(Child { tuple: t.clone(), left: l.clone(), right: None, dead: false });
        self.child_ix.insert(t.clone(), idx);
        self.by_left.entry(l.clone()).or_default().push(idx);
        t
    }

    /// The single live CE child of a left tuple, if propagated.
    fn ce_child_of(&self, l: &Tup) -> Option<usize> {
        self.by_left.get(l).and_then(|v| self.alive_children(v).next())
    }

    fn kill_child(&mut self, idx: usize) -> Tup {
        self.children[idx].dead = true;
        self.child_ix.remove(&self.children[idx].tuple);
        self.children[idx].tuple.clone()
    }

    /// reAddLeft: move the child to the END of its LEFT parent's list.
    fn re_add_left(&mut self, idx: usize) {
        if let Some(v) = self.by_left.get_mut(&self.children[idx].left) {
            if let Some(p) = v.iter().position(|&x| x == idx) {
                v.remove(p);
                v.push(idx);
            }
        }
    }

    /// reAddRight: move the child to the END of its RIGHT parent's list.
    fn re_add_right(&mut self, idx: usize) {
        let Some(r) = self.children[idx].right else { return };
        if let Some(v) = self.by_right.get_mut(&r) {
            if let Some(p) = v.iter().position(|&x| x == idx) {
                v.remove(p);
                v.push(idx);
            }
        }
    }

    /// Compare a stored index key against a probe key from the OTHER
    /// side: the probe is coerced to the stored component's type
    /// (fz_123_3057: a long probing a double-keyed memory widens, so
    /// -1 != -1.5; u14: a double probing a long-keyed memory truncates,
    /// so -1.5 == -1).
    fn keys_match(stored: &[Value], probe: &[Value]) -> bool {
        stored.len() == probe.len()
            && stored.iter().zip(probe).all(|(s, p)| match (s, p) {
                (Value::I64(a), Value::F64(b)) => *a == *b as i64,
                (Value::F64(a), Value::I64(b)) => *a == *b as f64,
                (a, b) => a == b,
            })
    }

    fn left_key(&self, l: &Tup) -> Option<&Vec<Value>> {
        self.lefts.iter().find(|(t, _)| t == l).and_then(|(_, k)| k.as_ref())
    }

    /// Lefts matching a probe key (indexed) or all lefts, memory order.
    /// The probe (a right-side key) coerces to each stored left key's type.
    fn lefts_bucket(&self, key: Option<&Vec<Value>>) -> Vec<Tup> {
        self.lefts
            .iter()
            .filter(|(_, k)| {
                !self.eq_indexed()
                    || match (k, key) {
                        (Some(sk), Some(pk)) => Node::keys_match(sk, pk),
                        _ => false,
                    }
            })
            .map(|(t, _)| t.clone())
            .collect()
    }

    /// Rights matching a probe key (a left-side key), coerced to each
    /// stored right key's type.
    fn rights_bucket(&self, key: Option<&Vec<Value>>) -> Vec<FactId> {
        self.rights
            .iter()
            .filter(|(_, k)| {
                !self.eq_indexed()
                    || match (k, key) {
                        (Some(sk), Some(pk)) => Node::keys_match(sk, pk),
                        _ => false,
                    }
            })
            .map(|(f, _)| *f)
            .collect()
    }

    /// Coerce a 1-element probe to the stored key's type (the probing
    /// side coerces to the memory side — u14/fz_123_3057 convention).
    fn coerce_probe(stored: &Value, probe: &Value) -> Value {
        match (stored, probe) {
            (Value::I64(_), Value::F64(b)) => Value::I64(*b as i64),
            (Value::F64(_), Value::I64(b)) => Value::F64(*b as f64),
            _ => probe.clone(),
        }
    }

    fn value_ord(a: &Value, b: &Value) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => x.cmp(y),
            (Value::F64(x), Value::F64(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
            (Value::Str(x), Value::Str(y)) => x.cmp(y),
            (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
            _ => Ordering::Equal,
        }
    }

    /// Matching lefts for a right probe in ITERATION order (D-032). For a
    /// COMPARISON index (TupleIndexRBTree): the walk starts at the range
    /// boundary nearest the probe and moves away from it — descending
    /// left keys for `field > $b` / `>=`, ascending for `<` / `<=` —
    /// FIFO within equal keys. Other indexes keep memory order.
    fn scan_lefts(&self, probe: Option<&Vec<Value>>) -> Vec<Tup> {
        let Index::Cmp(op) = self.index else {
            return self.lefts_bucket(probe);
        };
        let Some(pv) = probe.and_then(|p| p.first()) else { return Vec::new() };
        let mut hits: Vec<(usize, &Tup, &Value)> = Vec::new();
        for (i, (t, k)) in self.lefts.iter().enumerate() {
            let Some(kv) = k.as_ref().and_then(|k| k.first()) else { continue };
            let p = Node::coerce_probe(kv, pv);
            // constraint: right_field OP $b  ->  probe OP stored
            if crate::engine::eval_cmp_pub(&p, op, kv) {
                hits.push((i, t, kv));
            }
        }
        let desc = matches!(op, crate::drl::CmpOp::Gt | crate::drl::CmpOp::Ge);
        hits.sort_by(|(ai, _, ak), (bi, _, bk)| {
            let o = Node::value_ord(ak, bk);
            let o = if desc { o.reverse() } else { o };
            o.then(ai.cmp(bi))
        });
        hits.into_iter().map(|(_, t, _)| t.clone()).collect()
    }

    /// Matching rights for a left probe in ITERATION order (D-032):
    /// ascending right keys for `field > $b` / `>=` (nearest above the
    /// probe first), descending for `<` / `<=`; FIFO within equal keys.
    fn scan_rights(&self, probe: Option<&Vec<Value>>) -> Vec<FactId> {
        let Index::Cmp(op) = self.index else {
            return self.rights_bucket(probe);
        };
        let Some(pv) = probe.and_then(|p| p.first()) else { return Vec::new() };
        let mut hits: Vec<(usize, FactId, &Value)> = Vec::new();
        for (i, (f, k)) in self.rights.iter().enumerate() {
            let Some(kv) = k.as_ref().and_then(|k| k.first()) else { continue };
            let p = Node::coerce_probe(kv, pv);
            // constraint: right_field OP $b  ->  stored OP probe
            if crate::engine::eval_cmp_pub(kv, op, &p) {
                hits.push((i, *f, kv));
            }
        }
        let asc = matches!(op, crate::drl::CmpOp::Gt | crate::drl::CmpOp::Ge);
        hits.sort_by(|(ai, _, ak), (bi, _, bk)| {
            let o = Node::value_ord(ak, bk);
            let o = if asc { o } else { o.reverse() };
            o.then(ai.cmp(bi))
        });
        hits.into_iter().map(|(_, f, _)| f).collect()
    }
}

/// Callbacks the evaluation needs from the engine (constraint tests and
/// key computation read the fact store and compiled rule).
pub trait JoinEnv {
    /// Full constraint test (live values) for extending `l` with `f`.
    fn allowed(&self, node: usize, l: &Tup, f: FactId) -> bool;
    /// Index key of the LEFT side (binding-source values), live.
    fn key_of_left(&self, node: usize, l: &Tup) -> Option<Vec<Value>>;
    /// Index key of the RIGHT side (fact field values), live.
    fn key_of_right(&self, node: usize, f: FactId) -> Option<Vec<Value>>;
    /// isLeftUpdateOptimizationAllowed for this node's beta constraints:
    /// a still-allowed blocker survives a left update iff there is <=1
    /// beta constraint or every one is an equality (D-031).
    fn left_update_optimization(&self, node: usize) -> bool;
    /// Constraint test for existential nodes: the RANGE-indexed
    /// constraint is excluded — the index probe (with its coerce-to-
    /// stored-type truncation, ne_r3/ne_r5) already decided it and
    /// Drools never re-evaluates an indexed constraint (D-035).
    fn allowed_ce(&self, node: usize, l: &Tup, f: FactId) -> bool;
}

fn sr_ins_iter<T>(v: &[T]) -> Box<dyn Iterator<Item = &T> + '_> {
    if std::env::var("SEINE_JSR").map(|x| x == "tail").unwrap_or(false) {
        Box::new(v.iter().rev())
    } else {
        Box::new(v.iter())
    }
}

/// Run one node's doNode phases. `trg` receives the child deltas for the
/// next node (or the terminal). Dispatches on the node kind.
pub fn do_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    trg: &mut Staged<Tup>,
) {
    match node.kind {
        Kind::Join => do_join_node(env, node_idx, node, sl, sr, trg),
        Kind::Not | Kind::Exists => do_existential_node(env, node_idx, node, sl, sr, trg),
    }
}

/// PhreakJoinNode.doNode phase order.
fn do_join_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    trg: &mut Staged<Tup>,
) {
    let trace = std::env::var("SEINE_TRACE").is_ok();
    if trace {
        eprintln!(
            "do_node[{node_idx}] sl(ins={:?} upd={:?} del={:?}) sr(ins={:?} upd={:?} del={:?})",
            sl.ins, sl.upd, sl.del, sr.ins, sr.upd, sr.del
        );
    }
    // --- right deletes ---
    for (f, o, _) in &sr.del {
        if let Some(i) = node.rights.iter().position(|(x, _)| x == f) {
            node.rights.remove(i);
        }
        if let Some(ids) = node.by_right.get(f).cloned() {
            for c in ids {
                if !node.children[c].dead {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            }
        }
    }
    // --- left deletes ---
    for (l, o, _) in &sl.del {
        if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
            node.lefts.remove(i);
        }
        if let Some(ids) = node.by_left.get(l).cloned() {
            for c in ids {
                if !node.children[c].dead {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            }
        }
    }
    // --- reorder right memory: re-key + move to END; children reAddLeft ---
    for (f, _, _) in &sr.upd {
        if let Some(i) = node.rights.iter().position(|(x, _)| x == f) {
            node.rights.remove(i);
            node.rights.push((*f, env.key_of_right(node_idx, *f)));
        }
        if let Some(ids) = node.by_right.get(f).cloned() {
            for c in ids {
                if !node.children[c].dead {
                    node.re_add_left(c);
                }
            }
        }
    }
    // --- reorder left memory: remove all staged, re-add at the END in
    // staged-list order; children reAddRight ---
    for (l, _, _) in &sl.upd {
        if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
            node.lefts.remove(i);
        }
    }
    for (l, _, _) in &sl.upd {
        node.lefts.push((l.clone(), env.key_of_left(node_idx, l)));
        if let Some(ids) = node.by_left.get(l).cloned() {
            for c in ids {
                if !node.children[c].dead {
                    node.re_add_right(c);
                }
            }
        }
    }

    let staged_left_upd = |l: &Tup| sl.upd.iter().any(|(x, _, _)| x == l);

    // --- right updates ---
    for (f, o, _) in &sr.upd {
        if node.lefts.is_empty() {
            continue;
        }
        let rkey = node.rights.iter().find(|(x, _)| x == f).and_then(|(_, k)| k.clone());
        let bucket = node.lefts_bucket(rkey.as_ref());
        let mut first_child = node.first_child_of_right(*f);
        // Indexed bucket-change check via the FIRST child's left parent.
        if node.eq_indexed() {
            if let Some(fc) = first_child {
                let parent_key = node.left_key(&node.children[fc].left).cloned();
                let same = match (&parent_key, &rkey) {
                    (Some(pk), Some(rk)) => Node::keys_match(pk, rk),
                    _ => false,
                };
                if bucket.is_empty() || !same {
                    // index changed: delete all previous propagations
                    if let Some(ids) = node.by_right.get(f).cloned() {
                        for c in ids {
                            if !node.children[c].dead {
                                let t = node.kill_child(c);
                                trg.add_del(t, *o);
                            }
                        }
                    }
                    first_child = None;
                }
            }
        }
        if trace {
            eprintln!("  rupd f={f:?} rkey={rkey:?} bucket={bucket:?} first_child={first_child:?}");
        }
        if bucket.is_empty() {
            continue;
        }
        if first_child.is_none() {
            // fresh assert against the (new) bucket, skipping staged lefts
            for l in &bucket {
                if staged_left_upd(l) {
                    continue; // processed via left iteration
                }
                if env.allowed(node_idx, l, *f) {
                    let t = node.create_child(l, *f, None, None);
                    trg.add_ins_ph(t, *o, 2);
                }
            }
        } else {
            // same bucket: iterate and compare against the child list
            let ids: Vec<usize> = node.by_right.get(f).cloned().unwrap_or_default();
            let alive: Vec<usize> = ids.iter().copied().filter(|&i| !node.children[i].dead).collect();
            let mut ci = 0usize; // cursor into alive
            for l in &bucket {
                if staged_left_upd(l) {
                    continue; // children cannot be processed twice
                }
                let cur = alive.get(ci).copied();
                if env.allowed(node_idx, l, *f) {
                    match cur {
                        Some(c) if node.children[c].left == *l => {
                            trg.add_upd_ph(node.children[c].tuple.clone(), *o, 2);
                            node.re_add_left(c);
                            ci += 1;
                        }
                        _ => {
                            let t = node.create_child(l, *f, None, cur);
                            trg.add_ins_ph(t, *o, 2);
                        }
                    }
                } else if let Some(c) = cur {
                    if node.children[c].left == *l {
                        let t = node.kill_child(c);
                        trg.add_del(t, *o);
                        ci += 1;
                    }
                }
            }
        }
    }
    // --- left updates ---
    for (l, o, _) in &sl.upd {
        if !node.lefts.iter().any(|(x, _)| x == l) {
            continue; // was removed (invalid prefix upstream)
        }
        let lkey = node.left_key(l).cloned();
        let bucket = node.rights_bucket(lkey.as_ref());
        // stale-children pass (indexed only): drop children whose right
        // parent sits in a different bucket now
        if node.eq_indexed() {
            if let Some(ids) = node.by_left.get(l).cloned() {
                for c in ids {
                    if node.children[c].dead {
                        continue;
                    }
                    let rp = node.children[c].right.expect("join child has a right parent");
                    let rp_key =
                        node.rights.iter().find(|(x, _)| *x == rp).and_then(|(_, k)| k.clone());
                    let same = match (&rp_key, &lkey) {
                        (Some(rk), Some(lk)) => Node::keys_match(rk, lk),
                        _ => false,
                    };
                    if bucket.is_empty() || !same {
                        let t = node.kill_child(c);
                        trg.add_del(t, *o);
                    }
                }
            }
        }
        if bucket.is_empty() {
            continue;
        }
        let first_child = node.first_child_of_left(l);
        if first_child.is_none() {
            for f in &bucket {
                if env.allowed(node_idx, l, *f) {
                    let t = node.create_child(l, *f, None, None);
                    trg.add_ins_ph(t, *o, 2);
                }
            }
        } else {
            let ids: Vec<usize> = node.by_left.get(l).cloned().unwrap_or_default();
            let alive: Vec<usize> = ids.iter().copied().filter(|&i| !node.children[i].dead).collect();
            let mut ci = 0usize;
            for f in &bucket {
                let cur = alive.get(ci).copied();
                if env.allowed(node_idx, l, *f) {
                    match cur {
                        Some(c) if node.children[c].right == Some(*f) => {
                            trg.add_upd_ph(node.children[c].tuple.clone(), *o, 2);
                            node.re_add_right(c);
                            ci += 1;
                        }
                        _ => {
                            let t = node.create_child(l, *f, cur, None);
                            trg.add_ins_ph(t, *o, 2);
                        }
                    }
                } else if let Some(c) = cur {
                    if node.children[c].right == Some(*f) {
                        let t = node.kill_child(c);
                        trg.add_del(t, *o);
                        ci += 1;
                    }
                }
            }
        }
    }
    // --- right inserts: staged list head-first (newest staged first),
    // each APPENDED to memory (TupleList.add); joined against pre-batch
    // lefts ---
    for (f, o, _) in sr_ins_iter(&sr.ins) {
        let rkey = env.key_of_right(node_idx, *f);
        node.rights.push((*f, rkey.clone()));
        for l in node.lefts_bucket(rkey.as_ref()) {
            if env.allowed(node_idx, &l, *f) {
                let t = node.create_child(&l, *f, None, None);
                trg.add_ins_ph(t, *o, 1);
            }
        }
    }
    // --- left inserts: append to memory, join against full right memory ---
    for (l, o, _) in &sl.ins {
        node.lefts.push((l.clone(), env.key_of_left(node_idx, l)));
        let lkey = node.lefts.last().and_then(|(_, k)| k.clone());
        for f in node.rights_bucket(lkey.as_ref()) {
            if env.allowed(node_idx, l, f) {
                let t = node.create_child(l, f, None, None);
                trg.add_ins(t, *o);
            }
        }
    }
    if trace {
        eprintln!("  trg ins={:?} upd={:?} del={:?}", trg.ins, trg.upd, trg.del);
        eprintln!("  rights={:?} lefts={:?}", node.rights, node.lefts);
        for (f, ids) in &node.by_right {
            let alive: Vec<&Tup> =
                ids.iter().filter(|&&i| !node.children[i].dead).map(|&i| &node.children[i].tuple).collect();
            eprintln!("  post by_right[{f:?}] alive={alive:?}");
        }
    }
}

/// PhreakNotNode / PhreakExistsNode doNode — behavioral port (D-031).
///
/// Blocker model: every left tuple has at most one blocker (the first
/// matching right in bucket order); blocked lefts leave the left memory
/// and live on the blocker's blocked list (PREPEND). `not` propagates a
/// child while UNBLOCKED, `exists` while BLOCKED.
///
/// Phase order (both kinds): leftDel, existential-reorder-left,
/// existential-reorder-right, rightIns, rightUpd, rightDel, leftUpd,
/// leftIns. Staged-UPDATE lefts are skipped by every right-side walk
/// ("children cannot be processed twice") and re-attached to the walked
/// right's blocked list.
fn do_existential_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    trg: &mut Staged<Tup>,
) {
    let is_not = node.kind == Kind::Not;
    let trace = std::env::var("SEINE_TRACE").is_ok();
    if trace {
        eprintln!(
            "do_exist[{node_idx}:{:?}] sl(ins={:?} upd={:?} del={:?}) sr(ins={:?} upd={:?} del={:?})",
            node.kind, sl.ins, sl.upd, sl.del, sr.ins, sr.upd, sr.del
        );
    }

    // --- left deletes (BEFORE right processing — Not/Exists phase order) ---
    for (l, o, _) in &sl.del {
        if let Some(b) = node.blocker_of.remove(l) {
            if let Some(list) = node.blocked.get_mut(&b) {
                list.retain(|x| x != l);
            }
            if !is_not {
                if let Some(c) = node.ce_child_of(l) {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            }
        } else {
            if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
                node.lefts.remove(i);
            }
            if is_not {
                if let Some(c) = node.ce_child_of(l) {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            }
        }
    }

    // --- existential reorder LEFT memory: remove all staged-upd lefts,
    // re-add the unblocked ones at the END (re-keyed); a blocked left
    // whose blocker is itself staged is detached to force a fresh search
    // in the left-update phase ---
    for (l, _, _) in &sl.upd {
        if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
            node.lefts.remove(i);
        }
    }
    for (l, _, _) in &sl.upd {
        match node.blocker_of.get(l).copied() {
            None => {
                node.lefts.push((l.clone(), env.key_of_left(node_idx, l)));
            }
            Some(b) => {
                let b_staged = sr.upd.iter().any(|(x, _, _)| *x == b)
                    || sr.del.iter().any(|(x, _, _)| *x == b);
                if b_staged {
                    node.blocker_of.remove(l);
                    if let Some(list) = node.blocked.get_mut(&b) {
                        list.retain(|x| x != l);
                    }
                }
            }
        }
    }

    // --- existential reorder RIGHT memory (only when updates staged):
    // indexed memories temporarily remove staged deletes so a not-yet-
    // moved delete cannot split a bucket; each staged update captures its
    // blocked list (tempBlocked) and resume point (tempNextRightTuple =
    // next non-staged neighbor in its OLD bucket forward, else backward),
    // then re-keys to the END ---
    if !sr.upd.is_empty() {
        let staged_right_any = |f: FactId| {
            sr.del.iter().any(|(x, _, _)| *x == f) || sr.upd.iter().any(|(x, _, _)| *x == f)
        };
        let mut del_saved: Vec<(FactId, Option<Vec<Value>>)> = Vec::new();
        if node.index != Index::None {
            for (f, _, _) in &sr.del {
                if let Some(i) = node.rights.iter().position(|(x, _)| x == f) {
                    del_saved.push(node.rights.remove(i));
                }
            }
        }
        // resumeFromCurrent = false for COMPARISON indexes: no resume
        // points are captured, the tempBlocked walk restarts per left.
        let resume_from_current = !matches!(node.index, Index::Cmp(_));
        let mut readd: Vec<FactId> = Vec::new();
        for (f, _, _) in &sr.upd {
            let Some(i) = node.rights.iter().position(|(x, _)| x == f) else { continue };
            let f_key = node.rights[i].1.clone();
            let in_bucket = |k: &Option<Vec<Value>>| {
                !node.eq_indexed()
                    || match (k, &f_key) {
                        (Some(a), Some(b)) => Node::keys_match(a, b),
                        _ => false,
                    }
            };
            if node.blocked.get(f).map(|v| !v.is_empty()).unwrap_or(false) {
                if resume_from_current {
                    let mut tnext: Option<FactId> = None;
                    for (g, k) in node.rights[i + 1..].iter() {
                        if in_bucket(k) && !staged_right_any(*g) {
                            tnext = Some(*g);
                            break;
                        }
                    }
                    if tnext.is_none() {
                        for (g, k) in node.rights[..i].iter().rev() {
                            if in_bucket(k) && !staged_right_any(*g) {
                                tnext = Some(*g);
                                break;
                            }
                        }
                    }
                    node.temp_next.insert(*f, tnext);
                }
                let bl = node.blocked.remove(f).unwrap_or_default();
                for l in &bl {
                    node.blocker_of.remove(l);
                }
                node.temp_blocked.insert(*f, bl);
            }
            node.rights.remove(i);
            readd.push(*f);
        }
        for f in readd {
            node.rights.push((f, env.key_of_right(node_idx, f)));
        }
        for e in del_saved {
            node.rights.push(e);
        }
    }

    let staged_left_upd = |l: &Tup| sl.upd.iter().any(|(x, _, _)| x == l);

    // --- right inserts: add to memory, then block matching UNBLOCKED
    // lefts (bucket walk, staged-upd lefts skipped). not: kill the child;
    // exists: propagate one ---
    for (f, o, _) in sr_ins_iter(&sr.ins) {
        let rkey = env.key_of_right(node_idx, *f);
        node.rights.push((*f, rkey.clone()));
        if !node.lefts.is_empty() {
            for l in node.scan_lefts(rkey.as_ref()) {
                if staged_left_upd(&l) {
                    continue;
                }
                if env.allowed_ce(node_idx, &l, *f) {
                    node.blocker_of.insert(l.clone(), *f);
                    node.blocked.entry(*f).or_default().insert(0, l.clone());
                    if let Some(i) = node.lefts.iter().position(|(x, _)| x == &l) {
                        node.lefts.remove(i);
                    }
                    if is_not {
                        if let Some(c) = node.ce_child_of(&l) {
                            let t = node.kill_child(c);
                            trg.add_del(t, *o);
                        }
                    } else {
                        let t = node.create_ce_child(&l);
                        trg.add_ins_ph(t, *o, 1);
                    }
                }
            }
        }
    }

    // --- right updates: (1) block matching unblocked lefts like an
    // insert; (2) walk the tempBlocked lefts, re-searching from the
    // captured resume point (a missing resume point flips a loop-wide
    // from-start flag that PERSISTS for later staged updates) ---
    // isIndexedUnificationJoin || isComparison: range-indexed memories
    // always restart blocker searches from the range head (D-032).
    let mut iterate_from_start = matches!(node.index, Index::Cmp(_));
    for (f, o, _) in &sr.upd {
        let fkey = node.rights.iter().find(|(x, _)| x == f).and_then(|(_, k)| k.clone());
        if !node.lefts.is_empty() {
            for l in node.scan_lefts(fkey.as_ref()) {
                if staged_left_upd(&l) {
                    continue;
                }
                if env.allowed_ce(node_idx, &l, *f) {
                    node.blocker_of.insert(l.clone(), *f);
                    node.blocked.entry(*f).or_default().insert(0, l.clone());
                    if let Some(i) = node.lefts.iter().position(|(x, _)| x == &l) {
                        node.lefts.remove(i);
                    }
                    if is_not {
                        if let Some(c) = node.ce_child_of(&l) {
                            let t = node.kill_child(c);
                            trg.add_del(t, *o);
                        }
                    } else {
                        let t = node.create_ce_child(&l);
                        trg.add_ins_ph(t, *o, 2);
                    }
                }
            }
        }
        let temp = node.temp_blocked.remove(f).unwrap_or_default();
        if temp.is_empty() {
            continue;
        }
        let root = node.temp_next.remove(f).flatten();
        if root.is_none() {
            iterate_from_start = true;
        }
        for l in temp {
            if staged_left_upd(&l) {
                // re-attach so the left-update phase starts from a
                // consistent blocked state
                node.blocker_of.insert(l.clone(), *f);
                node.blocked.entry(*f).or_default().insert(0, l.clone());
                continue;
            }
            let start = if iterate_from_start { None } else { root };
            let nb = node.find_blocker(env, node_idx, &l, start, &sr);
            match nb {
                Some(b) => {
                    node.blocker_of.insert(l.clone(), b);
                    node.blocked.entry(b).or_default().insert(0, l.clone());
                }
                None => {
                    node.lefts.push((l.clone(), env.key_of_left(node_idx, &l)));
                    if is_not {
                        let t = node.create_ce_child(&l);
                        trg.add_ins_ph(t, *o, 2);
                    } else if let Some(c) = node.ce_child_of(&l) {
                        let t = node.kill_child(c);
                        trg.add_del(t, *o);
                    }
                }
            }
        }
    }

    // --- right deletes: remove from memory; each blocked left re-searches
    // from its bucket start (rights still staged for deletion are
    // ineligible). not: newly-unblocked lefts propagate; exists: their
    // child dies ---
    for (f, o, _) in &sr.del {
        if let Some(i) = node.rights.iter().position(|(x, _)| x == f) {
            node.rights.remove(i);
        }
        let bl = node.blocked.remove(f).unwrap_or_default();
        for l in bl {
            node.blocker_of.remove(&l);
            if staged_left_upd(&l) {
                continue; // handled by the left-update phase
            }
            let nb = node.find_blocker(env, node_idx, &l, None, &sr);
            match nb {
                Some(b) => {
                    node.blocker_of.insert(l.clone(), b);
                    node.blocked.entry(b).or_default().insert(0, l.clone());
                }
                None => {
                    node.lefts.push((l.clone(), env.key_of_left(node_idx, &l)));
                    if is_not {
                        let t = node.create_ce_child(&l);
                        trg.add_ins_ph(t, *o, 2);
                    } else if let Some(c) = node.ce_child_of(&l) {
                        let t = node.kill_child(c);
                        trg.add_del(t, *o);
                    }
                }
            }
        }
    }

    // --- left updates: keep a still-allowed blocker when every beta
    // constraint is equality-indexable (isLeftUpdateOptimizationAllowed),
    // else drop and re-search from the bucket start; propagate the state
    // transition ---
    let left_opt = env.left_update_optimization(node_idx);
    for (l, o, _) in &sl.upd {
        let lkey = env.key_of_left(node_idx, l);
        let mut blocker = node.blocker_of.get(l).copied();
        // memory can hold it (re-added by the reorder) — remove; it is
        // re-added on the propagation paths to keep iteration order
        if blocker.is_none() {
            if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
                node.lefts.remove(i);
            }
        } else if node.index != Index::None {
            // bucket-change check: for an equality index the blocker's
            // stored key must match the left's new key; for a comparison
            // index the blocker must head the left's new RANGE bucket
            // (firstRightTuple.getMemory() == blocker.getMemory()).
            let b = blocker.unwrap();
            let bkey = node.rights.iter().find(|(x, _)| *x == b).and_then(|(_, k)| k.clone());
            let same = match node.index {
                Index::Eq => match (&bkey, &lkey) {
                    (Some(bk), Some(lk)) => Node::keys_match(bk, lk),
                    _ => false,
                },
                Index::Cmp(_) => {
                    let first = node.scan_rights(lkey.as_ref()).into_iter().next();
                    match first {
                        None => false,
                        Some(fr) => {
                            let frk = node
                                .rights
                                .iter()
                                .find(|(x, _)| *x == fr)
                                .and_then(|(_, k)| k.clone());
                            match (&frk, &bkey) {
                                (Some(a), Some(b2)) => a == b2,
                                _ => false,
                            }
                        }
                    }
                }
                Index::None => true,
            };
            if !same {
                node.detach_blocked(l, b);
                blocker = None;
            }
        }
        if !left_opt {
            if let Some(b) = blocker {
                node.detach_blocked(l, b);
                blocker = None;
            }
        }
        let still_allowed = blocker.map(|b| env.allowed_ce(node_idx, l, b)).unwrap_or(false);
        if !still_allowed {
            if let Some(b) = blocker {
                node.detach_blocked(l, b);
            }
            // re-search from the beginning (it's a modify)
            let nb = node.find_blocker_plain(env, node_idx, l, lkey.as_ref());
            if let Some(b) = nb {
                node.blocker_of.insert(l.clone(), b);
                node.blocked.entry(b).or_default().insert(0, l.clone());
            }
        }
        let blocked_now = node.blocker_of.contains_key(l);
        let child = node.ce_child_of(l);
        if is_not {
            if blocked_now {
                if let Some(c) = child {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            } else if child.is_none() {
                node.lefts.push((l.clone(), lkey.clone()));
                let t = node.create_ce_child(l);
                trg.add_ins_ph(t, *o, 2);
            } else {
                let c = child.unwrap();
                trg.add_upd_ph(node.children[c].tuple.clone(), *o, 2);
                node.lefts.push((l.clone(), lkey.clone()));
            }
        } else {
            if !blocked_now {
                node.lefts.push((l.clone(), lkey.clone()));
                if let Some(c) = child {
                    let t = node.kill_child(c);
                    trg.add_del(t, *o);
                }
            } else if child.is_none() {
                let t = node.create_ce_child(l);
                trg.add_ins_ph(t, *o, 2);
            } else {
                let c = child.unwrap();
                trg.add_upd_ph(node.children[c].tuple.clone(), *o, 2);
            }
        }
    }

    // --- left inserts: find the first matching blocker in bucket order;
    // not propagates when none, exists when one is found ---
    for (l, o, _) in &sl.ins {
        let lkey = env.key_of_left(node_idx, l);
        let nb = node.find_blocker_plain(env, node_idx, l, lkey.as_ref());
        match nb {
            Some(b) => {
                node.blocker_of.insert(l.clone(), b);
                node.blocked.entry(b).or_default().insert(0, l.clone());
                if !is_not {
                    let t = node.create_ce_child(l);
                    trg.add_ins(t, *o);
                }
            }
            None => {
                node.lefts.push((l.clone(), lkey));
                if is_not {
                    let t = node.create_ce_child(l);
                    trg.add_ins(t, *o);
                }
            }
        }
    }

    if trace {
        eprintln!("  trg ins={:?} upd={:?} del={:?}", trg.ins, trg.upd, trg.del);
        eprintln!(
            "  rights={:?} lefts={:?} blocked={:?}",
            node.rights, node.lefts, node.blocked
        );
    }
}

impl Node {
    fn detach_blocked(&mut self, l: &Tup, b: FactId) {
        self.blocker_of.remove(l);
        if let Some(list) = self.blocked.get_mut(&b) {
            list.retain(|x| x != l);
        }
    }

    /// Blocker search from the bucket start (left inserts, left updates,
    /// right-delete rebinds use the staged-delete guard via `sr`).
    fn find_blocker_plain<E: JoinEnv>(
        &self,
        env: &E,
        node_idx: usize,
        l: &Tup,
        lkey: Option<&Vec<Value>>,
    ) -> Option<FactId> {
        self.scan_rights(lkey)
            .into_iter()
            .find(|f| env.allowed_ce(node_idx, l, *f))
    }

    /// Blocker search for existential right-update/right-delete walks:
    /// starts at `start` (a right fact in the left's bucket) or the bucket
    /// start, skipping rights staged for deletion.
    fn find_blocker<E: JoinEnv>(
        &self,
        env: &E,
        node_idx: usize,
        l: &Tup,
        start: Option<FactId>,
        sr: &Staged<FactId>,
    ) -> Option<FactId> {
        let lkey = env.key_of_left(node_idx, l);
        let bucket = self.scan_rights(lkey.as_ref());
        let begin = match start {
            Some(s) => bucket.iter().position(|f| *f == s).unwrap_or(0),
            None => 0,
        };
        bucket[begin..]
            .iter()
            .copied()
            .find(|f| {
                !sr.del.iter().any(|(x, _, _)| x == f) && env.allowed_ce(node_idx, l, *f)
            })
    }

    /// NotNode property-MISS reAdd (BetaNode.modifyObject with a
    /// non-intersecting mask -> NotNode.reorderRightTuple): the right
    /// tuple re-keys to the END of memory, its blocked lefts re-search
    /// from the captured resume point, and — faithfully to the null-sink
    /// call — lefts that find NO new blocker are NOT propagated and NOT
    /// returned to the left memory (D-031).
    pub fn not_mask_miss_re_add<E: JoinEnv>(&mut self, env: &E, node_idx: usize, f: FactId) {
        let Some(i) = self.rights.iter().position(|(x, _)| x == &f) else { return };
        let f_key = self.rights[i].1.clone();
        let in_bucket = |k: &Option<Vec<Value>>| {
            !self.eq_indexed()
                || match (k, &f_key) {
                    (Some(a), Some(b)) => Node::keys_match(a, b),
                    _ => false,
                }
        };
        let mut tnext: Option<FactId> = None;
        let has_blocked = self.blocked.get(&f).map(|v| !v.is_empty()).unwrap_or(false);
        if has_blocked && !matches!(self.index, Index::Cmp(_)) {
            for (g, k) in self.rights[i + 1..].iter() {
                if in_bucket(k) {
                    tnext = Some(*g);
                    break;
                }
            }
            if tnext.is_none() {
                for (g, k) in self.rights[..i].iter().rev() {
                    if in_bucket(k) {
                        tnext = Some(*g);
                        break;
                    }
                }
            }
        }
        self.rights.remove(i);
        self.rights.push((f, env.key_of_right(node_idx, f)));
        if !has_blocked {
            return;
        }
        let bl = self.blocked.remove(&f).unwrap_or_default();
        for l in &bl {
            self.blocker_of.remove(l);
        }
        let empty_sr: Staged<FactId> = Staged::default();
        for l in bl {
            let start = tnext; // None -> from-start
            let nb = self.find_blocker(env, node_idx, &l, start, &empty_sr);
            if let Some(b) = nb {
                self.blocker_of.insert(l.clone(), b);
                self.blocked.entry(b).or_default().insert(0, l.clone());
            }
            // no new blocker: null sink/ltm — the left stays detached
        }
    }
}
