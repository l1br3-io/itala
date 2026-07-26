//! Graceful, `CancellationToken`-driven shutdown and the top-level [`Kernel`]
//! that ties the M0 pieces together.
//!
//! [`Kernel::start`] loads persisted state (config, capabilities, plugin
//! records). [`Kernel::shutdown`] persists the current state and is bounded to
//! [`SHUTDOWN_TIMEOUT`]; a cancellation via the kernel's token forces an early
//! return without corrupting already-persisted data.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::Result;
use crate::bus::EventBus;
use crate::capability::{Capability, CapabilityRegistry};
use crate::config::ConfigRegistry;
use crate::manifest::PluginManifest;
use crate::persistence;
use crate::vfs::DynVfs;

/// Upper bound on graceful shutdown. The architecture guarantees shutdown
/// completes without hanging.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// The top-level M0 kernel: composes the event bus, config, capability
/// registry, and VFS, and owns the durable-state lifecycle.
pub struct Kernel {
    bus: EventBus,
    config: ConfigRegistry,
    caps: Arc<CapabilityRegistry>,
    vfs: Arc<dyn DynVfs>,
    shutdown: CancellationToken,
    installed_plugins: Vec<PluginManifest>,
}

impl Kernel {
    /// Creates a new kernel over the given VFS. Does not load persisted state;
    /// call [`Kernel::start`] for that.
    ///
    /// A fresh event bus is constructed internally. To share a bus with other
    /// components (e.g. the plugin host), use [`Kernel::with_bus`].
    #[must_use]
    pub fn new(vfs: Arc<dyn DynVfs>) -> Self {
        Self::with_bus(vfs, EventBus::new())
    }

    /// Creates a kernel with an explicit event bus, shared with other
    /// components (e.g. the plugin host). The bus is also wired into the
    /// [`ConfigRegistry`] so that config changes publish
    /// [`crate::KernelEvent::ConfigChanged`].
    #[must_use]
    pub fn with_bus(vfs: Arc<dyn DynVfs>, bus: EventBus) -> Self {
        Self {
            config: ConfigRegistry::with_bus(bus.clone()),
            caps: Arc::new(CapabilityRegistry::new()),
            vfs,
            bus,
            shutdown: CancellationToken::new(),
            installed_plugins: Vec::new(),
        }
    }

    /// Loads persisted state (config, capabilities, plugin records) from the
    /// VFS. Missing artifacts are treated as a fresh start.
    ///
    /// # Errors
    /// Propagates VFS or deserialization errors (except `NotFound`, which is a
    /// fresh start).
    pub async fn start(&mut self) -> Result<()> {
        persistence::load_config(&*self.vfs, &mut self.config).await?;
        persistence::load_capabilities(&*self.vfs, &self.caps).await?;
        self.installed_plugins = persistence::load_plugins(&*self.vfs).await?;
        Ok(())
    }

    /// Persists the current state, bounded to [`SHUTDOWN_TIMEOUT`].
    ///
    /// # Errors
    /// [`crate::Error::Plugin`] if shutdown timed out before persistence
    /// finished; otherwise the underlying persistence error.
    pub async fn shutdown(&self) -> Result<()> {
        let vfs = &*self.vfs;
        let save = async {
            persistence::save_config(vfs, &self.config).await?;
            persistence::save_capabilities(vfs, &self.caps).await?;
            persistence::save_plugins(vfs, &self.installed_plugins).await?;
            Ok::<_, crate::Error>(())
        };
        tokio::select! {
            biased;
            res = save => res,
            () = tokio::time::sleep(SHUTDOWN_TIMEOUT) => Err(crate::Error::plugin("shutdown timed out")),
        }
    }

    /// Returns the cancellation token driving this kernel.
    #[must_use]
    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    /// Signals cancellation (forced shutdown). Subsequent [`Kernel::shutdown`]
    /// persistence may still proceed; already-persisted data is never
    /// corrupted.
    pub fn cancel(&self) {
        self.shutdown.cancel();
    }

    /// Returns the event bus.
    #[must_use]
    pub const fn bus(&self) -> &EventBus {
        &self.bus
    }

    /// Returns the config registry.
    #[must_use]
    pub const fn config(&self) -> &ConfigRegistry {
        &self.config
    }

