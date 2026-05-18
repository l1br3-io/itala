# CLAUDE.md

Project entry point for AI coding agents.

## What Itala is

A cross-platform, extensible knowledge management system where every capability (storage, editing, search, sync, AI, export) is a plugin. The kernel is thin; plugins compose the experience. See [docs/VISION.md](docs/VISION.md) for the full stance.

## Current state

Active design. No runtime code yet. The work in this phase is shaping product scope, architecture, and the plugin contract.

## Where to look

When working on a task, load the docs that match the decision you're making. Each doc is a focused context chunk — don't load more than you need.

- **Principles and design orientation** → [docs/VISION.md](docs/VISION.md)
- **Personas and user stories** → [docs/PRODUCT.md](docs/PRODUCT.md)
- **MVP scope and phased delivery** → [docs/ROADMAP.md](docs/ROADMAP.md)
- **Kernel, plugin system, data model, security, accessibility** → [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Rust quick map

When a task touches Rust, load the rule(s) for the decision you're making — not the whole `rules/rust/` directory.

| Task | Load these rules |
|---|---|
| Designing an error enum | [rust/error-handling.md](.claude/rules/rust/error-handling.md) |
| Writing a new bounded-context crate | [rust/workspace.md](.claude/rules/rust/workspace.md), [rust/api-design.md](.claude/rules/rust/api-design.md), [rust/ownership-and-lifetimes.md](.claude/rules/rust/ownership-and-lifetimes.md) |
| Designing a public trait | [rust/api-design.md](.claude/rules/rust/api-design.md), [rust/traits-and-dispatch.md](.claude/rules/rust/traits-and-dispatch.md) |
| Writing async code | [rust/async.md](.claude/rules/rust/async.md), [rust/ownership-and-lifetimes.md](.claude/rules/rust/ownership-and-lifetimes.md) |
| Touching `unsafe` or FFI | [rust/unsafe-and-ffi.md](.claude/rules/rust/unsafe-and-ffi.md), [rust/security.md](.claude/rules/rust/security.md) |
| Authoring a WASM plugin or WIT interface | [rust/wasm-plugins.md](.claude/rules/rust/wasm-plugins.md), [rust/traits-and-dispatch.md](.claude/rules/rust/traits-and-dispatch.md) |
| Writing tests | [rust/testing.md](.claude/rules/rust/testing.md) |
| Toolchain or CI questions | [rust/tooling.md](.claude/rules/rust/tooling.md) |

## Writing docs

- One topic per file. If a doc grows to cover two unrelated concerns, split it.
- Keep stable content (principles, personas) separate from dynamic content (roadmap, stories) so diffs stay clean and retrieval stays focused.
- Link between docs rather than duplicating content.
- Don't add placeholder sections with no content — add the content when the need is concrete.
