---
name: Current Sprint Plan
description: Sprint plan for Itala. Empty until /sprint-plan is run against docs/ROADMAP.md MVP scope.
type: project
---
# Sprint Plan

Created: 2026-05-04
Phase: design → MVP foundation

## Status

No sprints planned yet. Run `/sprint-plan` to decompose the MVP scope from `docs/ROADMAP.md` into sprints with features, acceptance criteria, and a dependency graph.

## MVP scope (reference, from `docs/ROADMAP.md`)

The MVP includes:

- Local vault management (create, open, organize)
- Markdown editor with live preview
- Wiki-style linking and backlinks panel
- Full-text search (Tantivy)
- Basic theming and command palette
- Plugin installation from a curated repository

Deferred to post-MVP:

- Real-time collaboration
- Mobile applications
- AI-powered features
- Advanced PDF annotation
- Plugin development SDK stabilization

## Workflow

Once `/sprint-plan` is run, this file is the authoritative sprint state. Mutate via `/sprint-update`; never hand-edit.

The pipeline (`rules/workflow.md`):

```
/sprint-plan  →  /capability  →  /plan  →  /tdd  →  /verify  →  /sprint-update
```

## Definition of done

- All Must-Have features pass acceptance criteria.
- `cargo nextest run --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo deny check` passes.
- Performance budgets in `docs/ARCHITECTURE.md` §4.1 hold under benchmarks.
- A 10k-note test vault opens, indexes, and searches within budget.
