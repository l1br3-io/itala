//! Capability token types for the Itala permission system.
//!
//! A [`Capability`] is an unforgeable proof that the holder was granted a named
//! permission. Tokens are constructed only by types implementing
//! [`CapabilityGranter`], which is a sealed trait — external crates cannot
//! implement it and thus cannot mint tokens.
//!
//! ```
//! use itala_kernel::CapabilityName;
//!
//! let name = CapabilityName::new("host.vfs.read").unwrap();
//! assert_eq!(name.as_str(), "host.vfs.read");
//! ```

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::{Error, Result};

mod sealed {
    /// Zero-sized private type used to seal [`super::CapabilityGranter`].
    ///
    /// External crates cannot name this type and thus cannot satisfy the
    /// `super::CapabilityGranter: Sealed` bound.
    pub struct Seal(());

    /// Sealing supertrait — prevents external crates from implementing
    /// [`super::CapabilityGranter`].
    pub trait Sealed {}
}

/// The name of a capability, using dotted-kebab notation.
///
/// Valid format: at least two dot-separated segments of `[a-z][a-z0-9-]*`,
/// e.g. `host.vfs.read`, `editor.render-block`.
///
/// ```
/// use itala_kernel::CapabilityName;
///
/// let name = CapabilityName::new("host.events.note-saved").unwrap();
/// assert_eq!(name.as_str(), "host.events.note-saved");
///
/// assert!(CapabilityName::new("host").is_err());
/// assert!(CapabilityName::new("Host.vfs").is_err());
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityName(SmolStr);

impl CapabilityName {
    /// Construct a [`CapabilityName`], validating dotted-kebab format.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCapabilityName`] if `s` does not match the
    /// required format or exceeds 256 characters.
    pub fn new(s: impl Into<SmolStr>) -> Result<Self> {
        let s: SmolStr = s.into();
        validate(s.as_str())?;
        Ok(Self(s))
    }

    /// Return the capability name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl FromStr for CapabilityName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// An unforgeable proof that the holder was granted the named capability.
///
/// Tokens are constructed exclusively through [`CapabilityGranter::mint`].
/// The private `_seal` field blocks struct-literal construction outside this module,
/// and `CapabilityGranter` is sealed so only crate-authorized types can mint tokens.
///
/// `Capability` is intentionally not `Serialize` / `Deserialize` — deserializing
/// a token from untrusted input would undermine the unforgeability guarantee.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Capability {
    name: CapabilityName,
    // Private field blocks struct-literal construction outside this module.
    // PhantomData<sealed::Seal> is zero-sized and names a private type,
    // so external code cannot satisfy the field even with unsafe transmute attempts
    // (well, they can with unsafe, but we forbid unsafe_code workspace-wide).
    _seal: PhantomData<sealed::Seal>,
}

impl Capability {
    pub(crate) const fn new(name: CapabilityName) -> Self {
        Self {
            name,
            _seal: PhantomData::<sealed::Seal>,
        }
    }

    /// Return the name of this capability.
    #[must_use]
    pub const fn name(&self) -> &CapabilityName {
        &self.name
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

/// Sealed trait for types authorized to mint [`Capability`] tokens.
///
/// Only types within `itala-kernel` (and crates permitted via the sealed module)
/// can implement this trait, preventing external code from forging tokens.
pub trait CapabilityGranter: sealed::Sealed {
    /// Mint a [`Capability`] token for the given name.
    fn mint(&self, name: CapabilityName) -> Capability {
        Capability::new(name)
    }
}

fn validate(s: &str) -> Result<()> {
    // At least two dot-separated segments of [a-z][a-z0-9-]*
    if s.is_empty() || s.len() > 256 {
        return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
    }
    let mut dot_count = 0usize;
    for segment in s.split('.') {
        if !is_valid_kebab_segment(segment) {
            return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
        }
        dot_count += 1;
    }
    if dot_count < 2 {
        return Err(Error::InvalidCapabilityName { raw: s.to_owned() });
    }
    Ok(())
}

fn is_valid_kebab_segment(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyGranter;
    impl sealed::Sealed for DummyGranter {}
    impl CapabilityGranter for DummyGranter {}

    #[test]
    fn mint_and_inspect() {
        let granter = DummyGranter;
        let name = CapabilityName::new("host.vfs.read").unwrap();
        let cap = granter.mint(name.clone());
        assert_eq!(cap.name(), &name);
        assert_eq!(cap.to_string(), "host.vfs.read");
    }

    #[test]
    fn capability_name_round_trip() {
        let name = CapabilityName::new("host.events.note-saved").unwrap();
        let s = name.to_string();
        let parsed: CapabilityName = s.parse().unwrap();
        assert_eq!(name, parsed);
    }

    #[test]
    fn capability_name_serde() {
        let name = CapabilityName::new("host.vfs.read").unwrap();
        let json = serde_json::to_string(&name).unwrap();
        let parsed: CapabilityName = serde_json::from_str(&json).unwrap();
        assert_eq!(name, parsed);
    }

    #[test]
    fn capability_clone() {
        let granter = DummyGranter;
        let name = CapabilityName::new("host.vfs.read").unwrap();
        let cap = granter.mint(name);
        let cap2 = cap.clone();
        assert_eq!(cap, cap2);
    }

    #[test]
    fn capability_name_invalid() {
        assert!(CapabilityName::new("").is_err());
        assert!(CapabilityName::new("host").is_err()); // single segment
        assert!(CapabilityName::new("Host.vfs").is_err()); // uppercase
        assert!(CapabilityName::new("host.").is_err()); // trailing dot → empty segment
    }
}
