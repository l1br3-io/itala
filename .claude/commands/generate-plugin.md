# /generate-plugin

Scaffold a new WASM plugin crate from a WIT contract.

## Usage

```
/generate-plugin <plugin-name> <plugin-type>
```

- `<plugin-name>`: kebab-case (e.g. `mermaid-renderer`, `weather-block`).
- `<plugin-type>`: one of `editor`, `theme`, `storage`, `sync`, `ai-provider`, `exporter`, `command`, `renderer`.

The crate lives at `plugins/<plugin-name>/` (outside the main workspace — plugins build separately to `wasm32-wasip2`).

## What this command produces

When invoked, you (the agent) MUST:

1. Read `rules/rust/wasm-plugins.md`, `skills/wit-bindgen.md`, `skills/wasmtime-component-model.md`, `rules/rust/error-handling.md`.
2. Locate the WIT interface for `<plugin-type>` in `crates/itala-plugin-api/wit/`.
3. Confirm the plugin doesn't already exist.
4. Generate the files below using the Write tool.
5. Print a summary plus build instructions: `cd plugins/<plugin-name> && cargo build --target wasm32-wasip2 --release`.

## Files to create

### `plugins/<plugin-name>/Cargo.toml`

```toml
[package]
name = "<plugin-name>"
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
description = "<one-line description>"

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.34"

[profile.release]
opt-level = "z"        # smaller wasm
lto = true
codegen-units = 1
strip = "symbols"
```

The plugin is a standalone Cargo project, not a workspace member. It only depends on `wit-bindgen` plus whatever it needs internally.

### `plugins/<plugin-name>/plugin.toml`

```toml
[plugin]
id = "com.example.<plugin-name>"
name = "<Plugin Name>"
version = "0.1.0"
api = "0.1"
type = "<plugin-type>"
description = "<one-line description>"
homepage = ""
license = "MIT OR Apache-2.0"

[capabilities]
required = [
    # Minimum capabilities. Justify each.
]
optional = [
    # User-prompted on first use.
]

[entry]
wasm = "target/wasm32-wasip2/release/<plugin_name>.wasm"
```

Replace `<plugin_name>` with the snake_case wasm artifact name (cargo generates this from the package name).

### `plugins/<plugin-name>/src/lib.rs`

```rust
//! <plugin-name>: <one-line description>
//!
//! Itala plugin (type: <plugin-type>).

#![forbid(unsafe_code)]

wit_bindgen::generate!({
    path: "../../crates/itala-plugin-api/wit",
    world: "plugin",
    pub_export_macro: true,
});

struct Plugin;

// Implement each exported interface. Pick one based on plugin type:

// For type = "editor":
// impl exports::itala::plugin_api::editor::Guest for Plugin {
//     fn render_block(ctx: RenderContext, ast: Vec<u8>) -> Result<String, String> {
//         // …
//     }
// }

// Lifecycle is required for every plugin:
impl exports::itala::plugin_api::lifecycle::Guest for Plugin {
    fn init(_config: PluginConfig) -> Result<(), String> {
        Ok(())
    }
    fn activate() -> Result<(), String> {
        Ok(())
    }
    fn deactivate() -> Result<(), String> {
        Ok(())
    }
    fn config_changed(_config: PluginConfig) -> Result<(), String> {
        Ok(())
    }
}

export!(Plugin);
```

### `plugins/<plugin-name>/README.md`

```markdown
# <plugin-name>

<one-line description>

## Build

cargo build --target wasm32-wasip2 --release

## Install (local dev)

itala plugin install ./target/wasm32-wasip2/release/<plugin_name>.wasm

## Capabilities

<list each capability and why it's needed>
```

## Capability discipline

For each capability listed in `plugin.toml`, you MUST:
- State the function in this plugin that uses it.
- Confirm it is the minimum permission required (e.g. don't request `host.vfs.write` when `host.vfs.read` suffices).
- Document the user-facing prompt copy if it's `optional`.

If you can't justify a capability, remove it.

## Acceptance checks

- `cargo build --target wasm32-wasip2 --release` produces a `.wasm` artifact.
- `cargo clippy --target wasm32-wasip2 -- -D warnings` passes.
- The plugin's `lifecycle` interface is fully implemented.
- The plugin's primary type interface (e.g. `editor`) is fully implemented.
- `plugin.toml` has every required field and a non-empty `capabilities.required`.
- The plugin loads under `wasmtime` (the host's `cargo nextest run -p itala-plugin-host` should be able to instantiate the produced wasm).

## Anti-patterns

- Do not request `optional` capabilities you don't actually use — users will distrust the plugin.
- Do not call `panic!` from a plugin function. Return `Err(String)` from the WIT result.
- Do not store mutable global state via `static mut`. Use `OnceCell` or thread-local with documentation.
- Do not depend on host crates (`itala-kernel`, `itala-plugin-host`). The only contract is `itala-plugin-api`'s WIT.
- Do not bundle large data files into the wasm. Ship them separately in the plugin package.
