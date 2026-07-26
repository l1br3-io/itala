# Implementation Plan: M1 — Vaults & Notes

> **Status:** Complete — all 16 features implemented; `just ci` green
> (lint + test + deny + doc); 145+ tests across `itala-note`, `itala-vault`,
> `itala-store-files`, and `apps/itala-cli`, plus a kernel-history test suite.
> **Milestone:** [M1 — Vaults & Notes](../../ROADMAP.md)
> **Architecture:** [ARCHITECTURE.md §2 (crate map), §5 (data model), §6 (flows)](../../ARCHITECTURE.md)
> **ADR-0004:** [Per-domain ports for derived-state storage](../../adrs/0004-derived-state-storage-strategy.md)

## Overview

Stand up the `itala-vault` (vault lifecycle, note CRUD, link rewriting) and
`itala-note` (note data model, Markdown/frontmatter parsing) bounded contexts,
the `itala-store-files` JSON-on-Vfs derived-state adapter (ADR-0004 §3), the
`HistoryStore`/`DynHistoryStore` port in `itala-kernel` (ADR-0004 §1), and the
first shell — `apps/itala-cli` (MVP-α) — wired through `clap` + `tokio`.

**Exit condition:** an integration test that initializes a vault in a tempdir,
creates notes, lists/shows/moves/renames/deletes them with automatic
`[[wiki-link]]` rewriting, snapshots history on every save, restores a prior
version, and shuts down cleanly. `just ci` green.

## Crates created in M1

```text
crates/
  itala-vault/          # bounded context: vault lifecycle + note CRUD + link rewriting
  itala-note/           # bounded context: Note model, frontmatter parser, slugify
  itala-store-files/    # adapter (ADR-0004 §3): JSON-on-Vfs for derived-state ports

apps/
  itala-cli/            # shell: clap + tokio binary — MVP-α
```

Plus a new port module in `itala-kernel`:

```text
crates/itala-kernel/src/
  history.rs            # HistoryStore + DynHistoryStore + IntoDynHistory bridge
  test_support/
    memory_history.rs   # in-memory adapter (test-support feature)
```

