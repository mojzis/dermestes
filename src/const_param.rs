//! The `const-param` check: a parameter that every resolved call site leaves
//! at its default, or fills with the same literal or module constant.

use std::collections::HashMap;

use serde::Serialize;

use crate::callers::{listed, Callers, Calls, Site};
use crate::calls::{Arg, Call, FnId, FnKind, Function, ParamKind, Value, ValueKind};
use crate::config::Config;
use crate::index::{Index, Keep};
use crate::resolve::Target;

pub const ID: &str = "const-param";

/// One deletion candidate: a parameter of a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub path: String,
    /// The `def` line.
    pub line: usize,
    /// Qualified: `Transcript.done`.
    pub name: String,
    pub param: String,
    /// Source text of the value every site gives.
    pub value: String,
    /// `never-overridden` (the default always applies) or `same-literal`.
    pub form: &'static str,
    pub calls: Calls,
    /// `path:line` of each argument that passes the value explicitly.
    pub sites: Vec<String>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Run the check over the whole index. Findings are sorted by path, then line.
pub fn run(index: &Index, config: &Config) -> Vec<Finding> {
    let check = Callers::new(index);
    let mut findings: Vec<Finding> = (0..index.functions.len())
        .filter(|&id| !check.exempt(id, config))
        .flat_map(|id| check.findings(id))
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

/// What every site gives one parameter, as a comparable key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Literal(String),
    Constant(usize, String),
    /// A default that is neither: equal only to itself, left out.
    Default,
}

impl Callers<'_> {
    fn findings(&self, id: FnId) -> Vec<Finding> {
        let function = &self.index.functions[id];
        let sites = &self.sites[id];
        if sites.is_empty() || sites.iter().all(|site| self.index.calls[site.call].under_main) {
            return Vec::new();
        }
        let Some(bound) = sites
            .iter()
            .map(|site| bind(function, &self.index.calls[site.call], site.skip))
            .collect::<Option<Vec<_>>>()
        else {
            return Vec::new();
        };
        let (test, prod): (Vec<&Site>, Vec<&Site>) = sites
            .iter()
            .partition(|site| self.index.files[self.index.calls[site.call].file].is_test);
        // Called only from tests: dead code, not a constant parameter.
        if prod.is_empty() {
            return Vec::new();
        }
        let calls = Calls { prod: prod.len(), test: test.len() };
        let receiver = matches!(function.kind, FnKind::Method | FnKind::ClassMethod);

        let mut findings = Vec::new();
        for (position, param) in function.params.iter().enumerate() {
            if (receiver && position == 0)
                || matches!(param.kind, ParamKind::VarArgs | ParamKind::VarKeyword)
            {
                continue;
            }
            let default = param.default.as_ref();
            let default_key =
                default.map(|value| self.key(function.file, 0, value).unwrap_or(Key::Default));
            // Each site's value for `param`: `None` when it is left to the default.
            let given: Vec<Option<&Value>> = sites
                .iter()
                .zip(&bound)
                .map(|(_, binding)| binding.get(param.name.as_str()).copied())
                .collect();
            let mut keys = Vec::new();
            for (site, value) in sites.iter().zip(&given) {
                let call = &self.index.calls[site.call];
                keys.push(match value {
                    Some(value) => self.key(call.file, call.scope, value),
                    None => default_key.clone(),
                });
            }
            let Some(Some(key)) = keys.first().cloned() else { continue };
            if keys.iter().any(|other| other.as_ref() != Some(&key)) {
                continue;
            }
            if self.possibly_varied(function, &param.name, &key, default_key.as_ref()) {
                continue;
            }
            let never_overridden = default_key.as_ref() == Some(&key);
            let value = if never_overridden {
                default.map(|value| value.text.clone())
            } else {
                given.iter().flatten().next().map(|value| value.text.clone())
            };
            let Some(value) = value else { continue };
            let mut explicit: Vec<(&str, usize)> = sites
                .iter()
                .zip(&given)
                .filter_map(|(site, value)| {
                    let file = &self.index.files[self.index.calls[site.call].file];
                    value.map(|value| (file.relative.as_str(), value.line))
                })
                .collect();
            explicit.sort_unstable();
            let explicit: Vec<String> =
                explicit.iter().map(|(path, line)| format!("{path}:{line}")).collect();
            let file = &self.index.files[function.file];
            findings.push(Finding {
                check: ID,
                path: file.relative.clone(),
                line: function.line,
                name: function.qualname.clone(),
                param: param.name.clone(),
                suggest: suggest(&param.name, &value, &explicit),
                sites: explicit,
                value,
                form: if never_overridden { "never-overridden" } else { "same-literal" },
                calls,
                keep_missing_reason: function.keep == Keep::MissingReason,
            });
        }
        findings
    }

    /// An unresolved call of the same name could be a call to `function`
    /// that gives `param` another value.
    fn possibly_varied(
        &self,
        function: &Function,
        param: &str,
        key: &Key,
        default: Option<&Key>,
    ) -> bool {
        let Some(calls) = self.possible.get(function.name.as_str()) else { return false };
        let skip = usize::from(matches!(function.kind, FnKind::Method | FnKind::ClassMethod));
        calls.iter().any(|&id| {
            let call = &self.index.calls[id];
            if call.splat {
                return true;
            }
            // A call that cannot bind to `function` is a call to something else.
            let Some(binding) = bind(function, call, skip) else { return false };
            let value = match binding.get(param) {
                Some(value) => self.key(call.file, call.scope, value),
                None => default.cloned(),
            };
            value.as_ref() != Some(key)
        })
    }

    /// The comparable identity of a value: a literal, or a name resolved to
    /// a module constant. Anything else is never "the same value".
    fn key(&self, file: usize, scope: usize, value: &Value) -> Option<Key> {
        match &value.kind {
            ValueKind::Literal(literal) => Some(Key::Literal(literal.clone())),
            ValueKind::Name(parts) => match self.resolve_parts(file, scope, parts) {
                // A constant a test patches (`monkeypatch.setattr("m.X", ...)`) varies.
                Target::Constant(file, name) if !self.index.strings.contains(&name) => {
                    Some(Key::Constant(file, name))
                }
                _ => None,
            },
            ValueKind::Other => None,
        }
    }
}

