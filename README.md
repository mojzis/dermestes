# dermestes

Finds abstractions in Python code that have not earned their keep, and reports
them as deletion candidates an agent can act on.

📖 **[Documentation](https://mojzis.github.io/dermestes/)** · part of **[aesop](https://mojzis.github.io/aesop/)**, a small menagerie of tools for agentic Python

> **Status:** phase 6. `one-impl`, `const-param`, `pass-through`, `dead-code`,
> `unused-result` and `needless-async` work.
> See [`docs/plans/CONSTITUTION.md`](docs/plans/CONSTITUTION.md).

## Checks

| Phase | id | Flags |
|---|---|---|
| 1 | `one-impl` | ABC or Protocol with exactly one production implementation |
| 2 | `const-param` | Parameter whose default is never overridden, or that receives the same literal, at every call site; a `*args` or `**kwargs` no call site populates; the branches such a constant makes dead |
| 3 | `pass-through` | Function whose body only forwards its arguments to another repo function, or a private one-line helper with one production call site |
| 4 | `dead-code` | Function, method, class or method family no production code mentions: only tests (`test-only`) or nothing (`unused`) |
| 6 | `unused-result` | Function that returns a value every call site discards |
| 6 | `needless-async` | `async def` that awaits nothing and is awaited directly at every call site |

## Usage

```sh
uvx dermestes@latest              # findings introduced by staged + unstaged changes vs HEAD
uvx dermestes@latest --base main  # findings introduced since main
uvx dermestes@latest --all        # full audit
uvx dermestes@latest guide        # agent-oriented instructions
```

Exit codes: `0` no findings (and no output), `1` findings, `2` usage or
internal error.

## For your CLAUDE.md

<!-- BEGIN SHARED:claude-snippet -->
```markdown
### `dermestes` — over-abstraction

- `dermestes` lists abstractions your change introduced that have a single
  user, or none (an ABC with one implementation, a function with one caller,
  a parameter nobody varies, a result nobody reads).
- Each finding ends in a `suggest:` line. Apply it and run the tests, or keep
  it with `# dermestes: keep <reason>` in the definition's header.
```
<!-- END SHARED:claude-snippet -->

## Development

```sh
make review-quick   # fmt, clippy, tests
make review         # plus cargo-audit and cargo-deny
make wheel          # build a wheel via maturin
make docs           # build the mdBook site + llms.txt
```

## License

MIT
