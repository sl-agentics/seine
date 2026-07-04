//! Property-based scenario generator for the Phase 1+2 grammar.
//!
//! Every generated program is IN-SUBSET by construction and guaranteed to
//! terminate (D-010, D-013):
//! - inserts: a rule may only insert types with index strictly greater than
//!   every pattern's type index (chains strictly climb the type order);
//! - updates: guard-monotone — the updated pattern requires some bool field
//!   `g == false` and the RHS sets it true before update(); bool setters
//!   ONLY ever write true, so every bool field is monotone and each update
//!   rule fires at most once per fact per guarded position;
//! - bare update() (all-fields mask, non-terminating per j21) is never
//!   generated.
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
    fact_var: Option<String>,
    constraints: Vec<String>,
    /// (var name, field idx, field type) — usable by later patterns/RHS.
    bindings: Vec<(String, usize, Ft)>,
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
    let allow_mutation = rng.chance(60);
    let max_extra_pat = if allow_mutation { 2 } else { 3 };
    let mut drl = String::new();
    for ri in 0..nrules {
        let npat = 1 + if rng.chance(45) { rng.below(max_extra_pat) } else { 0 };
        let mut pats: Vec<GenPattern> = Vec::new();
        for pi in 0..npat {
            pats.push(GenPattern {
                ti: rng.below(ntypes),
                fact_var: None,
                constraints: Vec::new(),
                bindings: Vec::new(),
            });
            let _ = pi;
        }

        // Rule kind: update / delete / plain.
        let update_pos = {
            let with_bool: Vec<usize> = pats
                .iter()
                .enumerate()
                .filter(|(_, p)| types[p.ti].fields.iter().any(|(_, ft)| *ft == Ft::Bool))
                .map(|(i, _)| i)
                .collect();
            if allow_mutation && !with_bool.is_empty() && rng.chance(30) {
                Some(*rng.pick(&with_bool))
            } else {
                None
            }
        };
        let delete_pos = if update_pos.is_none() && rng.chance(20) {
            Some(rng.below(npat))
        } else {
            None
        };

        // Constraints, bindings, join tests.
        for pi in 0..npat {
            let ncmp = rng.below(3);
            for _ in 0..ncmp {
                let (fname, ft) = {
                    let fs = &types[pats[pi].ti].fields;
                    fs[rng.below(fs.len())].clone()
                };
                let op = match ft {
                    Ft::Bool => *rng.pick(OPS_EQ),
                    _ => *rng.pick(OPS_ORD),
                };
                let lit_ft = match ft {
                    Ft::I64 if rng.chance(20) => Ft::F64,
                    Ft::F64 if rng.chance(20) => Ft::I64,
                    other => other,
                };
                let lit = lit_drl(&mut rng, lit_ft, false);
                pats[pi].constraints.push(format!("{fname} {op} {lit}"));
            }
            // Join constraint against an earlier binding.
            if pi > 0 && rng.chance(55) {
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
            // Field bindings.
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
            let head = match &p.fact_var {
                Some(v) => format!("{v} : "),
                None => String::new(),
            };
            drl.push_str(&format!(
                "    {head}{}({})\n",
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
        // Try a getter.
        let mut cands: Vec<(usize, usize)> = Vec::new(); // (pattern pos, field idx)
        for (pi, p) in pats.iter().enumerate() {
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
