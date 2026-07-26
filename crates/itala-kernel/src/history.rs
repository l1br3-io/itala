//! The [`HistoryStore`] port: per-note version snapshots.
//!
//! Each [`crate::NoteId`] may have an ordered sequence of [`HistoryEntry`]
//! snapshots, identified by [`HistoryId`]. The store is responsible for
//! durability; the vault context decides *when* to snapshot.
//!
//! ## Dual-trait pattern
//!
//! Mirroring [`crate::vfs::Vfs`] / [`crate::vfs::DynVfs`] (ADR-0003): a
//! native-async [`HistoryStore`] for static dispatch, plus an object-safe
//! [`DynHistoryStore`] shadow built with `#[async_trait]` for runtime
//! dispatch (used by the shell and contexts that need `Arc<dyn
//! DynHistoryStore>`). Bridge with [`into_dyn_history`].
//!
//! This port lands as part of ADR-0004 §1 (M1).

use async_trait::async_trait;
use bytes::Bytes;
use time::OffsetDateTime;

use crate::{HistoryId, NoteId, Result};

/// A single snapshot of a note at a point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HistoryEntry {
    /// The snapshot's unique identifier.
    pub id: HistoryId,
    /// The note this snapshot belongs to.
    pub note_id: NoteId,
    /// When the snapshot was recorded.
    pub timestamp: OffsetDateTime,
    /// BLAKE3 checksum of the snapshot body, for integrity and dedup.
    pub checksum: [u8; 32],
}

impl HistoryEntry {
    /// Constructs a new history entry.
    #[must_use]
    pub const fn new(
        id: HistoryId,
        note_id: NoteId,
        timestamp: OffsetDateTime,
        checksum: [u8; 32],
    ) -> Self {
        Self {
            id,
            note_id,
            timestamp,
            checksum,
        }
    }
}

// Compile-time auto-trait assertion.
const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const E: fn() = check::<HistoryEntry>;
    const _: [(); 0] = [(); {
        let _ = E;
        0
    }];
};

/// The native-async history-store port — for monomorphized/static dispatch.
///
/// Implement on a storage backend. For runtime dispatch (e.g. holding an
/// arbitrary backend behind a trait object), wrap it with
/// [`into_dyn_history`].
pub trait HistoryStore: Send + Sync {
    /// Records a snapshot of `note_id` with the given `bytes` and `checksum`,
    /// returning the new entry's identifier.
    ///
    /// `checksum` is supplied by the caller; the store treats it as opaque
    /// integrity metadata and does not recompute it.
    fn snapshot(
        &self,
        note_id: NoteId,
        bytes: Bytes,
        checksum: [u8; 32],
    ) -> impl std::future::Future<Output = Result<HistoryId>> + Send;

    /// Lists all snapshots for `note_id`, newest-first.
    fn list(
        &self,
        note_id: NoteId,
    ) -> impl std::future::Future<Output = Result<Vec<HistoryEntry>>> + Send;

    /// Returns metadata for a specific snapshot.
    fn get(&self, id: HistoryId) -> impl std::future::Future<Output = Result<HistoryEntry>> + Send;

    /// Returns the body bytes of a specific snapshot.
    fn restore(&self, id: HistoryId) -> impl std::future::Future<Output = Result<Bytes>> + Send;
}

/// Object-safe shadow of [`HistoryStore`], built with `#[async_trait]` for
/// runtime dispatch as `Box<dyn DynHistoryStore>` / `Arc<dyn DynHistoryStore>`.
///
/// Method signatures mirror [`HistoryStore`] exactly. Use
/// [`into_dyn_history`] to bridge a concrete [`HistoryStore`] into a
/// `Box<dyn DynHistoryStore>`.
#[async_trait]
pub trait DynHistoryStore: Send + Sync {
    /// Records a snapshot. See [`HistoryStore::snapshot`].
    async fn snapshot(
        &self,
        note_id: NoteId,
        bytes: Bytes,
        checksum: [u8; 32],
    ) -> Result<HistoryId>;

    /// Lists snapshots for `note_id`, newest-first. See [`HistoryStore::list`].
    async fn list(&self, note_id: NoteId) -> Result<Vec<HistoryEntry>>;

    /// Returns metadata for a snapshot. See [`HistoryStore::get`].
    async fn get(&self, id: HistoryId) -> Result<HistoryEntry>;

    /// Returns the body of a snapshot. See [`HistoryStore::restore`].
    async fn restore(&self, id: HistoryId) -> Result<Bytes>;
}

/// Adapter that bridges any [`HistoryStore`] into a [`DynHistoryStore`] object.
pub struct IntoDynHistory<H: HistoryStore> {
    inner: H,
}

/// Wraps a concrete [`HistoryStore`] into a boxed [`DynHistoryStore`] for
/// runtime dispatch.
///
/// The implementor must be `'static` (no borrowed state).
///
/// ```
/// use bytes::Bytes;
/// use itala_kernel::history::{DynHistoryStore, HistoryStore, into_dyn_history};
/// use itala_kernel::{HistoryId, NoteId, Result};
/// use time::OffsetDateTime;
///
/// struct MyStore;
/// impl HistoryStore for MyStore {
///     async fn snapshot(&self, _n: NoteId, _b: Bytes, _c: [u8; 32]) -> Result<HistoryId> {
///         Ok(HistoryId::new())
///     }
///     async fn list(&self, _n: NoteId) -> Result<Vec<itala_kernel::history::HistoryEntry>> { Ok(Vec::new()) }
///     async fn get(&self, _id: HistoryId) -> Result<itala_kernel::history::HistoryEntry> {
///         Ok(itala_kernel::history::HistoryEntry::new(
///             HistoryId::new(), NoteId::new(), OffsetDateTime::now_utc(), [0u8; 32],
///         ))
///     }
///     async fn restore(&self, _id: HistoryId) -> Result<Bytes> { Ok(Bytes::new()) }
/// }
///
/// let _dyn: Box<dyn DynHistoryStore> = into_dyn_history(MyStore);
/// ```
#[must_use]
pub fn into_dyn_history<H: HistoryStore + 'static>(store: H) -> Box<dyn DynHistoryStore> {
    Box::new(IntoDynHistory { inner: store })
}

#[async_trait]
impl<H: HistoryStore> DynHistoryStore for IntoDynHistory<H> {
    async fn snapshot(
        &self,
        note_id: NoteId,
        bytes: Bytes,
        checksum: [u8; 32],
    ) -> Result<HistoryId> {
        self.inner.snapshot(note_id, bytes, checksum).await
    }
    async fn list(&self, note_id: NoteId) -> Result<Vec<HistoryEntry>> {
        self.inner.list(note_id).await
    }
    async fn get(&self, id: HistoryId) -> Result<HistoryEntry> {
        self.inner.get(id).await
    }
    async fn restore(&self, id: HistoryId) -> Result<Bytes> {
        self.inner.restore(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_dyn_compiles() {
        fn _accepts_dyn(_: &dyn DynHistoryStore) {}
        let _ = _accepts_dyn;
    }
}
