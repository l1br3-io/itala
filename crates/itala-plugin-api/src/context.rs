//! [`PluginContext`] — the host-provided handle plugins use at runtime.
//!
//! Every privileged call that uses this context is checked against the
//! plugin's granted capabilities before delegating to the host; a plugin
//! lacking a capability receives [`itala_kernel::Error::CapabilityDenied`].
//!
//! This is a **policy and audit** choke point, not a security boundary
//! (see [ADR-0005](../../../docs/adrs/0005-plugin-trust-and-rollout.md)):
//! trusted native plugins retain ambient host authority and can bypass the
//! context. WASM components at M5 will make the same checks enforceable
//! because they have no ambient authority.

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::capability::Capability;
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{PluginId, Result};

/// Host-provided runtime handle for a single plugin.
///
/// The host constructs one `PluginContext` per plugin and applies the
/// capability policy/audit check on each method. Plugins are *expected* to
/// route every privileged operation through this context; native plugins,
/// however, retain ambient host authority and can bypass it (see
/// [ADR-0005](../../../docs/adrs/0005-plugin-trust-and-rollout.md)). The M5
/// WASM runtime removes that ambient authority, making the context
/// enforceable for third-party code.
#[async_trait]
pub trait PluginContext: Send + Sync {
    /// Returns the id of the plugin this context belongs to.
    #[must_use]
    fn plugin_id(&self) -> PluginId;

    /// Returns `true` if the plugin currently holds the given capability.
    #[must_use]
    fn has_capability(&self, capability: &Capability) -> bool;

    /// Reads a file from the vault.
    ///
    /// Requires the `vault.read` capability.
    ///
    /// # Errors
    /// [`itala_kernel::Error::CapabilityDenied`] if the capability is missing,
    /// or [`itala_kernel::Error::Io`] / [`itala_kernel::Error::NotFound`] on
    /// read failure.
    async fn read_file(&self, path: &VfsPath) -> Result<Bytes>;

    /// Writes a file to the vault.
    ///
    /// Requires the `vault.write` capability.
    ///
    /// # Errors
    /// [`itala_kernel::Error::CapabilityDenied`] if the capability is missing,
    /// or [`itala_kernel::Error::Io`] on write failure.
    async fn write_file(&self, path: &VfsPath, data: Bytes) -> Result<()>;
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const ASSERT: fn() = check::<Box<dyn PluginContext>>;
    const _: [(); 0] = [(); {
        let _ = ASSERT;
        0
    }];
};
