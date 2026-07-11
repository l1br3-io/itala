# Product Requirements Documents (PRDs)

This directory holds detailed, per-area **product requirements** — the *what* of Itala. It sits one layer below the [roadmap](../ROADMAP.md) in the planning hierarchy:

```text
VISION   "What problem does this project solve?"   ../VISION.md
ROADMAP  "What directions / milestones?"            ../ROADMAP.md
PRDs     "Detailed requirements per area"           ./   (this directory)
```

A PRD describes user-visible behavior, acceptance criteria, and scope boundaries. It deliberately stays out of *how* the feature is implemented — that lives in [`../ARCHITECTURE.md`](../ARCHITECTURE.md) and the decision records under [`../adr/`](../adr/).

## Index

| File | Area |
|------|------|
| [0001-functional-contracts.md](0001-functional-contracts.md) | The full set of user-visible capabilities (vaults, editing, graph, search, sync, identity, AI, export, hub, accessibility, safety) |

## Authoring a new PRD

1. Copy the next available number: `NNNN-short-slug.md` (zero-padded).
2. Open with a one-paragraph summary and a link back to the roadmap milestone it serves.
3. State **user stories**, **acceptance criteria**, and **out of scope**.
4. Cross-link to any [ADR](../adr/) that constrains the design.
5. Reference, don't duplicate, [VISION](../VISION.md) and [ARCHITECTURE](../ARCHITECTURE.md).

Keep PRDs product-focused. Implementation detail belongs in the architecture doc or in code comments.
