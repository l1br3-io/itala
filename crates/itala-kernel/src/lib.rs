//! Non-replaceable Itala core: shared types, event bus, VFS port, config registry,
//! and the capability system that mediates plugin interactions routed through
//! host APIs (see [ADR-0005](../../docs/adr/0005-plugin-trust-and-rollout.md):
//! capability checks are policy and audit, not a security sandbox for native
//! code; WASM at M5 makes them enforceable for third-party code).
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
pub mod history;
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
pub use history::{DynHistoryStore, HistoryEntry, HistoryStore, IntoDynHistory, into_dyn_history};
pub use id::{BlockId, HistoryId, LinkId, NoteId, PluginId, VaultId};
pub use manifest::{PluginManifest, PluginType};
pub use shutdown::Kernel;
pub use vfs::{DynVfs, IntoDyn, Vfs, into_dyn};
pub use vfs_types::{DirEntry, Metadata, VfsEvent, VfsPath};
