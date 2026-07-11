//! [`Plugin`] — the lifecycle trait plugins implement.

use async_trait::async_trait;
use itala_kernel::{PluginId, Result};

use crate::context::PluginContext;

/// A plugin: versioned, capability-gated extension code.
///
/// Plugins implement the three lifecycle methods and receive a
/// [`PluginContext`] for all privileged operations. The host drives
/// `initialize → activate → deactivate` and guarantees that a failing plugin
/// never crashes the host.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns the plugin's stable identifier (must match its manifest).
    #[must_use]
    fn id(&self) -> PluginId;

    /// Called once after discovery, before activation. Use for setup that does
    /// not depend on the plugin being "live".
    ///
    /// # Errors
    /// Returned errors mark the plugin `Failed`; the host continues.
    async fn initialize(&self, ctx: &dyn PluginContext) -> Result<()>;

    /// Called when the plugin becomes active and may start doing work.
    ///
    /// # Errors
    /// Returned errors mark the plugin `Failed`; the host continues.
    async fn activate(&self, ctx: &dyn PluginContext) -> Result<()>;

    /// Called on graceful shutdown or before a re-activation. Must release
    /// resources and stop background work.
    ///
    /// # Errors
    /// Returned errors are logged but do not prevent shutdown.
    async fn deactivate(&self, ctx: &dyn PluginContext) -> Result<()>;
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const ASSERT: fn() = check::<Box<dyn Plugin>>;
    const _: [(); 0] = [(); {
        let _ = ASSERT;
        0
    }];
};
