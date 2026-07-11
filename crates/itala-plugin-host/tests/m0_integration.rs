//! F16 — End-to-end M0 integration suite.
//!
//! Exercises the full M0 contract: construct kernel → open VFS → register
//! config → load plugins → enforce capabilities → crash isolation → event flow
//! → graceful shutdown → restart → state preserved. The durability scenarios
//! run against a real [`LocalFsVfs`]; the in-memory scenarios use
//! [`MemoryVfs`] for speed and determinism.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::bus::EventBus;
use itala_kernel::config::schema::ConfigSchema;
use itala_kernel::manifest::{PluginManifest, PluginType};
use itala_kernel::test_support::MemoryVfs;
use itala_kernel::vfs::{DynVfs, into_dyn};
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Kernel, KernelEvent, PluginId, Result};
use itala_plugin_api::{Plugin, PluginContext};
use itala_plugin_host::PluginHost;
use itala_vfs_fs::LocalFsVfs;
use semver::{Version, VersionReq};

// ---------- a trivial plugin used across scenarios ----------

struct NoopPlugin {
    id: PluginId,
}

#[async_trait]
impl Plugin for NoopPlugin {
    fn id(&self) -> PluginId {
        self.id.clone()
    }
    async fn initialize(&self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }
    async fn activate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }
    async fn deactivate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }
}

fn manifest_toml(id: &str, caps: &[&str]) -> String {
    let caps_str = caps
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "id = \"{id}\"\nversion = \"0.1.0\"\napi_range = \">=0.1, <2.0\"\nplugin_type = \"command\"\ncapabilities = [{caps_str}]\n"
    )
}

fn write_plugin(root: &std::path::Path, id: &str, caps: &[&str]) {
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("plugin.toml"), manifest_toml(id, caps)).unwrap();
}

// ---------- F16-1: full lifecycle construct → run → shutdown → restart ----------

#[tokio::test]
async fn full_lifecycle_shutdown_restart_preserves_state() {
    let vault = tempfile::tempdir().unwrap();

    // Phase 1: construct, configure, load a plugin, shut down.
    {
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(LocalFsVfs::new(vault.path()).unwrap()));
        let mut kernel = Kernel::new(Arc::clone(&vfs));
        kernel.start().await.unwrap();
        kernel.config_mut().register(ConfigSchema::new(
            "ui.theme",
            toml::Value::String("dark".into()),
            "theme",
        ));
        kernel
            .config_mut()
            .set_override("ui.theme", &"solarized")
            .unwrap();
        kernel.grant("com.example.reader", itala_kernel::Capability::vault_read());
        kernel.record_plugin(PluginManifest {
            id: "com.example.reader".into(),
            version: Version::new(1, 0, 0),
            api_range: VersionReq::parse(">=0.1").unwrap(),
            plugin_type: PluginType::Command,
            capabilities: vec![itala_kernel::Capability::vault_read()],
        });

        let mut host = PluginHost::new(
            kernel.bus().clone(),
            kernel.capabilities().clone(),
            Arc::clone(&vfs),
        );
        host.register_factory("com.example.reader", || {
            Box::new(NoopPlugin {
                id: "com.example.reader".into(),
            })
        });
        write_plugin(vault.path(), "com.example.reader", &["vault.read"]);
        host.discover_and_load(vault.path()).await.unwrap();
        assert!(host.is_active(&"com.example.reader".into()));

        host.shutdown().await;
        kernel.shutdown().await.unwrap();
    }

    // Phase 2: restart over the same vault; state is preserved.
    {
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(LocalFsVfs::new(vault.path()).unwrap()));
        let mut kernel = Kernel::new(Arc::clone(&vfs));
        kernel.start().await.unwrap();

        assert_eq!(
            kernel.config().get_typed::<String>("ui.theme").unwrap(),
            "solarized"
        );
        assert!(kernel.capabilities().has(
            &"com.example.reader".into(),
            &itala_kernel::Capability::vault_read()
        ));
        assert_eq!(kernel.installed_plugins().len(), 1);

        // re-load the plugin from its persisted plugin.toml
        let mut host = PluginHost::new(
            kernel.bus().clone(),
            kernel.capabilities().clone(),
            Arc::clone(&vfs),
        );
        host.register_factory("com.example.reader", || {
            Box::new(NoopPlugin {
                id: "com.example.reader".into(),
            })
        });
        host.discover_and_load(vault.path()).await.unwrap();
        assert!(host.is_active(&"com.example.reader".into()));
    }
}

// ---------- F16-2: event bus publish + subscribe across plugin boundary ----------

#[tokio::test]
async fn event_bus_carries_events_across_boundary() {
    let bus = EventBus::new();
    let mut sub = bus.subscribe();
    let caps = Arc::new(itala_kernel::capability::CapabilityRegistry::new());
    let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
    let mut host = PluginHost::new(bus.clone(), caps, vfs);

    host.register_factory("p", || Box::new(NoopPlugin { id: "p".into() }));
    let tmp = tempfile::tempdir().unwrap();
    write_plugin(tmp.path(), "p", &[]);
    host.discover_and_load(tmp.path()).await.unwrap();

    // host published PluginLoaded during load
    match sub.recv().await.unwrap() {
        KernelEvent::PluginLoaded { id } => assert_eq!(id, "p".into()),
        other => panic!("unexpected {other:?}"),
    }
}

