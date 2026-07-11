# PRD-0001 — Functional Contracts

> **Status:** Draft — the *what* Itala does. For the *how* see [`../ARCHITECTURE.md`](../ARCHITECTURE.md); for the *why* see [`../VISION.md`](../VISION.md); for sequencing see [`../ROADMAP.md`](../ROADMAP.md).
> **Scope:** User-visible capabilities. Implementation detail (crate boundaries, data structures, control flow) lives in the architecture doc, not here.

This document describes the product-level functional contracts of Itala: the capabilities users get and the requirements each must satisfy. It was split out of the architecture document so that architecture can focus on *how the system is built* and this file can focus on *what the system does*.

Each section maps to a milestone or future direction in the [roadmap](../ROADMAP.md). Capabilities marked **(Plugin)** are not part of the non-replaceable core; they are realized by plugins against the stable plugin API.

---

## 1. Vaults and Notes

A **vault** is a logical container of notes, assets, and configuration. By default it maps to a folder on the local filesystem, but the physical storage is pluggable via the VFS abstraction — backends such as S3, IndexedDB (web), WebDAV, or a database can be provided by a Storage plugin without changing any other part of the system.

- Users can create, open, and switch between multiple vaults.
- Vault metadata and settings are stored in a `.itala` subfolder (or equivalent location in non-filesystem backends).
- Notes are stored as plain Markdown (`.md`) files with optional YAML frontmatter for metadata; the VFS layer handles translating this to the underlying storage format.
- Users can move, rename, and delete notes; links update automatically.
- File version history is maintained locally, with configurable retention.
- Users can view differences between versions and restore previous states.

## 2. Note Editing

The editor provides a fluid writing experience with real-time Markdown rendering.

- Support for CommonMark and GitHub Flavored Markdown.
- Wiki-style links: `[[Note Name]]` and `[[Note Name|Alias]]`.
- Block references: `[[Note Name#^block-id]]` to link to a specific paragraph or heading.
- Transclusion: `![[Note Name]]` embeds the content of another note inline.
- Live preview that shows formatted text while editing.
- Undo/redo stack with history.
- Multi-cursor editing support.
- Syntax highlighting for code blocks.
- LaTeX math rendering (via plugin).
- Diagram rendering (Mermaid, via plugin).
- Custom rendering extensions registered by plugins (e.g., query results, dynamic views).

## 3. Knowledge Graph

The system automatically builds and maintains a graph of note relationships.

- Automatic backlink index — every link creates a reciprocal connection.
- Users can view backlinks and outgoing links for any note.
- Unlinked mentions are surfaced (text that matches a note title but isn't linked).
- Graph visualization (as a separate plugin) showing nodes and connections.
- Link resolution works across vaults and external references (URLs).

## 4. Search and Discovery (Plugin)

Users can quickly find notes by content or metadata.

- Full-text search across all notes in a vault.
- Search results update incrementally as notes change.
- Filters by tag, folder, frontmatter field, or date range.
- Quick-open palette with fuzzy filename matching.
- Search index is stored locally and can be rebuilt if corrupted.

## 5. Synchronization (Plugin)

Sync keeps notes consistent across multiple devices.

- Sync is an optional plugin; local-only vaults require no network.
- Conflict-free merges for concurrent edits (CRDT-based).
- Pluggable transport backends: peer-to-peer, relay server, S3, WebDAV, Git.
- End-to-end encryption option at the sync layer.
- Selective sync — choose which folders or tags to synchronize per device.
- Offline-first: all edits work without connectivity; sync reconciles when online.

## 6. Identity and Security (Plugin)

Authentication and secret management are provided by plugins.

- Local vaults require no authentication.
- Supported auth methods: passphrase (local encryption), OAuth2, API keys, biometric (platform keychain).
- Secrets (API keys, tokens) are stored securely using OS keychain where available.
- Encryption-at-rest for vault data via a VFS wrapper plugin.

## 7. Intelligence (AI Plugin)

AI-powered features augment note-taking and discovery.

- Define a common interface for LLM providers (OpenAI, Anthropic, local Ollama, llama.cpp).
- Users can configure fallback chains and model selection per action.
- Inline completions (copilot-style) while typing.
- Summarize a note or selection.
- Chat with vault context (retrieve relevant notes and inject into prompt).
- Semantic search using embeddings.
- Auto-tagging and link suggestions.
- All AI features require explicit opt-in. Local-only models ensure zero data leaves the device.

## 8. Publishing and Export (Plugin)

Users can share their work in various formats.

- Export single notes or entire folders to PDF, HTML, DOCX, or plain Markdown.
- Styling via CSS templates for HTML/PDF output.
- PDF viewer with text selection, highlighting, and sticky-note annotations.
- Annotations stored as sidecar files, not modifying the original PDF.
- Annotations are linkable (`[[paper.pdf#annotation-id]]`).
- Extract highlights to a new note (template-driven).
- OCR support for image-only PDFs (via plugin).

---

## 9. Community Hub

- Central repository of plugins and workflow presets.
- Browse by category, popularity, or recency.
- Ratings, reviews, and usage statistics.
- Versioned releases with update notifications.
- Fork and remix existing plugins with attribution.
- Publishing requires plugin signing and review for certain capability levels.
- Remote kill switch for compromised plugin versions.

---

## 10. Accessibility

- Full keyboard operability — every action reachable via command palette and shortcuts.
- ARIA attributes and screen reader support; announcements for dynamic content changes.
- Focus management and ARIA landmarks.
- High-contrast themes included; color themes meet sufficient contrast ratios.
- Text resizing without loss of functionality.
- Localization framework for interface text (Fluent `.ftl` files).

## 11. Safety

- No features that could cause physical harm (e.g., no hardware control plugins by default).
- AI features do not make irreversible changes without user confirmation.
- Publishing features include clear previews before making content public.

---

_This is a living document. Each capability will be elaborated into a dedicated PRD as its milestone approaches. Non-functional targets (performance, reliability budgets) are summarized in [ARCHITECTURE.md § Design targets](../ARCHITECTURE.md#design-targets)._
