# /generate-adapter

Scaffold an adapter implementation against a kernel trait (port).

## Usage

```
/generate-adapter <context-crate> <port-trait> <adapter-name>
```

- `<context-crate>`: e.g. `itala-vault`, `itala-search`.
- `<port-trait>`: the trait that defines the port (e.g. `Vfs`, `SearchIndex`).
- `<adapter-name>`: kebab-case name for the concrete impl (e.g. `local-fs`, `s3`, `tantivy-fs`).

The adapter lives at `crates/<context-crate>/src/adapters/<adapter_name>.rs` (snake_case in source).

## What this command produces

When invoked, you (the agent) MUST:

1. Read `rules/rust/workspace.md`, `rules/rust/api-design.md`, `rules/rust/error-handling.md`, `rules/rust/testing.md`.
2. Read the port trait at `crates/<context-crate>/src/<area>.rs` to understand its signature.
3. Confirm `crates/<context-crate>/src/adapters/mod.rs` exists; if not, ask whether to create it.
4. Generate the adapter file plus a tests submodule using the Write tool.
5. Wire the new module into `adapters/mod.rs`.
6. Add any new dependency to the crate's `Cargo.toml` (one dep per adapter is typical).

## Adapter template

```rust
//! <Adapter-name> adapter for the <PortTrait> port.
//!
//! Implements [`<PortTrait>`] backed by <one-line description of backend>.

use crate::{<PortTrait>, Result};

/// Concrete adapter.
#[derive(Debug)]
pub struct <PascalAdapterName> {
    // Held resources (paths, clients, etc.). Keep fields private.
}

impl <PascalAdapterName> {
    /// Construct a new adapter.
    ///
    /// # Errors
    /// Returns [`crate::Error`] if initialization fails.
    pub fn new(/* args */) -> Result<Self> {
        Ok(Self { /* … */ })
    }
}

impl <PortTrait> for <PascalAdapterName> {
    // Implement each method. Push validation up to the trait's contract;
    // adapters do the bare minimum to satisfy the contract.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn satisfies_the_port_contract() {
        // Use the shared port-test harness from `crate::adapters::test_support`.
    }
}
```

## Wiring

Append to `crates/<context-crate>/src/adapters/mod.rs`:

```rust
pub(crate) mod <adapter_name>;

pub use <adapter_name>::<PascalAdapterName>;
```

Re-export at the crate root (`lib.rs`) only if the adapter is part of the public API.

## Shared port-contract tests

If `crates/<context-crate>/src/adapters/test_support.rs` exists, route the new adapter through it:

```rust
#[tokio::test]
async fn port_contract() {
    let adapter = <PascalAdapterName>::new(/* test fixture */)?;
    crate::adapters::test_support::run_port_contract(adapter).await;
}
```

If no shared harness exists yet, write one alongside the first concrete adapter — every later adapter gets it for free.

## Acceptance checks

- `cargo check -p <context-crate>` succeeds.
- `cargo clippy -p <context-crate> -- -D warnings` passes.
- `cargo nextest run -p <context-crate>` passes (including the new contract test).
- The adapter does **not** pull in deps not needed by its specific backend.
- Public API exposed at the crate root only when the adapter is intended for external use.

## Anti-patterns

- Do not duplicate validation that the port trait already enforces — keep adapters thin.
- Do not store unbounded caches inside an adapter. If caching is needed, that's a separate decorator type.
- Do not panic on configuration errors at construction time — return `Err`.
- Do not block in `async` methods. Wrap blocking I/O with `tokio::task::spawn_blocking`.
