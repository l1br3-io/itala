//! The [`Note`] data model (M1 plan, F2).
//!
//! A note is the unit of user content: a UTF-8 Markdown body optionally
//! preceded by YAML frontmatter. The on-disk representation is plain
//! `.md`; the in-memory representation adds a BLAKE3 checksum and identity
//! metadata supplied by the caller (id, path, timestamps).
//!
//! Round-trip contract: for any `bytes` that parses successfully,
//! `Note::from_markdown(bytes)` followed by `.to_markdown()` and another
//! `from_markdown` yields a `Note` equal in body, frontmatter, and checksum.
//! Identity of caller-supplied fields (id/path/timestamps) is the caller's
//! responsibility.

use bytes::Bytes;

use itala_kernel::{NoteId, vfs_types::VfsPath};
use time::OffsetDateTime;

use crate::frontmatter::{Frontmatter, FrontmatterError};
use crate::split::split_frontmatter;

/// Parse error returned by [`Note::from_markdown`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Input bytes were not valid UTF-8.
    #[error("note bytes are not valid UTF-8")]
    NotUtf8,
    /// Frontmatter parse failure.
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterError),
}

/// The in-memory representation of a single Markdown note.
///
/// Caller-supplied identity fields (`id`, `path`, `created_at`, `updated_at`)
/// are *not* serialized into the Markdown projection; they live in the
/// filesystem (path, mtime) or are derived from it (id from filename). This
/// keeps the Markdown projection free of system metadata and makes the
/// round-trip contract trivial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Stable ULID identifier.
    pub id: NoteId,
    /// Canonical vault-relative path.
    pub path: VfsPath,
    /// Parsed YAML frontmatter (empty if absent).
    pub frontmatter: Frontmatter,
    /// Markdown body bytes (UTF-8), excluding frontmatter.
    pub body: Bytes,
    /// Creation time (caller-supplied, typically filesystem btime/mtime).
    pub created_at: OffsetDateTime,
    /// Last-modified time (caller-supplied, typically filesystem mtime).
    pub updated_at: OffsetDateTime,
    /// BLAKE3 checksum of `body`, for integrity and dedup.
    pub checksum: [u8; 32],
}

impl Note {
    /// Constructs a `Note` from already-parsed components, computing the
    /// BLAKE3 checksum of `body`.
    ///
    /// This is the constructor to use when the caller already has the body
    /// bytes and (optionally) a parsed frontmatter in hand and does not need
    /// to round-trip through Markdown. For parsing a `.md` file, use
    /// [`Note::from_markdown`] instead.
    #[must_use]
    pub fn new(
        id: NoteId,
        path: VfsPath,
        frontmatter: Frontmatter,
        body: Bytes,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        let checksum = blake3::hash(&body);
        Self {
            id,
            path,
            frontmatter,
            body,
            created_at,
            updated_at,
            checksum: checksum.into(),
        }
    }

    /// Parses Markdown bytes (optionally frontmatter-prefixed) into a Note.
    ///
    /// The caller supplies identity metadata that lives outside the Markdown
    /// projection. The `bytes` argument is taken by value because parsing
    /// inherently claims the body slice; callers should `clone()` if they
    /// need to retain a handle (cheap — `Bytes` is reference-counted).
    ///
    /// # Errors
    /// [`ParseError::NotUtf8`] if `bytes` is not valid UTF-8; [`ParseError::Frontmatter`]
    /// if a frontmatter block is present but is not a valid YAML mapping.
    #[allow(clippy::needless_pass_by_value)] // Bytes is ref-counted; ownership is the natural API
    pub fn from_markdown(
        id: NoteId,
        path: VfsPath,
        bytes: Bytes,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Result<Self, ParseError> {
        let text = std::str::from_utf8(&bytes).map_err(|_| ParseError::NotUtf8)?;
        let split = split_frontmatter(text);
        let frontmatter = split
            .frontmatter
            .map_or_else(|| Ok(Frontmatter::empty()), Frontmatter::parse)?;
        let body = bytes.slice(split.body_range);
        let checksum = blake3::hash(&body);
        Ok(Self {
            id,
            path,
            frontmatter,
            body,
            created_at,
            updated_at,
            checksum: checksum.into(),
        })
    }

    /// Projects the note back to Markdown bytes.
    ///
    /// Frontmatter (if non-empty) is rendered as `---\n<yaml>\n---\n`,
    /// sorted by key. Body is appended verbatim.
    #[must_use]
    pub fn to_markdown(&self) -> Bytes {
        if self.frontmatter.is_empty() {
            return self.body.clone();
        }
        self.frontmatter.to_yaml().map_or_else(
            |_| self.body.clone(),
            |yaml| {
                let mut out = String::with_capacity(yaml.len() + self.body.len() + 16);
                out.push_str("---\n");
                out.push_str(yaml.trim_end_matches('\n'));
                out.push_str("\n---\n");
                out.push_str(std::str::from_utf8(&self.body).unwrap_or(""));
                Bytes::from(out)
            },
        )
    }

    /// Computes the BLAKE3 checksum of `body`.
    fn recompute_checksum(&self) -> [u8; 32] {
        blake3::hash(&self.body).into()
    }

    /// Verifies that the body matches the recorded checksum.
    ///
    /// Returns `true` if the checksum is consistent. A `false` result
    /// indicates corruption or an inconsistent edit.
    #[must_use]
    pub fn verify_checksum(&self) -> bool {
        self.recompute_checksum() == self.checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn note_path(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    #[test]
    fn parses_body_only() {
        let bytes = Bytes::from_static(b"# Hello\n\nSome text.\n");
        let n = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now()).unwrap();
        assert!(n.frontmatter.is_empty());
        assert_eq!(n.body, Bytes::from_static(b"# Hello\n\nSome text.\n"));
        assert!(n.verify_checksum());
    }

    #[test]
    fn parses_frontmatter_and_body() {
        let bytes = Bytes::from_static(b"---\ntitle: Hi\n---\n# Body\n");
        let n = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now()).unwrap();
        assert_eq!(
            n.frontmatter.get("title").and_then(|v| v.as_str()),
            Some("Hi")
        );
        assert_eq!(n.body, Bytes::from_static(b"# Body\n"));
    }

    #[test]
    fn rejects_non_utf8() {
        let bytes = Bytes::from_static(b"\xff\xfe\xfd");
        let err = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now());
        assert!(matches!(err, Err(ParseError::NotUtf8)));
    }

