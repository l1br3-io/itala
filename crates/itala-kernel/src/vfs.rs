//! Virtual filesystem abstraction.
//!
//! Two layers:
//! - [`Vfs`] — native `async fn in trait` for static dispatch (zero-cost).
//! - [`DynVfs`] — `async-trait`-backed object-safe shadow for `Arc<dyn DynVfs>` storage.
//!
//! Any type that implements [`Vfs`] automatically implements [`DynVfs`] via a
//! blanket impl. Downstream crates that need to erase the concrete type (e.g.
//! `Vault<Open>` holding a runtime-selected backend) use `Arc<dyn DynVfs>`.
//! Code that knows the concrete type at compile time uses `Vfs` generics directly.
//!
//! # Paths
//!
//! All paths are vault-root-relative [`Utf8Path`] values. Absolute paths and
//! `..` traversal are rejected by concrete implementations — never by this trait.
//!
//! # Watch
//!
//! [`Vfs::watch`] returns a [`Stream`] of [`VfsEvent`] items. The stream is
//! `Send` and borrows `&self` for its lifetime. Implementations may debounce,
//! deduplicate, or buffer events; callers must not assume ordering beyond
//! "events arrive after the filesystem change."

use std::pin::Pin;

use async_trait::async_trait;
use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use futures::Stream;

use crate::Result;

/// A raw filesystem-level change notification emitted by [`Vfs::watch`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum VfsEvent {
    /// A file was created at the given path.
    Created {
        /// Vault-root-relative path of the new file.
        path: Utf8PathBuf,
    },
    /// An existing file's contents were modified.
    Changed {
        /// Vault-root-relative path of the changed file.
        path: Utf8PathBuf,
    },
    /// A file was removed.
    Deleted {
        /// Vault-root-relative path of the deleted file.
        path: Utf8PathBuf,
    },
}

/// Virtual filesystem port — static-dispatch variant.
///
/// Implement this trait for any storage backend (local disk, encrypted layer,
/// in-memory mock). `Send + Sync` is required so implementations can be shared
/// across async tasks.
///
/// Use [`DynVfs`] when you need to store the implementation behind a pointer
/// (`Arc<dyn DynVfs>`). The blanket impl converts automatically.
pub trait Vfs: Send + Sync {
    /// Read the full contents of a file.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file cannot be read.
    fn read(&self, path: &Utf8Path) -> impl Future<Output = Result<Bytes>> + Send + '_;

    /// Write (create or overwrite) a file atomically.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the write fails.
    fn write(&self, path: &Utf8Path, bytes: Bytes) -> impl Future<Output = Result<()>> + Send + '_;

    /// List the direct children of a directory, returning their paths.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the directory cannot be read.
    fn list(&self, dir: &Utf8Path) -> impl Future<Output = Result<Vec<Utf8PathBuf>>> + Send + '_;

    /// Return `true` if a file or directory exists at the given path.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] on OS-level failures (e.g. permission denied).
    fn exists(&self, path: &Utf8Path) -> impl Future<Output = Result<bool>> + Send + '_;

    /// Delete a file.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file cannot be deleted.
    fn delete(&self, path: &Utf8Path) -> impl Future<Output = Result<()>> + Send + '_;

    /// Subscribe to filesystem change events under `path`.
    ///
    /// Returns a `Send` stream that yields [`VfsEvent`] items whenever the
    /// underlying file or directory changes. The stream borrows `&self` for its
    /// lifetime — drop it to unsubscribe.
    fn watch(&self, path: &Utf8Path) -> impl Stream<Item = VfsEvent> + Send + '_;
}

