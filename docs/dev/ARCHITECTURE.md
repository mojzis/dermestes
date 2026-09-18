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

Phase 1 adds the file walker, the class index, import resolution, the
`one-impl` check and the git diff filter.
