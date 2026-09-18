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
- `ignore-decorators`: narrows or extends the decorated-symbol exemption.
- `public`: globs treated as public API and exempt.
- `disable`: check ids to skip.

**Suppression** is per definition, on its line, with a mandatory reason:
`# dermestes: keep <reason>`.

**Always exempt**, whatever the config: names in `__all__`, names re-exported
from an `__init__.py`, dunder methods, and any name that appears as a string
literal or `getattr` argument in the repository.

next: run `dermestes --all`
