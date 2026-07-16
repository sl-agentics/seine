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

use std::collections::{HashMap, HashSet};

use crate::store::{FactId, Value};

pub type Tup = Vec<FactId>;
pub type Origin = Option<usize>;

/// TupleSets: three LIFO lists (index 0 = most recently staged) with the
/// upstream fold rules.
#[derive(Clone)]
pub struct Staged<T: Clone + PartialEq + Eq + std::hash::Hash> {
    pub ins: std::collections::VecDeque<(T, Origin, u8)>,
    pub upd: std::collections::VecDeque<(T, Origin, u8)>,
    pub del: std::collections::VecDeque<(T, Origin, u8)>,
    /// NORMALIZED deletes (D-041/fz_123_2748): a delete that cancelled a
    /// pending INSERT at the first sink still reaches the PEERS as a
    /// delete (TupleSetsImpl normalizedDeleteFirst / processPeerDeletes).
    /// Never consumed by the first sink; folded into peers' dels at
    /// propagation and dropped afterward.
    pub norm_del: Vec<(T, Origin, u8)>,
    /// KEPT-KIND inserts (D-071/fz_42_890): tuples whose child UPDATE
    /// resolved against the FIRST sink's pending INSERT at touch time
    /// (updateChildLeftTuple keeps the kind) — they travel in `ins` for
    /// the first sink but peer-copy as UPDATES: each Drools sink resolves
    /// its OWN child tuple's staged state, and an already-consumed peer
    /// stages the touch as an update (refiring its terminal).
    pub peer_upd: Vec<T>,
    /// SLOT MEMORY (D-047/fz_7_5801): enabled only on LIA-level pattern-0
    /// staging (trie s0_in). A cancelled staged INSERT records its list
    /// position; a later re-add of the same fact (external exit then
    /// re-enter while the rule is unlinked) takes the ORIGINAL slot
    /// instead of the head.
    pub slot_memory: bool,
    cancelled_slots: Vec<(T, usize)>,
    /// D-266: stale-positive membership accelerator — every element in
    /// ins/upd/del IS in `seen` (removed elements may linger; a hit just
    /// routes to the exact scans). A miss proves absence from all three
    /// lists, so the add_* dedup walk is skipped. EVERY site that puts
    /// an element into ins/upd/del must seen_add it — the grep audit in
    /// D-266 enumerates them.
    seen: HashSet<T>,
}

impl<T: Clone + PartialEq + Eq + std::hash::Hash> Default for Staged<T> {
    fn default() -> Self {
        Staged {
            ins: std::collections::VecDeque::new(),
            upd: std::collections::VecDeque::new(),
            del: std::collections::VecDeque::new(),
            norm_del: Vec::new(),
            peer_upd: Vec::new(),
            slot_memory: false,
            cancelled_slots: Vec::new(),
            seen: HashSet::new(),
        }
    }
}

impl<T: Clone + PartialEq + Eq + std::hash::Hash> Staged<T> {
    pub fn is_empty(&self) -> bool {
        self.ins.is_empty()
            && self.upd.is_empty()
            && self.del.is_empty()
            && self.norm_del.is_empty()
    }

    /// Drop remembered cancelled-slot positions (D-081: slot restore
    /// is scoped to the current fire boundary).
    pub fn clear_slots(&mut self) {
        self.cancelled_slots.clear();
    }

    pub fn take(&mut self) -> Staged<T> {
        let slot_memory = self.slot_memory;
        let out = std::mem::take(self);
        self.slot_memory = slot_memory;
        out
    }

    pub fn add_ins(&mut self, t: T, origin: Origin) {
        self.add_ins_ph(t, origin, 0)
    }

    /// NOTE: no del+ins fold — Drools folds by tuple OBJECT identity, and a
    /// re-created child is a NEW object (c13). `phase` records which processing
    /// phase created the entry: 0 = left-insert, 1 = right-insert,
    /// 2 = update-derived (terminal block ordering, D-027).
    pub fn add_ins_ph(&mut self, t: T, origin: Origin, phase: u8) {
        // D-266 fast path: a `seen` miss proves t is in NO list (and, since
        // cancelled_slots entries were once staged, not there either).
        if !self.seen.contains(&t) {
            self.seen.insert(t.clone());
            self.ins.push_front((t, origin, phase));
            return;
        }
        if self.upd.iter().any(|(x, _, _)| *x == t) || self.ins.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        if self.slot_memory {
            if let Some(i) = self.cancelled_slots.iter().position(|(x, _)| *x == t) {
                let (_, slot) = self.cancelled_slots.remove(i);
                let at = slot.min(self.ins.len());
                self.ins.insert(at, (t, origin, phase));
                return;
            }
        }
        self.ins.push_front((t, origin, phase));
    }

    pub fn add_upd(&mut self, t: T, origin: Origin) {
        self.add_upd_ph(t, origin, 2)
    }

