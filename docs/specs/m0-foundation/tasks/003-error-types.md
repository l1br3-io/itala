# F3: Error types

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 2: Kernel Core Types (ARCH §3.1, §3.5)

**Description:**
`itala_kernel::Error` enum via `thiserror`, `#[non_exhaustive]`. Minimum
variants: `NotFound { id }`, `Io(#[from] std::io::Error)`, `InvalidId(String)`,
`Config(String)`, `CapabilityDenied { plugin, capability }`, `Plugin(String)`.

**Acceptance criteria:**
- [x] `#[non_exhaustive]` on Error enum.
- [x] `std::error::Error` derived via thiserror.
- [x] Doctests showing `?` propagation from `io::Error`.
- [x] Every variant documented.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F2 (uses ID types in variants)

**Files likely touched:**
- `crates/itala-kernel/src/error.rs`

**Estimated scope:** S

---
