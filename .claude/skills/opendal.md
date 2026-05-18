# OpenDAL

Single-page reference for storage-backend abstraction.

## Why OpenDAL

One trait, ~30 backends. Itala uses OpenDAL for the VFS layer, so the same vault code works against:
- Local filesystem (default)
- S3 / R2 / GCS / Azure Blob
- WebDAV
- IPFS
- In-memory (for tests)

Plugins that implement custom storage backends do so as OpenDAL `Service` impls, then the kernel adapts via the `Vfs` trait.

## Operator construction

```rust
use opendal::{Operator, services};

// Local
let op = Operator::new(services::Fs::default().root("/home/user/vault"))?.finish();

// S3
let op = Operator::new(
    services::S3::default()
        .bucket("my-vault")
        .region("us-east-1")
        .endpoint("https://r2.cloudflarestorage.com"),
)?.finish();
```

The `Operator` is `Send + Sync` and `Clone` — share via clone, not `Arc`.

## Reads

```rust
let bytes = op.read("notes/today.md").await?;        // returns Buffer
let bytes_vec = bytes.to_vec();
```

Streaming reads:

```rust
use futures::StreamExt;

let mut stream = op.reader("notes/big.md").await?.into_bytes_stream(..).await?;
while let Some(chunk) = stream.next().await {
    process(chunk?);
}
```

## Writes

```rust
op.write("notes/today.md", body.as_bytes().to_vec()).await?;

// Atomic write with conditional
op.write_with("notes/today.md", body.as_bytes().to_vec())
    .if_not_exists(true)        // fail if exists
    .await?;
```

## Listing

```rust
let mut lister = op.lister("notes/").await?;
while let Some(entry) = lister.next().await {
    let entry = entry?;
    if entry.metadata().mode().is_file() {
        process(entry.path());
    }
}
```

`lister_with("notes/").recursive(true)` walks subfolders.

## Capability negotiation

Backends differ in what they support. Check before using advanced features:

```rust
let cap = op.info().full_capability();
if cap.write_can_append {
    // use append
} else {
    // read-modify-write
}
```

Code that requires specific capabilities documents that requirement in the `Vfs` trait impl.

## Layers

OpenDAL composes via layers:

```rust
use opendal::layers::{LoggingLayer, RetryLayer, TimeoutLayer};

let op = Operator::new(services::S3::default().bucket("v")?)?
    .layer(LoggingLayer::default())
    .layer(RetryLayer::new().with_max_times(3))
    .layer(TimeoutLayer::new().with_timeout(Duration::from_secs(30)))
    .finish();
```

Standard layers we apply globally:
- `LoggingLayer` — emits `tracing` events for every op.
- `RetryLayer` — exponential backoff for transient failures.
- `TimeoutLayer` — bounds individual operations.
- `MetricsLayer` — Prometheus-shaped counters (post-MVP).

## Adapting to the kernel `Vfs` trait

```rust
use itala_kernel::Vfs;

pub struct OpendalVfs(Operator);

impl Vfs for OpendalVfs {
    async fn read(&self, path: &Utf8Path) -> Result<Bytes> {
        Ok(self.0.read(path.as_str()).await?.to_bytes())
    }
    async fn write(&self, path: &Utf8Path, bytes: Bytes) -> Result<()> {
        self.0.write(path.as_str(), bytes.to_vec()).await?;
        Ok(())
    }
    // …
}
```

The kernel never sees OpenDAL types directly — only the `Vfs` trait. Storage plugins implement `Vfs` either via OpenDAL or however else they like.

## Tests

`services::Memory` is a real OpenDAL backend in RAM. Use for unit tests:

```rust
let op = Operator::new(services::Memory::default())?.finish();
let vfs = OpendalVfs(op);
// run tests
```

Tests stay backend-agnostic — they use the `Vfs` trait, not concrete OpenDAL types.

## What not to use OpenDAL for

- The CRDT yrs files — those are read/written through `Vfs` like any other note asset, but the bytes themselves are the CRDT's job.
- The search index — Tantivy directly manages its own files via `tantivy::directory::Directory`. Don't try to put it on top of OpenDAL.
- The metadata redb file — local-only, single-file, doesn't need an abstraction.
