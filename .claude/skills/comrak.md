# comrak

Single-page reference for Markdown parsing and rendering.

## Why comrak

- CommonMark + GitHub Flavored Markdown reference implementation in Rust.
- AST-first design — we walk and transform before rendering, which is essential for wiki links, transclusion, and plugin renderers.
- Active maintenance, broad ecosystem use.

Itala uses comrak for parsing only by default; rendering goes through our renderer pipeline so plugins can hook in.

## Parsing

```rust
use comrak::{parse_document, Arena, ComrakOptions};

let arena = Arena::new();
let opts = make_options();
let root = parse_document(&arena, source, &opts);
```

The arena owns all nodes; nodes reference each other by `&'a` lifetimes. Build a fresh arena per parse — they are cheap to construct and free everything at once when dropped.

## Options we enable

```rust
fn make_options() -> ComrakOptions<'static> {
    let mut opts = ComrakOptions::default();
    opts.extension.strikethrough = true;
    opts.extension.tagfilter = false;             // we sanitize separately
    opts.extension.table = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    opts.extension.front_matter_delimiter = Some("---".to_string());
    opts.extension.multiline_block_quotes = true;
    opts.extension.math_dollars = true;
    opts.extension.wikilinks_title_after_pipe = true;
    opts.parse.smart = true;
    opts.parse.relaxed_autolinks = false;
    opts.render.hardbreaks = false;
    opts.render.unsafe_ = false;                  // render passes raw HTML through? no
    opts.render.escape = false;
    opts
}
```

`unsafe_ = false` keeps embedded raw HTML escaped. We only allow HTML when the render pipeline explicitly opts in (e.g. for plugin-provided sanitized fragments).

## Walking the AST

```rust
use comrak::nodes::{AstNode, NodeValue};

fn visit<'a>(node: &'a AstNode<'a>, depth: usize) {
    match &node.data.borrow().value {
        NodeValue::Heading(h) => {
            println!("{:indent$}H{}", "", h.level, indent = depth * 2);
        }
        NodeValue::Link(link) => {
            // process link.url
        }
        _ => {}
    }
    for child in node.children() {
        visit(child, depth + 1);
    }
}
```

`node.data` is a `RefCell<Ast>` — `.borrow()` for read, `.borrow_mut()` for in-place transforms.

## Wiki links

`wikilinks_title_after_pipe` enables `[[Note Title|Alias]]` syntax. Itala extends the AST after parse:

1. Walk for `NodeValue::WikiLink` nodes.
2. Resolve target against the link graph (`itala-graph`).
3. Replace with a `NodeValue::Link` carrying a custom URL scheme `itala://note/{id}`.

For transclusion (`![[Note]]`), we add a custom `NodeValue::HtmlBlock` containing a placeholder that the renderer expands to inline content.

## Block IDs

Block references (`[[Note#^block-id]]`) require stable per-block IDs. Itala generates them lazily:
- Walk the AST.
- For each block-level node, compute a hash of the canonical content and use that as the block id.
- Persist `(note_id, block_id) -> position` in `itala-graph`'s redb tables.

Block IDs are content-addressed so they survive renames and minor edits without breaking incoming links.

## Rendering

For HTML output we use `comrak::format_html_with_plugins`:

```rust
use comrak::{format_html_with_plugins, ComrakPlugins};

let mut plugins = ComrakPlugins::default();
plugins.render.codefence_syntax_highlighter = Some(&syntax_plugin);

let mut buf = Vec::new();
format_html_with_plugins(root, &opts, &mut buf, &plugins)?;
let html = String::from_utf8(buf)?;
```

For Itala's rich rendering (block embeds, plugin renderers, math), we walk the AST ourselves and emit HTML through `itala-publish::render::Renderer`, with plugin renderers attached.

## Sanitization

comrak's escape settings handle Markdown-source HTML. Anything passed through plugin renderers goes through `ammonia` before being injected into the webview. Default: strict allowlist — `<a>`, `<code>`, `<pre>`, headings, lists, tables. Plugins that need richer HTML declare a sanitization profile in their manifest.

## Performance

- 10 KB note: parse <1 ms.
- Typical render <5 ms.
- For large notes (>100 KB), prefer parsing once and caching the AST in `itala-note-editor`'s session state. Re-parse on edit, not on every render request.

## Round-trip

comrak ships `format_commonmark` for AST → Markdown serialization. Useful for normalizing files on save, but **disabled by default** — we don't want to silently rewrite the user's file. If we add a "tidy on save" option later, this is the entry point.
