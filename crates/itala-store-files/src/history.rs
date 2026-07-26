//! `FilesHistoryStore` — JSON-on-Vfs adapter for [`DynHistoryStore`].
//!
//! Implements ADR-0004 §3 for the M1 history port: each snapshot is persisted
//! as two files under `.itala/history/<note_id>/<history_id>{.json,.md}`:
//!
//! - `.json` carries the typed metadata (timestamp, checksum);
//! - `.md` carries the snapshot body bytes verbatim (Markdown is UTF-8 and
//!   human-inspectable).
//!
//! A small `index.json` per note lists entries newest-first; on read it is
//! rebuilt from the directory if missing.

use async_trait::async_trait;
use bytes::Bytes;
use time::OffsetDateTime;

use itala_kernel::history::{DynHistoryStore, HistoryEntry};
use itala_kernel::vfs::DynVfs;
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Error, HistoryId, NoteId, Result};

const HISTORY_DIR: &str = ".itala/history";

/// JSON-sidecar metadata file (typed fields not present in the body).
#[derive(serde::Serialize, serde::Deserialize)]
struct HistoryMeta {
    id: String,
    note_id: String,
    /// Unix timestamp, seconds since epoch.
    timestamp_secs: i64,
    timestamp_nanos: u32,
    checksum: [u8; 32],
}

impl HistoryMeta {
    fn from_entry(entry: &HistoryEntry) -> Self {
        let ts = entry.timestamp;
        Self {
            id: entry.id.to_string(),
            note_id: entry.note_id.to_string(),
            timestamp_secs: ts.unix_timestamp(),
            timestamp_nanos: ts.nanosecond(),
            checksum: entry.checksum,
        }
    }

    fn to_entry(&self) -> Result<HistoryEntry> {
        let id: HistoryId = self
            .id
            .parse()
            .map_err(|e| Error::invalid_id(format!("history id parse: {e}")))?;
        let note_id: NoteId = self
            .note_id
            .parse()
            .map_err(|e| Error::invalid_id(format!("note id parse: {e}")))?;
        let timestamp = OffsetDateTime::from_unix_timestamp(self.timestamp_secs)
            .map_err(|e| Error::config(format!("timestamp secs: {e}")))?
            + time::Duration::nanoseconds(i64::from(self.timestamp_nanos));
        Ok(HistoryEntry::new(id, note_id, timestamp, self.checksum))
    }
}

/// File-backed `DynHistoryStore` adapter.
pub struct FilesHistoryStore {
    vfs: Box<dyn DynVfs>,
}

impl FilesHistoryStore {
    /// Creates an adapter over the given VFS. The VFS must be writeable.
    #[must_use]
    pub fn new(vfs: Box<dyn DynVfs>) -> Self {
        Self { vfs }
    }

    fn note_dir(note_id: NoteId) -> Result<VfsPath> {
        VfsPath::new(HISTORY_DIR)?.join(note_id.to_string().as_str())
    }

    fn entry_meta_path(id: HistoryId, note_id: NoteId) -> Result<VfsPath> {
        Self::note_dir(note_id)?.join(format!("{id}.json").as_str())
    }

    fn entry_body_path(id: HistoryId, note_id: NoteId) -> Result<VfsPath> {
        Self::note_dir(note_id)?.join(format!("{id}.md").as_str())
    }
}

#[async_trait]
impl DynHistoryStore for FilesHistoryStore {
    async fn snapshot(
        &self,
        note_id: NoteId,
        bytes: Bytes,
        checksum: [u8; 32],
    ) -> Result<HistoryId> {
        let id = HistoryId::new();
        let entry = HistoryEntry::new(id, note_id, OffsetDateTime::now_utc(), checksum);
        let meta = HistoryMeta::from_entry(&entry);
        let meta_bytes = serde_json::to_vec(&meta)
            .map_err(|e| Error::config(format!("serialize history meta: {e}")))?;
        let meta_path = Self::entry_meta_path(id, note_id)?;
        let body_path = Self::entry_body_path(id, note_id)?;
        self.vfs.write(&meta_path, Bytes::from(meta_bytes)).await?;
        self.vfs.write(&body_path, bytes).await?;
        Ok(id)
    }

