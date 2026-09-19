### `dermestes` — over-abstraction

- `dermestes` lists abstractions your change introduced that have a single
  user (an ABC with one implementation, a function with one caller).
- Each finding ends in a `suggest:` line. Apply it and run the tests, or keep
  it with `# dermestes: keep <reason>` in the definition's header.
