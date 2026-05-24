---
description: Explain the Itala development workflow — the gated pipeline from sprint planning to shipped feature, plus the discipline around it.
---

# Workflow Command

Read-only. Prints the Itala Claude workflow: the pipeline, what each stage produces, the source of truth, status rules, anti-patterns, and supporting skills. Use this when onboarding to the repo or when you've lost the thread on which stage you're in.

## What to output

When invoked, produce the explanation below verbatim (adapted only to the user's specific question, if any was passed in `$ARGUMENTS`). Do not start work; this command teaches the workflow, it does not execute it.

---

# The Itala Claude workflow

A gated pipeline from "vague phase" → "shipped feature." Each command has a narrow job; the discipline is to not skip stages.

```
/sprint-plan  →  /capability  →  /plan  →  /tdd  →  just ci  →  /sprint-update
   phases         features       steps     code     quality      persist
```

## The stages

**`/sprint-plan`** — decompose a phase or feature brief into sprints. Each sprint has features, acceptance criteria, and a definition of done. Output goes to `.claude/memories/sprint_plan_current.md`. That file is the source of truth.

**`/capability`** — pick one feature, nail down constraints, interfaces, and open questions *before* designing. This is where you discover that the feature is actually three features, or that the interface you assumed isn't viable.

**`/plan`** — produce a step-by-step implementation plan. **Wait for user CONFIRM before writing code.** No code without a confirmed plan.

**`/tdd`** — RED → GREEN → REFACTOR. Write a failing test, make it pass, clean up. Repeat per step in the plan.

**`just ci`** — build + tests + lint + typecheck must all pass. Green tests alone are not enough; acceptance criteria from the sprint plan must also be met.

**`/sprint-update`** — the **only** sanctioned way to mutate feature status. Never hand-edit `sprint_plan_current.md` for status.

## Source of truth

- `.claude/memories/sprint_plan_current.md` is authoritative. Read it via `/sprint-status` rather than opening the file.
- All status mutations go through `/sprint-update`. Hand-edits drift; the command does not.

## Status rules

- `in-progress` — set **before** starting code on a feature.
- `completed` — set **only** after `just ci` passes AND acceptance criteria are met.
- `blocked` — always requires a reason.
- `not-started` — use to walk back a premature status.

## Anti-patterns

- Coding without a confirmed plan.
- Marking complete on green tests alone.
- Skipping `just ci`.
- Editing the sprint plan by hand for status changes.
- Jumping straight to `/plan` on a fuzzy feature — run `/capability` first.

## Supporting commands and skills

- `/sprint-status` — read-only view of the current sprint plan.
- `/pr-review` — review pending changes or a GitHub PR.
- `/code-review` — focused correctness review of the current diff.
- `/run` — launch the app to confirm a change works in the real thing.
- `/init` — bootstrap a `CLAUDE.md` for a new area.

## Where the rules live

- Workflow rules: [.claude/rules/workflow.md](../rules/workflow.md)
- Code style (language-agnostic): [.claude/rules/code-style.md](../rules/code-style.md)
- Rust quick map: [CLAUDE.md](../../CLAUDE.md) — load only the `rules/rust/*.md` files that match the decision you're making.

## Quick start

If you have a fresh phase brief:
1. `/sprint-plan` — decompose into sprints.
2. `/sprint-status` — see the feature table.
3. `/capability F1` — design the first feature.
4. `/sprint-update F1 in-progress` — mark it before coding.
5. `/plan` — get a confirmed plan.
6. `/tdd` — implement.
7. `just ci` — gate.
8. `/sprint-update F1 completed` — close the loop.

If you're resuming work mid-flight, start at `/sprint-status` and pick up from whichever stage the active feature is in.
