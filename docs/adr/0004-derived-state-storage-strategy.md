# ADR-0004 — Per-domain ports for derived-state storage, with per-vault backend selection

- **Status:** Accepted
- **Date:** 2026-07-12

## Context

M0 landed the [`Vfs`](../../crates/itala-kernel/src/vfs.rs) port for **content** storage — the bytes that make up a vault's notes, assets, and `.itala/` metadata. `LocalFsVfs` is the default adapter; `MemoryVfs` is the test-support adapter; the `PluginType::Storage` variant is reserved for future content backends installed as plugins after the initial M5 extension surfaces are proven. For content, pluggability is in the plan (see [VISION.md §"Modularity by Default"](../VISION.md), [PRD-0001 §1](../prds/0001-functional-contracts.md), [ARCHITECTURE.md §3.3](../ARCHITECTURE.md)).

**Structured domain state** under `.itala/index/`, `.itala/graph/`, and `.itala/history/` has no port, engine choice, or selection mechanism. Search and graph projections are rebuildable from canonical Markdown; local history contains prior canonical content and is durable rather than derivable. `persistence.rs` writes `.itala/config.toml`, `.itala/capabilities.json`, and `.itala/plugins.json` as flat files via `Vfs`, but that covers kernel-internal state, not domain state. As M3 (graph), M4 (search), and M1's file-history feature approach, the question of *where and how* that data lives becomes unavoidable. [ADR-0006](./0006-markdown-canonical-source.md) defines these durability classes.

Two gaps need to be closed together:

1. **Engine choice.** A graph index is naturally relational (nodes + edges + metadata); file history is naturally ordered and timestamped; search metadata is structured. Flat files work but scale poorly; SQLite is a strong local-first fit; Tantivy owns full-text and is unaffected. Without a port, the engine choice is implicit and baked into whichever context lands first — locking the system to that choice and removing the option to swap.
2. **Selection mechanism.** Even for content storage, the docs are silent on *how* a user picks which installed backend backs a given vault. Architecture §6.1 step 3 says "The VFS adapter for the vault's backend is instantiated" without specifying the mechanism. For derived state, configurability is undefined entirely.

This ADR closes both gaps for derived state. Content-storage selection is explicitly out of scope and remains a future expanded plugin surface after M5; this ADR establishes the *pattern* that content could later adopt.

## Decision

### 1. Per-domain port traits, in `itala-kernel`

Each domain of derived state gets its own small port trait, defined in `itala-kernel` alongside `Vfs`. A port lands when its context lands:

- **M1** — `HistoryStore` (file version history).
- **M3** — `GraphStore` (links, backlinks, unlinked mentions).
- **M4** — `SearchMetaStore` (tags, frontmatter index, quick-open metadata — *not* the full-text index, which Tantivy owns).

Each port follows the [ADR-0003](./0003-async-trait-dynvfs.md) dual-trait pattern: a native-async `FooStore` for static dispatch, plus a `DynFooStore` object-safe shadow built with `#[async_trait]`, bridged by an `into_dyn` adapter.

### 2. Ports live in `itala-kernel`, not in context crates

