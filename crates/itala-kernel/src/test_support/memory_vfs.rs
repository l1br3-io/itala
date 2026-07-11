//! [`MemoryVfs`] — a reference in-memory implementation of the [`Vfs`] port.
//!
//! Backed by a `HashMap<VfsPath, Bytes>` with a bounded `tokio::sync::mpsc`
//! per watcher for change events. Intended for fast unit tests; the same
//! port-conformance contract as a real adapter.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bytes::Bytes;
use futures::stream::{BoxStream, StreamExt};
use tokio::sync::mpsc;

use crate::vfs::Vfs;
use crate::vfs_types::{DirEntry, Metadata, VfsEvent, VfsPath};
use crate::{Error, Result};

/// In-memory VFS — see the [module docs](self) for details.
pub struct MemoryVfs {
    files: RwLock<HashMap<VfsPath, Bytes>>,
    watchers: Mutex<Vec<Watcher>>,
}

struct Watcher {
    dir: VfsPath,
    tx: mpsc::Sender<VfsEvent>,
}

impl MemoryVfs {
    /// Creates an empty in-memory VFS.
    #[must_use]
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
            watchers: Mutex::new(Vec::new()),
        }
    }

    /// Returns the number of stored files.
    #[must_use]
    pub fn len(&self) -> usize {
        read_lock(&self.files).len()
    }

    /// Returns `true` if there are no stored files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Broadcasts an event to every watcher whose `dir` is an ancestor of (or
    /// equal to) the event path, pruning closed watchers along the way.
    fn emit(&self, event: &VfsEvent) {
        let mut watchers = lock(&self.watchers);
        watchers.retain(|w| !w.tx.is_closed());
        for w in watchers.iter() {
            if event_matches_dir(event, &w.dir) {
                let _ = w.tx.try_send(event.clone());
            }
        }
    }
}

impl Default for MemoryVfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs for MemoryVfs {
    async fn read(&self, path: &VfsPath) -> Result<Bytes> {
        let files = read_lock(&self.files);
        files
            .get(path)
            .cloned()
            .ok_or_else(|| Error::not_found(path.as_str()))
    }

    async fn write(&self, path: &VfsPath, data: Bytes) -> Result<()> {
        let was_new = {
            let mut files = write_lock(&self.files);
            let was_new = !files.contains_key(path);
            files.insert(path.clone(), data);
            was_new
        };
        let event = if was_new {
            VfsEvent::Created { path: path.clone() }
        } else {
            VfsEvent::Changed { path: path.clone() }
        };
        self.emit(&event);
        Ok(())
    }

    async fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        // Collect raw (path, len) pairs under the lock, then build entries
        // after releasing it to keep the critical section tight.
        let snapshot: Vec<(VfsPath, u64)> = {
            let files = read_lock(&self.files);
            files
                .iter()
                .filter_map(|(p, b)| relative_under(p, dir).map(|_| (p.clone(), b.len() as u64)))
                .collect()
        };

        let mut entries = Vec::new();
        let mut seen_dirs = std::collections::BTreeSet::new();
        for (path, len) in snapshot {
            if let Some(rel) = relative_under(&path, dir) {
                let mut comps = rel.split('/');
                let first = comps.next().unwrap_or("");
                if comps.next().is_some() {
                    if seen_dirs.insert(first.to_string()) {
                        let dir_path = if dir.as_str().is_empty() {
                            VfsPath::new(first)?
                        } else {
                            dir.join(first)?
                        };
                        entries.push(DirEntry {
                            path: dir_path,
                            metadata: Metadata::new(0, true, None),
                        });
                    }
                } else {
                    entries.push(DirEntry {
                        path,
                        metadata: Metadata::new(len, false, None),
                    });
                }
            }
        }
        Ok(entries)
    }

    async fn stat(&self, path: &VfsPath) -> Result<Metadata> {
        let (file_len, has_under) = {
            let files = read_lock(&self.files);
            (
                files.get(path).map(|b| b.len() as u64),
                files
                    .keys()
                    .any(|p| is_directly_or_transitively_under(p, path)),
            )
        };
        file_len.map_or_else(
            || {
                if has_under {
                    Ok(Metadata::new(0, true, None))
                } else {
                    Err(Error::not_found(path.as_str()))
                }
            },
            |len| Ok(Metadata::new(len, false, None)),
        )
    }

    async fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<()> {
        let data = {
            let mut files = write_lock(&self.files);
            files
                .remove(from)
                .ok_or_else(|| Error::not_found(from.as_str()))?
        };
        {
            let mut files = write_lock(&self.files);
            files.insert(to.clone(), data);
        }
        self.emit(&VfsEvent::Renamed {
            from: from.clone(),
            to: to.clone(),
        });
        Ok(())
    }

    async fn remove(&self, path: &VfsPath) -> Result<()> {
        let existed = {
            let mut files = write_lock(&self.files);
            files.remove(path).is_some()
        };
        if existed {
            self.emit(&VfsEvent::Removed { path: path.clone() });
            Ok(())
        } else {
            Err(Error::not_found(path.as_str()))
        }
    }

    fn watch(&self, dir: &VfsPath) -> BoxStream<'static, VfsEvent> {
        let (tx, rx) = mpsc::channel(256);
        lock(&self.watchers).push(Watcher {
            dir: dir.clone(),
            tx,
        });
        tokio_stream::wrappers::ReceiverStream::new(rx).boxed()
    }
}

