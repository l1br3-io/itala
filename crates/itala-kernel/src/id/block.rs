use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::Error;
use crate::id::validate;

/// Identifier for a specific block within a note, used in `[[Note#^block-id]]` references.
///
/// Valid characters: ASCII alphanumeric, `_`, `-`. Max 64 characters.
///
/// ```
/// use itala_kernel::BlockId;
///
/// let id = BlockId::new("intro").unwrap();
/// assert_eq!(id.as_str(), "intro");
/// let parsed: BlockId = "my-block-1".parse().unwrap();
/// assert_eq!(parsed.as_str(), "my-block-1");
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BlockId(SmolStr);

impl BlockId {
    /// Construct a [`BlockId`] from a string, validating format constraints.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidId`] if `s` is empty, exceeds 64 characters,
    /// or contains characters outside `[A-Za-z0-9_-]`.
    pub fn new(s: impl Into<SmolStr>) -> Result<Self, Error> {
        let s: SmolStr = s.into();
        validate::block_id(s.as_str())?;
        Ok(Self(s))
    }

    /// Return the block ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl FromStr for BlockId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl crate::id::sealed::Sealed for BlockId {}

impl crate::id::Identity for BlockId {
    const KIND: crate::id::IdKind = crate::id::IdKind::Block;
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn valid_samples() {
        assert!(BlockId::new("intro").is_ok());
        assert!(BlockId::new("my-block-1").is_ok());
        assert!(BlockId::new("UPPER_CASE").is_ok());
        assert!(BlockId::new("a".repeat(64)).is_ok());
    }

    #[test]
    fn invalid_samples() {
        assert!(BlockId::new("").is_err());
        assert!(BlockId::new("has space").is_err());
        assert!(BlockId::new("has/slash").is_err());
        assert!(BlockId::new("has.dot").is_err());
        assert!(BlockId::new("a".repeat(65)).is_err());
        assert!(BlockId::new("café").is_err());
    }

    #[test]
    fn round_trip() {
        let id = BlockId::new("my-block").unwrap();
        let s = id.to_string();
        let parsed: BlockId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn serde_round_trip() {
        let id = BlockId::new("my-block").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: BlockId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    proptest! {
        #[test]
        fn valid_grammar_accepted(s in "[A-Za-z0-9_-]{1,64}") {
            prop_assert!(BlockId::new(s).is_ok());
        }

        #[test]
        fn invalid_grammar_rejected(s in "[^A-Za-z0-9_-]+") {
            prop_assert!(BlockId::new(s).is_err());
        }
    }
}
