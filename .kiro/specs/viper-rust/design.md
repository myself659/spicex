# Design Document

## Overview

The Rust Spice implementation will be a configuration management library that provides a unified interface for reading configuration from multiple sources with a clear precedence hierarchy. The design follows Rust idioms and leverages the ecosystem's strengths in type safety, error handling, and serialization.

The core architecture centers around a `Spice` struct that manages configuration state and provides methods for reading, writing, and watching configuration. The library will use trait-based abstractions for different configuration sources and formats, enabling extensibility and testability.

## Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Spice Core    │    │  Config Sources │    │  Format Parsers │
│                 │    │                 │    │                 │
│ - State Mgmt    │◄───┤ - File Source   │    │ - JSON Parser   │
│ - Precedence    │    │ - Env Source    │    │ - YAML Parser   │
│ - Key Access    │    │ - Flag Source   │    │ - TOML Parser   │
│ - Watching      │    │ - KV Source     │    │ - INI Parser    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Value Storage  │
                    │                 │
                    │ - Layered Maps  │
                    │ - Type Coercion │
                    │ - Nested Access │
                    └─────────────────┘
```

### Key Design Principles

1. **Layered Configuration**: Each source maintains its own layer, with precedence resolved at access time
2. **Lazy Loading**: Configuration sources are loaded on-demand to improve startup performance
3. **Type Safety**: Strong typing through serde integration and generic methods
4. **Error Transparency**: Detailed error types that preserve context from all layers
5. **Zero-Copy Where Possible**: Minimize allocations through strategic use of references and Cow types

## Components and Interfaces

### Core Spice Struct

```rust
pub struct Spice {
    // Configuration layers ordered by precedence (highest first)
    layers: Vec<Box<dyn ConfigLayer>>,

    // File watching state
    watcher: Option<FileWatcher>,

    // Configuration metadata
    config_paths: Vec<PathBuf>,
    config_name: String,
    config_type: Option<ConfigFormat>,

    // Environment variable settings
    env_prefix: Option<String>,
    env_key_replacer: Option<Box<dyn Fn(&str) -> String>>,

    // Key transformation settings
    key_delimiter: String,
    automatic_env: bool,
}
```

### Configuration Layer Trait

```rust
pub trait ConfigLayer: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<ConfigValue>, ConfigError>;
    fn set(&mut self, key: &str, value: ConfigValue) -> Result<(), ConfigError>;
    fn keys(&self) -> Vec<String>;
    fn source_name(&self) -> &str;
    fn priority(&self) -> LayerPriority;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerPriority {
    Explicit = 0,    // Direct set() calls
    Flags = 1,       // Command line flags
    Environment = 2, // Environment variables
    ConfigFile = 3,  // Configuration files
    KeyValue = 4,    // Remote key-value stores
    Defaults = 5,    // Default values
}
```

### Configuration Value System

```rust
#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
    Null,
}

impl ConfigValue {
    pub fn as_str(&self) -> Option<&str> { /* ... */ }
    pub fn as_i64(&self) -> Option<i64> { /* ... */ }
    pub fn as_f64(&self) -> Option<f64> { /* ... */ }
    pub fn as_bool(&self) -> Option<bool> { /* ... */ }
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> { /* ... */ }
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> { /* ... */ }
}
```

### Format Parser Trait

```rust
pub trait ConfigParser: Send + Sync {
    fn parse(&self, content: &str) -> Result<HashMap<String, ConfigValue>, ParseError>;
    fn serialize(&self, data: &HashMap<String, ConfigValue>) -> Result<String, ParseError>;
    fn supported_extensions(&self) -> &[&str];
}
```

### File Source Implementation

```rust
pub struct FileConfigLayer {
    data: HashMap<String, ConfigValue>,
    file_path: PathBuf,
    parser: Box<dyn ConfigParser>,
    last_modified: Option<SystemTime>,
}

impl FileConfigLayer {
    pub fn new(path: PathBuf) -> Result<Self, ConfigError> {
        let parser = Self::detect_parser(&path)?;
        let mut layer = Self {
            data: HashMap::new(),
            file_path: path,
            parser,
            last_modified: None,
        };
        layer.reload()?;
        Ok(layer)
    }

    fn detect_parser(path: &Path) -> Result<Box<dyn ConfigParser>, ConfigError> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("json") => Ok(Box::new(JsonParser)),
            Some("yaml") | Some("yml") => Ok(Box::new(YamlParser)),
            Some("toml") => Ok(Box::new(TomlParser)),
            Some("ini") => Ok(Box::new(IniParser)),
            _ => Err(ConfigError::UnsupportedFormat),
        }
    }
}
```

### Environment Variable Layer

```rust
pub struct EnvConfigLayer {
    prefix: Option<String>,
    key_replacer: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,
    cached_vars: HashMap<String, String>,
    automatic: bool,
}

