//! The `pass-through` check: a function that only forwards its arguments to
//! another repository function (`forward`), or a private one-line helper with
//! a single production call site (`single-use`).

use std::collections::HashSet;

use serde::Serialize;

use crate::callers::{listed, Callers, Calls, Resolved, Site};
use crate::calls::{Arg, Callee, FnId, FnKind, Function, Value, ValueKind};
use crate::config::Config;
use crate::const_param::bind;
use crate::index::{Index, Keep};

pub const ID: &str = "pass-through";

/// The longest literal a forwarding call may add: `"mcp"`, not a query.
const SHORT_LITERAL: usize = 20;

/// The function a forward wrapper delegates to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Target {
    /// Qualified: `Store._read`.
    pub name: String,
    /// `path:line` of its `def`.
    pub at: String,
}

/// One deletion candidate: a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub path: String,
    /// The `def` line.
    pub line: usize,
    /// Qualified: `Store.load`.
    pub name: String,
    /// `forward` or `single-use`.
    pub form: &'static str,
    /// Set for `forward`.
    pub target: Option<Target>,
    pub calls: Calls,
    /// `path:line` of each production call.
    pub sites: Vec<String>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Run the check over the whole index. `entry_points` are `[project.scripts]`
/// values, whose functions are called from outside. Sorted by path, then line.
pub fn run(index: &Index, config: &Config, entry_points: &[String]) -> Vec<Finding> {
    let callers = Callers::new(index);
    let entry: HashSet<FnId> =
        entry_points.iter().filter_map(|value| callers.entry_point(value)).collect();
    let mut findings: Vec<Finding> = (0..index.functions.len())
        .filter(|id| !entry.contains(id))
        .filter_map(|id| callers.pass_through(id, config))
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

impl Callers<'_> {
    fn pass_through(&self, id: FnId, config: &Config) -> Option<Finding> {
        let function = &self.index.functions[id];
        let forward = self.forward(id);
        // A `super()`-only override is the one override this check reports.
        let inherits = forward.is_some_and(|(_, inherits)| inherits);
        if self.guarded(id, config) || (self.overrides(id) && !inherits) {
            return None;
        }
        let sites = &self.sites[id];
        if sites.iter().all(|site| self.index.calls[site.call].under_main) {
            return None;
        }
        // An example calling it makes it documented API, like a test makes a seam.
        let (test, prod): (Vec<&Site>, Vec<&Site>) = sites.iter().partition(|site| {
            let file = &self.index.files[self.index.calls[site.call].file];
            file.is_test || file.evidence_only
        });
        if !test.is_empty() || prod.is_empty() || self.possibly_called(function) {
            return None;
        }
        let single_use = prod.len() == 1 && single_use(function);
        if forward.is_none() && !single_use {
            return None;
        }
        if forward.is_some() && self.target_shadowed(function, &prod) {
            return None;
        }
        let mut places: Vec<(&str, usize)> = prod
            .iter()
            .map(|site| {
                let call = &self.index.calls[site.call];
                (self.index.files[call.file].relative.as_str(), call.line)
            })
            .collect();
        places.sort_unstable();
        let sites: Vec<String> =
            places.iter().map(|(path, line)| format!("{path}:{line}")).collect();

        let target = forward.map(|(target, _)| &self.index.functions[target]);
        let suggest = match (target, inherits) {
            (Some(target), true) => {
                format!("delete {}, {} is inherited", function.qualname, target.qualname)
            }
            (Some(target), false) => format!(
                "call {} directly at {}, delete {}",
                target.name,
                listed(&sites),
                function.qualname
            ),
            (None, _) => format!("inline at {}, delete {}", listed(&sites), function.qualname),
        };
        Some(Finding {
            check: ID,
            path: self.index.files[function.file].relative.clone(),
            line: function.line,
            name: function.qualname.clone(),
            form: if target.is_some() { "forward" } else { "single-use" },
            target: target.map(|target| Target {
                name: target.qualname.clone(),
                at: format!("{}:{}", self.index.files[target.file].relative, target.line),
            }),
            calls: Calls { prod: prod.len(), test: 0 },
            sites,
            keep_missing_reason: function.keep == Keep::MissingReason,
            suggest,
        })
    }

    /// The repository function `id`'s whole body forwards to, and whether that
    /// is `super().<same name>(<same arguments>)`.
    fn forward(&self, id: FnId) -> Option<(FnId, bool)> {
        let function = &self.index.functions[id];
        let call = &self.index.calls[function.body.forward?];
        let Resolved::Function(target, _) = self.resolve_call(call) else { return None };
        if call.splat || target == id || self.index.functions[target].name == "__init__" {
            return None;
        }
        let receiver = matches!(function.kind, FnKind::Method | FnKind::ClassMethod)
            .then(|| function.params.first())
            .flatten()
            .map(|param| param.name.as_str());
        let params: Vec<&str> = function
            .params
            .iter()
            .map(|param| param.name.as_str())
            .skip(usize::from(receiver.is_some()))
            .collect();
        let values = call.args.iter().map(|arg| match arg {
            Arg::Positional(value) | Arg::Keyword(_, value) => value,
        });
        if !values.clone().all(|value| forwardable(value, &params, receiver)) {
            return None;
        }
        let same_name = matches!(&call.callee, Callee::Super(_, name) if *name == function.name);
        let same_args = call.args.len() == params.len()
            && call.args.iter().enumerate().all(|(position, arg)| match arg {
                Arg::Positional(value) => names(value, params[position]),
                Arg::Keyword(key, value) => names(value, key) && params.contains(&key.as_str()),
            });
        Some((target, same_name && same_args))
    }

    /// The forwarding call names its target bare, and a function around some
    /// production call binds that name (a parameter, local, `for` target or
    /// import): calling the target there would call something else.
    fn target_shadowed(&self, function: &Function, prod: &[&Site]) -> bool {
        let Some(forward) = function.body.forward else { return false };
        let Callee::Name(name) = &self.index.calls[forward].callee else { return false };
        prod.iter().any(|site| {
            let call = &self.index.calls[site.call];
            let scopes = &self.index.files[call.file].scopes;
            let mut scope = call.scope;
            while scope != 0 {
                let enclosing = &scopes[scope];
                if enclosing.class.is_none()
                    && (enclosing.bindings.contains_key(name) || enclosing.defs.contains_key(name))
                {
                    return true;
                }
                scope = enclosing.parent;
            }
            false
        })
    }

    /// A call we could not resolve that could bind to `function`: the count
    /// of callers is unknown.
    fn possibly_called(&self, function: &Function) -> bool {
        let Some(calls) = self.possible.get(function.name.as_str()) else { return false };
        let skip = usize::from(matches!(function.kind, FnKind::Method | FnKind::ClassMethod));
        calls.iter().any(|&id| {
            let call = &self.index.calls[id];
            call.splat || bind(function, call, skip).is_some()
        })
    }
}

/// The single-use form's shape: private, top-level or a method, one short
/// simple statement, not a named predicate, not a type-erasing `Any` wrapper.
fn single_use(function: &Function) -> bool {
    let name = function.name.as_str();
    let body = &function.body;
    name.starts_with('_')
        && !(name.starts_with("__") && name.ends_with("__"))
        && function.decorators.is_empty()
        && !body.nested
        && body.simple
        && !body.returns_bool
        && !body.any_param
}

/// A parameter name, a `self`/`cls` attribute or a short literal.
fn forwardable(value: &Value, params: &[&str], receiver: Option<&str>) -> bool {
    match &value.kind {
        ValueKind::Name(parts) => match &parts[..] {
            [name] => params.contains(&name.as_str()) || Some(name.as_str()) == receiver,
            [object, _] => Some(object.as_str()) == receiver,
            _ => false,
        },
        ValueKind::Literal(_) => value.text.len() <= SHORT_LITERAL && !value.text.contains('\n'),
        ValueKind::Other => false,
    }
}

fn names(value: &Value, param: &str) -> bool {
    matches!(&value.kind, ValueKind::Name(parts) if parts.len() == 1 && parts[0] == param)
}
