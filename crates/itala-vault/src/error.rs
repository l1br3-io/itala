//! Vault error type and `Result` alias.

use camino::Utf8PathBuf;
use itala_kernel::NoteId;
use thiserror::Error;

/// Errors returned by `itala-vault` operations.
///
/// Library-level error; downstream binaries should bubble these to `anyhow` at
/// the edge.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The vault root does not exist or is not reachable.
    #[error("bad vault root: {path}")]
    BadRoot {
        /// The offending root path.
        path: Utf8PathBuf,
    },

    /// No note with the given id exists in the vault.
    #[error("note {id} not found")]
    NotFound {
        /// The id that was looked up.
        id: NoteId,
    },

    /// A relative path tried to escape the vault root (absolute or `..` segment).
    #[error("path escapes vault root: {path}")]
    PathEscape {
        /// The offending path.
        path: Utf8PathBuf,
    },

    /// YAML frontmatter failed to parse.
    #[error("frontmatter parse failed at line {line}: {message}")]
    Frontmatter {
        /// 1-indexed line within the YAML block (0 if unknown).
        line: usize,
        /// Human-readable cause.
        message: String,
    },

    /// The note body is not valid UTF-8.
    #[error("note body is not valid utf-8")]
    BodyNotUtf8,

    /// A kernel-level error bubbled up.
    #[error(transparent)]
    Kernel(#[from] itala_kernel::Error),

    /// An I/O error from the underlying operating system.
    #[error("io")]
    Io(#[from] std::io::Error),

    /// The `id:` frontmatter field is present but its value cannot be parsed as a [`NoteId`].
    #[error("frontmatter `id:` is not a valid NoteId: {value}")]
    InvalidFrontmatterId {
        /// The raw value that failed to parse.
        value: String,
    },

    /// Two notes in the same vault carry the same `id:` in their frontmatter.
    ///
    /// Raised by FS-backed vault scanners.  The payload fields are
    /// FS-specific context; they are not meaningful for non-FS backends.
    #[error("duplicate note id {id}: found in both {first_fs_location} and {second_fs_location}")]
    DuplicateNoteId {
        /// The repeated id.
        id: NoteId,
        /// First vault-root-relative path where the id was found (FS-backed vaults only).
        first_fs_location: Utf8PathBuf,
        /// Second vault-root-relative path where the same id was found (FS-backed vaults only).
        second_fs_location: Utf8PathBuf,
    },
}

/// Convenience alias for `Result<T, vault::Error>`.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_root_display() {
        let err = Error::BadRoot {
            path: Utf8PathBuf::from("/nope"),
        };
        let msg = err.to_string();
        assert!(msg.contains("bad vault root"));
        assert!(msg.contains("/nope"));
    }

    #[test]
    fn not_found_display() {
        let id = NoteId::new();
        let msg = Error::NotFound { id }.to_string();
        assert!(msg.contains("not found"));
        assert!(msg.contains(&id.to_string()));
    }

    #[test]
    fn path_escape_display() {
        let err = Error::PathEscape {
            path: Utf8PathBuf::from("../etc/passwd"),
        };
        let msg = err.to_string();
        assert!(msg.contains("escapes vault root"));
    }

    #[test]
    fn frontmatter_display() {
        let err = Error::Frontmatter {
            line: 3,
            message: "bad mapping".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("line 3"));
        assert!(msg.contains("bad mapping"));
    }

    #[test]
    fn body_not_utf8_display() {
        let msg = Error::BodyNotUtf8.to_string();
        assert!(msg.contains("utf-8"));
    }

    #[test]
    fn invalid_frontmatter_id_display() {
        let err = Error::InvalidFrontmatterId {
            value: "not-a-ulid".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("not-a-ulid"));
        assert!(msg.contains("NoteId") || msg.contains("id"));
    }

    #[test]
    fn duplicate_note_id_display() {
        let id = NoteId::new();
        let err = Error::DuplicateNoteId {
            id,
            first_fs_location: Utf8PathBuf::from("a.md"),
            second_fs_location: Utf8PathBuf::from("b.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("a.md"));
        assert!(msg.contains("b.md"));
    }

    #[test]
    fn duplicate_note_id_field_names_carry_fs_scope() {
        let id = NoteId::new();
        let err = Error::DuplicateNoteId {
            id,
            first_fs_location: Utf8PathBuf::from("notes/a.md"),
            second_fs_location: Utf8PathBuf::from("notes/b.md"),
        };
        // Field names must be matchable with the fs_location suffix.
        if let Error::DuplicateNoteId {
            first_fs_location,
            second_fs_location,
            ..
        } = err
        {
            assert_eq!(first_fs_location.as_str(), "notes/a.md");
            assert_eq!(second_fs_location.as_str(), "notes/b.md");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn io_from() {
        let e: Error = std::io::Error::other("disk full").into();
        assert!(matches!(e, Error::Io(_)));
    }

    #[test]
    fn kernel_from() {
        let k = itala_kernel::Error::Io(std::io::Error::other("x"));
        let e: Error = k.into();
        assert!(matches!(e, Error::Kernel(_)));
    }
}
