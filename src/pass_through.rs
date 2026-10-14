//! The `pass-through` check: a function that adds nothing over the call it makes.
//!
//! Three forms: it only forwards its arguments to another repository function
//! (`forward`), it is a private one-line helper with a single production call
//! site (`single-use`), or a private helper that only renames an outside call
//! for two call sites (`rename`).

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
    /// `forward`, `single-use` or `rename`.
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
        let inherits = forward.as_ref().is_some_and(|forward| forward.inherits);
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
        let patched = self.index.test_strings.contains(&function.name);
        if !test.is_empty() || patched || prod.is_empty() || self.possibly_called(function) {
            return None;
        }
        // A constant bound once or twice is the caller's to name; at more
        // sites the wrapper saves repeating it.
        if forward.as_ref().is_some_and(|forward| forward.constant) && prod.len() > 2 {
            return None;
        }
        let single_use = prod.len() == 1 && single_use(function);
        // With one site, a rename is single-use, and "inline" says the same.
        let rename =
            if forward.is_none() && prod.len() == 2 { self.rename(function) } else { None };
        if forward.is_none() && !single_use && rename.is_none() {
            return None;
        }
        if (forward.is_some() || rename.is_some()) && self.target_shadowed(function, &prod) {
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

        let target = forward.map(|forward| &self.index.functions[forward.target]);
        let suggest = match (target, inherits) {
            (Some(target), true) => {
                format!("delete {}, {} is inherited", function.qualname, target.qualname)
            }
            (Some(target), false) => format!(
                "call {} directly at {}{}, delete {}",
                self.forward_call(function, target),
                listed(&sites),
                if function.is_async && !target.is_async { " (drop the await)" } else { "" },
                function.qualname
            ),
            (None, _) => match &rename {
                Some(call) => format!(
                    "call {call} directly at {}, delete {}",
                    listed(&sites),
                    function.qualname
                ),
                None => format!("inline at {}, delete {}", listed(&sites), function.qualname),
            },
        };
        let form = match (target, &rename) {
            (Some(_), _) => "forward",
            (None, Some(_)) => "rename",
            (None, None) => "single-use",
        };
        Some(Finding {
            check: ID,
            path: self.index.files[function.file].relative.clone(),
            line: function.line,
            name: function.qualname.clone(),
            form,
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

    /// The repository function `id`'s whole body forwards to.
    fn forward(&self, id: FnId) -> Option<Forward> {
        let function = &self.index.functions[id];
        let call = &self.index.calls[function.body.forward?];
        let Resolved::Function(target, _) = self.resolve_call(call) else { return None };
        if call.splat || target == id || self.index.functions[target].name == "__init__" {
            return None;
        }
        // A public wrapper is the private target's public name.
        if !private(&function.name) && private(&self.index.functions[target].name) {
            return None;
        }
        let (params, receiver) = own_names(function);
        let bound: Vec<&Value> =
            values(&call.args).filter(|value| !own(value, &params, receiver)).collect();
        if !bound.iter().all(|value| short_literal(value) || constant(value)) {
            return None;
        }
        // A constant may be the one value a private wrapper binds. Next to
        // another value, or under a public name, the wrapper is a convention.
        let constant = bound.iter().any(|value| constant(value));
        if constant && (bound.len() > 1 || !private(&function.name)) {
            return None;
        }
        let same_name = matches!(&call.callee, Callee::Super(_, name) if *name == function.name);
        let inherits = same_name && passes_through(&call.args, &params);
        Some(Forward { target, inherits, constant })
    }

    /// The call a private helper's body makes to something outside the
    /// repository, `date.fromisoformat(v)`, when it passes only the helper's
    /// own parameters and receiver attributes: the helper is another name.
    fn rename(&self, function: &Function) -> Option<String> {
        if !single_use(function) {
            return None;
        }
        let call = &self.index.calls[function.body.forward?];
        let callee = match &call.callee {
            Callee::Name(name) => name.clone(),
            Callee::Dotted(parts, name) => format!("{}.{name}", parts.join(".")),
            _ => return None,
        };
        if call.splat || matches!(self.resolve_call(call), Resolved::Function(..)) {
            return None;
        }
        let (params, receiver) = own_names(function);
        if !values(&call.args).all(|value| own(value, &params, receiver)) {
            return None;
        }
        Some(format!("{callee}({})", rendered(&call.args)))
    }

    /// `target`, or `target(<arguments>)` when the wrapper does more than
    /// pass its own parameters through: callers must carry what it bound.
    fn forward_call(&self, function: &Function, target: &Function) -> String {
        let Some(forward) = function.body.forward else { return target.name.clone() };
        let args = &self.index.calls[forward].args;
        let skip = usize::from(matches!(function.kind, FnKind::Method | FnKind::ClassMethod));
        let params: Vec<&str> =
            function.params.iter().skip(skip).map(|param| param.name.as_str()).collect();
        if passes_through(args, &params) {
            return target.name.clone();
        }
        format!("{}({})", target.name, rendered(args))
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
    let body = &function.body;
    private(&function.name)
        && function.decorators.is_empty()
        && !body.nested
        && body.simple
        && !body.returns_bool
        && !body.any_param
}

/// `_x`, not a dunder.
fn private(name: &str) -> bool {
    name.starts_with('_') && !(name.starts_with("__") && name.ends_with("__"))
}

/// What a forward wrapper delegates to.
struct Forward {
    target: FnId,
    /// The call is `super().<same name>(<same arguments>)`.
    inherits: bool,
    /// The call binds a module constant.
    constant: bool,
}

/// A function's parameters, without the receiver, and the receiver of a
/// method or classmethod.
fn own_names(function: &Function) -> (Vec<&str>, Option<&str>) {
    let receiver = matches!(function.kind, FnKind::Method | FnKind::ClassMethod)
        .then(|| function.params.first())
        .flatten()
        .map(|param| param.name.as_str());
    let params = function
        .params
        .iter()
        .map(|param| param.name.as_str())
        .skip(usize::from(receiver.is_some()))
        .collect();
    (params, receiver)
}

fn values(args: &[Arg]) -> impl Iterator<Item = &Value> {
    args.iter().map(|arg| match arg {
        Arg::Positional(value) | Arg::Keyword(_, value) => value,
    })
}

/// The arguments as written: `resp, TOOL_NAME, flush=True`.
fn rendered(args: &[Arg]) -> String {
    let args: Vec<String> = args
        .iter()
        .map(|arg| match arg {
            Arg::Positional(value) => value.text.clone(),
            Arg::Keyword(key, value) => format!("{key}={}", value.text),
        })
        .collect();
    args.join(", ")
}

/// A parameter name or a `self`/`cls` attribute: nothing the wrapper binds.
fn own(value: &Value, params: &[&str], receiver: Option<&str>) -> bool {
    match &value.kind {
        ValueKind::Name(parts) => match &parts[..] {
            [name] => params.contains(&name.as_str()) || Some(name.as_str()) == receiver,
            [object, _] => Some(object.as_str()) == receiver,
            _ => false,
        },
        _ => false,
    }
}

/// `"mcp"`, not a query.
fn short_literal(value: &Value) -> bool {
    matches!(value.kind, ValueKind::Literal(_))
        && value.text.len() <= SHORT_LITERAL
        && !value.text.contains('\n')
}

/// `TOOL_NAME`: a bare name that is a module constant by convention.
fn constant(value: &Value) -> bool {
    let ValueKind::Name(parts) = &value.kind else { return false };
    let [name] = &parts[..] else { return false };
    name.len() > 1
        && name.chars().any(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Every parameter, once each, by position or as `name=name`, and nothing else.
fn passes_through(args: &[Arg], params: &[&str]) -> bool {
    args.len() == params.len()
        && args.iter().enumerate().all(|(position, arg)| match arg {
            Arg::Positional(value) => names(value, params[position]),
            Arg::Keyword(key, value) => names(value, key) && params.contains(&key.as_str()),
        })
}

fn names(value: &Value, param: &str) -> bool {
    matches!(&value.kind, ValueKind::Name(parts) if parts.len() == 1 && parts[0] == param)
}
