# F8: `MemoryHistoryStore` test adapter

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 5: History port + adapter (ADR-0004 §1, §5)

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
