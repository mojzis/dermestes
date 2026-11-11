# Read a dermestes check

What each check counts, how to read its evidence, and what it stays silent on.

- `one-impl`: an ABC or Protocol with one production implementation, named on
  the `impl:` line (for a Protocol, maybe a structural one).
- `const-param`, one line per parameter:
  - `default X never overridden`, `always X`, or `passed explicitly at N
    sites` when callers write the default out;
  - `*args`/`**kwargs: never populated`: no call adds a surplus argument;
  - `dead: lines A-B`: an `if`/`elif`/`else`/`while` arm that cannot run while
    the parameter has that value; `conditional at line N` is an arm of
    `a if p else b`. Only tests that are exactly `p`, `not p`, `p is None`,
    `p == literal` or `p in (literals)` are read, and only if `p` is never rebound.
- `pass-through`: `forward` only calls a repo function, binding at most short
  literals or one constant; `rename` renames an outside call for 2 callers;
  `single-use` is a private one-liner with one call.
- `dead-code`: no production code mentions the name; `tests:` lists the test
  lines that do. A class is reported once, without its methods. `(family of
  N)` is a method and its overrides, listed on `defs:`, that nothing calls.
- `unused-result`: every call is a bare statement, so the value on the listed
  `return` lines is never read.
- `needless-async`: the body awaits nothing and every call is `await f(...)`.

**Known misses** (it stays silent, by design):

- Calls count only when resolved (`f()`, `mod.f()`, `self.m()`, `Class()`).
  One on a variable (`tr.done()`) silences `const-param` when it could bind
  another value, and always silences a variadic, `unused-result` and
  `needless-async`: it may use what the resolved calls do not.
- All but `one-impl` are silent for functions used as values, decorated ones,
  overrides, overridden methods, methods of a class with a third-party base.
- `one-impl`: a test implementation, an unresolvable base or subclass, or a
  plain base class (no `ABC`, no abstract methods) with one subclass.
- `const-param`: a variable or enum member is never "the same value". A
  variadic is silent when named `_x`, and when its pair is populated
  (`*args` used, `**kwargs` not: they travel together). Tests with `and`/`or`
  give no `dead:` lines.
- `pass-through`: any test caller vetoes it, as do a third-party target for a
  forward, a public wrapper of a private one, `*args`, a multi-statement body.
- `dead-code`: callers that are themselves dead, a name only in prose or
  `__all__`, one shared with another method, public names in a `py.typed`
  package. Classes: any base outside the repo (exceptions, enums, Pydantic
  and ORM models), a metaclass, a decorator other than `dataclass`,
  `runtime_checkable`, `final` or `total_ordering`, an `__init_subclass__`
  ancestor, any `__subclasses__()` in the repo, a module nothing imports.
- `unused-result`: a `return` of a call or expression (`return log.debug(x)`
  passes on whatever comes back), fluent `return self`, generators, public
  names in a `py.typed` package, entry points.
- `needless-async`: a call inside `gather(...)` or `create_task(...)`, async
  generators, public names in a `py.typed` package.

next: run `dermestes guide tune`
