//! Verifies a minimal mock plugin compiles and runs against the plugin API.

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::capability::{Capability, CapabilityRegistry};
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Error, PluginId, Result};
use itala_plugin_api::{Plugin, PluginContext};

/// A host-side context backed by an in-memory capability registry and a
/// closure-based VFS, used to exercise plugin lifecycle in tests.
struct MockContext {
    plugin: PluginId,
    caps: CapabilityRegistry,
    read: ReadFn,
    write: WriteFn,
}

type ReadFn = Box<dyn Fn(&VfsPath) -> Result<Bytes> + Send + Sync>;
type WriteFn = Box<dyn Fn(&VfsPath, Bytes) -> Result<()> + Send + Sync>;

#[async_trait]
impl PluginContext for MockContext {
    fn plugin_id(&self) -> PluginId {
        self.plugin.clone()
    }
    fn has_capability(&self, capability: &Capability) -> bool {
        self.caps.has(&self.plugin, capability)
    }
    async fn read_file(&self, path: &VfsPath) -> Result<Bytes> {
        if !self.caps.has(&self.plugin, &Capability::vault_read()) {
            return Err(Error::capability_denied(self.plugin.clone(), "vault.read"));
        }
        (self.read)(path)
    }
    async fn write_file(&self, path: &VfsPath, data: Bytes) -> Result<()> {
        if !self.caps.has(&self.plugin, &Capability::new("vault.write")) {
            return Err(Error::capability_denied(self.plugin.clone(), "vault.write"));
        }
        (self.write)(path, data)
    }
}

struct EchoPlugin {
    id: PluginId,
    activated: std::sync::Mutex<bool>,
}

#[async_trait]
impl Plugin for EchoPlugin {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    async fn initialize(&self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }
    async fn activate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        *self.activated.lock().unwrap() = true;
        Ok(())
    }
    async fn deactivate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        *self.activated.lock().unwrap() = false;
        Ok(())
    }
}

#[tokio::test]
async fn mock_plugin_runs_lifecycle() {
    let id = PluginId::new("com.test.echo");
    let caps = CapabilityRegistry::new();
    let ctx = MockContext {
        plugin: id.clone(),
        caps,
        read: Box::new(|_| Ok(Bytes::from_static(b""))),
        write: Box::new(|_, _| Ok(())),
    };

    let plugin = EchoPlugin {
        id: id.clone(),
        activated: std::sync::Mutex::new(false),
    };

    plugin.initialize(&ctx).await.unwrap();
    plugin.activate(&ctx).await.unwrap();
    assert!(*plugin.activated.lock().unwrap());
    plugin.deactivate(&ctx).await.unwrap();
    assert!(!*plugin.activated.lock().unwrap());
}
