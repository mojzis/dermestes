# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user, or none.

```
const-param src/db.py:209 insert_history
  calls: 2 prod, 0 test
  duration_s: default None never overridden
  suggest: drop duration_s, use None inline
pass-through src/entries.py:474 resolve_relation
  form: forward → src/entries.py:459 resolve_entry; calls: 1 prod, 0 test
  suggest: call resolve_entry directly at src/entries.py:492, delete resolve_relation
dead-code src/report.py:543 _fetch_cost_rows
  calls: 0 prod, 2 test
  tests: tests/test_report.py:30, tests/test_report.py:41
  suggest: delete _fetch_cost_rows and the tests that only exercise it, or move it into tests/test_report.py if they use it as setup
```

The first line is the check id and the definition; indented lines are the
evidence and the fix. `one-impl` names the single implementation of an ABC or
Protocol (maybe structural). `const-param` has a line per parameter: `always
<value>`, or `passed explicitly at N sites` when callers write the default.
`pass-through` forms: `forward` only calls a repo function, binding at most
short literals or one constant; `rename` renames an outside call for 2
callers; `single-use` is a private one-liner with one call. `dead-code`: no
production code mentions the name; `tests:` lists the test lines that do.

**For each finding:**

1. Read the evidence. Is it a deliberate seam the counts cannot see (a plugin
   point, a public API, a knob a caller outside the repo turns)?
2. If not, apply the `suggest:` line, then run the tests.
   - `const-param`: remove the parameter, use the value in the body, delete
     every branch the value makes dead, then update the calls.
   - `pass-through`: replace each listed call with the call the suggestion
     writes (forward, rename) or the body (single-use), then delete it.
   - `dead-code`: delete it; delete or rewrite the listed tests with it.
3. If it is deliberate, suppress it with a reason on any line of its header
   (a decorator through the closing `:`): `# dermestes: keep <reason>`.
   Without a reason it still reports, with a `keep: missing reason` line.

**Known misses** (it stays silent, by design):

- `one-impl`: a test implementation, an unresolvable base or subclass, or a
  plain base class (no `ABC`, no abstract methods) with one subclass.
- Calls count only when resolved (`f()`, `mod.f()`, `self.m()`, `Class()`);
  one on a variable (`tr.done()`) that could bind silences the counters.
- All are silent for functions used as values, decorated ones, overrides,
  overridden methods, methods of a class with a third-party base.
- `const-param`: a variable or enum member is never "the same value".
- `pass-through`: any test caller vetoes it, as do a third-party target for a
  forward, a public wrapper of a private one, `*args`, a multi-statement body.
- `dead-code`: callers that are themselves dead, a name only in prose or
  `__all__`, one shared with another method, public names in a `py.typed` package.
- Syntax-error files are skipped. Diff mode ignores untracked files: `git add -N`.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
