//! Default configuration layer implementation.

use crate::error::ConfigResult;
use crate::layer::{ConfigLayer, LayerPriority};
use crate::value::ConfigValue;
use std::collections::HashMap;

/// Configuration layer for storing default values.
/// This layer has the lowest precedence in the configuration hierarchy.
#[derive(Debug, Clone)]
pub struct DefaultConfigLayer {
    /// Storage for default configuration values
    data: HashMap<String, ConfigValue>,
}

impl DefaultConfigLayer {
    /// Creates a new empty default configuration layer.
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigLayer};
    ///
    /// let defaults = DefaultConfigLayer::new();
    /// assert_eq!(defaults.keys().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Creates a new default configuration layer with initial values.
    ///
    /// # Arguments
    /// * `defaults` - A HashMap containing the initial default values
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    /// use std::collections::HashMap;
    ///
    /// let mut defaults = HashMap::new();
    /// defaults.insert("host".to_string(), ConfigValue::from("localhost"));
    /// defaults.insert("port".to_string(), ConfigValue::from(8080i64));
    ///
    /// let layer = DefaultConfigLayer::with_defaults(defaults);
    /// assert_eq!(layer.keys().len(), 2);
    /// ```
    pub fn with_defaults(defaults: HashMap<String, ConfigValue>) -> Self {
        Self { data: defaults }
    }

    /// Sets multiple default values at once.
    ///
    /// # Arguments
    /// * `defaults` - A HashMap containing the default values to set
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    /// use std::collections::HashMap;
    ///
    /// let mut layer = DefaultConfigLayer::new();
    /// let mut defaults = HashMap::new();
    /// defaults.insert("timeout".to_string(), ConfigValue::from(30i64));
    /// defaults.insert("retries".to_string(), ConfigValue::from(3i64));
    ///
    /// layer.set_defaults(defaults).unwrap();
    /// assert_eq!(layer.keys().len(), 2);
    /// ```
    pub fn set_defaults(&mut self, defaults: HashMap<String, ConfigValue>) -> ConfigResult<()> {
        for (key, value) in defaults {
            self.data.insert(key, value);
        }
        Ok(())
    }

    /// Clears all default values from the layer.
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    ///
    /// let mut layer = DefaultConfigLayer::new();
    /// layer.set("test", ConfigValue::from("value")).unwrap();
    /// assert_eq!(layer.keys().len(), 1);
    ///
    /// layer.clear();
    /// assert_eq!(layer.keys().len(), 0);
    /// ```
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns the number of default values stored in this layer.
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    ///
    /// let mut layer = DefaultConfigLayer::new();
    /// assert_eq!(layer.len(), 0);
    ///
    /// layer.set("key", ConfigValue::from("value")).unwrap();
    /// assert_eq!(layer.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the layer contains no default values.
    ///
    /// # Example
    /// ```
    /// use spice::DefaultConfigLayer;
    ///
    /// let layer = DefaultConfigLayer::new();
    /// assert!(layer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Checks if a specific key exists in the default values.
    ///
    /// # Arguments
    /// * `key` - The key to check for
    ///
    /// # Returns
    /// * `bool` - True if the key exists, false otherwise
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    ///
    /// let mut layer = DefaultConfigLayer::new();
    /// layer.set("database.host", ConfigValue::from("localhost")).unwrap();
    ///
    /// assert!(layer.contains_key("database.host"));
    /// assert!(!layer.contains_key("database.port"));
    /// ```
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Removes a default value by key.
    ///
    /// # Arguments
    /// * `key` - The key to remove
    ///
    /// # Returns
    /// * `Option<ConfigValue>` - The removed value if it existed, None otherwise
    ///
    /// # Example
    /// ```
    /// use spice::{DefaultConfigLayer, ConfigValue, ConfigLayer};
    ///
    /// let mut layer = DefaultConfigLayer::new();
    /// layer.set("temp_key", ConfigValue::from("temp_value")).unwrap();
    ///
    /// let removed = layer.remove("temp_key");
    /// assert_eq!(removed, Some(ConfigValue::from("temp_value")));
    /// assert!(!layer.contains_key("temp_key"));
    /// ```
    pub fn remove(&mut self, key: &str) -> Option<ConfigValue> {
        self.data.remove(key)
    }
}

