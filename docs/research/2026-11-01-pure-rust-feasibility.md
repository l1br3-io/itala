# Research: Pure Rust feasibility for Itala (all platforms except mobile)

- **Date:** 2026-11-01
- **Question:** Can Itala be built as "Pure Rust" on every platform except mobile
  (thin Kotlin/Swift shells hosting the Rust core there), and can the product
  goals — editor UX, accessibility, the ARCHITECTURE §8 performance budgets,
  cross-platform desktop+web — actually be achieved that way?
- **Depth note:** Lean, decision-grade pass. The commissioned deep pass
  (background agent) was killed by an account usage limit before writing
  anything; this pass was run inline with a bounded source budget. Anything
  not verified from a primary source this pass is marked **[open]** in §G.

## Verdict

**Yes — feasible, with the risk concentrated in exactly one place: the
desktop live-preview editor (MVP-β).** All other platforms are clean:

- **CLI (MVP-α):** already pure Rust. Nothing to decide.
- **Web (post-MVP):** Rust→WASM UI frameworks are judged production-ready as
  of early 2026; storage (IndexedDB/OPFS) is reachable from Rust with
  near-zero JS glue. Purity is realistic today.
- **Desktop (MVP-β):** two viable pure-Rust routes, both permissively
  licensed — (1) native UI via **iced** or **GPUI**, (2) Rust-authored
  webview via **Tauri v2 + Leptos** or **Dioxus desktop**. Route 1 is the
  *purist* choice and now has credible existence proofs (COSMIC on iced; Zed
  on GPUI). Its residual risks are accessibility parity and editor polish —
  real, shrinking, and mapped in §E.
- **Core dependencies:** every load-bearing crate already in or planned for
  the workspace is pure Rust and permissively licensed (§D); `cargo deny`
  already enforces this in-repo.

**Recommendation (evidence-weighted):** commit the architecture to *pure
Rust everywhere except mobile shells*, with the desktop renderer chosen at
MVP-β kickoff between **iced** (safer, COSMIC-proven, IME shipped, a11y in
progress) and **GPUI** (strongest editor existence proof via Zed, Apache-2.0,
young off-Zed ecosystem). Because the shell is isolated by the hexagonal
architecture, **M2 does not need this choice finalized** — it only needs the
render target to be a Rust-native semantic render tree (§F).

## Summary table

| Platform | Pure Rust? | Vehicle | Confidence | Key evidence |
|---|---|---|---|---|
| CLI | ✅ shipped | `itala-cli` | certain | M1 in-tree |
| Web | ✅ yes | Leptos / Yew / Dioxus → WASM; `idb`/`rexie`/`gloo` + OPFS via `wasm-bindgen` | high | [7][8] |
| Desktop | ✅ with trade-offs | native (iced / GPUI) or Rust-authored webview (Tauri+Leptos / Dioxus) | medium-high | [1][2][3][4][5][6] |
| Mobile | excluded by decision | thin Kotlin/Swift shells + Rust core; Dioxus may shrink shells later **[open]** | n/a | [12] |

## A. Desktop native UI frameworks

