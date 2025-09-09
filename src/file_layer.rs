//! File-based configuration layer implementation.

use crate::error::{ConfigError, ConfigResult};
use crate::layer::{ConfigLayer, LayerPriority};
use crate::parser::{detect_parser_by_extension, ConfigParser};
use crate::value::ConfigValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File-based configuration layer that loads configuration from files.
pub struct FileConfigLayer {
    /// Parsed configuration data
    data: HashMap<String, ConfigValue>,
    /// Path to the configuration file
    file_path: PathBuf,
    /// Parser for the file format
    parser: Box<dyn ConfigParser>,
    /// Last modification time for change detection
    last_modified: Option<SystemTime>,
    /// Source name for error reporting
    source_name: String,
}

impl std::fmt::Debug for FileConfigLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileConfigLayer")
            .field("data", &self.data)
            .field("file_path", &self.file_path)
            .field("parser", &self.parser.name())
            .field("last_modified", &self.last_modified)
            .field("source_name", &self.source_name)
            .finish()
    }
}

impl FileConfigLayer {
    /// Creates a new FileConfigLayer from a file path.
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    /// * `ConfigResult<Self>` - The created layer or an error
    ///
    /// # Errors
    /// * `ConfigError::UnsupportedFormat` - If the file extension is not supported
    /// * `ConfigError::Io` - If the file cannot be read
    /// * `ConfigError::Parse` - If the file content cannot be parsed
    pub fn new<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref().to_path_buf();
        let parser = Self::detect_parser(&path)?;
        let source_name = path.display().to_string();

        let mut layer = Self {
            data: HashMap::new(),
            file_path: path,
            parser,
            last_modified: None,
            source_name,
        };

        layer.load_file()?;
        Ok(layer)
    }

    /// Creates a new FileConfigLayer with explicit parser.
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    /// * `parser` - Parser to use for this file
    ///
    /// # Returns
    /// * `ConfigResult<Self>` - The created layer or an error
    pub fn with_parser<P: AsRef<Path>>(
        path: P,
        parser: Box<dyn ConfigParser>,
    ) -> ConfigResult<Self> {
        let path = path.as_ref().to_path_buf();
        let source_name = path.display().to_string();

        let mut layer = Self {
            data: HashMap::new(),
            file_path: path,
            parser,
            last_modified: None,
            source_name,
        };

        layer.load_file()?;
        Ok(layer)
    }

    /// Detects the appropriate parser based on file extension.
    fn detect_parser(path: &Path) -> ConfigResult<Box<dyn ConfigParser>> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or(ConfigError::UnsupportedFormat)?;

        detect_parser_by_extension(extension)
    }

    /// Loads and parses the configuration file.
    fn load_file(&mut self) -> ConfigResult<()> {
        // Check if file exists
        if !self.file_path.exists() {
            return Err(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Configuration file not found: {}", self.file_path.display()),
            )));
        }

        // Read file content
        let content = fs::read_to_string(&self.file_path).map_err(ConfigError::Io)?;

        // Parse content
        self.data = self.parser.parse(&content).map_err(|e| match e {
            ConfigError::Parse {
                source_name: _,
                message,
            } => ConfigError::parse_error(&self.source_name, message),
            other => other,
        })?;

        // Update last modified time
        self.last_modified = fs::metadata(&self.file_path)
            .and_then(|metadata| metadata.modified())
            .ok();

        Ok(())
    }

    /// Reloads the configuration from the file.
    ///
    /// This method refreshes the configuration data by re-reading and parsing the file.
    /// It's useful for implementing file watching and live reload functionality.
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success or an error if reload fails
    ///
    /// # Errors
    /// * `ConfigError::Io` - If the file cannot be read
    /// * `ConfigError::Parse` - If the file content cannot be parsed
    pub fn reload(&mut self) -> ConfigResult<()> {
        self.load_file()
    }

    /// Checks if the file has been modified since last load.
    ///
    /// # Returns
    /// * `ConfigResult<bool>` - True if the file has been modified, false otherwise
    ///
    /// # Errors
    /// * `ConfigError::Io` - If file metadata cannot be accessed
    pub fn is_modified(&self) -> ConfigResult<bool> {
        if let Some(last_modified) = self.last_modified {
            let current_modified = fs::metadata(&self.file_path)
                .and_then(|metadata| metadata.modified())
                .map_err(ConfigError::Io)?;

            Ok(current_modified > last_modified)
        } else {
            // If we don't have a last modified time, consider it modified
            Ok(true)
        }
    }

    /// Returns the file path of this configuration layer.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Returns the last modification time of the file when it was loaded.
    pub fn last_modified(&self) -> Option<SystemTime> {
        self.last_modified
    }

    /// Returns the parser used by this layer.
    pub fn parser(&self) -> &dyn ConfigParser {
        self.parser.as_ref()
    }
}

