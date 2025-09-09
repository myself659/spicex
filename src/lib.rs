//! # SPICE
//!
//! A complete configuration solution for Rust applications, inspired by Spice.
//!
//! Spice is designed to work within an application and can handle all types of
//! configuration needs and formats. It supports:
//!
//! - Setting defaults
//! - Reading from JSON, TOML, YAML, INI configuration files
//! - Reading from environment variables
//! - Reading from command line flags
//! - Reading from remote config systems (etcd, Consul)
//! - Watching and re-reading of config files (live reload)
//!
//! ## Architecture Overview
//!
//! Spice uses a layered configuration approach where different sources have different
//! precedence levels. The precedence order (highest to lowest) is:
//!
//! 1. **Explicit calls** - Values set directly via `set()` method
//! 2. **Command line flags** - Values from CLI arguments
//! 3. **Environment variables** - Values from environment
//! 4. **Configuration files** - Values from config files
//! 5. **Key-value stores** - Values from remote stores
//! 6. **Default values** - Fallback values
//!
//! ## Quick Start
//!
//! ```rust
//! use spice::{Spice, ConfigValue};
//! use std::collections::HashMap;
//!
//! // Create a new Spice instance
//! let mut viper = Spice::new();
//!
//! // Set some default values
//! viper.set_default("database.host", ConfigValue::from("localhost")).unwrap();
//! viper.set_default("database.port", ConfigValue::from(5432i64)).unwrap();
//!
//! // Load configuration from a file
//! viper.set_config_name("config");
//! viper.add_config_path("./configs");
//! // viper.read_in_config().unwrap(); // Uncomment when config file exists
//!
//! // Set environment variable prefix
//! viper.set_env_prefix("MYAPP");
//! viper.set_automatic_env(true);
//!
//! // Access configuration values
//! let host = viper.get_string("database.host").unwrap();
//! let port = viper.get_i64("database.port").unwrap();
//!
//! println!("Database: {}:{}", host.unwrap_or_default(), port.unwrap_or_default());
//! ```
//!
//! ## Configuration File Formats
//!
//! Spice supports multiple configuration file formats:
//!
//! ### JSON
//! ```json
//! {
//!   "database": {
//!     "host": "localhost",
//!     "port": 5432
//!   },
//!   "debug": true
//! }
//! ```
//!
//! ### YAML
//! ```yaml
//! database:
//!   host: localhost
//!   port: 5432
//! debug: true
//! ```
//!
//! ### TOML
//! ```toml
//! debug = true
//!
//! [database]
//! host = "localhost"
//! port = 5432
//! ```
//!
//! ### INI
//! ```ini
//! debug = true
//!
//! [database]
//! host = localhost
//! port = 5432
//! ```
//!
//! ## Environment Variables
//!
//! Environment variables are automatically mapped to configuration keys:
//!
//! ```bash
//! export MYAPP_DATABASE_HOST=localhost
//! export MYAPP_DATABASE_PORT=5432
//! export MYAPP_DEBUG=true
//! ```
//!
//! These will be available as `database.host`, `database.port`, and `debug` respectively.
//!
//! ## Command Line Flags
//!
//! When using the `cli` feature, command line flags can be integrated:
//!
//! ```rust
//! # #[cfg(feature = "cli")]
//! # {
//! use spice::{Spice, FlagConfigLayer};
//! use clap::{Arg, Command};
//!
//! let app = Command::new("myapp")
//!     .arg(Arg::new("host")
//!         .long("host")
//!         .value_name("HOST")
//!         .help("Database host"));
//!
//! let args = vec!["myapp", "--host", "localhost"];
//! let matches = app.try_get_matches_from(args).unwrap();
//!
//! let mut viper = Spice::new();
//! viper.bind_flags(matches);
//! # }
//! ```
//!
//! ## File Watching
//!
//! Spice can watch configuration files for changes and automatically reload:
//!
//! ```rust,no_run
//! use spice::Spice;
//!
//! let mut viper = Spice::new();
//! viper.set_config_file("./config.json").unwrap();
//! viper.watch_config().unwrap();
//!
//! // Register a callback for configuration changes
//! viper.on_config_change(|| {
//!     println!("Configuration changed!");
//! }).unwrap();
//! ```
//!
//! ## Struct Deserialization
//!
//! Configuration can be deserialized into Rust structs using serde:
//!
//! ```rust
//! use serde::Deserialize;
//! use spice::{Spice, ConfigValue};
//!
//! #[derive(Deserialize, Debug)]
//! struct DatabaseConfig {
//!     host: String,
//!     port: u16,
//!     ssl: bool,
//! }
//!
//! #[derive(Deserialize, Debug)]
//! struct AppConfig {
//!     database: DatabaseConfig,
//!     debug: bool,
//! }
//!
//! let mut viper = Spice::new();
//! viper.set_default("database.host", ConfigValue::from("localhost")).unwrap();
//! viper.set_default("database.port", ConfigValue::from(5432i64)).unwrap();
//! viper.set_default("database.ssl", ConfigValue::from(false)).unwrap();
//! viper.set_default("debug", ConfigValue::from(true)).unwrap();
//!
//! let config: AppConfig = viper.unmarshal().unwrap();
//! println!("Config: {:?}", config);
//! ```
//!
//! ## Error Handling
//!
//! All operations return `ConfigResult<T>` which is an alias for `Result<T, ConfigError>`.
//! The `ConfigError` enum provides detailed error information:
//!
//! ```rust
//! use spice::{Spice, ConfigError};
//!
//! let mut viper = Spice::new();
//! match viper.get_string("nonexistent.key") {
//!     Ok(Some(value)) => println!("Value: {}", value),
//!     Ok(None) => println!("Key not found"),
//!     Err(ConfigError::KeyNotFound { key }) => println!("Key '{}' not found", key),
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```

pub mod config;
pub mod default_layer;
pub mod env_layer;
pub mod error;
pub mod file_layer;
pub mod layer;
pub mod parser;
pub mod value;
pub mod watcher;

// Re-export main types for convenience
pub use config::Spice;
pub use default_layer::DefaultConfigLayer;
pub use env_layer::EnvConfigLayer;
pub use error::{ConfigError, ConfigResult};
pub use file_layer::FileConfigLayer;
pub use layer::{ConfigLayer, LayerPriority};
pub use value::ConfigValue;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "cli")]
pub use cli::FlagConfigLayer;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
