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
| `index.rs` | Parallel tree-sitter parse; per file: module names, import table, classes, `__all__`, dynamic-lookup strings and dict keys, `X.register`. |
| `resolve.rs` | Resolves a dotted base expression to an indexed class, `ABC`/`ABCMeta`/`Protocol`/`object`, or unknown. |
| `one_impl.rs` | The `one-impl` check and the `Finding` it produces. |
| `git.rs` | The only subprocess: `git diff --unified=0`, parsed into changed line ranges. |

## A run

`cli.rs` finds the project root (the nearest ancestor with `.git`, file or
directory, else the nearest with `pyproject.toml`, else the cwd), loads the
config there, reads the diff (unless `--all`), walks and indexes
the **whole** repository (counting users needs every file), runs the check,
then keeps a finding only if the abstraction's or its implementation's `class`
line falls inside a changed hunk. Output is sorted by path, then line.

## Resolution

Module roots are the repository root and every directory holding a
`pyproject.toml` (uv workspace members, feast's `sdk/python`), each plus its
`src/` when files live there. Imports are
read at module scope only; `import a.b`, `from a import b [as c]`, relative
imports and re-exports through `__init__.py` are followed. Anything else is
unknown. A class with an unknown base or a non-`ABCMeta` metaclass poisons
every ancestor; an unresolved base named `X` poisons every class named `X`.
Poisoned classes are never reported.
