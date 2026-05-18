---
paths:
  - "apps/**/*.ts"
  - "apps/**/*.tsx"
  - "apps/**/*.js"
  - "apps/**/*.jsx"
---
# TypeScript Coding Style

> Applies to the thin webview shell inside Tauri (`apps/*/ui/`). The webview hosts CodeMirror 6, the command palette, settings, and theme. Heavy logic lives in Rust.

## Strictness

`tsconfig.json` enables every strictness flag:

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "moduleResolution": "Bundler",
    "module": "ESNext",
    "target": "ES2022"
  }
}
```

No `any`. No `// @ts-ignore` without a comment explaining why.

## Types and interfaces

- Use `interface` for object shapes that may be extended or implemented.
- Use `type` for unions, intersections, tuples, mapped types, utility types.
- Prefer string literal unions over `enum`.

```ts
interface Note {
  id: NoteId
  title: string
  body: string
}

type SyncStatus = 'idle' | 'syncing' | 'error' | 'offline'
```

## Public APIs

Exported functions, shared utilities, and class methods declare parameter and return types. Local variable types can be inferred.

```ts
export function formatVaultPath(path: VaultPath): string {
  return path.startsWith('/') ? path : `/${path}`
}
```

## `unknown` over `any`

External or untrusted input is `unknown`; narrow safely.

```ts
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return 'Unexpected error'
}
```

## Tauri command typing

Every IPC call goes through a typed wrapper. Types are generated from Rust via `tauri-specta` into `ui/src/bindings.ts` — never hand-written.

```ts
import { invoke } from '@tauri-apps/api/core'
import type { Note, NoteId } from './bindings'

export const readNote = (id: NoteId): Promise<Note> =>
  invoke<Note>('read_note', { id })

export const saveNote = (id: NoteId, body: string): Promise<void> =>
  invoke('save_note', { id, body })
```

Components import from `ui/src/api/`, never from `@tauri-apps/api/core` directly.

## Components

Components are SolidJS (or Svelte 5 if the project picks Svelte). Define props with a named `interface`:

```tsx
import type { JSX } from 'solid-js'
import type { Note } from '../bindings'

interface NotePreviewProps {
  note: Note
  onSelect: (id: Note['id']) => void
}

export function NotePreview(props: NotePreviewProps): JSX.Element {
  return (
    <button onClick={() => props.onSelect(props.note.id)}>
      {props.note.title}
    </button>
  )
}
```

No `React.FC`. No `forwardRef` (irrelevant outside React). No JSX patterns that assume virtual DOM — Solid has fine-grained reactivity, write components accordingly.

## Immutability

Use spread for updates; never mutate props or store values.

```ts
function withUpdatedTitle(note: Note, title: string): Note {
  return { ...note, title }
}
```

Solid signals are read-only at the consumption site. Mutations go through setters.

## Error handling

Async functions return typed errors at their boundaries. The IPC layer surfaces structured `AppError` from Rust:

```ts
import type { AppError } from './bindings'

async function loadNote(id: NoteId): Promise<Note | { error: AppError }> {
  try {
    return await readNote(id)
  } catch (error) {
    return { error: toAppError(error) }
  }
}
```

Never `alert()` or `console.error` to communicate failure to the user — use the toast/banner system.

## Input validation

Webview inputs (URL params, paste content, plugin-provided strings) validate via `zod`:

```ts
import { z } from 'zod'

const SettingsSchema = z.object({
  theme: z.enum(['light', 'dark', 'system']),
  fontSize: z.number().int().min(10).max(28),
})

type Settings = z.infer<typeof SettingsSchema>
```

`zod` schemas live next to the types they validate.

## Console.log

Disallowed in committed code. The pre-commit hook flags it. Use the structured logger that bridges to `tracing` on the Rust side via `tauri-plugin-log`.

## Imports

```ts
// External libraries
import { createSignal, createEffect } from 'solid-js'
import { invoke } from '@tauri-apps/api/core'

// Internal imports
import { NotePreview } from './components/NotePreview'
import { useTheme } from './stores/theme'
import type { Note, NoteId } from './bindings'
```

Type-only imports use `import type`. Bundler-resolved paths only — no `.js` extensions in source.
