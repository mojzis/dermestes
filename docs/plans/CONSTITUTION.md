# dermestes — project constitution

> Named after *Dermestes*, the hide beetles museums use to clean a specimen down to bare skeleton: everything that is not structure goes. Name was free on PyPI and crates.io as of 2026-09-18; not yet reserved.

## Purpose

dermestes finds abstractions in Python code that have not earned their keep, and reports them as deletion candidates an agent can act on.

It exists because agents write speculative generality quickly: interfaces with one implementation, wrappers with one caller, parameters nobody varies. These are countable, so a machine should count them.

## What it is not

- Not a complexity judge. It never says "too complex", only "this abstraction has N=1 users, here is the evidence".
- Not a refactoring tool. It never rewrites code.
- Not a dead-code finder, clone detector, or style linter. Sibling tools cover those.
- Not an LLM reviewer. It produces the ranked suspect list that a reviewer (human or agent) rules on.

## Principles

Ordered. When two conflict, the earlier one wins.

1. **Precision over recall.** A false positive costs trust; a miss costs nothing, because review still exists downstream. When resolution is uncertain, stay silent. Every check documents its known misses.
2. **A finding is a deletion candidate.** Each finding states what, where, the evidence (counts and locations), and the concrete simplification (inline, collapse to function, drop parameter). If no concrete action can be stated, it is not a finding.
3. **Diff first.** The primary question is "what did this change introduce?". Whole-repo audit is the secondary mode.
4. **Deterministic and offline.** Same input, same output, stable ordering. No network, no LLM, no importing or executing target code.
5. **Fast enough to gate a commit.** Targets, to be measured on a real monorepo and revised: diff mode under 1 s, full audit of 5k files under 5 s.
6. **Quiet when fine.** No findings means no output and exit 0. When there are findings, output is dense and token-aware: no banners, no colour off-TTY, no prose.
7. **Syntactic evidence decides; a type server only confirms.** Candidates come from AST plus import resolution. A type server (via ty-find) may later confirm candidates that syntax cannot resolve. Without it, those candidates are suppressed, not shown with a caveat.
8. **The tool obeys its own rules.** No trait with one implementation, no config key nobody asked for, no shared core crate extracted in advance. Copy from sibling tools first; factor out only when a second consumer needs the same change.

## Checks

Each check has a stable id used in output, config and suppression markers.

| Phase | id | Flags |
|---|---|---|
| 1 | `one-impl` | ABC or Protocol with exactly one production implementation |
| 2 | `const-param` | Parameter whose default is never overridden, or that receives the same literal, at every call site |
| 3 | `pass-through` | Function whose body only forwards its arguments to another repo function |
| 3 | `one-caller` | Private function with one production call site and a one-statement body |

Definitions:

- **one-impl.**
  - A Protocol's implementations are counted structurally, not by subclass. In the corpus
    all 3 Protocols with implementations had zero nominal subclasses.
  - A plain base class with no abstract methods and exactly one production subclass is
    deferred to "later candidates". It would also match exception hierarchies and mixins.
    It is a known miss.
  - An intermediate abstract class is never counted as an implementation. A class counts as
    abstract only while it has unimplemented abstract methods. An `ABC` with none is
    concrete.
- **const-param.** "Same value" means one of:
  - a literal (`True`, `None`, `0.5`, `"cr"`);
  - a module-level constant resolved to its definition;
  - the parameter omitted, so the default applies.

  Two arguments that merely read the same (`conn` at two sites) are never "the same
  value". One call site is enough. `Class(...)` calls count as calls to `__init__`. An
  explicit literal equal to the default counts as not overriding it. A test call that
  passes a different value counts as variation, so the check stays silent.
- **pass-through.** The body, excluding the docstring, is one `return f(...)`,
  `await f(...)` or `f(...)` statement where:
  - `f` resolves to a function defined in the repo;
  - every argument is a parameter name, `self`/`cls` attribute or short literal;
  - `f(...)` is the outermost node: no `bool(...)`, method chain or subscript around it.

  Bodies that are SQL, templates or dict literals are not delegation.
- **one-caller.** Only private (`_x`) functions whose body is a single simple statement,
  with no test caller. This check is narrowed rather than removed: as first written it
  was 4 % precise (20 of 452), and after the narrowing it is 42 %. Revisit after phase 3
  data.

Candidates for later, only after the above prove useful on real repos: class with `__init__` plus one method, `**kwargs` never populated, wrap-and-reraise handlers, field-by-field mirrored models.

