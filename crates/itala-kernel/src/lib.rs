//! Non-replaceable Itala core: shared types, event bus, VFS port, config registry,
//! and the capability system that mediates all plugin interactions.
//!
//! Every other Itala crate depends on this one. The kernel is intentionally
//! small — see the [architecture doc](https://github.com/itala/itala/blob/main/docs/ARCHITECTURE.md#3-the-kernel)
//! for the design.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bus;
pub mod capability;
pub mod config;
pub mod error;
pub mod events;
pub mod id;
pub mod manifest;
pub mod persistence;
pub mod shutdown;
pub mod vfs;
pub mod vfs_types;

#[cfg(feature = "test-support")]
pub mod test_support;

pub use bus::{EventBus, EventSubscriber};
pub use capability::Capability;
pub use config::ConfigRegistry;
pub use error::{Error, Result};
pub use events::KernelEvent;
pub use id::{BlockId, LinkId, NoteId, PluginId, VaultId};
pub use manifest::{PluginManifest, PluginType};
pub use shutdown::Kernel;
pub use vfs::{DynVfs, IntoDyn, Vfs, into_dyn};
pub use vfs_types::{DirEntry, Metadata, VfsEvent, VfsPath};
