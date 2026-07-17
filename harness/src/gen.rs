//! Property-based scenario generator for the Phase 1-3 grammar.
//!
//! Every generated program is IN-SUBSET by construction and guaranteed to
//! terminate (D-010, D-013, D-032):
//! - inserts: a rule may only insert types with index strictly greater than
//!   every pattern's type index INCLUDING not/exists CE types (chains
//!   strictly climb the type order, so no consequence chain can ever
//!   re-insert a blocker/support type at or below its own LHS — not-driven
//!   refires stay bounded by the finite event pool of lower types);
//! - updates: guard-monotone — the updated pattern requires some bool field
//!   `g == false` and the RHS sets it true before update(); bool setters
//!   ONLY ever write true, so every bool field is monotone and each update
//!   rule fires at most once per fact per guarded position;
//! - bare update() (all-fields mask, non-terminating per j21) is never
//!   generated;
//! - not/exists patterns carry no bindings (D-031); update/delete targets
//!   and RHS getters reference positive patterns only.
//!
//! Deterministic: SplitMix64 from an explicit seed; case k of seed s is
//! always identical.

use serde_json::{json, Value as J};

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, pct: usize) -> bool {
        self.below(100) < pct
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Ft {
    I64,
    F64,
    Str,
    Bool,
}

impl Ft {
    fn json_name(self) -> &'static str {
        match self {
            Ft::I64 => "i64",
            Ft::F64 => "f64",
            Ft::Str => "String",
            Ft::Bool => "bool",
        }
    }

    fn numeric(self) -> bool {
        matches!(self, Ft::I64 | Ft::F64)
    }

    fn join_compatible(self, other: Ft) -> bool {
        (self.numeric() && other.numeric()) || self == other
    }
}

struct TypeDef {
    name: String,
    fields: Vec<(String, Ft)>,
}

const STR_POOL: &[&str] = &["", "a", "b", "ab", "zz", "alpha", "beta"];
const OPS_ORD: &[&str] = &["==", "!=", "<", "<=", ">", ">="];
const OPS_EQ: &[&str] = &["==", "!="];

fn gen_i64(rng: &mut Rng) -> i64 {
    match rng.below(10) {
        0 => -1_000_000_007,
        1 => 100,
        _ => rng.below(18) as i64 - 5,
    }
}

fn gen_f64(rng: &mut Rng) -> f64 {
    // Multiples of 0.5 in [-3, 6]; integral values on purpose to stress the
    // i64/f64 boundary. No NaN/inf (not expressible in JSON).
    (rng.below(19) as f64 - 6.0) * 0.5
}

fn lit_json(rng: &mut Rng, ft: Ft) -> J {
    match ft {
        Ft::I64 => json!(gen_i64(rng)),
        Ft::F64 => json!(gen_f64(rng)),
        Ft::Str => json!(*rng.pick(STR_POOL)),
        Ft::Bool => json!(rng.chance(50)),
    }
}

/// Literal for DRL text. Bool literals in SETTER position must use
/// `only_true_bools` to preserve guard monotonicity.
fn lit_drl(rng: &mut Rng, ft: Ft, only_true_bools: bool) -> String {
    match ft {
        Ft::I64 => format!("{}", gen_i64(rng)),
        Ft::F64 => {
            let v = gen_f64(rng);
            if v == v.trunc() {
                format!("{v:.1}")
            } else {
                format!("{v}")
            }
        }
        Ft::Str => format!("{:?}", rng.pick(STR_POOL)),
        Ft::Bool => {
            if only_true_bools {
                "true".into()
            } else {
                format!("{}", rng.chance(50))
            }
        }
    }
}

/// One alpha (literal-only) constraint on `fname` of type `ft`, over the
/// Phase 1-3 operator grammar: the six cmpops, and per D-030 `matches` /
/// `contains` (String fields), `in` / `not in` (all field types;
/// cross-type numeric items exercise the promote-only pin of op_i3).
/// Inline boolean group constraint (D-073): `a == 1 || b > 2`,
/// `!(x > 5)`, abbreviated `a > 5 && < 10`. Leaves draw over the
/// pattern's own fields; composites never join eq-hash groups
/// oracle-side (ib21/ib22), so `==` leaves are safe anywhere.
fn gen_group_constraint(rng: &mut Rng, fields: &[(String, Ft)]) -> String {
    let leaf = |rng: &mut Rng| {
        let (f, ft) = fields[rng.below(fields.len())].clone();
        gen_alpha_constraint(rng, &f, ft)
    };
    match rng.below(10) {
        // negated single test
        0..=2 => format!("!({})", leaf(rng)),
        // negated disjunction
        3 => format!("!({} || {})", leaf(rng), leaf(rng)),
        // abbreviated relational range on one numeric field
        4..=5 => {
            let nums: Vec<&(String, Ft)> =
                fields.iter().filter(|(_, ft)| matches!(ft, Ft::I64 | Ft::F64)).collect();
            if nums.is_empty() {
                return format!("{} || {}", leaf(rng), leaf(rng));
            }
            let (f, ft) = (*rng.pick(&nums)).clone();
            let a = lit_drl(rng, ft, false);
            let b = lit_drl(rng, ft, false);
            let (o1, o2) = if rng.chance(50) { (">", "<") } else { (">=", "<=") };
            if rng.chance(50) {
                format!("{f} {o1} {a} && < {b}")
            } else {
                format!("{f} {o1} {a} && {f} {o2} {b}")
            }
        }
        // 2-3 way disjunction, possibly with a nested !()
        _ => {
            let mut parts = vec![leaf(rng), leaf(rng)];
            if rng.chance(25) {
                parts.push(format!("!({})", leaf(rng)));
            }
            parts.join(" || ")
        }
    }
}

fn gen_alpha_constraint(rng: &mut Rng, fname: &str, ft: Ft) -> String {
    match ft {
        Ft::Str => match rng.below(10) {
            0..=3 => {
                let op = *rng.pick(OPS_ORD);
                let lit = lit_drl(rng, Ft::Str, false);
                format!("{fname} {op} {lit}")
            }
            4..=5 => format!("{fname} matches \"{}\"", gen_regex(rng)),
            6..=7 => format!("{fname} contains \"{}\"", gen_needle(rng)),
            _ => format!("{fname}{}", gen_in_list(rng, Ft::Str)),
        },
        Ft::I64 | Ft::F64 => {
            if rng.chance(25) {
                format!("{fname}{}", gen_in_list(rng, ft))
            } else {
                let op = *rng.pick(OPS_ORD);
                let lit_ft = match ft {
                    Ft::I64 if rng.chance(20) => Ft::F64,
                    Ft::F64 if rng.chance(20) => Ft::I64,
                    other => other,
                };
                let lit = lit_drl(rng, lit_ft, false);
                format!("{fname} {op} {lit}")
            }
        }
        Ft::Bool => {
            if rng.chance(10) {
                format!("{fname}{}", gen_in_list(rng, Ft::Bool))
            } else {
                let op = *rng.pick(OPS_EQ);
                let lit = lit_drl(rng, Ft::Bool, false);
                format!("{fname} {op} {lit}")
            }
        }
    }
}

/// ` in (a, b)` / ` not in (a, b)` suffix. Numeric lists mix i64/f64
/// literals (promote-only semantics pinned by op_i3/op_i4).
fn gen_in_list(rng: &mut Rng, ft: Ft) -> String {
    let neg = if rng.chance(35) { " not" } else { "" };
    let n = 1 + rng.below(3);
    let items: Vec<String> = (0..n)
        .map(|_| {
            let lit_ft = match ft {
                Ft::I64 if rng.chance(20) => Ft::F64,
                Ft::F64 if rng.chance(20) => Ft::I64,
                other => other,
            };
            lit_drl(rng, lit_ft, false)
        })
        .collect();
    format!("{neg} in ({})", items.join(", "))
}

fn gen_needle(rng: &mut Rng) -> String {
    (*rng.pick(&["a", "b", "ab", "z", "zz", "alpha", "", "l", "ph", "et"])).to_string()
}

/// Tame regex over the corpus alphabet (subset of both java.util.regex and
/// engine/src/rx.rs): literals, `.`, simple classes, groups, `|`, `* + ?`.
fn gen_regex(rng: &mut Rng) -> String {
    if rng.chance(4) {
        return String::new(); // `matches ""` (op_m5)
    }
    let n = 1 + rng.below(3);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(&gen_regex_piece(rng));
    }
    s
}

