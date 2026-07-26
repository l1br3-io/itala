# Implementation Plan: M1 — Vaults & Notes

> **Status:** Complete — all 16 features implemented; `just ci` green
> (lint + test + deny + doc); 145+ tests across `itala-note`, `itala-vault`,
> `itala-store-files`, and `apps/itala-cli`, plus a kernel-history test suite.
> **Milestone:** [M1 — Vaults & Notes](../../../docs/ROADMAP.md)
> **Architecture:** [ARCHITECTURE.md §2 (crate map), §5 (data model), §6 (flows)](../../../docs/ARCHITECTURE.md)
> **ADR-0004:** [Per-domain ports for derived-state storage](../../../docs/adr/0004-derived-state-storage-strategy.md)

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
[ADR-0006](../../../docs/adr/0006-markdown-canonical-source.md) makes that durable format
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

Phased so that each phase leaves `just ci` green and the prior behavior
unchanged. Features are sized S/M per `planning-and-task-breakdown`
guidelines.

### Phase 1: Scaffolding

#### F1: Crate skeletons + workspace wiring

**Description:** Create `crates/itala-vault`, `crates/itala-note`,
`crates/itala-store-files`, `apps/itala-cli` skeletons. Each with
`Cargo.toml` (inheriting workspace deps + lints), `lib.rs`/`main.rs` with
`#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`. Workspace `members`
gains `"apps/*"`. New workspace deps entries for `itala-vault`,
`itala-note`, `itala-store-files`. `apps/itala-cli` skeleton has a `clap`
subcommand that prints version and exits.

**Acceptance criteria:**
- [x] `cargo build --workspace` succeeds on a fresh checkout.
- [x] `just lint`, `just test`, `just deny`, `just doc` all pass.
- [x] `cargo run -p itala-cli -- --version` prints a version string.
- [x] Each new crate has `lints.workspace = true`.

**Verification:** `just ci` green.

**Dependencies:** None.

**Files likely touched:**
- `Cargo.toml` (workspace members + deps)
- `crates/itala-vault/{Cargo.toml,src/lib.rs}`
- `crates/itala-note/{Cargo.toml,src/lib.rs}`
- `crates/itala-store-files/{Cargo.toml,src/lib.rs}`
- `apps/itala-cli/{Cargo.toml,src/main.rs}`

**Estimated scope:** M

---

### Phase 2: Note model (ARCH §5.1)

#### F2: Note data model in `itala-note`

**Description:** `Note` struct: `id: NoteId`, `path: VfsPath`,
`frontmatter: Frontmatter`, `body: Bytes`, `created_at: OffsetDateTime`,
`updated_at: OffsetDateTime`, `checksum: [u8; 32]` (BLAKE3). `Frontmatter`
is a typed wrapper around a `serde-saphyr`-parsed YAML map. Methods:
`Note::new(...)` constructor that computes BLAKE3, `Note::from_markdown(bytes)`
parses frontmatter-delimited Markdown into the struct,
`Note::to_markdown()` projects back deterministically.

**Acceptance criteria:**
- [x] Round-trip: `from_markdown → to_markdown` is identity for 1k
      proptest-generated Markdown snippets.
- [x] BLAKE3 checksum recomputed on every persist (unit test).
- [x] Frontmatter absent → empty `Frontmatter`, body is the whole input.
- [x] Snapshot test (insta) on a representative Markdown+frontmatter doc.

**Verification:** `cargo nextest run -p itala-note`; `cargo clippy -p
itala-note -- -D warnings`.

**Dependencies:** F1.

**Files likely touched:**
- `crates/itala-note/src/{lib.rs,note.rs,frontmatter.rs}`

**Estimated scope:** M

---

### Phase 3: Vault context (ARCH §6.1, §6.3)

#### F3: Vault open/init in `itala-vault`

**Description:** `Vault::init(root: &Utf8Path) -> Result<Vault>` creates
`.itala/`, `.itala/config.toml` (minimal default), `.itala/history/`,
`Notes/`, `Assets/`. `Vault::open(root: &Utf8Path) -> Result<Vault>` opens
an existing vault, fails with `Error::Config` if `.itala/` is missing.
`Vault` holds `root: Utf8PathBuf`, `id: VaultId` (generated on init, loaded
from `.itala/vault.id` on open), and a `Box<dyn DynVfs>` rooted at `root`.

