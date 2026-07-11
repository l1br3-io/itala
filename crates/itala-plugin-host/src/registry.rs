//! Loaded-plugin bookkeeping used by the host.

use std::sync::Arc;

use itala_kernel::PluginId;
use itala_kernel::manifest::PluginManifest;
use itala_plugin_api::Plugin;

use crate::lifecycle::PluginStatus;

/// A loaded plugin: manifest, instance, and current status.
pub struct LoadedPlugin {
    /// The parsed manifest.
    pub manifest: PluginManifest,
    /// The plugin instance.
    pub instance: Arc<dyn Plugin>,
    /// Current lifecycle status.
    pub status: PluginStatus,
}

impl LoadedPlugin {
    /// Creates a new entry in the `Inactive` state.
    pub fn new(manifest: PluginManifest, instance: Arc<dyn Plugin>) -> Self {
        Self {
            manifest,
            instance,
            status: PluginStatus::Inactive,
        }
    }

    /// The plugin id.
    #[must_use]
    pub fn id(&self) -> PluginId {
        self.instance.id()
    }
}
