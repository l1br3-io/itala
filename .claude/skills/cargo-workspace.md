# Cargo Workspace

Single-page reference for working with the Itala Cargo workspace.

## Workspace `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/itala-kernel",
    "crates/itala-plugin-api",
    "crates/itala-plugin-host",
    "crates/itala-vault",
    "crates/itala-note-editor",
    "crates/itala-graph",
    "crates/itala-search",
    "crates/itala-sync",
    "crates/itala-ai",
    "crates/itala-publish",
    "crates/itala-cli",
    "apps/desktop",
    "apps/mobile",
    "apps/web",
]

[workspace.package]
edition = "2024"
rust-version = "1.84"
license = "MIT OR Apache-2.0"
repository = "https://github.com/jpt/itala"

[workspace.dependencies]
# Core
tokio       = { version = "1", features = ["full"] }
tokio-util  = { version = "0.7", features = ["rt"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
thiserror   = "2"
anyhow      = "1"
tracing     = "0.1"
ulid        = { version = "1", features = ["serde"] }
blake3      = "1"
camino      = { version = "1", features = ["serde1"] }
secrecy     = "0.10"

# Storage / search / sync
redb        = "2"
opendal     = "0.50"
tantivy     = "0.22"
yrs         = "0.21"

# Plugin host
wasmtime    = { version = "25", features = ["component-model"] }
wit-bindgen = "0.34"

# Markdown / editor
comrak      = { version = "0.29", default-features = false }

# Internal crates
itala-kernel       = { path = "crates/itala-kernel",       version = "0.1" }
itala-plugin-api   = { path = "crates/itala-plugin-api",   version = "0.1" }
itala-plugin-host  = { path = "crates/itala-plugin-host",  version = "0.1" }

[workspace.lints]
# See rules/rust/tooling.md
```

## Member crate `Cargo.toml`

Member crates inherit via `.workspace = true`:

```toml
[package]
name = "itala-vault"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
itala-kernel.workspace = true
tokio.workspace = true
serde.workspace = true
thiserror.workspace = true
camino.workspace = true

[dev-dependencies]
tempfile = "3"
proptest = "1"
insta = "1"

[lints]
workspace = true
```

Never duplicate version strings in member crates. The workspace is the single source of truth.

## Feature flags

- One feature per consumer-facing capability with a real off-state user (e.g. `feature = "encryption"` to disable XChaCha20 cost in tests).
- Never speculative features. If no consumer disables it, it isn't a feature.
- Internal feature unification: enable common features at the workspace `[workspace.dependencies]` level.

## Adding a new crate

1. `cargo new --lib crates/itala-foo`
2. Add `crates/itala-foo` to `[workspace] members`.
3. Replace the generated `Cargo.toml` body with the workspace-inheriting template above.
4. Add the crate to `[workspace.dependencies]` with `path = "crates/itala-foo"` and `version = "0.1"`.
5. Set crate-root attributes: `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`.
6. Add `mod error;` with the `thiserror` enum per `rules/rust/error-handling.md`.

## Common commands

```
cargo build --workspace
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo deny check
cargo machete
cargo doc --no-deps --workspace
```

## Workspace-wide refactors

- `cargo workspaces` (the cli) is helpful for bulk version bumps.
- For renames, prefer `cargo-edit` (`cargo set-version`, `cargo rm`, `cargo add`) over hand-editing.
