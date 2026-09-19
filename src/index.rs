//! Parse every file and extract what the checks need.
//!
//! Per file: module names, the module-level import table, every class,
//! `__all__`, names in dynamic-lookup strings, and `X.register` mentions. The tree-sitter
//! setup is biston's `parse.rs`.
//!
//! Nothing here resolves a name across files; that is [`crate::resolve`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use anyhow::{Context, Result};
use rayon::prelude::*;
use tree_sitter::Node;

use crate::calls::{Call, Callee, FnId, Function, Scope, TopName};
use crate::config::{matches_any, Config};
use crate::discovery::SourcePath;

/// Position of a class in [`Index::classes`].
pub type ClassId = usize;

/// What a module-level name was bound to by an import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// `import a.b` binds `a` to module `a`; `import a.b as c` binds `c` to `a.b`.
    Module(String),
    /// `from m import name`: a symbol of `m`, or its submodule `m.name`.
    Symbol { module: String, name: String },
    /// A relative import that climbs above the top-level package.
    Unknown,
}

/// A `# dermestes: keep` marker on a definition's header.
/// Ordered so the strongest marker on a multi-line header wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Keep {
    None,
    WithReason,
    MissingReason,
}

/// A class definition, anywhere in a file.
#[derive(Debug, Clone)]
pub struct Class {
    pub file: usize,
    pub name: String,
    /// Dotted through enclosing classes and functions: `make.LocalStep`.
    pub qualname: String,
    pub line: usize,
    /// Defined at module level, so other files can import it.
    pub top_level: bool,
    /// Each base as dotted parts; empty when it is not a dotted name (a call,
    /// `*bases`), which resolves to unknown.
    pub bases: Vec<Vec<String>>,
    /// The `metaclass=` keyword, as for `bases`.
    pub metaclass: Option<Vec<String>>,
    pub methods: BTreeSet<String>,
    pub abstract_methods: BTreeSet<String>,
    pub keep: Keep,
}

/// What a module-level name refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Class(ClassId),
    Function(FnId),
    /// Assigned a literal exactly once: a module constant.
    Constant,
}

/// One indexed file.
#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    pub relative: String,
    /// Every dotted module name this file can be imported as (flat and `src/`).
    pub modules: Vec<String>,
    pub is_init: bool,
    pub is_test: bool,
    pub bindings: HashMap<String, Binding>,
    pub has_star_import: bool,
    /// Module-level definitions; `None` when a name is bound more than once,
    /// or by anything but a class, a function or a literal.
    pub top: HashMap<String, Option<Symbol>>,
    /// Function and class bodies; `scopes[0]` is the module and stays empty.
    pub scopes: Vec<Scope>,
}

/// The whole repository.
#[derive(Debug, Default)]
pub struct Index {
    pub files: Vec<FileInfo>,
    pub classes: Vec<Class>,
    /// Names listed in any `__all__`.
    pub all_names: HashSet<String>,
    /// Names in string arguments to `getattr`/`setattr`/`hasattr`/`patch`/`patch.object`
    /// (last segment of a dotted path), and string keys of registry dict entries.
    pub strings: HashSet<String>,
    /// `X` for every `X.register` seen: ABC virtual subclass registration.
    pub registered: HashSet<String>,
    pub functions: Vec<Function>,
    pub calls: Vec<Call>,
    /// Every name used as a value rather than called: `f` in `register(f)`,
    /// `b` in `a.b`. A function of that name has call sites we cannot see.
    pub refs: HashSet<String>,
}

/// Per-file extraction output before class ids are assigned.
#[derive(Debug, Clone)]
struct Extracted {
    info: FileInfo,
    classes: Vec<Class>,
    all_names: Vec<String>,
    strings: Vec<String>,
    registered: Vec<String>,
    functions: Vec<Function>,
    calls: Vec<Call>,
    refs: Vec<String>,
    top_names: Vec<(String, TopName)>,
}

/// Every file parsed, before the index is assembled. Diff mode derives the
/// base side from the head side by re-parsing only the files the diff changed.
#[derive(Debug, Clone)]
pub struct Parsed {
    roots: Vec<String>,
    files: BTreeMap<String, Extracted>,
}