    pub fn add_upd_ph(&mut self, t: T, origin: Origin, phase: u8) {
        // TupleSetsImpl.addUpdate: already staged (any list) -> no-op.
        if !self.seen.contains(&t) {
            self.seen.insert(t.clone()); // D-266 fast path (see add_ins_ph)
            self.upd.push_front((t, origin, phase));
            return;
        }
        if self.ins.iter().any(|(x, _, _)| *x == t)
            || self.upd.iter().any(|(x, _, _)| *x == t)
            || self.del.iter().any(|(x, _, _)| *x == t)
        {
            return;
        }
        self.upd.push_front((t, origin, phase));
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
                if let Some(e) = pending.ins.remove(i) {
                    pending.ins.push_front(e); // stays an insert, moves to head
                }
                continue;
            }
            if let Some(i) = pending.upd.iter().position(|(x, _, _)| *x == t) {
                pending.upd.remove(i);
            }
            if pending.del.iter().any(|(x, _, _)| *x == t) {
                continue;
            }
            pending.seen_add(&t);
            pending.upd.push_front((t, o, ph));
        }
        // D-266: O(N+P) form of the per-element `.rev()` head-prepend walk
        // this replaces — the walk's result is exactly [trg.ins entries
        // not already pending, in trg order] ++ [pending.ins unchanged],
        // with the growing-list dedup also skipping intra-trg repeats
        // (the set absorbs kept keys). Byte-order-identical.
        let mut dedup: HashSet<T> =
            pending.ins.iter().map(|(x, _, _)| x.clone()).collect();
        let mut merged: std::collections::VecDeque<(T, Origin, u8)> =
            std::collections::VecDeque::with_capacity(trg.ins.len() + pending.ins.len());
        for (t, o, ph) in trg.ins.into_iter() {
            if dedup.contains(&t) {
                continue;
            }
            dedup.insert(t.clone());
            pending.seen_add(&t);
            merged.push_back((t, o, ph));
        }
        merged.extend(pending.ins.drain(..));
        pending.ins = merged;
        pending
    }

    /// Unstage a pending INSERT of `t` (true when found) — the
    /// cross-window clash primitive (updateChildLeftTuple, D-041).
    pub fn remove_ins(&mut self, t: &T) -> bool {
        if let Some(i) = self.ins.iter().position(|(x, _, _)| x == t) {
            self.ins.remove(i);
            return true;
        }
        false
    }

    pub fn remove_upd(&mut self, t: &T) -> bool {
        if let Some(i) = self.upd.iter().position(|(x, _, _)| x == t) {
            self.upd.remove(i);
            return true;
        }
        false
    }

    pub fn add_del(&mut self, t: T, origin: Origin) {
        if !self.seen.contains(&t) {
            self.seen.insert(t.clone()); // D-266 fast path (see add_ins_ph)
            self.del.push_front((t, origin, 0));
            return;
        }
        if let Some(i) = self.ins.iter().position(|(x, _, _)| *x == t) {
            self.ins.remove(i); // never materialized: cancel
            if self.slot_memory {
                self.cancelled_slots.push((t, i));
            }
            return;
        }
        if let Some(i) = self.upd.iter().position(|(x, _, _)| *x == t) {
            self.upd.remove(i);
        }
        if self.del.iter().any(|(x, _, _)| *x == t) {
            return;
        }
        self.del.push_front((t, origin, 0));
    }

    /// D-266: external staging sites that push into ins/upd/del directly
    /// must register the element here (see the `seen` invariant).
    pub fn seen_add(&mut self, t: &T) {
        self.seen.insert(t.clone());
    }

    /// D-267: stale-positive membership probe — `false` PROVES t is in
    /// none of ins/upd/del (seen ⊇ lists); `true` means "maybe", and the
    /// caller falls back to its exact scan.
    pub fn maybe_contains(&self, t: &T) -> bool {
        self.seen.contains(t)
    }

    /// Segment propagation to the FIRST-built sink (D-036/D-037/D-041):
    /// TupleSetsImpl.addAll is a BLIND tail concatenation — batches stack
    /// FIFO for a lagging first sink (fz_42_580) and cross-window clashes
    /// were already resolved at child-touch time inside do_node against
    /// this pending (updateChildLeftTuple, fz_123_8822).
    pub fn append_into_pending(mut pending: Staged<T>, fresh: Staged<T>) -> Staged<T> {
        // D-266: cross-Staged concatenation — register with pending's seen.
        for (t, _, _) in fresh.ins.iter().chain(fresh.del.iter()).chain(fresh.upd.iter()) {
            pending.seen_add(t);
        }
        pending.ins.extend(fresh.ins);
        pending.del.extend(fresh.del);
        pending.upd.extend(fresh.upd);
        // norm_del is a peer-only signal: the first sink's pending
        // insert was already cancelled at touch time. peer_upd markers
        // carry through (consumed only by peer copies, cleared on take).
        pending.peer_upd.extend(fresh.peer_upd);
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
    /// Accumulate/collect (D-038): memories and staging live here; the
    /// per-left contexts and result propagation are engine-side
    /// (eval_acc_node) because results are synthetic store facts.
    Acc,
    /// `?query` pull CE (D-056): only the left staging and sinks are
    /// used; evaluation is engine-side (eval_query_ce_node) through the
    /// Q1 stack machine. Never reaches do_node.
    Query,
    /// Subnetwork-fed not/exists (P1c/D-089): the counting machine
    /// (PhreakSubnetworkNotExistsNode port). Left staging and sinks live
    /// on the node; rights are subnetwork TUPLES staged engine-side
    /// (TrieNode.sn_right). Never reaches do_node.
    SubnetNot,
    SubnetExists,
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

/// D-266: hashable single-element key for the transient drain index.
#[derive(PartialEq, Eq, Hash)]
pub(crate) enum EqKey {
    I(i64),
    S(String),
    B(bool),
}

/// D-266: the transient per-drain-loop index (see Node::build_eq_idx).
pub(crate) struct EqIdx {
    class: u8, // 0=I64 1=Str 2=Bool 3=empty memory
    map: std::collections::HashMap<EqKey, Vec<usize>>,
}

const EMPTY_POS: &Vec<usize> = &Vec::new();

impl EqIdx {
    /// Memory positions matching the probe, ascending — or None when the
    /// probe's keys_match arm is not pure post-coercion equality (caller
    /// falls back to the linear filter).
    fn positions(&self, probe: Option<&Vec<Value>>) -> Option<&Vec<usize>> {
        let Some(p) = probe else { return Some(EMPTY_POS) }; // no key: eq never matches
        let [v] = p.as_slice() else { return Some(EMPTY_POS) }; // len mismatch: no match
        let ek = match (self.class, v) {
            (_, Value::Null) => return Some(EMPTY_POS), // null never equi-joins (D-097 pin F)
            (0, Value::I64(x)) => EqKey::I(*x),
            // stored I64 vs F64 probe: keys_match is `a == b as i64`
            (0, Value::F64(x)) => EqKey::I(*x as i64),
            (1, Value::Str(s)) => EqKey::S(s.clone()),
            (2, Value::Bool(b)) => EqKey::B(*b),
            (3, _) => return Some(EMPTY_POS), // empty memory
            // Dec probes (dec_cmp arms) or cross-class probes that the
            // match arms decide non-trivially: decline, caller filters.
            (0, Value::Dec { .. }) => return None,
            // remaining cross-class combinations are plain `a == b` on
            // different variants = false
            _ => return Some(EMPTY_POS),
        };
        Some(self.map.get(&ek).unwrap_or(EMPTY_POS))
    }
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
    /// CEP E1 (D-101): temporal-join node. Flips the insert-phase
    /// composition (leftIns fills BEFORE rightIns joins — t1/t6 pins)
    /// and both scans iterate partners DESCENDING by stored ts key.
    pub temporal: bool,
    /// D-082: left ARRIVAL sequence (stamped oldest-first per staged
    /// batch). Memory (`lefts`) order is certified staged-iteration
    /// order — NOT arrival — so the update-entry right pass sorts its
    /// walk by this instead.
    lseq: HashMap<Tup, u64>,
    lseq_next: u64,
    left_fire: HashMap<Tup, u64>,
    /// D-102 (721/526 rel_arrival): cross-side STAGE sequences — the
    /// partner scan orders lefts relative to the right's own staging
    /// moment (post-r arrival first, then pre-r arrival).
    pub left_sseq: HashMap<Tup, u64>,
    pub right_sseq: HashMap<FactId, u64>,
    /// D-102: >1 rule's path contains this node (set at lists_built).
    /// Shared temporal nodes use the this-fire-first partner scan and
    /// the stay-at-flush stash; unshared keep certified behavior.
    pub shared: bool,
    /// D-134 (§3B, temporal `not` firing-deferral): a satisfied temporal
    /// `not` left does NOT fire at insert (Drools defers to the pseudo-clock
    /// window close). `new_deferrals` carries (left, origin, fire_time) OUT
    /// to the engine, which schedules the release in `fire_deadlines`;
    /// `pending_release` carries due lefts back IN so a re-eval fires them.
    /// Both empty for non-temporal / non-not nodes (byte-identical path).
    pub new_deferrals: Vec<(Tup, Origin, i64)>,
    pub pending_release: Vec<(Tup, Origin)>,
    /// D-136 (shared temporal-join ORDER): the epoch's per-arrival D-125
    /// emissions, accumulated in FORWARD (D-125) order. A shared temporal
    /// join can't route to its peer sinks per-arrival — the peer copy
    /// reverses each single-tuple batch (a no-op) and term_pending drains
    /// between arrivals, so the WHOLE-epoch reversal the oracle wants never
    /// forms. Instead the flush accumulates here and the fire boundary
    /// drains the whole batch ONCE (first sink forward, peers reversed —
    /// `model_shared_tjo.py`, 0-div). Empty on every non-shared / non-
    /// temporal node (byte-identical path).
    pub tj_epoch: Vec<(Tup, Origin, u8)>,
    /// D-170 (T6 self-slot, temporal joins): right-memory order at the
    /// last fire boundary + this epoch's move/insert log. An entry scan
    /// replays these to place the ENTERING fact at its pre-epoch slot
    /// when its same-epoch moves were tag-class (model_tjupd_v4 T6-4).
    pub epoch_rights0: Vec<FactId>,
    /// (fact, op): 0 = ts-class move, 1 = tag-class move, 2 = insert.
    pub epoch_rlog: Vec<(FactId, u8)>,
    /// Facts tag-class-moved this epoch (self-slot candidates).
    pub self_dirty: HashSet<FactId>,
    /// D-170: arrival stamps for staged UPDATES (ins stamps live in
    /// left_sseq/right_sseq) — the mixed-batch replay orders ops by them.
    pub upd_rsseq: HashMap<FactId, u64>,
    pub upd_lsseq: HashMap<Tup, u64>,
    /// D-170: PENDING memory-move ops, one per update ACTION (stamped;
    /// dedup-proof — a re-touch of a staged upd appends another move,
    /// tu11x95), applied by the replay in global stamp order so they
    /// interleave correctly with still-staged inserts (tu11x92).
    pub pending_rmoves: Vec<(FactId, bool, u64)>,
    pub pending_lmoves: Vec<(Tup, u64)>,
    /// Stamps below this belong to PRIOR epochs (their moves apply
    /// silently — no epoch log / self-slot marking).
    pub epoch_floor: u64,
}

impl Node {
    pub fn stamp_left_seq(&mut self, l: &Tup) {
        if !self.lseq.contains_key(l) {
            self.lseq.insert(l.clone(), self.lseq_next);
            self.lseq_next += 1;
        }
    }

    pub fn left_seq(&self, l: &Tup) -> u64 {
        self.lseq.get(l).copied().unwrap_or(0)
    }

    /// D-166 (update-recency): give an updated left a FRESH arrival
    /// counter — it re-enters the partner scan at its new (tail) memory
    /// position, chronologically ordered against later fills.
    pub fn refresh_left_seq(&mut self, l: &Tup) {
        self.lseq.remove(l);
        self.stamp_left_seq(l);
    }

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
        Self::new_ex(index, kind, false)
    }

    pub fn new_ex(index: Index, kind: Kind, temporal: bool) -> Node {
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
            temporal,
            lseq: HashMap::new(),
            lseq_next: 1,
            left_fire: HashMap::new(),
            left_sseq: HashMap::new(),
            right_sseq: HashMap::new(),
            shared: false,
            new_deferrals: Vec::new(),
            pending_release: Vec::new(),
            tj_epoch: Vec::new(),
            epoch_rights0: Vec::new(),
            epoch_rlog: Vec::new(),
            self_dirty: HashSet::new(),
            upd_rsseq: HashMap::new(),
            upd_lsseq: HashMap::new(),
            pending_rmoves: Vec::new(),
            pending_lmoves: Vec::new(),
            epoch_floor: 0,
        }
    }

    /// D-170 (T6 self-slot): reset the per-epoch right-memory order
    /// bookkeeping at the fire boundary — the NEXT epoch's entry scans
    /// replay from this snapshot.
    pub fn epoch_reset(&mut self, floor: u64) {
        self.epoch_rights0 = self.rights.iter().map(|(f, _)| *f).collect();
        self.epoch_rlog.clear();
        self.self_dirty.clear();
        self.epoch_floor = floor;
        // upd stamps and pending moves are NOT cleared: an unlinked
        // node's staged upds survive fire boundaries and keep their
        // per-action arrival order (tju_r3 / tu11x95).
    }

    /// D-170 (T6-4): the entry scan's right-memory VIEW for the entering
    /// fact `f` — the epoch-start order with this epoch's move/insert log
    /// replayed, SKIPPING f's tag-class moves (they are invisible to f's
    /// own scan; ts-only moves and every other fact's moves replay).
    /// None when f is clean this epoch (use the live order).
    pub fn scan_rights_view(&self, f: FactId) -> Option<Vec<FactId>> {
        if !self.self_dirty.contains(&f) {
            return None;
        }
        let cur: std::collections::HashSet<FactId> =
            self.rights.iter().map(|(x, _)| *x).collect();
        let mut order = self.epoch_rights0.clone();
        for (g, op) in &self.epoch_rlog {
            match op {
                2 => {
                    if !order.contains(g) {
                        order.push(*g);
                    }
                }
                1 if *g == f => {}
                _ => {
                    if let Some(i) = order.iter().position(|x| x == g) {
                        order.remove(i);
                        order.push(*g);
                    }
                }
            }
        }
        order.retain(|x| cur.contains(x));
        for (x, _) in &self.rights {
            if !order.contains(x) {
                order.push(*x);
            }
        }
        Some(order)
    }

    /// Accumulate-node memory accessors (D-038): the engine-side
    /// evaluator manages contexts and results but reuses the node's
    /// memories, staging and bucket conventions.
    pub fn lefts_bucket_pub(&self, key: Option<&Vec<Value>>) -> Vec<Tup> {
        self.lefts_bucket(key)
    }

    pub fn rights_bucket_pub(&self, key: Option<&Vec<Value>>) -> Vec<FactId> {
        self.rights_bucket(key)
    }

    pub fn left_key_pub(&self, l: &Tup) -> Option<Vec<Value>> {
        self.left_key(l).cloned()
    }

    pub fn right_key_pub(&self, f: FactId) -> Option<Vec<Value>> {
        self.rights.iter().find(|(x, _)| *x == f).and_then(|(_, k)| k.clone())
    }

    /// D-101: drain staged right INSERTS into memory in ARRIVAL order
    /// (reverse of the prepend list), creating no children — the
    /// link-moment memory fill (model-check survivor family).
    pub fn drain_staged_rights_to_memory<E: JoinEnv>(
        &mut self,
        env: &E,
        node_idx: usize,
        exclude: Option<FactId>,
    ) {
        self.drain_staged_rights_to_memory_if(env, node_idx, exclude, &|_| true)
    }

    /// D-102/a3: the drain runs from LIA-loop link effects — BEFORE the
    /// trie loop stages the dying fact's delete — so DEAD facts' held
    /// ins must stay staged for the del-annihilation to find them.
    pub fn drain_staged_rights_to_memory_if<E: JoinEnv>(
        &mut self,
        env: &E,
        node_idx: usize,
        exclude: Option<FactId>,
        alive: &dyn Fn(FactId) -> bool,
    ) {
        let ins = std::mem::take(&mut self.s_right.ins);
        let (keep, drain): (Vec<_>, Vec<_>) = ins
            .into_iter()
            .partition(|(f, _, _)| Some(*f) == exclude || !alive(*f));
        for (f, _, _) in drain.iter().rev() {
            let rkey = env.key_of_right(node_idx, *f);
            self.rights.push((*f, rkey));
        }
        self.s_right.ins = keep.into();
    }

    /// D-102 (drain_t): an UNLINKED temporal node's per-insert flush
    /// moves the trigger's staged ins (both sides) to memory in
    /// arrival order, creating no children.
    pub fn self_drain_delta<E: JoinEnv>(&mut self, env: &E, node_idx: usize) {
        let fno = env.fire_no();
        let ins = std::mem::take(&mut self.s_right.ins);
        for (f, _, _) in ins.iter().rev() {
            let rkey = env.key_of_right(node_idx, *f);
            self.rights.push((*f, rkey));
        }
        let lins = std::mem::take(&mut self.s_left.ins);
        for (l, _, _) in lins.iter().rev() {
            self.stamp_left_seq(l);
            let lkey = env.key_of_left(node_idx, l);
            self.left_fire.insert(l.clone(), fno);
            self.lefts.push((l.clone(), lkey));
        }
    }

    /// D-125: the PER-ARRIVAL temporal flush (the v2 flush-model port,
    /// `tools/model_join_flush.py` — 0-div vs the gate oracle on ~4300
    /// shuffled cases). Consumes this node's staged INSERTS at a stream
    /// flush: a staged right (arrival order) appends to memory and
    /// eager-joins the left memory INDIVIDUALLY in memory order; a
    /// staged left (getInsertFirst order — `s0_folds` first, then
    /// s_left) appends to memory with lseq stamped in that SAME order —
    /// so a batch emitted by an anchor draining held partners keeps its
    /// single staged-prepend reversal — then joins the right memory in
    /// memory order. Emissions stage into `trg` via addInsert-prepend:
    /// a lone eager emit is identity, a batch of N held partners
    /// reverses exactly once. The caller routes `trg` to the sink.
    pub fn flush_ins_delta<E: JoinEnv>(
        &mut self,
        env: &E,
        node_idx: usize,
        s0_folds: Vec<(Tup, Origin, u8)>,
        trg: &mut Staged<Tup>,
    ) {
        let fno = env.fire_no();
        let rins = std::mem::take(&mut self.s_right.ins);
        // D-156 (tj pair-order): a SELF-join arrival stages the SAME fact on
        // both sides of one flush call. Drools propagates its LEFT insert
        // FIRST (that walk never sees the not-yet-inserted self-right), then
        // its RIGHT insert (whose walk sees the just-inserted self-left) —
        // probe battery t1-t9 + model_shared_tjo SEINE_TJO_SELF, 0-div/1000.
        // With trg's prepend inverting phase order, the rights-first loop
        // already puts the left-role batch first; only the self-pair's phase
        // MEMBERSHIP moves: the right walk appends the pending same-fact
        // staged left (⇒ post-reversal the self-pair heads the right-role
        // batch), and the left walk skips the same-flush self-right.
        // Single-side batches are byte-identical by construction.
        let flush_rights: Vec<FactId> = rins.iter().map(|(f, _, _)| *f).collect();
        for (f, o, _) in rins.iter().rev() {
            let rkey = env.key_of_right(node_idx, *f);
            self.rights.push((*f, rkey));
            let mut partners: Vec<Tup> = self.lefts.iter().map(|(l, _)| l.clone()).collect();
            for (l, _, _) in s0_folds.iter().chain(self.s_left.ins.iter()) {
                if l.len() == 1 && l[0] == *f {
                    partners.push(l.clone()); // the pending self-left
                }
            }
            for l in partners {
                if l.iter().any(|lf| env.is_expired(*lf)) {
                    continue; // D-102: corpse lefts make no NEW pairs
                }
                if env.allowed(node_idx, &l, *f) {
                    let t = self.create_child(&l, *f, None, None);
                    trg.add_ins_ph(t, *o, 1);
                }
            }
        }
        let lins = std::mem::take(&mut self.s_left.ins);
        for (l, o, _) in s0_folds.iter().chain(lins.iter()) {
            self.stamp_left_seq(l);
            self.left_fire.insert(l.clone(), fno);
            let lkey = env.key_of_left(node_idx, l);
            self.lefts.push((l.clone(), lkey));
            let partners: Vec<FactId> = self.rights.iter().map(|(f, _)| *f).collect();
            for f in partners {
                if env.is_expired(f) {
                    continue; // D-102: corpse rights make no NEW pairs
                }
                // D-156: the self-pair was emitted by the RIGHT walk above
                // (Drools' left propagation ran before the self-right existed)
                if l.len() == 1 && l[0] == f && flush_rights.contains(&f) {
                    continue;
                }
                if env.allowed(node_idx, l, f) {
                    let t = self.create_child(l, f, None, None);
                    trg.add_ins_ph(t, *o, 0);
                }
            }
        }
    }

    pub fn lefts_is_empty(&self) -> bool {
        self.lefts.is_empty()
    }

    pub fn rights_is_empty(&self) -> bool {
        self.rights.is_empty()
    }

    pub fn lefts_snapshot(&self) -> Vec<Tup> {
        self.lefts.iter().map(|(l, _)| l.clone()).collect()
    }

    pub fn push_left(&mut self, l: Tup, key: Option<Vec<Value>>) {
        self.lefts.push((l, key));
    }

    pub fn remove_left(&mut self, l: &Tup) {
        if let Some(i) = self.lefts.iter().position(|(x, _)| x == l) {
            self.lefts.remove(i);
        }
    }

    /// removeAdd for a staged left update: re-key and move to the END.
    pub fn re_add_left_tuple(&mut self, l: &Tup, key: Option<Vec<Value>>) {
        self.remove_left(l);
        self.lefts.push((l.clone(), key));
    }

    /// Peer-copy of a batch into THIS node's left staging (D-041,
    /// SegmentPropagator.processPeer* for a beta-node sink): per-entry
    /// prepends (batch reversal, LIFO stacking); update clashes SKIP
    /// (keep position and kind); insert clashes move to the head
    /// (updateChildLeftTupleDuringInsert); an insert whose tuple is
    /// ALREADY MATERIALIZED in this node's left memory is a memory
    /// removeAdd (move to the END, key kept) with NOTHING staged — the
    /// re-delivered peer neither re-joins nor refires (fz_123_8822).
    pub fn peer_merge_left(&mut self, fresh: &Staged<Tup>) {
        let mut pending = self.s_left.take();
        for (t, o, _) in &fresh.del {
            pending.add_del(t.clone(), *o);
        }
        for (t, o, _) in &fresh.norm_del {
            pending.add_del(t.clone(), *o); // processPeerDeletes(normalized)
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
            if fresh.peer_upd.contains(t) {
                // kept-kind insert (D-071): this peer's child was already
                // consumed — stage as an UPDATE with the usual
                // staged-clash skip (fz_999_3298 semantics).
                let staged = pending.ins.iter().any(|(x, _, _)| x == t)
                    || pending.upd.iter().any(|(x, _, _)| x == t)
                    || pending.del.iter().any(|(x, _, _)| x == t);
                if !staged {
                    pending.seen_add(t);
                    pending.upd.push_front((t.clone(), *o, *ph));
                }
                continue;
            }
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
            if let Some(i) = self.lefts.iter().position(|(x, _)| x == t) {
                let e = self.lefts.remove(i);
                self.lefts.push(e);
                continue;
            }
            pending.seen_add(t);
            pending.ins.push_front((t.clone(), *o, *ph));
        }
        self.s_left = pending;
    }

    /// Tuples currently blocked by `f` (D-076/tms_t21 park scope).
    pub fn blocked_of(&self, f: FactId) -> Option<Vec<Tup>> {
        self.blocked.get(&f).cloned()
    }

    /// D-201 (the mutfirst teardown, model x119/x30): reverse `f`'s
    /// blocked list so the right-del release emits INSERTION order —
    /// a mutfirst race key "never propagated": D consumes t0 order
    /// even when D is declared first.
    pub fn blocked_reverse_of(&mut self, f: FactId) {
        if let Some(v) = self.blocked.get_mut(&f) {
            v.reverse();
        }
    }

    pub fn push_right(&mut self, f: FactId, key: Option<Vec<Value>>) {
        self.rights.push((f, key));
    }

    pub fn remove_right(&mut self, f: FactId) {
        if let Some(i) = self.rights.iter().position(|(x, _)| *x == f) {
            self.rights.remove(i);
        }
    }

    /// removeAdd for a staged right update: re-key and move to the END.
    pub fn re_add_right_tuple(&mut self, f: FactId, key: Option<Vec<Value>>) {
        self.remove_right(f);
        self.rights.push((f, key));
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
        // (helper below is free-standing; see key_ts)
        Self::keys_match_inner(stored, probe)
    }

    fn keys_match_inner(stored: &[Value], probe: &[Value]) -> bool {
        stored.len() == probe.len()
            && stored.iter().zip(probe).all(|(s, p)| match (s, p) {
                // D-097/pin F: a null key component never equi-joins —
                // not even against another null (UNKNOWN, not equal).
                (Value::Null, _) | (_, Value::Null) => false,
                // D-098: decimal keys match by VALUE across scales (pin J).
                (Value::Dec { u: a, s: x }, Value::Dec { u: b, s: y }) => {
                    crate::store::dec_cmp(*a, *x, *b, *y) == std::cmp::Ordering::Equal
                }
                (Value::Dec { u: a, s: x }, Value::I64(b)) => {
                    crate::store::dec_cmp(*a, *x, *b as i128, 0) == std::cmp::Ordering::Equal
                }
                (Value::I64(a), Value::Dec { u: b, s: y }) => {
                    crate::store::dec_cmp(*a as i128, 0, *b, *y) == std::cmp::Ordering::Equal
                }
                (Value::I64(a), Value::F64(b)) => *a == *b as i64,
                (Value::F64(a), Value::I64(b)) => *a == *b as f64,
                (a, b) => a == b,
            })
    }

    fn left_key(&self, l: &Tup) -> Option<&Vec<Value>> {
        self.lefts.iter().find(|(t, _)| t == l).and_then(|(_, k)| k.as_ref())
    }

    /// D-266: an order-preserving transient index over one side's memory
    /// for the flush-drain probe loops. Built ONCE per drain loop while
    /// the probed memory is static (the staged-rights loop only pushes
    /// rights, so the lefts memory it probes is fixed, and vice versa).
    /// Scope is deliberately narrow so the semantics are EXACTLY the
    /// linear filter's: eq-indexed nodes, single-element stored keys,
    /// one homogeneous class (I64 / Str / Bool — F64/Dec/Null stored
    /// keys fall back), and only probe classes whose keys_match arm is
    /// a pure equality after the certified coercion (an I64-class probe
    /// may be I64 or F64 — `a == b as i64`, the same truncation; a Dec
    /// or other probe returns None = caller falls back). Bucket vecs
    /// hold memory POSITIONS in ascending order, so emission order is
    /// the filter's memory order, byte for byte.
    fn build_eq_idx<'a, I: Iterator<Item = &'a Option<Vec<Value>>>>(
        &self,
        keys: I,
    ) -> Option<EqIdx> {
        if !self.eq_indexed() {
            return None;
        }
        let mut class: Option<u8> = None; // 0=I64 1=Str 2=Bool
        let mut map: std::collections::HashMap<EqKey, Vec<usize>> =
            std::collections::HashMap::new();
        for (pos, k) in keys.enumerate() {
            let Some(k) = k else { continue }; // None-keyed: never eq-matches
            let [v] = k.as_slice() else { return None }; // multi-element: fall back
            let (c, ek) = match v {
                Value::I64(x) => (0u8, EqKey::I(*x)),
                Value::Str(s) => (1u8, EqKey::S(s.clone())),
                Value::Bool(b) => (2u8, EqKey::B(*b)),
                _ => return None, // F64/Dec/Null stored: fall back
            };
            match class {
                None => class = Some(c),
                Some(pc) if pc != c => return None, // mixed classes: fall back
                _ => {}
            }
            map.entry(ek).or_default().push(pos);
        }
        Some(EqIdx { class: class.unwrap_or(3), map })
    }

    pub(crate) fn build_lefts_eq_idx(&self) -> Option<EqIdx> {
        self.build_eq_idx(self.lefts.iter().map(|(_, k)| k))
    }

    pub(crate) fn build_rights_eq_idx(&self) -> Option<EqIdx> {
        self.build_eq_idx(self.rights.iter().map(|(_, k)| k))
    }

    /// lefts_bucket through a transient index when it can answer exactly;
    /// the linear filter otherwise. Same output, same order, always.
    fn lefts_bucket_idx(&self, idx: Option<&EqIdx>, key: Option<&Vec<Value>>) -> Vec<Tup> {
        if let Some(ix) = idx {
            if let Some(hits) = ix.positions(key) {
                return hits.iter().map(|&p| self.lefts[p].0.clone()).collect();
            }
        }
        self.lefts_bucket(key)
    }

    /// rights_bucket through a transient index (see lefts_bucket_idx).
    fn rights_bucket_idx(&self, idx: Option<&EqIdx>, key: Option<&Vec<Value>>) -> Vec<FactId> {
        if let Some(ix) = idx {
            if let Some(hits) = ix.positions(key) {
                return hits.iter().map(|&p| self.rights[p].0).collect();
            }
        }
        self.rights_bucket(key)
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
            (Value::Dec { u: x, s: xs }, Value::Dec { u: y, s: ys }) => {
                crate::store::dec_cmp(*x, *xs, *y, *ys)
            }
            (Value::Dec { u: x, s: xs }, Value::I64(y)) => {
                crate::store::dec_cmp(*x, *xs, *y as i128, 0)
            }
            (Value::I64(x), Value::Dec { u: y, s: ys }) => {
                crate::store::dec_cmp(*x as i128, 0, *y, *ys)
            }
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
    /// D-102: expiration-FLAGGED events are skipped as fresh JOIN
    /// partners (eager flag, lazy retraction — existential blocking
    /// persists until the quiescence delete).
    fn is_expired(&self, _f: FactId) -> bool {
        false
    }
    /// D-102 cycle 4: the current fire index — left fills AND left
    /// self-drains stamp it; the pop partner scan treats THIS-fire
    /// lefts as fresh (arrival) and prior-fire lefts newest-first.
    fn fire_no(&self) -> u64 {
        0
    }
    /// D-102 (cf101x134): TRUE during a stream-flush evaluation —
    /// temporal nodes then FILL lefts without pairing (children are
    /// created only at pop-time evaluations).
    fn in_flush(&self) -> bool {
        false
    }
    /// D-170 (T6): TRUE when the owning rule is the 2-pattern shape the
    /// mixed-batch replay is certified for (model_tjupd_v4's world).
    fn two_pattern(&self) -> bool {
        false
    }
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
    /// D-134 (§3B): fire_time for a temporal `not` left in the DEFERRED
    /// regime (window has not yet fully closed): after ⇒ anchor.ts+hi,
    /// before[0,hi] ⇒ anchor.ts. None = fire NOW (the IMMEDIATE regime
    /// before[lo>0], a non-after/before op, or a non-temporal not). The
    /// caller (`not_emit_or_defer`) defers whenever this is Some(ft); the
    /// engine's `drain_pending_fires` decides WHEN to release (clock >= ft).
    fn not_fire_time(&self, _node: usize, _l: &Tup) -> Option<i64> {
        None
    }
}

