# Rust Testing

## Layout

```
src/
├── lib.rs
├── note.rs              # impl + #[cfg(test)] mod tests at the bottom
└── note/
    └── parse.rs         # impl + colocated tests
tests/
└── vault_lifecycle.rs   # integration tests (each file = its own crate)
```

Unit tests are colocated with the code they test. Integration tests live in `tests/` and only see the public API.

```rust
// note/parse.rs
pub(crate) fn parse(bytes: &[u8]) -> Result<Ast> { /* … */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yaml_frontmatter() {
        let input = b"---\ntitle: hi\n---\nbody";
        let ast = parse(input).unwrap();
        assert_eq!(ast.frontmatter.get("title").unwrap(), "hi");
    }
}
```

Test functions: `snake_case`, descriptive (`parses_yaml_frontmatter` not `test1`). No `test_` prefix — `#[test]` already says so.

## Runner

`cargo nextest run` is the test runner. Configure in `.config/nextest.toml`:

```toml
[profile.default]
slow-timeout = { period = "30s", terminate-after = 2 }
fail-fast = false
```

Use `cargo test --doc` separately for doctests — nextest does not run them.

## Property tests: `proptest`

Anything with algebraic invariants gets property tests:
- CRDT operations (idempotence, commutativity for compatible ops)
- Link resolution (round-trip parse/render)
- Search ranking (monotonicity)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_render_roundtrip(text in any::<String>()) {
        let ast = parse(text.as_bytes()).unwrap();
        let rendered = render(&ast);
        let reparsed = parse(rendered.as_bytes()).unwrap();
        prop_assert_eq!(ast, reparsed);
    }
}
```

Pin `proptest` cases when they reveal a bug — keep the regression in tree.

## Snapshot tests: `insta`

Use `insta` for anything whose output is structured but tedious to assert by hand: parsed Markdown ASTs, error messages, serialized index entries.

```rust
#[test]
fn renders_wiki_link() {
    let html = render_link("[[Note Title]]");
    insta::assert_snapshot!(html);
}
```

Review snapshots with `cargo insta review`.

## Mutation testing: `cargo-mutants`

Run on `itala-kernel` and `itala-plugin-host` only — too slow for the whole workspace. Add to CI on a nightly cron, not per-PR. Mutation score below 70% on these crates blocks the next release.

## Doctests

Every public function with a non-trivial signature has a runnable example:

```rust
/// Parse a note's frontmatter into a `BTreeMap`.
///
/// ```
/// use itala_note_editor::parse_frontmatter;
/// let map = parse_frontmatter(b"---\ntitle: hi\n---").unwrap();
/// assert_eq!(map.get("title").unwrap(), "hi");
/// ```
pub fn parse_frontmatter(input: &[u8]) -> Result<BTreeMap<String, String>> { /* … */ }
```

`cargo test --doc` runs these. They double as docs that can never go stale silently.

## Async tests

```rust
#[tokio::test]
async fn opens_empty_vault() { /* … */ }

// For multi-thread runtime:
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_indexing() { /* … */ }
```

## Test fixtures

- Temp dirs via `tempfile::TempDir` — never `std::env::temp_dir()` directly.
- Test vaults via a fixture builder in `crates/itala-vault/src/test_support.rs`, gated `#[cfg(any(test, feature = "test-support"))]`.
- Network: never. Use mocks (`wiremock`) or fakes.
- Time: `tokio::time::pause()` + `advance()` for time-dependent code. Never `std::thread::sleep` in tests.

## Coverage

`cargo llvm-cov` reports coverage. Don't chase 100% — hot paths and parsing must be high; trivial getters can be uncovered.

## What not to test

- Trait derives (`Debug`, `Clone`, `Eq`) — the compiler tested them.
- Third-party libraries — not your job.
- Single-line wrappers that just forward arguments.
