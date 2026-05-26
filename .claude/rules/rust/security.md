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
- Advisory check against the RustSec DB.

CI runs `cargo deny check` on every PR. `cargo audit` is **not** run separately — `cargo-deny`'s `advisories` check consumes the same RustSec advisory database, so running both is redundant. See `rules/rust/tooling.md` § Supply chain.

## Secrets

- Use `secrecy::SecretString` for any in-memory **string** secret. It zeroes on drop and prints `[REDACTED]`. `secrecy` uses `zeroize` underneath.
- For non-string secret material (raw keys, derived bytes, IVs), use `zeroize::Zeroizing<Vec<u8>>` or a struct that derives `ZeroizeOnDrop`. Don't reach for `SecretString` for bytes.
- Compare secret bytes (MACs, tokens, fingerprints) with **`subtle::ConstantTimeEq`**. Byte-by-byte `==` leaks timing and lets attackers learn prefixes one byte at a time.
- Persistent secrets via `keyring-rs` (OS keychain). Never plain config files for production secrets.
- Test fixtures may use plain strings, gated `#[cfg(test)]`.

```rust
use secrecy::{ExposeSecret, SecretString};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub async fn open_vault(passphrase: SecretString) -> Result<Vault> {
    let key: Zeroizing<[u8; 32]> = derive_key(passphrase.expose_secret())?;
    // …
}

fn tokens_match(provided: &[u8], expected: &[u8]) -> bool {
    provided.ct_eq(expected).into()   // constant-time
}
```

## Cryptography

- TLS: `rustls` only. No `openssl-sys` in our tree.
- Symmetric: `chacha20poly1305` (XChaCha20-Poly1305) for vault-at-rest and sync payload encryption.
- Hashing: `blake3` for content addressing (note checksums); `sha2` only when interop demands it.
- Random: `rand::rngs::OsRng` for crypto material; `rand::thread_rng` is fine for non-crypto.
- Key derivation: `argon2` (Argon2id variant). Pin parameters from the OWASP Password Storage Cheat Sheet — current acceptable sets (any one is fine; pick the one that fits the device's memory budget):
  - `m=47104` (46 MiB), `t=1`, `p=1`  ← default for desktop/server
  - `m=19456` (19 MiB), `t=2`, `p=1`
  - `m=12288` (12 MiB), `t=3`, `p=1`
  - `m=9216`  (9 MiB),  `t=4`, `p=1`
  - `m=7168`  (7 MiB),  `t=5`, `p=1`  ← mobile/low-RAM
  - Output length: 32 bytes. Salt: 16 random bytes from `OsRng`, stored alongside.
  - Source: [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html).

Never roll your own crypto. Never use `rand::random` for keys.

## Plugin sandboxing

- Plugins run as wasmtime components with **no** ambient host access.
- The host grants explicit capability tokens per declared capability in `plugin.toml`.
- Capability tokens are checked at every host call boundary in `itala-plugin-host`.
- Plugin manifest signatures verified via `ed25519-dalek` (≥ 2.0; the 1.x line was subject to [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093.html), a public-key-oracle vector on `ExpandedSecretKey::sign` that 2.0 closed by removing the unsafe API) against the hub's known-good key list.

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