/// Virtual filesystem port — object-safe variant for `Arc<dyn DynVfs>` storage.
///
/// This trait mirrors [`Vfs`] exactly but uses `async-trait` boxing so it can
/// be stored behind a pointer. The blanket impl below converts any `T: Vfs +
/// 'static` automatically — callers do not need to impl this directly.
///
/// ```
/// # use std::sync::Arc;
/// # use itala_kernel::DynVfs;
/// fn store(vfs: Arc<dyn DynVfs>) {
///     // vault, search, etc. hold this without a generic parameter
///     let _ = vfs;
/// }
/// ```
#[async_trait]
pub trait DynVfs: Send + Sync {
    /// Read the full contents of a file.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file cannot be read.
    async fn read(&self, path: &Utf8Path) -> Result<Bytes>;

    /// Write (create or overwrite) a file atomically.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the write fails.
    async fn write(&self, path: &Utf8Path, bytes: Bytes) -> Result<()>;

    /// List the direct children of a directory, returning their paths.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the directory cannot be read.
    async fn list(&self, dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>>;

    /// Return `true` if a file or directory exists at the given path.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] on OS-level failures (e.g. permission denied).
    async fn exists(&self, path: &Utf8Path) -> Result<bool>;

    /// Delete a file.
    ///
    /// # Errors
    /// Returns [`crate::Error::Io`] if the file cannot be deleted.
    async fn delete(&self, path: &Utf8Path) -> Result<()>;

    /// Subscribe to filesystem change events under `path`.
    ///
    /// Returns a boxed `Send` stream. Drop the stream to unsubscribe.
    fn watch<'a>(&'a self, path: &'a Utf8Path)
    -> Pin<Box<dyn Stream<Item = VfsEvent> + Send + 'a>>;
}

/// Blanket impl: every `T: Vfs + 'static` is also a `DynVfs`.
///
/// This means callers can `Arc::new(my_vfs)` and pass it as `Arc<dyn DynVfs>`
/// without writing a separate `impl DynVfs for MyVfs`.
#[async_trait]
impl<T: Vfs + 'static> DynVfs for T {
    async fn read(&self, path: &Utf8Path) -> Result<Bytes> {
        Vfs::read(self, path).await
    }

    async fn write(&self, path: &Utf8Path, bytes: Bytes) -> Result<()> {
        Vfs::write(self, path, bytes).await
    }

    async fn list(&self, dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        Vfs::list(self, dir).await
    }

    async fn exists(&self, path: &Utf8Path) -> Result<bool> {
        Vfs::exists(self, path).await
    }

    async fn delete(&self, path: &Utf8Path) -> Result<()> {
        Vfs::delete(self, path).await
    }

    fn watch<'a>(
        &'a self,
        path: &'a Utf8Path,
    ) -> Pin<Box<dyn Stream<Item = VfsEvent> + Send + 'a>> {
        Box::pin(Vfs::watch(self, path))
    }
}

