//! Storage-provider port — the single swappable seam for all persistence.
//!
//! Two layers:
//! - [`StorageProvider`] — native `async fn in trait` for static dispatch (zero-cost).
//! - [`DynStorageProvider`] — `async-trait`-backed object-safe shadow for
//!   `Arc<dyn DynStorageProvider>` storage.
//!
//! Any type that implements [`StorageProvider`] automatically implements
//! [`DynStorageProvider`] via a blanket impl. Downstream crates that need to
//! erase the concrete type (e.g. `NoteStore` holding a runtime-selected backend)
//! use `Arc<dyn DynStorageProvider>`.
//!
//! # Collections and keys
//!
//! All records live in named collections (e.g. `"notes"`, `"config"`).
//! Keys are arbitrary strings; the mapping from domain IDs to collection keys is
//! the responsibility of the domain store layer (e.g. `NoteStore`), not the
//! provider.
//!
//! # Optional capabilities
//!
//! [`StorageProvider::query`] and [`StorageProvider::transaction`] are optional.
//! Providers that do not support them return a [`crate::Error::Provider`] error
//! from the default implementations.  Callers should check
//! [`ProviderCapabilities::native_query`] / [`ProviderCapabilities::transactions`]
//! before calling these methods, and fall back to host-side logic when unsupported.
//!
//! ```
//! # use std::sync::Arc;
//! # use itala_kernel::DynStorageProvider;
//! fn store(provider: Arc<dyn DynStorageProvider>) {
//!     // NoteStore, ConfigStore, etc. hold this without a generic parameter.
//!     let _ = provider;
//! }
//! ```

use std::collections::BTreeMap;
use std::future::Future;

use async_trait::async_trait;
use bytes::Bytes;
use camino::Utf8PathBuf;
use serde_json::Value;

use crate::Result;

/// An individual record stored in a collection.
///
/// `blob` is the primary storage unit (opaque bytes).  `indexed` holds
/// queryable fields extracted from the blob by the domain layer; providers
/// that advertise [`ProviderCapabilities::native_query`] may index these
/// fields for efficient lookup.
#[derive(Clone, Debug)]
pub struct Record {
    /// The record's key within its collection.
    pub key: String,
    /// Queryable fields extracted from the payload (e.g. `title`, `tags`).
    ///
    /// Values are [`serde_json::Value`] so any backend can serialise them
    /// without depending on domain types.
    pub indexed: BTreeMap<String, Value>,
    /// The full opaque record payload.
    pub blob: Bytes,
}

/// Capability flags advertised by a provider.
///
/// Callers check these before using optional methods like
/// [`DynStorageProvider::query`] and [`DynStorageProvider::transaction`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ProviderCapabilities {
    /// `true` when the provider can execute [`StorageProvider::query`]
    /// natively (e.g. Postgres JSONB + GIN index).  If `false`, the caller
    /// must implement a host-side scan-and-filter fallback.
    pub native_query: bool,
    /// `true` when the provider can execute [`StorageProvider::transaction`]
    /// atomically.  If `false`, callers must not rely on all-or-nothing
    /// semantics.
    pub transactions: bool,
}

/// A predicate applied to the [`Record::indexed`] map.
///
/// All conditions are combined with AND semantics.
#[derive(Clone, Debug)]
pub struct Filter {
    /// Every condition in this list must match.
    pub conditions: Vec<Condition>,
}

/// A single predicate on a named indexed field.
#[derive(Clone, Debug)]
pub struct Condition {
    /// Name of the [`Record::indexed`] field to test.
    pub field: String,
    /// The comparison operation.
    pub op: MatchOp,
    /// The value to compare against (as a string).
    pub value: String,
}

/// Comparison operator for a [`Condition`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MatchOp {
    /// Exact string equality (case-sensitive).
    Eq,
    /// Case-insensitive substring containment.
    Substring,
}

/// A write operation for use with [`StorageProvider::transaction`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum WriteOp {
    /// Write or overwrite a record.
    Put {
        /// Target collection name.
        collection: String,
        /// The record to write.
        record: Record,
    },
    /// Remove a record.
    Delete {
        /// Target collection name.
        collection: String,
        /// Key of the record to remove.
        key: String,
    },
}

