//! Test-support helpers — only available with the `test-support` feature.
//!
//! The headline exports are [`MemoryVfs`] and [`MemoryHistoryStore`]:
//! reference implementations of the kernel's ports used throughout the
//! kernel's and downstream crates' unit tests. The shared port-conformance
//! suite for VFS lives in [`conformance`].

pub mod conformance;
mod memory_history;
mod memory_vfs;

pub use memory_history::MemoryHistoryStore;
pub use memory_vfs::MemoryVfs;
