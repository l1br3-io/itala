# Rust Workspace Layout

> Idiomatic Rust workspace structure for Itala. All Rust-touching agents should load this first.

## Top-level layout

```
itala/
├── Cargo.toml                # [workspace] only — no [package]
├── rust-toolchain.toml       # Pinned MSRV + components
├── deny.toml                 # cargo-deny config
├── crates/
│   ├── itala-kernel/         # Shared types, event bus, VFS trait, capability system
│   ├── itala-plugin-api/     # Stable plugin trait + WIT — semver-frozen
│   ├── itala-plugin-host/    # wasmtime runtime, capability enforcement
│   ├── itala-vault/          # Vault open/close, frontmatter, version history
│   ├── itala-note-editor/    # CRDT body, parse, render contracts
│   ├── itala-graph/          # Backlinks, unlinked mentions, link resolution
│   ├── itala-search/         # Tantivy index, query, incremental updates
│   ├── itala-sync/           # Yrs-over-transport, conflict reconciliation
│   ├── itala-ai/             # Provider trait + adapters
│   ├── itala-publish/        # Export to PDF/HTML/DOCX
│   └── itala-cli/            # CLI binary
├── apps/
│   ├── desktop/              # Tauri 2 desktop shell + webview UI
│   ├── mobile/               # Tauri 2 mobile shell (Android/iOS)
│   └── web/                  # WASM web shell
└── plugins/                  # First-party WASM plugins (built separately)
```

## Crate naming

- Crate names: kebab-case, prefix `itala-` (e.g. `itala-kernel`)
- The CLI binary is **`itala`** (single word) but lives in `crates/itala-cli`; set the binary name via `[[bin]] name = "itala"`
- Module names: snake_case
- Type names: PascalCase
- Trait names: PascalCase, no `I` prefix; verb-form preferred for behaviors (`Render`, not `Renderer`, unless the trait *is* the noun)

## Module organization

Use the **2018+ edition idiom**: `foo.rs` + sibling `foo/` directory. Never `foo/mod.rs`.

```
src/
├── lib.rs
├── error.rs                  # crate-local Error enum
├── note.rs                   # entry point for the `note` module
└── note/                     # submodules of `note`
    ├── id.rs
    └── frontmatter.rs
```

`lib.rs` declares the public surface. Re-export only what consumers need:

```rust
//! itala-note-editor: CRDT-backed Markdown editing.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use crate::error::Error;
pub use crate::note::{Note, NoteId};

mod error;
mod note;
mod parse;        // pub(crate) — internal
```

Default visibility is `pub(crate)`. Promote to `pub` only when a consumer outside the crate has a concrete need.

## Dependency hygiene

- Define shared deps once in workspace `[workspace.dependencies]`; member crates use `tokio.workspace = true`
- Pin major versions; let cargo resolve patch
- No `path = "../foo"` deps without a workspace member entry — the workspace resolver handles it
- Run `cargo machete` weekly to flag unused deps
- Optional features only when there's a concrete consumer of the off-state — no speculative feature flags

## Stable vs. internal crates

- `itala-plugin-api` is the only crate plugins depend on. It is **semver-frozen** — breaking changes go through a deprecation cycle.
- `itala-kernel` is internal to the host; it can break on minor versions during pre-1.0.
- All other crates are internal. Plugins must not depend on them directly.

## What goes where

| Concern | Crate |
|---|---|
| ID types, error enum scaffolding, event bus | `itala-kernel` |
| VFS trait, capability tokens | `itala-kernel` |
| Plugin trait, WIT interfaces | `itala-plugin-api` |
| wasmtime runtime + sandbox | `itala-plugin-host` |
| Anything specific to a bounded context | that context's crate |
| Concrete adapter (filesystem, S3, OpenAI) | `itala-{context}/src/adapters/` or its own crate if reused |

## Bounded-context consumption

Downstream context crates (`itala-search`, `itala-graph`, `itala-sync`, `itala-publish`, `itala-ai`) consume notes as `&Note`, `&[Note]`, or `NoteId` — never `&Vault` and never `&Utf8Path`. The vault crate hands out semantic data; consumers do not reach into storage.

FS-shaped accessors on `Vault` and `Note` are explicitly `Option`-returning and intended for FS-aware shells (CLI, desktop) only:
- `Vault::fs_location_of(id) -> Result<Option<Utf8PathBuf>>`
- `Note::fs_location() -> Option<&Utf8Path>`

*Why:* keeps a future second note backend (encrypted blob, remote, database) additive rather than breaking. If a downstream crate stores `Utf8Path` today, adding that backend forces changes everywhere; if it stores `NoteId`, nothing changes.
