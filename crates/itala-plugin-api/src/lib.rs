//! The stable, semver'd surface that Itala plugins compile against.
//!
//! Plugins implement the [`Plugin`] trait and interact with the host
//! exclusively through a [`PluginContext`]. Breaking changes bump the major
//! version and coexist side-by-side so old plugins keep loading.
//!
//! **Versioning contract:** minor versions are backwards-compatible; a major
//! bump signals an ABI break. The host negotiates compatibility via the
//! plugin manifest's `api_range` and the value exposed by [`api_version()`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod context;
pub mod plugin;
pub mod version;

pub use context::PluginContext;
pub use plugin::Plugin;
pub use version::api_version;
