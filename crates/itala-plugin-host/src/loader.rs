//! Plugin discovery — scans a directory tree for installed plugins.

use std::path::Path;

use itala_kernel::Result;
use itala_kernel::manifest::PluginManifest;

/// A discovered plugin: its parsed manifest and the directory it lives in.
pub struct DiscoveredPlugin {
    /// The parsed `plugin.toml`.
    pub manifest: PluginManifest,
    /// The plugin's directory (absolute or relative to the scan root).
    pub dir: std::path::PathBuf,
}

/// Scans `root` for immediate subdirectories containing a `plugin.toml` and
/// parses each.
///
/// # Errors
/// Returns [`itala_kernel::Error::Io`] if `root` cannot be read. Individual malformed
/// manifests are skipped (logged), so one bad plugin does not abort discovery.
pub fn discover(root: &Path) -> Result<Vec<DiscoveredPlugin>> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let toml_path = path.join("plugin.toml");
        if !toml_path.is_file() {
            continue;
        }
        match std::fs::read_to_string(&toml_path) {
            Ok(contents) => match PluginManifest::parse(&contents) {
                Ok(manifest) => found.push(DiscoveredPlugin {
                    manifest,
                    dir: path,
                }),
                Err(e) => tracing::warn!(error = %e, ?toml_path, "skipping malformed manifest"),
            },
            Err(e) => tracing::warn!(error = %e, ?toml_path, "could not read manifest"),
        }
    }
    Ok(found)
}
