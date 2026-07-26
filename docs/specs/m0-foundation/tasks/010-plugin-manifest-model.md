# F10: Plugin manifest model

> Effort: `m0-foundation` · Status: shipped · Phase: Phase 5: Plugin System (ARCH §4)

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