fn sr_ins_iter<T>(v: &std::collections::VecDeque<T>) -> Box<dyn Iterator<Item = &T> + '_> {
    if std::env::var("SEINE_JSR").map(|x| x == "tail").unwrap_or(false) {
        Box::new(v.iter().rev())
    } else {
        Box::new(v.iter())
    }
}

/// Run one node's doNode phases. `trg` receives the child deltas for the
/// next node (or the terminal). Dispatches on the node kind.
/// Child staging with CROSS-WINDOW clash handling (D-041): Drools
/// resolves a touched child against the FIRST sink's pending staging at
/// touch time (updateChildLeftTuple / deleteChildLeftTuple /
/// normalizeStagedTuples) and restages it inside the CURRENT batch —
/// batch propagation itself (addAll) is a blind list concatenation.
pub struct Out<'a> {
    pub trg: &'a mut Staged<Tup>,
    pub pending: &'a mut Staged<Tup>,
}

impl<'a> Out<'a> {
    pub(crate) fn child_ins(&mut self, t: Tup, o: Origin, ph: u8) {
        self.trg.add_ins_ph(t, o, ph);
    }

    /// updateChildLeftTuple: a child staged as INSERT in the pending
    /// moves into the current batch KEEPING its insert kind; a pending
    /// UPDATE moves as an update; otherwise stage an update normally.
    pub(crate) fn child_upd(&mut self, t: Tup, o: Origin, ph: u8) {
        if self.pending.remove_ins(&t) {
            // kind kept for the first sink; peers resolve their own
            // staged state and see an UPDATE (D-071/fz_42_890).
            self.trg.peer_upd.push(t.clone());
            self.trg.add_ins_ph(t, o, ph);
        } else {
            self.pending.remove_upd(&t);
            self.trg.add_upd_ph(t, o, ph);
        }
    }

