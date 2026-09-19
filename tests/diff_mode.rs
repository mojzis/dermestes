//! Diff mode against a real temporary git repository: the whole tree is
//! indexed on both sides, and only findings new at head are reported.

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

/// A linked worktree has a `.git` file, not a directory, and may live far
/// from the main checkout. Run from one of its subdirectories, the root is the
/// worktree's top and diff mode works as in a plain checkout.
#[test]
fn a_linked_worktree_elsewhere_is_its_own_root() {
    let main = repo_with_old_finding();
    let away = TempDir::new().expect("temp dir");
    let worktree = away.path().join("wt");
    let worktree_arg = worktree.to_str().expect("utf-8 temp path");
    git(main.path(), &["worktree", "add", "-q", worktree_arg]);
    std::fs::create_dir_all(worktree.join("pkg")).expect("mkdir");
    std::fs::write(worktree.join("pkg/new.py"), NEW).expect("write");
    git(&worktree, &["add", "pkg/new.py"]);

    let (code, stdout) = dermestes(&worktree.join("pkg"), &[]);
    assert_eq!(code, Some(1), "the staged finding is reported: {stdout}");
    assert!(stdout.starts_with("one-impl pkg/new.py:4 New "), "root-relative path: {stdout}");
    assert!(!stdout.contains("Old"), "pre-existing finding is not: {stdout}");
    let (_, all) = dermestes(&worktree.join("pkg"), &["--all"]);
    assert!(all.contains("old.py") && all.contains("pkg/new.py"), "whole worktree: {all}");
}

/// Deleting the second implementation creates a one-impl without touching the
/// ABC's or the remaining impl's lines: findings new at head are reported.
#[test]
fn a_deletion_that_creates_a_finding_is_reported() {
    let tmp = TempDir::new().expect("temp dir");
    git(tmp.path(), &["init", "-q"]);
    std::fs::write(tmp.path().join("old.py"), OLD).expect("write");
    std::fs::write(
        tmp.path().join("second.py"),
        "from old import Old\n\n\nclass Second(Old):\n    def run(self):\n        return 2\n",
    )
    .expect("write");
    git(tmp.path(), &["add", "."]);
    git(tmp.path(), &["commit", "-q", "-m", "two impls"]);
    assert_eq!(dermestes(tmp.path(), &[]).0, Some(0), "two impls: quiet");

    std::fs::remove_file(tmp.path().join("second.py")).expect("delete");
    let (code, stdout) = dermestes(tmp.path(), &[]);
    assert_eq!(code, Some(1), "the deletion made Old a one-impl: {stdout}");
    assert!(stdout.starts_with("one-impl old.py:4 Old "), "reported at head: {stdout}");

    git(tmp.path(), &["commit", "-q", "-am", "drop second"]);
    let (code, stdout) = dermestes(tmp.path(), &["--base", "HEAD~1"]);
    assert_eq!(code, Some(1), "also against a base ref: {stdout}");
}