    #[test]
    fn rejects_top_level_scalar_frontmatter() {
        let bytes = Bytes::from_static(b"---\njust-a-string\n---\nbody\n");
        let err = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now());
        assert!(matches!(err, Err(ParseError::Frontmatter(_))));
    }

    #[test]
    fn body_only_round_trips_byte_for_byte() {
        let body = b"# Hello\n\nSome text with \xc3\xa9 unicode.\n";
        let bytes = Bytes::copy_from_slice(body);
        let n = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now()).unwrap();
        assert_eq!(n.to_markdown(), n.body);
    }

    #[test]
    fn frontmatter_round_trips_semantically() {
        let bytes = Bytes::from_static(b"---\ntitle: Hi\nscore: 7\ntags: [a, b]\n---\n# Body\n");
        let n1 =
            Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now()).unwrap();
        let md = n1.to_markdown();
        let n2 = Note::from_markdown(NoteId::new(), note_path("a.md"), md, now(), now()).unwrap();
        assert_eq!(n1.frontmatter, n2.frontmatter);
        assert_eq!(n1.body, n2.body);
        assert_eq!(n1.checksum, n2.checksum);
    }

    #[test]
    fn three_dashes_inside_body_are_not_frontmatter() {
        // Only an opening `---\n` at offset 0 starts frontmatter.
        let bytes = Bytes::from_static(b"# Title\n\n---\n\nnot frontmatter\n");
        let n = Note::from_markdown(NoteId::new(), note_path("a.md"), bytes, now(), now()).unwrap();
        assert!(n.frontmatter.is_empty());
        assert!(n.body.windows(3).any(|w| w == b"---"));
    }

    #[test]
    fn checksum_changes_when_body_changes() {
        let bytes_a = Bytes::from_static(b"body one");
        let bytes_b = Bytes::from_static(b"body two");
        let n_a =
            Note::from_markdown(NoteId::new(), note_path("a.md"), bytes_a, now(), now()).unwrap();
        let n_b =
            Note::from_markdown(NoteId::new(), note_path("a.md"), bytes_b, now(), now()).unwrap();
        assert_ne!(n_a.checksum, n_b.checksum);
    }

    #[test]
    fn new_computes_checksum_and_round_trips_empty_frontmatter() {
        let body = Bytes::from_static(b"# Hello\n");
        let n = Note::new(
            NoteId::new(),
            note_path("a.md"),
            Frontmatter::empty(),
            body.clone(),
            now(),
            now(),
        );
        assert!(n.frontmatter.is_empty());
        assert_eq!(n.body, body);
        assert!(n.verify_checksum());
        let expected: [u8; 32] = blake3::hash(&body).into();
        assert_eq!(n.checksum, expected);
    }

    #[test]
    fn new_checksum_matches_from_markdown_for_same_body() {
        // For a body-only note, `new` and `from_markdown` must agree on the
        // checksum because both are BLAKE3 of the body bytes.
        let body = Bytes::from_static(b"same body");
        let via_new = Note::new(
            NoteId::new(),
            note_path("a.md"),
            Frontmatter::empty(),
            body.clone(),
            now(),
            now(),
        );
        let via_parse =
            Note::from_markdown(NoteId::new(), note_path("a.md"), body, now(), now()).unwrap();
        assert_eq!(via_new.checksum, via_parse.checksum);
    }
}
