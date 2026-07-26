# F5: Event bus

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 3: Kernel Infrastructure (ARCH §3.2–3.4)

**Description:**
Typed pub/sub on `tokio::sync::broadcast`. `KernelEvent` enum
(`#[non_exhaustive]`, `Clone`, `Debug`): `VaultOpened`, `NoteSaved { id }`,
`NoteChanged { id }`, `NoteRenamed { from, to }`, `NoteDeleted { id }`,
`LinksUpdated { id }`, `PluginLoaded { id }`, `IndexRebuildRequested`.
Bounded capacity ≥ 1024. `CancellationToken`-driven shutdown. Best-effort
delivery to active subscribers with explicit lag detection and reconciliation
(handlers remain idempotent); see [ADR-0007](../../../adrs/0007-best-effort-event-delivery.md).

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
