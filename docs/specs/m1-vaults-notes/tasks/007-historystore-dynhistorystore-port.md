# F7: `HistoryStore` + `DynHistoryStore` port in `itala-kernel`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 5: History port + adapter (ADR-0004 §1, §5)

**Description:** New module `crates/itala-kernel/src/history.rs`. Native-async
`HistoryStore` trait: `snapshot(note_id, bytes, checksum) -> Result<HistoryId>`,
`list(note_id) -> Result<Vec<HistoryEntry>>`, `get(history_id) ->
Result<HistoryEntry>`, `restore(history_id) -> Result<Bytes>`. Object-safe
`DynHistoryStore` shadow via `#[async_trait]`. `into_dyn_history` bridge
mirroring `vfs::into_dyn`. `HistoryEntry { id: HistoryId, note_id: NoteId,
timestamp: OffsetDateTime, checksum: [u8; 32] }`. New `HistoryId(Ulid)`
newtype in `id.rs`.

**Acceptance criteria:**
- [x] Dual-trait pattern matches `Vfs`/`DynVfs` exactly.
- [x] `Box<dyn DynHistoryStore>` compiles (object-safety verified).
- [x] Doctest on `into_dyn_history` mirroring `into_dyn`'s doctest.
- [x] `Send + Sync + Unpin` assertions on `HistoryEntry`/`HistoryId`.

**Verification:** `cargo nextest run -p itala-kernel`; `cargo clippy -p
itala-kernel -- -D warnings`.

**Dependencies:** F1.

**Files likely touched:**
- `crates/itala-kernel/src/{lib.rs,history.rs,id.rs}`
- `crates/itala-kernel/src/test_support/{mod.rs,memory_history.rs}`

**Estimated scope:** M

---
