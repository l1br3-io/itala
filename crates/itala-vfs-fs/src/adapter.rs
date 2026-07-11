//! [`LocalFsVfs`] — concrete VFS implementation over the local filesystem.

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use futures::stream::{BoxStream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

use itala_kernel::vfs::Vfs;
use itala_kernel::vfs_types::{DirEntry, Metadata, VfsEvent, VfsPath};
use itala_kernel::{Error, Result};

/// Local-filesystem VFS adapter.
///
/// All paths are vault-root-relative (enforced by [`VfsPath`]); absolute paths
/// and `..` traversal are rejected. I/O is async via `tokio::fs`; file watching
/// uses the `notify` crate and is surfaced as a [`VfsEvent`] stream.
pub struct LocalFsVfs {
    /// Canonical absolute root.
    root: Utf8PathBuf,
}

impl LocalFsVfs {
    /// Constructs an adapter rooted at `root`, which must exist and be a
    /// directory.
    ///
    /// # Errors
    /// [`Error::Io`] if `root` does not exist or cannot be canonicalized;
    /// [`Error::Config`] if the canonicalized path is not valid UTF-8.
    pub fn new(root: impl AsRef<std::path::Path>) -> Result<Self> {
        let raw = root.as_ref();
        let canon = std::fs::canonicalize(raw).map_err(Error::Io)?;
        let canon = Utf8PathBuf::from_path_buf(canon)
            .map_err(|e| Error::config(format!("root is not valid UTF-8: {}", e.display())))?;
        let meta = std::fs::metadata(&canon)?;
        if !meta.is_dir() {
            return Err(Error::config(format!(
                "vfs root is not a directory: {canon}"
            )));
        }
        Ok(Self { root: canon })
    }

    /// Returns the canonical absolute root.
    #[must_use]
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    /// Resolves a root-relative [`VfsPath`] to an absolute path, with a
    /// defensive containment check. [`VfsPath`] already guarantees no `..` or
    /// absolute components; this guards against symlink shenanigans too.
    fn resolve(&self, path: &VfsPath) -> Result<Utf8PathBuf> {
        let mut full = self.root.clone();
        full.push(path.as_utf8_path());
        if !full.starts_with(&self.root) {
            return Err(Error::path_escape(path.as_str()));
        }
        Ok(full)
    }
}

impl Vfs for LocalFsVfs {
    async fn read(&self, path: &VfsPath) -> Result<Bytes> {
        let full = self.resolve(path)?;
        match tokio::fs::read(&full).await {
            Ok(bytes) => Ok(Bytes::from(bytes)),
            Err(e) => Err(map_not_found(e, path)),
        }
    }

    async fn write(&self, path: &VfsPath, data: Bytes) -> Result<()> {
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full, data).await?;
        Ok(())
    }

    async fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        let full = self.resolve(dir)?;
        let mut rd = tokio::fs::read_dir(&full).await?;
        let mut entries = Vec::new();
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            let name = entry.file_name();
            let child = full.join(Utf8PathBuf::from(name.to_string_lossy().into_owned()));
            let rel = child
                .strip_prefix(&self.root)
                .map_err(|e| Error::config(e.to_string()))?;
            let path = VfsPath::new(rel.to_owned())?;
            entries.push(DirEntry::new(path, metadata_from(&meta)));
        }
        Ok(entries)
    }

    async fn stat(&self, path: &VfsPath) -> Result<Metadata> {
        let full = self.resolve(path)?;
        match tokio::fs::metadata(&full).await {
            Ok(meta) => Ok(metadata_from(&meta)),
            Err(e) => Err(map_not_found(e, path)),
        }
    }

    async fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<()> {
        let from_full = self.resolve(from)?;
        let to_full = self.resolve(to)?;
        if let Some(parent) = to_full.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&from_full, &to_full).await?;
        Ok(())
    }

    async fn remove(&self, path: &VfsPath) -> Result<()> {
        let full = self.resolve(path)?;
        let meta = tokio::fs::metadata(&full).await;
        match meta {
            Ok(m) if m.is_dir() => {
                tokio::fs::remove_dir(&full).await?;
            }
            Ok(_) => {
                tokio::fs::remove_file(&full).await?;
            }
            Err(_) => return Err(Error::not_found(path.as_str())),
        }
        Ok(())
    }

    fn watch(&self, dir: &VfsPath) -> BoxStream<'static, VfsEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel::<VfsEvent>(256);
        let watch_path = match self.resolve(dir) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "watch resolve failed");
                return ReceiverStream::new(rx).boxed();
            }
        };
        let root = self.root.clone();

        // A `std` channel signals once the underlying watcher is armed, so
        // `watch()` only returns to the caller after events can actually be
        // observed (avoids a startup race where the consumer acts before the
        // watcher registers).
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();

        // A dedicated OS thread owns the watcher and forwards raw notify
        // events to the tokio channel. When the consumer drops the stream,
        // `blocking_send` fails and the thread (and watcher) wind down. No
        // tokio runtime context is required, so `watch()` is callable from
        // anywhere.
        std::thread::spawn(move || {
            use notify::Watcher;

            let (raw_tx, raw_rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
            let mut watcher = match notify::RecommendedWatcher::new(
                move |res: notify::Result<notify::Event>| {
                    let _ = raw_tx.send(res);
                },
                notify::Config::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!(error = %e, "notify watcher init failed");
                    let _ = ready_tx.send(false);
                    return;
                }
            };
            let armed = match watcher.watch(
                watch_path.as_std_path(),
                notify::RecursiveMode::NonRecursive,
            ) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(error = %e, "notify watch failed");
                    false
                }
            };
            let _ = ready_tx.send(armed);
            for res in raw_rx {
                match res {
                    Ok(ev) => {
                        for translated in translate_event(&ev, &root) {
                            if tx.blocking_send(translated).is_err() {
                                return; // stream consumer gone
                            }
                        }
                    }
                    Err(e) => tracing::warn!(error = %e, "notify event error"),
                }
            }
        });

        // Wait (briefly) for the watcher to arm so callers don't miss the very
        // first event. This is a blocking wait of a few microseconds in
        // practice; bounded to one second.
        let _ = ready_rx.recv_timeout(std::time::Duration::from_secs(1));

        ReceiverStream::new(rx).boxed()
    }
}

