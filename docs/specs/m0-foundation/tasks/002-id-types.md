# F2: ID types

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 2: Kernel Core Types (ARCH §3.1, §3.5)

**Description:**
Newtype IDs: `NoteId(Ulid)`, `VaultId(Ulid)`, `LinkId(Ulid)`,
`BlockId(SmolStr)`, `PluginId(SmolStr)`. Each with `Display`, `FromStr`,
`Serialize`, `Deserialize`. `Send + Sync + Unpin` verified via compile-time
assertions.

**Acceptance criteria:**
- [x] Proptest: every ID round-trips through `Display ↔ FromStr` for 1k random values.
- [x] Doctests on every public type.
- [x] `assert_send_sync` / `assert_unpin` compile-time tests pass.
- [x] `just lint` + `just test` green.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F1

**Files likely touched:**
- `crates/itala-kernel/src/id.rs`

**Estimated scope:** S

---
