//! Domain store for notes, riding [`itala_kernel::DynStorageProvider`].
//!
//! `NoteStore` is the single host-side domain implementation — it is not
//! per-backend.  All Note↔[`Record`] mapping and query logic lives here;
//! the provider is responsible only for raw byte persistence.
//!
//! [`NoteQuery`] filters notes by tag equality or title substring.  For
//! providers that advertise [`ProviderCapabilities::native_query`] = `false`
//! (including [`FileProvider`]) the query runs as a host-side scan-and-filter
//! over all records.
//!
//! [`FileProvider`]: crate::adapters::FileProvider

use std::collections::BTreeMap;
use std::sync::Arc;

use bytes::Bytes;
use camino::Utf8PathBuf;
use itala_kernel::{DynStorageProvider, NoteId, Record};
use time::OffsetDateTime;

use crate::error::{Error, Result};
use crate::note::{self, Note};

// ---------------------------------------------------------------------------
// NoteStore
// ---------------------------------------------------------------------------

/// Domain store for notes over a [`DynStorageProvider`].
pub struct NoteStore {
    provider: Arc<dyn DynStorageProvider>,
}

impl NoteStore {
    /// Wrap a provider in a new `NoteStore`.
    pub fn new(provider: Arc<dyn DynStorageProvider>) -> Self {
        Self { provider }
    }

    /// List all note IDs, sorted for determinism.
    ///
    /// # Errors
    /// Propagates provider and parse errors, including
    /// [`Error::DuplicateNoteId`] and [`Error::InvalidFrontmatterId`].
    pub async fn list(&self) -> Result<Vec<NoteId>> {
        let keys = self.provider.list("notes").await?;
        let mut ids = keys
            .iter()
            .map(|k| {
                k.parse::<NoteId>()
                    .map_err(|_| Error::InvalidFrontmatterId { value: k.clone() })
            })
            .collect::<Result<Vec<_>>>()?;
        ids.sort();
        Ok(ids)
    }

    /// Read a note by ID.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if no note has that id. Propagates
    /// provider, parse, and identity errors otherwise.
    pub async fn read(&self, id: NoteId) -> Result<Note> {
        let key = id.to_string();
        let record = self
            .provider
            .get("notes", &key)
            .await?
            .ok_or(Error::NotFound { id })?;
        // Fetch the FS path separately (may be None for non-FS backends).
        let path = self.provider.fs_location("notes", &key).await?;
        // Assemble using a placeholder path; override with actual path below.
        let mut note = note::assemble(
            Utf8PathBuf::from(&key),
            id,
            &record.blob,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )?;
        note.path = path;
        Ok(note)
    }

    /// Persist a note, creating or overwriting it.
    ///
    /// The blob is assembled from the note's frontmatter (with `id:` ensured)
    /// and body.
    ///
    /// # Errors
    /// Propagates provider errors.
    pub async fn write(&self, note: &Note) -> Result<()> {
        let blob = note_to_blob(note);
        let mut indexed = BTreeMap::new();
        if let Some(v) = note.frontmatter.get("title").cloned() {
            indexed.insert("title".to_owned(), v);
        }
        if let Some(v) = note.frontmatter.get("tags").cloned() {
            indexed.insert("tags".to_owned(), v);
        }
        let record = Record {
            key: note.id.to_string(),
            indexed,
            blob,
        };
        self.provider.put("notes", record).await?;
        Ok(())
    }

    /// Delete a note by ID.
    ///
    /// Deleting a non-existent note succeeds silently.
    ///
    /// # Errors
    /// Propagates provider errors.
    pub async fn delete(&self, id: NoteId) -> Result<()> {
        self.provider.delete("notes", &id.to_string()).await?;
        Ok(())
    }

    /// Return the vault-root-relative filesystem path for the given ID.
    ///
    /// Returns `Ok(None)` for non-FS backends.
    ///
    /// # Errors
    /// Propagates provider errors, including [`Error::DuplicateNoteId`].
    pub async fn fs_location_of(&self, id: NoteId) -> Result<Option<Utf8PathBuf>> {
        Ok(self.provider.fs_location("notes", &id.to_string()).await?)
    }

