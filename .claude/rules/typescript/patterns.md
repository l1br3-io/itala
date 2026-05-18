---
paths:
  - "apps/**/*.ts"
  - "apps/**/*.tsx"
  - "apps/**/*.js"
  - "apps/**/*.jsx"
---
# TypeScript Patterns

> Applies to the Tauri webview shell. Patterns chosen to fit a thin layer over Rust commands.

## Tauri command wrapper

All commands go through a typed wrapper file that mirrors Rust:

```ts
// ui/src/api/notes.ts
import { invoke } from '@tauri-apps/api/core'
import type { Note, NoteId } from '../bindings'

export const readNote = (id: NoteId): Promise<Note> =>
  invoke<Note>('read_note', { id })

export const listNotes = (vault: string): Promise<Note[]> =>
  invoke<Note[]>('list_notes', { vault })
```

Components consume these. Never `invoke` from a component file.

## Tauri event subscription

```ts
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { onCleanup } from 'solid-js'

import type { NoteSavedPayload } from '../bindings'

export function onNoteSaved(handler: (payload: NoteSavedPayload) => void) {
  let unlisten: UnlistenFn | undefined
  listen<NoteSavedPayload>('note-saved', e => handler(e.payload))
    .then(fn => { unlisten = fn })
  onCleanup(() => unlisten?.())
}
```

Always clean up listeners via `onCleanup` (Solid) or the framework's equivalent.

## Repository shape (client-only state)

For collections that the webview manages locally — recently-opened vaults, custom keymaps, theme presets stored in localStorage:

```ts
interface Repository<T, Id = string> {
  list(): Promise<T[]>
  get(id: Id): Promise<T | null>
  put(value: T): Promise<void>
  remove(id: Id): Promise<void>
}
```

Don't use this shape for note data — that goes through Tauri commands directly.

## Debounced derived signal (Solid)

```ts
import { createSignal, createEffect, onCleanup } from 'solid-js'

export function createDebouncedSignal<T>(
  source: () => T,
  delayMs: number,
): () => T {
  const [value, setValue] = createSignal(source())
  createEffect(() => {
    const next = source()
    const handle = setTimeout(() => setValue(() => next), delayMs)
    onCleanup(() => clearTimeout(handle))
  })
  return value
}
```

Use for search input, palette filters — anywhere a sync over IPC would be wasteful per keystroke.

## Async resource (Solid)

`createResource` is the standard for async fetches:

```ts
import { createResource } from 'solid-js'
import { listNotes } from '../api/notes'

export function useNotes(vault: () => string) {
  const [notes, { refetch }] = createResource(vault, listNotes)
  return { notes, refetch }
}
```

Resources track loading state automatically. Pair with `<Suspense>` for the loading UI.

## Pure helpers

Plain functions, no classes. Live in `ui/src/lib/`. Tested with Vitest.

```ts
export function basenameOf(path: string): string {
  const idx = path.lastIndexOf('/')
  return idx === -1 ? path : path.slice(idx + 1)
}
```

## Toast / banner

A single `toast` store funnels all transient UI messages. Never `alert()`. Never push messages from inside a component without bouncing through the store.

```ts
import { toast } from '../stores/toast'

toast.success('Note saved')
toast.error('Failed to save', { details: error.message })
```

## CodeMirror extension shape

Custom extensions follow the CodeMirror 6 idiom — `StateField` for state, `ViewPlugin` for view-side decorations, `Compartment` for runtime reconfiguration. See `skills/codemirror-6.md`.

```ts
import { StateField } from '@codemirror/state'
import { EditorView, type DecorationSet } from '@codemirror/view'

export const wikiLinkField = StateField.define<DecorationSet>({
  create: state => buildDecorations(state.doc),
  update: (deco, tr) => tr.docChanged ? buildDecorations(tr.newDoc) : deco,
  provide: f => EditorView.decorations.from(f),
})
```

## Anti-patterns

- Pulling note bodies into a Solid signal. CodeMirror owns the document.
- Polling Tauri commands when an event would do. Subscribe to events.
- Top-level `await` in modules. Wrap in an `init()` exported from the entrypoint.
- `useState`/hooks-style code (this is Solid; signals look similar but rules differ).
- Any global mutable singleton that isn't a Solid store or a `Y.Doc`.
