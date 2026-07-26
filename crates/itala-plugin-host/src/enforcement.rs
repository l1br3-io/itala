//! [`HostContext`] — the host's [`PluginContext`] implementation that applies
//! the capability policy/audit check to privileged calls routed through it.
//!
//! This is a **policy and audit** boundary, not a security sandbox (see
//! [ADR-0005](../../../docs/adr/0005-plugin-trust-and-rollout.md)): native
//! plugins can bypass `PluginContext` and access the host directly. WASM at
//! M5 will make the same checks enforceable by giving third-party code no
//! ambient authority.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::capability::{Capability, CapabilityRegistry};
use itala_kernel::vfs::DynVfs;
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Error, PluginId, Result};
use itala_plugin_api::PluginContext;

use crate::audit;

/// Host-side context handed to a plugin during lifecycle calls.
///
/// Every privileged method checks the plugin's granted capabilities against
/// the shared [`CapabilityRegistry`] and delegates to the shared VFS. Denials
/// are audit-logged and surface as [`Error::CapabilityDenied`].
#[derive(Clone)]
pub struct HostContext {
    plugin: PluginId,
    caps: Arc<CapabilityRegistry>,
    vfs: Arc<dyn DynVfs>,
}

impl HostContext {
    /// Creates a new context for `plugin`.
    #[must_use]
    pub fn new(plugin: PluginId, caps: Arc<CapabilityRegistry>, vfs: Arc<dyn DynVfs>) -> Self {
        Self { plugin, caps, vfs }
    }

    /// Checks a capability, returning `Ok(())` if held or a typed denial error.
    fn require(&self, capability: &Capability) -> Result<()> {
        if self.caps.has(&self.plugin, capability) {
            audit::used(&self.plugin, capability.as_str());
            Ok(())
        } else {
            audit::denied(&self.plugin, capability.as_str());
            Err(Error::capability_denied(
                self.plugin.clone(),
                capability.as_str(),
            ))
        }
    }
}

#[async_trait]
impl PluginContext for HostContext {
    fn plugin_id(&self) -> PluginId {
        self.plugin.clone()
    }

    fn has_capability(&self, capability: &Capability) -> bool {
        self.caps.has(&self.plugin, capability)
    }

    async fn read_file(&self, path: &VfsPath) -> Result<Bytes> {
        let _span = audit::call_span(&self.plugin, "read_file");
        self.require(&Capability::vault_read())?;
        self.vfs.read(path).await
    }

    async fn write_file(&self, path: &VfsPath, data: Bytes) -> Result<()> {
        let _span = audit::call_span(&self.plugin, "write_file");
        self.require(&Capability::vault_write())?;
        self.vfs.write(path, data).await
    }
}
