# Roadmap

> **Status:** Directional — no dates, no calendar. Product-level; implementation detail lives
> in the local `SPEC.md` working files (current effort, see AGENTS.md) and the [architecture doc](./ARCHITECTURE.md).

Itala is a free, open-source **cross-platform knowledge management system**: durable content
as plain Markdown you control, useful first-party capabilities behind explicit contracts, and
a capability-constrained plugin ecosystem. Local-first, cloud-optional, extensible by design.

---

## How we plan

### Scope decomposes top-down

```text
VISION   "What problem are we solving?"   docs/VISION.md
  │
ROADMAP  "What directions / milestones?"  docs/ROADMAP.md  (this)
  │
SPEC     "Current effort requirements"    SPEC.md + SPEC-<module-id>.md (local, gitignored)
```

Each effort ships through the harness lifecycle

```text
DEFINE → /spec       spec-driven-development             → SPEC.md (capability map) + SPEC-<id>.md (status: draft)
PLAN   → /plan       planning-and-task-breakdown         → tasks/{plan.md, todo.md, <id>.md chunks}
BUILD  → /build      incremental-implementation + TDD    → source + tests (per-task commits)
VERIFY → /test       debugging-and-error-recovery        → just test green
REVIEW → /review     code-review-and-quality             → sign-off
SHIP   → /ship       shipping-and-launch                 → merge PR (module marked done in SPEC.md)
```

In ZCode the planning command is `/planning` — `/plan` is a reserved
built-in there; opencode keeps `/plan`.

Specs and task chunks are **local working artifacts** (`SPEC*.md`, `tasks/`),
gitignored — never committed. Git is the full history of shipped code; the
local SPEC files are the browsable index of the initiative in flight. The
next effort starts with a fresh `/spec` run.

---

## Themes

Six strategic directions: five make up the **launch core**, and **Studio** extends the product
as a post-launch direction.

| Theme | What it means |
|-------|---------------|
| **Foundation** | The kernel and plugin host that everything runs on — event bus, VFS, capabilities |
| **Write** | A canonical Markdown document engine — parsing, editing transactions, wiki links, block references, rendering semantics |
| **Connect** | A knowledge graph that links your notes bidirectionally — backlinks, mentions |
| **Find** | Full-text search and quick-open discovery across your entire vault |
| **Extend** | A safe initial extension surface — WASM tools, commands, exporters, renderers installed locally or from a curated signed index |
| **Studio** *(post-launch)* | A workspace where any note-acting plugin becomes a tool — compose, transform, automate |

---

## Milestones

**CURRENT MILESTONE: M2 — Write Engine** (requirements to be defined via `/spec` in the local `SPEC-*.md` working files).

| # | Milestone | State |
|---|-----------|-------|
| M0 | Foundation | ✅ Shipped |
| M1 | Vaults & Notes | ✅ Shipped |
| M2 | Write Engine | 🔵 Current |
| M3 | Connect | Not started |
| M4 | Find | Not started |
| M5 | Extend | Not started |

The product is built **CLI-first** through M0–M5 to exercise the kernel and plugin host
end-to-end; the desktop shell (MVP-β) then turns the M2 engine into the live-preview writing
experience. Each milestone must pass its tests and a realistic dogfooding scenario before the
next one starts.

---

## Beyond the launch core

- **Platform phasing.** MVP-α (CLI) → MVP-β (Desktop, Tauri) → Post-MVP (Web, then Mobile).
  MVP-β begins only after MVP-α is green.
- **Studio** *(post-launch-core direction).* A workspace surface where any note-acting plugin
  becomes an invokable, composable **tool**. Deterministic-first (offline); AI is an opt-in
  tool type. Scoped in detail once MVP-α is green.
- **Future directions** (revisited once the core ships): synchronization & collaboration,
  intelligence (local-first AI), advanced graph analytics, whiteboard/canvas, workflow
  automation (evolves from Studio), mobile-first enhancements, enterprise features, expanded
  extension surfaces, community hub.

---

- Decisions: [`adrs/`](./adrs/) (immutable; supersede via a new ADR).
- How it's built: [`ARCHITECTURE.md`](./ARCHITECTURE.md).
