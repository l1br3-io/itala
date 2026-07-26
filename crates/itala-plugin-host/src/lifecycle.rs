//! Lifecycle data structures and panic-containment helpers.
//!
//! Each lifecycle method runs inside its own `tokio::task` so an ordinary
//! panic in the plugin's `initialize` / `activate` / `deactivate` is caught
//! (the [`JoinError`] is mapped to [`Error::PluginCrashed`]) and the host
//! keeps running. This is best-effort resilience to ordinary bugs, **not** a
//! security boundary — native plugins can still corrupt process state or
//! call `abort()` (see
//! [ADR-0005](../../../docs/adr/0005-plugin-trust-and-rollout.md)).

use std::sync::Arc;

use itala_plugin_api::Plugin;
use tokio::task::JoinError;

use itala_kernel::{Error, PluginId, Result};

use crate::enforcement::HostContext;

/// The runtime status of a loaded plugin.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PluginStatus {
    /// Loaded but not yet active.
    Inactive,
    /// Fully active and servicing requests.
    Active,
    /// Failed (lifecycle error or crash); carries the reason.
    Failed(String),
}

impl PluginStatus {
    /// Returns `true` if the plugin is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the plugin has failed.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// Runs `plugin.initialize` under task-scoped panic containment.
///
/// # Errors
/// [`Error::PluginCrashed`] if the task panicked; otherwise the plugin's own
/// error is forwarded.
pub async fn initialize(plugin: Arc<dyn Plugin>, ctx: HostContext) -> Result<()> {
    let id = plugin.id();
    let p = plugin.clone();
    run_with_panic_containment(id, async move { p.initialize(&ctx).await }).await
}

/// Runs `plugin.activate` under task-scoped panic containment. See [`initialize`].
pub async fn activate(plugin: Arc<dyn Plugin>, ctx: HostContext) -> Result<()> {
    let id = plugin.id();
    let p = plugin.clone();
    run_with_panic_containment(id, async move { p.activate(&ctx).await }).await
}

/// Runs `plugin.deactivate` under task-scoped panic containment. See [`initialize`].
pub async fn deactivate(plugin: Arc<dyn Plugin>, ctx: HostContext) -> Result<()> {
    let id = plugin.id();
    let p = plugin.clone();
    run_with_panic_containment(id, async move { p.deactivate(&ctx).await }).await
}

/// Spawns a future on the runtime and maps a panic to `PluginCrashed`.
async fn run_with_panic_containment<F>(id: PluginId, fut: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>> + Send + 'static,
{
    match tokio::spawn(fut).await {
        Ok(inner) => inner,
        Err(join_err) => Err(crashed(id, &join_err)),
    }
}

fn crashed(id: PluginId, join_err: &JoinError) -> Error {
    if join_err.is_panic() {
        Error::PluginCrashed(id)
    } else {
        Error::plugin(format!("lifecycle task cancelled for {id}"))
    }
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const S: fn() = check::<PluginStatus>;
    const _: [(); 0] = [(); {
        let _ = S;
        0
    }];
};