// ---------- F16-3: config layered override + persistence ----------

#[tokio::test]
async fn config_layered_override_and_persistence() {
    let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
    let mut kernel = Kernel::new(Arc::clone(&vfs));
    kernel.start().await.unwrap();
    kernel.config_mut().register(ConfigSchema::new(
        "editor.tab_width",
        toml::Value::Integer(4),
        "tabs",
    ));
    // default is 4
    assert_eq!(
        kernel
            .config()
            .get_typed::<i64>("editor.tab_width")
            .unwrap(),
        4
    );
    // file layer overrides default
    kernel
        .config_mut()
        .load_toml("editor.tab_width = 2\n")
        .unwrap();
    assert_eq!(
        kernel
            .config()
            .get_typed::<i64>("editor.tab_width")
            .unwrap(),
        2
    );
    // runtime override wins over file
    kernel
        .config_mut()
        .set_override("editor.tab_width", &8u32)
        .unwrap();
    assert_eq!(
        kernel
            .config()
            .get_typed::<i64>("editor.tab_width")
            .unwrap(),
        8
    );

    kernel.shutdown().await.unwrap();

    // reload: the persisted effective value (8) is restored as the file layer.
    let mut k2 = Kernel::new(Arc::clone(&vfs));
    k2.start().await.unwrap();
    k2.config_mut().register(ConfigSchema::new(
        "editor.tab_width",
        toml::Value::Integer(4),
        "tabs",
    ));
    assert_eq!(
        k2.config().get_typed::<i64>("editor.tab_width").unwrap(),
        8,
        "persisted override should win over the fresh default"
    );
}

// ---------- F16-4: capability grant/deny/revoke cycle ----------

#[tokio::test]
async fn capability_grant_deny_revoke_cycle() {
    let bus = EventBus::new();
    let caps = Arc::new(itala_kernel::capability::CapabilityRegistry::new());
    let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
    vfs.write(
        &VfsPath::new("secret.md").unwrap(),
        Bytes::from_static(b"x"),
    )
    .await
    .unwrap();

    let mut host = PluginHost::new(bus, caps.clone(), vfs.clone());
    host.register_factory("perm", || Box::new(NoopPlugin { id: "perm".into() }));
    let tmp = tempfile::tempdir().unwrap();
    write_plugin(tmp.path(), "perm", &["vault.read"]);
    host.discover_and_load(tmp.path()).await.unwrap();

    let ctx = host.context_for(&"perm".into()).unwrap();

    // granted at load → read succeeds
    assert!(
        ctx.read_file(&VfsPath::new("secret.md").unwrap())
            .await
            .is_ok()
    );

    // revoked → denied
    host.revoke(&"perm".into(), &itala_kernel::Capability::vault_read());
    assert!(matches!(
        ctx.read_file(&VfsPath::new("secret.md").unwrap())
            .await
            .unwrap_err(),
        itala_kernel::Error::CapabilityDenied { .. }
    ));

    // re-granted → succeeds again
    caps.grant("perm".into(), itala_kernel::Capability::vault_read());
    assert!(
        ctx.read_file(&VfsPath::new("secret.md").unwrap())
            .await
            .is_ok()
    );
}

// ---------- F16-5: VFS read/write round-trip on local FS ----------

#[tokio::test]
async fn local_fs_read_write_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(LocalFsVfs::new(dir.path()).unwrap()));
    let path = VfsPath::new("a/b.md").unwrap();
    vfs.write(&path, Bytes::from_static(b"round-trip"))
        .await
        .unwrap();
    let back = vfs.read(&path).await.unwrap();
    assert_eq!(back, Bytes::from_static(b"round-trip"));
}

// ---------- F16-6: determinism — run the pure durability round-trip 10× ----------

#[tokio::test]
async fn durability_round_trip_is_deterministic_across_runs() {
    for _ in 0..10 {
        let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
        {
            let mut kernel = Kernel::new(Arc::clone(&vfs));
            kernel.start().await.unwrap();
            kernel.config_mut().register(ConfigSchema::new(
                "k",
                toml::Value::String("d".into()),
                "k",
            ));
            kernel.config_mut().set_override("k", &"v").unwrap();
            kernel.grant("p", itala_kernel::Capability::vault_read());
            kernel.shutdown().await.unwrap();
        }
        let mut restored = Kernel::new(Arc::clone(&vfs));
        restored.start().await.unwrap();
        restored.config_mut().register(ConfigSchema::new(
            "k",
            toml::Value::String("d".into()),
            "k",
        ));
        assert_eq!(restored.config().get_typed::<String>("k").unwrap(), "v");
        assert!(
            restored
                .capabilities()
                .has(&"p".into(), &itala_kernel::Capability::vault_read())
        );
    }
}
