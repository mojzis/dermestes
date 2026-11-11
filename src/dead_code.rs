//! The `dead-code` check: a function, class or method family no production
//! code mentions by name.
//!
//! Unlike the caller-counting checks it does not resolve calls. Any
//! occurrence of the bare name in production outside the function's own body
//! keeps it alive: an identifier with any receiver, a string literal spelling
//! it, or a word in a non-Python file. That over-counts uses, so it loses only
//! recall.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::callers::{Callers, Calls, Lookup};
use crate::calls::{FnId, Function};
use crate::config::{matches_any, Config};
use crate::discovery::Tree;
use crate::index::{Binding, ClassId, Index, Keep};

pub const ID: &str = "dead-code";

/// One deletion candidate: a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub path: String,
    /// The `def` line.
    pub line: usize,
    /// Qualified: `Store.load`. For a family, its root method.
    pub name: String,
    /// `function`, `class`, or `family` (a method and its overrides).
    pub kind: &'static str,
    /// A family's definitions, root first: `path:line Class.method`.
    pub defs: Vec<String>,
    /// `unused` or `test-only`.
    pub form: &'static str,
    pub calls: Calls,
    /// `path:line` of each test line that mentions it.
    pub tests: Vec<String>,
    pub keep_missing_reason: bool,
    pub suggest: String,
}

/// Lines `start..=end` of a file: a definition's own body.
type Span = (usize, usize, usize);

/// Something that may be dead: its bare name and the bodies that are its own.
struct Candidate<'a> {
    name: &'a str,
    qualname: &'a str,
    kind: &'static str,
    spans: Vec<Span>,
    /// A family's definitions.
    defs: Vec<FnId>,
    /// The class a candidate method belongs to, or the candidate class.
    class: Option<ClassId>,
    keep_missing_reason: bool,
}

/// Class decorators that change what a class is, not who uses it.
const PLAIN_DECORATORS: [&str; 4] = ["dataclass", "runtime_checkable", "final", "total_ordering"];

/// Run the check over the whole index. `tree` gives the entry points, the
/// typed packages and the words of non-Python files. Sorted by path, then line.
pub fn run(callers: &Callers<'_>, config: &Config, tree: &Tree) -> Vec<Finding> {
    let index = callers.index;
    let entry: HashSet<FnId> =
        tree.entry_points.iter().filter_map(|value| callers.entry_point(value)).collect();
    let functions = (0..index.functions.len())
        .filter(|id| !entry.contains(id) && !callers.exempt(*id, config))
        .filter(|&id| target(index, tree, &index.functions[id]))
        .map(|id| {
            let function = &index.functions[id];
            Candidate {
                name: &function.name,
                qualname: &function.qualname,
                kind: "function",
                spans: vec![(function.file, function.line, function.end_line)],
                defs: Vec::new(),
                class: function.class,
                keep_missing_reason: function.keep == Keep::MissingReason,
            }
        });
    let mut candidates: Vec<Candidate<'_>> = functions.collect();
    candidates.extend(classes(callers, config, tree));
    candidates.extend(
        (0..index.functions.len())
            .filter(|id| !entry.contains(id))
            .filter_map(|id| family(callers, config, tree, id)),
    );
    let names: HashSet<&str> = candidates.iter().map(|candidate| candidate.name).collect();
    // Each candidate name's mentions: the file and the lines.
    let mut mentions: HashMap<&str, Vec<(usize, &[usize])>> = HashMap::new();
    for (file, info) in index.files.iter().enumerate() {
        for (name, lines) in &info.mentions {
            if let Some(name) = names.get(name.as_str()) {
                mentions.entry(name).or_default().push((file, lines));
            }
        }
    }
    let found: Vec<(&Candidate<'_>, Finding)> = candidates
        .iter()
        .filter_map(|candidate| {
            let mentions = mentions.get(candidate.name).map_or(&[][..], Vec::as_slice);
            dead(index, candidate, mentions).map(|finding| (candidate, finding))
        })
        .collect();
    // A dead class is one finding: its methods go with it.
    let dead_classes: HashSet<ClassId> = found
        .iter()
        .filter(|(candidate, _)| candidate.kind == "class")
        .filter_map(|(candidate, _)| candidate.class)
        .collect();
    let mut findings: Vec<Finding> = found
        .into_iter()
        .filter(|(candidate, _)| {
            candidate.kind != "function"
                || !candidate.class.is_some_and(|class| dead_classes.contains(&class))
        })
        .map(|(_, finding)| finding)
        .collect();
    findings.sort_by(|a, b| (&a.path, a.line).cmp(&(&b.path, b.line)));
    findings
}

/// Module-level classes whose hierarchy stays inside the repository and that
/// nothing can reach but by name.
fn classes<'a>(callers: &Callers<'a>, config: &Config, tree: &Tree) -> Vec<Candidate<'a>> {
    let index = callers.index;
    // `Base.__subclasses__()` finds classes nobody names.
    let walked = index.files.iter().any(|file| file.mentions.contains_key("__subclasses__"))
        || index.unparsed_words.contains("__subclasses__");
    if walked {
        return Vec::new();
    }
    // A module no production file imports is run by something else (a script,
    // a benchmark or spider runner), which may find its classes by itself.
    let mut imported: HashSet<String> = HashSet::new();
    for file in index.files.iter().filter(|file| !file.is_test) {
        let local = file.scopes.iter().flat_map(|scope| scope.bindings.values().flatten());
        for binding in file.bindings.values().chain(local) {
            match binding {
                Binding::Module(module) => imported.extend([module.clone()]),
                Binding::Symbol { module, name } => {
                    imported.extend([module.clone(), format!("{module}.{name}")]);
                }
                Binding::Unknown => {}
            }
        }
    }
    let hooked = |id: &ClassId| index.classes[*id].methods.contains("__init_subclass__");
    let plain = |decorator: &String| {
        PLAIN_DECORATORS.contains(&decorator.rsplit('.').next().unwrap_or_default())
    };
    let classes = index.classes.iter().enumerate().filter(|(id, class)| {
        let file = &index.files[class.file];
        let name = class.name.as_str();
        let typed = !name.starts_with('_')
            && tree.typed.iter().any(|dir| file.relative.starts_with(dir.as_str()));
        class.top_level
            && !file.is_test
            && !file.evidence_only
            && class.keep != Keep::WithReason
            && !matches_any(&file.relative, &config.public)
            && class.decorators.iter().all(plain)
            && !typed
            && !tree.words.contains(name)
            && !index.unparsed_words.contains(name)
            && file.modules.iter().any(|module| imported.contains(module))
            && !callers.unresolved_hierarchy(*id)
            && !callers.ancestors(*id).iter().any(hooked)
    });
    classes
        .map(|(id, class)| Candidate {
            name: &class.name,
            qualname: &class.qualname,
            kind: "class",
            spans: vec![(class.file, class.line, class.end_line)],
            defs: Vec::new(),
            class: Some(id),
            keep_missing_reason: class.keep == Keep::MissingReason,
        })
        .collect()
}