/// Returns the relative remainder of `path` under `dir`, or `None` if `path`
/// is not directly or transitively under `dir`.
fn relative_under<'a>(path: &'a VfsPath, dir: &VfsPath) -> Option<&'a str> {
    let p = path.as_str();
    let d = dir.as_str();
    if d.is_empty() {
        return Some(p);
    }
    let prefix = format!("{d}/");
    p.strip_prefix(&prefix)
}

/// Returns `true` if `path` is under (transitively) or equal to `dir`.
fn is_directly_or_transitively_under(path: &VfsPath, dir: &VfsPath) -> bool {
    let d = dir.as_str();
    if d.is_empty() {
        return true;
    }
    let p = path.as_str();
    p == d || p.starts_with(&format!("{d}/"))
}

/// Returns `true` if the event's path is under (or equal to) `dir`.
fn event_matches_dir(event: &VfsEvent, dir: &VfsPath) -> bool {
    match event {
        VfsEvent::Created { path } | VfsEvent::Changed { path } | VfsEvent::Removed { path } => {
            is_directly_or_transitively_under(path, dir)
        }
        VfsEvent::Renamed { from, to } => {
            is_directly_or_transitively_under(from, dir)
                || is_directly_or_transitively_under(to, dir)
        }
    }
}

fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

fn lock<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const M: fn() = check::<MemoryVfs>;
    const _: [(); 0] = [(); {
        let _ = M;
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::into_dyn;

    #[tokio::test]
    async fn round_trip_100_entries() {
        let vfs = MemoryVfs::new();
        for i in 0..100u32 {
            let path = VfsPath::new(format!("Notes/n{i}.md")).unwrap();
            vfs.write(&path, Bytes::from(format!("body-{i}")))
                .await
                .unwrap();
        }
        let entries = vfs.list(&VfsPath::new("Notes").unwrap()).await.unwrap();
        assert_eq!(entries.len(), 100);
        for entry in &entries {
            let bytes = vfs.read(&entry.path).await.unwrap();
            assert!(!bytes.is_empty());
        }
    }

    #[tokio::test]
    async fn watch_receives_created_event() {
        let vfs = MemoryVfs::new();
        let dir = VfsPath::new("Notes").unwrap();
        let mut stream = vfs.watch(&dir);
        let path = VfsPath::new("Notes/a.md").unwrap();
        vfs.write(&path, Bytes::from_static(b"hi")).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("timed out")
            .expect("stream ended");
        assert_eq!(event, VfsEvent::Created { path });
    }

    #[tokio::test]
    async fn stat_distinguishes_file_and_dir() {
        let vfs = MemoryVfs::new();
        let file = VfsPath::new("Notes/a.md").unwrap();
        vfs.write(&file, Bytes::from_static(b"abc")).await.unwrap();

        let m = vfs.stat(&file).await.unwrap();
        assert!(!m.is_dir);
        assert_eq!(m.len, 3);

        let d = vfs.stat(&VfsPath::new("Notes").unwrap()).await.unwrap();
        assert!(d.is_dir);
    }

    #[tokio::test]
    async fn rename_moves_bytes() {
        let vfs = MemoryVfs::new();
        let from = VfsPath::new("a.md").unwrap();
        let to = VfsPath::new("b.md").unwrap();
        vfs.write(&from, Bytes::from_static(b"xyz")).await.unwrap();
        vfs.rename(&from, &to).await.unwrap();
        assert!(vfs.read(&from).await.is_err());
        assert_eq!(vfs.read(&to).await.unwrap(), &b"xyz"[..]);
    }

    #[tokio::test]
    async fn bridge_into_dyn_vfs() {
        let vfs = into_dyn(MemoryVfs::new());
        let path = VfsPath::new("c.md").unwrap();
        vfs.write(&path, Bytes::from_static(b"dyn")).await.unwrap();
        let back = vfs.read(&path).await.unwrap();
        assert_eq!(back, Bytes::from_static(b"dyn"));
    }

    #[tokio::test]
    async fn port_conformance_suite() {
        super::super::conformance::assert_vfs_conformance(&MemoryVfs::new()).await;
    }

    #[tokio::test]
    async fn remove_returns_not_found_for_missing() {
        let vfs = MemoryVfs::new();
        let path = VfsPath::new("nope.md").unwrap();
        assert!(matches!(
            vfs.remove(&path).await,
            Err(Error::NotFound { .. })
        ));
    }
}
