# Rust Traits & Dispatch

> How Itala chooses between generics and trait objects, designs dyn-compatible traits, and uses the type system to enforce protocol state. Read alongside `rules/rust/api-design.md` (general API discipline) and `rules/rust/wasm-plugins.md` (WIT-bound traits).

## Static vs. dynamic dispatch — the Itala rule

| You're on… | Dispatch | Why |
|---|---|---|
| Per-keystroke render, parse, search hot path | **Generic** (`fn render<R: Renderer>(r: &R, …)`) | Inlining matters. Code bloat is acceptable; we have ≤4 implementors. |
| Plugin call boundary, runtime-selected adapter, `Vec<…>` of mixed types | **`dyn Trait`** (`Vec<Box<dyn Plugin>>`, `Arc<dyn Transport>`) | One type per slot is impossible; vtable cost is rounding error vs. wasmtime / network. |
| One concrete impl, just abstracting "for testability" | **No trait at all** | Premature trait. Use the concrete type; introduce the trait when there are two implementors. |
| Async trait with stable layout in a public API | `#[async_trait]` + `dyn` *or* native `async fn` + generics | See `rules/rust/async.md` — default to native, reach for `async-trait` only when boxing is required. |

Reach for generics when the caller already knows the type. Reach for trait objects when the *system* selects the type at runtime (config, plugin manifest, user choice).

## Dyn compatibility checklist

> Formerly called "object safety." Rustc diagnostics and the Reference adopted "dyn compatibility" in late 2024 (lang-team RFC #286); the older term still appears in older docs and search results.

A trait is **dyn-compatible** (i.e. usable as `dyn Trait`) iff every method:

- Has a receiver (`&self`, `&mut self`, `self: Box<Self>`, etc.) — or is exempted by `where Self: Sized`.
- Does **not** use generic type parameters.
- Does **not** return `Self` by value (returning `Box<Self>` is fine).
- Does **not** take `Self` by value (except via the receiver).

If a method violates these, exempt it instead of dropping dyn compatibility:

```rust
pub trait Plugin: Send + Sync {
    fn id(&self) -> &PluginId;
    fn handle(&self, evt: &Event) -> Result<()>;

    // Generic helper — exempt from dyn dispatch, still callable on concrete impls.
    fn typed_call<T: serde::Serialize>(&self, payload: T) -> Result<()>
    where
        Self: Sized,
    {
        self.handle(&Event::Json(serde_json::to_value(payload)?))
    }
}
```

`Vec<Box<dyn Plugin>>` works; `typed_call` stays available on concrete `MermaidPlugin`.

## Generic type parameters vs. associated types

| Shape | Use | Examples |
|---|---|---|
| Exactly one impl per type | **Associated type** | `Iterator::Item`, `Vault::Backend` |
| Many impls per type, parameterized by relationship | **Generic param** | `From<T>`, `TryFrom<T>`, `AsRef<T>` |

Decision rule: ask "can a type sensibly implement this trait twice with different parameters?" If yes, generic. If no (one canonical answer), associated type — it's stricter and reads cleaner at call sites.

```rust
// Associated type — a vault has exactly one backend at a time.
pub trait VaultBackend {
    type Handle;
    fn open(&self, id: VaultId) -> Result<Self::Handle>;
}

// Generic — a NoteId can convert from many things.
impl From<Ulid>   for NoteId { /* … */ }
impl From<&str>   for NoteId { /* … */ }   // would also try `FromStr`
```

## Marker traits and auto-trait assertions

`Send`, `Sync`, `Unpin`, `Sized` are **marker traits** (no methods, just promises). Their absence on a public type is a **breaking change** for downstream code that requires them. Lock that in with compile-time assertions in your test module:

```rust
#[cfg(test)]
mod assertions {
    use super::*;
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn note_is_send_sync_unpin() {
        assert_send_sync::<Note>();
        assert_unpin::<Note>();
    }
}
```

If a future change accidentally introduces `Rc<…>` inside `Note` (which would drop `Send`), the test fails before the API breaks for callers.

## Type-state pattern

Encode protocol state in the type, not in a runtime field. Itala uses this for two long-lived resources:

```rust
pub struct Vault<S>(VaultInner, PhantomData<S>);

pub struct Closed;
pub struct Open;

impl Vault<Closed> {
    pub async fn open(self, passphrase: SecretString) -> Result<Vault<Open>> { /* … */ }
}

impl Vault<Open> {
    pub async fn read(&self, id: NoteId) -> Result<Note>      { /* … */ }
    pub async fn write(&self, n: Note)   -> Result<()>        { /* … */ }
    pub async fn close(self)             -> Result<Vault<Closed>> { /* … */ }
}
```

`vault.read(id)` won't compile on a `Vault<Closed>`. No runtime check, no `Result<…, NotOpen>` variant.

Same shape applies to `SyncSession<Disconnected>` / `SyncSession<Connected>` — `send_update` only exists on the connected state.

## Sealed traits

When a trait is part of the public API but external impls would break invariants, seal it. Itala uses this for `Renderer` (the kernel chooses which renderers exist) and for `Transport` (each transport coordinates with sync internals).

```rust
pub trait Renderer: sealed::Sealed + Send + Sync {
    fn render(&self, ast: &Ast) -> Html;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::MarkdownRenderer {}
    impl Sealed for super::HtmlPassthrough {}
}
```

Downstream crates can call `Renderer` methods, but cannot `impl Renderer for MyType` — the unreachable `sealed::Sealed` supertrait blocks it. New variants happen by us editing the sealed module; that's the whole point.

This is also the canonical reference for the sealed-trait pattern; `rules/rust/api-design.md` links here.

## Coherence & the orphan rule (one paragraph)

You can `impl Trait for Type` only if either `Trait` or `Type` is local to your crate. This prevents two crates from both writing `impl Display for SomeForeignType` and the compiler having to pick. Workaround when you need to add behavior to a foreign type: wrap it in a newtype (`pub struct NoteBytes(bytes::Bytes);`) and `impl` on the newtype. Itala does this for every domain type that wraps a primitive — see `rules/rust/api-design.md` § Newtype IDs.

## Common pitfalls

- **`impl Trait` in argument position is a generic in disguise.** `fn foo(x: impl AsRef<str>)` desugars to `fn foo<T: AsRef<str>>(x: T)`. Each call site monomorphizes. Fine for leaf utilities; avoid on a hot trait method that already has many impls.
- **Returning `impl Trait` hides the concrete type.** Good for keeping flexibility, but auto-traits (`Send`, `Sync`) are inferred from the body — adding a non-`Send` local accidentally breaks downstream `tokio::spawn` callers. The compile-time auto-trait test above catches this.
- **`dyn Trait` is `?Sized`.** It must be behind a pointer (`&dyn`, `Box<dyn>`, `Arc<dyn>`). You cannot put one in a struct field by value.