**Acceptance criteria:**
- [x] Init then open round-trips in a `tempfile::TempDir`.
- [x] Open on a non-vault directory returns `Error::Config`.
- [x] On open, `VaultOpened { id }` is published on the bus.
- [x] `VaultId` persists across reopen (read from `.itala/vault.id`).

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F1, kernel's existing `LocalFsVfs`.

**Files likely touched:**
- `crates/itala-vault/src/{lib.rs,vault.rs}`

**Estimated scope:** M

---

#### F4: Note CRUD in `itala-vault`

**Description:** `Vault::create_note(title) -> Result<Note>`: slugifies title,
writes `Notes/<slug>-<ulid-prefix>.md` via `DynVfs::write`, returns `Note`.
`Vault::list_notes() -> Result<Vec<NoteMetadata>>` walks `Notes/` recursively
via `DynVfs::list`. `Vault::read_note(path) -> Result<Note>` reads bytes,
calls `Note::from_markdown`. `Vault::delete_note(path) -> Result<()>` calls
`DynVfs::remove`. Every mutation publishes the matching `KernelEvent`
(`NoteSaved`, `NoteDeleted`).

**Acceptance criteria:**
- [x] Create then list returns the new note.
- [x] Create then read round-trips the frontmatter + body.
- [x] Create then delete then list returns empty.
- [x] `NoteSaved` and `NoteDeleted` fire on the bus (subscriber test).
- [x] Slug collision is resolved by appending the ULID prefix (no overwrite).

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F2, F3.

**Files likely touched:**
- `crates/itala-vault/src/{lib.rs,notes.rs,slug.rs}`

**Estimated scope:** M

---

### Checkpoint A: CLI green for init / create / list

#### F5: CLI `init`, `notes create`, `notes list` commands

**Description:** `apps/itala-cli` gains three subcommands. `init` takes an
optional path (default `.`) and calls `Vault::init`. `notes create` takes a
`--title` and optional `--vault` (default `.`) and calls
`Vault::create_note`. `notes list` takes `--vault` and prints one note per
line (path + title-from-frontmatter). Uses `anyhow` at the boundary (ARCH
§7.1); typed errors propagate up via `?`.

**Acceptance criteria:**
- [x] `cargo run -p itala-cli -- init /tmp/x && cargo run -p itala-cli --
        notes create --vault /tmp/x --title "Hello"` writes a file under
        `/tmp/x/Notes/`.
- [x] `assert_cmd`-based integration test: init → create → list returns the
        created note.
- [x] CLI errors use `anyhow` context, not `panic!`/`unwrap`.
- [x] Help text (`--help`) renders cleanly for all three subcommands.

**Verification:** `cargo nextest run -p itala-cli`; `assert_cmd` end-to-end.

**Dependencies:** F4.

**Files likely touched:**
- `apps/itala-cli/src/main.rs`
- `apps/itala-cli/tests/cli.rs`

**Estimated scope:** M

---

### Phase 4: Note operations

#### F6: `notes show` and `notes delete` commands

**Description:** Extend CLI. `notes show <path-or-id>` reads and prints the
note (frontmatter + body). `notes delete <path>` deletes after `--yes`
confirmation flag (or non-interactive error without it).

**Acceptance criteria:**
- [x] `notes show` on an existing note prints Markdown to stdout.
- [x] `notes show` on a missing note exits non-zero with a typed error.
- [x] `notes delete` requires `--yes` in non-interactive mode.
- [x] `assert_cmd` tests cover both commands.

**Verification:** `cargo nextest run -p itala-cli`.

**Dependencies:** F5.

**Files likely touched:**
- `apps/itala-cli/src/main.rs`
- `apps/itala-cli/tests/cli.rs`

**Estimated scope:** S

---

### Phase 5: History port + adapter (ADR-0004 §1, §5)

#### F7: `HistoryStore` + `DynHistoryStore` port in `itala-kernel`

**Description:** New module `crates/itala-kernel/src/history.rs`. Native-async
`HistoryStore` trait: `snapshot(note_id, bytes, checksum) -> Result<HistoryId>`,
`list(note_id) -> Result<Vec<HistoryEntry>>`, `get(history_id) ->
Result<HistoryEntry>`, `restore(history_id) -> Result<Bytes>`. Object-safe
`DynHistoryStore` shadow via `#[async_trait]`. `into_dyn_history` bridge
mirroring `vfs::into_dyn`. `HistoryEntry { id: HistoryId, note_id: NoteId,
timestamp: OffsetDateTime, checksum: [u8; 32] }`. New `HistoryId(Ulid)`
newtype in `id.rs`.

