//! Storage backend adapters.
//!
//! Ships two adapters:
//! - [`LocalFsVfs`] — local OS filesystem [`itala_kernel::Vfs`] implementation.
//! - [`FileProvider`] — FS-backed [`itala_kernel::StorageProvider`] over any [`itala_kernel::DynVfs`].

pub mod file;
mod local;

pub use file::FileProvider;
pub use local::LocalFsVfs;
