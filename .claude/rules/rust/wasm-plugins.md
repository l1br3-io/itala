# Rust WASM Plugin Discipline

## Where things live

```
crates/itala-plugin-api/
├── Cargo.toml
├── wit/
│   ├── world.wit                # Top-level world: imports/exports
│   ├── editor.wit               # Editor extension interface
│   ├── storage.wit              # VFS backend interface
│   ├── sync.wit                 # Sync transport interface
│   └── ai.wit                   # AI provider interface
└── src/
    └── lib.rs                   # Generated bindings + Rust-side trait
```

The `wit/` directory is the source of truth. Rust traits are generated; never hand-written.

## WIT interface conventions

```wit
package itala:plugin-api@0.1.0;

interface editor {
    use types.{note-id, html};

    record render-context {
        note: note-id,
        viewport-width: u32,
    }

    render-block: func(ctx: render-context, ast-fragment: list<u8>) -> result<html, string>;
}

world plugin {
    import host-vfs;
    import host-events;
    export editor;
    export plugin-lifecycle;
}
```

- All field and function names: kebab-case (WIT idiom).
- All record names: kebab-case; the binding generator translates to `PascalCase` Rust types.
- Versions explicit on every package; bump on any breaking change.
- One `.wit` file per interface; never a god-file.

## Generated bindings

`build.rs` invokes `wit-bindgen` with the component model:

```rust
wit_bindgen::generate!({
    path: "wit",
    world: "plugin",
    pub_export_macro: true,
});
```

Re-export the generated types from `lib.rs`:

```rust
pub use bindings::editor::{Editor, RenderContext};
pub use bindings::host_vfs::HostVfs;
```

Plugin authors only see Rust types; WIT is an implementation detail.

## Capability declarations

Every plugin manifest declares required capabilities. The host enforces.

```toml
# plugin.toml
[plugin]
id = "com.example.mermaid-renderer"
version = "0.4.1"
api = "0.1"

[capabilities]
required = [
    "editor.render-block",        # extends the editor
    "host.events.note-saved",     # listens to events
]
optional = [
    "host.vfs.read",              # reads note files (asks user)
]
```

Plugins request only what they use. Optional capabilities prompt the user on first use.

## Host-guest boundary

```
┌──────────────────────┐                  ┌──────────────────────┐
│   itala-plugin-host  │  wasmtime call   │      WASM plugin     │
│                      │ ───────────────► │                      │
│  capability check    │                  │   bindings::Editor   │
│  type validation     │ ◄─────────────── │   user code (Rust /  │
│  resource lifetime   │   wasmtime call  │   AssemblyScript)    │
└──────────────────────┘                  └──────────────────────┘
```

Rules:
- Host calls happen through wasmtime's `Linker`. No raw FFI.
- Resources (note handles, vault handles) cross the boundary as `wasmtime::component::Resource<T>`. The host owns the data; the guest holds a handle.
- Errors cross as WIT `result<_, _>`. The guest cannot panic the host — wasmtime traps stay isolated.
- Cancellation: long-running plugin calls are wrapped in `tokio::time::timeout`. Plugins that exceed the budget are killed and isolated.

## Lifecycle

Every plugin world exports:

```wit
interface plugin-lifecycle {
    init: func(config: plugin-config) -> result<_, string>;
    activate: func() -> result<_, string>;
    deactivate: func() -> result<_, string>;
    config-changed: func(config: plugin-config) -> result<_, string>;
}
```

Host calls `init` once at install, `activate` on every host startup if the plugin is enabled, `deactivate` on disable/shutdown, `config-changed` when the user changes settings.

## Crash isolation

A panic in a plugin is a wasmtime trap. The host catches the trap, marks the plugin as crashed, surfaces the error to the user, and continues. The host never crashes because of a plugin.

## Versioning

- WIT package version is the plugin API version.
- `0.x` minor bumps are breaking; `1.0`+ minor bumps are additive only.
- Plugins declare `api = "0.1"` in `plugin.toml`. The host refuses to load mismatched majors.

## Testing plugins

- Unit-test plugin business logic against the trait (no WASM in the loop).
- Integration-test the WIT contract by loading the compiled plugin in a `wasmtime::component::Component` inside `itala-plugin-host` tests.
- The plugin author SDK ships a test harness crate that mocks the host for fast iteration.

## What plugins must not do

- No filesystem access except through `host.vfs.*` capabilities.
- No network access except through `host.fetch` (post-MVP) with explicit allowlist.
- No process spawning — the host has no API for it.
- No `unsafe` Rust in plugins; `wasm32-wasi` plus the component model give a sound execution environment.
