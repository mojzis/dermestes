//! Command-line surface. `main.rs` parses [`Cli`] and calls [`Cli::run`].
//!
//! The flags are exactly the constitution's interface contract; nothing is
//! added until a real repository needs it.

use std::collections::{HashMap, HashSet};
use std::io::Write;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use serde::Serialize;

use crate::config::Config;
use crate::index::{Index, Parsed};
use crate::{const_param, discovery, git, guide, one_impl, pass_through};

const ABOUT: &str = "Finds Python abstractions that have not earned their keep.";

const LONG_ABOUT: &str = "\
dermestes reports abstractions with a single user — an ABC or Protocol with one \
implementation, a function with one caller, a parameter nobody varies — as \
deletion candidates, each with its evidence and a concrete simplification.

By default it reports only what the working tree's changes (staged plus \
unstaged, against HEAD) introduced. The whole repository is always indexed, \
because counting users needs it.";

const AFTER_LONG_HELP: &str = "\
`dermestes guide` prints agent-facing instructions: `setup` until pyproject.toml \
has a [tool.dermestes] table, `triage` after; `tune` only when asked for.

Exit codes:

  0  no findings (and no output)
  1  findings reported
  2  usage or internal error";

/// Top-level arguments. With no subcommand, dermestes runs its checks.
#[derive(Parser, Debug)]
#[command(name = "dermestes", version, about = ABOUT, long_about = LONG_ABOUT)]
#[command(after_long_help = AFTER_LONG_HELP)]
pub struct Cli {
    /// Report findings introduced since <REF> instead of since HEAD.
    #[arg(long, value_name = "REF", conflicts_with = "all")]
    pub base: Option<String>,

    /// Report every finding in the repository, not only those a diff introduced.
    #[arg(long)]
    pub all: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// How findings are written to stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// One block per finding, sorted by path then line.
    Text,
    /// The same content, machine-readable.
    Json,
}

/// Subcommands. Running checks is the default and has none.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print agent-facing instructions.
    Guide {
        /// Which instructions to print; detected from ./pyproject.toml if omitted.
        #[arg(value_enum)]
        topic: Option<guide::Topic>,
    },
}

/// A finding of any check, serialised as that check's own fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Finding {
    OneImpl(one_impl::Finding),
    ConstParam(const_param::Finding),
    PassThrough(pass_through::Finding),
}

impl Finding {
    fn path_line(&self) -> (&str, usize) {
        match self {
            Self::OneImpl(finding) => (&finding.site.path, finding.site.line),
            Self::ConstParam(finding) => (&finding.path, finding.line),
            Self::PassThrough(finding) => (&finding.path, finding.line),
        }
    }

    /// What identifies a finding across base and head: line numbers shift.
    fn key(&self) -> (&'static str, &str, &str, &str) {
        match self {
            Self::OneImpl(f) => (f.check, &f.site.path, &f.site.name, ""),
            Self::ConstParam(f) => (f.check, &f.path, &f.name, &f.param),
            Self::PassThrough(f) => (f.check, &f.path, &f.name, ""),
        }
    }
}

/// Every enabled check over `index`, sorted by path, then line.
fn run_checks(index: &Index, config: &Config, entry_points: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if config.enabled(one_impl::ID) {
        findings.extend(one_impl::run(index, config).into_iter().map(Finding::OneImpl));
    }
    if config.enabled(const_param::ID) {
        findings.extend(const_param::run(index, config).into_iter().map(Finding::ConstParam));
    }
    if config.enabled(pass_through::ID) {
        let found = pass_through::run(index, config, entry_points);
        findings.extend(found.into_iter().map(Finding::PassThrough));
    }
    findings.sort_by(|a, b| a.path_line().cmp(&b.path_line()));
    findings
}

/// What a completed run found. Errors are the `Err` side of [`Cli::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to report; nothing was printed.
    Clean,
    /// At least one finding was printed.
    Findings,
}

impl Cli {
    /// Run the parsed command, writing its output to `out`.
    pub fn run(self, out: &mut impl Write) -> Result<Outcome> {
        if let Some(Command::Guide { topic }) = self.command {
            write!(out, "{}", guide_output(topic))?;
            return Ok(Outcome::Clean);
        }
        let cwd = std::env::current_dir().context("cannot read the current directory")?;
        let findings = self.findings(&project_root(&cwd))?;
        if findings.is_empty() {
            return Ok(Outcome::Clean);
        }
        match self.format {
            Format::Text => {
                findings.chunk_by(same_function).try_for_each(|group| write_text(out, group))?;
            }
            Format::Json => writeln!(out, "{}", serde_json::to_string_pretty(&findings)?)?,
        }
        Ok(Outcome::Findings)
    }

    /// Index the whole repository and run the checks. Unless `--all`, run them
    /// again on the base side and keep only what is new at head.
    fn findings(&self, root: &Path) -> Result<Vec<Finding>> {
        let config = Config::load(root)?;
        // Read the diff first: a bad `--base` should fail before any parsing.
        let base = if self.all {
            None
        } else {
            Some(git::base_sources(root, self.base.as_deref().unwrap_or("HEAD"))?)
        };
        if base.as_ref().is_some_and(HashMap::is_empty) {
            return Ok(Vec::new());
        }
        let tree = discovery::discover(root, &config.exclude)?;
        let head = Parsed::new(&tree.files, &tree.projects, &config);
        let entry = &tree.entry_points;
        let Some(base) = base else { return Ok(run_checks(&head.into_index(), &config, entry)) };
        let before = run_checks(&head.with_sources(&base, &config).into_index(), &config, entry);
        let before: HashSet<_> = before.iter().map(Finding::key).collect();
        let mut findings = run_checks(&head.into_index(), &config, entry);
        findings.retain(|finding| !before.contains(&finding.key()));
        Ok(findings)
    }
}

