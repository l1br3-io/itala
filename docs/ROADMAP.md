# Roadmap

> **Status:** Directional overview — no dates, no calendar. Business-level; no technical detail.
> **Companion:** [`VISION.md`](./VISION.md) — the problem and goals. This roadmap does not duplicate requirements; it communicates direction and sequencing.

Itala is a free, open-source **cross-platform knowledge management system**: every capability is a plugin, your notes live as plain Markdown you control, and the community extends the system through a plugin ecosystem. Local-first, cloud-optional, extensible by design.

This is the **top-level overview** of where Itala is heading — the journey, not the spec.

---

## How we plan

Each layer decomposes the one above it:

```text
VISION   "What problem does this project solve?"     VISION.md
  │
ROADMAP  "What directions / milestones?"   (this)     ROADMAP.md
  │
PRDs     "Detailed requirements per area"             docs/prds/   (forthcoming)
  │
EPICS    "Trackable feature groups"
  │
ISSUES   "Specific, actionable work items"
  │
PRs      "Implementation against issues"
  │
RELEASES "Ship it, tag it, communicate it"
```

The launch scope is **locked to the knowledge management core**: vaults, editing, linking, search, and a plugin ecosystem. Everything else is deferred.

---

## Themes

Six strategic directions: five make up the **launch core**, and **Studio** extends the product as a post-launch direction.

| Theme | What it means |
|-------|---------------|
| **Foundation** | The kernel and plugin host that everything runs on — event bus, VFS, capabilities |
| **Write** | A fluid Markdown editor with live preview, wiki links, and block references |
| **Connect** | A knowledge graph that links your notes bidirectionally — backlinks, mentions |
| **Find** | Full-text search and quick-open discovery across your entire vault |
| **Extend** | A plugin ecosystem where the community builds the rest — curated repo, hub |
| **Studio** *(post-launch)* | A workspace where any note-acting plugin becomes a tool — compose, transform, automate |

---

## Launch — Knowledge Management Core

Six milestones, each shipping a demoable increment built on the last. The product is built **CLI-first** through M0–M5 (exercising kernel + plugin host end-to-end), then the desktop shell generalizes the same core to a graphical experience.

| # | Milestone | Ships (user-visible) |
|---|-----------|----------------------|
| M0 | **Foundation** | The kernel and plugin host run — event bus, VFS abstraction, capability system, plugin lifecycle — and data persists durably across restarts |
| M1 | **Vaults & Notes** | Create, open, and switch vaults; notes are plain Markdown with frontmatter; move, rename, delete with automatic link updates; version history |
| M2 | **Write** | A Markdown editor with live preview — CommonMark, GFM, wiki links, block references, transclusion, syntax highlighting, undo/redo |
| M3 | **Connect** | The knowledge graph comes alive — automatic backlinks, outgoing links, unlinked mentions; bidirectional navigation across your notes |
| M4 | **Find** | Full-text search across every note; filters by tag, folder, frontmatter, date; quick-open palette with fuzzy matching |
| M5 | **Extend** | Install plugins from a curated repository — editors, themes, storage backends, sync, search engines, AI providers, exporters, commands, renderers |

### M0 — Foundation ✅
The kernel and plugin host run. The kernel provides shared types, an event bus, a virtual file system abstraction, a configuration registry, and the capability system that mediates all plugin interactions. Plugins are sandboxed modules with declared capabilities; a failing plugin does not crash the host. Data persists durably so nothing is lost across restarts. This milestone has no user-facing surface — it is the bedrock everything else stands on, verified entirely through tests. **Status: implemented** — the `itala-kernel`, `itala-plugin-api`, `itala-plugin-host`, and `itala-vfs-fs` crates are landed and green under `just ci`; see [the M0 plan](./plans/m0-foundation.md).

### M1 — Vaults & Notes
Users can create, open, and switch between multiple vaults — logical containers of notes, assets, and configuration that default to a local folder but can be backed by any storage plugin. Notes are plain Markdown with optional YAML frontmatter. Moving, renaming, and deleting notes updates links automatically. File version history is maintained locally, with diff viewing and restore. This milestone ships first through the CLI, exercising the kernel and vault subsystem end-to-end.

### M2 — Write
The editor provides a fluid writing experience with real-time Markdown rendering. CommonMark and GitHub Flavored Markdown are supported out of the box. Wiki-style links (`[[Note Name]]`), block references (`[[Note Name#^block-id]]`), and transclusion (`![[Note Name]]`) are first-class. Live preview shows formatted text while editing, with syntax highlighting for code blocks, undo/redo, and multi-cursor support. Custom rendering extensions registered by plugins can extend the editor.

### M3 — Connect
The system automatically builds and maintains a graph of note relationships. Every `[[link]]` creates a reciprocal connection; backlinks and outgoing links are visible for any note. Unlinked mentions — text matching a note title that isn't linked — are surfaced so users can connect related ideas. Link resolution works across vaults and external URLs. The graph turns a folder of notes into an interconnected knowledge base.

