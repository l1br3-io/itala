# Research: Recreating an Obsidian-class markdown editor in Rust as an Itala plugin

- **Date:** 2026-11-01
- **Question:** Can Itala recreate the kind of markdown editor Obsidian uses — in
  Rust, delivered as a plugin over the kernel — and what route should it take:
  pure-Rust editor, embedding CodeMirror 6 in a webview, or a hybrid?
- **Depth note:** Lean, decision-grade pass, inline with a bounded source
  budget. The commissioned background agent died silently before writing
  anything (second consecutive time — see the pure-Rust note's depth note).
  Everything below was verified against a primary source on 2026-11-01 unless
  marked **[open]**. This note drills into the exact risk the pure-Rust
  feasibility note concentrated in one place: "the desktop live-preview
  editor (MVP-β)".

## Verdict

**Yes — and it is legally cleaner than it sounds.** "What Obsidian is using"
is not proprietary: it is **CodeMirror 6 (MIT)**. Obsidian's own public plugin
API types import `@codemirror/state` and `@codemirror/view` and declare a
*"CodeMirror {@link ViewPlugin} for Live Preview"* [1]. What is closed is
Obsidian's *implementation* of its live-preview extensions — but the mechanism
they are built on (CM6 decorations + view plugins + a state/view split) is
documented public API [2][3], and an MIT open-source editor already implements
Obsidian-style inline live preview on plain CM6 *without any Obsidian code*
(atomic-editor: "CodeMirror 6 markdown editor with Obsidian-style inline live
preview", MIT, active 2026-10) [4]. So the target is reproducible; the work is
engineering, not reverse engineering.

Three routes are viable; none requires touching anything closed:

| Route | What | Editor fidelity | Effort | Risk | Purity |
|---|---|---|---|---|---|
| **A. Pure Rust** | ropey/crop buffer + tree-sitter-markdown + cosmic-text/parley (or GPUI/iced editor spike) | full control; Obsidian-class live preview **not yet proven shipped** by anyone in pure Rust | high | medium-high | pure Rust stack |
| **B. CM6 in webview** | Tauri v2 shell; CM6 + lezer markdown extensions (wikilinks) in the page; Rust owns document/vault via IPC | Obsidian-equivalent by construction (same foundation Obsidian uses) | medium | low | pure Rust *codebase*, webview renderer |
| **C. Hybrid** | Rust owns buffer/CRDT + markdown AST + decorations (semantic render tree); platform surface renders it | high, incremental | medium-high | medium | pure Rust core, thin surface |

**Recommendation (consistent with the pure-Rust note):** keep the staged
decision — prototype **Route A** at MVP-β kickoff (iced vs GPUI spike, per
prior §F), keep **Route B** as the recorded fallback behind the same core.
What this pass adds: design the **editor plugin interface now**, modeled on
CM6's state/view/extension separation (§F), so Routes A/B/C all satisfy one
kernel contract and the route choice remains a shell decision, not a core
decision. Route B is no longer "the safe unknown": today it would embed an
MIT editor foundation that is actively maintained — on a self-hosted Forgejo
since April 2026 — with a proven open live-preview reference [4].

## A. What Obsidian's editor actually is (primary evidence)

- Obsidian's first-party plugin API (`obsidian.d.ts`, obsidianmd/obsidian-api)
  begins with `import { Extension, StateField } from '@codemirror/state'` and
  `import { EditorView, ViewPlugin } from '@codemirror/view'` [1].
- The same file declares: an interface that "bridges the gap between
  CodeMirror 5 and CodeMirror 6"; `StateField`s to fetch the `EditorView`,
  file info, and *"whether Live Preview is active"*; a *"CodeMirror ViewPlugin
  for Live Preview"*; and `Plugin.registerEditorExtension(ext)` — *"Registers
  a CodeMirror 6 extension"* [1].
- Forum corroboration: Obsidian staff participated in the CM6-migration-era
  threads (e.g. "Vim support for the new editor (CodeMirror 6)") [5].
- Architecturally, "live preview" in CM6 terms = **decorations** (mark/widget
  replacements over ranges) driven by the syntax tree, applied by view
  plugins over an immutable document state; CM6's guide documents the
  state/view/transaction/extension split and Lezer incremental parsing
  integration [2][3]. Wikilink-style syntax is expressible as **lezer
  markdown extensions** (`MarkdownExtension`/`MarkdownConfig` — defineProps,
  custom node props, inline/block extensions) [6][7].
- The Obsidian app itself remains closed-source; nothing above derives from
  its internals — only from its published API types, docs, and staff-visible
  forum behavior.

**Consequence:** "recreate what Obsidian is using" = *recreate CM6-class
capabilities* (MIT, documented), not copy Obsidian (impossible and
unnecessary).

## B. Open/closed boundary and licensing (verified 2026-11-01)

| Component | License | Status | Evidence |
|---|---|---|---|
| CodeMirror 6 (`codemirror/dev` monorepo) | **MIT** | Active — **moved off GitHub**: whole `codemirror` org archived 2026-04-15; development continues on Haverbeke's Forgejo at `code.haverbeke.berlin/codemirror/dev` (not archived, updated 2026-10-07) | [8][9][10] |
| Lezer markdown parser | MIT | Same move (`code.haverbeke.berlin/lezer/markdown`) | [7][9] |
| `@codemirror/lang-markdown` | MIT | Part of the CM monorepo; standalone repo archived at the org move | [6][8] |
| Obsidian app + its live-preview extensions | proprietary | Closed; only behaviorally re-implementable | [1] (API surface only) |
| atomic-editor (Obsidian-style live preview on CM6) | **MIT** | Active (pushed 2026-10-13), 160★ | [4] |

Notable ecosystem event: the CodeMirror/Lezer **GitHub org was fully archived
on 2026-04-15** because the project self-hosts on Forgejo now [8][9]. The
project is alive (Forgejo repo updated 2026-10-07 [10]). For Route B this
means: pin npm artifacts normally; do not treat the archived GitHub repos as
the upstream.

## C. Rust building blocks (crates.io / repo status, 2026-11-01)

| Block | Crate | Version | Last release | License | Read |
|---|---|---|---|---|---|
| Text buffer (rope) | `ropey` | 2.0.0-beta.1 | 2025-08-02 | MIT **[verify repo]** | the standard; 1.x stable line in wide use (Helix) [11] |
| Text buffer (rope, alt) | `crop` | 0.4.3 | 2025-04-25 | MIT **[verify]** | smaller, fast; slower release cadence [12] |
| Incremental parser host | `tree-sitter` | 0.27.1 | 2026-10-08 | MIT | very active [13] |
| Markdown grammar | `tree-sitter-markdown` (crates.io) | 0.7.1 | **2021-04-18 (stale)** | MIT | **crates.io publication abandoned**; canonical grammar moved to `tree-sitter-grammars/tree-sitter-markdown` — active, pushed 2026-10-24, MIT; consume via git dep [14][15] |
| Text layout/shaping | `cosmic-text` | 0.19.0 | 2026-04-22 | MIT **[verify]** | "Pure Rust multi-line text handling"; COSMIC-adjacent [16] |
| Rich text layout | `parley` | 0.12.0 | 2026-10-09 | **Apache-2.0** (repo-verified) | Linebender, very active; prior note already routed Blitz/Blitz-adjacent stacks through it [17][18] |
| CRDT (future sync) | `yrs` | 0.28.0 | 2026-09-17 | MIT **[verify]** | Yjs port, active [19] |
| CRDT (alt) | `diamond-types` | 1.0.0 | 2022-08-25 | MIT **[verify]** | stale — avoid for now [20] |
| Markdown AST (render path) | `pulldown-cmark`/`comrak` | — | — | MIT / BSD-2 | already audited pure in prior note §D [21] |

Reading: **every load-bearing block exists, is pure Rust, and is permissively
licensed**; the two cautions are the abandoned crates.io publication of
tree-sitter-markdown (use the grammar org's repo) and ropey being mid-2.0
beta (pin 1.x until 2.0 final).

## D. Existence proofs — and what is actually extractable

| Project | Editor core in Rust? | Reusable by Itala? | Status |
|---|---|---|---|
| **Zed** | yes (`crates/editor` in monorepo; buffer = their own trees/CRDT) | **No as code** — app repo is multi/"Other"-licensed (GPL-bearing) and `crates/editor` is not published to crates.io; **yes as proof** (GPUI editor at production polish). `gpui` itself is Apache-2.0 on crates.io (prior note §A) and `gpui-kit` ships editor components | active (pushed 2026-11-01) [22][23] |
| **Helix** | yes | partially — buffer recipe is public (`helix-core/Cargo.toml` depends on `ropey`, verified) but `helix-core` on crates.io is a **0.0.0 placeholder** (2021); treat as recipe, not dependency | active [11][24] |
| **xi-editor** | yes (archetype) | as **prior art only** — discontinued 2019–2020; retrospective + "Rope Science" essays are the canonical design references; rope survives as `lapce-xi-rope` | discontinued [25][26][27] |
| **Lapce / floem** | yes (floem editor) | floem published (0.2.0, 2024-11) but lags its repo (pushed 2026-09-24); editor widget maturity **[open]** | active-ish [28] |
| **Velotype** | early attempt at exactly our target: native Rust (GPUI) markdown editor with WYSIWYG-as-you-type rendering | watch, don't rely: single-maintainer, license not marked on GitHub **[open: license/maturity]** | pushed 2026-10-17 [29][30] |
| **atomic-editor** | no (CM6/React) | **Route B reference**: open MIT implementation of Obsidian-style live preview without Obsidian code | active 2026-10 [4] |
| **markdown-oxide** | LSP, not an editor surface | useful Rust building block for Obsidian-compatible wikilink/backlink semantics from any editor (incl. Helix) | see repo [31] |

**Key negative finding:** no shipped, polished pure-Rust editor with
Obsidian-style *editable* live preview was found. Zed edits code (its
markdown surfaces are rendered previews **[open: verify scope]**), Helix is
modal source-editing, Velotype is early. The live-preview *rendering* in
pure Rust is exactly the "semantic render tree" already mandated by the
pure-Rust note §F — no one has finished gluing it to an editable buffer with
decoration performance at CM6 polish. That gap is Route A's real cost, and
Velotype is the live experiment to watch.

## E. The hard problems, mapped

1. **Live-preview decoration pipeline** (the big one). In CM6, decorations
   recompute per transaction over an incrementally-updated syntax tree [2][3].
   Rust equivalent: rope delta → tree-sitter incremental edit → re-query →
   decoration set → semantic render tree diff. All pieces exist (§C); the
   performance glue is unproven as a product. Budget an MVP-β spike.
2. **Wikilinks/embeds/tags/frontmatter as grammar extensions.** Route B:
   lezer `MarkdownExtension` (documented extension points) [6][7]. Route A:
   tree-sitter-markdown injections (grammar supports `injections` for
   embedded languages **[open: verify injection coverage for wikilinks]**).
3. **IME + accessibility.** Carried over from prior note §E — iced shipped
   IME (0.14), GPUI proves IME in production, egui's Linux regression is the
   trap. A11y: AccessKit route; CM6-in-webview inherits platform webview
   a11y for free (Route B's quiet advantage).
4. **Undo/collab coherence.** If `yrs` lands later for sync (M-later), the
   editor's history layer must be CRDT-compatible from day one — CM6's
   example pairing and `y-codemirror` are prior art for Route B; `yrs` +
   ropey bridge for Route A **[open]**.
5. **Bidirectional text / complex scripts** in the pure-Rust layout path:
   cosmic-text/parley handle shaping, but editor-level bidi caret movement is
   on us **[open]**.

## F. Editor-as-plugin: the interface Itala needs (≤1 page)

The kernel should own what Obsidian's `obsidian.d.ts` exposes as
platform-adjacent, and mirror CM6's split for what it doesn't:

- **Kernel-side (core truth):** note document (bytes + parsed AST index),
  change events as deltas, link resolution (`resolve_wikilink`), metadata
  (frontmatter), file/bundle association — the analog of Obsidian's
  `MarkdownFileInfo` StateField [1].
- **Editor-plugin side (surface truth):** buffer view (rope window), cursor/
  selection set, decoration set keyed by semantic spans, input handling,
  undo stack ownership with kernel-checked persistence points.
- **Extension seam:** an Itala `register_editor_extension` — the analog of
  Obsidian's `registerEditorExtension` [1] — where *other* plugins contribute
  decorations (e.g. AI plugin underlining, link plugin rendering `[[…]]`
  chips). This is the requirement that makes the editor a plugin like
  everything else, and it is satisfiable by all three routes because CM6's
  extension model is the design template for A and C, and literal for B.
- **Renderer contract:** decorations target the prior note's semantic render
  tree, so Route A (native) and Route B (HTML renderer) consume the same IR —
  the editor route stays a shell decision through MVP-β.

Minimal trait sketch (for the spec, not carved now): `EditorSurface` (open,
apply_delta, decorations_in, focus), `Decoration { span, kind, payload }`,
`DocumentDelta { base_rev, edits }`, kernel service `resolve_link(Target) ->
ResolvedLink`.

## G. Open items (for the MVP-β editor spec)

1. ropey 1.x vs 2.0-beta pin; crop bake-off (micro-bench inside our buffer).
2. tree-sitter-markdown via git dep: injection coverage for wikilinks/tags.
3. Velotype license + trajectory; gpui-kit editor component maturity.
4. Route B spike cost: Tauri v2 + CM6 + lezer wikilink extension + one Rust
   IPC decoration source (est. the cheapest credible demo).
5. `yrs` integration shape (deferred to sync milestone, but name the history
   contract now).
6. License spot-verify from repos: ropey, crop, cosmic-text, yrs (recorded
   permissive from ecosystem knowledge; not primary-verified this pass).
7. Zed markdown surface scope (claimed preview-only; not verified).

## Sources

1. Obsidian plugin API types (first-party; imports + Live Preview
   ViewPlugin + registerEditorExtension) —
   https://github.com/obsidianmd/obsidian-api/blob/master/obsidian.d.ts
2. CodeMirror 6 system guide (state/view/transactions/extensions) —
   https://codemirror.net/docs/guide/ (page unreachable from this network
   this pass — **[open: re-verify]**; facts cross-checked via [3] and API
   docs mirrored in [1])
3. CodeMirror 6 reference (decorations, ViewPlugin, Lezer integration) —
   https://codemirror.net/docs/ref/ (same caveat as [2])
4. atomic-editor — CodeMirror 6 markdown editor with Obsidian-style inline
   live preview, MIT — https://github.com/kenforthewin/atomic-editor
5. Obsidian forum, CM6 migration era —
   https://forum.obsidian.md/t/vim-support-for-the-new-editor-codemirror-6-neovim-libvim-or-something-else/21803
6. @codemirror/lang-markdown (README: `extensions` → lezer
   MarkdownExtension) — https://github.com/codemirror/lang-markdown
7. lezer markdown parser (extension API: MarkdownConfig, defineProps) —
   https://github.com/lezer-parser/markdown (docs at
   https://code.haverbeke.berlin/lezer/markdown)
8. CodeMirror GitHub org archived 2026-04-15 (all repos read-only) —
   https://github.com/codemirror/dev (archived banner; API-verified)
9. Migration announcement (Apr 2026, Forgejo self-hosting) — search-verified
   summary of the codemirror announcement; **[open: fetch primary post]**
10. CM6 monorepo, new home (Forgejo, active) —
    https://code.haverbeke.berlin/codemirror/dev (API: not archived,
    updated 2026-10-07)
11. ropey — https://crates.io/crates/ropey
12. crop — https://crates.io/crates/crop
13. tree-sitter — https://crates.io/crates/tree-sitter
14. tree-sitter-markdown (stale crates.io publication) —
    https://crates.io/crates/tree-sitter-markdown
15. tree-sitter-grammars/tree-sitter-markdown (canonical, active, MIT) —
    https://github.com/tree-sitter-grammars/tree-sitter-markdown
16. cosmic-text — https://crates.io/crates/cosmic-text
17. parley — https://crates.io/crates/parley
18. linebender/parley (repo, Apache-2.0) —
    https://github.com/linebender/parley
19. yrs — https://crates.io/crates/yrs
20. diamond-types — https://crates.io/crates/diamond-types
21. Prior note §D —
   docs/research/2026-11-01-pure-rust-feasibility.md (in-repo)
22. zed-industries/zed (crates/editor in-tree; license multi/"Other") —
    https://github.com/zed-industries/zed
23. gpui (Apache-2.0, 0.2.2) — https://crates.io/crates/gpui (verified in
    prior note §A)
24. helix-core placeholder — https://crates.io/crates/helix-core ;
    ropey dep verified in
    https://github.com/helix-editor/helix/blob/master/helix-core/Cargo.toml
25. xi-editor (discontinued) — https://github.com/xi-editor/xi-editor
26. xi retrospective (Raph Levien, 2020) —
    https://raphlinus.github.io/xi/2020/06/27/xi-retrospective.html
27. lapce-xi-rope — https://crates.io/crates/lapce-xi-rope
28. floem — https://crates.io/crates/floem ;
    https://github.com/lapce/floem
29. Velotype announcement —
    https://users.rust-lang.org/t/velotype-a-native-rust-markdown-editor-built-for-wysiwyg-writing/140046
30. Velotype repo — https://github.com/manyougz/velotype
31. markdown-oxide —
    https://www.reddit.com/r/rust/comments/1nhjir9/any_markdown_editor_written_in_rust_like_obsidian
    (pointer; repo: https://github.com/Fabian-Lsch/markdown-oxide **[open:
    verify repo]**)
