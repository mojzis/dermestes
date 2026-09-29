# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user.

```
one-impl src/pay/gateway.py:12 PaymentGateway (ABC, 3 abstract methods)
  impl: src/pay/stripe.py:8 StripeGateway
  suggest: inline into StripeGateway, delete PaymentGateway
const-param src/db.py:209 insert_history
  calls: 2 prod, 0 test
  duration_s: default None never overridden
  suggest: drop duration_s, use None inline
pass-through src/entries.py:474 resolve_relation
  form: forward → src/entries.py:459 resolve_entry; calls: 1 prod, 0 test
  suggest: call resolve_entry directly at src/entries.py:492, delete resolve_relation
```

The first line is the check id and the definition; indented lines are the
evidence and the fix. A Protocol's impl may be structural. `const-param`
has a line per parameter: `always <value>` when every call passes the same
literal or constant, `passed explicitly at N sites` when callers write it.
`pass-through` has two forms: `forward`, a body that only calls another repo
function with its own parameters, `self` attributes or short literals; and
`single-use`, a private one-line helper with exactly one production call.

**For each finding:**

1. Read the evidence. Is it a deliberate seam the counts cannot see (a plugin
   point, a public API, a knob a caller outside the repo turns)?
2. If not, apply the `suggest:` line, then run the tests.
   - `const-param`: remove the parameter, use the value in the body, delete
     every branch the value makes dead, then update the calls.
   - `pass-through`: replace each listed call with the call the suggestion
     writes, arguments the wrapper bound included (forward), or with the body
     (single-use), then delete it. A `super()`-only override is just deleted.
3. If it is deliberate, suppress it with a reason on any line of its header
   (a decorator through the closing `:`): `# dermestes: keep <reason>`.
   Without a reason it still reports, with a `keep: missing reason` line.

**Known misses** (it stays silent, by design):

- `one-impl`: a test implementation, an unresolvable base or subclass, or a
  plain base class (no `ABC`, no abstract methods) with one subclass.
- Calls count only when resolved: `f(...)`, `mod.f(...)`, `self.m(...)`,
  `super().m(...)`, `Class(...)`. A call on a variable (`tr.done()`) is not
  followed; one that could bind makes the caller-counting checks silent.
- Both are silent for functions used as values, decorated ones, overrides,
  overridden methods, methods of a class with a third-party base, calls with
  `*args`/`**kwargs`, and functions only tests call (dead code).
- `const-param`: a variable, enum member or class attribute is never "the
  same value".
- `pass-through`: any test caller vetoes it. Never flagged: a wrapper of a
  third-party function, a public wrapper of a private one, a `*args`
  forwarder, a multi-statement or `with`/`for`/`if` helper, a `-> bool`
  predicate, an `Any`-typed wrapper.
- Syntax-error files are skipped. Diff mode ignores untracked files: `git add -N`.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
