//! YAML frontmatter parsing.
//!
//! The frontmatter block is delimited by a literal `---` line at byte 0
//! (optionally preceded by a UTF-8 BOM) and a closing `---` line — that is,
//! a line whose only non-whitespace content is `---`. Everything between is
//! YAML; everything after is the note body.

use std::collections::BTreeMap;

use crate::error::{Error, Result};

/// Frontmatter value type.
///
/// Aliased to [`serde_json::Value`] so the kernel doesn't depend on a specific
/// YAML crate's value enum. If saphyr ever ships a native `Value`, the alias is
/// the only thing that needs to change.
pub type FrontmatterValue = serde_json::Value;

/// Parsed YAML frontmatter — a string-keyed map of [`FrontmatterValue`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontmatter(BTreeMap<String, FrontmatterValue>);

impl Frontmatter {
    /// Construct from a pre-built map.
    #[must_use]
    pub const fn new(map: BTreeMap<String, FrontmatterValue>) -> Self {
        Self(map)
    }

    /// An empty frontmatter block.
    #[must_use]
    pub const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Look up a value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&FrontmatterValue> {
        self.0.get(key)
    }

    /// Iterate the (key, value) pairs in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &FrontmatterValue)> {
        self.0.iter()
    }

    /// True when no keys are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of top-level keys.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Borrow the underlying map.
    #[must_use]
    pub const fn as_map(&self) -> &BTreeMap<String, FrontmatterValue> {
        &self.0
    }
}

impl From<BTreeMap<String, FrontmatterValue>> for Frontmatter {
    fn from(m: BTreeMap<String, FrontmatterValue>) -> Self {
        Self(m)
    }
}

/// UTF-8 byte-order mark.
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Split a raw note byte buffer into `(frontmatter, body)`.
///
/// If the buffer (after an optional BOM) starts with a `---` fence line, parse
/// the YAML block; otherwise return `(Frontmatter::empty(), bytes)` with the
/// original buffer (BOM included) as the body. Also public for integration tests
/// and downstream fixture builders; production callers should use
/// the `note::assemble` function (crate-private) instead.
///
/// # Errors
/// Returns [`Error::Frontmatter`] if YAML parsing fails or the closing fence is absent.
pub fn split(bytes: &[u8]) -> Result<(Frontmatter, &[u8])> {
    // Strip BOM for fence detection but remember its width so the returned
    // body slice (when no frontmatter is present) does not include it stripped
    // away silently — we keep semantics: "body is everything after the closing
    // fence; if no fence, body is the original buffer". So we only strip the
    // BOM when looking for the opening fence.
    let scan = if bytes.starts_with(BOM) {
        &bytes[BOM.len()..]
    } else {
        bytes
    };

    let after_open = match scan.strip_prefix(b"---\n") {
        Some(rest) => rest,
        None => match scan.strip_prefix(b"---\r\n") {
            Some(rest) => rest,
            None => return Ok((Frontmatter::empty(), bytes)),
        },
    };

    // Walk `after_open` line by line to find the closing fence — a line whose
    // only non-whitespace content is `---`. Track line endings so we can compute
    // an accurate byte offset for the body slice.
    let mut cursor = 0usize;
    let mut yaml_end: Option<usize> = None;
    let mut body_start: Option<usize> = None;
    while cursor < after_open.len() {
        let rest = &after_open[cursor..];
        let line_end_rel = rest.iter().position(|b| *b == b'\n');
        let (line, advance) =
            line_end_rel.map_or((rest, rest.len()), |pos| (&rest[..pos], pos + 1));
        // Trim CR if CRLF.
        let trimmed_line = if line.last() == Some(&b'\r') {
            &line[..line.len() - 1]
        } else {
            line
        };
        if is_close_fence(trimmed_line) {
            yaml_end = Some(cursor);
            body_start = Some(cursor + advance);
            break;
        }
        cursor += advance;
    }

    let (Some(yaml_end), Some(body_start)) = (yaml_end, body_start) else {
        return Err(Error::Frontmatter {
            line: 0,
            message: "missing closing `---` fence".to_owned(),
        });
    };

    let yaml_bytes = &after_open[..yaml_end];
    let map: BTreeMap<String, FrontmatterValue> = if yaml_bytes.iter().all(u8::is_ascii_whitespace)
    {
        BTreeMap::new()
    } else {
        serde_saphyr::from_slice(yaml_bytes).map_err(|e| Error::Frontmatter {
            line: 0,
            message: e.to_string(),
        })?
    };

    // Body offset: scan was either `bytes` or `bytes[BOM..]`. Adjust.
    let bom_offset = if bytes.starts_with(BOM) { BOM.len() } else { 0 };
    let open_len = if scan.starts_with(b"---\r\n") { 5 } else { 4 };
    let body_offset = bom_offset + open_len + body_start;
    let body = &bytes[body_offset..];

    Ok((Frontmatter::from(map), body))
}

