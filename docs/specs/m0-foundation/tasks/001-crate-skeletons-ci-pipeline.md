# F1: Crate skeletons + CI pipeline

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 1: Scaffolding

**Description:**
Create `itala-kernel`, `itala-plugin-api`, `itala-plugin-host`,
`itala-vfs-fs` crate skeletons. Each with `Cargo.toml` (inheriting workspace
deps + lints), `lib.rs` with `#![forbid(unsafe_code)]` and
`#![warn(missing_docs)]`. Add GitHub Actions CI running `just ci`.

**Acceptance criteria:**
- [x] `cargo build --workspace` succeeds on a fresh checkout.
- [x] `just lint` / `just test` / `just deny` / `just doc` all pass (trivially).
- [x] CI pipeline runs all four gates and blocks on failure.
- [x] Each crate has `lints.workspace = true`.

**Verification:**
- [x] `just ci` green.
- [x] Fresh clone → `just ci` green without any prerequisites beyond Rust + cargo-deny.

**Dependencies:** None

**Files likely touched:**
- `crates/itala-kernel/Cargo.toml`
- `crates/itala-kernel/src/lib.rs`
- `crates/itala-plugin-api/Cargo.toml`
- `crates/itala-plugin-api/src/lib.rs`
- `crates/itala-plugin-host/Cargo.toml`
- `crates/itala-plugin-host/src/lib.rs`
- `crates/itala-vfs-fs/Cargo.toml`
- `crates/itala-vfs-fs/src/lib.rs`
- `.github/workflows/ci.yml`

**Estimated scope:** S

---
