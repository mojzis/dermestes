//! Body shapes `const-param` reads: the branches that test one parameter
//! against a literal, and where a variadic is only forwarded.
//!
//! One walk per function body, driven by [`crate::calls`]. Nothing is
//! evaluated here: which arm is dead depends on the constant the call sites
//! give, which only the check knows.

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::calls::{Param, ParamKind};
use crate::index::Walker;

/// `p`, `p is None`, `p == "x"`, `p in ("x", "y")`, or the negation of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cond {
    pub param: String,
    pub negated: bool,
    pub test: Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Test {
    Truthy,
    IsNone,
    /// Equal to one of these literals, as [`crate::calls::ValueKind::Literal`] keys.
    Among(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum When {
    Test(Cond),
    /// A test we do not evaluate: `p and q`, a call, a rebound parameter.
    Unknown,
    Else,
}

/// One arm of a branch and the lines that go if it cannot run.
#[derive(Debug, Clone)]
pub struct Arm {
    pub when: When,
    pub start: usize,
    pub end: usize,
}

/// An `if` chain, a `while` or a conditional expression, arms in order.
#[derive(Debug, Clone)]
pub struct Branch {
    pub arms: Vec<Arm>,
    /// A conditional expression: its arms are operands, not whole lines.
    pub inline: bool,
}

struct Scan<'w, 's> {
    walker: &'w Walker<'s>,
    params: HashSet<&'w str>,
    rebound: HashSet<&'s str>,
    /// Per name: lines it is forwarded on as `*p`/`**p`, and whether it has any other use.
    uses: HashMap<&'s str, (Vec<usize>, bool)>,
    branches: Vec<Branch>,
}

impl Walker<'_> {
    /// The branches of `body` that test a parameter never rebound in it.
    /// Sets [`Param::forwarded`] on each variadic the body only forwards.
    pub(crate) fn shapes(&self, body: Node<'_>, params: &mut [Param]) -> Vec<Branch> {
        let mut scan = Scan {
            walker: self,
            params: params.iter().map(|param| param.name.as_str()).collect(),
            rebound: HashSet::new(),
            uses: HashMap::new(),
            branches: Vec::new(),
        };
        scan.node(body, false);
        let Scan { rebound, mut uses, mut branches, .. } = scan;
        for branch in &mut branches {
            for arm in &mut branch.arms {
                if matches!(&arm.when, When::Test(cond) if rebound.contains(cond.param.as_str())) {
                    arm.when = When::Unknown;
                }
            }
        }
        branches.retain(|branch| branch.arms.iter().any(|arm| matches!(arm.when, When::Test(_))));
        for param in params {
            if matches!(param.kind, ParamKind::VarArgs | ParamKind::VarKeyword) {
                if let Some((lines, false)) = uses.remove(param.name.as_str()) {
                    param.forwarded = lines;
                }
            }
        }
        branches
    }
}

impl Scan<'_, '_> {
    fn node(&mut self, node: Node<'_>, binding: bool) {
        match node.kind() {
            "identifier" => return self.identifier(node, binding),
            "if_statement" => self.if_statement(node),
            "while_statement" if node.child_by_field_name("alternative").is_none() => {
                let when = self.when(node.child_by_field_name("condition"));
                self.branches.push(Branch { arms: vec![arm(when, node, node)], inline: false });
            }
            "conditional_expression" if node.named_child_count() == 3 => {
                let operand = |i| node.named_child(i).unwrap_or(node);
                let arms = vec![
                    arm(self.when(node.named_child(1)), operand(0), operand(0)),
                    arm(When::Else, operand(2), operand(2)),
                ];
                self.branches.push(Branch { arms, inline: true });
            }
            _ => {}
        }
        let mut cursor = node.walk();
        if !cursor.goto_first_child() {
            return;
        }
        loop {
            let child = cursor.node();
            let field = cursor.field_name();
            // `f(p=1)` and `x.p` spell the name without reading the parameter.
            let spelled = matches!(
                (node.kind(), field),
                ("keyword_argument", Some("name")) | ("attribute", Some("attribute"))
            );
            if !spelled {
                self.node(child, binding || binds(node, field, child));
            }
            if !cursor.goto_next_sibling() {
                return;
            }
        }
    }

    fn identifier(&mut self, node: Node<'_>, binding: bool) {
        let name = self.walker.text(node);
        if !self.params.contains(name) {
            return;
        }
        if binding {
            self.rebound.insert(name);
        }
        let splat = node.parent().filter(|parent| {
            matches!(parent.kind(), "list_splat" | "dictionary_splat")
                && parent.parent().is_some_and(|call| call.kind() == "argument_list")
        });
        let (lines, other) = self.uses.entry(name).or_default();
        match splat {
            Some(_) => lines.push(node.start_position().row + 1),
            None => *other = true,
        }
    }

    fn if_statement(&mut self, node: Node<'_>) {
        let Some(block) = node.child_by_field_name("consequence") else { return };
        let mut cursor = node.walk();
        let rest: Vec<Node<'_>> = node.children_by_field_name("alternative", &mut cursor).collect();
        // Alone, the whole statement goes; with alternatives, only its block.
        let first = if rest.is_empty() { node } else { block };
        let mut arms = vec![arm(self.when(node.child_by_field_name("condition")), first, block)];
        for clause in rest {
            let when = match clause.kind() {
                "elif_clause" => self.when(clause.child_by_field_name("condition")),
                _ => When::Else,
            };
            arms.push(arm(when, clause, clause));
        }
        self.branches.push(Branch { arms, inline: false });
    }

    fn param(&self, node: Node<'_>) -> Option<String> {
        let name = self.walker.text(node);
        (node.kind() == "identifier" && self.params.contains(name)).then(|| name.to_owned())
    }

    fn when(&self, condition: Option<Node<'_>>) -> When {
        let Some(node) = condition.map(unwrap_parens) else { return When::Unknown };
        let cond = |param, negated, test| When::Test(Cond { param, negated, test });
        match node.kind() {
            "identifier" => self.param(node).map(|param| cond(param, false, Test::Truthy)),
            "not_operator" => node
                .child_by_field_name("argument")
                .and_then(|argument| self.param(unwrap_parens(argument)))
                .map(|param| cond(param, true, Test::Truthy)),
            "comparison_operator" if node.named_child_count() == 2 => (|| {
                let (left, right) = (node.named_child(0)?, node.named_child(1)?);
                let param = self.param(left)?;
                let source = self.walker.text(node);
                let between =
                    left.end_byte() - node.start_byte()..right.start_byte() - node.start_byte();
                let operator: Vec<&str> = source.get(between)?.split_whitespace().collect();
                let negated = matches!(operator[..], ["is", "not"] | ["!="] | ["not", "in"]);
                let test = match (&operator[..], right.kind()) {
                    (["is"] | ["is", "not"], "none") => Test::IsNone,
                    (["==" | "!="], _) => Test::Among(vec![self.walker.literal_key(right)?]),
                    (["in"] | ["not", "in"], "tuple" | "list" | "set") => {
                        let mut cursor = right.walk();
                        let keys: Option<Vec<String>> = right
                            .named_children(&mut cursor)
                            .map(|item| self.walker.literal_key(item))
                            .collect();
                        Test::Among(keys?)
                    }
                    _ => return None,
                };
                Some(cond(param, negated, test))
            })(),
            _ => None,
        }
        .unwrap_or(When::Unknown)
    }
}

fn arm(when: When, from: Node<'_>, to: Node<'_>) -> Arm {
    Arm { when, start: from.start_position().row + 1, end: to.end_position().row + 1 }
}

fn unwrap_parens(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" && node.named_child_count() == 1 {
        match node.named_child(0) {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// Whether `child` of `node` is a place names are bound, not read.
fn binds(node: Node<'_>, field: Option<&str>, child: Node<'_>) -> bool {
    let target = match node.kind() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => "left",
        "named_expression" => "name",
        _ => "",
    };
    field == Some(target)
        || matches!(
            child.kind(),
            "as_pattern_target"
                | "delete_statement"
                | "global_statement"
                | "nonlocal_statement"
                | "import_statement"
                | "import_from_statement"
                | "case_pattern"
                | "parameters"
                | "lambda_parameters"
        )
}