fn gen_regex_piece(rng: &mut Rng) -> String {
    let atom = match rng.below(8) {
        0 => ".".to_string(),
        1 => (*rng.pick(&["[ab]", "[a-z]", "[^b]", "[al]", "[b-t]"])).to_string(),
        2 => format!(
            "({}|{})",
            rng.pick(&["a", "b", "zz", "al", "et", ""]),
            rng.pick(&["a", "b", "z", "ph", "be"])
        ),
        _ => (*rng.pick(&["a", "b", "z", "l", "e", "t", "al", "zz", "ab", "ha", "p"])).to_string(),
    };
    match rng.below(6) {
        0 => format!("{atom}*"),
        1 => format!("{atom}+"),
        2 => format!("{atom}?"),
        _ => atom,
    }
}

fn accessor(name: &str, ft: Ft, prefix: &str) -> String {
    let mut cs = name.chars();
    let head = cs.next().unwrap().to_ascii_uppercase();
    let rest: String = cs.collect();
    let px = if prefix == "get" && ft == Ft::Bool { "is" } else { prefix };
    format!("{px}{head}{rest}")
}

/// One LHS pattern being generated.
#[derive(Clone)]
struct GenPattern {
    ti: usize,
    /// 0 = positive, 1 = not, 2 = exists, 3 = accumulate, 4 = collect.
    /// CE/accumulate patterns never carry fact vars; an accumulate's
    /// only outward binding is its RESULT (D-031/D-038).
    ce: u8,
    fact_var: Option<String>,
    constraints: Vec<String>,
    /// (var name, field idx, field type) — usable by later patterns/RHS.
    /// For accumulate patterns this holds the RESULT binding.
    bindings: Vec<(String, usize, Ft)>,
    /// Accumulate rendering: (function name, arg render, result var).
    acc: Option<(String, String, String)>,
    /// Group CE inners (P1c/D-089), ce 5 = not-group / 6 = exists-group:
    /// (type idx, constraints, render-as-bare-not). Inner bindings stay
    /// group-scoped (never registered outward). `group_or` renders the
    /// two inners as an `or` (the parse-time rewrite path: De Morgan /
    /// double negation).
    group: Vec<(usize, Vec<String>, bool)>,
    group_or: bool,
}

