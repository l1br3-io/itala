# F11: `[[wiki-link]]` scanner in `itala-vault`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 6: Move / rename + naive link rewriting

**Description:** Char-scanning parser (no regex dependency) that walks
Markdown bytes and yields `WikiLink { target: SmolStr, display: Option<SmolStr>,
byte_range: Range<usize> }`. Targets match `[[Name]]` and `[[Name|Display]]`
and `[[Name#^block-id]]` (block refs are part of the target on the rename
side — the name is the prefix). Excludes links inside fenced code blocks.

**Acceptance criteria:**
- [x] Snapshot tests (insta) on a corpus of ~10 Markdown snippets covering
      plain links, display-aliased links, block refs, code-fenced false
      positives.
- [x] Proptest: random ASCII bytes never panic the scanner.
- [x] Byte ranges round-trip: `bytes[range]` is the original link text.

**Verification:** `cargo nextest run -p itala-vault`.

**Dependencies:** F4.

**Files likely touched:**
- `crates/itala-vault/src/{wiki_link.rs,slug.rs}`

**Estimated scope:** M

---
