//! Type-state vault: open/close, listing, and reading notes.

use std::marker::PhantomData;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use itala_kernel::{DynVfs, NoteId};
use time::OffsetDateTime;

use crate::error::{Error, Result};
use crate::note::{self, Note};
use crate::{frontmatter, note::backfill};

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
    /// file.  Each file is read and its `id:` frontmatter field resolved;
    /// notes without `id:` are backfilled in place.  This is O(N) VFS reads.
    ///
    /// # Errors
    /// - [`Error::DuplicateNoteId`] if two notes share the same `id:`.
    /// - [`Error::InvalidFrontmatterId`] if any note has a malformed `id:`.
    /// - Propagates other VFS and parse errors.
    pub async fn list(&self) -> Result<Vec<NoteId>> {
        let paths = collect_md_paths(&self.vfs, &self.root).await?;
        let mut seen: Vec<(NoteId, Utf8PathBuf)> = Vec::new();
        for path in paths {
            let id = self.resolve_id_for(&path).await?;
            if let Some((_, first_path)) = seen.iter().find(|(eid, _)| *eid == id) {
                return Err(Error::DuplicateNoteId {
                    id,
                    first_fs_location: first_path.clone(),
                    second_fs_location: path,
                });
            }
            seen.push((id, path));
        }
        let mut ids: Vec<NoteId> = seen.into_iter().map(|(id, _)| id).collect();
        ids.sort();
        Ok(ids)
    }

    /// Read a note by ID.
    ///
    /// Locates the note's path via [`Self::fs_location_of`], then reads and assembles the
    /// note.  If the file did not yet contain an `id:` field, the backfill
    /// write already happened inside `fs_location_of`; the bytes read here are the
    /// post-backfill content.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if no note has that id.  Propagates VFS,
    /// parse, and identity errors otherwise.
    pub async fn read(&self, id: NoteId) -> Result<Note> {
        let path = self
            .fs_location_of(id)
            .await?
            .ok_or(Error::NotFound { id })?;
        let abs = self.root.join(&path);
        let raw = self.vfs.read(&abs).await.map_err(Error::from)?;
        let (fm, body) = frontmatter::split(&raw)?;
        // After path_of the file is guaranteed to have `id:` in it, but we
        // re-resolve defensively and handle the absent case for robustness.
        let resolved_id = if let Some(fid) = note::id_from_frontmatter(&fm)? {
            fid
        } else {
            let path_id = note::id_from_path(&path);
            let rewritten = backfill::inject_id(&raw, &fm, body, path_id);
            self.vfs.write(&abs, rewritten).await.map_err(Error::from)?;
            path_id
        };
        note::assemble(
            path,
            resolved_id,
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
    }

    /// Return the vault-root-relative filesystem path for the given ID, if this
    /// is an FS-shaped backend.
    ///
    /// Returns `Ok(Some(path))` when the note is found, `Ok(None)` when no note
    /// with that ID exists in the vault.  The `Option` signals that this method
    /// is only meaningful for FS-backed vaults; future non-FS backends return
    /// `Ok(None)` for all IDs they manage through other means.
    ///
    /// Scans the full vault, resolving each note's ID (and backfilling when
    /// absent).  Detects duplicate `id:` values and errors immediately.
    ///
    /// # Errors
    /// - [`Error::DuplicateNoteId`] if two notes share the same id.
    /// - [`Error::InvalidFrontmatterId`] if any scanned note has a malformed `id:`.
    pub async fn fs_location_of(&self, id: NoteId) -> Result<Option<Utf8PathBuf>> {
        let paths = collect_md_paths(&self.vfs, &self.root).await?;
        let mut found: Option<Utf8PathBuf> = None;
        for path in paths {
            let note_id = self.resolve_id_for(&path).await?;
            if note_id == id {
                if let Some(first) = found {
                    return Err(Error::DuplicateNoteId {
                        id,
                        first_fs_location: first,
                        second_fs_location: path,
                    });
                }
                found = Some(path);
            }
        }
        Ok(found)
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

    /// Resolve the [`NoteId`] for a vault-root-relative path.
    ///
    /// Reads the file, parses its frontmatter, and returns the `id:` field if
    /// present.  When absent, derives the id from the path (the legacy scheme)
    /// and immediately backfills the `id:` field into the file so subsequent
    /// reads use the stable frontmatter id.
    ///
    /// Callers must pass a relative path; the vault root is prepended internally.
    ///
    /// # Errors
    /// - [`Error::InvalidFrontmatterId`] if `id:` is present but malformed.
    /// - Propagates VFS and parse errors.
    async fn resolve_id_for(&self, rel_path: &Utf8Path) -> Result<NoteId> {
        let abs = self.root.join(rel_path);
        let raw = self.vfs.read(&abs).await.map_err(Error::from)?;
        let (fm, body) = frontmatter::split(&raw)?;
        if let Some(id) = note::id_from_frontmatter(&fm)? {
            Ok(id)
        } else {
            let id = note::id_from_path(rel_path);
            let rewritten = backfill::inject_id(&raw, &fm, body, id);
            self.vfs.write(&abs, rewritten).await.map_err(Error::from)?;
            Ok(id)
        }
    }
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