    /// deleteChildLeftTuple: a never-consumed pending INSERT cancels at
    /// the first sink but still reaches the peers as a NORMALIZED delete
    /// (fz_123_2748); a pending UPDATE is unstaged before the delete.
    /// D-170 (T6): the A' refire's emission — prepends AND STEALS an
    /// already-staged upd from this eval's earlier ($b-refire) pass, so
    /// the A' block owns the consume slot for shared pairs (dt4/int2/
    /// ip1: the self-pair fires in the A' block). Ins-staged absorb.
    pub(crate) fn child_upd_front(&mut self, t: Tup, o: Origin, ph: u8) {
        if self.pending.remove_ins(&t) {
            self.trg.peer_upd.push(t.clone());
            self.trg.add_ins_ph(t, o, ph);
            return;
        }
        self.pending.remove_upd(&t);
        self.trg.remove_upd(&t);
        if self.trg.ins.iter().any(|(x, _, _)| *x == t)
            || self.trg.del.iter().any(|(x, _, _)| *x == t)
        {
            return;
        }
        self.trg.seen_add(&t);
        self.trg.upd.push_front((t, o, ph));
    }

    pub(crate) fn child_del(&mut self, t: Tup, o: Origin) {
        if self.pending.remove_ins(&t) {
            self.trg.norm_del.push((t, o, 0));
            return;
        }
        self.pending.remove_upd(&t);
        self.trg.add_del(t, o);
    }
}

/// D-101: timestamp from a temporal node's stored key (i64 ms).
fn key_ts(k: &Option<Vec<Value>>) -> i64 {
    match k.as_ref().and_then(|v| v.first()) {
        Some(Value::I64(n)) => *n,
        _ => i64::MIN,
    }
}

pub fn do_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    trg: &mut Staged<Tup>,
    pending: &mut Staged<Tup>,
) {
    let mut out = Out { trg, pending };
    match node.kind {
        Kind::Join => do_join_node(env, node_idx, node, sl, sr, &mut out),
        Kind::Not | Kind::Exists => do_existential_node(env, node_idx, node, sl, sr, &mut out),
        Kind::Acc => unreachable!("accumulate nodes evaluate engine-side"),
        Kind::Query => unreachable!("?query CE nodes evaluate engine-side (D-056)"),
        Kind::SubnetNot | Kind::SubnetExists => {
            unreachable!("subnetwork CE nodes evaluate engine-side (D-089)")
        }
    }
}

