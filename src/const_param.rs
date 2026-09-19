//! The `const-param` check: a parameter that every resolved call site leaves
//! at its default, or fills with the same literal or module constant.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::calls::{Arg, Call, Callee, FnId, FnKind, Function, ParamKind, Value, ValueKind};
use crate::config::{matches_any, Config};
use crate::index::{ClassId, Index, Keep};
use crate::resolve::{Resolver, Target};

pub const ID: &str = "const-param";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Calls {
    pub prod: usize,
    pub test: usize,
}

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
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Run the check over the whole index. Findings are sorted by path, then line.
pub fn run(index: &Index, config: &Config) -> Vec<Finding> {
    let check = Check::new(index);
    let mut findings: Vec<Finding> = (0..index.functions.len())
        .filter(|&id| !check.exempt(id, config))
        .flat_map(|id| check.findings(id))
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

/// Where a method name leads from a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lookup {
    Found(FnId),
    /// Somewhere along the way a base, or the name itself, is ambiguous.
    Unknown,
    Missing,
}

/// A call resolved to a repository function, with how many leading
/// parameters the call binds implicitly (`self` or `cls`).
#[derive(Debug, Clone, Copy)]
struct Site {
    call: usize,
    skip: usize,
}

/// What every site gives one parameter, as a comparable key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Literal(String),
    Constant(usize, String),
    /// A default that is neither: equal only to itself, left out.
    Default,
}

struct Check<'a> {
    index: &'a Index,
    resolver: Resolver<'a>,
    /// Each class's bases in order; `None` for one we cannot follow.
    /// `object`, `ABC`, `Protocol` and `Generic` are left out.
    bases: Vec<Vec<Option<ClassId>>>,
    protocols: HashSet<ClassId>,
    /// Methods by class and name; `None` when the name is defined twice.
    methods: HashMap<(ClassId, &'a str), Option<FnId>>,
    /// Methods some subclass redefines: `self.m()` may land on the override.
    overridden: HashSet<FnId>,
    reexported: HashSet<FnId>,
    sites: Vec<Vec<Site>>,
    /// Calls we could not resolve, by the name they call.
    possible: HashMap<&'a str, Vec<usize>>,
}

impl<'a> Check<'a> {
    fn new(index: &'a Index) -> Self {
        let resolver = Resolver::new(index);
        let mut protocols = HashSet::new();
        let bases = index
            .classes
            .iter()
            .enumerate()
            .map(|(id, class)| {
                let mut bases = Vec::new();
                for base in &class.bases {
                    match resolver.dotted(class.file, base) {
                        Target::Class(parent) => bases.push(Some(parent)),
                        Target::Protocol => {
                            protocols.insert(id);
                        }
                        Target::Abc | Target::Object => {}
                        _ => bases.push(None),
                    }
                }
                bases
            })
            .collect();
        let mut methods = HashMap::new();
        for (id, function) in index.functions.iter().enumerate() {
            if let Some(class) = function.class {
                methods
                    .entry((class, function.name.as_str()))
                    .and_modify(|slot| *slot = None)
                    .or_insert(Some(id));
            }
        }
        let reexported = index
            .files
            .iter()
            .filter(|file| file.is_init)
            .flat_map(|file| file.bindings.values())
            .filter_map(|binding| match resolver.binding(binding) {
                Target::Function(id) => Some(id),
                _ => None,
            })
            .collect();
        let mut check = Self {
            index,
            resolver,
            bases,
            protocols,
            methods,
            overridden: HashSet::new(),
            reexported,
            sites: vec![Vec::new(); index.functions.len()],
            possible: HashMap::new(),
        };
        check.overridden = check.find_overridden();
        for (id, call) in index.calls.iter().enumerate() {
            match check.resolve_call(call) {
                Resolved::Function(function, skip) => {
                    check.sites[function].push(Site { call: id, skip });
                }
                Resolved::Possible(name) => check.possible.entry(name).or_default().push(id),
                Resolved::Nothing => {}
            }
        }
        check
    }

    fn find_overridden(&self) -> HashSet<FnId> {
        let mut overridden = HashSet::new();
        for function in &self.index.functions {
            let Some(class) = function.class else { continue };
            for ancestor in self.ancestors(class) {
                if let Some(Some(id)) = self.methods.get(&(ancestor, function.name.as_str())) {
                    overridden.insert(*id);
                }
            }
        }
        overridden
    }

    /// Every resolved class above `class`.
    fn ancestors(&self, class: ClassId) -> Vec<ClassId> {
        let mut seen = HashSet::from([class]);
        let mut stack = vec![class];
        let mut out = Vec::new();
        while let Some(current) = stack.pop() {
            for &parent in self.bases[current].iter().flatten() {
                if seen.insert(parent) {
                    out.push(parent);
                    stack.push(parent);
                }
            }
        }
        out
    }

    /// `name` on `class`, then along its bases in order, as Python's MRO would
    /// find it in a single-inheritance chain. An unfollowable base ends the
    /// search as unknown: it may define the name.
    fn lookup(&self, class: ClassId, name: &str, depth: usize) -> Lookup {
        match self.methods.get(&(class, name)) {
            Some(Some(id)) => return Lookup::Found(*id),
            Some(None) => return Lookup::Unknown,
            None => {}
        }
        self.lookup_bases(class, name, depth)
    }

