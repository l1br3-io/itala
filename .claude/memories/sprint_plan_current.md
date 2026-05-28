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

#### F5.5. `itala-vault` — persistent `NoteId` via frontmatter `id:` field
- **Status:** Done (2026-05-28)
- **Description:** Note identity is now stable across renames. The `id:` YAML frontmatter field is authoritative; notes without it are lazily backfilled on first `list`/`path_of`/`read`. Duplicate id detection in `list` and `path_of`. `Frontmatter::emit` hand-rolls canonical YAML (always double-quotes strings). `inject_id` rewrites raw bytes atomically. Two new error variants: `InvalidFrontmatterId` and `DuplicateNoteId`.
- **Acceptance criteria:**
  - [x] `list` backfills `id:` into notes that lack it; subsequent reads return the same id.
  - [x] Renaming a file (copy + delete) does not change the `NoteId` once `id:` is in frontmatter.
  - [x] Two notes with identical `id:` return `Error::DuplicateNoteId` from `list` and `path_of`.
  - [x] Malformed `id:` (non-ULID string or non-string value) returns `Error::InvalidFrontmatterId`.
  - [x] Body bytes and checksums are preserved across backfill writes.
  - [x] 10 integration tests in `crates/itala-vault/tests/identity_persistence.rs` pass.
  - [x] `just ci` green.
- **Files:**
  - `crates/itala-vault/src/note/backfill.rs` (new)
  - `crates/itala-vault/src/frontmatter.rs` (added `insert`, `emit`)
  - `crates/itala-vault/src/note.rs` (added `id_from_frontmatter`)
  - `crates/itala-vault/src/vault.rs` (rewrote `list`, `path_of`, `read`; added `resolve_id_for`)
  - `crates/itala-vault/src/error.rs` (two new variants)
  - `crates/itala-vault/tests/identity_persistence.rs` (new)
- **Complexity:** Medium
- **Depends on:** F4

#### F5. `itala-vault` — local filesystem adapter
- **Status:** Done (2026-05-27)
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
- **Status:** Done (2026-05-28)
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

#### F12. `StorageProvider` seam + `FileProvider` + `NoteStore`-over-provider
- **Status:** not-started
- **Description:** Introduce the single **swappable storage seam** that the whole app will persist through. The product goal: a user picks one backend and notes, config, history, and plugin-state all flow through it; a backend author implements **one** adapter (e.g. MySQL) and everything works. Three layers:
  1. **Provider seam** — `StorageProvider` trait in `itala-kernel` (domain-agnostic "collections of records"), sitting alongside `Vfs`. CORE methods every backend implements: `get`/`put`/`delete`/`list` over `(collection, key)` + `capabilities()`. OPTIONAL methods guarded by `ProviderCapabilities` (host falls back when absent): `query(collection, &Filter)` and `transaction(Vec<WriteOp>)`. `Record { key: String, indexed: BTreeMap<String, FrontmatterValue>, blob: Bytes }` — `indexed` holds queryable fields a backend may index; `blob` is the opaque payload. Object-safe `DynStorageProvider` shadow (async-trait + blanket impl) mirroring `Vfs`/`DynVfs`, `Send` bound explicit. **Capability negotiation** is what keeps the contract tiny (anyone can write an adapter) while still allowing native DB queries (smart backends advertise `native_query`/`transactions`; the host uses the fast path).
  2. **`FileProvider`** (in `itala-vault` adapters, over `Arc<dyn DynVfs>`; `native_query=false` → host-side scan+filter). Relocates today's `vault.rs` note logic (`collect_md_paths`, `resolve_id_for`, frontmatter split, `id:` backfill, duplicate detection) **verbatim** — the regression contract.
  3. **`NoteStore`** — a single host-side domain impl in `itala-vault` over `Arc<dyn DynStorageProvider>` (NOT per-backend). Owns the Note↔Record mapping (`indexed`: tags+title; `blob`: frontmatter+body), adds `write`/`delete` (first mutating surface) + minimal `NoteQuery` (tag + title-substring). `Vault<Open>` delegates to `NoteStore`.
  WASM/plugin exposure of `StorageProvider` is explicitly **DEFERRED** until `itala-plugin-host` exists (see F15); the trait is internal (not `itala-plugin-api`, not under the semver freeze). Designed in the `/capability` discussion superseding `~/.claude/plans/as-a-product-owner-merry-pnueli.md`.
