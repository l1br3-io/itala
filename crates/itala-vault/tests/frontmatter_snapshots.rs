//! Insta snapshot tests for the five canonical frontmatter fixture files.

use itala_vault::frontmatter;

fn split_fixture(name: &str) -> (itala_vault::Frontmatter, Vec<u8>) {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).expect("fixture not found");
    let (fm, body) = frontmatter::split(&raw).expect("split failed");
    (fm, body.to_vec())
}

#[test]
fn snapshot_01_no_frontmatter() {
    let (fm, body) = split_fixture("01-no-frontmatter.md");
    insta::assert_yaml_snapshot!("01_frontmatter", fm.as_map());
    assert!(std::str::from_utf8(&body).is_ok());
}

#[test]
fn snapshot_02_minimal() {
    let (fm, body) = split_fixture("02-minimal.md");
    insta::assert_yaml_snapshot!("02_frontmatter", fm.as_map());
    assert!(std::str::from_utf8(&body).unwrap().contains("Body text"));
}

#[test]
fn snapshot_03_nested() {
    let (fm, body) = split_fixture("03-nested.md");
    insta::assert_yaml_snapshot!("03_frontmatter", fm.as_map());
    assert!(std::str::from_utf8(&body).unwrap().contains("nested"));
}

#[test]
fn snapshot_04_unicode() {
    let (fm, body) = split_fixture("04-unicode.md");
    insta::assert_yaml_snapshot!("04_frontmatter", fm.as_map());
    assert!(std::str::from_utf8(&body).unwrap().contains("こんにちは"));
}

#[test]
fn snapshot_05_crlf() {
    let (fm, body) = split_fixture("05-crlf.md");
    insta::assert_yaml_snapshot!("05_frontmatter", fm.as_map());
    assert!(std::str::from_utf8(&body).unwrap().contains("Windows body"));
}
