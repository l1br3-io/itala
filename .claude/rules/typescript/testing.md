---
paths:
  - "apps/**/*.ts"
  - "apps/**/*.tsx"
  - "apps/**/*.js"
  - "apps/**/*.jsx"
---
# TypeScript Testing

> Applies to the Tauri webview shell.

## Layers

| Layer | Tool | What it covers |
|---|---|---|
| Unit | **Vitest** | Pure functions, signals/stores, IPC wrappers (with mocked `invoke`) |
| Component | **Vitest + happy-dom** or **@solidjs/testing-library** | Solid components in isolation |
| Integration | **Playwright component testing** | Component + Tauri mock |
| End-to-end | **Playwright** | Whole app against a real Tauri build |

## Vitest setup

```ts
// vitest.config.ts
import { defineConfig } from 'vitest/config'
import solid from 'vite-plugin-solid'

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: 'happy-dom',
    setupFiles: ['./src/test/setup.ts'],
  },
})
```

Co-locate tests with source: `notes.ts` → `notes.test.ts`.

## Mocking Tauri

```ts
// src/test/setup.ts
import { vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))
```

Each test sets the expected return:

```ts
import { invoke } from '@tauri-apps/api/core'
import { readNote } from '../api/notes'

test('reads a note via IPC', async () => {
  vi.mocked(invoke).mockResolvedValue({ id: 'n1', title: 'Hi', body: '' })
  const note = await readNote('n1')
  expect(invoke).toHaveBeenCalledWith('read_note', { id: 'n1' })
  expect(note.title).toBe('Hi')
})
```

## Component tests

```tsx
import { render, fireEvent } from '@solidjs/testing-library'
import { NotePreview } from './NotePreview'

test('emits onSelect when clicked', () => {
  const onSelect = vi.fn()
  const { getByRole } = render(() => (
    <NotePreview note={{ id: 'n1', title: 'Hi', body: '' }} onSelect={onSelect} />
  ))
  fireEvent.click(getByRole('button'))
  expect(onSelect).toHaveBeenCalledWith('n1')
})
```

## Playwright (E2E)

End-to-end tests run against a built Tauri app. Reserved for critical user flows:
- Open a vault.
- Create a note, type, save, reopen — content survives.
- Wiki-link click navigates.
- Plugin install + activate.
- Settings persist across restart.

Don't try to E2E-test every corner case. Push that into Vitest unit tests where they run in milliseconds.

## What not to test

- Generated bindings (`bindings.ts`) — produced by `tauri-specta`.
- Third-party libraries.
- Trivial getters and prop pass-throughs.

## Coverage

`vitest --coverage` reports c8 coverage. Aim for high coverage on `ui/src/lib/` (pure helpers) and `ui/src/api/` (IPC wrappers); component coverage matters less than coverage of the logic they call.

## Snapshot testing

Use Vitest snapshots sparingly — only for stable output (rendered prose, error messages). Component DOM snapshots churn too often to be useful.