/// The family rooted at method `id`: it and its overrides in repository
/// subclasses, when no ancestor defines the name and the hierarchy resolves.
fn family<'a>(
    callers: &Callers<'a>,
    config: &Config,
    tree: &Tree,
    id: FnId,
) -> Option<Candidate<'a>> {
    let index = callers.index;
    let root = &index.functions[id];
    let class = root.class?;
    let name = root.name.as_str();
    if callers.lookup_bases(class, name, 0) != Lookup::Missing
        || callers.unresolved_hierarchy(class)
    {
        return None;
    }
    let defs: Option<Vec<FnId>> = callers
        .family(class)
        .iter()
        .filter_map(|member| callers.methods.get(&(*member, name)).copied())
        .collect();
    let defs = defs.filter(|defs| defs.len() > 1 && defs.first() == Some(&id))?;
    let live = |def: &FnId| {
        callers.guarded_alone(*def, config) || !target(index, tree, &index.functions[*def])
    };
    if defs.iter().any(live) {
        return None;
    }
    let functions = defs.iter().map(|def| &index.functions[*def]);
    Some(Candidate {
        name,
        qualname: &root.qualname,
        kind: "family",
        spans: functions.clone().map(|f| (f.file, f.line, f.end_line)).collect(),
        keep_missing_reason: functions.clone().any(|f| f.keep == Keep::MissingReason),
        defs,
        class: Some(class),
    })
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

/// The finding for `candidate` when no production line outside its own
/// bodies mentions its name. A family is dead only if no test does either.
fn dead(
    index: &Index,
    candidate: &Candidate<'_>,
    mentions: &[(usize, &[usize])],
) -> Option<Finding> {
    let own = |file: usize, line: usize| {
        candidate.spans.iter().any(|&(f, start, end)| f == file && (start..=end).contains(&line))
    };
    let mut tests = Vec::new();
    for &(file, lines) in mentions {
        let info = &index.files[file];
        for &line in lines.iter().filter(|&&line| !own(file, line)) {
            if !info.is_test || candidate.kind == "family" {
                return None;
            }
            tests.push((info.relative.as_str(), line));
        }
    }
    tests.sort_unstable();
    let name = candidate.qualname;
    let at = |&(file, line, _): &Span| (index.files[file].relative.as_str(), line);
    let defs: Vec<String> = candidate
        .defs
        .iter()
        .map(|def| &index.functions[*def])
        .map(|f| format!("{}:{} {}", index.files[f.file].relative, f.line, f.qualname))
        .collect();
    let suggest = match (tests.as_slice(), defs.len()) {
        ([], 0) => format!("delete {name}"),
        ([], n) => {
            let class = name.rsplit_once('.').map_or(name, |(class, _)| class);
            let overrides =
                if n == 2 { "its override".to_owned() } else { format!("its {} overrides", n - 1) };
            format!("delete {} from {class} and {overrides}", candidate.name)
        }
        ([(first, _), rest @ ..], _) => {
            let into =
                if rest.iter().all(|(path, _)| path == first) { first } else { "a test helper" };
            format!(
                "delete {name} and the tests that only exercise it, \
                 or move it into {into} if they use it as setup"
            )
        }
    };
    let (path, line) = candidate.spans.first().map(at)?;
    Some(Finding {
        check: ID,
        path: path.to_owned(),
        line,
        name: name.to_owned(),
        kind: candidate.kind,
        defs,
        form: if tests.is_empty() { "unused" } else { "test-only" },
        calls: Calls { prod: 0, test: tests.len() },
        tests: tests.iter().map(|(path, line)| format!("{path}:{line}")).collect(),
        keep_missing_reason: candidate.keep_missing_reason,
        suggest,
    })
}
