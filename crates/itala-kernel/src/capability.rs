//! Capability model + registry.
//!
//! A [`Capability`] is a named, revocable grant (e.g. `vault.read`,
//! `network.outbound`, `search.index`). Plugins *declare* required
//! capabilities in their manifest; the user *grants* them; the host applies
//! them as **policy and audit** at privileged calls routed through
//! `PluginContext` — not a security sandbox for native code (see
//! [ADR-0005](../../../docs/adr/0005-plugin-trust-and-rollout.md)). WASM at M5
//! makes these checks enforceable for third-party code. The
//! [`CapabilityRegistry`] tracks granted capabilities per plugin and is
//! queryable at runtime.
//!
//! The serialization form of [`Capability`] is a bare TOML/JSON string —
//! this is a semver-frozen surface: the string *is* the capability, so it
//! never changes representation across versions.

use std::collections::{HashMap, HashSet};

use derive_more::{Display, From, FromStr};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub mod registry;

pub use registry::CapabilityRegistry;

/// Well-known capability name constants.
///
/// These are the canonical string identifiers. New capabilities may be added
/// by plugins; these are merely the common built-in ones.
pub mod known {
    /// Read access to vault contents.
    pub const VAULT_READ: &str = "vault.read";
    /// Write access to vault contents.
    pub const VAULT_WRITE: &str = "vault.write";
    /// Outbound network access.
    pub const NETWORK_OUTBOUND: &str = "network.outbound";
    /// Permission to update the search index.
    pub const SEARCH_INDEX: &str = "search.index";
    /// Permission to open a UI panel.
    pub const UI_PANEL: &str = "ui.panel";
    /// Permission to emit telemetry.
    pub const TELEMETRY_REPORT: &str = "telemetry.report";
}

/// A named, revocable capability grant.
///
/// Serializes transparently to its bare string form (see [`Serialize`]):
///
/// ```
/// use itala_kernel::capability::Capability;
/// use serde_json::json;
///
/// let cap = Capability::new("vault.read");
/// let s = serde_json::to_value(&cap).unwrap();
/// assert_eq!(s, json!("vault.read"));
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    FromStr,
    From,
)]
#[serde(transparent)]
#[from(forward)]
pub struct Capability(SmolStr);

impl Capability {
    /// Creates a capability from a name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(SmolStr::new(name))
    }

    /// Creates a capability from an owned [`SmolStr`].
    #[must_use]
    pub const fn from_smol_str(name: SmolStr) -> Self {
        Self(name)
    }

    /// Returns the capability name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convenience constructor for the well-known `vault.read` capability.
    #[must_use]
    pub fn vault_read() -> Self {
        Self::new(known::VAULT_READ)
    }

    /// Convenience constructor for the well-known `vault.write` capability.
    #[must_use]
    pub fn vault_write() -> Self {
        Self::new(known::VAULT_WRITE)
    }

    /// Convenience constructor for the well-known `network.outbound` capability.
    #[must_use]
    pub fn network_outbound() -> Self {
        Self::new(known::NETWORK_OUTBOUND)
    }

    /// Convenience constructor for the well-known `search.index` capability.
    #[must_use]
    pub fn search_index() -> Self {
        Self::new(known::SEARCH_INDEX)
    }

    /// Convenience constructor for the well-known `ui.panel` capability.
    #[must_use]
    pub fn ui_panel() -> Self {
        Self::new(known::UI_PANEL)
    }

    /// Convenience constructor for the well-known `telemetry.report` capability.
    #[must_use]
    pub fn telemetry_report() -> Self {
        Self::new(known::TELEMETRY_REPORT)
    }
}

impl From<&Capability> for String {
    fn from(c: &Capability) -> Self {
        c.as_str().to_owned()
    }
}

impl AsRef<str> for Capability {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A snapshot of all granted capabilities, keyed by plugin.
///
/// This is the serializable representation used by the registry for
/// persistence (see F15). The outer map is keyed by plugin id; the inner set
/// holds the granted capabilities.
pub type GrantMap = HashMap<crate::PluginId, HashSet<Capability>>;

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const CAP: fn() = check::<Capability>;
    const REG: fn() = check::<CapabilityRegistry>;
    const _: [(); 0] = [(); {
        let _ = (CAP, REG);
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginId;

    #[test]
    fn capability_serializes_to_bare_string() {
        let cap = Capability::new("network.outbound");
        let v = serde_json::to_value(&cap).unwrap();
        assert_eq!(v, serde_json::json!("network.outbound"));
        let back: Capability = serde_json::from_value(v).unwrap();
        assert_eq!(cap, back);
    }

    #[test]
    fn registry_grant_revoke_query() {
        let reg = CapabilityRegistry::new();
        let plugin = PluginId::new("com.example.a");
        let cap = Capability::new("vault.read");

        assert!(!reg.has(&plugin, &cap));
        reg.grant(plugin.clone(), cap.clone());
        assert!(reg.has(&plugin, &cap));
        reg.revoke(&plugin, &cap);
        assert!(!reg.has(&plugin, &cap));
    }

    #[test]
    fn registry_revoking_ungranted_is_noop() {
        let reg = CapabilityRegistry::new();
        let plugin = PluginId::new("com.example.b");
        reg.revoke(&plugin, &Capability::new("vault.read"));
        assert!(!reg.has(&plugin, &Capability::new("vault.read")));
    }

    #[test]
    fn registry_grants_are_per_plugin() {
        let reg = CapabilityRegistry::new();
        let a = PluginId::new("com.example.a");
        let b = PluginId::new("com.example.b");
        let cap = Capability::new("search.index");

        reg.grant(a.clone(), cap.clone());
        assert!(reg.has(&a, &cap));
        assert!(!reg.has(&b, &cap));
    }

    #[test]
    fn registry_capabilities_for_plugin() {
        let reg = CapabilityRegistry::new();
        let plugin = PluginId::new("com.example.c");
        reg.grant(plugin.clone(), Capability::new("vault.read"));
        reg.grant(plugin.clone(), Capability::new("network.outbound"));

        let caps = reg.capabilities_for(&plugin);
        assert_eq!(caps.len(), 2);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config {
            cases: 256, ..proptest::test_runner::Config::default()
        })]

        #[test]
        fn grant_query_revoke_cycle(
            plugin in "[a-z]{1,8}",
            caps in proptest::collection::vec("[a-z]{1,8}\\.[a-z]{1,8}", 0..16),
        ) {
            let reg = CapabilityRegistry::new();
            let pid = PluginId::new(&plugin);
            for raw in &caps {
                reg.grant(pid.clone(), Capability::new(raw));
                // after grant, the capability is present
                assert!(reg.has(&pid, &Capability::new(raw)));
            }
            for raw in &caps {
                assert!(reg.has(&pid, &Capability::new(raw)));
            }
            for raw in &caps {
                reg.revoke(&pid, &Capability::new(raw));
                assert!(!reg.has(&pid, &Capability::new(raw)));
            }
        }
    }
}
