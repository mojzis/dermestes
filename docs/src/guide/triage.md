# Triage a dermestes finding

Each finding is a deletion candidate: an abstraction with a single user.

```
one-impl src/pay/gateway.py:12 PaymentGateway (ABC, 3 abstract methods)
  impl: src/pay/stripe.py:8 StripeGateway
  tests: 0 impls
  suggest: inline into StripeGateway, delete PaymentGateway
```

The first line is the check id, the definition and what it is. The indented
lines are the evidence and the concrete simplification.

**For each finding:**

1. Read both ends of the evidence. Is the abstraction a deliberate seam the
   counts cannot see (a plugin point, a public API)?
2. If not, apply the `suggest:` line, then run the test suite.
3. If it is deliberate, suppress it on the definition line with a reason:
   `# dermestes: keep <reason>`. A marker without a reason is ignored.

`dermestes guide tune` lists the config keys that exempt whole paths.

next: run `dermestes`
