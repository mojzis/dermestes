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

/// What the walk found.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    /// Every non-excluded `.py` file, sorted by relative path.
    pub files: Vec<SourcePath>,
    /// Directories holding a `pyproject.toml` (`""` for the root), each a
    /// project whose packages import from there: uv workspace members,
    /// feast's `sdk/python`.
    pub projects: Vec<String>,
    /// `[project.scripts]` and `[project.entry-points.*]` values of every
    /// `pyproject.toml` (`pkg.cli:main`): functions something outside the
    /// repository calls.
    pub entry_points: Vec<String>,
}

/// Walk `root`. Hidden directories (`.venv`, `.tox`) and gitignored files are skipped.
pub fn discover(root: &Path, exclude: &[String]) -> Result<Tree> {
    let mut tree = Tree::default();
    for entry in WalkBuilder::new(root).build() {
        let entry = entry.context("error walking directory")?;
        let path = entry.into_path();
        let name = path.file_name().unwrap_or_default();
        let is_python = path.extension().is_some_and(|ext| ext == "py");
        if !(is_python || name == "pyproject.toml") || !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if matches_any(&relative, exclude) {
            continue;
        }
        if is_python {
            tree.files.push(SourcePath { path, relative });
        } else {
            let dir = relative.strip_suffix("pyproject.toml").unwrap_or_default();
            tree.projects.push(dir.to_owned());
            // An unreadable one only loses exemptions; it is noted, like a bad `.py`.
            match std::fs::read_to_string(&path)
                .map_err(anyhow::Error::from)
                .and_then(|text| entry_points(&text).with_context(|| format!("invalid {relative}")))
            {
                Ok(found) => tree.entry_points.extend(found),
                Err(err) => eprintln!("dermestes: skipped entry points: {err:#}"),
            }
        }
    }
    tree.files.sort_by(|a, b| a.relative.cmp(&b.relative));
    tree.projects.sort();
    Ok(tree)
}

/// The values of `[project.scripts]`, `[project.gui-scripts]` and every
/// `[project.entry-points.<group>]` table.
fn entry_points(text: &str) -> Result<Vec<String>> {
    let pyproject: toml::Table = toml::from_str(text)?;
    let Some(project) = pyproject.get("project").and_then(toml::Value::as_table) else {
        return Ok(Vec::new());
    };
    let groups = ["scripts", "gui-scripts"].into_iter().filter_map(|key| project.get(key));
    let plugins = project
        .get("entry-points")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|groups| groups.values());
    Ok(groups
        .chain(plugins)
        .filter_map(toml::Value::as_table)
        .flat_map(|table| table.values())
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relatives(root: &Path, exclude: &[String]) -> Vec<String> {
        discover(root, exclude)
            .expect("walks")
            .files
            .into_iter()
            .map(|file| file.relative)
            .collect()
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
    fn records_project_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("members/api")).expect("mkdir");
        std::fs::write(dir.path().join("pyproject.toml"), "").expect("write");
        std::fs::write(dir.path().join("members/api/pyproject.toml"), "").expect("write");
        let tree = discover(dir.path(), &[]).expect("walks");
        assert_eq!(tree.projects, vec!["", "members/api/"], "root and member, as prefixes");
    }

    #[test]
    fn reads_scripts_and_entry_points() {
        let text = "[project]\nname = \"x\"\n[project.scripts]\nda = \"esl.cli:da\"\n\
                    [project.entry-points.\"x.plugins\"]\ncsv = \"x.csv:Plugin\"\n";
        let found = entry_points(text).expect("parses");
        assert_eq!(found, vec!["esl.cli:da", "x.csv:Plugin"], "both tables");
        assert!(entry_points("[tool.x]\n").expect("parses").is_empty(), "no [project]");
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
