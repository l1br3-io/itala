//! The VFS port: the storage abstraction every backend implements.
//!
//! Two traits model the port:
//!
//! - [`Vfs`] — native `async fn` in trait, for monomorphized/static-dispatch
//!   call sites. *Not* object-safe.
//! - [`DynVfs`] — the object-safe shadow built with `#[async_trait]`, used as
//!   `Box<dyn DynVfs>` by the plugin host and contexts that need runtime
//!   dispatch over interchangeable backends.
//!
//! This dual-trait design is [AD-3](../../docs/plans/m0-foundation.md#ad-3-vfs-object-safety-via-async-trait):
//! native async stays available where it's fastest; the dyn shadow is the
//! runtime-dispatch path.
//!
//! A backend that implements [`Vfs`] is bridged into a `Box<dyn DynVfs>` via
//! [`into_dyn`]:
//!
//! ```
//! use bytes::Bytes;
//! use futures::stream::StreamExt;
//! use itala_kernel::vfs::{into_dyn, DynVfs, Vfs};
//! use itala_kernel::vfs_types::VfsPath;
//! # use itala_kernel::Result;
//! # use futures::stream::BoxStream;
//!
//! struct MyVfs;
//! impl Vfs for MyVfs {
//!     async fn read(&self, _path: &VfsPath) -> Result<Bytes> { Ok(Bytes::new()) }
//!     async fn write(&self, _path: &VfsPath, _data: Bytes) -> Result<()> { Ok(()) }
//!     async fn list(&self, _dir: &VfsPath) -> Result<Vec<itala_kernel::vfs_types::DirEntry>> { Ok(Vec::new()) }
//!     async fn stat(&self, _path: &VfsPath) -> Result<itala_kernel::vfs_types::Metadata> {
//!         Ok(itala_kernel::vfs_types::Metadata::new(0, false, None))
//!     }
//!     async fn rename(&self, _from: &VfsPath, _to: &VfsPath) -> Result<()> { Ok(()) }
//!     async fn remove(&self, _path: &VfsPath) -> Result<()> { Ok(()) }
//!     fn watch(&self, _dir: &VfsPath) -> BoxStream<'static, itala_kernel::vfs_types::VfsEvent> {
//!         futures::stream::empty().boxed()
//!     }
//! }
//!
//! let vfs: Box<dyn DynVfs> = into_dyn(MyVfs);
//! ```

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;

use crate::Result;
use crate::vfs_types::{DirEntry, Metadata, VfsEvent, VfsPath};

/// The VFS port — native async, for static dispatch.
///
/// Implement this on a storage backend. For runtime dispatch (e.g. holding an
/// arbitrary backend behind a trait object), wrap it with [`into_dyn`].
pub trait Vfs: Send + Sync {
    /// Reads the entire contents of `path`.
    ///
    /// # Errors
    /// [`Error::NotFound`](crate::Error::NotFound) if the path does not exist,
    /// or [`Error::Io`](crate::Error) for read failures.
    fn read(&self, path: &VfsPath) -> impl std::future::Future<Output = Result<Bytes>> + Send;

    /// Writes `data` to `path`, replacing any existing contents and creating
    /// parent directories as needed.
    fn write(
        &self,
        path: &VfsPath,
        data: Bytes,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Lists the entries directly within `dir` (non-recursive).
    fn list(
        &self,
        dir: &VfsPath,
    ) -> impl std::future::Future<Output = Result<Vec<DirEntry>>> + Send;

    /// Returns metadata for `path`.
    fn stat(&self, path: &VfsPath) -> impl std::future::Future<Output = Result<Metadata>> + Send;

    /// Renames/moves `from` to `to`, creating parent directories as needed.
    fn rename(
        &self,
        from: &VfsPath,
        to: &VfsPath,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Removes `path` (file or empty directory).
    fn remove(&self, path: &VfsPath) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Returns a reactive stream of external changes under `dir`.
    fn watch(&self, dir: &VfsPath) -> BoxStream<'static, VfsEvent>;
}

/// Object-safe shadow of [`Vfs`], built with `#[async_trait]` for runtime
/// dispatch as `Box<dyn DynVfs>`.
///
/// Method signatures mirror [`Vfs`] exactly. Use [`into_dyn`] to bridge a
/// concrete [`Vfs`] into a `Box<dyn DynVfs>`.
#[async_trait]
pub trait DynVfs: Send + Sync {
    /// Reads the entire contents of `path`. See [`Vfs::read`].
    async fn read(&self, path: &VfsPath) -> Result<Bytes>;

    /// Writes `data` to `path`. See [`Vfs::write`].
    async fn write(&self, path: &VfsPath, data: Bytes) -> Result<()>;

    /// Lists entries directly within `dir`. See [`Vfs::list`].
    async fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>>;

    /// Returns metadata for `path`. See [`Vfs::stat`].
    async fn stat(&self, path: &VfsPath) -> Result<Metadata>;

    /// Renames `from` to `to`. See [`Vfs::rename`].
    async fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<()>;

    /// Removes `path`. See [`Vfs::remove`].
    async fn remove(&self, path: &VfsPath) -> Result<()>;

    /// Returns a reactive stream of changes under `dir`. See [`Vfs::watch`].
    fn watch(&self, dir: &VfsPath) -> BoxStream<'static, VfsEvent>;
}

/// Adapter that bridges any [`Vfs`] into a [`DynVfs`] object.
pub struct IntoDyn<V: Vfs> {
    inner: V,
}

/// Wraps a concrete [`Vfs`] into a boxed [`DynVfs`] for runtime dispatch.
///
/// The implementor must be `'static` (no borrowed state).
#[must_use]
pub fn into_dyn<V: Vfs + 'static>(vfs: V) -> Box<dyn DynVfs> {
    Box::new(IntoDyn { inner: vfs })
}

#[async_trait]
impl<V: Vfs> DynVfs for IntoDyn<V> {
    async fn read(&self, path: &VfsPath) -> Result<Bytes> {
        self.inner.read(path).await
    }
    async fn write(&self, path: &VfsPath, data: Bytes) -> Result<()> {
        self.inner.write(path, data).await
    }
    async fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        self.inner.list(dir).await
    }
    async fn stat(&self, path: &VfsPath) -> Result<Metadata> {
        self.inner.stat(path).await
    }
    async fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<()> {
        self.inner.rename(from, to).await
    }
    async fn remove(&self, path: &VfsPath) -> Result<()> {
        self.inner.remove(path).await
    }
    fn watch(&self, dir: &VfsPath) -> BoxStream<'static, VfsEvent> {
        self.inner.watch(dir)
    }
}

// `Box<dyn DynVfs>` is inherently Send + Sync because the trait requires those
// bounds; `IntoDyn<V>` mirrors them through its `Vfs` supertrait bound, so no
// separate compile-time assertion is needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_dyn_compiles() {
        // Compile-only check that `Box<dyn DynVfs>` is constructible.
        fn _accepts_dyn(_: &dyn DynVfs) {}
        let _ = _accepts_dyn;
    }
}
