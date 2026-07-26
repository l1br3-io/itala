# F4: Capability model + registry

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 2: Kernel Core Types (ARCH §3.1, §3.5)

**Description:**
`Capability` newtype (opaque `SmolStr` name, serializes to stable string form).
`CapabilityRegistry` — tracks granted/revoked capabilities per plugin,
queryable at runtime. Capabilities are named grants like `vault.read`,
`network.outbound`, `search.index`.

**Acceptance criteria:**
- [x] `Capability` serializes to stable string form (semver-frozen surface).
- [x] `CapabilityRegistry::grant(plugin_id, capability)` / `revoke` / `has(plugin_id, capability)` work.
- [x] Proptest: grant → query → revoke → query returns correct results.
- [x] `Send + Sync` for `CapabilityRegistry`.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F2, F3

**Files likely touched:**
- `crates/itala-kernel/src/capability.rs`
- `crates/itala-kernel/src/capability/registry.rs`

**Estimated scope:** M

---