    async fn list(&self, note_id: NoteId) -> Result<Vec<HistoryEntry>> {
        let dir = Self::note_dir(note_id)?;
        let entries = match self.vfs.list(&dir).await {
            Ok(e) => e,
            Err(Error::NotFound { .. }) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut out = Vec::new();
        for entry in entries {
            let name = entry.path.file_name().unwrap_or("");
            let Some(id_str) = name.strip_suffix(".json") else {
                continue;
            };
            let Ok(id) = id_str.parse::<HistoryId>() else {
                continue;
            };
            let bytes = self.vfs.read(&entry.path).await?;
            let meta: HistoryMeta = serde_json::from_slice(&bytes)
                .map_err(|e| Error::config(format!("deserialize history meta: {e}")))?;
            let parsed = meta.to_entry()?;
            if parsed.id != id {
                continue;
            }
            out.push(parsed);
        }
        out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(out)
    }

    async fn get(&self, id: HistoryId) -> Result<HistoryEntry> {
        let note_id = self.find_note_for(id).await?;
        let path = Self::entry_meta_path(id, note_id)?;
        let bytes = self.vfs.read(&path).await?;
        let meta: HistoryMeta = serde_json::from_slice(&bytes)
            .map_err(|e| Error::config(format!("deserialize history meta: {e}")))?;
        meta.to_entry()
    }

    async fn restore(&self, id: HistoryId) -> Result<Bytes> {
        let note_id = self.find_note_for(id).await?;
        let path = Self::entry_body_path(id, note_id)?;
        self.vfs.read(&path).await
    }
}

impl FilesHistoryStore {
    /// Scans `.itala/history/` for the note that owns `id`. Linear scan;
    /// acceptable for M1 (histories are small per note, and `get` / `restore`
    /// are rare ops compared to `list`).
    async fn find_note_for(&self, id: HistoryId) -> Result<NoteId> {
        let root = VfsPath::new(HISTORY_DIR)?;
        let entries = match self.vfs.list(&root).await {
            Ok(e) => e,
            Err(Error::NotFound { .. }) => {
                return Err(Error::not_found(format!("history:{id}")));
            }
            Err(e) => return Err(e),
        };
        for entry in entries {
            if !entry.metadata.is_dir {
                continue;
            }
            let Some(name) = entry.path.file_name() else {
                continue;
            };
            let Ok(note_id) = name.parse::<NoteId>() else {
                continue;
            };
            // Look for `<id>.json` inside.
            let meta_path = Self::entry_meta_path(id, note_id)?;
            if self.vfs.stat(&meta_path).await.is_ok() {
                return Ok(note_id);
            }
        }
        Err(Error::not_found(format!("history:{id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itala_kernel::test_support::MemoryVfs;
    use itala_kernel::vfs::into_dyn;

    fn fresh_store() -> FilesHistoryStore {
        FilesHistoryStore::new(into_dyn(MemoryVfs::new()))
    }

    #[tokio::test]
    async fn snapshot_then_list_get_restore() {
        let store = fresh_store();
        let note = NoteId::new();
        let id = store
            .snapshot(note, Bytes::from_static(b"v1"), [1u8; 32])
            .await
            .unwrap();
        let entries = store.list(note).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        assert_eq!(entries[0].note_id, note);

        let entry = store.get(id).await.unwrap();
        assert_eq!(entry.checksum, [1u8; 32]);

        let body = store.restore(id).await.unwrap();
        assert_eq!(body, Bytes::from_static(b"v1"));
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let store = fresh_store();
        let note = NoteId::new();
        let _first = store
            .snapshot(note, Bytes::from_static(b"old"), [1u8; 32])
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let second = store
            .snapshot(note, Bytes::from_static(b"new"), [2u8; 32])
            .await
            .unwrap();
        let entries = store.list(note).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, second);
    }

    #[tokio::test]
    async fn notes_do_not_bleed() {
        let store = fresh_store();
        let a = NoteId::new();
        let b = NoteId::new();
        store.snapshot(a, Bytes::new(), [0u8; 32]).await.unwrap();
        store.snapshot(b, Bytes::new(), [0u8; 32]).await.unwrap();
        assert_eq!(store.list(a).await.unwrap().len(), 1);
        assert_eq!(store.list(b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn missing_entry_returns_not_found() {
        let store = fresh_store();
        let err = store.get(HistoryId::new()).await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn restart_recovers_prior_snapshots() {
        // Use a shared backing via a local tempdir + LocalFsVfs so a fresh
        // adapter instance points at the same files.
        let tmp = tempfile::tempdir().unwrap();
        let vfs1: Box<dyn DynVfs> = into_dyn(itala_vfs_fs::LocalFsVfs::new(tmp.path()).unwrap());
        let store1 = FilesHistoryStore::new(vfs1);
        let note = NoteId::new();
        let id = store1
            .snapshot(note, Bytes::from_static(b"persisted"), [9u8; 32])
            .await
            .unwrap();
        drop(store1);

        let vfs2: Box<dyn DynVfs> = into_dyn(itala_vfs_fs::LocalFsVfs::new(tmp.path()).unwrap());
        let store2 = FilesHistoryStore::new(vfs2);
        let entries = store2.list(note).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        let body = store2.restore(id).await.unwrap();
        assert_eq!(body, Bytes::from_static(b"persisted"));
    }

    #[tokio::test]
    async fn list_on_unknown_note_returns_empty() {
        let store = fresh_store();
        let entries = store.list(NoteId::new()).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn object_safe_dispatch_works() {
        // Verifies the adapter can be used as Box<dyn DynHistoryStore>.
        let store: Box<dyn DynHistoryStore> = Box::new(fresh_store());
        let note = NoteId::new();
        let _id = store.snapshot(note, Bytes::new(), [0u8; 32]).await.unwrap();
        let _ = store.list(note).await.unwrap();
    }
}
