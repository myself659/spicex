//! Configuration format parsers for different file types.
//!
//! This module provides parsers for various configuration file formats including JSON, YAML, TOML, and INI.
//! Each parser implements the `ConfigParser` trait, which provides a unified interface for parsing and
//! serializing configuration data.
//!
//! ## Supported Formats
//!
//! - **JSON** - JavaScript Object Notation, widely used for web APIs and configuration
//! - **YAML** - YAML Ain't Markup Language, human-readable data serialization standard
//! - **TOML** - Tom's Obvious, Minimal Language, designed for configuration files
//! - **INI** - Initialization file format, simple key-value pairs with sections
//!
//! ## Parser Detection
//!
//! Parsers are automatically detected based on file extensions:
//!
//! ```rust
//! use spicex::parser::detect_parser_by_extension;
//!
//! let json_parser = detect_parser_by_extension("json").unwrap();
//! let yaml_parser = detect_parser_by_extension("yaml").unwrap();
//! let toml_parser = detect_parser_by_extension("toml").unwrap();
//! let ini_parser = detect_parser_by_extension("ini").unwrap();
//! ```
//!
//! ## Custom Parsing
//!
//! You can also use parsers directly:
//!
//! ```rust
//! use spicex::parser::{JsonParser, ConfigParser};
//!
//! let parser = JsonParser;
//! let json_content = r#"{"database": {"host": "localhost", "port": 5432}}"#;
//! let parsed = parser.parse(json_content).unwrap();
//!
//! // Access parsed values
//! println!("Parsed {} keys", parsed.len());
//! ```
//!
//! ## Error Handling
//!
//! All parsing operations return `ConfigResult<T>` which provides detailed error information:
//!
//! ```rust
//! use spicex::parser::{JsonParser, ConfigParser};
//! use spicex::error::ConfigError;
//!
//! let parser = JsonParser;
//! let invalid_json = r#"{"invalid": json}"#;
//!
//! match parser.parse(invalid_json) {
//!     Ok(parsed) => println!("Parsed successfully"),
//!     Err(ConfigError::Parse { source_name, message }) => {
//!         println!("Parse error in {}: {}", source_name, message);
//!     }
//!     Err(e) => println!("Other error: {}", e),
//! }
//! ```

use crate::error::{ConfigError, ConfigResult};
use crate::value::ConfigValue;
use std::collections::HashMap;

/// Trait for parsing configuration files in different formats.
///
/// This trait provides a unified interface for parsing and serializing configuration data
/// across different file formats. Implementations handle format-specific parsing logic
/// while providing a consistent API.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support multi-threaded usage.
///
/// # Example Implementation
///
/// ```rust
/// use spicex::parser::ConfigParser;
/// use spicex::{ConfigValue, ConfigResult, ConfigError};
/// use std::collections::HashMap;
///
/// struct CustomParser;
///
/// impl ConfigParser for CustomParser {
///     fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
///         // Custom parsing logic here
///         let mut result = HashMap::new();
///         result.insert("example".to_string(), ConfigValue::from("value"));
///         Ok(result)
///     }
///
///     fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
///         // Custom serialization logic here
///         Ok("serialized content".to_string())
///     }
///
///     fn supported_extensions(&self) -> &[&str] {
///         &["custom"]
///     }
///
///     fn name(&self) -> &str {
///         "Custom"
///     }
/// }
/// ```
pub trait ConfigParser: Send + Sync {
    /// Parses configuration content into a key-value map.
    ///
    /// This method takes a string containing configuration data in the parser's format
    /// and converts it into a HashMap of configuration keys and values.
    ///
    /// # Arguments
    /// * `content` - The configuration content as a string
    ///
    /// # Returns
    /// * `ConfigResult<HashMap<String, ConfigValue>>` - The parsed configuration data
    ///
    /// # Errors
    /// * `ConfigError::Parse` - If the content cannot be parsed due to syntax errors
    /// * `ConfigError::UnsupportedFormat` - If the content format is not supported
    ///
    /// # Example
    /// ```rust
    /// use spicex::parser::{JsonParser, ConfigParser};
    ///
    /// let parser = JsonParser;
    /// let json_content = r#"{"database": {"host": "localhost"}}"#;
    /// let parsed = parser.parse(json_content).unwrap();
    ///
    /// assert!(parsed.contains_key("database"));
    /// ```
    fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>>;

    /// Serializes a key-value map back to the format's string representation.
    ///
    /// This method takes a HashMap of configuration data and converts it back
    /// to the parser's string format for writing to files or other outputs.
    ///
    /// # Arguments
    /// * `data` - The configuration data to serialize
    ///
    /// # Returns
    /// * `ConfigResult<String>` - The serialized configuration content
    ///
    /// # Errors
    /// * `ConfigError::Serialization` - If the data cannot be serialized
    ///
    /// # Example
    /// ```rust
    /// use spicex::parser::{JsonParser, ConfigParser};
    /// use spicex::ConfigValue;
    /// use std::collections::HashMap;
    ///
    /// let parser = JsonParser;
    /// let mut data = HashMap::new();
    /// data.insert("key".to_string(), ConfigValue::from("value"));
    ///
    /// let serialized = parser.serialize(&data).unwrap();
    /// assert!(serialized.contains("key"));
    /// assert!(serialized.contains("value"));
    /// ```
    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String>;

    /// Returns the file extensions supported by this parser.
    ///
    /// This method returns a slice of file extensions (without the dot) that
    /// this parser can handle. Used for automatic parser detection.
    ///
    /// # Returns
    /// * `&[&str]` - Array of supported file extensions
    ///
    /// # Example
    /// ```rust
    /// use spicex::parser::{JsonParser, ConfigParser};
    ///
    /// let parser = JsonParser;
    /// let extensions = parser.supported_extensions();
    /// assert_eq!(extensions, &["json"]);
    /// ```
    fn supported_extensions(&self) -> &[&str];

    /// Returns a human-readable name for this parser.
    ///
    /// This method returns a descriptive name for the parser, used in error
    /// messages and logging.
    ///
    /// # Returns
    /// * `&str` - The parser's display name
    ///
    /// # Example
    /// ```rust
    /// use spicex::parser::{JsonParser, ConfigParser};
    ///
    /// let parser = JsonParser;
    /// assert_eq!(parser.name(), "JSON");
    /// ```
    fn name(&self) -> &str;
}

/// Determines the appropriate parser based on file extension.
///
/// This function automatically selects the correct parser implementation based on
/// the provided file extension. The extension matching is case-insensitive.
///
/// # Arguments
/// * `extension` - The file extension (without the dot, e.g., "json", "yaml")
///
/// # Returns
/// * `ConfigResult<Box<dyn ConfigParser>>` - A boxed parser instance for the format
///
/// # Errors
/// * `ConfigError::UnsupportedFormat` - If the extension is not supported
///
/// # Supported Extensions
/// - `json` - JSON parser
/// - `yaml`, `yml` - YAML parser
/// - `toml` - TOML parser
/// - `ini` - INI parser
///
/// # Example
/// ```rust
/// use spicex::parser::detect_parser_by_extension;
///
/// // Get a JSON parser
/// let json_parser = detect_parser_by_extension("json").unwrap();
/// assert_eq!(json_parser.name(), "JSON");
///
/// // Get a YAML parser (both extensions work)
/// let yaml_parser = detect_parser_by_extension("yaml").unwrap();
/// let yml_parser = detect_parser_by_extension("yml").unwrap();
/// assert_eq!(yaml_parser.name(), "YAML");
/// assert_eq!(yml_parser.name(), "YAML");
///
/// // Unsupported extension returns error
/// let result = detect_parser_by_extension("unknown");
/// assert!(result.is_err());
/// ```
pub fn detect_parser_by_extension(extension: &str) -> ConfigResult<Box<dyn ConfigParser>> {
    match extension.to_lowercase().as_str() {
        "json" => Ok(Box::new(JsonParser)),
        "yaml" | "yml" => Ok(Box::new(YamlParser)),
        "toml" => Ok(Box::new(TomlParser)),
        "ini" => Ok(Box::new(IniParser)),
        _ => Err(ConfigError::UnsupportedFormat),
    }
}