/// Storage-provider port — static-dispatch variant.
///
/// Implement this trait for any storage backend (local files, Postgres,
/// `SQLite`, …). `Send + Sync` is required so implementations can be shared
/// across async tasks.
///
/// Use [`DynStorageProvider`] when you need to store the implementation behind
/// a pointer (`Arc<dyn DynStorageProvider>`). The blanket impl converts any
/// `T: StorageProvider + 'static` automatically.
pub trait StorageProvider: Send + Sync {
    /// Return the capabilities advertised by this provider.
    #[must_use]
    fn capabilities(&self) -> ProviderCapabilities;

    /// Read a record by collection and key.
    ///
    /// Returns `Ok(None)` when the key does not exist.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    fn get(
        &self,
        collection: &str,
        key: &str,
    ) -> impl Future<Output = Result<Option<Record>>> + Send + '_;

    /// Write (create or overwrite) a record.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    fn put(&self, collection: &str, record: Record)
    -> impl Future<Output = Result<()>> + Send + '_;

    /// Delete a record by collection and key.
    ///
    /// Deleting a non-existent key succeeds silently.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    fn delete(&self, collection: &str, key: &str) -> impl Future<Output = Result<()>> + Send + '_;

    /// List all keys in a collection.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    fn list(&self, collection: &str) -> impl Future<Output = Result<Vec<String>>> + Send + '_;

    /// Return the filesystem path for a record, if this provider is FS-backed.
    ///
    /// The default implementation returns `Ok(None)`.  `FileProvider` overrides
    /// this to expose vault-root-relative paths; other backends return `None`.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the scan fails.
    fn fs_location(
        &self,
        _collection: &str,
        _key: &str,
    ) -> impl Future<Output = Result<Option<Utf8PathBuf>>> + Send + '_ {
        std::future::ready(Ok(None))
    }

    /// Query records whose [`Record::indexed`] fields satisfy a [`Filter`].
    ///
    /// Optional — check [`ProviderCapabilities::native_query`] before calling.
    /// The default returns a [`crate::Error::Provider`] "not supported" error.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] when unsupported or when the backend
    /// fails.
    fn query(
        &self,
        _collection: &str,
        _filter: &Filter,
    ) -> impl Future<Output = Result<Vec<Record>>> + Send + '_ {
        std::future::ready(Err(provider_unsupported("query")))
    }

    /// Apply a batch of writes as a single atomic operation.
    ///
    /// Optional — check [`ProviderCapabilities::transactions`] before calling.
    /// The default returns a [`crate::Error::Provider`] "not supported" error.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] when unsupported or when the backend
    /// fails.
    fn transaction(&self, _ops: Vec<WriteOp>) -> impl Future<Output = Result<()>> + Send + '_ {
        std::future::ready(Err(provider_unsupported("transaction")))
    }
}

/// Storage-provider port — object-safe variant for `Arc<dyn DynStorageProvider>`.
///
/// This trait mirrors [`StorageProvider`] exactly but uses `async-trait` boxing
/// so it can be stored behind a pointer.  The blanket impl below converts any
/// `T: StorageProvider + 'static` automatically — callers do not need to impl
/// this directly.
///
/// ```
/// # use std::sync::Arc;
/// # use itala_kernel::DynStorageProvider;
/// fn store(provider: Arc<dyn DynStorageProvider>) {
///     // NoteStore, ConfigStore, etc. hold this without a generic parameter.
///     let _ = provider;
/// }
/// ```
#[async_trait]
pub trait DynStorageProvider: Send + Sync {
    /// Return the capabilities advertised by this provider.
    #[must_use]
    fn capabilities(&self) -> ProviderCapabilities;

    /// Read a record by collection and key.
    ///
    /// Returns `Ok(None)` when the key does not exist.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    async fn get(&self, collection: &str, key: &str) -> Result<Option<Record>>;

    /// Write (create or overwrite) a record.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    async fn put(&self, collection: &str, record: Record) -> Result<()>;

    /// Delete a record by collection and key.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    async fn delete(&self, collection: &str, key: &str) -> Result<()>;

