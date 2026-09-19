//! The call-site index: functions, their parameters, every call with its
//! arguments, the scopes that bind names, and every name used as a value.
//!
//! This is the half of the walk `const-param` needs; [`crate::index`] drives
//! it. Nothing here resolves a name across files; that is [`crate::resolve`].

use std::collections::HashMap;

use tree_sitter::Node;

use crate::index::{dotted, join, Binding, ClassId, Keep, Walker};

/// Position of a function in [`crate::index::Index::functions`].
pub type FnId = usize;

/// Position of a scope in [`crate::index::FileInfo::scopes`]; `0` is the module.
pub type ScopeId = usize;

/// How a function is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnKind {
    Function,
    /// Bound to an instance: the first parameter is `self`.
    Method,
    /// Bound to the class: the first parameter is `cls`.
    ClassMethod,
    StaticMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    PositionalOnly,
    Positional,
    KeywordOnly,
    /// `*args`
    VarArgs,
    /// `**kwargs`
    VarKeyword,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub kind: ParamKind,
    pub default: Option<Value>,
}

/// A `def`, anywhere in a file.
#[derive(Debug, Clone)]
pub struct Function {
    pub file: usize,
    pub name: String,
    /// Dotted through enclosing classes and functions: `Transcript.done`.
    pub qualname: String,
    /// The `def` line, not a decorator's.
    pub line: usize,
    /// The class whose body defines it.
    pub class: Option<ClassId>,
    pub kind: FnKind,
    pub params: Vec<Param>,
    /// Dotted names of decorators other than `staticmethod`, `classmethod` or
    /// an `abstract*` one (`app.get` for `@app.get("/")`; empty when not a
    /// name): a framework may call it where we cannot see.
    pub decorators: Vec<String>,
    pub is_abstract: bool,
    pub keep: Keep,
}

/// An argument or default, as far as "same value" can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    /// Source text, for output.
    pub text: String,
    pub kind: ValueKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    /// `True`, `None`, `0.5`, `"cr"`, `-1`, a short tuple of those; the
    /// payload is a normalised key (`'cr'` and `"cr"` are the same).
    Literal(String),
    /// A dotted name; the same value only if it resolves to one constant.
    Name(Vec<String>),
    Other,
}

#[derive(Debug, Clone)]
pub enum Arg {
    Positional(Value),
    Keyword(String, Value),
}

/// What a call's function expression names, before resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callee {
    /// `f(...)`
    Name(String),
    /// `a.b.f(...)`: the object's parts, then the attribute.
    Dotted(Vec<String>, String),
    /// `self.m(...)` or `cls.m(...)` directly inside a method of the class.
    SelfMethod(ClassId, String),
    /// `super().m(...)` inside a method of the class.
    Super(ClassId, String),
    /// Some attribute call we cannot follow, `x().m(...)`; `cls(...)` and
    /// `type(self)(...)` are `__init__`.
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct Call {
    pub file: usize,
    pub scope: ScopeId,
    pub callee: Callee,
    pub args: Vec<Arg>,
    /// Has a `*x` or `**x` argument: which parameters it binds is unknown.
    pub splat: bool,
    /// Under `if __name__ == "__main__":`.
    pub under_main: bool,
}

/// A function or class body. Class bodies are skipped when a nested function
/// looks a name up, as in Python.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub parent: ScopeId,
    /// Set on a class body.
    pub class: Option<ClassId>,
    /// Set on a method's body: its class and the name of its `self`/`cls`.
    pub method: Option<(ClassId, String)>,
    /// Nested `def`s; `None` when a name is defined twice.
    pub defs: HashMap<String, Option<FnId>>,
    /// Other local names: `Some` for an import, `None` for a parameter or
    /// assignment, which hides any outer definition.
    pub bindings: HashMap<String, Option<Binding>>,
}

/// A module-level name before ids are assigned; `add` turns these into
/// [`crate::index::Symbol`]s.
#[derive(Debug, Clone, Copy)]
pub enum TopName {
    Class(ClassId),
    Function(FnId),
    /// Assigned a literal.
    Constant,
    /// Assigned anything else.
    Other,
}