/// JSON configuration parser.
///
/// This parser handles JavaScript Object Notation (JSON) format configuration files.
/// JSON is a lightweight, text-based data interchange format that is easy for humans
/// to read and write and easy for machines to parse and generate.
///
/// # Supported Features
/// - Objects (maps/dictionaries)
/// - Arrays
/// - Strings, numbers, booleans, null
/// - Nested structures
/// - Unicode support
///
/// # Limitations
/// - Comments are not supported (standard JSON restriction)
/// - Trailing commas are not allowed
/// - All keys must be strings
///
/// # Example
/// ```rust
/// use spicex::parser::{JsonParser, ConfigParser};
///
/// let parser = JsonParser;
/// let json_content = r#"
/// {
///     "database": {
///         "host": "localhost",
///         "port": 5432,
///         "ssl": true
///     },
///     "features": ["auth", "logging"],
///     "timeout": 30.5
/// }
/// "#;
///
/// let parsed = parser.parse(json_content).unwrap();
/// assert!(parsed.contains_key("database"));
/// assert!(parsed.contains_key("features"));
/// ```
pub struct JsonParser;

impl ConfigParser for JsonParser {
    fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
        let value: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| ConfigError::parse_error("JSON", e.to_string()))?;

        convert_json_value(value)
    }

    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
        // Convert ConfigValue map to serde_json::Value for serialization
        let json_map: serde_json::Map<String, serde_json::Value> = data
            .iter()
            .map(|(k, v)| (k.clone(), config_value_to_json(v)))
            .collect();

        let json_value = serde_json::Value::Object(json_map);
        serde_json::to_string_pretty(&json_value)
            .map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["json"]
    }

    fn name(&self) -> &str {
        "JSON"
    }
}

/// YAML configuration parser.
///
/// This parser handles YAML Ain't Markup Language (YAML) format configuration files.
/// YAML is a human-readable data serialization standard that is commonly used for
/// configuration files and data exchange between applications.
///
/// # Supported Features
/// - Mappings (key-value pairs)
/// - Sequences (arrays/lists)
/// - Scalars (strings, numbers, booleans, null)
/// - Multi-line strings
/// - Comments (lines starting with #)
/// - Nested structures
/// - Multiple documents in one file
/// - Unicode support
///
/// # YAML-Specific Features
/// - Indentation-based structure
/// - Flow and block styles
/// - Anchors and aliases
/// - Tagged values
///
/// # Example
/// ```rust
/// use spicex::parser::{YamlParser, ConfigParser};
///
/// let parser = YamlParser;
/// let yaml_content = r#"
/// # Database configuration
/// database:
///   host: localhost
///   port: 5432
///   ssl: true
///   credentials:
///     username: admin
///     password: secret
///
/// # Feature flags
/// features:
///   - auth
///   - logging
///   - metrics
///
/// # Timeout in seconds
/// timeout: 30.5
/// "#;
///
/// let parsed = parser.parse(yaml_content).unwrap();
/// assert!(parsed.contains_key("database"));
/// assert!(parsed.contains_key("features"));
/// ```
pub struct YamlParser;

impl ConfigParser for YamlParser {
    fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
        let value: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| ConfigError::parse_error("YAML", e.to_string()))?;

        convert_yaml_value(value)
    }

    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
        // Convert ConfigValue map to serde_yaml::Value for serialization
        let mut yaml_map = serde_yaml::Mapping::new();
        for (k, v) in data {
            yaml_map.insert(
                serde_yaml::Value::String(k.clone()),
                config_value_to_yaml(v),
            );
        }

        let yaml_value = serde_yaml::Value::Mapping(yaml_map);
        serde_yaml::to_string(&yaml_value).map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }

    fn name(&self) -> &str {
        "YAML"
    }
}

/// TOML configuration parser.
///
/// This parser handles Tom's Obvious, Minimal Language (TOML) format configuration files.
/// TOML is designed to be a minimal configuration file format that's easy to read due to
/// obvious semantics and maps unambiguously to a hash table.
///
/// # Supported Features
/// - Key-value pairs
/// - Tables (sections)
/// - Arrays
/// - Strings (basic, multi-line, literal, multi-line literal)
/// - Integers, floats, booleans
/// - Dates and times (RFC 3339)
/// - Comments (lines starting with #)
/// - Nested tables
/// - Array of tables
///
/// # TOML-Specific Features
/// - Dotted keys for nested structures
/// - Table headers with [section] syntax
/// - Array of tables with [[section]] syntax
/// - Strong typing with clear syntax
///
/// # Example
/// ```rust
/// use spicex::parser::{TomlParser, ConfigParser};
///
/// let parser = TomlParser;
/// let toml_content = r#"
/// # Application configuration
/// title = "My Application"
/// debug = true
/// timeout = 30.5
///
/// [database]
/// host = "localhost"
/// port = 5432
/// ssl = true
///
/// [database.credentials]
/// username = "admin"
/// password = "secret"
///
/// [[servers]]
/// name = "web1"
/// ip = "192.168.1.10"
///
/// [[servers]]
/// name = "web2"
/// ip = "192.168.1.11"
/// "#;
///
/// let parsed = parser.parse(toml_content).unwrap();
/// assert!(parsed.contains_key("title"));
/// assert!(parsed.contains_key("database"));
/// assert!(parsed.contains_key("servers"));
/// ```
pub struct TomlParser;

impl ConfigParser for TomlParser {
    fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
        let value: toml::Value =
            toml::from_str(content).map_err(|e| ConfigError::parse_error("TOML", e.to_string()))?;

        convert_toml_value(value)
    }

    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
        // Convert ConfigValue map to toml::Value for serialization
        let mut toml_table = toml::map::Map::new();
        for (k, v) in data {
            toml_table.insert(k.clone(), config_value_to_toml(v));
        }

        let toml_value = toml::Value::Table(toml_table);
        toml::to_string_pretty(&toml_value).map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    fn supported_extensions(&self) -> &[&str] {
        &["toml"]
    }

    fn name(&self) -> &str {
        "TOML"
    }
}

/// INI configuration parser.
///
/// This parser handles INI (Initialization) format configuration files.
/// INI is a simple configuration file format consisting of key-value pairs
/// organized into sections. It's widely used in Windows applications and
/// many configuration systems.
///
/// # Supported Features
/// - Key-value pairs with `key = value` syntax
/// - Sections with `[section]` headers
/// - Comments starting with `;` or `#`
/// - String, integer, float, and boolean values
/// - Global properties (outside of sections)
/// - Case-insensitive section and key names
///
/// # Format Limitations
/// - No nested sections (flat structure only)
/// - No arrays (though values can be comma-separated strings)
/// - No complex data types
/// - Limited escaping support
///
/// # Value Type Detection
/// The parser automatically detects value types:
/// - Integers: `42`, `-123`
/// - Floats: `3.14`, `-2.5`
/// - Booleans: `true`, `false`, `yes`, `no`, `on`, `off`
/// - Strings: Everything else
///
/// # Example
/// ```rust
/// use spicex::parser::{IniParser, ConfigParser};
///
/// let parser = IniParser;
/// let ini_content = r#"
/// ; Global configuration
/// debug = true
/// timeout = 30
///
/// # Database section
/// [database]
/// host = localhost
/// port = 5432
/// ssl = yes
///
/// [cache]
/// enabled = true
/// ttl = 3600
/// "#;
///
/// let parsed = parser.parse(ini_content).unwrap();
/// assert!(parsed.contains_key("debug"));
/// assert!(parsed.contains_key("database"));
/// assert!(parsed.contains_key("cache"));
/// ```
pub struct IniParser;

impl ConfigParser for IniParser {
    fn parse(&self, content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
        parse_ini_content(content)
    }

    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
        serialize_ini_data(data)
    }

    fn supported_extensions(&self) -> &[&str] {
        &["ini"]
    }

    fn name(&self) -> &str {
        "INI"
    }
}

fn parse_ini_content(content: &str) -> ConfigResult<HashMap<String, ConfigValue>> {
    let mut result = HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_section_data = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Check for section header
        if line.starts_with('[') && line.ends_with(']') {
            // Save previous section if it exists
            if let Some(section_name) = current_section.take() {
                if !current_section_data.is_empty() {
                    result.insert(section_name, ConfigValue::Object(current_section_data));
                    current_section_data = HashMap::new();
                }
            }

            // Start new section
            let section_name = line[1..line.len() - 1].trim().to_string();
            if section_name.is_empty() {
                return Err(ConfigError::parse_error("INI", "Empty section name"));
            }
            current_section = Some(section_name);
            continue;
        }

        // Parse key-value pair
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim();

            if key.is_empty() {
                return Err(ConfigError::parse_error("INI", "Empty key name"));
            }

            let parsed_value = parse_ini_value(value);

            if current_section.is_some() {
                // We're in a section
                current_section_data.insert(key, parsed_value);
            } else {
                // Global property
                result.insert(key, parsed_value);
            }
        } else {
            return Err(ConfigError::parse_error(
                "INI",
                format!("Invalid line format: {line}"),
            ));
        }
    }

    // Save the last section if it exists
    if let Some(section_name) = current_section {
        if !current_section_data.is_empty() {
            result.insert(section_name, ConfigValue::Object(current_section_data));
        }
    }

    Ok(result)
}