    /// List all keys in a collection.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the backend fails.
    async fn list(&self, collection: &str) -> Result<Vec<String>>;

    /// Return the filesystem path for a record, if this provider is FS-backed.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] if the scan fails.
    async fn fs_location(&self, collection: &str, key: &str) -> Result<Option<Utf8PathBuf>>;

    /// Query records whose indexed fields satisfy a filter.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] when unsupported or if the backend
    /// fails.
    async fn query(&self, collection: &str, filter: &Filter) -> Result<Vec<Record>>;

    /// Apply a batch of writes atomically.
    ///
    /// # Errors
    /// Returns [`crate::Error::Provider`] when unsupported or if the backend
    /// fails.
    async fn transaction(&self, ops: Vec<WriteOp>) -> Result<()>;
}

/// Blanket impl: every `T: StorageProvider + 'static` is also a
/// `DynStorageProvider`.
///
/// This means callers can `Arc::new(my_provider)` and pass it as
/// `Arc<dyn DynStorageProvider>` without writing a separate impl.
#[async_trait]
impl<T: StorageProvider + 'static> DynStorageProvider for T {
    fn capabilities(&self) -> ProviderCapabilities {
        StorageProvider::capabilities(self)
    }

    async fn get(&self, collection: &str, key: &str) -> Result<Option<Record>> {
        StorageProvider::get(self, collection, key).await
    }

    async fn put(&self, collection: &str, record: Record) -> Result<()> {
        StorageProvider::put(self, collection, record).await
    }

    async fn delete(&self, collection: &str, key: &str) -> Result<()> {
        StorageProvider::delete(self, collection, key).await
    }

    async fn list(&self, collection: &str) -> Result<Vec<String>> {
        StorageProvider::list(self, collection).await
    }

    async fn fs_location(&self, collection: &str, key: &str) -> Result<Option<Utf8PathBuf>> {
        StorageProvider::fs_location(self, collection, key).await
    }

    async fn query(&self, collection: &str, filter: &Filter) -> Result<Vec<Record>> {
        StorageProvider::query(self, collection, filter).await
    }

    async fn transaction(&self, ops: Vec<WriteOp>) -> Result<()> {
        StorageProvider::transaction(self, ops).await
    }
}

