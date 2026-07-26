//! [`MemoryHistoryStore`] — a reference in-memory implementation of the
//! [`HistoryStore`](crate::history::HistoryStore) port.
//!
//! Backed by `HashMap`s guarded by a `Mutex`. Intended for fast unit tests
//! throughout the kernel and downstream crates; the same port-conformance
//! contract as a real adapter.

use std::collections::HashMap;
use std::sync::Mutex;

use bytes::Bytes;
use time::OffsetDateTime;

use crate::history::{HistoryEntry, HistoryStore};
use crate::{Error, HistoryId, NoteId, Result};

/// In-memory history store.
pub struct MemoryHistoryStore {
    entries: Mutex<HashMap<HistoryId, HistoryEntry>>,
    bodies: Mutex<HashMap<HistoryId, Bytes>>,
}

impl Default for MemoryHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryHistoryStore {
    /// Creates an empty in-memory history store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            bodies: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the number of stored snapshots across all notes.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned (another task panicked while
    /// holding it).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.lock().expect("entries lock").len()
    }

    /// Returns `true` if there are no stored snapshots.
    ///
    /// # Panics
    /// Panics if the internal lock is poisoned (see [`Self::len`]).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl HistoryStore for MemoryHistoryStore {
    async fn snapshot(
        &self,
        note_id: NoteId,
        bytes: Bytes,
        checksum: [u8; 32],
    ) -> Result<HistoryId> {
        let id = HistoryId::new();
        let entry = HistoryEntry::new(id, note_id, OffsetDateTime::now_utc(), checksum);
        self.entries.lock().expect("entries lock").insert(id, entry);
        self.bodies.lock().expect("bodies lock").insert(id, bytes);
        Ok(id)
    }

    async fn list(&self, note_id: NoteId) -> Result<Vec<HistoryEntry>> {
        let mut out: Vec<HistoryEntry> = {
            let entries = self.entries.lock().expect("entries lock");
            entries
                .values()
                .filter(|e| e.note_id == note_id)
                .cloned()
                .collect()
        };
        out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(out)
    }

    async fn get(&self, id: HistoryId) -> Result<HistoryEntry> {
        self.entries
            .lock()
            .expect("entries lock")
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("history:{id}")))
    }

    async fn restore(&self, id: HistoryId) -> Result<Bytes> {
        self.bodies
            .lock()
            .expect("bodies lock")
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::not_found(format!("history:{id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_then_list_get_restore() {
        let store = MemoryHistoryStore::new();
        let note = NoteId::new();
        let id = store
            .snapshot(note, Bytes::from_static(b"v1"), [1u8; 32])
            .await
            .unwrap();
        let entries = store.list(note).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);

        let entry = store.get(id).await.unwrap();
        assert_eq!(entry.note_id, note);

        let body = store.restore(id).await.unwrap();
        assert_eq!(body, Bytes::from_static(b"v1"));
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let store = MemoryHistoryStore::new();
        let note = NoteId::new();
        let _first = store
            .snapshot(note, Bytes::from_static(b"old"), [1u8; 32])
            .await
            .unwrap();
        // tiny sleep so the ULID sort and timestamp both go the same way.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
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
        let store = MemoryHistoryStore::new();
        let a = NoteId::new();
        let b = NoteId::new();
        store.snapshot(a, Bytes::new(), [0u8; 32]).await.unwrap();
        store.snapshot(b, Bytes::new(), [0u8; 32]).await.unwrap();
        assert_eq!(store.list(a).await.unwrap().len(), 1);
        assert_eq!(store.list(b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn missing_entry_returns_not_found() {
        let store = MemoryHistoryStore::new();
        let err = store.get(HistoryId::new()).await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }
}
