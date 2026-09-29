# Build prompt: fix round after phase 3 (no new check)

Paste this into Claude Code at the root of `~/git/dermestes`. It was written on 2026-09-28 from the
phase-3 evaluation (`~/git/pp/dermestes/history/2026-09-28-phase3-eval.md` and its `verdicts.md`).
First step: copy this file to `docs/plans/BUILD_PROMPT_FIXES4.md` in the first commit.

---

Phases 1–3 are pushed (`d501f11`): `one-impl`, `const-param` and `pass-through` (forward and
single-use). This round fixes what real code showed and adds no new check. Read
`docs/plans/CONSTITUTION.md`, `CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The constitution is
binding.

Reply with a plan of at most 12 lines, then stop and wait for my approval. Budget: about 400 lines of
Rust, excluding fixtures.

**Constitution changes:** only amendment 3 below. If anything else looks wrong, stop and ask.

Each fix gets its own commit, done TDD with a fixture first. Every false positive below becomes a
`no_flag` fixture before its fix.

## Amendment 3 (apply first, in its own commit)

- `exclude` has two meanings:
  - **Default excludes** (`examples/`, `docs/`, `**/test/resources/**`) are still indexed. Their
    calls and value references count as evidence, but they produce no findings. Reason: dlt
    `rest_api_source` got 6 const-param findings once a value reference in `docs/education` stopped
    counting. Each would drop a parameter from dlt's public API.
  - An **explicit `exclude`** keeps its meaning of "never indexed", for vendored or broken code.
    Document the difference in `tune.md`.
- **Test references include string literals.** In test code, a string literal equal to a symbol's
  bare name counts as a test reference to that symbol, if the symbol is a module-level function or
  method of the same repo and its name is private. Reason: dlt `_load_raw_text` is patched through
  `patch(f"dlt.helpers.marimo.utils.{fn}")` with the name as a `parametrize` value.
- **forward pass-through** stays silent when the wrapper is public and the target is private. The
  wrapper is then the public name of the private function (introspect `refresh_status`,
  `session_cost_subquery_filtered`).
- **const-param findings of one function are grouped** into one block (see 5).

## Fixes

1. **Excluded-by-default files count as callers** (amendment 3). Fixtures:
   - `no_flag`: a library function whose only varying call is in `docs/lesson.py`;
   - `no_flag`: a function returned as a value from `examples/x.py`;
   - `flag`: an explicit `exclude = ["vendor/**"]` whose calls are not counted.
2. **Shadowing guard for forward.** Skip a forward finding when, at any production call site, the
   target's name is bound in the enclosing function (a parameter, local assignment, `for` target or
   import alias). Case: braindump `_body_revision` forwards to `body_revision`, but both call sites
   are inside `update_entry_partial(…, body_revision: str | None)`. The suggestion would call a str.
3. **String-literal test references** (amendment 3). Fixture: a `no_flag` single-use helper patched
   via `patch(f"pkg.mod.{name}")`, with the name in `pytest.mark.parametrize`.
4. **forward: public wrapper of a private target is silent** (amendment 3). The suggestion must also
   carry every bound argument. When the wrapper passes a literal or a `self`/`cls` attribute, the
   suggestion names it:
   `call _current_indicator(request, notify=True) directly at …`. Today the text drops `notify=True`,
   litestar `self._write_sync` and the 3 `self` attributes of `_create_transfer_data_fn`. Also say
   when the wrapper is `async` and the target is not: `(drop the await)`.
5. **Group const-param findings per function.** One block per function, with one line per parameter:
   ```
   const-param bookmaker/images.py:62 compose_strips
     calls: 3 prod, 0 test
     gap: default 0 never overridden
     margin: default 0 never overridden
     border: default 0 never overridden
     suggest: drop gap, margin, border, use 0 inline
   ```
   JSON keeps one object per parameter (the scorer joins on `param`), plus a shared `group` key
   (`path:line`). Corpus: 108 findings → 85 blocks; feast 78 → 46.

## Out of scope

New checks (dead code, 1-method Protocol arity, the constitution's "later" list), type inference,
auto-fix, and new config keys.

## Validation

1. `cargo build --release`, then `python3 ~/git/pp/dermestes/harness/score.py <check> --list` for all
   three checks. Expected:
   - pass-through ≥ 82 %, 0 traps, recall 14/30 or better; `refresh_status` and
     `session_cost_subquery_filtered` go silent (`session_cost_subquery_filtered` is labelled
     debatable).
   - const-param 76 %, 0 traps, 56/75. esl `get_next_version(base_dir)` goes silent again.
   - one-impl silent.
2. Outside repos `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`):
   - dlt `rest_api_source` is silent;
   - dlt `_load_raw_text` is silent;
   - report the const-param and pass-through counts before and after, and `--all` wall-clock.
3. Live: braindump `_body_revision` is silent.
4. Don't write to `~/git/pp`.

## Done when

`make review-quick` passes. You have reported the score lines, the outside counts, and the list of
findings each fix removed. Commits are logical and unpushed.
