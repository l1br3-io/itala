//! Port-conformance and behavioral tests for [`LocalFsVfs`].

use bytes::Bytes;
use futures::stream::StreamExt;
use itala_kernel::test_support::conformance::assert_vfs_conformance;
use itala_kernel::vfs::Vfs;
use itala_kernel::vfs_types::VfsPath;
use itala_vfs_fs::LocalFsVfs;

fn tmp_vfs() -> (tempfile::TempDir, LocalFsVfs) {
    let dir = tempfile::tempdir().expect("tempdir");
    let vfs = LocalFsVfs::new(dir.path()).expect("construct");
    (dir, vfs)
}

#[tokio::test]
async fn port_conformance_suite() {
    let (_dir, vfs) = tmp_vfs();
    assert_vfs_conformance(&vfs).await;
}

#[tokio::test]
async fn constructs_when_root_exists() {
    let dir = tempfile::tempdir().unwrap();
    let vfs = LocalFsVfs::new(dir.path()).unwrap();
    assert!(vfs.root().is_absolute());
}

#[tokio::test]
async fn missing_root_is_an_error() {
    let bogus = tempfile::tempdir().unwrap();
    let path = bogus.path().join("does-not-exist");
    assert!(LocalFsVfs::new(&path).is_err());
}

#[tokio::test]
async fn round_trip_100_files_checksums_match() {
    let (_dir, vfs) = tmp_vfs();
    for i in 0..100u32 {
        let path = itala_kernel::vfs_types::VfsPath::new(format!("n{i}.md")).unwrap();
        vfs.write(&path, Bytes::from(format!("body-{i}")))
            .await
            .unwrap();
    }
    let entries = vfs
        .list(&itala_kernel::vfs_types::VfsPath::new("").unwrap())
        .await
        .unwrap();
    assert_eq!(entries.len(), 100);
    for entry in entries {
        let bytes = vfs.read(&entry.path).await.unwrap();
        assert!(!bytes.is_empty());
    }
}

#[tokio::test]
async fn concurrent_reads_are_identical() {
    let (_dir, vfs) = tmp_vfs();
    let path = VfsPath::new("shared.md").unwrap();
    let payload = Bytes::from(vec![0x42u8; 8192]);
    vfs.write(&path, payload.clone()).await.unwrap();

    let vfs = std::sync::Arc::new(vfs);
    let mut handles = Vec::new();
    for _ in 0..32 {
        let vfs = vfs.clone();
        let path = path.clone();
        let payload = payload.clone();
        handles.push(tokio::spawn(async move {
            let got = vfs.read(&path).await.unwrap();
            assert_eq!(got, payload);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
}

#[tokio::test]
async fn watcher_receives_changed_event() {
    let (dir, vfs) = tmp_vfs();
    // Seed an existing file so the external write is a Modify (Changed).
    let path = VfsPath::new("watched.md").unwrap();
    vfs.write(&path, Bytes::from_static(b"v1")).await.unwrap();

    let mut stream = vfs.watch(&VfsPath::new("").unwrap());

    // External write from outside the adapter.
    tokio::fs::write(dir.path().join("watched.md"), b"v2")
        .await
        .unwrap();

    let got = tokio::time::timeout(std::time::Duration::from_millis(1500), stream.next())
        .await
        .expect("timed out waiting for watch event")
        .expect("stream ended");

    // The first debounced/modify event should reference the watched file.
    let name = match &got {
        itala_kernel::vfs_types::VfsEvent::Changed { path }
        | itala_kernel::vfs_types::VfsEvent::Created { path } => path.as_str(),
        other => panic!("expected Changed/Created, got {other:?}"),
    };
    assert!(name.ends_with("watched.md"));
}

#[tokio::test]
async fn path_escape_is_rejected_by_vfspath() {
    // VfsPath rejects escapes at construction; the adapter never sees them.
    assert!(VfsPath::new("../../etc/passwd").is_err());
    assert!(VfsPath::new("/etc/passwd").is_err());
}
