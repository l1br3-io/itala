# Tauri 2

Single-page reference for the Tauri 2 desktop and mobile shell.

## Project layout

```
apps/desktop/
├── Cargo.toml
├── tauri.conf.json
├── src/
│   ├── main.rs               # tauri::Builder + commands
│   └── commands/             # one file per command group
└── ui/                       # webview frontend (TS + bundler)
    ├── package.json
    ├── vite.config.ts
    └── src/
```

`apps/mobile/` mirrors the same shape with `tauri.conf.json` mobile entries.

## Commands (Rust → TS)

```rust
// apps/desktop/src/commands/notes.rs
use tauri::State;
use itala_kernel::{NoteId, Note};

#[tauri::command]
pub async fn read_note(
    id: NoteId,
    vault: State<'_, VaultHandle>,
) -> Result<Note, AppError> {
    vault.read(id).await
}

// main.rs
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![commands::notes::read_note])
    .manage(vault_handle)
    .run(tauri::generate_context!())?;
```

```ts
// ui/src/api.ts
import { invoke } from '@tauri-apps/api/core'
import type { Note, NoteId } from './bindings'

export const readNote = (id: NoteId) => invoke<Note>('read_note', { id })
```

Errors: implement `serde::Serialize` on `AppError` so JS receives structured failures, not strings.

## Type sharing (Rust ↔ TS)

Use **`specta`** + `tauri-specta` to generate TS bindings from Rust types. Annotate every type that crosses the IPC boundary:

```rust
#[derive(Serialize, Deserialize, specta::Type)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub body: String,
}
```

`tauri-specta` emits `ui/src/bindings.ts` at build time. Never hand-write IPC types in TS.

## Events (Rust → TS, push)

```rust
app_handle.emit("note-saved", &payload)?;
```

```ts
import { listen } from '@tauri-apps/api/event'
const unlisten = await listen<NoteSavedPayload>('note-saved', e => { /* … */ })
```

Events are fire-and-forget. Use commands for request/response.

## Capabilities (security)

`tauri.conf.json` declares which webview origins can call which commands:

```json
{
  "app": {
    "security": {
      "capabilities": ["main"]
    }
  }
}
```

`src-tauri/capabilities/main.json`:

```json
{
  "identifier": "main",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:allow-read-text-file",
    { "identifier": "core:event:allow-listen", "allow": [{ "event": "note-*" }] }
  ]
}
```

Default-deny: only listed permissions are reachable from the webview. Add to the allowlist consciously.

## Mobile

Same `Builder`. Different conf:

```json
"bundle": {
  "android": { "minSdkVersion": 24 },
  "ios": { "minimumSystemVersion": "14.0" }
}
```

Build with `cargo tauri android build` / `cargo tauri ios build`. Plugins that need native code go through `tauri::plugin::Builder` with platform-specific impls.

## State

```rust
struct VaultHandle(Arc<itala_vault::Vault>);

tauri::Builder::default()
    .manage(VaultHandle(Arc::new(vault)))
    .invoke_handler(...)
```

Access in commands via `State<'_, VaultHandle>`. `Arc` because Tauri shares state across worker threads.

## File watching

Use the `tauri-plugin-fs` permissions for vault folder access; pair with the `notify` crate inside the kernel for change events. Re-emit kernel events into Tauri's event bus.

## Logging

`tauri-plugin-log` bridges `tracing` to the platform logger:

```rust
.plugin(tauri_plugin_log::Builder::new().build())
```

Frontend logs flow through `info`/`warn`/`error` from `@tauri-apps/plugin-log`.

## Updating

`tauri-plugin-updater` for desktop. Ship signed update artifacts; the public key lives in `tauri.conf.json`.

## What lives where

| Concern | Layer |
|---|---|
| Editor state, CRDT, parse, links | Rust kernel |
| Search index | Rust kernel |
| Render layer | TypeScript (CodeMirror + custom renderers) |
| Theme | TypeScript (CSS custom properties) |
| File I/O | Rust kernel — TS never touches `fs:*` directly |
| AI calls | Rust (`itala-ai`) — keys never leave Rust |
| Sync | Rust (`itala-sync`) |
