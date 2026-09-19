//! The `one-impl` check: an ABC or Protocol with exactly one production
//! implementation and no test implementations.

use std::collections::{BTreeSet, HashSet};

use serde::Serialize;

use crate::config::{matches_any, Config};
use crate::index::{ClassId, Index, Keep};
use crate::resolve::{Resolver, Target};

pub const ID: &str = "one-impl";

/// A class definition a finding points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Site {
    pub path: String,
    pub line: usize,
    pub name: String,
}

/// One deletion candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    #[serde(flatten)]
    pub site: Site,
    /// `ABC` or `Protocol`.
    pub kind: &'static str,
    /// Abstract methods of an ABC, or methods of a Protocol.
    pub members: usize,
    #[serde(rename = "impl")]
    pub implementation: Site,
    pub test_impls: usize,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Method sets per class, inherited through resolved bases.
#[derive(Debug, Clone, Default)]
struct Methods {
    /// Every method name, own and inherited.
    all: BTreeSet<String>,
    /// Methods with a concrete definition somewhere in the ancestry.
    concrete: BTreeSet<String>,
    /// Abstract methods not yet given a concrete definition.
    remaining: BTreeSet<String>,
}

/// Everything the check derives about one class from its resolved bases.
struct Facts {
    parents: Vec<ClassId>,
    children: Vec<ClassId>,
    /// Some base or the metaclass could not be resolved.
    unknown_edge: bool,
    /// Inherits `ABC` directly or uses `metaclass=ABCMeta`. Alone it does not
    /// make a class abstract: with no abstract methods it is instantiable.
    abc_marker: bool,
    is_protocol: bool,
    methods: Methods,
}

impl Facts {
    fn is_abstract(&self) -> bool {
        self.is_protocol || !self.methods.remaining.is_empty()
    }
}

struct Check<'a> {
    index: &'a Index,
    facts: Vec<Facts>,
    /// Last segment of every base expression that did not resolve: a class of
    /// that name might be subclassed where we cannot see it.
    unresolved_names: HashSet<&'a str>,
    /// Classes re-exported from some `__init__.py`.
    reexported: HashSet<ClassId>,
}

/// Run the check over the whole index. Findings are sorted by path, then line.
pub fn run(index: &Index, config: &Config) -> Vec<Finding> {
    let check = Check::new(index);
    let mut findings: Vec<Finding> = (0..index.classes.len())
        .filter(|&id| !check.exempt(id, config))
        .filter_map(|id| check.finding(id))
        .collect();
    findings.sort_by(|a, b| (&a.site.path, a.site.line).cmp(&(&b.site.path, b.site.line)));
    findings
}

impl<'a> Check<'a> {
    fn new(index: &'a Index) -> Self {
        let resolver = Resolver::new(index);
        let mut unresolved_names = HashSet::new();
        let mut facts: Vec<Facts> = index
            .classes
            .iter()
            .map(|class| {
                let mut fact = Facts {
                    parents: Vec::new(),
                    children: Vec::new(),
                    unknown_edge: false,
                    abc_marker: false,
                    is_protocol: false,
                    methods: Methods::default(),
                };
                for base in &class.bases {
                    match resolver.dotted(class.file, base) {
                        Target::Class(parent) => fact.parents.push(parent),
                        Target::Abc => fact.abc_marker = true,
                        Target::Protocol => fact.is_protocol = true,
                        Target::Object => {}
                        Target::AbcMeta | Target::Module(_) | Target::Unknown => {
                            fact.unknown_edge = true;
                            unresolved_names.extend(base.last().map(String::as_str));
                        }
                    }
                }
                match &class.metaclass {
                    None => {}
                    Some(parts) if resolver.dotted(class.file, parts) == Target::AbcMeta => {
                        fact.abc_marker = true;
                    }
                    Some(_) => fact.unknown_edge = true,
                }
                fact
            })
            .collect();

        for id in 0..facts.len() {
            for parent in facts[id].parents.clone() {
                facts[parent].children.push(id);
            }
        }
        let mut done = vec![false; facts.len()];
        for id in 0..facts.len() {
            compute_methods(index, &mut facts, &mut done, id);
        }

        let reexported = index
            .files
            .iter()
            .filter(|file| file.is_init)
            .flat_map(|file| file.bindings.values())
            .filter_map(|binding| match resolver.binding(binding) {
                Target::Class(id) => Some(id),
                _ => None,
            })
            .collect();

        Self { index, facts, unresolved_names, reexported }
    }

