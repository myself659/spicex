//! Configuration layer abstractions and priority management.

use crate::error::ConfigResult;
use crate::value::ConfigValue;

/// Trait for configuration layers that provide key-value access.
pub trait ConfigLayer: Send + Sync {
    /// Gets a configuration value by key.
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>>;

    /// Sets a configuration value by key.
    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()>;

    /// Returns all available keys in this layer.
    fn keys(&self) -> Vec<String>;

    /// Returns a human-readable name for this configuration source.
    fn source_name(&self) -> &str;

    /// Returns the priority of this layer for precedence resolution.
    fn priority(&self) -> LayerPriority;

    /// Returns a reference to the layer as Any for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Returns a mutable reference to the layer as Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Priority levels for configuration layers.
/// Lower numeric values have higher precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerPriority {
    /// Explicit set() calls - highest precedence
    Explicit = 0,
    /// Command line flags
    Flags = 1,
    /// Environment variables
    Environment = 2,
    /// Configuration files
    ConfigFile = 3,
    /// Remote key-value stores
    KeyValue = 4,
    /// Default values - lowest precedence
    Defaults = 5,
}

impl LayerPriority {
    /// Returns a human-readable description of the priority level.
    pub fn description(&self) -> &'static str {
        match self {
            LayerPriority::Explicit => "Explicit calls",
            LayerPriority::Flags => "Command line flags",
            LayerPriority::Environment => "Environment variables",
            LayerPriority::ConfigFile => "Configuration files",
            LayerPriority::KeyValue => "Key-value stores",
            LayerPriority::Defaults => "Default values",
        }
    }
}

/// Layer management utilities for sorting and merging configuration layers.
pub mod utils {
    use super::*;
    use std::collections::HashMap;

    /// Sorts configuration layers by priority (highest precedence first).
    ///
    /// # Arguments
    /// * `layers` - A mutable slice of configuration layers to sort
    ///
    /// # Example
    /// ```
    /// use spice::layer::{ConfigLayer, LayerPriority, utils::sort_layers_by_priority};
    /// use spice::value::ConfigValue;
    /// use spice::error::ConfigResult;
    /// use std::collections::HashMap;
    ///
    /// // Mock layer for example
    /// struct MockLayer { priority: LayerPriority }
    /// impl ConfigLayer for MockLayer {
    ///     fn get(&self, _key: &str) -> ConfigResult<Option<ConfigValue>> { Ok(None) }
    ///     fn set(&mut self, _key: &str, _value: ConfigValue) -> ConfigResult<()> { Ok(()) }
    ///     fn keys(&self) -> Vec<String> { vec![] }
    ///     fn source_name(&self) -> &str { "mock" }
    ///     fn priority(&self) -> LayerPriority { self.priority }
    ///     fn as_any(&self) -> &dyn std::any::Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// }
    ///
    /// let mut layers: Vec<Box<dyn ConfigLayer>> = vec![
    ///     Box::new(MockLayer { priority: LayerPriority::Defaults }),
    ///     Box::new(MockLayer { priority: LayerPriority::Explicit }),
    /// ];
    /// // layers will be sorted with highest precedence (lowest numeric value) first
    /// sort_layers_by_priority(&mut layers);
    /// assert_eq!(layers[0].priority(), LayerPriority::Explicit);
    /// ```
    pub fn sort_layers_by_priority(layers: &mut [Box<dyn ConfigLayer>]) {
        layers.sort_by_key(|a| a.priority());
    }

