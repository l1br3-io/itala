//! FS-backed [`StorageProvider`] over an injected [`DynVfs`].
//!
//! Maps the `"notes"` collection to `*.md` files under the vault root.
//! CORE capabilities: `get`/`put`/`delete`/`list`.
//! Overrides `fs_location` to return vault-root-relative paths.
//! Does **not** support `query` or `transaction` natively — the host layer
//! falls back to host-side scan-and-filter via `NoteStore`.
//!
//! **Side effect:** `list` and `fs_location` (and transitively `get`) backfill
//! the `id:` frontmatter field for any note that lacks it.  This is documented
//! on the impl, not the trait; `NoteStore` must not also write.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use itala_kernel::{
    DynVfs, NoteId, ProviderCapabilities, Record, Result as KernelResult, StorageProvider,
};

use crate::note::backfill;
use crate::{frontmatter, note};

/// FS-backed [`StorageProvider`] over an injected [`DynVfs`].
///
/// One collection is currently meaningful: `"notes"` maps to all `*.md` files
/// recursively under the vault root.  Operations on unknown collections succeed
/// silently with empty results.
pub struct FileProvider {
    vfs: Arc<dyn DynVfs>,
    root: Utf8PathBuf,
}

impl FileProvider {
    /// Create a new `FileProvider` over the given VFS and vault root.
    #[must_use]
    pub fn new(vfs: Arc<dyn DynVfs>, root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            vfs,
            root: root.into(),
        }
    }
}

impl StorageProvider for FileProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_query: false,
            transactions: false,
        }
    }

    fn get(
        &self,
        collection: &str,
        key: &str,
    ) -> impl Future<Output = KernelResult<Option<Record>>> + Send + '_ {
        let vfs = Arc::clone(&self.vfs);
        let root = self.root.clone();
        let col = collection.to_owned();
        let key = key.to_owned();
        async move {
            if col != "notes" {
                return Ok(None);
            }
            let id = parse_id(&key)?;
            let Some(rel) = find_path(&vfs, &root, id).await? else {
                return Ok(None);
            };
            let abs = root.join(&rel);
            let raw = vfs.read(&abs).await?;
            let (fm, _) = frontmatter::split(&raw).map_err(wrap)?;
            let indexed = extract_indexed(&fm);
            Ok(Some(Record {
                key,
                indexed,
                blob: raw,
            }))
        }
    }

    fn put(
        &self,
        collection: &str,
        record: Record,
    ) -> impl Future<Output = KernelResult<()>> + Send + '_ {
        let vfs = Arc::clone(&self.vfs);
        let root = self.root.clone();
        let col = collection.to_owned();
        async move {
            if col != "notes" {
                return Ok(());
            }
            let id = parse_id(&record.key)?;
            let target = match find_path(&vfs, &root, id).await? {
                Some(rel) => root.join(rel),
                None => root.join(format!("{}.md", record.key)),
            };
            vfs.write(&target, record.blob).await?;
            Ok(())
        }
    }

    fn delete(
        &self,
        collection: &str,
        key: &str,
    ) -> impl Future<Output = KernelResult<()>> + Send + '_ {
        let vfs = Arc::clone(&self.vfs);
        let root = self.root.clone();
        let col = collection.to_owned();
        let key = key.to_owned();
        async move {
            if col != "notes" {
                return Ok(());
            }
            let id = parse_id(&key)?;
            if let Some(rel) = find_path(&vfs, &root, id).await? {
                vfs.delete(&root.join(rel)).await?;
            }
            Ok(())
        }
    }

    fn list(
        &self,
        collection: &str,
    ) -> impl Future<Output = KernelResult<Vec<String>>> + Send + '_ {
        let vfs = Arc::clone(&self.vfs);
        let root = self.root.clone();
        let col = collection.to_owned();
        async move {
            if col != "notes" {
                return Ok(vec![]);
            }
            let paths = collect_md_paths(&vfs, &root).await?;
            let mut seen: Vec<(NoteId, Utf8PathBuf)> = Vec::new();
            for path in paths {
                let id = resolve_id(&vfs, &root, &path).await?;
                if let Some((_, first)) = seen.iter().find(|(eid, _)| *eid == id) {
                    return Err(wrap(crate::Error::DuplicateNoteId {
                        id,
                        first_fs_location: first.clone(),
                        second_fs_location: path,
                    }));
                }
                seen.push((id, path));
            }
            Ok(seen.into_iter().map(|(id, _)| id.to_string()).collect())
        }
    }

    fn fs_location(
        &self,
        collection: &str,
        key: &str,
    ) -> impl Future<Output = KernelResult<Option<Utf8PathBuf>>> + Send + '_ {
        let vfs = Arc::clone(&self.vfs);
        let root = self.root.clone();
        let col = collection.to_owned();
        let key = key.to_owned();
        async move {
            if col != "notes" {
                return Ok(None);
            }
            let id = parse_id(&key)?;
            find_path(&vfs, &root, id).await
        }
    }
}

