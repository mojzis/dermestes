//! End-to-end tests of the binary's argument handling and exit codes.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn dermestes(dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dermestes"));
    cmd.current_dir(dir);
    cmd
}

#[test]
fn help_names_the_contract_flags_and_guide() {
    let tmp = TempDir::new().expect("temp dir");
    dermestes(tmp.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--base"))
        .stdout(contains("--all"))
        .stdout(contains("--format"))
        .stdout(contains("guide"));
}

/// `guide` is what an agent runs before anything is set up: no repository,
/// no pyproject. It must still print, and exit 0.
#[test]
fn guide_prints_setup_with_no_repository() {
    let tmp = TempDir::new().expect("temp dir");
    let output = dermestes(tmp.path()).arg("guide").output().expect("runs");
    assert_eq!(output.status.code(), Some(0), "guide never fails");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("# dermestes guide: not configured here -> setup\n\n# Setup dermestes"),
        "got: {stdout}"
    );
    assert!(output.stderr.is_empty(), "the guide is output, not logging");
}

#[test]
fn guide_switches_to_triage_once_configured() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::write(tmp.path().join("pyproject.toml"), "[tool.dermestes]\n").expect("write");
    dermestes(tmp.path()).arg("guide").assert().success().stdout(contains("-> triage\n\n# Triage"));
}

#[test]
fn a_usage_error_exits_2() {
    let tmp = TempDir::new().expect("temp dir");
    dermestes(tmp.path()).args(["--all", "--base", "main"]).assert().code(2);
}
