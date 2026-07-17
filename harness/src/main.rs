//! seine-harness: differential test driver.
//!
//! Subcommands:
//!   run <scenario.json>...     run scenarios through seine-engine, NDJSON out
//!   oracle <scenario.json>...  run scenarios through the Drools oracle, NDJSON out
//!   diff <scenario.json>...    run both, compare canonically, report; exit 1 on any divergence
//!
//! Canonical comparison rules (DECISIONS.md D-003):
//!   - final facts are a MULTISET (both engines emit them in arbitrary order)
//!   - the firing log is ORDER-SIGNIFICANT
//!   - matched facts within one firing are compared as a multiset
//!   - f64 equality is IEEE-754 bit equality; i64 exact

#[cfg(feature = "alloc_stats")]
mod alloc_stats;
mod canon;
mod gen;
mod oracle;
mod runner;
mod ser;

#[cfg(feature = "alloc_stats")]
#[global_allocator]
static COUNTING_ALLOC: alloc_stats::CountingAlloc = alloc_stats::CountingAlloc;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, paths) = match args.split_first() {
        Some((c, rest)) if !rest.is_empty() => (c.as_str(), rest.to_vec()),
        _ => {
            eprintln!(
                "usage: seine-harness <run|oracle|diff> <scenario.json>...\n       seine-harness fuzz <count> [seed]"
            );
            return ExitCode::from(2);
        }
    };
    #[cfg(feature = "prof")]
    let _prof = pprof::ProfilerGuardBuilder::default()
        .frequency(997)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .ok();
    #[cfg(feature = "prof")]
    let _flush = ProfDump(_prof);
    #[cfg(feature = "alloc_stats")]
    let _alloc_dump = alloc_stats::AllocDump;
    match cmd {
        "run" => cmd_run(&paths),
        "oracle" => cmd_oracle(&paths),
        "diff" => cmd_diff(&paths),
        "fuzz" => cmd_fuzz(&paths),
        "gen" => cmd_gen(&paths),
        other => {
            eprintln!("unknown subcommand {other:?}");
            ExitCode::from(2)
        }
    }
}

fn cmd_run(paths: &[String]) -> ExitCode {
    // SEINE_TIME=1: per-scenario wall time on stderr ("TIME <name> <ms>",
    // parse+build+run+serialize) — the oracle runner emits the same shape,
    // so tools/bench_oracle.py can compare like for like.
    let timed = std::env::var("SEINE_TIME").is_ok();
    use std::io::Write;
    // D-272: one buffered writer, lines streamed as they serialize —
    // a 2M-fact result line no longer exists as a whole Vec<u8> before
    // reaching stdout.
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for path in paths {
        let t0 = std::time::Instant::now();
        // D-267: serialize the OK path directly from the engine-shaped
        // parts — no intermediate Value tree. Byte-identical to the old
        // json! assembly (see ser.rs); errors keep the cold json! path.
        let name = match runner::run_scenario_file_parts(path) {
            Ok((name, parts)) => {
                serde_json::to_writer(&mut out, &ser::LineOk { name: &name, parts: &parts })
                    .expect("result serialization");
                name
            }
            Err((name, e)) => {
                let l = serde_json::json!({"scenario": name, "error": e});
                serde_json::to_writer(&mut out, &l).expect("result serialization");
                name
            }
        };
        out.write_all(b"\n").expect("stdout write");
        if timed {
            eprintln!("TIME {name} {:.3}", t0.elapsed().as_secs_f64() * 1e3);
        }
    }
    out.flush().expect("stdout flush");
    ExitCode::SUCCESS
}

