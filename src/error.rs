//! Error types and utilities for Spice configuration management.

/// Result type alias for Spice operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Comprehensive error types for configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// IO operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration parsing failed
    #[error("Parse error in {source_name}: {message}")]
    Parse {
        source_name: String,
        message: String,
    },

    /// Requested configuration key was not found
    #[error("Key not found: {key}")]
    KeyNotFound { key: String },

    /// Type conversion failed
    #[error("Type conversion error: cannot convert {from} to {to}")]
    TypeConversion { from: String, to: String },

    /// Unsupported configuration file format
    #[error("Unsupported configuration format")]
    UnsupportedFormat,

    /// File watching operation failed
    #[error("File watching error: {0}")]
    FileWatch(String),

    /// Serialization operation failed
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization operation failed
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Invalid configuration value
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::Deserialization(err.to_string())
    }
}

impl ConfigError {
    /// Creates a new parse error with context.
    pub fn parse_error(source_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            source_name: source_name.into(),
            message: message.into(),
        }
    }

    /// Creates a new type conversion error.
    pub fn type_conversion(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::TypeConversion {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Creates a new key not found error.
    pub fn key_not_found(key: impl Into<String>) -> Self {
        Self::KeyNotFound { key: key.into() }
    }

    /// Creates a new file watch error.
    pub fn file_watch(message: impl Into<String>) -> Self {
        Self::FileWatch(message.into())
    }

    /// Creates a new serialization error.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Creates a new deserialization error.
    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::Deserialization(message.into())
    }

    /// Creates a new invalid value error.
    pub fn invalid_value(message: impl Into<String>) -> Self {
        Self::InvalidValue(message.into())
    }

    /// Creates a new unsupported operation error.
    pub fn unsupported_operation(message: impl Into<String>) -> Self {
        Self::UnsupportedOperation(message.into())
    }

    /// Creates a new parse error with context (alias for parse_error).
    pub fn parse(source_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::parse_error(source_name, message)
    }

    /// Returns true if this error is related to a missing key.
    pub fn is_key_not_found(&self) -> bool {
        matches!(self, ConfigError::KeyNotFound { .. })
    }

    /// Returns true if this error is related to type conversion.
    pub fn is_type_conversion(&self) -> bool {
        matches!(self, ConfigError::TypeConversion { .. })
    }

    /// Returns true if this error is related to parsing.
    pub fn is_parse_error(&self) -> bool {
        matches!(self, ConfigError::Parse { .. })
    }

    /// Returns true if this error is related to IO operations.
    pub fn is_io_error(&self) -> bool {
        matches!(self, ConfigError::Io(_))
    }
}

/// Extension trait for adding context to Results.
pub trait ConfigResultExt<T> {
    /// Adds context to a ConfigError if the result is an error.
    fn with_context<F>(self, f: F) -> ConfigResult<T>
    where
        F: FnOnce() -> String;

    /// Maps a ConfigError to a different ConfigError variant.
    fn map_config_err<F>(self, f: F) -> ConfigResult<T>
    where
        F: FnOnce(ConfigError) -> ConfigError;
}