impl Parsed {
    /// Parse and extract every file in parallel. A file that cannot be read or
    /// parsed is skipped with a note on stderr.
    ///
    /// `projects` are the directories holding a `pyproject.toml`, as `/`-ended
    /// prefixes; each is an import root, as is the repository root.
    pub fn new(files: &[SourcePath], projects: &[String], config: &Config) -> Self {
        let roots = import_roots(files, projects);
        let extracted: Vec<Result<Extracted>> = files
            .par_iter()
            .map(|file| {
                let source = std::fs::read_to_string(&file.path)
                    .with_context(|| format!("cannot read {}", file.relative))?;
                extract(&file.relative, &source, &roots, config)
            })
            .collect();
        let mut parsed = Self { roots, files: BTreeMap::new() };
        for result in extracted {
            match result {
                Ok(extracted) => {
                    parsed.files.insert(extracted.info.relative.clone(), extracted);
                }
                Err(err) => eprintln!("dermestes: skipped: {err:#}"),
            }
        }
        parsed
    }

    /// The same tree with some files replaced (`Some(source)`) or removed
    /// (`None`). A replacement that does not parse is dropped quietly: its
    /// head side already had its note.
    #[must_use]
    pub fn with_sources(&self, sources: &HashMap<String, Option<String>>, config: &Config) -> Self {
        let replaced: Vec<Extracted> = sources
            .par_iter()
            .filter_map(|(relative, source)| {
                let is_python =
                    std::path::Path::new(relative).extension().is_some_and(|ext| ext == "py")
                        && !matches_any(relative, &config.exclude);
                let source = source.as_deref().filter(|_| is_python)?;
                extract(relative, source, &self.roots, config).ok()
            })
            .collect();
        let mut parsed = self.clone();
        for relative in sources.keys() {
            parsed.files.remove(relative);
        }
        for extracted in replaced {
            parsed.files.insert(extracted.info.relative.clone(), extracted);
        }
        parsed
    }

    /// Assign ids and merge, in path order.
    pub fn into_index(self) -> Index {
        let mut index = Index::default();
        for extracted in self.files.into_values() {
            index.add(extracted);
        }
        index
    }
}

impl Index {
    /// Parse `files` and assemble the index in one go.
    pub fn build(files: &[SourcePath], projects: &[String], config: &Config) -> Self {
        Parsed::new(files, projects, config).into_index()
    }

    /// Append one file, shifting its file-local class and function positions
    /// to index-wide ids.
    fn add(&mut self, extracted: Extracted) {
        let file = self.files.len();
        let (class_base, fn_base) = (self.classes.len(), self.functions.len());
        let mut info = extracted.info;
        for mut class in extracted.classes {
            class.file = file;
            self.classes.push(class);
        }
        for mut function in extracted.functions {
            function.file = file;
            function.class = function.class.map(|class| class + class_base);
            self.functions.push(function);
        }
        for mut call in extracted.calls {
            call.file = file;
            if let Callee::SelfMethod(class, _) | Callee::Super(class, _) = &mut call.callee {
                *class += class_base;
            }
            self.calls.push(call);
        }
        for scope in &mut info.scopes {
            scope.class = scope.class.map(|class| class + class_base);
            if let Some((class, _)) = &mut scope.method {
                *class += class_base;
            }
            for def in scope.defs.values_mut().flatten() {
                *def += fn_base;
            }
        }
        for (name, top) in extracted.top_names {
            let symbol = match top {
                TopName::Class(class) => Some(Symbol::Class(class + class_base)),
                TopName::Function(function) => Some(Symbol::Function(function + fn_base)),
                TopName::Constant => Some(Symbol::Constant),
                TopName::Other => None,
            };
            info.top.entry(name).and_modify(|slot| *slot = None).or_insert(symbol);
        }
        for name in info.bindings.keys() {
            if let Some(slot) = info.top.get_mut(name) {
                *slot = None;
            }
        }
        self.files.push(info);
        self.all_names.extend(extracted.all_names);
        self.strings.extend(extracted.strings);
        self.registered.extend(extracted.registered);
        self.refs.extend(extracted.refs);
    }
}