/// The nearest ancestor holding `.git` (a directory, or the file a linked
/// worktree or submodule has), else the nearest holding `pyproject.toml`,
/// else `cwd`. Counting users needs the whole project, not the cwd's subtree.
fn project_root(cwd: &Path) -> PathBuf {
    let nearest = |marker: &str| cwd.ancestors().find(|dir| dir.join(marker).exists());
    nearest(".git").or_else(|| nearest("pyproject.toml")).unwrap_or(cwd).to_path_buf()
}

/// A function's `const-param` findings print as one block.
fn same_function(a: &Finding, b: &Finding) -> bool {
    matches!((a, b), (Finding::ConstParam(a), Finding::ConstParam(b)) if a.group == b.group)
}

/// One block: a single finding, or a function's `const-param` findings.
fn write_text(out: &mut impl Write, group: &[Finding]) -> Result<()> {
    let params: Vec<&const_param::Finding> = group
        .iter()
        .filter_map(|finding| match finding {
            Finding::ConstParam(finding) => Some(finding),
            _ => None,
        })
        .collect();
    match group {
        [Finding::OneImpl(finding)] => write_one_impl(out, finding),
        [Finding::PassThrough(finding)] => write_pass_through(out, finding),
        _ => write_const_param(out, &params),
    }
}

fn write_pass_through(out: &mut impl Write, finding: &pass_through::Finding) -> Result<()> {
    let f = finding;
    writeln!(out, "{} {}:{} {}", f.check, f.path, f.line, f.name)?;
    let form = match &f.target {
        Some(target) => format!("{} → {} {}", f.form, target.at, target.name),
        None => f.form.to_owned(),
    };
    writeln!(out, "  form: {form}; calls: {} prod, {} test", f.calls.prod, f.calls.test)?;
    if f.keep_missing_reason {
        writeln!(out, "  keep: missing reason")?;
    }
    writeln!(out, "  suggest: {}", f.suggest)?;
    Ok(())
}

fn write_const_param(out: &mut impl Write, group: &[&const_param::Finding]) -> Result<()> {
    let Some(f) = group.first() else { return Ok(()) };
    writeln!(out, "{} {}:{} {}", f.check, f.path, f.line, f.name)?;
    writeln!(out, "  calls: {} prod, {} test", f.calls.prod, f.calls.test)?;
    for f in group {
        let (explicit, sites) = (f.sites.len(), if f.sites.len() == 1 { "site" } else { "sites" });
        let evidence = match (f.form, explicit) {
            ("never-overridden", 0) => format!("default {} never overridden", f.value),
            ("never-overridden", n) if n == f.calls.prod + f.calls.test => {
                format!("always {} (= default), passed explicitly at {n} {sites}", f.value)
            }
            ("never-overridden", n) => {
                format!("default {} never overridden, passed explicitly at {n} {sites}", f.value)
            }
            _ => format!("always {}", f.value),
        };
        writeln!(out, "  {}: {evidence}", f.param)?;
    }
    if f.keep_missing_reason {
        writeln!(out, "  keep: missing reason")?;
    }
    writeln!(out, "  suggest: {}", const_param::suggest_group(group))?;
    Ok(())
}

fn write_one_impl(out: &mut impl Write, finding: &one_impl::Finding) -> Result<()> {
    let what = if finding.kind == "ABC" { "abstract methods" } else { "methods" };
    let site = &finding.site;
    let implementation = &finding.implementation;
    writeln!(
        out,
        "{} {}:{} {} ({}, {} {what})",
        finding.check, site.path, site.line, site.name, finding.kind, finding.members
    )?;
    writeln!(
        out,
        "  impl: {}:{} {}",
        implementation.path, implementation.line, implementation.name
    )?;
    if finding.keep_missing_reason {
        writeln!(out, "  keep: missing reason")?;
    }
    writeln!(out, "  suggest: {}", finding.suggest)?;
    Ok(())
}

/// Resolve the guide topic and render it. Detection reads the current
/// directory only; an unreadable one counts as "not configured".
fn guide_output(topic: Option<guide::Topic>) -> String {
    if let Some(topic) = topic {
        return guide::render(topic, guide::Selection::Explicit);
    }
    let configured = std::env::current_dir().is_ok_and(|cwd| guide::is_configured(&cwd));
    guide::render(guide::auto_topic(configured), guide::Selection::Auto(configured))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_and_all_are_mutually_exclusive() {
        let err = Cli::try_parse_from(["dermestes", "--all", "--base", "main"])
            .expect_err("--all with --base must be rejected");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict, "got: {err}");
    }

    #[test]
    fn bare_invocation_is_diff_mode_in_text() {
        let cli = Cli::try_parse_from(["dermestes"]).expect("bare invocation parses");
        assert!(cli.command.is_none(), "no subcommand means run the checks");
        assert!(!cli.all, "diff mode is the default");
        assert_eq!(cli.base, None, "HEAD is the default base");
        assert_eq!(cli.format, Format::Text, "text is the default format");
    }

    #[test]
    fn guide_takes_an_optional_topic() {
        let cli = Cli::try_parse_from(["dermestes", "guide"]).expect("bare guide parses");
        assert!(matches!(cli.command, Some(Command::Guide { topic: None })), "no topic = detect");
        let cli = Cli::try_parse_from(["dermestes", "guide", "tune"]).expect("a topic parses");
        assert!(matches!(cli.command, Some(Command::Guide { topic: Some(guide::Topic::Tune) })));
        assert!(Cli::try_parse_from(["dermestes", "guide", "how"]).is_err(), "unknown topic");
    }
}
