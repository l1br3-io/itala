# F9: Snapshot-on-save integration in vault

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 5: History port + adapter (ADR-0004 §1, §5)

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
