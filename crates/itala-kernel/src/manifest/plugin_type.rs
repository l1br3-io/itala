//! [`PluginType`] — the category of functionality a plugin provides.

use serde::{Deserialize, Serialize};

/// The kind of extension a plugin implements.
///
/// Serialized in lower case (e.g. `"editor"`, `"ai_provider"`). The four
/// `#[M5 initial]` variants (`Tool`, `Exporter`, `Command`, `Renderer`) ship
/// as the first public WASM extension surfaces; the others remain future
/// replacement surfaces (see
/// [ADR-0005](../../../../docs/adr/0005-plugin-trust-and-rollout.md) and
/// ARCHITECTURE.md §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PluginType {
    /// Acts on notes through a deterministic input/output contract; composes
    /// into Studio pipelines. M5 initial surface.
    Tool,
    /// Extends or replaces the note editor. Future surface.
    Editor,
    /// CSS custom properties and styling rules. Future surface.
    Theme,
    /// Implements a VFS backend. Future surface.
    Storage,
    /// Provides a sync transport and conflict resolver. Future surface.
    Sync,
    /// Implements the search index and query interface. Future surface.
    Search,
    /// Implements an LLM/embedding provider. Future surface.
    AiProvider,
    /// Adds export formats or templates. M5 initial surface.
    Exporter,
    /// Adds CLI subcommands or command-palette actions. M5 initial surface.
    Command,
    /// Custom inline or block rendering. M5 initial surface.
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

    #[test]
    fn tool_round_trips() {
        let v = serde_json::to_value(PluginType::Tool).unwrap();
        assert_eq!(v, serde_json::json!("tool"));
        let back: PluginType = serde_json::from_value(v).unwrap();
        assert_eq!(back, PluginType::Tool);
    }
}
