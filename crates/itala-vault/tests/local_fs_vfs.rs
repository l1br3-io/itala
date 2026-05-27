//! Integration tests for [`LocalFsVfs`].
//!
//! Watcher tests rely on real OS events and are timing-sensitive. They use a
//! 500 ms timeout (3× the 150 ms debounce window) which is comfortable for
//! fast CI runners; mark `#[ignore]` if flaky on a particular target.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use camino::Utf8Path;
use futures::StreamExt as _;
use itala_kernel::{Vfs, VfsEvent};
use itala_vault::LocalFsVfs;
use tokio::time::timeout;

const WATCHER_TIMEOUT: Duration = Duration::from_millis(500);

// ---------------------------------------------------------------------------
// Watcher: Created
// ---------------------------------------------------------------------------

#[tokio::test]
async fn watcher_fires_created_event() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let vfs = LocalFsVfs::new(root).await.unwrap();

    let mut stream = Vfs::watch(&vfs, root);
    // Brief pause so the watcher has subscribed before the trigger write.
    tokio::time::sleep(Duration::from_millis(50)).await;

    Vfs::write(
        &vfs,
        Utf8Path::new("new_note.md"),
        Bytes::from_static(b"hello"),
    )
    .await
    .unwrap();

    let event = timeout(WATCHER_TIMEOUT, stream.next())
        .await
        .expect("timed out waiting for Created event")
        .expect("stream ended");

    // The atomic write (temp→rename) can surface as Created or Changed depending
    // on the OS and debounce coalescing.
    assert!(
        matches!(event, VfsEvent::Created { .. } | VfsEvent::Changed { .. }),
        "unexpected event: {event:?}"
    );
}

// ---------------------------------------------------------------------------
// Watcher: Changed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn watcher_fires_changed_event() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let vfs = LocalFsVfs::new(root).await.unwrap();

    // Pre-create the file so we know the first event is a change, not create.
    Vfs::write(
        &vfs,
        Utf8Path::new("watch_me.md"),
        Bytes::from_static(b"v1"),
    )
    .await
    .unwrap();

    let mut stream = Vfs::watch(&vfs, root);
    tokio::time::sleep(Duration::from_millis(50)).await;

    Vfs::write(
        &vfs,
        Utf8Path::new("watch_me.md"),
        Bytes::from_static(b"v2"),
    )
    .await
    .unwrap();

    let event = timeout(WATCHER_TIMEOUT, stream.next())
        .await
        .expect("timed out waiting for Changed event")
        .expect("stream ended");

    assert!(
        matches!(event, VfsEvent::Changed { .. } | VfsEvent::Created { .. }),
        "unexpected event: {event:?}"
    );
}

// ---------------------------------------------------------------------------
// Watcher: Deleted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn watcher_fires_deleted_event() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let vfs = LocalFsVfs::new(root).await.unwrap();

    Vfs::write(&vfs, Utf8Path::new("bye.md"), Bytes::from_static(b"gone"))
        .await
        .unwrap();

    let mut stream = Vfs::watch(&vfs, root);
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Remove via the OS directly (bypassing the Vfs layer) to keep it pure.
    std::fs::remove_file(dir.path().join("bye.md")).unwrap();

    let event = timeout(WATCHER_TIMEOUT, stream.next())
        .await
        .expect("timed out waiting for Deleted event")
        .expect("stream ended");

    assert!(
        matches!(event, VfsEvent::Deleted { .. }),
        "unexpected event: {event:?}"
    );
}

// ---------------------------------------------------------------------------
// 32 concurrent readers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn thirty_two_concurrent_readers_return_identical_bytes() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let vfs = Arc::new(LocalFsVfs::new(root).await.unwrap());

    let body = Bytes::from(vec![0xABu8; 4096]);
    vfs.write(Utf8Path::new("big.md"), body.clone())
        .await
        .unwrap();

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..32 {
        let vfs = Arc::clone(&vfs);
        let expected = body.clone();
        tasks.spawn(async move {
            let got = vfs.read(Utf8Path::new("big.md")).await.unwrap();
            assert_eq!(got, expected);
        });
    }
    while let Some(res) = tasks.join_next().await {
        res.unwrap();
    }
}

// ---------------------------------------------------------------------------
// Proptest: 100 random (filename, bytes) round-trips
// ---------------------------------------------------------------------------

use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_random_contents(
        filename in "[a-z][a-z0-9_]{0,15}\\.md",
        content in proptest::collection::vec(any::<u8>(), 0..=4096usize),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ok = rt.block_on(async {
            let dir = tempfile::TempDir::new().unwrap();
            let root = Utf8Path::from_path(dir.path()).unwrap();
            let vfs = LocalFsVfs::new(root).await.unwrap();
            let bytes = Bytes::from(content.clone());
            let path_str = filename.clone();
            let path = Utf8Path::new(&path_str);
            vfs.write(path, bytes.clone()).await.unwrap();
            let got = vfs.read(path).await.unwrap();
            got == bytes
        });
        prop_assert!(ok, "round-trip failed for {filename}");
    }
}