impl Walker<'_> {
    pub(crate) fn function(&mut self, node: Node<'_>, prefix: &str, top: bool) {
        let Some(name) = node.child_by_field_name("name").map(|n| self.text(n).to_owned()) else {
            return;
        };
        let qualname = join(prefix, &name);
        let mut staticmethod = false;
        let mut classmethod = false;
        let mut is_abstract = false;
        let mut decorators = Vec::new();
        if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
            let mut cursor = parent.walk();
            for decorator in parent.named_children(&mut cursor).filter(|c| c.kind() == "decorator")
            {
                let expr = decorator.named_child(0).map(|expr| {
                    let called = expr.kind() == "call";
                    if called {
                        expr.child_by_field_name("function").unwrap_or(expr)
                    } else {
                        expr
                    }
                });
                let parts = expr.and_then(|expr| dotted(self, expr));
                match parts.as_ref().and_then(|parts| parts.last()).map(String::as_str) {
                    Some("staticmethod") => staticmethod = true,
                    Some("classmethod") => classmethod = true,
                    Some(last) if last.starts_with("abstract") => is_abstract = true,
                    _ => decorators.push(parts.map(|parts| parts.join(".")).unwrap_or_default()),
                }
            }
        }
        let class = self.info.scopes[self.scope].class;
        let kind = match class {
            None => FnKind::Function,
            Some(_) if staticmethod => FnKind::StaticMethod,
            Some(_) if classmethod => FnKind::ClassMethod,
            Some(_) => FnKind::Method,
        };
        let params = node
            .child_by_field_name("parameters")
            .map_or_else(Vec::new, |parameters| self.params(parameters, prefix, top));

        let id = self.functions.len();
        self.functions.push(Function {
            file: 0,
            name: name.clone(),
            qualname: qualname.clone(),
            line: node.start_position().row + 1,
            class,
            kind,
            params: params.clone(),
            decorators,
            is_abstract,
            keep: self.header_keep(node),
        });
        if top {
            self.top_names.push((name, TopName::Function(id)));
        } else if class.is_none() {
            let defs = &mut self.info.scopes[self.scope].defs;
            defs.entry(name).and_modify(|slot| *slot = None).or_insert(Some(id));
        }

        let receiver = params.first().filter(|_| kind != FnKind::StaticMethod);
        let scope = Scope {
            parent: self.scope,
            method: class.zip(receiver.map(|param| param.name.clone())),
            bindings: params.iter().map(|param| (param.name.clone(), None)).collect(),
            ..Scope::default()
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.in_scope(scope, |walker| walker.visit(body, &qualname, false));
        }
    }

    /// Run `f` inside a new child scope, then return to the current one.
    pub(crate) fn in_scope(&mut self, scope: Scope, f: impl FnOnce(&mut Self)) {
        let outer = self.scope;
        self.scope = self.info.scopes.len();
        self.info.scopes.push(scope);
        f(self);
        self.scope = outer;
    }

    /// Parameters in order. Defaults are evaluated, and visited, in the
    /// enclosing scope; annotations are never values.
    fn params(&mut self, parameters: Node<'_>, prefix: &str, top: bool) -> Vec<Param> {
        let mut params: Vec<Param> = Vec::new();
        let mut keyword_only = false;
        let mut cursor = parameters.walk();
        for node in parameters.named_children(&mut cursor) {
            let (target, default) = match node.kind() {
                "default_parameter" | "typed_default_parameter" => {
                    (node.child_by_field_name("name"), node.child_by_field_name("value"))
                }
                "typed_parameter" => (node.named_child(0), None),
                "keyword_separator" => {
                    keyword_only = true;
                    continue;
                }
                "positional_separator" => {
                    for param in &mut params {
                        param.kind = ParamKind::PositionalOnly;
                    }
                    continue;
                }
                _ => (Some(node), None),
            };
            let Some(target) = target else { continue };
            let (kind, name_node) = match target.kind() {
                "list_splat_pattern" => {
                    keyword_only = true;
                    (ParamKind::VarArgs, target.named_child(0))
                }
                "dictionary_splat_pattern" => (ParamKind::VarKeyword, target.named_child(0)),
                "identifier" if keyword_only => (ParamKind::KeywordOnly, Some(target)),
                "identifier" => (ParamKind::Positional, Some(target)),
                _ => continue,
            };
            let Some(name_node) = name_node else { continue };
            if let Some(default) = default {
                self.visit(default, prefix, top);
            }
            params.push(Param {
                name: self.text(name_node).to_owned(),
                kind,
                default: default.map(|default| self.value(default)),
            });
        }
        params
    }

    /// Record a call, then visit what it contains.
    pub(crate) fn call(&mut self, node: Node<'_>, prefix: &str, top: bool) {
        self.lookup_call(node);
        let function = node.child_by_field_name("function");
        let callee = function.and_then(|function| self.callee(function));
        match function {
            Some(function) if function.kind() == "identifier" => {}
            Some(function) if function.kind() == "attribute" => {
                self.attribute(function, prefix, top, true);
            }
            Some(function) => self.visit(function, prefix, top),
            None => {}
        }
        let mut args = Vec::new();
        let mut splat = false;
        if let Some(arguments) = node.child_by_field_name("arguments") {
            let mut cursor = arguments.walk();
            for argument in arguments.named_children(&mut cursor) {
                match argument.kind() {
                    "list_splat" | "dictionary_splat" => splat = true,
                    "keyword_argument" => {
                        let name = argument.child_by_field_name("name").map(|n| self.text(n));
                        let value = argument.child_by_field_name("value");
                        if let (Some(name), Some(value)) = (name, value) {
                            args.push(Arg::Keyword(name.to_owned(), self.value(value)));
                        }
                    }
                    "comment" => continue,
                    _ => args.push(Arg::Positional(self.value(argument))),
                }
                self.visit(argument, prefix, top);
            }
        }
        // `getattr(mod, "is_" + name)`: any function of `mod` may be called.
        if let (Some(Callee::Name(name)), [Arg::Positional(object), Arg::Positional(attr), ..]) =
            (&callee, &args[..])
        {
            if let (true, ValueKind::Name(parts), false) =
                (name == "getattr", &object.kind, matches!(attr.kind, ValueKind::Literal(_)))
            {
                self.dynamic.push((self.scope, parts.clone()));
            }
        }
        if let Some(callee) = callee {
            self.calls.push(Call {
                file: 0,
                scope: self.scope,
                callee,
                args,
                splat,
                under_main: self.under_main,
            });
        }
    }

    fn callee(&self, function: Node<'_>) -> Option<Callee> {
        let receiver = self.info.scopes[self.scope].method.as_ref();
        match function.kind() {
            "identifier" => {
                let name = self.text(function);
                let is_cls = receiver.is_some_and(|(_, receiver)| receiver == name);
                Some(if is_cls {
                    Callee::Unknown("__init__".into())
                } else {
                    Callee::Name(name.into())
                })
            }
            "attribute" => {
                let name = self.text(function.child_by_field_name("attribute")?).to_owned();
                if name == "__class__" {
                    return Some(Callee::Unknown("__init__".into()));
                }
                let object = function.child_by_field_name("object")?;
                if object.kind() == "call" && self.is_super(object) {
                    return Some(match receiver {
                        Some((class, _)) => Callee::Super(*class, name),
                        None => Callee::Unknown(name),
                    });
                }
                Some(match (dotted(self, object), receiver) {
                    (Some(parts), Some((class, receiver))) if parts == [receiver.clone()] => {
                        Callee::SelfMethod(*class, name)
                    }
                    (Some(parts), _) if object.kind() != "subscript" => Callee::Dotted(parts, name),
                    _ => Callee::Unknown(name),
                })
            }
            // `type(self)(...)`
            "call" => Some(Callee::Unknown("__init__".into())),
            _ => None,
        }
    }

    fn is_super(&self, call: Node<'_>) -> bool {
        call.child_by_field_name("function").is_some_and(|f| self.text(f) == "super")
    }

    /// `a.b`: note `X.register`, record `b` as used by value unless it is being
    /// called, and visit the object.
    pub(crate) fn attribute(&mut self, node: Node<'_>, prefix: &str, top: bool, called: bool) {
        let attr = node.child_by_field_name("attribute").map(|n| self.text(n));
        let object = node.child_by_field_name("object");
        if let (Some("register"), Some(parts)) = (attr, object.and_then(|n| dotted(self, n))) {
            self.registered.extend(parts.last().cloned());
        }
        if !called {
            self.refs.extend(attr.map(str::to_owned));
        }
        if let Some(object) = object {
            self.visit(object, prefix, top);
        }
    }

    /// Bind assignment targets, then visit the value.
    pub(crate) fn assignment(&mut self, node: Node<'_>, prefix: &str, top: bool) {
        let right = node.child_by_field_name("right");
        if let Some(left) = node.child_by_field_name("left") {
            let is_literal = left.kind() == "identifier"
                && node.kind() == "assignment"
                && right
                    .is_some_and(|right| matches!(self.value(right).kind, ValueKind::Literal(_)));
            self.bind_targets(left, prefix, top, is_literal);
        }
        if let Some(right) = right {
            self.visit(right, prefix, top);
        }
    }

    /// Names a target pattern binds become local (or module-level) names;
    /// anything else in it, `a.b` or `a[i]`, is visited as an expression.
    pub(crate) fn bind_targets(
        &mut self,
        target: Node<'_>,
        prefix: &str,
        top: bool,
        constant: bool,
    ) {
        match target.kind() {
            "identifier" => {
                let name = self.text(target).to_owned();
                if top {
                    let kind = if constant { TopName::Constant } else { TopName::Other };
                    self.top_names.push((name, kind));
                } else {
                    self.info.scopes[self.scope].bindings.insert(name, None);
                }
            }
            "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "parenthesized_expression"
            | "list_splat_pattern"
            | "tuple"
            | "list" => {
                let mut cursor = target.walk();
                for child in target.named_children(&mut cursor) {
                    self.bind_targets(child, prefix, top, false);
                }
            }
            _ => self.visit(target, prefix, top),
        }
    }

    pub(crate) fn value(&self, node: Node<'_>) -> Value {
        let text = self.text(node).to_owned();
        let kind = self
            .literal_key(node)
            .map(ValueKind::Literal)
            .or_else(|| {
                dotted(self, node).filter(|_| node.kind() != "subscript").map(ValueKind::Name)
            })
            .unwrap_or(ValueKind::Other);
        Value { text, kind }
    }

    fn literal_key(&self, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "integer" | "float" | "true" | "false" | "none" => Some(self.text(node).to_owned()),
            "string" => {
                let content = self.string(node)?;
                let start = node.child(0).map_or("", |start| self.text(start));
                let prefix = start.trim_end_matches(['"', '\'']).to_ascii_lowercase();
                Some(format!("{prefix}{content:?}"))
            }
            "unary_operator" => {
                let operand = node.child_by_field_name("argument")?;
                let operator = node.child_by_field_name("operator").map(|n| self.text(n))?;
                matches!(operand.kind(), "integer" | "float")
                    .then(|| format!("{operator}{}", self.text(operand)))
            }
            "tuple" => {
                let mut cursor = node.walk();
                let items: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
                if items.len() > 4 {
                    return None;
                }
                let keys: Option<Vec<String>> =
                    items.into_iter().map(|item| self.literal_key(item)).collect();
                Some(format!("({})", keys?.join(",")))
            }
            _ => None,
        }
    }

    /// `if __name__ == "__main__":` at module level.
    pub(crate) fn is_main_guard(&self, node: Node<'_>) -> bool {
        node.child_by_field_name("condition").is_some_and(|condition| {
            let text = self.text(condition);
            text.contains("__name__") && text.contains("__main__")
        })
    }
}
