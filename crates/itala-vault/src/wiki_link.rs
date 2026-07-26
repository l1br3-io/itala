//! `[[wiki-link]]` scanner (M1 plan, F11 / AD-2).
//!
//! Character-scanning parser that yields [`WikiLink`] occurrences in a body
//! of text. Used by the vault context to rewrite links on note rename/move
//! (M1's naive rewrite path; replaced by an indexed lookup in M3).
//!
//! ## Supported syntax
//!
//! - `[[Name]]`             — bare link to `Name`
//! - `[[Name|Display]]`     — link with explicit display text
//! - `[[Name#^block-id]]`   — block reference; the link target is `Name`
//!   (the `#^block-id` suffix is preserved as `block`)
//! - `[[#^block-id]]`       — same-page block reference; `target` is empty
//!
//! ## Code-fence exclusion
//!
//! Links inside fenced code blocks (` ``` `) and inline code (`` `…` ``) are
//! NOT yielded, matching the `CommonMark` convention that inline structure is
//! suppressed inside code spans.

use std::ops::Range;

use smol_str::SmolStr;

/// A single `[[wiki-link]]` occurrence found by [`scan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// The link's target (the name part before any `|` or `#^`). Empty for
    /// same-page block refs (`[[#^id]]`).
    pub target: SmolStr,
    /// Optional display text (the part after `|`). Does not include the
    /// `#^block-id` suffix.
    pub display: Option<SmolStr>,
    /// Optional block reference (the `^id` part after `#^`).
    pub block: Option<SmolStr>,
    /// Byte range of the full `[[…]]` in the source text.
    pub byte_range: Range<usize>,
}

/// Scans `input` for `[[wiki-link]]` occurrences, skipping code spans.
#[must_use]
pub fn scan(input: &str) -> Vec<WikiLink> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut in_fenced = false;
    let mut fence_marker: Option<&[u8]> = None;
    let mut in_inline = false;

    while i < bytes.len() {
        let rest = &input[i..];

        // Detect fenced code block delimiters (``` or ~~~), line-start only.
        if !in_inline
            && is_at_line_start(bytes, i)
            && let Some((marker, _len)) = match_fence(rest)
        {
            if in_fenced {
                if fence_marker == Some(marker) {
                    in_fenced = false;
                    fence_marker = None;
                }
            } else {
                in_fenced = true;
                fence_marker = Some(marker);
            }
            // Skip past the fence line.
            if let Some(nl) = rest.find('\n') {
                i += nl + 1;
                continue;
            }
            break;
        }

        if in_fenced {
            // Just advance line-by-line.
            match rest.find('\n') {
                Some(nl) => i += nl + 1,
                None => break,
            }
            continue;
        }

        // Inline code toggle: a backtick not preceded by another backtick.
        if rest.as_bytes().first() == Some(&b'`') {
            in_inline = !in_inline;
            i += 1;
            continue;
        }
        if in_inline {
            i += 1;
            continue;
        }

        // Try to match a wiki link starting here.
        if let Some((fields, consumed)) = try_match_link(rest) {
            let start = i;
            let end = i + consumed;
            out.push(WikiLink {
                target: fields.target,
                display: fields.display,
                block: fields.block,
                byte_range: start..end,
            });
            i = end;
            continue;
        }

        i += 1;
    }
    out
}

fn is_at_line_start(bytes: &[u8], i: usize) -> bool {
    i == 0 || bytes[i - 1] == b'\n'
}

fn match_fence(rest: &str) -> Option<(&'static [u8], usize)> {
    let b = rest.as_bytes();
    if b.starts_with(b"```") {
        Some((B_TICKS, 3))
    } else if b.starts_with(b"~~~") {
        Some((B_TILDES, 3))
    } else {
        None
    }
}

const B_TICKS: &[u8] = b"```";
const B_TILDES: &[u8] = b"~~~";

