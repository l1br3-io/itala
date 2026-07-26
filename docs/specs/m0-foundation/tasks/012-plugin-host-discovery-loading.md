# F12: Plugin host — discovery & loading

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 5: Plugin System (ARCH §4)

**Description:**
`itala-plugin-host` crate. Scan a directory for installed plugins (each has a
`plugin.toml`). Verify manifest (API range compatibility, required fields).
Load plugin (M0: Rust-native — plugins are registered via a factory function).
Maintain a registry of loaded plugins. Crash isolation: a panicking plugin is
caught (via `std::panic::catch_unwind` or `tokio::task` isolation) and marked
as failed without crashing the host.

**Acceptance criteria:**
- [x] Scan directory with 3 mock plugins → all 3 discovered and loaded.
- [x] Plugin with incompatible API range → rejected with typed error, others still load.
- [x] Plugin that panics during load → caught, marked failed, host continues.
- [x] `PluginHost::list()` returns loaded plugins with status (Active, Failed, Inactive).
- [x] Events emitted: `PluginLoaded { id }` for each successful load.

**Verification:**
- [x] `cargo nextest run -p itala-plugin-host` passes.
- [x] `cargo clippy -p itala-plugin-host -- -D warnings` passes.

**Dependencies:** F10, F11

**Files likely touched:**
- `crates/itala-plugin-host/src/host.rs`
- `crates/itala-plugin-host/src/registry.rs`
- `crates/itala-plugin-host/src/loader.rs`

**Estimated scope:** M-L

---
