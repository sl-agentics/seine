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

/// One rule's ordered match list (PHREAK-observable order, D-013/D-014):
/// initial batch in staged-reversal order; still-matching tuples keep their
/// position across updates; newly-matching tuples append per staged batch;
/// removed tuples drop out. `fired` is per-entry refraction.
struct MatchEntry {
    tuple: Vec<FactId>,
    fired: bool,
}

/// A working-memory delta staged for a rule, merged lazily the next time the
/// rule is considered by the agenda (D-014). The ALPHA network is evaluated
/// EAGERLY at event time (fz_7_58: a fact that starts alpha-passing later
/// keeps its later queue position) — so events arrive pre-resolved per rule.
#[derive(Clone)]
enum StagedEv {
    /// Fact became alpha-active at this pattern position (insert, or update
    /// transitioning the alpha test to pass).
    Act { fact: FactId, pos: usize },
    /// Property-reactive update touching already-active occurrences.
    Hot {
        fact: FactId,
        positions: Vec<usize>,
        src_ri: usize,
        src_tuple: Vec<FactId>,
        no_loop: bool,
    },
    /// Deletion / deactivation: pruning happens from current values at merge;
    /// this only forces the merge to run.
    Del,
}

/// Persistent join-network state for one rule (D-014). List ORDERS are
/// semantically observable and pinned by probes:
/// - `alpha[p]`: alpha-passing facts at pattern position p; new batches
///   BLOCK-PREPEND (FIFO within the batch) — u09 pinned iteration
///   [new..., old...].
/// - `prefixes[l]`: partial tuples of length l+1 (l = 0..k-2); batches also
///   BLOCK-PREPEND, in processing order within the block (fz_7_159 pinned
///   batch-2 prefixes iterating before batch-1).
/// The full-length tuples live in `matches` (with refraction flags); kept
/// entries hold position, new emissions append (terminal never reorders).
struct RuleNet {
    alpha: Vec<Vec<FactId>>,
    prefixes: Vec<Vec<Vec<FactId>>>,
    matches: Vec<MatchEntry>,
    /// Eager mirror of alpha membership (alpha lists update only at merge;
    /// this set updates at event time so transitions are detected eagerly).
    active: Vec<HashSet<FactId>>,
}

