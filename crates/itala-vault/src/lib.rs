//! Itala vault — read-side vault operations over an injected [`itala_kernel::DynVfs`].
//!
//! F4 scope: open/close type-state, frontmatter parsing, note assembly with a
//! stable content-addressed checksum, and recursive note listing. No writes,
//! deletes, watches, or event publication in this crate's surface — those are
//! later sprint features.
//!
//! The crate has zero direct filesystem access — every byte travels through
//! `Arc<dyn DynVfs>` injected by the caller.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
pub mod frontmatter;
pub mod markdown;
mod note;
mod vault;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use error::{Error, Result};
pub use frontmatter::{Frontmatter, FrontmatterValue, split as split_frontmatter};
pub use note::Note;
pub use vault::{Closed, Open, Vault};
