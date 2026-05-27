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

## Who does what

The agent writes tests and code. **You run every command.** The agent never executes `cargo`, `just`, or any shell command — its only actions are writing files and outputting prompts for you to execute. Every execution step is a hard stop: the agent outputs the command, waits for your report, then continues.

## The cycle (per behavior)

1. **RED — agent writes the test.** Write a failing test against the public API of the step's target.
   - Unit tests colocate (`#[cfg(test)] mod tests`); integration tests live in `tests/`; snapshots via `insta`; properties via `proptest`. See [`rules/rust/testing.md`](../rules/rust/testing.md).
   - Name the test after the behavior (`parses_yaml_frontmatter`), not the function.

2. **RED — you run.** The agent stops and outputs:
   > Run `just test` (or narrow it: `cargo nextest run -p itala-<crate> <test_name>`) and report back. It should fail. If it passes unexpectedly, tell me — I'll fix the test before writing any implementation code.

3. **GREEN — agent writes the implementation.** After you confirm red, write the minimum code that turns the test green. No anticipated edge cases, no speculative branches.

4. **GREEN — you run.** The agent stops and outputs:
   > Run `just test` (same scope as before) and report back. It should now pass.

5. **REFACTOR — agent cleans up.** After you confirm green: extract helpers, tighten types, remove duplication. After each non-trivial change, the agent stops and outputs:
   > Run `just test` again to confirm still green, then report back.

6. Repeat for the next behavior in the same step (edge cases, error variants, properties).

## Hard rules

- **The agent never runs commands.** No `cargo`, `just`, or any shell invocation — ever. Output the command; wait for the user's report.
- **No implementation before its test exists and the user reports it failing.** If the agent writes ahead of a confirmed-red test, delete the code, write the test, wait for red confirmation.
- **Never edit a test to fit existing code.** If the code is right and the test is wrong, the test was lying — replace it.
- **Never advance a step until the user reports `just test` green.** "It should work" is not a user-reported green.
- **No `unwrap()` / `expect("")` outside tests** (see [`rules/rust/error-handling.md`](../rules/rust/error-handling.md)).

## Exit criteria

- Every plan step is implemented behind a passing test.
- The user has reported `just test` green across the workspace (not just the focused crate).
- `.claude/plans/<id>.md` status is at `in-progress` (flip via `/sprint-update` if not).

## After exiting

When all steps are complete and you have reported `just test` green, the agent outputs:
> Run `just ci` — the full gate (`fmt` → `clippy` → `nextest` → `deny` → `doc`) — and report back. Only after you report a passing `just ci` may you call `/sprint-update <F#> completed`. Local green is not the same as a green CI gate.

## Pipeline position

```
/plan (confirmed)  →  /tdd  →  just ci  →  /sprint-update <F#> completed
```

## Related

- [`rules/rust/testing.md`](../rules/rust/testing.md) — test layout, `proptest`, `insta`, fixtures, what not to test.
- [`/plan`](plan.md) — must be confirmed before `/tdd` runs.
- [`/sprint-update`](sprint-update.md) — only sanctioned status mutation after `just ci`.