This preserves the [dependency DAG](../ARCHITECTURE.md#2-crate-map): adapters may depend only on the kernel, never on a bounded context. If `GraphStore` lived in `itala-graph`, then `itala-store-sqlite` would have to depend on `itala-graph`, breaking the dependency rule. Putting ports in the kernel keeps the rule intact and matches how `Vfs`, `ConfigRegistry`, and `EventBus` already work.

### 3. Two adapter crates, each implementing every port

- **`itala-store-files`** — JSON-on-`Vfs` adapter; zero new dependencies; the default everywhere. Generalizes what `persistence.rs` already does.
- **`itala-store-sqlite`** — `rusqlite` (synchronous, mature, lightweight) called via `spawn_blocking` on the tokio runtime. Opt-in per concern.

Each adapter crate ships port-conformance tests against a kernel-provided test harness, mirroring the VFS conformance suite ([ARCHITECTURE.md §7.4](../ARCHITECTURE.md)).

### 4. Selection via per-vault `config.toml` — shell wiring, not a kernel registry

A vault declares its derived-state backends in `.itala/config.toml`, one section per domain:

```toml
[storage.history]
backend = "files"            # default

[storage.graph]
backend = "sqlite"           # opt-in

[storage.graph.sqlite]       # optional; backend-specific
journal_mode = "wal"
cache_size   = -64000

[storage.search]
backend = "files"            # default
```

The **shell** reads these sections at vault-open time, instantiates the matching adapter directly, and injects the resulting `Arc<dyn Dyn*Store>` into the appropriate context. A missing or unknown backend id returns a typed `Error::Config` with the offending path — no silent fallback.

**Section-per-domain** leaves room for each backend to carry its own configuration subsection (e.g. `[storage.graph.sqlite]` for SQLite-specific options like WAL mode or cache size) without ever breaking the schema. Backend-specific subsections are forwarded verbatim to the adapter's constructor.

Defaults: `"files"` for every domain. SQLite is opt-in per concern, not all-or-nothing. A user can use SQLite for the graph while leaving history and search on flat files, or any combination.

A new `KernelEvent::StorageChanged { domain, backend }` variant (allowed by the existing `#[non_exhaustive]` declaration) signals that the vault's storage selection changed; a live hot-swap is generally not possible (existing `Arc<dyn Dyn*Store>` references are outstanding), so the typical reaction is to schedule a vault reopen.

**No kernel-level `StoreRegistry` or `StoreProvider` trait.** Two earlier drafts of this ADR proposed each in turn; both were rejected because they hard-coded the set of domains in the kernel (closing the set: every new domain would force a kernel edit) while providing no value over direct shell wiring. The kernel's job is to *define the ports*; *picking which adapter backs a vault* is wiring, and wiring belongs to the shell. When a later extension phase allows plugins to contribute backends dynamically, the registration surface naturally lives in the **plugin host** (already in the shell layer), not in the kernel — and its shape can be designed against concrete needs rather than speculated now.

### 5. `.itala/` artifact migration

| Artifact                       | Disposition                                                            |
| ------------------------------ | ---------------------------------------------------------------------- |
| `.itala/config.toml`           | Unchanged — kernel-internal, stays on `Vfs`.                           |
| `.itala/capabilities.json`     | Unchanged — kernel-internal grant map, stays on `Vfs`.                 |
| `.itala/plugins.json`          | Unchanged — kernel-internal install records, stays on `Vfs`.           |
| `.itala/history/`              | Moves to the `HistoryStore` port (M1).                                 |
| `.itala/graph/`                | Moves to the `GraphStore` port when M3 lands.                          |
| `.itala/index/`                | Search metadata moves to `SearchMetaStore` (M4); full-text stays Tantivy-owned. |

Kernel-internal artifacts stay on `Vfs` because they are flat key-value state, not relational domain state — putting them on the new ports would be forced.

### 6. Out of scope

- **Content storage backends** (alternative `Vfs` implementations). That remains a future expanded plugin surface after M5; this ADR neither adds nor removes anything from it.
- **A unified generic "structured store" trait.** Considered and rejected: it would front-load a relational abstraction before any context needs it, and object-safety is harder for a generic query surface. Per-domain ports stay small, domain-meaningful, and easy to make object-safe. If a pattern emerges across all three ports, it can be extracted later.
- **A kernel-level `StoreRegistry` or `StoreProvider` trait.** Two earlier drafts of this ADR proposed each. Both were rejected: they coupled the kernel to a closed set of domains while providing no value over direct shell wiring. Backend selection is wiring; it belongs to the shell (and, for plugin-contributed backends, to the plugin host when M5 lands).

## Consequences

- **Positive:** Real configurability — users pick the engine per concern via `config.toml`, with sane defaults.
- **Positive:** The proven `Vfs` / `DynVfs` pattern (ADR-0003) is reused, so contributors encounter one consistent port shape across content and derived state.
- **Positive:** SQLite is opt-in and isolated to one adapter crate; the kernel and the default experience carry zero new dependencies.
- **Positive:** The dependency DAG stays clean — adapters depend only on `itala-kernel`.
- **Positive:** New ports arrive with their contexts (M1, M3, M4), so the API surface is designed against real needs rather than speculated upfront.
- **Positive:** The kernel stays small — it defines ports only. Backend selection is shell wiring, where it belongs; the kernel carries no registry/provider abstraction that would couple it to a closed set of domains.
- **Positive:** The TOML schema is section-per-domain (`[storage.history] backend = "files"`), so each backend can carry its own config subsection without breaking the format.
- **Negative:** Each shell that opens a vault repeats the (small) wiring logic: read config, instantiate adapter, inject. Acceptable while there is one shell (CLI); worth extracting into a helper when the desktop shell (MVP-β) arrives.
- **Negative:** `KernelEvent::StorageChanged` cannot truly hot-swap a live store (outstanding `Arc` references), so it signals "schedule a vault reopen" rather than "swap in place." Consumers must handle the reopen semantics.
- **Negative:** `rusqlite` is a non-trivial transitive dependency surface, but it lives entirely behind `itala-store-sqlite` and is invisible to anyone using the default `files` adapter.
- **Neutral:** A future ADR may need to address the same selection gap for content storage (which `Vfs` backend backs a vault). This ADR establishes the pattern — per-vault `config.toml` sections resolved by shell wiring — that content storage can adopt directly.
- **Neutral:** The in-memory test-support adapters (e.g. `MemoryHistoryStore`) live in `itala-kernel` behind `#[cfg(feature = "test-support")]`, mirroring `MemoryVfs`.
- **Neutral:** If plugins later contribute backends dynamically, the plugin host (in the shell layer) will grow whatever registration surface is needed. Its shape is deferred until concrete requirements exist.
