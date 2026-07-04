//! Property-based scenario generator for the Phase 1 grammar
//! (single-pattern rules only).
//!
//! Every generated program is IN-SUBSET by construction and guaranteed to
//! terminate: types are numbered T0..Tn and a rule whose pattern matches Ti
//! may only insert Tj with j > i, so insertion chains strictly climb the
//! type order (max depth = number of types).
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
    // Multiples of 0.5 in [-3, 6], integral values included on purpose to
    // stress the i64/f64 boundary. No NaN/inf (not expressible in JSON).
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

fn lit_drl(rng: &mut Rng, ft: Ft) -> String {
    match ft {
        Ft::I64 => format!("{}", gen_i64(rng)),
        Ft::F64 => {
            let v = gen_f64(rng);
            // Always keep a decimal point so it lexes as a float literal.
            if v == v.trunc() {
                format!("{v:.1}")
            } else {
                format!("{v}")
            }
        }
        Ft::Str => format!("{:?}", rng.pick(STR_POOL)),
        Ft::Bool => format!("{}", rng.chance(50)),
    }
}

fn getter(name: &str, ft: Ft) -> String {
    let mut cs = name.chars();
    let head = cs.next().unwrap().to_ascii_uppercase();
    let rest: String = cs.collect();
    // Boolean fields on Drools declared types only generate isX() (D-009).
    if ft == Ft::Bool {
        format!("is{head}{rest}()")
    } else {
        format!("get{head}{rest}()")
    }
}

/// Generate one scenario as (name, JSON).
pub fn gen_scenario(seed: u64, case: u64) -> (String, J) {
    let mut rng = Rng(seed ^ case.wrapping_mul(0xA24BAED4963EE407));
    // burn a few to decorrelate nearby cases
    for _ in 0..4 {
        rng.next();
    }
    let name = format!("fz_{seed}_{case}");

    // Types
    let ntypes = 2 + rng.below(3); // 2..4
    let mut types = Vec::new();
    for ti in 0..ntypes {
        let nfields = 1 + rng.below(3); // 1..3
        let fields = (0..nfields)
            .map(|fi| {
                let ft = *rng.pick(&[Ft::I64, Ft::I64, Ft::F64, Ft::Str, Ft::Bool]);
                (format!("f{fi}"), ft)
            })
            .collect();
        types.push(TypeDef { name: format!("T{ti}"), fields });
    }

    // Rules
    let nrules = 1 + rng.below(6); // 1..6
    let mut drl = String::new();
    for ri in 0..nrules {
        let pat_ti = rng.below(ntypes);
        let pat = &types[pat_ti];
        let salience = if rng.chance(35) {
            (rng.below(21) as i64) - 10
        } else {
            0
        };
        let no_loop = rng.chance(10);

        // Constraints: 0..3 field tests, plus bindings collected for RHS use.
        let mut constraints: Vec<String> = Vec::new();
        let mut bindings: Vec<(String, usize)> = Vec::new(); // (var, field idx)
        let ncmp = rng.below(4);
        for _ in 0..ncmp {
            let (fi, (fname, ft)) = {
                let fi = rng.below(pat.fields.len());
                (fi, pat.fields[fi].clone())
            };
            let _ = fi;
            let op = match ft {
                Ft::Bool => *rng.pick(OPS_EQ),
                _ => *rng.pick(OPS_ORD),
            };
            // Cross numeric literal types occasionally (pinned by pr10).
            let lit_ft = match ft {
                Ft::I64 if rng.chance(20) => Ft::F64,
                Ft::F64 if rng.chance(20) => Ft::I64,
                other => other,
            };
            constraints.push(format!("{fname} {op} {}", lit_drl(&mut rng, lit_ft)));
        }
        let nbind = rng.below(3);
        for bi in 0..nbind {
            let fi = rng.below(pat.fields.len());
            let var = format!("$b{ri}_{bi}");
            constraints.push(format!("{var} : {}", pat.fields[fi].0));
            bindings.push((var, fi));
        }

        // RHS: 0..2 inserts into strictly-later types.
        let mut actions: Vec<String> = Vec::new();
        let can_insert = pat_ti + 1 < ntypes;
        let pat_var = format!("$p{ri}");
        let mut used_pat_var = false;
        if can_insert {
            let nins = rng.below(3);
            for _ in 0..nins {
                let tgt_ti = pat_ti + 1 + rng.below(ntypes - pat_ti - 1);
                let tgt = &types[tgt_ti];
                let mut args = Vec::new();
                for (_, tft) in &tgt.fields {
                    // arg sources: literal / field binding / getter
                    let mut candidates: Vec<usize> = pat
                        .fields
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, ft))| {
                            ft == tft || (*ft == Ft::I64 && *tft == Ft::F64)
                        })
                        .map(|(i, _)| i)
                        .collect();
                    let bind_candidates: Vec<&(String, usize)> = bindings
                        .iter()
                        .filter(|(_, fi)| {
                            let ft = pat.fields[*fi].1;
                            ft == *tft || (ft == Ft::I64 && *tft == Ft::F64)
                        })
                        .collect();
                    let choice = rng.below(100);
                    if choice < 40 || (candidates.is_empty() && bind_candidates.is_empty()) {
                        args.push(lit_drl(&mut rng, *tft));
                    } else if choice < 70 && !bind_candidates.is_empty() {
                        let (var, _) = rng.pick(&bind_candidates);
                        args.push(var.clone());
                    } else if !candidates.is_empty() {
                        let fi = candidates.remove(rng.below(candidates.len()));
                        let (fname, ft) = &pat.fields[fi];
                        args.push(format!("{pat_var}.{}", getter(fname, *ft)));
                        used_pat_var = true;
                    } else {
                        args.push(lit_drl(&mut rng, *tft));
                    }
                }
                actions.push(format!("insert(new {}({}));", tgt.name, args.join(", ")));
            }
        }

        let binding_prefix = if used_pat_var || rng.chance(30) {
            format!("{pat_var} : ")
        } else {
            String::new()
        };
        drl.push_str(&format!("rule \"R{ri}\"\n"));
        if salience != 0 {
            drl.push_str(&format!("salience {salience}\n"));
        }
        if no_loop {
            drl.push_str("no-loop\n");
        }
        drl.push_str(&format!(
            "when\n    {binding_prefix}{}({})\nthen\n",
            pat.name,
            constraints.join(", ")
        ));
        for a in &actions {
            drl.push_str(&format!("    {a}\n"));
        }
        drl.push_str("end\n");
    }

    // Facts: 0..8, values from the same small domains as literals.
    let nfacts = rng.below(9);
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
