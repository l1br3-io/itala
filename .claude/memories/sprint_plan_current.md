---
name: Current Sprint Plan
description: Sprint plan for Itala MVP — Foundation. Authoritative source for sprint state; mutate via /sprint-update.
type: project
---
# Sprint Plan: MVP — Foundation

> Decomposed from: "MVP — Foundation: Cargo workspace + kernel + vault context"
> Source phase: `docs/ROADMAP.md` → MVP-α (CLI) precursor.
> Created: 2026-05-21
> Phase: design → MVP foundation

## Goal

Stand up the Itala Cargo workspace, the `itala-kernel` crate (shared types, errors, event bus, VFS trait, capability tokens), the `itala-vault` crate (open/close with type-state, Markdown + YAML frontmatter, version history), and a thin `itala-cli` smoke binary that exercises every kernel + vault capability end-to-end. The exit condition is a `cargo run -p itala-cli -- vault open <path> && note list && note read <id> && note watch` flow that hits real filesystem I/O without panicking, while `just lint test deny doc` is green on every commit.

This sprint lays the runtime bedrock every later phase (Plugin Host, Search, Desktop, Sync, AI) depends on. Nothing else may start until this is done.

## Workflow contract

Pipeline (`.claude/rules/workflow.md`):

```
/sprint-plan  →  /capability  →  /plan  →  /tdd  →  just ci  →  /sprint-update
```

- Mark features `in-progress` BEFORE starting any code.
- Mark `completed` ONLY after `just ci` is green AND acceptance criteria are met.
- This file is the source of truth; mutate via `/sprint-update`, never hand-edit statuses.

---

## Features

### Sprint 1 — Must-Have

