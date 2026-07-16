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

mod canon;
mod gen;
mod oracle;
mod runner;
mod ser;

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
    let stdout = std::io::stdout();
    for path in paths {
        let t0 = std::time::Instant::now();
        // D-267: serialize the OK path directly from the engine-shaped
        // parts — no intermediate Value tree. Byte-identical to the old
        // json! assembly (see ser.rs); errors keep the cold json! path.
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let name = match runner::run_scenario_file_parts(path) {
            Ok((name, parts)) => {
                serde_json::to_writer(&mut buf, &ser::LineOk { name: &name, parts: &parts })
                    .expect("result serialization");
                name
            }
            Err((name, e)) => {
                let l = serde_json::json!({"scenario": name, "error": e});
                serde_json::to_writer(&mut buf, &l).expect("result serialization");
                name
            }
        };
        buf.push(b'\n');
        use std::io::Write;
        stdout.lock().write_all(&buf).expect("stdout write");
        if timed {
            eprintln!("TIME {name} {:.3}", t0.elapsed().as_secs_f64() * 1e3);
        }
    }
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
            // fire limit is agreement, not divergence.
            if e.contains("fire limit") && oe.contains("fire limit") {
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
