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

const ONE_IMPL: &str = "from abc import ABC, abstractmethod\n\n\nclass Base(ABC):\n    @abstractmethod\n    def run(self): ...\n\n\nclass Only(Base):\n    def run(self):\n        return 1\n";

#[test]
fn an_unparseable_file_is_skipped_with_a_note() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::write(tmp.path().join("broken.py"), "class (:\n").expect("write");
    std::fs::write(tmp.path().join("fine.py"), ONE_IMPL).expect("write");
    let output = dermestes(tmp.path()).arg("--all").output().expect("runs");
    assert_eq!(output.status.code(), Some(1), "the rest of the run still reports");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "one line: {stderr}");
    assert!(stderr.contains("broken.py"), "names the file: {stderr}");
}

#[test]
fn an_unknown_config_key_exits_2() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::write(tmp.path().join("pyproject.toml"), "[tool.dermestes]\nverbose = true\n")
        .expect("write");
    dermestes(tmp.path()).arg("--all").assert().code(2).stderr(contains("verbose"));
}

#[test]
fn json_carries_the_same_finding() {
    let tmp = TempDir::new().expect("temp dir");
    std::fs::write(tmp.path().join("m.py"), ONE_IMPL).expect("write");
    let output = dermestes(tmp.path()).args(["--all", "--format", "json"]).output().expect("runs");
    assert_eq!(output.status.code(), Some(1), "findings exit 1");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let finding = &json[0];
    assert_eq!(finding["check"], "one-impl", "check id");
    assert_eq!(finding["path"], "m.py", "path");
    assert_eq!(finding["line"], 4, "line");
    assert_eq!(finding["name"], "Base", "name");
    assert_eq!(finding["kind"], "ABC", "kind");
    assert_eq!(finding["impl"]["name"], "Only", "impl");
    assert_eq!(finding["suggest"], "inline into Only, delete Base", "suggest");
}

#[test]
fn diff_mode_outside_a_git_repository_exits_2() {
    let tmp = TempDir::new().expect("temp dir");
    dermestes(tmp.path()).assert().code(2).stderr(contains("git diff"));
}
