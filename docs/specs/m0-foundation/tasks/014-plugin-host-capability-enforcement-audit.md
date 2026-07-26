# F14: Plugin host — capability enforcement & audit

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 5: Plugin System (ARCH §4)

**Description:**
Every privileged call made through `PluginContext` goes through the host,
which checks the `CapabilityRegistry`. Denied capabilities return
`Error::CapabilityDenied`. All such capability usage is recorded in an audit
log (structured, via `tracing`). Native M0 plugins are trusted and can bypass
this policy boundary; WASM makes it enforceable for third-party code in M5.

**Acceptance criteria:**
- [x] Plugin with `vault.read` capability → `ctx.read_file()` succeeds.
- [x] Plugin without `vault.read` → `ctx.read_file()` returns `Error::CapabilityDenied`.
- [x] Revoking a capability at runtime → next call by that plugin is denied.
- [x] Audit log entries: plugin_id, capability, action (grant/deny/use), timestamp.
- [x] `tracing` spans follow plugin boundaries (e.g., `plugin.<id>.read_file`).

**Verification:**
- [x] `cargo nextest run -p itala-plugin-host` passes.

**Dependencies:** F4 (CapabilityRegistry), F12, F13

**Files likely touched:**
- `crates/itala-plugin-host/src/enforcement.rs`
- `crates/itala-plugin-host/src/audit.rs`

**Estimated scope:** M

---
