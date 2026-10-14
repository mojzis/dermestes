# Build prompt: pass-through recall (rename + constant binding)

Paste this into Claude Code at the root of `~/git/dermestes`. Written on 2026-10-14 from a prototype
run in pp (scratch clone of `8c0be63`). First step: copy this file to
`docs/plans/BUILD_PROMPT_RECALL.md` in the first commit.

A prototype patch is in `~/git/pp/dermestes/patches/2026-10-14-pass-through-recall.diff`, against
`8c0be63`. It is **a reference, not something to apply**: it has env-var switches
(`DERM_PUBLIC_RENAME`, `DERM_NO_CONST`), a redundant helper (`binds_constant` next to
`binds_any_constant`), no fixtures and one failing test. Use it to see where the gates go, then
write the change properly.

---

pass-through finds 14 of the 30 corpus candidates, at 88 % precision. This round adds two narrow
forms that get it to 16/30 with no new false positives on the corpus or on dlt, feast and litestar.
Read `docs/plans/CONSTITUTION.md`, `CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The
constitution is binding.

**Branching:** other feature work may be in progress. If the working tree is dirty, or the current
branch is not `main`, stop and ask. Otherwise branch `pass-through-recall` from `main`.

Reply with a plan of at most 10 lines, then stop and wait for my approval. Budget: about 150 lines
of Rust, excluding fixtures.

**Constitution changes:** only the amendment below, under the next free number. If anything else
looks wrong, stop and ask.

Each rule gets its own commit, done TDD with fixtures first.

## Amendment (apply first, in its own commit)

From the 2026-10-14 recall prototype (`~/git/pp/dermestes/recall-build-prompt.md`).

- **Rename form of `pass-through`.** A private function's body is one call to something that is not
  a repository function (unresolved, external, or an attribute call such as
  `date.fromisoformat(value)`). It is reported when every argument is one of the wrapper's own
  parameters or a receiver attribute (nothing bound), and it has 1–2 production call sites. The
  other single-use gates apply: no decorators, not nested, a ≤2-line simple body, not `-> bool`,
  no `Any` parameter. Reason: braindump `_require_date` is `date.fromisoformat` under another name.
  Public renames stay silent: on dlt and feast they are API vocabulary (`truncate_delta_table`,
  `pretty_format_exception`).
- **Forward may bind one module constant.** A forward wrapper may pass an `UPPER_CASE` module name
  in addition to its parameters, if the wrapper is private, the constant is the only value it binds
  (no literal as well), and there are ≤2 production call sites. Reason: libris `_extract_tool_input`
  binds `TOOL_NAME`. A constant bound at many sites (introspect `_get_read_connection`, 8 callers),
  with a literal as well (`_create_session_stats`), or by a public wrapper (dlt `generate_dlt_id`,
  which owns the ID-length convention) is a reasonable wrapper.

## Rules

1. **Constant binding** (amendment). Extend the forward wrapper's argument check with "an
   `UPPER_CASE` bare name". The suggestion keeps the constant:
   `call extract_tool_input(resp, TOOL_NAME) directly at …`. Fixtures:
   - `flag`: private wrapper binding one constant, 2 callers;
   - `no_flag`: the same with 3 callers;
   - `no_flag`: binds a constant and a literal;
   - `no_flag`: public wrapper binding a constant.
2. **Rename form** (amendment). The suggestion is `call <callee>(<args>) directly at …, delete …`,
   e.g. `call date.fromisoformat(value) directly at braindump/cli/main.py:731, …`. When there is
   one site and the single-use form also applies, keep its `inline at …` wording. Pick the JSON
   `form` value (`rename`, or `single-use`) and document it. Fixtures:
   - `flag`: private `_parse(v): return date.fromisoformat(v)`, 2 callers;
   - `no_flag`: the same, public;
   - `no_flag`: 3 callers;
   - `no_flag`: binds a literal (`request.session.get("user_id")`);
   - `no_flag`: `-> bool` (`croniter.is_valid(s)`);
   - `no_flag`: `Any`-typed parameter (typer-agentic `_show`).
3. **Fix `tests/diff_mode.rs::deleting_the_second_caller_reveals_a_single_use_helper`.** Its
   helper `_label(n): return n.title()` is now a rename with 2 callers, so it is reported before
   the diff too and diff mode stays quiet. Change the helper so it is not a pure rename (e.g. it
   binds a literal: `n.title().strip("_")`), so the test still checks what it says.

## Out of scope

Public renames, dead code (zero callers: bookmaker `get`, libris `_lookup`), multi-statement
bodies, `__all__`-guarded or test-called wrappers, and the polish nits from the fix round
(`self.` in method suggestions, long grouped const-param lines).

## Validation

1. `cargo build --release`, then `python3 ~/git/pp/dermestes/harness/score.py <check> --list` for all
   three checks. Prototype numbers:
   - pass-through 89 %, recall 16/30, 0 traps, 0 unlabelled; new hits are braindump `_require_date`
     and libris `_extract_tool_input`, nothing else;
   - const-param 76 %, 56/75, unchanged;
   - one-impl silent.
2. Outside repos `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`): pass-through
   findings unchanged (17 / 11 / 7). Only dlt `LanceDBClient._create_sentinel_table` may change
   suggestion text (it becomes a forward with a bound constant). Report any other new finding with
   its source lines.
3. Don't write to `~/git/pp`.

## Done when

`make review-quick` passes. You have reported the score lines, the outside counts, and the list of
findings each rule added. Commits are logical and unpushed.
