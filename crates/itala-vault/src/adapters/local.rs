//! Local filesystem [`Vfs`] adapter.
//!
//! [`LocalFsVfs`] roots every operation at a canonicalised directory so that
//! symlink-chasing and `..`-traversal attacks are detected before any I/O.
//! Writes are atomic (temp-file → fsync → rename) and the watcher is
//! backed by `notify-debouncer-full` with a 150 ms debounce window.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use futures::Stream;
use itala_kernel::{Error as KernelError, Vfs, VfsEvent};
use notify::RecommendedWatcher;
use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tokio::io::AsyncWriteExt as _;
use tokio_stream::wrappers::ReceiverStream;
use ulid::Ulid;

use crate::error::Result as VaultResult;

// `itala_kernel::Result<T>` is the return type for all Vfs methods.
// We use the full path to avoid shadowing `VaultResult`.
type KernelResult<T> = itala_kernel::Result<T>;

// ---------------------------------------------------------------------------
// Path-escape guard — used only by LocalFsVfs::join_path
// ---------------------------------------------------------------------------

/// Validate that `candidate` is relative and contains no `..` segments.
///
/// Returns the joined absolute path (root + candidate) on success.
///
/// # Errors
/// Returns [`crate::Error::PathEscape`] if the path is absolute or contains `..`.
fn validate_relative(root: &Utf8Path, candidate: &Utf8Path) -> VaultResult<Utf8PathBuf> {
    if candidate.is_absolute() {
        return Err(crate::Error::PathEscape {
            path: candidate.to_owned(),
        });
    }
    for component in candidate.components() {
        use camino::Utf8Component;
        if matches!(component, Utf8Component::ParentDir) {
            return Err(crate::Error::PathEscape {
                path: candidate.to_owned(),
            });
        }
    }
    Ok(root.join(candidate))
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// A [`Vfs`] implementation backed by the local OS filesystem.
///
/// All operations are rooted at the directory established at construction time.
/// Path-escape attempts (absolute paths outside the root, `..` traversal, or
/// symlinks pointing outside the root) are rejected with
/// [`KernelError::VaultPathNotRelative`] before any I/O is performed.
#[derive(Debug)]
pub struct LocalFsVfs {
    root: Utf8PathBuf,
}

impl LocalFsVfs {
    /// Create a `LocalFsVfs` rooted at `root`.
    ///
    /// The root is canonicalised immediately so that subsequent symlink checks
    /// always compare canonical-to-canonical paths.
    ///
    /// # Errors
    /// Returns [`crate::Error::BadRoot`] if `root` does not exist, is not a
    /// directory, or cannot be expressed as valid UTF-8 after canonicalisation.
    pub async fn new(root: impl AsRef<Utf8Path>) -> VaultResult<Self> {
        let root = root.as_ref();
        let canonical = tokio::fs::canonicalize(root)
            .await
            .map_err(|_| crate::Error::BadRoot {
                path: root.to_owned(),
            })?;
        let canonical = Utf8PathBuf::try_from(canonical).map_err(|_| crate::Error::BadRoot {
            path: root.to_owned(),
        })?;
        let meta = tokio::fs::metadata(&canonical).await?;
        if !meta.is_dir() {
            return Err(crate::Error::BadRoot { path: canonical });
        }
        Ok(Self { root: canonical })
    }

    // -----------------------------------------------------------------------
    // Private path helpers
    // -----------------------------------------------------------------------

    /// Resolve `path` (absolute-within-root or vault-relative) to an absolute
    /// path inside `self.root`, without canonicalising.
    ///
    /// Accepts absolute paths already within `self.root` (as passed by
    /// [`crate::vault::collect_md_paths`]) and relative paths without `..`.
    /// Does **not** detect symlinks — use [`resolve_for_read`] for that.
    fn join_path(&self, path: &Utf8Path) -> KernelResult<Utf8PathBuf> {
        if path.is_absolute() {
            if !path.starts_with(&self.root) {
                return Err(KernelError::VaultPathNotRelative {
                    path: path.to_owned(),
                });
            }
            Ok(path.to_owned())
        } else {
            // validate_relative rejects `..` segments and returns root.join(path).
            validate_relative(&self.root, path).map_err(|e| match e {
                crate::Error::PathEscape { path } => KernelError::VaultPathNotRelative { path },
                e => KernelError::Io(std::io::Error::other(e.to_string())),
            })
        }
    }

    /// Resolve `path` for a **read** operation, canonicalising to detect
    /// symlinks pointing outside `self.root`.
    ///
    /// Requires the target to already exist (canonicalise needs a real path).
    async fn resolve_for_read(&self, path: &Utf8Path) -> KernelResult<Utf8PathBuf> {
        let joined = self.join_path(path)?;
        let canonical = tokio::fs::canonicalize(&joined)
            .await
            .map_err(KernelError::Io)?;
        let canonical =
            Utf8PathBuf::try_from(canonical).map_err(|_| KernelError::VaultPathNotRelative {
                path: path.to_owned(),
            })?;
        if !canonical.starts_with(&self.root) {
            return Err(KernelError::VaultPathNotRelative {
                path: path.to_owned(),
            });
        }
        Ok(canonical)
    }

    /// Resolve `path` for a **write** operation.
    ///
    /// The target file may not yet exist, so only the parent directory is
    /// canonicalised. If the parent also does not exist (new nested path),
    /// a static prefix check is performed instead.
    async fn resolve_for_write(&self, path: &Utf8Path) -> KernelResult<Utf8PathBuf> {
        let joined = self.join_path(path)?;

        let parent = joined
            .parent()
            .ok_or_else(|| KernelError::VaultPathNotRelative {
                path: path.to_owned(),
            })?;
        let filename = joined
            .file_name()
            .ok_or_else(|| KernelError::VaultPathNotRelative {
                path: path.to_owned(),
            })?;

        let canonical_parent = if let Ok(p) = tokio::fs::canonicalize(parent).await {
            let p = Utf8PathBuf::try_from(p).map_err(|_| KernelError::VaultPathNotRelative {
                path: path.to_owned(),
            })?;
            if !p.starts_with(&self.root) {
                return Err(KernelError::VaultPathNotRelative {
                    path: path.to_owned(),
                });
            }
            p
        } else {
            // Parent not yet created; fall back to a static prefix check.
            if !joined.starts_with(&self.root) {
                return Err(KernelError::VaultPathNotRelative {
                    path: path.to_owned(),
                });
            }
            parent.to_owned()
        };

        Ok(canonical_parent.join(filename))
    }

    // -----------------------------------------------------------------------
    // Watcher setup
    // -----------------------------------------------------------------------

    fn make_watch_stream(
        &self,
        path: &Utf8Path,
    ) -> std::result::Result<WatcherStream, Box<dyn std::error::Error + Send + Sync>> {
        let abs = self.join_path(path)?;
        let (tx, rx) = tokio::sync::mpsc::channel::<VfsEvent>(1024);

        let mut debouncer = new_debouncer(
            Duration::from_millis(150),
            None,
            move |result: DebounceEventResult| {
                let events = match result {
                    Ok(ev) => ev,
                    Err(errors) => {
                        for err in &errors {
                            tracing::warn!(err = %err, "LocalFsVfs: watcher error");
                        }
                        return;
                    }
                };

                for ev in events {
                    let paths: Vec<Utf8PathBuf> = ev
                        .event
                        .paths
                        .iter()
                        .filter_map(|p| Utf8PathBuf::try_from(p.clone()).ok())
                        .collect();

                    let vfs_events: Vec<VfsEvent> = match ev.event.kind {
                        EventKind::Create(_) => paths
                            .iter()
                            .map(|p| VfsEvent::Created { path: p.clone() })
                            .collect(),
                        EventKind::Modify(ModifyKind::Data(_)) => paths
                            .iter()
                            .map(|p| VfsEvent::Changed { path: p.clone() })
                            .collect(),
                        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                            // RecommendedCache pairs rename events; paths = [from, to].
                            let mut evs: Vec<VfsEvent> = Vec::new();
                            if paths.len() >= 2 {
                                evs.push(VfsEvent::Deleted {
                                    path: paths[0].clone(),
                                });
                                evs.push(VfsEvent::Created {
                                    path: paths[1].clone(),
                                });
                            } else if let Some(p) = paths.first() {
                                evs.push(VfsEvent::Deleted { path: p.clone() });
                            }
                            evs
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => paths
                            .iter()
                            .map(|p| VfsEvent::Deleted { path: p.clone() })
                            .collect(),
                        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => paths
                            .iter()
                            .map(|p| VfsEvent::Created { path: p.clone() })
                            .collect(),
                        EventKind::Remove(_) => paths
                            .iter()
                            .map(|p| VfsEvent::Deleted { path: p.clone() })
                            .collect(),
                        // Access, metadata, and unknown events are intentionally ignored.
                        _ => vec![],
                    };

                    for event in vfs_events {
                        if tx.try_send(event).is_err() {
                            tracing::warn!("LocalFsVfs: watcher channel full, dropping event");
                        }
                    }
                }
            },
        )?;

        debouncer.watch(abs.as_std_path(), RecursiveMode::Recursive)?;

        Ok(WatcherStream {
            _debouncer: debouncer,
            inner: ReceiverStream::new(rx),
        })
    }
}

// ---------------------------------------------------------------------------
// Vfs impl
// ---------------------------------------------------------------------------

impl Vfs for LocalFsVfs {
    fn read(&self, path: &Utf8Path) -> impl Future<Output = KernelResult<Bytes>> + Send + '_ {
        // Own `path` so the future only borrows `self` (satisfies the single
        // `'_` lifetime bound on the impl trait return).
        let path = path.to_owned();
        async move {
            let abs = self.resolve_for_read(&path).await?;
            let bytes = tokio::fs::read(&abs).await.map_err(KernelError::Io)?;
            Ok(Bytes::from(bytes))
        }
    }

    fn write(
        &self,
        path: &Utf8Path,
        bytes: Bytes,
    ) -> impl Future<Output = KernelResult<()>> + Send + '_ {
        let path = path.to_owned();
        async move {
            let abs = self.resolve_for_write(&path).await?;

            // Ensure parent directory hierarchy exists.
            if let Some(parent) = abs.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(KernelError::Io)?;
            }

            let parent = abs
                .parent()
                .ok_or_else(|| KernelError::VaultPathNotRelative { path: path.clone() })?;
            let stem = abs.file_stem().unwrap_or("tmp");
            let temp_name = format!(".{stem}.tmp.{}", Ulid::new());
            let temp_path = parent.join(&temp_name);

            // Write to temp, fsync, atomically rename into place.
            let write_result: std::io::Result<()> = async {
                let mut file = tokio::fs::File::create(&temp_path).await?;
                file.write_all(&bytes).await?;
                file.sync_all().await?;
                drop(file);
                tokio::fs::rename(&temp_path, &abs).await?;
                Ok(())
            }
            .await;

            if let Err(e) = write_result {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(KernelError::Io(e));
            }

            // Flush parent directory entry on POSIX so the rename is durable.
            // Windows relies on MoveFileExW atomicity and has no equivalent.
            #[cfg(not(windows))]
            {
                let dir = tokio::fs::File::open(parent)
                    .await
                    .map_err(KernelError::Io)?;
                dir.sync_all().await.map_err(KernelError::Io)?;
            }

            Ok(())
        }
    }

    fn list(
        &self,
        dir: &Utf8Path,
    ) -> impl Future<Output = KernelResult<Vec<Utf8PathBuf>>> + Send + '_ {
        let dir = dir.to_owned();
        async move {
            let abs = self.resolve_for_read(&dir).await?;
            let mut entries = Vec::new();
            let mut rd = tokio::fs::read_dir(&abs).await.map_err(KernelError::Io)?;
            while let Some(entry) = rd.next_entry().await.map_err(KernelError::Io)? {
                // `read_dir` returns absolute OS paths; convert to Utf8.
                let utf8 = Utf8PathBuf::try_from(entry.path())
                    .map_err(|e| KernelError::Io(std::io::Error::other(e.to_string())))?;
                entries.push(utf8);
            }
            Ok(entries)
        }
    }

    fn exists(&self, path: &Utf8Path) -> impl Future<Output = KernelResult<bool>> + Send + '_ {
        let path = path.to_owned();
        async move {
            let joined = self.join_path(&path)?;
            match tokio::fs::metadata(&joined).await {
                Ok(_) => {
                    // Path exists; canonicalise to catch symlinks pointing outside root.
                    let canonical = tokio::fs::canonicalize(&joined)
                        .await
                        .map_err(KernelError::Io)?;
                    let canonical = Utf8PathBuf::try_from(canonical)
                        .map_err(|_| KernelError::VaultPathNotRelative { path: path.clone() })?;
                    if !canonical.starts_with(&self.root) {
                        return Err(KernelError::VaultPathNotRelative { path });
                    }
                    Ok(true)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(KernelError::Io(e)),
            }
        }
    }

    fn delete(&self, path: &Utf8Path) -> impl Future<Output = KernelResult<()>> + Send + '_ {
        let path = path.to_owned();
        async move {
            let abs = self.resolve_for_read(&path).await?;
            tokio::fs::remove_file(&abs)
                .await
                .map_err(KernelError::Io)?;
            Ok(())
        }
    }

    fn watch(&self, path: &Utf8Path) -> impl Stream<Item = VfsEvent> + Send + '_ {
        let boxed: Pin<Box<dyn Stream<Item = VfsEvent> + Send>> = match self.make_watch_stream(path)
        {
            Ok(s) => Box::pin(s),
            Err(e) => {
                tracing::error!(
                    err = %e,
                    path = %path,
                    "LocalFsVfs: failed to start watcher"
                );
                Box::pin(futures::stream::empty())
            }
        };
        boxed
    }
}

