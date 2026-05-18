# Rust API Design

> Extends `rules/api-design.md` (generic) with Rust-specific guidance.

## Newtype IDs

Never pass raw strings or integers as identity. Wrap them.

```rust
use ulid::Ulid;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NoteId(Ulid);

impl NoteId {
    #[must_use]
    pub fn new() -> Self { Self(Ulid::new()) }

    #[must_use]
    pub fn as_ulid(&self) -> Ulid { self.0 }
}

impl fmt::Display for NoteId { /* … */ }
impl FromStr   for NoteId    { /* … */ }
```

ID types: `NoteId(Ulid)`, `VaultId(Ulid)`, `LinkId(Ulid)`, `PluginId(SmolStr)`, `BlockId(SmolStr)`. Plugins are user-named so they get strings; everything else is a ULID for sortability and uniqueness.

## Function arguments

| Use this | Not that | Why |
|---|---|---|
| `&str` | `&String` | Caller flexibility |
| `&[T]` | `&Vec<T>` | Same |
| `impl AsRef<Path>` or `&Utf8Path` | `&PathBuf` | Same |
| `impl Into<Bytes>` | `Vec<u8>` | Lets callers pass shared buffers |
| `impl IntoIterator<Item = T>` | `Vec<T>` | Streaming or one-shot, caller's choice |

Prefer `Utf8Path`/`Utf8PathBuf` (from `camino`) over `Path`/`PathBuf` for vault paths — Itala paths are always UTF-8 and the type system should enforce that.

## Builders

For >3 args (or any optional args), use a builder. Prefer **`bon`** (compile-time, zero runtime overhead, supports defaults and validation) over `derive_builder`.

```rust
use bon::Builder;

#[derive(Builder)]
pub struct OpenVault {
    pub root: Utf8PathBuf,
    #[builder(default)]
    pub read_only: bool,
    pub passphrase: Option<SecretString>,
}

let vault = OpenVault::builder()
    .root(path)
    .passphrase(passphrase)
    .build();
```

## Returns

- Owned value when small or always-needed: `String`, `Vec<T>`.
- `&T` borrowed view when caller can hold the lifetime.
- `Cow<'_, str>` when the result is sometimes borrowed, sometimes computed.
- `impl Trait` for monomorphized iterators or futures where the caller doesn't need a name.
- `Box<dyn Trait>` only across plugin boundaries or where the concrete type must be erased.

`#[must_use]` on:
- All `Result<_, _>` wrappers and bare `Result` returns whose ignoring would lose an error.
- Any builder's terminal `.build()` method.
- Anything whose entire purpose is the return value (`fn checksum(&self) -> Hash`).

## Sealed traits

Crate-internal extension points use the sealed-trait pattern so external crates can't `impl` them and break invariants:

```rust
pub trait Renderer: sealed::Sealed {
    fn render(&self, ast: &Ast) -> Html;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::MarkdownRenderer {}
    impl Sealed for super::HtmlPassthrough {}
}
```

## Generics vs. trait objects

- Hot paths (per-keystroke, per-token): generics + monomorphization.
- Plugin boundaries, runtime selection, `Vec<Box<dyn Plugin>>`: trait objects.
- Don't reach for generics when one type is enough — a concrete type is the simplest API.

## Conversions

- `impl From<X> for Y` where the conversion is total and lossless.
- `impl TryFrom<X> for Y` when the conversion can fail — pair with a real error type.
- Never `impl From` where the conversion can panic.

## Documentation

Public items need a doc comment. The first line is a one-sentence summary; expanded sections follow as needed:

```rust
/// Read a note's body as parsed Markdown.
///
/// The body is decoded from the CRDT, then parsed with the active
/// `comrak` extensions. Errors are surfaced if the CRDT is missing
/// or the bytes are not valid UTF-8.
///
/// # Errors
/// Returns [`Error::NotFound`] if `id` is not present.
pub async fn read_body(&self, id: NoteId) -> Result<Markdown> { /* … */ }
```

Public-API crates (`itala-kernel`, `itala-plugin-api`) deny missing docs in CI.
