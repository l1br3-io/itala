//! Itala kernel — shared identity types, error vocabulary, capability tokens,
//! VFS abstraction, and event bus.
//!
//! This crate is the foundational vocabulary every other Itala crate speaks.
//!
//! # What lives here
//!
//! - **IDs** — strongly-typed identifiers for every domain entity
//!   ([`NoteId`], [`VaultId`], [`LinkId`], [`BlockId`], [`PluginId`]).
//! - **Errors** — a single non-exhaustive [`Error`] enum that downstream crates
//!   convert into via `#[from]`.
//! - **Capability tokens** — [`Capability`] (unforgeable proof of a permission
//!   grant) and [`CapabilityName`] (its name), with the sealed
//!   [`CapabilityGranter`] trait for authorized minting.
//! - **VFS** — [`Vfs`] (static dispatch) and [`DynVfs`] (object-safe) abstract
//!   over storage backends; [`VfsEvent`] carries raw change notifications.
//! - **Storage provider** — [`StorageProvider`] (static dispatch) and
//!   [`DynStorageProvider`] (object-safe) define the single swappable
//!   persistence seam; [`Record`], [`Filter`], [`WriteOp`], and
//!   [`ProviderCapabilities`] describe the contract.
//! - **Event bus** — [`EventBus`] is a generic broadcast primitive over any
//!   `Clone + Send + 'static` payload. [`KernelEventBus`] aliases it for the
//!   kernel's own [`KernelEvent`] enum.
//! - **Cancellation** — [`CancellationToken`] re-exported for cooperative shutdown.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod capability;
mod error;
pub mod events;
mod id;
pub mod storage;
pub mod vfs;

pub use tokio_util::sync::CancellationToken;

pub use capability::{Capability, CapabilityGranter, CapabilityName};
pub use error::{Error, Result};
pub use events::{EventBus, KernelEvent, KernelEventBus};
pub use id::{BlockId, IdKind, Identity, LinkId, NoteId, PluginId, VaultId};
pub use storage::{
    Condition, DynStorageProvider, Filter, MatchOp, ProviderCapabilities, Record, StorageProvider,
    WriteOp,
};
pub use vfs::{DynVfs, Vfs, VfsEvent};

#[cfg(test)]
mod assertions {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}
    fn assert_identity<T: Identity>() {}

    #[test]
    fn all_ids_satisfy_identity() {
        assert_identity::<NoteId>();
        assert_identity::<VaultId>();
        assert_identity::<LinkId>();
        assert_identity::<BlockId>();
        assert_identity::<PluginId>();
    }

    #[test]
    fn all_public_types_are_send_sync_unpin() {
        // IDs
        assert_send_sync::<NoteId>();
        assert_unpin::<NoteId>();
        assert_send_sync::<VaultId>();
        assert_unpin::<VaultId>();
        assert_send_sync::<LinkId>();
        assert_unpin::<LinkId>();
        assert_send_sync::<BlockId>();
        assert_unpin::<BlockId>();
        assert_send_sync::<PluginId>();
        assert_unpin::<PluginId>();
        // Capability
        assert_send_sync::<CapabilityName>();
        assert_unpin::<CapabilityName>();
        assert_send_sync::<Capability>();
        assert_unpin::<Capability>();
        // Error
        assert_send_sync::<Error>();
        assert_unpin::<Error>();
        // VFS
        assert_send_sync::<VfsEvent>();
        assert_unpin::<VfsEvent>();
        // Events
        assert_send_sync::<KernelEvent>();
        assert_unpin::<KernelEvent>();
        assert_send_sync::<KernelEventBus>();
    }
}
