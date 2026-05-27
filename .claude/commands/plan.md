---
description: Restate requirements, assess risks, and create step-by-step implementation plan. WAIT for user CONFIRM before touching any code.
---

# Plan Command

This command invokes the **planner** agent to create a comprehensive implementation plan before writing any code.

## What This Command Does

1. **Restate Requirements** - Clarify what needs to be built
2. **Identify Risks** - Surface potential issues and blockers
3. **Create Step Plan** - Break down implementation into phases
4. **Wait for Confirmation** - MUST receive user approval before proceeding

## When to Use

Use `/plan` when:
- Starting a new feature
- Making significant architectural changes
- Working on complex refactoring
- Multiple files/components will be affected
- Requirements are unclear or ambiguous

## How It Works

The planner agent will:

1. **Analyze the request** and restate requirements in clear terms
2. **Break down into phases** with specific, actionable steps
3. **Identify dependencies** between components
4. **Assess risks** and potential blockers
5. **Estimate complexity** (High/Medium/Low)
6. **Save the plan** to `.claude/plans/` with a stable ID (see § Plan File below)
7. **Present the plan** and WAIT for your explicit confirmation

## Example Usage

```
User: /plan Add a backlinks panel that shows incoming wiki links to the active note.

Agent (planner):
# Implementation Plan: Backlinks Panel

## Requirements Restatement
- For the currently open note, list every other note that contains a wiki link
  pointing to it.
- Update the panel reactively when notes are saved or links are added/removed.
- Clickable rows navigate to the source note.

## Sprint Context
- Maps to `MVP — Foundation` / Backlinks feature in the active sprint plan.

## Implementation Steps

### Step 1: Link index types (`crates/itala-graph/src/index.rs`)
- Action: Define `LinkIndex` trait with `incoming(target: NoteId) -> Vec<LinkRef>`.
- Why: Pure trait so adapters (in-memory for tests, redb for prod) can vary.
- Risk: Low.
- Test: trait-contract harness in `adapters/test_support.rs`.

### Step 2: redb-backed adapter (`crates/itala-graph/src/adapters/redb.rs`)
- Action: Persist `(target_id, source_id, link_kind)` rows; range-scan by target.
- Risk: Medium — schema decisions persist.
- Test: round-trip in `tests/redb_link_index.rs` + the trait-contract harness.

### Step 3: Event-driven updates (`crates/itala-graph/src/events.rs`)
- Action: Subscribe to `note-saved`; diff old vs. new outgoing links; update rows.
- Risk: Medium — must be idempotent.
- Test: property test that any sequence of edits leaves the index consistent.

### Step 4: Tauri command (`apps/desktop/src-tauri/src/commands/graph.rs`)
- Action: `#[tauri::command] async fn backlinks(id: NoteId) -> Result<Vec<LinkRef>, AppError>`.
- Risk: Low.
- Test: integration test using a fixture vault.

### Step 5: Webview panel (`apps/desktop/ui/src/components/BacklinksPanel.tsx`)
- Action: Solid component; subscribe to `note-saved` event for live refresh.
- Risk: Low.
- Test: Vitest unit test with mocked `invoke`.

## Dependencies
- `itala-kernel` event bus already exists.
- `itala-graph` crate already scaffolded.

## Risks
- MEDIUM: redb schema choice — version-tag the row format from day one.
- LOW: Event ordering — `note-saved` is delivered after persistence completes.

## Success Criteria
- [ ] `cargo nextest run -p itala-graph` passes.
- [ ] `cargo clippy -p itala-graph -- -D warnings` passes.
- [ ] Backlinks panel updates within 200 ms of a save (per `docs/ARCHITECTURE.md` §4.1).
- [ ] Acceptance criteria from the sprint feature met.

**WAITING FOR CONFIRMATION**: Proceed with this plan? (yes/no/modify)

> Once you confirm, the agent updates the plan status to `confirmed` and stops. Run `/tdd [step]` yourself to begin implementation.
```

## Plan File

Every plan is saved to `.claude/plans/` **before** asking for confirmation so it survives a lost terminal. It is a **working document** — active only while the feature is in flight. When `/sprint-update <F#> completed` runs, the file is deleted automatically.

### Naming

| Situation | File name |
|---|---|
| Sprint feature in scope | `<feature-id>_<slug>.md` → `F3_kernel-vfs.md` |
| No sprint feature | `<YYYY-MM-DD>_<slug>.md` → `2026-05-24_backlinks-panel.md` |

Slug: 2–4 words from the task description, kebab-cased.

### Format

```markdown
---
id: F3_kernel-vfs
feature: F3          # omit if no sprint feature
date: 2026-05-24
status: draft        # draft | confirmed | in-progress
---

# Plan: <title>

## Requirements
…

## Steps
…

## Risks
…

## Success Criteria
- [ ] …
```

Status values while active: `draft` (awaiting confirmation) → `confirmed` (user approved) → `in-progress` (`/tdd` started). The `completed` transition is owned by `/sprint-update`, which deletes the file.

**To resume after a lost terminal:** `cat .claude/plans/<id>.md` and continue the pipeline from the recorded status.

---

## Important Notes

**CRITICAL**: The planner agent will **NOT** write any code — before or after confirmation. After you confirm, the agent updates the plan status to `confirmed`, outputs the prompt _"Plan confirmed. Run `/tdd [step]` to begin implementation."_, and stops. It does not invoke `/tdd` automatically.

If you want changes, respond with:
- "modify: [your changes]"
- "different approach: [alternative]"
- "skip phase 2 and do phase 3 first"

## Integration with Other Commands

After planning:
- **You** run `/tdd` to implement (the agent will not invoke it automatically)
- Use `/code-review` to review the diff after `just ci` passes
- Use `/pr-review` when raising the PR

> **Need deeper planning?** Use `/capability` to decompose a phase or feature
> into a capability plan before implementation planning. Use `/sprint-plan` to
> break a phase into structured sprints.

## Related

- `agents/planner.yaml` — The planner agent definition
- `/capability` — Capability planning (constraints, interfaces, open questions)
- `/sprint-plan` — Sprint decomposition (features, acceptance criteria, definition of done)
- `/tdd` — Test-driven development after planning
