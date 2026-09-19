//! The only subprocess dermestes spawns: `git diff --unified=0`.
//!
//! From the diff and the working tree, every changed file is rebuilt as it was
//! at the base, so the checks can run on both sides and report what is new.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// One file's changes. A side is `None` when the file does not exist there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileDiff {
    old: Option<String>,
    new: Option<String>,
    hunks: Vec<Hunk>,
}

/// With `--unified=0` a hunk is a run of removed lines replaced by `new_count`
/// added lines starting at `new_start` (1-based; after that line when the
/// count is zero).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hunk {
    new_start: usize,
    new_count: usize,
    removed: Vec<String>,
}

/// Each changed file's content at `base`, keyed by its `/`-separated path
/// relative to `root`; `None` when the file did not exist there. Files the
/// diff does not mention are the same on both sides.
pub fn base_sources(root: &Path, base: &str) -> Result<HashMap<String, Option<String>>> {
    let output = Command::new("git")
        .args(["diff", "--unified=0", "--no-color", "--no-ext-diff", "--no-renames"])
        .args(["--relative", base, "--"])
        .current_dir(root)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        bail!("git diff {base} failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let mut sources = HashMap::new();
    for file in parse(&String::from_utf8_lossy(&output.stdout)) {
        let head = match &file.new {
            Some(path) => std::fs::read_to_string(root.join(path))
                .with_context(|| format!("cannot read {path}"))?,
            None => String::new(),
        };
        let content = file.old.is_some().then(|| reverse(&head, &file.hunks));
        if let Some(path) = file.old.or(file.new) {
            sources.insert(path, content);
        }
    }
    Ok(sources)
}

/// Undo `hunks` on `head`, giving the base side.
fn reverse(head: &str, hunks: &[Hunk]) -> String {
    let lines: Vec<&str> = head.lines().collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut next = 0;
    for hunk in hunks {
        let start = if hunk.new_count == 0 { hunk.new_start } else { hunk.new_start - 1 };
        let start = start.min(lines.len());
        out.extend(lines.get(next..start).unwrap_or_default());
        out.extend(hunk.removed.iter().map(String::as_str));
        next = (start + hunk.new_count).min(lines.len());
    }
    out.extend(lines.get(next..).unwrap_or_default());
    let mut text = out.join("\n");
    text.push('\n');
    text
}

fn parse(diff: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    // Lines still owed by the current hunk: body lines can look like headers.
    let (mut old_left, mut new_left) = (0, 0);
    for line in diff.lines() {
        let Some(file) = files.last_mut() else {
            if line.starts_with("diff --git ") {
                files.push(FileDiff { old: None, new: None, hunks: Vec::new() });
            }
            continue;
        };
        if old_left > 0 && line.starts_with('-') {
            old_left -= 1;
            if let Some(hunk) = file.hunks.last_mut() {
                hunk.removed.push(line[1..].to_owned());
            }
        } else if new_left > 0 && line.starts_with('+') {
            new_left -= 1;
        } else if line.starts_with("diff --git ") {
            files.push(FileDiff { old: None, new: None, hunks: Vec::new() });
        } else if let Some(path) = line.strip_prefix("--- ") {
            file.old = side(path, "a/");
        } else if let Some(path) = line.strip_prefix("+++ ") {
            file.new = side(path, "b/");
        } else if let Some((old_count, hunk)) = line.strip_prefix("@@ ").and_then(hunk_header) {
            (old_left, new_left) = (old_count, hunk.new_count);
            file.hunks.push(hunk);
        }
    }
    files
}

fn side(path: &str, prefix: &str) -> Option<String> {
    path.strip_prefix(prefix).map(|path| path.trim_matches('"').to_owned())
}

/// `-a,b +c,d @@ ...` → `b`, and the hunk's new side `c` and `d`.
fn hunk_header(header: &str) -> Option<(usize, Hunk)> {
    let range = |sign: char| -> Option<(usize, usize)> {
        let range = header.split_whitespace().find_map(|part| part.strip_prefix(sign))?;
        match range.split_once(',') {
            Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
            None => Some((range.parse().ok()?, 1)),
        }
    };
    let (_, old_count) = range('-')?;
    let (new_start, new_count) = range('+')?;
    Some((old_count, Hunk { new_start, new_count, removed: Vec::new() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sides_and_hunks() {
        let diff = "diff --git a/a.py b/a.py\n--- a/a.py\n+++ b/a.py\n@@ -1,0 +2,3 @@ x\n+a\n+b\n+c\n@@ -9,2 +12 @@\n-old\n--- not a header\n+new\n\
                    diff --git a/gone.py b/gone.py\n--- a/gone.py\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-x\n-y\n\
                    diff --git a/n.py b/n.py\n--- /dev/null\n+++ b/n.py\n@@ -0,0 +1 @@\n+z\n";
        let files = parse(diff);
        assert_eq!(files.len(), 3, "three files");
        assert_eq!(files[0].hunks[1].removed, vec!["old", "-- not a header"], "removed lines kept");
        assert_eq!((files[1].old.as_deref(), files[1].new.as_deref()), (Some("gone.py"), None));
        assert_eq!((files[2].old.as_deref(), files[2].new.as_deref()), (None, Some("n.py")));
    }

    #[test]
    fn reverse_rebuilds_the_base() {
        let base = "l1\nl2\nl3\nl4\nl5\n";
        // base → head: insert "x" after l1, replace l3 with "y" and "z", delete l5.
        let head = "l1\nx\nl2\ny\nz\nl4\n";
        let hunks = [
            Hunk { new_start: 2, new_count: 1, removed: vec![] },
            Hunk { new_start: 4, new_count: 2, removed: vec!["l3".into()] },
            Hunk { new_start: 6, new_count: 0, removed: vec!["l5".into()] },
        ];
        assert_eq!(reverse(head, &hunks), base, "insert, replace and delete undone");
        let gone = [Hunk { new_start: 0, new_count: 0, removed: vec!["a".into(), "b".into()] }];
        assert_eq!(reverse("", &gone), "a\nb\n", "deleted file comes back whole");
    }
}