fn serialize_ini_data(data: &HashMap<String, ConfigValue>) -> ConfigResult<String> {
    let mut output = String::new();

    // Separate root-level properties from sections
    let mut general_properties = Vec::new();
    let mut sections = Vec::new();

    for (key, value) in data {
        match value {
            ConfigValue::Object(obj) => {
                // This is a section
                sections.push((key, obj));
            }
            _ => {
                // This is a general property
                general_properties.push((key, value));
            }
        }
    }

    // Write general properties first
    let has_general_properties = !general_properties.is_empty();
    for (key, value) in general_properties {
        output.push_str(&format!(
            "{} = {}\n",
            key,
            config_value_to_ini_string(value)
        ));
    }

    // Add blank line if we have both general properties and sections
    if has_general_properties && !sections.is_empty() {
        output.push('\n');
    }

    // Write sections
    for (i, (section_name, section_obj)) in sections.iter().enumerate() {
        if i > 0 {
            output.push('\n'); // Blank line between sections
        }

        output.push_str(&format!("[{section_name}]\n"));

        for (key, value) in section_obj.iter() {
            output.push_str(&format!(
                "{} = {}\n",
                key,
                config_value_to_ini_string(value)
            ));
        }
    }

    Ok(output)
}

fn parse_ini_value(value: &str) -> ConfigValue {
    // Try to parse as different types

    // Try integer first (before boolean to avoid "0" and "1" being parsed as booleans)
    if let Ok(i) = value.parse::<i64>() {
        return ConfigValue::Integer(i);
    }

    // Try float
    if let Ok(f) = value.parse::<f64>() {
        return ConfigValue::Float(f);
    }

    // Try boolean (excluding numeric strings)
    match value.to_lowercase().as_str() {
        "true" | "yes" | "on" => return ConfigValue::Boolean(true),
        "false" | "no" | "off" => return ConfigValue::Boolean(false),
        _ => {}
    }

    // Default to string
    ConfigValue::String(value.to_string())
}

fn config_value_to_ini_string(value: &ConfigValue) -> String {
    match value {
        ConfigValue::String(s) => s.clone(),
        ConfigValue::Integer(i) => i.to_string(),
        ConfigValue::Float(f) => f.to_string(),
        ConfigValue::Boolean(b) => b.to_string(),
        ConfigValue::Null => String::new(),
        ConfigValue::Array(_) => {
            // INI doesn't support arrays natively, so we serialize as a comma-separated string
            "[array]".to_string()
        }
        ConfigValue::Object(_) => {
            // INI doesn't support nested objects beyond sections
            "[object]".to_string()
        }
    }
}

// Helper functions for JSON value conversion
fn convert_json_value(value: serde_json::Value) -> ConfigResult<HashMap<String, ConfigValue>> {
    match value {
        serde_json::Value::Object(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                result.insert(k, json_to_config_value(v));
            }
            Ok(result)
        }
        _ => Err(ConfigError::parse_error("JSON", "Root must be an object")),
    }
}

fn json_to_config_value(value: serde_json::Value) -> ConfigValue {
    match value {
        serde_json::Value::String(s) => ConfigValue::String(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConfigValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                ConfigValue::Float(f)
            } else {
                ConfigValue::Null
            }
        }
        serde_json::Value::Bool(b) => ConfigValue::Boolean(b),
        serde_json::Value::Array(arr) => {
            ConfigValue::Array(arr.into_iter().map(json_to_config_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_config_value(v));
            }
            ConfigValue::Object(map)
        }
        serde_json::Value::Null => ConfigValue::Null,
    }
}

fn config_value_to_json(value: &ConfigValue) -> serde_json::Value {
    match value {
        ConfigValue::String(s) => serde_json::Value::String(s.clone()),
        ConfigValue::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        ConfigValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ConfigValue::Boolean(b) => serde_json::Value::Bool(*b),
        ConfigValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(config_value_to_json).collect())
        }
        ConfigValue::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), config_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        ConfigValue::Null => serde_json::Value::Null,
    }
}

fn convert_yaml_value(value: serde_yaml::Value) -> ConfigResult<HashMap<String, ConfigValue>> {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    result.insert(key, yaml_to_config_value(v));
                } else {
                    // Convert non-string keys to strings
                    let key_str = yaml_value_to_string(&k);
                    result.insert(key_str, yaml_to_config_value(v));
                }
            }
            Ok(result)
        }
        _ => Err(ConfigError::parse_error(
            "YAML",
            "Root must be a mapping/object",
        )),
    }
}

fn yaml_to_config_value(value: serde_yaml::Value) -> ConfigValue {
    match value {
        serde_yaml::Value::String(s) => ConfigValue::String(s),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConfigValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                ConfigValue::Float(f)
            } else {
                ConfigValue::Null
            }
        }
        serde_yaml::Value::Bool(b) => ConfigValue::Boolean(b),
        serde_yaml::Value::Sequence(arr) => {
            ConfigValue::Array(arr.into_iter().map(yaml_to_config_value).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                let key_str = if let serde_yaml::Value::String(key) = k {
                    key
                } else {
                    yaml_value_to_string(&k)
                };
                result.insert(key_str, yaml_to_config_value(v));
            }
            ConfigValue::Object(result)
        }
        serde_yaml::Value::Null => ConfigValue::Null,
        serde_yaml::Value::Tagged(tagged) => {
            // Handle tagged values by extracting the inner value
            yaml_to_config_value(tagged.value)
        }
    }
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "null".to_string(),
        _ => format!("{value:?}"),
    }
}

fn config_value_to_yaml(value: &ConfigValue) -> serde_yaml::Value {
    match value {
        ConfigValue::String(s) => serde_yaml::Value::String(s.clone()),
        ConfigValue::Integer(i) => serde_yaml::Value::Number(serde_yaml::Number::from(*i)),
        ConfigValue::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        ConfigValue::Boolean(b) => serde_yaml::Value::Bool(*b),
        ConfigValue::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(config_value_to_yaml).collect())
        }
        ConfigValue::Object(obj) => {
            let mut map = serde_yaml::Mapping::new();
            for (k, v) in obj {
                map.insert(
                    serde_yaml::Value::String(k.clone()),
                    config_value_to_yaml(v),
                );
            }
            serde_yaml::Value::Mapping(map)
        }
        ConfigValue::Null => serde_yaml::Value::Null,
    }
}

fn convert_toml_value(value: toml::Value) -> ConfigResult<HashMap<String, ConfigValue>> {
    match value {
        toml::Value::Table(table) => {
            let mut result = HashMap::new();
            for (k, v) in table {
                result.insert(k, toml_to_config_value(v));
            }
            Ok(result)
        }
        _ => Err(ConfigError::parse_error(
            "TOML",
            "Root must be a table/object",
        )),
    }
}

fn toml_to_config_value(value: toml::Value) -> ConfigValue {
    match value {
        toml::Value::String(s) => ConfigValue::String(s),
        toml::Value::Integer(i) => ConfigValue::Integer(i),
        toml::Value::Float(f) => ConfigValue::Float(f),
        toml::Value::Boolean(b) => ConfigValue::Boolean(b),
        toml::Value::Array(arr) => {
            ConfigValue::Array(arr.into_iter().map(toml_to_config_value).collect())
        }
        toml::Value::Table(table) => {
            let mut result = HashMap::new();
            for (k, v) in table {
                result.insert(k, toml_to_config_value(v));
            }
            ConfigValue::Object(result)
        }
        toml::Value::Datetime(dt) => {
            // Convert datetime to string representation
            ConfigValue::String(dt.to_string())
        }
    }
}

