//! [`PluginManifest`] — the parsed `plugin.toml` that describes a plugin.

pub mod plugin_type;

pub use plugin_type::PluginType;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::PluginId;
use crate::capability::Capability;

/// The host-side API version this crate's contract targets.
///
/// Manifests declare an `api_range` that must match this version for a plugin
/// to load. Bump the major version for breaking API changes.
pub const HOST_API_VERSION: Version = Version::new(0, 1, 0);

/// A parsed `plugin.toml` manifest.
///
/// All fields are required; a manifest missing any of them fails to parse with
/// a typed error naming the missing field.
///
/// ```
/// use itala_kernel::manifest::PluginManifest;
///
/// let toml = r#"
/// id = "com.example.editor"
/// version = "1.2.3"
/// api_range = ">=0.1, <2.0"
/// plugin_type = "editor"
/// capabilities = ["vault.read"]
/// "#;
/// let manifest = PluginManifest::parse(toml).unwrap();
/// assert_eq!(manifest.id.as_str(), "com.example.editor");
/// assert!(manifest.is_compatible());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Stable plugin identifier (e.g. `"com.example.my-editor"`).
    pub id: PluginId,
    /// The plugin's own version.
    pub version: Version,
    /// Range of host API versions this plugin is compatible with.
    pub api_range: VersionReq,
    /// The plugin's category.
    pub plugin_type: PluginType,
    /// Capabilities the plugin requires.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

impl PluginManifest {
    /// Parses a manifest from a TOML document string.
    ///
    /// # Errors
    /// Returns [`crate::Error::Plugin`] with a message naming the offending
    /// field if the document is malformed or a required field is missing.
    pub fn parse(toml_str: &str) -> crate::Result<Self> {
        toml::from_str::<Self>(toml_str)
            .map_err(|e| crate::Error::plugin(format!("invalid plugin manifest: {e}")))
    }

    /// Returns `true` if the manifest's `api_range` matches the current
    /// [`HOST_API_VERSION`].
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        self.api_range.matches(&HOST_API_VERSION)
    }

    /// Returns `true` if the manifest's `api_range` matches `host_api`.
    #[must_use]
    pub fn is_compatible_with(&self, host_api: &Version) -> bool {
        self.api_range.matches(host_api)
    }
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const M: fn() = check::<PluginManifest>;
    const _: [(); 0] = [(); {
        let _ = M;
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
id = "com.example.editor"
version = "1.2.3"
api_range = ">=0.1, <2.0"
plugin_type = "editor"
capabilities = ["vault.read", "ui.panel"]
"#;

    #[test]
    fn parses_valid_manifest() {
        let m = PluginManifest::parse(VALID).unwrap();
        assert_eq!(m.id.as_str(), "com.example.editor");
        assert_eq!(m.version, Version::new(1, 2, 3));
        assert_eq!(m.plugin_type, PluginType::Editor);
        assert_eq!(m.capabilities.len(), 2);
        assert!(m.is_compatible());
    }

    #[test]
    fn missing_required_field_names_it() {
        let bad = r#"
id = "x"
version = "1.0.0"
api_range = ">=0.1"
"#;
        let err = PluginManifest::parse(bad).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("plugin_type"), "got: {msg}");
    }

    #[test]
    fn incompatible_api_range_is_rejected() {
        let m = PluginManifest::parse(
            r#"
id = "x"
version = "1.0.0"
api_range = ">=999.0"
plugin_type = "theme"
"#,
        )
        .unwrap();
        assert!(!m.is_compatible());
    }

    #[test]
    fn empty_capabilities_default() {
        let m = PluginManifest::parse(
            r#"
id = "x"
version = "1.0.0"
api_range = "*"
plugin_type = "command"
"#,
        )
        .unwrap();
        assert!(m.capabilities.is_empty());
    }

    #[test]
    fn snapshot_representative_manifest() {
        let m = PluginManifest::parse(VALID).unwrap();
        insta::assert_yaml_snapshot!(m);
    }
}
