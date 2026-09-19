//! Syntactic import resolution.
//!
//! A dotted base-class expression becomes a class in the index, a well-known
//! marker (`ABC`, `Protocol`, ...), or `Unknown`. `Unknown` is the precision fence: nothing that depends on an
//! unknown edge is ever flagged.

use std::collections::HashMap;

use crate::index::{Binding, ClassId, Index};

/// Re-exports are followed at most this deep, which also stops import cycles.
const MAX_HOPS: usize = 16;

/// What a name or dotted expression refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Class(ClassId),
    Module(String),
    /// `abc.ABC`
    Abc,
    /// `abc.ABCMeta`
    AbcMeta,
    /// `typing.Protocol` or `typing_extensions.Protocol`
    Protocol,
    /// `object` or `typing.Generic`: adds nothing to a hierarchy.
    Object,
    Unknown,
}

pub struct Resolver<'a> {
    index: &'a Index,
    /// Module name to file; `None` when two files claim the same name.
    modules: HashMap<&'a str, Option<usize>>,
}

impl<'a> Resolver<'a> {
    pub fn new(index: &'a Index) -> Self {
        let mut modules = HashMap::new();
        for (file, info) in index.files.iter().enumerate() {
            for module in &info.modules {
                modules
                    .entry(module.as_str())
                    .and_modify(|slot| *slot = None)
                    .or_insert(Some(file));
            }
        }
        Self { index, modules }
    }

    /// Resolve `parts` (e.g. `["core", "sinks", "Sink"]`) in `file`'s module scope.
    pub fn dotted(&self, file: usize, parts: &[String]) -> Target {
        let Some((head, rest)) = parts.split_first() else { return Target::Unknown };
        let mut target = self.local(file, head);
        for part in rest {
            target = match target {
                Target::Module(module) => self.symbol(&module, part, 0),
                _ => Target::Unknown,
            };
        }
        target
    }

    /// What an import binding refers to.
    pub fn binding(&self, binding: &Binding) -> Target {
        self.follow(binding, 0)
    }

    fn local(&self, file: usize, name: &str) -> Target {
        let info = &self.index.files[file];
        match info.top_classes.get(name) {
            Some(Some(id)) => return Target::Class(*id),
            Some(None) => return Target::Unknown,
            None => {}
        }
        match info.bindings.get(name) {
            Some(binding) => self.follow(binding, 0),
            None if name == "object" => Target::Object,
            None => Target::Unknown,
        }
    }

    fn follow(&self, binding: &Binding, hops: usize) -> Target {
        match binding {
            Binding::Module(module) => Target::Module(module.clone()),
            Binding::Symbol { module, name } => self.symbol(module, name, hops),
            Binding::Unknown => Target::Unknown,
        }
    }

    /// `name` looked up as an attribute of `module`.
    fn symbol(&self, module: &str, name: &str, hops: usize) -> Target {
        if hops > MAX_HOPS {
            return Target::Unknown;
        }
        let submodule = format!("{module}.{name}");
        match self.modules.get(module) {
            Some(Some(file)) => {
                let info = &self.index.files[*file];
                match info.top_classes.get(name) {
                    Some(Some(id)) => return Target::Class(*id),
                    Some(None) => return Target::Unknown,
                    None => {}
                }
                if let Some(binding) = info.bindings.get(name) {
                    return self.follow(binding, hops + 1);
                }
                if self.modules.contains_key(submodule.as_str()) {
                    return Target::Module(submodule);
                }
                Target::Unknown
            }
            Some(None) => Target::Unknown,
            // Not in the repository: a namespace package or a third-party module.
            None if self.modules.contains_key(submodule.as_str()) => Target::Module(submodule),
            None => well_known(module, name),
        }
    }
}

/// The few names outside the repository that shape a class hierarchy.
fn well_known(module: &str, name: &str) -> Target {
    match (module, name) {
        ("abc", "ABC") => Target::Abc,
        ("abc", "ABCMeta") => Target::AbcMeta,
        ("typing" | "typing_extensions", "Protocol") => Target::Protocol,
        ("typing" | "typing_extensions", "Generic") | ("builtins", "object") => Target::Object,
        _ => Target::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::discovery::SourcePath;

    fn index(files: &[(&str, &str)]) -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut paths = Vec::new();
        for (relative, source) in files {
            let path = dir.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
            std::fs::write(&path, source).expect("write");
            paths.push(SourcePath { path, relative: (*relative).to_owned() });
        }
        let index = Index::build(&paths, &[], &Config::default());
        (dir, index)
    }

    fn parts(dotted: &str) -> Vec<String> {
        dotted.split('.').map(str::to_owned).collect()
    }

    #[test]
    fn follows_imports_and_reexports() {
        let (_dir, index) = index(&[
            ("pkg/__init__.py", "from .base import Base\n"),
            ("pkg/base.py", "class Base: ...\n"),
            ("app.py", "import pkg\nfrom pkg import Base as B\nimport pkg.base as pb\n"),
        ]);
        let resolver = Resolver::new(&index);
        let app = index.files.iter().position(|f| f.relative == "app.py").expect("app");
        assert_eq!(resolver.dotted(app, &parts("B")), Target::Class(0), "via __init__ re-export");
        assert_eq!(resolver.dotted(app, &parts("pkg.Base")), Target::Class(0), "attribute");
        assert_eq!(resolver.dotted(app, &parts("pkg.base.Base")), Target::Class(0), "submodule");
        assert_eq!(resolver.dotted(app, &parts("pb.Base")), Target::Class(0), "module alias");
        assert_eq!(resolver.dotted(app, &parts("Missing")), Target::Unknown, "undefined");
    }

    #[test]
    fn well_known_names_and_third_party() {
        let (_dir, index) = index(&[(
            "m.py",
            "import abc\nfrom typing import Protocol, Generic\nfrom pydantic import BaseModel\n",
        )]);
        let resolver = Resolver::new(&index);
        assert_eq!(resolver.dotted(0, &parts("abc.ABC")), Target::Abc, "abc.ABC");
        assert_eq!(resolver.dotted(0, &parts("Protocol")), Target::Protocol, "Protocol");
        assert_eq!(resolver.dotted(0, &parts("Generic")), Target::Object, "Generic");
        assert_eq!(resolver.dotted(0, &parts("BaseModel")), Target::Unknown, "third party");
    }

    #[test]
    fn import_cycles_terminate() {
        let (_dir, index) = index(&[
            ("a.py", "from b import X\n"),
            ("b.py", "from a import X\n"),
            ("c.py", "from a import X\n"),
        ]);
        let resolver = Resolver::new(&index);
        assert_eq!(resolver.dotted(2, &parts("X")), Target::Unknown, "cycle is unknown");
    }
}
