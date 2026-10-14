# Build prompt: phase 5, `dead-code`

Paste this into Claude Code at the root of `~/git/dermestes`. Written 2026-10-14 from the round-6
corpus (`~/git/pp/dermestes/summary.md` round 6, `dead-code-amendment-draft.md`,
`notes/*.dead-code.md`). First step: copy this file to `docs/plans/BUILD_PROMPT_PHASE5.md` in the
first commit.

---

Phases 1–3 and the fix round are pushed (`8c0be63`). This phase adds one check, `dead-code`. Read
`docs/plans/CONSTITUTION.md`, `CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The constitution is
binding, and principle 8 applies to your own code.

Reply with a plan of at most 12 lines, then stop and wait for my approval. Budget: about 500 lines of
Rust, excluding fixtures.

**Constitution changes:** none. Amendment 4 (below, for reference) is already committed as `90fd7f6`.
If anything in it looks wrong, stop and ask.

## Amendment 4 (already applied in `90fd7f6`)

- **"What it is not"**: replace "Not a dead-code finder, clone detector, or style linter. Sibling
  tools cover those." with:
  > Not a reachability analyser, clone detector, or style linter. It flags only a repository
  > function that production code never mentions by name: an abstraction kept alive by its tests,
  > or by nothing.

  Reason: no sibling tool covers dead code (biston = clones, gerenuk = test selection, tyreach =
  a reachability snapshot). Whole-program reachability needs resolution dermestes lacks (principle
  7); "nobody mentions the name" does not.
- **Check table**: add `| 4 | dead-code | Function or method no production code mentions: only tests (test-only) or nothing (unused) |`.
- **Definition** (add under Definitions):
  - **dead-code.** A module- or class-level function or method, not nested, whose bare name occurs
    nowhere in production outside its own body. Two forms: `unused` (no occurrence at all) and
    `test-only` (only test code mentions it).
    - An *occurrence* is any of: a name or attribute (`f`, `obj.f`, `obj.f()`, any receiver,
      resolved or not), an imported name, a string literal that spells the name (`"f"`,
      `"pkg.mod.f"`, `"pkg.mod:f"`), or the name as a whole word in any non-`.py` file the walk
      sees (templates, TOML, YAML, Markdown, `.mako`). Default-excluded paths count as production.
    - Public names (no leading `_`) are flagged only outside a package that ships `py.typed`. In a
      typed package they may be public API used by people the repo cannot see.
    - Exempt, beyond the shared guards: `pyproject.toml` entry points; `upgrade`/`downgrade` in a
      module that assigns `revision` and `down_revision` at top level (Alembic); `_repr_*_` display
      hooks.

    Reason (corpus round 6: 247 rows labelled, 16 repos including dlt, feast, litestar):
    - Counting resolved calls, as the other checks do, is wrong here: 36 of the 60 rows with 0
      resolved production calls but a production occurrence of the name were traps (registries,
      `Depends`, `partial`, `Thread(target=…)`, Jinja globals, argparse `type=`). Name occurrence
      over-counts uses and loses only recall.
    - Name-dead rows with no guard: 73 % precise, 4 traps. Private names: 21/21 candidates. Public
      names in the 6 repos with `py.typed`: 20 candidates, 37 debatable, 21 justified (documented
      library API), 4 traps. The gate "private, or outside a typed package" keeps 46 candidates, 5 debatable, 0
      justified, 0 traps: 100 % on non-debatable, 46/66 recall.
    - The 4 traps (dlt `is_iso_date` etc., `getattr(detections, "is_" + name)`) are already
      silenced by the existing computed-`getattr` module guard.
- **Known misses** (also in `triage.md`): a function whose only callers are
  themselves dead; a name mentioned only in prose (`plans/*.md`) or only in an application's
  `__all__`; a name that collides with another method; any public name in a typed package.

## The check

Output, one block per function, sorted with the others:

```
dead-code src/pkg/report.py:543 _fetch_cost_rows
  calls: 0 prod, 2 test
  tests: tests/test_report.py:30, tests/test_report.py:41
  suggest: delete _fetch_cost_rows and the tests that only exercise it, or move it into tests/test_report.py if they use it as setup
```

```
dead-code braindump/digest.py:314 _validate_item
  calls: 0 prod, 0 test
  suggest: delete _validate_item
```

JSON: one object per finding with `form` (`unused` | `test-only`), `name`, `path`, `line`, the test
sites, and `suggest`. `score.py` joins on (path, line, name).

Implementation notes:
- The occurrence set is a new per-file index pass (names, attribute names, import names, string
  spellings), split into production and test. Reuse the walker for non-`.py` files: read files under
  2 MB that decode as UTF-8, split into identifier words, keep a set. No new dependency.
- `py.typed`: the walker already sees every file; record the directories that hold one. A function
  is in a typed package when any ancestor directory of its file holds `py.typed`.
- Every shared precision guard applies (decorated, dunders, `__all__`, `__init__` re-exports,
  overrides and overridden methods, external bases, `keep`, computed `getattr`, generated files,
  default excludes and test files never targets).
- `const-param` and `pass-through` already stay silent on test-only functions; keep that.
- Diff mode: "new at head vs base" already covers deleting the last production mention. Add a
  diff-mode test for it.

## Fixtures (TDD; each before its code)

`flag`:
- an unused private function; an unused private method;
- a test-only private function (both the `delete` and `move` wording appear in `suggest`);
- an unused public function in a package without `py.typed`;
- a recursive private function with no other caller (its own body does not count).

`no_flag`, one per trap class the corpus found:
- a public unused function in a package with `py.typed`;
- a callback passed as a value (`Thread(target=f)`, `partial(f)`, `Depends(f)`);
- a function in a registry dict, and one registered as a Jinja global under another name
  (`env.globals["file_url"] = url_for_file`);
- `getattr(obj, "_hook")`, and a module read with `getattr(mod, "is_" + name)`;
- a name used in a `.html` template, and one used only in a `.md` file;
- a `pyproject.toml` `[project.scripts]` entry;
- Alembic `upgrade`/`downgrade` (module with `revision`/`down_revision`);
- a decorated route and a `@app.cell`;
- an override of an external base (`asyncio.Queue._init`);
- `A.close` kept alive by `obj.close()` on an unresolved receiver;
- a function called only from `docs/` or `examples/`;
- `_repr_html_`.

## Out of scope

Transitive dead code (callers that are themselves dead), dead classes and modules, unused
parameters, the 1-method Protocol arity probes, the constitution's "later" list, new config keys, and
the round-5 nits.

## Validation

1. `cargo build --release`, then `python3 ~/git/pp/dermestes/harness/score.py dead-code --list`.
   It scores the 13 snapshots plus dlt, feast and litestar (`truth/<repo>.dead-code.jsonl`).
   Targets: precision ≥ 90 % on non-debatable rows, 0 trap hits, recall reported (simulated
   46/66). dlt and feast rows were sampled (40 per repo), so unlabelled findings there are expected:
   give each one your own verdict. Every unlabelled finding elsewhere is a bug or a missing label;
   say which.
2. The other checks must not move: pass-through 89 %, 16/30 (after the `pass-through-recall` merge; 88 %, 14/30 without it); const-param 76 %, 56/75; one-impl
   silent.
3. Seeds: introspect `_fetch_cost_rows` flags (test-only); live braindump `append_index` (test-only
   at d501f11's time; check whether it still is).
4. Outside repos `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`):
   dead-code counts and `--all` wall-clock before and after (dlt was 0.63 s).
5. Don't write to `~/git/pp`.

## Done when

`make review-quick` passes, `make wheel` runs via `uvx`, `docs/src/guide/` covers the new check
(each page ≤ 60 lines). You have reported the score lines, the unlabelled findings with your
verdicts, and the outside counts and timings. Commits are logical and unpushed.