impl FileConfigLayer {
    /// Helper function to set nested values recursively.
    fn set_nested_value(
        data: &mut HashMap<String, ConfigValue>,
        keys: &[&str],
        value: ConfigValue,
    ) -> ConfigResult<()> {
        if keys.is_empty() {
            return Err(ConfigError::invalid_value("Empty key path"));
        }

        if keys.len() == 1 {
            // Final key - set the value
            data.insert(keys[0].to_string(), value);
            return Ok(());
        }

        // Get or create the intermediate key
        let key_part = keys[0];
        let entry = data
            .entry(key_part.to_string())
            .or_insert_with(|| ConfigValue::Object(HashMap::new()));

        // Ensure it's an object
        if let ConfigValue::Object(ref mut nested_obj) = entry {
            // Recursively set the remaining path
            Self::set_nested_value(nested_obj, &keys[1..], value)
        } else {
            // Intermediate key exists but is not an object - replace it
            *entry = ConfigValue::Object(HashMap::new());
            if let ConfigValue::Object(ref mut nested_obj) = entry {
                Self::set_nested_value(nested_obj, &keys[1..], value)
            } else {
                Err(ConfigError::invalid_value("Failed to create nested object"))
            }
        }
    }
}

impl ConfigLayer for FileConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        // Handle nested key access with dot notation
        let keys: Vec<&str> = key.split('.').collect();
        let mut current = &self.data;

        for (i, &key_part) in keys.iter().enumerate() {
            if let Some(value) = current.get(key_part) {
                if i == keys.len() - 1 {
                    // This is the final key, return the value
                    return Ok(Some(value.clone()));
                } else {
                    // This is an intermediate key, continue traversing
                    if let Some(nested_obj) = value.as_object() {
                        current = nested_obj;
                    } else {
                        // Path doesn't exist (intermediate key is not an object)
                        return Ok(None);
                    }
                }
            } else {
                // Key not found
                return Ok(None);
            }
        }

        Ok(None)
    }

    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        // File layers are typically read-only, but we can support in-memory modifications
        // Handle nested key setting with dot notation
        let keys: Vec<&str> = key.split('.').collect();

        if keys.is_empty() {
            return Err(ConfigError::invalid_value("Empty key"));
        }

        if keys.len() == 1 {
            // Simple key
            self.data.insert(key.to_string(), value);
        } else {
            // Nested key - need to create nested structure
            Self::set_nested_value(&mut self.data, &keys, value)?;
        }

        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        // Return all keys including nested ones with dot notation
        let mut all_keys = Vec::new();
        collect_keys(&self.data, String::new(), &mut all_keys);
        all_keys.sort();
        all_keys
    }

    fn source_name(&self) -> &str {
        &self.source_name
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::ConfigFile
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Recursively collects all keys from a nested configuration structure.
fn collect_keys(data: &HashMap<String, ConfigValue>, prefix: String, keys: &mut Vec<String>) {
    for (key, value) in data {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        keys.push(full_key.clone());

        // If this value is an object, recursively collect its keys
        if let ConfigValue::Object(nested_obj) = value {
            collect_keys(nested_obj, full_key, keys);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_json_file(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_file_config_layer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"
        {
            "database": {
                "host": "localhost",
                "port": 5432
            },
            "debug": true,
            "name": "test-app"
        }
        "#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let layer = FileConfigLayer::new(&file_path).unwrap();

        assert_eq!(layer.source_name(), file_path.display().to_string());
        assert_eq!(layer.priority(), LayerPriority::ConfigFile);
        assert_eq!(layer.file_path(), file_path.as_path());
        assert!(layer.last_modified().is_some());
    }

    #[test]
    fn test_file_config_layer_get_simple_keys() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"
        {
            "name": "test-app",
            "debug": true,
            "port": 8080,
            "timeout": 30.5
        }
        "#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let layer = FileConfigLayer::new(&file_path).unwrap();

        assert_eq!(
            layer.get("name").unwrap(),
            Some(ConfigValue::String("test-app".to_string()))
        );
        assert_eq!(
            layer.get("debug").unwrap(),
            Some(ConfigValue::Boolean(true))
        );
        assert_eq!(layer.get("port").unwrap(), Some(ConfigValue::Integer(8080)));
        assert_eq!(
            layer.get("timeout").unwrap(),
            Some(ConfigValue::Float(30.5))
        );
        assert_eq!(layer.get("nonexistent").unwrap(), None);
    }

    #[test]
    fn test_file_config_layer_get_nested_keys() {
        let temp_dir = TempDir::new().unwrap();
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
            "cache": {
                "enabled": true
            }
        }
        "#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let layer = FileConfigLayer::new(&file_path).unwrap();

        assert_eq!(
            layer.get("database.host").unwrap(),
            Some(ConfigValue::String("localhost".to_string()))
        );
        assert_eq!(
            layer.get("database.port").unwrap(),
            Some(ConfigValue::Integer(5432))
        );
        assert_eq!(
            layer.get("database.credentials.username").unwrap(),
            Some(ConfigValue::String("admin".to_string()))
        );
        assert_eq!(
            layer.get("database.credentials.password").unwrap(),
            Some(ConfigValue::String("secret".to_string()))
        );
        assert_eq!(
            layer.get("cache.enabled").unwrap(),
            Some(ConfigValue::Boolean(true))
        );

        // Test non-existent nested keys
        assert_eq!(layer.get("database.nonexistent").unwrap(), None);
        assert_eq!(layer.get("nonexistent.key").unwrap(), None);
        assert_eq!(layer.get("database.host.invalid").unwrap(), None);
    }

    #[test]
    fn test_file_config_layer_set_simple_keys() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"{"existing": "value"}"#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let mut layer = FileConfigLayer::new(&file_path).unwrap();

        // Set new key
        layer
            .set("new_key", ConfigValue::String("new_value".to_string()))
            .unwrap();
        assert_eq!(
            layer.get("new_key").unwrap(),
            Some(ConfigValue::String("new_value".to_string()))
        );

        // Override existing key
        layer.set("existing", ConfigValue::Integer(42)).unwrap();
        assert_eq!(
            layer.get("existing").unwrap(),
            Some(ConfigValue::Integer(42))
        );
    }

    #[test]
    fn test_file_config_layer_set_nested_keys() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"{}"#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let mut layer = FileConfigLayer::new(&file_path).unwrap();

        // Set nested key (creates intermediate objects)
        layer
            .set(
                "database.host",
                ConfigValue::String("localhost".to_string()),
            )
            .unwrap();
        assert_eq!(
            layer.get("database.host").unwrap(),
            Some(ConfigValue::String("localhost".to_string()))
        );

        // Set another key in the same nested object
        layer
            .set("database.port", ConfigValue::Integer(5432))
            .unwrap();
        assert_eq!(
            layer.get("database.port").unwrap(),
            Some(ConfigValue::Integer(5432))
        );

        // Set deeply nested key
        layer
            .set(
                "app.cache.redis.host",
                ConfigValue::String("redis-server".to_string()),
            )
            .unwrap();
        assert_eq!(
            layer.get("app.cache.redis.host").unwrap(),
            Some(ConfigValue::String("redis-server".to_string()))
        );
    }

    #[test]
    fn test_file_config_layer_keys() {
        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"
        {
            "simple_key": "value",
            "database": {
                "host": "localhost",
                "credentials": {
                    "username": "admin"
                }
            },
            "cache": {
                "enabled": true
            }
        }
        "#;

        let file_path = create_test_json_file(&temp_dir, "config.json", json_content);
        let layer = FileConfigLayer::new(&file_path).unwrap();

        let keys = layer.keys();
        let expected_keys = vec![
            "cache",
            "cache.enabled",
            "database",
            "database.credentials",
            "database.credentials.username",
            "database.host",
            "simple_key",
        ];

        assert_eq!(keys, expected_keys);
    }

    #[test]
    fn test_file_config_layer_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.unknown");
        fs::write(&file_path, "content").unwrap();

        let result = FileConfigLayer::new(&file_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::UnsupportedFormat
        ));
    }

    #[test]
    fn test_file_config_layer_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");

        let result = FileConfigLayer::new(&file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_)));
    }

    #[test]
    fn test_file_config_layer_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_json = r#"{"invalid": json}"#;

        let file_path = create_test_json_file(&temp_dir, "config.json", invalid_json);
        let result = FileConfigLayer::new(&file_path);

        assert!(result.is_err());
        if let Err(ConfigError::Parse {
            source_name,
            message: _,
        }) = result
        {
            assert!(source_name.contains("config.json"));
        } else {
            panic!("Expected Parse error");
        }
    }

    #[test]
    fn test_file_config_layer_reload() {
        let temp_dir = TempDir::new().unwrap();
        let initial_content = r#"{"key": "initial_value"}"#;

        let file_path = create_test_json_file(&temp_dir, "config.json", initial_content);
        let mut layer = FileConfigLayer::new(&file_path).unwrap();

        assert_eq!(
            layer.get("key").unwrap(),
            Some(ConfigValue::String("initial_value".to_string()))
        );

        // Modify the file
        let updated_content = r#"{"key": "updated_value", "new_key": 42}"#;
        fs::write(&file_path, updated_content).unwrap();

        // Reload the layer
        layer.reload().unwrap();

        assert_eq!(
            layer.get("key").unwrap(),
            Some(ConfigValue::String("updated_value".to_string()))
        );
        assert_eq!(
            layer.get("new_key").unwrap(),
            Some(ConfigValue::Integer(42))
        );
    }

    #[test]
    fn test_file_config_layer_is_modified() {
        let temp_dir = TempDir::new().unwrap();
        let initial_content = r#"{"key": "value"}"#;

        let file_path = create_test_json_file(&temp_dir, "config.json", initial_content);
        let layer = FileConfigLayer::new(&file_path).unwrap();

        // Initially should not be modified
        assert!(!layer.is_modified().unwrap());

        // Wait a bit to ensure different modification time
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Modify the file
        let updated_content = r#"{"key": "updated_value"}"#;
        fs::write(&file_path, updated_content).unwrap();

        // Should now be modified
        assert!(layer.is_modified().unwrap());
    }

    #[test]
    fn test_file_config_layer_different_formats() {
        let temp_dir = TempDir::new().unwrap();

        // Test YAML
        let yaml_content = r#"
database:
  host: localhost
  port: 5432
debug: true
        "#;
        let yaml_path = temp_dir.path().join("config.yaml");
        fs::write(&yaml_path, yaml_content).unwrap();
        let yaml_layer = FileConfigLayer::new(&yaml_path).unwrap();

        assert_eq!(
            yaml_layer.get("database.host").unwrap(),
            Some(ConfigValue::String("localhost".to_string()))
        );
        assert_eq!(
            yaml_layer.get("debug").unwrap(),
            Some(ConfigValue::Boolean(true))
        );

        // Test TOML
        let toml_content = r#"
debug = true

[database]
host = "localhost"
port = 5432
        "#;
        let toml_path = temp_dir.path().join("config.toml");
        fs::write(&toml_path, toml_content).unwrap();
        let toml_layer = FileConfigLayer::new(&toml_path).unwrap();

        assert_eq!(
            toml_layer.get("database.host").unwrap(),
            Some(ConfigValue::String("localhost".to_string()))
        );
        assert_eq!(
            toml_layer.get("debug").unwrap(),
            Some(ConfigValue::Boolean(true))
        );

        // Test INI
        let ini_content = r#"
debug = true

[database]
host = localhost
port = 5432
        "#;
        let ini_path = temp_dir.path().join("config.ini");
        fs::write(&ini_path, ini_content).unwrap();
        let ini_layer = FileConfigLayer::new(&ini_path).unwrap();

        assert_eq!(
            ini_layer.get("database.host").unwrap(),
            Some(ConfigValue::String("localhost".to_string()))
        );
        assert_eq!(
            ini_layer.get("debug").unwrap(),
            Some(ConfigValue::Boolean(true))
        );
    }

    #[test]
    fn test_collect_keys_function() {
        let mut data = HashMap::new();
        data.insert(
            "simple".to_string(),
            ConfigValue::String("value".to_string()),
        );

        let mut nested = HashMap::new();
        nested.insert("inner".to_string(), ConfigValue::Integer(42));
        data.insert("nested".to_string(), ConfigValue::Object(nested));

        let mut keys = Vec::new();
        collect_keys(&data, String::new(), &mut keys);
        keys.sort();

        let expected = vec!["nested", "nested.inner", "simple"];
        assert_eq!(keys, expected);
    }

    #[test]
    fn test_file_config_layer_with_explicit_parser() {
        use crate::parser::JsonParser;

        let temp_dir = TempDir::new().unwrap();
        let json_content = r#"{"key": "value"}"#;

        // Create file with .txt extension but JSON content
        let file_path = temp_dir.path().join("config.txt");
        fs::write(&file_path, json_content).unwrap();

        // Should fail with auto-detection
        let result = FileConfigLayer::new(&file_path);
        assert!(result.is_err());

        // Should work with explicit parser
        let layer = FileConfigLayer::with_parser(&file_path, Box::new(JsonParser)).unwrap();
        assert_eq!(
            layer.get("key").unwrap(),
            Some(ConfigValue::String("value".to_string()))
        );
    }
}
