# F8: In-memory VFS (test support)

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 4: VFS Implementations

**Description:**
`MemoryVfs` implementing `DynVfs` — `HashMap<VfsPath, Bytes>` with a
`tokio::sync::mpsc` for watch events. Gated behind
`#[cfg(feature = "test-support")]` in `itala-kernel`. Reference implementation
for port conformance.

**Acceptance criteria:**
- [x] Round-trip: write 100 entries → list → read each → bytes match.
- [x] Watch test: inject a `VfsEvent` → subscriber receives it.
- [x] `assert_send_sync` passes.
- [x] Port-conformance test suite runs against `MemoryVfs`.

**Verification:**
- [x] `cargo nextest run -p itala-kernel --features test-support` passes.

**Dependencies:** F6

**Files likely touched:**
- `crates/itala-kernel/src/test_support/memory_vfs.rs`

**Estimated scope:** S

---
