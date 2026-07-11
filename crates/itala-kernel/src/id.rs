//! Strongly-typed identity types used across the kernel and downstream crates.
//!
//! Every entity in Itala is identified by a newtype wrapper so that a
//! [`NoteId`] can never be confused with a [`VaultId`] at the type level.
//! ULID-backed identifiers are lexicographically sortable and need no
//! coordination; string-backed identifiers carry a human-readable name.
//!
//! All identifier types are `Send + Sync + Unpin` (verified by compile-time
//! assertions at the bottom of this module) and round-trip through
//! [`Display`](std::fmt::Display) ↔ [`FromStr`].

use derive_more::{Display, From, FromStr, Into};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use ulid::Ulid;

/// Uniquely identifies a [`Note`](crate) within a vault.
///
/// ULID-backed, lexicographically sortable. Round-trips through its
/// 26-character base32 string form:
///
/// ```
/// use std::str::FromStr;
/// use itala_kernel::NoteId;
///
/// let id = NoteId::new();
/// let s = id.to_string();
/// let back = NoteId::from_str(&s).unwrap();
/// assert_eq!(id, back);
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Display,
    From,
    Into,
    FromStr,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct NoteId(pub Ulid);

impl NoteId {
    /// Creates a new, randomly-generated identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Creates an identifier from an existing [`Ulid`].
    #[must_use]
    pub const fn from_ulid(id: Ulid) -> Self {
        Self(id)
    }

    /// Returns the underlying [`Ulid`].
    #[must_use]
    pub const fn ulid(&self) -> Ulid {
        self.0
    }
}

/// Uniquely identifies a vault.
///
/// ULID-backed. See [`NoteId`] for round-trip semantics.
///
/// ```
/// use std::str::FromStr;
/// use itala_kernel::VaultId;
///
/// let id = VaultId::new();
/// let back = VaultId::from_str(&id.to_string()).unwrap();
/// assert_eq!(id, back);
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Display,
    From,
    Into,
    FromStr,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct VaultId(pub Ulid);

impl VaultId {
    /// Creates a new, randomly-generated identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Creates an identifier from an existing [`Ulid`].
    #[must_use]
    pub const fn from_ulid(id: Ulid) -> Self {
        Self(id)
    }

    /// Returns the underlying [`Ulid`].
    #[must_use]
    pub const fn ulid(&self) -> Ulid {
        self.0
    }
}

/// Uniquely identifies a link between two notes.
///
/// ULID-backed. See [`NoteId`] for round-trip semantics.
///
/// ```
/// use std::str::FromStr;
/// use itala_kernel::LinkId;
///
/// let id = LinkId::new();
/// let back = LinkId::from_str(&id.to_string()).unwrap();
/// assert_eq!(id, back);
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Display,
    From,
    Into,
    FromStr,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct LinkId(pub Ulid);

impl LinkId {
    /// Creates a new, randomly-generated identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Creates an identifier from an existing [`Ulid`].
    #[must_use]
    pub const fn from_ulid(id: Ulid) -> Self {
        Self(id)
    }

    /// Returns the underlying [`Ulid`].
    #[must_use]
    pub const fn ulid(&self) -> Ulid {
        self.0
    }
}

/// Identifies a block reference target within a note
/// (e.g. the `block-id` in `[[Note#^block-id]]`).
///
/// String-backed. Round-trips trivially through [`Display`](std::fmt::Display):
///
/// ```
/// use std::str::FromStr;
/// use itala_kernel::BlockId;
///
/// let id = BlockId::new("intro");
/// let back = BlockId::from_str(&id.to_string()).unwrap();
/// assert_eq!(id, back);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    FromStr,
    From,
)]
#[serde(transparent)]
#[from(forward)]
pub struct BlockId(pub SmolStr);

