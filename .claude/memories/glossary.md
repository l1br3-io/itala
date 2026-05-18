# Glossary

Itala terminology. Stable reference — update when a term's meaning shifts.

## Product terms

**Vault** — a folder containing notes, assets, and Itala configuration (`.itala/` subfolder). The user owns the vault; Itala reads and writes into it.

**Note** — a Markdown file (`.md`) inside a vault, with optional YAML frontmatter and CRDT-backed body.

**Frontmatter** — YAML metadata at the top of a note, fenced by `---`. Stores tags, custom fields, plugin-managed properties.

**Wiki link** — `[[Note Name]]` or `[[Note Name|Display]]`. Resolved against the vault's link graph.

**Block reference** — `[[Note Name#^block-id]]`. Targets a specific block within a note. Block IDs are content-addressed (hash of canonical content).

**Transclusion** — `![[Note Name]]`. Embeds another note's content inline at render time.

**Backlink** — the reverse direction of a link. Every link in the graph creates a backlink in the target's view.

**Unlinked mention** — text that matches a note title but isn't formatted as a link. Surfaced for the user to optionally link.

**Knowledge graph** — the live graph of notes and their connections (wiki links, block refs, tags).

## Architecture terms

**Kernel** — `crates/itala-kernel`. Shared types, event bus, VFS trait, capability system.

**Bounded context** — a self-contained domain area (note editor, graph, search, sync, vault, AI, publish). Each lives in its own crate. Contexts communicate only via the event bus or explicit trait calls.

**Adapter** — a concrete implementation of a port (a trait). Local-filesystem adapter, S3 adapter, OpenAI adapter, etc.

**Shell** — a platform-specific entry point (Tauri desktop, Tauri mobile, CLI, WASM web) that wires adapters to contexts.

**Plugin** — an extension that compiles to a WASM component and runs in `wasmtime`. Declares capabilities in `plugin.toml`.

**Capability** — a permission a plugin requests from the host (e.g. `host.vfs.read`, `host.events.note-saved`). The kernel mediates every host call against the plugin's manifest.

**Plugin manifest** — `plugin.toml` describing the plugin's id, version, type, and required/optional capabilities.

**Plugin API** — the WIT-defined contract in `crates/itala-plugin-api`. Semver-frozen post-1.0.

**WIT** — WebAssembly Interface Type. The DSL we use to define the plugin contract.

**Event bus** — a `tokio::sync::broadcast`-backed pub/sub for cross-context notifications (note saved, link updated, index rebuilt).

## Editor terms

**CRDT** — Conflict-free Replicated Data Type. Itala uses `yrs` (the Rust Y-CRDT) for note bodies, enabling concurrent edits without merge conflicts.

**State vector** — a compact summary of a Y document's known clocks. Used to compute the diff between two peers.

**Y document** — `yrs::Doc`. The CRDT root for a single note. Holds a `body` text + `frontmatter` map + per-block metadata maps.

**Awareness** — ephemeral, presence-style state shared between collaborators (cursor, selection). Not persisted.

## Sync terms

**Transport** — the trait `itala-sync` exposes. Each concrete transport (S3, WebDAV, p2p) implements send/receive of binary updates.

**Update** — a binary-encoded Y-CRDT diff. Idempotent: applying the same update twice equals applying it once.

**Selective sync** — filtering which notes flow through a transport, by folder or tag.

**E2E encryption** — sync layer encrypts every update with a user-derived key before handing to the transport. Transports see only ciphertext.

## Other

**MSRV** — Minimum Supported Rust Version. Pinned in `rust-toolchain.toml`.

**MVP** — Minimum Viable Product. See `docs/ROADMAP.md` for what's in and what's deferred.

**Hub** — the community plugin repository. Plugins are published here with signatures and reviewed for capability requests.
