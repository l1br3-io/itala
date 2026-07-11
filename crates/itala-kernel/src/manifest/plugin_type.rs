//! [`PluginType`] — the category of functionality a plugin provides.

use serde::{Deserialize, Serialize};

/// The kind of extension a plugin implements.
///
/// Serialized in lower case (e.g. `"editor"`, `"ai_provider"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PluginType {
    /// Extends or replaces the note editor.
    Editor,
    /// CSS custom properties and styling rules.
    Theme,
    /// Implements a VFS backend.
    Storage,
    /// Provides a sync transport and conflict resolver.
    Sync,
    /// Implements the search index and query interface.
    Search,
    /// Implements an LLM/embedding provider.
    AiProvider,
    /// Adds export formats or templates.
    Exporter,
    /// Adds CLI subcommands or command-palette actions.
    Command,
    /// Custom inline or block rendering.
    Renderer,
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const T: fn() = check::<PluginType>;
    const _: [(); 0] = [(); {
        let _ = T;
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_snake_case() {
        let v = serde_json::to_value(PluginType::AiProvider).unwrap();
        assert_eq!(v, serde_json::json!("ai_provider"));
        let back: PluginType = serde_json::from_value(v).unwrap();
        assert_eq!(back, PluginType::AiProvider);
    }
}
