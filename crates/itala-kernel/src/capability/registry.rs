//! The [`CapabilityRegistry`] — runtime store of granted capabilities.

use std::collections::HashSet;
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{Capability, GrantMap};

/// Acquires a read guard, recovering transparently from a poisoned lock
/// (poisoning only signals a prior panic; the data is still readable).
fn read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Acquires a write guard, recovering transparently from a poisoned lock.
fn write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

/// Tracks granted and revoked capabilities per plugin.
///
/// Operations are synchronous and non-blocking (a short critical section under
/// a [`RwLock`]); guards are never held across `.await`. The registry is
/// `Send + Sync` and safe to share across tasks via `Arc`.
///
/// ```
/// use itala_kernel::capability::{Capability, CapabilityRegistry};
/// use itala_kernel::PluginId;
///
/// let reg = CapabilityRegistry::new();
/// let plugin = PluginId::new("com.example.a");
/// reg.grant(plugin.clone(), Capability::vault_read());
/// assert!(reg.has(&plugin, &Capability::vault_read()));
/// ```
pub struct CapabilityRegistry {
    grants: RwLock<GrantMap>,
}

impl CapabilityRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            grants: RwLock::new(GrantMap::new()),
        }
    }

    /// Grants a capability to a plugin. Idempotent.
    pub fn grant(&self, plugin: crate::PluginId, capability: Capability) {
        let mut map = write(&self.grants);
        map.entry(plugin).or_default().insert(capability);
    }

    /// Revokes a capability from a plugin. Idempotent.
    pub fn revoke(&self, plugin: &crate::PluginId, capability: &Capability) {
        let mut map = write(&self.grants);
        if let Some(set) = map.get_mut(plugin) {
            set.remove(capability);
            if set.is_empty() {
                map.remove(plugin);
            }
        }
    }

    /// Returns `true` if the plugin currently holds the given capability.
    #[must_use]
    pub fn has(&self, plugin: &crate::PluginId, capability: &Capability) -> bool {
        let map = read(&self.grants);
        map.get(plugin).is_some_and(|set| set.contains(capability))
    }

    /// Returns the set of capabilities currently granted to `plugin`.
    ///
    /// The returned set is a snapshot; subsequent mutations to the registry do
    /// not affect it.
    #[must_use]
    pub fn capabilities_for(&self, plugin: &crate::PluginId) -> HashSet<Capability> {
        let map = read(&self.grants);
        map.get(plugin).cloned().unwrap_or_default()
    }

    /// Returns a snapshot of all grants, suitable for persistence.
    #[must_use]
    pub fn snapshot(&self) -> GrantMap {
        read(&self.grants).clone()
    }

    /// Replaces all grants from a previously-serialized [`GrantMap`] (e.g. on
    /// restart after reading persisted state).
    pub fn restore(&self, grants: GrantMap) {
        *write(&self.grants) = grants;
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