pub fn gen_scenario(seed: u64, case: u64) -> (String, J) {
    let mut rng = Rng(seed ^ case.wrapping_mul(0xA24BAED4963EE407));
    for _ in 0..4 {
        rng.next();
    }
    let name = format!("fz_{seed}_{case}");

    // Types: ensure a decent supply of bool fields so update rules happen.
    let ntypes = 2 + rng.below(3); // 2..4
    let mut types = Vec::new();
    for ti in 0..ntypes {
        let nfields = 1 + rng.below(3); // 1..3
        let mut fields: Vec<(String, Ft)> = (0..nfields)
            .map(|fi| {
                let ft = *rng.pick(&[Ft::I64, Ft::I64, Ft::F64, Ft::Str, Ft::Bool]);
                (format!("f{fi}"), ft)
            })
            .collect();
        if rng.chance(60) {
            let n = fields.len();
            fields.push((format!("f{n}"), Ft::Bool));
        }
        types.push(TypeDef { name: format!("T{ti}"), fields });
    }

    let nrules = 1 + rng.below(6); // 1..6
    // No structural walls remain: mutation, 3-pattern rules, CEs and
    // shared prefixes all mix freely (D-028 lifted D-017; D-036 lifted
    // D-035 — node-sharing identity incl. the bound-field set is modeled
    // by the engine's static sink-order flips).
    let allow_mutation = rng.chance(60);
    // D-093 wall LIFTED (D-163): the stale-extremum defect (D-092) is
    // fixed in the ORACLE by the vendored upstream repair (#6796,
    // oracle/src/.../PhreakAccumulateNode.java — 9.44.0.Final+p1), so
    // min/max accumulates mix freely with mutation again and external
    // updates stop rerouting.
    // D-076 TMS scenarios (~30%): the LAST type is the LOGICAL type —
    // insertLogical targets it exclusively, and (mutation wall) no rule
    // setter/update and no external update ever touches it. Deletes stay
    // free (the delete-quirk model is part of the certified surface).
    let logical_ti = ntypes - 1;
    let tms = ntypes >= 2 && rng.chance(30);
    let max_extra_pat = 3;
    let mut drl = String::new();
    // Types any rule may DELETE: external actions must not target them
    // (a dead target errors in both engines, flagged by the judge).
    let mut rule_deleted_types: std::collections::HashSet<usize> = std::collections::HashSet::new();
    // Finished rules' patterns, kept so later rules can REUSE a prefix:
    // random draws almost never collide on the full sharing identity
    // (types + constraints + bound-field sets), so without deliberate
    // reuse the D-033/D-036 sharing surface would go unfuzzed.
    let mut prev_rules: Vec<Vec<GenPattern>> = Vec::new();
    // D-106 pre-pass: agenda-group attrs decided up front so setFocus
    // draws target only DECLARED groups (undeclared = Drools NPE, walled)
    let rule_groups: Vec<Option<&str>> = (0..nrules)
        .map(|_| {
            if rng.chance(12) {
                Some(*rng.pick(&["ga", "gb"]))
            } else {
                None
            }
        })
        .collect();
    let declared_groups: Vec<&str> = {
        let mut v: Vec<&str> = rule_groups.iter().flatten().copied().collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    for ri in 0..nrules {
        let mut pats: Vec<GenPattern> = Vec::new();
        // Prefix reuse (true node sharing): copy 2..all leading patterns
        // from an earlier rule, renaming its binding vars to this rule.
        // Accumulate-bearing prefixes are excluded from reuse: their
        // sharing identity (arg names, contexts) is unprobed (D-038).
        let reusable: Vec<usize> = (0..prev_rules.len())
            .filter(|&i| {
                prev_rules[i].len() >= 2 && prev_rules[i].iter().all(|p| p.ce < 3)
            })
            .collect();
        let mut copied = 0usize;
        if !reusable.is_empty() && rng.chance(15) {
            let src = *rng.pick(&reusable);
            let take = if rng.chance(30) {
                prev_rules[src].len() // identical-LHS twin candidate
            } else {
                2
            };
            let from = format!("$b{src}_");
            let to = format!("$b{ri}_");
            for p in &prev_rules[src][..take] {
                pats.push(GenPattern {
                    ti: p.ti,
                    ce: p.ce,
                    fact_var: None,
                    constraints: p
                        .constraints
                        .iter()
                        .map(|c| c.replace(&from, &to))
                        .collect(),
                    bindings: p
                        .bindings
                        .iter()
                        .map(|(v, fi, ft)| (v.replace(&from, &to), *fi, *ft))
                        .collect(),
                    acc: None,
                    group: Vec::new(),
                    group_or: false,
                });
            }
            copied = take;
        }
        let npat = if copied > 0 {
            copied + if rng.chance(50) { rng.below(2) } else { 0 }
        } else {
            1 + if rng.chance(45) { rng.below(max_extra_pat) } else { 0 }
        };
        for pi in copied..npat {
            // CE probability: rare in first position (InitialFact path),
            // more common later (D-031/D-032).
            let ce = if pi == 0 {
                if rng.chance(7) {
                    1 + rng.below(2) as u8
                } else {
                    0
                }
            } else if rng.chance(22) {
                1 + rng.below(2) as u8
            } else {
                0
            };
            // ~10% of fresh patterns become accumulate (3) or collect
            // (4) — decided HERE so the join-constraint gate can exclude
            // collect sources (subnetwork fence, D-041); the function
            // itself is picked once constraints are generated.
            let ti = rng.below(ntypes);
            let ce = if rng.chance(10) {
                let numeric = types[ti]
                    .fields
                    .iter()
                    .any(|(_, ft)| matches!(ft, Ft::I64 | Ft::F64));
                let c = rng.below(100);
                if c < 15 || (!numeric && c < 55) { 4 } else { 3 }
            } else {
                ce
            };
            // P1c/D-089: ~35% of bare-CE draws become GROUP CEs
            // (5 = not-group, 6 = exists-group; inners drawn below).
            let ce = if (ce == 1 || ce == 2) && rng.chance(35) { ce + 4 } else { ce };
            pats.push(GenPattern {
                ti,
                ce,
                fact_var: None,
                constraints: Vec::new(),
                bindings: Vec::new(),
                acc: None,
                group: Vec::new(),
                group_or: false,
            });
        }

        // Rule kind: update / delete / plain. Mutation targets must be
        // POSITIVE patterns (CE patterns cannot bind), and — because the
        // guard constraint would change the pattern's identity — must
        // not sit inside a reused prefix.
        let update_pos = {
            let with_bool: Vec<usize> = pats
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    *i >= copied
                        && p.ce == 0
                        && !(tms && p.ti == logical_ti)
                        && types[p.ti].fields.iter().any(|(_, ft)| *ft == Ft::Bool)
                })
                .map(|(i, _)| i)
                .collect();
            if allow_mutation && !with_bool.is_empty() && rng.chance(30) {
                Some(*rng.pick(&with_bool))
            } else {
                None
            }
        };
        let delete_pos = {
            let positives: Vec<usize> = pats
                .iter()
                .enumerate()
                .filter(|(_, p)| p.ce == 0 && !(tms && p.ti == logical_ti))
                .map(|(i, _)| i)
                .collect();
            if update_pos.is_none() && !positives.is_empty() && rng.chance(20) {
                Some(*rng.pick(&positives))
            } else {
                None
            }
        };

        // Constraints, bindings, join tests (fresh patterns only — a
        // reused prefix keeps its constraint list verbatim).
        for pi in copied..npat {
            let ncmp = if pats[pi].ce >= 5 { 0 } else { rng.below(3) };
            for _ in 0..ncmp {
                let (fname, ft) = {
                    let fs = &types[pats[pi].ti].fields;
                    fs[rng.below(fs.len())].clone()
                };
                let c = gen_alpha_constraint(&mut rng, &fname, ft);
                pats[pi].constraints.push(c);
            }
            // D-073 inline boolean groups (~12% of patterns; collect
            // sources excluded — their gate/identity story is scoped to
            // plain constraints).
            if pats[pi].ce != 4 && pats[pi].ce < 5 && rng.chance(12) {
                let fs = types[pats[pi].ti].fields.clone();
                pats[pi].constraints.push(gen_group_constraint(&mut rng, &fs));
            }
            // Join constraint against an earlier binding. Collect
            // patterns are excluded: a var-referencing collect source
            // is an RIA subnetwork, out of subset (D-041).
            if pi > 0 && pats[pi].ce != 4 && pats[pi].ce < 5 && rng.chance(55) {
                let earlier: Vec<(String, Ft)> = pats[..pi]
                    .iter()
                    .flat_map(|p| p.bindings.iter().map(|(v, _, ft)| (v.clone(), *ft)))
                    .collect();
                if !earlier.is_empty() {
                    let fs = &types[pats[pi].ti].fields;
                    let fi = rng.below(fs.len());
                    let (fname, ft) = fs[fi].clone();
                    let compat: Vec<&(String, Ft)> = earlier
                        .iter()
                        .filter(|(_, bft)| ft.join_compatible(*bft))
                        .collect();
                    if !compat.is_empty() {
                        let (var, bft) = (*rng.pick(&compat)).clone();
                        let op = if ft == Ft::Bool || bft == Ft::Bool {
                            *rng.pick(OPS_EQ)
                        } else {
                            *rng.pick(OPS_ORD)
                        };
                        pats[pi].constraints.push(format!("{fname} {op} {var}"));
                    }
                }
            }
            // D-291 axis: LHS arithmetic, AGREE SUBSET only — single-op
            // expressions (never the bare a+b*c defect surface, D-281);
            // int '/' and '%' take nonzero int literal divisors and int
            // comparands; f64 shapes are free. Residency (D-290): gen
            // scenarios cap at 6 facts + small epochs, far under the
            // ~20-eval jit threshold — every emitted constraint lives
            // its whole life in mode 1.
            if pats[pi].ce == 0 && rng.chance(6) {
                let numeric: Vec<(String, Ft)> = types[pats[pi].ti]
                    .fields
                    .iter()
                    .filter(|(_, ft)| matches!(ft, Ft::I64 | Ft::F64))
                    .cloned()
                    .collect();
                if !numeric.is_empty() {
                    let (fname, ft) = rng.pick(&numeric).clone();
                    let cmp = *rng.pick(OPS_ORD);
                    // 30%: an earlier same-class binding as the comparand
                    // (the cross beta path). The '/' variant keeps a
                    // LITERAL comparand — eq/ne between an int division
                    // and a binding is fenced (D-290 boxed comparison).
                    let earlier: Vec<String> = pats[..pi]
                        .iter()
                        .flat_map(|p| {
                            p.bindings
                                .iter()
                                .filter(|(_, _, bft)| *bft == ft)
                                .map(|(v, _, _)| v.clone())
                        })
                        .collect();
                    let bind_cmp = !earlier.is_empty() && rng.chance(30);
                    let c = match ft {
                        Ft::I64 => {
                            let rhs = if bind_cmp {
                                rng.pick(&earlier).clone()
                            } else {
                                format!("{}", rng.below(9))
                            };
                            match rng.below(5) {
                                0 => format!("{fname} + {} {cmp} {rhs}", rng.below(7)),
                                1 => format!("{fname} - {} {cmp} {rhs}", rng.below(7)),
                                2 => format!("{fname} * {} {cmp} {rhs}", rng.below(5)),
                                3 => format!("{fname} % {} {cmp} {rhs}", rng.below(8) + 1),
                                _ => format!(
                                    "{fname} / {} {cmp} {}",
                                    rng.below(8) + 1,
                                    rng.below(5)
                                ),
                            }
                        }
                        _ => {
                            let rhs = if bind_cmp {
                                rng.pick(&earlier).clone()
                            } else {
                                format!("{}.0", rng.below(9))
                            };
                            match rng.below(3) {
                                0 => format!("{fname} + {}.5 {cmp} {rhs}", rng.below(5)),
                                1 => format!("{fname} * {}.0 {cmp} {rhs}", rng.below(4) + 1),
                                _ => format!("{fname} / {}.5 {cmp} {rhs}", rng.below(4)),
                            }
                        }
                    };
                    pats[pi].constraints.push(c);
                }
            }
            // Field bindings (positive patterns only — D-031).
            if pats[pi].ce == 0 {
                let nbind = rng.below(3);
                for bi in 0..nbind {
                    let fs = &types[pats[pi].ti].fields;
                    let fi = rng.below(fs.len());
                    let ft = fs[fi].1;
                    let var = format!("$b{ri}_{pi}_{bi}");
                    let fname = fs[fi].0.clone();
                    pats[pi].constraints.push(format!("{var} : {fname}"));
                    pats[pi].bindings.push((var, fi, ft));
                }
            }
            // Accumulate/collect materialization (D-038/D-041): the
            // collect-vs-accumulate split was decided at creation; pick
            // the function by the source's numeric supply. The result
            // var is an ordinary downstream binding (except collect).
            if pats[pi].ce == 4 {
                let rvar = format!("$a{ri}_{pi}");
                pats[pi].acc = Some(("collect".into(), String::new(), rvar));
            } else if pats[pi].ce == 3 {
                let numeric: Vec<(usize, String, Ft)> = types[pats[pi].ti]
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, ft))| matches!(ft, Ft::I64 | Ft::F64))
                    .map(|(i, (n, ft))| (i, n.clone(), *ft))
                    .collect();
                let choice = rng.below(100);
                if choice < 25 || numeric.is_empty() {
                    // count()
                    let rvar = format!("$a{ri}_{pi}");
                    pats[pi].acc = Some(("count".into(), String::new(), rvar.clone()));
                    pats[pi].bindings.push((rvar, usize::MAX, Ft::I64));
                } else {
                    let (fi, fname, ft) = rng.pick(&numeric).clone();
                    // D-108: the collectors draw too — their results are
                    // OPAQUE (Collections; no downstream comparisons).
                    // D-163: min/max draw under mutation too (wall lifted).
                    let funcs: &[&str] =
                        &["sum", "average", "min", "max", "collectList", "collectSet"];
                    let func = *rng.pick(funcs);
                    let avar = format!("$s{ri}_{pi}");
                    pats[pi].constraints.push(format!("{avar} : {fname}"));
                    let rvar = format!("$a{ri}_{pi}");
                    let rft = if func == "average" { Ft::F64 } else { ft };
                    pats[pi].acc = Some((func.into(), avar, rvar.clone()));
                    // min/max over double args do not COMPILE in
                    // downstream comparisons (Drools Number typing,
                    // D-039) — leave those results unbound outward.
                    if !(matches!(func, "min" | "max") && ft == Ft::F64)
                        && !matches!(func, "collectList" | "collectSet")
                    {
                        pats[pi].bindings.push((rvar, fi, rft));
                    }
                }
            }
            // Group CE inners (P1c/D-089): two inner patterns; shapes —
            // plain `and`, outer-binding correlation (sn_a5),
            // inner-crossing binding (sn_a6), bare-not inner (sn_g5 /
            // forall substrate), `or` form (fuzzes the De Morgan /
            // double-negation rewrites; no inner bindings there —
            // rewritten branches are bare CEs, D-031).
            if pats[pi].ce >= 5 {
                let or_form = rng.chance(18);
                let t1 = rng.below(ntypes);
                let t2 = rng.below(ntypes);
                let mut c1: Vec<String> = Vec::new();
                let mut c2: Vec<String> = Vec::new();
                if rng.chance(55) {
                    let fs = &types[t1].fields;
                    let (fname, ft) = fs[rng.below(fs.len())].clone();
                    c1.push(gen_alpha_constraint(&mut rng, &fname, ft));
                }
                if rng.chance(55) {
                    let fs = &types[t2].fields;
                    let (fname, ft) = fs[rng.below(fs.len())].clone();
                    c2.push(gen_alpha_constraint(&mut rng, &fname, ft));
                }
                // correlation to an earlier OUTER binding (constraint,
                // not a binding — legal in or-forms too)
                if pi > 0 && rng.chance(45) {
                    let earlier: Vec<(String, Ft)> = pats[..pi]
                        .iter()
                        .flat_map(|p| p.bindings.iter().map(|(v, _, ft)| (v.clone(), *ft)))
                        .collect();
                    if !earlier.is_empty() {
                        let (tgt, cs) =
                            if rng.chance(50) { (t1, &mut c1) } else { (t2, &mut c2) };
                        let fs = &types[tgt].fields;
                        let fi = rng.below(fs.len());
                        let (fname, ft) = fs[fi].clone();
                        let compat: Vec<&(String, Ft)> = earlier
                            .iter()
                            .filter(|(_, bft)| ft.join_compatible(*bft))
                            .collect();
                        if !compat.is_empty() {
                            let (var, bft) = (*rng.pick(&compat)).clone();
                            let op = if ft == Ft::Bool || bft == Ft::Bool {
                                *rng.pick(OPS_EQ)
                            } else {
                                *rng.pick(OPS_ORD)
                            };
                            cs.push(format!("{fname} {op} {var}"));
                        }
                    }
                }
                let mut bare_not_2 = false;
                if !or_form {
                    // bare-not second inner (no bindings on IT, D-031) —
                    // combinable with the crossing below: the combined
                    // shape is the forall correlation substrate
                    // not(A($g : f) and not(B(f' op $g))), sn_a10
                    bare_not_2 = rng.chance(17);
                    // inner-crossing binding: inner-1 binds, inner-2
                    // filters on it (group-scoped var)
                    if rng.chance(30) {
                        let fs1 = &types[t1].fields;
                        let fi1 = rng.below(fs1.len());
                        let (f1name, f1t) = fs1[fi1].clone();
                        let gv = format!("$g{ri}_{pi}");
                        let fs2 = &types[t2].fields;
                        let compat: Vec<(String, Ft)> = fs2
                            .iter()
                            .filter(|(_, ft)| ft.join_compatible(f1t))
                            .cloned()
                            .collect();
                        if !compat.is_empty() {
                            let (f2name, f2t) = rng.pick(&compat).clone();
                            let op = if f1t == Ft::Bool || f2t == Ft::Bool {
                                *rng.pick(OPS_EQ)
                            } else {
                                *rng.pick(OPS_ORD)
                            };
                            c1.push(format!("{gv} : {f1name}"));
                            c2.push(format!("{f2name} {op} {gv}"));
                        }
                    }
                }
                pats[pi].group = vec![(t1, c1, false), (t2, c2, bare_not_2)];
                pats[pi].group_or = or_form;
            }
        }

        // Guard constraint for the update rule.
        let mut guard_field: Option<(usize, String)> = None;
        if let Some(pos) = update_pos {
            let bool_fields: Vec<(usize, String)> = types[pats[pos].ti]
                .fields
                .iter()
                .enumerate()
                .filter(|(_, (_, ft))| *ft == Ft::Bool)
                .map(|(i, (n, _))| (i, n.clone()))
                .collect();
            let (gfi, gname) = rng.pick(&bool_fields).clone();
            pats[pos].constraints.push(format!("{gname} == false"));
            guard_field = Some((gfi, gname));
        }

        // Snapshot the finished LHS so later rules can reuse its prefix
        // (taken after the guard append so copies share the FINAL
        // pattern identity).
        prev_rules.push(
            pats.iter()
                .map(|p| GenPattern {
                    ti: p.ti,
                    ce: p.ce,
                    fact_var: None,
                    constraints: p.constraints.clone(),
                    bindings: p.bindings.clone(),
                    acc: None,
                    group: p.group.clone(),
                    group_or: p.group_or,
                })
                .collect(),
        );

        // RHS actions.
        let mut actions: Vec<String> = Vec::new();
        // D-106: setFocus at low probability — pushes/relocations
        // compose against every other mechanism
        if !declared_groups.is_empty() && rng.chance(10) {
            let g = *rng.pick(&declared_groups);
            actions.push(format!("drools.setFocus(\"{g}\");"));
        }
        let max_ti = pats
            .iter()
            .flat_map(|p| {
                std::iter::once(p.ti).chain(p.group.iter().map(|(gt, _, _)| *gt))
            })
            .max()
            .unwrap();
        let can_insert = max_ti + 1 < ntypes && delete_pos.is_none();
        // insertLogical eligibility (D-076): never from acc/collect
        // rules (engine wall); the DAG bound uses POSITIVE patterns only
        // — not/exists over the logical type may still justify it (the
        // t10 self-defeat family terminates by the eager-retract pin).
        let has_acc = pats.iter().any(|p| p.ce >= 3 && p.ce <= 4);
        // D-089 wall (Bryan's ruling): group-CE justifiers are out —
        // the generator never draws insertLogical from group rules.
        let has_group = pats.iter().any(|p| p.ce >= 5);

        if can_insert {
            let nins = rng.below(3);
            for _ in 0..nins {
                let tgt_ti = max_ti + 1 + rng.below(ntypes - max_ti - 1);
                let is_logical = tms && tgt_ti == logical_ti;
                let mut args = Vec::new();
                let tgt_fields = types[tgt_ti].fields.clone();
                for (_, tft) in &tgt_fields {
                    let base = gen_arg(&mut rng, &types, &mut pats, ri, *tft, false);
                    // D-283/D-284 axis: computed args on plain inserts AND
                    // logical inserts. Gen shapes stay ACYCLIC by
                    // construction (targets always have higher type
                    // indices than premises) — a D-296 directive, not a
                    // wall: cyclic computed = designed runaways/deep
                    // teardowns, which pr_ub_* probes carry instead of
                    // fuzz. Typed so javac agrees: int arithmetic
                    // into i64 fields, any-numeric into f64. Divisors are
                    // NONZERO literals (div0 is judge-parity agreement but
                    // wastes the scenario).
                    let _ = is_logical;
                    if matches!(*tft, Ft::I64 | Ft::F64) && rng.chance(5) {
                        let op = ['+', '-', '*', '/', '%'][rng.below(5)];
                        let rhs = match (op, *tft) {
                            ('/' | '%', Ft::I64) => format!("{}", rng.below(9) + 1),
                            ('/' | '%', _) => format!("{}.5", rng.below(9)),
                            (_, Ft::I64) => format!("{}", rng.below(7)),
                            _ => format!("{}.0", rng.below(7)),
                        };
                        args.push(format!("{base} {op} {rhs}"));
                    } else {
                        args.push(base);
                    }
                }
                // D-080 envelope: the logical type is PURE — only
                // insertLogical touches it, and justifiers never mutate
                // in the same RHS (the compound transient-visibility
                // micro-timings are documented-open, not fuzzed).
                if tms && tgt_ti == logical_ti {
                    if has_acc || has_group || update_pos.is_some() || delete_pos.is_some() {
                        continue;
                    }
                    actions.push(format!(
                        "insertLogical(new {}({}));",
                        types[tgt_ti].name,
                        args.join(", ")
                    ));
                } else {
                    actions.push(format!("insert(new {}({}));", types[tgt_ti].name, args.join(", ")));
                }
            }
        }
        // D-080: CE-only self-justifiers (not/exists over the logical
        // type + insertLogical of it) are NOT fuzzed — multi-dep
        // self-defeat cycles are genuine Drools RUNAWAYS (or-twin /
        // fz_42_946 family) and the transient-visibility drain rule has
        // unpinned structure beyond the t20 matrix (min812). The
        // single-tuple semantics stay certified via the hand probes
        // (pr_tms_t10/t11/t15/t21).
        if let Some(pos) = update_pos {
            let var = ensure_fact_var(&mut pats, ri, pos);
            let (gfi, gname) = guard_field.clone().unwrap();
            // guard setter + 0..2 extra setters (bools only ever set true)
            let mut setters: Vec<(String, String)> = Vec::new();
            setters.push((accessor(&gname, Ft::Bool, "set"), "true".into()));
            let nextra = rng.below(3);
            for _ in 0..nextra {
                let fs = &types[pats[pos].ti].fields;
                let fi = rng.below(fs.len());
                if fi == gfi {
                    continue;
                }
                let (fname, ft) = fs[fi].clone();
                let mut arg = gen_arg(&mut rng, &types, &mut pats, ri, ft, true);
                // D-288 axis: computed setter args, the insert-axis shape
                // verbatim (typed so javac agrees; nonzero literal
                // divisors). The guard setter stays atom-true, so the
                // guard-flip termination invariant is untouched.
                if matches!(ft, Ft::I64 | Ft::F64) && rng.chance(5) {
                    let op = ['+', '-', '*', '/', '%'][rng.below(5)];
                    let rhs = match (op, ft) {
                        ('/' | '%', Ft::I64) => format!("{}", rng.below(9) + 1),
                        ('/' | '%', _) => format!("{}.5", rng.below(9)),
                        (_, Ft::I64) => format!("{}", rng.below(7)),
                        _ => format!("{}.0", rng.below(7)),
                    };
                    arg = format!("{arg} {op} {rhs}");
                }
                setters.push((accessor(&fname, ft, "set"), arg));
            }
            if rng.chance(50) {
                let body: Vec<String> =
                    setters.iter().map(|(s, a)| format!("{s}({a})")).collect();
                actions.push(format!("modify({var}) {{ {} }}", body.join(", ")));
            } else {
                for (s, a) in &setters {
                    actions.push(format!("{var}.{s}({a});"));
                }
                actions.push(format!("update({var});"));
            }
        }
        if let Some(pos) = delete_pos {
            let var = ensure_fact_var(&mut pats, ri, pos);
            actions.push(format!("delete({var});"));
            rule_deleted_types.insert(pats[pos].ti);
        }

        // D-070 `or` grammar: ~18% of acc-free rules become 2-3 branch
        // or-rules. Extra branches COPY the pattern list (same types,
        // same binding names — every RHS/salience reference stays
        // every-branch-bound; duplicate fact bindings keep their type,
        // or_b1/or_b4) with literal-alpha mutation. The update GUARD
        // constraint is never mutated (termination by construction).
        // Acc/collect rules stay single-branch: identical acc twins
        // would fuzz the unprobed acc-sharing surface (D-038 spirit).
        let guard_text = guard_field.as_ref().map(|(_, g)| format!("{g} == false"));
        let mut branches: Vec<Vec<GenPattern>> = vec![pats.clone()];
        if pats.iter().all(|p| p.ce < 3 || p.ce >= 5) && rng.chance(18) {
            let nb = if rng.chance(25) { 3 } else { 2 };
            for _ in 1..nb {
                let mut copy: Vec<GenPattern> = pats.clone();
                for cp in &mut copy {
                    for c in cp.constraints.iter_mut() {
                        if !c.contains('$')
                            && guard_text.as_deref() != Some(c.as_str())
                            && rng.chance(40)
                        {
                            let fs = &types[cp.ti].fields;
                            let (fname, ft) = fs[rng.below(fs.len())].clone();
                            *c = gen_alpha_constraint(&mut rng, &fname, ft);
                        }
                    }
                    if cp.ce == 0 && rng.chance(15) {
                        let fs = &types[cp.ti].fields;
                        let (fname, ft) = fs[rng.below(fs.len())].clone();
                        cp.constraints.push(gen_alpha_constraint(&mut rng, &fname, ft));
                    }
                }
                branches.push(copy);
            }
        }

        // Render the rule.
        // Salience: static int, or a computed expression over this
        // rule's numeric bindings (D-043). Accumulate-result bindings
        // ($a…) are excluded (typing unprobed).
        let numeric_binds: Vec<String> = pats
            .iter()
            .flat_map(|p| {
                p.bindings
                    .iter()
                    .filter(|(v, _, ft)| {
                        !v.starts_with("$a") && matches!(ft, Ft::I64 | Ft::F64)
                    })
                    .map(|(v, _, _)| v.clone())
            })
            .collect();
        let sal_expr = if !numeric_binds.is_empty() && rng.chance(15) {
            let a = rng.pick(&numeric_binds).clone();
            let form = rng.below(3);
            Some(match form {
                0 => format!("salience({a})\n"),
                1 => {
                    let op = *rng.pick(&["+", "-", "*"]);
                    let lit = (rng.below(9) as i64) - 4;
                    format!("salience({a} {op} {lit})\n")
                }
                _ => {
                    let b = rng.pick(&numeric_binds).clone();
                    let op = *rng.pick(&["+", "-"]);
                    format!("salience({a} {op} {b})\n")
                }
            })
        } else {
            None
        };
        let salience = if sal_expr.is_none() && rng.chance(35) {
            (rng.below(21) as i64) - 10
        } else {
            0
        };
        drl.push_str(&format!("rule \"R{ri}\"\n"));
        if let Some(se) = sal_expr {
            drl.push_str(&se);
        } else if salience != 0 {
            drl.push_str(&format!("salience {salience}\n"));
        }
        if rng.chance(10) {
            drl.push_str("no-loop\n");
        }
        // D-106: agenda groups at low probability (two group names so
        // stacking/relocation paths get exercised)
        if let Some(g) = rule_groups[ri] {
            drl.push_str(&format!("agenda-group \"{g}\"\n"));
        }
        drl.push_str("when\n");
        let render_pat = |p: &GenPattern| -> String {
            if let Some((func, avar, rvar)) = &p.acc {
                let src = format!("{}({})", types[p.ti].name, p.constraints.join(", "));
                if p.ce == 4 {
                    return format!("{rvar} : ArrayList() from collect( {src} )");
                }
                let arg = if func == "count" { String::new() } else { avar.clone() };
                return format!("accumulate( {src}; {rvar} : {func}({arg}) )");
            }
            if p.ce >= 5 {
                let kw = if p.ce == 5 { "not" } else { "exists" };
                let inners: Vec<String> = p
                    .group
                    .iter()
                    .map(|(ti, cs, bare_not)| {
                        let body = format!("{}({})", types[*ti].name, cs.join(", "));
                        if *bare_not { format!("not({body})") } else { body }
                    })
                    .collect();
                let joiner = if p.group_or { " or " } else { " and " };
                return format!("{kw}({})", inners.join(joiner));
            }
            let ce = match p.ce {
                1 => "not ",
                2 => "exists ",
                _ => "",
            };
            let head = match &p.fact_var {
                Some(v) => format!("{v} : "),
                None => String::new(),
            };
            format!("{ce}{head}{}({})", types[p.ti].name, p.constraints.join(", "))
        };
        if branches.len() == 1 {
            for p in &pats {
                drl.push_str(&format!("    {}\n", render_pat(p)));
            }
        } else if rng.chance(30) {
            // prefix form: (or (and p q) (and p q)) — or_a7/or_a14
            let bs: Vec<String> = branches
                .iter()
                .map(|b| {
                    let ps: Vec<String> = b.iter().map(&render_pat).collect();
                    if ps.len() == 1 {
                        ps.into_iter().next().unwrap()
                    } else {
                        format!("(and {})", ps.join(" "))
                    }
                })
                .collect();
            drl.push_str(&format!("    (or {})\n", bs.join(" ")));
        } else {
            // infix form: ( p and q ) or ( p and q ) — or_a35/or_a43
            for (bi, b) in branches.iter().enumerate() {
                if bi > 0 {
                    drl.push_str("    or\n");
                }
                let ps: Vec<String> = b.iter().map(&render_pat).collect();
                drl.push_str(&format!("    ( {} )\n", ps.join(" and ")));
            }
        }
        drl.push_str("then\n");
        for a in &actions {
            drl.push_str(&format!("    {a}\n"));
        }
        drl.push_str("end\n");
    }

    // Facts: 0..6, plus MULTI-FIRE epochs (D-046) in ~30% of scenarios:
    // 1-2 extra insert-then-fire batches over the same session.
    let nfacts = rng.below(7);
    let mut facts = Vec::new();
    for _ in 0..nfacts {
        let ti = rng.below(if tms { logical_ti } else { ntypes });
        let t = &types[ti];
        let mut fields = serde_json::Map::new();
        for (fname, ft) in &t.fields {
            fields.insert(fname.clone(), lit_json(&mut rng, *ft));
        }
        facts.push(json!({"type": t.name, "fields": fields}));
    }
    let mut epochs = Vec::new();
    if rng.chance(30) {
        // external update/delete targets: INITIAL facts only (their
        // visible indices are static; later indices depend on how many
        // facts rules inserted) of types no rule deletes; each target
        // deleted at most once, never updated after deletion (D-047).
        let safe: Vec<usize> = facts
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                let tname = f["type"].as_str().unwrap();
                let ti = types.iter().position(|t| t.name == tname).unwrap();
                !rule_deleted_types.contains(&ti)
            })
            .map(|(i, _)| i)
            .collect();
        let mut ext_deleted: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let nepochs = 1 + rng.below(2);
        for _ in 0..nepochs {
            let mut eactions = Vec::new();
            let nact = rng.below(3);
            for _ in 0..nact {
                let alive: Vec<usize> =
                    safe.iter().copied().filter(|i| !ext_deleted.contains(i)).collect();
                let roll = rng.below(100);
                if roll < 40 || alive.is_empty() {
                    let ti = rng.below(if tms { logical_ti } else { ntypes });
                    let t = &types[ti];
                    let mut fields = serde_json::Map::new();
                    for (fname, ft) in &t.fields {
                        fields.insert(fname.clone(), lit_json(&mut rng, *ft));
                    }
                    eactions.push(json!({"op": "insert", "type": t.name, "fields": fields}));
                } else if roll < 80 {
                    let target = *rng.pick(&alive);
                    let tname = facts[target]["type"].as_str().unwrap();
                    let t = types.iter().find(|t| t.name == tname).unwrap();
                    // D-076 mutation wall: external updates never touch
                    // the logical type — delete it instead (in-subset,
                    // exercises the quirk model). (The D-093 min/max
                    // reroute is LIFTED — D-163, oracle 9.44.0.Final+p1.)
                    if tms && tname == types[logical_ti].name {
                        ext_deleted.insert(target);
                        eactions.push(json!({"op": "delete", "target": target}));
                        continue;
                    }
                    let nf = 1 + rng.below(2.min(t.fields.len()));
                    let mut fields = serde_json::Map::new();
                    for _ in 0..nf {
                        let (fname, ft) = &t.fields[rng.below(t.fields.len())];
                        fields.insert(fname.clone(), lit_json(&mut rng, *ft));
                    }
                    eactions.push(json!({"op": "update", "target": target, "fields": fields}));
                } else {
                    let target = *rng.pick(&alive);
                    ext_deleted.insert(target);
                    eactions.push(json!({"op": "delete", "target": target}));
                }
            }
            let nef = rng.below(3);
            let mut efacts = Vec::new();
            for _ in 0..nef {
                let ti = rng.below(if tms { logical_ti } else { ntypes });
                let t = &types[ti];
                let mut fields = serde_json::Map::new();
                for (fname, ft) in &t.fields {
                    fields.insert(fname.clone(), lit_json(&mut rng, *ft));
                }
                efacts.push(json!({"type": t.name, "fields": fields}));
            }
            epochs.push(json!({"actions": eactions, "facts": efacts}));
        }
    }

    // ------------------------------------------------------------------
    // Queries: Phase Q0 (D-049..D-053) + Phase Q1 or-branches and the
    // recursive transitive-closure family (D-054/D-055). Attached to
    // INSERT-ONLY programs only — no rule mutation, no external
    // update/delete (staged-insert/mutation interplay is walled).
    // Literal `==` constraints are never generated on T-type patterns
    // inside query bodies (a query alpha node would join the oracle's
    // D-029 eq-literal hash groups, which the engine's rules-only rewrite
    // does not model); the Rel/Mark types are query-only, so positional
    // call literals and Mark equality filters are safe. Query joins are
    // SAME-type only; recursion data is a DAG by construction (edges run
    // low->high node index) so evaluation always terminates.
    let epochs_insert_only = epochs.iter().all(|e| {
        e["actions"]
            .as_array()
            .map(|a| a.iter().all(|x| x["op"] == "insert"))
            .unwrap_or(true)
    });
    let mut queries_json: Vec<J> = Vec::new();
    // Queries visible to ?query CEs (D-056): name + per-param (name,
    // type, unbound-eligible). An UNBOUND CE arg requires the param to be
    // bound in EVERY callee branch (D-057) — tracked at generation.
    let mut q2_queries: Vec<(String, Vec<(Ft, bool)>)> = Vec::new();
    if !tms && !allow_mutation && epochs_insert_only && rng.chance(45) {
        const QOPS_NOEQ: &[&str] = &["!=", "<", "<=", ">", ">="];
        let nqueries = 1 + rng.below(2); // 1..2
        for qi in 0..nqueries {
            let qname = format!("Q{qi}");
            // or-branches (D-054): 1 (60%), 2 (32%), 3 (8%); with >1
            // branch every branch is parenthesized and 1..2 patterns long
            let nbranches = match rng.below(100) {
                0..=59 => 1,
                60..=91 => 2,
                _ => 3,
            };
            let mut params: Vec<(String, Ft)> = Vec::new();
            // branches each param is unified in (unbound-arg eligibility)
            let mut param_branches: Vec<std::collections::HashSet<usize>> = Vec::new();
            let mut bind_n = 0usize;
            let mut pat_n = 0usize;
            let mut branch_texts: Vec<String> = Vec::new();
            for bi in 0..nbranches {
                // locals are per-branch (cross-branch reuse is rejected,
                // D-055); params are shared across branches
                let mut scalar_binds: Vec<(String, Ft)> = Vec::new();
                let npats = if nbranches == 1 { 1 + rng.below(3) } else { 1 + rng.below(2) };
                let mut pats: Vec<String> = Vec::new();
                for pj in 0..npats {
                    let ti = rng.below(ntypes);
                    let nfields = types[ti].fields.len();
                    let mut cons: Vec<String> = Vec::new();
                    // param unifications (first pattern biased to have one)
                    let nunif = if pj == 0 { 1 + rng.below(2) } else { rng.below(2) };
                    for _ in 0..nunif {
                        let (fname, ft) = types[ti].fields[rng.below(nfields)].clone();
                        let reuse: Vec<usize> = params
                            .iter()
                            .enumerate()
                            .filter(|(_, (_, pt))| *pt == ft)
                            .map(|(i, _)| i)
                            .collect();
                        let pn = if !reuse.is_empty() && (params.len() >= 3 || rng.chance(30)) {
                            let pi = *rng.pick(&reuse);
                            param_branches[pi].insert(bi);
                            params[pi].0.clone()
                        } else if params.len() < 3 {
                            let pn = format!("$qa{qi}_{}", params.len());
                            params.push((pn.clone(), ft));
                            let mut set = std::collections::HashSet::new();
                            set.insert(bi);
                            param_branches.push(set);
                            pn
                        } else {
                            continue;
                        };
                        cons.push(format!("{fname} == {pn}"));
                    }
                    // literal filters: never `==`, always same-type
                    for _ in 0..rng.below(3) {
                        let (fname, ft) = &types[ti].fields[rng.below(nfields)];
                        let op = match ft {
                            Ft::Bool => "!=",
                            _ => *rng.pick(QOPS_NOEQ),
                        };
                        cons.push(format!("{fname} {op} {}", lit_drl(&mut rng, *ft, false)));
                    }
                    // joins to earlier same-branch bindings (same-type only)
                    for _ in 0..rng.below(3) {
                        let compat: Vec<(String, Ft)> = scalar_binds
                            .iter()
                            .filter(|(_, bt)| types[ti].fields.iter().any(|(_, ft)| ft == bt))
                            .cloned()
                            .collect();
                        if compat.is_empty() {
                            break;
                        }
                        let (bvar, bt) = rng.pick(&compat).clone();
                        let fnames: Vec<String> = types[ti]
                            .fields
                            .iter()
                            .filter(|(_, ft)| *ft == bt)
                            .map(|(n, _)| n.clone())
                            .collect();
                        let fname = rng.pick(&fnames).clone();
                        let op = if bt == Ft::Bool {
                            if rng.chance(60) { "==" } else { "!=" }
                        } else if rng.chance(60) {
                            "=="
                        } else {
                            *rng.pick(QOPS_NOEQ)
                        };
                        cons.push(format!("{fname} {op} {bvar}"));
                    }
                    // field bindings, visible to later same-branch patterns
                    for _ in 0..rng.below(3) {
                        let (fname, ft) = types[ti].fields[rng.below(nfields)].clone();
                        let bvar = format!("$qb{qi}_{bind_n}");
                        bind_n += 1;
                        scalar_binds.push((bvar.clone(), ft));
                        cons.push(format!("{bvar} : {fname}"));
                    }
                    let _ = pj;
                    pats.push(format!("$qp{qi}_{pat_n} : {}({})", types[ti].name, cons.join(", ")));
                    pat_n += 1;
                }
                branch_texts.push(pats.join(" and "));
            }
            // every param must be used in >=1 branch: minted at use sites
            let body = if nbranches == 1 {
                branch_texts[0]
                    .split(" and ")
                    .map(|p| format!("    {p}\n"))
                    .collect::<String>()
            } else {
                branch_texts
                    .iter()
                    .map(|b| format!("    ( {b} )\n"))
                    .collect::<Vec<_>>()
                    .join("    or\n")
            };
            q2_queries.push((
                qname.clone(),
                params
                    .iter()
                    .zip(&param_branches)
                    .map(|((_, ft), brs)| (*ft, brs.len() == nbranches))
                    .collect(),
            ));
            if params.is_empty() {
                drl.push_str(&format!("query {qname}\n{body}end\n\n"));
            } else {
                let plist = params
                    .iter()
                    .map(|(n, ft)| {
                        let jt = match ft {
                            Ft::I64 => "long",
                            Ft::F64 => "double",
                            Ft::Str => "String",
                            Ft::Bool => "boolean",
                        };
                        format!("{jt} {n}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                drl.push_str(&format!("query {qname}({plist})\n{body}end\n\n"));
            }
            for _ in 0..(1 + rng.below(3)) {
                let args: Vec<J> = params
                    .iter()
                    .map(|(_, ft)| {
                        if rng.chance(45) {
                            J::Null
                        } else {
                            lit_json(&mut rng, *ft)
                        }
                    })
                    .collect();
                queries_json.push(json!({"call": qname, "args": args}));
            }
        }

        // Recursive transitive-closure family (D-054/D-055): Rel/Mark are
        // query-only types over a generated DAG; the canonical base-first
        // 2-branch shape, optionally Mark-filtered after the self-call,
        // plus an optional wrapper query with fresh/literal/param args.
        if rng.chance(55) {
            let node_ft = if rng.chance(50) { Ft::Str } else { Ft::I64 };
            let n_nodes = 4 + rng.below(5); // 4..8
            let node_json = |i: usize| -> J {
                match node_ft {
                    Ft::Str => json!(format!("v{i}")),
                    _ => json!(i as i64 * 3 + 1),
                }
            };
            let node_drl = |i: usize| -> String {
                match node_ft {
                    Ft::Str => format!("{:?}", format!("v{i}")),
                    _ => format!("{}", i as i64 * 3 + 1),
                }
            };
            let jt = match node_ft {
                Ft::Str => "String",
                _ => "long",
            };
            types.push(TypeDef {
                name: "RelR".into(),
                fields: vec![("src".into(), node_ft), ("dst".into(), node_ft)],
            });
            types.push(TypeDef {
                name: "MarkR".into(),
                fields: vec![("node".into(), node_ft)],
            });
            let n_edges = 3 + rng.below(n_nodes + 2);
            for _ in 0..n_edges {
                let a = rng.below(n_nodes - 1);
                let b = a + 1 + rng.below(n_nodes - a - 1);
                facts.push(json!({"type": "RelR", "fields": {"src": node_json(a), "dst": node_json(b)}}));
            }
            for i in 0..n_nodes {
                if rng.chance(45) {
                    facts.push(json!({"type": "MarkR", "fields": {"node": node_json(i)}}));
                }
            }
            let base = if rng.chance(30) {
                "( RelR(src == $a, dst == $b) )".to_string()
            } else {
                "( RelR($a, $b;) )".to_string()
            };
            let markf = if rng.chance(30) { " and MarkR(node == $m)" } else { "" };
            drl.push_str(&format!(
                "query TCr({jt} $a, {jt} $b)\n    {base}\n    or\n    ( RelR($m, $b;) and TCr($a, $m;){markf} )\nend\n\n"
            ));
            // both params bound in every branch: base unifies both; the
            // recursive branch unifies $b and threads $a into the
            // self-call (bottoming out at the base branch)
            q2_queries.push(("TCr".into(), vec![(node_ft, true), (node_ft, true)]));
            let arg = |rng: &mut Rng| -> J {
                if rng.chance(45) {
                    J::Null
                } else if rng.chance(12) {
                    // a value outside the node pool
                    match node_ft {
                        Ft::Str => json!("vnone"),
                        _ => json!(-7),
                    }
                } else {
                    node_json(rng.below(n_nodes))
                }
            };
            for _ in 0..(2 + rng.below(3)) {
                let a = arg(&mut rng);
                let b = arg(&mut rng);
                queries_json.push(json!({"call": "TCr", "args": [a, b]}));
            }
            if rng.chance(40) {
                // wrapper: fresh-threading or literal second arg (qd1/qb5)
                if rng.chance(50) {
                    let post = if rng.chance(50) { "    MarkR(node == $f)\n" } else { "" };
                    drl.push_str(&format!(
                        "query TWr({jt} $w)\n    TCr($w, $f;)\n{post}end\n\n"
                    ));
                } else {
                    let lit = node_drl(rng.below(n_nodes));
                    drl.push_str(&format!(
                        "query TWr({jt} $w)\n    TCr($w, {lit};)\nend\n\n"
                    ));
                }
                // single branch, $w threaded into TCr param 0 (eligible)
                q2_queries.push(("TWr".into(), vec![(node_ft, true)]));
                for _ in 0..(1 + rng.below(2)) {
                    let a = arg(&mut rng);
                    queries_json.push(json!({"call": "TWr", "args": [a]}));
                }
            }
        }

        // --------------------------------------------------------------
        // Phase Q2 (D-056/D-057): rules with `?query` pull CEs over the
        // queries above. Termination: QR rules insert only the QOut sink
        // (matched by no pattern) and pull CEs are NOT reactive — new
        // rows never retrigger existing lefts; lefts stay bounded by the
        // T-type event pool. Unbound CE args only target params bound in
        // EVERY callee branch (the engine compile-rejects the rest,
        // D-057). Twin rules copy an LHS verbatim to draw the shared-CE
        // multi-sink polarity (qx3_two_rules/qx5_three_rules).
        // D-107: the D-057 walls are LIFTED — QR rules now draw beside
        // mutation programs too (pull-at-activation composes; the qmut
        // ladder pins the semantics).
        if !q2_queries.is_empty() && rng.chance(60) {
            let mut qout_used = false;
            let mut qr_lhs: Vec<String> = Vec::new(); // twin candidates
            let nqr = 1 + rng.below(3); // 1..3
            let mut tag = 0i64;
            for qri in 0..nqr {
                let rn = format!("QR{qri}");
                // leading T patterns: 0 (leading CE, InitialFact) .. 2
                let nlead = match rng.below(100) {
                    0..=19 => 0,
                    20..=74 => 1,
                    _ => 2,
                };
                let mut lhs: Vec<String> = Vec::new();
                // (var, ft) scalar bindings visible to CE args / RHS
                let mut binds: Vec<(String, Ft)> = Vec::new();
                let mut bind_n = 0usize;
                for _ in 0..nlead {
                    let ti = rng.below(ntypes);
                    let nfields = types[ti].fields.len();
                    let mut cons: Vec<String> = Vec::new();
                    for _ in 0..rng.below(2) {
                        let (fname, ft) = &types[ti].fields[rng.below(nfields)];
                        let op = match ft {
                            Ft::Bool => *rng.pick(&["==", "!="]),
                            _ => *rng.pick(OPS_ORD),
                        };
                        cons.push(format!("{fname} {op} {}", lit_drl(&mut rng, *ft, false)));
                    }
                    if rng.chance(60) {
                        let (fname, ft) = types[ti].fields[rng.below(nfields)].clone();
                        let bv = format!("$qr{qri}_b{bind_n}");
                        bind_n += 1;
                        cons.push(format!("{bv} : {fname}"));
                        binds.push((bv, ft));
                    }
                    lhs.push(format!("{}({})", types[ti].name, cons.join(", ")));
                }
                // 1..2 ?query CEs; fresh vars thread into later CEs/RHS
                let nce = 1 + rng.below(2);
                let mut fresh: Vec<(String, Ft)> = Vec::new();
                let mut fresh_n = 0usize;
                for _ in 0..nce {
                    let (qname, qparams) = rng.pick(&q2_queries).clone();
                    let mut args: Vec<String> = Vec::new();
                    // fresh vars from EARLIER CEs are bound here; vars
                    // minted by THIS call are repeated-unbound (not
                    // bound!) and stay out of the pool (fz_42_4330)
                    let fresh_before: Vec<(String, Ft)> = fresh.clone();
                    for (pt, eligible) in &qparams {
                        let bound_compat: Vec<String> = binds
                            .iter()
                            .chain(fresh_before.iter())
                            .filter(|(_, bt)| bt == pt)
                            .map(|(v, _)| v.clone())
                            .collect();
                        let roll = rng.below(100);
                        if roll < 45 && *eligible {
                            let v = format!("$qr{qri}_x{fresh_n}");
                            fresh_n += 1;
                            fresh.push((v.clone(), *pt));
                            args.push(v);
                        } else if roll < 75 && !bound_compat.is_empty() {
                            args.push(rng.pick(&bound_compat).clone());
                        } else {
                            args.push(lit_drl(&mut rng, *pt, false));
                        }
                    }
                    if args.is_empty() {
                        lhs.push(format!("?{qname}()"));
                    } else {
                        lhs.push(format!("?{qname}({};)", args.join(", ")));
                    }
                }
                // optional trailing T pattern joining a CE var (qx0_after)
                if !fresh.is_empty() && rng.chance(35) {
                    let (v, vt) = rng.pick(&fresh).clone();
                    let cands: Vec<(usize, String)> = types[..ntypes]
                        .iter()
                        .enumerate()
                        .flat_map(|(ti, t)| {
                            t.fields
                                .iter()
                                .filter(|(_, ft)| *ft == vt)
                                .map(move |(n, _)| (ti, n.clone()))
                        })
                        .collect();
                    if !cands.is_empty() {
                        let (ti, fname) = rng.pick(&cands).clone();
                        lhs.push(format!("{}({fname} == {v})", types[ti].name));
                    }
                }
                // RHS: QOut(tag, i, d, s, z) with CE/pattern vars where
                // types line up (i64 widens into d)
                let all_vars: Vec<(String, Ft)> =
                    binds.iter().cloned().chain(fresh.iter().cloned()).collect();
                let arg_for = |ft: Ft, rng: &mut Rng| -> String {
                    let compat: Vec<String> = all_vars
                        .iter()
                        .filter(|(_, vt)| *vt == ft || (ft == Ft::F64 && *vt == Ft::I64))
                        .map(|(v, _)| v.clone())
                        .collect();
                    if !compat.is_empty() && rng.chance(60) {
                        rng.pick(&compat).clone()
                    } else {
                        lit_drl(rng, ft, false)
                    }
                };
                let sal = if rng.chance(35) {
                    format!(" salience {}", rng.below(21) as i64 - 10)
                } else {
                    String::new()
                };
                tag += 1;
                let rhs = format!(
                    "    insert(new QOut({tag}, {}, {}, {}, {}));\n",
                    arg_for(Ft::I64, &mut rng),
                    arg_for(Ft::F64, &mut rng),
                    arg_for(Ft::Str, &mut rng),
                    arg_for(Ft::Bool, &mut rng)
                );
                let lhs_text = lhs
                    .iter()
                    .map(|p| format!("    {p}\n"))
                    .collect::<String>();
                drl.push_str(&format!(
                    "rule {rn}{sal}\nwhen\n{lhs_text}then\n{rhs}end\n\n"
                ));
                qout_used = true;
                qr_lhs.push(lhs_text);
            }
            // twin rules: identical LHS text = shared LIA + shared CE
            // node (multi-sink drain polarity, D-056)
            if !qr_lhs.is_empty() && rng.chance(30) {
                let src = rng.pick(&qr_lhs).clone();
                let sal = if rng.chance(35) {
                    format!(" salience {}", rng.below(21) as i64 - 10)
                } else {
                    String::new()
                };
                tag += 1;
                drl.push_str(&format!(
                    "rule QRtwin{sal}\nwhen\n{src}then\n    insert(new QOut({tag}, 0, 0.0, \"t\", false));\nend\n\n"
                ));
            }
            if qout_used {
                types.push(TypeDef {
                    name: "QOut".into(),
                    fields: vec![
                        ("tag".into(), Ft::I64),
                        ("i".into(), Ft::I64),
                        ("d".into(), Ft::F64),
                        ("s".into(), Ft::Str),
                        ("z".into(), Ft::Bool),
                    ],
                });
            }
        }
    }

    let mut types_json: Vec<J> = types
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "fields": t.fields.iter()
                    .map(|(n, ft)| json!({"name": n, "type": ft.json_name()}))
                    .collect::<Vec<J>>(),
            })
        })
        .collect();

    // D-107: per-epoch query draws — each scenario-level call may also
    // run mid-scenario against an epoch's post-quiescence WM
    if !epochs.is_empty() && !queries_json.is_empty() {
        for q in queries_json.clone() {
            if rng.chance(30) {
                let ei = rng.below(epochs.len());
                let e = &mut epochs[ei];
                if e.get("queries").is_none() {
                    e["queries"] = json!([]);
                }
                e["queries"].as_array_mut().unwrap().push(q);
            }
        }
    }

    // D-254: the >96-distinct-key resize region is IN subset — ~10% of
    // query-bearing scenarios gain a DEDICATED swarm type + its own
    // unification query + standalone call, sized past the 96-key
    // threshold, so the index exercises the bulk pre-size + incremental
    // resize + chain-reversal model against the oracle at scale.
    // Dedicated on purpose: swarming a rule-matched type scales every
    // join in the scenario (a 3-pattern rule over a 100+-fact swarm is
    // millions of tuples — an effective hang on both sides of the
    // diff, not a probe), and swarming the RelR recursion DAG can go
    // cyclic and hang the oracle JVM (D-055). Keys draw WIDE (the
    // standard literal pools never reach 96 distinct values).
    if !queries_json.is_empty() && rng.chance(10) {
        let nswarm = 90 + rng.below(120); // 90..209: crosses 96, sometimes 192
        drl.push_str("query QSwarm(long $v)\n    SwarmT(k == $v)\nend\n\n");
        types_json.push(json!({
            "name": "SwarmT",
            "fields": [{"name": "k", "type": "i64"}],
        }));
        for _ in 0..nswarm {
            facts.push(json!({"type": "SwarmT", "fields": {"k": rng.below(1_000_000) as i64}}));
        }
        queries_json.push(json!({"call": "QSwarm", "args": [null]}));
    }

    let mut scenario = json!({
        "name": name,
        "types": types_json,
        "facts": facts,
        "drl": drl,
    });
    if !epochs.is_empty() {
        scenario["epochs"] = json!(epochs);
    }
    if !queries_json.is_empty() {
        scenario["queries"] = json!(queries_json);
    }
    (name, scenario)
}