    /// Merges configuration values from multiple layers according to precedence.
    /// Returns the first non-None value found when searching layers in priority order.
    ///
    /// # Arguments
    /// * `layers` - Slice of configuration layers sorted by priority (highest first)
    /// * `key` - The configuration key to search for
    ///
    /// # Returns
    /// * `ConfigResult<Option<ConfigValue>>` - The merged value or None if not found
    ///
    /// # Example
    /// ```
    /// use spice::layer::{ConfigLayer, LayerPriority, utils::merge_value_from_layers};
    /// use spice::value::ConfigValue;
    /// use spice::error::ConfigResult;
    /// use std::collections::HashMap;
    ///
    /// // Mock layer for example
    /// struct MockLayer { data: HashMap<String, ConfigValue> }
    /// impl ConfigLayer for MockLayer {
    ///     fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
    ///         Ok(self.data.get(key).cloned())
    ///     }
    ///     fn set(&mut self, _key: &str, _value: ConfigValue) -> ConfigResult<()> { Ok(()) }
    ///     fn keys(&self) -> Vec<String> { self.data.keys().cloned().collect() }
    ///     fn source_name(&self) -> &str { "mock" }
    ///     fn priority(&self) -> LayerPriority { LayerPriority::ConfigFile }
    ///     fn as_any(&self) -> &dyn std::any::Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// }
    ///
    /// let mut data = HashMap::new();
    /// data.insert("database.host".to_string(), ConfigValue::String("localhost".to_string()));
    /// let layers: Vec<Box<dyn ConfigLayer>> = vec![Box::new(MockLayer { data })];
    /// let value = merge_value_from_layers(&layers, "database.host").unwrap();
    /// assert_eq!(value, Some(ConfigValue::String("localhost".to_string())));
    /// ```
    pub fn merge_value_from_layers(
        layers: &[Box<dyn ConfigLayer>],
        key: &str,
    ) -> ConfigResult<Option<ConfigValue>> {
        for layer in layers {
            match layer.get(key)? {
                Some(value) => return Ok(Some(value)),
                None => continue,
            }
        }
        Ok(None)
    }

    /// Collects all unique keys from multiple configuration layers.
    ///
    /// # Arguments
    /// * `layers` - Slice of configuration layers
    ///
    /// # Returns
    /// * `Vec<String>` - Vector of all unique keys across all layers
    ///
    /// # Example
    /// ```
    /// use spice::layer::{ConfigLayer, LayerPriority, utils::collect_all_keys};
    /// use spice::value::ConfigValue;
    /// use spice::error::ConfigResult;
    /// use std::collections::HashMap;
    ///
    /// // Mock layer for example
    /// struct MockLayer { keys: Vec<String> }
    /// impl ConfigLayer for MockLayer {
    ///     fn get(&self, _key: &str) -> ConfigResult<Option<ConfigValue>> { Ok(None) }
    ///     fn set(&mut self, _key: &str, _value: ConfigValue) -> ConfigResult<()> { Ok(()) }
    ///     fn keys(&self) -> Vec<String> { self.keys.clone() }
    ///     fn source_name(&self) -> &str { "mock" }
    ///     fn priority(&self) -> LayerPriority { LayerPriority::ConfigFile }
    ///     fn as_any(&self) -> &dyn std::any::Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// }
    ///
    /// let layers: Vec<Box<dyn ConfigLayer>> = vec![
    ///     Box::new(MockLayer { keys: vec!["key1".to_string(), "key2".to_string()] }),
    ///     Box::new(MockLayer { keys: vec!["key2".to_string(), "key3".to_string()] }),
    /// ];
    /// let all_keys = collect_all_keys(&layers);
    /// assert_eq!(all_keys.len(), 3); // key1, key2, key3
    /// ```
    pub fn collect_all_keys(layers: &[Box<dyn ConfigLayer>]) -> Vec<String> {
        let mut all_keys = std::collections::HashSet::new();

        for layer in layers {
            for key in layer.keys() {
                all_keys.insert(key);
            }
        }

        let mut keys: Vec<String> = all_keys.into_iter().collect();
        keys.sort();
        keys
    }

