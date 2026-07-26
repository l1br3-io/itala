# F16: Documentation update

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 8: Integration + sign-off

**Description:** Update `docs/ARCHITECTURE.md` to mark `itala-vault`,
`itala-note`, `itala-vfs-fs`, `apps/itala-cli` as *(shipped)* rather than
*(target)* where applicable. Update `Cargo.toml` comment about `apps/*`.
Mark M1 in `docs/ROADMAP.md` as ✅.

**Acceptance criteria:**
- [x] Architecture crate map reflects M1 reality.
- [x] Roadmap status reflects M1 completion.
- [x] `just doc` green.

**Verification:** `just doc`.

**Dependencies:** F15.

**Files likely touched:**
- `docs/ARCHITECTURE.md`
- `docs/ROADMAP.md`

**Estimated scope:** S

---