- **Acceptance criteria:**
  - [ ] `StorageProvider` in `itala-kernel` with CORE `get`/`put`/`delete`/`list` + `capabilities()`; OPTIONAL `query`/`transaction` guarded by `ProviderCapabilities`. References no domain types (kernel gains no vault dep).
  - [ ] `Record { key, indexed, blob }`, `Filter`, `ProviderCapabilities`, `WriteOp` defined; `DynStorageProvider` async-trait shadow + blanket impl; `Send` bound explicit; `assert_send_sync` + `assert_unpin` for `FileProvider` and `Arc<dyn DynStorageProvider>`.
  - [ ] `FileProvider` over `Arc<dyn DynVfs>`; `native_query=false`; host-side scan+filter fallback for `query`.
  - [ ] `NoteStore` (single impl) over `Arc<dyn DynStorageProvider>`; Note↔Record mapping round-trips frontmatter + body + checksum losslessly.
  - [ ] **Regression contract:** `tests/vault_lifecycle.rs` and `tests/identity_persistence.rs` pass **unchanged** against `NoteStore`-over-`FileProvider`; `Vault::<Closed>::new(Arc<dyn DynVfs>, root)` still compiles and works (wraps VFS→`FileProvider`→`NoteStore` internally).
  - [ ] `write` round-trip: write 10 notes → read each → frontmatter + body + checksum match. `delete`: write → delete → `read` returns `Error::NotFound`.
  - [ ] `NoteQuery` (tag + title-substring) returns correct sets via the host-side fallback path.
  - [ ] `fs_location_of` returns `Some` for `FileProvider`; documented `None` for non-FS providers.
  - [ ] `just ci` green (no new deps for the FS path).
