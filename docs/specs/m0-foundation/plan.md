# Implementation Plan: M0 — Foundation

> **Status:** Complete — all 16 features implemented; `just ci` green (lint + test + deny + doc); 73 tests + 12 doctests passing deterministically.
> **Milestone:** [M0 — Foundation](../../ROADMAP.md)
> **Architecture:** [ARCHITECTURE.md §3 (Kernel) + §4 (Plugin System)](../../ARCHITECTURE.md)
> **SPEC:** [M0 requirements](./spec.md)

## Overview

Stand up the `itala-kernel` crate (shared types, event bus, VFS port, config
registry, capability system), the `itala-plugin-api` crate (versioned plugin
contract), the `itala-plugin-host` crate (discovery, loading, lifecycle panic
containment, host-call capability policy), and the `itala-vfs-fs` crate (local
filesystem VFS adapter). No user-facing surface — everything is verified
through tests.

**Exit condition:** an integration test suite that constructs the kernel,
opens a VFS, loads plugins, enforces host-call capabilities, contains a plugin
lifecycle panic, shuts down gracefully, restarts, and verifies state was
preserved.

## Crates created in M0

```text
crates/
  itala-kernel/        # shared types, event bus, VFS port, config, capabilities
  itala-plugin-api/    # versioned contract plugins compile against
  itala-plugin-host/   # discovery, loading, lifecycle, policy, panic containment
  itala-vfs-fs/        # local filesystem VFS adapter + file watching
```

No `apps/` shells. No `itala-vault`, `itala-note`, `itala-graph` — those are M1+.

---

## Architecture decisions

### AD-1: Plugin runtime — trusted Rust-native first, WASM deferred to M5

The architecture doc targets WASM (wasmtime) for third-party plugins. But
`wasmtime` is a massive dependency and implementing the WIT/ABI boundary is a
project unto itself. For M0:

- **M0 ships trusted Rust-native plugins** — plugins implement traits from
  `itala-plugin-api`, registered at startup (compiled into the binary or loaded
  as `cdylib`). Capability checks mediate and audit calls made through the
  host, but native code can bypass the host and is therefore not sandboxed.
- **WASM sandboxing deferred to M5 (Extend)** — when the plugin ecosystem
  milestone arrives, we add the WASM runtime and WIT contract. The plugin API
  trait is designed to be WASM-compatible from the start (no host-only types
  in the contract, explicit `Send` bounds).

**Trade-off:** M0 plugins are trusted in-process components. Lifecycle panic
handling improves resilience to ordinary failures, but capability enforcement
does not limit malicious native code. [ADR-0005](../../adrs/0005-plugin-trust-and-rollout.md)
supersedes the original security interpretation while preserving this rollout.

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

Per-task detail (acceptance criteria, files, verification, dependencies) lives in [`tasks/`](./tasks/).

### Phase 1: Scaffolding

- [x] [F1: Crate skeletons + CI pipeline](./tasks/001-crate-skeletons-ci-pipeline.md)

### Phase 2: Kernel Core Types (ARCH §3.1, §3.5)

- [x] [F2: ID types](./tasks/002-id-types.md)
- [x] [F3: Error types](./tasks/003-error-types.md)
- [x] [F4: Capability model + registry](./tasks/004-capability-model-registry.md)

### Phase 3: Kernel Infrastructure (ARCH §3.2–3.4)

- [x] [F5: Event bus](./tasks/005-event-bus.md)
- [x] [F6: VFS port trait + types](./tasks/006-vfs-port-trait-types.md)
- [x] [F7: Configuration registry](./tasks/007-configuration-registry.md)

### Phase 4: VFS Implementations

- [x] [F8: In-memory VFS (test support)](./tasks/008-in-memory-vfs-test-support.md)
- [x] [F9: Local filesystem VFS adapter](./tasks/009-local-filesystem-vfs-adapter.md)

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

### Phase 5: Plugin System (ARCH §4)

- [x] [F10: Plugin manifest model](./tasks/010-plugin-manifest-model.md)
- [x] [F11: Plugin API contract crate](./tasks/011-plugin-api-contract-crate.md)
- [x] [F12: Plugin host — discovery & loading](./tasks/012-plugin-host-discovery-loading.md)
- [x] [F13: Plugin host — lifecycle management](./tasks/013-plugin-host-lifecycle-management.md)
- [x] [F14: Plugin host — capability enforcement & audit](./tasks/014-plugin-host-capability-enforcement-audit.md)

### Checkpoint B: Plugin host working

```text
F10 ──► F11 ──► F12 ──► F13 ──► F14
                   │
                   └── lifecycle panic containment verified
```

**Gate:**
- [x] All Phase 5 features complete.
- [x] `just ci` green.
- [x] Integration test: load 3 plugins → one panics → host survives → enforce capabilities on the other two → lifecycle events in correct order.
- [x] **Review with human before proceeding to durability.**

### Phase 6: Durability & Integration

- [x] [F15: Graceful shutdown & state persistence](./tasks/015-graceful-shutdown-state-persistence.md)
- [x] [F16: End-to-end M0 integration test suite](./tasks/016-end-to-end-m0-integration-test-suite.md)

### Checkpoint C: M0 complete

**Gate:**
- [x] All 16 features complete and acceptance criteria met.
- [x] `just ci` green (lint + test + deny + doc).
- [x] F16 integration suite passes deterministically (10 consecutive runs).
- [x] [Definition of Done](../../.opencode/harness/rules/definition-of-done.md) checklist satisfied.
- [x] **Human review and approval.**


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
| Rust-native plugins are not sandboxed | Medium | Treat them as trusted first-party components. Capability checks provide policy and audit only; third-party plugins use WASM from M5 (ADR-0005). |
| `notify` file watcher cross-platform parity | Medium | F9 tests must pass on Linux (CI). macOS/Windows parity deferred to MVP-β (desktop). |
| Plugin lifecycle complexity | Medium | Keep M0 lifecycle simple: init → activate → deactivate. Config-change-triggered re-init can be basic. |
| Event bus subscriber lag at capacity | Low | Lag is explicit; consumers reconcile from canonical or durable state. Handlers are idempotent (ADR-0007). |

---

## Open questions

> These are flagged for review. Unanswered ones will be resolved before the
> relevant feature starts.

1. **Stale rules.** The `.opencode/harness/rules/` checklists (testing-patterns,
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
require human review before proceeding. The [Definition of Done](../../.opencode/harness/rules/definition-of-done.md) applies to every feature on top of its acceptance criteria._
