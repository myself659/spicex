//! Command line flag configuration layer implementation.

use crate::error::{ConfigError, ConfigResult};
use crate::layer::{ConfigLayer, LayerPriority};
use crate::value::ConfigValue;
use clap::ArgMatches;
use std::collections::HashMap;

/// Configuration layer that provides values from command line flags.
///
/// This layer integrates with clap to parse command line arguments and make them
/// available as configuration values. It supports both short and long flag formats
/// and has high precedence in the configuration hierarchy.
///
/// # Example
/// ```
/// use spice::cli::FlagConfigLayer;
/// use clap::{Arg, Command};
///
/// let app = Command::new("myapp")
///     .disable_help_flag(true)
///     .arg(Arg::new("host")
///         .long("host")
///         .short('h')
///         .value_name("HOST")
///         .action(clap::ArgAction::Set)
///         .help("Database host"));
///
/// let args = vec!["myapp", "--host", "localhost"];
/// let matches = app.try_get_matches_from(args).unwrap();
/// let flag_layer = FlagConfigLayer::new(matches);
///
/// // The flag values are now available as configuration
/// ```
pub struct FlagConfigLayer {
    /// Parsed command line arguments
    matches: ArgMatches,
    /// Cached flag values for efficient access
    cached_values: HashMap<String, ConfigValue>,
    /// Flag to key mappings for custom key names
    flag_mappings: HashMap<String, String>,
}

impl FlagConfigLayer {
    /// Creates a new flag configuration layer from parsed command line arguments.
    ///
    /// # Arguments
    /// * `matches` - The parsed ArgMatches from clap
    ///
    /// # Returns
    /// * `FlagConfigLayer` - A new flag configuration layer
    ///
    /// # Example
    /// ```
    /// use spice::cli::FlagConfigLayer;
    /// use clap::{Arg, Command};
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("verbose")
    ///         .long("verbose")
    ///         .short('v')
    ///         .action(clap::ArgAction::SetTrue)
    ///         .help("Enable verbose output"));
    ///
    /// let args = vec!["myapp", "--verbose"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    /// let flag_layer = FlagConfigLayer::new(matches);
    /// ```
    pub fn new(matches: ArgMatches) -> Self {
        let mut layer = Self {
            matches,
            cached_values: HashMap::new(),
            flag_mappings: HashMap::new(),
        };
        layer.cache_flag_values();
        layer
    }

    /// Creates a new flag configuration layer with custom flag-to-key mappings.
    ///
    /// # Arguments
    /// * `matches` - The parsed ArgMatches from clap
    /// * `mappings` - HashMap mapping flag names to configuration keys
    ///
    /// # Returns
    /// * `FlagConfigLayer` - A new flag configuration layer with custom mappings
    ///
    /// # Example
    /// ```
    /// use spice::cli::FlagConfigLayer;
    /// use clap::{Arg, Command};
    /// use std::collections::HashMap;
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("db_host")
    ///         .long("db-host")
    ///         .value_name("HOST")
    ///         .help("Database host"));
    ///
    /// let args = vec!["myapp", "--db-host", "localhost"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    ///
    /// let mut mappings = HashMap::new();
    /// mappings.insert("db_host".to_string(), "database.host".to_string());
    ///
    /// let flag_layer = FlagConfigLayer::with_mappings(matches, mappings);
    /// // Now the --db-host flag will be available as "database.host" key
    /// ```
    pub fn with_mappings(matches: ArgMatches, mappings: HashMap<String, String>) -> Self {
        let mut layer = Self {
            matches,
            cached_values: HashMap::new(),
            flag_mappings: mappings,
        };
        layer.cache_flag_values();
        layer
    }

