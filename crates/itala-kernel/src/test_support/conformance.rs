//! Shared port-conformance suite — every [`Vfs`] implementation must pass it.
//!
//! Called from each adapter's tests so backends are interchangeable by
//! construction (see architecture doc §7.4).

use bytes::Bytes;

use crate::vfs::Vfs;
use crate::vfs_types::VfsPath;

/// Asserts that `vfs` satisfies the VFS port contract.
///
/// The given `vfs` must be empty (no pre-existing files).
///
/// # Panics
/// Panics on any contract violation (failed read/write/stat/list/rename/remove
/// or incorrect bytes). Intended for use in test code only.
pub async fn assert_vfs_conformance(vfs: &impl Vfs) {
    // write + read round-trip
    let p = VfsPath::new("conformance.md").expect("valid path");
    let data = Bytes::from_static(b"hello itala");
    vfs.write(&p, data.clone()).await.expect("write");
    let back = vfs.read(&p).await.expect("read");
    assert_eq!(back, data, "read bytes should match written bytes");

    // stat
    let meta = vfs.stat(&p).await.expect("stat");
    assert!(!meta.is_dir, "file should not be a dir");
    assert_eq!(meta.len, u64::try_from(data.len()).unwrap());

    // list includes the file at the root
    let entries = vfs
        .list(&VfsPath::new("").expect("root path"))
        .await
        .expect("list");
    assert!(
        entries.iter().any(|e| e.path == p),
        "list should include the written file"
    );

    // rename
    let p2 = VfsPath::new("renamed.md").expect("valid path");
    vfs.rename(&p, &p2).await.expect("rename");
    assert!(vfs.read(&p).await.is_err(), "old path should be gone");
    assert_eq!(
        vfs.read(&p2).await.expect("read renamed"),
        data,
        "renamed file should hold the same bytes"
    );

    // remove
    vfs.remove(&p2).await.expect("remove");
    assert!(vfs.read(&p2).await.is_err(), "removed file should be gone");

    // path escape is rejected at the type level — the adapter never sees it
    assert!(VfsPath::new("../escape").is_err());
    assert!(VfsPath::new("/etc/passwd").is_err());
}
