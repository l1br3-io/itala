# Rust Ownership & Lifetimes

> Itala-specific ownership and lifetime discipline. Read alongside `rules/rust/async.md` (lock-across-await) and `rules/rust/api-design.md` (newtype IDs, function arguments).

## Mental model

A **value** is typed data. A **place** is somewhere a value lives (a stack slot, a struct field, a heap cell). A **pointer** is a value whose meaning is the address of a place. Ownership is "this place is responsible for dropping this value." Borrowing is "I have a temporary view of that place." Lifetimes are the compiler's name for *how long* the borrow is valid — they are not scopes; they can have holes (a value can be moved out and a new one moved in).

If you find yourself wanting to "trick the borrow checker," stop. The fix is almost always:
- Move the value into a `Vec` and pass an index (the "indeference" pattern), or
- Restructure ownership (one owner, message-pass to others), or
- Use interior mutability (`Cell` / `RefCell` / `Mutex`) deliberately.

## Copy vs. Clone for our types

| Type | `Copy` | `Clone` | Why |
|---|---|---|---|
| `NoteId(Ulid)`, `VaultId(Ulid)`, `LinkId(Ulid)` | yes | yes (derived) | 16 bytes, no heap, comparable. Pass by value freely. |
| `BlockId(SmolStr)`, `PluginId(SmolStr)` | no | yes (cheap) | Inline ≤22 bytes, refcounted heap above. Clone is fine; `Copy` would mask the sometimes-allocation. |
| `bytes::Bytes` | no | yes (refcount bump) | Refcounted slice. Clone is O(1); never `to_vec()` to "make a copy." |
| `Note`, `Vault`, CRDT handles, plugin handles | no | no | Identity matters; clone would either be lossy or a hidden deep copy. Move them. |
| `SecretString` | no | no | Don't make secrets cloneable; force the caller to think about lifetimes. |

Rule: **derive `Copy` only for tiny POD-shaped types whose byte-for-byte copy is unambiguously the right semantic.** For everything else, `Clone` is opt-in and shows up in diffs.

## Drop order matters

Locals are dropped in **reverse declaration order**; struct fields are dropped in **declaration order**. This is load-bearing in three places in Itala:

```rust
async fn save_note(vault: &Vault, id: NoteId, body: Bytes) -> Result<()> {
    let _span = tracing::info_span!("save_note", note_id = %id).entered();
    let txn = vault.begin_txn().await?;     // dropped second  → commit/rollback runs *before* span exits
    let mut note = txn.note_mut(id).await?; // dropped first   → releases note borrow before txn drops
    note.replace_body(body)?;
    txn.commit().await
}
```

Other places where reverse-drop is the contract:
- **Plugin lifecycle:** `PluginInstance` field declared after `Store` so the instance drops first, then the wasmtime store.
- **`tokio::task::JoinSet`:** dropping cancels every spawned task. Don't store a `JoinSet` in a struct that must outlive its tasks; it's a footgun.

If you need explicit ordering, name the cleanup with a binding (`let _guard = …`) or use `ManuallyDrop` plus a manual `drop()` call. **Never** rely on the order of drops between sibling `let _ = …` lines that you didn't write together.

## Interior mutability — pick the cheapest tool that fits

| You have… | Use |
|---|---|
| `Copy` state, single-threaded, tiny | `Cell<T>` |
| Non-`Copy` state, single-threaded, runtime borrow-checked | `RefCell<T>` |
| Cross-thread state, **never held across `.await`** | `std::sync::Mutex<T>` |
| Cross-thread state, **held across `.await`** | `tokio::sync::Mutex<T>` (and only after considering message-passing) |
| Many readers actually contending with rare writes | `tokio::sync::RwLock<T>` |
| One writer, many observers of the latest value | `tokio::sync::watch` |
| Producer/consumer with back-pressure | `tokio::sync::mpsc` (bounded) |

Holding a `std::sync::Mutex` across `.await` is a deadlock waiting to happen — see `rules/rust/async.md`. The compiler does not catch this for you.

`Arc<T>` vs `Arc<RwLock<T>>` vs `Arc<Mutex<T>>`: start with `Arc<T>` (immutable shared state). Only wrap in a lock when you have an actual mutation. Choose `RwLock` only when readers actually contend; otherwise `Mutex` is simpler and faster under contention.

## Lifetimes & variance

A lifetime is a region of the program where a borrow is valid. It can have holes — moving a value invalidates the borrow until you reassign:

```rust
let mut note = vault.open(id).await?;
let title = note.title();        // borrow starts
drop(title);                     // borrow ends
note = vault.open(other_id).await?;  // OK — `note` was reassigned, no live borrow
```

**Variance** rules of thumb:
- `&'a T` is covariant in `T` and `'a`. Subtypes flow up. Default; use it.
- `&'a mut T` is **invariant in `T`**. This is why a `&mut Vec<&'static str>` can't accept a shorter-lived borrow. It bites at plugin boundaries — never expose `&mut Foo<'a>` across an extension trait that callers may parametrize differently.
- `fn(T) -> U` is contravariant in `T`, covariant in `U`. Function pointers can accept *broader* inputs.

If a lifetime parameter on a struct is making your API unusable, ask whether it should be:
1. Replaced with an owned type (often the right answer at module boundaries).
2. Replaced with `Arc<T>` (when the lifetime exists only because you wanted shared read access).
3. Hidden behind `impl Trait` in return position (carries lifetimes implicitly).

## Avoid unnecessary lifetime parameters

Bad — three lifetime parameters because the writer panicked at the borrow checker:

```rust
pub struct NoteRef<'doc, 'txn, 'vault> {
    doc: &'doc yrs::Doc,
    txn: &'txn yrs::Transaction<'doc>,
    vault: &'vault Vault,
}
```

Good — one lifetime, restructured ownership, no `'vault` because the vault is reachable through the transaction:

```rust
pub struct NoteRef<'txn> {
    txn: &'txn yrs::Transaction<'txn>,
}

impl<'txn> NoteRef<'txn> {
    pub fn body(&self) -> Bytes { /* … */ }
}
```

Use a single lifetime unless two methods on the same type genuinely need to distinguish borrows of different fields. `'a, 'b, 'c` on a public type is a code smell.

## The "indeference" pattern

When the borrow checker would force a self-referential struct, store the data in a `Vec` (or `Slab`) and pass indices. `petgraph` and `indexmap` are built on this. Itala uses it for the link graph (nodes in a `Vec<NoteNode>`, edges hold `NodeIndex` values), and for plugin handles (instances in a `Slab<PluginInstance>`, callers hold `PluginHandle(u32)`).

The cost is one indirection on access; the win is no lifetime parameter on the public API.
