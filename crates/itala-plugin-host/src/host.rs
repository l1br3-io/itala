//! [`PluginHost`] — discovery, loading, lifecycle, and capability policy/audit.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use itala_kernel::bus::EventBus;
use itala_kernel::capability::{Capability, CapabilityRegistry};
use itala_kernel::events::KernelEvent;
use itala_kernel::manifest::PluginManifest;
use itala_kernel::vfs::DynVfs;
use itala_kernel::{Error, PluginId, Result};
use itala_plugin_api::Plugin;

use crate::enforcement::HostContext;
use crate::lifecycle::{self, PluginStatus};
use crate::loader::{DiscoveredPlugin, discover};
use crate::registry::LoadedPlugin;

/// A factory that instantiates a plugin instance.
pub type PluginFactory = Box<dyn Fn() -> Box<dyn Plugin> + Send + Sync>;

/// The plugin host: discovers, loads, drives lifecycle, and applies the
/// capability policy/audit check to privileged calls routed through
/// `PluginContext`.
///
/// Native M0 plugins are trusted and can bypass this policy; WASM at M5
/// makes it enforceable for third-party code
/// (see [ADR-0005](../../../docs/adrs/0005-plugin-trust-and-rollout.md)).
pub struct PluginHost {
    bus: EventBus,
    caps: Arc<CapabilityRegistry>,
    vfs: Arc<dyn DynVfs>,
    factories: HashMap<PluginId, PluginFactory>,
    plugins: HashMap<PluginId, LoadedPlugin>,
}

