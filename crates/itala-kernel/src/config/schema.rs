//! Typed configuration schemas registered by plugins and contexts.

use serde::Serialize;
use serde::de::DeserializeOwned;
use smol_str::SmolStr;

/// A typed configuration entry describing one key, its default, and docs.
///
/// The default is carried as a [`toml::Value`] so it round-trips through TOML
/// persistence without loss. Typed reads deserialize through
/// [`ConfigSchema::default_as`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConfigSchema {
    /// The dotted config key (e.g. `"editor.tab_width"`).
    pub key: SmolStr,
    /// The default value used when no file or override is present.
    pub default: toml::Value,
    /// Human-readable description.
    pub description: &'static str,
}

impl ConfigSchema {
    /// Creates a new schema.
    #[must_use]
    pub fn new(key: impl Into<SmolStr>, default: toml::Value, description: &'static str) -> Self {
        Self {
            key: key.into(),
            default,
            description,
        }
    }

    /// Creates a schema whose default is derived from a serializable value.
    ///
    /// # Errors
    /// Returns [`crate::Error::Config`] if `default` cannot be serialized to a
    /// TOML value.
    pub fn with_serialized_default<T>(
        key: impl Into<SmolStr>,
        default: &T,
        description: &'static str,
    ) -> crate::Result<Self>
    where
        T: Serialize,
    {
        let value =
            toml::Value::try_from(default).map_err(|e| crate::Error::config(e.to_string()))?;
        Ok(Self {
            key: key.into(),
            default: value,
            description,
        })
    }

    /// Returns the default deserialized into `T`.
    ///
    /// # Errors
    /// Returns [`crate::Error::Config`] if the default does not deserialize
    /// into `T`.
    pub fn default_as<T: DeserializeOwned>(&self) -> crate::Result<T> {
        from_toml_value(&self.default)
    }
}

/// Deserializes a `T` from a [`toml::Value`].
pub(crate) fn from_toml_value<T: DeserializeOwned>(value: &toml::Value) -> crate::Result<T> {
    use serde::de::IntoDeserializer;
    T::deserialize(value.clone().into_deserializer())
        .map_err(|e| crate::Error::config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_default_roundtrips() {
        let schema = ConfigSchema::new("editor.tab_width", toml::Value::Integer(4), "tab width");
        assert_eq!(schema.default_as::<i64>().unwrap(), 4);
    }

    #[test]
    fn schema_default_from_serializable() {
        #[derive(Serialize)]
        struct Editor {
            tab_width: u32,
        }
        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct EditorCfg {
            tab_width: u32,
        }

        let schema = ConfigSchema::with_serialized_default(
            "editor",
            &Editor { tab_width: 2 },
            "editor settings",
        )
        .unwrap();
        let cfg: EditorCfg = schema.default_as().unwrap();
        assert_eq!(cfg, EditorCfg { tab_width: 2 });
    }
}
