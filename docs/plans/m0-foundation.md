# Implementation Plan: M0 — Foundation

> **Status:** Complete — all 16 features implemented; `just ci` green (lint + test + deny + doc); 73 tests + 12 doctests passing deterministically.
> **Milestone:** [M0 — Foundation](../ROADMAP.md#m0--foundation)
> **Architecture:** [ARCHITECTURE.md §3 (Kernel) + §4 (Plugin System)](../ARCHITECTURE.md)
> **PRD:** [0001-functional-contracts.md](../prds/0001-functional-contracts.md)

## Overview

Stand up the `itala-kernel` crate (shared types, event bus, VFS port, config
registry, capability system), the `itala-plugin-api` crate (versioned plugin
contract), the `itala-plugin-host` crate (discovery, loading, lifecycle, crash
isolation, capability enforcement), and the `itala-vfs-fs` crate (local
filesystem VFS adapter). No user-facing surface — everything is verified
through tests.

**Exit condition:** an integration test suite that constructs the kernel,
opens a VFS, loads plugins, enforces capabilities, survives a plugin crash,
shuts down gracefully, restarts, and verifies state was preserved.

## Crates created in M0

```text
crates/
  itala-kernel/        # shared types, event bus, VFS port, config, capabilities
  itala-plugin-api/    # versioned contract plugins compile against
  itala-plugin-host/   # discovery, loading, lifecycle, enforcement, crash isolation
  itala-vfs-fs/        # local filesystem VFS adapter + file watching
```

No `apps/` shells. No `itala-vault`, `itala-note`, `itala-graph` — those are M1+.

---

## Architecture decisions

### AD-1: Plugin runtime — Rust-native first, WASM deferred to M5

The architecture doc targets WASM (wasmtime) for third-party plugins. But
`wasmtime` is a massive dependency and implementing the WIT/ABI boundary is a
project unto itself. For M0:

- **M0 ships Rust-native plugins** — plugins implement traits from
  `itala-plugin-api`, registered at startup (compiled into the binary or loaded
  as `cdylib`). The capability system still sandboxes: a plugin can only do
  what its granted capabilities allow. Every privileged call goes through the
  host.
- **WASM sandboxing deferred to M5 (Extend)** — when the plugin ecosystem
  milestone arrives, we add the WASM runtime and WIT contract. The plugin API
  trait is designed to be WASM-compatible from the start (no host-only types
  in the contract, explicit `Send` bounds).

**Trade-off:** M0 plugins aren't memory-safe sandboxed (they're native Rust),
but capability enforcement still limits what they can do.

### AD-2: In-memory VFS lives in `itala-kernel` behind `test-support` feature

For fast unit tests throughout the kernel and plugin host, a simple
`HashMap<VfsPath, Bytes>` VFS implementation lives in `itala-kernel` behind a
`test-support` feature flag. Avoids a separate crate and keeps it available to
all downstream crates' tests.

### AD-3: VFS object safety via `async-trait`

The `Vfs` trait uses `async-trait` to produce an object-safe `DynVfs` shadow.
This is the path illustrated in ARCHITECTURE.md §3.3. Native `async fn in
trait` stays available for monomorphized call sites; the dyn shadow is the
runtime-dispatch path used by plugin host and contexts.

---

## Feature breakdown

### Phase 1: Scaffolding

---

#### F1: Crate skeletons + CI pipeline

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

### Phase 2: Kernel Core Types (ARCH §3.1, §3.5)

---

#### F2: ID types

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

#### F3: Error types

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

#### F4: Capability model + registry

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

### Phase 3: Kernel Infrastructure (ARCH §3.2–3.4)

---

#### F5: Event bus

**Description:**
Typed pub/sub on `tokio::sync::broadcast`. `KernelEvent` enum
(`#[non_exhaustive]`, `Clone`, `Debug`): `VaultOpened`, `NoteSaved { id }`,
`NoteChanged { id }`, `NoteRenamed { from, to }`, `NoteDeleted { id }`,
`LinksUpdated { id }`, `PluginLoaded { id }`, `IndexRebuildRequested`.
Bounded capacity ≥ 1024. `CancellationToken`-driven shutdown. At-least-once
delivery (handlers must be idempotent).