Workspace gains `apps/*` in `members` (per `Cargo.toml` comment: "re-joins when
the first shell lands").

---

## Architecture decisions

### AD-1: Markdown remains canonical; CRDT evaluation deferred

M1 ships `Note` with canonical plain Markdown bytes + typed frontmatter.
[ADR-0006](../../adrs/0006-markdown-canonical-source.md) makes that durable format
authoritative. M2 adds a subordinate document model and editing transactions,
not a CRDT field. CRDT evaluation is deferred until synchronization or
collaboration has concrete requirements.

### AD-2: Naive link rewriting lives in the vault context (user-confirmed)

ROADMAP §M1 promises "automatic link updates" on move/rename. The proper
graph index (backlinks, unlinked mentions) is M3. For M1 the vault scans
candidate notes for `[[wiki-link]]` patterns and rewrites text in-place on
rename. A small dedicated parser module in `itala-vault` (regex-free,
character-scanning) handles the matching. M3's derived graph index can later
accelerate candidate discovery, but the vault keeps ownership of durable link
rewrites and must be able to reconcile when that index is stale.

### AD-3: CLI shell lands in M1 (user-confirmed)

ROADMAP §M1 explicitly says "ships first through the CLI." The shell is
`apps/itala-cli/` with `clap` derive subcommands and a `tokio::main`. Initial
surface: `init`, `notes {create,list,show,move,delete}`, `history
{list,show,restore}`. `apps/*` rejoins the workspace `members`.

### AD-4: Snapshot-on-save history (user-confirmed)

Every successful `save_note` produces a new history entry in `HistoryStore`:
`{ note_id, timestamp, blake3_checksum, bytes }`. Storage of the snapshot is
owned by the adapter (`MemoryHistoryStore` in tests, `FilesHistoryStore` in
`itala-store-files`). `history restore` writes the prior bytes back through
the vault's save path. A diff/merge viewer UI is out of scope (post-M1).

### AD-5: New crate `itala-store-files` per ADR-0004 §3

JSON-on-`Vfs` adapter generalizing what `persistence.rs` already does. Default
backend for every derived-state domain. Implements `DynHistoryStore` for M1;
will grow `DynGraphStore` and `DynSearchMetaStore` implementations when M3
and M4 land. No new dependencies.

### AD-6: HistoryStore port in `itala-kernel` per ADR-0004 §1

Dual-trait pattern (ADR-0003): native-async `HistoryStore` for static
dispatch + `DynHistoryStore` shadow built with `#[async_trait]`, bridged by
`into_dyn_history`. `MemoryHistoryStore` test adapter lives in
`itala-kernel` behind `#[cfg(feature = "test-support")]`, mirroring
`MemoryVfs`.

### AD-7: One vault per process

CLI opens one vault per invocation. Multi-vault state (desktop shell) is
post-M1. The `Vault` type is therefore not internally synchronized — the
shell holds a single owned `Vault`.

### AD-8: Free-form YAML frontmatter

Frontmatter is a `serde_yaml::Value`-equivalent map (we use `serde-saphyr`,
already in workspace deps — see `Cargo.toml`). Typed schemas (e.g.
`tags: Vec<String>`) are layered later as named accessors, not by tightening
the field type. Mirrors how `persistence.rs` treats config as a TOML doc.

---

## Feature breakdown

Per-task detail (acceptance criteria, files, verification, dependencies) lives in [`tasks/`](./tasks/).

Phased so that each phase leaves `just ci` green and the prior behavior
unchanged. Features are sized S/M per `planning-and-task-breakdown`
guidelines.

### Phase 1: Scaffolding

- [x] [F1: Crate skeletons + workspace wiring](./tasks/001-crate-skeletons-workspace-wiring.md)

### Phase 2: Note model (ARCH §5.1)

- [x] [F2: Note data model in `itala-note`](./tasks/002-note-data-model.md)

### Phase 3: Vault context (ARCH §6.1, §6.3)

- [x] [F3: Vault open/init in `itala-vault`](./tasks/003-vault-open-init.md)
- [x] [F4: Note CRUD in `itala-vault`](./tasks/004-note-crud.md)

### Checkpoint A: CLI green for init / create / list

- [x] [F5: CLI `init`, `notes create`, `notes list` commands](./tasks/005-cli-init-create-list.md)

### Phase 4: Note operations

- [x] [F6: `notes show` and `notes delete` commands](./tasks/006-notes-show-delete.md)

### Phase 5: History port + adapter (ADR-0004 §1, §5)

- [x] [F7: `HistoryStore` + `DynHistoryStore` port in `itala-kernel`](./tasks/007-historystore-dynhistorystore-port.md)
- [x] [F8: `MemoryHistoryStore` test adapter](./tasks/008-memory-history-store.md)
- [x] [F9: Snapshot-on-save integration in vault](./tasks/009-snapshot-on-save-integration.md)
- [x] [F10: `history list`, `history show`, `history restore` CLI commands](./tasks/010-history-list-show-restore-cli.md)

### Checkpoint B: CLI green for CRUD + history

### Phase 6: Move / rename + naive link rewriting

- [x] [F11: `[[wiki-link]]` scanner in `itala-vault`](./tasks/011-wiki-link-scanner.md)
- [x] [F12: `notes move` + link rewriting](./tasks/012-notes-move-link-rewriting.md)

### Phase 7: Files adapter (ADR-0004 §3)

- [x] [F13: `FilesHistoryStore` in `itala-store-files`](./tasks/013-fileshistorystore.md)
- [x] [F14: CLI wires `FilesHistoryStore` by default](./tasks/014-cli-wires-files-history-store.md)

### Phase 8: Integration + sign-off

- [x] [F15: M1 integration suite](./tasks/015-m1-integration-suite.md)
- [x] [F16: Documentation update](./tasks/016-documentation-update.md)

## Out of scope for M1

- **CRDT synchronization state** — considered only when sync or collaboration
  requires it; it remains subordinate to canonical Markdown (AD-1, ADR-0006).
- **Graph index, backlinks, unlinked mentions** — deferred to M3 (AD-2
  covers the M1 stopgap).
- **SQLite derived-state adapter** — ADR-0004 §3 lists it as opt-in; M1
  ships only `files`.
- **`[storage.history] backend = "sqlite"` selection** — ADR-0004 §4 wiring
  lands with the SQLite adapter.
- **Multi-vault concurrent sessions** — AD-7.
- **Frontmatter typed schemas** — AD-8.
- **Diff/merge viewer UI** — post-M1.
- **Web/desktop shells** — MVP-β and beyond.

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `[[wiki-link]]` scanner edge cases (display aliases, block refs) | Med | Snapshot tests on a curated corpus (F11); M3 can accelerate candidate discovery but durable rewriting remains in the vault |
| Frontmatter parser divergence from CommonMark/GFM | Med | Use `serde-saphyr` for YAML, `comrak` for body (already in deps); both are mature |
| `HistoryStore` API churn once the SQLite adapter lands (F13→post-M1) | Low | ADR-0004 §1 committed to the dual-trait pattern; object-safety is the constraint, and the M1 API is object-safe by construction |
| Snapshot storage bloat (every save = new file) | Med | M1 accepts this; dedup/gc is a post-M1 concern (BLAKE3 makes dedup trivial) |

## Open questions

_None at this time._ All four critical assumptions (A1–A4) were confirmed
before this plan was written.
