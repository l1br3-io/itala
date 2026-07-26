# F15: M1 integration suite

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 8: Integration + sign-off

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
