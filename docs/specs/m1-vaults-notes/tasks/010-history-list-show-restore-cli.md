# F10: `history list`, `history show`, `history restore` CLI commands

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 5: History port + adapter (ADR-0004 §1, §5)

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
