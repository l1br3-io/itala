# F4: Note CRUD in `itala-vault`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 3: Vault context (ARCH §6.1, §6.3)

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
