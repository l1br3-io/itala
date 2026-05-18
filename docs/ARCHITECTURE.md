# Itala Architecture

This document describes how Itala is built under the hood: the detailed functional contracts, the plugin system, the kernel design, and the non-functional constraints that shape the implementation.

For the high-level vision see the [README](../README.md); for delivery phasing see [ROADMAP.md](./ROADMAP.md).

---

## 1. Functional Contracts

### 1.1 Vaults and Notes

A **vault** is a logical container of notes, assets, and configuration. By default it maps to a folder on the local filesystem, but the physical storage is pluggable via the VFS abstraction — backends such as S3, IndexedDB (web), or a database can be provided by a Storage plugin without changing any other part of the system.

- Users can create, open, and switch between multiple vaults.
- Vault metadata and settings are stored in a `.itala` subfolder (or equivalent location in non-filesystem backends).
- Notes are stored as plain Markdown (`.md`) files with optional YAML frontmatter for metadata; the VFS layer handles translating this to the underlying storage format.
- Users can move, rename, and delete notes; links update automatically.
- File version history is maintained locally, with configurable retention.
- Users can view differences between versions and restore previous states.

### 1.2 Note Editing

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

### 1.3 Knowledge Graph

The system automatically builds and maintains a graph of note relationships.

- Automatic backlink index—every link creates a reciprocal connection.
- Users can view backlinks and outgoing links for any note.
- Unlinked mentions are surfaced (text that matches a note title but isn't linked).
- Graph visualization (as a separate plugin) showing nodes and connections.
- Link resolution works across vaults and external references (URLs).

### 1.4 Search and Discovery

Users can quickly find notes by content or metadata.

- Full-text search across all notes in a vault.
- Search results update incrementally as notes change.
- Filters by tag, folder, frontmatter field, or date range.
- Quick-open palette with fuzzy filename matching.
- Search index is stored locally and can be rebuilt if corrupted.

### 1.5 Synchronization (Plugin)

Sync keeps notes consistent across multiple devices.

- Sync is an optional plugin; local-only vaults require no network.
- Conflict-free merges for concurrent edits (CRDT-based).
- Pluggable transport backends: peer-to-peer, relay server, S3, WebDAV, Git.
- End-to-end encryption option at the sync layer.
- Selective sync—choose which folders or tags to synchronize per device.
- Offline-first: all edits work without connectivity; sync reconciles when online.

### 1.6 Identity and Security (Plugin)

Authentication and secret management are provided by plugins.

- Local vaults require no authentication.
- Supported auth methods: passphrase (local encryption), OAuth2, API keys, biometric (platform keychain).
- Secrets (API keys, tokens) are stored securely using OS keychain where available.
- Encryption-at-rest for vault data via a VFS wrapper plugin.

### 1.7 Intelligence (AI Plugin)

AI-powered features augment note-taking and discovery.

- Define a common interface for LLM providers (OpenAI, Anthropic, local Ollama, llama.cpp).
- Users can configure fallback chains and model selection per action.
- Inline completions (copilot-style) while typing.
- Summarize a note or selection.
- Chat with vault context (retrieve relevant notes and inject into prompt).
- Semantic search using embeddings.
- Auto-tagging and link suggestions.
- All AI features require explicit opt-in. Local-only models ensure zero data leaves the device.

### 1.8 Publishing and Export (Plugin)

Users can share their work in various formats.

- Export single notes or entire folders to PDF, HTML, DOCX, or plain Markdown.
- Styling via CSS templates for HTML/PDF output.
- PDF viewer with text selection, highlighting, and sticky-note annotations.
- Annotations stored as sidecar files, not modifying the original PDF.
- Annotations are linkable (`[[paper.pdf#annotation-id]]`).
- Extract highlights to a new note (template-driven).
- OCR support for image-only PDFs (via plugin).

---

## 2. Plugin System & Community Hub

### 2.1 Plugin Architecture

- Plugins are sandboxed modules with declared capabilities.
- Manifest file (`plugin.toml`) specifies ID, version, API compatibility, required permissions, and provided capabilities.
- Plugins can be written in Rust (native, compiled), WebAssembly (portable, sandboxed), or scripting languages (Lua/Rhai).
- Lifecycle hooks: initialization, activation, deactivation, configuration changes.
- Crash isolation: a failing plugin does not crash the host application.

### 2.2 Plugin Types

| Type | Description |
|---|---|
| Editor | Extends or replaces the note editor (e.g., Vim mode, WYSIWYG). |
| Theme | CSS custom properties and styling rules. |
| Storage | Implements a VFS backend (local, S3, IndexedDB). |
| Sync | Provides a sync transport and conflict resolver. |
| AI Provider | Implements the LLM/embedding provider trait. |
| Exporter | Adds new export formats or templates. |
| Command | Adds CLI subcommands or command palette actions. |
| Renderer | Custom inline or block rendering (e.g., Dataview queries). |

### 2.3 Community Hub

- Central repository of plugins and workflow presets.
- Browse by category, popularity, or recency.
- Ratings, reviews, and usage statistics.
- Versioned releases with update notifications.
- Fork and remix existing plugins with attribution.
- Publishing requires plugin signing and review for certain capability levels.
- Remote kill switch for compromised plugin versions.

### 2.4 Inter-Plugin Communication

- **Event bus:** Loose coupling for cross-cutting concerns (note saved, file renamed, index updated).
- **Capability resolution:** Plugins can request services from other plugins (e.g., "give me the active search index").
- **Permission enforcement:** The kernel mediates all inter-plugin calls based on declared capabilities.

---

## 3. Core Components

- **Kernel:** Provides shared types (IDs, errors), event bus, virtual file system abstraction, configuration registry, and capability system.
- **Bounded Contexts:** Independent modules for note editing, knowledge graph, vault management, search, sync, identity, AI, and publishing. Contexts communicate only through events.
- **Adapters:** Implementations of ports for specific technologies (local filesystem, Tantivy search, Yrs CRDT, OpenAI API, etc.).
- **Shells:** Platform-specific entry points (CLI, Tauri desktop, WASM web, mobile wrappers) that wire adapters to contexts.
- **Plugins:** Extensions that depend only on the stable plugin API.

### 3.1 Communication Patterns

- **Direct trait calls** for performance-sensitive hot paths (keystroke → CRDT → parse → render).
- **Event bus** for decoupled, asynchronous cross-context notifications (note saved → re-index, link updated → refresh backlinks).

### 3.2 Data Model

- **Note:** ULID identifier, canonical path, frontmatter, CRDT body, timestamps, BLAKE3 checksum.
- **Link:** Source note, target (note, URL, block), type, position, display text.
- **Vault:** Root URI, configuration, installed plugins list.

### 3.3 Platform Support

| Platform | Primary Language | UI Layer | Storage |
|---|---|---|---|
| Desktop | Rust (Tauri) | Webview (TypeScript) | Local filesystem |
| Web | Rust (WASM) | Browser DOM | IndexedDB / OPFS |
| Android | Kotlin + Rust (JNI) | Webview or Native | Scoped Storage |
| iOS | Swift + Rust (FFI) | Webview or Native | App Container / iCloud |

---

## 4. Non-Functional Requirements

### 4.1 Performance

- Cold startup to interactive: < 500 ms on desktop, < 1 s on mobile.
- Open a 10 KB note: < 50 ms.
- Full-text search over 10,000 notes: < 200 ms.
- Smooth 60 fps scrolling in editor with large documents.
- Keystroke to screen update: < 16 ms (including CRDT and parsing).
- Memory: kernel < 20 MB; each plugin has a defined budget.

### 4.2 Reliability

- All operations work offline; sync reconciles when connectivity returns.
- Index corruption is recoverable via full rebuild.
- Crash recovery: unsaved changes are preserved in local CRDT state.
- Event delivery is at-least-once with idempotency where required.

### 4.3 Extensibility

- Stable plugin API with versioning.
- Developer documentation and examples.
- Test harness for plugin authors.
- Backward compatibility for at least one major API version.

---

## 5. Security & Threat Model

### 5.1 Trust Boundaries

- The user trusts the Itala core and built-in plugins.
- Third-party plugins run with limited, user-granted capabilities.
- The community hub acts as a distribution channel; plugins are signed and can be revoked.
- Cloud sync and AI providers receive only data the user explicitly chooses to share.

### 5.2 Mitigations

- **Plugin sandboxing:** WASM plugins have no host access except through granted capabilities.
- **Capability system:** Plugins declare required permissions; kernel enforces at runtime.
- **Signed plugins:** Prevent tampering and allow revocation.
- **User consent:** All network-bound features require opt-in with clear disclosure of data flows.
- **Encryption:** Optional at-rest encryption for vaults (XChaCha20-Poly1305); TLS 1.3 minimum for all network traffic.
- **Audit logs:** Record plugin capability usage for transparency.
- **CSP:** Content Security Policy enforced in web contexts.
- **Telemetry:** None by default; opt-in only, anonymized.

### 5.3 Privacy

- All user data remains local by default.
- Cloud sync and AI features require explicit opt-in.
- AI prompts are redacted of sensitive identifiers before transmission.
- No user content is sent to telemetry or analytics without consent.
- Supervisors and collaborators see only explicitly shared content.

### 5.4 Out of Scope

- Protection against a compromised operating system or kernel-level malware.
- Formal verification of plugin behavior; the capability system limits blast radius but cannot guarantee absence of logic errors.
- Regulatory compliance (GDPR, HIPAA) is the user's responsibility; Itala provides tools for data control but is not a certified compliance solution.

---

## 6. Accessibility & Safety

### 6.1 Accessibility

- Full keyboard operability — every action reachable via command palette and shortcuts.
- ARIA attributes and screen reader support; announcements for dynamic content changes.
- Focus management and ARIA landmarks.
- High-contrast themes included; color themes meet sufficient contrast ratios.
- Text resizing without loss of functionality.
- Localization framework for interface text (Fluent `.ftl` files).

### 6.2 Safety

- No features that could cause physical harm (e.g., no hardware control plugins by default).
- AI features do not make irreversible changes without user confirmation.
- Publishing features include clear previews before making content public.

---

_This document describes the intended architecture and capabilities of Itala. It is a living specification that guides development priorities and community contributions._
