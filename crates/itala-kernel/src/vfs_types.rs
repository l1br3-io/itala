//! Value types shared across VFS implementations and consumers.
//!
//! [`VfsPath`] is the canonical path type: a [`camino::Utf8PathBuf`] newtype
//! that *rejects absolute paths and `..` traversal at construction time*. Path
//! safety is baked into the type, so a `Vfs` implementation that accepts a
//! [`VfsPath`] never has to re-check for escapes.

use std::fmt;
use std::str::FromStr;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A vault-root-relative path that is guaranteed to contain no `..` traversal
/// and no absolute/leading-root component.
///
/// Construct with [`VfsPath::new`] (validating) or [`VfsPath::join`]. Direct
/// field access is intentionally unavailable so the invariant cannot be
/// violated.
///
/// ```
/// use itala_kernel::vfs_types::VfsPath;
///
/// let p = VfsPath::new("Notes/Welcome.md").unwrap();
/// assert_eq!(p.as_str(), "Notes/Welcome.md");
///
/// // traversal attempts are rejected
/// assert!(VfsPath::new("../etc/passwd").is_err());
/// assert!(VfsPath::new("/etc/passwd").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VfsPath(Utf8PathBuf);

impl VfsPath {
    /// Validates and wraps a path.
    ///
    /// # Errors
    /// Returns [`Error::PathEscape`] if the path is absolute or contains a `..`
    /// component.
    pub fn new(path: impl AsRef<Utf8Path> + Into<Utf8PathBuf>) -> Result<Self> {
        let path = path.into();
        validate(&path)?;
        Ok(Self(path))
    }

    /// Returns the path as a UTF-8 path slice.
    #[must_use]
    pub fn as_utf8_path(&self) -> &Utf8Path {
        &self.0
    }

    /// Returns the path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Appends a relative segment, validating the result stays root-relative.
    ///
    /// # Errors
    /// Returns [`Error::PathEscape`] if the joined result would escape the root.
    pub fn join(&self, segment: impl AsRef<Utf8Path>) -> Result<Self> {
        let segment = segment.as_ref();
        // Reject a segment that is absolute on its own.
        if segment.is_absolute() || segment.components().any(is_escape) {
            return Err(Error::path_escape(segment.as_str()));
        }
        let mut next = self.0.clone();
        next.push(segment);
        // The combination is still safe (base is safe, segment has no `..`),
        // but validate once more to be defensive.
        validate(&next)?;
        Ok(Self(next))
    }

    /// Returns the parent path, if any (the root has no parent).
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        // The parent of a validated path is also valid (no `..` introduced).
        self.0.parent().map(|p| Self(p.to_owned()))
    }

    /// Returns the final component (file/directory name), if any.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.0.file_name()
    }

    /// Consumes and returns the inner [`Utf8PathBuf`].
    #[must_use]
    pub fn into_utf8_path_buf(self) -> Utf8PathBuf {
        self.0
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl FromStr for VfsPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(Utf8PathBuf::from(s))
    }
}

impl AsRef<Utf8Path> for VfsPath {
    fn as_ref(&self) -> &Utf8Path {
        &self.0
    }
}

/// Returns `true` if a single path component is a traversal escape.
const fn is_escape(c: camino::Utf8Component<'_>) -> bool {
    matches!(
        c,
        camino::Utf8Component::ParentDir
            | camino::Utf8Component::RootDir
            | camino::Utf8Component::Prefix(_)
    )
}

/// Validates a path contains no absolute/escape components.
fn validate(path: &Utf8Path) -> Result<()> {
    if path.is_absolute() {
        return Err(Error::path_escape(path.as_str()));
    }
    if path.components().any(is_escape) {
        return Err(Error::path_escape(path.as_str()));
    }
    Ok(())
}

/// File/directory metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Metadata {
    /// Size in bytes (directories report their platform-defined size).
    pub len: u64,
    /// Whether the entry is a directory.
    pub is_dir: bool,
    /// Last modification time, if known.
    pub modified: Option<time::OffsetDateTime>,
}

impl Metadata {
    /// Creates new metadata.
    #[must_use]
    pub const fn new(len: u64, is_dir: bool, modified: Option<time::OffsetDateTime>) -> Self {
        Self {
            len,
            is_dir,
            modified,
        }
    }
}

/// A directory listing entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirEntry {
    /// The full path of the entry relative to the vault root.
    pub path: VfsPath,
    /// Metadata for the entry.
    pub metadata: Metadata,
}

impl DirEntry {
    /// Creates a new directory entry.
    #[must_use]
    pub const fn new(path: VfsPath, metadata: Metadata) -> Self {
        Self { path, metadata }
    }
}

/// A reactive change surfaced by [`crate::vfs::DynVfs::watch`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VfsEvent {
    /// A file or directory was created.
    Created {
        /// The created path.
        path: VfsPath,
    },
    /// A file's contents changed.
    Changed {
        /// The changed path.
        path: VfsPath,
    },
    /// A file or directory was removed.
    Removed {
        /// The removed path.
        path: VfsPath,
    },
    /// A file or directory was renamed/moved.
    Renamed {
        /// Previous path.
        from: VfsPath,
        /// New path.
        to: VfsPath,
    },
}

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}
    const P: fn() = check::<VfsPath>;
    const E: fn() = check::<VfsEvent>;
    const _: [(); 0] = [(); {
        let _ = (P, E);
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert!(VfsPath::new("../etc/passwd").is_err());
        assert!(VfsPath::new("Notes/../../etc/passwd").is_err());
        assert!(VfsPath::new("/etc/passwd").is_err());
        assert!(VfsPath::new("Notes/../..").is_err());
    }

    #[test]
    fn accepts_relative() {
        let p = VfsPath::new("Notes/Ideas/Mornings.md").unwrap();
        assert_eq!(p.as_str(), "Notes/Ideas/Mornings.md");
    }

    #[test]
    fn join_stays_safe() {
        let base = VfsPath::new("Notes").unwrap();
        assert!(base.join("Welcome.md").is_ok());
        assert!(base.join("../etc/passwd").is_err());
        assert!(base.join("/etc/passwd").is_err());
    }

    #[test]
    fn parent_of_valid_path_is_valid() {
        let p = VfsPath::new("Notes/Ideas/x.md").unwrap();
        let parent = p.parent().unwrap();
        assert_eq!(parent.as_str(), "Notes/Ideas");
    }

    #[test]
    fn root_has_no_parent() {
        // A bare filename's parent is the empty (root) path; std returns Some("").
        let p = VfsPath::new("file.md").unwrap();
        assert_eq!(p.parent().unwrap().as_str(), "");
    }

    #[test]
    fn from_str_roundtrips() {
        let p = VfsPath::new("a/b.md").unwrap();
        let back = VfsPath::from_str(p.as_str()).unwrap();
        assert_eq!(p, back);
    }
}