**Acceptance criteria:**
- [x] Integration test: two subscribers, publish 1k events, both receive all.
- [x] Cancellation test: `tokio::select!` between `token.cancelled()` and `bus.recv()` exits cleanly.
- [x] `KernelEvent` is `#[non_exhaustive]`, `Clone`, `Debug`, `Send + Sync`.
- [x] Bounded channels only — no `unbounded_channel` anywhere.
- [x] No `std::sync::Mutex` held across `.await` (enforced by `await_holding_lock = deny`).

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F2, F3

**Files likely touched:**
- `crates/itala-kernel/src/events.rs`
- `crates/itala-kernel/src/bus.rs`

**Estimated scope:** M

---

#### F6: VFS port trait + types

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

#### F7: Configuration registry

**Description:**
Typed, layered config store: defaults → directory config (`config.toml`) →
runtime overrides. Plugins and contexts register typed config schemas.
Registry validates and provides change notifications via the event bus.
Persists to TOML.

**Acceptance criteria:**
- [x] Register a typed schema → load from TOML → query returns typed value.
- [x] Override at runtime → query reflects override → persists on save.
- [x] Invalid TOML returns typed `Error::Config`.
- [x] Change notification fires when config is updated.
- [x] Round-trip: save → drop registry → reload → values match.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo clippy -p itala-kernel -- -D warnings` passes.

**Dependencies:** F3, F5 (uses event bus for notifications)

**Files likely touched:**
- `crates/itala-kernel/src/config.rs`
- `crates/itala-kernel/src/config/schema.rs`

**Estimated scope:** M

---

### Phase 4: VFS Implementations

---

#### F8: In-memory VFS (test support)

**Description:**
`MemoryVfs` implementing `DynVfs` — `HashMap<VfsPath, Bytes>` with a
`tokio::sync::mpsc` for watch events. Gated behind
`#[cfg(feature = "test-support")]` in `itala-kernel`. Reference implementation
for port conformance.

**Acceptance criteria:**
- [x] Round-trip: write 100 entries → list → read each → bytes match.
- [x] Watch test: inject a `VfsEvent` → subscriber receives it.
- [x] `assert_send_sync` passes.
- [x] Port-conformance test suite runs against `MemoryVfs`.

**Verification:**
- [x] `cargo nextest run -p itala-kernel --features test-support` passes.

**Dependencies:** F6

**Files likely touched:**
- `crates/itala-kernel/src/test_support/memory_vfs.rs`

**Estimated scope:** S

---

#### F9: Local filesystem VFS adapter

**Description:**
`itala-vfs-fs` crate — concrete `Vfs` implementation against local filesystem.
Built on `tokio::fs` for async I/O. File watching via `notify` /
`notify-debouncer-full`, surfaced as `BoxStream<Item = VfsEvent>`. Path
safety: vault-root-relative paths only; absolute paths and `..` traversal
rejected.

**Acceptance criteria:**
- [x] `LocalFsVfs::new(root)` constructs when `root` exists.
- [x] Path-escape attempt (`read("../../etc/passwd")`) returns `Error` — never reads outside root.
- [x] Round-trip: write 100 files, list, read each, checksums match.
- [x] Watcher: `tokio::fs::write` → receive `VfsEvent::Changed` within 500ms.
- [x] Concurrent reads: 32 simultaneous readers return identical bytes.
- [x] Port-conformance test suite passes.
- [x] No blocking I/O on async runtime threads (`spawn_blocking` discipline).

**Verification:**
- [x] `cargo nextest run -p itala-vfs-fs` passes.
- [x] `cargo clippy -p itala-vfs-fs -- -D warnings` passes.

**Dependencies:** F6

**Files likely touched:**
- `crates/itala-vfs-fs/src/lib.rs`
- `crates/itala-vfs-fs/src/adapter.rs`
- `crates/itala-vfs-fs/tests/conformance.rs`

**Estimated scope:** M

---

### Checkpoint A: Kernel + VFS proven

```text
F1 ──► F2 ──► F3 ──► F4
                │
                ├──► F5 (event bus)
                ├──► F6 ──► F8 (memory VFS)
                │         └──► F9 (local FS VFS)
                └──► F7 (config)
```

**Gate:**
- [x] All Phase 1–4 features complete.
- [x] `just ci` green.
- [x] Integration test: construct kernel → open `LocalFsVfs` on `tempfile::TempDir` → read/write files → subscribe to events → receive `VfsEvent` on external file change.
- [x] **Review with human before proceeding to plugin system.**

---

### Phase 5: Plugin System (ARCH §4)

---

#### F10: Plugin manifest model

