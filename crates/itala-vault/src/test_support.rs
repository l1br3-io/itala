//! Test helpers: a `MemoryVfs`-backed vault builder for fast in-process fixtures.
//!
//! Gated behind `#[cfg(any(test, feature = "test-support"))]` so it never ships
//! in production binaries. Enable the `test-support` feature in dev-dependencies
//! if downstream crates need this.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use futures::Stream;
use itala_kernel::{DynVfs, Result, Vfs, VfsEvent};

use crate::vault::{Open, Vault};

/// Minimal in-memory VFS for tests.
///
/// Duplicated from `itala_kernel::vfs::tests::MemoryVfs` — the kernel's copy is
/// private to its test module per `rules/rust/api-design.md`. We accept the small
/// copy so neither crate exposes a test fixture as public API.
#[derive(Clone, Default)]
pub struct MemoryVfs {
    files: Arc<Mutex<HashMap<Utf8PathBuf, Bytes>>>,
}

impl MemoryVfs {
    /// Insert a file at `path` with the given `bytes` (copied).
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned (only possible after a panic in another thread).
    pub fn put(&self, path: impl Into<Utf8PathBuf>, bytes: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), Bytes::copy_from_slice(bytes));
    }
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

    fn write(&self, path: &Utf8Path, bytes: Bytes) -> impl Future<Output = Result<()>> + Send + '_ {
        let files = Arc::clone(&self.files);
        let path = path.to_owned();
        async move {
            files.lock().unwrap().insert(path, bytes);
            Ok(())
        }
    }

    fn list(&self, dir: &Utf8Path) -> impl Future<Output = Result<Vec<Utf8PathBuf>>> + Send + '_ {
        let files = Arc::clone(&self.files);
        let prefix = format!("{dir}/");
        async move {
            let mut children: Vec<Utf8PathBuf> = {
                let guard = files.lock().unwrap();
                guard
                    .keys()
                    .filter_map(|p| {
                        let rest = p.as_str().strip_prefix(&prefix)?;
                        // Direct children only: no additional '/' in the remaining segment.
                        if rest.is_empty() || rest.contains('/') {
                            None
                        } else {
                            Some(p.clone())
                        }
                    })
                    .collect()
            };
            children.sort();
            Ok(children)
        }
    }

    fn exists(&self, path: &Utf8Path) -> impl Future<Output = Result<bool>> + Send + '_ {
        let files = Arc::clone(&self.files);
        let path = path.to_owned();
        async move {
            // Exists as a file or as a directory prefix (any file under path/).
            let prefix = format!("{path}/");
            let found = {
                let guard = files.lock().unwrap();
                guard.contains_key(&path) || guard.keys().any(|k| k.as_str().starts_with(&prefix))
            };
            Ok(found)
        }
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

/// Build an open vault backed by a [`MemoryVfs`].
///
/// `notes` is a slice of `(relative_path, raw_bytes)` pairs. The root is
/// `/vault`. The function panics if the vault cannot be opened (i.e. all file
/// inserts succeed).
///
/// # Panics
/// Panics if the vault root does not register as existing (internal logic error).
pub async fn memory_vault(notes: &[(Utf8PathBuf, &[u8])]) -> Vault<Open> {
    let vfs = MemoryVfs::default();
    // `exists("/vault")` in MemoryVfs returns true when any file path starts with
    // "/vault/", so no sentinel key is needed — inserting the notes is sufficient.
    let root = Utf8PathBuf::from("/vault");
    for (rel, bytes) in notes {
        let abs = root.join(rel);
        vfs.put(abs, bytes);
    }
    let arc: Arc<dyn DynVfs> = Arc::new(vfs);
    Vault::new(arc, root)
        .open()
        .await
        .expect("memory_vault: open should always succeed")
}
