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

    /// Insert or replace a key.  Returns the previous value for that key, if any.
    pub fn insert(&mut self, key: String, value: FrontmatterValue) -> Option<FrontmatterValue> {
        self.0.insert(key, value)
    }

    /// Emit the frontmatter as canonical YAML (no fences).
    ///
    /// The output is suitable for wrapping between `---\n` and `---\n` delimiters.
    /// Keys are sorted alphabetically (`BTreeMap` order). Strings are always
    /// double-quoted to prevent type ambiguity on round-trip.  Sequences and
    /// nested maps use block style with two-space indent.
    ///
    /// Empty arrays and empty objects are emitted as `[]` and `{}` (flow style)
    /// so that they survive the parse→emit round-trip.
    #[must_use]
    pub fn emit(&self) -> String {
        let mut out = String::new();
        emit_entries(&mut out, self.0.iter(), 0);
        out
    }
}

// ---------------------------------------------------------------------------
// Private YAML emission helpers
// ---------------------------------------------------------------------------

fn emit_entries<'a>(
    out: &mut String,
    entries: impl Iterator<Item = (&'a String, &'a serde_json::Value)>,
    indent: usize,
) {
    for (key, value) in entries {
        push_indent(out, indent);
        out.push_str(key);
        emit_after_key(out, value, indent);
    }
}

fn emit_after_key(out: &mut String, value: &serde_json::Value, indent: usize) {
    match value {
        serde_json::Value::Array(arr) if arr.is_empty() => {
            out.push_str(": []\n");
        }
        serde_json::Value::Object(obj) if obj.is_empty() => {
            out.push_str(": {}\n");
        }
        serde_json::Value::Array(arr) => {
            out.push_str(":\n");
            for item in arr {
                emit_block_seq_item(out, item, indent + 2);
            }
        }
        serde_json::Value::Object(obj) => {
            out.push_str(":\n");
            emit_entries(out, obj.iter(), indent + 2);
        }
        _ => {
            out.push_str(": ");
            emit_scalar(out, value);
            out.push('\n');
        }
    }
}

fn emit_block_seq_item(out: &mut String, item: &serde_json::Value, indent: usize) {
    match item {
        serde_json::Value::Array(arr) if arr.is_empty() => {
            push_indent(out, indent);
            out.push_str("- []\n");
        }
        serde_json::Value::Object(obj) if obj.is_empty() => {
            push_indent(out, indent);
            out.push_str("- {}\n");
        }
        serde_json::Value::Array(arr) => {
            push_indent(out, indent);
            out.push_str("-\n");
            for nested in arr {
                emit_block_seq_item(out, nested, indent + 2);
            }
        }
        serde_json::Value::Object(obj) => {
            push_indent(out, indent);
            out.push_str("-\n");
            emit_entries(out, obj.iter(), indent + 2);
        }
        _ => {
            push_indent(out, indent);
            out.push_str("- ");
            emit_scalar(out, item);
            out.push('\n');
        }
    }
}

fn emit_scalar(out: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        serde_json::Value::Number(n) => out.push_str(&n.to_string()),
        serde_json::Value::String(s) => {
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        // Arrays and objects are handled by emit_after_key / emit_block_seq_item.
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            unreachable!("non-scalar passed to emit_scalar")
        }
    }
}

