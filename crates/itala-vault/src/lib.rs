//! Itala vault — vault operations over an injected [`itala_kernel::DynVfs`].
//!
//! F4 scope: open/close type-state, frontmatter parsing, note assembly with a
//! stable content-addressed checksum, and recursive note listing.
//!
//! F5 scope: adds [`LocalFsVfs`] — the first production [`itala_kernel::Vfs`]
//! impl, backed by the OS filesystem with atomic writes and debounced watching.
//!
//! F12 scope: introduces the [`itala_kernel::StorageProvider`] seam via
//! [`adapters::FileProvider`] and `NoteStore`, and wires [`Vault`] to
//! delegate all note operations through the store.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod adapters;
mod error;
pub mod frontmatter;
pub mod markdown;
mod note;
mod note_store;
mod vault;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use adapters::{FileProvider, LocalFsVfs};
pub use error::{Error, Result};
pub use frontmatter::{Frontmatter, FrontmatterValue, split as split_frontmatter};
pub use note::Note;
pub use note_store::NoteQuery;
pub use vault::{Closed, Open, Vault};