// ---------------------------------------------------------------------------
// Module-level helpers
// ---------------------------------------------------------------------------

/// Box a vault error into a kernel `Provider` error.
fn wrap(e: crate::Error) -> itala_kernel::Error {
    itala_kernel::Error::Provider(Box::new(e))
}

/// Parse a [`NoteId`] from a string key, wrapping failures as a `Provider` error.
fn parse_id(key: &str) -> KernelResult<NoteId> {
    key.parse::<NoteId>().map_err(|_| {
        wrap(crate::Error::InvalidFrontmatterId {
            value: key.to_owned(),
        })
    })
}

/// Extract `title` and `tags` from frontmatter into the [`Record::indexed`] map.
fn extract_indexed(fm: &crate::Frontmatter) -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    if let Some(v) = fm.get("title").cloned() {
        m.insert("title".to_owned(), v);
    }
    if let Some(v) = fm.get("tags").cloned() {
        m.insert("tags".to_owned(), v);
    }
    m
}

/// Recursively collect vault-root-relative `Utf8PathBuf` entries for every
/// `*.md` file under `root`.
async fn collect_md_paths(
    vfs: &Arc<dyn DynVfs>,
    root: &Utf8PathBuf,
) -> KernelResult<Vec<Utf8PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let children = vfs.list(&dir).await?;
        for child in children {
            let rel = child
                .strip_prefix(root.as_path())
                .unwrap_or(&child)
                .to_owned();
            if child.extension() == Some("md") {
                out.push(rel);
            } else if child.extension().is_none() {
                stack.push(child);
            }
        }
    }
    tracing::debug!(count = out.len(), root = %root, "file provider listed notes");
    Ok(out)
}

/// Resolve (and optionally backfill) the [`NoteId`] for a vault-root-relative path.
///
/// Reads the file, returns the `id:` frontmatter field if present, or derives
/// the id from the path and **writes** the backfilled bytes before returning.
async fn resolve_id(
    vfs: &Arc<dyn DynVfs>,
    root: &Utf8PathBuf,
    rel: &Utf8Path,
) -> KernelResult<NoteId> {
    let abs = root.join(rel);
    let raw = vfs.read(&abs).await?;
    let (fm, body) = frontmatter::split(&raw).map_err(wrap)?;
    if let Some(id) = note::id_from_frontmatter(&fm).map_err(wrap)? {
        Ok(id)
    } else {
        let id = note::id_from_path(rel);
        let rewritten = backfill::inject_id(&raw, &fm, body, id);
        vfs.write(&abs, rewritten).await?;
        Ok(id)
    }
}

