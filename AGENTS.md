# Itala

<!--
  This is the always-loaded map for AI coding agents working in this
  repository. It follows the agents.md standard and the OpenCode Agent
  Harness skill-driven model. /adopt fills the <PLACEHOLDER>s; downstream
  projects derive their AGENTS.md from this file.
-->

This file orients AI coding agents working in this repository. It is the
always-loaded **map**: project specifics plus the skill-driven workflow this
project uses. Detail lives on-demand in the skills and rules — follow the
pointers, don't try to hold it all at once.

## Project Overview

Itala is a cross-platform, extensible knowledge management system for notes,
links, and ideas. The core is a thin orchestration **kernel** that provides
shared primitives; every capability — editing, search, sync, AI, PDF handling —
is a plugin that composes the full experience. The codebase is a Rust workspace
(`crates/*` for kernel, vault, note, storage, plugin api/host, vfs; `apps/*`
for binaries like `itala-cli`). The project is in active design, pre-release.

## Commands

The canonical commands are driven by [`just`](https://github.com/casey/just):

```sh
just          # lint + test (default)
just fmt      # cargo fmt --all
just lint     # fmt --check + clippy -D warnings (workspace, all targets/features)
just test     # cargo nextest --workspace + doctests
just deny     # cargo deny check (license + supply-chain)
just doc      # RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
just ci       # lint + test + deny + doc
```

The project uses the OpenCode Agent Harness — for the RED→GREEN test loop and
quiet-verification defaults, see
`.opencode/harness/rules/verification-commands.md`.

## Tech

Declared in `.opencode/harness/rules/tech.md` (injected into every session as a
lazy-load router). Each entry points at `.opencode/harness/tech/<name>/` — Read
the matching folder before editing that stack's code.

Active stack: **rust**.

## Skill-Driven Execution

This project runs on a **skill-driven model**: if a task matches a skill, use
it. Don't implement directly when a skill applies; follow its workflow exactly
— no partial application.

Skill locations (one canonical copy, symlinked per agent — mirrors the
[addyosmani/agent-skills](https://github.com/addyosmani/agent-skills) setup;
`skills-lock.json` pins the upstream sources):

| Asset | Path |
|-------|------|
| Canonical skills | `.agents/skills/<name>/SKILL.md` |
| Shared checklists | `.agents/references/*.md` |
| opencode (harness-adapted) | `.opencode/skills/<name>/SKILL.md` |
| ZCode / Claude Code | `.zcode/skills/<name>`, `.claude/skills/<name>` → symlinks into `.agents/skills/` |

### Intent → Skill

| If the task is… | Use this skill |
| --- | --- |
| New feature / functionality | `spec-driven-development` → `incremental-implementation` + `test-driven-development` |
| Planning / breakdown | `planning-and-task-breakdown` |
| Bug / failure / unexpected behavior | `debugging-and-error-recovery` |
| Code review | `code-review-and-quality` |
| Refactor / simplification | `code-simplification` |
| API / interface design | `api-and-interface-design` |
| Rust code / review | `rust-best-practices` |
| UI / frontend | `frontend-ui-engineering` |
| Security hardening | `security-and-hardening` |
| Docs / decisions | `documentation-and-adrs` |

More skills exist — reach for the matching one before inventing a process.

### Lifecycle

The workflow maps to slash commands (this harness ships `.opencode/commands/`,
mirrored to `.zcode/commands/` for ZCode) and the skills behind them:

```
DEFINE → /spec        spec-driven-development
PLAN   → /plan        planning-and-task-breakdown
BUILD  → /build       incremental-implementation + test-driven-development
VERIFY → /test        debugging-and-error-recovery
REVIEW → /review      code-review-and-quality
SHIP   → /ship        shipping-and-launch
```

In ZCode the planning command is `/planning` — `/plan` is a reserved
built-in there, so its mirror is renamed to `planning.md`; opencode keeps
`/plan`.

### Anti-rationalization

These thoughts are wrong — ignore them:

- "This is too small for a skill."
- "I can just quickly implement this."
- "I'll gather context first."

Correct behavior: always check for and use a skill first, even for small tasks.

## Orchestration

Three composable layers — different jobs, don't confuse them:

- **Skills** (`.opencode/skills/<name>/SKILL.md`) — workflows with steps and exit criteria. The *how*. Mandatory when an intent matches.
- **Agents** (`.opencode/agents/<role>.md`) — specialist subagents with a perspective and output format. The *who*. opencode exposes each as a `mode: subagent` tool.
- **Commands** (`.opencode/commands/*.md`) — slash entry points. The *when*. The orchestration layer.

Composition rule: **the user (or a command) is the orchestrator. Agents do not invoke other agents.** An agent may invoke skills. The only multi-agent pattern this harness endorses is **parallel fan-out with a merge step** — used by `/ship`. See `.opencode/harness/rules/orchestration-patterns.md`.

## Spec & task artifacts (local working docs)

- `SPEC.md` is the master index: the capability map (module ids, dependency
  direction, build order) and per-module status. Per-module specs live at
  `SPEC-<module-id>.md`; per-module task chunks at `tasks/<module-id>.md`.
  Global aggregates: `tasks/plan.md` and `tasks/todo.md`.
- All of the above are local working artifacts, gitignored via `SPEC*.md`
  and `/tasks`. Never commit them.
- Module ids come from the capability map in `SPEC.md`, are chosen once, and
  are never renamed mid-initiative.
- Before implementing: read `SPEC.md` for orientation, then load the
  module's `SPEC-<id>.md` and `tasks/<id>.md` chunk. Never implement a
  module whose spec is not approved or whose chunk is not written.
- `/build` consumes the chunk files; see `.opencode/commands/build.md`
  (mirrored at `.zcode/commands/build.md`).
- PRs must copy the module's success criteria into the PR body, since
  reviewers cannot see the local spec files.

## Boundaries

<!-- TODO(refine): replace these defaults with project-specific guidance.
     What follows is a starter set inferred from the workspace lints and
     README; tighten to match Itala's real constraints. -->

Always do:
- `cargo fmt --all` before commit; clippy must stay warning-free (`-D warnings`).
- Public items require docs — the workspace sets `missing_docs = "warn"`.
- Write tests alongside code (`crates/*/tests`, doctests); follow `testing-patterns.md`.

Ask first:
- Adding a new workspace member crate or moving code across crate boundaries.
- Introducing a new plugin host boundary or changing the kernel's public API.
- Changing `rust-toolchain.toml` or bumping the MSRV (`rust-version`).

Never do:
- `unsafe` code — the workspace sets `unsafe_code = "forbid"`.
- Hold locks across `.await` — the workspace sets `await_holding_lock = "deny"`.
- Commit `target/`, or anything under `node_modules/` aside from the harness's own.

Harness defaults (apply unless the project overrides):

- Rules are load-on-demand — only `.opencode/harness/rules/tech.md` sits in every session. Keep `.opencode.jsonc` → `instructions` lean.
- Gotchas go in the code next to the trap (`// GOTCHA`), never accumulated here. See `documentation-and-adrs`.
- Restart opencode after any config change — config loads once at startup.
