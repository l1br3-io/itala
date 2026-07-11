# Vision

> **Status:** Living document — the "why" behind Itala.
> **Owner:** Itala Team — **l1br3** *(pronounced "libre")*
> **Tagline:** *"Your knowledge, your way. Extend everything."*

Itala is a free, open-source **cross-platform knowledge management system** — a product of the **l1br3** organization (pronounced "libre") — for notes, links, and ideas. Every capability — editing, search, sync, AI, export — is a plugin. The core is a thin orchestration layer that provides shared primitives and lets plugins compose the full experience. Your data lives wherever you choose: plain Markdown files locally by default, with a pluggable storage layer that can back a vault with a database, an S3 bucket, IndexedDB, or any other backend.

You **capture**, **connect**, and **work with** your notes. The first two are the editor and the knowledge graph; the third is **Studio** — a workspace surface where any plugin that acts on notes becomes a tool you can invoke and compose. Notes aren't just stored — they become inputs to study guides, summaries, reports, flashcards, exports, and more, through tool pipelines that work fully offline by default, with AI as an opt-in tool type.

Itala runs on desktop, mobile, and the web from a single core. The same kernel logic runs everywhere; plugins adapt to platform capabilities while presenting a consistent experience. It is local-first and cloud-optional: all core functionality works entirely offline, and syncing or AI features are opt-in plugins that never send data the user hasn't explicitly chosen to share.

The disruption: competitors are monolithic — they lock your data into proprietary formats or clouds, limit extensibility, and charge recurring subscriptions. Itala makes everything replaceable, keeps your data in plain Markdown you control, and lets the community build the ecosystem.

**Product positioning:** *"Your knowledge, your way. Extend everything. Free, by l1br3."*

---

## The problem

Users increasingly rely on digital tools to capture, connect, and retrieve knowledge — notes, links, research, ideas. But existing tools are **monolithic**: each bundles editing, storage, search, sync, and AI into a closed stack the user cannot reshape.

Existing solutions all impose friction or limitations:

- **Obsidian** — Excellent editor and graph, but closed-source, desktop-first, and plugins are unsandboxed JavaScript with full system access.
- **Notion** — Powerful and collaborative, but cloud-only, not local-first, and locks your data in a proprietary database with limited export.
- **Logseq** — Open source, but outliner-first with a different paradigm; not a drop-in replacement for linear note-takers.
- **Joplin** — Free and open-source, but monolithic with a limited plugin system and inconsistent cross-platform UX.
- **Bear / Apple Notes** — Polished but Apple-only, closed, and not extensible.
- **VS Code + markdown extensions** — General-purpose editor, not optimized for knowledge management, linking, or graph navigation.

The core pains:

1. **Monolithic tools** — you can't replace or extend individual capabilities without forking the whole product.
2. **Data lock-in** — notes live in proprietary formats or vendor clouds, not yours.
3. **Platform fragmentation** — different apps on different platforms; no single consistent experience.
4. **Plugin limitations** — existing plugin systems are constrained, unsandboxed, or limited to one language.
5. **Cost** — recurring subscriptions for personal knowledge management.
6. **Privacy erosion** — cloud-only tools send your thoughts to third-party servers by default.
7. **Dead-end documents** — notes are captured but rarely reshaped; even tools that transform them tend to be opaque and non-composable, giving you fixed outputs in a fixed way rather than letting you assemble your own workflows.

Itala solves this by making every capability a plugin, keeping your data in plain Markdown you control, and letting the community compose the full experience — local-first, cross-platform, and extensible by design.

---

## Core principles

### Modularity by Default

Every functional area of Itala is a plugin:

- **Storage backends** (local files, cloud, WebDAV)
- **Editors** (Markdown, rich text, canvas)
- **Search engines** (full-text, semantic)
- **Sync providers** (peer-to-peer, relay server, Git)
- **AI services** (local models, cloud APIs)
- **Export formats** (PDF, HTML, DOCX)
- **Studio tools** (deterministic + AI transforms that act on notes)

Plugins are discoverable, installable, and composable. Users can replace or extend any part of the system.

### Local-First, Cloud-Optional

All core functionality works entirely offline. The user's data lives wherever they choose — plain Markdown files in a local folder by default, but the storage layer is pluggable. Syncing and cloud features are optional plugins. The user controls where their data resides and in what form.

### Extensible Knowledge Graph

