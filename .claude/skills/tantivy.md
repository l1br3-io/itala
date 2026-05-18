# Tantivy

Single-page reference for the search index in `crates/itala-search`.

## Schema

Define once in `schema.rs`:

```rust
use tantivy::schema::*;

pub fn build_schema() -> Schema {
    let mut s = Schema::builder();
    s.add_text_field("note_id", STRING | STORED);
    s.add_text_field("title",
        TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions))
            .set_stored());
    s.add_text_field("body",
        TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions)));
    s.add_facet_field("tags", FacetOptions::default());
    s.add_date_field("modified", FAST | STORED);
    s.build()
}
```

- `STRING` for IDs (no tokenization).
- `TEXT` with positions for prose (enables phrase queries).
- `FAST` for fields used in sorting/filtering.
- `FACET` for hierarchical tags.

## Index

```rust
let index = Index::create_in_dir(&path, schema)?;
let mut writer = index.writer(50_000_000)?;  // 50 MB heap

writer.add_document(doc!(
    note_id => id.to_string(),
    title   => title,
    body    => body,
    modified => DateTime::from(SystemTime::now()),
))?;

writer.commit()?;
```

- `IndexWriter` is single-instance; share via `Arc<Mutex<IndexWriter>>` if multi-threaded.
- Commit batches — one commit per save event is fine; batch under high write load.
- `writer.delete_term(Term::from_field_text(note_id_field, &id))` then `add_document` for updates.

## Queries

```rust
let reader = index.reader_builder()
    .reload_policy(ReloadPolicy::OnCommitWithDelay)
    .try_into()?;

let parser = QueryParser::for_index(&index, vec![title, body]);
let query = parser.parse_query("crdt OR \"local first\"")?;

let searcher = reader.searcher();
let top = searcher.search(&query, &TopDocs::with_limit(20))?;
for (score, doc_addr) in top {
    let doc: TantivyDocument = searcher.doc(doc_addr)?;
    // …
}
```

`reader.reload()` after a commit picks up new docs. With `OnCommitWithDelay`, this is automatic.

## Incremental updates

The kernel emits `note-saved` events; the search context handles them:

```rust
async fn on_note_saved(&self, id: NoteId) -> Result<()> {
    let note = self.vault.read(id).await?;
    let id_term = Term::from_field_text(self.fields.note_id, &id.to_string());
    let mut w = self.writer.lock().await;
    w.delete_term(id_term);
    w.add_document(self.fields.to_doc(&note))?;
    w.commit()?;
    Ok(())
}
```

Debounce commits at the call site — don't commit on every keystroke. Aim for one commit per second of activity.

## Performance

- 10k notes: index size ~50 MB; query <50 ms typical, well under Itala's 200 ms budget.
- Avoid `body` in `STORED` if you don't need snippet rendering — saves disk.
- `FAST` field for `modified` lets you sort by recency in O(n).

## Recovery

If the index is corrupted (kill -9 mid-commit, etc.), the host:
1. Detects via `Index::open_in_dir` failure or schema mismatch.
2. Deletes the index dir.
3. Walks the vault and re-indexes from scratch.

The vault Markdown files are authoritative — the index is always reconstructible.

## What goes in the index

- Note title + body (full-text)
- Frontmatter values (extracted via tokenization)
- Tags (faceted)
- Modified date (sort/filter)

What does **not** go in:
- The CRDT body bytes (that's `itala-vault`'s job)
- Plugin-injected metadata (a future per-plugin index, not this one)
