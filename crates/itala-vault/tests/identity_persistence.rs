//! Integration tests for persistent `NoteId` via frontmatter `id:` field (F5.5).
//!
//! These tests use the `MemoryVfs` so they run fast and in-process.  The key
//! invariants exercised:
//!
//! 1. A note without `id:` gets backfilled on the first read/list.
//! 2. The same `NoteId` is returned before and after backfill.
//! 3. Moving (renaming) a note file does not change its `NoteId` once `id:` is set.
//! 4. Two notes with the same `id:` trigger `DuplicateNoteId`.
//! 5. A malformed `id:` triggers `InvalidFrontmatterId`.

use camino::Utf8PathBuf;
use itala_kernel::DynVfs;
use itala_kernel::NoteId;
use itala_vault::test_support::memory_vault;
use itala_vault::{Closed, Open, Vault};
use itala_vault::{Error, test_support::MemoryVfs};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a vault from `(rel_path, raw_bytes)` pairs. Root is `/vault`.
async fn build(notes: &[(&str, &[u8])]) -> Vault<Open> {
    let pairs: Vec<(Utf8PathBuf, &[u8])> = notes
        .iter()
        .map(|(p, b)| (Utf8PathBuf::from(*p), *b))
        .collect();
    memory_vault(&pairs).await
}

// ---------------------------------------------------------------------------
// Backfill on first listing/read
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_backfills_id_into_note_without_id() {
    let vfs = MemoryVfs::default();
    let root = Utf8PathBuf::from("/vault");
    let raw = b"---\ntitle: Alpha\n---\nAlpha body.\n";
    vfs.put("/vault/alpha.md", raw);

    let arc: Arc<dyn DynVfs> = Arc::new(vfs.clone());
    let vault = Vault::<Closed>::new(arc, root).open().await.unwrap();

    // First list: triggers backfill.
    let ids = vault.list().await.unwrap();
    assert_eq!(ids.len(), 1);

    // The file on disk should now contain `id:`.
    let written = vfs.get("/vault/alpha.md").expect("file must still exist");
    let written_str = std::str::from_utf8(&written).unwrap();
    assert!(
        written_str.contains("id:"),
        "backfill should have written `id:` into the file"
    );
}

#[tokio::test]
async fn read_returns_same_id_before_and_after_backfill() {
    let vfs = MemoryVfs::default();
    let root = Utf8PathBuf::from("/vault");
    vfs.put("/vault/alpha.md", b"---\ntitle: Alpha\n---\nAlpha body.\n");

    let arc: Arc<dyn DynVfs> = Arc::new(vfs.clone());
    let vault = Arc::new(
        Vault::<Closed>::new(Arc::clone(&arc), root.clone())
            .open()
            .await
            .unwrap(),
    );

    // First list — no `id:` yet.  Derive from path, backfill.
    let listing_one = vault.list().await.unwrap();
    assert_eq!(listing_one.len(), 1);
    let stable_id = listing_one[0];

    // Second list — `id:` now present in the file. Must return the same id.
    let listing_two = vault.list().await.unwrap();
    assert_eq!(listing_two.len(), 1);
    assert_eq!(
        stable_id, listing_two[0],
        "id must be stable across two list calls"
    );
}

#[tokio::test]
async fn id_survives_rename() {
    // Simulate rename by writing the same bytes (with `id:`) under a new path.
    let vfs = MemoryVfs::default();
    let root = Utf8PathBuf::from("/vault");
    vfs.put("/vault/original.md", b"---\ntitle: T\n---\nbody\n");

    let arc: Arc<dyn DynVfs> = Arc::new(vfs.clone());
    let vault = Vault::<Closed>::new(Arc::clone(&arc), root.clone())
        .open()
        .await
        .unwrap();

    // List triggers backfill.
    let pre_rename = vault.list().await.unwrap();
    assert_eq!(pre_rename.len(), 1);
    let original_id = pre_rename[0];

    // Grab the backfilled bytes, "rename" (copy to new path, delete old).
    let backfilled = vfs.get("/vault/original.md").unwrap();
    vfs.put("/vault/renamed.md", &backfilled);
    vfs.remove("/vault/original.md");

    // Re-open the vault (same VFS state) and list again.
    let vault2 = Vault::<Closed>::new(arc, root).open().await.unwrap();
    let ids_after = vault2.list().await.unwrap();
    assert_eq!(ids_after.len(), 1);
    assert_eq!(
        original_id, ids_after[0],
        "id must survive rename when `id:` is already in frontmatter"
    );
}