/// Attempts to match a `[[wiki-link]]` at the start of `rest`.
/// Returns `(fields, byte_count_consumed)` on success.
fn try_match_link(rest: &str) -> Option<(WikiLinkFields, usize)> {
    let b = rest.as_bytes();
    if !b.starts_with(b"[[") {
        return None;
    }
    // Find the closing `]]`.
    let after = &rest[2..];
    let close_rel = find_close(after)?;
    let inner = &after[..close_rel];
    if inner.is_empty() {
        return None;
    }
    let total = 2 + close_rel + 2;
    let fields = parse_inner(inner);
    Some((fields, total))
}

/// Locates the first `]]` not nested inside another `[[`. For simplicity we
/// disallow nesting entirely.
fn find_close(s: &str) -> Option<usize> {
    s.find("]]")
}

struct WikiLinkFields {
    target: SmolStr,
    display: Option<SmolStr>,
    block: Option<SmolStr>,
}

fn parse_inner(inner: &str) -> WikiLinkFields {
    // Split off display alias first.
    let (main, display) = inner.find('|').map_or_else(
        || (inner, None),
        |i| (&inner[..i], Some(SmolStr::new(inner[i + 1..].trim()))),
    );
    // Split off block reference (`#^id`).
    let (target, block) = main.find("#^").map_or_else(
        || (SmolStr::new(main.trim()), None),
        |i| {
            let target = main[..i].trim();
            let block = SmolStr::new(main[i + 2..].trim());
            (SmolStr::new(target), Some(block))
        },
    );
    WikiLinkFields {
        target,
        display,
        block,
    }
}

impl WikiLink {
    /// Returns the byte range in the source text.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.byte_range.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_link() {
        let links = scan("hello [[World]] goodbye");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "World");
        assert!(links[0].display.is_none());
        assert!(links[0].block.is_none());
        assert_eq!(links[0].byte_range, 6..15);
    }

    #[test]
    fn display_alias() {
        let links = scan("[[Foo|Some Display]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Foo");
        assert_eq!(links[0].display.as_deref(), Some("Some Display"));
    }

    #[test]
    fn block_reference() {
        let links = scan("[[Foo#^intro]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Foo");
        assert_eq!(links[0].block.as_deref(), Some("intro"));
    }

    #[test]
    fn same_page_block_ref_has_empty_target() {
        let links = scan("[[#^only-a-block]]");
        assert_eq!(links.len(), 1);
        assert!(links[0].target.is_empty());
        assert_eq!(links[0].block.as_deref(), Some("only-a-block"));
    }

    #[test]
    fn skips_fenced_code_block() {
        let input = "before\n```rust\nlet x = [[NotALink]];\n```\nafter [[Real]]";
        let links = scan(input);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Real");
    }

    #[test]
    fn skips_inline_code() {
        let links = scan("text `[[NotALink]]` then [[Yes]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Yes");
    }

    #[test]
    fn multiple_links_in_one_line() {
        let links = scan("[[A]] and [[B]] and [[C]]");
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].target.as_str(), "A");
        assert_eq!(links[1].target.as_str(), "B");
        assert_eq!(links[2].target.as_str(), "C");
    }

    #[test]
    fn no_links_yields_empty() {
        assert!(scan("just plain text").is_empty());
        assert!(scan("").is_empty());
    }

    #[test]
    fn empty_link_is_skipped() {
        assert!(scan("[[]]").is_empty());
    }

    #[test]
    fn unterminated_link_is_skipped() {
        assert!(scan("[[Never closed").is_empty());
    }

    #[test]
    fn tilde_fenced_block_is_skipped() {
        let input = "before\n~~~\n[[Nope]]\n~~~\n[[Yes]]";
        let links = scan(input);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Yes");
    }

    #[test]
    fn display_with_block_ref() {
        let links = scan("[[Foo#^b|Display]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.as_str(), "Foo");
        assert_eq!(links[0].block.as_deref(), Some("b"));
        assert_eq!(links[0].display.as_deref(), Some("Display"));
    }
}
