# Build prompt — phase 1 (`one-impl`)

Paste into Claude Code at the root of the dermestes repo (`~/git/dermestes`). The scaffold is already in place.

---

We are building **dermestes**, a Rust CLI that flags over-abstraction in Python code. Read `docs/plans/CONSTITUTION.md` first, then `CLAUDE.md`. The constitution is binding, and principle 8 applies to the code you write here: no traits with one implementation, no options nobody asked for.

## Before writing code

Reply with a plan of at most 15 lines: module layout, data structures, dependency list with one-line justifications. Then stop and wait for my approval.

Budget for this phase: roughly 1,200 lines of Rust excluding fixtures and the existing scaffold, five or six new source files. If your plan needs much more, the plan is wrong; say why instead of proceeding.

## What already exists

Scaffolded from the sibling tool gerenuk (`~/git/gerenuk`): maturin/pyproject packaging, lint and CI config, the mdBook docs site, and:

- `src/cli.rs`: the constitution's interface (`--base`, `--all`, `--format text|json`, `guide`). Running checks currently bails with "no checks are implemented yet"; replace that.
- `src/guide.rs` plus `docs/src/guide/{setup,triage,tune}.md`: `dermestes guide` prints those pages verbatim. Edit the pages, not `guide.rs`. Each page is capped at 60 lines and ends with one `next: run` line; the tests enforce it.
- `Cargo.toml` with only `clap`, `anyhow`, `toml`. Add each new dependency with a one-line justification comment, as the existing ones have.

## Copy from biston

The sibling tool biston is at `~/git/biston`. Copy (do not depend on) its file walker and ignore handling (`src/discovery.rs`, built on the `ignore` crate) and its tree-sitter-python setup (`src/parse.rs`). Adapt them to dermestes's config; do not extract anything shared. Do not copy its clone-detection logic.

## Scope: the `one-impl` check, end to end

**1. Class index** over all non-excluded `.py` files. Per class: qualified name, file, line, base expressions, method names, and
- `is_abstract`: inherits `ABC`, or `metaclass=ABCMeta`, or has any `@abstractmethod`
- `is_protocol`: inherits `Protocol` (from `typing` or `typing_extensions`)
- `is_test`: file matches `test-paths` (default: `tests/**`, `test_*.py`, `*_test.py`, `conftest.py`)

**2. Import resolution**, syntactic only: `import a.b`, `from a import b`, `from a import b as c`, relative imports, and re-exports through `__init__.py`. It resolves a base-class expression to a class in the index or returns unknown. Unknown means: nothing depending on that edge gets flagged. Support both flat and `src/` layouts. No `sys.path` tricks, no namespace-package heuristics.

**3. The check**
- ABC: flag when exactly one concrete (non-abstract) production class descends from it, directly or transitively, and no edge in that subtree is unknown.
- Protocol: implementations are structural. Count production classes whose method-name set is a superset of the protocol's method names, plus nominal subclasses. Flag when the count is exactly one. Skip protocols with no methods (attribute-only) and protocols with a single dunder method such as `__call__`.
- Zero production implementations: not flagged in this phase.
- Test implementations (fakes) are counted and shown separately. If one or more exist, do **not** flag: the abstraction may be a test seam. Record this as a known miss in the guide.
- Apply every precision guard in the constitution that is relevant to classes: `__all__`, `__init__` re-exports, `public` globs, string-literal mentions, `# dermestes: keep <reason>`.

**4. Diff mode.** The index always covers the whole repo, since counts need it. Reporting is then filtered: default mode reports a finding only if the abstract class or its sole implementation has a definition line inside a hunk of `git diff HEAD` (staged plus unstaged). `--base <ref>` swaps the comparison point. `--all` disables the filter. Shell out to `git diff --unified=0` from a single place; no git library.

**5. CLI and output** exactly as in the constitution's interface contract: text and json formats, exit codes 0/1/2, silent on no findings, `[tool.dermestes]` config with only the keys listed there. Update the guide pages so that `dermestes guide` covers: when to run it, how to read a finding, how to act on it (try the inlining, run tests), how to suppress with a reason, and the known misses. Keep `docs/dev/ARCHITECTURE.md` and the invariants in `CLAUDE.md` true.

## Tests

Fixture-driven, as the constitution describes: `fixtures/one-impl/flag/*` and `fixtures/one-impl/no_flag/*`, each case a small Python tree plus expected text output, driven by one test harness. Minimum `no_flag` cases:

- ABC with two production implementations
- ABC whose only subclass is itself abstract
- base class imported from a third-party package (unknown edge)
- Protocol satisfied structurally by two classes, neither subclassing it
- Protocol with one production impl and one fake under `tests/`
- abstraction listed in `__all__`
- abstraction re-exported from a package `__init__.py`
- `# dermestes: keep` with a reason; and a `flag` case where the marker has no reason
- attribute-only Protocol

Plus one diff-mode test using a temporary git repo: a pre-existing `one-impl` case is not reported, a newly added one is.

## Out of scope — do not build

Call graphs, call-site analysis, any other check id, ty-find or any LSP integration, auto-fix, caching, a watch mode, a plugin system, progress bars, colour, a `--verbose` flag, config keys beyond the constitution, a public library API (`src/lib.rs` exists only so tests can reach the modules).

## Done when

1. `make review-quick` passes (fmt, `cargo clippy --all-targets --all-features -- -D warnings`, tests).
2. `make wheel` produces a wheel and `uvx --from target/wheels/<wheel> dermestes --all` runs on a real repo I will point you at.
3. You report wall-clock time for `--all` and default mode on that repo.
4. You list every finding on that repo with your own verdict: true positive, false positive, or debatable. Each false positive becomes a `no_flag` fixture before you fix it.
5. You give me a short list of what felt over-built in your own implementation, and what you would delete.
