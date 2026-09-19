# Tune dermestes

All configuration lives in `[tool.dermestes]` in `pyproject.toml`.

```toml
[tool.dermestes]
exclude = ["migrations/**"]
test-paths = ["tests/**", "test_*.py", "*_test.py", "conftest.py"]
ignore-decorators = ["app.route"]
public = ["src/mypkg/api/**"]
disable = ["one-impl"]
```

- `exclude`: files never indexed.
- `test-paths`: files whose code is counted as tests, never as targets.
- `ignore-decorators`: narrows or extends the decorated-symbol exemption
  (read by the caller-counting checks; `one-impl` does not use it).
- `public`: globs treated as public API and exempt.
- `disable`: check ids to skip.

**Suppression** is per definition, on its line, with a mandatory reason:
`# dermestes: keep <reason>`.

**Always exempt**, whatever the config: test code, names in `__all__`, names
re-exported from an `__init__.py`, dunder methods, ABCs passed to
`X.register(...)`, and any name passed as a string to `getattr`, `setattr`,
`hasattr`, `patch` or `patch.object` (a dotted path counts by its last part), or
used as a string dict key. Docstrings and string type annotations do not count.

next: run `dermestes --all`
