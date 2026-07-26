# F2: Note data model in `itala-note`

> Effort: `m1-vaults-notes` · Status: shipped · Phase: Phase 2: Note model (ARCH §5.1)

**Description:** `Note` struct: `id: NoteId`, `path: VfsPath`,
`frontmatter: Frontmatter`, `body: Bytes`, `created_at: OffsetDateTime`,
`updated_at: OffsetDateTime`, `checksum: [u8; 32]` (BLAKE3). `Frontmatter`
is a typed wrapper around a `serde-saphyr`-parsed YAML map. Methods:
`Note::new(...)` constructor that computes BLAKE3, `Note::from_markdown(bytes)`
parses frontmatter-delimited Markdown into the struct,
`Note::to_markdown()` projects back deterministically.

**Acceptance criteria:**
- [x] Round-trip: `from_markdown → to_markdown` is identity for 1k
      proptest-generated Markdown snippets.
- [x] BLAKE3 checksum recomputed on every persist (unit test).
- [x] Frontmatter absent → empty `Frontmatter`, body is the whole input.
- [x] Snapshot test (insta) on a representative Markdown+frontmatter doc.

**Verification:** `cargo nextest run -p itala-note`; `cargo clippy -p
itala-note -- -D warnings`.

**Dependencies:** F1.

**Files likely touched:**
- `crates/itala-note/src/{lib.rs,note.rs,frontmatter.rs}`

**Estimated scope:** M

---