    /// Adds a mapping from a flag name to a configuration key.
    ///
    /// # Arguments
    /// * `flag_name` - The name of the command line flag (as defined in clap)
    /// * `config_key` - The configuration key to map to
    ///
    /// # Example
    /// ```
    /// use spice::cli::FlagConfigLayer;
    /// use clap::{Arg, Command};
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .value_name("HOST"));
    ///
    /// let args = vec!["myapp", "--host", "localhost"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    /// let mut flag_layer = FlagConfigLayer::new(matches);
    ///
    /// // Map the "host" flag to "database.host" configuration key
    /// flag_layer.add_flag_mapping("host", "database.host");
    /// ```
    pub fn add_flag_mapping(
        &mut self,
        flag_name: impl Into<String>,
        config_key: impl Into<String>,
    ) {
        self.flag_mappings
            .insert(flag_name.into(), config_key.into());
        // Re-cache values to apply new mapping
        self.cache_flag_values();
    }

    /// Removes a flag mapping.
    ///
    /// # Arguments
    /// * `flag_name` - The flag name to remove mapping for
    ///
    /// # Returns
    /// * `Option<String>` - The previous mapping if it existed
    pub fn remove_flag_mapping(&mut self, flag_name: &str) -> Option<String> {
        let result = self.flag_mappings.remove(flag_name);
        // Re-cache values to apply mapping removal
        self.cache_flag_values();
        result
    }

    /// Gets all current flag mappings.
    ///
    /// # Returns
    /// * `&HashMap<String, String>` - Reference to the flag mappings
    pub fn flag_mappings(&self) -> &HashMap<String, String> {
        &self.flag_mappings
    }

    /// Caches all flag values for efficient access.
    /// This method converts clap's ArgMatches into ConfigValue objects.
    fn cache_flag_values(&mut self) {
        self.cached_values.clear();

        // Iterate through all arguments that were provided
        for arg_id in self.matches.ids() {
            let arg_name = arg_id.as_str();

            // Determine the configuration key (use mapping if available, otherwise use flag name)
            let config_key = self
                .flag_mappings
                .get(arg_name)
                .cloned()
                .unwrap_or_else(|| self.normalize_flag_name(arg_name));

            // Convert the argument value to ConfigValue
            if let Some(config_value) = self.convert_arg_to_config_value(arg_name) {
                self.cached_values.insert(config_key, config_value);
            }
        }
    }

    /// Normalizes a flag name to a configuration key format.
    /// Converts hyphens and underscores to dots and ensures lowercase.
    ///
    /// # Arguments
    /// * `flag_name` - The original flag name
    ///
    /// # Returns
    /// * `String` - The normalized configuration key
    fn normalize_flag_name(&self, flag_name: &str) -> String {
        flag_name.replace('-', ".").replace('_', ".").to_lowercase()
    }

    /// Converts a clap argument to a ConfigValue.
    ///
    /// # Arguments
    /// * `arg_name` - The name of the argument to convert
    ///
    /// # Returns
    /// * `Option<ConfigValue>` - The converted value, or None if not present
    fn convert_arg_to_config_value(&self, arg_name: &str) -> Option<ConfigValue> {
        // First check if the argument was provided at all
        if !self.matches.contains_id(arg_name) {
            return None;
        }

        // Try different value types in order of specificity

        // 1. Try as boolean flag (SetTrue/SetFalse actions)
        if let Ok(Some(&flag_val)) = self.matches.try_get_one::<bool>(arg_name) {
            return Some(ConfigValue::Boolean(flag_val));
        }

        // 2. Try as multiple string values first (Append action)
        if let Ok(Some(values)) = self.matches.try_get_many::<String>(arg_name) {
            let config_values: Vec<ConfigValue> =
                values.map(|v| self.parse_string_value(v)).collect();

            if config_values.len() == 1 {
                return Some(config_values.into_iter().next().unwrap());
            } else {
                return Some(ConfigValue::Array(config_values));
            }
        }

        // 3. Try as single string value (Set actions)
        if let Ok(Some(string_val)) = self.matches.try_get_one::<String>(arg_name) {
            return Some(self.parse_string_value(string_val));
        }

        // 4. Check for count values (Count action)
        let count = self.matches.get_count(arg_name);
        if count > 0 {
            return Some(ConfigValue::Integer(count as i64));
        }

        // 5. Fallback: check if it's a flag that was set (for SetTrue actions without value)
        if self.matches.get_flag(arg_name) {
            return Some(ConfigValue::Boolean(true));
        }

        None
    }

    /// Parses a string value and attempts to convert it to the most appropriate ConfigValue type.
    ///
    /// # Arguments
    /// * `value` - The string value to parse
    ///
    /// # Returns
    /// * `ConfigValue` - The parsed value with appropriate type
    fn parse_string_value(&self, value: &str) -> ConfigValue {
        // Try to parse as boolean
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => return ConfigValue::Boolean(true),
            "false" | "0" | "no" | "off" => return ConfigValue::Boolean(false),
            _ => {}
        }

        // Try to parse as integer
        if let Ok(int_val) = value.parse::<i64>() {
            return ConfigValue::Integer(int_val);
        }

        // Try to parse as float
        if let Ok(float_val) = value.parse::<f64>() {
            return ConfigValue::Float(float_val);
        }

        // Default to string
        ConfigValue::String(value.to_string())
    }

    /// Gets the raw ArgMatches for advanced usage.
    ///
    /// # Returns
    /// * `&ArgMatches` - Reference to the underlying ArgMatches
    pub fn matches(&self) -> &ArgMatches {
        &self.matches
    }

    /// Checks if a specific flag was provided.
    ///
    /// # Arguments
    /// * `flag_name` - The name of the flag to check
    ///
    /// # Returns
    /// * `bool` - True if the flag was provided
    pub fn has_flag(&self, flag_name: &str) -> bool {
        self.matches.contains_id(flag_name)
    }

    /// Gets the number of times a flag was provided (for counting flags like -vvv).
    ///
    /// # Arguments
    /// * `flag_name` - The name of the flag to count
    ///
    /// # Returns
    /// * `u8` - The number of times the flag was provided
    pub fn flag_count(&self, flag_name: &str) -> u8 {
        self.matches.get_count(flag_name)
    }
}

