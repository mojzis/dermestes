//! Diff mode against a real temporary git repository: the whole tree is
//! indexed, but only findings whose definitions the diff touched are reported.

#![allow(clippy::expect_used, reason = "integration-test helpers treat setup failures as fatal")]

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const OLD: &str = "from abc import ABC, abstractmethod\n\n\nclass Old(ABC):\n    @abstractmethod\n    def run(self): ...\n\n\nclass OldImpl(Old):\n    def run(self):\n        return 1\n";
const NEW: &str = "from abc import ABC, abstractmethod\n\n\nclass New(ABC):\n    @abstractmethod\n    def run(self): ...\n\n\nclass NewImpl(New):\n    def run(self):\n        return 1\n";

/// A command with every inherited `GIT_*` variable removed. A hook's `GIT_DIR`
/// leaking in would point git at the outer repository instead of the temp one.
fn isolated(program: &str) -> Command {
    let mut command = Command::new(program);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
}

fn git(dir: &Path, args: &[&str]) {
    let status = isolated("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

fn dermestes(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let output = isolated(env!("CARGO_BIN_EXE_dermestes"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("binary runs");
    (output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned())
}

fn repo_with_old_finding() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    git(tmp.path(), &["init", "-q"]);
    std::fs::write(tmp.path().join("old.py"), OLD).expect("write");
    git(tmp.path(), &["add", "."]);
    git(tmp.path(), &["commit", "-q", "-m", "old"]);
    tmp
}

#[test]
fn only_what_the_working_tree_introduced_is_reported() {
    let tmp = repo_with_old_finding();
    assert_eq!(dermestes(tmp.path(), &[]), (Some(0), String::new()), "clean tree is quiet");

    std::fs::write(tmp.path().join("new.py"), NEW).expect("write");
    git(tmp.path(), &["add", "new.py"]);
    let (code, stdout) = dermestes(tmp.path(), &[]);
    assert_eq!(code, Some(1), "the new finding is reported: {stdout}");
    assert!(stdout.contains("new.py:4 New "), "new finding: {stdout}");
    assert!(!stdout.contains("Old"), "pre-existing finding is not: {stdout}");

    let (code, stdout) = dermestes(tmp.path(), &["--all"]);
    assert_eq!(code, Some(1), "--all reports both");
    assert!(stdout.contains("Old") && stdout.contains("New"), "both: {stdout}");
}

#[test]
fn base_moves_the_comparison_point() {
    let tmp = repo_with_old_finding();
    std::fs::write(tmp.path().join("new.py"), NEW).expect("write");
    git(tmp.path(), &["add", "."]);
    git(tmp.path(), &["commit", "-q", "-m", "new"]);
    assert_eq!(dermestes(tmp.path(), &[]).0, Some(0), "nothing changed against HEAD");
    let (code, stdout) = dermestes(tmp.path(), &["--base", "HEAD~1"]);
    assert_eq!(code, Some(1), "new since HEAD~1: {stdout}");
    assert!(stdout.contains("New") && !stdout.contains("Old"), "only the new one: {stdout}");
}
