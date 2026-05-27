//! Type-state vault: open/close, listing, and reading notes.

use std::marker::PhantomData;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use itala_kernel::{DynVfs, NoteId};
use time::OffsetDateTime;

use crate::error::{Error, Result};
use crate::note::{self, Note};

/// Marker for a vault that has not been opened yet.
pub struct Closed;

/// Marker for a vault that is open and ready for reads.
pub struct Open;

/// A vault over an injected [`DynVfs`] backend.
///
/// Type state encodes whether the vault is open or closed; the compiler
/// rejects operations on a closed vault at the call site.
pub struct Vault<State> {
    vfs: Arc<dyn DynVfs>,
    root: Utf8PathBuf,
    _state: PhantomData<State>,
}

impl Vault<Closed> {
    /// Create a new closed vault over the given VFS and root path.
    #[must_use]
    pub fn new(vfs: Arc<dyn DynVfs>, root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            vfs,
            root: root.into(),
            _state: PhantomData,
        }
    }

    /// Open the vault, confirming the root exists on the VFS.
    ///
    /// # Errors
    /// Returns [`Error::BadRoot`] if the root does not exist.
    pub async fn open(self) -> Result<Vault<Open>> {
        let exists = self.vfs.exists(&self.root).await.map_err(Error::from)?;
        if !exists {
            return Err(Error::BadRoot { path: self.root });
        }
        Ok(Vault {
            vfs: self.vfs,
            root: self.root,
            _state: PhantomData,
        })
    }
}

impl Vault<Open> {
    /// List all note IDs in the vault, sorted for determinism.
    ///
    /// Recursively walks the vault root via the VFS and collects every `*.md`
    /// file. Each call is O(N) in the number of notes; see TODO(F9) for
    /// the indexing redesign trigger.
    ///
    /// # Errors
    /// Propagates VFS errors.
    pub async fn list(&self) -> Result<Vec<NoteId>> {
        let mut ids = collect_md_ids(&self.vfs, &self.root).await?;
        ids.sort();
        Ok(ids)
    }

    /// Read a note by ID.
    ///
    /// Scans the vault for the matching path, then reads and assembles the note.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if no note has that id. Propagates VFS and
    /// parse errors otherwise.
    pub async fn read(&self, id: NoteId) -> Result<Note> {
        let path = self.path_of(id).await?;
        let abs = self.root.join(&path);
        let raw = self.vfs.read(&abs).await.map_err(Error::from)?;
        let bytes = bytes::Bytes::from(raw.to_vec());
        note::assemble(
            path,
            &bytes,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    /// Return the vault-root-relative path for the given ID.
    ///
    /// Scans the full vault; acceptable for F4 (see the recursive-list risk note).
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if no note has that id.
    pub async fn path_of(&self, id: NoteId) -> Result<Utf8PathBuf> {
        let paths = collect_md_paths(&self.vfs, &self.root).await?;
        for rel in paths {
            if note::id_from_path(&rel) == id {
                return Ok(rel);
            }
        }
        Err(Error::NotFound { id })
    }

    /// Close the vault, returning it to the `Closed` state.
    ///
    /// Sync in F4 (no buffered writes to flush). Will become `async` when writes land.
    #[must_use]
    pub fn close(self) -> Vault<Closed> {
        Vault {
            vfs: self.vfs,
            root: self.root,
            _state: PhantomData,
        }
    }
}

/// Validate that `candidate` is relative and contains no `..` segments.
///
/// Returns the joined absolute path (root + candidate) on success.
///
/// # Errors
/// Returns [`Error::PathEscape`] if the path is absolute or contains `..`.
// Used by write-path operations (F5+); dead_code suppressed until then.
#[allow(dead_code)]
pub fn validate_relative(root: &Utf8Path, candidate: &Utf8Path) -> Result<Utf8PathBuf> {
    if candidate.is_absolute() {
        return Err(Error::PathEscape {
            path: candidate.to_owned(),
        });
    }
    for component in candidate.components() {
        use camino::Utf8Component;
        if matches!(component, Utf8Component::ParentDir) {
            return Err(Error::PathEscape {
                path: candidate.to_owned(),
            });
        }
    }
    Ok(root.join(candidate))
}

/// Recursively collect vault-root-relative `Utf8PathBuf` entries for every
/// `*.md` file under `root`. `DynVfs::list` is shallow (direct children only),
/// so we recurse into children that have no extension (heuristic for directories).
async fn collect_md_paths(vfs: &Arc<dyn DynVfs>, root: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let children = vfs.list(&dir).await.map_err(Error::from)?;
        for child in children {
            // `child` is an absolute path. Derive the vault-root-relative form.
            let rel = child.strip_prefix(root).unwrap_or(&child).to_owned();
            if child.extension() == Some("md") {
                out.push(rel);
            } else if child.extension().is_none() {
                // No extension → likely a subdirectory; recurse.
                stack.push(child);
            }
            // Files with other extensions are skipped.
        }
    }
    tracing::debug!(count = out.len(), root = %root, "vault listed notes");
    Ok(out)
}

/// Collect `NoteId`s for all `*.md` files, using [`collect_md_paths`].
async fn collect_md_ids(vfs: &Arc<dyn DynVfs>, root: &Utf8Path) -> Result<Vec<NoteId>> {
    let paths = collect_md_paths(vfs, root).await?;
    Ok(paths.into_iter().map(|p| note::id_from_path(&p)).collect())
}

// ---------------------------------------------------------------------------
// Compile-time auto-trait assertions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod assertions {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn vault_closed_is_send_sync_unpin() {
        assert_send_sync::<Vault<Closed>>();
        assert_unpin::<Vault<Closed>>();
    }

    #[test]
    fn vault_open_is_send_sync_unpin() {
        assert_send_sync::<Vault<Open>>();
        assert_unpin::<Vault<Open>>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_relative_rejects_absolute() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("/etc/passwd")).unwrap_err();
        assert!(matches!(err, Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_rejects_dotdot_direct() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("../etc")).unwrap_err();
        assert!(matches!(err, Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_rejects_dotdot_nested() {
        let root = Utf8Path::new("/vault");
        let err = validate_relative(root, Utf8Path::new("foo/../bar")).unwrap_err();
        assert!(matches!(err, Error::PathEscape { .. }));
    }

    #[test]
    fn validate_relative_accepts_plain_path() {
        let root = Utf8Path::new("/vault");
        let joined = validate_relative(root, Utf8Path::new("notes/hello.md")).unwrap();
        assert_eq!(joined, Utf8PathBuf::from("/vault/notes/hello.md"));
    }

    /// ```compile_fail
    /// use std::sync::Arc;
    /// use itala_vault::{Vault, Closed};
    ///
    /// async fn bad(vfs: Arc<dyn itala_kernel::DynVfs>) {
    ///     let v: Vault<Closed> = Vault::new(vfs, "/vault");
    ///     // ERROR: `read` is not defined on `Vault<Closed>`
    ///     let _ = v.read(itala_kernel::NoteId::new()).await;
    /// }
    /// ```
    #[allow(dead_code)]
    fn compile_fail_doc_is_above() {}
}
