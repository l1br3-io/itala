//! Discovery, loading, lifecycle, capability enforcement, and crash isolation
//! for Itala plugins.
//!
//! The host scans directories for `plugin.toml` manifests, verifies API-range
//! compatibility, drives the `initialize → activate → deactivate` lifecycle,
//! mediates every privileged call through the [`itala_kernel`] capability
//! registry, and guarantees that a panicking plugin never crashes the host.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod audit;
pub mod enforcement;
pub mod host;
pub mod lifecycle;
pub mod loader;
pub mod registry;

pub use enforcement::HostContext;
pub use host::{PluginFactory, PluginHost};
pub use lifecycle::PluginStatus;
pub use loader::DiscoveredPlugin;
pub use registry::LoadedPlugin;