    /// Query notes whose indexed fields match the given [`NoteQuery`].
    ///
    /// Uses a host-side scan-and-filter fallback for providers without native
    /// query support.
    ///
    /// # Errors
    /// Propagates provider and parse errors.
    pub async fn query(&self, q: &NoteQuery) -> Result<Vec<Note>> {
        let keys = self.provider.list("notes").await?;
        let mut results = Vec::new();
        for key in &keys {
            let Some(record) = self.provider.get("notes", key).await? else {
                continue;
            };
            if q.matches(&record.indexed) {
                let id = key
                    .parse::<NoteId>()
                    .map_err(|_| Error::InvalidFrontmatterId { value: key.clone() })?;
                let path = self.provider.fs_location("notes", key).await?;
                let mut note = note::assemble(
                    Utf8PathBuf::from(key),
                    id,
                    &record.blob,
                    OffsetDateTime::UNIX_EPOCH,
                    OffsetDateTime::UNIX_EPOCH,
                )?;
                note.path = path;
                results.push(note);
            }
        }
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// NoteQuery
// ---------------------------------------------------------------------------

/// Filter for querying notes by indexed fields.
///
/// All specified fields must match (AND semantics).
pub struct NoteQuery {
    /// Require an exact match in the `tags` array (case-sensitive).
    pub tag: Option<String>,
    /// Require the `title` to contain this substring (case-insensitive).
    pub title_substring: Option<String>,
}

impl NoteQuery {
    fn matches(&self, indexed: &BTreeMap<String, serde_json::Value>) -> bool {
        if let Some(tag) = &self.tag {
            match indexed.get("tags") {
                Some(serde_json::Value::Array(arr)) => {
                    if !arr.iter().any(|v| v.as_str() == Some(tag.as_str())) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        if let Some(substr) = &self.title_substring {
            match indexed.get("title") {
                Some(serde_json::Value::String(title)) => {
                    if !title.to_lowercase().contains(&substr.to_lowercase()) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Assemble a note's on-disk bytes from its in-memory representation.
///
/// Inserts `id:` into the frontmatter (overwriting any existing value),
/// emits the canonical YAML, and concatenates with the body.
fn note_to_blob(note: &Note) -> Bytes {
    let mut fm = note.frontmatter.clone();
    fm.insert(
        "id".to_owned(),
        serde_json::Value::String(note.id.to_string()),
    );
    let mut out = Vec::new();
    out.extend_from_slice(b"---\n");
    out.extend_from_slice(fm.emit().as_bytes());
    out.extend_from_slice(b"---\n");
    out.extend_from_slice(&note.body);
    Bytes::from(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bytes::Bytes;
    use camino::Utf8PathBuf;
    use itala_kernel::{DynVfs, NoteId};
    use time::OffsetDateTime;

    use super::*;
    use crate::adapters::FileProvider;
    use crate::frontmatter::Frontmatter;
    use crate::note::assemble;
    use crate::test_support::MemoryVfs;

    fn make_store(vfs: MemoryVfs) -> NoteStore {
        let provider = Arc::new(FileProvider::new(
            Arc::new(vfs) as Arc<dyn DynVfs>,
            Utf8PathBuf::from("/vault"),
        ));
        NoteStore::new(provider)
    }

    fn build_raw(fm: &Frontmatter, body: &[u8]) -> Bytes {
        let mut v = Vec::new();
        v.extend_from_slice(b"---\n");
        v.extend_from_slice(fm.emit().as_bytes());
        v.extend_from_slice(b"---\n");
        v.extend_from_slice(body);
        Bytes::from(v)
    }

    fn simple_note(title: &str, body: &[u8]) -> Note {
        let id = NoteId::new();
        let mut fm = Frontmatter::empty();
        fm.insert("title".to_owned(), serde_json::json!(title));
        let raw = build_raw(&fm, body);
        assemble(
            Utf8PathBuf::from("placeholder.md"),
            id,
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap()
    }

    fn make_note_with_tags(title: &str, tags: &[&str], body: &[u8]) -> Note {
        let id = NoteId::new();
        let tags_json: Vec<serde_json::Value> = tags
            .iter()
            .map(|t| serde_json::Value::String((*t).to_owned()))
            .collect();
        let mut fm = Frontmatter::empty();
        fm.insert("title".to_owned(), serde_json::json!(title));
        fm.insert("tags".to_owned(), serde_json::Value::Array(tags_json));
        let raw = build_raw(&fm, body);
        assemble(
            Utf8PathBuf::from("placeholder.md"),
            id,
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap()
    }

    // -----------------------------------------------------------------------
    // write / read round-trips
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn write_then_read_body_matches() {
        let store = make_store(MemoryVfs::default());
        let note = simple_note("Hello", b"body content\n");
        let id = note.id;
        store.write(&note).await.unwrap();

        let read = store.read(id).await.unwrap();
        assert_eq!(&read.body[..], b"body content\n");
    }

    #[tokio::test]
    async fn write_then_read_title_matches() {
        let store = make_store(MemoryVfs::default());
        let note = simple_note("My Title", b"body\n");
        let id = note.id;
        store.write(&note).await.unwrap();

        let read = store.read(id).await.unwrap();
        assert_eq!(
            read.frontmatter.get("title").unwrap(),
            &serde_json::json!("My Title")
        );
    }

    #[tokio::test]
    async fn write_then_read_checksum_is_stable() {
        let store = make_store(MemoryVfs::default());
        let note = simple_note("Stable", b"body\n");
        let id = note.id;
        store.write(&note).await.unwrap();

        let read1 = store.read(id).await.unwrap();
        let read2 = store.read(id).await.unwrap();
        assert_eq!(read1.checksum, read2.checksum, "checksum must be stable");
    }

    #[tokio::test]
    async fn write_ten_notes_all_read_back() {
        let store = make_store(MemoryVfs::default());
        let mut notes = Vec::new();
        for i in 0..10_u8 {
            let note = simple_note(&format!("Note {i}"), format!("body {i}\n").as_bytes());
            notes.push(note);
        }
        for n in &notes {
            store.write(n).await.unwrap();
        }
        for n in &notes {
            let read = store.read(n.id).await.unwrap();
            assert_eq!(read.id, n.id);
            assert_eq!(read.frontmatter.get("title"), n.frontmatter.get("title"));
        }
    }

    // -----------------------------------------------------------------------
    // delete
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_then_read_returns_not_found() {
        let store = make_store(MemoryVfs::default());
        let note = simple_note("Gone", b"bye\n");
        let id = note.id;
        store.write(&note).await.unwrap();
        store.delete(id).await.unwrap();

        let err = store.read(id).await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    // -----------------------------------------------------------------------
    // list
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_returns_sorted_ids() {
        let store = make_store(MemoryVfs::default());
        for i in 0..5_u8 {
            store
                .write(&simple_note(&format!("Note {i}"), b"body\n"))
                .await
                .unwrap();
        }
        let ids = store.list().await.unwrap();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "list must return sorted ids");
    }

    // -----------------------------------------------------------------------
    // fs_location_of
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn fs_location_of_returns_some_after_write() {
        let store = make_store(MemoryVfs::default());
        let note = simple_note("Located", b"body\n");
        let id = note.id;
        store.write(&note).await.unwrap();

        let loc = store.fs_location_of(id).await.unwrap();
        assert!(loc.is_some(), "fs_location_of must return Some after write");
        assert_eq!(loc.unwrap().extension(), Some("md"));
    }

    #[tokio::test]
    async fn fs_location_of_returns_none_for_unknown() {
        let store = make_store(MemoryVfs::default());
        let loc = store.fs_location_of(NoteId::new()).await.unwrap();
        assert!(loc.is_none());
    }

    // -----------------------------------------------------------------------
    // NoteQuery — host-side scan-and-filter
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn query_by_tag_returns_matching_notes() {
        let store = make_store(MemoryVfs::default());
        let n1 = make_note_with_tags("Rust Note", &["rust", "kernel"], b"body\n");
        let n2 = make_note_with_tags("Python Note", &["python"], b"body\n");
        store.write(&n1).await.unwrap();
        store.write(&n2).await.unwrap();

        let q = NoteQuery {
            tag: Some("rust".to_owned()),
            title_substring: None,
        };
        let results = store.query(&q).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, n1.id);
    }

    #[tokio::test]
    async fn query_by_title_substring_case_insensitive() {
        let store = make_store(MemoryVfs::default());
        let n1 = make_note_with_tags("Rust Kernel Notes", &[], b"body\n");
        let n2 = make_note_with_tags("Python Guide", &[], b"body\n");
        store.write(&n1).await.unwrap();
        store.write(&n2).await.unwrap();

        let q = NoteQuery {
            tag: None,
            title_substring: Some("kernel".to_owned()),
        };
        let results = store.query(&q).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, n1.id);
    }

    #[tokio::test]
    async fn query_no_match_returns_empty() {
        let store = make_store(MemoryVfs::default());
        store
            .write(&make_note_with_tags("Alpha", &["a"], b"body\n"))
            .await
            .unwrap();

        let q = NoteQuery {
            tag: Some("nonexistent".to_owned()),
            title_substring: None,
        };
        let results = store.query(&q).await.unwrap();
        assert!(results.is_empty());
    }
}