Each phase must be usable on its own and validated before the next begins, against
the labelled corpus in `~/git/pp/dermestes/truth/` (precision on non-debatable rows, trap
hits, recall on candidates) and at least one repo with ≥5 abstractions of the check's kind.
Reason: `one-impl` has only 8 ABC/Protocol targets across `~/git`, and all of them are
correctly silent, so no repo there can validate its positive path.

## Precision guards

Always applied, in every check:

- **Framework entry points.** Decorated functions and methods are exempt from **every**
  check, not only caller counting: routes, fixtures, Typer commands, DI providers,
  registries and marimo cells otherwise produce pass-through and const-param noise. So is
  any function whose only call site is under `if __name__ == "__main__":`. Class
  decorators (`@runtime_checkable`, `@dataclass`) do not exempt a class.
  `ignore-decorators` can narrow or extend this.
- Names in `__all__`, names re-exported from an `__init__.py`, and dunder methods are exempt.
- A name is exempt when it appears as a whole string argument to `getattr`, `setattr`,
  `hasattr`, `monkeypatch.setattr`, `patch`, `patch.object`, or as a key in a dict or
  registry literal. Docstrings, free text and string (forward-reference) type annotations
  (`"HasID"`, `bound="HasID"`) never count; those are static references. Reason: the broad
  form suppressed real candidates because the names are common words (`"done"`, `"fetch"`,
  a pandas column called `"annual_kwh"`).
- **A symbol referenced as a value anywhere is exempt from every caller-counting check.**
  That covers being passed as an argument, assigned, returned, put in a container, or
  registered (`Depends(f)`, `env.filters[...] = f`, `partial(f)`, `to_thread(f)`,
  `x is f`). Reason: a callback's call sites are invisible.
- **Calls are counted only after resolution.** A bare-name call counts only when the name
  resolves, through imports including `import x as y`, to the definition. An attribute
  call `obj.name(...)` counts only when `obj` resolves to the defining module or class.
  Anything else is unknown, and unknown means silent. Reason: `dict.update` counted as
  calls to a Typer command called `update`; `jobs.get` got 154 calls, all of them
  `dict.get`.
- **Test callers veto.** One-caller and pass-through stay silent when any test calls or
  patches the symbol, because that makes it a seam. For const-param, a test passing a
  different value counts as variation.
- **Overrides are exempt.** Methods that override a base-class method, with the base
  resolved or not, are exempt, except overrides whose body only calls `super()` with the
  same arguments. Those are pass-through candidates.
- Test code is never a target. Test usages are counted separately from production usages and shown as such.
- If a base class or callee cannot be resolved to a definition inside the repo, the hierarchy or call is unknown, and nothing depending on it is flagged.
- Inline suppression: `# dermestes: keep <reason>` on the definition line. The reason is mandatory.

## Interface contract

```
dermestes                 # findings introduced by staged + unstaged changes vs HEAD
dermestes --base <ref>    # findings introduced since <ref>
dermestes --all           # full audit
dermestes --format json   # machine-readable, same content
dermestes guide           # agent-oriented usage instructions
```

Exit codes: `0` no findings, `1` findings, `2` usage or internal error.

Config lives in `[tool.dermestes]` in `pyproject.toml`. Initial keys, nothing else until a real need appears: `exclude`, `test-paths`, `ignore-decorators`, `public` (globs treated as public API and exempt), `disable` (check ids).

Text output, one block per finding, sorted by path then line:

```
one-impl src/pay/gateway.py:12 PaymentGateway (ABC, 3 abstract methods)
  impl: src/pay/stripe.py:8 StripeGateway
  tests: 0 impls
  suggest: inline into StripeGateway, delete PaymentGateway
```

## Engineering rules

- Rust, tree-sitter-python, shipped as a Python wheel via maturin, runnable as `uvx dermestes@latest`.
- Tests are fixture-driven. Each check has `fixtures/<id>/flag/` and `fixtures/<id>/no_flag/` Python cases with expected output. Every false positive found in the wild becomes a `no_flag` fixture before the fix is written.
- An unparseable file is skipped with a one-line note on stderr. It never fails the run.
- Every dependency needs a one-line justification in the PR that adds it. No async runtime. Parallel parsing via rayon is fine.
- No feature, flag or config key is added speculatively. It is added when a real repo needed it, and the commit says which.

## Non-goals

Auto-fix. Languages other than Python. Editor or LSP integration. Complexity scores. Dashboards.

## Amendments

This document changes only in a commit that states the reason. If practice and constitution disagree, fix one of them in that commit.
