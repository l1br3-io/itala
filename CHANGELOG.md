# Changelog

Reverse-chronological ship history. Seeded from the former roadmap milestone narratives
when the planning docs were refactored to the harness layout; future entries come from each
phase's `/ship` step.

## M1 — Vaults & Notes ✅

Users can create and open vaults — logical containers of notes, assets, and configuration
that default to a local folder but can later be backed by storage plugins. Notes are
canonical plain Markdown with optional YAML frontmatter. Moving, renaming, and deleting notes
updates links automatically. Snapshot history is maintained locally and prior content can be
restored; visual diff/merge remains deferred.

- Crates: `itala-note`, `itala-vault`, `itala-store-files`, and the `apps/itala-cli` shell.
- Kernel gained the `HistoryStore`/`DynHistoryStore` port and the `FilesHistoryStore`
  adapter (ADR-0004 §3).
- CLI surface shipped: `init`, `notes {create,list,show,move,delete}`,
  `history {list,show,restore}`.
- Deferred: concurrent multi-vault sessions, diff/merge viewer UI, SQLite derived-state
  adapter, CRDT sync state.
- `just ci` green (145+ tests + a kernel-history suite).

## M0 — Foundation ✅

The kernel and plugin host run. The kernel provides shared types, an event bus, a virtual
file system abstraction, a configuration registry, and the capability policy that mediates
calls made through the host. M0 plugins are trusted, compile-time-registered native Rust
components: lifecycle panics are contained at host call boundaries, but they are not a
security sandbox and can bypass host policy if malicious. Data persists durably across
restarts.

- Crates: `itala-kernel`, `itala-plugin-api`, `itala-plugin-host`, `itala-vfs-fs`.
- No user-facing surface — verified entirely through tests.
- `just ci` green (73 tests + 12 doctests).
