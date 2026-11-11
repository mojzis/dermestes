# Build prompt: phase 6, the rest of the signature and classes

First step: copy this file to `docs/plans/BUILD_PROMPT_PHASE6.md` in the first commit.

---

Phases 1–5 are released. This phase widens what dermestes counts, from functions and parameters to
the rest of the signature (variadics, branches, results, `async`) and to classes. Read
`docs/plans/CONSTITUTION.md`, `CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The constitution is
binding, and principle 8 applies to your own code.

It is three stages. Each one ends at a gate where you stop and wait for me. Do not start a stage
before I approve the previous gate. Budget: about 400 lines of Rust per stage, excluding fixtures.

No plan step: start with stage A. First check these claims about the current code, which came from
reading the source without running it. If one is wrong in a way that changes a stage's design, stop
and tell me; otherwise note the correction in your first gate report and carry on:

- `bind()` in `src/const_param.rs` already sees surplus positional and keyword arguments and
  discards them (around lines 253 and 266).
- A call on a call, `C(a).m(b)`, has its callee collapsed to unknown (`src/calls.rs`, around 445).
- `one-impl` checks unknown bases on a class and its descendants but not its ancestors
  (`src/one_impl.rs`, around 173–189).
- The index does not record: class decorators, class end line, whether a call's result is used or
  awaited, return expressions, `await` presence in a body.
- A decorator factory call such as `@retry(times=3)` is walked as an ordinary call, so
  `const-param` already covers factory arguments.

Also record the baseline: `python3 ~/git/pp/dermestes/harness/score.py <check> --list` for the four
existing checks, and `--all` wall-clock on dlt, feast and litestar.

## Amendment 6 (commit it first)

- **"What it is not"**: the sentence "It flags only a repository function that production code
  never mentions by name" becomes "It flags only a repository function or class that production
  code never mentions by name". Reason: a Protocol nobody mentions and an exception nobody raises
  are abstractions kept alive by nothing, which is the tool's purpose. Unused variables, imports
  and unreachable statements stay out; ruff and vulture cover them.
- **Check table**: extend the `const-param` and `dead-code` rows, and add `unused-result`,
  `needless-async` and `fn-class`, with the definitions below under Definitions.
- **"Candidates for later"**: remove `**kwargs` never populated and the class with `__init__` plus
  one method (built here). Remove wrap-and-reraise handlers: ruff `TRY203` and pylint `W0706` cover
  the plain form. Keep mirrored models.
- Write the corpus numbers into the amendment only at each gate, when they exist.

## Stage A: `const-param` forms (no new index pass beyond branch shapes)

**A1. Variadic never populated.** A `*args` or `**kwargs` parameter of a function with at least one
resolved production call, where no call passes a surplus positional or keyword, no call uses a
splat, and no call is unresolved. Names starting with `_` (`**_`) are deliberate sinks and silent.
Reported as one more line in the function's `const-param` block:

```
const-param pkg/client.py:40 fetch
  calls: 5 prod, 1 test
  **kwargs: never populated
  suggest: drop **kwargs
```

When the body's only use of the variadic is forwarding it (`g(**kwargs)`), the suggestion adds "and
the `**kwargs` forwarded at line N". A test call that populates it counts as variation.

**A2. Dead branch of a constant parameter.** This never creates a finding; it adds evidence to one.
For a parameter `const-param` already reports with a literal value: every `if`, `elif`, conditional
expression or `while` whose test is exactly `p`, `not p`, `p is None`, `p is not None`,
`p == <literal>`, `p != <literal>` or `p in (<literals>)`, where `p` is not rebound anywhere in the
body. Evaluate the test with the constant and report the spans that cannot run:

```
  strict: default False never overridden; dead: lines 48-61
  suggest: drop strict, use False inline, delete lines 48-61 (14 lines)
```

Silent on `and`/`or` tests, walrus, and any rebinding of `p`. JSON gains `dead_spans` on the
parameter.

Fixtures, `flag`: `**kwargs` never populated; `*args` never populated; `**kwargs` forwarded and
never populated; dead `if` branch; dead `else` branch under `p is None`; dead `elif` under
`p == "x"`. `no_flag`: a splat call site; an unresolved call site; `**_`; a test that passes an
extra keyword; a decorated function; an override; `p` rebound before the test; `if p and q`; a
callback passed as a value.

**Gate A.** See "Labelling gate" below.

## Stage B: call context (`unused-result`, `needless-async`)

One new extraction: for each call, whether it is a bare expression statement and whether it is
directly awaited; for each function, whether it has a `return <value>`, whether every return value
is the receiver, and whether the body contains `await`, `async for`, `async with` or `yield`.

**B1. `unused-result`.** A function with at least one value-returning `return`, at least one
resolved production call, no unresolved call, where every resolved call, production and test, is a
bare expression statement (or a bare `await f(...)` statement).

```
unused-result pkg/sync.py:88 push_rows
  calls: 3 prod, 1 test; result used at 0
  suggest: return nothing; drop the return value at lines 95, 102
```

Silent when every return is `self`/`cls` (fluent), for generators, and when any test uses the
result: an assertion on it is a use.

**B2. `needless-async`.** An `async def` whose body has no `await`, `async for`, `async with` or
`yield`, that is not an override and not overridden, with at least one resolved production call, no
unresolved call, and every resolved call directly awaited (`await f(...)`, not `gather(f())`,
`create_task(f())` or an assignment of the coroutine).

```
needless-async pkg/cache.py:17 load_index
  calls: 3 prod, 0 test; awaits in body: 0
  suggest: drop async; drop the await at a.py:10, b.py:22, c.py:7
