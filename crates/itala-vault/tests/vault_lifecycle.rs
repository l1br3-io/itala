//! End-to-end vault lifecycle integration test: open → list → read → close.

use camino::Utf8PathBuf;
use itala_vault::Error;
use itala_vault::test_support::memory_vault;

const NOTE_A: &[u8] = b"---\ntitle: Alpha\n---\nAlpha body.\n";
const NOTE_B: &[u8] = b"---\ntitle: Beta\n---\nBeta body.\n";
const NOTE_C: &[u8] = b"# No frontmatter\n\nJust text.\n";

async fn build_vault() -> itala_vault::Vault<itala_vault::Open> {
    memory_vault(&[
        (Utf8PathBuf::from("alpha.md"), NOTE_A),
        (Utf8PathBuf::from("beta.md"), NOTE_B),
        (Utf8PathBuf::from("gamma.md"), NOTE_C),
    ])
    .await
}

#[tokio::test]
async fn open_succeeds() {
    let _vault = build_vault().await;
    // If we get here without panic, open succeeded.
}

#[tokio::test]
async fn list_returns_three_sorted_ids() {
    let vault = build_vault().await;
    let ids = vault.list().await.unwrap();
    assert_eq!(ids.len(), 3);
    // IDs must be sorted.
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[tokio::test]
async fn read_returns_correct_note() {
    let vault = build_vault().await;
    let ids = vault.list().await.unwrap();

    // Find the note whose path is "alpha.md" by reading them all.
    let mut alpha = None;
    for id in &ids {
        let note = vault.read(*id).await.unwrap();
        if note.path.as_str() == "alpha.md" {
            alpha = Some(note);
            break;
        }
    }
    let alpha = alpha.expect("alpha.md should be in the list");
    assert_eq!(
        alpha.frontmatter.get("title").unwrap(),
        &serde_json::json!("Alpha")
    );
    assert_eq!(&alpha.body[..], b"Alpha body.\n");
    // Checksum must be stable across two reads.
    let c2 = blake3::hash(NOTE_A);
    assert_eq!(alpha.checksum, c2);
}

#[tokio::test]
async fn path_of_returns_correct_path() {
    let vault = build_vault().await;
    let ids = vault.list().await.unwrap();

    for id in ids {
        let path = vault.path_of(id).await.unwrap();
        assert!(
            path.extension() == Some("md"),
            "path_of should return an .md path"
        );
    }
}

#[tokio::test]
async fn read_unknown_id_returns_not_found() {
    let vault = build_vault().await;
    let unknown = itala_kernel::NoteId::new();
    let err = vault.read(unknown).await.unwrap_err();
    assert!(matches!(err, Error::NotFound { .. }));
}

#[tokio::test]
async fn close_returns_closed_vault() {
    let vault = build_vault().await;
    // `close` is infallible and returns `Vault<Closed>`.
    let _closed = vault.close();
    // If this compiles, the type-state transition worked.
}

#[tokio::test]
async fn concurrent_reads_are_consistent() {
    use std::sync::Arc;
    let vault = Arc::new(build_vault().await);
    let ids = vault.list().await.unwrap();
    assert!(!ids.is_empty());
    let id = ids[0];

    let expected = vault.read(id).await.unwrap().body.clone();

    let mut handles = Vec::new();
    for _ in 0..16 {
        let v = Arc::clone(&vault);
        handles.push(tokio::spawn(async move { v.read(id).await.unwrap().body }));
    }
    for h in handles {
        let body = h.await.unwrap();
        assert_eq!(body, expected);
    }
}
