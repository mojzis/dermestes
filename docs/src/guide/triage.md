# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user, or none.

```
const-param src/db.py:209 insert_history
  calls: 2 prod, 0 test
  strict: default False never overridden; dead: lines 214-219
  **kwargs: never populated
  suggest: drop strict, use False inline, delete lines 214-219 (6 lines); drop **kwargs
pass-through src/entries.py:474 resolve_relation
  form: forward → src/entries.py:459 resolve_entry; calls: 1 prod, 0 test
  suggest: call resolve_entry directly at src/entries.py:492, delete resolve_relation
dead-code src/report.py:543 _fetch_cost_rows
  calls: 0 prod, 2 test
  tests: tests/test_report.py:30, tests/test_report.py:41
  suggest: delete _fetch_cost_rows and the tests that only exercise it, or move it into tests/test_report.py if they use it as setup
unused-result src/sync.py:88 push_rows
  calls: 3 prod, 1 test; result used at 0
  suggest: return nothing; drop the return value at lines 95, 102
```

The first line is the check id and the definition; indented lines are the
evidence and the fix. `dermestes guide checks` explains each check's evidence
lines and what it deliberately stays silent on.

**For each finding:**

1. Read the evidence. Is it a deliberate seam the counts cannot see (a plugin
   point, a public API, a knob a caller outside the repo turns)?
2. If not, apply the `suggest:` line, then run the tests.
   - `one-impl`: inline the ABC or Protocol into its one implementation.
   - `const-param`: remove the parameter, use the value in the body, delete
     the `dead:` lines and simplify the listed conditionals, then update the
     calls. A `never populated` variadic: remove it and what it was forwarded to.
   - `pass-through`: replace each listed call with the call the suggestion
     writes (forward, rename) or the body (single-use), then delete it.
   - `dead-code`: delete it; delete or rewrite the listed tests with it. A
     class goes with its methods; a family goes from every class on `defs:`.
   - `unused-result`: turn each listed `return <value>` into a bare `return`
     (or delete a trailing one) and drop the return annotation.
   - `needless-async`: remove `async` and the `await` at each listed site.
3. If it is deliberate, suppress it with a reason on any line of its header
   (a decorator through the closing `:`): `# dermestes: keep <reason>`.
   Without a reason it still reports, with a `keep: missing reason` line.

Work one finding at a time and re-run: deleting one abstraction often changes
the counts of another. Syntax-error files are skipped. Diff mode ignores
untracked files: `git add -N` them first.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