Notes link to each other via `[[wiki-style]]` links, block references, and tags. The system maintains a live graph of connections. Backlinks, unlinked mentions, and graph visualizations are first-class features.

### Studio — A Workspace for Your Notes

Studio treats *working with notes* as a first-class act. Any plugin that acts on notes — an exporter, a renderer, a command, a search, an AI provider — surfaces as an invokable **tool**. Tools compose into pipelines: chain a few together to turn notes into a study guide, a merged report, a flashcard deck, or an export. Deterministic tools run offline on day one; AI is an opt-in tool type, not a dependency. Saved compositions (**recipes**) are shareable presets that reproduce a workflow in one click. Studio is note-centric by design — infrastructure plugins (storage, sync, themes) stay under the hood; only note-acting plugins become workspace tools.

### Cross-Platform Consistency

The same core logic runs on desktop (Windows, macOS, Linux), mobile (iOS, Android), and the web. Plugins adapt to platform capabilities while presenting a consistent user experience.

### Community-Driven Ecosystem

A central hub allows users to browse, install, rate, and share plugins and workflow presets. Developers can publish their work with versioning and update mechanisms.

---

## Goals

| Goal | Target |
|------|--------|
| Every capability is a plugin | Kernel + plugin host form the only non-replaceable code |
| Local-first, works offline | All core editing, linking, and search work with zero connectivity |
| Plain Markdown, no lock-in | Notes stored as `.md` files with optional YAML frontmatter |
| Cross-platform from one core | CLI, desktop, and web from the same kernel logic |
| Knowledge graph built-in | Wiki links, backlinks, and unlinked mentions on day one |
| Work with notes, not just store them | Studio ships with deterministic tools that compose into workflows — fully offline on day one |
| Plugin ecosystem from the start | Install from a curated repo in the MVP |
| Fast | Cold startup < 500 ms desktop; open a 10 KB note < 50 ms; search 10k notes < 200 ms |
| Genuinely free | $0 monthly app cost; MIT OR Apache-2.0 |
| Community adoption | Growing pulls, stars, and plugin submissions |

### Non-goals (explicitly out of scope)

**At launch — NOT shipping:**

- Real-time collaborative editing.
- Mobile applications (iOS, Android).
- AI-powered features (completions, semantic search, auto-tagging). Studio ships with **deterministic** tools only; AI-powered Studio tools arrive with the Intelligence milestone.
- Advanced PDF annotation.
- Plugin development SDK stabilization.

**Across all phases — NOT building:**

- A proprietary cloud service — always local-first; sync is an opt-in plugin, never mandatory.
- Paid subscription tiers — the app is always free; sustainability comes from the open-source community model.
- A closed plugin marketplace — the hub is open; anyone can publish, fork, and remix.
- Hardware control or IoT features — knowledge management only.

---

## Who it's for

| Persona | Description | Key need |
|---------|-------------|----------|
| **The Knowledge Worker** *(primary)* | Takes notes daily; links ideas; navigates a growing personal knowledge base | A fast, reliable editor with wiki links and backlinks — local-first, no cloud dependency |
| The Developer / Power User | Wants to extend the system, write plugins, customize workflows | A stable plugin API, sandboxed execution, multiple plugin languages (Rust, WASM, Lua), and Studio as a composition playground for tools |
| The Privacy-Conscious User | Values data sovereignty; rejects cloud-only tools | Local-first, plain Markdown, optional encryption at rest, zero telemetry by default |
| The Cross-Platform User | Works across desktop, mobile, and browser | One consistent experience from a shared core, with optional sync |
| The Researcher / Academic | Manages PDFs, highlights, citations, and a knowledge graph | PDF annotation, extract highlights to notes, link annotations, block references; turn highlights into study artifacts via Studio |
| The Open-Source Advocate | Prefers community-driven, transparent tools | Free, open-source, community hub for plugins and presets |

---

## Where this fits

This vision is the top of the planning hierarchy. Direction and milestones live in [`ROADMAP.md`](./ROADMAP.md).

```text
VISION   "What problem does this project solve?"     VISION.md  (this)
  │
ROADMAP  "What directions / milestones?"              ROADMAP.md
  │
PRDs     "Detailed requirements per area"            docs/prds/   (forthcoming)
  │
EPICS    "Trackable feature groups"
  │
ISSUES   "Specific, actionable work items"
  │
PRs      "Implementation against issues"
  │
RELEASES "Ship it, tag it, communicate it"
```
