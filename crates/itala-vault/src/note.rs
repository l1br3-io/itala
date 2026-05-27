//! Note value type, ID derivation, and assembly from raw bytes.

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
    /// Stable content-addressed identifier derived from the vault-root-relative path.
    pub id: NoteId,
    /// Vault-root-relative path to the Markdown file.
    pub path: Utf8PathBuf,
    /// Parsed YAML frontmatter (empty when none is present).
    pub frontmatter: Frontmatter,
    /// Raw note body — everything after the closing frontmatter fence, as-is.
    pub body: Bytes,
    /// `blake3` hash of the **on-disk** bytes (BOM included if present).
    ///
    /// Computed over the full file buffer, not just the body. Stable across two
    /// reads of identical content; changes when any byte changes.
    pub checksum: blake3::Hash,
    /// File creation time — `OffsetDateTime::UNIX_EPOCH` until the VFS trait grows
    /// a `metadata` method (TODO: F5+).
    pub created_at: OffsetDateTime,
    /// File modification time — same placeholder as `created_at`.
    pub modified_at: OffsetDateTime,
}

/// Derive a `NoteId` from a vault-root-relative path.
///
/// Deterministic: `blake3(path_str_bytes)`, first 16 bytes interpreted as a
/// big-endian `u128`, promoted to a `Ulid`.
///
/// # TODO(F7/F10)
/// Rename stability — when history/CRDT lands, IDs migrate to a frontmatter
/// `id:` field with this path-derived value as the default.
pub fn id_from_path(path: &Utf8Path) -> NoteId {
    let hash = blake3::hash(path.as_str().as_bytes());
    let bytes: [u8; 16] = hash.as_bytes()[..16]
        .try_into()
        .expect("blake3 is 32 bytes");
    let bits = u128::from_be_bytes(bytes);
    NoteId::from(Ulid::from(bits))
}

/// Assemble a `Note` from a vault-root-relative `path`, the raw `bytes` read
/// from disk, and filesystem timestamps.
///
/// Checksum is over the full `bytes` buffer (BOM included if present). Body is
/// the bytes after the closing frontmatter fence, validated as UTF-8.
///
/// # Errors
/// - [`Error::Frontmatter`] if YAML parsing fails.
/// - [`Error::BodyNotUtf8`] if the body is not valid UTF-8.
pub fn assemble(
    path: Utf8PathBuf,
    raw_bytes: &Bytes,
    created_at: OffsetDateTime,
    modified_at: OffsetDateTime,
) -> Result<Note> {
    let checksum = blake3::hash(raw_bytes);
    let (fm, body_slice) = frontmatter::split(raw_bytes)?;
    // Validate body is UTF-8 (body_slice is a sub-slice of raw_bytes).
    std::str::from_utf8(body_slice).map_err(|_| Error::BodyNotUtf8)?;
    let body = Bytes::copy_from_slice(body_slice);
    let id = id_from_path(&path);
    Ok(Note {
        id,
        path,
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

    #[test]
    fn assemble_simple_frontmatter_and_body() {
        let raw = Bytes::from_static(b"---\ntitle: hi\n---\nbody here\n");
        let note = assemble(
            Utf8PathBuf::from("notes/test.md"),
            &raw,
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap();
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
        let note = assemble(
            Utf8PathBuf::from("notes/bare.md"),
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
            &Bytes::from(raw),
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH,
        )
        .unwrap_err();
        assert!(matches!(err, Error::BodyNotUtf8));
    }

    proptest! {
        #[test]
        fn checksum_is_stable_across_two_calls(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let raw = Bytes::from(bytes);
            let note = assemble(
                Utf8PathBuf::from("notes/prop.md"),
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