/// PhreakJoinNode.doNode phase order.
/// D-170 (T6): the MIXED-batch replay for a temporal 2-pattern join —
/// a batch carrying staged UPDATES (per-action evals of tag/ts writes,
/// and unlinked accumulations across actions) replays its ops in
/// ARRIVAL-STAMP order, each op with the model_tjupd_v4 semantics:
///   LIns  = anchor entry/fill: memory append, then scan the CURRENT
///           right memory (scan_rights_view supplies the self-slot);
///   RIns  = right arrival: memory append, partners in LEFT-memory order;
///   RUpd  = $b refire: tail move (epoch-logged by class) + children
///           refired anchors-in-memory-order, childlist move-on-refire;
///   LUpd  = A' refire: left tail move + lseq refresh + children in
///           child-list order via the steal-prepend (phase A slot).
/// Deletes were processed by the caller (matchCancelled first); clean
/// insert-only batches never come here (the certified D-125/D-156 arms).
fn temporal_upd_replay<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: &Staged<Tup>,
    sr: &Staged<FactId>,
    out: &mut Out<'_>,
) {
    enum Op {
        RIns(FactId, Origin),
        RUpd(FactId, Origin),
        LIns(Tup, Origin),
        LUpd(Tup, Origin),
        RMove(FactId, bool, bool),
        LMove(Tup, u64),
    }
    let mut ops: Vec<(u64, usize, Op)> = Vec::new();
    for (f, o, ph) in sr.ins.iter() {
        if *ph == 1 {
            continue; // re-entries keep the certified late pass below
        }
        let s = node.right_sseq.get(f).copied().unwrap_or(0);
        ops.push((s, ops.len(), Op::RIns(*f, *o)));
    }
    for (f, o, _) in sr.upd.iter() {
        let s = node.upd_rsseq.get(f).copied().unwrap_or(0);
        ops.push((s, ops.len(), Op::RUpd(*f, *o)));
    }
    for (l, o, _) in sl.ins.iter() {
        let s = node.left_sseq.get(l).copied().unwrap_or(0);
        ops.push((s, ops.len(), Op::LIns(l.clone(), *o)));
    }
    for (l, o, _) in sl.upd.iter() {
        let s = node.upd_lsseq.get(l).copied().unwrap_or(0);
        ops.push((s, ops.len(), Op::LUpd(l.clone(), *o)));
    }
    for (f, tagc, s) in std::mem::take(&mut node.pending_rmoves) {
        // prior-epoch moves (at or below the floor — the floor IS the
        // last pre-fire stamp) apply silently: they are pre-epoch state
        // to this epoch's self-slot view.
        let log = s > node.epoch_floor;
        ops.push((s, ops.len(), Op::RMove(f, tagc && log, log)));
    }
    for (l, s) in std::mem::take(&mut node.pending_lmoves) {
        ops.push((s, ops.len(), Op::LMove(l, s)));
    }
    ops.sort_by_key(|(s, i, _)| (*s, *i));
    if std::env::var("SEINE_TRACE").is_ok() {
        let names: Vec<String> = ops
            .iter()
            .map(|(s, _, op)| match op {
                Op::RIns(f, _) => format!("RIns({f:?})@{s}"),
                Op::RUpd(f, _) => format!("RUpd({f:?})@{s}"),
                Op::LIns(l, _) => format!("LIns({l:?})@{s}"),
                Op::LUpd(l, _) => format!("LUpd({l:?})@{s}"),
                Op::RMove(f, t, _) => format!("RMove({f:?},{t})@{s}"),
                Op::LMove(l, _) => format!("LMove({l:?})@{s}"),
            })
            .collect();
        eprintln!("  replay ops: {}", names.join(" "));
    }
    for (_, _, op) in ops {
        // Each op emits into its OWN staging (the op's single certified
        // prepend-reversal), then the block APPENDS to the eval's trg —
        // ops compose FIFO (the model's per-action buffer), unlike the
        // arm walk's global LIFO. An A' (LUpd) block STEALS its dup keys
        // from earlier blocks (the phase-A slot); other dups keep first
        // (ins absorb / u5).
        let mut op_trg: Staged<Tup> = Staged::default();
        let mut steal = false;
        {
            let mut op_out = Out { trg: &mut op_trg, pending: out.pending };
            match op {
                Op::LIns(l, o) => {
                    // a RE-ENTRY must not inherit its exited era's lseq —
                    // the memory append is a fresh arrival (tu11x197)
                    node.refresh_left_seq(&l);
                    node.left_fire.insert(l.clone(), env.fire_no());
                    let lkey = env.key_of_left(node_idx, &l);
                    node.lefts.push((l.clone(), lkey));
                    let view = match l.as_slice() {
                        [f] => node.scan_rights_view(*f),
                        _ => None,
                    };
                    let scan: Vec<FactId> = view
                        .unwrap_or_else(|| node.rights.iter().map(|(f, _)| *f).collect());
                    for f in scan {
                        if env.is_expired(f) {
                            continue;
                        }
                        if env.allowed(node_idx, &l, f) {
                            let t = node.create_child(&l, f, None, None);
                            op_out.child_ins(t, o, 0);
                        }
                    }
                }
                Op::RIns(f, o) => {
                    let rkey = env.key_of_right(node_idx, f);
                    node.rights.push((f, rkey));
                    let partners: Vec<Tup> =
                        node.lefts.iter().map(|(l, _)| l.clone()).collect();
                    for l in partners {
                        if l.iter().any(|x| env.is_expired(*x)) {
                            continue;
                        }
                        if env.allowed(node_idx, &l, f) {
                            let t = node.create_child(&l, f, None, None);
                            op_out.child_ins(t, o, 1);
                        }
                    }
                }
                Op::RMove(f, tagc, log) => {
                    // D-173 (⚖ the dedup/side-effect law: staged-op dedup
                    // folds EMISSIONS only — per-touch side effects run
                    // once per ACTION): the $b-refire's CHILDLIST
                    // move-to-end rides the per-action move op, not the
                    // dedup'd RUpd — a leading same-epoch tag-VI would
                    // otherwise pin the refire pass before an entry's
                    // self-child exists (tu51x207).
                    let ids: Vec<usize> =
                        node.by_right.get(&f).cloned().unwrap_or_default();
                    for c in ids {
                        if !node.children[c].dead {
                            node.re_add_left(c);
                        }
                    }
                    // one memory-move per update ACTION (dedup-proof) at
                    // its own stamp — interleaves with staged inserts
                    // (tu11x92/x95). Prior-epoch moves apply silently.
                    if let Some(i) = node.rights.iter().position(|(x, _)| *x == f) {
                        node.rights.remove(i);
                        node.rights.push((f, env.key_of_right(node_idx, f)));
                        if log {
                            node.epoch_rlog.push((f, if tagc { 1 } else { 0 }));
                        }
                        if tagc {
                            node.self_dirty.insert(f);
                        }
                    }
                }
                Op::LMove(l, stamp) => {
                    if let Some(i) = node.lefts.iter().position(|(x, _)| *x == l) {
                        node.lefts.remove(i);
                        node.lefts.push((l.clone(), env.key_of_left(node_idx, &l)));
                        node.refresh_left_seq(&l);
                        // the anchor's tag-touch move jumps sseq ERAS too —
                        // the rel_arrival partner sort orders by (sseq, lseq)
                        // and a stale era pins the anchor early (tu21x20)
                        node.left_sseq.insert(l.clone(), stamp);
                    }
                }
                Op::RUpd(f, o) => {
                    // pure refire EMISSIONS — the memory moves AND the
                    // childlist re-adds are per-action RMove side effects
                    // (D-173: dedup folds emissions, not effects).
                    let ids: Vec<usize> =
                        node.by_right.get(&f).cloned().unwrap_or_default();
                    let mut by_pos: Vec<(usize, usize)> = ids
                        .iter()
                        .copied()
                        .filter(|&c| !node.children[c].dead)
                        .map(|c| {
                            let p = node
                                .lefts
                                .iter()
                                .position(|(l, _)| *l == node.children[c].left)
                                .unwrap_or(usize::MAX);
                            (p, c)
                        })
                        .collect();
                    by_pos.sort_by_key(|(p, _)| *p);
                    for (_, c) in by_pos {
                        op_out.child_upd(node.children[c].tuple.clone(), o, 2);
                    }
                }
                Op::LUpd(l, o) => {
                    steal = true;
                    // pure A' refire — the memory moves are LMove ops.
                    let ids: Vec<usize> = node.by_left.get(&l).cloned().unwrap_or_default();
                    for c in ids {
                        if !node.children[c].dead {
                            op_out.child_upd(node.children[c].tuple.clone(), o, 2);
                            node.re_add_right(c);
                        }
                    }
                }
            }
        }
        // merge the op block FIFO into the eval's trg
        out.trg.peer_upd.extend(op_trg.peer_upd);
        for (t, o, ph) in op_trg.ins {
            if out.trg.ins.iter().any(|(x, _, _)| *x == t)
                || out.trg.upd.iter().any(|(x, _, _)| *x == t)
            {
                continue;
            }
            out.trg.seen_add(&t);
            out.trg.ins.push_back((t, o, ph));
        }
        for (t, o, ph) in op_trg.upd {
            if out.trg.ins.iter().any(|(x, _, _)| *x == t)
                || out.trg.del.iter().any(|(x, _, _)| *x == t)
            {
                continue;
            }
            if let Some(i) = out.trg.upd.iter().position(|(x, _, _)| *x == t) {
                if !steal {
                    continue; // keep first (u5)
                }
                out.trg.upd.remove(i); // A' steals the slot
            }
            out.trg.seen_add(&t);
            out.trg.upd.push_back((t, o, ph));
        }
    }
    // UPDATE-ENTRY right re-inserts (ph=1): the certified late pass.
    for (f, o, ph) in sr.ins.iter() {
        if *ph != 1 {
            continue;
        }
        let rkey = env.key_of_right(node_idx, *f);
        node.rights.push((*f, rkey.clone()));
        let mut lefts = node.lefts_bucket(rkey.as_ref());
        lefts.sort_by_key(|l| std::cmp::Reverse(node.left_seq(l)));
        for l in lefts {
            if env.allowed(node_idx, &l, *f) {
                let t = node.create_child(&l, *f, None, None);
                out.child_ins(t, *o, 1);
            }
        }
    }
}

