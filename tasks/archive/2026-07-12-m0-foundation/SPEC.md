# SPEC — M0: Foundation

> **Status:** Shipped — archived. Paired with `plan.md` and `todo.md` in this folder.
> **Phase:** M0 — Foundation (infrastructure; no user-facing surface).
> **Architecture:** [`ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) (§3 Kernel, §4 Plugin System).
> **Decisions:** [ADR-0002](../../../docs/adr/0002-rust-native-plugins-m0.md) (superseded by ADR-0005),
> [ADR-0003](../../../docs/adr/0003-async-trait-dynvfs.md) (`async-trait` DynVfs),
> [ADR-0005](../../../docs/adr/0005-plugin-trust-and-rollout.md) (plugin trust & rollout),
> [ADR-0007](../../../docs/adr/0007-best-effort-event-delivery.md) (best-effort events).

M0 had no dedicated product-requirements section (it is pure infrastructure with no
user-facing surface). This spec is drawn from the phase plan's overview and exit condition;
see [`plan.md`](./plan.md) for the full feature-level breakdown.

## Overview

Stand up the `itala-kernel` crate (shared types, event bus, VFS port, config registry,
capability system), the `itala-plugin-api` crate (versioned plugin contract), the
`itala-plugin-host` crate (discovery, loading, lifecycle panic containment, host-call
capability policy), and the `itala-vfs-fs` crate (local filesystem VFS adapter).

## Requirements

- The kernel and trusted native plugin host run: event bus, VFS abstraction, capability
  policy, plugin lifecycle.
- M0 plugins are trusted, compile-time-registered native Rust components: lifecycle panics
  are contained at host call boundaries; they are not a security sandbox and can bypass host
  policy if malicious (WASM sandboxing deferred to M5 — ADR-0005).
- Data persists durably across restarts.

## Exit condition

An integration test suite that constructs the kernel, opens a VFS, loads plugins, enforces
host-call capabilities, contains a plugin lifecycle panic, shuts down gracefully, restarts,
and verifies state was preserved. No user-facing surface — verified entirely through tests.

## Crates

```text
crates/
  itala-kernel/        # shared types, event bus, VFS port, config, capabilities
  itala-plugin-api/    # versioned contract plugins compile against
  itala-plugin-host/   # discovery, loading, lifecycle, policy, panic containment
  itala-vfs-fs/        # local filesystem VFS adapter + file watching
```
