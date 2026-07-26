# M1 — Vaults & Notes: checklist (archived)

> **Shipped.** All 16 features complete; `just ci` green (lint + test + deny + doc).
> 145+ tests across `itala-note`, `itala-vault`, `itala-store-files`, and `apps/itala-cli`,
> plus a kernel-history suite. Full detail in [`plan.md`](./plan.md).

## Phase 1 — Scaffolding
- [x] F1: Crate skeletons + workspace wiring

## Phase 2 — Note model (ARCH §5.1)
- [x] F2: Note data model in `itala-note`

## Phase 3 — Vault context (ARCH §6.1, §6.3)
- [x] F3: Vault open/init in `itala-vault`
- [x] F4: Note CRUD in `itala-vault`

### Checkpoint A — CLI green for init / create / list
- [x] F5: CLI `init`, `notes create`, `notes list`

## Phase 4 — Note operations
- [x] F6: `notes show` and `notes delete`

## Phase 5 — History port + adapter (ADR-0004 §1, §5)
- [x] F7: `HistoryStore` + `DynHistoryStore` port in `itala-kernel`
- [x] F8: `MemoryHistoryStore` test adapter
- [x] F9: Snapshot-on-save integration in vault
- [x] F10: `history list`, `history show`, `history restore` CLI commands

### Checkpoint B — CLI green for CRUD + history

## Phase 6 — Move / rename + naive link rewriting
- [x] F11: `[[wiki-link]]` scanner in `itala-vault`
- [x] F12: `notes move` + link rewriting

## Phase 7 — Files adapter (ADR-0004 §3)
- [x] F13: `FilesHistoryStore` in `itala-store-files`
- [x] F14: CLI wires `FilesHistoryStore` by default

## Phase 8 — Integration + sign-off
- [x] F15: M1 integration suite
- [x] F16: Documentation update