impl BlockId {
    /// Creates an identifier from a string slice.
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(SmolStr::new(s))
    }

    /// Creates an identifier from an already-owned [`SmolStr`].
    #[must_use]
    pub const fn from_smol_str(s: SmolStr) -> Self {
        Self(s)
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BlockId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Uniquely identifies an installed plugin (e.g. `"com.example.my-plugin"`).
///
/// String-backed. See [`BlockId`] for round-trip semantics.
///
/// ```
/// use std::str::FromStr;
/// use itala_kernel::PluginId;
///
/// let id = PluginId::new("com.example.my-plugin");
/// let back = PluginId::from_str(&id.to_string()).unwrap();
/// assert_eq!(id, back);
/// ```
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    FromStr,
    From,
)]
#[serde(transparent)]
#[from(forward)]
pub struct PluginId(pub SmolStr);

impl PluginId {
    /// Creates an identifier from a string slice.
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(SmolStr::new(s))
    }

    /// Creates an identifier from an already-owned [`SmolStr`].
    #[must_use]
    pub const fn from_smol_str(s: SmolStr) -> Self {
        Self(s)
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PluginId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// --- Compile-time auto-trait assertions --------------------------------------
//
// Binding the monomorphized check functions to `fn` pointers in `const`
// items forces the trait bound resolution at compile time. If a type ever
// loses `Send`, `Sync`, or `Unpin`, the corresponding constant fails to
// compile.

const _: () = {
    const fn check<T: Send + Sync + Unpin>() {}

    const NOTE_ID: fn() = check::<NoteId>;
    const VAULT_ID: fn() = check::<VaultId>;
    const LINK_ID: fn() = check::<LinkId>;
    const BLOCK_ID: fn() = check::<BlockId>;
    const PLUGIN_ID: fn() = check::<PluginId>;

    // Silence unused-constant warnings while keeping them compile-time-evaluated.
    const _: [(); 0] = [(); {
        let _ = (NOTE_ID, VAULT_ID, LINK_ID, BLOCK_ID, PLUGIN_ID);
        0
    }];
};

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assert_eq;
    use std::str::FromStr;

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config {
            cases: 1024, ..proptest::test_runner::Config::default()
        })]

        #[test]
        fn note_id_roundtrips(raw in 0u128..u128::MAX) {
            let id = NoteId::from_ulid(Ulid::from(raw));
            let s = id.to_string();
            let back = NoteId::from_str(&s).expect("parse");
            prop_assert_eq!(id, back);
        }

        #[test]
        fn vault_id_roundtrips(raw in 0u128..u128::MAX) {
            let id = VaultId::from_ulid(Ulid::from(raw));
            let s = id.to_string();
            let back = VaultId::from_str(&s).expect("parse");
            prop_assert_eq!(id, back);
        }

        #[test]
        fn link_id_roundtrips(raw in 0u128..u128::MAX) {
            let id = LinkId::from_ulid(Ulid::from(raw));
            let s = id.to_string();
            let back = LinkId::from_str(&s).expect("parse");
            prop_assert_eq!(id, back);
        }

        #[test]
        fn block_id_roundtrips(s in ".{0,32}") {
            let id = BlockId::new(&s);
            let back = BlockId::from_str(id.as_ref()).unwrap();
            prop_assert_eq!(id.as_str(), back.as_str());
        }

        #[test]
        fn plugin_id_roundtrips(s in ".{0,32}") {
            let id = PluginId::new(&s);
            let back = PluginId::from_str(id.as_ref()).unwrap();
            prop_assert_eq!(id.as_str(), back.as_str());
        }
    }

    #[test]
    fn note_id_serde_transparent() {
        let id = NoteId::from_ulid(Ulid::from(42));
        let json = serde_json::to_string(&id).unwrap();
        // Transparent: serializes to the bare ULID string, not an object.
        assert!(!json.contains(':'));
        let back: NoteId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn plugin_id_serde_transparent() {
        let id = PluginId::new("com.example.x");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"com.example.x\"");
        let back: PluginId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