impl ConfigLayer for DefaultConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        Ok(self.data.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn source_name(&self) -> &str {
        "defaults"
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::Defaults
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for DefaultConfigLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_layer() {
        let layer = DefaultConfigLayer::new();
        assert!(layer.is_empty());
        assert_eq!(layer.len(), 0);
        assert_eq!(layer.keys().len(), 0);
        assert_eq!(layer.source_name(), "defaults");
        assert_eq!(layer.priority(), LayerPriority::Defaults);
    }

    #[test]
    fn test_with_defaults() {
        let mut defaults = HashMap::new();
        defaults.insert("host".to_string(), ConfigValue::from("localhost"));
        defaults.insert("port".to_string(), ConfigValue::from(8080i64));

        let layer = DefaultConfigLayer::with_defaults(defaults);
        assert_eq!(layer.len(), 2);
        assert!(layer.contains_key("host"));
        assert!(layer.contains_key("port"));

        let host = layer.get("host").unwrap();
        assert_eq!(host, Some(ConfigValue::from("localhost")));
    }

    #[test]
    fn test_set_and_get() {
        let mut layer = DefaultConfigLayer::new();

        // Test setting a value
        layer
            .set("database.host", ConfigValue::from("localhost"))
            .unwrap();
        assert_eq!(layer.len(), 1);

        // Test getting the value
        let value = layer.get("database.host").unwrap();
        assert_eq!(value, Some(ConfigValue::from("localhost")));

        // Test getting non-existent value
        let missing = layer.get("nonexistent").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_set_defaults_bulk() {
        let mut layer = DefaultConfigLayer::new();

        let mut defaults = HashMap::new();
        defaults.insert("timeout".to_string(), ConfigValue::from(30i64));
        defaults.insert("retries".to_string(), ConfigValue::from(3i64));
        defaults.insert("debug".to_string(), ConfigValue::from(false));

        layer.set_defaults(defaults).unwrap();
        assert_eq!(layer.len(), 3);

        assert_eq!(
            layer.get("timeout").unwrap(),
            Some(ConfigValue::from(30i64))
        );
        assert_eq!(layer.get("retries").unwrap(), Some(ConfigValue::from(3i64)));
        assert_eq!(layer.get("debug").unwrap(), Some(ConfigValue::from(false)));
    }

    #[test]
    fn test_keys() {
        let mut layer = DefaultConfigLayer::new();
        layer.set("key1", ConfigValue::from("value1")).unwrap();
        layer.set("key2", ConfigValue::from("value2")).unwrap();

        let mut keys = layer.keys();
        keys.sort(); // Sort for consistent testing

        assert_eq!(keys, vec!["key1".to_string(), "key2".to_string()]);
    }

    #[test]
    fn test_clear() {
        let mut layer = DefaultConfigLayer::new();
        layer.set("key1", ConfigValue::from("value1")).unwrap();
        layer.set("key2", ConfigValue::from("value2")).unwrap();
        assert_eq!(layer.len(), 2);

        layer.clear();
        assert_eq!(layer.len(), 0);
        assert!(layer.is_empty());
        assert_eq!(layer.keys().len(), 0);
    }

    #[test]
    fn test_contains_key() {
        let mut layer = DefaultConfigLayer::new();
        layer
            .set("existing.key", ConfigValue::from("value"))
            .unwrap();

        assert!(layer.contains_key("existing.key"));
        assert!(!layer.contains_key("missing.key"));
    }

    #[test]
    fn test_remove() {
        let mut layer = DefaultConfigLayer::new();
        layer.set("removable", ConfigValue::from("value")).unwrap();
        assert!(layer.contains_key("removable"));

        let removed = layer.remove("removable");
        assert_eq!(removed, Some(ConfigValue::from("value")));
        assert!(!layer.contains_key("removable"));

        // Test removing non-existent key
        let missing = layer.remove("nonexistent");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_overwrite_values() {
        let mut layer = DefaultConfigLayer::new();
        layer.set("key", ConfigValue::from("original")).unwrap();
        assert_eq!(
            layer.get("key").unwrap(),
            Some(ConfigValue::from("original"))
        );

        // Overwrite with new value
        layer.set("key", ConfigValue::from("updated")).unwrap();
        assert_eq!(
            layer.get("key").unwrap(),
            Some(ConfigValue::from("updated"))
        );
        assert_eq!(layer.len(), 1); // Should still be only one key
    }

    #[test]
    fn test_different_value_types() {
        let mut layer = DefaultConfigLayer::new();

        // Test different ConfigValue types
        layer.set("string_val", ConfigValue::from("hello")).unwrap();
        layer.set("int_val", ConfigValue::from(42i64)).unwrap();
        layer.set("float_val", ConfigValue::from(3.14)).unwrap();
        layer.set("bool_val", ConfigValue::from(true)).unwrap();
        layer.set("null_val", ConfigValue::Null).unwrap();

        assert_eq!(
            layer.get("string_val").unwrap(),
            Some(ConfigValue::from("hello"))
        );
        assert_eq!(
            layer.get("int_val").unwrap(),
            Some(ConfigValue::from(42i64))
        );
        assert_eq!(
            layer.get("float_val").unwrap(),
            Some(ConfigValue::from(3.14))
        );
        assert_eq!(
            layer.get("bool_val").unwrap(),
            Some(ConfigValue::from(true))
        );
        assert_eq!(layer.get("null_val").unwrap(), Some(ConfigValue::Null));
    }

    #[test]
    fn test_nested_keys() {
        let mut layer = DefaultConfigLayer::new();

        // Test nested key patterns
        layer
            .set("database.host", ConfigValue::from("localhost"))
            .unwrap();
        layer
            .set("database.port", ConfigValue::from(5432i64))
            .unwrap();
        layer
            .set("server.timeout", ConfigValue::from(30i64))
            .unwrap();

        assert_eq!(
            layer.get("database.host").unwrap(),
            Some(ConfigValue::from("localhost"))
        );
        assert_eq!(
            layer.get("database.port").unwrap(),
            Some(ConfigValue::from(5432i64))
        );
        assert_eq!(
            layer.get("server.timeout").unwrap(),
            Some(ConfigValue::from(30i64))
        );

        let keys = layer.keys();
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn test_layer_priority() {
        let layer = DefaultConfigLayer::new();
        assert_eq!(layer.priority(), LayerPriority::Defaults);

        // Ensure defaults have the lowest priority (highest numeric value)
        assert!(layer.priority() > LayerPriority::Explicit);
        assert!(layer.priority() > LayerPriority::Environment);
        assert!(layer.priority() > LayerPriority::ConfigFile);
    }

    #[test]
    fn test_default_trait() {
        let layer = DefaultConfigLayer::default();
        assert!(layer.is_empty());
        assert_eq!(layer.source_name(), "defaults");
    }

    #[test]
    fn test_clone() {
        let mut original = DefaultConfigLayer::new();
        original.set("key", ConfigValue::from("value")).unwrap();

        let cloned = original.clone();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned.get("key").unwrap(), Some(ConfigValue::from("value")));

        // Ensure they are independent
        original.set("key2", ConfigValue::from("value2")).unwrap();
        assert_eq!(original.len(), 2);
        assert_eq!(cloned.len(), 1); // Clone should not be affected
    }
}
