//! Find the Python files to index. Copied from biston's walker: gitignore-aware
//! via the `ignore` crate, then filtered by the `exclude` globs.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use crate::config::matches_any;

/// A discovered file: its path on disk and its `/`-separated path from the
/// root, which is what output, globs and git diffs all use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePath {
    pub path: PathBuf,
    pub relative: String,
}

/// Every non-excluded `.py` file under `root`, sorted by relative path.
/// Hidden directories (`.venv`, `.tox`) and gitignored files are skipped.
pub fn discover(root: &Path, exclude: &[String]) -> Result<Vec<SourcePath>> {
    let mut files = Vec::new();
    for entry in WalkBuilder::new(root).build() {
        let entry = entry.context("error walking directory")?;
        let path = entry.into_path();
        if path.extension().is_none_or(|ext| ext != "py") || !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if !matches_any(&relative, exclude) {
            files.push(SourcePath { path, relative });
        }
    }
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relatives(root: &Path, exclude: &[String]) -> Vec<String> {
        discover(root, exclude).expect("walks").into_iter().map(|file| file.relative).collect()
    }

    #[test]
    fn finds_python_files_sorted_and_excludes() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("pkg/migrations")).expect("mkdir");
        for file in ["z.py", "a.py", "notes.txt", "pkg/m.py", "pkg/migrations/0001.py"] {
            std::fs::write(dir.path().join(file), "").expect("write");
        }
        assert_eq!(
            relatives(dir.path(), &["migrations/**".to_owned()]),
            vec!["a.py", "pkg/m.py", "z.py"],
            "sorted, .py only, exclude applies at any depth"
        );
    }

    #[test]
    fn skips_hidden_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".venv/site")).expect("mkdir");
        std::fs::write(dir.path().join(".venv/site/lib.py"), "").expect("write");
        std::fs::write(dir.path().join("main.py"), "").expect("write");
        assert_eq!(relatives(dir.path(), &[]), vec!["main.py"], "virtualenvs are not source");
    }
}
