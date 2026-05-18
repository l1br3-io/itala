# Rust Error Handling

## Library crates: `thiserror`

Every library crate has a single public `Error` enum in `src/error.rs`.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("note {id} not found")]
    NotFound { id: NoteId },

    #[error("vault path is not absolute: {path}")]
    VaultPathRelative { path: PathBuf },

    #[error("io")]
    Io(#[from] std::io::Error),

    #[error("yrs: {0}")]
    Yrs(#[from] yrs::error::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
```

Rules:
- `#[non_exhaustive]` on every public error enum — adding a variant must not be a breaking change.
- One variant per failure mode; do not collapse unrelated failures into a single `Other(String)`.
- `#[from]` for sources that are already error types and should propagate transparently.
- Variants carry **structured context**, not formatted strings. The display impl can format; the variant data stays queryable.
- Error messages: lowercase, no trailing period, no leading "error:" — let the formatter add framing.

## Binary crates: `anyhow`

Top-level binaries (`itala-cli`, Tauri shell) use `anyhow::Result` only at the entry point.

```rust
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await.context("itala-cli")
}
```

Rules:
- `anyhow` lives at the binary edge only — never in library APIs.
- Use `.context("…")` and `.with_context(|| …)` to add a layer at every meaningful boundary.

## Forbidden patterns

- `unwrap()` outside tests — replace with `?` or `expect("invariant: …")`.
- `expect("")` with empty or vague messages — the message states the invariant being relied on.
- `.ok()` to silently drop an error — log it via `tracing::warn!` or propagate it.
- `panic!` for recoverable failures — only for genuine programmer bugs.
- Catching at every layer — bubble up via `?`; only catch where you can do something useful.

## Conversion between layers

Cross-crate errors convert via `From`/`#[from]`. Avoid stringifying except at the binary edge:

```rust
// CORRECT
fn load(id: NoteId) -> kernel::Result<Note> {
    let bytes = vfs.read(&id.path())?;   // io::Error → kernel::Error::Io via #[from]
    Note::parse(&bytes).map_err(Error::from)
}

// WRONG — loses structure
fn load(id: NoteId) -> kernel::Result<Note> {
    let bytes = vfs.read(&id.path()).map_err(|e| Error::Other(e.to_string()))?;
    // …
}
```

## Logging vs. returning

- Public APIs **return** errors. Logging is the caller's choice.
- Internal background tasks (`tokio::spawn`-ed loops) log errors and continue, since there is no caller to receive them — but log at `error!` with structured fields.