#### F1. Cargo workspace + standards gates
- **Status:** Done (2026-05-21)
- **Description:** Stand up the workspace skeleton and CI gates. Workspace `Cargo.toml` (no `[package]`, only `[workspace]`), `rust-toolchain.toml` (pinned channel, `rustfmt`/`clippy`/`llvm-tools-preview`, targets `wasm32-wasip2` + `x86_64-unknown-linux-gnu`), `rustfmt.toml`, `deny.toml` (license allowlist, banned-crate list), `.config/nextest.toml`, `justfile`, GitHub Actions pipeline (fmt → clippy → nextest → doctest → deny → doc broken-link).
- **Acceptance criteria:**
  - [ ] `cargo build --workspace` succeeds on a fresh checkout.
  - [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` succeeds.
  - [ ] `cargo nextest run --workspace` runs (zero crates exist yet, so passes trivially).
  - [ ] `cargo deny check` succeeds; `openssl-sys`, `chrono` are banned per `rules/rust/security.md`.
  - [ ] `cargo test --doc --workspace` runs.
  - [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` succeeds.
  - [ ] `just lint`, `just test`, `just deny`, `just doc` are wired.
  - [ ] Workspace `[workspace.lints.rust]` and `[workspace.lints.clippy]` are configured per `rules/rust/tooling.md`; each crate inherits via `lints.workspace = true`.
  - [ ] CI runs all six gates and blocks merge on failure.
- **Files:**
  - `Cargo.toml` (root)
  - `rust-toolchain.toml`
  - `rustfmt.toml`
  - `deny.toml`
  - `.config/nextest.toml`
  - `justfile`
  - `.github/workflows/ci.yml`
- **Complexity:** Low
- **Depends on:** none
- **Rules to load:** `rules/rust/workspace.md`, `rules/rust/tooling.md`, `rules/rust/security.md`.

#### F2. `itala-kernel` — IDs, errors, capability tokens
- **Status:** Done (2026-05-23)
- **Description:** Foundation types for the whole system. Newtype IDs (`NoteId(Ulid)`, `VaultId(Ulid)`, `LinkId(Ulid)`, `BlockId(SmolStr)`, `PluginId(SmolStr)`) with `Display`/`FromStr`. Crate-public `Error` enum with `thiserror`, `#[non_exhaustive]`, structured variants. `Capability` token type representing a granted permission. Crate root sets `#![forbid(unsafe_code)]` and `#![warn(missing_docs)]`.
- **Acceptance criteria:**
  - [ ] `#![forbid(unsafe_code)]` at crate root.
  - [ ] Every public type passes `assert_send_sync` and `assert_unpin` compile-time tests (`rules/rust/traits-and-dispatch.md`).
  - [ ] Proptest: every ID round-trips through `Display` ↔ `FromStr` for 1k random values.
  - [ ] `Error` is `#[non_exhaustive]` with at minimum: `NotFound { id }`, `Io(#[from] std::io::Error)`, `InvalidId(String)`, `VaultPathRelative { path }`.
  - [ ] Capability token holds an opaque `SmolStr` name and serializes to a stable string form.
  - [ ] Doctests on every public function (semver-frozen surface; `rules/rust/testing.md`).
  - [ ] `cargo nextest run -p itala-kernel` passes.
  - [ ] `cargo clippy -p itala-kernel -- -D warnings` passes.
- **Files:**
  - `crates/itala-kernel/Cargo.toml`
  - `crates/itala-kernel/src/lib.rs`
  - `crates/itala-kernel/src/error.rs`
  - `crates/itala-kernel/src/id.rs` (+ submodule files for each ID type if needed)
  - `crates/itala-kernel/src/capability.rs`
- **Complexity:** Medium
- **Depends on:** F1
- **Rules to load:** `rules/rust/workspace.md`, `rules/rust/api-design.md`, `rules/rust/error-handling.md`, `rules/rust/ownership-and-lifetimes.md`, `rules/rust/testing.md`.

#### F3. `itala-kernel` — VFS trait + event bus
- **Status:** Done (2026-05-26) — adopted pre-existing vfs.rs/events.rs under post-hoc review; added `clippy::await_holding_lock = deny` and EventBus doctests; just ci green
- **Description:** Two cross-cutting kernel ports. `Vfs` trait (`async fn` in trait, `Send + Sync`) with `read`, `write`, `list`, `watch` over `&Utf8Path`. Event bus on `tokio::sync::broadcast` with a typed `KernelEvent` enum (`NoteSaved`, `NoteDeleted`, `VaultOpened`, `VaultClosed`). `CancellationToken`-driven shutdown integrated.
- **Acceptance criteria:**
  - [ ] `Vfs` trait is object-safe via a generated `DynVfs` shadow using `async-trait` (per `rules/rust/async.md`), OR the kernel exposes only the native `async fn in trait` variant and documents the choice.
  - [ ] `KernelEvent` enum is `#[non_exhaustive]`, `Clone`, `Debug`.
  - [ ] Event-bus integration test: spawn two subscribers, publish 1k events, every subscriber sees all of them (no drops at bounded capacity ≥ 1024).
  - [ ] Cancellation test: `tokio::select!` between `token.cancelled()` and `bus.recv()` exits cleanly.
  - [ ] `Vfs::watch` returns a `Stream<Item = VfsEvent>` (per `rules/rust/async.md` § Streams).
  - [ ] Bounded channels only — no `unbounded_channel` anywhere.
  - [ ] No `std::sync::Mutex` held across `.await` (lint enforced via `clippy::await_holding_lock`).
- **Files:**
  - `crates/itala-kernel/src/vfs.rs`
  - `crates/itala-kernel/src/events.rs`
  - `crates/itala-kernel/src/lib.rs` (re-exports)
- **Complexity:** Medium
- **Depends on:** F2
- **Rules to load:** `rules/rust/async.md`, `rules/rust/traits-and-dispatch.md`, `rules/rust/api-design.md`.

#### F4. `itala-vault` — type-state vault + frontmatter
- **Status:** Done (2026-05-27)
- **Description:** Vault lifecycle as a typestate (`Vault<Closed>` → `Vault<Open>`). Note reading via `comrak` for Markdown and `serde_yaml` (or `yaml-rust2`) for YAML frontmatter. `Note` type holds `NoteId`, canonical `Utf8PathBuf`, `BTreeMap<String, serde_yaml::Value>` frontmatter, `Bytes` body, `blake3::Hash` checksum, `OffsetDateTime` timestamps (from `time`, not `chrono`).
- **Acceptance criteria:**
  - [ ] `Vault<Closed>::open(root: impl Into<Utf8PathBuf>)` returns `Result<Vault<Open>>`.
  - [ ] `Vault<Open>::read(NoteId)` returns `Result<Note>` with parsed frontmatter and decoded body.
  - [ ] `Vault::close` consumes `Vault<Open>` and returns `Vault<Closed>` (compile-time enforced).
  - [ ] Proptest: parse → re-serialize → parse round-trip preserves frontmatter and body bytes.
  - [ ] Insta snapshot tests on parsed AST for 5 representative fixtures (no frontmatter / minimal frontmatter / nested frontmatter / unicode / windows-line-endings).
  - [ ] 100-note fixture (each note ≈ 1 KB) opens in < 200 ms (preliminary, not budget-binding).
  - [ ] `#![forbid(unsafe_code)]`.
  - [ ] No `unwrap()` outside test modules; all errors flow through `vault::Error` (with `#[from] kernel::Error`).
- **Files:**
  - `crates/itala-vault/Cargo.toml`
  - `crates/itala-vault/src/lib.rs`
  - `crates/itala-vault/src/error.rs`
  - `crates/itala-vault/src/vault.rs` (typestate + builder via `bon`)
  - `crates/itala-vault/src/note.rs`
  - `crates/itala-vault/src/frontmatter.rs`
  - `crates/itala-vault/src/test_support.rs` (gated `#[cfg(any(test, feature = "test-support"))]`)
- **Complexity:** Medium-High
- **Depends on:** F2, F3
- **Rules to load:** `rules/rust/traits-and-dispatch.md` (typestate), `rules/rust/api-design.md`, `rules/rust/error-handling.md`, `rules/rust/testing.md`.

#### F5. `itala-vault` — local filesystem adapter
- **Status:** not-started
- **Description:** Concrete `Vfs` implementation against the local filesystem. Implemented as `crates/itala-vault/src/adapters/local.rs` per `rules/rust/workspace.md` ("Concrete adapter (filesystem, S3, OpenAI) → that context's `adapters/` dir"). Path safety: all paths are vault-root-relative `Utf8Path`s; absolute paths and `..` traversal are rejected. File watcher via `notify` crate, debounced through `notify-debouncer-mini`, surfaced as a `Stream<Item = VfsEvent>`.
- **Acceptance criteria:**
  - [ ] `LocalFsVfs::new(root: Utf8PathBuf)` constructs successfully when `root` exists.
  - [ ] Path-escape attempt (`vault.read("../../etc/passwd")`) returns `Error::VaultPathRelative` (or equivalent) — never reads outside root.
  - [ ] Round-trip test: write 100 random notes, list, read each, checksums match.
  - [ ] Watcher test: edit a file via `tokio::fs::write`, receive a `VfsEvent::Changed { path }` within 500 ms.
  - [ ] Concurrent reads: 32 simultaneous readers on the same note return identical bytes (no torn reads).
  - [ ] `cargo nextest run -p itala-vault` passes; `tempfile::TempDir` used everywhere (no leaks).
  - [ ] No blocking I/O on async runtime threads — verified via `tokio::task::spawn_blocking` discipline.
- **Files:**
  - `crates/itala-vault/src/adapters/mod.rs`
  - `crates/itala-vault/src/adapters/local.rs`
- **Complexity:** Medium
- **Depends on:** F3, F4
- **Rules to load:** `rules/rust/async.md`, `rules/rust/ownership-and-lifetimes.md`, `rules/rust/security.md` (path validation).

#### F6. `itala-cli` — smoke binary
- **Status:** not-started
- **Description:** Headless CLI that exercises kernel + vault end-to-end. Crate `crates/itala-cli` produces binary `itala` (set via `[[bin]] name = "itala"` per `rules/rust/workspace.md`). `clap` derive for argument parsing; `anyhow::Result` only at the `main()` edge. Commands: `vault open <path>`, `note list`, `note read <id>`, `note watch`. `tokio::main` lives here only.
- **Acceptance criteria:**
  - [ ] `itala vault open ./fixtures/sample-vault` opens and prints "opened vault <id>".
  - [ ] `itala note list` prints `<note-id>\t<path>` per note in the fixture.
  - [ ] `itala note read <id>` prints YAML frontmatter followed by `---` followed by the body.
  - [ ] `itala note watch` blocks and emits one log line per change event (verified via `tokio::process::Command` in an integration test).
  - [ ] Integration test in `crates/itala-cli/tests/end_to_end.rs` walks all four commands on a `tempfile::TempDir` vault.
  - [ ] `anyhow` does not leak into any library crate; only the binary edge uses it.
  - [ ] Exit code is non-zero on any error path; error message is a single line, no Rust backtrace by default.
- **Files:**
  - `crates/itala-cli/Cargo.toml`
  - `crates/itala-cli/src/main.rs`
  - `crates/itala-cli/src/commands/mod.rs`
  - `crates/itala-cli/src/commands/{vault.rs,note.rs}`
  - `crates/itala-cli/tests/end_to_end.rs`
  - `crates/itala-cli/tests/fixtures/sample-vault/` (5 notes with assorted frontmatter)
- **Complexity:** Medium
- **Depends on:** F2, F3, F4, F5
- **Rules to load:** `rules/rust/error-handling.md` (anyhow at edge), `rules/rust/async.md` (single `#[tokio::main]`), `rules/rust/api-design.md`.

---

### Sprint 2 — Should-Have

#### F7. Local version history
- **Status:** not-started
- **Description:** Per-note history stored as deltas (or full snapshots, capped) in `.itala/history/<note-id>/`. Configurable retention via `itala.toml`. Restore previous version through `Vault::restore(note_id, version)`.
- **Acceptance criteria:**
  - [ ] Writing a note 5 times produces 5 history entries (or N if retention < 5).
  - [ ] `Vault::history(note_id)` returns versions sorted newest-first with ULID-based version IDs.
  - [ ] `Vault::restore(note_id, version)` writes the historical bytes back as the current version (and pushes a new history entry — restores are themselves edits).
  - [ ] Diff API returns a unified diff (`similar` crate) between any two versions.
  - [ ] Retention test: write 100 versions with cap 10, oldest 90 are pruned.
- **Files:**
  - `crates/itala-vault/src/history.rs`
  - `crates/itala-vault/src/config.rs` (extends)
- **Complexity:** Medium-High
- **Depends on:** F4, F5

#### F8. Link parsing + in-memory graph
- **Status:** not-started
- **Description:** Parse wiki links (`[[Note]]`, `[[Note|Alias]]`, `[[Note#^block-id]]`, `![[Note]]`) from the Markdown AST. Build an in-memory `LinkGraph` with forward and backlink indexes. This lives in a thin sibling crate `crates/itala-graph` (not in vault) per `rules/rust/workspace.md`.
- **Acceptance criteria:**
  - [ ] Link parser extracts all four link forms from a 1k-line Markdown fixture with no false positives.
  - [ ] `LinkGraph::backlinks(NoteId)` returns the correct set for the fixture.
  - [ ] Proptest: parse → render-as-markdown → parse round-trips (links survive).
  - [ ] Insta snapshot tests for unlinked-mention detection over fixture set.
  - [ ] `LinkGraph` is `Send + Sync` and uses the "indeference" pattern (`Slab<Node>` + `LinkId(u32)`) per `rules/rust/ownership-and-lifetimes.md`.
- **Files:**
  - `crates/itala-graph/Cargo.toml`
  - `crates/itala-graph/src/lib.rs`
  - `crates/itala-graph/src/parser.rs`
  - `crates/itala-graph/src/graph.rs`
- **Complexity:** Medium-High
- **Depends on:** F2 (IDs), F4 (Note AST source)

#### F9. Performance baseline benchmarks
- **Status:** not-started
- **Description:** Criterion benchmarks against the §4.1 budgets so we have a baseline before adding more crates. Benches: open-10k-note-vault, read-10kb-note, parse-frontmatter, watch-100-files-burst.
- **Acceptance criteria:**
  - [ ] `cargo bench --workspace` produces criterion reports for each bench.
  - [ ] Synthetic 10k-note vault generator lives in `itala-vault::test_support`.
  - [ ] Open 10 KB note: median < 50 ms on the reference machine (recorded; budget per `docs/ARCHITECTURE.md` §4.1).
  - [ ] Kernel memory footprint measured via `procfs` / `mach`: < 20 MB with 10k notes loaded.
  - [ ] Bench results checked into `docs/benchmarks/baseline-<date>.md` (one-time; subsequent regressions are diffs).
- **Files:**
  - `crates/itala-kernel/benches/*`
  - `crates/itala-vault/benches/*`
  - `crates/itala-vault/src/test_support.rs` (extends with 10k generator)
- **Complexity:** Low-Medium
- **Depends on:** F4, F5

---

### Sprint 3 — Nice-to-Have (deferred — track but don't pull forward)

#### F10. CRDT body wiring (yrs)
- **Status:** not-started
- **Description:** Replace the `Bytes` body with a `yrs::Doc` whose `body` text and `frontmatter` map are CRDT-backed. Persistence as `body.yjs` next to the `.md` file; the `.md` is the canonical user-visible form, regenerated from the CRDT on save.
- **Acceptance criteria sketch:**
  - [ ] Open note → mutate body via Y-Doc API → close → reopen → mutations visible.
  - [ ] State-vector diff between two `Doc`s is binary-stable.
  - [ ] Proptest: apply N updates in any order, result is identical (commutativity).
- **Depends on:** F4
- **Note:** Likely lives in the editor / desktop phase since the keystroke→CRDT loop is editor-scoped. Listed here for awareness, not commitment.

#### F11. At-rest vault encryption (sketch)
- **Status:** not-started
- **Description:** VFS wrapper that XChaCha20-Poly1305-encrypts every read/write using a key derived from a passphrase via Argon2id (per `rules/rust/security.md` and `docs/ARCHITECTURE.md` §5).
- **Depends on:** F3, F5
- **Note:** Out of strict Foundation scope; tracked so the threat model isn't forgotten when designing the VFS trait surface.

---

## Dependency graph

```
F1 ─┬─► F2 ─┬─► F3 ─┬─► F4 ─┬─► F5 ──► F6
    │       │       │       │
    │       │       │       └─► F7 (S2)
    │       │       │
    │       │       └─► F9 (S2)
    │       │
    │       └─► F8 (S2)
    │
    └─► (every other phase: Plugin Host, Search, Desktop, …)
```

Critical path: F1 → F2 → F3 → F4 → F5 → F6. Sprint 1 ships when F1–F6 are `completed`.

---

## Definition of done

Sprint is **done** when every Must-Have feature (F1–F6) satisfies all acceptance criteria AND the following workspace-wide gates pass on `main`:

- [ ] `just lint` green (`cargo fmt --check` + `cargo clippy -- -D warnings`).
- [ ] `just test` green (`cargo nextest run --workspace` + `cargo test --doc --workspace`).
- [ ] `just deny` green (`cargo deny check` + `cargo audit`).
- [ ] `just doc` green (`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`).
- [ ] `cargo mutants --package itala-kernel` reports ≥ 70% mutation score (per MVP shipping criteria in `docs/ROADMAP.md`).
- [ ] `cargo +nightly miri test -p itala-kernel -p itala-vault` is green (no `unsafe` yet, so this should be trivially green — confirms `#![forbid(unsafe_code)]` is holding).
- [ ] End-to-end CLI run on `crates/itala-cli/tests/fixtures/sample-vault/` succeeds: open → list → read → watch one edit.
- [ ] Performance bench output recorded in `docs/benchmarks/baseline-<date>.md` (from F9).
- [ ] Phase-status memory updated: `MVP — Foundation` → `Done`; `MVP — CLI` and `MVP — Plugin Host` → `Ready to start`.

## Out of scope (this sprint)

- WIT plugin contracts and `itala-plugin-host` — separate phase (MVP — Plugin Host).
- Tantivy search index — separate phase (MVP — Search).
- Tauri desktop shell — separate phase (MVP — Desktop), and per `docs/ROADMAP.md` it does not start until MVP-α exit is met.
- Sync, AI, encryption, theming, command palette, plugin SDK — all post-Foundation.

## Risks & open questions

- **VFS object-safety call.** `async fn in trait` vs `async-trait` for the `dyn Vfs` slot. Decide in F3's `/capability` step; not blocking F2.
- **Frontmatter representation.** `serde_yaml` is unmaintained; `yaml-rust2` is the candidate. Lock in during F4's `/capability` step.
- **File watcher cross-platform parity.** `notify` debouncing on macOS FSEvents vs Linux inotify vs Windows ReadDirectoryChangesW. F5 acceptance test must run in CI on all three.
- **Mutation testing cost.** `cargo-mutants` may not be installable in CI by F6; if so, move that gate to a nightly cron and add a sprint-2 follow-up task.