pub struct Engine {
    store: FactStore,
    rules: Vec<CompiledRule>,
    /// Rule indices sorted by (salience desc, declaration order).
    rule_order: Vec<usize>,
    /// Per-rule incremental join networks; seeded lazily at first fire_all.
    nets: Vec<RuleNet>,
    /// Per-rule staged deltas awaiting merge.
    staged: Vec<Vec<StagedEv>>,
    lists_built: bool,
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
            nets: Vec::new(),
            staged: Vec::new(),
            lists_built: false,
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
        if !self.lists_built {
            self.nets = self
                .rules
                .iter()
                .map(|r| RuleNet {
                    alpha: vec![Vec::new(); r.patterns.len()],
                    prefixes: vec![Vec::new(); r.patterns.len().saturating_sub(1)],
                    matches: Vec::new(),
                    active: vec![HashSet::new(); r.patterns.len()],
                })
                .collect();
            self.staged = vec![Vec::new(); self.rules.len()];
            self.lists_built = true;
            // Initial facts enter as one staged batch in handle order; the
            // lazy per-rule merge then reproduces PHREAK's batch evaluation.
            let initial: Vec<FactId> = self.store.live_facts().collect();
            for f in initial {
                self.on_insert(f);
            }
        }
        let mut firings = Vec::new();
        while let Some((ri, idx)) = self.next_activation() {
            if firings.len() >= limit {
                return Err(EngineError(format!(
                    "fire limit {limit} reached (non-terminating?)"
                )));
            }
            self.nets[ri].matches[idx].fired = true;
            let tuple = self.nets[ri].matches[idx].tuple.clone();
            self.execute_rhs(ri, &tuple)?;
            // Post-RHS rendering (D-013 / j03): values reflect the mutations
            // this firing just performed.
            let matches: Vec<FactView> = tuple.iter().map(|&f| self.store.render(f)).collect();
            firings.push(Firing { rule: self.rules[ri].def.name.clone(), matches });
        }
        Ok(firings)
    }

    fn next_activation(&mut self) -> Option<(usize, usize)> {
        for i in 0..self.rule_order.len() {
            let ri = self.rule_order[i];
            self.merge_staged(ri);
            if let Some(idx) = self.nets[ri].matches.iter().position(|e| !e.fired) {
                return Some((ri, idx));
            }
        }
        None
    }

    /// Eager alpha propagation for an inserted fact (D-014/fz_7_58): resolve
    /// pass/fail per rule/position NOW; beta merging stays lazy.
    fn on_insert(&mut self, f: FactId) {
        for ri in 0..self.rules.len() {
            for pos in 0..self.rules[ri].patterns.len() {
                if self.alpha_passes(ri, pos, f) {
                    self.nets[ri].active[pos].insert(f);
                    self.staged[ri].push(StagedEv::Act { fact: f, pos });
                }
            }
        }
    }

    /// Eager alpha transition handling + property-reactive staging for an
    /// update event.
    fn on_update(
        &mut self,
        f: FactId,
        mask: u64,
        src_ri: usize,
        src_tuple: &[FactId],
    ) {
        let no_loop = self.rules[src_ri].def.no_loop;
        let ftype = self.store.fact_type(f);
        for ri in 0..self.rules.len() {
            let mut hot_positions = Vec::new();
            for pos in 0..self.rules[ri].patterns.len() {
                let pat = &self.rules[ri].patterns[pos];
                if pat.type_id != ftype {
                    continue;
                }
                let was = self.nets[ri].active[pos].contains(&f);
                let now = self.alpha_passes(ri, pos, f);
                match (was, now) {
                    (false, true) => {
                        self.nets[ri].active[pos].insert(f);
                        self.staged[ri].push(StagedEv::Act { fact: f, pos });
                    }
                    (true, false) => {
                        self.nets[ri].active[pos].remove(&f);
                        self.staged[ri].push(StagedEv::Del);
                    }
                    (true, true) => {
                        if pat.listen_mask & mask != 0 {
                            hot_positions.push(pos);
                        }
                    }
                    (false, false) => {}
                }
            }
            if !hot_positions.is_empty() {
                self.staged[ri].push(StagedEv::Hot {
                    fact: f,
                    positions: hot_positions,
                    src_ri,
                    src_tuple: src_tuple.to_vec(),
                    no_loop,
                });
            }
        }
    }

    fn on_delete(&mut self, f: FactId) {
        for ri in 0..self.rules.len() {
            let mut touched = false;
            for pos in 0..self.rules[ri].patterns.len() {
                if self.nets[ri].active[pos].remove(&f) {
                    touched = true;
                }
            }
            if touched {
                self.staged[ri].push(StagedEv::Del);
            }
        }
    }

    /// Merge this rule's staged delta batch into its join network (D-014).
    /// Mirrors PHREAK's per-node batch processing:
    ///  1. prune everything invalidated by the batch (values are current);
    ///  2. block-prepend newly alpha-passing facts into alpha memories;
    ///  3. cascade emissions join by join: update-driven pairs first, then
    ///     staged-left inserts against the full right memory, then staged
    ///     rights against pre-batch lefts; emissions REVERSE when propagated
    ///     to the next join, and append unreversed at the terminal;
    ///  4. kept full matches hold position; hot updates clear their fired
    ///     flag (no-loop's own tuple excepted).
    fn merge_staged(&mut self, ri: usize) {
        if self.staged[ri].is_empty() {
            return;
        }
        let k = self.rules[ri].patterns.len();

        // ---- 0. segment linking (D-014/fz_7_145): while any pattern
        // position has no alpha-active facts, the rule is UNLINKED — staged
        // events accumulate into one batch that is only processed once every
        // position has data. Pruning (cancellations) still happens.
        if (0..k).any(|p| self.nets[ri].active[p].is_empty()) {
            for p in 0..k {
                let active = self.nets[ri].active[p].clone();
                let kept: Vec<FactId> = self.nets[ri].alpha[p]
                    .iter()
                    .copied()
                    .filter(|f| active.contains(f) && self.alpha_passes(ri, p, *f))
                    .collect();
                self.nets[ri].alpha[p] = kept;
            }
            for l in 0..k.saturating_sub(1) {
                let kept: Vec<Vec<FactId>> = self.nets[ri].prefixes[l]
                    .iter()
                    .filter(|t| self.prefix_valid(ri, t))
                    .cloned()
                    .collect();
                self.nets[ri].prefixes[l] = kept;
            }
            // An unlinked rule can have no valid full match (some position
            // is empty), so validity pruning clears the agenda entries.
            let old_matches = std::mem::take(&mut self.nets[ri].matches);
            self.nets[ri].matches = old_matches
                .into_iter()
                .filter(|e| self.prefix_valid(ri, &e.tuple))
                .collect();
            return;
        }

        let events = std::mem::take(&mut self.staged[ri]);

        // ---- 1. staged activations per position (event order, last
        // occurrence wins), hot updates (FIFO) ----
        let mut staged_at: Vec<Vec<FactId>> = vec![Vec::new(); k];
        let mut hot_events: Vec<(FactId, Vec<usize>)> = Vec::new();
        for ev in &events {
            match ev {
                StagedEv::Act { fact, pos } => {
                    // Re-staging moves the fact to the end of the batch.
                    staged_at[*pos].retain(|x| x != fact);
                    // Only if the activation still stands right now.
                    if self.nets[ri].active[*pos].contains(fact)
                        && self.alpha_passes(ri, *pos, *fact)
                    {
                        staged_at[*pos].push(*fact);
                    }
                }
                StagedEv::Hot { fact, positions, .. } => {
                    hot_events.push((*fact, positions.clone()));
                }
                StagedEv::Del => {}
            }
        }

        // ---- 2. prune (deletes, alpha failures, constraint failures, and
        // deactivate->reactivate cycles, which lose their list position) ----
        for p in 0..k {
            let staged = staged_at[p].clone();
            let active = self.nets[ri].active[p].clone();
            let kept: Vec<FactId> = self.nets[ri].alpha[p]
                .iter()
                .copied()
                .filter(|f| {
                    active.contains(f) && self.alpha_passes(ri, p, *f) && !staged.contains(f)
                })
                .collect();
            self.nets[ri].alpha[p] = kept;
        }
        let restaged = |t: &[FactId], staged_at: &[Vec<FactId>]| {
            t.iter().enumerate().any(|(p, f)| staged_at[p].contains(f))
        };
        for l in 0..k.saturating_sub(1) {
            let kept: Vec<Vec<FactId>> = self.nets[ri].prefixes[l]
                .iter()
                .filter(|t| self.prefix_valid(ri, t) && !restaged(t, &staged_at))
                .cloned()
                .collect();
            self.nets[ri].prefixes[l] = kept;
        }
        let old_matches = std::mem::take(&mut self.nets[ri].matches);
        let mut kept_matches: Vec<MatchEntry> = Vec::new();
        for mut e in old_matches {
            if !self.prefix_valid(ri, &e.tuple) || restaged(&e.tuple, &staged_at) {
                continue;
            }
            if self.tuple_hot(ri, &e.tuple, &events) {
                e.fired = false; // re-created activation, refires (D-013)
            }
            kept_matches.push(e);
        }
        self.nets[ri].matches = kept_matches;

        if staged_at.iter().all(|s| s.is_empty()) && hot_events.is_empty() {
            return;
        }

        // ---- 3. commit alpha memories: block-prepend (u09/fz_7_159: every
        // memory iterates [new batch, processing order..., older batches]) ----
        for p in 0..k {
            let mut merged = staged_at[p].clone();
            merged.extend(self.nets[ri].alpha[p].iter().copied());
            self.nets[ri].alpha[p] = merged;
        }

        // ---- 4. emission cascade ----
        // Level l holds prefixes of length l+1. L = staged left tuples.
        let mut staged_lefts: Vec<Vec<FactId>> =
            staged_at[0].iter().map(|&f| vec![f]).collect();
        if k == 1 {
            for t in staged_lefts {
                self.nets[ri].matches.push(MatchEntry { tuple: t, fired: false });
            }
            return;
        }
        for join_pos in 1..k {
            let terminal = join_pos == k - 1;
            // Existing tuples of length join_pos+1 (kept, pre-append).
            let existing: HashSet<Vec<FactId>> = if terminal {
                self.nets[ri].matches.iter().map(|e| e.tuple.clone()).collect()
            } else {
                self.nets[ri].prefixes[join_pos].iter().cloned().collect()
            };
            // Old lefts (kept, pre-batch) at this join.
            let old_lefts: Vec<Vec<FactId>> = if join_pos == 1 {
                self.nets[ri].alpha[0]
                    .iter()
                    .filter(|f| !staged_at[0].contains(f))
                    .map(|&f| vec![f])
                    .collect()
            } else {
                self.nets[ri].prefixes[join_pos - 1].clone()
            };
            let rights: Vec<FactId> = self.nets[ri].alpha[join_pos].clone();
            let mut emitted: HashSet<Vec<FactId>> = HashSet::new();
            let mut emit: Vec<Vec<FactId>> = Vec::new();

            let push =
                |t: Vec<FactId>, emit: &mut Vec<Vec<FactId>>, emitted: &mut HashSet<Vec<FactId>>| {
                    if !existing.contains(&t) && emitted.insert(t.clone()) {
                        emit.push(t);
                    }
                };

            // 4a. update-driven pairs (u07: appended in update-event order).
            for (f, hot_pos) in &hot_events {
                // left-update: existing prefixes holding f at a hot position.
                for l in &old_lefts {
                    if hot_pos.iter().any(|&hp| hp < join_pos && l.get(hp) == Some(f)) {
                        for &r in &rights {
                            let mut t = l.clone();
                            t.push(r);
                            if self.prefix_valid(ri, &t) {
                                push(t, &mut emit, &mut emitted);
                            }
                        }
                    }
                }
                // right-update: f itself at this join's right side.
                if hot_pos.contains(&join_pos) {
                    for l in &old_lefts {
                        let mut t = l.clone();
                        t.push(*f);
                        if self.prefix_valid(ri, &t) {
                            push(t, &mut emit, &mut emitted);
                        }
                    }
                }
            }
            // 4b. staged left tuples against the FULL right memory.
            for l in &staged_lefts {
                for &r in &rights {
                    let mut t = l.clone();
                    t.push(r);
                    if self.prefix_valid(ri, &t) {
                        push(t, &mut emit, &mut emitted);
                    }
                }
            }
            // 4c. staged rights against pre-batch lefts.
            for &f in &staged_at[join_pos] {
                for l in &old_lefts {
                    let mut t = l.clone();
                    t.push(f);
                    if self.prefix_valid(ri, &t) {
                        push(t, &mut emit, &mut emitted);
                    }
                }
            }

            if terminal {
                for t in emit {
                    self.nets[ri].matches.push(MatchEntry { tuple: t, fired: false });
                }
                break;
            }
            // Propagate: reverse into the next join's staged-left list, and
            // BLOCK-PREPEND (processing order within the block) into this
            // level's prefix memory (fz_7_159: batch-2 prefixes iterate
            // before batch-1 for later right-staged joins).
            emit.reverse();
            let mut merged = emit.clone();
            merged.extend(self.nets[ri].prefixes[join_pos].iter().cloned());
            self.nets[ri].prefixes[join_pos] = merged;
            staged_lefts = emit;
        }
    }

    /// Alpha test: alive + prefix-independent constraints (literals and
    /// same-pattern binding references).
    fn alpha_passes(&self, ri: usize, pos: usize, f: FactId) -> bool {
        let pat = &self.rules[ri].patterns[pos];
        if !self.store.is_alive(f) || self.store.fact_type(f) != pat.type_id {
            return false;
        }
        pat.cmps.iter().all(|c| match &c.rhs {
            Src::Lit(v) => eval_cmp(&self.store.value(f, c.field_idx), c.op, v),
            Src::Field(pi, fi) if *pi == pos => {
                let rhs = self.store.value(f, *fi);
                eval_cmp(&self.store.value(f, c.field_idx), c.op, &rhs)
            }
            Src::Field(..) => true, // join constraint, checked with prefix
        })
    }

    /// Full validity of a (partial) tuple: every position alive, alpha-
    /// passing, and every join constraint up to the tuple's length holds.
    fn prefix_valid(&self, ri: usize, tuple: &[FactId]) -> bool {
        let rule = &self.rules[ri];
        for (pos, &f) in tuple.iter().enumerate() {
            if !self.alpha_passes(ri, pos, f) {
                return false;
            }
            let pat = &rule.patterns[pos];
            for c in &pat.cmps {
                if let Src::Field(pi, fi) = &c.rhs {
                    if *pi != pos {
                        let rhs = self.store.value(tuple[*pi], *fi);
                        if !eval_cmp(&self.store.value(f, c.field_idx), c.op, &rhs) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Does any staged hot update touch `tuple` at a listening position?
    fn tuple_hot(&self, ri: usize, tuple: &[FactId], events: &[StagedEv]) -> bool {
        events.iter().any(|ev| {
            let StagedEv::Hot { fact, positions, src_ri, src_tuple, no_loop } = ev else {
                return false;
            };
            if *no_loop && *src_ri == ri && tuple == src_tuple.as_slice() {
                return false; // no-loop: own tuple's refraction survives own update
            }
            positions.iter().any(|&p| tuple[p] == *fact)
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
                    let fid = self.store.insert(tid, values).map_err(EngineError)?;
                    self.on_insert(fid);
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
                    self.on_update(f, mask, ri, tuple);
                }
                CompiledAction::Delete { pos } => {
                    self.store.kill(tuple[*pos]);
                    self.on_delete(tuple[*pos]);
                }
            }
        }
        Ok(())
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
