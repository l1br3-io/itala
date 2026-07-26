//! Bounded context for vault lifecycle, note CRUD, and link rewriting.
//!
//! Depends only on `itala-kernel` (the dependency rule, ARCHITECTURE.md §2).
//! Communicates with other contexts solely through the kernel event bus.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod notes;
pub mod slug;
pub mod vault;
pub mod wiki_link;

pub use notes::{CreatedNote, NoteMetadata, note_id_from_path};
pub use slug::slugify;
pub use vault::Vault;
pub use wiki_link::{WikiLink, scan as scan_wiki_links};