    fn lookup_bases(&self, class: ClassId, name: &str, depth: usize) -> Lookup {
        if depth > 32 {
            return Lookup::Unknown;
        }
        for base in &self.bases[class] {
            match base.map(|base| self.lookup(base, name, depth + 1)) {
                None | Some(Lookup::Unknown) => return Lookup::Unknown,
                Some(Lookup::Found(id)) => return Lookup::Found(id),
                Some(Lookup::Missing) => {}
            }
        }
        Lookup::Missing
    }

    /// A bare name as Python looks it up from `scope`: enclosing function
    /// bodies (not class bodies), then the module.
    fn resolve_name(&self, file: usize, mut scope: usize, name: &str) -> Target {
        let scopes = &self.index.files[file].scopes;
        while scope != 0 {
            let current = &scopes[scope];
            if current.class.is_none() {
                match (current.defs.get(name), current.bindings.get(name)) {
                    (Some(Some(id)), None) => return Target::Function(*id),
                    (None, Some(Some(binding))) => return self.resolver.binding(binding),
                    (None, None) => {}
                    _ => return Target::Unknown,
                }
            }
            scope = current.parent;
        }
        self.resolver.local(file, name)
    }

    fn resolve_parts(&self, file: usize, scope: usize, parts: &[String]) -> Target {
        let Some((head, rest)) = parts.split_first() else { return Target::Unknown };
        let head = self.resolve_name(file, scope, head);
        rest.iter().fold(head, |target, part| self.resolver.member(target, part))
    }

    fn resolve_call(&self, call: &'a Call) -> Resolved<'a> {
        let init = |class: ClassId| match self.lookup(class, "__init__", 0) {
            Lookup::Found(id) => Resolved::Function(id, 1),
            Lookup::Unknown => Resolved::Possible("__init__"),
            Lookup::Missing => Resolved::Nothing,
        };
        let method = |lookup: Lookup, name: &'a str, bound: bool| match lookup {
            Lookup::Found(id) => {
                let receiver = match self.index.functions[id].kind {
                    FnKind::Method => bound,
                    FnKind::ClassMethod => true,
                    FnKind::StaticMethod | FnKind::Function => false,
                };
                Resolved::Function(id, usize::from(receiver))
            }
            Lookup::Unknown | Lookup::Missing => Resolved::Possible(name),
        };
        match &call.callee {
            Callee::Name(name) => match self.resolve_name(call.file, call.scope, name) {
                Target::Function(id) => Resolved::Function(id, 0),
                Target::Class(class) => init(class),
                // A star import may have brought the name in.
                Target::Unknown if self.index.files[call.file].has_star_import => {
                    Resolved::Possible(name)
                }
                _ => Resolved::Nothing,
            },
            Callee::Dotted(parts, name) => match self.resolve_parts(call.file, call.scope, parts) {
                object @ Target::Module(_) => match self.resolver.member(object, name) {
                    Target::Function(id) => Resolved::Function(id, 0),
                    Target::Class(class) => init(class),
                    _ => Resolved::Nothing,
                },
                Target::Class(class) => method(self.lookup(class, name, 0), name, false),
                Target::External => Resolved::Nothing,
                _ => Resolved::Possible(name),
            },
            Callee::SelfMethod(class, name) => method(self.lookup(*class, name, 0), name, true),
            Callee::Super(class, name) => method(self.lookup_bases(*class, name, 0), name, true),
            Callee::Unknown(name) => Resolved::Possible(name),
        }
    }

    /// Precision guards that do not depend on the call sites' values.
    fn exempt(&self, id: FnId, config: &Config) -> bool {
        let function = &self.index.functions[id];
        let file = &self.index.files[function.file];
        let name = function.name.as_str();
        let is_dunder = name.starts_with("__") && name.ends_with("__");
        file.is_test
            || function.keep == Keep::WithReason
            || function.decorators.iter().any(|decorator| !ignored(decorator, config))
            || function.is_abstract
            || (is_dunder && name != "__init__")
            || self.index.all_names.contains(name)
            || self.index.strings.contains(name)
            || self.index.refs.contains(name)
            // A class passed around is instantiated where we cannot see.
            || (name == "__init__"
                && function.class.is_some_and(|class| {
                    self.index.refs.contains(&self.index.classes[class].name)
                }))
            || self.reexported.contains(&id)
            || self.overridden.contains(&id)
            || matches_any(&file.relative, &config.public)
            || function.class.is_some_and(|class| {
                self.protocols.contains(&class)
                    || self.lookup_bases(class, name, 0) != Lookup::Missing
            })
    }

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
            let file = &self.index.files[function.file];
            findings.push(Finding {
                check: ID,
                path: file.relative.clone(),
                line: function.line,
                name: function.qualname.clone(),
                param: param.name.clone(),
                suggest: format!("drop {}, use {value} inline", param.name),
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

/// A decorator `ignore-decorators` lists, by its dotted name or a dotted suffix
/// of it: `cache` covers `functools.cache`.
fn ignored(decorator: &str, config: &Config) -> bool {
    !decorator.is_empty()
        && config.ignore_decorators.iter().any(|entry| {
            decorator == entry
                || decorator.strip_suffix(entry.as_str()).is_some_and(|rest| rest.ends_with('.'))
        })
}

enum Resolved<'a> {
    Function(FnId, usize),
    Possible(&'a str),
    Nothing,
}

/// Bind `call`'s arguments to `function`'s parameters, the first `skip` of
/// which the call fills implicitly. `None` when the call cannot be a call to
/// `function`: a splat, too many arguments, an unknown keyword, a required
/// parameter left out.
fn bind<'c>(
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