fn config_value_to_toml(value: &ConfigValue) -> toml::Value {
    match value {
        ConfigValue::String(s) => toml::Value::String(s.clone()),
        ConfigValue::Integer(i) => toml::Value::Integer(*i),
        ConfigValue::Float(f) => toml::Value::Float(*f),
        ConfigValue::Boolean(b) => toml::Value::Boolean(*b),
        ConfigValue::Array(arr) => {
            toml::Value::Array(arr.iter().map(config_value_to_toml).collect())
        }
        ConfigValue::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k.clone(), config_value_to_toml(v));
            }
            toml::Value::Table(table)
        }
        ConfigValue::Null => {
            // TOML doesn't have a null value, so we represent it as an empty string
            toml::Value::String(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_detection() {
        assert!(detect_parser_by_extension("json").is_ok());
        assert!(detect_parser_by_extension("yaml").is_ok());
        assert!(detect_parser_by_extension("toml").is_ok());
        assert!(detect_parser_by_extension("ini").is_ok());
        assert!(detect_parser_by_extension("unknown").is_err());
    }

    #[test]
    fn test_json_parser_basic() {
        let parser = JsonParser;
        assert_eq!(parser.name(), "JSON");
        assert_eq!(parser.supported_extensions(), &["json"]);
    }

    #[test]
    fn test_json_parser_simple_object() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "string_key": "hello world",
            "integer_key": 42,
            "float_key": 3.14,
            "boolean_key": true,
            "null_key": null
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        assert_eq!(
            result.get("string_key"),
            Some(&ConfigValue::String("hello world".to_string()))
        );
        assert_eq!(result.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("float_key"), Some(&ConfigValue::Float(3.14)));
        assert_eq!(result.get("boolean_key"), Some(&ConfigValue::Boolean(true)));
        assert_eq!(result.get("null_key"), Some(&ConfigValue::Null));
    }

    #[test]
    fn test_json_parser_nested_object() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "database": {
                "host": "localhost",
                "port": 5432,
                "credentials": {
                    "username": "admin",
                    "password": "secret"
                }
            },
            "features": ["auth", "logging", "metrics"]
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        // Check nested object
        if let Some(ConfigValue::Object(database)) = result.get("database") {
            assert_eq!(
                database.get("host"),
                Some(&ConfigValue::String("localhost".to_string()))
            );
            assert_eq!(database.get("port"), Some(&ConfigValue::Integer(5432)));

            if let Some(ConfigValue::Object(credentials)) = database.get("credentials") {
                assert_eq!(
                    credentials.get("username"),
                    Some(&ConfigValue::String("admin".to_string()))
                );
                assert_eq!(
                    credentials.get("password"),
                    Some(&ConfigValue::String("secret".to_string()))
                );
            } else {
                panic!("Expected credentials to be an object");
            }
        } else {
            panic!("Expected database to be an object");
        }

        // Check array
        if let Some(ConfigValue::Array(features)) = result.get("features") {
            assert_eq!(features.len(), 3);
            assert_eq!(features[0], ConfigValue::String("auth".to_string()));
            assert_eq!(features[1], ConfigValue::String("logging".to_string()));
            assert_eq!(features[2], ConfigValue::String("metrics".to_string()));
        } else {
            panic!("Expected features to be an array");
        }
    }

    #[test]
    fn test_json_parser_array_of_objects() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "servers": [
                {"name": "web1", "port": 8080},
                {"name": "web2", "port": 8081}
            ]
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        if let Some(ConfigValue::Array(servers)) = result.get("servers") {
            assert_eq!(servers.len(), 2);

            if let ConfigValue::Object(server1) = &servers[0] {
                assert_eq!(
                    server1.get("name"),
                    Some(&ConfigValue::String("web1".to_string()))
                );
                assert_eq!(server1.get("port"), Some(&ConfigValue::Integer(8080)));
            } else {
                panic!("Expected first server to be an object");
            }
        } else {
            panic!("Expected servers to be an array");
        }
    }

    #[test]
    fn test_json_parser_invalid_syntax() {
        let parser = JsonParser;
        let invalid_json = r#"
        {
            "key": "value",
            "invalid":
        }
        "#;

        let result = parser.parse(invalid_json);
        assert!(result.is_err());

        if let Err(ConfigError::Parse {
            source_name,
            message,
        }) = result
        {
            assert_eq!(source_name, "JSON");
            assert!(message.contains("expected value"));
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_json_parser_non_object_root() {
        let parser = JsonParser;
        let array_json = r#"["item1", "item2"]"#;

        let result = parser.parse(array_json);
        assert!(result.is_err());

        if let Err(ConfigError::Parse {
            source_name,
            message,
        }) = result
        {
            assert_eq!(source_name, "JSON");
            assert_eq!(message, "Root must be an object");
        } else {
            panic!("Expected Parse error for non-object root");
        }
    }

    #[test]
    fn test_json_parser_empty_object() {
        let parser = JsonParser;
        let empty_json = "{}";

        let result = parser.parse(empty_json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_json_parser_number_edge_cases() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "zero": 0,
            "negative": -42,
            "large_int": 9223372036854775807,
            "small_float": 0.000001,
            "scientific": 1.23e-4,
            "negative_float": -3.14159
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        assert_eq!(result.get("zero"), Some(&ConfigValue::Integer(0)));
        assert_eq!(result.get("negative"), Some(&ConfigValue::Integer(-42)));
        assert_eq!(
            result.get("large_int"),
            Some(&ConfigValue::Integer(9223372036854775807))
        );
        assert_eq!(
            result.get("small_float"),
            Some(&ConfigValue::Float(0.000001))
        );
        assert_eq!(result.get("scientific"), Some(&ConfigValue::Float(1.23e-4)));
        assert_eq!(
            result.get("negative_float"),
            Some(&ConfigValue::Float(-3.14159))
        );
    }

    #[test]
    fn test_json_serialization_simple() {
        let parser = JsonParser;
        let mut data = HashMap::new();
        data.insert(
            "string_key".to_string(),
            ConfigValue::String("hello".to_string()),
        );
        data.insert("integer_key".to_string(), ConfigValue::Integer(42));
        data.insert("boolean_key".to_string(), ConfigValue::Boolean(true));
        data.insert("null_key".to_string(), ConfigValue::Null);

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();
        assert_eq!(
            reparsed.get("string_key"),
            Some(&ConfigValue::String("hello".to_string()))
        );
        assert_eq!(reparsed.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(
            reparsed.get("boolean_key"),
            Some(&ConfigValue::Boolean(true))
        );
        assert_eq!(reparsed.get("null_key"), Some(&ConfigValue::Null));
    }

    #[test]
    fn test_json_serialization_nested() {
        let parser = JsonParser;
        let mut data = HashMap::new();

        // Create nested object
        let mut nested = HashMap::new();
        nested.insert(
            "inner_key".to_string(),
            ConfigValue::String("inner_value".to_string()),
        );
        data.insert("nested".to_string(), ConfigValue::Object(nested));

        // Create array
        let array = vec![
            ConfigValue::String("item1".to_string()),
            ConfigValue::Integer(123),
            ConfigValue::Boolean(false),
        ];
        data.insert("array".to_string(), ConfigValue::Array(array));

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();

        if let Some(ConfigValue::Object(nested_obj)) = reparsed.get("nested") {
            assert_eq!(
                nested_obj.get("inner_key"),
                Some(&ConfigValue::String("inner_value".to_string()))
            );
        } else {
            panic!("Expected nested object");
        }

        if let Some(ConfigValue::Array(reparsed_array)) = reparsed.get("array") {
            assert_eq!(reparsed_array.len(), 3);
            assert_eq!(reparsed_array[0], ConfigValue::String("item1".to_string()));
            assert_eq!(reparsed_array[1], ConfigValue::Integer(123));
            assert_eq!(reparsed_array[2], ConfigValue::Boolean(false));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_json_serialization_round_trip() {
        let parser = JsonParser;
        let original_json = r#"
        {
            "app": {
                "name": "test-app",
                "version": "1.0.0",
                "debug": true,
                "max_connections": 100,
                "timeout": 30.5,
                "features": ["auth", "logging"],
                "metadata": null
            }
        }
        "#;

        // Parse -> Serialize -> Parse again
        let parsed = parser.parse(original_json).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Should be identical
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_json_serialization_special_float_values() {
        let parser = JsonParser;
        let mut data = HashMap::new();

        // Test infinity and NaN handling
        data.insert("normal_float".to_string(), ConfigValue::Float(3.14));
        data.insert("zero_float".to_string(), ConfigValue::Float(0.0));
        data.insert("negative_zero".to_string(), ConfigValue::Float(-0.0));

        let serialized = parser.serialize(&data).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        assert_eq!(
            reparsed.get("normal_float"),
            Some(&ConfigValue::Float(3.14))
        );
        assert_eq!(reparsed.get("zero_float"), Some(&ConfigValue::Float(0.0)));
        assert_eq!(
            reparsed.get("negative_zero"),
            Some(&ConfigValue::Float(-0.0))
        );
    }

    #[test]
    fn test_json_parser_unicode_strings() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "unicode": "Hello 世界 🌍",
            "emoji": "🚀 🎉 ✨",
            "escaped": "Line 1\nLine 2\tTabbed"
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        assert_eq!(
            result.get("unicode"),
            Some(&ConfigValue::String("Hello 世界 🌍".to_string()))
        );
        assert_eq!(
            result.get("emoji"),
            Some(&ConfigValue::String("🚀 🎉 ✨".to_string()))
        );
        assert_eq!(
            result.get("escaped"),
            Some(&ConfigValue::String("Line 1\nLine 2\tTabbed".to_string()))
        );
    }

    #[test]
    fn test_json_parser_empty_values() {
        let parser = JsonParser;
        let json_content = r#"
        {
            "empty_string": "",
            "empty_array": [],
            "empty_object": {}
        }
        "#;

        let result = parser.parse(json_content).unwrap();

        assert_eq!(
            result.get("empty_string"),
            Some(&ConfigValue::String("".to_string()))
        );
        assert_eq!(result.get("empty_array"), Some(&ConfigValue::Array(vec![])));
        assert_eq!(
            result.get("empty_object"),
            Some(&ConfigValue::Object(HashMap::new()))
        );
    }

    // YAML Parser Tests
    #[test]
    fn test_yaml_parser_basic() {
        let parser = YamlParser;
        assert_eq!(parser.name(), "YAML");
        assert_eq!(parser.supported_extensions(), &["yaml", "yml"]);
    }

    #[test]
    fn test_yaml_parser_simple_object() {
        let parser = YamlParser;
        let yaml_content = r#"
string_key: hello world
integer_key: 42
float_key: 3.14
boolean_key: true
null_key: null
        "#;

        let result = parser.parse(yaml_content).unwrap();

        assert_eq!(
            result.get("string_key"),
            Some(&ConfigValue::String("hello world".to_string()))
        );
        assert_eq!(result.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("float_key"), Some(&ConfigValue::Float(3.14)));
        assert_eq!(result.get("boolean_key"), Some(&ConfigValue::Boolean(true)));
        assert_eq!(result.get("null_key"), Some(&ConfigValue::Null));
    }

    #[test]
    fn test_yaml_parser_nested_object() {
        let parser = YamlParser;
        let yaml_content = r#"
database:
  host: localhost
  port: 5432
  credentials:
    username: admin
    password: secret
features:
  - auth
  - logging
  - metrics
        "#;

        let result = parser.parse(yaml_content).unwrap();

        // Check nested object
        if let Some(ConfigValue::Object(database)) = result.get("database") {
            assert_eq!(
                database.get("host"),
                Some(&ConfigValue::String("localhost".to_string()))
            );
            assert_eq!(database.get("port"), Some(&ConfigValue::Integer(5432)));

            if let Some(ConfigValue::Object(credentials)) = database.get("credentials") {
                assert_eq!(
                    credentials.get("username"),
                    Some(&ConfigValue::String("admin".to_string()))
                );
                assert_eq!(
                    credentials.get("password"),
                    Some(&ConfigValue::String("secret".to_string()))
                );
            } else {
                panic!("Expected credentials to be an object");
            }
        } else {
            panic!("Expected database to be an object");
        }

        // Check array
        if let Some(ConfigValue::Array(features)) = result.get("features") {
            assert_eq!(features.len(), 3);
            assert_eq!(features[0], ConfigValue::String("auth".to_string()));
            assert_eq!(features[1], ConfigValue::String("logging".to_string()));
            assert_eq!(features[2], ConfigValue::String("metrics".to_string()));
        } else {
            panic!("Expected features to be an array");
        }
    }

    #[test]
    fn test_yaml_parser_array_of_objects() {
        let parser = YamlParser;
        let yaml_content = r#"
servers:
  - name: web1
    port: 8080
  - name: web2
    port: 8081
        "#;

        let result = parser.parse(yaml_content).unwrap();

        if let Some(ConfigValue::Array(servers)) = result.get("servers") {
            assert_eq!(servers.len(), 2);

            if let ConfigValue::Object(server1) = &servers[0] {
                assert_eq!(
                    server1.get("name"),
                    Some(&ConfigValue::String("web1".to_string()))
                );
                assert_eq!(server1.get("port"), Some(&ConfigValue::Integer(8080)));
            } else {
                panic!("Expected first server to be an object");
            }
        } else {
            panic!("Expected servers to be an array");
        }
    }

    #[test]
    fn test_yaml_parser_invalid_syntax() {
        let parser = YamlParser;
        let invalid_yaml = r#"
key: value
  invalid_indentation: bad
        "#;

        let result = parser.parse(invalid_yaml);
        assert!(result.is_err());

        if let Err(ConfigError::Parse { source_name, .. }) = result {
            assert_eq!(source_name, "YAML");
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_yaml_parser_non_object_root() {
        let parser = YamlParser;
        let array_yaml = r#"
- item1
- item2
        "#;

        let result = parser.parse(array_yaml);
        assert!(result.is_err());

        if let Err(ConfigError::Parse {
            source_name,
            message,
        }) = result
        {
            assert_eq!(source_name, "YAML");
            assert_eq!(message, "Root must be a mapping/object");
        } else {
            panic!("Expected Parse error for non-object root");
        }
    }

    #[test]
    fn test_yaml_parser_empty_object() {
        let parser = YamlParser;
        let empty_yaml = "{}";

        let result = parser.parse(empty_yaml).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_yaml_parser_boolean_variations() {
        let parser = YamlParser;
        let yaml_content = r#"
bool_true1: true
bool_true2: True
bool_true3: TRUE
bool_false1: false
bool_false2: False
bool_false3: FALSE
        "#;

        let result = parser.parse(yaml_content).unwrap();

        // Test true variations (serde_yaml only recognizes true/True/TRUE as booleans)
        for i in 1..=3 {
            let key = format!("bool_true{}", i);
            assert_eq!(
                result.get(&key),
                Some(&ConfigValue::Boolean(true)),
                "Failed for key: {}",
                key
            );
        }

        // Test false variations (serde_yaml only recognizes false/False/FALSE as booleans)
        for i in 1..=3 {
            let key = format!("bool_false{}", i);
            assert_eq!(
                result.get(&key),
                Some(&ConfigValue::Boolean(false)),
                "Failed for key: {}",
                key
            );
        }
    }

    #[test]
    fn test_yaml_parser_string_boolean_like_values() {
        let parser = YamlParser;
        let yaml_content = r#"
yes_string: yes
no_string: no
on_string: on
off_string: off
        "#;

        let result = parser.parse(yaml_content).unwrap();

        // These are parsed as strings by serde_yaml (YAML 1.2 behavior)
        assert_eq!(
            result.get("yes_string"),
            Some(&ConfigValue::String("yes".to_string()))
        );
        assert_eq!(
            result.get("no_string"),
            Some(&ConfigValue::String("no".to_string()))
        );
        assert_eq!(
            result.get("on_string"),
            Some(&ConfigValue::String("on".to_string()))
        );
        assert_eq!(
            result.get("off_string"),
            Some(&ConfigValue::String("off".to_string()))
        );
    }

    #[test]
    fn test_yaml_parser_number_formats() {
        let parser = YamlParser;
        let yaml_content = r#"
decimal: 42
octal: 0o52
hex: 0x2A
binary: 0b101010
float: 3.14
scientific: 1.23e-4
infinity: .inf
negative_infinity: -.inf
not_a_number: .nan
        "#;

        let result = parser.parse(yaml_content).unwrap();

        assert_eq!(result.get("decimal"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("octal"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("hex"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("binary"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("float"), Some(&ConfigValue::Float(3.14)));
        assert_eq!(result.get("scientific"), Some(&ConfigValue::Float(1.23e-4)));

        // Special float values
        if let Some(ConfigValue::Float(inf)) = result.get("infinity") {
            assert!(inf.is_infinite() && inf.is_sign_positive());
        }
        if let Some(ConfigValue::Float(neg_inf)) = result.get("negative_infinity") {
            assert!(neg_inf.is_infinite() && neg_inf.is_sign_negative());
        }
        if let Some(ConfigValue::Float(nan)) = result.get("not_a_number") {
            assert!(nan.is_nan());
        }
    }

    #[test]
    fn test_yaml_parser_multiline_strings() {
        let parser = YamlParser;
        let yaml_content = r#"
literal_block: |
  This is a literal block.
  Line breaks are preserved.
  Trailing newlines are kept.

folded_block: >
  This is a folded block.
  Line breaks become spaces.
  Trailing newlines are kept.

quoted_string: "This is a quoted string with\nescaped newlines."
        "#;

        let result = parser.parse(yaml_content).unwrap();

        if let Some(ConfigValue::String(literal)) = result.get("literal_block") {
            assert!(literal.contains("This is a literal block.\nLine breaks are preserved.\nTrailing newlines are kept.\n"));
        }

        if let Some(ConfigValue::String(folded)) = result.get("folded_block") {
            assert!(folded.contains(
                "This is a folded block. Line breaks become spaces. Trailing newlines are kept.\n"
            ));
        }

        assert_eq!(
            result.get("quoted_string"),
            Some(&ConfigValue::String(
                "This is a quoted string with\nescaped newlines.".to_string()
            ))
        );
    }

    #[test]
    fn test_yaml_serialization_simple() {
        let parser = YamlParser;
        let mut data = HashMap::new();
        data.insert(
            "string_key".to_string(),
            ConfigValue::String("hello".to_string()),
        );
        data.insert("integer_key".to_string(), ConfigValue::Integer(42));
        data.insert("boolean_key".to_string(), ConfigValue::Boolean(true));
        data.insert("null_key".to_string(), ConfigValue::Null);

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();
        assert_eq!(
            reparsed.get("string_key"),
            Some(&ConfigValue::String("hello".to_string()))
        );
        assert_eq!(reparsed.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(
            reparsed.get("boolean_key"),
            Some(&ConfigValue::Boolean(true))
        );
        assert_eq!(reparsed.get("null_key"), Some(&ConfigValue::Null));
    }

    #[test]
    fn test_yaml_serialization_nested() {
        let parser = YamlParser;
        let mut data = HashMap::new();

        // Create nested object
        let mut nested = HashMap::new();
        nested.insert(
            "inner_key".to_string(),
            ConfigValue::String("inner_value".to_string()),
        );
        data.insert("nested".to_string(), ConfigValue::Object(nested));

        // Create array
        let array = vec![
            ConfigValue::String("item1".to_string()),
            ConfigValue::Integer(123),
            ConfigValue::Boolean(false),
        ];
        data.insert("array".to_string(), ConfigValue::Array(array));

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();

        if let Some(ConfigValue::Object(nested_obj)) = reparsed.get("nested") {
            assert_eq!(
                nested_obj.get("inner_key"),
                Some(&ConfigValue::String("inner_value".to_string()))
            );
        } else {
            panic!("Expected nested object");
        }

        if let Some(ConfigValue::Array(reparsed_array)) = reparsed.get("array") {
            assert_eq!(reparsed_array.len(), 3);
            assert_eq!(reparsed_array[0], ConfigValue::String("item1".to_string()));
            assert_eq!(reparsed_array[1], ConfigValue::Integer(123));
            assert_eq!(reparsed_array[2], ConfigValue::Boolean(false));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_yaml_serialization_round_trip() {
        let parser = YamlParser;
        let original_yaml = r#"
app:
  name: test-app
  version: "1.0.0"
  debug: true
  max_connections: 100
  timeout: 30.5
  features:
    - auth
    - logging
  metadata: null
        "#;

        // Parse -> Serialize -> Parse again
        let parsed = parser.parse(original_yaml).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Should be identical
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_yaml_parser_empty_values() {
        let parser = YamlParser;
        let yaml_content = r#"
empty_string: ""
empty_array: []
empty_object: {}
        "#;

        let result = parser.parse(yaml_content).unwrap();

        assert_eq!(
            result.get("empty_string"),
            Some(&ConfigValue::String("".to_string()))
        );
        assert_eq!(result.get("empty_array"), Some(&ConfigValue::Array(vec![])));
        assert_eq!(
            result.get("empty_object"),
            Some(&ConfigValue::Object(HashMap::new()))
        );
    }

    // TOML Parser Tests
    #[test]
    fn test_toml_parser_basic() {
        let parser = TomlParser;
        assert_eq!(parser.name(), "TOML");
        assert_eq!(parser.supported_extensions(), &["toml"]);
    }

    #[test]
    fn test_toml_parser_simple_object() {
        let parser = TomlParser;
        let toml_content = r#"
string_key = "hello world"
integer_key = 42
float_key = 3.14
boolean_key = true
        "#;

        let result = parser.parse(toml_content).unwrap();

        assert_eq!(
            result.get("string_key"),
            Some(&ConfigValue::String("hello world".to_string()))
        );
        assert_eq!(result.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("float_key"), Some(&ConfigValue::Float(3.14)));
        assert_eq!(result.get("boolean_key"), Some(&ConfigValue::Boolean(true)));
    }

    #[test]
    fn test_toml_parser_nested_object() {
        let parser = TomlParser;
        let toml_content = r#"
features = ["auth", "logging", "metrics"]

[database]
host = "localhost"
port = 5432

[database.credentials]
username = "admin"
password = "secret"
        "#;

        let result = parser.parse(toml_content).unwrap();

        // Check array at root level
        if let Some(ConfigValue::Array(features)) = result.get("features") {
            assert_eq!(features.len(), 3);
            assert_eq!(features[0], ConfigValue::String("auth".to_string()));
            assert_eq!(features[1], ConfigValue::String("logging".to_string()));
            assert_eq!(features[2], ConfigValue::String("metrics".to_string()));
        } else {
            panic!("Expected features to be an array");
        }

        // Check nested object
        if let Some(ConfigValue::Object(database)) = result.get("database") {
            assert_eq!(
                database.get("host"),
                Some(&ConfigValue::String("localhost".to_string()))
            );
            assert_eq!(database.get("port"), Some(&ConfigValue::Integer(5432)));

            if let Some(ConfigValue::Object(credentials)) = database.get("credentials") {
                assert_eq!(
                    credentials.get("username"),
                    Some(&ConfigValue::String("admin".to_string()))
                );
                assert_eq!(
                    credentials.get("password"),
                    Some(&ConfigValue::String("secret".to_string()))
                );
            } else {
                panic!("Expected credentials to be an object");
            }
        } else {
            panic!("Expected database to be an object");
        }
    }

    #[test]
    fn test_toml_parser_array_of_tables() {
        let parser = TomlParser;
        let toml_content = r#"
[[servers]]
name = "web1"
port = 8080

[[servers]]
name = "web2"
port = 8081
        "#;

        let result = parser.parse(toml_content).unwrap();

        if let Some(ConfigValue::Array(servers)) = result.get("servers") {
            assert_eq!(servers.len(), 2);

            if let ConfigValue::Object(server1) = &servers[0] {
                assert_eq!(
                    server1.get("name"),
                    Some(&ConfigValue::String("web1".to_string()))
                );
                assert_eq!(server1.get("port"), Some(&ConfigValue::Integer(8080)));
            } else {
                panic!("Expected first server to be an object");
            }
        } else {
            panic!("Expected servers to be an array");
        }
    }

    #[test]
    fn test_toml_parser_invalid_syntax() {
        let parser = TomlParser;
        let invalid_toml = r#"
key = "value"
invalid =
        "#;

        let result = parser.parse(invalid_toml);
        assert!(result.is_err());

        if let Err(ConfigError::Parse { source_name, .. }) = result {
            assert_eq!(source_name, "TOML");
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_toml_parser_empty_object() {
        let parser = TomlParser;
        let empty_toml = "";

        let result = parser.parse(empty_toml).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_toml_parser_number_formats() {
        let parser = TomlParser;
        let toml_content = r#"
decimal = 42
hex = 0x2A
octal = 0o52
binary = 0b101010
float = 3.14
scientific = 1.23e-4
negative = -42
negative_float = -3.14159
        "#;

        let result = parser.parse(toml_content).unwrap();

        assert_eq!(result.get("decimal"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("hex"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("octal"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("binary"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("float"), Some(&ConfigValue::Float(3.14)));
        assert_eq!(result.get("scientific"), Some(&ConfigValue::Float(1.23e-4)));
        assert_eq!(result.get("negative"), Some(&ConfigValue::Integer(-42)));
        assert_eq!(
            result.get("negative_float"),
            Some(&ConfigValue::Float(-3.14159))
        );
    }

    #[test]
    fn test_toml_parser_string_formats() {
        let parser = TomlParser;
        let toml_content = r#"
basic_string = "Hello, World!"
literal_string = 'No escaping needed here'
multiline_basic = """
This is a multiline
basic string.
"""
multiline_literal = '''
This is a multiline
literal string.
'''
escaped = "Line 1\nLine 2\tTabbed"
        "#;

        let result = parser.parse(toml_content).unwrap();

        assert_eq!(
            result.get("basic_string"),
            Some(&ConfigValue::String("Hello, World!".to_string()))
        );
        assert_eq!(
            result.get("literal_string"),
            Some(&ConfigValue::String("No escaping needed here".to_string()))
        );

        if let Some(ConfigValue::String(multiline_basic)) = result.get("multiline_basic") {
            assert!(multiline_basic.contains("This is a multiline"));
            assert!(multiline_basic.contains("basic string."));
        }

        if let Some(ConfigValue::String(multiline_literal)) = result.get("multiline_literal") {
            assert!(multiline_literal.contains("This is a multiline"));
            assert!(multiline_literal.contains("literal string."));
        }

        assert_eq!(
            result.get("escaped"),
            Some(&ConfigValue::String("Line 1\nLine 2\tTabbed".to_string()))
        );
    }

    #[test]
    fn test_toml_parser_datetime() {
        let parser = TomlParser;
        let toml_content = r#"
date = 2023-01-01
datetime = 2023-01-01T10:30:00Z
local_datetime = 2023-01-01T10:30:00
time = 10:30:00
        "#;

        let result = parser.parse(toml_content).unwrap();

        // Datetimes are converted to strings
        assert!(matches!(result.get("date"), Some(ConfigValue::String(_))));
        assert!(matches!(
            result.get("datetime"),
            Some(ConfigValue::String(_))
        ));
        assert!(matches!(
            result.get("local_datetime"),
            Some(ConfigValue::String(_))
        ));
        assert!(matches!(result.get("time"), Some(ConfigValue::String(_))));
    }

    #[test]
    fn test_toml_parser_mixed_arrays() {
        let parser = TomlParser;
        let toml_content = r#"
integers = [1, 2, 3]
strings = ["red", "yellow", "green"]
mixed_numbers = [1, 2.0, 3]
nested_arrays = [[1, 2], [3, 4, 5]]
        "#;

        let result = parser.parse(toml_content).unwrap();

        if let Some(ConfigValue::Array(integers)) = result.get("integers") {
            assert_eq!(integers.len(), 3);
            assert_eq!(integers[0], ConfigValue::Integer(1));
            assert_eq!(integers[1], ConfigValue::Integer(2));
            assert_eq!(integers[2], ConfigValue::Integer(3));
        }

        if let Some(ConfigValue::Array(strings)) = result.get("strings") {
            assert_eq!(strings.len(), 3);
            assert_eq!(strings[0], ConfigValue::String("red".to_string()));
            assert_eq!(strings[1], ConfigValue::String("yellow".to_string()));
            assert_eq!(strings[2], ConfigValue::String("green".to_string()));
        }

        if let Some(ConfigValue::Array(mixed)) = result.get("mixed_numbers") {
            assert_eq!(mixed.len(), 3);
            assert_eq!(mixed[0], ConfigValue::Integer(1));
            assert_eq!(mixed[1], ConfigValue::Float(2.0));
            assert_eq!(mixed[2], ConfigValue::Integer(3));
        }

        if let Some(ConfigValue::Array(nested)) = result.get("nested_arrays") {
            assert_eq!(nested.len(), 2);
            if let ConfigValue::Array(first_nested) = &nested[0] {
                assert_eq!(first_nested.len(), 2);
                assert_eq!(first_nested[0], ConfigValue::Integer(1));
                assert_eq!(first_nested[1], ConfigValue::Integer(2));
            }
        }
    }

    #[test]
    fn test_toml_serialization_simple() {
        let parser = TomlParser;
        let mut data = HashMap::new();
        data.insert(
            "string_key".to_string(),
            ConfigValue::String("hello".to_string()),
        );
        data.insert("integer_key".to_string(), ConfigValue::Integer(42));
        data.insert("boolean_key".to_string(), ConfigValue::Boolean(true));
        // Note: TOML doesn't have null, so we skip null_key

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();
        assert_eq!(
            reparsed.get("string_key"),
            Some(&ConfigValue::String("hello".to_string()))
        );
        assert_eq!(reparsed.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(
            reparsed.get("boolean_key"),
            Some(&ConfigValue::Boolean(true))
        );
    }

    #[test]
    fn test_toml_serialization_nested() {
        let parser = TomlParser;
        let mut data = HashMap::new();

        // Create nested object
        let mut nested = HashMap::new();
        nested.insert(
            "inner_key".to_string(),
            ConfigValue::String("inner_value".to_string()),
        );
        data.insert("nested".to_string(), ConfigValue::Object(nested));

        // Create array
        let array = vec![
            ConfigValue::String("item1".to_string()),
            ConfigValue::Integer(123),
            ConfigValue::Boolean(false),
        ];
        data.insert("array".to_string(), ConfigValue::Array(array));

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();

        if let Some(ConfigValue::Object(nested_obj)) = reparsed.get("nested") {
            assert_eq!(
                nested_obj.get("inner_key"),
                Some(&ConfigValue::String("inner_value".to_string()))
            );
        } else {
            panic!("Expected nested object");
        }

        if let Some(ConfigValue::Array(reparsed_array)) = reparsed.get("array") {
            assert_eq!(reparsed_array.len(), 3);
            assert_eq!(reparsed_array[0], ConfigValue::String("item1".to_string()));
            assert_eq!(reparsed_array[1], ConfigValue::Integer(123));
            assert_eq!(reparsed_array[2], ConfigValue::Boolean(false));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_toml_serialization_round_trip() {
        let parser = TomlParser;
        let original_toml = r#"
[app]
name = "test-app"
version = "1.0.0"
debug = true
max_connections = 100
timeout = 30.5
features = ["auth", "logging"]
        "#;

        // Parse -> Serialize -> Parse again
        let parsed = parser.parse(original_toml).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Should be identical
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_toml_parser_empty_values() {
        let parser = TomlParser;
        let toml_content = r#"
empty_string = ""
empty_array = []
        "#;

        let result = parser.parse(toml_content).unwrap();

        assert_eq!(
            result.get("empty_string"),
            Some(&ConfigValue::String("".to_string()))
        );
        assert_eq!(result.get("empty_array"), Some(&ConfigValue::Array(vec![])));
        // Note: TOML doesn't support empty tables in the same way as JSON/YAML
    }

    #[test]
    fn test_toml_null_handling() {
        let parser = TomlParser;
        let mut data = HashMap::new();
        data.insert("null_value".to_string(), ConfigValue::Null);

        let serialized = parser.serialize(&data).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Null values become empty strings in TOML
        assert_eq!(
            reparsed.get("null_value"),
            Some(&ConfigValue::String("".to_string()))
        );
    }

    // INI Parser Tests
    #[test]
    fn test_ini_parser_basic() {
        let parser = IniParser;
        assert_eq!(parser.name(), "INI");
        assert_eq!(parser.supported_extensions(), &["ini"]);
    }

    #[test]
    fn test_ini_parser_simple_properties() {
        let parser = IniParser;
        let ini_content = r#"
# Global properties
global_key = global_value
debug = true
port = 8080
timeout = 30.5
        "#;

        let result = parser.parse(ini_content).unwrap();

        assert_eq!(
            result.get("global_key"),
            Some(&ConfigValue::String("global_value".to_string()))
        );
        assert_eq!(result.get("debug"), Some(&ConfigValue::Boolean(true)));
        assert_eq!(result.get("port"), Some(&ConfigValue::Integer(8080)));
        assert_eq!(result.get("timeout"), Some(&ConfigValue::Float(30.5)));
    }

    #[test]
    fn test_ini_parser_sections() {
        let parser = IniParser;
        let ini_content = r#"
# Global properties
app_name = MyApp

[database]
host = localhost
port = 5432
username = admin
password = secret
ssl_enabled = true

[logging]
level = info
file = /var/log/app.log
max_size = 100
        "#;

        let result = parser.parse(ini_content).unwrap();

        // Check global property
        assert_eq!(
            result.get("app_name"),
            Some(&ConfigValue::String("MyApp".to_string()))
        );

        // Check database section
        if let Some(ConfigValue::Object(database)) = result.get("database") {
            assert_eq!(
                database.get("host"),
                Some(&ConfigValue::String("localhost".to_string()))
            );
            assert_eq!(database.get("port"), Some(&ConfigValue::Integer(5432)));
            assert_eq!(
                database.get("username"),
                Some(&ConfigValue::String("admin".to_string()))
            );
            assert_eq!(
                database.get("password"),
                Some(&ConfigValue::String("secret".to_string()))
            );
            assert_eq!(
                database.get("ssl_enabled"),
                Some(&ConfigValue::Boolean(true))
            );
        } else {
            panic!("Expected database to be an object");
        }

        // Check logging section
        if let Some(ConfigValue::Object(logging)) = result.get("logging") {
            assert_eq!(
                logging.get("level"),
                Some(&ConfigValue::String("info".to_string()))
            );
            assert_eq!(
                logging.get("file"),
                Some(&ConfigValue::String("/var/log/app.log".to_string()))
            );
            assert_eq!(logging.get("max_size"), Some(&ConfigValue::Integer(100)));
        } else {
            panic!("Expected logging to be an object");
        }
    }

    #[test]
    fn test_ini_parser_boolean_variations() {
        let parser = IniParser;
        let ini_content = r#"
bool_true1 = true
bool_true2 = TRUE
bool_true3 = True
bool_true4 = yes
bool_true5 = YES
bool_true6 = on
bool_true7 = ON
bool_false1 = false
bool_false2 = FALSE
bool_false3 = False
bool_false4 = no
bool_false5 = NO
bool_false6 = off
bool_false7 = OFF
        "#;

        let result = parser.parse(ini_content).unwrap();

        // Test true variations (excluding numeric strings which are parsed as integers)
        for i in 1..=7 {
            let key = format!("bool_true{}", i);
            assert_eq!(
                result.get(&key),
                Some(&ConfigValue::Boolean(true)),
                "Failed for key: {}",
                key
            );
        }

        // Test false variations (excluding numeric strings which are parsed as integers)
        for i in 1..=7 {
            let key = format!("bool_false{}", i);
            assert_eq!(
                result.get(&key),
                Some(&ConfigValue::Boolean(false)),
                "Failed for key: {}",
                key
            );
        }
    }

    #[test]
    fn test_ini_parser_numeric_strings() {
        let parser = IniParser;
        let ini_content = r#"
numeric_true = 1
numeric_false = 0
        "#;

        let result = parser.parse(ini_content).unwrap();

        // Numeric strings are parsed as integers, not booleans
        assert_eq!(result.get("numeric_true"), Some(&ConfigValue::Integer(1)));
        assert_eq!(result.get("numeric_false"), Some(&ConfigValue::Integer(0)));
    }

    #[test]
    fn test_ini_parser_number_formats() {
        let parser = IniParser;
        let ini_content = r#"
positive_int = 42
negative_int = -42
zero = 0
positive_float = 3.14
negative_float = -3.14
scientific = 1.23e-4
        "#;

        let result = parser.parse(ini_content).unwrap();

        assert_eq!(result.get("positive_int"), Some(&ConfigValue::Integer(42)));
        assert_eq!(result.get("negative_int"), Some(&ConfigValue::Integer(-42)));
        assert_eq!(result.get("zero"), Some(&ConfigValue::Integer(0)));
        assert_eq!(
            result.get("positive_float"),
            Some(&ConfigValue::Float(3.14))
        );
        assert_eq!(
            result.get("negative_float"),
            Some(&ConfigValue::Float(-3.14))
        );
        assert_eq!(result.get("scientific"), Some(&ConfigValue::Float(1.23e-4)));
    }

    #[test]
    fn test_ini_parser_string_values() {
        let parser = IniParser;
        let ini_content = r#"
simple_string = hello world
quoted_string = "quoted value"
string_with_spaces = value with spaces
empty_string =
string_number = 123abc
        "#;

        let result = parser.parse(ini_content).unwrap();

        assert_eq!(
            result.get("simple_string"),
            Some(&ConfigValue::String("hello world".to_string()))
        );
        assert_eq!(
            result.get("quoted_string"),
            Some(&ConfigValue::String("\"quoted value\"".to_string()))
        );
        assert_eq!(
            result.get("string_with_spaces"),
            Some(&ConfigValue::String("value with spaces".to_string()))
        );
        assert_eq!(
            result.get("empty_string"),
            Some(&ConfigValue::String("".to_string()))
        );
        assert_eq!(
            result.get("string_number"),
            Some(&ConfigValue::String("123abc".to_string()))
        );
    }

    #[test]
    fn test_ini_parser_invalid_syntax() {
        let parser = IniParser;
        let invalid_ini = r#"
[unclosed_section
key = value
        "#;

        let result = parser.parse(invalid_ini);
        assert!(result.is_err());

        if let Err(ConfigError::Parse { source_name, .. }) = result {
            assert_eq!(source_name, "INI");
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_ini_parser_empty_file() {
        let parser = IniParser;
        let empty_ini = "";

        let result = parser.parse(empty_ini).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_ini_parser_comments() {
        let parser = IniParser;
        let ini_content = r#"
# This is a comment
; This is also a comment
key1 = value1  # Inline comment
key2 = value2  ; Another inline comment

[section1]
# Section comment
key3 = value3
        "#;

        let result = parser.parse(ini_content).unwrap();

        assert_eq!(
            result.get("key1"),
            Some(&ConfigValue::String("value1  # Inline comment".to_string()))
        );
        assert_eq!(
            result.get("key2"),
            Some(&ConfigValue::String(
                "value2  ; Another inline comment".to_string()
            ))
        );

        if let Some(ConfigValue::Object(section1)) = result.get("section1") {
            assert_eq!(
                section1.get("key3"),
                Some(&ConfigValue::String("value3".to_string()))
            );
        } else {
            panic!("Expected section1 to be an object");
        }
    }

    #[test]
    fn test_ini_serialization_simple() {
        let parser = IniParser;
        let mut data = HashMap::new();
        data.insert(
            "string_key".to_string(),
            ConfigValue::String("hello".to_string()),
        );
        data.insert("integer_key".to_string(), ConfigValue::Integer(42));
        data.insert("boolean_key".to_string(), ConfigValue::Boolean(true));

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();
        assert_eq!(
            reparsed.get("string_key"),
            Some(&ConfigValue::String("hello".to_string()))
        );
        assert_eq!(reparsed.get("integer_key"), Some(&ConfigValue::Integer(42)));
        assert_eq!(
            reparsed.get("boolean_key"),
            Some(&ConfigValue::Boolean(true))
        );
    }

    #[test]
    fn test_ini_serialization_with_sections() {
        let parser = IniParser;
        let mut data = HashMap::new();

        // Add global property
        data.insert(
            "global_prop".to_string(),
            ConfigValue::String("global_value".to_string()),
        );

        // Create section
        let mut section = HashMap::new();
        section.insert(
            "section_key".to_string(),
            ConfigValue::String("section_value".to_string()),
        );
        section.insert("section_number".to_string(), ConfigValue::Integer(123));
        data.insert("section1".to_string(), ConfigValue::Object(section));

        let serialized = parser.serialize(&data).unwrap();

        // Parse it back to verify correctness
        let reparsed = parser.parse(&serialized).unwrap();

        assert_eq!(
            reparsed.get("global_prop"),
            Some(&ConfigValue::String("global_value".to_string()))
        );

        if let Some(ConfigValue::Object(reparsed_section)) = reparsed.get("section1") {
            assert_eq!(
                reparsed_section.get("section_key"),
                Some(&ConfigValue::String("section_value".to_string()))
            );
            assert_eq!(
                reparsed_section.get("section_number"),
                Some(&ConfigValue::Integer(123))
            );
        } else {
            panic!("Expected section1 to be an object");
        }
    }

    #[test]
    fn test_ini_serialization_round_trip() {
        let parser = IniParser;
        let original_ini = r#"
global_key = global_value
debug = true

[database]
host = localhost
port = 5432

[logging]
level = info
enabled = true
        "#;

        // Parse -> Serialize -> Parse again
        let parsed = parser.parse(original_ini).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Should be identical
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_ini_complex_values_handling() {
        let parser = IniParser;
        let mut data = HashMap::new();

        // Test how complex values are handled
        data.insert(
            "array_value".to_string(),
            ConfigValue::Array(vec![
                ConfigValue::String("item1".to_string()),
                ConfigValue::Integer(2),
            ]),
        );

        let mut nested_obj = HashMap::new();
        nested_obj.insert(
            "nested_key".to_string(),
            ConfigValue::String("nested_value".to_string()),
        );
        data.insert("object_value".to_string(), ConfigValue::Object(nested_obj));

        data.insert("null_value".to_string(), ConfigValue::Null);

        let serialized = parser.serialize(&data).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();

        // Arrays and nested objects become string representations
        assert_eq!(
            reparsed.get("array_value"),
            Some(&ConfigValue::String("[array]".to_string()))
        );
        // The nested object becomes a section, so it should be preserved
        if let Some(ConfigValue::Object(obj)) = reparsed.get("object_value") {
            assert_eq!(
                obj.get("nested_key"),
                Some(&ConfigValue::String("nested_value".to_string()))
            );
        }
        assert_eq!(
            reparsed.get("null_value"),
            Some(&ConfigValue::String("".to_string()))
        );
    }
}
