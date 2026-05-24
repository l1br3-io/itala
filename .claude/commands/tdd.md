---
description: Drive implementation via RED → GREEN → REFACTOR. Invoke after a confirmed /plan; do not write code outside this cycle.
---

# TDD Command

Gate 1 of the [Itala workflow](../rules/workflow.md): once `/plan` is confirmed, all implementation code is written inside `/tdd`. The skill enforces order — a failing test exists and is observed red **before** any production code is written.

## Usage

```
/tdd [step-or-description]
```

If `$ARGUMENTS` names a step from the active plan in `.claude/plans/<id>.md`, scope the cycle to that step. Otherwise, walk the plan's steps in order.

## The cycle (per behavior)

1. **RED** — Write a failing test against the public API of the step's target.
   - Unit tests colocate (`#[cfg(test)] mod tests`); integration tests live in `tests/`; snapshots via `insta`; properties via `proptest`. See [`rules/rust/testing.md`](../rules/rust/testing.md).
   - Name the test after the behavior (`parses_yaml_frontmatter`), not the function.
2. **Run** `just test`, or scope it: `cargo nextest run -p itala-<crate> <test_name>`. **Observe red.** If the test passes accidentally, it doesn't exercise the new behavior yet — fix the test before writing code.
3. **GREEN** — Write the minimum code that turns the test green. No anticipated edge cases, no speculative branches.
4. **Run** the test again. **Observe green.**
5. **REFACTOR** — Clean up while green: extract helpers, tighten types, remove duplication. Re-run after each non-trivial change.
6. Repeat for the next behavior in the same step (edge cases, error variants, properties).

## Hard rules

- **No implementation before its test exists and fails.** If you catch yourself coding ahead, delete the code, write the test, re-derive.
- **Never edit a test to fit existing code.** If the code is right and the test is wrong, the test was lying — replace it.
- **Never claim a step is done without observing `just test` green in this session.** "It should work" is not green.
- **No `unwrap()` / `expect("")` outside tests** (see [`rules/rust/error-handling.md`](../rules/rust/error-handling.md)).

## Exit criteria

- Every plan step is implemented behind a passing test.
- `just test` is green across the workspace, not just the focused crate.
- `.claude/plans/<id>.md` status is at `in-progress` (flip via `/sprint-update` if not).

## After exiting

Run `just ci` — the full gate (`fmt` → `clippy` → `nextest` → `deny` → `doc`). Only then may you call `/sprint-update <F#> completed`. Local green is not the same as a green CI gate.

## Pipeline position

```
/plan (confirmed)  →  /tdd  →  just ci  →  /sprint-update <F#> completed
```

## Related

- [`rules/rust/testing.md`](../rules/rust/testing.md) — test layout, `proptest`, `insta`, fixtures, what not to test.
- [`/plan`](plan.md) — must be confirmed before `/tdd` runs.
- [`/sprint-update`](sprint-update.md) — only sanctioned status mutation after `just ci`.
