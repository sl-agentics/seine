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

    // FIFO (append) variants: LEFT-input staging from working-memory
    // actions is consumed oldest-first (pr08/pr04/j01/c2 pins), unlike
    // right inputs and intra-evaluation propagation.
    pub fn add_ins_back(&mut self, t: T, origin: Origin) {
        if self.upd.iter().any(|(x, _, _)| *x == t) || self.ins.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        self.ins.push((t, origin, 0));
    }

    pub fn add_upd_back(&mut self, t: T, origin: Origin) {
        if self.ins.iter().any(|(x, _, _)| *x == t)
            || self.upd.iter().any(|(x, _, _)| *x == t)
            || self.del.iter().any(|(x, _, _)| *x == t)
        {
            return;
        }
        self.upd.push((t, origin, 2));
    }

    pub fn add_del_back(&mut self, t: T, origin: Origin) {
        if let Some(i) = self.ins.iter().position(|(x, _, _)| *x == t) {
            self.ins.remove(i);
            return;
        }
        if let Some(i) = self.upd.iter().position(|(x, _, _)| *x == t) {
            self.upd.remove(i);
        }
        if self.del.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        self.del.push((t, origin, 0));
    }
}

/// One child tuple of a join (a tuple of length j+1 at node j).
struct Child {
    tuple: Tup,
    left: Tup,
    right: FactId,
    dead: bool,
}

/// A join node's beta memory. Index keys are STORED: they are recomputed
/// only when the owning tuple is staged as an update (stale-index
/// semantics — constraints always evaluate live values, the bucket lookup
/// uses the stored key).
pub struct Node {
    /// (left prefix, stored key). List order is memory order.
    lefts: Vec<(Tup, Option<Vec<Value>>)>,
    rights: Vec<(FactId, Option<Vec<Value>>)>,
    pub s_left: Staged<Tup>,
    pub s_right: Staged<FactId>,
    children: Vec<Child>,
    child_ix: HashMap<Tup, usize>,
    by_left: HashMap<Tup, Vec<usize>>,
    by_right: HashMap<FactId, Vec<usize>>,
    /// Whether this join has an equality join constraint (indexed).
    pub indexed: bool,
    /// True for the first join node (left input fed by the LIA/segment
    /// root rather than a previous join).
    pub first: bool,
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

    pub fn new(indexed: bool, first: bool) -> Node {
        Node {
            lefts: Vec::new(),
            rights: Vec::new(),
            s_left: Staged::default(),
            s_right: Staged::default(),
            children: Vec::new(),
            child_ix: HashMap::new(),
            by_left: HashMap::new(),
            by_right: HashMap::new(),
            indexed,
            first,
        }
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
        self.children.push(Child { tuple: t.clone(), left: l.clone(), right: f, dead: false });
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
        if let Some(v) = self.by_right.get_mut(&self.children[idx].right) {
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
                !self.indexed
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
                !self.indexed
                    || match (k, key) {
                        (Some(sk), Some(pk)) => Node::keys_match(sk, pk),
                        _ => false,
                    }
            })
            .map(|(f, _)| *f)
            .collect()
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
}

/// Run one join node's doNode phases. `trg` receives the child deltas for
/// the next node (or the terminal).
pub fn do_node<E: JoinEnv>(
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
        if node.indexed {
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
        if node.indexed {
            if let Some(ids) = node.by_left.get(l).cloned() {
                for c in ids {
                    if node.children[c].dead {
                        continue;
                    }
                    let rp = node.children[c].right;
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
                        Some(c) if node.children[c].right == *f => {
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
                    if node.children[c].right == *f {
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
    for (f, o, _) in sr.ins.iter() {
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
