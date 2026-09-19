//! The only subprocess dermestes spawns: `git diff --unified=0`, parsed into
//! the new-side line ranges each changed file gained.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Changed lines per `/`-separated path relative to the working directory.
/// Each range is inclusive, 1-based, on the new side of the diff.
pub type Changes = HashMap<String, Vec<(usize, usize)>>;

/// Lines the working tree (staged plus unstaged) changed against `base`.
pub fn changed_lines(root: &Path, base: &str) -> Result<Changes> {
    let output = Command::new("git")
        .args(["diff", "--unified=0", "--no-color", "--no-ext-diff", "--relative", base, "--"])
        .current_dir(root)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        bail!("git diff {base} failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(parse(&String::from_utf8_lossy(&output.stdout)))
}

/// Whether `line` of `path` falls inside a changed range.
pub fn touches(changes: &Changes, path: &str, line: usize) -> bool {
    changes
        .get(path)
        .is_some_and(|ranges| ranges.iter().any(|&(start, end)| (start..=end).contains(&line)))
}

fn parse(diff: &str) -> Changes {
    let mut changes = Changes::new();
    let mut current: Option<String> = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            current = path.strip_prefix("b/").map(|path| path.trim_matches('"').to_owned());
        } else if let (Some(hunk), Some(path)) = (line.strip_prefix("@@ "), &current) {
            if let Some(range) = new_range(hunk) {
                changes.entry(path.clone()).or_default().push(range);
            }
        }
    }
    changes
}

/// `-a,b +c,d @@ ...` → `(c, c + d - 1)`; a pure deletion (`d == 0`) adds nothing.
fn new_range(hunk: &str) -> Option<(usize, usize)> {
    let new = hunk.split_whitespace().find_map(|part| part.strip_prefix('+'))?;
    let (start, count) = match new.split_once(',') {
        Some((start, count)) => (start.parse().ok()?, count.parse::<usize>().ok()?),
        None => (new.parse().ok()?, 1),
    };
    (count > 0).then(|| (start, start + count - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_new_side_ranges() {
        let diff = "diff --git a/a.py b/a.py\n--- a/a.py\n+++ b/a.py\n@@ -1,0 +2,3 @@ x\n+a\n@@ -9 +12 @@\n@@ -20,2 +22,0 @@\ndiff --git a/gone.py b/gone.py\n--- a/gone.py\n+++ /dev/null\n@@ -1,4 +0,0 @@\n";
        let changes = parse(diff);
        assert_eq!(
            changes["a.py"],
            vec![(2, 4), (12, 12)],
            "additions only; deletions add nothing"
        );
        assert!(!changes.contains_key("gone.py"), "deleted file");
        assert!(touches(&changes, "a.py", 3) && !touches(&changes, "a.py", 5), "range check");
    }
}