| | iced | egui | GPUI | Dioxus/Blitz | floem | Xilem | Slint |
|---|---|---|---|---|---|---|---|
| Version (as of 2026-11) | 0.14 (Dec 2025) [1] | ~0.33+ (Oct 2025) [2] | 0.2.2 (Oct 2025) [3][4] | 0.7 (Sep 2025) [5] | **[open]** | **[open]** | — |
| License | MIT **[verify]** | MIT/Apache **[verify]** | **Apache-2.0** (crates.io API) [4] | MIT/Apache **[verify]** | **[open]** | **[open]** | GPL-3.0 / royalty-free / commercial tri-license — **conflicts with our MIT/Apache-2.0** [10] |
| Accessibility | AccessKit WIP (#552, System76-driven); per-widget a11y properties landing 2026 [1] | AccessKit default-on native (Win/macOS) since 0.21 (2022); web canvas invisible to screen readers [2] | **[open]** | inherits webview a11y on desktop [5] | **[open]** | **[open]** | good, but license blocks |
| IME | **Shipped in 0.14** (Dec 2025) [1] | Windows OK; Linux regression since 0.29 (#5544), fix in progress [2] | Zed ships IME in production **[verify]** | via webview (platform) | **[open]** | **[open]** | yes |
| Text/editor primitives | built-in `text_editor` widget + syntax highlighter [1] | TextEdit basic; large buffers re-laid-out every frame — documented perf pain [2] | Zed's editor is the existence proof; `gpui-kit` ships editor components [6] | HTML/CSS via webview (or pre-alpha Blitz) [5] | Lapce's editor stack **[open]** | — | — |
| Large-doc / latency evidence | COSMIC desktop at scale **[verify]**; 60fps scrolling **[open]** | known weak point for very large text [2] | Zed's latency reputation; primary blog **[open]** | webview-grade | Lapce **[open]** | research | — |
| Existence proofs | System76 COSMIC | Rerun | **Zed** | many apps (webview mode) | Lapce | none (research) | embedded |

**Reading:** the editor-critical axis favors **GPUI** (Zed is a shipping,
latency-celebrated editor built on it; now Apache-2.0 with an ecosystem
forming — `gpui-kit`, `guise-ui` [6]). The safe, documented, desktop-app-scale
axis favors **iced** (COSMIC; IME shipped; System76 actively driving a11y
upstream [1]). egui is strongest on accessibility maturity but weakest on
large-text editing performance — the exact profile Itala's editor needs most
[2]. Blitz is promising (Servo-derived stylo/taffy/parley/vello) but
explicitly pre-alpha; dioxus-desktop still defaults to the system webview
[5].

## B. Rust-authored webview (the pragmatic middle)

- Tauri v2 + Leptos "100% Rust, no TypeScript" starter kits exist and the
  combination is in active use, including on Android (with known input/IME
  bugs there) [12]. Tauri's own renderer discussion confirms the desktop
  story is the system webview, with no native renderer planned [11].
- Purity verdict: **no TypeScript authored**, but the renderer is the OS
  webview and occasional JS glue is unavoidable. Satisfies "pure Rust
  codebase", not "pure Rust stack".
- Editor upside: the live-preview editor becomes a solved problem (render
  tree → HTML in Rust + platform text editing).

## C. Web (post-MVP)

- Leptos / Yew / Dioxus judged **production-ready** in early-2026
  comparisons; Leptos and Dioxus reportedly beat React in browser benchmarks;
  Leptos best for web-first, Dioxus best for one-codebase-cross-platform [7].
- Storage from Rust: IndexedDB via `idb` / `rexie` / `gloo-storage`; OPFS via
  `navigator.storage.get_directory()` through `wasm-bindgen`/`js_sys`; `std::fs`
  does not map to browser storage [8]. JS glue: near-zero, service-worker
 -sized.

## D. Core dependency purity audit

| Crate | Pure Rust? | License | Status |
|---|---|---|---|
| pulldown-cmark | ✅ (SIMD in Rust) | MIT | event-stream CommonMark + tables/footnotes/strikethrough; fastest class [9] |
| comrak | ✅ (1:1 cmark translation, no C) | BSD-2-Clause | full GFM, AST, max spec fidelity [9] |
| tantivy | ✅ (no runtime deps) | MIT | full-text search (M4) [13] |
| wasmtime | ✅ (Cranelift backend in Rust); optional C API bindings are avoidable | Apache-2.0 WITH LLVM-exception | M5 plugin runtime [14] |
| tokio, camino, smol_str, blake3, time, serde_yaml, fluent-rs, notify | expected ✅ | permissive | already vetted in-repo by `just deny`; `notify` uses OS-API **bindings**, not compiled C **[open: spot-verify]** |

The workspace already runs `cargo deny` (licenses + supply chain) and M0/M1
shipped with this stack, so the existing dependency set is proven pure and
permissive in practice.

## E. Risk register (mapped to VISION goals + §8 budgets)

| Risk | Severity | Evidence | Mitigation |
|---|---|---|---|
| Desktop editor polish parity vs web-tech UIs | **high** | egui large-text perf pain [2]; GPUI/iced editors are real but young [1][6] | decide renderer at MVP-β kickoff; prototype editor early in MVP-β; keep Tauri+Leptos as fallback shell behind the same core |
| Accessibility parity (screen readers) | medium | iced #552 WIP but System76-driven [1]; egui web invisible to SRs [2]; GPUI **[open]** | prefer iced/GPUI (native a11y APIs) over egui-web; track AccessKit; a11y metadata already in the plugin renderer contract |
| IME correctness (CJK users) | medium | iced shipped IME 0.14 [1]; egui Linux regression #5544 [2] | iced/GPUI avoid the egui Linux trap; test with CJK input during dogfooding |
| Large-doc 60fps scrolling (<16 ms keystroke) | medium | egui documented weakness [2]; Zed proves it's achievable in Rust **[open: cite blog]** | render-tree-level virtualization in our engine; benchmark against §8 in MVP-β |
| Ecosystem churn (GPUI off-Zed, Blitz pre-alpha) | medium | gpui 0.2.x [4]; Blitz README "pre-alpha" [5] | iced as the stable anchor; GPUI only if editor needs dominate |
| Web storage edge cases (OPFS browser support) | low | standard web APIs via bindings [8] | post-MVP concern |
| Theming / UI polish breadth | low-medium | component kits emerging (gpui-kit, guise-ui [6]) | first-party design tokens; we control the whole stack |

## F. Options and recommendation

1. **Native pure-Rust UI (iced or GPUI)** — purist; aligns with VISION
   criterion #1 (learning/craftsmanship); existence proofs at both
   desktop-app scale (COSMIC/iced) and editor scale (Zed/GPUI). Risks: §E
   rows 1–3.
2. **Rust-authored webview (Tauri v2 + Leptos / Dioxus desktop)** — zero
   TypeScript, dramatically lower editor risk; fails "pure Rust *stack*"
   purism only.
3. **Staged hybrid** — build M2–M5 CLI-first (already pure), prototype *both*
   desktop shells on the frozen core during MVP-β kickoff, pick on evidence.

**Recommended:** Option 3, structured as a commitment to Option 1: declare
"pure Rust except mobile" the architecture principle now (ADR), prototype
iced vs GPUI editor spikes at MVP-β kickoff, keep Option 2 as the recorded
fallback. **M2 impact:** the render target becomes a **Rust-native semantic
render tree** (styled-text/widget IR) with concrete renderers derived from
it; an HTML renderer is optional insurance, not the primary target.

**Mobile (settled direction, 2026-11-01):** mobile shells are fully native
(SwiftUI / Jetpack Compose) and consume the Rust core across a thin UniFFI
surface — the pattern proven in-repo by l1br3's taletrek
(`crates/ffi`: "every decision stays in Rust; the wrappers only flatten
types and forward calls"; uniffi 0.32, cdylib, proc-macro mode, checked-in
Kotlin bindings, `just android-bindings` / `just ios-bindings`, build systems
never invoke each other, `unsafe_code = forbid` coexists with the FFI
boundary). No Rust UI on mobile. Third-party plugin hosting on mobile is a
per-shell policy decision for the future; wasmi remains the pure-Rust option
if it is ever wanted, so the iOS no-JIT constraint (§H) stays a non-issue
for launch.

## G. Open items for the deep pass

1. floem and Xilem current status (skipped this pass — likely outranked).
2. License verification from repos for: iced, egui, Dioxus (recorded as
   MIT/Apache from ecosystem knowledge; not primary-verified here). GPUI
   Apache-2.0 *is* primary-verified [4].
3. Slint license tri-terms exact reading (conflict conclusion is robust).
4. `markdown-rs` crate as a third parser option.
5. `notify` (and any other current dep) — confirm no compiled C.
6. GPUI: Linux/Windows backend maturity (Blade/Vulkan), IME on all
   desktops, AccessKit story, documentation depth.
7. iced: 60fps large-doc scrolling evidence; `text_editor` limitations in
   detail (multi-cursor? plugins?).
8. Zed latency blog (first-party) as the <16 ms existence citation.
9. egui issue #56 / #5544 resolution state (only if egui stays on the
   shortlist — currently it doesn't for the editor surface).

## H. Addendum: the plugin system across platforms

The lean pass audited wasmtime's *purity* but not plugin *hosting* per
platform. Corrected here — two hard platform facts and the architectural
consequence.

**Platform facts (verified):**

- **iOS forbids JIT** in third-party apps (W^X memory; only browsers are
  exempt) → wasmtime's Cranelift JIT cannot run in a normal iOS app [15].
  Interpreter runtimes are the standard answer; **wasmi** (pure Rust) is the
  proven embeddable choice (Stellar/Soroban picked it for exactly this
  reason) [15]. Interpreter overhead is roughly ~10× vs ~20% for JIT —
  acceptable for M5's deterministic tools/commands/exporters, not for
  hot-path code [15].
- **Wasmtime cannot run inside `wasm32`** — a native host cannot be hosted
  by the browser's WASM runtime. Web plugin execution needs either **jco**
  (Bytecode Alliance; transpiles WASM Components to ES modules running on
  browser-native WebAssembly; reached 1.0) [16][17], or **wasmi compiled
  into our own wasm core** (pure Rust, uniform host semantics, metering,
  slower) — Substrate-class runtimes demonstrate wasmi-on-wasm32 works.
- **Gas/epoch metering** (DoS protection for untrusted plugins) exists in
  wasmtime (fuel + epochs); most engines lack built-in gas metering, and
  wasmi provides metering middleware — depth per engine to verify [18].
- **jco caveat:** its WASI shims grant components broad access to host
  resources; we would not use `wasi:*` worlds but our own minimal
  capability-checked WIT world, so host imports are exactly the granted
  capabilities [19].

**Per-platform hosting matrix (M5):**

| Platform | Plugin runtime | Purity | Notes |
|---|---|---|---|
| CLI / Desktop / Android | wasmtime (Cranelift) | pure Rust | fuel + epoch interruption; JIT allowed |
| Web | jco-transpiled components, or wasmi-in-wasm | jco = JS toolchain (build/install-time only; browser reality) · wasmi = pure Rust | decision at M5 spec |
| iOS | wasmi interpreter (or defer plugins on iOS) | pure Rust | no-JIT constraint [15] |

**Architectural consequence (cheap now, expensive later):** ADR-0005 and
ARCHITECTURE §4.3 already commit to a *versioned component contract* rather
than a specific engine — nothing to un-decide. Carry that through: define
the plugin ABI as **WIT worlds** so one plugin package runs on any conformant
host, and make the plugin runtime itself a **port** (`PluginRuntime`) with
per-platform adapters (wasmtime / jco / wasmi), mirroring `Vfs`. M0–M4
native compile-time plugins are unaffected — they compile into the app on
every platform by construction.

**Impact on pending decisions:** the pure-Rust principle is *strengthened*
(every candidate runtime is pure Rust and permissive; jco is a build-time
web toolchain, not shipped product code). ADR-0008 should carry one clause:
plugin runtime = port + adapters, ABI = WIT. M2 is unaffected; the concrete
web-host choice lands at the M5 spec. Under the settled mobile direction
(native shells + core-via-UniFFI), mobile shells do not host plugins at
launch, so the iOS JIT constraint is dormant, not a design driver.

## Sources

1. iced 0.14 release + FAQ + a11y tracking —
   https://www.phoronix.com/news/Iced-0.14-Rust-GUI ;
   https://book.iced.rs/faq.html ;
   https://github.com/iced-rs/iced/issues/552
2. egui accessibility docs + IME/large-text issues —
   https://github.com/emilk/egui/blob/main/docs/accessibility.md ;
   https://github.com/emilk/egui/issues/5544 (Linux IME regression);
   https://github.com/emilk/egui/issues/56 (text perf);
   https://accesskit.dev/looking-back-looking-forward
3. gpui on crates.io — https://crates.io/crates/gpui
4. gpui crates.io API (license/version, primary) —
   https://crates.io/api/v1/crates/gpui → Apache-2.0, 0.2.2, 2025-10-22
5. Dioxus 0.7 + Blitz — https://dioxuslabs.com/learn/0.7/guides/platforms/desktop ;
   https://github.com/DioxusLabs/blitz ;
   https://github.com/DioxusLabs/dioxus/discussions/1519
6. GPUI ecosystem — https://github.com/longbridge/gpui-kit ; guise-ui on
   crates.io (Sep 2026)
7. Framework comparison (Feb 2026) —
   https://reintech.io/blog/leptos-vs-yew-vs-dioxus-rust-frontend-framework-comparison-2026
8. Web storage from Rust — gloo/idb/rexie crates; OPFS via wasm-bindgen
   (ecosystem-standard pattern; spot-verify in deep pass)
9. pulldown-cmark vs comrak — https://github.com/pulldown-cmark/pulldown-cmark ;
   https://github.com/kivikakk/comrak
10. Slint licensing — https://github.com/slint-ui/slint (README licensing
    section; tri-license)
11. Tauri native-renderer discussion —
    https://github.com/tauri-apps/tauri/discussions/10964
12. Tauri v2 + Rust-frontend starter / Android input bug —
    https://github.com/kitlib/tauri-app-template ;
    https://github.com/tauri-apps/tauri/issues (Tauri 2 + Leptos Android
    keyboard, Aug 2024)
13. Tantivy — https://github.com/quickwit-oss/tantivy ; https://deps.dev/cargo/tantivy
14. Wasmtime — Bytecode Alliance; Apache-2.0 WITH LLVM-exception; Cranelift
    Rust backend (deep pass to primary-verify)
15. iOS JIT prohibition + interpreter runtimes — Stellar "Why doesn't Soroban
    use a JIT?" (wasmi choice): https://www.stellar.org/blog ;
    WasmKit/interpreter landscape coverage (Medium)
16. jco — https://github.com/bytecodealliance/jco ;
    https://bytecodealliance.github.io/jco/ (1.0 announcement)
17. jco transpiling modes + browser use — https://jco.dev ;
    https://github.com/bytecodealliance/jco/issues/42
18. Gas metering landscape —
    https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status
19. jco WASI security note —
    https://component-model.bytecodealliance.org
