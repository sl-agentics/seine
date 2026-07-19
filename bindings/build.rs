// Stamp the certification data into the build (D-214): corpus counts
// use the same directory globs as the repo's `make diff` gate; wheels
// built outside the source tree stamp zeros/"unknown".
use std::path::Path;
use std::process::Command;

fn count_json(root: &Path, dirs: &[&str]) -> usize {
    fn walk(p: &Path, n: &mut usize) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    walk(&path, n);
                } else if path.extension().is_some_and(|x| x == "json") {
                    *n += 1;
                }
            }
        }
    }
    let mut n = 0;
    for d in dirs {
        walk(&root.join(d), &mut n);
    }
    n
}

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo = Path::new(&manifest).join("..");
    let sc = repo.join("scenarios");
    let baseline = count_json(&sc, &["baseline"]);
    let probes = count_json(
        &sc,
        &["probes", "phase0", "phase1", "phase2", "demo", "failures"],
    );
    let regressions = count_json(&sc, &["regressions"]);
    let xfail = count_json(&sc, &["xfail"]);
    // D-329: shipped-artifact identity. CI wheel builds run inside
    // manylinux containers where the git call fails (no git binary /
    // dubious-ownership refusal on the mounted workspace) — the
    // workflow writes the short sha to .seine-commit at the repo root
    // before building. Priority: explicit SEINE_BUILD_COMMIT env (a
    // manual escape hatch) > the workflow's file > local git. A build
    // from an sdist (no git, no file) honestly stamps "unknown".
    let commit = std::env::var("SEINE_BUILD_COMMIT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::fs::read_to_string(repo.join(".seine-commit"))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .current_dir(&repo)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=SEINE_CORPUS_BASELINE={baseline}");
    println!("cargo:rustc-env=SEINE_CORPUS_PROBES={probes}");
    println!("cargo:rustc-env=SEINE_CORPUS_REGRESSIONS={regressions}");
    println!("cargo:rustc-env=SEINE_CORPUS_XFAIL={xfail}");
    println!("cargo:rustc-env=SEINE_GIT_COMMIT={commit}");
    println!("cargo:rerun-if-env-changed=SEINE_BUILD_COMMIT");
    println!("cargo:rerun-if-changed=../.seine-commit");
    println!("cargo:rerun-if-changed=../scenarios");
}