fn push_indent(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
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
    use proptest::prelude::*;

    fn body_str(bytes: &[u8]) -> &str {
        core::str::from_utf8(bytes).unwrap()
    }

    // -----------------------------------------------------------------------
    // Frontmatter::insert
    // -----------------------------------------------------------------------

    #[test]
    fn insert_adds_new_key() {
        let mut fm = Frontmatter::empty();
        let old = fm.insert("title".to_owned(), serde_json::json!("hello"));
        assert!(old.is_none());
        assert_eq!(fm.get("title").unwrap(), &serde_json::json!("hello"));
    }

    #[test]
    fn insert_returns_previous_value() {
        let mut fm = Frontmatter::empty();
        fm.insert("x".to_owned(), serde_json::json!(1));
        let old = fm.insert("x".to_owned(), serde_json::json!(2));
        assert_eq!(old, Some(serde_json::json!(1)));
        assert_eq!(fm.get("x").unwrap(), &serde_json::json!(2));
    }

    // -----------------------------------------------------------------------
    // Frontmatter::emit — unit tests for each scalar type and structure
    // -----------------------------------------------------------------------

    fn parse_emit(fm: &Frontmatter) -> Frontmatter {
        let yaml = fm.emit();
        let map: std::collections::BTreeMap<String, serde_json::Value> =
            if yaml.is_empty() || yaml.bytes().all(|b| b.is_ascii_whitespace()) {
                std::collections::BTreeMap::new()
            } else {
                serde_saphyr::from_str(&yaml).expect("emit produced invalid YAML")
            };
        Frontmatter::new(map)
    }

    #[test]
    fn emit_empty_frontmatter() {
        let fm = Frontmatter::empty();
        assert_eq!(fm.emit(), "");
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_null_value() {
        let mut fm = Frontmatter::empty();
        fm.insert("n".to_owned(), serde_json::Value::Null);
        assert!(fm.emit().contains("n: null"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_bool_true() {
        let mut fm = Frontmatter::empty();
        fm.insert("flag".to_owned(), serde_json::json!(true));
        assert!(fm.emit().contains("flag: true"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_bool_false() {
        let mut fm = Frontmatter::empty();
        fm.insert("flag".to_owned(), serde_json::json!(false));
        assert!(fm.emit().contains("flag: false"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_integer() {
        let mut fm = Frontmatter::empty();
        fm.insert("count".to_owned(), serde_json::json!(42i64));
        assert!(fm.emit().contains("count: 42"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_negative_integer() {
        let mut fm = Frontmatter::empty();
        fm.insert("n".to_owned(), serde_json::json!(-7i64));
        assert!(fm.emit().contains("n: -7"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_float() {
        let mut fm = Frontmatter::empty();
        fm.insert("ratio".to_owned(), serde_json::json!(1.5f64));
        assert!(fm.emit().contains("ratio: 1.5"));
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_plain_string() {
        let mut fm = Frontmatter::empty();
        fm.insert("title".to_owned(), serde_json::json!("hello world"));
        let yaml = fm.emit();
        assert!(
            yaml.contains("\"hello world\""),
            "expected double-quoted string in: {yaml}"
        );
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_string_that_looks_like_bool_survives_roundtrip() {
        // "true" as a string must not become a bool after round-trip.
        let mut fm = Frontmatter::empty();
        fm.insert("s".to_owned(), serde_json::json!("true"));
        assert_eq!(
            parse_emit(&fm).get("s").unwrap(),
            &serde_json::json!("true")
        );
    }

    #[test]
    fn emit_string_that_looks_like_null_survives_roundtrip() {
        let mut fm = Frontmatter::empty();
        fm.insert("s".to_owned(), serde_json::json!("null"));
        assert_eq!(
            parse_emit(&fm).get("s").unwrap(),
            &serde_json::json!("null")
        );
    }

    #[test]
    fn emit_string_with_quote_and_backslash() {
        let mut fm = Frontmatter::empty();
        fm.insert("s".to_owned(), serde_json::json!("say \"hi\" \\now"));
        assert_eq!(
            parse_emit(&fm).get("s").unwrap(),
            &serde_json::json!("say \"hi\" \\now")
        );
    }

    #[test]
    fn emit_string_with_newline() {
        let mut fm = Frontmatter::empty();
        fm.insert("s".to_owned(), serde_json::json!("line1\nline2"));
        assert_eq!(
            parse_emit(&fm).get("s").unwrap(),
            &serde_json::json!("line1\nline2")
        );
    }

    #[test]
    fn emit_sequence_of_strings() {
        let mut fm = Frontmatter::empty();
        fm.insert("tags".to_owned(), serde_json::json!(["a", "b", "c"]));
        let yaml = fm.emit();
        assert!(
            yaml.contains("tags:\n"),
            "expected block sequence in: {yaml}"
        );
        assert!(
            yaml.contains("- \"a\"") || yaml.contains("- \"b\""),
            "items: {yaml}"
        );
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_empty_sequence() {
        let mut fm = Frontmatter::empty();
        fm.insert("tags".to_owned(), serde_json::json!([]));
        let yaml = fm.emit();
        assert!(
            yaml.contains("tags: []"),
            "expected flow empty seq in: {yaml}"
        );
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_nested_map() {
        let mut fm = Frontmatter::empty();
        fm.insert(
            "meta".to_owned(),
            serde_json::json!({"author": "jane", "year": 2024}),
        );
        let yaml = fm.emit();
        assert!(
            yaml.contains("meta:\n"),
            "expected nested block map in: {yaml}"
        );
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_empty_map() {
        let mut fm = Frontmatter::empty();
        fm.insert("meta".to_owned(), serde_json::json!({}));
        let yaml = fm.emit();
        assert!(
            yaml.contains("meta: {}"),
            "expected flow empty map in: {yaml}"
        );
        assert_eq!(parse_emit(&fm), fm);
    }

    #[test]
    fn emit_keys_sorted_alphabetically() {
        let mut fm = Frontmatter::empty();
        fm.insert("z".to_owned(), serde_json::json!(1));
        fm.insert("a".to_owned(), serde_json::json!(2));
        fm.insert("m".to_owned(), serde_json::json!(3));
        let yaml = fm.emit();
        let a_pos = yaml.find("a:").unwrap();
        let m_pos = yaml.find("m:").unwrap();
        let z_pos = yaml.find("z:").unwrap();
        assert!(
            a_pos < m_pos && m_pos < z_pos,
            "keys must be alphabetical: {yaml}"
        );
    }

    // -----------------------------------------------------------------------
    // Property test: parse(emit(fm)) == fm
    // -----------------------------------------------------------------------

    fn arb_safe_string() -> impl Strategy<Value = serde_json::Value> {
        // Printable ASCII — avoids control chars.  Keywords (true, null, 1) are
        // fine because the emitter always double-quotes strings.
        "[a-zA-Z0-9 _.!?:/-]{0,20}".prop_map(serde_json::Value::String)
    }

    fn arb_json_leaf() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            Just(serde_json::Value::Null),
            any::<bool>().prop_map(serde_json::Value::Bool),
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            arb_safe_string(),
        ]
    }

    fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
        let leaf = arb_json_leaf();
        leaf.prop_recursive(2, 8, 4, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::Array),
                proptest::collection::btree_map("[a-zA-Z][a-zA-Z0-9_]{0,10}", inner, 0..4,)
                    .prop_map(|m| serde_json::Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        #[test]
        fn emit_roundtrip(
            map in proptest::collection::btree_map(
                "[a-zA-Z][a-zA-Z0-9_]{0,10}",
                arb_json_value(),
                0..8,
            )
        ) {
            let fm = Frontmatter::new(map);
            prop_assert_eq!(parse_emit(&fm), fm);
        }
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
