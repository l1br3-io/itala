# F13: Plugin host — lifecycle management

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 5: Plugin System (ARCH §4)

**Description:**
Drive the plugin lifecycle: `initialize → activate → (config-change) →
deactivate`. Host calls lifecycle methods on each plugin at the right time.
Config changes trigger re-initialization. Graceful deactivation on shutdown.

**Acceptance criteria:**
- [x] Load plugin → `initialize` called → `activate` called → plugin is Active.
- [x] Config change → plugin receives notification → can re-read config.
- [x] Shutdown signal → `deactivate` called on all active plugins → clean exit.
- [x] Plugin failing during `activate` → marked Failed, others continue.
- [x] Integration test: full lifecycle on 3 plugins, verified via event sequence.

**Verification:**
- [x] `cargo nextest run -p itala-plugin-host` passes.

**Dependencies:** F12, F7 (config changes), F5 (events)

**Files likely touched:**
- `crates/itala-plugin-host/src/lifecycle.rs`
- `crates/itala-plugin-host/src/host.rs` (modified)

**Estimated scope:** M

---
