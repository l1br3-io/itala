# Code Style Guide

Style conventions across Itala. Language-specific rules live in subfolders; this file states the principles that apply everywhere.

## Principles

1. **Clarity over cleverness** — code is read more than written.
2. **Consistency** — follow existing patterns; don't introduce a third way without good reason.
3. **No premature abstractions** — write what's needed, not what might be.
4. **Semantic naming** — names tell you what something is, not how it's stored.
5. **DRY where it counts** — duplicate three lines before extracting a helper for two.
6. **No comments restating the code** — only explain *why* when the why isn't obvious.

## Language-specific rules

- **Rust** (kernel, contexts, plugin host, CLI, all libraries): see `rules/rust/` (11 files — workspace, api-design, error-handling, async, ownership-and-lifetimes, traits-and-dispatch, unsafe-and-ffi, security, testing, tooling, wasm-plugins).
- **TypeScript** (Tauri webview shell only): see `rules/typescript/` (5 files).

If you are touching Rust, load the relevant `rules/rust/*.md` files before writing code.

## Where each language lives

| Language | Where |
|---|---|
| Rust | `crates/`, `apps/desktop/src-tauri/`, `apps/mobile/src-tauri/`, `plugins/` |
| TypeScript | `apps/desktop/ui/`, `apps/mobile/ui/`, `apps/web/` |

The kernel never touches TypeScript. The webview never touches Rust except through generated Tauri command bindings.

## Comments

- Why, not what: `// Hash the canonical form so block IDs survive whitespace edits`
- Clarify assumptions: `// Caller already validated `path` is inside the vault root`
- Document non-obvious logic: complex algorithms, workarounds, hacks
- TODOs reference an issue: `// TODO(#42): handle CRDT garbage collection`

Don't:
- Restate obvious code
- Comment out code (delete it; git remembers)
- Comment for every line

## Imports

Within each language, imports are grouped — standard library / third-party / local. The `rules/<lang>/coding-style.md` file gives concrete examples.

## Line length

- Rust: 100 (rustfmt default); the formatter is authoritative.
- TypeScript: 100; Prettier/Biome is authoritative.

## Formatting

Code is formatted by tooling, not by hand. Editors run formatters on save; CI rejects unformatted code. See `rules/rust/tooling.md` and `rules/typescript/coding-style.md`.

## Testing

- Tests are first-class. Untested code is unfinished code.
- Test names describe behavior: `parses_yaml_frontmatter`, not `test1`.
- Per-language testing rules in `rules/rust/testing.md` and `rules/typescript/testing.md`.