fn do_join_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    out: &mut Out<'_>,
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
                    out.child_del(t, *o);
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
                    out.child_del(t, *o);
                }
            }
        }
    }
    // D-170 (T6, temporal 2-pattern): a batch carrying staged UPDATES
    // takes the arrival-ordered replay instead of the kind-grouped arm
    // walk (per-action evals + unlinked cross-action accumulations —
    // the model_tjupd_v4-certified world). Deletes were processed above.
    if node.temporal
        && !node.eq_indexed()
        && env.two_pattern()
        && (!sr.upd.is_empty() || !sl.upd.is_empty())
    {
        temporal_upd_replay(env, node_idx, node, &sl, &sr, out);
        if trace {
            eprintln!(
                "  (replay) trg ins={:?} upd={:?} del={:?}",
                out.trg.ins, out.trg.upd, out.trg.del
            );
            eprintln!("  rights={:?} lefts={:?}", node.rights, node.lefts);
        }
        return;
    }

    // D-170 (T6, temporal): snapshot each updated anchor's child list
    // BEFORE any right-side processing moves entries within it (the
    // reorder block's re_add_left included) — the A' iterates the
    // phase-A child list (ip1: (A,N) leads even though the $b side
    // moved the self-pair).
    let aprime_ids: Vec<(Tup, Vec<usize>)> = if node.temporal && !node.eq_indexed() {
        sl.upd
            .iter()
            .map(|(l, _, _)| (l.clone(), node.by_left.get(l).cloned().unwrap_or_default()))
            .collect()
    } else {
        Vec::new()
    };

    // --- reorder right memory: re-key + move to END; children reAddLeft ---
    // D-170 (T6, temporal): a DEFERRED multi-update batch (an unlinked
    // join accumulating staged upds across actions) applies its memory
    // moves in ACTION order = the staged prepend-list REVERSED (tju_r3:
    // two value-identical updates restore insertion order). Non-temporal
    // nodes keep the certified list order; per-action singleton batches
    // are identical either way.
    let sr_upd_ordered: Vec<&(FactId, Origin, u8)> = if node.temporal {
        sr.upd.iter().rev().collect()
    } else {
        sr.upd.iter().collect()
    };
    for (f, _, ph) in sr_upd_ordered {
        if let Some(i) = node.rights.iter().position(|(x, _)| x == f) {
            node.rights.remove(i);
            node.rights.push((*f, env.key_of_right(node_idx, *f)));
            // D-170 (T6): log the move for the self-slot replay; ph=6
            // marks a TAG-class update (the anchor-watched field was
            // written — staged by on_update), invisible to the fact's
            // own later entry scan.
            if node.temporal {
                let tagc = *ph == 6;
                node.epoch_rlog.push((*f, if tagc { 1 } else { 0 }));
                if tagc {
                    node.self_dirty.insert(*f);
                }
            }
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
        // D-166 (update-recency): a TEMPORAL node's partner scan sorts by
        // the (left_sseq, lseq) arrival stamps, which erases the move-to-END
        // above — refresh the updated left's lseq so it re-enters the scan
        // chronologically at its NEW memory position (tail now, before any
        // later fill; model_join_flush v3 usimulate, fuzzu 2000/2000; the
        // tj-tail family cf313x346/cf933x385).
        if node.temporal {
            node.refresh_left_seq(l);
        }
    }

    let staged_left_upd = |l: &Tup| sl.upd.iter().any(|(x, _, _)| x == l);

    // --- right updates ---
    for (f, o, _) in &sr.upd {
        if node.lefts.is_empty() {
            continue;
        }
        // D-170 (T6, temporal): a temporal match is ts0-frozen, so a
        // right update can never change the match set — it is a PURE
        // REFIRE of the live children, anchors in LEFT-memory order
        // (rendered reversed by the trg prepend, the model's ltm-scan
        // + prepend), each child moved to its left parent's list END
        // (re_add_left, the model's childlist move-on-refire).
        if node.temporal && !node.eq_indexed() {
            let ids: Vec<usize> = node.by_right.get(f).cloned().unwrap_or_default();
            let mut by_pos: Vec<(usize, usize)> = ids
                .iter()
                .copied()
                .filter(|&c| !node.children[c].dead)
                .map(|c| {
                    let p = node
                        .lefts
                        .iter()
                        .position(|(l, _)| *l == node.children[c].left)
                        .unwrap_or(usize::MAX);
                    (p, c)
                })
                .collect();
            by_pos.sort_by_key(|(p, _)| *p);
            for (_, c) in by_pos {
                out.child_upd(node.children[c].tuple.clone(), *o, 2);
                node.re_add_left(c);
            }
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
                                out.child_del(t, *o);
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
                    out.child_ins(t, *o, 2);
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
                            out.child_upd(node.children[c].tuple.clone(), *o, 2);
                            node.re_add_left(c);
                            ci += 1;
                        }
                        _ => {
                            let t = node.create_child(l, *f, None, cur);
                            out.child_ins(t, *o, 2);
                        }
                    }
                } else if let Some(c) = cur {
                    if node.children[c].left == *l {
                        let t = node.kill_child(c);
                        out.child_del(t, *o);
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
        // D-170 (T6, temporal): the A' refire — an anchor's in-place
        // update refires its own live children in child-list order via
        // the STEAL-PREPEND (child_upd_front): the A' block lands ahead
        // of this eval's $b-refire block at the consume, owning shared
        // pairs' slots, each block internally reversed (the model's
        // phase A-then-B with reversed-childlist emission). Match set
        // ts0-invariant, so pure refire.
        if node.temporal && !node.eq_indexed() {
            let ids: Vec<usize> = aprime_ids
                .iter()
                .find(|(x, _)| x == l)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            for c in ids {
                if !node.children[c].dead {
                    out.child_upd_front(node.children[c].tuple.clone(), *o, 2);
                    node.re_add_right(c);
                }
            }
            continue;
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
                        out.child_del(t, *o);
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
                    out.child_ins(t, *o, 2);
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
                            out.child_upd(node.children[c].tuple.clone(), *o, 2);
                            node.re_add_right(c);
                            ci += 1;
                        }
                        _ => {
                            let t = node.create_child(l, *f, cur, None);
                            out.child_ins(t, *o, 2);
                        }
                    }
                } else if let Some(c) = cur {
                    if node.children[c].right == Some(*f) {
                        let t = node.kill_child(c);
                        out.child_del(t, *o);
                        ci += 1;
                    }
                }
            }
        }
    }
    // --- FRESH right inserts (ph=0): staged list head-first (newest
    // staged first), each APPENDED to memory (TupleList.add); joined
    // against pre-batch lefts ---
    // --- FRESH right inserts (ph=0): staged list head-first (newest
    // staged first), each APPENDED to memory (TupleList.add); joined
    // against pre-batch lefts. Memory push order is UNCHANGED certified
    // behavior; arrival is tracked separately in lseq for pass B
    // (D-082). ---
    if node.temporal {
        // CEP E1 (D-101, model-check survivor): pre-link staged rights
        // drained to memory at the LINK moment (engine hook). The walk:
        // (a) ALL staged lefts FILL first (lseq stamped, no joins);
        // (b) rightIns staged head-first (newest) joins lefts in
        //     ARRIVAL (lseq) order;
        // (c) leftIns joins the PRE-BATCH right memory (incl. the
        //     link drain) in MEMORY order.
        let pre_rights: Vec<FactId> = node.rights.iter().map(|(f, _)| *f).collect();
        // D-102 cycle-4 (853/192/510, model_check_stream survivor
        // ws=per_fact_ab): a same-batch AB SELF-JOIN batch (identical
        // fact sets staged on both sides) walks PER-FACT newest-first:
        // (1) own-right x {older-staged + memory} lefts, (2) SELF-pair,
        // (3) own-left x older-staged rights (arrival) + memory rights.
        let l_fids: Vec<FactId> = sl.ins.iter().filter_map(|(l, _, _)| {
            if l.len() == 1 { Some(l[0]) } else { None }
        }).collect();
        let r_fids: Vec<FactId> =
            sr.ins.iter().filter(|(_, _, ph)| *ph != 1).map(|(f, _, _)| *f).collect();
        let is_ab_batch = !l_fids.is_empty()
            && l_fids.len() == sl.ins.len()
            && {
                let mut a = l_fids.clone();
                let mut b = r_fids.clone();
                a.sort_unstable();
                b.sort_unstable();
                a == b
            };
        if is_ab_batch {
            let fno = env.fire_no();
            // facts newest-first = staged order (prepend)
            let facts: Vec<(Tup, FactId)> = sl
                .ins
                .iter()
                .map(|(l, _, _)| (l.clone(), l[0]))
                .collect();
            // pre-batch memory snapshot in ARRIVAL (lseq) order — the
            // per-fact arm1 iterates it (cf53: fire-2 memory arm is
            // arrival, fire-3 confirms lseq carries across fires)
            let mut mem_lefts: Vec<Tup> =
                node.lefts.iter().map(|(l, _)| l.clone()).collect();
            mem_lefts.sort_by_key(|l| node.left_seq(l));
            for (l, _, _) in sl.ins.iter().rev() {
                node.stamp_left_seq(l);
                node.left_fire.insert(l.clone(), fno);
            }
            // fills enter memory in ARRIVAL order
            for (l, f) in facts.iter().rev() {
                let lkey = env.key_of_left(node_idx, l);
                node.lefts.push((l.clone(), lkey));
                let _ = f;
            }
            let origin = sl.ins.front().map(|(_, o, _)| *o).unwrap_or(None);
            // rights enter MEMORY in ARRIVAL order (853 fire-2: the
            // next fire's leftIns x memory iterates arrival)
            for (_, fid) in facts.iter().rev() {
                let rkey = env.key_of_right(node_idx, *fid);
                node.rights.push((*fid, rkey));
            }
            for (i, (l_f, fid)) in facts.iter().enumerate() {
                // arm 1: older staged lefts (arrival = reverse of the
                // remaining prepend list) then memory lefts
                let older_staged: Vec<&Tup> =
                    facts[i + 1..].iter().rev().map(|(l, _)| l).collect();
                for a in older_staged.into_iter().chain(mem_lefts.iter()) {
                    if a.iter().any(|x| env.is_expired(*x)) {
                        continue;
                    }
                    if env.allowed(node_idx, a, *fid) {
                        let t = node.create_child(a, *fid, None, None);
                        out.child_ins(t, origin, 1);
                    }
                }
                // arm 2: self
                if !env.is_expired(*fid) && env.allowed(node_idx, l_f, *fid) {
                    let t = node.create_child(l_f, *fid, None, None);
                    out.child_ins(t, origin, 1);
                }
                // arm 3: older staged rights (arrival) then memory rights
                let older_r: Vec<FactId> =
                    facts[i + 1..].iter().rev().map(|(_, f)| *f).collect();
                for b in older_r.into_iter().chain(pre_rights.iter().copied()) {
                    if env.is_expired(b) {
                        continue;
                    }
                    if env.allowed(node_idx, l_f, b) {
                        let t = node.create_child(l_f, b, None, None);
                        out.child_ins(t, origin, 0);
                    }
                }
                // D-170 (T6): the arriving anchor's CHILD LIST orders
                // [scan children..., self-pair] — the model appends the
                // left-scan pairs (its left_insert) before the self
                // (its right_insert), while the arm walk above created
                // the self first. Emission order stays certified; only
                // the by_left slot moves (the A' iterates it — dt4/ip1).
                if let Some(ids) = node.by_left.get(l_f).cloned() {
                    if let Some(&selfc) = ids
                        .iter()
                        .find(|&&c| !node.children[c].dead && node.children[c].right == Some(*fid))
                    {
                        node.re_add_left(selfc);
                    }
                }
            }
        } else {
        if node.shared {
            for (l, _, _) in sl.ins.iter().rev() {
                node.stamp_left_seq(l);
            }
        } else {
            // D-125 (v2 flush model): an UNSHARED temporal fill stamps
            // lseq in STAGED (getInsertFirst) order = memory order — a
            // genuine anchor-drain batch keeps its single staged-prepend
            // reversal for later right-insert partner scans. Eager
            // singles (the per-arrival flush cascade) are order-free.
            for (l, _, _) in &sl.ins {
                node.stamp_left_seq(l);
            }
        }
        for (l, _, _) in &sl.ins {
            let lkey = env.key_of_left(node_idx, l);
            node.left_fire.insert(l.clone(), env.fire_no());
            node.lefts.push((l.clone(), lkey));
        }
        for (f, o, ph) in sr_ins_iter(&sr.ins) {
            if *ph == 1 {
                continue;
            }
            let rkey = env.key_of_right(node_idx, *f);
            node.rights.push((*f, rkey));
            // D-102 cycle-4 round-2 survivor (model_check_stream,
            // simulated states): partner scan = THIS-FIRE lefts
            // (filled OR self-drained this fire) in ARRIVAL order,
            // then prior-fire lefts NEWEST-first.
            // D-102 rel_arrival (cycle-4 final survivor): lefts staged
            // strictly AFTER this right (arrival order) first, then
            // lefts staged at-or-before it (arrival order). Equals the
            // certified lseq-ASC scan when no post-right lefts exist.
            let rseq = node.right_sseq.get(f).copied().unwrap_or(0);
            let lsq = |l: &Tup| node.left_sseq.get(l).copied().unwrap_or(0);
            let mut post: Vec<Tup> = node
                .lefts
                .iter()
                .filter(|(l, _)| lsq(l) > rseq)
                .map(|(l, _)| l.clone())
                .collect();
            post.sort_by_key(|l| (lsq(l), node.left_seq(l)));
            let mut pre_p: Vec<Tup> = node
                .lefts
                .iter()
                .filter(|(l, _)| lsq(l) <= rseq)
                .map(|(l, _)| l.clone())
                .collect();
            pre_p.sort_by_key(|l| (lsq(l), node.left_seq(l)));
            let partners: Vec<Tup> = post.into_iter().chain(pre_p).collect();
            for l in partners {
                if l.iter().any(|lf| env.is_expired(*lf)) {
                    continue; // D-102: corpse lefts make no NEW pairs
                }
                if env.allowed(node_idx, &l, *f) {
                    let t = node.create_child(&l, *f, None, None);
                    out.child_ins(t, *o, 1);
                }
            }
        }
        for (l, o, _) in &sl.ins {
            // D-170 (T6-4): a single-fact left ENTERING the join scans a
            // VIEW that places the fact ITSELF at its pre-epoch slot when
            // its same-epoch moves were tag-class (its own eval's move
            // included — the log above already recorded it).
            let view: Option<Vec<FactId>> = match l.as_slice() {
                [f] => node.scan_rights_view(*f).map(|v| {
                    v.into_iter().filter(|x| pre_rights.contains(x)).collect()
                }),
                _ => None,
            };
            let scan: &[FactId] = view.as_deref().unwrap_or(&pre_rights);
            for f in scan {
                if env.is_expired(*f) {
                    continue; // D-102: corpse rights make no NEW pairs
                }
                if env.allowed(node_idx, l, *f) {
                    let t = node.create_child(l, *f, None, None);
                    out.child_ins(t, *o, 0);
                }
            }
        }
        } // end per-fact-AB / phased temporal insert split
    } else {
    // D-102 (pre_lifo_then_post_arr): rights staged while the path was
    // UNLINKED (ph=4, event sessions) process LIFO first; post-link
    // rights (ph=0) follow in ARRIVAL order (list reversed). Cloud
    // sessions never stamp ph=4, so this is the certified head-first
    // walk there.
    let has_pre = sr.ins.iter().any(|(_, _, ph)| *ph == 4);
    let ordered: Vec<&(FactId, Origin, u8)> = if has_pre {
        // event-session mixed generations: pre-link LIFO, then
        // post-link ARRIVAL (list reversed)
        sr.ins
            .iter()
            .filter(|(_, _, ph)| *ph == 4)
            .chain(sr.ins.iter().filter(|(_, _, ph)| *ph == 0).rev())
            .collect()
    } else {
        // certified head-first walk (cloud + pure-post batches)
        sr.ins.iter().filter(|(_, _, ph)| *ph != 1).collect()
    };
    // D-266: one transient index over the (static) lefts memory for the
    // whole staged-rights walk — the loop only pushes rights.
    let lefts_idx = if ordered.len() >= 16 && node.lefts.len() >= 64 {
        node.build_lefts_eq_idx()
    } else {
        None
    };
    for (f, o, _) in ordered {
        let rkey = env.key_of_right(node_idx, *f);
        node.rights.push((*f, rkey.clone()));
        // D-179/D-180 (fe4): the WALKING fact is corpse-checked too — a
        // flagged right whose fold was deferred past its flagging
        // (unlinked-path staging) makes no NEW pairs; the memory push
        // above stays (flag-eager, retraction-lazy).
        if env.is_expired(*f) {
            continue;
        }
        for l in node.lefts_bucket_idx(lefts_idx.as_ref(), rkey.as_ref()) {
            if l.iter().any(|lf| env.is_expired(*lf)) {
                continue; // D-102: corpse lefts make no NEW pairs
            }
            if env.allowed(node_idx, &l, *f) {
                let t = node.create_child(&l, *f, None, None);
                out.child_ins(t, *o, 1);
            }
        }
    }
    // --- left inserts: append to memory, join against full right
    // memory; arrival seq stamped oldest-first (.rev of the
    // prepend-staged list) ---
    for (l, _, _) in sl.ins.iter().rev() {
        node.stamp_left_seq(l);
    }
    // D-266: one transient index over the (static) rights memory for the
    // whole staged-lefts walk — the loop only pushes lefts.
    let rights_idx = if sl.ins.len() >= 16 && node.rights.len() >= 64 {
        node.build_rights_eq_idx()
    } else {
        None
    };
    for (l, o, _) in &sl.ins {
        node.lefts.push((l.clone(), env.key_of_left(node_idx, l)));
        // D-179/D-180 (fe2/fe6): the walking left tuple is corpse-checked
        // too (see the right walk above).
        if l.iter().any(|x| env.is_expired(*x)) {
            continue;
        }
        let lkey = node.lefts.last().and_then(|(_, k)| k.clone());
        for f in node.rights_bucket_idx(rights_idx.as_ref(), lkey.as_ref()) {
            if env.is_expired(f) {
                continue; // D-102: corpse rights make no NEW pairs
            }
            if env.allowed(node_idx, l, f) {
                let t = node.create_child(l, f, None, None);
                out.child_ins(t, *o, 0);
            }
        }
    }
    } // end plain-join insert phases (temporal flip above, D-101)
    // --- UPDATE-ENTRY right inserts (ph=1, D-082 model-check survivor):
    // processed AFTER left inserts (they see same-batch lefts in
    // memory), walking the left bucket NEWEST-first. Pinned by the
    // jr1..jr10 re-entry ladder vs the jw fresh-right matrix
    // (tools/model_check_join.py — jw3 and jr10 are event-identical
    // with opposite oracle orders; provenance is the discriminator).
    for (f, o, ph) in sr_ins_iter(&sr.ins) {
        if *ph != 1 {
            continue;
        }
        let rkey = env.key_of_right(node_idx, *f);
        node.rights.push((*f, rkey.clone()));
        let mut lefts = node.lefts_bucket(rkey.as_ref());
        // newest ARRIVAL first — bucket order is memory order, which is
        // NOT arrival order for same-batch staged lefts; lseq is.
        lefts.sort_by_key(|l| std::cmp::Reverse(node.left_seq(l)));
        for l in lefts {
            if env.allowed(node_idx, &l, *f) {
                let t = node.create_child(&l, *f, None, None);
                out.child_ins(t, *o, 1);
            }
        }
    }
    if trace {
        eprintln!("  trg ins={:?} upd={:?} del={:?}", out.trg.ins, out.trg.upd, out.trg.del);
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
///
/// D-134 (§3B) temporal `not`: a FRESH satisfied left does not fire at
/// insert — Drools DEFERS to the pseudo-clock window close (fire_time). A
/// blocker's later removal never re-fires it (blocked ⇒ silent forever, the
/// arc-B model_not_infer rule), so the un-block re-fire paths are SUPPRESSED
/// for a temporal not. The only fire path is the deferral release, replayed
/// via `pending_release`. All of this is gated on `node.temporal` — a
/// non-temporal `not` keeps the certified behavior byte-for-byte.
///
/// Emit a `not` child for a FRESH unblocked left, or DEFER it. Returns true
/// if a child was emitted (fired now: immediate regime, an already-due
/// deferral, or a non-temporal not); false if deferred (recorded in
/// `node.new_deferrals`, left kept in `node.lefts`, no child).
fn not_emit_or_defer<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    l: &Tup,
    o: Origin,
    ph: u8,
    out: &mut Out<'_>,
) -> bool {
    if node.temporal {
        if let Some(ft) = env.not_fire_time(node_idx, l) {
            // Deferred-regime temporal not: ALWAYS defer, even when already
            // due (ft <= clock). The engine's `drain_pending_fires` releases
            // the due ones at THIS fire's quiescence in the model's
            // (−fire_time, creation) order — so a clock-0 firing reads the
            // same order as an advance batch, and a blocker arriving later in
            // the SAME insert batch still suppresses (blocked ⇒ silent).
            node.new_deferrals.push((l.clone(), o, ft));
            return false;
        }
    }
    let t = node.create_ce_child(l);
    out.child_ins(t, o, ph);
    true
}

