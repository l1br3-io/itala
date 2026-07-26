//! Note CRUD operations on a [`Vault`].
//!
//! All paths live under `Notes/`. The vault publishes [`KernelEvent::NoteSaved`]
//! after a successful create and [`KernelEvent::NoteDeleted`] after a delete;
//! consumers (graph, search — later milestones) react on the bus.

use bytes::Bytes;

use itala_kernel::events::KernelEvent;
use itala_kernel::vfs_types::DirEntry;
use itala_kernel::{Error, HistoryId, NoteId, Result};
use itala_note::{Note, ParseError};
use smol_str::SmolStr;

use crate::Vault;

/// Subdirectory of the vault root that holds note Markdown files.
pub const NOTES_DIR: &str = "Notes";

/// Lightweight note metadata for `list` operations; body is not loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteMetadata {
    /// The note's stable identifier (parsed from the filename; see
    /// [`note_id_from_path`]).
    pub id: NoteId,
    /// Canonical vault-relative path of the note file.
    pub path: itala_kernel::vfs_types::VfsPath,
    /// File size in bytes.
    pub len: u64,
    /// Filesystem modified time, if available.
    pub modified: Option<time::OffsetDateTime>,
}

/// Result of [`Vault::move_note`]: the move + every referring note path that
/// had its links rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveReport {
    /// The path the note was moved from.
    pub moved_from: itala_kernel::vfs_types::VfsPath,
    /// The path the note was moved to.
    pub moved_to: itala_kernel::vfs_types::VfsPath,
    /// Other note paths whose bodies had `[[wiki-links]]` rewritten.
    pub rewritten: Vec<itala_kernel::vfs_types::VfsPath>,
}

/// Result of [`Vault::create_note`]: the newly-persisted note plus the
/// human-readable title that was slugified into the filename.
#[derive(Debug, Clone)]
pub struct CreatedNote {
    /// The persisted note.
    pub note: Note,
    /// The title as supplied to `create_note` (preserves case and spacing).
    pub title: String,
}

/// Parses a `NoteId` from a note file's path, expecting the filename to
/// contain a ULID segment (e.g. `Notes/hello-01JPTULIDXXX.md`).
///
/// The first 26-character ASCII-alphanumeric run inside the filename is
/// taken as the ULID. Returns [`Error::InvalidId`] if no such segment is
/// found.
pub fn note_id_from_path(path: &itala_kernel::vfs_types::VfsPath) -> Result<NoteId> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::invalid_id(format!("no filename in {path}")))?;
    let bytes = name.as_bytes();
    let mut i = 0;
    while i + 26 <= bytes.len() {
        let candidate = &name[i..i + 26];
        if candidate.chars().all(|c| c.is_ascii_alphanumeric()) {
            let upper: String = candidate.chars().map(|c| c.to_ascii_uppercase()).collect();
            return upper
                .parse()
                .map_err(|e| Error::invalid_id(format!("path ulid parse: {e}")));
        }
        i += 1;
    }
    Err(Error::invalid_id(format!(
        "no 26-char ULID segment in {name}"
    )))
}

/// Extracts the human-readable slug portion of a note filename, given the
/// convention `<slug>-<ulid>.md`. Returns `None` if the convention is not
/// matched (no `.md`/`.markdown` extension or no ULID suffix).
pub(crate) fn slug_from_path(path: &itala_kernel::vfs_types::VfsPath) -> Option<SmolStr> {
    let name = path.file_name()?;
    let stem = name
        .strip_suffix(".md")
        .or_else(|| name.strip_suffix(".markdown"))?;
    // Need at least "-<26 alphanumerics>" before stem end.
    if stem.len() < 27 {
        return None;
    }
    let ulid_start = stem.len() - 26;
    let dash_idx = ulid_start - 1;
    let bytes = stem.as_bytes();
    if bytes.get(dash_idx) != Some(&b'-') {
        return None;
    }
    let ulid_part = &stem[ulid_start..];
    if !ulid_part.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(SmolStr::new(&stem[..dash_idx]))
}