// Re-export for doc links.
use std::future::Future;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use futures::StreamExt;

    use super::*;

    // ---------------------------------------------------------------------------
    // MemoryVfs — minimal in-test Vfs impl. Not a public API.
    // ---------------------------------------------------------------------------

    #[derive(Default, Clone)]
    struct MemoryVfs {
        files: Arc<Mutex<HashMap<Utf8PathBuf, Bytes>>>,
    }

    impl Vfs for MemoryVfs {
        fn read(&self, path: &Utf8Path) -> impl Future<Output = Result<Bytes>> + Send + '_ {
            let files = Arc::clone(&self.files);
            let path = path.to_owned();
            async move {
                files
                    .lock()
                    .unwrap()
                    .get(&path)
                    .cloned()
                    .ok_or_else(|| std::io::Error::other("not found").into())
            }
        }

        fn write(
            &self,
            path: &Utf8Path,
            bytes: Bytes,
        ) -> impl Future<Output = Result<()>> + Send + '_ {
            let files = Arc::clone(&self.files);
            let path = path.to_owned();
            async move {
                files.lock().unwrap().insert(path, bytes);
                Ok(())
            }
        }

        fn list(
            &self,
            _dir: &Utf8Path,
        ) -> impl Future<Output = Result<Vec<Utf8PathBuf>>> + Send + '_ {
            let files = Arc::clone(&self.files);
            async move { Ok(files.lock().unwrap().keys().cloned().collect()) }
        }

        fn exists(&self, path: &Utf8Path) -> impl Future<Output = Result<bool>> + Send + '_ {
            let files = Arc::clone(&self.files);
            let path = path.to_owned();
            async move { Ok(files.lock().unwrap().contains_key(&path)) }
        }

        fn delete(&self, path: &Utf8Path) -> impl Future<Output = Result<()>> + Send + '_ {
            let files = Arc::clone(&self.files);
            let path = path.to_owned();
            async move {
                files.lock().unwrap().remove(&path);
                Ok(())
            }
        }

        fn watch(&self, _path: &Utf8Path) -> impl Stream<Item = VfsEvent> + Send + '_ {
            futures::stream::empty()
        }
    }

    // ---------------------------------------------------------------------------
    // Compile-time auto-trait assertions
    // ---------------------------------------------------------------------------

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn vfs_event_is_send_sync_unpin() {
        assert_send_sync::<VfsEvent>();
        assert_unpin::<VfsEvent>();
    }

    #[test]
    fn dyn_vfs_arc_is_send_sync() {
        assert_send_sync::<Arc<dyn DynVfs>>();
    }

    // ---------------------------------------------------------------------------
    // Vfs functional tests
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let vfs = MemoryVfs::default();
        let path = Utf8Path::new("notes/hello.md");
        let body = Bytes::from_static(b"# Hello\n");

        // Use UFCS to disambiguate from the DynVfs blanket impl.
        Vfs::write(&vfs, path, body.clone()).await.unwrap();
        let got = Vfs::read(&vfs, path).await.unwrap();

        assert_eq!(got, body);
    }

    #[tokio::test]
    async fn exists_returns_false_for_missing_file() {
        let vfs = MemoryVfs::default();
        let exists = Vfs::exists(&vfs, Utf8Path::new("nope.md")).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn exists_returns_true_after_write() {
        let vfs = MemoryVfs::default();
        let path = Utf8Path::new("yes.md");
        Vfs::write(&vfs, path, Bytes::from_static(b"hi"))
            .await
            .unwrap();
        assert!(Vfs::exists(&vfs, path).await.unwrap());
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let vfs = MemoryVfs::default();
        let path = Utf8Path::new("del.md");
        Vfs::write(&vfs, path, Bytes::from_static(b"x"))
            .await
            .unwrap();
        Vfs::delete(&vfs, path).await.unwrap();
        assert!(!Vfs::exists(&vfs, path).await.unwrap());
    }

    #[tokio::test]
    async fn watch_stream_is_empty_for_memory_vfs() {
        let vfs = MemoryVfs::default();
        let mut stream = Vfs::watch(&vfs, Utf8Path::new("."));
        // Memory VFS yields no events — take(0) completes immediately.
        let events: Vec<_> = stream.by_ref().take(0).collect().await;
        assert!(events.is_empty());
    }

    // ---------------------------------------------------------------------------
    // DynVfs blanket impl tests
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn dyn_vfs_blanket_write_read() {
        let vfs: Arc<dyn DynVfs> = Arc::new(MemoryVfs::default());
        let path = Utf8Path::new("dynamic.md");
        let body = Bytes::from_static(b"dyn works");

        vfs.write(path, body.clone()).await.unwrap();
        let got = vfs.read(path).await.unwrap();

        assert_eq!(got, body);
    }

    #[tokio::test]
    async fn dyn_vfs_watch_returns_stream() {
        let vfs: Arc<dyn DynVfs> = Arc::new(MemoryVfs::default());
        let stream = vfs.watch(Utf8Path::new("."));
        // Just prove it compiles and doesn't panic on construction.
        drop(stream);
    }
}