fn do_existential_node<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    out: &mut Out<'_>,
) {
    let is_not = node.kind == Kind::Not;
    let trace = std::env::var("SEINE_TRACE").is_ok();
    if trace {
        eprintln!(
            "do_exist[{node_idx}:{:?}] sl(ins={:?} upd={:?} del={:?}) sr(ins={:?} upd={:?} del={:?})",
            node.kind, sl.ins, sl.upd, sl.del, sr.ins, sr.upd, sr.del
        );
    }

    // D-127: a temporal EXISTS node is processed POP-time/batched (it is
    // NOT on the join per-arrival flush path — different machinery), so a
    // right that admits N held anchors would emit them insertion-order.
    // The oracle admits most-recently-blocked-FIRST (RightTuple.addBlocked
    // PREPENDS). For a pure-insert batch we replay the staged inserts in
    // ARRIVAL order (the exists analog of flush_ins_delta; model
    // tools/model_exists_flush.py, 0-div vs the gate oracle on the shuffled
    // exists×temporal population). `not` stays fenced; non-temporal exists
    // keeps the legacy batched path (its single-anchor shapes are reversal-
    // identity, so byte-identical).
    if !is_not
        && node.temporal
        && sl.upd.is_empty()
        && sl.del.is_empty()
        && sr.upd.is_empty()
        && sr.del.is_empty()
    {
        exists_flush_admit(env, node_idx, node, sl, sr, out);
        if trace {
            eprintln!("  [flush_admit] trg ins={:?} blocked={:?}", out.trg.ins, node.blocked);
        }
        return;
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
                    out.child_del(t, *o);
                }
            }
        } else {
            if let Some(i) = node.lefts.iter().position(|(x, _)| x == l) {
                node.lefts.remove(i);
            }
            if is_not {
                if let Some(c) = node.ce_child_of(l) {
                    let t = node.kill_child(c);
                    out.child_del(t, *o);
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
                            out.child_del(t, *o);
                        }
                    } else {
                        let t = node.create_ce_child(&l);
                        out.child_ins(t, *o, 1);
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
                            out.child_del(t, *o);
                        }
                    } else {
                        let t = node.create_ce_child(&l);
                        out.child_ins(t, *o, 2);
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
                        // D-134 (§3B): a temporal not does NOT re-fire when a
                        // blocker is removed (blocked ⇒ silent forever); the
                        // left returns to node.lefts but stays childless.
                        if !node.temporal {
                            let t = node.create_ce_child(&l);
                            out.child_ins(t, *o, 2);
                        }
                    } else if let Some(c) = node.ce_child_of(&l) {
                        let t = node.kill_child(c);
                        out.child_del(t, *o);
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
                        // D-134 (§3B): a temporal not does NOT re-fire when a
                        // blocker is removed (blocked ⇒ silent forever); the
                        // left returns to node.lefts but stays childless.
                        if !node.temporal {
                            let t = node.create_ce_child(&l);
                            out.child_ins(t, *o, 2);
                        }
                    } else if let Some(c) = node.ce_child_of(&l) {
                        let t = node.kill_child(c);
                        out.child_del(t, *o);
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
                    out.child_del(t, *o);
                }
            } else if child.is_none() {
                // D-134 (§3B): a temporal not defers a newly-satisfied left
                // to its window close (not_emit_or_defer); non-temporal fires.
                node.lefts.push((l.clone(), lkey.clone()));
                not_emit_or_defer(env, node_idx, node, l, *o, 2, out);
            } else {
                let c = child.unwrap();
                out.child_upd(node.children[c].tuple.clone(), *o, 2);
                node.lefts.push((l.clone(), lkey.clone()));
            }
        } else {
            if !blocked_now {
                node.lefts.push((l.clone(), lkey.clone()));
                if let Some(c) = child {
                    let t = node.kill_child(c);
                    out.child_del(t, *o);
                }
            } else if child.is_none() {
                let t = node.create_ce_child(l);
                out.child_ins(t, *o, 2);
            } else {
                let c = child.unwrap();
                out.child_upd(node.children[c].tuple.clone(), *o, 2);
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
                    out.child_ins(t, *o, 0);
                }
            }
            None => {
                node.lefts.push((l.clone(), lkey));
                if is_not {
                    // D-134 (§3B): a temporal not DEFERS to its window close
                    // instead of firing at insert; the left stays in
                    // node.lefts so a later blocker still blocks it.
                    not_emit_or_defer(env, node_idx, node, l, *o, 0, out);
                }
            }
        }
    }

    // D-134 (§3B): fire the deferrals that came due — the engine drained
    // fire_deadlines at quiescence and re-injected the due lefts here. Fire
    // only lefts still UNBLOCKED (in node.lefts) with no child yet: a left
    // that was blocked (blocker present, or blocked-then-expired) is not in
    // node.lefts ⇒ silent forever (the arc-B model rule). Uses the ph=0
    // left-insert slot so a released firing reads as a fresh child.
    if !node.pending_release.is_empty() {
        for (l, o) in std::mem::take(&mut node.pending_release) {
            let unblocked = node.lefts.iter().any(|(x, _)| *x == l);
            if unblocked && node.ce_child_of(&l).is_none() {
                let t = node.create_ce_child(&l);
                out.child_ins(t, o, 0);
            }
        }
    }

    if trace {
        eprintln!("  trg ins={:?} upd={:?} del={:?}", out.trg.ins, out.trg.upd, out.trg.del);
        eprintln!(
            "  rights={:?} lefts={:?} blocked={:?}",
            node.rights, node.lefts, node.blocked
        );
    }
}

