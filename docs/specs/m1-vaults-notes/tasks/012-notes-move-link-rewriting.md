# F12: `notes move` + link rewriting

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 6: Move / rename + naive link rewriting

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
