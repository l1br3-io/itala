---
description: Decompose a project phase or feature brief into structured sprints with features, acceptance criteria, and definition of done.
---

# Sprint Planning

This command invokes the **sprint-planner** agent to expand a phase description
or feature brief into a structured sprint plan.

## What This Command Does

1. **Read current phase status** from `docs/ROADMAP.md`, `.claude/memories/phase-status.md`, and `.claude/memories/sprint_plan_current.md`
2. **Decompose into sprints** — Must-Have (Sprint 1), Should-Have (Sprint 2), Nice-to-Have (Sprint 3+)
3. **Define features** with acceptance criteria and file-level scope
4. **Map dependencies** between features
5. **Set definition of done** for the overall sprint plan

## When to Use

Use `/sprint-plan` when:
- Starting a new phase and need to break it into deliverable chunks
- Prioritizing work within an in-progress phase
- Planning what "done" looks like for a milestone
- Need to communicate scope to collaborators

## Example Usage

```
User: /sprint-plan MVP — Foundation: Cargo workspace + kernel + vault context

Agent:
# Sprint Plan: MVP — Foundation

> Decomposed from: "Cargo workspace + kernel + vault context"

## Goal
Stand up the Itala Cargo workspace and the first two crates — `itala-kernel`
(shared types, event bus, VFS trait) and `itala-vault` (open/close, Markdown
+ frontmatter persistence). Output: a CLI binary that can open a folder,
list notes, read a single note, and watch for filesystem changes.

## Features (prioritized)

### Must-Have (Sprint 1)
1. **Cargo workspace + standards gates**
   - Description: Workspace `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`,
     `rustfmt.toml`, `.config/nextest.toml`, justfile.
   - Acceptance Criteria:
     - [ ] `cargo build --workspace` succeeds.
     - [ ] `cargo clippy --workspace --all-targets -- -D warnings` succeeds.
     - [ ] `cargo deny check` succeeds.
     - [ ] `just lint` and `just test` are wired.
   - Crate(s): repo root.
   - Complexity: Low.

2. **`itala-kernel` foundation**
   - Description: Newtype IDs (`NoteId`, `VaultId`), `Vfs` trait, event-bus
     skeleton (broadcast channel), capability token type.
   - Acceptance Criteria:
     - [ ] `#![forbid(unsafe_code)]` at crate root.
     - [ ] `cargo nextest run -p itala-kernel` passes (proptests on ID round-trip).
   - Crate(s): `crates/itala-kernel`.
   - Complexity: Medium.
...
```

## Flow

```
/sprint-plan → sprint plan → /capability (per feature) → /plan → /tdd → code
```

## Related

- `/capability` — Deep-dive a single feature into a capability plan
- `/plan` — Create implementation plan for a specific feature
- `agents/sprint-planner.yaml` — Agent definition
