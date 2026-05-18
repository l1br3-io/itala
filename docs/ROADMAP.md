# Itala Roadmap

This document describes what's in the first release, what's deferred, and what's on the long-term horizon. Vision and principles live in the [README](../README.md); detailed contracts and technical design live in [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## MVP

The initial release focuses on a solid foundation with a curated set of built-in plugins. Advanced features like collaborative editing, semantic search, and mobile clients are delivered in subsequent phases.

**MVP includes:**

- Local vault management (create, open, organize)
- Markdown editor with live preview
- Wiki-style linking and backlinks panel
- Full-text search
- Basic theming and command palette
- Plugin installation from a curated repository

**Deferred to post-MVP:**

- Real-time collaboration
- Mobile applications
- AI-powered features
- Advanced PDF annotation
- Plugin development SDK stabilization

---

## Platform Phasing

Itala targets consistent behavior across every platform, rolled out in stages:

| Phase | Platform | Notes |
|---|---|---|
| MVP | **Desktop** (Tauri) | Full system integration: tray, deep links, file watching. First platform to ship. |
| MVP | **CLI** | Scripting, batch operations, headless environments. |
| Post-MVP | **Web** | Progressive Web App with IndexedDB / OPFS storage. |
| Post-MVP | **Mobile** (iOS, Android) | Native apps with share extensions and background sync. |

---

## Future Scope (V2+)

- **Real-time collaboration** — multiple users editing the same note simultaneously.
- **Advanced graph analytics** — note similarity, cluster detection, knowledge gap analysis.
- **Whiteboard / canvas** — visual note-taking and spatial organization.
- **Workflow automation** — trigger actions based on note changes (similar to IFTTT).
- **Mobile-first enhancements** — widgets, Siri/Assistant shortcuts, watch companion.
- **Enterprise features** — team management, audit logs, SSO, policy enforcement.
- **Plugin monetization** — paid plugins and revenue sharing for creators.

---

_This roadmap is a living document. Priorities may shift as the project evolves and the community weighs in._