/// D-127: PER-ARRIVAL existential admission for a temporal exists node's
/// pure-insert batch — the exists analog of `flush_ins_delta`. The exists
/// node is not on the join per-arrival flush path, so `do_existential_node`
/// sees the whole accumulated batch; we reconstruct ARRIVAL order from
/// FactIds (a left tuple "arrives" when its last-completing element does —
/// its max FactId; a right arrives at its own FactId; all ids distinct so
/// left/right keys never tie, and same-key lefts — one right completing
/// several join tuples — keep staged reverse-arrival order under the stable
/// sort) and replay each arrival like the model:
///  * a RIGHT appends to memory and blocks every matching UNBLOCKED left in
///    memory order, emitting that batch REVERSED exactly once
///    (RightTuple.addBlocked / addInsert PREPEND → most-recently-blocked
///    first);
///  * a LEFT BATCH (all staged lefts sharing a completing fact = ONE upstream
///    join emission, kept in staged order = the join's own single reversal):
///    each left finds its first blocker or parks; the admitted ones emit as a
///    batch REVERSED once (doLeftInserts addInsert PREPEND).
/// A batch's arrival instant is its completing fact (max FactId, monotonic
/// with insertion); rights arrive at their own id. All keys are distinct
/// (one completing fact per join emission, distinct from any right), so the
/// merge is a plain sort — independent of staged list order (s0_in prepends,
/// s_left appends). Emissions stage so `trg.ins` (getInsertFirst = the
/// static-salience FIFO firing order) equals the replay order. Validated
/// 0-div vs the gate oracle by `tools/model_exists_flush.py` and
/// `tools/fuzz_exists_temporal.py`.
fn exists_flush_admit<E: JoinEnv>(
    env: &E,
    node_idx: usize,
    node: &mut Node,
    sl: Staged<Tup>,
    sr: Staged<FactId>,
    out: &mut Out<'_>,
) {
    enum Ev {
        LBatch(Vec<(Tup, Origin)>),
        R(FactId, Origin),
    }
    // group staged lefts into batches by completing fact (max id), keeping
    // staged order within a batch (= the upstream join's emission order)
    let mut batches: Vec<(u32, Vec<(Tup, Origin)>)> = Vec::new();
    for (l, o, _) in sl.ins.iter() {
        let key = l.iter().map(|f| f.0).max().unwrap_or(0);
        match batches.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push((l.clone(), *o)),
            None => batches.push((key, vec![(l.clone(), *o)])),
        }
    }
    let mut evs: Vec<(u32, Ev)> = Vec::with_capacity(batches.len() + sr.ins.len());
    for (k, v) in batches {
        evs.push((k, Ev::LBatch(v)));
    }
    for (f, o, _) in sr.ins.iter() {
        evs.push((f.0, Ev::R(*f, *o)));
    }
    evs.sort_by_key(|(k, _)| *k);

    let fno = env.fire_no();
    // fire_order: child inserts in desired firing (FIFO) order
    let mut fire_order: Vec<(Tup, Origin)> = Vec::new();
    for (_, ev) in evs {
        match ev {
            Ev::R(f, o) => {
                let rkey = env.key_of_right(node_idx, f);
                node.rights.push((f, rkey.clone()));
                let mut blocked_fwd: Vec<Tup> = Vec::new();
                for l in node.scan_lefts(rkey.as_ref()) {
                    if env.allowed_ce(node_idx, &l, f) {
                        node.blocker_of.insert(l.clone(), f);
                        node.blocked.entry(f).or_default().insert(0, l.clone());
                        node.remove_left(&l);
                        blocked_fwd.push(l);
                    }
                }
                // a batch of N blocked lefts reverses exactly once (PREPEND)
                for l in blocked_fwd.into_iter().rev() {
                    let t = node.create_ce_child(&l);
                    fire_order.push((t, o));
                }
            }
            Ev::LBatch(batch) => {
                // doLeftInserts over the batch: park the unblocked, collect
                // the admitted in batch order, then emit them REVERSED once
                let mut emitted: Vec<(Tup, Origin)> = Vec::new();
                for (l, o) in &batch {
                    node.stamp_left_seq(l);
                    node.left_fire.insert(l.clone(), fno);
                    let lkey = env.key_of_left(node_idx, l);
                    match node.find_blocker_plain(env, node_idx, l, lkey.as_ref()) {
                        Some(b) => {
                            node.blocker_of.insert(l.clone(), b);
                            node.blocked.entry(b).or_default().insert(0, l.clone());
                            emitted.push((l.clone(), *o));
                        }
                        None => {
                            node.push_left(l.clone(), lkey);
                        }
                    }
                }
                for (l, o) in emitted.into_iter().rev() {
                    let t = node.create_ce_child(&l);
                    fire_order.push((t, o));
                }
            }
        }
    }
    // child_ins PREPENDS, so emit in REVERSE to make trg.ins == fire_order
    for (t, o) in fire_order.into_iter().rev() {
        out.child_ins(t, o, 0);
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
                // staged-deleted rights are ineligible — UNLESS the same
                // fact is ALSO staged-inserted (alpha out-and-back within
                // one batch, hw_ex1a/fz_42_3924): Drools' re-added right
                // is a fresh unstaged RightTuple and re-blocks; only a
                // del-then-ins sequence leaves both staged (ins-then-del
                // folds), so both-present uniquely marks re-entry.
                let staged_del = sr.del.iter().any(|(x, _, _)| x == f);
                let re_added = sr.ins.iter().any(|(x, _, _)| x == f);
                (!staged_del || re_added) && env.allowed_ce(node_idx, l, *f)
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