/// True iff this byte-trimmed line is a closing `---` fence: only whitespace
/// surrounding exactly three hyphens.
fn is_close_fence(line: &[u8]) -> bool {
    let Ok(s) = core::str::from_utf8(line) else {
        return false;
    };
    s.trim() == "---"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_str(bytes: &[u8]) -> &str {
        core::str::from_utf8(bytes).unwrap()
    }

    #[test]
    fn no_frontmatter_returns_empty_and_full_body() {
        let raw = b"# Hello\nworld\n";
        let (fm, body) = split(raw).unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, raw);
    }

    #[test]
    fn empty_frontmatter_block() {
        let raw = b"---\n---\nbody\n";
        let (fm, body) = split(raw).unwrap();
        assert!(fm.is_empty());
        assert_eq!(body_str(body), "body\n");
    }

    #[test]
    fn simple_frontmatter() {
        let raw = b"---\ntitle: hello\ncount: 3\n---\nbody\n";
        let (fm, body) = split(raw).unwrap();
        assert_eq!(fm.len(), 2);
        assert_eq!(fm.get("title").unwrap(), &serde_json::json!("hello"));
        assert_eq!(fm.get("count").unwrap(), &serde_json::json!(3));
        assert_eq!(body_str(body), "body\n");
    }

    #[test]
    fn crlf_line_endings() {
        let raw = b"---\r\ntitle: hi\r\n---\r\nbody\r\n";
        let (fm, body) = split(raw).unwrap();
        assert_eq!(fm.get("title").unwrap(), &serde_json::json!("hi"));
        assert_eq!(body_str(body), "body\r\n");
    }

    #[test]
    fn bom_prefixed_input() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"\xEF\xBB\xBF");
        raw.extend_from_slice(b"---\ntitle: bom\n---\nbody\n");
        let (fm, body) = split(&raw).unwrap();
        assert_eq!(fm.get("title").unwrap(), &serde_json::json!("bom"));
        assert_eq!(body_str(body), "body\n");
    }

    #[test]
    fn malformed_yaml_surfaces_frontmatter_error() {
        let raw = b"---\n: : :\n---\n";
        let err = split(raw).unwrap_err();
        assert!(matches!(err, Error::Frontmatter { .. }));
    }

    #[test]
    fn missing_close_fence_is_error() {
        let raw = b"---\ntitle: hi\nbody never closed\n";
        let err = split(raw).unwrap_err();
        assert!(matches!(err, Error::Frontmatter { .. }));
    }

    #[test]
    fn close_fence_must_be_line_alone() {
        // A `---` inside a quoted string spans the line; the fence rule is
        // "line whose only non-whitespace content is `---`". The string here
        // contains `---` mid-line so it should not close.
        let raw = b"---\ntitle: \"a --- b\"\n---\nbody\n";
        let (fm, body) = split(raw).unwrap();
        assert_eq!(fm.get("title").unwrap(), &serde_json::json!("a --- b"));
        assert_eq!(body_str(body), "body\n");
    }

    #[test]
    fn horizontal_rule_in_body_is_not_a_fence() {
        // Opening fence is byte-0 only; a `---` in the body must remain intact.
        let raw = b"# title\n\n---\n\ncontent\n";
        let (fm, body) = split(raw).unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, raw);
    }

    #[test]
    fn nested_and_seq_values() {
        let raw = b"---\ntags:\n  - a\n  - b\nmeta:\n  author: jane\n---\n";
        let (fm, _body) = split(raw).unwrap();
        assert_eq!(fm.get("tags").unwrap(), &serde_json::json!(["a", "b"]));
        assert_eq!(
            fm.get("meta").unwrap(),
            &serde_json::json!({"author": "jane"})
        );
    }

    #[test]
    fn scalar_typing_pins() {
        // saphyr+serde_json scalar typing contract.
        let raw = b"---\ns: \"1\"\ni: 1\nf: 1.5\nb: true\nn: null\n---\n";
        let (fm, _body) = split(raw).unwrap();
        assert_eq!(fm.get("s").unwrap(), &serde_json::json!("1"));
        assert_eq!(fm.get("i").unwrap(), &serde_json::json!(1));
        assert_eq!(fm.get("f").unwrap(), &serde_json::json!(1.5));
        assert_eq!(fm.get("b").unwrap(), &serde_json::json!(true));
        assert_eq!(fm.get("n").unwrap(), &serde_json::Value::Null);
    }
}
