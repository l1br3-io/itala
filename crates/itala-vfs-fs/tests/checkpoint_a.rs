//! Checkpoint A — the kernel and VFS proven together.
//!
//! Constructs the kernel primitives (event bus, config, capabilities) and a
//! real [`LocalFsVfs`], then exercises: read/write round-trip, event-bus
//! subscription, and receiving a `VfsEvent` from an external file change.

use bytes::Bytes;
use futures::stream::StreamExt;
use itala_kernel::PluginId;
use itala_kernel::bus::EventBus;
use itala_kernel::capability::{Capability, CapabilityRegistry};
use itala_kernel::config::ConfigRegistry;
use itala_kernel::events::KernelEvent;
use itala_kernel::vfs::Vfs;
use itala_kernel::vfs_types::VfsPath;
use itala_vfs_fs::LocalFsVfs;

#[tokio::test]
async fn kernel_and_vfs_proven_together() {
    // --- construct the kernel ---
    let bus = EventBus::new();
    let mut events = bus.subscribe();
    let mut config = ConfigRegistry::with_bus(bus.clone());
    let caps = CapabilityRegistry::new();

    config.register(itala_kernel::config::schema::ConfigSchema::new(
        "vault.display_name",
        toml::Value::String("My Vault".into()),
        "display name",
    ));
    assert_eq!(
        config.get_typed::<String>("vault.display_name").unwrap(),
        "My Vault"
    );

    // --- open a VFS on a temp dir ---
    let dir = tempfile::tempdir().expect("tempdir");
    let vfs = LocalFsVfs::new(dir.path()).expect("construct vfs");

    // --- read/write round-trip ---
    let note = VfsPath::new("Notes/Welcome.md").unwrap();
    vfs.write(&note, Bytes::from_static(b"# Welcome to Itala"))
        .await
        .expect("write");
    let back = vfs.read(&note).await.expect("read");
    assert_eq!(back, Bytes::from_static(b"# Welcome to Itala"));

    // --- capability registry works alongside ---
    let plugin = PluginId::new("com.example.demo");
    caps.grant(plugin.clone(), Capability::vault_read());
    assert!(caps.has(&plugin, &Capability::vault_read()));

    // --- publish a kernel event and observe it ---
    bus.publish(KernelEvent::VaultOpened {
        id: itala_kernel::VaultId::new(),
    });
    let ev = tokio::time::timeout(std::time::Duration::from_millis(500), events.recv())
        .await
        .expect("timed out")
        .expect("closed");
    assert!(matches!(ev, KernelEvent::VaultOpened { .. }));

    // --- receive a VfsEvent from an external file change ---
    let watch_dir = VfsPath::new("Notes").unwrap();
    let mut vfs_events = vfs.watch(&watch_dir);
    // external write (outside the adapter)
    tokio::fs::write(dir.path().join("Notes").join("Welcome.md"), b"# changed")
        .await
        .unwrap();
    let vfs_ev = tokio::time::timeout(std::time::Duration::from_millis(2000), vfs_events.next())
        .await
        .expect("timed out waiting for vfs event")
        .expect("stream ended");
    let name = match &vfs_ev {
        itala_kernel::vfs_types::VfsEvent::Changed { path }
        | itala_kernel::vfs_types::VfsEvent::Created { path } => path.as_str(),
        other => panic!("expected Changed/Created, got {other:?}"),
    };
    assert!(name.ends_with("Welcome.md"));
}
