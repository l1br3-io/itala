# F6: `notes show` and `notes delete` commands

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 4: Note operations

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
