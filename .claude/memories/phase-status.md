# Phase Status

Current development progress for Itala.

## Current state

**Phase: Design**
**Next: MVP foundation**

No runtime code in the repo yet. Active work is shaping product scope, architecture, and the plugin contract.

## Phase definitions (from `docs/ROADMAP.md`)

| Phase | Title | Status |
|---|---|---|
| Design | Vision, product, roadmap, architecture, plugin contract | In Progress |
| MVP — Foundation | Cargo workspace, kernel, vault context | Not started |
| MVP — Desktop | Tauri shell + webview + CodeMirror + yrs bridge | Not started |
| MVP — Search | Tantivy integration, incremental updates | Not started |
| MVP — Plugin Host | wasmtime, WIT contracts, capability enforcement | Not started |
| MVP — CLI | `itala` binary for headless operations | Not started |
| MVP — Curated Plugins | First-party plugins (themes, exporters) | Not started |
| Post-MVP | Web + mobile shells | Not started |
| Post-MVP | AI plugin (cloud + local) | Not started |
| Post-MVP | Sync plugin | Not started |

## Documentation status

- `docs/VISION.md` — Done
- `docs/PRODUCT.md` — Done
- `docs/ROADMAP.md` — Done
- `docs/ARCHITECTURE.md` — Done
- `CLAUDE.md` — Done
- Plugin contract (WIT files in `crates/itala-plugin-api/wit/`) — pending; design happens in the first MVP sprint

## Performance targets (MVP)

From `docs/ARCHITECTURE.md` §4.1:

- Cold start desktop: < 500 ms
- Cold start mobile: < 1 s
- Open 10 KB note: < 50 ms
- Full-text search over 10k notes: < 200 ms
- Keystroke → screen: < 16 ms
- Kernel memory: < 20 MB

## Dependencies between phases

- MVP — Foundation precedes everything else (workspace, kernel types).
- Desktop and Search can proceed in parallel once Foundation lands.
- Plugin Host can begin after Foundation; it does not depend on Desktop or Search.
- CLI can begin after Foundation; useful as a smoke test for kernel APIs.
- Curated Plugins requires Plugin Host.

## Stack lock-in

The tech-stack decision (Rust kernel + Tauri + thin TS shell + yrs/tantivy/wasmtime/redb/opendal/comrak) was made on 2026-05-04. It is the assumed substrate for every sprint plan and capability brief unless explicitly revisited.
