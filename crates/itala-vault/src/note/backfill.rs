//! Lazy `id:` backfill for notes that pre-date the frontmatter identity scheme.

use bytes::Bytes;
use itala_kernel::NoteId;

use crate::frontmatter::Frontmatter;

/// True iff `raw` begins with a YAML frontmatter opening fence (`---\n` or
/// `---\r\n`), optionally preceded by a UTF-8 BOM.
#[must_use]
fn has_frontmatter_block(raw: &[u8]) -> bool {
    const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
    let scan = raw.strip_prefix(BOM).unwrap_or(raw);
    scan.starts_with(b"---\n") || scan.starts_with(b"---\r\n")
}

/// Rewrite `raw` to include `id: <id>` in the YAML frontmatter.
///
/// Two cases:
/// - **No existing frontmatter block** — prepend `---\nid: <id>\n---\n` then
///   `body` verbatim (the caller passes `body == raw` in this case).
/// - **Existing frontmatter block** — clone `fm`, insert `id`, emit the full
///   frontmatter map, and concat with `---\n` fences and the original `body`.
///
/// The body is preserved byte-for-byte.  Key order and comments in the
/// existing frontmatter are not preserved (the parser uses `BTreeMap`).
#[must_use]
pub fn inject_id(raw: &[u8], fm: &Frontmatter, body: &[u8], id: NoteId) -> Bytes {
    if has_frontmatter_block(raw) {
        let mut new_fm = fm.clone();
        new_fm.insert("id".to_owned(), serde_json::Value::String(id.to_string()));
        let emitted = new_fm.emit();
        let mut out = Vec::new();
        out.extend_from_slice(b"---\n");
        out.extend_from_slice(emitted.as_bytes());
        out.extend_from_slice(b"---\n");
        out.extend_from_slice(body);
        Bytes::from(out)
    } else {
        let header = format!("---\nid: {id}\n---\n");
        let mut out = Vec::with_capacity(header.len() + body.len());
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(body);
        Bytes::from(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter;

    fn parse_id_from_bytes(raw: &[u8]) -> NoteId {
        let (fm, _) = frontmatter::split(raw).unwrap();
        let id_val = fm.get("id").expect("id key should be present");
        let s = id_val.as_str().expect("id should be a string");
        s.parse::<NoteId>().expect("id should be a valid NoteId")
    }

    #[test]
    fn inject_id_prepends_when_no_frontmatter() {
        let raw = b"# Hello\nworld\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        assert!(result.starts_with(b"---\n"), "must start with fence");
        assert_eq!(parse_id_from_bytes(&result), id);
    }

    #[test]
    fn inject_id_preserves_body_when_no_frontmatter() {
        let raw = b"# Hello\nworld\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        assert!(
            result.ends_with(b"# Hello\nworld\n"),
            "body must be preserved verbatim"
        );
    }

    #[test]
    fn inject_id_inserts_into_existing_frontmatter() {
        let raw = b"---\ntitle: Alpha\n---\nAlpha body.\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        assert!(fm.get("id").is_none(), "fixture must have no id yet");
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        assert_eq!(parse_id_from_bytes(&result), id);
        // Title must survive.
        let (result_fm, _) = frontmatter::split(&result).unwrap();
        assert_eq!(result_fm.get("title").unwrap(), &serde_json::json!("Alpha"));
    }

    #[test]
    fn inject_id_preserves_body_with_existing_frontmatter() {
        let raw = b"---\ntitle: Alpha\n---\nAlpha body.\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        let (_, result_body) = frontmatter::split(&result).unwrap();
        assert_eq!(result_body, b"Alpha body.\n");
    }

    #[test]
    fn inject_id_result_is_parseable_yaml() {
        let raw = b"---\ntags:\n  - a\n  - b\n---\nbody\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        let (result_fm, _) = frontmatter::split(&result).unwrap();
        assert_eq!(parse_id_from_bytes(&result), id);
        assert_eq!(
            result_fm.get("tags").unwrap(),
            &serde_json::json!(["a", "b"])
        );
    }

    #[test]
    fn inject_id_overwrites_existing_id() {
        let old_id = NoteId::new();
        let raw = format!("---\nid: \"{old_id}\"\n---\nbody\n");
        let raw = raw.as_bytes();
        let (fm, body) = frontmatter::split(raw).unwrap();
        let new_id = NoteId::new();
        let result = inject_id(raw, &fm, body, new_id);
        assert_eq!(parse_id_from_bytes(&result), new_id);
    }

    #[test]
    fn inject_id_handles_empty_body() {
        let raw = b"---\ntitle: hi\n---\n";
        let (fm, body) = frontmatter::split(raw).unwrap();
        let id = NoteId::new();
        let result = inject_id(raw, &fm, body, id);
        let (_, result_body) = frontmatter::split(&result).unwrap();
        assert_eq!(result_body, b"");
    }
}
