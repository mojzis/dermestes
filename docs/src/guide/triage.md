# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user.

```
one-impl src/pay/gateway.py:12 PaymentGateway (ABC, 3 abstract methods)
  impl: src/pay/stripe.py:8 StripeGateway
  suggest: inline into StripeGateway, delete PaymentGateway
```

The first line is the check id, the definition and what it is. The indented
lines are the evidence and the concrete simplification. A Protocol's impl may
be structural (it has every method, it never subclasses the Protocol).

**For each finding:**

1. Read both ends of the evidence. Is the abstraction a deliberate seam the
   counts cannot see (a plugin point, a public API)?
2. If not, apply the `suggest:` line, then run the test suite.
3. If it is deliberate, suppress it with a reason on any line of its header
   (a decorator through the closing `:`): `# dermestes: keep <reason>`.
   Without a reason it still reports, with a `keep: missing reason` line.

**Known misses** (it stays silent, by design):

- Any test implementation hides the finding, even when the one production
  implementation is the only real one: a fake may be a deliberate seam.
- A base or subclass it cannot resolve (third-party, dynamic `type()`,
  metaclass other than `ABCMeta`, a `sys.path` layout other than the repo root
  or `src/`) silences the whole hierarchy.
- A plain base class (no `ABC`, no abstract methods) with one subclass: it
  would also match exception hierarchies and mixins.
- Files with syntax errors are skipped, so their subclasses go uncounted.
- Diff mode sees staged and unstaged changes, not untracked files: `git add`
  (or `git add -N`) a new file first.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
