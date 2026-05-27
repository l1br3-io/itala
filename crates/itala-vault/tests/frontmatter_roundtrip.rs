//! Proptest round-trip: serialize a BTreeMap+body into `---\n...\n---\n<body>`,
//! then parse back, and assert keys and body match.

use proptest::prelude::*;
use std::collections::BTreeMap;

fn valid_key() -> impl Strategy<Value = String> {
    // Keys in [a-z][a-z0-9_]{0,15} to keep the YAML tractable.
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s)
}

fn valid_string_value() -> impl Strategy<Value = String> {
    // Arbitrary printable strings — avoiding null bytes and control chars that
    // break serde-saphyr's scalar deserialization.
    "[^\x00-\x1f]{0,40}".prop_map(|s| s)
}

fn build_yaml(map: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (k, v) in map {
        // Wrap value in double-quotes to avoid YAML special char ambiguity.
        let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(k);
        out.push_str(": \"");
        out.push_str(&escaped);
        out.push_str("\"\n");
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..Default::default()
    })]

    #[test]
    fn keys_and_body_roundtrip(
        pairs in prop::collection::btree_map(valid_key(), valid_string_value(), 0..8),
        body in "[^\x00]{0,200}",
    ) {
        // Serialize to frontmatter format.
        let yaml = build_yaml(&pairs);
        let raw = if pairs.is_empty() {
            // No frontmatter — body is the whole thing.
            body.as_bytes().to_vec()
        } else {
            format!("---\n{yaml}---\n{body}").into_bytes()
        };

        let result = itala_vault::frontmatter::split(&raw);

        if pairs.is_empty() {
            // No frontmatter path: should always succeed.
            let (fm, parsed_body) = result.unwrap();
            prop_assert!(fm.is_empty());
            prop_assert_eq!(parsed_body, raw.as_slice());
        } else {
            // With frontmatter: key count must match; body must match.
            match result {
                Ok((fm, parsed_body)) => {
                    prop_assert_eq!(fm.len(), pairs.len(), "key count mismatch");
                    for k in pairs.keys() {
                        prop_assert!(fm.get(k).is_some(), "key {k} missing from parsed frontmatter");
                    }
                    prop_assert_eq!(
                        std::str::from_utf8(parsed_body).unwrap_or(""),
                        body.as_str()
                    );
                }
                Err(e) => {
                    // Acceptable only if the value contained YAML-breaking chars;
                    // our escaping should handle this, so treat as failure.
                    prop_assert!(false, "unexpected parse error: {e}");
                }
            }
        }
    }
}