```

Fixtures, `flag`: result discarded at every site; result discarded under `await`; async with no
await and every site awaited. `no_flag` for `unused-result`: one site assigns the result; a test
asserts on it; `return self`; a generator; the function passed as a value; a decorated route; an
override; an unresolved attribute call with the same name. `no_flag` for `needless-async`: a site
inside `asyncio.gather(...)`; `create_task`; a method that overrides an external base; a Protocol
method; an async generator; a decorated fixture or route.

**Gate B.**

## Stage C: classes (`dead-code` forms, `fn-class`)

Index additions: class decorators, class end line, and an ancestor-side unknown-edge check (a class
is "resolved" only when it, every ancestor and every descendant resolve inside the repo).

**C1. Dead class** (`dead-code`, forms `unused` and `test-only`, JSON `kind: "class"`). A
module-level class whose bare name occurs nowhere in production outside its own span, by the same
occurrence rule as functions. Silent on: any class decorator other than `dataclass`,
`runtime_checkable`, `final`, `total_ordering`; any `metaclass=`; an ancestor that defines
`__init_subclass__`; any `.__subclasses__` use in the repo; an unresolved hierarchy (this silences
ORM models, Pydantic models, enums and exceptions, whose roots are outside the repo, so a
never-raised exception is only reported if the corpus shows the ancestor guard is too strict: tell
me at the gate); public names in a `py.typed` package.

**C2. Dead method family** (`dead-code`, form `unused`, JSON `kind: "family"`). A method defined on
a repo class and overridden in repo subclasses, resolved hierarchy, whose name occurs nowhere in
production or tests outside the family's own bodies. This lifts today's exemption for abstract and
overridden methods in exactly this case. For a Protocol family, also require that no class outside
the family defines the name.

```
dead-code pkg/stages.py:12 Stage.validate (family of 3)
  defs: pkg/stages.py:12 Stage.validate, pkg/parse.py:30 ParseStage.validate, pkg/load.py:8 LoadStage.validate
  calls: 0 prod, 0 test
  suggest: delete validate from Stage and its 2 overrides
```

**C3. `fn-class`.** A class with no bases, no decorator, no class-level assignments, exactly
`__init__` plus one non-dunder method, whose name is never used as a value, where every
instantiation in the repo, tests included, is `C(args).m(args2)` in one expression. Needs the
call-on-call callee kept instead of collapsed to unknown.

```
fn-class pkg/report.py:20 ReportBuilder
  sites: 3 prod, 0 test, all ReportBuilder(...).build(...)
  suggest: make build a function, delete ReportBuilder
```

Fixtures, `flag`: unused private class; test-only class; unused Protocol with no implementation;
dead family across three classes; `fn-class` with three sites. `no_flag`: a Pydantic model; an
exception subclass of a builtin; a class in a registry dict; a class named in a string to
`getattr`; a class with a metaclass; a class used only in an annotation (that is a mention); a
family where one override is called on an unresolved receiver; a family with an external base;
`fn-class` where one site keeps the instance (`b = C(x)`); `fn-class` with a subclass; a class in
`__all__`.

**Gate C.**

## Labelling gate (same at A, B and C)

There are no truth files for these checks, so the gate is a labelling round, not a score.

1. `cargo build --release`. Run `--all --format json` for the stage's checks on the corpus
   snapshots and on `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`).
2. Write one JSONL row per finding to `/tmp/dermestes-phase6/<stage>.jsonl` with repo, check, form,
   path, line, name, and your own verdict: `candidate`, `debatable`, `justified` or `trap`, with a
   one-line reason. Read the code for each one. Cap at 40 rows per repo per check, sampled evenly;
   say how many you skipped. Don't write to `~/git/pp`.
3. Report per check: counts by verdict, every `trap` and `justified` row in full, the guard that
   would silence each, and what that guard costs in candidates.
4. Report that the four existing checks' score lines did not move from the baseline, and the
   `--all` timings before and after.
5. Stop. I label, decide the guards, and tell you what to change. Ship bar per check: 0 traps and
   precision ≥ 85 % on non-debatable rows; a check that misses it is cut or narrowed, and the stage
   still ships without it.

## Implementation notes

- TDD: each fixture before its code. Every trap found at a gate becomes a `no_flag` fixture before
  the fix.
- Every shared precision guard applies to every new form (decorated, dunders, `__all__`,
  re-exports, overrides, external bases, value references, `keep`, computed `getattr`, generated
  files, default excludes, test files never targets).
- Diff-mode identity stays `(check, path, name, param)`. A2 changes a finding's text, not its key;
  add a diff-mode test that a newly dead branch on an already-reported parameter does not appear as
  a new finding, and one where the constant parameter itself is new.
- `suggest:` wording follows the existing checks: imperative, names the symbol, lists sites.
- No new config key, no new dependency. If you think one is needed, stop and ask.

## Out of scope

Constant return, plain base with one subclass, unused parameters, assign-only attributes, dead
constants and aliases, single-entry registries, enum members, modules nothing imports. Cascade
findings (a transitive chain reported as one). Suppression hardening (reporting `keep` markers a
diff adds, config from the base ref). Anything needing types. Scores of any kind.

## Done when

All three gates are approved. `make review-quick` passes and `make wheel` runs via `uvx`.
`docs/src/guide/` covers the new checks and forms with each page still ≤ 60 lines (split a page and
add it to `SUMMARY.md` if needed), and `README.md`, `ARCHITECTURE.md` and the shared CLAUDE snippet
match. Amendment 6 carries the gate numbers. Commits are logical and unpushed; no release.
