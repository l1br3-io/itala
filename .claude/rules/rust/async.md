# Rust Async & Concurrency

## Runtime

- One runtime per process: `tokio` with the multi-threaded scheduler.
- `#[tokio::main]` belongs **only** at binary roots (`main()` in `itala-cli`, Tauri commands setup).
- Library crates never start runtimes — they accept and use the ambient runtime.

## Async traits

Rust 1.75+ has stable `async fn in trait`. Use it.

```rust
pub trait Vfs: Send + Sync {
    async fn read(&self, path: &Utf8Path) -> Result<Bytes>;
    async fn write(&self, path: &Utf8Path, bytes: Bytes) -> Result<()>;
}
```

When you need `dyn Trait` (plugin boundaries, runtime-selected adapters), use `async-trait`:

```rust
#[async_trait::async_trait]
pub trait DynVfs: Send + Sync {
    async fn read(&self, path: &Utf8Path) -> Result<Bytes>;
}

pub fn make_box(v: impl DynVfs + 'static) -> Arc<dyn DynVfs> { Arc::new(v) }
```

Default to native async-fn-in-trait. Reach for `async-trait` only when boxing is required.

### The `Send` leak on RPITIT

Native `async fn` in trait desugars to a return-position-impl-Trait (RPITIT). The returned future is **not automatically `Send`** — auto-traits are inferred from each impl's body, and a single non-`Send` local will silently drop the bound. Callers using `tokio::spawn` will then get cryptic "future is not `Send`" errors at the call site, not the trait.

Two workarounds:

```rust
// (1) Manual return-type with the bound — explicit, no extra dep.
pub trait Vfs: Send + Sync {
    fn read(&self, path: &Utf8Path) -> impl Future<Output = Result<Bytes>> + Send;
}

// (2) `trait-variant` — generates a `Send`-bounded sibling trait.
#[trait_variant::make(SendVfs: Send)]
pub trait Vfs: Send + Sync {
    async fn read(&self, path: &Utf8Path) -> Result<Bytes>;
}
// Callers that need `Send` futures impl/use `SendVfs`; others use `Vfs`.
```

Itala rule: any async trait whose impls will be `spawn`-ed (storage, sync transport, AI provider) declares the `Send` bound explicitly via one of these two patterns. Don't rely on inference.

## Send + Sync

- All public types are `Send + Sync` unless explicitly documented otherwise — make impossibility a comment.
- Use `Arc<T>` for shared ownership across tasks. Use `Arc<RwLock<T>>` only when readers actually contend; otherwise `Arc<Mutex<T>>` is simpler.
- Prefer message passing (`tokio::sync::mpsc`) over shared mutable state when the access pattern is naturally sequential.

## Spawning

Use **structured concurrency** via `tokio::task::JoinSet`. Loose `tokio::spawn` lets background tasks outlive their context and is forbidden outside the top-level runtime owner.

```rust
let mut tasks = JoinSet::new();
for note in notes {
    tasks.spawn(reindex(note));
}
while let Some(res) = tasks.join_next().await {
    res??;   // task panic → propagate; reindex Result → propagate
}
```

Spawning blocking work uses `tokio::task::spawn_blocking` — never block the runtime threads with sync I/O or CPU-bound work over a few μs.

## Channels

| Pattern | Channel |
|---|---|
| Single producer → single consumer | `tokio::sync::mpsc` (bounded; pick a real bound) |
| Many producers → single consumer | `tokio::sync::mpsc` |
| Single producer → many consumers, replay needed | `tokio::sync::broadcast` |
| Latest value only (event bus) | `tokio::sync::watch` |
| One-shot reply | `tokio::sync::oneshot` |

Bounded channels only. Unbounded queues hide back-pressure bugs.

## Cancellation

Use `tokio_util::sync::CancellationToken` for cooperative shutdown.

```rust
pub async fn run(token: CancellationToken) -> Result<()> {
    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            event = bus.recv() => process(event?).await?,
        }
    }
    Ok(())
}
```

Drop-on-cancel is fine for leaf tasks; long-running services accept a token explicitly.

### `select!` cancel-safety

`tokio::select!` polls every branch, then drops the futures of branches that didn't complete. A branch's future must be **cancel-safe**: dropping it mid-poll must not lose data or leave the underlying resource in an unusable state.

Per the tokio `select!` rustdoc:

| Cancel-safe (use freely in `select!`) | Not cancel-safe (loop outside `select!` or use `Box::pin` + manual poll) |
|---|---|
| `mpsc::Receiver::recv`, `UnboundedReceiver::recv` | `AsyncReadExt::read_exact` |
| `oneshot::Receiver` (`.await`) | `AsyncReadExt::read_to_end`, `read_to_string` |
| `broadcast::Receiver::recv` | `AsyncWriteExt::write_all` |
| `AsyncReadExt::read` (returns however many bytes arrived) | `AsyncBufReadExt::read_line` (partial-line state is lost) |
| `Stream::next` (on a `FusedStream`) | Anything with multi-step internal state machines you didn't write |

When in doubt, check the rustdoc page of the specific method — tokio marks cancel-safety explicitly.

## Locks

- Acquire only across `await` if the state is genuinely shared — prefer scoping the lock to a sync block.
- Never hold a `std::sync::Mutex` across `.await`. Use `tokio::sync::Mutex` only when you must.
- Document lock ordering in a comment when more than one lock is held.

## Streams

Use `futures::Stream` + `tokio_stream` adapters. Hand-rolled `Future` impls are rarely needed — if you reach for `Pin<Box<dyn …>>`, reconsider.

```rust
pub fn watch_changes(vault: VaultId) -> impl Stream<Item = ChangeEvent> { /* … */ }
```