/// Replaces every `[[<from>#^block-id]]` or `[[<from>]]` or `[[<from>|...]]`
/// in `text` with the same form using `to` as the target. Returns the
/// rewritten text.
pub(crate) fn rewrite_links(text: &str, from: &str, to: &str) -> String {
    use crate::wiki_link::scan;
    let links = scan(text);
    if links.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    for link in links {
        // Skip same-page block refs (empty target) and non-matching targets.
        if link.target != from {
            continue;
        }
        // Emit unchanged text up to this link.
        out.push_str(&text[cursor..link.byte_range.start]);
        // Reconstruct the link with the new target.
        out.push_str("[[");
        out.push_str(to);
        if let Some(b) = &link.block {
            out.push_str("#^");
            out.push_str(b);
        }
        if let Some(d) = &link.display {
            out.push('|');
            out.push_str(d);
        }
        out.push_str("]]");
        cursor = link.byte_range.end;
    }
    out.push_str(&text[cursor..]);
    out
}

impl Vault {
    /// Creates a new note in `Notes/`, persisting it to the VFS.
    ///
    /// The filename is `<slug>-<ulid>.md` where `<ulid>` is the full 26-char
    /// ULID form of the new [`NoteId`]. The full ULID is preserved so
    /// [`note_id_from_path`] can recover the id without external lookup.
    ///
    /// # Errors
    /// VFS write errors propagate.
    pub async fn create_note(&self, title: &str, body: &str) -> Result<CreatedNote> {
        let id = NoteId::new();
        let slug = crate::slug::slugify(title);
        let ulid = id.to_string();
        let filename = format!("{slug}-{ulid}.md");
        let path = itala_kernel::vfs_types::VfsPath::new(NOTES_DIR)?.join(filename.as_str())?;

        let now = time::OffsetDateTime::now_utc();
        let mut frontmatter = itala_note::Frontmatter::empty();
        frontmatter.set("title", serde_json::Value::String(title.to_owned()));

        let body_bytes = Bytes::from(body.to_owned());
        let checksum = blake3::hash(&body_bytes);

        let note = Note {
            id,
            path: path.clone(),
            frontmatter,
            body: body_bytes.clone(),
            created_at: now,
            updated_at: now,
            checksum: checksum.into(),
        };
        let markdown = note.to_markdown();
        self.save_note_inner(&note, markdown).await?;
        self.bus().publish(KernelEvent::NoteSaved { id });
        Ok(CreatedNote {
            note,
            title: title.to_owned(),
        })
    }

    /// Saves an existing note, writing its Markdown projection to the VFS
    /// and snapshotting into the history store (if attached).
    ///
    /// # Errors
    /// VFS write errors propagate; history-store errors propagate.
    pub async fn save_note(&self, note: &Note) -> Result<()> {
        let markdown = note.to_markdown();
        self.save_note_inner(note, markdown).await?;
        self.bus().publish(KernelEvent::NoteSaved { id: note.id });
        Ok(())
    }

    /// Shared write+snapshot path used by both `create_note` and `save_note`.
    /// The `markdown` argument is the durable byte form to persist and snapshot.
    async fn save_note_inner(&self, note: &Note, markdown: Bytes) -> Result<()> {
        self.vfs().write(&note.path, markdown.clone()).await?;
        if let Some(h) = self.history_store() {
            let snap_checksum = blake3::hash(&markdown);
            h.snapshot(note.id, markdown, snap_checksum.into()).await?;
        }
        Ok(())
    }

    /// Lists all notes under `Notes/`, returning metadata without bodies.
    ///
    /// # Errors
    /// VFS list errors propagate. Notes whose filenames lack a ULID segment
    /// are skipped silently (logged at `debug`).
    pub async fn list_notes(&self) -> Result<Vec<NoteMetadata>> {
        let root = itala_kernel::vfs_types::VfsPath::new(NOTES_DIR)?;
        let mut out = Vec::new();
        list_recursive(self.vfs(), &root, &mut out).await?;
        out.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
        Ok(out)
    }

