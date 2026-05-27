//! Storage backend adapters.
//!
//! Currently ships one adapter: [`LocalFsVfs`] for the local OS filesystem.

mod local;

pub use local::LocalFsVfs;
