# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user.

```
one-impl src/pay/gateway.py:12 PaymentGateway (ABC, 3 abstract methods)
  impl: src/pay/stripe.py:8 StripeGateway
  suggest: inline into StripeGateway, delete PaymentGateway
const-param src/db.py:209 insert_history(duration_s)
  calls: 2 prod, 0 test; default None never overridden
  suggest: drop duration_s, use None inline
```

The first line is the check id, the definition and what it is. The indented
lines are the evidence and the concrete simplification. A Protocol's impl may
be structural (it has every method, it never subclasses the Protocol).
`const-param` says `always <value>` when every call passes the same literal or
module constant instead of leaving the default.

**For each finding:**

1. Read the evidence. Is it a deliberate seam the counts cannot see (a plugin
   point, a public API, a knob a caller outside the repo turns)?
2. If not, apply the `suggest:` line. For `const-param`: remove the parameter,
   use the value inside the body, delete every branch the value makes dead
   (`if debug:` with `debug=False`), then update the calls. Run the tests.
3. If it is deliberate, suppress it with a reason on any line of its header
   (a decorator through the closing `:`): `# dermestes: keep <reason>`.
   Without a reason it still reports, with a `keep: missing reason` line.

**Known misses** (it stays silent, by design):

- `one-impl`: any test implementation hides it; so does a base or subclass it
  cannot resolve (third-party, dynamic `type()`, a non-`ABCMeta` metaclass),
  or a plain base class (no `ABC`, no abstract methods) with one subclass.
- `const-param` counts only calls it resolves: `f(...)` and `mod.f(...)`
  through imports, `self.m(...)`, `super().m(...)`, `Class(...)`. A call on a
  variable (`tr = T(); tr.done()`, `self.client.get()`) is not followed.
- `const-param` is silent for functions passed or referenced as values,
  decorated ones, overrides and overridden methods, methods of a class with a
  third-party base, and any function called with `*args`/`**kwargs`.
- A variable is never "the same value", even when every caller passes its
  own parameter straight through; nor are enum members or class attributes.
- Files with syntax errors are skipped, so what they define or call is not
  counted.
- Diff mode sees staged and unstaged changes, not untracked files: `git add`
  (or `git add -N`) a new file first.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
