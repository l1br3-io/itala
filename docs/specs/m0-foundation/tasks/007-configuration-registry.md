# F7: Configuration registry

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 3: Kernel Infrastructure (ARCH §3.2–3.4)

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