impl EnvConfigLayer {
    pub fn new(prefix: Option<String>, automatic: bool) -> Self {
        Self {
            prefix,
            key_replacer: None,
            cached_vars: HashMap::new(),
            automatic,
        }
    }

    fn transform_key(&self, key: &str) -> String {
        let mut env_key = key.to_uppercase().replace(".", "_");

        if let Some(replacer) = &self.key_replacer {
            env_key = replacer(&env_key);
        }

        if let Some(prefix) = &self.prefix {
            format!("{}_{}", prefix, env_key)
        } else {
            env_key
        }
    }
}
```

## Data Models

### Configuration Access Patterns

The library supports multiple access patterns for configuration values:

1. **Direct Access**: `viper.get_string("database.host")`
2. **Nested Access**: `viper.get("database")?.get("host")`
3. **Deserialization**: `viper.unmarshal::<DatabaseConfig>()`
4. **Sub-configuration**: `viper.sub("database")?.get_string("host")`

### Key Normalization

All configuration keys are normalized to lowercase with dots as separators. This ensures consistent access regardless of the source format:

- Environment: `DATABASE_HOST` → `database.host`
- JSON: `{"database": {"host": "..."}}` → `database.host`
- Command line: `--database-host` → `database.host`

### Type Coercion Rules

The library implements intelligent type coercion:

```rust
impl ConfigValue {
    pub fn coerce_to_string(&self) -> String {
        match self {
            ConfigValue::String(s) => s.clone(),
            ConfigValue::Integer(i) => i.to_string(),
            ConfigValue::Float(f) => f.to_string(),
            ConfigValue::Boolean(b) => b.to_string(),
            _ => String::new(),
        }
    }

    pub fn coerce_to_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            ConfigValue::Integer(i) => Some(*i != 0),
            _ => None,
        }
    }
}
```

## Error Handling

### Error Types Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error in {source}: {error}")]
    Parse { source: String, error: String },

    #[error("Key not found: {key}")]
    KeyNotFound { key: String },

    #[error("Type conversion error: cannot convert {from} to {to}")]
    TypeConversion { from: String, to: String },

    #[error("Unsupported configuration format")]
    UnsupportedFormat,

    #[error("File watching error: {0}")]
    FileWatch(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;
```

### Error Context Preservation

Errors maintain context about their source layer and the operation that failed:

```rust
impl Spice {
    pub fn get_string(&self, key: &str) -> ConfigResult<String> {
        self.get(key)?
            .ok_or_else(|| ConfigError::KeyNotFound { key: key.to_string() })?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| ConfigError::TypeConversion {
                from: "non-string".to_string(),
                to: "string".to_string(),
            })
    }
}
```

## Testing Strategy

### Unit Testing Approach

1. **Layer Testing**: Each configuration layer is tested in isolation
2. **Parser Testing**: Format parsers are tested with valid and invalid inputs
3. **Integration Testing**: Full Spice instances are tested with multiple sources
4. **Property Testing**: Use proptest for testing type coercion edge cases

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_precedence_order() {
        let mut viper = Spice::new();
        viper.set_default("key", "default");
        viper.set_config_file("test.json");
        viper.set_env_prefix("TEST");

        // Test that env vars override config file
        std::env::set_var("TEST_KEY", "env_value");
        assert_eq!(viper.get_string("key").unwrap(), "env_value");
    }

    #[test]
    fn test_file_watching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Test file modification detection
        let mut viper = Spice::new();
        viper.set_config_file(&config_path);
        viper.watch_config();

        // Modify file and verify reload
        std::fs::write(&config_path, r#"{"key": "new_value"}"#).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        assert_eq!(viper.get_string("key").unwrap(), "new_value");
    }
}
```

### Benchmarking Strategy

Performance benchmarks will focus on:

1. **Configuration Loading**: Time to load large configuration files
2. **Key Access**: Performance of nested key lookups
3. **Type Coercion**: Speed of value type conversions
4. **File Watching**: Overhead of file system monitoring

### Mock Testing

Mock implementations for testing:

```rust
pub struct MockConfigLayer {
    data: HashMap<String, ConfigValue>,
    priority: LayerPriority,
}

impl ConfigLayer for MockConfigLayer {
    fn get(&self, key: &str) -> Result<Option<ConfigValue>, ConfigError> {
        Ok(self.data.get(key).cloned())
    }

    // ... other trait methods
}
```

This design provides a robust, extensible, and performant configuration management system that closely mirrors Spice's functionality while leveraging Rust's type system and ecosystem strengths.