    /// Creates a merged view of all configuration values from multiple layers.
    /// Values from higher precedence layers override those from lower precedence layers.
    ///
    /// # Arguments
    /// * `layers` - Slice of configuration layers sorted by priority (highest first)
    ///
    /// # Returns
    /// * `ConfigResult<HashMap<String, ConfigValue>>` - Merged configuration map
    ///
    /// # Example
    /// ```
    /// use spice::layer::{ConfigLayer, LayerPriority, utils::merge_all_layers};
    /// use spice::value::ConfigValue;
    /// use spice::error::ConfigResult;
    /// use std::collections::HashMap;
    ///
    /// // Mock layer for example
    /// struct MockLayer { data: HashMap<String, ConfigValue> }
    /// impl ConfigLayer for MockLayer {
    ///     fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
    ///         Ok(self.data.get(key).cloned())
    ///     }
    ///     fn set(&mut self, _key: &str, _value: ConfigValue) -> ConfigResult<()> { Ok(()) }
    ///     fn keys(&self) -> Vec<String> { self.data.keys().cloned().collect() }
    ///     fn source_name(&self) -> &str { "mock" }
    ///     fn priority(&self) -> LayerPriority { LayerPriority::ConfigFile }
    ///     fn as_any(&self) -> &dyn std::any::Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// }
    ///
    /// let mut data = HashMap::new();
    /// data.insert("key1".to_string(), ConfigValue::String("value1".to_string()));
    /// let layers: Vec<Box<dyn ConfigLayer>> = vec![Box::new(MockLayer { data })];
    /// let merged_config = merge_all_layers(&layers).unwrap();
    /// assert_eq!(merged_config.len(), 1);
    /// ```
    pub fn merge_all_layers(
        layers: &[Box<dyn ConfigLayer>],
    ) -> ConfigResult<HashMap<String, ConfigValue>> {
        let mut merged = HashMap::new();
        let all_keys = collect_all_keys(layers);

        for key in all_keys {
            if let Some(value) = merge_value_from_layers(layers, &key)? {
                merged.insert(key, value);
            }
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock implementation for testing
    struct MockConfigLayer {
        data: HashMap<String, ConfigValue>,
        priority: LayerPriority,
        name: String,
    }

    impl MockConfigLayer {
        fn new(name: &str, priority: LayerPriority) -> Self {
            Self {
                data: HashMap::new(),
                priority,
                name: name.to_string(),
            }
        }

        fn with_value(mut self, key: &str, value: ConfigValue) -> Self {
            self.data.insert(key.to_string(), value);
            self
        }
    }

    impl ConfigLayer for MockConfigLayer {
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
            &self.name
        }

        fn priority(&self) -> LayerPriority {
            self.priority
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_priority_ordering() {
        assert!(LayerPriority::Explicit < LayerPriority::Flags);
        assert!(LayerPriority::Flags < LayerPriority::Environment);
        assert!(LayerPriority::Environment < LayerPriority::ConfigFile);
        assert!(LayerPriority::ConfigFile < LayerPriority::KeyValue);
        assert!(LayerPriority::KeyValue < LayerPriority::Defaults);
    }

    #[test]
    fn test_priority_descriptions() {
        assert_eq!(LayerPriority::Explicit.description(), "Explicit calls");
        assert_eq!(LayerPriority::Defaults.description(), "Default values");
    }

    #[test]
    fn test_sort_layers_by_priority() {
        let mut layers: Vec<Box<dyn ConfigLayer>> = vec![
            Box::new(MockConfigLayer::new("defaults", LayerPriority::Defaults)),
            Box::new(MockConfigLayer::new("explicit", LayerPriority::Explicit)),
            Box::new(MockConfigLayer::new("env", LayerPriority::Environment)),
            Box::new(MockConfigLayer::new("config", LayerPriority::ConfigFile)),
        ];

        utils::sort_layers_by_priority(&mut layers);

        // Should be sorted by priority: Explicit, Environment, ConfigFile, Defaults
        assert_eq!(layers[0].priority(), LayerPriority::Explicit);
        assert_eq!(layers[1].priority(), LayerPriority::Environment);
        assert_eq!(layers[2].priority(), LayerPriority::ConfigFile);
        assert_eq!(layers[3].priority(), LayerPriority::Defaults);
    }

    #[test]
    fn test_merge_value_from_layers_precedence() {
        let layers: Vec<Box<dyn ConfigLayer>> = vec![
            Box::new(
                MockConfigLayer::new("explicit", LayerPriority::Explicit)
                    .with_value("key1", ConfigValue::String("explicit_value".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("env", LayerPriority::Environment)
                    .with_value("key1", ConfigValue::String("env_value".to_string()))
                    .with_value("key2", ConfigValue::String("env_only".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("defaults", LayerPriority::Defaults)
                    .with_value("key1", ConfigValue::String("default_value".to_string()))
                    .with_value("key3", ConfigValue::String("default_only".to_string())),
            ),
        ];

        // key1 should return explicit value (highest precedence)
        let result = utils::merge_value_from_layers(&layers, "key1").unwrap();
        assert_eq!(
            result,
            Some(ConfigValue::String("explicit_value".to_string()))
        );

        // key2 should return env value (only available in env layer)
        let result = utils::merge_value_from_layers(&layers, "key2").unwrap();
        assert_eq!(result, Some(ConfigValue::String("env_only".to_string())));

        // key3 should return default value (only available in defaults layer)
        let result = utils::merge_value_from_layers(&layers, "key3").unwrap();
        assert_eq!(
            result,
            Some(ConfigValue::String("default_only".to_string()))
        );

        // non-existent key should return None
        let result = utils::merge_value_from_layers(&layers, "nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_collect_all_keys() {
        let layers: Vec<Box<dyn ConfigLayer>> = vec![
            Box::new(
                MockConfigLayer::new("layer1", LayerPriority::Explicit)
                    .with_value("key1", ConfigValue::String("value1".to_string()))
                    .with_value("key2", ConfigValue::String("value2".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("layer2", LayerPriority::Environment)
                    .with_value("key2", ConfigValue::String("value2_env".to_string()))
                    .with_value("key3", ConfigValue::String("value3".to_string())),
            ),
        ];

        let keys = utils::collect_all_keys(&layers);
        let mut expected = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        expected.sort();

        assert_eq!(keys, expected);
    }

    #[test]
    fn test_merge_all_layers() {
        let layers: Vec<Box<dyn ConfigLayer>> = vec![
            Box::new(
                MockConfigLayer::new("explicit", LayerPriority::Explicit)
                    .with_value("key1", ConfigValue::String("explicit_value".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("env", LayerPriority::Environment)
                    .with_value("key1", ConfigValue::String("env_value".to_string()))
                    .with_value("key2", ConfigValue::String("env_only".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("defaults", LayerPriority::Defaults)
                    .with_value("key3", ConfigValue::String("default_only".to_string())),
            ),
        ];

        let merged = utils::merge_all_layers(&layers).unwrap();

        // Should have 3 keys total
        assert_eq!(merged.len(), 3);

        // key1 should have explicit value (highest precedence)
        assert_eq!(
            merged.get("key1"),
            Some(&ConfigValue::String("explicit_value".to_string()))
        );

        // key2 should have env value
        assert_eq!(
            merged.get("key2"),
            Some(&ConfigValue::String("env_only".to_string()))
        );

        // key3 should have default value
        assert_eq!(
            merged.get("key3"),
            Some(&ConfigValue::String("default_only".to_string()))
        );
    }

    #[test]
    fn test_layer_precedence_resolution() {
        // Test the complete precedence chain
        let mut layers: Vec<Box<dyn ConfigLayer>> = vec![
            Box::new(
                MockConfigLayer::new("defaults", LayerPriority::Defaults)
                    .with_value("shared_key", ConfigValue::String("default".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("config", LayerPriority::ConfigFile)
                    .with_value("shared_key", ConfigValue::String("config".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("env", LayerPriority::Environment)
                    .with_value("shared_key", ConfigValue::String("env".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("flags", LayerPriority::Flags)
                    .with_value("shared_key", ConfigValue::String("flags".to_string())),
            ),
            Box::new(
                MockConfigLayer::new("explicit", LayerPriority::Explicit)
                    .with_value("shared_key", ConfigValue::String("explicit".to_string())),
            ),
        ];

        // Sort layers by priority
        utils::sort_layers_by_priority(&mut layers);

        // The explicit layer should win
        let result = utils::merge_value_from_layers(&layers, "shared_key").unwrap();
        assert_eq!(result, Some(ConfigValue::String("explicit".to_string())));

        // Remove explicit layer and flags should win
        layers.remove(0);
        let result = utils::merge_value_from_layers(&layers, "shared_key").unwrap();
        assert_eq!(result, Some(ConfigValue::String("flags".to_string())));

        // Remove flags layer and env should win
        layers.remove(0);
        let result = utils::merge_value_from_layers(&layers, "shared_key").unwrap();
        assert_eq!(result, Some(ConfigValue::String("env".to_string())));
    }
}