**Description:**
`PluginManifest` struct: `id: PluginId`, `version: semver::Version`,
`api_range: semver::VersionReq`, `capabilities: Vec<Capability>`,
`plugin_type: PluginType` (Editor, Theme, Storage, Sync, Search, AiProvider,
Exporter, Command, Renderer). Parsed from TOML (`plugin.toml`).

**Acceptance criteria:**
- [x] Parse a representative `plugin.toml` → correct `PluginManifest`.
- [x] Missing required field → typed error with field name.
- [x] API range compatibility check: `api_range.matches(host_api_version)`.
- [x] Insta snapshot tests for manifest parsing.
- [x] `Send + Sync` for `PluginManifest`.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes (manifest model lives in kernel).

**Dependencies:** F2 (PluginId), F4 (Capability)

**Files likely touched:**
- `crates/itala-kernel/src/manifest.rs`
- `crates/itala-kernel/src/manifest/plugin_type.rs`

**Estimated scope:** S-M

---

#### F11: Plugin API contract crate

**Description:**
`itala-plugin-api` crate — the versioned, semver'd surface plugins implement.
Defines the `Plugin` trait (lifecycle entry points: `initialize`, `activate`,
`deactivate`) and the `PluginContext` (host-provided handle for capabilities,
event bus access, config reads). Stable across minor versions; breaking
changes bump major.

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

#### F12: Plugin host — discovery & loading

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

#### F13: Plugin host — lifecycle management

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

#### F14: Plugin host — capability enforcement & audit

**Description:**
Every privileged call from a plugin goes through the host, which checks the
`CapabilityRegistry`. Denied capabilities return `Error::CapabilityDenied`.
All capability usage is recorded in an audit log (structured, via `tracing`).

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

### Checkpoint B: Plugin host working

```text
F10 ──► F11 ──► F12 ──► F13 ──► F14
                   │
                   └── crash isolation verified
```

**Gate:**
- [x] All Phase 5 features complete.
- [x] `just ci` green.
- [x] Integration test: load 3 plugins → one panics → host survives → enforce capabilities on the other two → lifecycle events in correct order.
- [x] **Review with human before proceeding to durability.**

---

### Phase 6: Durability & Integration

---

#### F15: Graceful shutdown & state persistence

**Description:**
`CancellationToken`-driven shutdown propagates through the kernel: event bus
drains, plugins deactivate, config/capability/plugin-state persisted to VFS.
On restart, kernel reads back persisted state and reconstructs: config values,
capability grants, plugin installation records.

**Acceptance criteria:**
- [x] Grant capability → shutdown → restart → capability still granted.
- [x] Register config schema + set value → shutdown → restart → value preserved.
- [x] Plugin installation record → shutdown → restart → plugin still known (re-loaded).
- [x] Shutdown completes within 5 seconds (no hangs).
- [x] Forced shutdown (cancellation) → no corruption; next start recovers.

**Verification:**
- [x] `cargo nextest run -p itala-kernel` passes.
- [x] `cargo nextest run -p itala-plugin-host` passes.

**Dependencies:** All prior features

**Files likely touched:**
- `crates/itala-kernel/src/shutdown.rs`
- `crates/itala-kernel/src/persistence.rs`

**Estimated scope:** M

---

#### F16: End-to-end M0 integration test suite

**Description:**
Comprehensive integration tests that exercise the full M0 contract: construct
kernel → open VFS → register config → load plugins → enforce capabilities →
crash isolation → event flow → graceful shutdown → restart → state preserved.

**Acceptance criteria:**
- [x] Test: full lifecycle — construct, run, shutdown, restart, verify state.
- [x] Test: 3 plugins, one panics, others continue, host survives.
- [x] Test: capability grant/deny/revoke cycle.
- [x] Test: event bus — publish + subscribe across plugin boundaries.
- [x] Test: VFS read/write/watch round-trip on local FS.
- [x] Test: config layered override + persistence.
- [x] All tests pass deterministically (run 10×, no flakiness).

**Verification:**
- [x] `cargo nextest run --workspace` passes 10 consecutive runs.
- [x] `just ci` green.

**Dependencies:** F1–F15

**Files likely touched:**
- `crates/itala-kernel/tests/m0_integration.rs`
- `crates/itala-plugin-host/tests/m0_integration.rs`

**Estimated scope:** M

---

### Checkpoint C: M0 complete

