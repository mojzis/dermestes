# Build prompt — phase 3 (`pass-through`, with one-caller folded in)

Paste into Claude Code at the root of `~/git/dermestes`. Written 2026-09-19 from amendment 1 §1,
the round-3 corpus (`~/git/pp/dermestes/summary.md`, round 3) and the phase-2 review
(`~/git/pp/dermestes/history/2026-09-19-phase2-eval.md`). First step: copy this file to
`docs/plans/BUILD_PROMPT_PHASE3.md` in the phase-3 commit.

---

We are building phase 3 of **dermestes**: the `pass-through` check, which absorbs `one-caller`.
Phases 1 (`one-impl`) and 2 (`const-param`) are pushed (`2b7a46b`). Read `docs/plans/CONSTITUTION.md`,
`CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The constitution is binding. Principle 8 applies
to your own code.

Reply with a plan of at most 15 lines, covering step 0, the amendment and phase 3: new modules, what
you reuse from phase 2's call-site index, data structures, and any new dependency with a one-line
reason. Then stop and wait for my approval. Budget: about 800 lines of Rust excluding fixtures.
Phase 2 went 30 % over budget; if the plan needs more, say why instead of proceeding.

**Constitution changes:** only the amendment below. If anything else in the constitution looks wrong
while you build, stop and ask. Phase 2 changed four guards without asking; they were accepted after
the fact, but don't do that again.

## Step 0: phase-2 fixes (before phase 3, separate commits, TDD each with a fixture)

From the phase-2 evaluation (live repos, outside repos, corpus):
1. **const-param skips functions with 0 production calls.** A function called only from tests is
   dead code, not a constant parameter. Today it gets `suggest: drop <p>`, which is wrong and can be
   harmful: braindump `append_index(type_or_dir)` would turn a generic index writer into a
   todos-only one. Stay silent (out of scope: a dead-code check).
2. **Clearer const-param text when callers pass the value explicitly.** "default X never overridden"
   is misleading when every caller writes `p=X`. Say `always X (= default), passed explicitly at N
   sites`. When callers pass it, the suggestion must say that they need editing:
   `suggest: drop <p> (and the argument at <path:line>, …), use <v> inline`. List at most 3 sites,
   then `+N more`. The JSON gets a `sites` array (path:line of each explicit argument).
3. **Default excludes: `examples/`, `docs/`, `**/test/resources/**`.** These are 11 % of findings on
   feast and 21 % on dlt (one dlt tutorial alone gives 11). All are technically correct, and all
   are noise: the explicit parameter is what the example teaches. They go into the built-in default
   `exclude`, and an explicit `exclude` in config replaces the defaults. Document this in `tune.md`. Expected cost on the corpus: 2 const-param candidates in esl and
   cteni `examples/`, which is accepted.
   Add a line to the constitution's config section; this is part of the amendment.
4. **Skip generated files:** a file whose first 5 lines contain `Do not edit` or `DO NOT EDIT`
   (litestar `repository/abc/_sync.py`, generated from `_async.py`).

## Amendment 2 (apply first, in its own commit)

Edit `CONSTITUTION.md`:
- **Retroactive record** of the phase-2 guard changes, which the user accepted on 2026-09-19:
  - `@staticmethod`/`@classmethod`/`@abstractmethod` don't exempt;
  - `ignore-decorators` only narrows;
  - methods overridden by a subclass, and every method of a class with an external base, are exempt;
  - a constant patched by string is not "the same value";
  - `keep` works on any header line.

  They are already in the text. Add a dated "Amendment 2" note listing them, next to amendment 1.
- **one-caller is folded into pass-through.** The check table has three checks. `pass-through`
  has two forms:
  - **forward** (the existing definition): the body, excluding the docstring, is one
    `return f(...)`, `await f(...)` or `f(...)` where `f` resolves to a repo function, every
    argument is a parameter name, a `self`/`cls` attribute or a short literal, and `f(...)` is the
    outermost node. Any number of callers.
  - **single-use** (was one-caller). All of these must hold:
    - the name is private (`_x`, not a dunder);
    - it is defined at module or class level, not nested in a function;
    - the body, excluding the docstring, is exactly one *simple* statement: an expression,
      `return`, assignment or `raise`, not `with`/`for`/`if`/`try`, spanning at most 2 lines;
    - the return annotation is not `bool` (named predicates: `_is_truthy`, `_has_author`);
    - it has exactly one resolved production call site, no test caller and no decorator.

    Suggestion: `inline at <path:line>, delete`.
  - Reason (corpus, 596 labelled one-caller rows): the constitution's one-caller as written is
    43 % precise (10 candidates, 12 justified, 1 trap). The ≤2-line and non-`bool` gates bring it
    to 82 % (9 candidates, 1 justified, 1 trap) at 9/20 recall. The misses are multi-statement
    helpers that nobody needs flagged.
- Default excludes and the generated-file rule (step 0.3 and 0.4).

## Why this check

The corpus's forward pass-through candidates are wrappers that only rename or bind a constant:
braindump `resolve_relation`, libris `_load_timeline`, newsparser `search_mcp_mentions` (deprecated,
binds `"mcp"`) and the `super()`-only `ReportStage.get_inputs`. Its noise is almost all framework wiring and
name collisions, which phases 1 and 2 already handle: resolution, the value-reference guard and the
decorator guard. So this phase is mostly the body-shape matcher plus reuse.

## Scope

**1. Body shape** (both forms), on the AST from phase 1's parse, reusing phase 2's call-site
index and resolution. `f` must resolve to a repo function. An unresolved callee (`exc.show()`,
`bool(x.get())`, `conn.execute("…")`, `max(…)`) is never a pass-through.

**2. Guards.** All of phase 2's apply: decorated, overrides (including external bases), abstract
and Protocol methods, dunders, `__all__`/re-exports/`public`, `keep`, value references,
only-under-`__main__`, and the zero-resolved-calls skip. In addition:
- test callers veto both forms (constitution: "Test callers veto");
- `super()`-only overrides that forward the same arguments are forward pass-throughs (the
  override guard defers them to this phase);
- a function named in `[project.scripts]` or `[project.entry-points]` of any `pyproject.toml` is
  exempt (esl `da`/`md2pdf`/`doit`, cteni console mains).

**3. Output.**
```
pass-through braindump/core/entries.py:474 resolve_relation
  form: forward → braindump/core/entries.py:459 resolve_entry; calls: 1 prod, 0 test
  suggest: call resolve_entry directly at braindump/core/entries.py:492, delete resolve_relation
```
JSON fields: `check`, `path`, `line` (the `def` line), `name` (qualified), `form`
(`forward` | `single-use`), `target` (for forward: the callee's qualified name and path:line),
`calls` {`prod`, `test`}, `sites` (prod call sites), `suggest`. The corpus scorer matches `path`,
`line` and `name`.

**4. Diff mode.** The base-versus-head comparison is unchanged: a new wrapper, or the deletion of
the second caller, shows up.

**5. Guide pages.** Cover the check and its two forms in `triage.md` (how to read a finding and how
to act on it: inline, delete, run the tests) and in `tune.md`. List the known misses.

## Fixtures (`fixtures/pass-through/`)

`flag`:
- forward `return f(a, b)`, `await f(x)`, and a bare `f(x)` statement;
- forward via `self.m(...)`, and via a `super().m(same args)`-only override;
- single-use: a private one-line helper with one call;
- a `keep` marker without a reason.

`no_flag`, one per trap class seen in the corpus:
- marimo cells (`@app.cell`), FastAPI routes and a `Depends(provider)`, a Jinja filter registered
  via `env.filters["x"] = f`;
- a `dict.get`-style collision: the callee name matches a repo function, but the receiver is an
  unresolved local;
- a `bool(f(x))` wrapper and a `f(x).attr` chain (not outermost);
- a wrapper whose parameter is `Any` and whose body calls a method on it (typer-agentic `_show`:
  the callee is unresolved, and the `Any` is what makes callers type-check);
- watchdog-style handler overrides (`on_modified` → `self._rebuild()`) of an external base;
- a nested closure passed to `create_task`/`Thread(target=…)`;
- a `_is_x() -> bool` predicate;
- a `main()` in `[project.scripts]`;
- a private helper with the same name as a method of another class that has the calls (cteni
  `MemoryVerseGenerator._get_instructions`: its real call count is 0);
- a single-use helper with 2 test callers;
- a single-use helper whose one statement is a 5-line dict literal (`_card_view`);
- a single-use helper that is a `with`/`for` block.

Every false positive in the wild becomes a `no_flag` fixture before the fix.

## Validation (constitution §3)

1. **Corpus.** `python3 ~/git/pp/dermestes/harness/score.py pass-through --list` after
   `cargo build --release`. It joins on (path, line, name) against 243 labelled pass-through rows
   and 596 one-caller rows, which are scored under pass-through now. That is 29 candidates after
   de-duplicating sites labelled under both checks. Targets:
   - precision ≥ 70 % on non-debatable rows;
   - ≤ 1 trap hit;
   - recall reported.

   The simulated gates gave forward 7 of 12 and single-use 9 of 11 non-debatable hits. `const-param` must stay at
   ≥ 77 % precision with 0 trap hits, and `one-impl` silent. Give every unlabelled finding your
   own verdict.
2. **Outside repos** in `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`):
   `--all` and diff-mode wall-clock, plus the findings with your verdict. Also report the
   const-param counts before and after step 0.
3. Don't write to `~/git/pp`. Report the numbers instead.

## Out of scope

Type inference, ty-find or LSP, auto-fix, caching, a dead-code check (test-only or zero-caller
functions), unused parameters, grouping several const-param findings on one function (noted for
later), and new config keys beyond the default excludes.

## Done when

1. `make review-quick` passes, and `make wheel` runs via `uvx`.
2. You have reported the `score.py` lines for all three checks, the unlabelled findings with your
   verdicts, and the outside-repo timings and findings.
3. You have listed what felt over-built.
4. Commits are logical and unpushed. Stop before any phase 4.
