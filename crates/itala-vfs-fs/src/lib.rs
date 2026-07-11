//! Local-filesystem VFS adapter for Itala.
//!
//! Implements the kernel's `DynVfs` port against the local filesystem using
//! `tokio::fs` for async I/O and `notify` for change watching. All paths are
//! vault-root relative; absolute paths and `..` traversal are rejected.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod adapter;

pub use adapter::LocalFsVfs;
