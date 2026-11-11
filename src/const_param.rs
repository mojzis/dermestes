//! The `const-param` check: a parameter nobody varies.
//!
//! That is a parameter every resolved call site leaves at its default, or
//! fills with the same literal or module constant; a variadic no call site
//! populates; and, as evidence, the branches such a constant makes dead.

use std::collections::HashMap;

use serde::Serialize;

use crate::branches::{Cond, Test, When};
use crate::callers::{listed, Callers, Calls, Site};
use crate::calls::{Arg, Call, FnId, FnKind, Function, Param, ParamKind, Value, ValueKind};
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
    /// `path:line` of the function: its findings print as one block.
    pub group: String,
    pub param: String,
    /// Source text of the value every site gives.
    pub value: String,
    /// `never-overridden` (the default always applies), `same-literal`, or
    /// `never-populated` (a `*args`/`**kwargs` no site adds to).
    pub form: &'static str,
    pub calls: Calls,
    /// `path:line` of each argument that passes the value explicitly.
    pub sites: Vec<String>,
    /// What cannot run while the parameter has this value.
    pub dead_spans: Vec<Span>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Lines `start..=end` of the function's file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    /// An arm of a conditional expression: part of its lines, not all of them.
    pub inline: bool,
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
        let files = sites.iter().map(|site| &self.index.files[self.index.calls[site.call].file]);
        let test = files.clone().filter(|file| file.is_test).count();
        // Examples vary a parameter but never pin one on their own.
        let prod = files.filter(|file| !file.is_test && !file.evidence_only).count();
        // Called only from tests: dead code, not a constant parameter.
        if prod == 0 {
            return Vec::new();
        }
        let calls = Calls { prod, test };
        let receiver = matches!(function.kind, FnKind::Method | FnKind::ClassMethod);

        let file = &self.index.files[function.file];
        // What every finding of this function shares.
        let blank = Finding {
            check: ID,
            path: file.relative.clone(),
            line: function.line,
            name: function.qualname.clone(),
            group: format!("{}:{}", file.relative, function.line),
            param: String::new(),
            value: String::new(),
            form: "",
            calls,
            sites: Vec::new(),
            dead_spans: Vec::new(),
            keep_missing_reason: function.keep == Keep::MissingReason,
            suggest: String::new(),
        };
        let mut findings = Vec::new();
        for (position, param) in function.params.iter().enumerate() {
            if receiver && position == 0 {
                continue;
            }
            if matches!(param.kind, ParamKind::VarArgs | ParamKind::VarKeyword) {
                let never_populated = self.never_populated(function, param, &bound);
                findings.extend(never_populated.map(|(param, suggest)| Finding {
                    param,
                    form: "never-populated",
                    suggest,
                    ..blank.clone()
                }));
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
            let explicit = self.explicit(sites, &given);
            let dead_spans = match &key {
                Key::Literal(literal) => dead_spans(function, &param.name, literal),
                _ => Vec::new(),
            };
            findings.push(Finding {
                param: param.name.clone(),
                suggest: suggest(&param.name, &value, &explicit, &dead_spans),
                value,
                form: if never_overridden { "never-overridden" } else { "same-literal" },
                sites: explicit,
                dead_spans,
                ..blank.clone()
            });
        }
        findings
    }

    /// `path:line` of each argument that passes a value explicitly, sorted.
    fn explicit(&self, sites: &[Site], given: &[Option<&Value>]) -> Vec<String> {
        let mut explicit: Vec<(&str, usize)> = sites
            .iter()
            .zip(given)
            .filter_map(|(site, value)| {
                let file = &self.index.files[self.index.calls[site.call].file];
                value.map(|value| (file.relative.as_str(), value.line))
            })
            .collect();
        explicit.sort_unstable();
        explicit.iter().map(|(path, line)| format!("{path}:{line}")).collect()
    }

    /// A `*args`/`**kwargs` no site adds to, as `**kwargs` and its suggestion.
    /// `**_` is a deliberate sink, and any unresolved call of the name may
    /// populate it.
    fn never_populated(
        &self,
        function: &Function,
        param: &Param,
        bound: &[HashMap<&str, &Value>],
    ) -> Option<(String, String)> {
        let populated = param.name.starts_with('_')
            || bound.iter().any(|binding| binding.contains_key(param.name.as_str()))
            || self.possible.get(function.name.as_str()).is_some_and(|calls| !calls.is_empty());
        let stars = if param.kind == ParamKind::VarArgs { "*" } else { "**" };
        let name = format!("{stars}{}", param.name);
        let lines: Vec<String> = param.forwarded.iter().map(usize::to_string).collect();
        let suggest = match lines.len() {
            0 => format!("drop {name}"),
            1 => format!("drop {name} (and the {name} forwarded at line {})", lines[0]),
            _ => format!("drop {name} (and the {name} forwarded at lines {})", lines.join(", ")),
        };
        (!populated).then_some((name, suggest))
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

/// What `cond` evaluates to while its parameter is the literal `key`;
/// `None` when the literal is one we do not model (hex, bytes, a tuple).
fn truth(cond: &Cond, key: &str) -> Option<bool> {
    #[derive(PartialEq)]
    enum Lit<'a> {
        None,
        Num(f64),
        Str(&'a str),
    }
    fn lit(key: &str) -> Option<Lit<'_>> {
        match key {
            "None" => Some(Lit::None),
            "True" => Some(Lit::Num(1.0)),
            "False" => Some(Lit::Num(0.0)),
            _ if key.starts_with('"') => Some(Lit::Str(key)),
            _ => key.replace('_', "").parse().ok().map(Lit::Num),
        }
    }
    let value = lit(key)?;
    let holds = match &cond.test {
        Test::Truthy => match value {
            Lit::None => false,
            Lit::Num(number) => number != 0.0,
            Lit::Str(text) => text != "\"\"",
        },
        Test::IsNone => value == Lit::None,
        Test::Among(keys) => {
            let others: Option<Vec<Lit<'_>>> = keys.iter().map(|other| lit(other)).collect();
            others?.contains(&value)
        }
    };
    Some(holds != cond.negated)
}