// ---------------------------------------------------------------------------
// WatcherStream — holds the debouncer (keeps OS handles alive) + the receiver.
// ---------------------------------------------------------------------------

struct WatcherStream {
    /// Dropping this stops the OS watcher and releases all file-descriptor handles.
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    inner: ReceiverStream<VfsEvent>,
}

impl Stream for WatcherStream {
    type Item = VfsEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}

// `ReceiverStream` is always `Unpin`. Declaring `Unpin` here lets `poll_next`
// pin-project to `inner` without unsafe. The `_debouncer` field is never
// accessed through a pin projection so its `Unpin`-ness is irrelevant.
impl Unpin for WatcherStream {}

// ---------------------------------------------------------------------------
// Compile-time auto-trait assertions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod assertions {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn local_fs_vfs_is_send_sync_unpin() {
        assert_send_sync::<LocalFsVfs>();
        assert_unpin::<LocalFsVfs>();
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bytes::Bytes;
    use camino::Utf8Path;
    use itala_kernel::{DynVfs, Error as KernelError, Vfs};
    use tempfile::TempDir;

    use super::*;

    // -- validate_relative (co-located with the only caller) --

    #[test]
    fn validate_relative_rejects_absolute() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("/etc/passwd")).unwrap_err();
        assert!(matches!(err, crate::Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_rejects_dotdot_direct() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("../etc")).unwrap_err();
        assert!(matches!(err, crate::Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_rejects_dotdot_nested() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("foo/../bar")).unwrap_err();
        assert!(matches!(err, crate::Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_accepts_plain_path() {
        let root = Utf8Path::new("/vault");
        let joined = validate_relative(root, Utf8Path::new("notes/hello.md")).unwrap();
        assert_eq!(joined, Utf8PathBuf::from("/vault/notes/hello.md"));
    }

    async fn new_vfs() -> (LocalFsVfs, TempDir) {
        let dir = TempDir::new().unwrap();
        let vfs = LocalFsVfs::new(Utf8Path::from_path(dir.path()).expect("tempdir utf-8"))
            .await
            .unwrap();
        (vfs, dir)
    }

    // -- Constructor --

    #[tokio::test]
    async fn new_rejects_missing_root() {
        let err = LocalFsVfs::new(Utf8Path::new("/no/such/vault/xyz_99999"))
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::BadRoot { .. }));
    }

    #[tokio::test]
    async fn new_rejects_file_as_root() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("not_a_dir.txt");
        std::fs::write(&file, b"hi").unwrap();
        let utf8 = Utf8Path::from_path(&file).unwrap();
        let err = LocalFsVfs::new(utf8).await.unwrap_err();
        assert!(matches!(err, crate::Error::BadRoot { .. }));
    }

    // -- Path escape --

    #[tokio::test]
    async fn read_rejects_absolute_outside_root() {
        let (vfs, _dir) = new_vfs().await;
        let err = Vfs::read(&vfs, Utf8Path::new("/etc/passwd"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, KernelError::VaultPathNotRelative { .. }),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn read_rejects_dotdot() {
        let (vfs, _dir) = new_vfs().await;
        let err = Vfs::read(&vfs, Utf8Path::new("../etc/passwd"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, KernelError::VaultPathNotRelative { .. }),
            "got {err:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn read_rejects_symlink_escape() {
        let (vfs, dir) = new_vfs().await;
        // Symlink inside the vault root pointing to /tmp (always exists on Unix).
        let link = dir.path().join("escape_link");
        std::os::unix::fs::symlink("/tmp", &link).unwrap();
        let err = Vfs::read(&vfs, Utf8Path::new("escape_link"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, KernelError::VaultPathNotRelative { .. }),
            "got {err:?}"
        );
    }

    // -- Vfs round-trip --

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let (vfs, _dir) = new_vfs().await;
        let body = Bytes::from_static(b"# Hello\n\nbody\n");
        Vfs::write(&vfs, Utf8Path::new("note.md"), body.clone())
            .await
            .unwrap();
        let got = Vfs::read(&vfs, Utf8Path::new("note.md")).await.unwrap();
        assert_eq!(got, body);
    }

    #[tokio::test]
    async fn write_creates_nested_parent_dirs() {
        let (vfs, _dir) = new_vfs().await;
        let path = Utf8Path::new("a/b/c/note.md");
        Vfs::write(&vfs, path, Bytes::from_static(b"deep"))
            .await
            .unwrap();
        let got = Vfs::read(&vfs, path).await.unwrap();
        assert_eq!(&got[..], b"deep");
    }

    #[tokio::test]
    async fn exists_false_for_missing() {
        let (vfs, _dir) = new_vfs().await;
        assert!(
            !Vfs::exists(&vfs, Utf8Path::new("missing.md"))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn exists_true_after_write() {
        let (vfs, _dir) = new_vfs().await;
        Vfs::write(&vfs, Utf8Path::new("p.md"), Bytes::from_static(b"x"))
            .await
            .unwrap();
        assert!(Vfs::exists(&vfs, Utf8Path::new("p.md")).await.unwrap());
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let (vfs, _dir) = new_vfs().await;
        Vfs::write(&vfs, Utf8Path::new("del.md"), Bytes::from_static(b"y"))
            .await
            .unwrap();
        Vfs::delete(&vfs, Utf8Path::new("del.md")).await.unwrap();
        assert!(!Vfs::exists(&vfs, Utf8Path::new("del.md")).await.unwrap());
    }

    #[tokio::test]
    async fn list_returns_absolute_paths() {
        let (vfs, dir) = new_vfs().await;
        let root = Utf8Path::from_path(dir.path()).unwrap();
        Vfs::write(&vfs, Utf8Path::new("a.md"), Bytes::from_static(b"a"))
            .await
            .unwrap();
        Vfs::write(&vfs, Utf8Path::new("b.md"), Bytes::from_static(b"b"))
            .await
            .unwrap();
        let entries = Vfs::list(&vfs, root).await.unwrap();
        assert_eq!(entries.len(), 2);
        for p in &entries {
            assert!(p.is_absolute(), "{p} is not absolute");
            assert!(p.starts_with(root), "{p} does not start with root");
        }
    }

    #[tokio::test]
    async fn dyn_vfs_blanket_round_trip() {
        let (vfs, _dir) = new_vfs().await;
        let arc: Arc<dyn DynVfs> = Arc::new(vfs);
        let path = Utf8Path::new("dyn.md");
        let body = Bytes::from_static(b"via dyn");
        arc.write(path, body.clone()).await.unwrap();
        let got = arc.read(path).await.unwrap();
        assert_eq!(got, body);
    }
}