### M4 — Find
Users can find any note instantly. Full-text search runs across all notes in a vault, updating incrementally as notes change. Filters by tag, folder, frontmatter field, or date range narrow results. A quick-open palette with fuzzy filename matching gets you to any note in keystrokes. The search index is stored locally and can be rebuilt if corrupted.

### M5 — Extend
The plugin ecosystem comes alive. Users can install plugins from a curated repository — editors, themes, storage backends, sync transports, search engines, AI providers, exporters, commands, and renderers. Each plugin declares its capabilities in a manifest; the kernel enforces permissions at runtime. A community hub allows browsing by category, popularity, or recency, with ratings, versioned releases, and update notifications.

---

## Platform Phasing

The same core ships on multiple platforms, in stages:

| Phase | Platform | Notes |
|-------|----------|-------|
| **MVP-α** | **CLI** | First surface to ship. Exercises every kernel + plugin-host capability end-to-end; scripting, batch operations, headless environments. |
| **MVP-β** | **Desktop** (Tauri) | Begins only after MVP-α is green. Full system integration: tray, deep links, file watching. |
| **Post-MVP** | **Web** | Progressive Web App with IndexedDB / OPFS storage. |
| **Post-MVP** | **Mobile** (iOS, Android) | Native apps with share extensions and background sync. |

**MVP-α exit trigger:** a CLI user can create a vault, write linked notes across a week of daily use, install one plugin from the curated repo, and search and navigate backlinks — entirely from the CLI. This is a user-shaped finish line: the smallest demonstration that the kernel and plugin contract compose into a working knowledge management system. MVP-β does not begin until this trigger is met.

---

## Studio — The Notes Workspace *(post-launch-core direction)*

> **Status:** The headline direction after the launch core ships. Not part of M0–M5; scoped in detail once MVP-α is green and a PRD lands under `docs/prds/`.

You **capture**, **connect**, and **work with** your notes. The launch core owns the first two. **Studio** owns the third — a workspace surface, a peer to the editor, where notes stop being read-only documents and start being **inputs**.

### The idea

Any plugin that acts on notes — an exporter, a renderer, a command, a search, an AI provider — surfaces in Studio as an invokable **tool**. You bring notes into the workspace (a selection, a folder, a query result) and apply tools to them. The same plugins that power the rest of Itala become the building blocks of Studio, so the ecosystem built during M5 is the ecosystem Studio draws from on day one.

Tools **compose**: chain several together to turn notes into a study guide, a merged report, a flashcard deck, or an export. Most users never see the plumbing — they pick a **recipe** (a one-click preset) and get a result. Power users open the workspace and rearrange the tools themselves.

### Principles

- **Deterministic-first.** Studio ships with offline, deterministic tools (merge, extract, format, summarize structure, build decks, export). AI is an opt-in tool type that arrives with the Intelligence milestone — never a dependency.
- **One contract for every tool.** Deterministic and AI tools share the same interface, so they're interchangeable and composable. The "with or without AI" stance is structural, not a special case.
- **Note-centric boundary.** Only plugins that *act on notes* become workspace tools. Infrastructure plugins (storage backends, sync transports, themes) stay under the hood. Studio is a focused surface, not a renamed copy of the whole app.
- **Provenance and reproducibility.** Outputs are notes (so backlinks keep working) or linkable assets, and derived content carries citations back to its source notes. Saved recipes are reproducible and diff-able.

### Why it follows the launch core

Studio needs the editor (M2), the plugin ecosystem (M5), and benefits from a graphical shell — its exploratory, "playground" UX is awkward in a pure CLI. It is therefore a post-launch-core direction, sequenced after MVP-α, rather than crammed into M0–M5.

### The convergence: Studio → automation

A saved Studio composition, triggered by events instead of a button, *is* a workflow automation. Building Studio's composition engine now (designed to be event-triggerable) means the **Workflow automation** future direction falls out of it almost for free. See *Future directions* below.

---

## Future directions *(not current priorities)*

Revisited once the core system is shipped and adopted:

- **Real-time collaboration** — multiple users editing the same note simultaneously (CRDT-based).
- **Intelligence** — inline completions, semantic search, auto-tagging, link suggestions — local-first with zero data leaving the device by default.
- **Advanced graph analytics** — note similarity, cluster detection, knowledge gap analysis.
- **Whiteboard / canvas** — visual note-taking and spatial organization.
- **Workflow automation** — trigger actions based on note changes (similar to IFTTT). Evolves naturally from Studio: a saved Studio composition, triggered by events instead of a button, *is* an automation.
- **Mobile-first enhancements** — widgets, voice shortcuts, watch companion.
- **Enterprise features** — team management, audit logs, SSO, policy enforcement.

---

## Status

M0 — Foundation is **implemented** (kernel, plugin API/host, local-FS VFS adapter). [`VISION.md`](./VISION.md) holds the problem and goals; this roadmap is directional and will be updated as milestones ship and PRDs land under `docs/prds/`.
