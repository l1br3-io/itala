# F5: CLI `init`, `notes create`, `notes list` commands

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Checkpoint A: CLI green for init / create / list

**Description:** `apps/itala-cli` gains three subcommands. `init` takes an
optional path (default `.`) and calls `Vault::init`. `notes create` takes a
`--title` and optional `--vault` (default `.`) and calls
`Vault::create_note`. `notes list` takes `--vault` and prints one note per
line (path + title-from-frontmatter). Uses `anyhow` at the boundary (ARCH
§7.1); typed errors propagate up via `?`.

**Acceptance criteria:**
- [x] `cargo run -p itala-cli -- init /tmp/x && cargo run -p itala-cli --
        notes create --vault /tmp/x --title "Hello"` writes a file under
        `/tmp/x/Notes/`.
- [x] `assert_cmd`-based integration test: init → create → list returns the
        created note.
- [x] CLI errors use `anyhow` context, not `panic!`/`unwrap`.
- [x] Help text (`--help`) renders cleanly for all three subcommands.

**Verification:** `cargo nextest run -p itala-cli`; `assert_cmd` end-to-end.

**Dependencies:** F4.

**Files likely touched:**
- `apps/itala-cli/src/main.rs`
- `apps/itala-cli/tests/cli.rs`

**Estimated scope:** M

---