// ---------------------------------------------------------------------------
// Body preservation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn backfill_preserves_body_bytes() {
    let vault = build(&[("note.md", b"---\ntitle: T\n---\nbody content here\n")]).await;
    let ids = vault.list().await.unwrap();
    let note = vault.read(ids[0]).await.unwrap();
    assert_eq!(&note.body[..], b"body content here\n");
}

#[tokio::test]
async fn backfill_preserves_body_when_no_frontmatter() {
    let vault = build(&[("bare.md", b"# No frontmatter\n\nJust text.\n")]).await;
    let ids = vault.list().await.unwrap();
    let note = vault.read(ids[0]).await.unwrap();
    assert_eq!(&note.body[..], b"# No frontmatter\n\nJust text.\n");
}

// ---------------------------------------------------------------------------
// Checksum stability
// ---------------------------------------------------------------------------

#[tokio::test]
async fn checksum_stable_across_two_reads_after_backfill() {
    let vault = build(&[("note.md", b"---\ntitle: hi\n---\nbody\n")]).await;
    let ids = vault.list().await.unwrap();
    let id = ids[0];

    let note1 = vault.read(id).await.unwrap();
    let note2 = vault.read(id).await.unwrap();
    assert_eq!(
        note1.checksum, note2.checksum,
        "checksum must be stable once the file is backfilled"
    );
}

// ---------------------------------------------------------------------------
// Duplicate id detection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn duplicate_note_id_returns_error_from_list() {
    let shared_id = NoteId::new();
    let note_a = format!("---\nid: \"{shared_id}\"\n---\nbody a\n");
    let note_b = format!("---\nid: \"{shared_id}\"\n---\nbody b\n");

    let vault = build(&[("a.md", note_a.as_bytes()), ("b.md", note_b.as_bytes())]).await;

    let err = vault.list().await.unwrap_err();
    assert!(
        matches!(err, Error::DuplicateNoteId { .. }),
        "expected DuplicateNoteId, got: {err}"
    );
}

#[tokio::test]
async fn duplicate_note_id_returns_error_from_fs_location_of() {
    let shared_id = NoteId::new();
    let note_a = format!("---\nid: \"{shared_id}\"\n---\nbody a\n");
    let note_b = format!("---\nid: \"{shared_id}\"\n---\nbody b\n");

    let vault = build(&[("a.md", note_a.as_bytes()), ("b.md", note_b.as_bytes())]).await;

    // fs_location_of scans all paths and detects the duplicate for the target id.
    let err = vault.fs_location_of(shared_id).await.unwrap_err();
    assert!(
        matches!(err, Error::DuplicateNoteId { .. }),
        "expected DuplicateNoteId, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Malformed id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_id_in_frontmatter_returns_invalid_frontmatter_id() {
    let vault = build(&[("bad.md", b"---\nid: \"not-a-valid-ulid\"\n---\nbody\n")]).await;
    let err = vault.list().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidFrontmatterId { .. }),
        "expected InvalidFrontmatterId, got: {err}"
    );
}

#[tokio::test]
async fn non_string_id_in_frontmatter_returns_invalid_frontmatter_id() {
    let vault = build(&[("bad.md", b"---\nid: 42\n---\nbody\n")]).await;
    let err = vault.list().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidFrontmatterId { .. }),
        "expected InvalidFrontmatterId, got: {err}"
    );
}
