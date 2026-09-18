# Setup dermestes in this repository

dermestes is not configured in this repository yet. Run everything below at
the repository root.

**1. Install.** It is a single binary shipped as a Python wheel; it never
imports or runs your code.

```bash
uv add --dev dermestes
```

**2. Configure.** An empty table is enough; it is also what makes
`dermestes guide` switch to triage.

```toml
[tool.dermestes]
```

**3. Run it on your changes.** With no flags it reports only what the working
tree introduced against `HEAD`. `--base <REF>` compares against another ref;
`--all` audits the whole repository.

**4. Exit codes.** `0` and no output when there is nothing to report, `1` when
findings were printed, `2` on a usage or internal error.

next: run `dermestes`
