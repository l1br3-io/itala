# M0 — Foundation: checklist (archived)

> **Shipped.** All 16 features complete; `just ci` green (lint + test + deny + doc).
> 73 tests + 12 doctests passing deterministically. Full detail in [`plan.md`](./plan.md).

## Phase 1 — Scaffolding
- [x] F1: Crate skeletons + CI pipeline

## Phase 2 — Kernel Core Types (ARCH §3.1, §3.5)
- [x] F2: ID types
- [x] F3: Error types
- [x] F4: Capability model + registry

## Phase 3 — Kernel Infrastructure (ARCH §3.2–3.4)
- [x] F5: Event bus
- [x] F6: VFS port trait + types
- [x] F7: Configuration registry

## Phase 4 — VFS Implementations
- [x] F8: In-memory VFS (test support)
- [x] F9: Local filesystem VFS adapter

### Checkpoint A — Kernel + VFS proven

## Phase 5 — Plugin System (ARCH §4)
- [x] F10: Plugin manifest model
- [x] F11: Plugin API contract crate
- [x] F12: Plugin host — discovery & loading
- [x] F13: Plugin host — lifecycle management
- [x] F14: Plugin host — capability enforcement & audit

### Checkpoint B — Plugin host working

## Phase 6 — Durability & Integration
- [x] F15: Graceful shutdown & state persistence
- [x] F16: End-to-end M0 integration test suite

### Checkpoint C — M0 complete
