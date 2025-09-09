//! Environment variable configuration layer implementation.

use crate::error::ConfigResult;
use crate::layer::{ConfigLayer, LayerPriority};
use crate::value::ConfigValue;
use std::collections::HashMap;
use std::env;

/// Configuration layer that reads from environment variables.
///
/// This layer provides automatic environment variable discovery and key transformation
/// to support various naming conventions and nested structures.
pub struct EnvConfigLayer {
    /// Optional prefix for environment variable names
    prefix: Option<String>,

    /// Custom key transformation function
    key_replacer: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,

    /// Cached environment variables for performance
    cached_vars: HashMap<String, String>,

    /// Whether to automatically discover environment variables
    automatic: bool,
}

impl EnvConfigLayer {
    /// Creates a new environment variable configuration layer.
    ///
    /// # Arguments
    /// * `prefix` - Optional prefix to filter environment variables
    /// * `automatic` - Whether to automatically discover all matching environment variables
    ///
    /// # Example
    /// ```
    /// use spice::env_layer::EnvConfigLayer;
    ///
    /// // Create layer with prefix "APP_"
    /// let env_layer = EnvConfigLayer::new(Some("APP".to_string()), true);
    ///
    /// // Create layer without prefix
    /// let env_layer = EnvConfigLayer::new(None, false);
    /// ```
    pub fn new(prefix: Option<String>, automatic: bool) -> Self {
        let mut layer = Self {
            prefix,
            key_replacer: None,
            cached_vars: HashMap::new(),
            automatic,
        };

        if automatic {
            layer.refresh_cache();
        }

        layer
    }

    /// Sets a custom key replacement function for transforming configuration keys
    /// to environment variable names.
    ///
    /// # Arguments
    /// * `replacer` - Function that takes a config key and returns an env var name
    ///
    /// # Example
    /// ```
    /// use spice::env_layer::EnvConfigLayer;
    ///
    /// let mut env_layer = EnvConfigLayer::new(None, false);
    /// env_layer.set_key_replacer(Box::new(|key: &str| {
    ///     key.replace(".", "__").to_uppercase()
    /// }));
    /// ```
    pub fn set_key_replacer<F>(&mut self, replacer: Box<F>)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.key_replacer = Some(replacer);
    }

    /// Refreshes the cached environment variables.
    /// This is automatically called when `automatic` is true during construction.
    pub fn refresh_cache(&mut self) {
        self.cached_vars.clear();

        for (key, value) in env::vars() {
            if let Some(ref prefix) = self.prefix {
                if key.starts_with(&format!("{prefix}_")) {
                    // Remove prefix and convert to config key format
                    let config_key = key
                        .strip_prefix(&format!("{prefix}_"))
                        .unwrap()
                        .to_lowercase()
                        .replace("_", ".");
                    self.cached_vars.insert(config_key, value);
                }
            } else if self.automatic {
                // Convert all env vars to config key format
                let config_key = key.to_lowercase().replace("_", ".");
                self.cached_vars.insert(config_key, value);
            }
        }
    }

    /// Transforms a configuration key to an environment variable name.
    ///
    /// This method applies the following transformations:
    /// 1. Convert to uppercase
    /// 2. Replace dots with underscores
    /// 3. Apply custom key replacer if set
    /// 4. Add prefix if configured
    ///
    /// # Arguments
    /// * `key` - The configuration key to transform
    ///
    /// # Returns
    /// The transformed environment variable name
    ///
    /// # Example
    /// ```
    /// use spice::env_layer::EnvConfigLayer;
    ///
    /// let env_layer = EnvConfigLayer::new(Some("APP".to_string()), false);
    /// let env_var = env_layer.transform_key("database.host");
    /// assert_eq!(env_var, "APP_DATABASE_HOST");
    /// ```
    pub fn transform_key(&self, key: &str) -> String {
        // Start with basic transformation: lowercase to uppercase, dots to underscores
        let mut env_key = key.to_uppercase().replace(".", "_");

        // Apply custom key replacer if set
        if let Some(ref replacer) = self.key_replacer {
            env_key = replacer(&env_key);
        }

        // Add prefix if configured
        if let Some(ref prefix) = self.prefix {
            format!("{prefix}_{env_key}")
        } else {
            env_key
        }
    }

    /// Gets an environment variable value by its name.
    ///
    /// # Arguments
    /// * `env_var_name` - The environment variable name
    ///
    /// # Returns
    /// The environment variable value wrapped in ConfigValue::String, or None if not found
    fn get_env_var(&self, env_var_name: &str) -> Option<ConfigValue> {
        env::var(env_var_name).ok().map(ConfigValue::String)
    }

    /// Attempts to parse a string value into a more specific ConfigValue type.
    ///
    /// This method tries to intelligently convert string values from environment
    /// variables into appropriate types (integers, floats, booleans).
    ///
    /// # Arguments
    /// * `value` - The string value to parse
    ///
    /// # Returns
    /// A ConfigValue with the most appropriate type
    fn parse_env_value(&self, value: String) -> ConfigValue {
        // Try to parse as integer
        if let Ok(int_val) = value.parse::<i64>() {
            return ConfigValue::Integer(int_val);
        }

        // Try to parse as float
        if let Ok(float_val) = value.parse::<f64>() {
            return ConfigValue::Float(float_val);
        }

        // Try to parse as boolean
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "t" | "y" => return ConfigValue::Boolean(true),
            "false" | "0" | "no" | "off" | "f" | "n" => return ConfigValue::Boolean(false),
            _ => {}
        }

        // Default to string
        ConfigValue::String(value)
    }
}

