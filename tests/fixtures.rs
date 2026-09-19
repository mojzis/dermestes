//! Fixture harness: every case under `fixtures/<check>/{flag,no_flag}/<case>/`
//! is a small Python tree. `flag` cases must print exactly `expected.txt` and
//! exit 1; `no_flag` cases must print nothing and exit 0.

#![allow(clippy::expect_used, reason = "integration-test helpers treat setup failures as fatal")]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn cases(kind: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/one-impl").join(kind);
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("fixture directory exists")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no fixtures under {}", root.display());
    cases
}

fn run_all(case: &Path) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_dermestes"))
        .arg("--all")
        .current_dir(case)
        .output()
        .expect("binary runs");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn flag_cases_print_the_expected_findings() {
    let mut failures = Vec::new();
    for case in cases("flag") {
        let expected = std::fs::read_to_string(case.join("expected.txt")).expect("expected.txt");
        let (code, stdout, stderr) = run_all(&case);
        if code != Some(1) || stdout != expected {
            failures.push(format!(
                "{}: exit {code:?}\n--- expected\n{expected}--- got\n{stdout}--- stderr\n{stderr}",
                case.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn no_flag_cases_are_silent() {
    let mut failures = Vec::new();
    for case in cases("no_flag") {
        let (code, stdout, stderr) = run_all(&case);
        if code != Some(0) || !stdout.is_empty() {
            failures.push(format!(
                "{}: exit {code:?}\n--- got\n{stdout}--- stderr\n{stderr}",
                case.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
