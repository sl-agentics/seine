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
/// Returns (constraint text, canonical key part) — the key normalizes
/// `==` numeric literals to the field's type, mirroring the engine's
/// alpha-node identity (D-029/D-033) for the sharing wall.
fn gen_alpha_constraint(rng: &mut Rng, fname: &str, ft: Ft) -> (String, String) {
    match ft {
        Ft::Str => match rng.below(10) {
            0..=3 => {
                let op = *rng.pick(OPS_ORD);
                let lit = lit_drl(rng, Ft::Str, false);
                let c = format!("{fname} {op} {lit}");
                let k = c.clone();
                (c, k)
            }
            4..=5 => {
                let c = format!("{fname} matches \"{}\"", gen_regex(rng));
                let k = c.clone();
                (c, k)
            }
            6..=7 => {
                let c = format!("{fname} contains \"{}\"", gen_needle(rng));
                let k = c.clone();
                (c, k)
            }
            _ => {
                let c = format!("{fname}{}", gen_in_list(rng, Ft::Str));
                let k = c.clone();
                (c, k)
            }
        },
        Ft::I64 | Ft::F64 => {
            if rng.chance(25) {
                let c = format!("{fname}{}", gen_in_list(rng, ft));
                let k = c.clone();
                (c, k)
            } else {
                let op = *rng.pick(OPS_ORD);
                let cross = rng.chance(20);
                // generate the numeric value, then format text + key
                let (text_lit, key_lit) = match (ft, cross) {
                    (Ft::I64, false) => {
                        let v = gen_i64(rng);
                        (format!("{v}"), format!("i{v}"))
                    }
                    (Ft::I64, true) => {
                        let v = gen_f64(rng);
                        let t = if v == v.trunc() { format!("{v:.1}") } else { format!("{v}") };
                        // engine eq-node identity truncates to the field type
                        let k = if op == "==" { format!("i{}", v as i64) } else { format!("f{v}") };
                        (t, k)
                    }
                    (Ft::F64, false) => {
                        let v = gen_f64(rng);
                        let t = if v == v.trunc() { format!("{v:.1}") } else { format!("{v}") };
                        (t, format!("f{v}"))
                    }
                    (Ft::F64, true) => {
                        let v = gen_i64(rng);
                        let k = if op == "==" { format!("f{}", v as f64) } else { format!("i{v}") };
                        (format!("{v}"), k)
                    }
                    _ => unreachable!(),
                };
                (
                    format!("{fname} {op} {text_lit}"),
                    format!("{fname} {op} {key_lit}"),
                )
            }
        }
        Ft::Bool => {
            if rng.chance(10) {
                let c = format!("{fname}{}", gen_in_list(rng, Ft::Bool));
                let k = c.clone();
                (c, k)
            } else {
                let op = *rng.pick(OPS_EQ);
                let lit = lit_drl(rng, Ft::Bool, false);
                let c = format!("{fname} {op} {lit}");
                let k = c.clone();
                (c, k)
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
struct GenPattern {
    ti: usize,
    /// 0 = positive, 1 = not, 2 = exists. CE patterns never carry
    /// bindings or fact vars (D-031).
    ce: u8,
    fact_var: Option<String>,
    constraints: Vec<String>,
    /// Canonical constraint key parts (bindings excluded; var refs by
    /// source tuple position; eq literals field-type-normalized) — the
    /// structural node identity used by the D-035 sharing wall.
    keys: Vec<String>,
    /// (var name, field idx, field type) — usable by later patterns/RHS.
    bindings: Vec<(String, usize, Ft)>,
}

fn pattern_key(p: &GenPattern) -> String {
    format!("{}|{}|{}", p.ti, p.ce, p.keys.join(";"))
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
    // Subset wall (D-017, re-imposed by D-025): programs containing
    // update/modify keep every rule at <= 2 patterns. The widened
    // grammar's residual class — requeue placement among pending join
    // activations after repeated updates (xfail/) — is not yet pinned.
    // D-017 wall lifted (D-028): mutation and 3-pattern rules may mix
    // freely now that the requeue-placement class is closed.
    let allow_mutation = rng.chance(60);
    let max_extra_pat = 3;
    let mut drl = String::new();
    // D-035 sharing wall: in mutation programs, no two rules may have
    // structurally identical pattern PREFIXES (>= 2 patterns) — shared
    // beta nodes evaluate in the first sharer's window, which our
    // per-rule networks model only for insert-only programs.
    let mut seen_prefixes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ri in 0..nrules {
        let want_npat = 1 + if rng.chance(45) { rng.below(max_extra_pat) } else { 0 };
        let (mut pats, update_pos, delete_pos, guard_field) = 'gen: {
            for attempt in 0..9 {
                // last attempt falls back to a single pattern (never shares)
                let npat = if attempt == 8 { 1 } else { want_npat };
                let mut pats: Vec<GenPattern> = Vec::new();
                for pi in 0..npat {
                    // CE probability: rare in first position (InitialFact
                    // path), more common later (D-031/D-032).
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
                    pats.push(GenPattern {
                        ti: rng.below(ntypes),
                        ce,
                        fact_var: None,
                        constraints: Vec::new(),
                        keys: Vec::new(),
                        bindings: Vec::new(),
                    });
                }

                // Rule kind: update / delete / plain. Mutation targets must
                // be POSITIVE patterns (CE patterns cannot bind). Deletes
                // are gated on allow_mutation so insert-only programs stay
                // mutation-free for the sharing wall.
                let update_pos = {
                    let with_bool: Vec<usize> = pats
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| {
                            p.ce == 0
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
                        .filter(|(_, p)| p.ce == 0)
                        .map(|(i, _)| i)
                        .collect();
                    if allow_mutation
                        && update_pos.is_none()
                        && !positives.is_empty()
                        && rng.chance(20)
                    {
                        Some(*rng.pick(&positives))
                    } else {
                        None
                    }
                };

                // Constraints, bindings, join tests.
                for pi in 0..npat {
                    let ncmp = rng.below(3);
                    for _ in 0..ncmp {
                        let (fname, ft) = {
                            let fs = &types[pats[pi].ti].fields;
                            fs[rng.below(fs.len())].clone()
                        };
                        let (c, k) = gen_alpha_constraint(&mut rng, &fname, ft);
                        pats[pi].constraints.push(c);
                        pats[pi].keys.push(k);
                    }
                    // Join constraint against an earlier binding; the key
                    // references the source (tuple position, field) so
                    // binding names stay identity-irrelevant (ne_s5).
                    if pi > 0 && rng.chance(55) {
                        let mut earlier: Vec<(String, Ft, usize, usize)> = Vec::new();
                        let mut tpos = 0usize;
                        for p in pats[..pi].iter() {
                            if p.ce == 0 {
                                for (v, fi, ft) in &p.bindings {
                                    earlier.push((v.clone(), *ft, tpos, *fi));
                                }
                                tpos += 1;
                            }
                        }
                        if !earlier.is_empty() {
                            let fs = &types[pats[pi].ti].fields;
                            let fi = rng.below(fs.len());
                            let (fname, ft) = fs[fi].clone();
                            let compat: Vec<&(String, Ft, usize, usize)> = earlier
                                .iter()
                                .filter(|(_, bft, _, _)| ft.join_compatible(*bft))
                                .collect();
                            if !compat.is_empty() {
                                let (var, bft, btpos, bfi) = (*rng.pick(&compat)).clone();
                                let op = if ft == Ft::Bool || bft == Ft::Bool {
                                    *rng.pick(OPS_EQ)
                                } else {
                                    *rng.pick(OPS_ORD)
                                };
                                pats[pi].constraints.push(format!("{fname} {op} {var}"));
                                pats[pi].keys.push(format!("{fname} {op} @{btpos}.{bfi}"));
                            }
                        }
                    }
                    // Field bindings (positive patterns only — D-031);
                    // no key parts (bindings don't affect node identity).
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
                    pats[pos].keys.push(format!("{gname} == false"));
                    guard_field = Some((gfi, gname));
                }

                // Sharing wall check (ALL programs — D-035: the
                // preserved-vs-flipped sink is claimed dynamically by
                // the first sharer whose agenda item evaluates the
                // shared segment, so salience and linking asymmetries
                // reach beyond the statically-modeled class).
                if pats.len() > 1 {
                    let keys: Vec<String> = pats.iter().map(pattern_key).collect();
                    let collides = (1..pats.len())
                        .any(|j| seen_prefixes.contains(&keys[..=j].join("||")));
                    if collides && attempt < 8 {
                        continue;
                    }
                }
                break 'gen (pats, update_pos, delete_pos, guard_field);
            }
            unreachable!("attempt 8 always breaks");
        };
        if pats.len() > 1 {
            let keys: Vec<String> = pats.iter().map(pattern_key).collect();
            for j in 1..pats.len() {
                seen_prefixes.insert(keys[..=j].join("||"));
            }
        }

        // RHS actions.
        let mut actions: Vec<String> = Vec::new();
        let max_ti = pats.iter().map(|p| p.ti).max().unwrap();
        let can_insert = max_ti + 1 < ntypes && delete_pos.is_none();
        if can_insert {
            let nins = rng.below(3);
            for _ in 0..nins {
                let tgt_ti = max_ti + 1 + rng.below(ntypes - max_ti - 1);
                let mut args = Vec::new();
                let tgt_fields = types[tgt_ti].fields.clone();
                for (_, tft) in &tgt_fields {
                    args.push(gen_arg(&mut rng, &types, &mut pats, ri, *tft, false));
                }
                actions.push(format!("insert(new {}({}));", types[tgt_ti].name, args.join(", ")));
            }
        }
        if let Some(pos) = update_pos {
            let var = ensure_fact_var(&mut pats, ri, pos);
            let (gfi, gname) = guard_field.unwrap();
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
                let arg = gen_arg(&mut rng, &types, &mut pats, ri, ft, true);
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
        }

        // Render the rule.
        let salience = if rng.chance(35) {
            (rng.below(21) as i64) - 10
        } else {
            0
        };
        drl.push_str(&format!("rule \"R{ri}\"\n"));
        if salience != 0 {
            drl.push_str(&format!("salience {salience}\n"));
        }
        if rng.chance(10) {
            drl.push_str("no-loop\n");
        }
        drl.push_str("when\n");
        for p in &pats {
            let ce = match p.ce {
                1 => "not ",
                2 => "exists ",
                _ => "",
            };
            let head = match &p.fact_var {
                Some(v) => format!("{v} : "),
                None => String::new(),
            };
            drl.push_str(&format!(
                "    {ce}{head}{}({})\n",
                types[p.ti].name,
                p.constraints.join(", ")
            ));
        }
        drl.push_str("then\n");
        for a in &actions {
            drl.push_str(&format!("    {a}\n"));
        }
        drl.push_str("end\n");
    }

    // Facts: 0..6.
    let nfacts = rng.below(7);
    let mut facts = Vec::new();
    for _ in 0..nfacts {
        let ti = rng.below(ntypes);
        let t = &types[ti];
        let mut fields = serde_json::Map::new();
        for (fname, ft) in &t.fields {
            fields.insert(fname.clone(), lit_json(&mut rng, *ft));
        }
        facts.push(json!({"type": t.name, "fields": fields}));
    }

    let types_json: Vec<J> = types
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

    let scenario = json!({
        "name": name,
        "types": types_json,
        "facts": facts,
        "drl": drl,
    });
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
