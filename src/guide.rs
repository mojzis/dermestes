//! Agent-facing instructions for the three moments someone meets dermestes.
//!
//! The prose lives in `docs/src/guide/*.md` and is pulled in with
//! [`include_str!`], so the documentation site and the CLI serve the same
//! bytes. There is no guide text in this file, and there must never be: a
//! second copy is a copy that drifts.
//!
//! `guide` needs no repository and no `git`. Topic selection reads one file,
//! `./pyproject.toml`, and nothing else.

use std::path::Path;

/// Instructions for a repository dermestes is not configured in yet.
const SETUP: &str = include_str!("../docs/src/guide/setup.md");
/// Instructions for reading a finding and acting on it.
const TRIAGE: &str = include_str!("../docs/src/guide/triage.md");
/// Reference for the config keys and suppression.
const TUNE: &str = include_str!("../docs/src/guide/tune.md");

/// Which set of instructions to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Topic {
    /// dermestes is not configured in this repository yet.
    Setup,
    /// dermestes reported findings; what to do with them.
    Triage,
    /// Config keys and suppression.
    Tune,
}

impl Topic {
    /// The topic's name as written on the command line and in the header.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::Triage => "triage",
            Self::Tune => "tune",
        }
    }

    /// The guide text, byte-identical to the docs page it is included from.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Setup => SETUP,
            Self::Triage => TRIAGE,
            Self::Tune => TUNE,
        }
    }

    /// Every topic, taken from the `ValueEnum` derive so a new variant is
    /// covered by every content check without being listed by hand.
    pub fn all() -> impl Iterator<Item = Self> {
        <Self as clap::ValueEnum>::value_variants().iter().copied()
    }
}

/// How the printed topic was chosen, carried into the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// The user named the topic on the command line.
    Explicit,
    /// Derived from whether `./pyproject.toml` configures dermestes.
    Auto(bool),
}

/// Whether `dir/pyproject.toml` has a `[tool.dermestes]` table.
///
/// Unreadable or unparseable files are "not configured", not errors: a guide
/// that refuses to print has failed at its one job.
#[must_use]
pub fn is_configured(dir: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(dir.join("pyproject.toml")) else {
        return false;
    };
    let Ok(document) = toml::from_str::<toml::Table>(&contents) else {
        return false;
    };
    document.get("tool").and_then(|tool| tool.get("dermestes")).is_some()
}

/// The topic to print when the user named none. `tune` is a reference and
/// is never auto-selected.
#[must_use]
pub const fn auto_topic(configured: bool) -> Topic {
    if configured {
        Topic::Triage
    } else {
        Topic::Setup
    }
}

/// The complete guide output: a header line naming the topic and how it was
/// chosen, a blank line, then the docs page verbatim.
#[must_use]
pub fn render(topic: Topic, selection: Selection) -> String {
    let header = match selection {
        Selection::Explicit => format!("# dermestes guide: {}", topic.name()),
        Selection::Auto(false) => {
            format!("# dermestes guide: not configured here -> {}", topic.name())
        }
        Selection::Auto(true) => {
            format!("# dermestes guide: configured via [tool.dermestes] -> {}", topic.name())
        }
    };
    format!("{header}\n\n{}", topic.text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const MAX_LINES: usize = 60;

    #[test]
    fn every_page_fits_the_cap_and_ends_with_one_next_line() {
        for topic in Topic::all() {
            let text = topic.text();
            let lines = text.lines().count();
            assert!(lines <= MAX_LINES, "{} is {lines} lines; cut it", topic.name());
            let next = text.lines().filter(|l| l.starts_with("next: run `")).count();
            assert_eq!(next, 1, "{} needs exactly one `next: run` line", topic.name());
            assert!(
                text.trim_end().lines().last().is_some_and(|l| l.starts_with("next: run `")),
                "{} must end with its `next: run` line",
                topic.name()
            );
        }
    }

    #[test]
    fn a_tool_dermestes_table_means_configured() {
        let tmp = TempDir::new().expect("temp dir");
        assert!(!is_configured(tmp.path()), "no pyproject is not configured");

        std::fs::write(tmp.path().join("pyproject.toml"), "[tool.ruff]\n").expect("write");
        assert!(!is_configured(tmp.path()), "another tool's table is not ours");

        std::fs::write(tmp.path().join("pyproject.toml"), "[tool.dermestes]\n").expect("write");
        assert!(is_configured(tmp.path()), "an empty table still counts");

        std::fs::write(tmp.path().join("pyproject.toml"), "[tool.dermestes\n").expect("write");
        assert!(!is_configured(tmp.path()), "malformed is not configured, not an error");
    }

    #[test]
    fn auto_selection_never_picks_tune() {
        assert_eq!(auto_topic(false), Topic::Setup, "unconfigured starts at setup");
        assert_eq!(auto_topic(true), Topic::Triage, "configured means reading findings");
    }
}
