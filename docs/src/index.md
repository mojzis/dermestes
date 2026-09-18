# dermestes

> Named after *Dermestes*, the hide beetles museums use to clean a specimen
> down to bare skeleton: everything that is not structure goes.

dermestes finds abstractions in Python code that have not earned their keep,
and reports them as deletion candidates an agent can act on: an ABC or
Protocol with one implementation, a function with one caller, a parameter
nobody varies.

It is deterministic and offline. It never imports or runs your code, never
rewrites it, and prints nothing when there is nothing to report.

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
