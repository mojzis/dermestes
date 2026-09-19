# Build prompt — phase 2 (`const-param`)

Paste into Claude Code at the root of `~/git/dermestes`. Written 2026-09-19 from amendment 1 §1,
the corpus (`~/git/pp/dermestes/summary.md`) and the phase-1 review. First step: copy this file to
`docs/plans/BUILD_PROMPT_PHASE2.md` in the phase-2 commit.

---

We are building phase 2 of **dermestes**: the `const-param` check. Phase 1 (`one-impl`) is
committed (`14b1e6d`, `9f3637b`, `4b0c3a9`). Read `docs/plans/CONSTITUTION.md` (as amended),
`CLAUDE.md` and `docs/dev/ARCHITECTURE.md` first. The constitution is binding. Principle 8 applies
to your own code.

## Step 0: phase-1 fixes from the evaluation (before phase 2, separate commits)

`~/git/pp/dermestes/history/2026-09-19-phase1-eval.md` measured `one-impl` at 34/35 mutation
recall with 0 extra findings. It also found shared-infrastructure bugs that phase 2 would inherit.
Repro trees are in `history/2026-09-19-phase1-eval/repros/`. TDD each fix: add the fixture, see it
fail, then fix.
1. `keep` marker on a wrapped class header (`):  # dermestes: keep …`, which is what `ruff format`
   produces). Accept it on any line of the `class`/`def` header. Phase 2 needs the same rule for
   `def`.
2. Root = git toplevel, or else the nearest `pyproject.toml`, not the cwd. Run from a subdirectory,
   it currently gives false positives, even with `--all`.
3. Diff mode misses a one-impl that the diff created by deleting the other impl. Report findings
   that are new at head compared with base, not just findings whose definition lines were touched.
   Phase 2 has the same issue: deleting the one call site that varied a parameter makes it constant.
4. uv workspace members (`[tool.uv.workspace] members`) as import roots: the parlint miss.
   feast's `sdk/python` is the same problem.
5. Unresolved bases are matched by bare name across the repo (`Formatter` vs `logging.Formatter`).
   Match them by resolution.
6. Optional: a 1-method Protocol matched structurally by name against an unrelated class
   (`filter` vs `logging.Filter`). Compare arity.

Disagreement 1 (the §3 bar) is resolved: dlt has 42 non-public ABCs/Protocols and feast has 27.
Run feast from `sdk/python`. There it gives 5 findings instead of 1; judge the 4 new ones.

## Before writing code

Reply with a plan of at most 15 lines, covering step 0 and phase 2: new modules, how the call-site index sits next to the
class index, data structures, and any new dependency with a one-line reason. Then stop and wait
for my approval. Budget: about 1,000 lines of Rust excluding fixtures. If the plan needs much
more, say why instead of proceeding.

## Why this check

In the corpus, const-param was the check that pays: 73 % precision after gates, and the best
hits are dead branches (`extract_content(debug)` with 8 dead `if debug:` blocks,
`compose_strips(border)`, `create_assignment(group_id)`). 12 repos were cleaned up from those
labels on 2026-09-19. Most of the noise came from name collisions (`dict.update` counted as
calls to a Typer command `update`), so resolution is the core of this phase, not the check.

## Scope

**1. Call-site index** over all non-excluded files, production and test. A call is attributed to a
repo function only when it resolves:
- bare name: through local defs, nested scopes and imports, including `import x as y`,
  `from m import f as g` and relative imports (reuse phase 1's import resolution);
- `module.f(...)` where `module` resolves to a repo module;
- `self.m(...)` / `cls.m(...)` inside a class, following the class's resolved bases, and
  `super().m(...)`;
- `Class(...)` counts as a call to `Class.__init__` (the first positional arg binds after `self`).

Anything else is unresolved. Record unresolved attribute calls by method name. For a method,
an unresolved `x.name(...)` is a *possible* call. It vetoes a finding only if it could contradict
it: it passes that parameter with a different value, or uses `*args`/`**kwargs`. So `tr.done()`
does not veto `Transcript.done(cost_usd)`, but `d.update(x=1)` with `d` unresolved does veto an
`update(x)` finding. Unresolved bare names never match anything.

**2. Binding.** Bind each call's arguments to parameters: positional by index (skip `self`/`cls`,
and take care with `@classmethod`/`@staticmethod`), then keywords. A call with `*args` or
`**kwargs` splat makes that function unknown, so it stays silent.

**3. The check.** Flag parameter `p` of function `f` when `f` has at least one resolved call site
and at every call site, production or test, `p` is one of:
- omitted, so the default applies;
- a literal (`True`, `None`, `0.5`, `"cr"`, a short tuple of literals);
- a module-level constant resolved to its definition.