/// Directory prefixes packages import from: the repository root and every
/// project directory, each plus its `src/` when files live there.
fn import_roots(files: &[SourcePath], projects: &[String]) -> Vec<String> {
    let mut roots: Vec<String> =
        std::iter::once(String::new()).chain(projects.iter().cloned()).collect();
    roots.sort();
    roots.dedup();
    let with_src: Vec<String> = roots
        .iter()
        .map(|root| format!("{root}src/"))
        .filter(|src| files.iter().any(|file| file.relative.starts_with(src.as_str())))
        .collect();
    roots.extend(with_src);
    roots
}

/// Module names for a file: one per import root it lies under. Paths that are
/// not valid dotted names give none.
fn module_names(relative: &str, roots: &[String]) -> Vec<String> {
    let Some(stem) = relative.strip_suffix(".py") else { return Vec::new() };
    let stem = stem.strip_suffix("/__init__").unwrap_or(stem);
    let mut names: Vec<String> = roots
        .iter()
        .filter_map(|root| stem.strip_prefix(root.as_str()))
        .filter(|path| path.split('/').all(is_identifier) && *path != "__init__")
        .map(|path| path.replace('/', "."))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Parse one file and pull out everything the index keeps.
fn extract(relative: &str, source: &str, roots: &[String], config: &Config) -> Result<Extracted> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .context("failed to set Python language")?;
    let tree = parser.parse(source, None).context("tree-sitter returned no tree")?;
    let root = tree.root_node();
    if root.has_error() {
        anyhow::bail!("{relative}: syntax error");
    }

    let modules = module_names(relative, roots);
    let is_init = relative == "__init__.py" || relative.ends_with("/__init__.py");
    // The shortest name is the `src/`-rooted one, the name the package
    // imports itself by; relative imports resolve from it.
    let package = modules.iter().min_by_key(|name| name.len()).map(|module| {
        if is_init {
            module.clone()
        } else {
            module.rsplit_once('.').map_or(String::new(), |(parent, _)| parent.to_owned())
        }
    });

    let mut walker = Walker {
        source,
        lines: source.lines().collect(),
        package,
        info: FileInfo {
            relative: relative.to_owned(),
            modules,
            is_init,
            is_test: matches_any(relative, &config.test_paths),
            scopes: vec![Scope::default()],
            ..FileInfo::default()
        },
        classes: Vec::new(),
        all_names: Vec::new(),
        strings: Vec::new(),
        registered: Vec::new(),
        functions: Vec::new(),
        calls: Vec::new(),
        refs: Vec::new(),
        top_names: Vec::new(),
        scope: 0,
        under_main: false,
    };
    walker.visit(root, "", true);
    Ok(Extracted {
        info: walker.info,
        classes: walker.classes,
        all_names: walker.all_names,
        strings: walker.strings,
        registered: walker.registered,
        functions: walker.functions,
        calls: walker.calls,
        refs: walker.refs,
        top_names: walker.top_names,
    })
}

/// One file's walk. Positions in `classes` and `functions` are file-local
/// until `Index::add` shifts them.
pub(crate) struct Walker<'s> {
    source: &'s str,
    lines: Vec<&'s str>,
    /// The package relative imports start from; `None` if the file has no module name.
    package: Option<String>,
    pub(crate) info: FileInfo,
    classes: Vec<Class>,
    all_names: Vec<String>,
    strings: Vec<String>,
    pub(crate) registered: Vec<String>,
    pub(crate) functions: Vec<Function>,
    pub(crate) calls: Vec<Call>,
    pub(crate) refs: Vec<String>,
    pub(crate) top_names: Vec<(String, TopName)>,
    /// The scope being walked, in `info.scopes`.
    pub(crate) scope: usize,
    pub(crate) under_main: bool,
}

