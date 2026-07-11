//! [`PluginContext`] — the host-provided handle plugins use at runtime.
//!
//! Every privileged call goes through the context, which checks the plugin's
//! granted capabilities before delegating to the host. A plugin that lacks a
//! capability receives [`itala_kernel::Error::CapabilityDenied`].

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::capability::Capability;
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{PluginId, Result};

/// Host-provided runtime handle for a single plugin.
///
/// The host constructs one `PluginContext` per plugin and enforces capability
/// checks on each method. Plugins never receive ambient host access — only
/// what the context exposes.
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