impl ConfigLayer for FlagConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        Ok(self.cached_values.get(key).cloned())
    }

    fn set(&mut self, _key: &str, _value: ConfigValue) -> ConfigResult<()> {
        // Command line flags are read-only after parsing
        Err(ConfigError::unsupported_operation(
            "Cannot set values in flag configuration layer - flags are read-only",
        ))
    }

    fn keys(&self) -> Vec<String> {
        self.cached_values.keys().cloned().collect()
    }

    fn source_name(&self) -> &str {
        "command line flags"
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::Flags
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
    use clap::{Arg, Command};

    fn create_test_app() -> Command {
        Command::new("test")
            .disable_help_flag(true) // Disable auto-generated help to avoid conflicts
            .arg(
                Arg::new("host")
                    .long("host")
                    .short('h')
                    .value_name("HOST")
                    .action(clap::ArgAction::Set) // Explicitly set action for string values
                    .help("Database host"),
            )
            .arg(
                Arg::new("port")
                    .long("port")
                    .short('p')
                    .value_name("PORT")
                    .action(clap::ArgAction::Set) // Explicitly set action for string values
                    .help("Database port"),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .short('v')
                    .action(clap::ArgAction::SetTrue)
                    .help("Enable verbose output"),
            )
            .arg(
                Arg::new("include")
                    .long("include")
                    .short('i')
                    .value_name("PATH")
                    .action(clap::ArgAction::Append)
                    .help("Include paths"),
            )
    }

    #[test]
    fn test_flag_layer_creation() {
        let app = create_test_app();
        let args = vec!["test", "--host", "localhost", "--port", "5432", "--verbose"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        assert_eq!(flag_layer.source_name(), "command line flags");
        assert_eq!(flag_layer.priority(), LayerPriority::Flags);
    }

    #[test]
    fn test_string_flag_parsing() {
        let app = create_test_app();
        let args = vec!["test", "--host", "localhost", "--port", "5432"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        let host_value = flag_layer.get("host").unwrap();
        assert_eq!(
            host_value,
            Some(ConfigValue::String("localhost".to_string()))
        );

        let port_value = flag_layer.get("port").unwrap();
        assert_eq!(port_value, Some(ConfigValue::Integer(5432)));
    }

    #[test]
    fn test_boolean_flag_parsing() {
        let app = create_test_app();
        let args = vec!["test", "--verbose"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        let verbose_value = flag_layer.get("verbose").unwrap();
        assert_eq!(verbose_value, Some(ConfigValue::Boolean(true)));

        // Test flag not present
        let nonexistent_value = flag_layer.get("nonexistent").unwrap();
        assert_eq!(nonexistent_value, None);
    }

    #[test]
    fn test_count_flag_parsing() {
        let app = Command::new("test").disable_help_flag(true).arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .action(clap::ArgAction::Count)
                .help("Debug level"),
        );

        let args = vec!["test", "-ddd"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        assert_eq!(flag_layer.flag_count("debug"), 3);

        // Test that count values are converted to integers
        let debug_value = flag_layer.get("debug").unwrap();
        assert_eq!(debug_value, Some(ConfigValue::Integer(3)));
    }

    #[test]
    fn test_multi_value_flag_parsing() {
        let app = create_test_app();
        let args = vec!["test", "-i", "path1", "-i", "path2", "-i", "path3"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        let include_value = flag_layer.get("include").unwrap();
        match include_value {
            Some(ConfigValue::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], ConfigValue::String("path1".to_string()));
                assert_eq!(arr[1], ConfigValue::String("path2".to_string()));
                assert_eq!(arr[2], ConfigValue::String("path3".to_string()));
            }
            _ => panic!("Expected array value, got: {:?}", include_value),
        }
    }

    #[test]
    fn test_flag_mappings() {
        let app = create_test_app();
        let args = vec!["test", "--host", "localhost"];
        let matches = app.try_get_matches_from(args).unwrap();

        let mut mappings = HashMap::new();
        mappings.insert("host".to_string(), "database.host".to_string());

        let flag_layer = FlagConfigLayer::with_mappings(matches, mappings);

        // Should be available under the mapped key
        let host_value = flag_layer.get("database.host").unwrap();
        assert_eq!(
            host_value,
            Some(ConfigValue::String("localhost".to_string()))
        );

        // Should not be available under the original key
        let original_value = flag_layer.get("host").unwrap();
        assert_eq!(original_value, None);
    }

    #[test]
    fn test_flag_name_normalization() {
        let app = Command::new("test").disable_help_flag(true).arg(
            Arg::new("db_host")
                .long("db-host")
                .value_name("HOST")
                .action(clap::ArgAction::Set),
        );

        let args = vec!["test", "--db-host", "localhost"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        // Debug: print all keys to see what's actually available
        println!("Available keys: {:?}", flag_layer.keys());

        // The arg name is "db_host", which should normalize to "db.host"
        let host_value = flag_layer.get("db.host").unwrap();
        assert_eq!(
            host_value,
            Some(ConfigValue::String("localhost".to_string()))
        );
    }

    #[test]
    fn test_value_type_parsing() {
        let app = Command::new("test")
            .disable_help_flag(true)
            .arg(
                Arg::new("string_val")
                    .long("string")
                    .value_name("VAL")
                    .action(clap::ArgAction::Set),
            )
            .arg(
                Arg::new("int_val")
                    .long("int")
                    .value_name("VAL")
                    .action(clap::ArgAction::Set),
            )
            .arg(
                Arg::new("float_val")
                    .long("float")
                    .value_name("VAL")
                    .action(clap::ArgAction::Set),
            )
            .arg(
                Arg::new("bool_val")
                    .long("bool")
                    .value_name("VAL")
                    .action(clap::ArgAction::Set),
            );

        let args = vec![
            "test", "--string", "hello", "--int", "42", "--float", "3.14", "--bool", "true",
        ];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        assert_eq!(
            flag_layer.get("string.val").unwrap(),
            Some(ConfigValue::String("hello".to_string()))
        );
        assert_eq!(
            flag_layer.get("int.val").unwrap(),
            Some(ConfigValue::Integer(42))
        );
        assert_eq!(
            flag_layer.get("float.val").unwrap(),
            Some(ConfigValue::Float(3.14))
        );
        assert_eq!(
            flag_layer.get("bool.val").unwrap(),
            Some(ConfigValue::Boolean(true))
        );
    }

    #[test]
    fn test_read_only_layer() {
        let app = create_test_app();
        let args = vec!["test", "--host", "localhost"];
        let matches = app.try_get_matches_from(args).unwrap();
        let mut flag_layer = FlagConfigLayer::new(matches);

        // Should not be able to set values
        let result = flag_layer.set("new_key", ConfigValue::String("value".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_keys_method() {
        let app = create_test_app();
        let args = vec!["test", "--host", "localhost", "--verbose"];
        let matches = app.try_get_matches_from(args).unwrap();
        let flag_layer = FlagConfigLayer::new(matches);

        let keys = flag_layer.keys();
        assert!(keys.contains(&"host".to_string()));
        assert!(keys.contains(&"verbose".to_string()));
        assert_eq!(keys.len(), 2);
    }
}
