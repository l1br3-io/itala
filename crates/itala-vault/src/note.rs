//! Note value type, ID derivation, and assembly from raw bytes.

pub mod backfill;

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use itala_kernel::NoteId;
use time::OffsetDateTime;
use ulid::Ulid;

use crate::Frontmatter;
use crate::error::{Error, Result};
use crate::frontmatter;

/// A parsed note snapshot: identity, frontmatter, body, and content addressing.
///
/// `Note` is not `Copy` — moving it is the right semantic. Use `Arc<Note>` if
/// sharing across tasks.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Note {
    /// Stable identifier — authoritative from the `id:` frontmatter field;
    /// derived from the vault-root-relative path and backfilled on first read
    /// for notes that pre-date this field.
    pub id: NoteId,
    /// Vault-root-relative path to the Markdown file, if the backend is FS-shaped.
    ///
    /// `None` for non-FS backends (encrypted-blob, remote, database) where a
    /// filesystem path is not meaningful.  Callers use [`Note::fs_location`].
    pub(crate) path: Option<Utf8PathBuf>,
    /// Parsed YAML frontmatter (empty when none is present).
    pub frontmatter: Frontmatter,
    /// Raw note body — everything after the closing frontmatter fence, as-is.
    pub body: Bytes,
    /// `blake3` hash of the **on-disk** bytes at the time of this read (BOM included if present).
    ///
    /// Computed over the full file buffer, not just the body. Stable across two
    /// reads of identical on-disk content; changes when any byte changes (including
    /// after a backfill write that injects the `id:` field).
    pub checksum: blake3::Hash,
    /// File creation time — `OffsetDateTime::UNIX_EPOCH` until the VFS trait grows
    /// a `metadata` method (TODO: F5+).
    pub created_at: OffsetDateTime,
    /// File modification time — same placeholder as `created_at`.
    pub modified_at: OffsetDateTime,
}

impl Note {
    /// Return the vault-root-relative filesystem path for this note, if the
    /// underlying backend is FS-shaped.
    ///
    /// Always `Some` for [`crate::LocalFsVfs`]-backed vaults.
    /// Returns `None` for future non-FS backends (encrypted-blob, remote,
    /// database) where a filesystem location is not meaningful.
    #[must_use]
    pub fn fs_location(&self) -> Option<&Utf8Path> {
        self.path.as_deref()
    }
}

/// Derive a `NoteId` from a vault-root-relative path.
///
/// Deterministic: `blake3(path_str_bytes)`, first 16 bytes interpreted as a
/// big-endian `u128`, promoted to a `Ulid`.
///
/// This is the **fallback identity** for notes that have not yet had an `id:`
/// field written into their frontmatter.  Once the backfill write occurs, the
/// frontmatter `id:` field is authoritative and this function is no longer
/// consulted for that note.
pub fn id_from_path(path: &Utf8Path) -> NoteId {
    let hash = blake3::hash(path.as_str().as_bytes());
    let bytes: [u8; 16] = hash.as_bytes()[..16]
        .try_into()
        .expect("blake3 is 32 bytes");
    let bits = u128::from_be_bytes(bytes);
    NoteId::from(Ulid::from(bits))
}

/// Read the `id:` frontmatter key and parse it as a [`NoteId`].
///
/// Returns `Ok(Some(id))` when present and well-formed, `Ok(None)` when the
/// key is absent, and `Err(Error::InvalidFrontmatterId)` when present but not
/// parseable as a `NoteId`.
pub fn id_from_frontmatter(fm: &Frontmatter) -> Result<Option<NoteId>> {
    match fm.get("id") {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => s
            .parse::<NoteId>()
            .map(Some)
            .map_err(|_| Error::InvalidFrontmatterId { value: s.clone() }),
        Some(other) => Err(Error::InvalidFrontmatterId {
            value: other.to_string(),
        }),
    }
}

/// Assemble a `Note` from a vault-root-relative `path`, an explicit `id`, the
/// raw `bytes` read from disk, and filesystem timestamps.
///
/// Checksum is over the full `bytes` buffer (BOM included if present). Body is
/// the bytes after the closing frontmatter fence, validated as UTF-8.
///
/// The caller is responsible for resolving the `id` (from the `id:` frontmatter
/// field or from [`id_from_path`] as a fallback); this function does not derive
/// identity itself.
///
/// # Errors
/// - [`Error::Frontmatter`] if YAML parsing fails.
/// - [`Error::BodyNotUtf8`] if the body is not valid UTF-8.
pub fn assemble(
    path: Utf8PathBuf,
    id: NoteId,
    raw_bytes: &Bytes,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,
) -> Result<Note> {
    let checksum = blake3::hash(raw_bytes);
    let (fm, body_slice) = frontmatter::split(raw_bytes)?;
    // Validate body is UTF-8 (body_slice is a sub-slice of raw_bytes).
    std::str::from_utf8(body_slice).map_err(|_| Error::BodyNotUtf8)?;
    let body = Bytes::copy_from_slice(body_slice);
    Ok(Note {
        id,
        path: Some(path),
        frontmatter: fm,
        body,
        checksum,
        created_at,
        modified_at,
    })
}