**Acceptance criteria:**
- [x] Dual-trait pattern matches `Vfs`/`DynVfs` exactly.
- [x] `Box<dyn DynHistoryStore>` compiles (object-safety verified).
- [x] Doctest on `into_dyn_history` mirroring `into_dyn`'s doctest.
- [x] `Send + Sync + Unpin` assertions on `HistoryEntry`/`HistoryId`.

**Verification:** `cargo nextest run -p itala-kernel`; `cargo clippy -p
itala-kernel -- -D warnings`.

**Dependencies:** F1.

**Files likely touched:**
- `crates/itala-kernel/src/{lib.rs,history.rs,id.rs}`
- `crates/itala-kernel/src/test_support/{mod.rs,memory_history.rs}`

**Estimated scope:** M

---

#### F8: `MemoryHistoryStore` test adapter

**Description:** In-memory `HistoryStore` behind
`#[cfg(feature = "test-support")]`. `HashMap<HistoryId, HistoryEntry>` +
`HashMap<HistoryId, Bytes>`. Use this for unit tests throughout M1; the
real adapter (F12) is added in Phase 7.

**Acceptance criteria:**
- [x] Snapshot → list → get → restore round-trip in tests.
- [x] Multiple notes' histories don't bleed into each other.
- [x] `list(note_id)` returns entries newest-first.

**Verification:** `cargo nextest run -p itala-kernel --features test-support`.

**Dependencies:** F7.

**Files likely touched:**
- `crates/itala-kernel/src/test_support/memory_history.rs`

**Estimated scope:** S

---

#### F9: Snapshot-on-save integration in vault

**Description:** `Vault` constructor accepts an optional `Arc<dyn
DynHistoryStore>`. `Vault::save_note` writes the note, then calls
`history.snapshot(...)` if a store is configured. `NoteSaved` publishes
after the snapshot succeeds. `Vault::history(note_id)` and
`Vault::restore_history(history_id)` expose the store to the shell.

**Acceptance criteria:**
- [x] Saving a note three times produces three history entries.
- [x] `restore_history` writes the prior bytes back through `save_note`
      (which itself snapshots the restored content — test that this is
      intentional and documented).
- [x] No history store configured → save path is unchanged.

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F4, F8.

**Files likely touched:**
- `crates/itala-vault/src/vault.rs`

**Estimated scope:** M

---

#### F10: `history list`, `history show`, `history restore` CLI commands

**Description:** Three new subcommands. `list <note-path>` prints history
entries (timestamp + checksum prefix). `show <history-id>` prints the bytes
of a specific entry. `restore <history-id>` writes those bytes back as a
new save.

**Acceptance criteria:**
- [x] All three commands tested via `assert_cmd`.
- [x] Restore round-trip: save v1, save v2, restore v1, read note ==
      v1 content (+ new v3 history entry).
- [x] History of an unknown note → typed error, non-zero exit.

**Verification:** `cargo nextest run -p itala-cli`.

**Dependencies:** F9.

**Files likely touched:**
- `apps/itala-cli/src/main.rs`
- `apps/itala-cli/tests/cli.rs`

**Estimated scope:** M

---

### Checkpoint B: CLI green for CRUD + history

---

### Phase 6: Move / rename + naive link rewriting

#### F11: `[[wiki-link]]` scanner in `itala-vault`

**Description:** Char-scanning parser (no regex dependency) that walks
Markdown bytes and yields `WikiLink { target: SmolStr, display: Option<SmolStr>,
byte_range: Range<usize> }`. Targets match `[[Name]]` and `[[Name|Display]]`
and `[[Name#^block-id]]` (block refs are part of the target on the rename
side — the name is the prefix). Excludes links inside fenced code blocks.

**Acceptance criteria:**
- [x] Snapshot tests (insta) on a corpus of ~10 Markdown snippets covering
      plain links, display-aliased links, block refs, code-fenced false
      positives.
- [x] Proptest: random ASCII bytes never panic the scanner.
- [x] Byte ranges round-trip: `bytes[range]` is the original link text.

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F4.

**Files likely touched:**
- `crates/itala-vault/src/{wiki_link.rs,slug.rs}`

**Estimated scope:** M

---

#### F12: `notes move` + link rewriting

**Description:** `Vault::move_note(from_path, to_path) -> Result<MoveReport>`
renames via `DynVfs::rename`, then scans every other note for `[[from-slug]]`
and rewrites to `[[to-slug]]` in-place via `DynVfs::write`. `MoveReport {
moved: VfsPath, rewritten: Vec<VfsPath> }`. Publishes `NoteRenamed {
from, to }`. CLI: `notes move <from> <to>`.