/// Scan the vault for the path of a note with the given `target` id.
///
/// Returns `Ok(None)` when not found.  Returns a `Provider`-boxed
/// [`crate::Error::DuplicateNoteId`] when two files share the same id.
async fn find_path(
    vfs: &Arc<dyn DynVfs>,
    root: &Utf8PathBuf,
    target: NoteId,
) -> KernelResult<Option<Utf8PathBuf>> {
    let paths = collect_md_paths(vfs, root).await?;
    let mut found: Option<Utf8PathBuf> = None;
    for path in paths {
        let id = resolve_id(vfs, root, &path).await?;
        if id == target {
            if let Some(first) = found {
                return Err(wrap(crate::Error::DuplicateNoteId {
                    id: target,
                    first_fs_location: first,
                    second_fs_location: path,
                }));
            }
            found = Some(path);
        }
    }
    Ok(found)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bytes::Bytes;
    use camino::Utf8PathBuf;
    use itala_kernel::{DynStorageProvider, DynVfs, NoteId};

    use super::*;
    use crate::test_support::MemoryVfs;

    fn make_provider(vfs: MemoryVfs) -> Arc<dyn DynStorageProvider> {
        Arc::new(FileProvider::new(
            Arc::new(vfs) as Arc<dyn DynVfs>,
            Utf8PathBuf::from("/vault"),
        ))
    }

    fn vault_with_note(path: &str, bytes: &[u8]) -> (MemoryVfs, Arc<dyn DynStorageProvider>) {
        let vfs = MemoryVfs::default();
        vfs.put(path, bytes);
        let provider = make_provider(vfs.clone());
        (vfs, provider)
    }

    // -----------------------------------------------------------------------
    // list
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_returns_key_for_note() {
        let (_, provider) = vault_with_note("/vault/note.md", b"---\ntitle: Hi\n---\nbody\n");
        let keys = provider.list("notes").await.unwrap();
        assert_eq!(keys.len(), 1);
        // Key must be parseable as a NoteId.
        keys[0]
            .parse::<NoteId>()
            .expect("key must be a valid NoteId");
    }

    #[tokio::test]
    async fn list_unknown_collection_returns_empty() {
        let (_, provider) = vault_with_note("/vault/note.md", b"body\n");
        let keys = provider.list("config").await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn list_duplicate_id_produces_provider_error_that_downcasts() {
        let shared = NoteId::new();
        let content = format!("---\nid: \"{shared}\"\n---\nbody\n");
        let vfs = MemoryVfs::default();
        vfs.put("/vault/a.md", content.as_bytes());
        vfs.put("/vault/b.md", content.as_bytes());
        let provider = make_provider(vfs);

        let err = provider.list("notes").await.unwrap_err();
        assert!(matches!(err, itala_kernel::Error::Provider(_)));
        if let itala_kernel::Error::Provider(b) = err {
            let vault_err = b.downcast::<crate::Error>().expect("must be vault::Error");
            assert!(matches!(*vault_err, crate::Error::DuplicateNoteId { .. }));
        }
    }

    #[tokio::test]
    async fn list_malformed_id_produces_invalid_frontmatter_id() {
        let (_, provider) =
            vault_with_note("/vault/bad.md", b"---\nid: \"not-a-ulid\"\n---\nbody\n");
        let err = provider.list("notes").await.unwrap_err();
        assert!(matches!(err, itala_kernel::Error::Provider(_)));
        if let itala_kernel::Error::Provider(b) = err {
            let vault_err = b.downcast::<crate::Error>().expect("must be vault::Error");
            assert!(matches!(
                *vault_err,
                crate::Error::InvalidFrontmatterId { .. }
            ));
        }
    }

    // -----------------------------------------------------------------------
    // get
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_returns_record_with_blob_and_indexed() {
        let id = NoteId::new();
        let content = format!("---\nid: \"{id}\"\ntitle: \"My Note\"\n---\nbody here\n");
        let (_, provider) = vault_with_note("/vault/note.md", content.as_bytes());

        let record = provider
            .get("notes", &id.to_string())
            .await
            .unwrap()
            .expect("note must exist");

        assert_eq!(record.key, id.to_string());
        assert_eq!(
            record.indexed.get("title").unwrap(),
            &serde_json::json!("My Note")
        );
        assert!(!record.blob.is_empty());
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let (_, provider) = vault_with_note("/vault/note.md", b"body\n");
        let key = NoteId::new().to_string();
        let result = provider.get("notes", &key).await.unwrap();
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // put / delete
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn put_then_get_round_trip() {
        let id = NoteId::new();
        let vfs = MemoryVfs::default();
        // No notes yet — put creates a new one.
        let provider: Arc<dyn DynStorageProvider> = Arc::new(FileProvider::new(
            Arc::new(vfs) as Arc<dyn DynVfs>,
            Utf8PathBuf::from("/vault"),
        ));

        let content = format!("---\nid: \"{id}\"\ntitle: \"New\"\n---\nbody\n");
        let record = Record {
            key: id.to_string(),
            indexed: BTreeMap::new(),
            blob: Bytes::from(content.into_bytes()),
        };
        provider.put("notes", record).await.unwrap();

        let got = provider.get("notes", &id.to_string()).await.unwrap();
        assert!(got.is_some(), "put note must be retrievable");
    }

    #[tokio::test]
    async fn delete_removes_note() {
        let id = NoteId::new();
        let content = format!("---\nid: \"{id}\"\n---\nbody\n");
        let (_, provider) = vault_with_note("/vault/note.md", content.as_bytes());

        provider.delete("notes", &id.to_string()).await.unwrap();
        let result = provider.get("notes", &id.to_string()).await.unwrap();
        assert!(result.is_none(), "note must be gone after delete");
    }

    // -----------------------------------------------------------------------
    // fs_location
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn fs_location_returns_path_for_known_note() {
        let id = NoteId::new();
        let content = format!("---\nid: \"{id}\"\n---\nbody\n");
        let (_, provider) = vault_with_note("/vault/note.md", content.as_bytes());

        let path = provider
            .fs_location("notes", &id.to_string())
            .await
            .unwrap();
        assert!(path.is_some());
        assert_eq!(path.unwrap().extension(), Some("md"));
    }

    #[tokio::test]
    async fn fs_location_returns_none_for_unknown_id() {
        let (_, provider) = vault_with_note("/vault/note.md", b"body\n");
        let unknown = NoteId::new().to_string();
        let path = provider.fs_location("notes", &unknown).await.unwrap();
        assert!(path.is_none());
    }

    // -----------------------------------------------------------------------
    // compile-time assertions
    // -----------------------------------------------------------------------

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn file_provider_is_send_sync() {
        assert_send_sync::<FileProvider>();
    }
}
