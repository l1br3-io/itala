# F1: Crate skeletons + workspace wiring

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 1: Scaffolding

**Description:** Create `crates/itala-vault`, `crates/itala-note`,
`crates/itala-store-files`, `apps/itala-cli` skeletons. Each with
`Cargo.toml` (inheriting workspace deps + lints), `lib.rs`/`main.rs` with
`#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`. Workspace `members`
gains `"apps/*"`. New workspace deps entries for `itala-vault`,
`itala-note`, `itala-store-files`. `apps/itala-cli` skeleton has a `clap`
subcommand that prints version and exits.

**Acceptance criteria:**
- [x] `cargo build --workspace` succeeds on a fresh checkout.
- [x] `just lint`, `just test`, `just deny`, `just doc` all pass.
- [x] `cargo run -p itala-cli -- --version` prints a version string.
- [x] Each new crate has `lints.workspace = true`.

**Verification:** `just ci` green.

**Dependencies:** None.

**Files likely touched:**
- `Cargo.toml` (workspace members + deps)
- `crates/itala-vault/{Cargo.toml,src/lib.rs}`
- `crates/itala-note/{Cargo.toml,src/lib.rs}`
- `crates/itala-store-files/{Cargo.toml,src/lib.rs}`
- `apps/itala-cli/{Cargo.toml,src/main.rs}`

**Estimated scope:** M

---
