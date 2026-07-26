# F13: `FilesHistoryStore` in `itala-store-files`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 7: Files adapter (ADR-0004 §3)

**Description:** `DynHistoryStore` implementation that persists snapshots as
files under `.itala/history/<note_id>/<history_id>.json` (metadata) +
`.itala/history/<note_id>/<history_id>.md` (body bytes — Markdown is UTF-8
and human-inspectable). Generalizes the pattern in `persistence.rs`. Lists
map over `DynVfs::list`.

**Acceptance criteria:**
- [x] Snapshot → list → get → restore round-trip via a `MemoryVfs`.
- [x] Restart simulation: new `FilesHistoryStore` instance pointed at the
      same `DynVfs` sees all prior snapshots.
- [x] Port-conformance test suite shared with `MemoryHistoryStore` (one
      test module, two impls).
- [x] No new external dependencies (`serde_json` + `bytes` only).

**Verification:** `cargo nextest run -p itala-store-files`.

**Dependencies:** F7.

**Files likely touched:**
- `crates/itala-store-files/src/{lib.rs,history.rs}`
- `crates/itala-store-files/tests/conformance.rs`

**Estimated scope:** M

---
