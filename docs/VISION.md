# Vision

> **Design orientation.** Itala is designed for **personal knowledge management and collaboration by consent**. The user owns their data and decides what runs locally or in the cloud. The plugin model allows community contributions while maintaining clear boundaries and permission controls.

---

## Core Principles

### Modularity by Default

Every functional area of Itala is a plugin:

- **Storage backends** (local files, cloud, WebDAV)
- **Editors** (Markdown, rich text, canvas)
- **Search engines** (full-text, semantic)
- **Sync providers** (peer-to-peer, relay server, Git)
- **AI services** (local models, cloud APIs)
- **Export formats** (PDF, HTML, DOCX)

Plugins are discoverable, installable, and composable. Users can replace or extend any part of the system.

### Local-First, Cloud-Optional

All core functionality works entirely offline. The user's data lives in plain files (Markdown with YAML frontmatter) in a folder of their choice. Syncing and cloud features are optional plugins. The user controls where their data resides.

### Extensible Knowledge Graph

Notes link to each other via `[[wiki-style]]` links, block references, and tags. The system maintains a live graph of connections. Backlinks, unlinked mentions, and graph visualizations are first-class features.

### Cross-Platform Consistency

The same core logic runs on desktop (Windows, macOS, Linux), mobile (iOS, Android), and the web. Plugins adapt to platform capabilities while presenting a consistent user experience.

### Community-Driven Ecosystem

A central hub allows users to browse, install, rate, and share plugins and workflow presets. Developers can publish their work with versioning and update mechanisms.
