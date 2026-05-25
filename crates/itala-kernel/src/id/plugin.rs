use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::Error;

/// Unique identifier for a plugin, using reverse-domain-name notation.
///
/// Valid format: at least two dot-separated segments of `[a-z][a-z0-9-]*`,
/// e.g. `com.example.mermaid-renderer`. Max 128 characters.
///
/// ```
/// use itala_kernel::PluginId;
///
/// let id = PluginId::new("com.example.search").unwrap();
/// assert_eq!(id.as_str(), "com.example.search");
/// let parsed: PluginId = "org.itala.search".parse().unwrap();
/// assert_eq!(parsed.as_str(), "org.itala.search");
/// ```
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginId(SmolStr);

impl PluginId {
    /// Construct a [`PluginId`] from a reverse-domain-name string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidId`] if `s` does not match RDN format or exceeds
    /// 128 characters.
    pub fn new(s: impl Into<SmolStr>) -> Result<Self, Error> {
        let s: SmolStr = s.into();
        validate(s.as_str())?;
        Ok(Self(s))
    }

    /// Return the plugin ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl FromStr for PluginId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl crate::id::sealed::Sealed for PluginId {}

impl crate::id::Identity for PluginId {
    const KIND: crate::id::IdKind = crate::id::IdKind::Plugin;
}

fn validate(s: &str) -> Result<(), Error> {
    if s.is_empty() || s.len() > 128 {
        return Err(Error::InvalidId {
            kind: crate::id::IdKind::Plugin,
            raw: s.to_owned(),
        });
    }
    // RDN: at least two dot-separated segments of [a-z][a-z0-9-]*
    let mut dot_count = 0usize;
    for segment in s.split('.') {
        if !is_valid_kebab_segment(segment) {
            return Err(Error::InvalidId {
                kind: crate::id::IdKind::Plugin,
                raw: s.to_owned(),
            });
        }
        dot_count += 1;
    }
    if dot_count < 2 {
        return Err(Error::InvalidId {
            kind: crate::id::IdKind::Plugin,
            raw: s.to_owned(),
        });
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
    use proptest::prelude::*;

    proptest::prop_compose! {
        fn valid_plugin_id()(
            seg1 in "[a-z][a-z0-9-]{0,10}",
            seg2 in "[a-z][a-z0-9-]{0,10}",
        ) -> String {
            format!("{seg1}.{seg2}")
        }
    }

    #[test]
    fn valid_samples() {
        assert!(PluginId::new("com.example.foo").is_ok());
        assert!(PluginId::new("org.itala.search").is_ok());
        assert!(PluginId::new("io.github.user.plugin").is_ok());
    }

    #[test]
    fn invalid_samples() {
        assert!(PluginId::new("").is_err());
        assert!(PluginId::new("foo").is_err());
        assert!(PluginId::new("foo.").is_err()); // trailing dot → empty segment
        assert!(PluginId::new(".foo.bar").is_err()); // leading dot → empty segment
        assert!(PluginId::new("Foo.bar").is_err());
        assert!(PluginId::new("1foo.bar").is_err());
        assert!(PluginId::new("foo.bar baz").is_err());
        assert!(PluginId::new("a.b".repeat(100)).is_err()); // > 128
    }

    #[test]
    fn round_trip() {
        let id = PluginId::new("com.example.foo").unwrap();
        let s = id.to_string();
        let parsed: PluginId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn serde_round_trip() {
        let id = PluginId::new("com.example.foo").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: PluginId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    proptest! {
        #[test]
        fn valid_grammar_accepted(s in valid_plugin_id()) {
            prop_assert!(PluginId::new(s).is_ok());
        }
    }
}
