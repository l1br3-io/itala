# CodeMirror 6

Single-page reference for the editor surface inside the Tauri webview.

## Why CodeMirror 6

- Modular extension model — every feature (line numbers, syntax, autocomplete) is an opt-in extension.
- Excellent performance with 100k-line documents.
- First-class collaborative editing via `y-codemirror.next` (the bridge to our yrs CRDT).
- Plain TS, no React/Solid coupling.

For Itala, CodeMirror 6 is the editor surface. The yrs `Doc` lives in Rust; the JS bridge mirrors it via the standard Y protocol.

## Bare-bones setup

```ts
import { EditorState } from '@codemirror/state'
import { EditorView, keymap, lineNumbers } from '@codemirror/view'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { markdown } from '@codemirror/lang-markdown'

const state = EditorState.create({
  doc: '',
  extensions: [
    lineNumbers(),
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    markdown(),
  ],
})

const view = new EditorView({ state, parent: document.querySelector('#editor')! })
```

## State / view split

- `EditorState` is **immutable**. Edits produce a new state via transactions.
- `EditorView` renders a state and forwards user input as transactions.
- All plugin state lives in `StateField`s; all derived UI lives in `ViewPlugin`s.

```ts
const wordCount = StateField.define<number>({
  create: state => state.doc.toString().split(/\s+/).length,
  update: (value, tr) => tr.docChanged
    ? tr.newDoc.toString().split(/\s+/).length
    : value,
})
```

## Yjs/yrs bridge

```ts
import * as Y from 'yjs'
import { yCollab } from 'y-codemirror.next'
import { Awareness } from 'y-protocols/awareness'

const ydoc = new Y.Doc()
const ytext = ydoc.getText('body')
const awareness = new Awareness(ydoc)

const state = EditorState.create({
  doc: ytext.toString(),
  extensions: [
    markdown(),
    yCollab(ytext, awareness),
  ],
})
```

The Tauri shell pipes yrs updates between Rust and JS:
- Rust → JS: `app.emit('y-update', updateBytes)` → JS applies via `Y.applyUpdate(ydoc, bytes)`.
- JS → Rust: `ydoc.on('update', bytes => invoke('apply_y_update', { id, bytes }))`.

The Y wire format is binary-compatible between yjs and yrs as of yrs 0.17+.

## Itala-specific extensions

- **wiki-link decorator** — regex-based decoration of `[[Note]]` with click-to-navigate.
- **block-ref decorator** — same for `[[Note#^block-id]]`.
- **transclusion preview** — inline embed widgets for `![[Note]]`.
- **command palette trigger** — `Cmd-K` opens the palette UI.
- **custom renderers** — plugin-provided block renderers register via a `ViewPlugin` that consumes a registration table from the host.

```ts
const wikiLinkPlugin = ViewPlugin.fromClass(class {
  decorations: DecorationSet
  constructor(view: EditorView) {
    this.decorations = this.compute(view)
  }
  update(u: ViewUpdate) {
    if (u.docChanged || u.viewportChanged) this.decorations = this.compute(u.view)
  }
  compute(view: EditorView): DecorationSet {
    // walk view.visibleRanges, regex match [[…]], emit Decoration.mark
  }
}, { decorations: v => v.decorations })
```

## Theming

- Light/dark themes via `EditorView.theme({...}, { dark: true })`.
- Itala themes are CSS custom properties; the CodeMirror theme reads them with `var(--editor-bg)` etc.
- High-contrast theme is bundled.

## Keymaps

- Defaults: `defaultKeymap`, `historyKeymap`, `searchKeymap`, `closeBracketsKeymap`.
- Itala bindings (palette, link follow, sidebar toggles) live in `ui/src/keymap/itala.ts` and are merged at construction.
- Multi-cursor: `EditorState.allowMultipleSelections.of(true)`.

## Performance

- `@codemirror/lang-markdown` ships its own incremental parser (Lezer). Don't add another.
- Large documents (>1 MB): enable `lineNumbers({ formatNumber: ... })` with caution; the gutter render is O(viewport).
- Avoid `EditorView.updateListener.of(u => ...)` for hot paths — define a `ViewPlugin` so updates batch.

## Testing

- `EditorState.create` works headless. Unit-test transactions and state fields without a DOM.
- For DOM-bound view tests, use `vitest` + `happy-dom` or Playwright component tests.

## What CodeMirror does NOT do

- Markdown rendering / preview — that's our renderer pipeline (Rust + plugin renderers in iframes or shadow DOM).
- File I/O — the editor is content-only; saves go through Tauri commands.
- AI features — completions and suggestions arrive as decorations or a side panel, not as built-in CodeMirror features.