impl PluginHost {
    /// Creates a new host backed by the given bus, capability registry, and VFS.
    #[must_use]
    pub fn new(bus: EventBus, caps: Arc<CapabilityRegistry>, vfs: Arc<dyn DynVfs>) -> Self {
        Self {
            bus,
            caps,
            vfs,
            factories: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    /// Registers a factory that instantiates the plugin identified by `id`.
    pub fn register_factory<F>(&mut self, id: impl Into<PluginId>, factory: F)
    where
        F: Fn() -> Box<dyn Plugin> + Send + Sync + 'static,
    {
        self.factories.insert(id.into(), Box::new(factory));
    }

    /// Discovers and loads every plugin found under `dir`, driving each through
    /// `initialize` and `activate`. Incompatible or unregistered plugins are
    /// skipped (logged); a plugin that errors or panics during load is marked
    /// `Failed` and the rest still load.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the directory cannot be read.
    pub async fn discover_and_load(&mut self, dir: &Path) -> Result<()> {
        let discovered = discover(dir)?;
        for found in discovered {
            self.load_one(found).await;
        }
        Ok(())
    }

    /// Loads a single discovered plugin, isolating failures from the rest.
    async fn load_one(&mut self, found: DiscoveredPlugin) {
        let id = found.manifest.id.clone();

        if !found.manifest.is_compatible() {
            tracing::warn!(plugin = %id, api_range = %found.manifest.api_range, "incompatible plugin, skipping");
            return;
        }

        let Some(factory) = self.factories.get(&id) else {
            tracing::warn!(plugin = %id, "no factory registered, skipping");
            return;
        };

        let instance: Arc<dyn Plugin> = Arc::from(factory());
        let ctx = HostContext::new(id.clone(), Arc::clone(&self.caps), Arc::clone(&self.vfs));

        // Grant the manifest-declared capabilities (M0 models user consent as
        // automatic at install time).
        for cap in &found.manifest.capabilities {
            self.caps.grant(id.clone(), cap.clone());
            crate::audit::grant(&id, cap.as_str());
        }

        let mut entry = LoadedPlugin::new(found.manifest.clone(), instance.clone());

        let init = lifecycle::initialize(Arc::clone(&instance), ctx.clone()).await;
        if let Err(e) = &init {
            entry.status = PluginStatus::Failed(e.to_string());
            self.plugins.insert(id, entry);
            return;
        }

        let act = lifecycle::activate(Arc::clone(&instance), ctx).await;
        match act {
            Ok(()) => {
                entry.status = PluginStatus::Active;
                self.bus
                    .publish(KernelEvent::PluginLoaded { id: id.clone() });
            }
            Err(e) => {
                entry.status = PluginStatus::Failed(e.to_string());
            }
        }
        self.plugins.insert(id, entry);
    }

    /// Gracefully deactivates every active plugin, used on shutdown.
    ///
    /// Deactivation errors (including crashes) are logged but never abort the
    /// loop, so one bad plugin cannot hang shutdown.
    pub async fn shutdown(&mut self) {
        let ids: Vec<PluginId> = self.plugins.keys().cloned().collect();
        for id in ids {
            let Some(entry) = self.plugins.get(&id) else {
                continue;
            };
            if !entry.status.is_active() {
                continue;
            }
            let instance = Arc::clone(&entry.instance);
            let ctx = HostContext::new(id.clone(), Arc::clone(&self.caps), Arc::clone(&self.vfs));
            match lifecycle::deactivate(instance, ctx).await {
                Ok(()) => {
                    if let Some(e) = self.plugins.get_mut(&id) {
                        e.status = PluginStatus::Inactive;
                    }
                }
                Err(e) => {
                    tracing::warn!(plugin = %id, error = %e, "deactivation failed");
                    if let Some(en) = self.plugins.get_mut(&id) {
                        en.status = PluginStatus::Failed(e.to_string());
                    }
                }
            }
        }
    }

    /// Returns the ids and statuses of all loaded plugins.
    #[must_use]
    pub fn list(&self) -> Vec<(PluginId, PluginStatus)> {
        self.plugins
            .iter()
            .map(|(id, e)| (id.clone(), e.status.clone()))
            .collect()
    }

    /// Returns a [`HostContext`] for `plugin`, if loaded.
    #[must_use]
    pub fn context_for(&self, plugin: &PluginId) -> Option<HostContext> {
        if self.plugins.contains_key(plugin) {
            Some(HostContext::new(
                plugin.clone(),
                Arc::clone(&self.caps),
                Arc::clone(&self.vfs),
            ))
        } else {
            None
        }
    }

    /// Returns `true` if the plugin is currently active.
    #[must_use]
    pub fn is_active(&self, plugin: &PluginId) -> bool {
        self.plugins
            .get(plugin)
            .is_some_and(|e| e.status.is_active())
    }

    /// Returns the manifest for `plugin`, if loaded.
    #[must_use]
    pub fn manifest(&self, plugin: &PluginId) -> Option<&PluginManifest> {
        self.plugins.get(plugin).map(|e| &e.manifest)
    }

    /// Revokes a capability at runtime; the next privileged call by that
    /// plugin is denied.
    pub fn revoke(&self, plugin: &PluginId, capability: &Capability) {
        self.caps.revoke(plugin, capability);
    }

    /// Returns a reference to the shared capability registry.
    #[must_use]
    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.caps
    }

    /// Loads a plugin directly from an already-parsed manifest and a registered
    /// factory — used by tests that don't need directory discovery.
    ///
    /// # Errors
    /// [`Error::NotFound`] if no factory is registered for the manifest's id.
    pub async fn load_from_manifest(&mut self, manifest: PluginManifest) -> Result<()> {
        let id = manifest.id.clone();
        let dir = std::env::temp_dir().join(format!("itala-inmem-{id}"));
        let _ = std::fs::create_dir_all(&dir);
        self.load_one(DiscoveredPlugin { manifest, dir }).await;
        if self.plugins.contains_key(&id) {
            Ok(())
        } else {
            Err(Error::not_found(format!("plugin {id} did not load")))
        }
    }
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const H: fn() = check::<PluginHost>;
    const _: [(); 0] = [(); {
        let _ = H;
        0
    }];
};