impl<'s> Walker<'s> {
    pub(crate) fn text(&self, node: Node<'_>) -> &'s str {
        &self.source[node.byte_range()]
    }

    /// Visit `node`; `prefix` is the enclosing qualname and `top` whether we
    /// are still in module scope (not inside a `def` or `class`).
    pub(crate) fn visit(&mut self, node: Node<'_>, prefix: &str, top: bool) {
        match node.kind() {
            "import_statement" | "import_from_statement" => return self.import(node),
            "class_definition" => return self.class(node, prefix, top),
            "function_definition" => return self.function(node, prefix, top),
            "assignment" | "augmented_assignment" => {
                if top {
                    self.dunder_all(node);
                }
                return self.assignment(node, prefix, top);
            }
            "for_statement" | "for_in_clause" => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.bind_targets(left, prefix, top, false);
                }
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if Some(child) != node.child_by_field_name("left") {
                        self.visit(child, prefix, top);
                    }
                }
                return;
            }
            // Only an f-string's interpolations are code.
            "string" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "interpolation" {
                        self.visit(child, prefix, top);
                    }
                }
                return;
            }
            "type" | "global_statement" | "nonlocal_statement" => return,
            "call" => return self.call(node, prefix, top),
            "attribute" => return self.attribute(node, prefix, top, false),
            "identifier" => return self.refs.push(self.text(node).to_owned()),
            "keyword_argument" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.visit(value, prefix, top);
                }
                return;
            }
            "if_statement" if top && self.is_main_guard(node) => {
                self.under_main = true;
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.visit(child, prefix, top);
                }
                self.under_main = false;
                return;
            }
            // A registry entry, `{"csv": CsvExporter}`: the key names a symbol
            // looked up at runtime. `{"done": 3}` is data, not a lookup.
            "pair" => {
                let is_reference = node.child_by_field_name("value").is_some_and(|value| {
                    matches!(value.kind(), "identifier" | "attribute" | "lambda")
                });
                let key = node.child_by_field_name("key").and_then(|key| self.string(key));
                self.strings.extend(key.filter(|key| is_reference && is_identifier(key)));
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, prefix, top);
        }
    }

    fn import(&mut self, node: Node<'_>) {
        let mut cursor = node.walk();
        if node.kind() == "import_statement" {
            for item in node.children_by_field_name("name", &mut cursor) {
                if item.kind() == "aliased_import" {
                    let (Some(name), Some(alias)) =
                        (item.child_by_field_name("name"), item.child_by_field_name("alias"))
                    else {
                        continue;
                    };
                    let module = self.text(name).to_owned();
                    self.bind(self.text(alias), Binding::Module(module));
                } else {
                    let module = self.text(item);
                    let head = module.split('.').next().unwrap_or(module);
                    self.bind(head, Binding::Module(head.to_owned()));
                }
            }
            return;
        }

        let module = node.child_by_field_name("module_name").map(|n| self.absolute_module(n));
        let Some(module) = module else { return };
        if node.named_children(&mut cursor).any(|child| child.kind() == "wildcard_import") {
            self.info.has_star_import = true;
            return;
        }
        for item in node.children_by_field_name("name", &mut cursor) {
            let (name, local) = if item.kind() == "aliased_import" {
                let name = item.child_by_field_name("name").map(|n| self.text(n));
                (name, item.child_by_field_name("alias").map(|n| self.text(n)))
            } else {
                (Some(self.text(item)), Some(self.text(item)))
            };
            let (Some(name), Some(local)) = (name, local) else { continue };
            let binding = module.as_ref().map_or(Binding::Unknown, |module| Binding::Symbol {
                module: module.clone(),
                name: name.to_owned(),
            });
            self.bind(local, binding);
        }
    }

    /// The absolute module a `from X import` refers to; `None` if a relative
    /// import cannot be anchored.
    fn absolute_module(&self, node: Node<'_>) -> Option<String> {
        if node.kind() != "relative_import" {
            return Some(self.text(node).to_owned());
        }
        let text = self.text(node);
        let rest = text.trim_start_matches('.');
        let dots = text.len() - rest.len();
        let mut package = self.package.as_deref()?;
        for _ in 1..dots {
            package = package.rsplit_once('.')?.0;
        }
        Some(match (package.is_empty(), rest.is_empty()) {
            (_, true) => package.to_owned(),
            (true, false) => rest.to_owned(),
            (false, false) => format!("{package}.{rest}"),
        })
    }

    /// An import binds at module level, or locally inside a function.
    fn bind(&mut self, local: &str, binding: Binding) {
        if self.scope == 0 {
            self.info.bindings.insert(local.to_owned(), binding);
        } else {
            self.info.scopes[self.scope].bindings.insert(local.to_owned(), Some(binding));
        }
    }

    fn class(&mut self, node: Node<'_>, prefix: &str, top: bool) {
        let Some(name) = node.child_by_field_name("name").map(|n| self.text(n)) else { return };
        let qualname = join(prefix, name);
        let mut bases = Vec::new();
        let mut metaclass = None;
        if let Some(args) = node.child_by_field_name("superclasses") {
            let mut cursor = args.walk();
            for arg in args.named_children(&mut cursor) {
                if arg.kind() == "keyword_argument" {
                    let key = arg.child_by_field_name("name").map(|n| self.text(n));
                    if key == Some("metaclass") {
                        let value = arg.child_by_field_name("value");
                        metaclass = Some(value.and_then(|n| dotted(self, n)).unwrap_or_default());
                    }
                } else if arg.kind() != "comment" {
                    bases.push(dotted(self, arg).unwrap_or_default());
                }
            }
        }

        let mut methods = BTreeSet::new();
        let mut abstract_methods = BTreeSet::new();
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for item in body.named_children(&mut cursor) {
                let (function, is_abstract) = match item.kind() {
                    "function_definition" => (Some(item), false),
                    "decorated_definition" => {
                        let mut inner = item.walk();
                        let is_abstract = item
                            .named_children(&mut inner)
                            .filter(|child| child.kind() == "decorator")
                            .any(|decorator| self.is_abstract_decorator(decorator));
                        (item.child_by_field_name("definition"), is_abstract)
                    }
                    _ => (None, false),
                };
                let method = function
                    .filter(|f| f.kind() == "function_definition")
                    .and_then(|f| f.child_by_field_name("name"))
                    .map(|n| self.text(n).to_owned());
                if let Some(method) = method {
                    if is_abstract {
                        abstract_methods.insert(method.clone());
                    }
                    methods.insert(method);
                }
            }
        }

        let line = node.start_position().row;
        let id = self.classes.len();
        if top {
            self.top_names.push((name.to_owned(), TopName::Class(id)));
        } else {
            self.info.scopes[self.scope].bindings.insert(name.to_owned(), None);
        }
        self.classes.push(Class {
            file: 0,
            name: name.to_owned(),
            qualname: qualname.clone(),
            line: line + 1,
            top_level: top,
            bases,
            metaclass,
            methods,
            abstract_methods,
            keep: self.header_keep(node),
        });
        if let Some(body) = node.child_by_field_name("body") {
            let scope = Scope { parent: self.scope, class: Some(id), ..Scope::default() };
            self.in_scope(scope, |walker| walker.visit(body, &qualname, false));
        }
    }

    /// The `keep` marker anywhere in a definition's header: its decorators
    /// through the line with the closing `:`, which is where `ruff format`
    /// moves a comment when it wraps the header.
    pub(crate) fn header_keep(&self, node: Node<'_>) -> Keep {
        let start = node
            .parent()
            .filter(|parent| parent.kind() == "decorated_definition")
            .unwrap_or(node)
            .start_position()
            .row;
        let mut cursor = node.walk();
        let colon = node.children(&mut cursor).find(|child| child.kind() == ":");
        let end =
            colon.map_or(start, |colon| colon.start_position().row).max(node.start_position().row);
        let lines = self.lines.get(start..=end).unwrap_or_default();
        lines.iter().map(|line| keep_marker(line)).max().unwrap_or(Keep::None)
    }

    /// `@abstractmethod`, `@abc.abstractmethod`, `@abstractproperty`, ...
    fn is_abstract_decorator(&self, decorator: Node<'_>) -> bool {
        let Some(mut expr) = decorator.named_child(0) else { return false };
        if expr.kind() == "call" {
            match expr.child_by_field_name("function") {
                Some(function) => expr = function,
                None => return false,
            }
        }
        dotted(self, expr)
            .and_then(|parts| parts.last().cloned())
            .is_some_and(|last| last.starts_with("abstract"))
    }

    fn dunder_all(&mut self, node: Node<'_>) {
        let target = node.child_by_field_name("left").map(|n| self.text(n));
        if target != Some("__all__") {
            return;
        }
        if let Some(value) = node.child_by_field_name("right") {
            self.all_strings(value);
        }
    }

    /// Every identifier-shaped string under `node`, into `all_names`.
    fn all_strings(&mut self, node: Node<'_>) {
        if let Some(name) = self.string(node) {
            self.all_names.extend(Some(name).filter(|name| is_identifier(name)));
            return;
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.all_strings(child);
        }
    }

    /// `getattr(m, "C")`, `patch("pkg.mod.C")`, `patch.object(m, "C")`,
    /// `monkeypatch.setattr("pkg.mod.C", ...)`: record `C`.
    pub(crate) fn lookup_call(&mut self, node: Node<'_>) {
        let function = node.child_by_field_name("function").and_then(|n| dotted(self, n));
        let is_lookup = function.is_some_and(|parts| match &parts[..] {
            [.., patch, object] if patch == "patch" && object == "object" => true,
            [.., last] => ["getattr", "setattr", "hasattr", "patch"].contains(&last.as_str()),
            [] => false,
        });
        let Some(arguments) = node.child_by_field_name("arguments").filter(|_| is_lookup) else {
            return;
        };
        let mut cursor = arguments.walk();
        let names: Vec<String> = arguments
            .named_children(&mut cursor)
            .filter_map(|argument| self.string(argument))
            .filter_map(|path| path.rsplit('.').next().map(str::to_owned))
            .filter(|name| is_identifier(name))
            .collect();
        self.strings.extend(names);
    }

    /// The content of a plain string literal; `None` for anything else,
    /// f-strings with interpolation included.
    pub(crate) fn string(&self, node: Node<'_>) -> Option<String> {
        if node.kind() != "string" {
            return None;
        }
        let mut cursor = node.walk();
        let mut content = String::new();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "string_content" => content.push_str(self.text(child)),
                "interpolation" => return None,
                _ => {}
            }
        }
        Some(content)
    }
}

