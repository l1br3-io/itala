# Vision

> **Status:** Living document — the "why" behind Itala.
> **Owner:** Itala Team — **l1br3** *(pronounced "libre")*
> **Tagline:** *"Your knowledge, your way. Built to extend."*

Itala is a free, open-source **cross-platform knowledge management system** — a product of the **l1br3** organization (pronounced "libre") — for notes, links, and ideas. It ships with strong first-party defaults while designing functional areas behind explicit contracts so they can be extended or replaced as those contracts mature. The non-replaceable framework is the kernel, core bounded contexts, and plugin host; third-party extensions supply additional tools and, over time, alternative implementations. Your durable content is plain Markdown stored locally by default, with storage adapters able to back a vault with a database, an S3 bucket, IndexedDB, or another backend.

You **capture**, **connect**, and **work with** your notes. The first two are the editor and the knowledge graph; the third is **Studio** — a workspace surface where any plugin that acts on notes becomes a tool you can invoke and compose. Notes aren't just stored — they become inputs to study guides, summaries, reports, flashcards, exports, and more, through tool pipelines that work fully offline by default, with AI as an opt-in tool type.

Itala targets desktop, mobile, and the web from a shared core, introduced one platform at a time. Plugins adapt to platform capabilities while presenting a consistent experience. It is local-first and cloud-optional: all core functionality works entirely offline, and syncing or AI features are opt-in plugins that never send data the user hasn't explicitly chosen to share.

The opportunity is not that every existing tool is bad or monolithic; each makes a different trade-off among polish, openness, ownership, extensibility, collaboration, and platform support. Itala aims to combine transparent Markdown ownership, local operation, capable first-party workflows, and permission-constrained extensions in one coherent system.

**Product positioning:** *"Your knowledge, your way. Built to extend. Free, by l1br3."*

---

## The problem

Users increasingly rely on digital tools to capture, connect, and retrieve knowledge — notes, links, research, ideas. Existing tools necessarily bundle opinions about editing, storage, search, sync, and AI. Those choices often work well, but users who need a different privacy model, data location, workflow, or extension boundary may have to replace the entire application.

Existing solutions all impose friction or limitations:

- **Obsidian** — Excellent editor, graph, and ecosystem, but closed-source; community plugins execute with broad application privileges.
- **Notion** — Powerful and collaborative, but cloud-centered rather than local-first, with data represented in its service rather than as a directly usable Markdown folder.
- **Logseq** — Open source, but outliner-first with a different paradigm; not a drop-in replacement for linear note-takers.
- **Joplin** — Free, open-source, mature, and extensible, but makes different trade-offs around filesystem transparency, graph-oriented work, and extension boundaries.
- **Bear / Apple Notes** — Polished but Apple-only, closed, and not extensible.
- **VS Code + markdown extensions** — General-purpose editor, not optimized for knowledge management, linking, or graph navigation.

The core pains:

1. **Fixed product boundaries** — many tools expose only selected capabilities for replacement or extension.
2. **Data lock-in** — some tools keep notes primarily in proprietary formats or vendor clouds rather than directly usable files.
3. **Platform fragmentation** — platform support and behavior can vary significantly between desktop, mobile, and web.
4. **Plugin limitations** — depending on the tool, extensions may be narrowly constrained, broadly privileged, or tied to one runtime.
5. **Cost** — some advanced personal knowledge features require recurring subscriptions.
6. **Privacy erosion** — cloud-centered tools process personal knowledge on third-party infrastructure by default.
7. **Dead-end documents** — notes are captured but rarely reshaped; even tools that transform them tend to be opaque and non-composable, giving you fixed outputs in a fixed way rather than letting you assemble your own workflows.

Itala addresses this with strong built-in capabilities, explicit extension contracts, plain Markdown the user controls, and a path for the community to extend the experience — local-first, cross-platform, and extensible by design.

---

## Core principles

### Modularity by Default

Every functional area of Itala is designed behind a clear contract:

- **Storage backends** (local files, cloud, WebDAV)
- **Editors** (Markdown, rich text, canvas)
- **Search engines** (full-text, semantic)
- **Sync providers** (peer-to-peer, relay server, Git)
- **AI services** (local models, cloud APIs)
- **Export formats** (PDF, HTML, DOCX)
- **Studio tools** (deterministic + AI transforms that act on notes)

First-party implementations establish a useful product and prove each contract before it becomes public API. Third-party plugins are then discoverable, installable, and composable. Initial third-party extensions focus on WASM tools, commands, exporters, and renderers; deeper replacement surfaces such as editors, storage, and sync arrive only when their contracts are proven.

### Local-First, Cloud-Optional

All core functionality works entirely offline. The user's durable content lives wherever they choose — plain Markdown files in a local folder by default, but the storage layer is pluggable. Markdown is the canonical source of truth; editor models, indexes, and future synchronization state must always project changes back to it. Syncing and cloud features are optional plugins. The user controls where their data resides and in what form.

### Extensible Knowledge Graph

Notes link to each other via `[[wiki-style]]` links, block references, and tags. The system maintains a live graph of connections. Backlinks, unlinked mentions, and graph visualizations are first-class features.

### Studio — A Workspace for Your Notes

