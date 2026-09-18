---
name: rust-review
context: fork
background: false
description: Deep Rust code quality review. Auto-invoke when finishing a task, before marking work complete, when the user asks to review code, or when preparing a PR. Covers error handling, duplicated logic, test quality, performance patterns, idiomatic Rust, docs-code alignment, and API design beyond what clippy catches.
---

# Deep Code Review for dermestes

Perform a thorough code quality review of the changes in this project. Go beyond
what clippy and rustfmt catch. Focus on the areas below and report findings
grouped by severity: 🔴 Must Fix, 🟡 Should Fix, 🟢 Suggestion.

First, run `cargo fmt --all -- --check` and
`cargo clippy --all-targets --all-features -- -D warnings` to confirm the
automated checks pass. If they don't, report them as findings — do **not** fix
them yourself. This skill reviews; it does not edit.

## Returning results

This skill runs in a fork, so its transcript is never shown to the user. The
complete report must be your **final message** — the full findings text, not a
summary of it, not a pointer to a file, not "see above". Anything you leave out
of that last message is lost.

Concretely:

- Emit every finding in full (severity, location, why, concrete fix) in the
  final message.
- Do not write findings to a scratch file and reference the path instead.
- If the review is clean, say so explicitly and state what you checked.
- Do not end with a question — the caller cannot answer it.

## 1. Error Handling Quality

- `.unwrap()` / `.expect()` outside tests is denied by lint config. Flag any
  instance that slipped through (macros, build scripts, `#[allow]`s).
- Check that `.context("message")` is used on every `?` that crosses an I/O,
  parsing or subprocess boundary — bare `?` loses which operation failed.
- Flag `Result` return types where the function never returns `Err`.
- Flag `.map_err(|_| ...)` that silently discards error information.
- The crate is `anyhow`-based end to end. Flag `String` as an error type, and
  flag any place where a caller has to string-match an error message to make a
  decision — that wants a typed error instead.

## 2. Process and I/O Seams

dermestes's core invariant: `git` is the only subprocess, and it is spawned in
one place. Everything downstream of parsing takes already-parsed data.

- Flag any new `std::process::Command`, or any `std::fs` read or environment
  lookup outside the file walker, config loader and git seam.
- Flag check logic that reads the filesystem itself instead of taking the
  parsed index — that is what keeps checks unit-testable with no `git`.
- Check that command bodies live in `cli.rs`, not `main.rs`.
- Nothing imports or executes target code, and nothing touches the network
  (constitution, principle 4).

## 3. Duplicated Logic

- Search for functions or code blocks that do substantially the same thing with
  minor variations. Flag them and suggest extraction into a shared function,
  trait, or generic.
- Pay special attention to:
  - Qualified-name construction and import resolution
  - Path normalization against the repo root
  - Test-path classification
  - Error handling boilerplate that could be a helper
  - Similar match arms across different functions
- But: the constitution forbids abstractions with one user (principle 8).
  Suggest extraction only when there are two real call sites.
- Suggest concrete refactoring: which function to extract, what parameters it
  should take.

## 4. Test Quality

- **Tests that don't assert**: Flag any test that just calls code without
  asserting on the result. Running without panicking is not a test.
- **Tests that assert too little**: `assert!(result.is_ok())` without checking
  the value inside is weak. Suggest asserting on the actual content.
- **Overly verbose tests**: Tests with excessive setup that obscure what's
  actually being tested. Suggest extracting test helpers or fixtures.
- **Missing edge cases**: For each public function, check if tests cover: empty
  input, error paths, boundary conditions.
- **Test naming**: Test names should describe the behavior being tested, not the
  implementation (`returns_error_on_missing_file`, not `test_function_xyz`).
- **Integration vs unit**: unit tests (`#[cfg(test)] mod tests`) test internal
  logic; `tests/*.rs` exercise the public CLI interface.
- **Exit codes are contract.** `0` no findings, `1` findings, `2` usage or
  internal error; `guide` never returns `1`. Flag any new path that could
  return the wrong one, and any test that asserts on stdout without asserting
  on the exit code.
- **Fixtures are the spec.** Each check has `fixtures/<id>/flag/` and
  `fixtures/<id>/no_flag/` cases. Flag a fix for a false positive that lands
  without a `no_flag` fixture reproducing it.
- **Quiet when fine.** No findings means empty stdout. Flag any banner, summary
  line or stderr noise on a clean run.

## 5. Performance Patterns

- Flag unnecessary `.clone()` — suggest borrowing or restructuring ownership.
- Flag `.collect::<Vec<_>>()` immediately followed by `.iter()`.
- Prefer `&str` over `String`, `&[T]` over `Vec<T>`, `&Path` over `PathBuf` in
  function parameters when ownership isn't needed.
- Flag large structs passed by value instead of by reference.
- Flag repeated linear scans over a collection that is looked up by key — the
  reference graph walk is the likely offender; suggest a `HashMap`/`HashSet`.

## 6. Idiomatic Rust

- Prefer iterator chains over manual `for` loops with `push`.
- Use `if let` / `let else` instead of `match` with a single interesting arm and
  a wildcard.
- Suggest `impl From<X> for Y` instead of standalone conversion functions.
- Suggest `Display` implementations instead of `to_string()` methods.
- Flag `&String`, `&Vec<T>`, `&Box<T>` in signatures — use `&str`, `&[T]`, `&T`.
- Check doc comments on all public types and functions.

## 7. Documentation ↔ Code Alignment

- **`docs/plans/CONSTITUTION.md`** is binding. Flag any output format, flag,
  config key or exit code that differs from its interface contract — the fix
  is an amendment commit, not a silent divergence.
- **`docs/src/guide/*.md`** is what `dermestes guide` prints. Every command,
  flag and config key it names must exist.
- **`docs/dev/ARCHITECTURE.md`** and the **Key Invariants** list in `CLAUDE.md`
  must still be true of the code. An invariant that the code no longer upholds
  is a 🔴 either way: either the code regressed or the doc is lying.
- **`docs/adr/`**: if the change contradicts a recorded decision, flag it — the
  fix is a new ADR superseding it, not a silent divergence.
- **Shared prose**: `README.md` and `docs/src/index.md` are generated between
  `<!-- BEGIN SHARED:... -->` markers. Flag any edit made inside the markers
  instead of in `docs/shared/`.
- Flag any mismatch as 🔴 Must Fix — stale docs are worse than no docs.

## 8. API and Module Design

- Is `main.rs` thin? Argument parsing and exit-code mapping only.
- Are module boundaries clean? Each module should have a clear responsibility.
- Check for any `pub` items that don't need to be public.
- Flag a trait with one implementation, or a config key or flag no real repo
  needed (constitution, principle 8).
- Flag circular dependencies between modules.

## Output Format

Group all findings by severity, then by area. For each finding:

- State **what** the issue is and **where** (`file.rs:line` or function name)
- Explain **why** it matters
- Suggest a **concrete fix** (not just "improve this")

End with a summary: X must-fix, Y should-fix, Z suggestions.

$ARGUMENTS