/// Construct a "not supported" provider error for the given operation name.
fn provider_unsupported(op: &'static str) -> crate::Error {
    crate::Error::Provider(Box::new(std::io::Error::other(format!(
        "storage provider operation not supported: {op}"
    ))))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::*;

    // -----------------------------------------------------------------------
    // MemoryStorageProvider — minimal in-test StorageProvider impl.
    // Mirrors MemoryVfs from vfs.rs.
    // -----------------------------------------------------------------------

    #[derive(Default, Clone)]
    struct MemoryStorageProvider {
        records: Arc<Mutex<HashMap<(String, String), Record>>>,
    }

    impl StorageProvider for MemoryStorageProvider {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn get(
            &self,
            collection: &str,
            key: &str,
        ) -> impl Future<Output = Result<Option<Record>>> + Send + '_ {
            let records = Arc::clone(&self.records);
            let col = collection.to_owned();
            let k = key.to_owned();
            async move { Ok(records.lock().unwrap().get(&(col, k)).cloned()) }
        }

        fn put(
            &self,
            collection: &str,
            record: Record,
        ) -> impl Future<Output = Result<()>> + Send + '_ {
            let records = Arc::clone(&self.records);
            let col = collection.to_owned();
            async move {
                let key = record.key.clone();
                records.lock().unwrap().insert((col, key), record);
                Ok(())
            }
        }

        fn delete(
            &self,
            collection: &str,
            key: &str,
        ) -> impl Future<Output = Result<()>> + Send + '_ {
            let records = Arc::clone(&self.records);
            let col = collection.to_owned();
            let k = key.to_owned();
            async move {
                records.lock().unwrap().remove(&(col, k));
                Ok(())
            }
        }

        fn list(&self, collection: &str) -> impl Future<Output = Result<Vec<String>>> + Send + '_ {
            let records = Arc::clone(&self.records);
            let col = collection.to_owned();
            async move {
                let keys = records
                    .lock()
                    .unwrap()
                    .keys()
                    .filter(|(c, _)| *c == col)
                    .map(|(_, k)| k.clone())
                    .collect();
                Ok(keys)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Compile-time auto-trait assertions
    // -----------------------------------------------------------------------

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn record_is_send_sync_unpin() {
        assert_send_sync::<Record>();
        assert_unpin::<Record>();
    }

    #[test]
    fn provider_capabilities_is_send_sync_unpin() {
        assert_send_sync::<ProviderCapabilities>();
        assert_unpin::<ProviderCapabilities>();
    }

    #[test]
    fn filter_is_send_sync_unpin() {
        assert_send_sync::<Filter>();
        assert_unpin::<Filter>();
    }

    #[test]
    fn write_op_is_send_sync_unpin() {
        assert_send_sync::<WriteOp>();
        assert_unpin::<WriteOp>();
    }

    #[test]
    fn dyn_storage_provider_arc_is_send_sync() {
        assert_send_sync::<Arc<dyn DynStorageProvider>>();
    }

    // -----------------------------------------------------------------------
    // Functional tests — all through Arc<dyn DynStorageProvider>
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn put_then_get_round_trip() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let record = Record {
            key: "k1".to_owned(),
            indexed: BTreeMap::new(),
            blob: Bytes::from_static(b"hello"),
        };
        provider.put("notes", record.clone()).await.unwrap();
        let got = provider
            .get("notes", "k1")
            .await
            .unwrap()
            .expect("record must exist after put");
        assert_eq!(got.key, "k1");
        assert_eq!(got.blob, Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let got = provider.get("notes", "no-such-key").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn list_returns_keys_in_collection() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        for i in 0..3_u8 {
            provider
                .put(
                    "notes",
                    Record {
                        key: format!("key-{i}"),
                        indexed: BTreeMap::new(),
                        blob: Bytes::from_static(b"x"),
                    },
                )
                .await
                .unwrap();
        }
        let mut keys = provider.list("notes").await.unwrap();
        keys.sort();
        assert_eq!(keys, ["key-0", "key-1", "key-2"]);
    }

    #[tokio::test]
    async fn delete_removes_record() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        provider
            .put(
                "notes",
                Record {
                    key: "del-me".to_owned(),
                    indexed: BTreeMap::new(),
                    blob: Bytes::from_static(b"bye"),
                },
            )
            .await
            .unwrap();
        provider.delete("notes", "del-me").await.unwrap();
        let got = provider.get("notes", "del-me").await.unwrap();
        assert!(got.is_none(), "record must be gone after delete");
    }

    #[tokio::test]
    async fn fs_location_returns_none_by_default() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let loc = provider.fs_location("notes", "any-key").await.unwrap();
        assert!(loc.is_none(), "default fs_location must return None");
    }

    #[tokio::test]
    async fn query_returns_unsupported_by_default() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let filter = Filter { conditions: vec![] };
        let err = provider.query("notes", &filter).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::Provider(_)),
            "default query must return Provider error, got: {err}"
        );
    }

    #[tokio::test]
    async fn transaction_returns_unsupported_by_default() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let err = provider.transaction(vec![]).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::Provider(_)),
            "default transaction must return Provider error, got: {err}"
        );
    }

    #[tokio::test]
    async fn indexed_fields_survive_round_trip() {
        let provider: Arc<dyn DynStorageProvider> = Arc::new(MemoryStorageProvider::default());
        let mut indexed = BTreeMap::new();
        indexed.insert("title".to_owned(), serde_json::json!("My Note"));
        indexed.insert("tags".to_owned(), serde_json::json!(["rust", "kernel"]));
        provider
            .put(
                "notes",
                Record {
                    key: "note-1".to_owned(),
                    indexed: indexed.clone(),
                    blob: Bytes::from_static(b"body"),
                },
            )
            .await
            .unwrap();
        let got = provider
            .get("notes", "note-1")
            .await
            .unwrap()
            .expect("record must exist");
        assert_eq!(got.indexed["title"], serde_json::json!("My Note"));
        assert_eq!(got.indexed["tags"], serde_json::json!(["rust", "kernel"]));
    }
}