#[cfg(test)]
mod assertions {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    #[test]
    fn note_is_send_sync_unpin() {
        assert_send_sync::<Note>();
        assert_unpin::<Note>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn id_from_path_is_deterministic() {
        let p = Utf8Path::new("notes/hello.md");
        let a = id_from_path(p);
        let b = id_from_path(p);
        assert_eq!(a, b);
    }

    #[test]
    fn id_from_path_differs_for_distinct_paths() {
        let a = id_from_path(Utf8Path::new("notes/a.md"));
        let b = id_from_path(Utf8Path::new("notes/b.md"));
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // id_from_frontmatter
    // -----------------------------------------------------------------------

    #[test]
    fn id_from_frontmatter_absent_key_returns_none() {
        let fm = Frontmatter::empty();
        assert!(id_from_frontmatter(&fm).unwrap().is_none());
    }

    #[test]
    fn id_from_frontmatter_valid_id_returns_some() {
        let id = NoteId::new();
        let mut fm = Frontmatter::empty();
        fm.insert("id".to_owned(), serde_json::json!(id.to_string()));
        let parsed = id_from_frontmatter(&fm).unwrap().unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn id_from_frontmatter_malformed_string_returns_error() {
        let mut fm = Frontmatter::empty();
        fm.insert("id".to_owned(), serde_json::json!("not-a-ulid"));
        let err = id_from_frontmatter(&fm).unwrap_err();
        assert!(matches!(err, crate::Error::InvalidFrontmatterId { .. }));
    }

    #[test]
    fn id_from_frontmatter_non_string_value_returns_error() {
        let mut fm = Frontmatter::empty();
        fm.insert("id".to_owned(), serde_json::json!(42));
        let err = id_from_frontmatter(&fm).unwrap_err();
        assert!(
            matches!(err, crate::Error::InvalidFrontmatterId { value } if value.contains("42"))
        );
    }

    // -----------------------------------------------------------------------
    // assemble
    // -----------------------------------------------------------------------

    #[test]
    fn assemble_uses_provided_id() {
        let raw = Bytes::from_static(b"---\ntitle: hi\n---\nbody here\n");
        let id = NoteId::new();
        let note = assemble(
            Utf8PathBuf::from("notes/test.md"),
            id,
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert_eq!(note.id, id);
        assert_eq!(
            note.frontmatter.get("title").unwrap(),
            &serde_json::json!("hi")
        );
        assert_eq!(&note.body[..], b"body here\n");
        // Checksum must be stable.
        let c2 = blake3::hash(&raw);
        assert_eq!(note.checksum, c2);
    }

    #[test]
    fn assemble_no_frontmatter() {
        let raw = Bytes::from_static(b"# Title\ntext\n");
        let id = NoteId::new();
        let note = assemble(
            Utf8PathBuf::from("notes/bare.md"),
            id,
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap();
        assert!(note.frontmatter.is_empty());
        assert_eq!(&note.body[..], &raw[..]);
    }

    #[test]
    fn assemble_invalid_utf8_body_returns_error() {
        // Frontmatter header + invalid UTF-8 in body.
        let mut raw: Vec<u8> = b"---\ntitle: ok\n---\n".to_vec();
        raw.push(0xFF);
        raw.push(0xFE);
        let err = assemble(
            Utf8PathBuf::from("notes/bad.md"),
            NoteId::new(),
            &Bytes::from(raw),
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap_err();
        assert!(matches!(err, crate::Error::BodyNotUtf8));
    }

    #[test]
    fn fs_location_returns_some_with_expected_path() {
        let raw = Bytes::from_static(b"---\ntitle: hi\n---\nbody\n");
        let note = assemble(
            Utf8PathBuf::from("notes/test.md"),
            NoteId::new(),
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap();
        let loc = note.fs_location().expect("FS-backed note has a location");
        assert_eq!(loc, Utf8Path::new("notes/test.md"));
    }

    #[test]
    fn fs_location_returns_none_when_path_is_none() {
        // Simulate a non-FS backend: path field is None.
        let raw = Bytes::from_static(b"---\ntitle: hi\n---\nbody\n");
        let (fm, body_slice) = crate::frontmatter::split(&raw).unwrap();
        let note = Note {
            id: NoteId::new(),
            path: None,
            frontmatter: fm,
            body: Bytes::copy_from_slice(body_slice),
            checksum: blake3::hash(&raw),
            created_at: OffsetDateTime::UNIX_EPOCH,
            modified_at: OffsetDateTime::UNIX_EPOCH,
        };
        assert!(note.fs_location().is_none(), "non-FS note must return None");
    }

    proptest! {
        #[test]
        fn checksum_is_stable_across_two_calls(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let raw = Bytes::from(bytes);
            let note = assemble(
                Utf8PathBuf::from("notes/prop.md"),
                NoteId::new(),
                &raw,
                OffsetDateTime::UNIX_EPOCH,
                OffsetDateTime::UNIX_EPOCH,
            );
            if let Ok(note) = note {
                let c2 = blake3::hash(&raw);
                prop_assert_eq!(note.checksum, c2);
            }
        }
    }
}
