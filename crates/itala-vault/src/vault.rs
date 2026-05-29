//! Type-state vault: open/close, listing, reading, writing, and deleting notes.

use std::marker::PhantomData;
use std::sync::Arc;

use camino::Utf8PathBuf;
use itala_kernel::{DynVfs, NoteId};

use crate::adapters::FileProvider;
use crate::error::{Error, Result};
use crate::note::Note;
use crate::note_store::{NoteQuery, NoteStore};

/// Marker for a vault that has not been opened yet.
pub struct Closed;

/// Marker for a vault that is open and ready for reads and writes.
pub struct Open;

/// A vault over an injected [`DynVfs`] backend.
///
/// Type state encodes whether the vault is open or closed; the compiler
/// rejects operations on a closed vault at the call site.
pub struct Vault<State> {
    vfs: Arc<dyn DynVfs>,
    root: Utf8PathBuf,
    // None when State = Closed; Some when State = Open.
    store: Option<NoteStore>,
    _state: PhantomData<State>,
}

impl Vault<Closed> {
    /// Create a new closed vault over the given VFS and root path.
    #[must_use]
    pub fn new(vfs: Arc<dyn DynVfs>, root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            vfs,
            root: root.into(),
            store: None,
            _state: PhantomData,
        }
    }

    /// Open the vault, confirming the root exists on the VFS.
    ///
    /// Builds the [`FileProvider`] → `NoteStore` chain internally.
    ///
    /// # Errors
    /// Returns [`Error::BadRoot`] if the root does not exist.
    pub async fn open(self) -> Result<Vault<Open>> {
        let exists = self.vfs.exists(&self.root).await.map_err(Error::from)?;
        if !exists {
            return Err(Error::BadRoot { path: self.root });
        }
        let provider = FileProvider::new(Arc::clone(&self.vfs), self.root.clone());
        let store = NoteStore::new(Arc::new(provider));
        Ok(Vault {
            vfs: self.vfs,
            root: self.root,
            store: Some(store),
            _state: PhantomData,
        })
    }
}

impl Vault<Open> {
    /// List all note IDs in the vault, sorted for determinism.
    ///
    /// Recursively walks the vault root and collects every `*.md` file.
    /// Notes without `id:` are backfilled in place.
    ///
    /// # Errors
    /// - [`Error::DuplicateNoteId`] if two notes share the same `id:`.
    /// - [`Error::InvalidFrontmatterId`] if any note has a malformed `id:`.
    /// - Propagates other VFS and parse errors.
    pub async fn list(&self) -> Result<Vec<NoteId>> {
        self.store().list().await
    }

    /// Read a note by ID.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if no note has that id. Propagates VFS,
    /// parse, and identity errors otherwise.
    pub async fn read(&self, id: NoteId) -> Result<Note> {
        self.store().read(id).await
    }

    /// Write (create or overwrite) a note.
    ///
    /// The note's `id` and frontmatter are serialised into the on-disk blob.
    ///
    /// # Errors
    /// Propagates VFS errors.
    pub async fn write(&self, note: &Note) -> Result<()> {
        self.store().write(note).await
    }

    /// Delete a note by ID.
    ///
    /// Succeeds silently if no note with that ID exists.
    ///
    /// # Errors
    /// Propagates VFS errors.
    pub async fn delete(&self, id: NoteId) -> Result<()> {
        self.store().delete(id).await
    }

    /// Return the vault-root-relative filesystem path for the given ID, if
    /// this is an FS-shaped backend.
    ///
    /// Returns `Ok(Some(path))` when found, `Ok(None)` when the id does not
    /// exist (or the backend is non-FS).
    ///
    /// # Errors
    /// - [`Error::DuplicateNoteId`] if two notes share the same id.
    /// - [`Error::InvalidFrontmatterId`] if any scanned note has a malformed `id:`.
    pub async fn fs_location_of(&self, id: NoteId) -> Result<Option<Utf8PathBuf>> {
        self.store().fs_location_of(id).await
    }

    /// Query notes whose indexed fields match the given [`NoteQuery`].
    ///
    /// Uses a host-side scan-and-filter for [`FileProvider`]-backed vaults.
    ///
    /// # Errors
    /// Propagates provider and parse errors.
    pub async fn query(&self, q: &NoteQuery) -> Result<Vec<Note>> {
        self.store().query(q).await
    }

    /// Close the vault, returning it to the `Closed` state.
    #[must_use]
    pub fn close(self) -> Vault<Closed> {
        Vault {
            vfs: self.vfs,
            root: self.root,
            store: None,
            _state: PhantomData,
        }
    }

    /// Access the inner `NoteStore`.
    ///
    /// # Panics
    /// Panics if called on a `Vault<Open>` whose store was not initialised
    /// (this is an invariant violation that cannot happen through the public API).
    const fn store(&self) -> &NoteStore {
        match self.store.as_ref() {
            Some(s) => s,
            None => panic!("invariant: Vault<Open> always has a NoteStore"),
        }
    }
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
