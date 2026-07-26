---
status: shipped
started: 2026-05-29
shipped: 2026-05-29
---

# SPEC — M1: Vaults & Notes

> **Status:** Shipped. Paired with `plan.md` and `todo.md` in this folder.
> **Phase:** M1 — Vaults & Notes (CLI-first MVP-α increment).
> **Architecture:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) (§2 crate map, §5 data model, §6 flows).
> **Decisions:** [ADR-0004](../../adrs/0004-derived-state-storage-strategy.md) (derived-state storage), [ADR-0006](../../adrs/0006-markdown-canonical-source.md) (Markdown canonical).

This is the phase spec for M1, extracted from the former omnibus functional-contracts
document. It states the user-visible requirements the phase had to satisfy.

## 1. Vaults and Notes

A **vault** is a logical container of notes, assets, and configuration. By default it maps
to a folder on the local filesystem, but the physical storage is pluggable via the VFS
abstraction — backends such as S3, IndexedDB (web), WebDAV, or a database can be provided
by a Storage plugin without changing any other part of the system.

- Users can create, open, and switch between multiple vaults. M1 opens one vault per CLI
  process; concurrent switching is a desktop requirement.
- Vault metadata and settings are stored in a `.itala` subfolder (or equivalent location
  in non-filesystem backends).
- Notes are stored as plain Markdown (`.md`) files with optional YAML frontmatter for
  metadata; this representation is the canonical source of truth (ADR-0006). The VFS layer
  handles translating it to the underlying storage backend without making an editor model,
  index, or synchronization structure authoritative over the Markdown.
- Users can move, rename, and delete notes; links update automatically.
- File version history is maintained locally. M1 provides snapshots and restore;
  configurable retention and visual diff viewing are later increments.

## Scope (M1 CLI surface)

`init`, `notes {create,list,show,move,delete}`, `history {list,show,restore}`.

## Out of scope for M1

- Concurrent multi-vault sessions (desktop shell, post-M1).
- Graph index, backlinks, unlinked mentions (M3).
- Diff/merge viewer UI (post-M1).
- SQLite derived-state adapter (ADR-0004 §3 lists it as opt-in; M1 ships `files` only).
- CRDT synchronization state — subordinate to canonical Markdown; deferred (ADR-0006).
