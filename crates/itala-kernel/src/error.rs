//! Kernel error type and `Result` alias.
//!
//! All library crates downstream of `itala-kernel` wrap their internal errors
//! into their own `Error` enum and convert via `#[from] kernel::Error` for
//! transparent propagation.
//!
//! ```
//! use itala_kernel::{Error, NoteId, Result};
//!
//! fn find(id: NoteId) -> Result<()> {
//!     Err(Error::NotFound { id })
//! }
//!
//! let id = NoteId::new();
//! let msg = find(id).unwrap_err().to_string();
//! assert!(msg.contains("not found"));
//! ```

use camino::Utf8PathBuf;
use thiserror::Error;

use crate::id::{IdKind, NoteId, PluginId};

/// The kernel error type.
///
/// Carry structured context in every variant so callers can pattern-match and
/// log with full detail. Display strings are lowercase with no trailing period.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The note with the given ID was not found in the vault.
    #[error("note {id} not found")]
    NotFound {
        /// The ID of the missing note.
        id: NoteId,
    },

    /// An ID string could not be parsed into its expected type.
    #[error("invalid {kind} id: {raw}")]
    InvalidId {
        /// Which kind of ID was expected.
        kind: IdKind,
        /// The raw input that failed to parse.
        raw: String,
    },

    /// A capability name string was syntactically invalid.
    #[error("invalid capability name: {raw}")]
    InvalidCapabilityName {
        /// The raw input that failed to validate.
        raw: String,
    },

    /// A vault path was not relative to the vault root.
    #[error("vault path is not relative to vault root: {path}")]
    VaultPathNotRelative {
        /// The offending path.
        path: Utf8PathBuf,
    },

    /// A plugin attempted to use a capability it was not granted.
    #[error("plugin {plugin} lacks capability {capability}")]
    CapabilityDenied {
        /// The plugin that was denied.
        plugin: PluginId,
        /// The capability that was not granted.
        capability: String,
    },

    /// An I/O error from the underlying operating system.
    #[error("io")]
    Io(#[from] std::io::Error),

    /// An error originating in a storage-provider backend.
    ///
    /// The payload carries the original provider error without requiring
    /// `itala-kernel` to know any backend types.  Construct explicitly —
    /// no `#[from]` because `std::io::Error` already satisfies
    /// `Error + Send + Sync` and the `Io` variant already owns `#[from]`
    /// for it; a second `#[from]` would be a coherence/ambiguity error.
    ///
    /// # Examples
    ///
    /// ```
    /// use itala_kernel::Error;
    ///
    /// let io_err = std::io::Error::other("disk full");
    /// let e = Error::Provider(Box::new(io_err));
    /// assert!(e.to_string().starts_with("provider:"));
    /// ```
    #[error("provider: {0}")]
    Provider(
        /// The upstream provider error.
        Box<dyn std::error::Error + Send + Sync>,
    ),
}

/// Kernel result alias.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoteId, PluginId};

    #[test]
    fn not_found_display() {
        let id = NoteId::new();
        let id_str = id.to_string();
        let msg = Error::NotFound { id }.to_string();
        assert!(msg.contains("not found"));
        assert!(msg.contains(&id_str));
    }

    #[test]
    fn invalid_id_display() {
        let err = Error::InvalidId {
            kind: IdKind::Note,
            raw: "bad-input".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Note"));
        assert!(msg.contains("bad-input"));
    }

    #[test]
    fn vault_path_display() {
        let err = Error::VaultPathNotRelative {
            path: Utf8PathBuf::from("/etc/passwd"),
        };
        let msg = err.to_string();
        assert!(msg.contains("vault root"));
    }

    #[test]
    fn capability_denied_display() {
        let err = Error::CapabilityDenied {
            plugin: PluginId::new("com.example.plugin").unwrap(),
            capability: "host.vfs.read".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("com.example.plugin"));
        assert!(msg.contains("host.vfs.read"));
    }

    #[test]
    fn io_from() {
        let io_err = std::io::Error::other("disk full");
        let e: Error = io_err.into();
        assert!(matches!(e, Error::Io(_)));
    }

    #[test]
    fn provider_display() {
        let io_err = std::io::Error::other("backend unavailable");
        let e = Error::Provider(Box::new(io_err));
        let msg = e.to_string();
        assert!(!msg.is_empty(), "Display must be non-empty");
        assert!(
            msg.contains("provider"),
            "Display must contain 'provider', got: {msg}"
        );
    }
}
