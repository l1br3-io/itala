# wit-bindgen

Single-page reference for the WIT contract between host and plugins.

## WIT file structure

```
crates/itala-plugin-api/wit/
├── world.wit              # The exported "plugin" world
├── deps.toml              # External WIT package deps (e.g. wasi)
├── types.wit              # Shared records, enums, type aliases
├── editor.wit             # Editor extension interface
├── storage.wit            # VFS backend interface
├── sync.wit               # Sync transport interface
├── ai.wit                 # AI provider interface
└── lifecycle.wit          # Plugin lifecycle hooks
```

## Shared types

```wit
// types.wit
package itala:plugin-api@0.1.0;

interface types {
    record note-id { value: string }
    record vault-id { value: string }
    record plugin-config { json: string }

    variant link-target {
        note(note-id),
        url(string),
        block(tuple<note-id, string>),
    }

    type html = string;
    type ast-bytes = list<u8>;
}
```

WIT primitives: `bool`, `s8..s64`, `u8..u64`, `f32`, `f64`, `char`, `string`, `list<T>`, `option<T>`, `result<T, E>`, `tuple<...>`. Compound: `record`, `variant`, `enum`, `flags`, `resource`.

## Interface example

```wit
// editor.wit
package itala:plugin-api@0.1.0;

interface editor {
    use types.{note-id, html, ast-bytes};

    record render-context {
        note: note-id,
        viewport-width: u32,
        theme: string,
    }

    render-block: func(ctx: render-context, ast: ast-bytes) -> result<html, string>;
    render-inline: func(ctx: render-context, ast: ast-bytes) -> result<html, string>;
}
```

## World

```wit
// world.wit
package itala:plugin-api@0.1.0;

world plugin {
    import host-vfs;
    import host-events;
    import host-config;

    export editor;
    export lifecycle;
}
```

The world is what plugins implement (`export`) and what the host provides (`import`).

## Generated bindings — host side

```rust
wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
    async: true,
    with: {
        "itala:plugin-api/types/note-id": itala_kernel::NoteId,
    },
});
```

`with` lets you reuse existing Rust types instead of generated wrappers. Use it for kernel types (`NoteId`, `VaultId`).

## Generated bindings — guest side (Rust plugin)

```rust
wit_bindgen::generate!({
    path: "../wit",
    world: "plugin",
    pub_export_macro: true,
});

struct MyPlugin;

impl exports::itala::plugin_api::editor::Guest for MyPlugin {
    fn render_block(ctx: RenderContext, ast: Vec<u8>) -> Result<String, String> {
        // user code
    }
}

export!(MyPlugin);
```

The `export!` macro wires the Rust impl into the component's exports.

## Versioning

Every WIT package has a version: `package itala:plugin-api@0.1.0;`. Rules:
- Pre-1.0: minor bump = breaking. Refuse to load mismatched majors.
- Post-1.0: minor bump = additive only. Major bump = breaking.

Plugins record the API version they were built against in `plugin.toml`. The host uses this to decide compatibility.

## Naming

WIT idiom: kebab-case for everything (interfaces, functions, records, fields). The bindgen translates:

| WIT | Rust |
|---|---|
| `note-id` | `NoteId` |
| `render-block` | `render_block` |
| `render-context` | `RenderContext` |

## Testing the contract

`crates/itala-plugin-host/tests/contract.rs` instantiates a tiny test plugin (`tests/fixtures/echo-plugin.wasm`) and asserts the round-trip works. Build the fixture in `build.rs`:

```rust
// build.rs
let status = std::process::Command::new("cargo")
    .args(["build", "--target", "wasm32-wasip2", "--release",
           "--manifest-path", "tests/fixtures/echo-plugin/Cargo.toml"])
    .status()?;
assert!(status.success());
```

## Editor support

`vscode-wit` syntax highlighting. `wit-deps` for dep management. `wkg` (wasm pkg tool) to publish/consume WIT packages from a registry once the API stabilizes.