fn ensure_fact_var(pats: &mut [GenPattern], ri: usize, pos: usize) -> String {
    debug_assert_eq!(pats[pos].ce, 0, "fact vars only on positive patterns");
    if pats[pos].fact_var.is_none() {
        pats[pos].fact_var = Some(format!("$p{ri}_{pos}"));
    }
    pats[pos].fact_var.clone().unwrap()
}

/// An RHS argument of target type `tft`: literal, earlier field binding, or
/// getter on some pattern's fact var. `only_true_bools` guards setter args.
fn gen_arg(
    rng: &mut Rng,
    types: &[TypeDef],
    pats: &mut [GenPattern],
    ri: usize,
    tft: Ft,
    only_true_bools: bool,
) -> String {
    let choice = rng.below(100);
    // Bool args in monotone-guard positions must be literal `true`.
    if tft == Ft::Bool && only_true_bools {
        return "true".into();
    }
    if choice >= 40 {
        // Try a binding.
        let binds: Vec<String> = pats
            .iter()
            .flat_map(|p| {
                p.bindings
                    .iter()
                    .filter(|(_, _, ft)| *ft == tft || (*ft == Ft::I64 && tft == Ft::F64))
                    .map(|(v, _, _)| v.clone())
            })
            .collect();
        if choice < 70 && !binds.is_empty() {
            return rng.pick(&binds).clone();
        }
        // Try a getter (positive patterns only — CE patterns cannot bind).
        let mut cands: Vec<(usize, usize)> = Vec::new(); // (pattern pos, field idx)
        for (pi, p) in pats.iter().enumerate() {
            if p.ce != 0 {
                continue;
            }
            for (fi, (_, ft)) in types[p.ti].fields.iter().enumerate() {
                if *ft == tft || (*ft == Ft::I64 && tft == Ft::F64) {
                    cands.push((pi, fi));
                }
            }
        }
        if !cands.is_empty() {
            let (pi, fi) = *rng.pick(&cands);
            let var = ensure_fact_var(pats, ri, pi);
            let (fname, ft) = types[pats[pi].ti].fields[fi].clone();
            return format!("{var}.{}()", accessor(&fname, ft, "get"));
        }
    }
    lit_drl(rng, tft, only_true_bools)
}
