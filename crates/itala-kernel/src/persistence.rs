//! Durable state persistence.
//!
//! The kernel persists three artifacts under `.itala/` so the next start
//! reconstructs config values, capability grants, and plugin installation
//! records:
//!
//! - `.itala/config.toml` — the merged config document;
//! - `.itala/capabilities.json` — the capability grant map;
//! - `.itala/plugins.json` — installed plugin manifests.
//!
//! All loads treat a missing artifact as a fresh start (no error), so a forced
//! shutdown that fails to finish saving is recoverable on the next start.

use bytes::Bytes;

use crate::capability::{CapabilityRegistry, GrantMap};
use crate::config::ConfigRegistry;
use crate::manifest::PluginManifest;
use crate::vfs::DynVfs;
use crate::vfs_types::VfsPath;
use crate::{Error, Result};

/// Path (relative to vault root) of the persisted config document.
pub const CONFIG_PATH: &str = ".itala/config.toml";
/// Path of the persisted capability grants.
pub const CAPABILITIES_PATH: &str = ".itala/capabilities.json";
/// Path of the persisted plugin install records.
pub const PLUGINS_PATH: &str = ".itala/plugins.json";

fn path(literal: &str) -> Result<VfsPath> {
    VfsPath::new(literal)
}

fn as_text(bytes: &Bytes) -> Result<String> {
    String::from_utf8(bytes.to_vec())
        .map_err(|e| Error::config(format!("persisted data is not UTF-8: {e}")))
}

/// Persists the config document.
pub async fn save_config(vfs: &dyn DynVfs, config: &ConfigRegistry) -> Result<()> {
    let doc = config.save_toml()?;
    vfs.write(&path(CONFIG_PATH)?, Bytes::from(doc)).await
}

/// Loads the config document into `config` (missing file = fresh start).
pub async fn load_config(vfs: &dyn DynVfs, config: &mut ConfigRegistry) -> Result<()> {
    match vfs.read(&path(CONFIG_PATH)?).await {
        Ok(bytes) => config.load_toml(&as_text(&bytes)?),
        Err(Error::NotFound { .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Persists the capability grant map.
pub async fn save_capabilities(vfs: &dyn DynVfs, caps: &CapabilityRegistry) -> Result<()> {
    let json = serde_json::to_vec(&caps.snapshot())
        .map_err(|e| Error::config(format!("serialize capabilities: {e}")))?;
    vfs.write(&path(CAPABILITIES_PATH)?, Bytes::from(json))
        .await
}

/// Loads the capability grant map into `caps` (missing file = fresh start).
pub async fn load_capabilities(vfs: &dyn DynVfs, caps: &CapabilityRegistry) -> Result<()> {
    match vfs.read(&path(CAPABILITIES_PATH)?).await {
        Ok(bytes) => {
            let grants: GrantMap = serde_json::from_slice(&bytes)
                .map_err(|e| Error::config(format!("deserialize capabilities: {e}")))?;
            caps.restore(grants);
            Ok(())
        }
        Err(Error::NotFound { .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Persists the installed-plugin manifest records.
pub async fn save_plugins(vfs: &dyn DynVfs, plugins: &[PluginManifest]) -> Result<()> {
    let json = serde_json::to_vec(plugins)
        .map_err(|e| Error::config(format!("serialize plugins: {e}")))?;
    vfs.write(&path(PLUGINS_PATH)?, Bytes::from(json)).await
}

/// Loads the installed-plugin manifest records (missing file = empty).
pub async fn load_plugins(vfs: &dyn DynVfs) -> Result<Vec<PluginManifest>> {
    match vfs.read(&path(PLUGINS_PATH)?).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| Error::config(format!("deserialize plugins: {e}"))),
        Err(Error::NotFound { .. }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}
