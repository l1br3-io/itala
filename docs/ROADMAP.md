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

**MVP shipping criteria:**

- All kernel and plugin-host code arrived via RED-GREEN-REFACTOR ([workflow rules](../.claude/rules/workflow.md))
- ≥70% mutation score on `itala-kernel` and `itala-plugin-host` ([testing rules](../.claude/rules/rust/testing.md))
- Doctest on every public function in `itala-plugin-api` (the semver-frozen surface)
- `just lint test deny` green on every merge to `main`
- Desktop shell is not started until `itala-cli` exercises every kernel + plugin-host capability end-to-end via TDD

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
| MVP-α | **CLI** | First surface to ship. Exercises kernel + plugin-host end-to-end; scripting, batch operations, headless environments. |
| MVP-β | **Desktop** (Tauri) | Begins only after MVP-α is green. Full system integration: tray, deep links, file watching. |
| Post-MVP | **Web** | Progressive Web App with IndexedDB / OPFS storage. |
| Post-MVP | **Mobile** (iOS, Android) | Native apps with share extensions and background sync. |

**MVP-α exit trigger:** a CLI user can create a vault, write linked notes across a week of daily use, install one plugin from the curated repo, and search/backlink across the result — entirely from the CLI. This is a user-shaped finish line, not a feature-list finish line: the smallest demonstration that the kernel and plugin contract compose into a working knowledge management system. MVP-β does not begin until this trigger is met.

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