**Acceptance criteria:**
- [x] Move a note with one inbound link → referring note's content updated.
- [x] Move a note with no inbound links → `rewritten` is empty.
- [x] `NoteRenamed` fires on the bus (subscriber test).
- [x] Move into a non-existent directory → directory created; move succeeds.
- [x] Code-fenced `[[not-a-link]]` is preserved verbatim.

**Verification:** `cargo nextest run -p itala-vault`; `cargo nextest run -p
itala-cli`.

**Dependencies:** F9, F11.

**Files likely touched:**
- `crates/itala-vault/src/{vault.rs,wiki_link.rs}`
- `apps/itala-cli/src/main.rs`
- `apps/itala-cli/tests/cli.rs`

**Estimated scope:** M

---

### Phase 7: Files adapter (ADR-0004 §3)

#### F13: `FilesHistoryStore` in `itala-store-files`

**Description:** `DynHistoryStore` implementation that persists snapshots as
files under `.itala/history/<note_id>/<history_id>.json` (metadata) +
`.itala/history/<note_id>/<history_id>.md` (body bytes — Markdown is UTF-8
and human-inspectable). Generalizes the pattern in `persistence.rs`. Lists
map over `DynVfs::list`.

**Acceptance criteria:**
- [x] Snapshot → list → get → restore round-trip via a `MemoryVfs`.
- [x] Restart simulation: new `FilesHistoryStore` instance pointed at the
      same `DynVfs` sees all prior snapshots.
- [x] Port-conformance test suite shared with `MemoryHistoryStore` (one
      test module, two impls).
- [x] No new external dependencies (`serde_json` + `bytes` only).

**Verification:** `cargo nextest run -p itala-store-files`.

**Dependencies:** F7.

**Files likely touched:**
- `crates/itala-store-files/src/{lib.rs,history.rs}`
- `crates/itala-store-files/tests/conformance.rs`

**Estimated scope:** M

---

#### F14: CLI wires `FilesHistoryStore` by default

**Description:** CLI constructs a `LocalFsVfs` for the vault root, wraps it
in `FilesHistoryStore`, passes both to `Vault::new`. Memory adapter still
used in unit tests. Config-section selection (`[storage.history] backend =
"files" | "sqlite"`) is deferred to ADR-0004 §4 — M1 ships `"files"` only.

**Acceptance criteria:**
- [x] End-to-end CLI test: create note, save twice, list history, restore
      prior version, all via the files adapter.
- [x] `.itala/history/<note_id>/` directory structure matches F13.
- [x] `cargo run -p itala-cli -- notes create ... && ls .itala/history/`
      shows the snapshot files.

**Verification:** `cargo nextest run -p itala-cli`.

**Dependencies:** F10, F13.

**Files likely touched:**
- `apps/itala-cli/src/main.rs`

**Estimated scope:** S

---

### Phase 8: Integration + sign-off

#### F15: M1 integration suite

**Description:** One end-to-end test in `apps/itala-cli/tests/m1_integration.rs`:
init tempdir → create three notes (two linking to the third) → list → move
the third (verify inbound links updated) → show one → delete one → save
twice → history list → history restore prior → verify content. Shuts down
gracefully, reopens the vault, verifies state persisted.

**Acceptance criteria:**
- [x] Test passes deterministically on 10 consecutive runs.
- [x] Test exercises every public Vault/CLI surface added in M1.
- [x] `just ci` green.

**Verification:** `just ci`.

**Dependencies:** F14.

**Files likely touched:**
- `apps/itala-cli/tests/m1_integration.rs`

**Estimated scope:** M

---

#### F16: Documentation update

**Description:** Update `docs/ARCHITECTURE.md` to mark `itala-vault`,
`itala-note`, `itala-vfs-fs`, `apps/itala-cli` as *(shipped)* rather than
*(target)* where applicable. Update `Cargo.toml` comment about `apps/*`.
Mark M1 in `docs/ROADMAP.md` as ✅.

**Acceptance criteria:**
- [x] Architecture crate map reflects M1 reality.
- [x] Roadmap status reflects M1 completion.
- [x] `just doc` green.

**Verification:** `just doc`.

**Dependencies:** F15.

**Files likely touched:**
- `docs/ARCHITECTURE.md`
- `docs/ROADMAP.md`

**Estimated scope:** S

---

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
