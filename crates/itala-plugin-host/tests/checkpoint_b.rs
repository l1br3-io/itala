//! Checkpoint B — the plugin host proven end to end.
//!
//! Loads three plugins (one of which panics during activation), verifies the
//! host survives, capability enforcement works on the survivors, and the
//! lifecycle runs `initialize → activate → deactivate` in order.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use itala_kernel::bus::EventBus;
use itala_kernel::capability::CapabilityRegistry;
use itala_kernel::test_support::MemoryVfs;
use itala_kernel::vfs::{DynVfs, into_dyn};
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Error, PluginId, Result};
use itala_plugin_api::{Plugin, PluginContext};
use itala_plugin_host::PluginHost;

/// Records the ordered sequence of lifecycle calls across plugins.
#[derive(Default)]
struct Trace {
    log: std::sync::Mutex<Vec<(&'static str, &'static str)>>,
}

impl Trace {
    fn push(&self, id: &'static str, phase: &'static str) {
        self.log.lock().unwrap().push((id, phase));
    }
}

struct TracedPlugin {
    id: &'static str,
    trace: Arc<Trace>,
}

#[async_trait]
impl Plugin for TracedPlugin {
    fn id(&self) -> PluginId {
        self.id.into()
    }
    async fn initialize(&self, _ctx: &dyn PluginContext) -> Result<()> {
        self.trace.push(self.id, "init");
        Ok(())
    }
    async fn activate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        self.trace.push(self.id, "activate");
        Ok(())
    }
    async fn deactivate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        self.trace.push(self.id, "deactivate");
        Ok(())
    }
}

struct BoomPlugin {
    id: &'static str,
}

#[async_trait]
impl Plugin for BoomPlugin {
    fn id(&self) -> PluginId {
        self.id.into()
    }
    async fn initialize(&self, _ctx: &dyn PluginContext) -> Result<()> {
        Ok(())
    }
    async fn activate(&self, _ctx: &dyn PluginContext) -> Result<()> {
        panic!("boom");
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

#[tokio::test]
async fn checkpoint_b_host_survives_crash_and_enforces_capabilities() {
    let bus = EventBus::new();
    let caps = Arc::new(CapabilityRegistry::new());
    let vfs: Arc<dyn DynVfs> = Arc::from(into_dyn(MemoryVfs::new()));
    // seed a file for the reader to read
    vfs.write(
        &VfsPath::new("shared.md").unwrap(),
        Bytes::from_static(b"ok"),
    )
    .await
    .unwrap();

    let trace = Arc::new(Trace::default());
    let t1 = trace.clone();
    let t2 = trace.clone();

    let tmp = tempfile::tempdir().unwrap();
    for (id, caps_list) in [("reader", &["vault.read"][..]), ("bystander", &[])] {
        let dir = tmp.path().join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plugin.toml"), manifest_toml(id, caps_list)).unwrap();
    }
    // crasher
    let cdir = tmp.path().join("crasher");
    std::fs::create_dir_all(&cdir).unwrap();
    std::fs::write(cdir.join("plugin.toml"), manifest_toml("crasher", &[])).unwrap();

    let mut host = PluginHost::new(bus, caps, vfs);
    host.register_factory("reader", move || {
        Box::new(TracedPlugin {
            id: "reader",
            trace: t1.clone(),
        })
    });
    host.register_factory("bystander", move || {
        Box::new(TracedPlugin {
            id: "bystander",
            trace: t2.clone(),
        })
    });
    host.register_factory("crasher", || Box::new(BoomPlugin { id: "crasher" }));

    host.discover_and_load(tmp.path()).await.unwrap();

    // crasher failed; the other two are active
    let list = host.list();
    assert!(
        list.iter()
            .any(|(id, s)| id.as_str() == "crasher" && s.is_failed())
    );
    assert!(
        list.iter()
            .any(|(id, s)| id.as_str() == "reader" && s.is_active())
    );
    assert!(
        list.iter()
            .any(|(id, s)| id.as_str() == "bystander" && s.is_active())
    );

    // capability enforcement: reader can read, bystander cannot
    let reader_ctx = host.context_for(&"reader".into()).unwrap();
    let bystander_ctx = host.context_for(&"bystander".into()).unwrap();
    let bytes = reader_ctx
        .read_file(&VfsPath::new("shared.md").unwrap())
        .await
        .expect("reader has vault.read");
    assert_eq!(bytes, Bytes::from_static(b"ok"));
    let err = bystander_ctx
        .read_file(&VfsPath::new("shared.md").unwrap())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::CapabilityDenied { .. }));

    // lifecycle order: each surviving plugin ran init then activate
    host.shutdown().await;

    let log = trace.log.lock().unwrap();
    let init_pos = |id: &'static str| {
        log.iter()
            .position(|(p, phase)| *p == id && *phase == "init")
    };
    let act_pos = |id: &'static str| {
        log.iter()
            .position(|(p, phase)| *p == id && *phase == "activate")
    };
    let deact_pos = |id: &'static str| {
        log.iter()
            .position(|(p, phase)| *p == id && *phase == "deactivate")
    };
    for id in ["reader", "bystander"] {
        let (i, a, d) = (init_pos(id), act_pos(id), deact_pos(id));
        assert!(
            i < a && a < d,
            "{id}: lifecycle out of order ({i:?}, {a:?}, {d:?})"
        );
    }
}
