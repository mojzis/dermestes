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
| 2 | `one-caller` | Function or private method with exactly one production call site |
| 2 | `pass-through` | Function whose body only delegates to another call |
| 3 | `const-param` | Parameter receiving the same value at every call site, or a default never overridden |

Candidates for later, only after the above prove useful on real repos: class with `__init__` plus one method, `**kwargs` never populated, wrap-and-reraise handlers, field-by-field mirrored models.

Each phase must be usable on its own and validated on a real repository before the next begins.

## Precision guards

Always applied, in every check:

- Decorated symbols are exempt from caller-counting checks (routes, fixtures, CLI commands, DI providers, registries). `ignore-decorators` can narrow or extend this.
- Names in `__all__`, names re-exported from an `__init__.py`, and dunder methods are exempt.
- A name that appears as a string literal or `getattr` argument anywhere in the repo is exempt.
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
