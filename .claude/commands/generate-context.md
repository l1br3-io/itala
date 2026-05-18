# /generate-context

Scaffold a new bounded-context crate in the Itala Cargo workspace.

## Usage

```
/generate-context <kebab-name> [--with-adapters] [--with-events]
```

- `<kebab-name>`: short name (e.g. `note-editor`, `graph`). The crate becomes `itala-<name>`.
- `--with-adapters`: include an `adapters/` submodule scaffold.
- `--with-events`: include event-bus integration boilerplate.

## What this command produces

When invoked, you (the agent) MUST:

1. Read `rules/rust/workspace.md`, `rules/rust/error-handling.md`, `rules/rust/api-design.md`, `rules/rust/testing.md`.
2. Confirm the crate doesn't already exist in `crates/`.
3. Generate the files below using the Write tool.
4. Append the new member path to the workspace `Cargo.toml` `[workspace] members` array.
5. Add a workspace-dependency entry: `itala-<name> = { path = "crates/itala-<name>", version = "0.1" }`.
6. Print a summary of created files and remind the user to run `cargo build` and `cargo nextest run`.

## File template

### `crates/itala-<name>/Cargo.toml`

```toml
[package]
name = "itala-<name>"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "<one-line description>"

[dependencies]
itala-kernel.workspace = true
tokio.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true

[dev-dependencies]
tempfile = "3"
proptest = "1"
insta = "1"

[lints]
workspace = true
```

### `crates/itala-<name>/src/lib.rs`

```rust
//! itala-<name>: <one-line description>
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use crate::error::{Error, Result};

mod error;

// Re-export public surface here. Keep everything else `pub(crate)`.
```

### `crates/itala-<name>/src/error.rs`

```rust
//! Error type for `itala-<name>`.

use thiserror::Error;

/// Errors produced by the `<name>` context.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Wraps an underlying I/O failure.
    #[error("io")]
    Io(#[from] std::io::Error),
}

/// Convenience alias.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts() {
        let e: Error = std::io::Error::other("boom").into();
        assert!(matches!(e, Error::Io(_)));
    }
}
```

### Optional: `crates/itala-<name>/src/adapters/mod.rs` (when `--with-adapters`)

```rust
//! Adapter implementations for ports defined by this context.

pub(crate) mod memory;

#[cfg(test)]
pub(crate) mod test_support;
```

### Optional: `crates/itala-<name>/src/events.rs` (when `--with-events`)

```rust
//! Events emitted by this context onto the kernel bus.

use itala_kernel::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum <PascalName>Event {
    // Variant per concrete event.
}

impl Event for <PascalName>Event {
    const TOPIC: &'static str = "<name>";
}
```

## Acceptance checks (you must verify before reporting success)

- The new `Cargo.toml` builds: `cargo check -p itala-<name>` succeeds.
- `cargo clippy -p itala-<name> -- -D warnings` passes with no warnings.
- `cargo nextest run -p itala-<name>` passes (the placeholder error-conversion test should run).
- `cargo fmt --check -p itala-<name>` passes.
- The crate root has `#![forbid(unsafe_code)]`.
- `Error` enum is `#[non_exhaustive]` and uses `thiserror`.
- Workspace `Cargo.toml` lists the new member.

## Anti-patterns to avoid

- Do not add deps speculatively — only what's used by the scaffolding.
- Do not pre-create empty modules. Add modules when they have content.
- Do not write speculative builders or trait hierarchies. Start with concrete types.
- Do not import `anyhow` into a library crate.

When done, summarize the created files and the workspace `Cargo.toml` change in a short list. Do not start writing the context's domain logic — that is a follow-up task driven by `/capability` and `/plan`.