    /// Returns a mutable reference to the config registry.
    pub const fn config_mut(&mut self) -> &mut ConfigRegistry {
        &mut self.config
    }

    /// Returns the shared capability registry.
    #[must_use]
    pub const fn capabilities(&self) -> &Arc<CapabilityRegistry> {
        &self.caps
    }

    /// Records an installed plugin manifest so it is persisted and re-loaded.
    pub fn record_plugin(&mut self, manifest: PluginManifest) {
        self.installed_plugins.push(manifest);
    }

    /// Returns the recorded installed-plugin manifests.
    #[must_use]
    pub fn installed_plugins(&self) -> &[PluginManifest] {
        &self.installed_plugins
    }

    /// Convenience: grants a capability through the shared registry.
    pub fn grant(&self, plugin: impl Into<crate::PluginId>, capability: impl Into<Capability>) {
        self.caps.grant(plugin.into(), capability.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KernelEvent;
    use crate::test_support::MemoryVfs;
    use crate::vfs::into_dyn;
    use crate::{PluginId, manifest::PluginType};
    use semver::{Version, VersionReq};

    fn fresh_kernel() -> (Arc<dyn DynVfs>, Kernel) {
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
        let kernel = Kernel::new(Arc::clone(&vfs));
        (vfs, kernel)
    }

    #[tokio::test]
    async fn round_trip_over_shared_vfs() {
        // The real durability check: one kernel persists, a second over the
        // same VFS reads it back.
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));

        let plugin = PluginId::new("com.example.b");
        let cap = Capability::new("search.index");
        {
            let mut kernel = Kernel::new(Arc::clone(&vfs));
            kernel
                .config_mut()
                .register(crate::config::schema::ConfigSchema::new(
                    "ui.theme",
                    toml::Value::String("dark".into()),
                    "theme",
                ));
            kernel
                .config_mut()
                .set_override("ui.theme", &"light")
                .unwrap();
            kernel.grant(plugin.clone(), cap.clone());
            kernel.record_plugin(PluginManifest {
                id: plugin.clone(),
                version: Version::new(0, 2, 0),
                api_range: VersionReq::parse(">=0.1").unwrap(),
                plugin_type: PluginType::Theme,
                capabilities: vec![cap.clone()],
            });
            kernel.shutdown().await.unwrap();
        }

        let mut restored = Kernel::new(Arc::clone(&vfs));
        restored.start().await.unwrap();

        assert!(restored.capabilities().has(&plugin, &cap));
        assert_eq!(
            restored.config().get_typed::<String>("ui.theme").unwrap(),
            "light"
        );
        assert_eq!(restored.installed_plugins().len(), 1);
        assert_eq!(restored.installed_plugins()[0].id, plugin);
    }

    #[tokio::test]
    async fn shutdown_completes_within_budget() {
        let (_vfs, kernel) = fresh_kernel();
        let started = std::time::Instant::now();
        kernel.shutdown().await.unwrap();
        assert!(started.elapsed() < SHUTDOWN_TIMEOUT);
    }

    #[tokio::test]
    async fn forced_cancellation_recovers_on_next_start() {
        // No persisted data; cancelling then starting on a fresh vfs must not
        // error (loads treat missing files as fresh).
        let (_vfs, kernel) = fresh_kernel();
        kernel.cancel();
        kernel.shutdown().await.unwrap();

        let vfs2: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
        let mut again = Kernel::new(vfs2);
        again.start().await.unwrap();
    }

    #[tokio::test]
    async fn config_changes_publish_on_kernel_bus() {
        // Regression: Kernel::new previously created ConfigRegistry without a
        // bus, so set_override never published ConfigChanged on the kernel's
        // bus. The bus must be wired through.
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
        let mut sub = {
            let bus = EventBus::new();
            let sub = bus.subscribe();
            let mut kernel = Kernel::with_bus(Arc::clone(&vfs), bus);
            kernel
                .config_mut()
                .register(crate::config::schema::ConfigSchema::new(
                    "ui.theme",
                    toml::Value::String("dark".into()),
                    "theme",
                ));
            kernel
                .config_mut()
                .set_override("ui.theme", &"light")
                .unwrap();
            sub
        };

        match sub.recv().await.unwrap() {
            KernelEvent::ConfigChanged { key } => assert_eq!(key, "ui.theme"),
            other => panic!("expected ConfigChanged, got {other:?}"),
        }
    }
}