/// The arms of `function`'s branches that cannot run while `param` is the
/// literal `key`: a false test's arm, and every arm after a true one.
fn dead_spans(function: &Function, param: &str, key: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for branch in &function.branches {
        let mut decided = false;
        for arm in &branch.arms {
            let holds = match &arm.when {
                When::Test(cond) if cond.param == param && !decided => truth(cond, key),
                _ => None,
            };
            if decided || holds == Some(false) {
                spans.push(Span { start: arm.start, end: arm.end, inline: branch.inline });
            }
            decided |= holds == Some(true);
        }
    }
    spans.sort_unstable();
    let mut merged: Vec<Span> = Vec::new();
    for span in spans {
        match merged.last_mut() {
            // Inside a dead block, or the block right after one.
            Some(last) if !last.inline && span.start <= last.end + usize::from(!span.inline) => {
                last.end = last.end.max(span.end);
            }
            Some(last) if last.inline && span.inline && span.start == last.start => {}
            _ => merged.push(span),
        }
    }
    merged
}

/// `lines 48-61, conditional at line 70`: the spans as evidence.
pub fn spans_text(spans: &[Span]) -> String {
    let texts: Vec<String> = spans
        .iter()
        .map(|span| match (span.inline, span.start == span.end) {
            (true, _) => format!("conditional at line {}", span.start),
            (false, true) => format!("line {}", span.start),
            (false, false) => format!("lines {}-{}", span.start, span.end),
        })
        .collect();
    texts.join(", ")
}

/// `drop p, use v inline`, naming up to three arguments callers must lose,
/// then what to do with each dead span.
fn suggest(param: &str, value: &str, sites: &[String], dead: &[Span]) -> String {
    let mut text = format!("drop {}, use {value} inline", dropped(param, sites));
    let (inline, blocks): (Vec<Span>, Vec<Span>) = dead.iter().partition(|span| span.inline);
    if !blocks.is_empty() {
        let lines: usize = blocks.iter().map(|span| span.end - span.start + 1).sum();
        let unit = if lines == 1 { "line" } else { "lines" };
        text = format!("{text}, delete {} ({lines} {unit})", spans_text(&blocks));
    }
    if !inline.is_empty() {
        text = format!("{text}, simplify the {}", spans_text(&inline));
    }
    text
}

/// One suggestion for a function's findings.
///
/// Parameters that share a value share a clause: `drop gap, margin, use 0
/// inline; drop strict, use True inline`. A variadic, or a parameter with
/// dead spans, keeps its own clause.
pub fn suggest_group(findings: &[&Finding]) -> String {
    let mut clauses: Vec<(Option<&str>, Vec<String>)> = Vec::new();
    for finding in findings {
        if finding.form == "never-populated" || !finding.dead_spans.is_empty() {
            clauses.push((None, vec![finding.suggest.clone()]));
            continue;
        }
        let param = dropped(&finding.param, &finding.sites);
        match clauses.iter_mut().find(|(value, _)| *value == Some(finding.value.as_str())) {
            Some((_, params)) => params.push(param),
            None => clauses.push((Some(&finding.value), vec![param])),
        }
    }
    let clauses: Vec<String> = clauses
        .iter()
        .map(|(value, params)| match value {
            Some(value) => format!("drop {}, use {value} inline", params.join(", ")),
            None => params.concat(),
        })
        .collect();
    clauses.join("; ")
}

fn dropped(param: &str, sites: &[String]) -> String {
    if sites.is_empty() {
        return param.to_owned();
    }
    format!("{param} (and the argument at {})", listed(sites))
}

/// Bind `call`'s arguments to `function`'s parameters, the first `skip` of
/// which the call fills implicitly. A surplus argument binds to the variadic
/// that takes it. `None` when the call cannot be a call to `function`: a
/// splat, too many arguments, an unknown keyword, a required parameter left out.
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
    let variadic =
        |kind: ParamKind| params.iter().find(|param| param.kind == kind).map(|p| p.name.as_str());
    let mut bound: HashMap<&'c str, &'c Value> = HashMap::new();
    let mut index = skip;
    for arg in &call.args {
        match arg {
            Arg::Positional(value) => {
                match positional.get(index) {
                    Some(name) => {
                        bound.insert(name, value);
                    }
                    None => {
                        bound.insert(variadic(ParamKind::VarArgs)?, value);
                    }
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
                    None => {
                        bound.insert(variadic(ParamKind::VarKeyword)?, value);
                    }
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
