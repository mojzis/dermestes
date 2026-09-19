//! Call sites resolved to repository functions, and the precision guards
//! every caller-counting check shares. `const-param` and `pass-through` add
//! their own methods to [`Callers`] in their modules.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::calls::{Call, Callee, FnId, FnKind};
use crate::config::{matches_any, Config};
use crate::index::{ClassId, Index, Keep};
use crate::resolve::{Resolver, Target};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Calls {
    pub prod: usize,
    pub test: usize,
}

/// Where a method name leads from a class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lookup {
    Found(FnId),
    /// Somewhere along the way a base, or the name itself, is ambiguous.
    Unknown,
    Missing,
}

/// A call resolved to a repository function, with how many leading
/// parameters the call binds implicitly (`self` or `cls`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Site {
    pub(crate) call: usize,
    pub(crate) skip: usize,
}

pub struct Callers<'a> {
    pub(crate) index: &'a Index,
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
    /// Functions of a module or class `getattr` reads a computed name from.
    dynamic: HashSet<FnId>,
    pub(crate) sites: Vec<Vec<Site>>,
    /// Calls we could not resolve, by the name they call.
    pub(crate) possible: HashMap<&'a str, Vec<usize>>,
}

impl<'a> Callers<'a> {
    pub(crate) fn new(index: &'a Index) -> Self {
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
            dynamic: HashSet::new(),
            sites: vec![Vec::new(); index.functions.len()],
            possible: HashMap::new(),
        };
        check.overridden = check.find_overridden();
        check.dynamic = check.find_dynamic();
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

    fn find_dynamic(&self) -> HashSet<FnId> {
        let mut files = HashSet::new();
        let mut classes = HashSet::new();
        for (file, scope, parts) in &self.index.dynamic {
            match self.resolve_parts(*file, *scope, parts) {
                Target::Module(module) => files.extend(self.resolver.module_file(&module)),
                Target::Class(class) => {
                    classes.insert(class);
                }
                _ => {}
            }
        }
        let functions = self.index.functions.iter().enumerate();
        functions
            .filter(|(_, f)| match f.class {
                Some(class) => classes.contains(&class),
                None => files.contains(&f.file),
            })
            .map(|(id, _)| id)
            .collect()
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

    pub(crate) fn lookup_bases(&self, class: ClassId, name: &str, depth: usize) -> Lookup {
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

    pub(crate) fn resolve_parts(&self, file: usize, scope: usize, parts: &[String]) -> Target {
        let Some((head, rest)) = parts.split_first() else { return Target::Unknown };
        let head = self.resolve_name(file, scope, head);
        rest.iter().fold(head, |target, part| self.resolver.member(target, part))
    }

    pub(crate) fn resolve_call(&self, call: &'a Call) -> Resolved<'a> {
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
    pub(crate) fn exempt(&self, id: FnId, config: &Config) -> bool {
        self.guarded(id, config) || self.overrides(id)
    }

    /// A method a base may define: an override, or a method of a class with a
    /// base we cannot follow.
    pub(crate) fn overrides(&self, id: FnId) -> bool {
        let function = &self.index.functions[id];
        function
            .class
            .is_some_and(|class| self.lookup_bases(class, &function.name, 0) != Lookup::Missing)
    }

    /// Every guard of [`Self::exempt`] but [`Self::overrides`].
    pub(crate) fn guarded(&self, id: FnId, config: &Config) -> bool {
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
            || self.dynamic.contains(&id)
            || matches_any(&file.relative, &config.public)
            || function.class.is_some_and(|class| self.protocols.contains(&class))
    }

    /// The function a `[project.scripts]` or `[project.entry-points]` value
    /// (`pkg.cli:main`, `pkg.cli:App.run`) names.
    pub(crate) fn entry_point(&self, value: &str) -> Option<FnId> {
        let value = value.split('[').next().unwrap_or(value).trim();
        let (module, attr) = value.split_once(':')?;
        let target = attr
            .trim()
            .split('.')
            .fold(Target::Module(module.trim().to_owned()), |target, part| {
                self.resolver.member(target, part)
            });
        match target {
            Target::Function(id) => Some(id),
            _ => None,
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

pub(crate) enum Resolved<'a> {
    Function(FnId, usize),
    Possible(&'a str),
    Nothing,
}

/// `path:line` sites for a suggestion: up to three, then `+N more`.
pub(crate) fn listed(sites: &[String]) -> String {
    let mut listed: Vec<String> = sites.iter().take(3).cloned().collect();
    if sites.len() > 3 {
        listed.push(format!("+{} more", sites.len() - 3));
    }
    listed.join(", ")
}