    /// Reads a single note by path, parsing it into a [`Note`].
    ///
    /// # Errors
    /// [`Error::NotFound`] if the file does not exist; VFS read errors
    /// propagate; [`Error::Note`] if the bytes fail to parse as Markdown.
    pub async fn read_note(&self, path: &itala_kernel::vfs_types::VfsPath) -> Result<Note> {
        let bytes = self.vfs().read(path).await?;
        let id = note_id_from_path(path)?;
        let now = time::OffsetDateTime::now_utc();
        Note::from_markdown(id, path.clone(), bytes, now, now).map_err(|e| match e {
            ParseError::NotUtf8 => Error::Note {
                id,
                reason: "note bytes are not UTF-8".to_owned(),
            },
            ParseError::Frontmatter(fe) => Error::Note {
                id,
                reason: format!("frontmatter parse: {fe}"),
            },
            other => Error::Note {
                id,
                reason: format!("parse: {other}"),
            },
        })
    }

    /// Deletes a note by path.
    ///
    /// # Errors
    /// [`Error::NotFound`] if the file does not exist; VFS remove errors
    /// propagate.
    pub async fn delete_note(&self, path: &itala_kernel::vfs_types::VfsPath) -> Result<()> {
        let id = note_id_from_path(path);
        self.vfs().remove(path).await?;
        // best-effort: only publish NoteDeleted when we could extract an id.
        if let Ok(id) = id {
            self.bus().publish(KernelEvent::NoteDeleted { id });
        }
        Ok(())
    }

    /// Moves a note to a new path, rewriting any `[[wiki-link]]` referring
    /// to the note's old slug so it points at the new slug.
    ///
    /// For M1 (AD-2 in the M1 plan): the rewrite is a naive scan — every
    /// other note is read, scanned for `[[<old-slug>…]]`, and rewritten
    /// in-place. The graph context (M3) will replace this with an indexed
    /// lookup.
    ///
    /// # Errors
    /// [`Error::NotFound`] if `from` does not exist; VFS errors propagate.
    pub async fn move_note(
        &self,
        from: &itala_kernel::vfs_types::VfsPath,
        to: &itala_kernel::vfs_types::VfsPath,
    ) -> Result<MoveReport> {
        // Parse the old slug from the filename: `<slug>-<ulid>.md` → `<slug>`.
        let from_slug = slug_from_path(from)
            .ok_or_else(|| Error::invalid_id(format!("cannot derive slug from {from}")))?;
        let to_slug = slug_from_path(to).unwrap_or_else(|| {
            // If the destination filename lacks the convention, fall back to
            // the file-stem minus any ULID-like suffix.
            SmolStr::from(to.file_name().unwrap_or("untitled"))
        });

        // 1. VFS rename (creates parent dirs as needed by Vfs::write contract).
        //    We read+write+remove because Vfs::rename semantics for cross-dir
        //    moves are not guaranteed to create intermediate parents.
        let bytes = self.vfs().read(from).await?;
        self.vfs().write(to, bytes).await?;
        self.vfs().remove(from).await?;

        // 2. Rewrite links in every other note.
        let mut rewritten = Vec::new();
        let all = self.list_notes().await?;
        for meta in all {
            if meta.path == *to {
                continue;
            }
            let note_bytes = self.vfs().read(&meta.path).await?;
            let Ok(text) = std::str::from_utf8(&note_bytes) else {
                continue;
            };
            let updated = rewrite_links(text, &from_slug, &to_slug);
            if updated != text {
                self.vfs().write(&meta.path, Bytes::from(updated)).await?;
                rewritten.push(meta.path);
            }
        }

        // 3. Publish NoteRenamed if we can recover an id.
        if let Ok(id_from) = note_id_from_path(from)
            && let Ok(id_to) = note_id_from_path(to)
        {
            self.bus().publish(KernelEvent::NoteRenamed {
                from: id_from,
                to: id_to,
            });
        }

        Ok(MoveReport {
            moved_from: from.clone(),
            moved_to: to.clone(),
            rewritten,
        })
    }

    /// Lists history entries for a note, newest-first.
    ///
    /// # Errors
    /// [`Error::Config`] if no history store is attached; store errors
    /// propagate.
    pub async fn history(&self, note_id: NoteId) -> Result<Vec<itala_kernel::HistoryEntry>> {
        let h = self
            .history_store()
            .ok_or_else(|| Error::config("no history store attached"))?;
        h.list(note_id).await
    }

