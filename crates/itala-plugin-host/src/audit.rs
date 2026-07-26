//! Audit logging for capability policy decisions.
//!
//! Every capability grant, use, and denial is emitted as a structured
//! `tracing` event under the `itala.plugin.audit` target, and each privileged
//! call runs inside a span that follows the plugin boundary
//! (e.g. `plugin.read_file` keyed by the plugin id).

use itala_kernel::PluginId;

/// The `tracing` target all audit events use.
pub const AUDIT_TARGET: &str = "itala.plugin.audit";

/// A span for a privileged call by `plugin` of kind `op` (e.g. `read_file`).
pub fn call_span(plugin: &PluginId, op: &'static str) -> tracing::Span {
    tracing::info_span!(
        target: AUDIT_TARGET,
        "plugin.call",
        op,
        plugin = %plugin,
    )
}

/// Records that a capability was granted to `plugin`.
pub fn grant(plugin: &PluginId, capability: &str) {
    tracing::info!(
        target: AUDIT_TARGET,
        plugin = %plugin,
        capability,
        action = "grant",
        "capability granted"
    );
}

/// Records that `plugin` used a capability successfully.
pub fn used(plugin: &PluginId, capability: &str) {
    tracing::debug!(
        target: AUDIT_TARGET,
        plugin = %plugin,
        capability,
        action = "use",
        "capability used"
    );
}

/// Records that `plugin` was denied a capability.
pub fn denied(plugin: &PluginId, capability: &str) {
    tracing::warn!(
        target: AUDIT_TARGET,
        plugin = %plugin,
        capability,
        action = "deny",
        "capability denied"
    );
}
