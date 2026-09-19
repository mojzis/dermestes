//! `[tool.dermestes]` from `./pyproject.toml`: the constitution's five keys and
//! nothing else. An unknown key is an error, so a typo never silently widens
//! or narrows what gets reported.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Check ids this build knows. `disable` may only name these.
const CHECK_IDS: &[&str] = &["one-impl", "const-param"];

/// The parsed config, with defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Globs for files never indexed.
    pub exclude: Vec<String>,
    /// Globs for files counted as tests, never as targets.
    pub test_paths: Vec<String>,
    /// Decorators that do not exempt a function from the caller-counting
    /// checks (`functools.cache`); `one-impl` does not read it.
    pub ignore_decorators: Vec<String>,
    /// Globs treated as public API: nothing defined there is flagged.
    pub public: Vec<String>,
    /// Check ids to skip.
    pub disable: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exclude: Vec::new(),
            test_paths: ["tests/**", "test_*.py", "*_test.py", "conftest.py"]
                .map(str::to_owned)
                .to_vec(),
            ignore_decorators: Vec::new(),
            public: Vec::new(),
            disable: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct Pyproject {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    dermestes: Option<Config>,
}

impl Config {
    /// Load from `root/pyproject.toml`. A missing file or table means defaults;
    /// an unreadable file or a bad table is an error.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("pyproject.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("invalid {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        let pyproject: Pyproject = toml::from_str(text)?;
        let config = pyproject.tool.and_then(|tool| tool.dermestes).unwrap_or_default();
        if let Some(unknown) = config.disable.iter().find(|id| !CHECK_IDS.contains(&id.as_str())) {
            bail!("`disable` names unknown check id `{unknown}`");
        }
        Ok(config)
    }

    /// Whether the check with this id runs.
    pub fn enabled(&self, id: &str) -> bool {
        !self.disable.iter().any(|disabled| disabled == id)
    }
}

/// Whether `relative` (a `/`-separated path from the root) matches any glob.
///
/// A pattern also matches at any depth, the way `.gitignore` patterns do:
/// `test_*.py` covers `pkg/test_x.py`, `tests/**` covers `svc/tests/a.py`.
pub fn matches_any(relative: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        glob_match::glob_match(pattern, relative)
            || glob_match::glob_match(&format!("**/{pattern}"), relative)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_table_means_defaults() {
        let config = Config::parse("[project]\nname = \"x\"\n").expect("parses");
        assert_eq!(config, Config::default(), "no table, no changes");
    }

    #[test]
    fn keys_are_kebab_case_and_override_defaults() {
        let config = Config::parse("[tool.dermestes]\ntest-paths = [\"spec/**\"]\n").expect("ok");
        assert_eq!(config.test_paths, vec!["spec/**"], "override replaces the default list");
    }

    #[test]
    fn unknown_key_is_an_error() {
        let err = Config::parse("[tool.dermestes]\nverbose = true\n").expect_err("rejected");
        assert!(format!("{err:#}").contains("verbose"), "names the key: {err:#}");
    }

    #[test]
    fn disable_rejects_unknown_ids() {
        assert!(Config::parse("[tool.dermestes]\ndisable = [\"one-imp\"]\n").is_err(), "typo");
        let config = Config::parse("[tool.dermestes]\ndisable = [\"one-impl\"]\n").expect("ok");
        assert!(!config.enabled("one-impl"), "disabled");
    }

    #[test]
    fn globs_match_at_any_depth() {
        let tests = Config::default().test_paths;
        assert!(matches_any("test_a.py", &tests), "root-level test file");
        assert!(matches_any("pkg/test_a.py", &tests), "nested test file");
        assert!(matches_any("tests/unit/a.py", &tests), "tests dir");
        assert!(matches_any("svc/tests/a.py", &tests), "nested tests dir");
        assert!(matches_any("pkg/conftest.py", &tests), "conftest");
        assert!(!matches_any("pkg/testing.py", &tests), "not a test");
        assert!(!matches_any("contests/a.py", &tests), "no partial dir match");
    }
}
