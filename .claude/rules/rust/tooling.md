# Rust Tooling

## Toolchain

`rust-toolchain.toml` at the repo root:

```toml
[toolchain]
channel = "1.93.0"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
targets = ["wasm32-wasip2", "x86_64-unknown-linux-gnu"]
```

Pin the channel. Bump deliberately, with a CI run, and update this file in the same PR.

## Formatting

`rustfmt.toml`:

```toml
edition = "2024"
use_field_init_shorthand = true
use_try_shorthand = true
reorder_imports = true
```

CI runs `cargo fmt --all -- --check`. Local pre-commit runs `cargo fmt`. Never check in unformatted code.

## Linting

`Cargo.toml` workspace lints:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"        # override per-crate where needed
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
must_use_candidate = "warn"
```

Each crate inherits via `lints.workspace = true`. CI runs:

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

`-D warnings` makes any warning a build break. Override locally with `#[allow(...)]` and a comment explaining why.

## Tests

```
cargo nextest run --workspace
cargo test --doc --workspace
```

Nextest config in `.config/nextest.toml` per `rules/rust/testing.md`.

## Supply chain

```
cargo deny check          # licenses + bans + advisories (consumes RustSec DB)
cargo machete             # unused deps
cargo hack --feature-powerset check  # feature-combination build check
```

`deny.toml` lives at the repo root.

`cargo audit` is **not** run separately — `cargo-deny`'s `advisories` check consumes the same RustSec advisory database. Running both is redundant ([cargo-deny advisories docs](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html)).

`cargo-hack` catches feature-flag combinations that compile individually but break together. Run on every PR for any crate with optional features.

## API stability

`itala-plugin-api` is semver-frozen (see `rules/rust/workspace.md`). CI enforces this:

```
cargo semver-checks --package itala-plugin-api
```

`cargo-semver-checks` is the de facto standard for catching breaking changes in public APIs ([Rust Project Goal 2026](https://rust-lang.github.io/rust-project-goals/2026/cargo-semver-checks.html)). Failing this check blocks the merge.

Other plugin-facing crates declare `rust-version = "..."` in their `Cargo.toml` so downstream plugin authors get a clear MSRV signal; the workspace toolchain pin alone is not visible to consumers.

## Documentation

Public-API crates set `#![deny(missing_docs)]`. CI runs:

```
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

Broken intra-doc links and missing summaries break the build.

## Local commands

The repo ships a `justfile` (or `make` if preferred) with the canonical shortcuts:

```just
default: lint test

ci: lint test deny doc

fmt:
    cargo fmt --all

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo nextest run --workspace
    cargo test --doc --workspace

deny:
    cargo deny check
    cargo hack --feature-powerset check
    cargo semver-checks --package itala-plugin-api

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --open
```

`just` is the recommended driver — agents invoke `just lint`, `just test`, `just deny` rather than memorizing flags.

## CI

Every PR runs in this order:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo nextest run`
4. `cargo test --doc` (nextest does not run doctests as of v0.9.x — keep this step separate)
5. `cargo deny check`
6. `cargo hack --feature-powerset check`
7. `cargo semver-checks --package itala-plugin-api`
8. `cargo doc` (broken-link check)

Mutation testing (`cargo mutants`) runs on a nightly cron against `itala-kernel` and `itala-plugin-host`.

## Cross-compilation

Mobile and WASM targets:
- `wasm32-wasip2` — plugin runtime target.
- `aarch64-apple-darwin`, `x86_64-apple-darwin` — desktop macOS.
- `aarch64-linux-android`, `aarch64-apple-ios` — mobile (Tauri 2 mobile build chain).

Use `cargo-zigbuild` for reliable cross-builds in CI; `cross` is a fallback for niche targets.

## Editor integration

- `rust-analyzer` is the assumed LSP. Workspace settings in `.vscode/settings.json` enable clippy on save and check on save.
- `cargo expand` for macro debugging when generated code is suspect.
