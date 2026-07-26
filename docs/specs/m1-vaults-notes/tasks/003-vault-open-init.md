# F3: Vault open/init in `itala-vault`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 3: Vault context (ARCH §6.1, §6.3)

**Description:** `Vault::init(root: &Utf8Path) -> Result<Vault>` creates
`.itala/`, `.itala/config.toml` (minimal default), `.itala/history/`,
`Notes/`, `Assets/`. `Vault::open(root: &Utf8Path) -> Result<Vault>` opens
an existing vault, fails with `Error::Config` if `.itala/` is missing.
`Vault` holds `root: Utf8PathBuf`, `id: VaultId` (generated on init, loaded
from `.itala/vault.id` on open), and a `Box<dyn DynVfs>` rooted at `root`.

**Acceptance criteria:**
- [x] Init then open round-trips in a `tempfile::TempDir`.
- [x] Open on a non-vault directory returns `Error::Config`.
- [x] On open, `VaultOpened { id }` is published on the bus.
- [x] `VaultId` persists across reopen (read from `.itala/vault.id`).

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F1, kernel's existing `LocalFsVfs`.

**Files likely touched:**
- `crates/itala-vault/src/{lib.rs,vault.rs}`

**Estimated scope:** M

---
