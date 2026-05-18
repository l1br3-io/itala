---
paths:
  - "apps/**/*.ts"
  - "apps/**/*.tsx"
  - "apps/**/*.js"
  - "apps/**/*.jsx"
---
# TypeScript Tooling Hooks

> "Hooks" here means Claude Code automation, not framework hooks.

## PostToolUse hooks

Configured via `.claude/hooks/post-edit-format.js` (project-level) or `~/.claude/settings.json` (global):

- **Formatter on save**: Biome or Prettier auto-format `.ts` / `.tsx` / `.js` / `.jsx` files after edit. Detected by the presence of a project config.
- **TypeScript check**: optional — run `tsc --noEmit` after edits to catch type errors immediately.
- **`console.log` warning**: flag any committed `console.log` in edited files.

## Pre-commit hook

`.claude/hooks/pre-commit-quality.js`:

- Detects staged `.ts`, `.tsx`, `.js`, `.jsx` files via `git diff --cached --name-only`.
- Runs ESLint or Biome on staged files (whichever is configured).
- Scans for `console.log`, `debugger`, hardcoded secrets, and TODOs without an issue reference.
- Validates commit messages against the conventional-commits format.

## Stop hooks

- **`console.log` audit**: at session end, sweep all modified files for stragglers and report.

## What not to put in hooks

- Long-running tasks. Keep hooks under a few seconds — anything heavier belongs in CI.
- Tests. Hooks shouldn't run the test suite on every edit; that's `pnpm test` or `cargo nextest run` on demand.
- Network operations. Hooks should be deterministic and offline-safe.

## Adding a new hook

1. Add the script to `.claude/hooks/` following the existing module shape (`run(rawInput)` exported, stdin entry at the bottom).
2. Wire it via `.claude/settings.local.json` or `~/.claude/settings.json` under the relevant event.
3. Make the script fail-soft — print a warning, never block the user's workflow on a missing tool.
