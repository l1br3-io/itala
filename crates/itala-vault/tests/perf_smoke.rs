//! Preliminary performance check: 100-note vault open + list < 200 ms.
//!
//! Tagged `#[ignore]` — not part of the standard `cargo nextest` run.
//! Run explicitly with `cargo nextest run -- --ignored` to catch regressions.
//! F9 owns the full benchmark suite.

use camino::Utf8PathBuf;
use itala_vault::test_support::memory_vault;

#[tokio::test]
#[ignore = "performance check; run with --ignored to catch regressions before F9 benchmarks"]
async fn hundred_note_open_and_list_under_200ms() {
    let notes: Vec<(Utf8PathBuf, Vec<u8>)> = (0..100)
        .map(|i| {
            let path = Utf8PathBuf::from(format!("note-{i:03}.md"));
            let content = format!("---\ntitle: Note {i}\n---\nBody {i}.\n").into_bytes();
            (path, content)
        })
        .collect();

    let note_refs: Vec<(Utf8PathBuf, &[u8])> = notes
        .iter()
        .map(|(p, b)| (p.clone(), b.as_slice()))
        .collect();

    let start = std::time::Instant::now();
    let vault = memory_vault(&note_refs).await;
    let ids = vault.list().await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(ids.len(), 100, "expected 100 notes");
    assert!(
        elapsed.as_millis() < 200,
        "open + list took {elapsed:?}, expected < 200 ms"
    );
}
