//! Free-form YAML frontmatter for notes (AD-8 in the M1 plan).
//!
//! Stored as a [`BTreeMap`] of `String → serde_json::Value`. Round-trip
//! semantics: parsing arbitrary YAML produces an equivalent map, and
//! projecting back to YAML produces a deterministic, sorted rendering.

use std::collections::BTreeMap;
use std::fmt;

// --
// thiserror isn't strictly needed for one variant, but matches the codebase
// convention (typed errors) and leaves room to grow.

/// Parse or projection error on frontmatter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FrontmatterError {
    /// YAML parse failure.
    #[error("frontmatter yaml parse error: {0}")]
    Yaml(String),
    /// YAML serialization failure.
    #[error("frontmatter yaml serialize error: {0}")]
    Serialize(String),
}

/// Free-form YAML frontmatter, sorted by key for deterministic output.
///
/// Stored values use [`serde_json::Value`] (works as a generic YAML scalar
/// / seq / map holder) and are round-tripped through `serde-saphyr`. The
/// map is sorted ([`BTreeMap`]) so projection is byte-stable for the same
/// logical content.
///
/// ```
/// use itala_note::Frontmatter;
///
/// let fm = Frontmatter::parse("title: Hello\ntags: [a, b]\n").unwrap();
/// assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
/// let yaml = fm.to_yaml().unwrap();
/// let fm2 = Frontmatter::parse(&yaml).unwrap();
/// assert_eq!(fm, fm2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frontmatter(pub BTreeMap<String, serde_json::Value>);

impl Frontmatter {
    /// Creates an empty frontmatter.
    #[must_use]
    pub const fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Parses a YAML document into a frontmatter map.
    ///
    /// # Errors
    /// [`FrontmatterError::Yaml`] if the input is not valid YAML or does
    /// not deserialize to a mapping (e.g. a bare scalar at the top level).
    pub fn parse(yaml: &str) -> Result<Self, FrontmatterError> {
        let map: BTreeMap<String, serde_json::Value> =
            serde_saphyr::from_str(yaml).map_err(|e| FrontmatterError::Yaml(e.to_string()))?;
        Ok(Self(map))
    }

    /// Serializes the map back to a YAML document (without trailing newline).
    ///
    /// # Errors
    /// [`FrontmatterError::Serialize`] on serialization failure.
    pub fn to_yaml(&self) -> Result<String, FrontmatterError> {
        serde_saphyr::to_string(&self.0).map_err(|e| FrontmatterError::Serialize(e.to_string()))
    }

    /// Returns the value for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Inserts or replaces `key` with `value`.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.0.insert(key.into(), value);
    }

    /// Removes `key` and returns its value, if present.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.0.remove(key)
    }

    /// Returns `true` if no fields are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of fields.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterates over `(key, value)` pairs in sorted key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.0.iter()
    }
}

impl fmt::Display for Frontmatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_yaml() {
            Ok(s) => f.write_str(&s),
            Err(e) => write!(f, "<frontmatter error: {e}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_mapping() {
        let fm = Frontmatter::parse("title: Hello\nscore: 42\n").unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
        assert_eq!(
            fm.get("score").and_then(serde_json::Value::as_i64),
            Some(42)
        );
    }

    #[test]
    fn parse_list_value() {
        let fm = Frontmatter::parse("tags: [a, b, c]\n").unwrap();
        let tags = fm.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].as_str(), Some("a"));
    }

    #[test]
    fn parse_empty_string_is_empty_map() {
        let fm = Frontmatter::parse("").unwrap();
        assert!(fm.is_empty());
    }

    #[test]
    fn round_trip_preserves_semantics() {
        let input = "zeta: 1\nalpha: hello\nmid: [x, y]\n";
        let fm = Frontmatter::parse(input).unwrap();
        let out = fm.to_yaml().unwrap();
        let fm2 = Frontmatter::parse(&out).unwrap();
        assert_eq!(fm, fm2);
    }

    #[test]
    fn set_and_remove() {
        let mut fm = Frontmatter::empty();
        fm.set("k", serde_json::Value::String("v".into()));
        assert_eq!(fm.get("k").and_then(|v| v.as_str()), Some("v"));
        assert_eq!(fm.len(), 1);
        fm.remove("k");
        assert!(fm.is_empty());
    }

    #[test]
    fn scalar_top_level_is_error() {
        // A bare scalar at the top level doesn't deserialize to a mapping.
        assert!(Frontmatter::parse("just-a-string").is_err());
    }
}
