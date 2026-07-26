//! The [`KernelEvent`] enum — the typed payload carried on the event bus.
//!
//! Events are the *only* cross-context communication channel (see the
//! architecture doc, §3.2). The bus provides **best-effort delivery to active
//! subscribers**: a slow subscriber that falls behind observes a `Lagged`
//! condition and must reconcile from canonical state. Handlers must therefore
//! be idempotent (duplicate notifications and rebuilds are valid). See
//! [ADR-0007](../../../docs/adrs/0007-best-effort-event-delivery.md).

use smol_str::SmolStr;

use crate::{NoteId, PluginId, VaultId};

/// An event published on the kernel event bus.
///
/// `#[non_exhaustive]` so new variants can be added without a semver break;
/// external crates must include a `_` arm in `match`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum KernelEvent {
    /// A vault was opened. All contexts warm caches in response.
    VaultOpened {
        /// The vault that was opened.
        id: VaultId,
    },
    /// A note was persisted.
    NoteSaved {
        /// The saved note.
        id: NoteId,
    },
    /// A note's content changed (hot path, debounced).
    NoteChanged {
        /// The changed note.
        id: NoteId,
    },
    /// A note was renamed/moved. The graph rewrites backlinks in response.
    NoteRenamed {
        /// The previous identifier.
        from: NoteId,
        /// The new identifier.
        to: NoteId,
    },
    /// A note was deleted.
    NoteDeleted {
        /// The deleted note.
        id: NoteId,
    },
    /// The link graph for a note was updated.
    LinksUpdated {
        /// The note whose links changed.
        id: NoteId,
    },
    /// A plugin finished loading and is active.
    PluginLoaded {
        /// The plugin that loaded.
        id: PluginId,
    },
    /// The search index should be rebuilt.
    IndexRebuildRequested,
    /// A configuration value changed. The `key` is the dotted config path
    /// (e.g. `"editor.tab_width"`).
    ConfigChanged {
        /// The dotted config key that changed.
        key: String,
    },
    /// The vault's storage selection for a derived-state domain changed
    /// (see [ADR-0004](../../../docs/adrs/0004-derived-state-storage-strategy.md) §4).
    /// A live hot-swap is generally not possible — outstanding
    /// `Arc<dyn Dyn*Store>` references keep the old backend alive — so the
    /// typical reaction is to schedule a vault reopen.
    StorageChanged {
        /// The derived-state domain that changed backends
        /// (e.g. `"history"`, `"graph"`, `"search"`).
        domain: SmolStr,
        /// The new backend identifier (e.g. `"files"`, `"sqlite"`).
        backend: SmolStr,
    },
}

const _: () = {
    const fn check<T: Send + Sync + Clone + std::fmt::Debug>() {}
    const E: fn() = check::<KernelEvent>;
    const _: [(); 0] = [(); {
        let _ = E;
        0
    }];
};
