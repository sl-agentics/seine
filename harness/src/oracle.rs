//! Drives the Java reference runner (real Drools, version pinned in
//! oracle/pom.xml) as a subprocess, batching all scenarios into one JVM.

use serde_json::Value as J;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub enum OracleEntry {
    Result(J),
    Error(String),
}

/// Raw NDJSON lines from the oracle.
pub fn run_oracle(paths: &[String]) -> Result<Vec<String>, String> {
    let root = repo_root()?;
    let cp_file = root.join("oracle/target/classpath.txt");
    let cp = std::fs::read_to_string(&cp_file).map_err(|e| {
        format!(
            "cannot read {} ({e}) — build the oracle first: make oracle",
            cp_file.display()
        )
    })?;
    let classpath = format!(
        "{}:{}",
        root.join("oracle/target/classes").display(),
        cp.trim()
    );
    let out = Command::new("java")
        // D-295: Drools' removeLogicalDependencies teardown is call-
        // recursive — the default thread stack SOEs ~[300,400] levels
        // deep (ub_teardown_500). 1g covers the fire-limit-maximal
        // ~100k chain with ~3x margin (≈2.6–3.3 KB/level measured).
        .arg("-Xss1g")
        .arg("-cp")
        .arg(&classpath)
        .arg("dev.seine.oracle.OracleRunner")
        .args(paths)
        .output()
        .map_err(|e| format!("failed to spawn java: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "oracle exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Oracle results keyed by scenario name.
pub fn run_oracle_map(paths: &[String]) -> Result<HashMap<String, OracleEntry>, String> {
    let mut map = HashMap::new();
    for line in run_oracle(paths)? {
        let v: J =
            serde_json::from_str(&line).map_err(|e| format!("bad oracle NDJSON line: {e}: {line}"))?;
        let name = v
            .get("scenario")
            .and_then(J::as_str)
            .ok_or_else(|| format!("oracle line missing 'scenario': {line}"))?
            .to_string();
        let entry = if let Some(err) = v.get("error").and_then(J::as_str) {
            OracleEntry::Error(err.to_string())
        } else if let Some(result) = v.get("result") {
            OracleEntry::Result(result.clone())
        } else {
            return Err(format!("oracle line has neither result nor error: {line}"));
        };
        map.insert(name, entry);
    }
    Ok(map)
}

/// Find the repo root (directory containing `oracle/`) from cwd upward, so
/// the harness works from the workspace root or any subdirectory.
fn repo_root() -> Result<PathBuf, String> {
    let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        if dir.join("oracle/pom.xml").is_file() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => {
                return Err("could not locate repo root (oracle/pom.xml) from cwd".into());
            }
        }
    }
}

#[allow(dead_code)]
fn _assert_path_types(_: &Path) {}
