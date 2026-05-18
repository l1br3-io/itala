---
description: Update a feature's status in the current sprint plan so /sprint-status stays accurate.
---

# Sprint Update

Persist a feature status change into the sprint plan memory. This is the "close-the-loop" step of the planning flow — the one that keeps `sprint_plan_current.md` authoritative after work ships.

## Usage

```
/sprint-update <feature-id> <status> [note]
```

- `feature-id` — feature marker from the plan (e.g., `F1`, `F2`, `F7`)
- `status` — one of: `not-started` | `in-progress` | `completed` | `blocked`
- `note` (optional) — short free-text appended to the status line

## What This Command Does

1. Read `.claude/memories/sprint_plan_current.md` from the project root.
2. Locate the section heading `### F<N>.` matching the feature id (case-insensitive on the letter).
3. Rewrite the feature's `- **Status**: ...` line based on the new status:
   - `completed` → `Done (YYYY-MM-DD)` using today's absolute date, plus the note if given.
   - `in-progress` → `In progress` (append `— <note>` when a note is provided).
   - `blocked` → `Blocked — <note>` (note is **required** for blocked).
   - `not-started` → `Not started` (resets prior status).
4. If the flip completes every feature inside a sprint (Sprint 1, 2, or 3), **ask** the user whether to bump the relevant phase completion % in the top "Status Summary" table. Do not edit the table silently.
5. Write the file back and echo a concise before/after delta (old status line → new status line) so the user can confirm.

## Pre-flight Checks

- If the feature id is not found, list the available feature ids from the plan and stop.
- If the status argument isn't one of the four allowed values, reject and show the valid set.
- If `blocked` is requested without a note, refuse and ask for the blocker reason.

## When to Use

- **After `/verify` passes** on a sprint feature's acceptance criteria → flip to `completed`.
- When starting work on a feature → flip to `in-progress` (so `/sprint-status` shows accurate "next up").
- When a dependency or external decision halts progress → flip to `blocked` with a reason.
- When walking back a premature completion → flip back to `in-progress` or `not-started`.

Do **not** mark `completed` just because tests pass — confirm the feature's acceptance criteria in the plan are all met first.

## Flow

```
/tdd  →  /verify  →  /sprint-update <F#> completed  →  /sprint-status
```

## Examples

```
/sprint-update F2 completed redb-backed link index with proptest convergence
/sprint-update F5 in-progress wiring backlinks panel into the desktop shell
/sprint-update F9 blocked waiting on plugin-api WIT version decision
```

## Related

- `/sprint-status` — Read the current sprint plan (no writes)
- `/sprint-plan` — Create or re-decompose a phase into sprints
- `/verify` — Verification loop that should precede `completed`
