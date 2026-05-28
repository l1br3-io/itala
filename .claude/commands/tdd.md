---
description: Drive implementation via tests-first then implementation. Invoke after a confirmed /plan; do not write code outside this cycle.
---

# TDD Command

Gate 1 of the [Itala workflow](../rules/workflow.md): once `/plan` is confirmed, all implementation code is written inside `/tdd`. The agent writes tests and code; **you run every command**.

## Usage

```
/tdd [step-or-description]
```

If `$ARGUMENTS` names a step from the active plan in `.claude/plans/<id>.md`, scope the cycle to that step only. Otherwise, implement all steps in order.

## Mode selection

On every `/tdd` invocation, before writing any code, ask the user:

> **Which TDD mode?**
>
> **A — Batch** *(default)*: agent writes all tests, then all implementation in one pass; you run `just test` once at the end.
>
> **B — Per-spec**: agent writes one test at a time; you run after each write and report RED/GREEN before the next spec begins.

Wait for the user to choose. Do not proceed until a mode is selected.

---

## Mode A — Batch

Agent writes everything for the full scope in one pass; you run two commands total.

### Who does what

The agent writes tests and code. **You run every command.** The agent never executes `cargo`, `just`, or any shell command — its only actions are writing files and outputting commands for you to run.

### The cycle

1. **Write all tests.** For every plan step in scope, write tests covering the behavior, error variants, and edge cases against the public API.
   - Unit tests colocate (`#[cfg(test)] mod tests`); integration tests in `tests/`; snapshots via `insta`; properties via `proptest`. See [`rules/rust/testing.md`](../rules/rust/testing.md).
   - Name tests after the behavior (`parses_yaml_frontmatter`), not the function.

2. **Write all implementation.** After all tests are written, implement every plan step — minimum code that makes the tests pass. No speculative branches or anticipated edge cases beyond what the tests require.

3. **Refactor.** Clean up inline — extract helpers, tighten types, remove duplication. No confirmation needed.

4. **You run once.**
   > Run `just test` (or narrow: `cargo nextest run -p itala-<crate>`) and report back.

5. **Fix if red.** On failure, analyze all failures, fix everything identifiable in one pass, then ask for another run. Repeat until green.

6. **CI gate.** Once `just test` is green:
   > Run `just ci` — the full gate (`fmt` → `clippy` → `nextest` → `deny` → `doc`) — and report back. Only after a passing `just ci` may you call `/sprint-update <F#> completed`.

---

## Mode B — Per-spec

One RED-GREEN-REFACTOR loop per spec. You run after every write and report the result before the agent continues.

### Who does what

Same as Batch: the agent writes files and outputs commands; **you run every command and report back**.

### The cycle (repeat for each spec)

1. **Agent writes one test** for the next spec — covers the behavior, one error variant, or one edge case. Outputs:
   > Run `just test` (or `cargo nextest run -p itala-<crate>`) and report back.

2. **You run and report RED.** The agent does not write implementation until you confirm the test fails. If you report green without implementation, the agent stops and diagnoses why (stale snapshot, wrong test target, etc.).

3. **Agent writes minimum implementation** — only enough to make that one test pass.
   > Run `just test` and report back.

4. **You run and report GREEN.** The agent does not move to the next spec until you confirm passing.

5. **Agent refactors** if warranted (inline — no new functionality). Then loops back to step 1 for the next spec.

6. **CI gate.** After all specs are green:
   > Run `just ci` — the full gate (`fmt` → `clippy` → `nextest` → `deny` → `doc`) — and report back. Only after a passing `just ci` may you call `/sprint-update <F#> completed`.

---

## Hard rules (both modes)

- **The agent never runs commands.** No `cargo`, `just`, or any shell invocation.
- **Tests are written before implementation** — always, in both modes.
- **Never edit a test to fit existing code.** If a test is wrong, replace it with a correct one.
- **No `unwrap()` / `expect("")` outside tests** — see [`rules/rust/error-handling.md`](../rules/rust/error-handling.md).
- **Mode B only:** never advance from RED to GREEN without a user-reported failing test; never advance past GREEN without a user-reported passing test.

## Exit criteria

- Every plan step is implemented behind passing tests.
- You have reported `just test` green across the workspace.
- You have reported `just ci` passing before `/sprint-update <F#> completed` is called.

## Pipeline position

```
/plan (confirmed)  →  /tdd  →  just ci  →  /sprint-update <F#> completed
```

## Related

- [`rules/rust/testing.md`](../rules/rust/testing.md) — test layout, `proptest`, `insta`, fixtures, what not to test.
- [`/plan`](plan.md) — must be confirmed before `/tdd` runs.
- [`/sprint-update`](sprint-update.md) — only sanctioned status mutation after `just ci`.
