# Rust Security

> Extends `rules/security.md` (generic) with Rust-specific guidance.

## Unsafe policy

Default at every crate root:

```rust
#![forbid(unsafe_code)]
```

Crates that genuinely need `unsafe` (FFI, mobile JNI/Swift bridges, vectorized hot paths) replace the forbid with:

```rust
#![deny(unsafe_op_in_unsafe_fn)]
```

Every `unsafe` block carries a `// SAFETY:` comment stating the invariant being upheld:

```rust
// SAFETY: `ptr` is non-null and points to `len` initialized bytes — guaranteed
// by the contract of `wasmtime_alloc_buffer`, which is the only producer.
let bytes = unsafe { slice::from_raw_parts(ptr, len) };
```

PR review denies any new `unsafe` block without a SAFETY comment.

## Lints

Crate-root attributes:

```rust
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
)]
#![allow(
    clippy::module_name_repetitions,   // we name modules after their primary type
    clippy::missing_errors_doc,        // covered by Error enum docs
)]
```

CI runs:

```
cargo clippy --workspace --all-targets -- -D warnings
```

## Supply-chain

`deny.toml` enforces:
- License allowlist (`MIT`, `Apache-2.0`, `BSD-3-Clause`, `MPL-2.0`, `Zlib`).
- Banned crates (`openssl-sys` if `rustls` is reachable; `chrono` in favor of `time`).
- Advisory check via `cargo audit` integration.

CI runs `cargo deny check` and `cargo audit` on every PR.

## Secrets

- Use `secrecy::SecretString` for any in-memory secret. It zeroes on drop and prints `[REDACTED]`.
- Persistent secrets via `keyring-rs` (OS keychain). Never plain config files for production secrets.
- Test fixtures may use plain strings, gated `#[cfg(test)]`.

```rust
use secrecy::{ExposeSecret, SecretString};

pub async fn open_vault(passphrase: SecretString) -> Result<Vault> {
    let key = derive_key(passphrase.expose_secret())?;
    // …
}
```

## Cryptography

- TLS: `rustls` only. No `openssl-sys` in our tree.
- Symmetric: `chacha20poly1305` (XChaCha20-Poly1305) for vault-at-rest and sync payload encryption.
- Hashing: `blake3` for content addressing (note checksums); `sha2` only when interop demands it.
- Random: `rand::rngs::OsRng` for crypto material; `rand::thread_rng` is fine for non-crypto.
- Key derivation: `argon2` with parameters matching OWASP recommendations.

Never roll your own crypto. Never use `rand::random` for keys.

## Plugin sandboxing

- Plugins run as wasmtime components with **no** ambient host access.
- The host grants explicit capability tokens per declared capability in `plugin.toml`.
- Capability tokens are checked at every host call boundary in `itala-plugin-host`.
- Plugin manifest signatures verified via `ed25519-dalek` against the hub's known-good key list.

## Input validation

External inputs (file contents, network responses, plugin output, IPC commands from the webview) are validated at the boundary:
- File parsing: parser is fallible; never `unwrap()` on input.
- Network: validate via typed `serde` deserialization with `#[serde(deny_unknown_fields)]`.
- Plugin output: validated against the WIT type — wasmtime enforces the type but you still validate semantic invariants.
- Tauri IPC: each command takes typed args; `serde` rejects malformed JSON; range/length checks are explicit.

## Logging

- Never log secrets. `tracing::field::display(secret.expose_secret())` is wrong.
- Use structured fields, not concatenated strings: `info!(note_id = %id, "loaded")`.
- PII (note content, vault paths beyond the basename) does not leak to telemetry.

## Telemetry

Off by default. If enabled (opt-in), only counts and durations — never content.
