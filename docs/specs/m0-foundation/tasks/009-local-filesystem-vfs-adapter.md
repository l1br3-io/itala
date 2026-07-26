# F9: Local filesystem VFS adapter

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 4: VFS Implementations

**Description:**
`itala-vfs-fs` crate — concrete `Vfs` implementation against local filesystem.
Built on `tokio::fs` for async I/O. File watching via `notify` /
`notify-debouncer-full`, surfaced as `BoxStream<Item = VfsEvent>`. Path
safety: vault-root-relative paths only; absolute paths and `..` traversal
rejected.

**Acceptance criteria:**
- [x] `LocalFsVfs::new(root)` constructs when `root` exists.
- [x] Path-escape attempt (`read("../../etc/passwd")`) returns `Error` — never reads outside root.
- [x] Round-trip: write 100 files, list, read each, checksums match.
- [x] Watcher: `tokio::fs::write` → receive `VfsEvent::Changed` within 500ms.
- [x] Concurrent reads: 32 simultaneous readers return identical bytes.
- [x] Port-conformance test suite passes.
- [x] No blocking I/O on async runtime threads (`spawn_blocking` discipline).

**Verification:**
- [x] `cargo nextest run -p itala-vfs-fs` passes.
- [x] `cargo clippy -p itala-vfs-fs -- -D warnings` passes.

**Dependencies:** F6

**Files likely touched:**
- `crates/itala-vfs-fs/src/lib.rs`
- `crates/itala-vfs-fs/src/adapter.rs`
- `crates/itala-vfs-fs/tests/conformance.rs`

**Estimated scope:** M

---
