# F11: Plugin API contract crate

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 5: Plugin System (ARCH §4)

**Description:**
`itala-plugin-api` crate — the versioned, semver'd surface plugins implement.
The M0 contract is experimental; compatibility guarantees follow only after
M5 proves the first public extension surfaces (ADR-0005).
Defines the `Plugin` trait (lifecycle entry points: `initialize`, `activate`,
`deactivate`) and the `PluginContext` (host-provided handle for capabilities,
event bus access, config reads). Versions identify compatibility; pre-1.0
breaking changes remain possible and require migration notes.

**Acceptance criteria:**
- [x] `Plugin` trait defined with lifecycle methods.
- [x] `PluginContext` provides capability-gated access to host services.
- [x] Crate-level docs explain the versioning contract.
- [x] `cargo doc` produces clean docs with no broken links.
- [x] A minimal mock plugin compiles against the API.

**Verification:**
- [x] `cargo nextest run -p itala-plugin-api` passes.
- [x] `cargo doc -p itala-plugin-api` passes with no warnings.

**Dependencies:** F2, F4, F5 (events), F6 (VFS types), F10 (manifest)

**Files likely touched:**
- `crates/itala-plugin-api/src/lib.rs`
- `crates/itala-plugin-api/src/plugin.rs`
- `crates/itala-plugin-api/src/context.rs`
- `crates/itala-plugin-api/src/version.rs`

**Estimated scope:** M

---
