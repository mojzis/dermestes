# Architecture

The rules the code obeys are in [`../plans/CONSTITUTION.md`](../plans/CONSTITUTION.md);
the decisions and their costs are in [`../adr/`](../adr/). This file says how
the code is laid out.

## Modules

| Module | Responsibility |
|---|---|
| `main.rs` | Parse arguments, call `Cli::run`, map the outcome to an exit code. |
| `cli.rs` | The CLI surface from the constitution's interface contract. |
| `guide.rs` | Serves `docs/src/guide/*.md` verbatim; picks a topic from `./pyproject.toml`. |
| `config.rs` | `[tool.dermestes]` with the constitution's five keys; unknown keys are errors. Glob matching. |
| `discovery.rs` | gitignore-aware walk for `.py` files and `pyproject.toml` directories, minus `exclude` (copied from biston). |
| `index.rs` | Parallel tree-sitter parse; per file: module names, import table, classes, `__all__`, dynamic-lookup strings and registry dict keys, `X.register`. |
| `calls.rs` | The call-site half of the same walk: functions and parameters, calls and their arguments, function and class scopes, names used as values. |
| `resolve.rs` | Resolves a dotted expression to an indexed class, function or module constant, `ABC`/`ABCMeta`/`Protocol`/`object`, external, or unknown. |
| `one_impl.rs` | The `one-impl` check and the `Finding` it produces. |
| `const_param.rs` | The `const-param` check: resolves every call, binds its arguments, compares each parameter's values across sites. |
| `git.rs` | The only subprocess: `git diff --unified=0`, plus the working tree, rebuilds each changed file's base side. |

## A run

`cli.rs` finds the project root (the nearest ancestor with `.git`, file or
directory, else the nearest with `pyproject.toml`, else the cwd), loads the
config there, reads the diff (unless `--all`), walks and indexes
the **whole** repository (counting users needs every file), runs the checks,
then, unless `--all`, runs the checks again on the base side and keeps only
the findings new at head. The base side is the head's parsed files with each
file the diff changed rebuilt from the diff (`git.rs`) and re-parsed; a
finding's identity is its check, path, name and parameter, since lines shift. Output is
sorted by path, then line.

## Resolution

Module roots are the repository root and every directory holding a
`pyproject.toml` (uv workspace members, feast's `sdk/python`), each plus its
`src/` when files live there. Imports are
read at module scope, and inside functions as local names; `import a.b`, `from a import b [as c]`, relative
imports and re-exports through `__init__.py` are followed. Anything else is
unknown. A class with an unknown base or a non-`ABCMeta` metaclass poisons
every ancestor; an unresolved base named `X` poisons every class named `X`, unless it is
looked up in a module outside the repository (`logging.Formatter`), which
cannot be a repository class.
Poisoned classes are never reported.

Calls (`const_param.rs`) resolve only through what the index can see: a bare
name through enclosing function scopes then the module, `mod.f` through
imports, `self.m`/`cls.m`/`super().m` through the class's resolved bases.
`Class(...)` is `__init__`. Every other call is kept by method name only, as
a possible call: it cannot count toward a finding, but it vetoes one when its
arguments could bind to that function with a different value.
