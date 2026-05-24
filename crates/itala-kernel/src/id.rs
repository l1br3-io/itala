//! Identity types for all Itala domain entities.

pub mod validate;

mod block;
mod link;
mod macros;
mod note;
mod plugin;
mod vault;

pub use block::BlockId;
pub use link::LinkId;
pub(crate) use macros::ulid_id;
pub use note::NoteId;
pub use plugin::PluginId;
pub use vault::VaultId;

use std::fmt;

/// Discriminant for ID parsing errors — tells callers which kind of ID was invalid.
///
/// ```
/// use itala_kernel::IdKind;
///
/// assert_eq!(IdKind::Note.to_string(), "Note");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdKind {
    /// A [`NoteId`].
    Note,
    /// A [`VaultId`].
    Vault,
    /// A [`LinkId`].
    Link,
    /// A [`BlockId`].
    Block,
    /// A [`PluginId`].
    Plugin,
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Note => f.write_str("Note"),
            Self::Vault => f.write_str("Vault"),
            Self::Link => f.write_str("Link"),
            Self::Block => f.write_str("Block"),
            Self::Plugin => f.write_str("Plugin"),
        }
    }
}

/// Contract every identity type in the kernel satisfies.
///
/// Sealed — third-party crates cannot add new identity kinds; new kinds
/// land via additions to this module.
pub trait Identity:
    Clone
    + Eq
    + std::hash::Hash
    + Ord
    + std::fmt::Debug
    + std::fmt::Display
    + std::str::FromStr<Err = crate::error::Error>
    + serde::Serialize
    + serde::de::DeserializeOwned
    + Send
    + Sync
    + Unpin
    + 'static
    + sealed::Sealed
{
    /// Discriminant for routing and error context.
    const KIND: IdKind;
}

pub mod sealed {
    pub trait Sealed {}
}
