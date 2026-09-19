# Tune dermestes

All configuration lives in `[tool.dermestes]` in `pyproject.toml`.

```toml
[tool.dermestes]
exclude = ["migrations/**"]
test-paths = ["tests/**", "test_*.py", "*_test.py", "conftest.py"]
ignore-decorators = ["functools.cache"]
public = ["src/mypkg/api/**"]
disable = ["one-impl"]
```

- `exclude`: files never indexed.
- `test-paths`: files whose code is counted as tests, never as targets.
- `ignore-decorators`: decorators that no longer exempt a function from
  `const-param`, by dotted name or suffix (`cache` covers `functools.cache`).
- `public`: globs treated as public API and exempt.
- `disable`: check ids to skip.

**Suppression** is per definition, on any line of its header (decorators
through the closing `:`), with a mandatory reason: `# dermestes: keep <reason>`.

**Always exempt**, whatever the config: test code, names in `__all__`, names
re-exported from an `__init__.py`, dunder methods (`const-param` checks
`__init__`), ABCs passed to `X.register(...)`, and any name passed as a string
to `getattr`, `setattr`, `hasattr`, `patch` or `patch.object` (a dotted path
counts by its last part), or used as a string key in a registry dict
(`{"csv": CsvExporter}`). Docstrings and string type annotations do not count.

`const-param` also exempts decorated functions (unless `ignore-decorators`
lists every decorator), functions used as a value anywhere (`register(f)`,
`x.f` without a call), overrides, abstract and Protocol methods, functions
called only under `if __name__ == "__main__":`, and constants a test patches.

A library's public functions are called from outside the repository. List
their modules in `public` rather than keeping each one.

next: run `dermestes --all`
