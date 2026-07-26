//! Discovery, loading, lifecycle, capability policy/audit, and panic
//! containment for Itala plugins.
//!
//! The host scans directories for `plugin.toml` manifests, verifies API-range
//! compatibility, drives the `initialize → activate → deactivate` lifecycle,
//! applies the capability policy/audit check to privileged calls routed
//! through `PluginContext`, and contains ordinary lifecycle-task panics.
//! Native M0 plugins are trusted in-process components: the capability checks
//! here are policy and audit, **not** a security sandbox, and panic
//! containment is resilience to ordinary bugs rather than isolation of
//! malicious code. WASM at M5 will make both guarantees enforceable for
//! third-party code (see
//! [ADR-0005](../../../docs/adrs/0005-plugin-trust-and-rollout.md)).

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