- **Files:**
  - `crates/itala-kernel/src/store.rs` (new — `StorageProvider`, `Record`, `Filter`, `ProviderCapabilities`, `WriteOp`, `DynStorageProvider`)
  - `crates/itala-kernel/src/lib.rs` (re-exports)
  - `crates/itala-vault/src/note_store.rs` (new — `NoteStore`, `NoteQuery`, Note↔Record mapping)
  - `crates/itala-vault/src/adapters/file.rs` (new — `FileProvider`; absorbs today's `vault.rs` note logic)
  - `crates/itala-vault/src/vault.rs` (modify — delegate to `NoteStore`; `new` wraps VFS→`FileProvider`)
  - `crates/itala-vault/src/lib.rs` (re-exports)
- **Complexity:** High
- **Depends on:** F6, F5.5
- **Rules to load:** `rules/rust/traits-and-dispatch.md` (assoc-vs-generic, sealed, dyn-compat), `rules/rust/async.md` (Send-leak, async-trait shadow), `rules/rust/api-design.md`, `rules/rust/error-handling.md`.

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

#### F13. `PostgresProvider` (feature-gated)
- **Status:** not-started
- **Description:** Native Postgres implementation of **`StorageProvider`** (not a note-specific store). Located at `crates/itala-vault/src/adapters/postgres.rs` behind an optional `postgres` feature (off by default). Uses a **generic record schema** (supersedes the prior bespoke `note` table — accepted cost of "one adapter for everything"): a single `record` table keyed by `(collection, key)`, with `indexed JSONB` (+ GIN) and `blob BYTEA`. Advertises `native_query=true` + `transactions=true`. `sqlx` with `runtime-tokio` + `tls-rustls` (never `native-tls`/`openssl-sys`), the `time` feature (not `chrono`), connection string via `secrecy::SecretString` from `DATABASE_URL`; never logged. **Because every domain rides `StorageProvider`, `NoteStore` (and later `ConfigStore`/`HistoryStore`) work through Postgres with zero domain-store changes** — the backend swap is one config line. Real `created_at`/`modified_at` close the `UNIX_EPOCH` gap.
- **Schema (sqlx migration):**
  ```sql
  CREATE TABLE record (
    collection  TEXT  NOT NULL,
    key         TEXT  NOT NULL,
    indexed     JSONB NOT NULL DEFAULT '{}',
    blob        BYTEA NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (collection, key)
  );
  CREATE INDEX record_indexed_idx ON record USING GIN (indexed);
  ```
- **Acceptance criteria:**
  - [ ] `PostgresProvider` implements `StorageProvider` + `DynStorageProvider`; `capabilities()` advertises `native_query` + `transactions`.
  - [ ] `sqlx` with `runtime-tokio` + `tls-rustls`; `cargo deny check` passes after allowlist updates.
  - [ ] Migration creates the `record` table + `record_indexed_idx` GIN index.
  - [ ] `get`/`put`(upsert `ON CONFLICT`)/`delete`/`list` map to single statements; `put` sets `modified_at = now()`.
  - [ ] `NoteStore` over `PostgresProvider` passes the **same** round-trip suite as over `FileProvider` (write 100 → `list` → `read` → fields/checksums match), via `sqlx::test`/testcontainers.
  - [ ] `query` translates `Filter` to SQL over `indexed` JSONB (containment + ILIKE); returns correct sets for tag and title cases.
  - [ ] `transaction(Vec<WriteOp>)` commits atomically in one Pg transaction.
  - [ ] `created_at`/`modified_at` are real timestamps (not `UNIX_EPOCH`).
  - [ ] Provider errors surface through a backend-agnostic `itala-kernel` error variant (e.g. `Provider(Box<dyn Error + Send + Sync>)`) — **no `sqlx` dependency in `itala-kernel`**; `PostgresProvider` maps `sqlx::Error` into it. FS-specific `DuplicateNoteId` stays FS-scoped.
  - [ ] `assert_send_sync` for `PostgresProvider` and `Arc<dyn DynStorageProvider>` covering this impl.
  - [ ] `DATABASE_URL` via `SecretString` from env; never logged.
  - [ ] `just ci` green; `cargo nextest run -p itala-vault --features postgres` passes against testcontainers Postgres.
- **Files:**
  - `crates/itala-vault/src/adapters/postgres.rs` (new — behind `#[cfg(feature = "postgres")]`)
  - `crates/itala-vault/migrations/` (new — `sqlx` migration files)
  - `crates/itala-vault/Cargo.toml` (optional `postgres` feature + sqlx dep)
  - `crates/itala-kernel/src/error.rs` (new backend-agnostic `Provider` variant)
  - `deny.toml` (allowlist widening for sqlx/rustls tree)
- **Complexity:** High
- **Depends on:** F12

#### F14. `ConfigStore` + `HistoryStore` over the provider (sketch)
- **Status:** not-started
- **Description:** Move config (`itala.toml` settings) and version history onto `StorageProvider` so they ride the user's chosen backend automatically. `ConfigStore` keeps a singleton `load`/`save` shape (collection `"config"`); `HistoryStore` is a versioned append-log (collection `"history"`, key `"<note-id>:<version>"`). Each is host-side domain code over `Arc<dyn DynStorageProvider>` — **no per-backend impls**. This is the payoff of the seam: config/history work on Postgres for free once written, and proves the contract generalizes beyond notes (the test of whether the generic seam was the right shape).
- **Acceptance criteria sketch:**
  - [ ] `ConfigStore` round-trips settings through any provider; FS and Postgres give identical results.
  - [ ] `HistoryStore` append/list-versions/read-version/prune work over the provider; retention honored.
  - [ ] No new provider methods required — both fit CORE + optional `query`.
- **Depends on:** F12, F7 (history semantics)
- **Note:** Tracked, not pulled forward. Confirms the seam shape against a second and third domain.

#### F15. `StorageProvider` as a WASM plugin contract (sketch)
- **Status:** not-started
- **Description:** Promote the internal `StorageProvider` Rust trait to a WIT-defined plugin interface so third-party backends (e.g. MySQL, cloud) are user-installable plugins — the "just change this adapter" end-state. Capability negotiation (`native_query`/`transactions`) crosses the WASM boundary. Once exposed, this interface falls under the `itala-plugin-api` semver freeze.
- **Depends on:** F12, `itala-plugin-host` (separate phase)
- **Note:** Deferred until the plugin host exists; do **not** pull forward. The internal trait shape in F12 should anticipate this (kebab-case-able names, no host-only types in the contract).

#### F16. User-facing storage selection + bootstrap tier (sketch)
- **Status:** not-started
- **Description:** Let the user choose their backend (file / postgres / plugin) and switch it. Selection, connection secret (`SecretString`), and the provider plugin binary live in a **local primitive tier** (file/keyring) — they cannot live behind the provider they configure (the bootstrap loop). CLI/desktop surface to pick and migrate.
- **Depends on:** F12, F13
- **Note:** The bootstrap exception from the storage design — only the storage *selection* + provider *code* stay local; all domain data rides the chosen provider.

---

## Dependency graph

```
F1 ─┬─► F2 ─┬─► F3 ─┬─► F4 ─┬─► F5 ──► F6 ──► F12 (S2) ─┬─► F13 (S3, PostgresProvider)
    │       │       │       │                            ├─► F14 (S3, Config/History stores)
    │       │       │       └─► F7 (S2)                  ├─► F15 (WASM plugin contract — needs plugin-host)
    │       │       │                                    └─► F16 (storage selection + bootstrap)
    │       │       └─► F9 (S2)
    │       │
    │       └─► F8 (S2)
    │
    └─► (every other phase: Plugin Host, Search, Desktop, …)
```

Critical path: F1 → F2 → F3 → F4 → F5 → F6. Sprint 1 ships when F1–F6 are `completed`.
Storage backend path: F6 → **F12 (`StorageProvider` seam + `FileProvider` + `NoteStore`)** → { F13 PostgresProvider · F14 Config/History stores · F15 WASM plugin contract · F16 selection+bootstrap }. F12 is the one seam every domain and every backend rides.

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
