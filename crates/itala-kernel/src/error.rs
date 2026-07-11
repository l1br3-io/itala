//! Typed errors for the Itala kernel and the crate-level error surface that
//! downstream crates build on.
//!
//! The kernel defines a single, `#[non_exhaustive]` [`Error`] enum via
//! [`thiserror`]. Every variant is documented; `?`-propagation works from
//! [`std::io::Error`] via the `#[from]` attribute.
//!
//! Each downstream crate (vault, plugin host, …) is expected to define its
//! own typed error that converts into [`Error`] where appropriate, keeping
//! the error surface structured and part of the public API (see the
//! architecture doc, §7.1).

use thiserror::Error as ThisError;

use crate::{NoteId, PluginId};

/// The crate-level error type for `itala-kernel`.
///
/// `#[non_exhaustive]` so new variants can be added without a semver break;
/// callers must handle the `_` arm in `match`.
///
/// # Example
///
/// `io::Error` propagates with `?` thanks to the `#[from]` conversion:
///
/// ```
/// use std::io;
/// use itala_kernel::Error;
///
/// fn read_len() -> Result<usize, Error> {
///     // imagine real I/O here
///     let _n: usize = io::Read::read(&mut io::empty(), &mut [0u8; 0])?;
///     Ok(_n)
/// }
/// # read_len().unwrap();
/// ```
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
    /// An entity referenced by the given identifier was not found.
    #[error("not found: {id}")]
    NotFound {
        /// A human-readable identifier describing what was missing.
        id: String,
    },

    /// An underlying I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A string could not be parsed into a valid identifier.
    #[error("invalid identifier: {0}")]
    InvalidId(String),

    /// A configuration value was missing, malformed, or violated a schema.
    #[error("config error: {0}")]
    Config(String),

    /// A plugin attempted to use a capability it has not been granted.
    #[error("capability '{capability}' denied for plugin '{plugin}'")]
    CapabilityDenied {
        /// The plugin that was denied.
        plugin: PluginId,
        /// The capability that was required.
        capability: String,
    },

    /// A plugin-reported or plugin-host error not covered by a more specific
    /// variant.
    #[error("plugin error: {0}")]
    Plugin(String),

    /// A VFS path attempted to escape its root (absolute path or `..`
    /// traversal).
    #[error("path escapes vfs root: {0}")]
    PathEscape(String),

    /// A plugin panicked or otherwise crashed and was isolated.
    #[error("plugin '{0}' crashed and was isolated")]
    PluginCrashed(PluginId),

    /// A note-level operation failed (e.g. conflicting note, invalid note
    /// path). Carries the affected note where known.
    #[error("note error for {id}: {reason}")]
    Note {
        /// The affected note.
        id: NoteId,
        /// What went wrong.
        reason: String,
    },
}

impl Error {
    /// Convenience constructor for [`Error::NotFound`].
    #[must_use]
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// Convenience constructor for [`Error::InvalidId`].
    #[must_use]
    pub fn invalid_id(msg: impl Into<String>) -> Self {
        Self::InvalidId(msg.into())
    }

    /// Convenience constructor for [`Error::Config`].
    #[must_use]
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Convenience constructor for [`Error::CapabilityDenied`].
    #[must_use]
    pub fn capability_denied(plugin: impl Into<PluginId>, capability: impl Into<String>) -> Self {
        Self::CapabilityDenied {
            plugin: plugin.into(),
            capability: capability.into(),
        }
    }

    /// Convenience constructor for [`Error::Plugin`].
    #[must_use]
    pub fn plugin(msg: impl Into<String>) -> Self {
        Self::Plugin(msg.into())
    }

    /// Convenience constructor for [`Error::PathEscape`].
    #[must_use]
    pub fn path_escape(msg: impl Into<String>) -> Self {
        Self::PathEscape(msg.into())
    }
}

/// A specialized [`Result`] for kernel operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_propagates_via_from() {
        fn fallible() -> Result<()> {
            Err::<(), std::io::Error>(std::io::Error::other("boom"))?;
            Ok(())
        }
        match fallible() {
            Err(Error::Io(_)) => {}
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn variants_classify() {
        // Every variant is reachable; `#[non_exhaustive]` additionally forces
        // external crates to keep a `_` arm (a cross-crate compile property
        // that can't be exercised from within this crate).
        fn classify(e: &Error) -> &'static str {
            match e {
                Error::NotFound { .. } => "missing",
                Error::Io(_) => "io",
                Error::InvalidId(_) => "bad id",
                Error::Config(_) => "config",
                Error::CapabilityDenied { .. } => "denied",
                Error::Plugin(_) => "plugin",
                Error::PathEscape(_) => "escape",
                Error::PluginCrashed(_) => "crashed",
                Error::Note { .. } => "note",
            }
        }
        assert_eq!(classify(&Error::not_found("x")), "missing");
        assert_eq!(classify(&Error::config("bad")), "config");
        assert_eq!(
            classify(&Error::capability_denied("p", "vault.read")),
            "denied"
        );
        assert_eq!(
            classify(&Error::PluginCrashed(PluginId::new("p"))),
            "crashed"
        );
    }

    #[test]
    fn display_is_human_readable() {
        let e = Error::capability_denied("com.example.p", "network.outbound");
        assert_eq!(
            e.to_string(),
            "capability 'network.outbound' denied for plugin 'com.example.p'"
        );
    }
}
