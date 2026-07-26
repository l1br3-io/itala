//! JSON-on-Vfs adapter for derived-state storage ports.
//!
//! Implements every kernel port (currently just `DynHistoryStore`; grows with
//! M3's graph and M4's search-meta) by projecting entries as flat files under
//! `.itala/<domain>/`. See ADR-0004 §3 for the rationale — this is the default
//! backend, generalizing what `itala_kernel::persistence` already does for
//! kernel-internal state.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod history;

pub use history::FilesHistoryStore;
