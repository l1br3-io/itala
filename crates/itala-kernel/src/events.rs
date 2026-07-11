//! The [`KernelEvent`] enum — the typed payload carried on the event bus.
//!
//! Events are the *only* cross-context communication channel (see the
//! architecture doc, §3.2). The bus provides at-least-once delivery; handlers
//! must therefore be idempotent.

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
}

const _: () = {
    const fn check<T: Send + Sync + Clone + std::fmt::Debug>() {}
    const E: fn() = check::<KernelEvent>;
    const _: [(); 0] = [(); {
        let _ = E;
        0
    }];
};
