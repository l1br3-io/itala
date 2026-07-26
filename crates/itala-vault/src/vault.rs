//! Vault lifecycle: [`Vault::init`] / [`Vault::open`].
//!
//! A vault is an open handle to a directory containing `.itala/`. The bounded
//! context owns no filesystem knowledge — it operates through a [`DynVfs`]
//! supplied by the shell. The shell therefore chooses the storage backend
//! (local FS in M1's CLI; S3/IndexedDB in future shells), preserving the
//! dependency DAG (ARCHITECTURE.md §2: contexts depend only on `itala-kernel`).
//!
//! On both `init` and `open`, a [`KernelEvent::VaultOpened`] is published on
//! the bus so other contexts can warm caches.

use bytes::Bytes;

use itala_kernel::bus::EventBus;
use itala_kernel::events::KernelEvent;
use itala_kernel::history::DynHistoryStore;
use itala_kernel::vfs::DynVfs;
use itala_kernel::vfs_types::VfsPath;
use itala_kernel::{Error, Result, VaultId};

/// Relative path of the vault-identity artifact within `.itala/`.
const VAULT_ID_PATH: &str = ".itala/vault.id";
/// Relative path of the (minimal, default-empty) vault config document.
const CONFIG_PATH: &str = ".itala/config.toml";

/// An open handle to an Itala vault.
///
/// Carries the vault's [`VaultId`], the [`DynVfs`] it reads/writes through,
/// an optional [`DynHistoryStore`] for snapshot-on-save history, and an
/// [`EventBus`] for publishing context-broadcast events.
pub struct Vault {
    id: VaultId,
    vfs: Box<dyn DynVfs>,
    history: Option<Box<dyn DynHistoryStore>>,
    bus: EventBus,
}

enum VaultMode {
    Init,
    Open,
}

impl Vault {
    /// Initializes a fresh vault at the root the VFS is anchored to.
    ///
    /// Writes `.itala/vault.id` (a freshly-generated [`VaultId`]) and a
    /// minimal `.itala/config.toml`. The VFS must be writeable.
    ///
    /// # Errors
    /// [`Error::Io`] or [`Error::PathEscape`] on VFS write failure.
    pub async fn init(vfs: Box<dyn DynVfs>) -> Result<Self> {
        Self::init_with_bus(vfs, EventBus::new()).await
    }

    /// Like [`Vault::init`] but publishes on the provided [`EventBus`].
    pub async fn init_with_bus(vfs: Box<dyn DynVfs>, bus: EventBus) -> Result<Self> {
        Self::setup(vfs, bus, VaultMode::Init).await
    }

    /// Opens an existing vault, reading `.itala/vault.id` to recover the
    /// [`VaultId`].
    ///
    /// # Errors
    /// [`Error::Config`] if `.itala/vault.id` is missing (the directory is
    /// not an Itala vault) or malformed; other VFS errors propagate as-is.
    pub async fn open(vfs: Box<dyn DynVfs>) -> Result<Self> {
        Self::open_with_bus(vfs, EventBus::new()).await
    }

    /// Like [`Vault::open`] but publishes on the provided [`EventBus`].
    pub async fn open_with_bus(vfs: Box<dyn DynVfs>, bus: EventBus) -> Result<Self> {
        Self::setup(vfs, bus, VaultMode::Open).await
    }

    async fn setup(vfs: Box<dyn DynVfs>, bus: EventBus, mode: VaultMode) -> Result<Self> {
        let id = match mode {
            VaultMode::Init => {
                let id = VaultId::new();
                let id_path = VfsPath::new(VAULT_ID_PATH)?;
                let cfg_path = VfsPath::new(CONFIG_PATH)?;
                vfs.write(&id_path, Bytes::from(id.to_string())).await?;
                vfs.write(&cfg_path, Bytes::from("# Itala vault configuration.\n"))
                    .await?;
                id
            }
            VaultMode::Open => {
                let id_path = VfsPath::new(VAULT_ID_PATH)?;
                let bytes = vfs.read(&id_path).await.map_err(|e| match e {
                    Error::NotFound { .. } => {
                        Error::config(format!("not an Itala vault: '{VAULT_ID_PATH}' missing"))
                    }
                    other => other,
                })?;
                let s = String::from_utf8(bytes.to_vec())
                    .map_err(|e| Error::config(format!("vault.id is not valid UTF-8: {e}")))?;
                s.trim()
                    .parse()
                    .map_err(|e| Error::invalid_id(format!("vault.id parse failed: {e}")))?
            }
        };
        let vault = Self {
            id,
            vfs,
            history: None,
            bus,
        };
        vault.bus.publish(KernelEvent::VaultOpened { id });
        Ok(vault)
    }

    /// Attaches a history store. Subsequent `save` operations in
    /// [`crate::Vault`] (e.g. `create_note`) will snapshot into it. Returns
    /// the previous store, if any.
    pub fn set_history(
        &mut self,
        history: Box<dyn DynHistoryStore>,
    ) -> Option<Box<dyn DynHistoryStore>> {
        self.history.replace(history)
    }

    /// Returns a handle to the history store, if attached.
    #[must_use]
    pub fn history_store(&self) -> Option<&dyn DynHistoryStore> {
        self.history.as_deref()
    }

    /// Returns the vault's stable identifier.
    #[must_use]
    pub const fn id(&self) -> VaultId {
        self.id
    }

    /// Returns the underlying VFS (read/write through this).
    #[must_use]
    pub fn vfs(&self) -> &dyn DynVfs {
        self.vfs.as_ref()
    }

    /// Returns the event bus.
    #[must_use]
    pub const fn bus(&self) -> &EventBus {
        &self.bus
    }
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itala_kernel::vfs::into_dyn;
    use itala_vfs_fs::LocalFsVfs;

    fn fresh_local_vfs(dir: &std::path::Path) -> Box<dyn DynVfs> {
        into_dyn(LocalFsVfs::new(dir).unwrap())
    }

    #[tokio::test]
    async fn init_writes_vault_id_and_config() {
        let tmp = tempfile::tempdir().unwrap();
        let v = Vault::init(fresh_local_vfs(tmp.path())).await.unwrap();
        let _ = v;
    }

    #[tokio::test]
    async fn init_then_open_roundtrips_id() {
        let tmp = tempfile::tempdir().unwrap();
        let v1 = Vault::init(fresh_local_vfs(tmp.path())).await.unwrap();
        let original = v1.id();
        drop(v1);
        let v2 = Vault::open(fresh_local_vfs(tmp.path())).await.unwrap();
        assert_eq!(original, v2.id());
    }

    #[tokio::test]
    async fn open_on_non_vault_returns_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Vault::open(fresh_local_vfs(tmp.path())).await.unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn open_publishes_vault_opened() {
        let tmp = tempfile::tempdir().unwrap();
        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        Vault::init_with_bus(fresh_local_vfs(tmp.path()), bus)
            .await
            .unwrap();
        let ev = sub.recv().await.unwrap();
        assert!(matches!(ev, KernelEvent::VaultOpened { .. }));
    }

    #[tokio::test]
    async fn init_id_round_trips_through_string() {
        let tmp = tempfile::tempdir().unwrap();
        let v = Vault::init(fresh_local_vfs(tmp.path())).await.unwrap();
        let s = v.id().to_string();
        let back: VaultId = s.parse().unwrap();
        assert_eq!(v.id(), back);
    }
}