fn cmd_oracle(paths: &[String]) -> ExitCode {
    match oracle::run_oracle(paths) {
        Ok(lines) => {
            for l in lines {
                println!("{l}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("oracle failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_diff(paths: &[String]) -> ExitCode {
    let oracle_results = match oracle::run_oracle_map(paths) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("oracle failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = 0usize;
    let mut passed = 0usize;
    for path in paths {
        let (name, engine_result) = match runner::run_scenario_file(path) {
            Ok((n, r)) => (n, Ok(r)),
            Err((n, e)) => (n, Err(e)),
        };
        let verdict = judge(&name, &engine_result, oracle_results.get(&name));
        match verdict {
            Ok(()) => {
                passed += 1;
                println!("PASS {name}");
            }
            Err(msgs) => {
                failed += 1;
                println!("FAIL {name}");
                for m in msgs {
                    println!("     {m}");
                }
                if let Some(tag) = mode1_residency_tag(path) {
                    println!("     {tag}");
                }
            }
        }
    }
    println!("---");
    println!("{passed} passed, {failed} failed, {} total", passed + failed);
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// D-290/D-291 mode-1 residency: a conservative static bound on how
/// often an LHS-division constraint could be evaluated. Above the
/// oracle's jit threshold (~20) the constraint may flip to jitted
/// java semantics MID-RUN, nondeterministically (the D-290 race:
/// 5000-fact prefix cliffs at 127/128/135; zero divisors start
/// throwing) — so a divergence on such a scenario is RACE-SUSPECT
/// and must be volume-triaged before it is read as an engine defect.
/// The certified corpus and the generator live far below the bound
/// (≤6 facts + small epochs); this tag exists so a future volume
/// scenario cannot masquerade as a clean red gate.
fn mode1_residency_tag(path: &str) -> Option<String> {
    use seine_engine::drl::{AExpr, Constraint};
    fn has_div(e: &AExpr) -> bool {
        match e {
            AExpr::Bin(op, a, b) => *op == '/' || has_div(a) || has_div(b),
            AExpr::Neg(a) => has_div(a),
            _ => false,
        }
    }
    let txt = std::fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&txt).ok()?;
    let drl = doc.get("drl")?.as_str()?;
    let rules = seine_engine::drl::parse_rules(drl).ok()?;
    let mut div_types: std::collections::HashSet<&str> = Default::default();
    for r in &rules {
        for p in &r.patterns {
            for c in &p.constraints {
                if let Constraint::ArithCmp { left, right, .. } = c {
                    if has_div(left) || has_div(right) {
                        div_types.insert(p.type_name.as_str());
                    }
                }
            }
        }
    }
    if div_types.is_empty() {
        return None;
    }
    let count_of = |t: &str| -> usize {
        let in_facts = |v: Option<&serde_json::Value>| {
            v.and_then(|f| f.as_array())
                .map(|a| {
                    a.iter()
                        .filter(|x| x.get("type").and_then(|v| v.as_str()) == Some(t))
                        .count()
                })
                .unwrap_or(0)
        };
        let base = in_facts(doc.get("facts"));
        let ep: usize = doc
            .get("epochs")
            .and_then(|e| e.as_array())
            .map(|es| {
                es.iter()
                    .map(|e| {
                        in_facts(e.get("facts"))
                            + e.get("actions")
                                .and_then(|a| a.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter(|x| {
                                            x.get("op").and_then(|v| v.as_str()) == Some("update")
                                        })
                                        .count()
                                })
                                .unwrap_or(0)
                    })
                    .sum()
            })
            .unwrap_or(0);
        base + ep
    };
    const RESIDENCY_BOUND: usize = 16; // conservative vs the oracle's ~20
    for t in div_types {
        let n = count_of(t);
        if n >= RESIDENCY_BOUND {
            return Some(format!(
                "MODE1-RESIDENCY EXCEEDED (D-290 jit-race suspect): type {t} reaches ~{n} \
                 evaluations of an LHS-division constraint (oracle jit threshold ~20; \
                 RHS-driven updates add more) — the oracle may have flipped to jitted \
                 java semantics mid-run; triage VOLUME before engine"
            ));
        }
    }
    None
}

/// `gen <count> [seed]`: print generated scenarios as NDJSON (grammar
/// inspection; the same stream fuzz consumes).
fn cmd_gen(paths: &[String]) -> ExitCode {
    let count: u64 = paths[0].parse().expect("gen <count> [seed]");
    let seed: u64 = paths.get(1).map(|s| s.parse().expect("seed")).unwrap_or(1);
    for case in 0..count {
        let (_, scenario) = gen::gen_scenario(seed, case);
        println!("{scenario}");
    }
    ExitCode::SUCCESS
}

fn cmd_fuzz(args: &[String]) -> ExitCode {
    let count: u64 = match args.first().map(|s| s.parse()) {
        Some(Ok(n)) => n,
        _ => {
            eprintln!("usage: seine-harness fuzz <count> [seed]");
            return ExitCode::from(2);
        }
    };
    let seed: u64 = args
        .get(1)
        .map(|s| s.parse().expect("seed must be a u64"))
        .unwrap_or(42);
    const BATCH: u64 = 250;
    const MAX_FAILURES: usize = 20;

    let fuzz_dir = std::path::PathBuf::from("target/fuzz");
    let fail_dir = std::path::PathBuf::from("scenarios/failures");
    let xfail_dir = std::path::PathBuf::from("scenarios/xfail");
    std::fs::create_dir_all(&fuzz_dir).expect("mkdir target/fuzz");

    let started = std::time::Instant::now();
    let mut failures = 0usize;
    let mut xfails = 0usize;
    let mut done = 0u64;
    let mut case = 0u64;
    while done < count {
        let n = BATCH.min(count - done);
        let mut paths = Vec::new();
        for _ in 0..n {
            let (name, scenario) = gen::gen_scenario(seed, case);
            case += 1;
            let path = fuzz_dir.join(format!("{name}.json"));
            std::fs::write(&path, serde_json::to_string_pretty(&scenario).unwrap())
                .expect("write fuzz scenario");
            paths.push(path.to_string_lossy().to_string());
        }
        let oracle_results = match oracle::run_oracle_map(&paths) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("oracle failed mid-fuzz: {e}");
                return ExitCode::FAILURE;
            }
        };
        for path in &paths {
            let (name, engine_result) = match runner::run_scenario_file(path) {
                Ok((n, r)) => (n, Ok(r)),
                Err((n, e)) => (n, Err(e)),
            };
            if let Err(msgs) = judge(&name, &engine_result, oracle_results.get(&name)) {
                // D-259: quarantine files carry either the bare fuzz name
                // (pre-D-255) or an xf_ prefix (the D-255 re-files) — the
                // suppression must match both, or a re-fuzzed known latent
                // lands in the GATED scenarios/failures/ (the D-255 CI trap).
                if xfail_dir.join(format!("{name}.json")).is_file()
                    || xfail_dir.join(format!("xf_{name}.json")).is_file()
                {
                    // documented-open divergence (D-042): counted apart
                    xfails += 1;
                    println!("XFAIL {name} (documented, scenarios/xfail/)");
                    continue;
                }
                failures += 1;
                std::fs::create_dir_all(&fail_dir).ok();
                std::fs::copy(path, fail_dir.join(format!("{name}.json"))).ok();
                println!("DIVERGENCE {name} (saved to scenarios/failures/)");
                for m in msgs {
                    println!("     {m}");
                }
                if let Some(tag) = mode1_residency_tag(path) {
                    println!("     {tag}");
                }
                if failures >= MAX_FAILURES {
                    println!("--- stopping early: {failures} divergences");
                    return ExitCode::FAILURE;
                }
            }
        }
        done += n;
        println!(
            "fuzz progress: {done}/{count} cases, {failures} divergences, {:.0}s elapsed",
            started.elapsed().as_secs_f64()
        );
    }
    println!(
        "--- fuzz complete: {count} cases, seed {seed}, {failures} divergences, {xfails} xfail, {:.0}s",
        started.elapsed().as_secs_f64()
    );
    if failures > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn judge(
    name: &str,
    engine_result: &Result<serde_json::Value, String>,
    oracle_entry: Option<&oracle::OracleEntry>,
) -> Result<(), Vec<String>> {
    let oracle_entry = oracle_entry
        .ok_or_else(|| vec![format!("oracle produced no output for scenario {name}")])?;
    match (engine_result, oracle_entry) {
        (Ok(er), oracle::OracleEntry::Result(or)) => canon::compare(er, or),
        (Err(e), oracle::OracleEntry::Result(_)) => {
            Err(vec![format!("engine errored but oracle succeeded: {e}")])
        }
        (Ok(_), oracle::OracleEntry::Error(oe)) => {
            Err(vec![format!("oracle errored but engine succeeded: {oe}")])
        }
        (Err(e), oracle::OracleEntry::Error(oe)) => {
            // Non-termination parity (D-013/j21): both engines hitting the
            // fire limit is agreement, not divergence. Same for a
            // consequence division by zero (D-283: computed RHS args) —
            // both sides throw Java's "/ by zero" shape; the message
            // wrappers differ (ConsequenceException vs EngineError).
            if e.contains("fire limit") && oe.contains("fire limit") {
                Ok(())
            } else if e.contains("/ by zero") && oe.contains("/ by zero") {
                Ok(())
            } else {
                Err(vec![format!(
                    "both sides errored (scenario likely out of subset): engine={e}; oracle={oe}"
                )])
            }
        }
    }
}

#[cfg(feature = "prof")]
struct ProfDump(Option<pprof::ProfilerGuard<'static>>);

#[cfg(feature = "prof")]
impl Drop for ProfDump {
    fn drop(&mut self) {
        if let Some(g) = self.0.take() {
            if let Ok(report) = g.report().build() {
                let path = std::env::var("SEINE_FLAME")
                    .unwrap_or_else(|_| "flame.svg".into());
                if let Ok(f) = std::fs::File::create(&path) {
                    let _ = report.flamegraph(f);
                    eprintln!("FLAMEGRAPH {path}");
                }
            }
        }
    }
}
