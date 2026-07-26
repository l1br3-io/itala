//! Bounded context for the note data model.
//!
//! Holds the plain-Markdown-with-frontmatter representation used by M1.
//! The CRDT body field lands with the editor milestone (M2); see the
//! M1 plan, AD-1.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod frontmatter;
pub mod note;
pub mod split;

pub use frontmatter::{Frontmatter, FrontmatterError};
pub use note::{Note, ParseError};
