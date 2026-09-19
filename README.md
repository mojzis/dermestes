# dermestes

Finds abstractions in Python code that have not earned their keep, and reports
them as deletion candidates an agent can act on.

> **Status:** phase 1. `one-impl` works; the other checks are not built yet.
> See [`docs/plans/CONSTITUTION.md`](docs/plans/CONSTITUTION.md).

## Checks

| Phase | id | Flags |
|---|---|---|
| 1 | `one-impl` | ABC or Protocol with exactly one production implementation |
| 2 | `one-caller` | Function or private method with exactly one production call site |
| 2 | `pass-through` | Function whose body only delegates to another call |
| 3 | `const-param` | Parameter receiving the same value at every call site, or a default never overridden |

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
  user (an ABC with one implementation, a function with one caller).
- Each finding ends in a `suggest:` line. Apply it and run the tests, or keep
  it with `# dermestes: keep <reason>` on the definition line.
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
