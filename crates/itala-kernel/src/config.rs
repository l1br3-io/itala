//! Typed, layered configuration store.
//!
//! Layers, in increasing precedence:
//! 1. **defaults** — declared via registered [`ConfigSchema`]s;
//! 2. **file** — values loaded from a `config.toml` document;
//! 3. **runtime overrides** — set programmatically.
//!
//! Internally all three layers are normalized to *flat dotted-key → leaf
//! value* maps; a schema whose default is a table is flattened into its leaf
//! components at registration time. The effective value for a key resolves
//! per-leaf with the precedence above; a section query (e.g. `"editor"`)
//! reassembles its leaves back into a table. Changes publish a
//! [`KernelEvent::ConfigChanged`] on the optional event bus, and the whole
//! config persists to TOML for durability.

use std::collections::{BTreeMap, HashMap};

use serde::de::DeserializeOwned;

use crate::bus::EventBus;
use crate::config::schema::{ConfigSchema, from_toml_value};
use crate::events::KernelEvent;
use crate::{Error, Result};

pub mod schema;

/// Layered configuration registry.
pub struct ConfigRegistry {
    schemas: HashMap<String, ConfigSchema>,
    defaults: BTreeMap<String, toml::Value>,
    file: BTreeMap<String, toml::Value>,
    overrides: BTreeMap<String, toml::Value>,
    bus: Option<EventBus>,
}

impl ConfigRegistry {
    /// Creates an empty registry with no event bus.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            defaults: BTreeMap::new(),
            file: BTreeMap::new(),
            overrides: BTreeMap::new(),
            bus: None,
        }
    }

    /// Creates a registry that publishes [`KernelEvent::ConfigChanged`] on the
    /// given bus whenever a value is loaded or overridden.
    #[must_use]
    pub fn with_bus(bus: EventBus) -> Self {
        Self {
            schemas: HashMap::new(),
            defaults: BTreeMap::new(),
            file: BTreeMap::new(),
            overrides: BTreeMap::new(),
            bus: Some(bus),
        }
    }

    /// Registers a typed schema. If the schema's default is a table, it is
    /// flattened into leaf entries under `schema.key`.
    pub fn register(&mut self, schema: ConfigSchema) {
        let key = schema.key.to_string();
        flatten_value(&key, schema.default.clone(), &mut self.defaults);
        self.schemas.insert(key, schema);
    }

    /// Loads layered values from a TOML document string.
    ///
    /// Nested tables and dotted keys are both flattened into dotted leaf keys.
    ///
    /// # Errors
    /// Returns [`Error::Config`] if the string is not valid TOML.
    pub fn load_toml(&mut self, toml_str: &str) -> Result<()> {
        let table: toml::Table =
            toml::from_str(toml_str).map_err(|e| Error::config(format!("invalid TOML: {e}")))?;

        let mut flat = BTreeMap::new();
        flatten_table("", &table, &mut flat);
        self.file = flat;

        if let Some(bus) = &self.bus {
            for key in self.file.keys() {
                bus.publish(KernelEvent::ConfigChanged { key: key.clone() });
            }
        }
        Ok(())
    }

    /// Sets a runtime override for `key` (a table value is flattened into its
    /// leaves) and publishes a change notification.
    ///
    /// # Errors
    /// Returns [`Error::Config`] if `value` cannot be serialized to TOML.
    pub fn set_override<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let toml_val = toml::Value::try_from(value).map_err(|e| Error::config(e.to_string()))?;
        flatten_value(key, toml_val, &mut self.overrides);

        if let Some(bus) = &self.bus {
            bus.publish(KernelEvent::ConfigChanged {
                key: key.to_string(),
            });
        }
        Ok(())
    }

    /// Returns the effective [`toml::Value`] for `key`, or `None` if unknown.
    ///
    /// Both leaf keys (`"editor.tab_width"`) and section keys (`"editor"`,
    /// reassembled from its leaves) are accepted.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<toml::Value> {
        let merged = self.merged();
        if let Some(v) = merged.get(key) {
            return Some(v.clone());
        }
        reassemble(key, &merged)
    }

    /// Returns the effective value for `key` deserialized into `T`.
    ///
    /// # Errors
    /// [`Error::Config`] if the key is unknown or the value does not
    /// deserialize into `T`.
    pub fn get_typed<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        let Some(value) = self.get(key) else {
            return Err(Error::config(format!("unknown config key: {key}")));
        };
        from_toml_value(&value)
    }

    /// Serializes the effective merged configuration to a TOML document string.
    ///
    /// # Errors
    /// [`Error::Config`] if serialization fails.
    pub fn save_toml(&self) -> Result<String> {
        let merged = self.merged();
        let table = unflatten(&merged);
        toml::to_string(&table).map_err(|e| Error::config(e.to_string()))
    }

    /// Returns whether a schema has been registered for `key`.
    #[must_use]
    pub fn has_schema(&self, key: &str) -> bool {
        self.schemas.contains_key(key)
    }

    /// Computes the per-leaf merged config (defaults < file < overrides).
    fn merged(&self) -> BTreeMap<String, toml::Value> {
        let mut out = self.defaults.clone();
        for (k, v) in &self.file {
            out.insert(k.clone(), v.clone());
        }
        for (k, v) in &self.overrides {
            out.insert(k.clone(), v.clone());
        }
        out
    }
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Flattens a value (table or leaf) at `key` into leaf entries in `out`.
fn flatten_value(key: &str, value: toml::Value, out: &mut BTreeMap<String, toml::Value>) {
    match value {
        toml::Value::Table(t) => flatten_table(key, &t, out),
        other => {
            out.insert(key.to_string(), other);
        }
    }
}