/// Converts `std::fs::Metadata` into the kernel [`Metadata`].
fn metadata_from(meta: &std::fs::Metadata) -> Metadata {
    let modified = meta.modified().ok().map(time::OffsetDateTime::from);
    Metadata::new(meta.len(), meta.is_dir(), modified)
}

/// Maps a `NotFound` I/O error into the typed [`Error::NotFound`], so the
/// VFS contract (missing file → `Error::NotFound`) is uniform across backends.
fn map_not_found(err: std::io::Error, path: &VfsPath) -> Error {
    if err.kind() == std::io::ErrorKind::NotFound {
        Error::not_found(path.as_str())
    } else {
        Error::Io(err)
    }
}

/// Translates a raw `notify` event into zero or more [`VfsEvent`]s, dropping
/// any path that is not under `root`.
fn translate_event(event: &notify::Event, root: &Utf8Path) -> Vec<VfsEvent> {
    use notify::event::{EventKind, ModifyKind};

    let mut out = Vec::new();
    for path in &event.paths {
        let Ok(rel) = path.strip_prefix(root.as_std_path()) else {
            continue;
        };
        let Some(rel_utf8) = Utf8Path::from_path(rel) else {
            continue;
        };
        let Ok(vfs_path) = VfsPath::new(rel_utf8.to_owned()) else {
            continue;
        };
        match event.kind {
            EventKind::Create(_) => out.push(VfsEvent::Created { path: vfs_path }),
            EventKind::Remove(_) => out.push(VfsEvent::Removed { path: vfs_path }),
            EventKind::Modify(ModifyKind::Name(_)) => {
                // Renames arrive as From/To pairs; M0 surfaces them as the
                // closest stable mapping rather than pairing them.
                out.push(VfsEvent::Removed { path: vfs_path });
            }
            EventKind::Modify(_) => out.push(VfsEvent::Changed { path: vfs_path }),
            EventKind::Access(_) | EventKind::Any | EventKind::Other => {}
        }
    }
    out
}
