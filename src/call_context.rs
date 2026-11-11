//! The `unused-result` and `needless-async` checks: what every call site does
//! with a call.
//!
//! Both need the same evidence: every call of the function resolved, none
//! left unresolved, and each one's context. A result every site drops is not
//! a result; an `async def` that awaits nothing and is only ever awaited
//! directly is a plain function.

use std::collections::HashSet;

use serde::Serialize;

use crate::callers::{listed, Callers, Calls};
use crate::calls::FnId;
use crate::config::Config;
use crate::discovery::Tree;
use crate::index::Keep;

pub const UNUSED_RESULT: &str = "unused-result";
pub const NEEDLESS_ASYNC: &str = "needless-async";

/// One deletion candidate: a function's return value, or its `async`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub path: String,
    /// The `def` line.
    pub line: usize,
    /// Qualified: `Store.flush`.
    pub name: String,
    pub calls: Calls,
    /// `unused-result`: the line of each `return <value>`. `needless-async`:
    /// `path:line` of each `await` to drop.
    pub sites: Vec<String>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Run both checks over every resolved call. Sorted by path, then line.
pub fn run(callers: &Callers<'_>, config: &Config, tree: &Tree) -> Vec<Finding> {
    let index = callers.index;
    let entry: HashSet<FnId> =
        tree.entry_points.iter().filter_map(|value| callers.entry_point(value)).collect();
    let mut findings: Vec<Finding> = (0..index.functions.len())
        .filter(|id| !entry.contains(id) && !callers.exempt(*id, config))
        .flat_map(|id| callers.call_context(id, tree))
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

impl Callers<'_> {
    fn call_context(&self, id: FnId, tree: &Tree) -> Vec<Finding> {
        let function = &self.index.functions[id];
        let file = &self.index.files[function.file];
        let sites: Vec<_> =
            self.sites[id].iter().map(|site| &self.index.calls[site.call]).collect();
        // A public name in a typed package has callers outside the repository.
        let typed = !function.name.starts_with('_')
            && tree.typed.iter().any(|dir| file.relative.starts_with(dir.as_str()));
        let unresolved = self.possible.get(function.name.as_str()).is_some_and(|c| !c.is_empty());
        if typed || unresolved || sites.iter().all(|call| call.under_main) {
            return Vec::new();
        }
        let files = sites.iter().map(|call| &self.index.files[call.file]);
        let calls = Calls {
            prod: files.clone().filter(|file| !file.is_test && !file.evidence_only).count(),
            test: files.filter(|file| file.is_test).count(),
        };
        if calls.prod == 0 {
            return Vec::new();
        }
        let finding = |check, sites: Vec<String>, suggest| Finding {
            check,
            path: file.relative.clone(),
            line: function.line,
            name: function.qualname.clone(),
            calls,
            sites,
            keep_missing_reason: function.keep == Keep::MissingReason,
            suggest,
        };
        let mut findings = Vec::new();
        let returns = &function.returns;
        if !returns.is_empty()
            && !function.returns_receiver
            && !function.returns_computed
            && !function.yields
            && sites.iter().all(|call| call.bare)
        {
            let lines: Vec<String> = returns.iter().map(usize::to_string).collect();
            let unit = if lines.len() == 1 { "line" } else { "lines" };
            let suggest =
                format!("return nothing; drop the return value at {unit} {}", lines.join(", "));
            findings.push(finding(UNUSED_RESULT, lines, suggest));
        }
        if function.is_async
            && !function.awaits
            && !function.yields
            && sites.iter().all(|call| call.awaited)
        {
            let mut awaits: Vec<(&str, usize)> = sites
                .iter()
                .map(|call| (self.index.files[call.file].relative.as_str(), call.line))
                .collect();
            awaits.sort_unstable();
            let awaits: Vec<String> =
                awaits.iter().map(|(path, line)| format!("{path}:{line}")).collect();
            let suggest = format!("drop async; drop the await at {}", listed(&awaits));
            findings.push(finding(NEEDLESS_ASYNC, awaits, suggest));
        }
        findings
    }
}
