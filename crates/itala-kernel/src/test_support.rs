//! Test-support helpers — only available with the `test-support` feature.
//!
//! The headline export is [`MemoryVfs`]: a reference [`Vfs`] implementation
//! used throughout the kernel's and downstream crates' unit tests. The shared
//! port-conformance suite lives in [`conformance`].

pub mod conformance;
mod memory_vfs;

pub use memory_vfs::MemoryVfs;