/// `drop p, use v inline`, naming up to three arguments callers must lose.
fn suggest(param: &str, value: &str, sites: &[String]) -> String {
    if sites.is_empty() {
        return format!("drop {param}, use {value} inline");
    }
    format!("drop {param} (and the argument at {}), use {value} inline", listed(sites))
}

/// Bind `call`'s arguments to `function`'s parameters, the first `skip` of
/// which the call fills implicitly. `None` when the call cannot be a call to
/// `function`: a splat, too many arguments, an unknown keyword, a required
/// parameter left out.
pub(crate) fn bind<'c>(
    function: &'c Function,
    call: &'c Call,
    skip: usize,
) -> Option<HashMap<&'c str, &'c Value>> {
    if call.splat {
        return None;
    }
    let params = &function.params;
    let positional: Vec<&'c str> = params
        .iter()
        .filter(|param| matches!(param.kind, ParamKind::PositionalOnly | ParamKind::Positional))
        .map(|param| param.name.as_str())
        .collect();
    let has = |kind: ParamKind| params.iter().any(|param| param.kind == kind);
    let mut bound: HashMap<&'c str, &'c Value> = HashMap::new();
    let mut index = skip;
    for arg in &call.args {
        match arg {
            Arg::Positional(value) => {
                match positional.get(index) {
                    Some(name) => {
                        bound.insert(name, value);
                    }
                    None if has(ParamKind::VarArgs) => {}
                    None => return None,
                }
                index += 1;
            }
            Arg::Keyword(name, value) => {
                let param = params.iter().find(|param| {
                    &param.name == name
                        && matches!(param.kind, ParamKind::Positional | ParamKind::KeywordOnly)
                });
                match param {
                    Some(_) if bound.insert(name.as_str(), value).is_some() => return None,
                    Some(_) => {}
                    None if has(ParamKind::VarKeyword) => {}
                    None => return None,
                }
            }
        }
    }
    let missing = params.iter().skip(skip).any(|param| {
        param.default.is_none()
            && !matches!(param.kind, ParamKind::VarArgs | ParamKind::VarKeyword)
            && !bound.contains_key(param.name.as_str())
    });
    (!missing).then_some(bound)
}