Studio treats *working with notes* as a first-class act. Any plugin that acts on notes — an exporter, a renderer, a command, a search, an AI provider — surfaces as an invokable **tool**. Tools compose into pipelines: chain a few together to turn notes into a study guide, a merged report, a flashcard deck, or an export. Deterministic tools run offline on day one; AI is an opt-in tool type, not a dependency. Saved compositions (**recipes**) are shareable presets that reproduce a workflow in one click. Studio is note-centric by design — infrastructure plugins (storage, sync, themes) stay under the hood; only note-acting plugins become workspace tools.

### Cross-Platform Consistency

The same core logic runs on desktop (Windows, macOS, Linux), mobile (iOS, Android), and the web. Plugins adapt to platform capabilities while presenting a consistent user experience.

### Community-Driven Ecosystem

The ecosystem begins with local installation and a small, curated, signed index. As participation justifies it, this can grow into a community hub for discovering and sharing versioned plugins and workflow presets. Governance-heavy features such as ratings, reviews, and automated revocation are later concerns, not prerequisites for a useful extension system.

---

## Goals

| Goal | Target |
|------|--------|
| Extensible by proven contracts | Kernel, core bounded contexts, and plugin host form the framework; public extension points follow working first-party implementations |
| Local-first, works offline | All core editing, linking, and search work with zero connectivity |
| Plain Markdown, no lock-in | Notes stored as `.md` files with optional YAML frontmatter |
| Cross-platform from one core | CLI first, then desktop, web, and mobile from shared kernel logic |
| Knowledge graph built-in | Wiki links, backlinks, and unlinked mentions in the launch core |
| Work with notes, not just store them | The first Studio release ships deterministic tools that compose into fully offline workflows |
| Safe extension path | Install capability-constrained WASM tools from a curated signed index in MVP-alpha |
| Fast | Cold startup < 500 ms desktop; open a 10 KB note < 50 ms; search 10k notes < 200 ms |
| Genuinely free | $0 required monthly app cost; MIT OR Apache-2.0 |
| Healthy project | Useful to its maintainers, documented for contributors, and improved through sustained dogfooding; community adoption is welcome upside |

### Non-goals (explicitly out of scope)

**At launch — NOT shipping:**

- Real-time collaborative editing.
- Mobile applications (iOS, Android).
- AI-powered features (completions, semantic search, auto-tagging). Studio ships with **deterministic** tools only; AI-powered Studio tools arrive with the Intelligence milestone.
- Advanced PDF annotation.
- Stable third-party plugin SDK guarantees; the first extension contracts remain explicitly experimental.

**Across all phases — NOT building:**

- A proprietary cloud service — always local-first; sync is an opt-in plugin, never mandatory.
- Paid access requirements — the app remains free; optional donations, sponsorships, grants, and paid support may fund project costs without gating product capabilities.
- A closed plugin marketplace — any future community registry remains open to publishing, forking, and remixing under documented review rules.
- Hardware control or IoT features — knowledge management only.

---

## Who it's for

| Persona | Description | Key need |
|---------|-------------|----------|
| **The Knowledge Worker** *(primary)* | Takes notes daily; links ideas; navigates a growing personal knowledge base | A fast, reliable editor with wiki links and backlinks — local-first, no cloud dependency |
| The Developer / Power User | Wants to extend the system, write plugins, customize workflows | A documented, capability-constrained WASM API and Studio as a composition playground for tools; additional runtimes only where they add clear value |
| The Privacy-Conscious User | Values data sovereignty; rejects cloud-only tools | Local-first, plain Markdown, optional encryption at rest, zero telemetry by default |
| The Cross-Platform User | Works across desktop, mobile, and browser | One consistent experience from a shared core, with optional sync |
| The Researcher / Academic | Manages PDFs, highlights, citations, and a knowledge graph | PDF annotation, extract highlights to notes, link annotations, block references; turn highlights into study artifacts via Studio |
| The Open-Source Advocate | Prefers community-driven, transparent tools | Free, open-source development, an open extension index, and transparent project decisions |

---

## What success means

Itala is an open-source hobby project and a vehicle for exploring Rust through a real product. It has no deadline and does not need commercial adoption to justify its existence. Success is evaluated in this order:

1. **Learning and craftsmanship** — each milestone explores meaningful systems problems while leaving understandable, well-tested code behind.
2. **Trustworthy personal use** — maintainers can use Itala on real notes without data loss, lock-in, or fragile recovery procedures.
3. **A genuinely good product** — first-party defaults are coherent and useful without requiring users to assemble the application from plugins.
4. **Contributor accessibility** — architecture, decisions, extension examples, and issue boundaries make participation practical.
5. **Organic adoption** — users, contributors, and plugin authors are welcome indicators that the product has value, but they are an outcome rather than a schedule or growth target.

Stage-appropriate dogfooding is therefore part of every milestone. Infrastructure milestones must survive integrated failure and restart scenarios; user-facing increments must survive realistic use on non-critical copies of real vaults. A milestone is not complete merely because its abstractions compile. Product communication — documentation, release notes, demos, and easy installation — is treated as part of open-source usability, not as commercial marketing.

---

## Where this fits

This vision is the top of the planning hierarchy — the *why*. Direction,
milestones, and the delivery lifecycle (DEFINE → PLAN → BUILD → VERIFY →
REVIEW → SHIP) live in [`ROADMAP.md`](./ROADMAP.md).
