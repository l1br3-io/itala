# Specification Notes

Key architectural notes for Itala. Authoritative source is `docs/`; this file is a quick-reference cache.

## Core Philosophy

**Local-first, plugin-first, user-owned.**
- All data lives in plain Markdown + sidecar files in a folder the user picks.
- Every capability (storage, editor, search, sync, AI, export) is a plugin.
- Sync, cloud AI, telemetry: opt-in, off by default.
- See `docs/VISION.md`.

## Stack (chosen 2026-05-04)

**Kernel + bounded contexts (Rust)**
- `tokio` async runtime, `serde`, `thiserror`, `tracing`.
- `ulid` for IDs, `blake3` for content checksums.
- `redb` for local metadata, `opendal` for VFS adapters.
- `tantivy` for full-text search.
- `yrs` (Y-CRDT Rust port) for note bodies.
- `comrak` for Markdown parsing.

**Plugin host (Rust)**
- `wasmtime` ≥ 25 with the component model.
- `wit-bindgen` for binding generation.
- WIT contracts in `crates/itala-plugin-api/wit/`.
- Capability-token enforcement; ed25519 plugin signatures.

**Shells**
- Desktop + mobile: Tauri 2 (Rust + webview).
- Web: WASM target (`wasm32-wasip2`) with OPFS storage.
- CLI: `clap` derive.

**Webview UI (TypeScript, thin)**
- Vite + TypeScript strict mode.
- SolidJS components (Svelte 5 is the alternate).
- CodeMirror 6 editor with `y-codemirror.next` bridging to yrs.
- Tailwind 4 over CSS custom properties.
- Vitest + Playwright.

**AI (`crates/itala-ai`)**
- Provider trait with adapters for `async-openai`, `anthropic-sdk-rs`, `ollama-rs`, `candle`/`ort` (local embeddings).
- Fallback chains, redaction layer for cloud calls, opt-in per action category.

**Sync (`crates/itala-sync`)**
- Yrs CRDT updates over OpenDAL-backed transports (S3, R2, GCS, WebDAV).
- E2E encryption: XChaCha20-Poly1305, key from passphrase via Argon2.
- Selective sync by folder/tag.

## Performance budgets (`docs/ARCHITECTURE.md` §4.1)

- Cold start desktop: < 500 ms
- Cold start mobile: < 1 s
- Open 10 KB note: < 50 ms
- Search 10k notes: < 200 ms
- Keystroke → screen: < 16 ms
- Kernel memory: < 20 MB
- Each plugin: declared budget per manifest

## API design pattern

The "API" in Itala is the **plugin API**, expressed as WIT interfaces. Itala has no FastAPI-style HTTP backend.

For collections that need an in-memory contract, use the repository shape from `rules/api-design.md`. Inside Rust, prefer concrete types and trait-based ports over dynamic envelopes.

## Security highlights

- `#![forbid(unsafe_code)]` at every Rust crate root unless an FFI shell justifies otherwise.
- Plugins run in wasmtime with no ambient host access; capabilities are explicit.
- Secrets via `keyring-rs`; never plain config or env vars for production.
- TLS via `rustls` (no `openssl-sys`); at-rest encryption via `chacha20poly1305`.
- See `rules/rust/security.md` and `docs/ARCHITECTURE.md` §5.

## What Itala is NOT

- Not a SaaS. There is no managed backend. Sync is bring-your-own.
- Not a wrapper around an existing app. The kernel is original.
- Not browser-extension-based. The web target uses OPFS, not extension storage.
- Not a notes app for non-technical users yet — MVP targets power users; UX polish for general audience is post-MVP.