impl ConfigLayer for EnvConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        // First check cached vars if automatic mode is enabled
        if self.automatic {
            if let Some(value) = self.cached_vars.get(key) {
                return Ok(Some(self.parse_env_value(value.clone())));
            }
        }

        // Transform the key to environment variable format and check directly
        let env_var_name = self.transform_key(key);
        if let Some(value) = self.get_env_var(&env_var_name) {
            if let ConfigValue::String(s) = value {
                return Ok(Some(self.parse_env_value(s)));
            }
        }

        Ok(None)
    }

    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        // Environment variables are read-only from the system perspective
        // We can update our cache for testing purposes, but we don't set actual env vars
        if self.automatic {
            self.cached_vars
                .insert(key.to_string(), value.coerce_to_string());
        }
        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        if self.automatic {
            self.cached_vars.keys().cloned().collect()
        } else {
            // In non-automatic mode, we can't enumerate all possible keys
            // since we don't know what environment variables exist
            Vec::new()
        }
    }

    fn source_name(&self) -> &str {
        "environment variables"
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::Environment
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_new_env_layer() {
        let env_layer = EnvConfigLayer::new(Some("TEST".to_string()), false);
        assert_eq!(env_layer.prefix, Some("TEST".to_string()));
        assert!(!env_layer.automatic);
        assert_eq!(env_layer.source_name(), "environment variables");
        assert_eq!(env_layer.priority(), LayerPriority::Environment);
    }

    #[test]
    fn test_transform_key_basic() {
        let env_layer = EnvConfigLayer::new(None, false);

        // Test basic transformation
        assert_eq!(env_layer.transform_key("database.host"), "DATABASE_HOST");
        assert_eq!(env_layer.transform_key("app.port"), "APP_PORT");
        assert_eq!(
            env_layer.transform_key("nested.config.value"),
            "NESTED_CONFIG_VALUE"
        );
    }

    #[test]
    fn test_transform_key_with_prefix() {
        let env_layer = EnvConfigLayer::new(Some("MYAPP".to_string()), false);

        assert_eq!(
            env_layer.transform_key("database.host"),
            "MYAPP_DATABASE_HOST"
        );
        assert_eq!(env_layer.transform_key("port"), "MYAPP_PORT");
    }

    #[test]
    fn test_transform_key_with_custom_replacer() {
        let mut env_layer = EnvConfigLayer::new(None, false);
        env_layer.set_key_replacer(Box::new(|key: &str| key.replace("_", "__")));

        // The custom replacer should be applied after basic transformation
        assert_eq!(env_layer.transform_key("database.host"), "DATABASE__HOST");
    }

    #[test]
    fn test_get_environment_variable() {
        // Set a test environment variable
        env::set_var("TEST_ENV_VAR", "test_value");

        let env_layer = EnvConfigLayer::new(Some("TEST".to_string()), false);

        // Should find the environment variable
        let result = env_layer.get("env.var").unwrap();
        assert_eq!(result, Some(ConfigValue::String("test_value".to_string())));

        // Clean up
        env::remove_var("TEST_ENV_VAR");
    }

    #[test]
    fn test_get_nonexistent_variable() {
        let env_layer = EnvConfigLayer::new(Some("NONEXISTENT".to_string()), false);

        let result = env_layer.get("some.key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_env_value_types() {
        let env_layer = EnvConfigLayer::new(None, false);

        // Test integer parsing
        assert_eq!(
            env_layer.parse_env_value("42".to_string()),
            ConfigValue::Integer(42)
        );
        assert_eq!(
            env_layer.parse_env_value("-123".to_string()),
            ConfigValue::Integer(-123)
        );

        // Test float parsing
        assert_eq!(
            env_layer.parse_env_value("3.14".to_string()),
            ConfigValue::Float(3.14)
        );
        assert_eq!(
            env_layer.parse_env_value("-2.5".to_string()),
            ConfigValue::Float(-2.5)
        );

        // Test boolean parsing - truthy values
        assert_eq!(
            env_layer.parse_env_value("true".to_string()),
            ConfigValue::Boolean(true)
        );
        assert_eq!(
            env_layer.parse_env_value("TRUE".to_string()),
            ConfigValue::Boolean(true)
        );
        assert_eq!(
            env_layer.parse_env_value("1".to_string()),
            ConfigValue::Integer(1)
        ); // Note: 1 parses as integer first
        assert_eq!(
            env_layer.parse_env_value("yes".to_string()),
            ConfigValue::Boolean(true)
        );
        assert_eq!(
            env_layer.parse_env_value("on".to_string()),
            ConfigValue::Boolean(true)
        );

        // Test boolean parsing - falsy values
        assert_eq!(
            env_layer.parse_env_value("false".to_string()),
            ConfigValue::Boolean(false)
        );
        assert_eq!(
            env_layer.parse_env_value("FALSE".to_string()),
            ConfigValue::Boolean(false)
        );
        assert_eq!(
            env_layer.parse_env_value("0".to_string()),
            ConfigValue::Integer(0)
        ); // Note: 0 parses as integer first
        assert_eq!(
            env_layer.parse_env_value("no".to_string()),
            ConfigValue::Boolean(false)
        );
        assert_eq!(
            env_layer.parse_env_value("off".to_string()),
            ConfigValue::Boolean(false)
        );

        // Test string fallback
        assert_eq!(
            env_layer.parse_env_value("hello".to_string()),
            ConfigValue::String("hello".to_string())
        );
        assert_eq!(
            env_layer.parse_env_value("mixed123".to_string()),
            ConfigValue::String("mixed123".to_string())
        );
    }

    #[test]
    fn test_automatic_mode_cache() {
        // Set some test environment variables
        env::set_var("AUTO_TEST_VAR1", "value1");
        env::set_var("AUTO_TEST_VAR2", "42");

        let env_layer = EnvConfigLayer::new(Some("AUTO".to_string()), true);

        // Should find cached values
        let result1 = env_layer.get("test.var1").unwrap();
        assert_eq!(result1, Some(ConfigValue::String("value1".to_string())));

        let result2 = env_layer.get("test.var2").unwrap();
        assert_eq!(result2, Some(ConfigValue::Integer(42)));

        // Should have keys available
        let keys = env_layer.keys();
        assert!(keys.contains(&"test.var1".to_string()));
        assert!(keys.contains(&"test.var2".to_string()));

        // Clean up
        env::remove_var("AUTO_TEST_VAR1");
        env::remove_var("AUTO_TEST_VAR2");
    }

    #[test]
    fn test_refresh_cache() {
        let mut env_layer = EnvConfigLayer::new(Some("REFRESH".to_string()), false);

        // Initially no cached vars
        assert_eq!(env_layer.keys().len(), 0);

        // Set environment variable
        env::set_var("REFRESH_TEST_KEY", "test_value");

        // Enable automatic mode and refresh cache
        env_layer.automatic = true;
        env_layer.refresh_cache();

        // Should now have the cached variable
        let keys = env_layer.keys();
        assert!(keys.contains(&"test.key".to_string()));

        let result = env_layer.get("test.key").unwrap();
        assert_eq!(result, Some(ConfigValue::String("test_value".to_string())));

        // Clean up
        env::remove_var("REFRESH_TEST_KEY");
    }

    #[test]
    fn test_set_operation() {
        let mut env_layer = EnvConfigLayer::new(None, true);

        // Set should work (updates cache in automatic mode)
        let result = env_layer.set("test.key", ConfigValue::String("test_value".to_string()));
        assert!(result.is_ok());

        // Should be able to retrieve the set value from cache
        let retrieved = env_layer.get("test.key").unwrap();
        assert_eq!(
            retrieved,
            Some(ConfigValue::String("test_value".to_string()))
        );
    }

    #[test]
    fn test_nested_key_handling() {
        // Test deeply nested keys
        env::set_var("NESTED_A_B_C_D", "deep_value");

        let env_layer = EnvConfigLayer::new(Some("NESTED".to_string()), true);

        let result = env_layer.get("a.b.c.d").unwrap();
        assert_eq!(result, Some(ConfigValue::String("deep_value".to_string())));

        // Clean up
        env::remove_var("NESTED_A_B_C_D");
    }

    #[test]
    fn test_key_transformation_edge_cases() {
        let env_layer = EnvConfigLayer::new(Some("TEST".to_string()), false);

        // Test empty key
        assert_eq!(env_layer.transform_key(""), "TEST_");

        // Test key with multiple consecutive dots
        assert_eq!(env_layer.transform_key("a..b"), "TEST_A__B");

        // Test key that's already uppercase
        assert_eq!(
            env_layer.transform_key("ALREADY.UPPER"),
            "TEST_ALREADY_UPPER"
        );
    }

    #[test]
    fn test_no_prefix_automatic_mode() {
        // Set some environment variables
        env::set_var("NO_PREFIX_TEST1", "value1");
        env::set_var("NO_PREFIX_TEST2", "value2");

        let env_layer = EnvConfigLayer::new(None, true);

        // Should find variables without prefix filtering
        let result1 = env_layer.get("no.prefix.test1").unwrap();
        assert_eq!(result1, Some(ConfigValue::String("value1".to_string())));

        let result2 = env_layer.get("no.prefix.test2").unwrap();
        assert_eq!(result2, Some(ConfigValue::String("value2".to_string())));

        // Clean up
        env::remove_var("NO_PREFIX_TEST1");
        env::remove_var("NO_PREFIX_TEST2");
    }
}
