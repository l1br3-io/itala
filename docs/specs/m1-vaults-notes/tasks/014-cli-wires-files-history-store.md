# F14: CLI wires `FilesHistoryStore` by default

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 7: Files adapter (ADR-0004 §3)

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
