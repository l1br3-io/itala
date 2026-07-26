# F6: VFS port trait + types

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 3: Kernel Infrastructure (ARCH §3.2–3.4)

**Description:**
`Vfs` async trait (`Send + Sync`): `read`, `write`, `list`, `stat`, `rename`,
`remove`, `watch`. Supporting types: `VfsPath` (newtype over `Utf8PathBuf`),
`DirEntry`, `Metadata`, `VfsEvent` (`Created`, `Changed`, `Removed`,
`Renamed`). `watch` returns `BoxStream<Item = VfsEvent>`. Object-safe
`DynVfs` shadow via `async-trait`.

**Acceptance criteria:**
- [x] `Vfs` trait defined with all 7 methods.
- [x] `DynVfs` object-safe shadow works (`Box<dyn DynVfs>`).
- [x] `VfsPath` rejects `..` traversal (path safety baked into the type).
- [x] Doctests showing trait usage.
- [x] `assert_send_sync` / `assert_unpin` for `DynVfs`.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F2, F3

**Files likely touched:**
- `crates/itala-kernel/src/vfs.rs`
- `crates/itala-kernel/src/vfs_types.rs`

**Estimated scope:** M

---