/// Recursively flattens a nested TOML table into dotted leaf keys.
fn flatten_table(prefix: &str, table: &toml::Table, out: &mut BTreeMap<String, toml::Value>) {
    for (k, v) in table {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::Table(t) => flatten_table(&key, t, out),
            other => {
                out.insert(key, other.clone());
            }
        }
    }
}

/// Reassembles a section (`prefix`) from its leaf entries into a table value.
fn reassemble(prefix: &str, map: &BTreeMap<String, toml::Value>) -> Option<toml::Value> {
    let needle = format!("{prefix}.");
    let mut sub = BTreeMap::new();
    for (k, v) in map {
        if let Some(rest) = k.strip_prefix(&needle) {
            sub.insert(rest.to_string(), v.clone());
        }
    }
    if sub.is_empty() {
        return None;
    }
    Some(toml::Value::Table(unflatten(&sub)))
}

/// Rebuilds a nested TOML table from dotted keys.
fn unflatten(flat: &BTreeMap<String, toml::Value>) -> toml::Table {
    let mut root = toml::Table::new();
    for (key, val) in flat {
        let parts: Vec<&str> = key.split('.').collect();
        let mut cur = &mut root;
        for (idx, part) in parts.iter().enumerate() {
            if idx + 1 == parts.len() {
                cur.insert((*part).to_string(), val.clone());
            } else {
                let entry = cur
                    .entry((*part).to_string())
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()));
                cur = entry
                    .as_table_mut()
                    .expect("dotted-key prefix collided with a non-table value");
            }
        }
    }
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::EventBus;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct EditorCfg {
        tab_width: u32,
        theme: String,
    }

    fn register_editor(reg: &mut ConfigRegistry) {
        reg.register(
            ConfigSchema::with_serialized_default(
                "editor",
                &EditorCfg {
                    tab_width: 4,
                    theme: "dark".to_string(),
                },
                "editor settings",
            )
            .unwrap(),
        );
    }

    #[test]
    fn schema_default_is_returned_when_nothing_loaded() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        let cfg: EditorCfg = reg.get_typed("editor").unwrap();
        assert_eq!(cfg.tab_width, 4);
        assert_eq!(cfg.theme, "dark");
    }

    #[test]
    fn load_toml_overrides_defaults() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        reg.load_toml("[editor]\ntab_width = 2\ntheme = \"light\"\n")
            .unwrap();
        let cfg: EditorCfg = reg.get_typed("editor").unwrap();
        assert_eq!(cfg.tab_width, 2);
        assert_eq!(cfg.theme, "light");
    }

    #[test]
    fn runtime_override_wins_and_persists() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        reg.load_toml("[editor]\ntab_width = 2\ntheme = \"light\"\n")
            .unwrap();
        reg.set_override(
            "editor",
            &EditorCfg {
                tab_width: 8,
                theme: "solarized".to_string(),
            },
        )
        .unwrap();

        let cfg: EditorCfg = reg.get_typed("editor").unwrap();
        assert_eq!(cfg.tab_width, 8);
        assert_eq!(cfg.theme, "solarized");

        let saved = reg.save_toml().unwrap();
        assert!(saved.contains("tab_width = 8"));
        assert!(saved.contains("solarized"));
    }

    #[test]
    fn invalid_toml_returns_config_error() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        let err = reg.load_toml("this is = = not toml").unwrap_err();
        assert!(matches!(err, Error::Config(_)), "got {err:?}");
    }

    #[test]
    fn unknown_key_query_returns_config_error() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        let result = reg.get_typed::<i64>("nope.does.not.exist");
        assert!(matches!(result, Err(Error::Config(_))));
    }

    #[tokio::test]
    async fn change_notification_fires_on_override() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        let mut reg = ConfigRegistry::with_bus(bus);
        register_editor(&mut reg);

        reg.set_override(
            "editor",
            &EditorCfg {
                tab_width: 1,
                theme: "x".to_string(),
            },
        )
        .unwrap();

        match sub.recv().await.unwrap() {
            KernelEvent::ConfigChanged { key } => assert_eq!(key, "editor"),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[test]
    fn round_trip_save_drop_reload_values_match() {
        let doc = {
            let mut reg = ConfigRegistry::new();
            register_editor(&mut reg);
            reg.load_toml("[editor]\ntab_width = 2\ntheme = \"light\"\n")
                .unwrap();
            reg.save_toml().unwrap()
        };

        let mut reg2 = ConfigRegistry::new();
        register_editor(&mut reg2);
        reg2.load_toml(&doc).unwrap();

        let cfg: EditorCfg = reg2.get_typed("editor").unwrap();
        assert_eq!(cfg.tab_width, 2);
        assert_eq!(cfg.theme, "light");
    }

    #[test]
    fn dotted_leaf_schema_roundtrips() {
        let mut reg = ConfigRegistry::new();
        reg.register(ConfigSchema::new(
            "a.b.c",
            toml::Value::Integer(0),
            "nested",
        ));
        reg.load_toml("a.b.c = 42").unwrap();
        assert_eq!(reg.get_typed::<i64>("a.b.c").unwrap(), 42);
        let doc = reg.save_toml().unwrap();
        assert!(doc.contains("c = 42"));
    }

    #[test]
    fn leaf_override_within_section() {
        let mut reg = ConfigRegistry::new();
        register_editor(&mut reg);
        reg.load_toml("[editor]\ntab_width = 2\ntheme = \"light\"\n")
            .unwrap();
        // Override a single leaf; the other stays from the file.
        reg.set_override("editor.tab_width", &16u32).unwrap();
        let cfg: EditorCfg = reg.get_typed("editor").unwrap();
        assert_eq!(cfg.tab_width, 16);
        assert_eq!(cfg.theme, "light");
    }
}