All the sites must give the same value, where an omitted `p` counts as its default. Two
arguments that merely read the same (`conn` at two sites) are never "the same value".
- Form A, never overridden (every site omits `p` or passes a literal equal to the default).
  `suggest: drop <p>, use <default> inline`.
- Form B, same literal (no default, or a default that is never used).
  `suggest: drop <p>, use <value> inline`.
- Skip `self`, `cls`, `*args`, `**kwargs`, and any function with zero resolved call sites.

**4. Guards** (constitution §Precision guards, as amended): exempt decorated functions and
methods; overrides (the method name exists on a base, whether the base is resolved or not;
`super()`-only bodies are phase 3's business); abstract and Protocol methods (their signatures
are contracts); dunders other than `__init__`; `__all__`, `__init__` re-exports and `public`
globs; `# dermestes: keep <reason>`; functions referenced as a value anywhere (passed, assigned,
returned, put in a container, `partial(f)`, `Depends(f)`), since their call sites are invisible;
and functions whose only call site is under `if __name__ == "__main__":`.

Also fix one open point from amendment 1 (disagreement 3): a string dict key exempts a symbol only
when the dict's value at that key is a callable or class reference (a registry), not for
`{"done": 3}`. TDD: first a `flag` fixture in `one-impl` where a class name appears only as a plain
dict key, then the fix.

**5. Output.** Text, following the phase-1 style:

```
const-param libris/db.py:209 insert_history(duration_s)
  calls: 2 prod, 0 test; default None never overridden
  suggest: drop duration_s, use None inline
```

JSON fields: `check`, `path`, `line` (the `def` line, not a decorator), `name` (qualified,
e.g. `Transcript.done`), `param`, `value` (source text), `form` (`never-overridden` or
`same-literal`), `calls` {`prod`, `test`}, `suggest`. The corpus scorer depends on `path`,
`line` and `param`.

**6. Diff mode.** Use step 0's base-versus-head comparison. A finding is reported when it exists
at head but not at base. Adding a call with the same literal, or deleting the only call that
varied the parameter, is what makes a parameter constant, and neither has to touch the `def` line.

**7. Guide pages.** Cover the new check in `triage.md` (how to read it, how to act on it: delete
the parameter, delete the dead branches it guarded, run the tests) and in `tune.md`. List the
known misses.

## Fixtures (`fixtures/const-param/`)

`flag`: default never passed with 1 caller; explicit literal equal to the default; same literal
with no default across 2 sites; `Class(...)` → `__init__`; module constant; a caller that goes
through `import … as …`; a `self.` method call; a test call passing the same value; a
`keep` marker without a reason.

`no_flag`: a test passes a different value; a value reference (callback); a decorated route;
an override; a call with a `*args`/`**kwargs` splat; an unresolved `x.name(p=other)`; the
`dict.update` collision; two sites passing the same variable name; `__all__`; an abstract or
Protocol method; zero call sites; a call only under `if __name__ == "__main__":`.

Every false positive in the wild becomes a `no_flag` fixture before the fix.

## Validation (constitution §3)

1. Corpus: `python3 ~/git/pp/dermestes/harness/score.py const-param --list` after
   `cargo build --release`. It joins your JSON with 132 labelled const-param rows (65 candidates)
   on (path, line, param). Targets: precision ≥ 70 % on non-debatable rows, at most 1 trap hit,
   and recall reported. Give every unlabelled finding your own verdict. Snapshots are fixed at
   the labelled SHAs, so the numbers are stable. `one-impl` must stay silent on all 13
   snapshots.
2. Outside repos in `~/git/tyftest/repos/{dlt,feast,litestar}-src` (feast from `sdk/python`). Report `--all` and diff-mode wall-clock,
   and list the findings with your verdict.
3. Don't write to `~/git/pp`. Report the numbers instead.

## Also in this phase (small, separate commits)

- `tests/diff_mode.rs`: remove `GIT_*` env vars from its git commands. On 2026-09-19, a hook's
  `GIT_DIR` leaked into tests like this in two other repos and force-pushed junk.

## Out of scope

pass-through, one-caller, type inference, ty-find or LSP, auto-fix, caching, value-flow beyond
literal/constant, new config keys (unless a real repo needs one, and the commit says which).

## Done when

1. `make review-quick` passes, and `make wheel` runs via `uvx`.
2. You report the score.py line, the unlabelled findings with your verdicts, and the outside-repo
   timings and findings.
3. You list what felt over-built.
4. Commits are logical and unpushed. Stop before phase 3.
