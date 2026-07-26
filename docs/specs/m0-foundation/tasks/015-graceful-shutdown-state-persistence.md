# F15: Graceful shutdown & state persistence

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 6: Durability & Integration

**Description:**
`CancellationToken`-driven shutdown propagates through the kernel: event bus
drains, plugins deactivate, config/capability/plugin-state persisted to VFS.
On restart, kernel reads back persisted state and reconstructs: config values,
capability grants, plugin installation records.

**Acceptance criteria:**
- [x] Grant capability → shutdown → restart → capability still granted.
- [x] Register config schema + set value → shutdown → restart → value preserved.
- [x] Plugin installation record → shutdown → restart → plugin still known (re-loaded).
- [x] Shutdown completes within 5 seconds (no hangs).
- [x] Forced shutdown (cancellation) → no corruption; next start recovers.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo nextest run -p itala-plugin-host` passes.

**Dependencies:** All prior features

**Files likely touched:**
- `crates/itala-kernel/src/shutdown.rs`
- `crates/itala-kernel/src/persistence.rs`

**Estimated scope:** M

---