**Gate:**
- [x] All 16 features complete and acceptance criteria met.
- [x] `just ci` green (lint + test + deny + doc).
- [x] F16 integration suite passes deterministically (10 consecutive runs).
- [x] [Definition of Done](../../.opencode/rules/definition-of-done.md) checklist satisfied.
- [x] **Human review and approval.**

---

## Dependency graph

```text
F1 ──┬──► F2 ──► F3 ──┬──► F4 ──────────────────┐
     │                │                          │
     │                ├──► F5 ──► F7             │
     │                │      │                   │
     │                ├──► F6 ──► F8 (mem VFS)   │
     │                │      └──► F9 (local FS)  │
     │                │                          │
     │                └──────────────────────────┼──► F15 ──┐
     │                                           │          │
     └──► F10 ──► F11 ──► F12 ──► F13 ──► F14 ──┘          │
                                                             ├──► F16
                                                             │
                         Checkpoint A (after F9)             │
                         Checkpoint B (after F14)            │
                         Checkpoint C (after F16)            │
```

**Critical path:** F1 → F2 → F3 → F6 → F9 *(Checkpoint A)* → F10 → F11 →
F12 → F13 → F14 *(Checkpoint B)* → F15 → F16 *(Checkpoint C)*

**Parallelizable after F3:** F4, F5, F6 can be done concurrently.
**Parallelizable after F6:** F8, F9 can be done concurrently.
**F10 can start** as soon as F4 is done.

---

## Sizing summary

| Feature | Size | Files |
|---------|------|-------|
| F1: Scaffolding | S | 4 Cargo.toml + 4 lib.rs + CI |
| F2: ID types | S | 1–2 |
| F3: Error types | S | 1 |
| F4: Capability model | M | 2 |
| F5: Event bus | M | 2 |
| F6: VFS port trait | M | 2 |
| F7: Config registry | M | 2–3 |
| F8: In-memory VFS | S | 1 |
| F9: Local FS adapter | M | 2–3 |
| F10: Manifest model | S–M | 2 |
| F11: Plugin API crate | M | 3–4 |
| F12: Host — discovery/loading | M–L | 3–4 |
| F13: Host — lifecycle | M | 2 |
| F14: Host — capability enforcement | M | 2 |
| F15: Durability | M | 2–3 |
| F16: Integration tests | M | test files |

---

## Risks & mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `async fn in trait` vs `async-trait` for `DynVfs` | Medium | Decided: use `async-trait` for object safety (AD-3). Native `async fn in trait` available for monomorphized paths. |
| Rust-native plugins aren't truly sandboxed | Medium | Capability enforcement limits blast radius. WASM deferred to M5. Documented in plugin API docs. |
| `notify` file watcher cross-platform parity | Medium | F9 tests must pass on Linux (CI). macOS/Windows parity deferred to MVP-β (desktop). |
| Plugin lifecycle complexity | Medium | Keep M0 lifecycle simple: init → activate → deactivate. Config-change-triggered re-init can be basic. |
| Event bus message loss at capacity | Low | Bounded capacity ≥ 1024 + at-least-once contract documented. Handlers idempotent by convention. |

---

## Open questions

> These are flagged for review. Unanswered ones will be resolved before the
> relevant feature starts.

1. **Stale rules.** The `.opencode/rules/` checklists (testing-patterns,
   security, performance, observability) are web/TypeScript-focused. The old
   `.claude/rules/rust/` conventions were deleted in the cleanup. Should we
   write Rust-specific conventions as part of F1, or defer until after M0?

2. **CI pipeline.** `.github/workflows/` is empty. Should F1 include GitHub
   Actions setup running `just ci`? *(Recommended: yes.)*

3. **Scope check.** The plugin host (F10–F14) is the big addition vs. the
   previous sprint plan (which deferred it). Confirm it belongs in M0 vs.
   splitting it out. *(Roadmap M0 description explicitly includes "plugin
   lifecycle.")*

4. **Plugin loading mechanism detail.** M0 uses Rust-native plugins
   registered via a factory function. Should we also support loading `cdylib`
   plugins (dlopen), or is compile-time registration sufficient for M0?
   *(Recommended: compile-time registration only for M0; `cdylib` in M5.)*

---

_Living document. Features are marked `[x]` as they complete. Checkpoint gates
require human review before proceeding. The [Definition of Done](../../.opencode/rules/definition-of-done.md) applies to every feature on top of its acceptance criteria._
