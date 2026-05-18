# Yrs (Y-CRDT)

Single-page reference for using `yrs` as the note body CRDT.

## Document model

Each note has one `yrs::Doc`. Inside that doc:

```rust
use yrs::{Doc, Text, GetString, Transact};

let doc = Doc::new();
let body = doc.get_or_insert_text("body");

let mut tx = doc.transact_mut();
body.push(&mut tx, "# Hello\nWorld");
drop(tx);

let snapshot = doc.transact().encode_state_as_update_v2(&yrs::StateVector::default());
// `snapshot` is the persistable bytes
```

- `body` is a `yrs::Text` shared type — supports edits at any offset, awareness of formatting marks.
- Frontmatter lives in a `yrs::Map` named `frontmatter` so structured edits stay collaborative.
- Block-level metadata (block IDs, anchors) live in `yrs::Map` keyed by block id.

## Transactions

- Reads: `doc.transact()` → `Transaction`.
- Writes: `doc.transact_mut()` → `TransactionMut`.
- Hold transactions briefly. Never hold across `.await`.
- A write transaction takes an exclusive document lock — batch related edits into one.

## Persistence

Two formats:
- **State updates (`encode_update_v2`)** — incremental diff, suitable for sync messages.
- **Full state (`encode_state_as_update_v2(StateVector::default())`)** — entire doc, used when writing to disk on save.

Itala persists the full state as `note.body.yrs` next to `note.md`. The Markdown file holds a CRDT-flattened render so non-Itala tools can read notes; the `.yrs` file is authoritative.

## Sync messages

`itala-sync` uses the standard Y protocol:
1. Each peer sends its state vector.
2. Each peer replies with the missing updates.
3. Subsequent edits flow as small `update` messages.

```rust
let state_vec = doc.transact().state_vector();
let updates = remote_doc.transact().encode_diff_v2(&state_vec);
```

## Observers

Subscribe to changes for incremental search-index updates and render invalidation:

```rust
let _sub = body.observe(|tx, event| {
    for delta in event.delta(tx) {
        // Delta::Inserted / Deleted / Retained
    }
});
```

Hold the subscription handle to keep it alive — drop = unsubscribe.

## Undo manager

```rust
use yrs::undo::UndoManager;

let mut undo = UndoManager::new(&doc, &body);
// edits get tracked
undo.undo()?;
undo.redo()?;
```

Scope undo manager to the editor session; one per note open in the UI.

## CodeMirror bridge

The TS shell uses `y-codemirror.next` to bind a CodeMirror 6 `EditorState` to a `yrs::Text`. The Rust kernel exposes the doc bytes; the bridge reconstructs a `Y.Doc` (yjs JS port) compatible with yrs over the same wire format.

## Common pitfalls

- **Don't share a `Doc` across `Send` boundaries without `Arc`.** `Doc` is `Send + Sync` but cheap to clone-as-`Arc`.
- **`get_or_insert_text` on every access** is correct — it's idempotent.
- **Don't store CRDT structures in serde-serialized state.** Persist as Y updates only.
- **GC**: yrs garbage-collects deleted state; you don't need to manage it. But snapshots before a peer has caught up will look bigger than expected.

## Testing CRDTs

Property tests (`proptest` per `rules/rust/testing.md`) cover:
- Convergence: two peers applying any interleaving of updates reach the same state.
- Idempotence: applying an update twice equals applying it once.
- Determinism: encoding round-trip is stable.
