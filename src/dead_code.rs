//! The `dead-code` check: a function no production code mentions by name.
//!
//! Unlike the caller-counting checks it does not resolve calls. Any
//! occurrence of the bare name in production outside the function's own body
//! keeps it alive: an identifier with any receiver, a string literal spelling
//! it, or a word in a non-Python file. That over-counts uses, so it loses only
//! recall.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::callers::{Callers, Calls};
use crate::calls::{FnId, Function};
use crate::config::Config;
use crate::discovery::Tree;
use crate::index::{Index, Keep};

pub const ID: &str = "dead-code";

/// One deletion candidate: a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub path: String,
    /// The `def` line.
    pub line: usize,
    /// Qualified: `Store.load`.
    pub name: String,
    /// `unused` or `test-only`.
    pub form: &'static str,
    pub calls: Calls,
    /// `path:line` of each test line that mentions it.
    pub tests: Vec<String>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Run the check over the whole index. `tree` gives the entry points, the
/// typed packages and the words of non-Python files. Sorted by path, then line.
pub fn run(index: &Index, config: &Config, tree: &Tree) -> Vec<Finding> {
    let callers = Callers::new(index);
    let entry: HashSet<FnId> =
        tree.entry_points.iter().filter_map(|value| callers.entry_point(value)).collect();
    let candidates: Vec<FnId> = (0..index.functions.len())
        .filter(|id| !entry.contains(id) && !callers.exempt(*id, config))
        .filter(|&id| target(index, tree, &index.functions[id]))
        .collect();
    let names: HashSet<&str> =
        candidates.iter().map(|&id| index.functions[id].name.as_str()).collect();
    // Each candidate name's mentions: the file and the lines.
    let mut mentions: HashMap<&str, Vec<(usize, &[usize])>> = HashMap::new();
    for (file, info) in index.files.iter().enumerate() {
        for (name, lines) in &info.mentions {
            if let Some(name) = names.get(name.as_str()) {
                mentions.entry(name).or_default().push((file, lines));
            }
        }
    }
    let mut findings: Vec<Finding> = candidates
        .into_iter()
        .filter_map(|id| {
            let function = &index.functions[id];
            dead(index, function, mentions.get(function.name.as_str()).map_or(&[], Vec::as_slice))
        })
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

/// Guards of this check beyond the shared ones: what may be called from
/// outside the repository, or by a convention rather than by name.
fn target(index: &Index, tree: &Tree, function: &Function) -> bool {
    let file = &index.files[function.file];
    let name = function.name.as_str();
    let dunder = name.starts_with("__") && name.ends_with("__");
    let display_hook = name.starts_with("_repr_") && name.ends_with('_');
    // Alembic calls these in a module that assigns both revisions.
    let migration = matches!(name, "upgrade" | "downgrade")
        && function.class.is_none()
        && ["revision", "down_revision"].iter().all(|top| file.top.contains_key(*top));
    let typed = !name.starts_with('_')
        && tree.typed.iter().any(|dir| file.relative.starts_with(dir.as_str()));
    !function.body.nested
        && !dunder
        && !display_hook
        && !migration
        && !typed
        && !tree.words.contains(name)
        && !index.unparsed_words.contains(name)
}

/// The finding for `function` when no production line outside its own body
/// mentions its name.
fn dead(index: &Index, function: &Function, mentions: &[(usize, &[usize])]) -> Option<Finding> {
    let own = |file: usize, line: usize| {
        file == function.file && (function.line..=function.end_line).contains(&line)
    };
    let mut tests = Vec::new();
    for &(file, lines) in mentions {
        let info = &index.files[file];
        for &line in lines.iter().filter(|&&line| !own(file, line)) {
            if !info.is_test {
                return None;
            }
            tests.push((info.relative.as_str(), line));
        }
    }
    tests.sort_unstable();
    let name = &function.qualname;
    let suggest = match tests.as_slice() {
        [] => format!("delete {name}"),
        [(first, _), rest @ ..] => {
            let into =
                if rest.iter().all(|(path, _)| path == first) { first } else { "a test helper" };
            format!(
                "delete {name} and the tests that only exercise it, \
                 or move it into {into} if they use it as setup"
            )
        }
    };
    Some(Finding {
        check: ID,
        path: index.files[function.file].relative.clone(),
        line: function.line,
        name: name.clone(),
        form: if tests.is_empty() { "unused" } else { "test-only" },
        calls: Calls { prod: 0, test: tests.len() },
        tests: tests.iter().map(|(path, line)| format!("{path}:{line}")).collect(),
        keep_missing_reason: function.keep == Keep::MissingReason,
        suggest,
    })
}
