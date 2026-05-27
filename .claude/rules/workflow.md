# Development Workflow

## Pipeline

```
/sprint-plan  →  /capability  →  /plan  →  /tdd  →  just ci  →  /sprint-update
  (phases)       (features)     (steps)    (code)   (quality)    (persist status)
```

## Source of Truth

- The sprint plan is authoritative; check it before starting any work (`/sprint-status`)
- `/sprint-update` is the ONLY sanctioned way to mutate feature statuses
- Do not hand-edit `sprint_plan_current.md` for status changes
- Every `/plan` output is saved to `.claude/plans/<id>.md` before confirmation — check there to resume after a lost terminal; deleted automatically when the feature is marked `completed`

## Status Rules

- `in-progress`: mark BEFORE starting any coding on a feature
- `completed`: mark ONLY after acceptance criteria are met (not just green tests)
- `blocked`: always requires a reason
- `not-started`: use to walk back a premature status

## Gated Process

> The two hard gates (Gate 1: no code before `/tdd`; Gate 2: no `completed` before `just ci`) are stated as behavioral imperatives in `CLAUDE.md`, which is always loaded. This file provides the full pipeline detail.

1. `/capability` — nail down constraints and interfaces before coding
2. `/plan` — create implementation plan; saved to `.claude/plans/<id>.md`; WAIT for user confirmation before writing code
3. `/tdd` — RED-GREEN-REFACTOR cycle during implementation
4. `just ci` — build + test + lint + typecheck must all pass
5. `/sprint-update` — persist status after `just ci` passes AND acceptance criteria are met

Each gate is **mandatory and ordered**. A gate may not be skipped even if prior work seems complete.

## Hard Gates — Never Cross Without Running the Gate

| Gate | Blocks | Rule |
|---|---|---|
| User confirms `/plan` | Writing any implementation code | Stop. Prompt the user to run `/tdd` manually. Do not invoke it automatically. |
| `/tdd` session complete | Running the full quality check | Run `just ci` first. |
| `just ci` passes | Marking a feature done | Invoke `/sprint-update <F#> completed` — no other path. |

If you find yourself writing implementation code, stop immediately and prompt the user to run `/tdd`.
If you find yourself about to call `/sprint-update completed` without a passing `just ci`, stop and run it first.

## Anti-Patterns

- **Never write implementation code after a confirmed `/plan`** — stop, prompt the user to run `/tdd`, and wait. Do not invoke `/tdd` automatically. This is the most common bypass (it happened on F2).
- Never mark complete on green tests alone — verify acceptance criteria
- **Never call `/sprint-update <F#> completed` without a prior passing `just ci`** — tests passing locally is not the same as the full CI gate
- Never code without a confirmed plan
- Never edit sprint plan statuses outside of `/sprint-update`
