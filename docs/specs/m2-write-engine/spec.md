---
status: draft
started: 2026-07-27
---

# SPEC — M2: Write Engine

> **Status:** DRAFT — current effort spec. Seeded from the former functional-contracts doc
> (Note Editing) and the roadmap M2 narrative; elaborate via `/spec` before building.
> **Phase:** M2 — Write Engine (current).
> **Architecture:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md).
> **Decisions:** [ADR-0006](../../adrs/0006-markdown-canonical-source.md) (Markdown canonical).

M2 gives the shared core the document engine needed by every future writing surface.
Markdown remains the canonical source of truth (ADR-0006); the document model and editing
transactions are subordinate to it and project changes deterministically back to Markdown.

## Requirements

### Note Editing

M2 provides a shared Markdown document engine; MVP-β turns it into a fluid desktop writing
experience with real-time rendering. Both preserve Markdown as canonical data.

- Support for CommonMark and GitHub Flavored Markdown.
- Wiki-style links: `[[Note Name]]` and `[[Note Name|Alias]]`.
- Block references: `[[Note Name#^block-id]]` to link to a specific paragraph or heading.
- Transclusion: `![[Note Name]]` embeds the content of another note inline.
- Deterministic parse and rendering semantics in M2, including round-tripping edits back
  to Markdown.
- Undo/redo editing transactions in M2.

### Parser & rendering

- Parser, renderer, external-file-change, and crash-recovery behavior are proven through
  fixtures, property tests, and CLI-level operations.

## Out of scope for M2

- Live preview, syntax highlighting, multi-cursor interaction, and plugin-provided UI
  extensions — belong to the desktop phase (MVP-β), which requires a graphical surface.
- LaTeX math rendering and diagram (Mermaid) rendering — via plugin, later.
- Custom WASM rendering extensions — registered through the M5 API.

---

_This is a draft effort spec. Run `/spec` to refine requirements and acceptance criteria,
then `/planning` to decompose into [`plan.md`](./plan.md) + [`todo.md`](./todo.md)._