impl<T> ConfigResultExt<T> for ConfigResult<T> {
    fn with_context<F>(self, f: F) -> ConfigResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|err| match err {
            ConfigError::Parse {
                source_name,
                message,
            } => ConfigError::Parse {
                source_name,
                message: format!("{}: {}", f(), message),
            },
            other => other,
        })
    }

    fn map_config_err<F>(self, f: F) -> ConfigResult<T>
    where
        F: FnOnce(ConfigError) -> ConfigError,
    {
        self.map_err(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        let error = ConfigError::key_not_found("test.key");
        assert!(matches!(error, ConfigError::KeyNotFound { .. }));

        let error = ConfigError::type_conversion("string", "int");
        assert!(matches!(error, ConfigError::TypeConversion { .. }));

        let error = ConfigError::parse_error("config.json", "invalid syntax");
        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    fn test_error_display() {
        let error = ConfigError::key_not_found("database.host");
        assert_eq!(error.to_string(), "Key not found: database.host");

        let error = ConfigError::type_conversion("string", "integer");
        assert_eq!(
            error.to_string(),
            "Type conversion error: cannot convert string to integer"
        );

        let error = ConfigError::parse_error("config.yaml", "invalid YAML syntax");
        assert_eq!(
            error.to_string(),
            "Parse error in config.yaml: invalid YAML syntax"
        );
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let config_error: ConfigError = io_error.into();
        assert!(matches!(config_error, ConfigError::Io(_)));
    }

    #[test]
    fn test_error_context_preservation() {
        // Test that error context is preserved through helper methods
        let error = ConfigError::parse_error("test.toml", "missing closing bracket");
        if let ConfigError::Parse {
            source_name,
            message,
        } = error
        {
            assert_eq!(source_name, "test.toml");
            assert_eq!(message, "missing closing bracket");
        } else {
            panic!("Expected Parse error variant");
        }
    }

    #[test]
    fn test_all_error_variants() {
        // Test all error variants can be created and displayed
        let errors = vec![
            ConfigError::UnsupportedFormat,
            ConfigError::FileWatch("watch failed".to_string()),
            ConfigError::Serialization("serialize failed".to_string()),
            ConfigError::InvalidValue("invalid value".to_string()),
        ];

        for error in errors {
            // Ensure all errors implement Display
            let _display = error.to_string();
            // Ensure all errors implement Debug
            let _debug = format!("{:?}", error);
        }
    }

    #[test]
    fn test_error_helper_methods() {
        // Test all error creation helper methods
        let file_watch_error = ConfigError::file_watch("file system error");
        assert!(matches!(file_watch_error, ConfigError::FileWatch(_)));

        let serialization_error = ConfigError::serialization("JSON serialization failed");
        assert!(matches!(serialization_error, ConfigError::Serialization(_)));

        let invalid_value_error = ConfigError::invalid_value("value out of range");
        assert!(matches!(invalid_value_error, ConfigError::InvalidValue(_)));
    }

    #[test]
    fn test_error_type_checking() {
        let key_error = ConfigError::key_not_found("test.key");
        assert!(key_error.is_key_not_found());
        assert!(!key_error.is_type_conversion());
        assert!(!key_error.is_parse_error());
        assert!(!key_error.is_io_error());

        let type_error = ConfigError::type_conversion("string", "int");
        assert!(!type_error.is_key_not_found());
        assert!(type_error.is_type_conversion());
        assert!(!type_error.is_parse_error());
        assert!(!type_error.is_io_error());

        let parse_error = ConfigError::parse_error("config.json", "syntax error");
        assert!(!parse_error.is_key_not_found());
        assert!(!parse_error.is_type_conversion());
        assert!(parse_error.is_parse_error());
        assert!(!parse_error.is_io_error());

        let io_error = ConfigError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert!(!io_error.is_key_not_found());
        assert!(!io_error.is_type_conversion());
        assert!(!io_error.is_parse_error());
        assert!(io_error.is_io_error());
    }

    #[test]
    fn test_config_result_ext() {
        use super::ConfigResultExt;

        // Test with_context
        let result: ConfigResult<String> =
            Err(ConfigError::parse_error("test.json", "invalid syntax"));
        let result_with_context = result.with_context(|| "while loading configuration".to_string());

        if let Err(ConfigError::Parse {
            source_name,
            message,
        }) = result_with_context
        {
            assert_eq!(source_name, "test.json");
            assert!(message.contains("while loading configuration"));
            assert!(message.contains("invalid syntax"));
        } else {
            panic!("Expected Parse error with context");
        }

        // Test map_config_err
        let result: ConfigResult<String> = Err(ConfigError::key_not_found("test.key"));
        let mapped_result = result.map_config_err(|_| ConfigError::invalid_value("mapped error"));

        assert!(matches!(mapped_result, Err(ConfigError::InvalidValue(_))));
    }

    #[test]
    fn test_error_propagation() {
        // Test that errors propagate correctly through Result chains
        fn inner_function() -> ConfigResult<String> {
            Err(ConfigError::key_not_found("inner.key"))
        }

        fn outer_function() -> ConfigResult<String> {
            inner_function().map_config_err(|err| {
                ConfigError::parse_error("outer", format!("inner error: {}", err))
            })
        }

        let result = outer_function();
        assert!(result.is_err());
        if let Err(ConfigError::Parse {
            source_name,
            message,
        }) = result
        {
            assert_eq!(source_name, "outer");
            assert!(message.contains("inner error"));
            assert!(message.contains("Key not found: inner.key"));
        } else {
            panic!("Expected Parse error with propagated context");
        }
    }
}