    /// Restores the body bytes of a specific history snapshot.
    ///
    /// Note that restore does *not* re-save through [`Vault::save_note`]; the
    /// caller is responsible for writing the restored bytes back if a new
    /// snapshot should be recorded.
    ///
    /// # Errors
    /// [`Error::Config`] if no history store is attached; store errors
    /// propagate.
    pub async fn restore_history(&self, id: HistoryId) -> Result<Bytes> {
        let h = self
            .history_store()
            .ok_or_else(|| Error::config("no history store attached"))?;
        h.restore(id).await
    }

    /// Fetches metadata for a single history entry.
    ///
    /// # Errors
    /// [`Error::Config`] if no history store is attached; store errors
    /// propagate.
    pub async fn get_history(&self, id: HistoryId) -> Result<itala_kernel::HistoryEntry> {
        let h = self
            .history_store()
            .ok_or_else(|| Error::config("no history store attached"))?;
        h.get(id).await
    }
}

/// Recursive walk helper. Pushes any `.md` file found under `dir`.
async fn list_recursive(
    vfs: &dyn itala_kernel::vfs::DynVfs,
    dir: &itala_kernel::vfs_types::VfsPath,
    out: &mut Vec<NoteMetadata>,
) -> Result<()> {
    let entries = vfs.list(dir).await;
    let entries = match entries {
        Ok(e) => e,
        Err(Error::NotFound { .. }) => return Ok(()), // empty vault has no Notes/ yet
        Err(e) => return Err(e),
    };
    for DirEntry { path, metadata, .. } in entries {
        if metadata.is_dir {
            Box::pin(list_recursive(vfs, &path, out)).await?;
        } else if std::path::Path::new(path.as_str())
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            && let Ok(id) = note_id_from_path(&path)
        {
            out.push(NoteMetadata {
                id,
                path,
                len: metadata.len,
                modified: metadata.modified,
            });
            // Non-conforming filenames are silently skipped; the slug-ULID
            // convention is established at create time.
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vault;
    use itala_kernel::vfs::into_dyn;
    use itala_vfs_fs::LocalFsVfs;

    async fn fresh_vault() -> (tempfile::TempDir, Vault) {
        let tmp = tempfile::tempdir().unwrap();
        let vfs = into_dyn(LocalFsVfs::new(tmp.path()).unwrap());
        let vault = Vault::init(vfs).await.unwrap();
        (tmp, vault)
    }

    #[tokio::test]
    async fn create_persists_and_publishes() {
        let (_tmp, vault) = fresh_vault().await;
        let mut sub = vault.bus().subscribe();
        let created = vault
            .create_note("Hello World", "# Hello\n\nFirst note.\n")
            .await
            .unwrap();
        assert_eq!(created.title, "Hello World");
        assert_eq!(
            created
                .note
                .frontmatter
                .get("title")
                .and_then(serde_json::Value::as_str),
            Some("Hello World")
        );
        assert!(created.note.path.as_str().starts_with("Notes/hello-world-"));
        // Event published.
        let ev = sub.recv().await.unwrap();
        assert!(matches!(ev, KernelEvent::NoteSaved { .. }));
    }

    #[tokio::test]
    async fn list_returns_created_note() {
        let (_tmp, vault) = fresh_vault().await;
        vault.create_note("Alpha", "body a").await.unwrap();
        vault.create_note("Beta", "body b").await.unwrap();
        let notes = vault.list_notes().await.unwrap();
        assert_eq!(notes.len(), 2);
        assert!(
            notes
                .iter()
                .any(|n| n.path.as_str().starts_with("Notes/alpha-"))
        );
        assert!(
            notes
                .iter()
                .any(|n| n.path.as_str().starts_with("Notes/beta-"))
        );
    }

    #[tokio::test]
    async fn list_on_empty_vault_returns_empty() {
        let (_tmp, vault) = fresh_vault().await;
        let notes = vault.list_notes().await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn read_note_round_trips() {
        let (_tmp, vault) = fresh_vault().await;
        let created = vault.create_note("Title", "body text").await.unwrap();
        let path = created.note.path.clone();
        let read = vault.read_note(&path).await.unwrap();
        assert_eq!(read.body, Bytes::from_static(b"body text"));
        assert_eq!(
            read.frontmatter
                .get("title")
                .and_then(serde_json::Value::as_str),
            Some("Title")
        );
    }

    #[tokio::test]
    async fn delete_removes_and_publishes() {
        let (_tmp, vault) = fresh_vault().await;
        let created = vault.create_note("Bye", "content").await.unwrap();
        let mut sub = vault.bus().subscribe();
        let path = created.note.path.clone();
        vault.delete_note(&path).await.unwrap();
        let notes = vault.list_notes().await.unwrap();
        assert!(notes.is_empty());
        let ev = sub.recv().await.unwrap();
        assert!(matches!(ev, KernelEvent::NoteDeleted { .. }));
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_not_found() {
        let (_tmp, vault) = fresh_vault().await;
        let path =
            itala_kernel::vfs_types::VfsPath::new("Notes/doesnotexist-01JPTULIDULIDULIDULIDUL.md")
                .unwrap();
        let err = vault.delete_note(&path).await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    async fn fresh_vault_owned() -> (tempfile::TempDir, Vault) {
        let tmp = tempfile::tempdir().unwrap();
        let vfs = into_dyn(LocalFsVfs::new(tmp.path()).unwrap());
        let vault = Vault::init(vfs).await.unwrap();
        (tmp, vault)
    }

    #[tokio::test]
    async fn snapshot_on_save_records_history() {
        use itala_kernel::history::into_dyn_history;
        use itala_kernel::test_support::MemoryHistoryStore;

        let (_tmp, mut vault) = fresh_vault_owned().await;
        vault.set_history(into_dyn_history(MemoryHistoryStore::new()));

        let created = vault.create_note("H", "v1").await.unwrap();
        let note_id = created.note.id;
        let mut note = created.note;
        let h1 = vault.history(note_id).await.unwrap();
        assert_eq!(h1.len(), 1);

        note.body = Bytes::from_static(b"v2");
        vault.save_note(&note).await.unwrap();
        note.body = Bytes::from_static(b"v3");
        vault.save_note(&note).await.unwrap();

        let h3 = vault.history(note_id).await.unwrap();
        assert_eq!(h3.len(), 3);
    }

    #[tokio::test]
    async fn restore_history_returns_prior_bytes() {
        use itala_kernel::history::into_dyn_history;
        use itala_kernel::test_support::MemoryHistoryStore;

        let (_tmp, mut vault) = fresh_vault_owned().await;
        vault.set_history(into_dyn_history(MemoryHistoryStore::new()));

        let created = vault.create_note("R", "v1").await.unwrap();
        let mut note = created.note;
        let original_snapshot = vault.history(note.id).await.unwrap()[0].id;

        note.body = Bytes::from_static(b"v2");
        vault.save_note(&note).await.unwrap();

        let restored = vault.restore_history(original_snapshot).await.unwrap();
        assert!(restored.windows(2).any(|w| w == b"v1"));
    }

    #[tokio::test]
    async fn no_history_attached_save_still_works() {
        let (_tmp, vault) = fresh_vault().await;
        vault.create_note("Plain", "body").await.unwrap();
        let notes = vault.list_notes().await.unwrap();
        assert_eq!(notes.len(), 1);
        let err = vault.history(notes[0].id).await.unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[tokio::test]
    async fn move_note_rewrites_inbound_links() {
        let (_tmp, vault) = fresh_vault().await;
        // Target note: "alpha".
        let a = vault.create_note("Alpha", "first").await.unwrap();
        // Referrer: body links to [[alpha]] (lowercase slug).
        let b = vault
            .create_note("Referrer", "see [[alpha]] and [[alpha|the alpha]]")
            .await
            .unwrap();

        let dest_ulid = itala_kernel::VaultId::new().to_string();
        let to_path =
            itala_kernel::vfs_types::VfsPath::new(format!("Notes/renamed-{dest_ulid}.md")).unwrap();
        let report = vault.move_note(&a.note.path, &to_path).await.unwrap();
        assert_eq!(report.moved_to, to_path);
        assert!(report.rewritten.contains(&b.note.path));

        // The referrer's body now points at "renamed".
        let updated = vault.read_note(&b.note.path).await.unwrap();
        let text = std::str::from_utf8(&updated.body).unwrap_or("");
        assert!(text.contains("[[renamed]]"), "body was: {text}");
        assert!(text.contains("[[renamed|the alpha]]"), "body was: {text}");
        assert!(!text.contains("[[alpha]]"));
    }

    #[tokio::test]
    async fn move_note_with_no_referrers_yields_empty_rewritten() {
        let (_tmp, vault) = fresh_vault().await;
        let a = vault.create_note("Lonely", "solo").await.unwrap();
        let dest_ulid = itala_kernel::VaultId::new().to_string();
        let to_path =
            itala_kernel::vfs_types::VfsPath::new(format!("Notes/lonely-2-{dest_ulid}.md"))
                .unwrap();
        let report = vault.move_note(&a.note.path, &to_path).await.unwrap();
        assert!(report.rewritten.is_empty());
        // Old path gone, new path exists.
        let notes = vault.list_notes().await.unwrap();
        assert!(notes.iter().any(|n| n.path == to_path));
        assert!(!notes.iter().any(|n| n.path == a.note.path));
    }

    #[tokio::test]
    async fn move_note_preserves_code_fenced_links() {
        let (_tmp, vault) = fresh_vault().await;
        let a = vault.create_note("Alpha", "first").await.unwrap();
        let b = vault
            .create_note("WithCode", "real [[alpha]]\n```\n[[alpha]] in code\n```\n")
            .await
            .unwrap();
        let dest_ulid = itala_kernel::VaultId::new().to_string();
        let to_path =
            itala_kernel::vfs_types::VfsPath::new(format!("Notes/renamed-{dest_ulid}.md")).unwrap();
        vault.move_note(&a.note.path, &to_path).await.unwrap();
        let updated = vault.read_note(&b.note.path).await.unwrap();
        let text = std::str::from_utf8(&updated.body).unwrap_or("");
        // Real one rewritten.
        assert!(text.contains("real [[renamed]]"));
        // Code-fenced one preserved.
        assert!(text.contains("[[alpha]] in code"));
    }

    #[test]
    fn slug_from_path_extracts_slug() {
        let id = itala_kernel::VaultId::new().to_string();
        let p =
            itala_kernel::vfs_types::VfsPath::new(format!("Notes/hello-world-{id}.md")).unwrap();
        assert_eq!(slug_from_path(&p).as_deref(), Some("hello-world"));
    }

    #[test]
    fn slug_from_path_rejects_non_conforming() {
        let p = itala_kernel::vfs_types::VfsPath::new("Notes/just-a-name.md").unwrap();
        assert!(slug_from_path(&p).is_none());
    }

    #[test]
    fn rewrite_links_preserves_non_matching() {
        let out = rewrite_links("[[a]] [[b]] [[a|display]]", "a", "x");
        assert_eq!(out, "[[x]] [[b]] [[x|display]]");
    }

    #[test]
    fn note_id_from_path_round_trips() {
        let id = NoteId::new();
        let s = id.to_string();
        let path_str = format!("Notes/anything-{s}.md");
        let path = itala_kernel::vfs_types::VfsPath::new(&path_str).unwrap();
        let parsed = note_id_from_path(&path).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn note_id_from_lowercase_path_round_trips() {
        let id = NoteId::new();
        let lower = id.to_string().to_lowercase();
        let path_str = format!("Notes/x-{lower}.md");
        let path = itala_kernel::vfs_types::VfsPath::new(&path_str).unwrap();
        let parsed = note_id_from_path(&path).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn note_id_from_path_without_ulid_errors() {
        let path = itala_kernel::vfs_types::VfsPath::new("Notes/just-a-name.md").unwrap();
        let err = note_id_from_path(&path).unwrap_err();
        assert!(matches!(err, Error::InvalidId(_)));
    }
}
