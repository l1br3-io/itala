# redb

Single-page reference for the local metadata/index sidecar database.

## Why redb

- ACID, single-file, embedded — no daemon, no schema migration story to manage manually.
- Pure Rust, MIT license, actively maintained.
- Lower-overhead than sled for our workload (typed tables, no compaction surprises).

Use `redb` for: link graph edges, search-index pointers, plugin state, vault settings cache, the unsaved-changes journal. Do **not** use it for: note bodies (those are CRDT bytes on disk) or full-text indexes (Tantivy).

## Database setup

```rust
use redb::{Database, TableDefinition};

const NOTES_BY_PATH: TableDefinition<&str, &[u8]> = TableDefinition::new("notes_by_path");
const LINKS:         TableDefinition<&[u8], &[u8]> = TableDefinition::new("links");

let db = Database::create(vault_root.join(".itala/index.redb"))?;
```

Tables are typed at compile time via `TableDefinition<K, V>`. The types must implement `redb::Key` / `redb::Value`.

## Reads

```rust
let tx = db.begin_read()?;
let table = tx.open_table(NOTES_BY_PATH)?;
if let Some(bytes) = table.get("daily/2026-05-04.md")? {
    let note: NoteRecord = postcard::from_bytes(bytes.value())?;
}
```

`begin_read()` returns a snapshot. Multiple readers can run concurrently with one writer.

## Writes

```rust
let tx = db.begin_write()?;
{
    let mut table = tx.open_table(NOTES_BY_PATH)?;
    table.insert("daily/2026-05-04.md", &serialized[..])?;
}
tx.commit()?;
```

Open table inside a block so it drops before `tx.commit()`. One writer at a time — `begin_write` blocks if another writer is active.

## Serialization

redb stores `&[u8]`. Use `postcard` (compact, schema-evolution-friendly) for record values. Define a versioned envelope:

```rust
#[derive(Serialize, Deserialize)]
struct NoteRecordV1 {
    id: NoteId,
    title: String,
    tags: Vec<String>,
    modified: u64,
    checksum: [u8; 32],
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "v")]
enum NoteRecord {
    #[serde(rename = "1")]
    V1(NoteRecordV1),
}
```

Version-tag every persisted struct from day one. Future schema changes add new variants; readers handle older variants explicitly.

## Schema evolution

redb has no schema migration framework — you handle it explicitly:
1. Add a new variant (`NoteRecord::V2(...)`) and a converter.
2. On read, if a record is older, decode + convert + write back.
3. Lazy migration: never block startup on a full pass.

For breaking changes that need a one-shot pass, store a `schema_version` key in a `META` table and run a migration step at open time.

## Multi-table transactions

A single `WriteTransaction` can open multiple tables and commit atomically:

```rust
let tx = db.begin_write()?;
{
    let mut notes = tx.open_table(NOTES_BY_PATH)?;
    let mut links = tx.open_table(LINKS)?;
    notes.insert(path, &record)?;
    for link in note.links {
        links.insert(&link.key(), &link.value())?;
    }
}
tx.commit()?;
```

Use this whenever a logical change spans tables — never split into multiple commits.

## Iteration

```rust
let tx = db.begin_read()?;
let table = tx.open_table(LINKS)?;
for entry in table.range::<&[u8]>(..)? {
    let (key, value) = entry?;
    // …
}
```

Range scans are cheap; iteration order is sorted by key bytes. Design keys with that in mind (e.g. prefix by note id for backlinks).

## Backup / corruption

redb is single-file. Backup = copy the file when no writer is active. Corruption is rare but possible on power loss; if `Database::open` fails, surface to the user and offer reindex.

The vault Markdown files remain authoritative — `.itala/index.redb` is always reconstructible.

## Performance

- Open per-vault, keep handle alive for the session.
- `Arc<Database>` is cheap to share (`Database` is `Send + Sync`).
- Small writes: prefer batching into one transaction. Many tiny commits hurt throughput.
- For sustained write pressure (sync replays), use a dedicated writer task and a bounded mpsc.
