# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

`dermestes` finds abstractions in Python code that have not earned their keep
(an ABC with one implementation, a function with one caller, a parameter
nobody varies) and reports them as deletion candidates. It is a hybrid
Rust/Python project: a Rust binary packaged as a Python wheel via maturin,
runnable as `uvx dermestes@latest`.

**`docs/plans/CONSTITUTION.md` is binding.** Read it before any change. Its
principles are ordered; when two conflict, the earlier wins. Principle 8
applies to this code: no trait with one implementation, no config key or flag
nobody asked for, no shared crate with the sibling tools (gerenuk, biston) —
copy from them, don't depend on them.

Architecture: `docs/dev/ARCHITECTURE.md`. Decisions and their costs:
`docs/adr/`. Phase plans: `docs/plans/`.

## Common Commands

```sh
# Pre-commit checks (always run before committing)
cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features

make review        # the above, plus cargo-audit and cargo-deny
make review-quick  # skip the network checks
make wheel         # maturin wheel into target/wheels
make docs          # build the mdBook site + llms.txt
```

If formatting fails, fix it with `cargo fmt --all` and re-run.

## Development Workflow

All features and bug fixes follow TDD (red-green-refactor). No implementation
code without a failing test first. Every false positive found in the wild
becomes a `no_flag` fixture before the fix is written.

## Test Changes Require Deliberation

When a test fails during implementation:

1. **Stop and diagnose.** Understand WHY it fails before changing anything.
2. **Default assumption: the test is right.** Fix the implementation first.
3. **If the test genuinely needs updating**, explain what changed and why the
   old assertion is no longer correct before modifying it.
4. **Never weaken an assertion just to make it pass.**
5. **If uncertain, ask.** A 2-line question is cheaper than a silent wrong
   decision.

## Key Invariants

- **Precision over recall.** When resolution is uncertain, stay silent. An
  unresolvable base class or callee makes everything depending on it
  unflaggable.
- **Deterministic and offline.** Same input, same output, stable ordering. No
  network, no LLM, never import or execute target code.
- **Quiet when fine.** No findings means empty stdout and exit `0`. Findings
  exit `1`; usage or internal error exits `2`. `guide` never returns `1`.
- **`git` is the only subprocess**, spawned from one place, and only for
  `git diff --unified=0`. No git library.
- **An unparseable file is skipped** with a one-line note on stderr. It never
  fails the run.
- **`clippy::unwrap_used` / `expect_used` warn outside tests, and the documented
  `cargo clippy … -D warnings` makes them fatal.** Use `anyhow::Context` on
  every `?` that crosses an I/O or parsing boundary.
- **Guide prose lives in `docs/src/guide/`, never in `guide.rs`.** The CLI
  `include_str!`s the same bytes the site publishes. Each page is capped at 60
  lines and ends with exactly one `next: run` line.
- **Every dependency has a one-line justification** in `Cargo.toml` and in the
  commit that adds it. No async runtime.

## Docs

The mdBook site under `docs/` deploys to GitHub Pages on push to `main`
(`.github/workflows/docs.yml`), along with `llms.txt` and `llms-full.txt`.

- Shared prose lives in `docs/shared/` and is injected into `README.md` and
  `docs/src/index.md` by `docs/inject-shared.sh`. Edit the shared file, then
  run the script — never edit between the `<!-- BEGIN SHARED:... -->` markers.
- `docs/gen-version.sh` syncs the docs version badge with `Cargo.toml`.
- `docs/toolchain.sh` pins the `mdbook` / `mdbook-mermaid` pair used by both
  `make docs` and the workflow.
- Adding a page means adding it to `docs/src/SUMMARY.md`; the `llms.txt`
  generator reads that file.