/// A base expression as dotted parts: `a.b.C` or `C[T]`. Anything else is `None`.
pub(crate) fn dotted(walker: &Walker<'_>, node: Node<'_>) -> Option<Vec<String>> {
    match node.kind() {
        "identifier" => Some(vec![walker.text(node).to_owned()]),
        "attribute" => {
            let mut parts = dotted(walker, node.child_by_field_name("object")?)?;
            parts.push(walker.text(node.child_by_field_name("attribute")?).to_owned());
            Some(parts)
        }
        "subscript" | "generic_type" => dotted(walker, node.named_child(0)?),
        _ => None,
    }
}

pub(crate) fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// Read `# dermestes: keep <reason>` from a definition line.
fn keep_marker(line: &str) -> Keep {
    let Some((_, after)) = line.split_once("# dermestes:") else { return Keep::None };
    let Some(reason) = after.trim_start().strip_prefix("keep") else { return Keep::None };
    if !reason.is_empty() && !reason.starts_with(char::is_whitespace) {
        return Keep::None;
    }
    if reason.trim().is_empty() {
        Keep::MissingReason
    } else {
        Keep::WithReason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_one(relative: &str, source: &str) -> Extracted {
        extract(relative, source, &roots(&["", "src/"]), &Config::default()).expect("parses")
    }

    fn roots(prefixes: &[&str]) -> Vec<String> {
        prefixes.iter().map(|&prefix| prefix.to_owned()).collect()
    }

    #[test]
    fn module_names_cover_flat_and_src_layouts() {
        let flat = roots(&[""]);
        assert_eq!(module_names("pkg/a.py", &flat), vec!["pkg.a"], "flat");
        let src = roots(&["", "src/"]);
        assert_eq!(module_names("src/pkg/__init__.py", &src), vec!["pkg", "src.pkg"], "src");
        assert!(module_names("my-scripts/a.py", &flat).is_empty(), "not importable");
    }

    #[test]
    fn project_directories_are_import_roots() {
        let files = ["api/src/api/m.py", "sdk/python/feast/m.py"]
            .map(|relative| SourcePath { path: relative.into(), relative: relative.to_owned() });
        let found = import_roots(&files, &roots(&["", "api/", "sdk/python/"]));
        assert_eq!(found, roots(&["", "api/", "sdk/python/", "api/src/"]), "member src/ too");
        assert_eq!(
            module_names("sdk/python/feast/m.py", &found),
            vec!["feast.m", "sdk.python.feast.m"],
            "both"
        );
    }

    #[test]
    fn imports_bind_the_names_python_binds() {
        let src = "import a.b\nimport c.d as e\nfrom f import G as H, I\nfrom ..x import Y\n";
        let bindings = extract_one("src/pkg/sub/m.py", src).info.bindings;
        assert_eq!(bindings["a"], Binding::Module("a".into()), "import a.b binds a");
        assert_eq!(bindings["e"], Binding::Module("c.d".into()), "alias binds the full module");
        let symbol =
            |module: &str, name: &str| Binding::Symbol { module: module.into(), name: name.into() };
        assert_eq!(bindings["H"], symbol("f", "G"), "from-import alias");
        assert_eq!(bindings["I"], symbol("f", "I"), "plain from-import");
        assert_eq!(bindings["Y"], symbol("pkg.x", "Y"), "relative import from src root");
    }

    #[test]
    fn classes_record_bases_methods_and_markers() {
        let src = "class A(b.Base, Generic[T], metaclass=M):  # dermestes: keep seam\n    @abc.abstractmethod\n    def run(self): ...\n    def stop(self): ...\n    class Inner: ...\n";
        let extracted = extract_one("m.py", src);
        let a = &extracted.classes[0];
        assert_eq!(
            a.bases,
            vec![vec!["b".to_owned(), "Base".to_owned()], vec!["Generic".to_owned()]],
            "bases"
        );
        assert_eq!(a.metaclass, Some(vec!["M".to_owned()]), "metaclass");
        assert_eq!(a.abstract_methods.iter().collect::<Vec<_>>(), vec!["run"], "abstract");
        assert_eq!(a.methods.len(), 2, "both methods");
        assert_eq!(a.keep, Keep::WithReason, "marker with reason");
        assert_eq!(extracted.classes[1].qualname, "A.Inner", "nested qualname");
        assert!(!extracted.classes[1].top_level, "nested is not importable");
    }

    #[test]
    fn collects_all_strings_and_register() {
        let src = "__all__ = ['A'] + [\"B\"]\nx = getattr(m, 'C')\nf'{y}z'\nBase.register(int)\n\
                   patch('p.q.D')\nmock.patch.object(m, 'E')\nr = {'F': f, 'K': 1}\n\
                   def g(x: 'G') -> 'H':\n    \"\"\"I\"\"\"\nJ = 'J'\n";
        let extracted = extract_one("m.py", src);
        assert_eq!(extracted.all_names, vec!["A", "B"], "__all__");
        let mut strings = extracted.strings.clone();
        strings.sort();
        assert_eq!(strings, vec!["C", "D", "E", "F"], "lookups and registry keys only");
        assert_eq!(extracted.registered, vec!["Base"], "register");
    }

    #[test]
    fn keep_marker_needs_the_whole_word() {
        assert_eq!(keep_marker("class A:  # dermestes: keep"), Keep::MissingReason, "bare");
        assert_eq!(keep_marker("class A:  # dermestes: keep  "), Keep::MissingReason, "spaces");
        assert_eq!(keep_marker("class A:  # dermestes: keeper"), Keep::None, "other word");
        assert_eq!(keep_marker("class A:  # dermestes: keep api"), Keep::WithReason, "reason");
    }

    #[test]
    fn syntax_errors_are_reported_not_indexed() {
        assert!(extract("m.py", "class (:\n", &[], &Config::default()).is_err(), "skipped");
    }
}
