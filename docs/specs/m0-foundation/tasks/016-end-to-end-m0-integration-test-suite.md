# F16: End-to-end M0 integration test suite

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 6: Durability & Integration

**Description:**
Comprehensive integration tests that exercise the full M0 contract: construct
kernel → open VFS → register config → load plugins → enforce capabilities →
lifecycle panic containment → event flow → graceful shutdown → restart → state
preserved.

**Acceptance criteria:**
- [x] Test: full lifecycle — construct, run, shutdown, restart, verify state.
- [x] Test: 3 plugins, one panics, others continue, host survives.
- [x] Test: capability grant/deny/revoke cycle.
- [x] Test: event bus — publish + subscribe across plugin boundaries.
- [x] Test: VFS read/write/watch round-trip on local FS.
- [x] Test: config layered override + persistence.
- [x] All tests pass deterministically (run 10×, no flakiness).

**Verification:**
- [x] `cargo nextest run --workspace` passes 10 consecutive runs.
- [x] `just ci` green.

**Dependencies:** F1–F15

**Files likely touched:**
- `crates/itala-kernel/tests/m0_integration.rs`
- `crates/itala-plugin-host/tests/m0_integration.rs`

**Estimated scope:** M

---