    /// Precision guards that apply whatever the hierarchy looks like.
    fn exempt(&self, id: ClassId, config: &Config) -> bool {
        let class = &self.index.classes[id];
        let file = &self.index.files[class.file];
        file.is_test
            || class.keep == Keep::WithReason
            || self.index.all_names.contains(&class.name)
            || self.index.strings.contains(&class.name)
            || self.index.registered.contains(&class.name)
            || self.reexported.contains(&id)
            || matches_any(&file.relative, &config.public)
    }

    /// Every class below `id`, and whether any of them (or `id`) has an unknown edge.
    fn descendants(&self, id: ClassId) -> (Vec<ClassId>, bool) {
        let mut seen = HashSet::from([id]);
        let mut stack = vec![id];
        let mut out = Vec::new();
        let mut unknown = self.facts[id].unknown_edge;
        while let Some(current) = stack.pop() {
            for &child in &self.facts[current].children {
                if seen.insert(child) {
                    unknown |= self.facts[child].unknown_edge;
                    out.push(child);
                    stack.push(child);
                }
            }
        }
        (out, unknown)
    }

    fn finding(&self, id: ClassId) -> Option<Finding> {
        let class = &self.index.classes[id];
        let fact = &self.facts[id];
        let declares_abstract = (fact.abc_marker || !class.abstract_methods.is_empty())
            && !fact.methods.remaining.is_empty();
        if !(fact.is_protocol || declares_abstract)
            || self.unresolved_names.contains(class.name.as_str())
        {
            return None;
        }
        let (descendants, unknown) = self.descendants(id);
        if unknown {
            return None;
        }
        let concrete = |c: &ClassId| !self.facts[*c].is_abstract();
        let mut impls: BTreeSet<ClassId> = descendants.iter().copied().filter(concrete).collect();

        let (kind, members, verb) = if fact.is_protocol {
            let methods = &fact.methods.all;
            let lone_dunder = methods.len() == 1 && methods.iter().all(|m| m.starts_with("__"));
            if methods.is_empty() || lone_dunder {
                return None;
            }
            for (other, other_fact) in self.facts.iter().enumerate() {
                if other == id || other_fact.is_abstract() {
                    continue;
                }
                if other_fact.methods.all.is_superset(methods) {
                    impls.insert(other);
                } else if other_fact.unknown_edge && !other_fact.methods.all.is_disjoint(methods) {
                    // It may inherit the rest from a base we cannot see.
                    return None;
                }
            }
            ("Protocol", methods.len(), "use {impl} directly")
        } else {
            ("ABC", fact.methods.remaining.len(), "inline into {impl}")
        };

        let (tests, production): (Vec<ClassId>, Vec<ClassId>) =
            impls.into_iter().partition(|&c| self.index.files[self.index.classes[c].file].is_test);
        let [only] = production[..] else { return None };
        if !tests.is_empty() {
            return None;
        }
        let implementation = self.site(only);
        Some(Finding {
            check: ID,
            site: self.site(id),
            kind,
            members,
            suggest: format!(
                "{}, delete {}",
                verb.replace("{impl}", &implementation.name),
                class.qualname
            ),
            implementation,
            test_impls: tests.len(),
            keep_missing_reason: class.keep == Keep::MissingReason,
        })
    }

    fn site(&self, id: ClassId) -> Site {
        let class = &self.index.classes[id];
        Site {
            path: self.index.files[class.file].relative.clone(),
            line: class.line,
            name: class.qualname.clone(),
        }
    }
}

/// Fill in `facts[id].methods` after its parents'. A cycle in the (bogus)
/// hierarchy sees the unfinished parent as empty rather than looping.
fn compute_methods(index: &Index, facts: &mut [Facts], done: &mut [bool], id: ClassId) {
    if done[id] {
        return;
    }
    done[id] = true;
    let parents = facts[id].parents.clone();
    for &parent in &parents {
        compute_methods(index, facts, done, parent);
    }
    let class = &index.classes[id];
    let own_abstract = &class.abstract_methods;
    let mut methods = Methods { all: class.methods.clone(), ..Methods::default() };
    methods.concrete = class.methods.difference(own_abstract).cloned().collect();
    let mut inherited_remaining = BTreeSet::new();
    for &parent in &parents {
        let inherited = &facts[parent].methods;
        methods.all.extend(inherited.all.iter().cloned());
        methods.concrete.extend(inherited.concrete.difference(own_abstract).cloned());
        inherited_remaining.extend(inherited.remaining.iter().cloned());
    }
    methods.remaining.clone_from(own_abstract);
    methods.remaining.extend(inherited_remaining.difference(&methods.concrete).cloned());
    facts[id].methods = methods;
}
